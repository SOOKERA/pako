// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! The Direct node: one iroh endpoint for this Pako, its contacts, live connections, the
//! one-time handshake and file transfers. Works without the window (tests drive it directly);
//! `mod.rs` wires it to Tauri.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::{Endpoint, EndpointAddr, EndpointId, Watcher};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot, Notify};
use tokio_util::sync::CancellationToken;

use super::pace::{self, Tries};
use super::proto::{self, FileHave, Msg, OfferFile, Part, Restart, ALPN, MAX_HEADER, MAX_MSG};
use super::resume::{self, InNote, OutFile, OutNote, Ranges};
use crate::notices::Go;
use super::store::{self, Blocked, Contact, HistoryItem, Store};

#[path = "devices.rs"]
pub mod devices;
#[path = "relay.rs"]
pub mod relay;

/// Size of each piece a file is cut into, so one big file uses several streams at once.
const PIECE: u64 = 8 * 1024 * 1024;
/// Streams sending at the same time for one transfer.
const STREAMS: usize = 4;
/// Read/write buffer.
const BUF: usize = 256 * 1024;
/// How long trying to reach a contact may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Room always kept free on the disk on top of what an offer needs (or 5% of the drive, if more).
const DISK_MARGIN_MIN: u64 = 1024 * 1024 * 1024;
/// A transfer where nothing moves for this long stops (the other side went quiet).
const STALL: Duration = Duration::from_secs(120);
/// After "all sent", how long to wait for the last pieces with nothing arriving.
const LAST_PIECES_QUIET: Duration = Duration::from_secs(30);
/// Sends from one contact waiting for your answer at once; more are turned away.
const MAX_ASKING: usize = 10;
/// "… wants to connect" questions open at once; more people trying now are turned away (they
/// try again later by themselves).
const MAX_REQUESTS: usize = 20;
/// Sends one contact may offer in a minute. Past it they're turned away with a reason, and past
/// twice that without an answer: a contact's Pako gone wrong (or hostile) can't fill the screen
/// with questions, push your history out, or make this Pako queue answers without end.
const MAX_OFFERS_PER_MINUTE: usize = 20;
/// A connection kept this long is no longer one of two made at the same moment: when the same
/// contact connects again, their Pako has lost the old one (restarted, slept, changed network).
const SETTLED_AFTER: Duration = Duration::from_secs(30);
/// Longest text kept from the other side's "couldn't take it" answer.
const MAX_PEER_MESSAGE: usize = 300;
/// A contact that can't be reached while the internet is down (they're not on this network).
const NO_INTERNET: &str = "Not reachable without internet";
/// How long someone you declined or removed is turned away without asking you again.
const BLOCK_FOR_MS: u64 = 7 * 24 * 3600 * 1000;
/// While a transfer waits for a contact to come back, they're tried this often (others every
/// few minutes), for its first hour of waiting.
const RETRY_WAITING: Duration = Duration::from_secs(30);
const RETRY_WAITING_FOR: u64 = 3600 * 1000;
/// How often the receiver syncs what arrived to the disk and saves its note.
const NOTE_EVERY: Duration = Duration::from_secs(1);
/// Why a transfer waiting for the other PC was cleared, on each side.
const EXPIRED_OUT: &str = "Expired: the other PC didn't come back within 7 days.";
const EXPIRED_IN: &str = "Expired: the other PC didn't come back within 7 days, so the part that arrived was deleted.";

/// Room kept free on a drive of `size` bytes: 1 GB, or 5% of the drive if that's more.
fn margin_for(size: u64) -> u64 {
    DISK_MARGIN_MIN.max(size / 20)
}

pub type Changed = Arc<dyn Fn() + Send + Sync>;
/// A notification: its kind (see notices.rs), title, body and where a click leads.
pub type Notify2 = Arc<dyn Fn(&str, &str, &str, Go) + Send + Sync>;
/// A question settled (see notices.rs `resolve`): its kind ("direct-offer" with the offer's
/// id, "direct-request" with the person's id) and how it ended ("Accepted", "Declined"...).
pub type Settle = Arc<dyn Fn(&str, &str, &str) + Send + Sync>;

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn human(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{n} B") } else { format!("{v:.1} {}", U[i]) }
}

/// How a connection travels right now: "local" (the home network), "direct6", "direct" or
/// "relay".
fn path_of(conn: &Connection) -> Option<String> {
    let paths = conn.paths();
    let sel = paths.iter().find(|p| p.is_selected())?;
    Some(super::local::route_of(sel.remote_addr()).into())
}

struct Live {
    conn: Connection,
    tx: mpsc::UnboundedSender<Msg>,
    /// Who opened this connection (the same pair keeps the one opened by the smaller ID).
    initiator: String,
    /// When it was kept.
    since: std::time::Instant,
    /// Last time something came in on it (a quiet one may close: see `pace.rs`).
    used: std::time::Instant,
}

/// A contact is connected and connects again: whether the new connection takes the place of
/// the one kept. Two made at the same moment (both sides dialled): the one opened by the
/// smaller ID stays, so both sides keep the same one. Otherwise the new one wins: a Pako only
/// dials a contact it isn't connected to, so the kept one is dead on their side (their Pako
/// restarted or lost it) even if this side hasn't noticed yet.
fn replaces(old_opener: &str, old_age: Duration, new_opener: &str) -> bool {
    new_opener == old_opener || old_age >= SETTLED_AFTER || new_opener < old_opener
}

/// "… wants to connect" questions from people who aren't contacts, in ten minutes, all of
/// them together. Anyone who has this Pako ID (or sees it on the same network) can make new
/// keys at will and ask again as soon as a question closes: past this, they're turned away
/// without a question or a notification.
const MAX_NEW_REQUESTS: usize = 10;
const NEW_REQUESTS_WINDOW: Duration = Duration::from_secs(600);

/// Counts one more such question at `now`: false past the limit (not asked, not counted).
fn request_ok(times: &mut VecDeque<std::time::Instant>, now: std::time::Instant) -> bool {
    while times.front().is_some_and(|t| now.duration_since(*t) >= NEW_REQUESTS_WINDOW) {
        times.pop_front();
    }
    if times.len() >= MAX_NEW_REQUESTS {
        return false;
    }
    times.push_back(now);
    true
}

/// What to do with one more offer from a contact.
#[derive(Debug, PartialEq)]
enum Flood {
    Fine,
    /// Too many in a minute: turned away, with the reason.
    TurnAway,
    /// Far too many: not even answered.
    Ignore,
}

/// Name changes and "my device" messages one contact may send in a minute (each one rewrites
/// contacts.json and may show a notification); more are ignored. A normal Pako sends a few.
const MAX_SMALL_PER_MINUTE: usize = 30;

/// Counts one more such message at `now`: false past the limit (ignored, not counted).
fn small_ok(times: &mut VecDeque<std::time::Instant>, now: std::time::Instant) -> bool {
    while times.front().is_some_and(|t| now.duration_since(*t) >= Duration::from_secs(60)) {
        times.pop_front();
    }
    if times.len() >= MAX_SMALL_PER_MINUTE {
        return false;
    }
    times.push_back(now);
    true
}

/// Counts an offer from a contact at `now`; `times` holds their offers of the last minute.
fn offer_flood(times: &mut VecDeque<std::time::Instant>, now: std::time::Instant) -> Flood {
    while times.front().is_some_and(|t| now.duration_since(*t) >= Duration::from_secs(60)) {
        times.pop_front();
    }
    if times.len() >= MAX_OFFERS_PER_MINUTE * 2 {
        return Flood::Ignore;
    }
    times.push_back(now);
    if times.len() > MAX_OFFERS_PER_MINUTE { Flood::TurnAway } else { Flood::Fine }
}

struct Request {
    name: String,
    /// They pair this PC as their own device (shown in My devices, not Direct).
    device: bool,
    answer: Option<oneshot::Sender<bool>>,
    /// Arrival order (questions are shown oldest first).
    at: u64,
}

/// How the person answered "… wants to connect".
enum Answer {
    Yes,
    No,
    /// Nobody answered (away, or the window closed the question).
    Silent,
}

/// Marks a contact as being dialled; dropped when the handshake ends, so the same contact is
/// never dialled twice at once (presence and the Connect button).
struct Dialing {
    node: Arc<Node>,
    id: String,
}

impl Drop for Dialing {
    fn drop(&mut self) {
        self.node.inner.lock().unwrap().connecting.remove(&self.id);
    }
}

/// How fast a transfer moves: started when bytes start flowing, sampled about twice a second;
/// the peak is the best rate over roughly one second.
#[derive(Default)]
struct Meter {
    started: Mutex<Option<std::time::Instant>>,
    ended: Mutex<Option<std::time::Instant>>,
    peak: AtomicU64,
}

impl Meter {
    /// Starts timing and samples `counter` until `stop` is cancelled.
    fn start(self: &Arc<Self>, counter: Arc<AtomicU64>, stop: CancellationToken, rt: &tokio::runtime::Handle) {
        {
            let mut s = self.started.lock().unwrap();
            if s.is_some() {
                return;
            }
            *s = Some(std::time::Instant::now());
        }
        let me = self.clone();
        rt.spawn(async move {
            let mut window: VecDeque<(std::time::Instant, u64)> = VecDeque::new();
            loop {
                tokio::select! {
                    _ = stop.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {}
                }
                let now = std::time::Instant::now();
                let bytes = counter.load(Ordering::Relaxed);
                window.push_back((now, bytes));
                while window.len() > 2 && now.duration_since(window[0].0) > Duration::from_millis(1100) {
                    window.pop_front();
                }
                if let Some(&(t0, b0)) = window.front() {
                    let dt = now.duration_since(t0).as_secs_f64();
                    if dt >= 0.9 {
                        let rate = (bytes.saturating_sub(b0) as f64 / dt) as u64;
                        me.peak.fetch_max(rate, Ordering::Relaxed);
                    }
                }
            }
            *me.ended.lock().unwrap() = Some(std::time::Instant::now());
        });
    }

    /// (duration ms, average bytes/s, peak bytes/s) for `bytes` moved.
    fn summary(&self, bytes: u64) -> (u64, u64, u64) {
        let Some(start) = *self.started.lock().unwrap() else { return (0, 0, 0) };
        let end = self.ended.lock().unwrap().unwrap_or_else(std::time::Instant::now);
        let ms = end.duration_since(start).as_millis() as u64;
        let avg = speed_of(bytes, ms);
        // A short transfer may end before one full sample: its average is its peak.
        (ms, avg, self.peak.load(Ordering::Relaxed).max(avg))
    }
}

/// Bytes per second for `bytes` moved in `ms`.
fn speed_of(bytes: u64, ms: u64) -> u64 {
    if ms == 0 { 0 } else { (bytes as u128 * 1000 / ms as u128) as u64 }
}

struct Outgoing {
    contact: String,
    name: String,
    files: Vec<(PathBuf, OfferFile)>,
    total: u64,
    sent: Arc<AtomicU64>,
    state: String,
    message: Option<String>,
    cancel: CancellationToken,
    /// Answers from the receiver for this transfer.
    events: mpsc::UnboundedSender<Msg>,
    /// A temporary ZIP made for "Compressed ZIP", deleted afterwards.
    temp: Option<PathBuf>,
    meter: Arc<Meter>,
    /// The receiver keeps what arrives: after a cut, it continues where it stopped.
    resume: bool,
    /// Each file's last-modified time when offered (ms): one changed since goes again from zero.
    mtimes: Vec<u64>,
    /// Bytes already there when this round started (the meter counts from here).
    base: u64,
    /// When it last moved or stopped (ms): a transfer waits 7 days for the other PC.
    since: u64,
}

/// What arrived of a transfer coming in, per file: written (maybe not synced to the disk yet).
#[derive(Default)]
struct Got {
    ranges: Vec<Ranges>,
    /// Per file: writes so far, and how many of them a saved note covers (synced first). A file
    /// stays to sync until a note that synced it is saved, so a newer snapshot saved before an
    /// older one (or a sync that failed) never leaves bytes in the note that aren't on the disk.
    writes: Vec<u64>,
    synced: Vec<u64>,
    any: bool,
    /// Counts snapshots, so an older one is never saved after a newer one.
    seq: u64,
}

impl Got {
    fn new(ranges: Vec<Ranges>) -> Self {
        let n = ranges.len();
        Got { ranges, writes: vec![0; n], synced: vec![0; n], ..Default::default() }
    }

    /// `n` bytes written at `at` in file `file`: returns how many of them are new.
    fn wrote(&mut self, file: usize, at: u64, n: u64) -> u64 {
        let added = self.ranges.get_mut(file).map_or(0, |r| r.insert(at, at + n));
        if let Some(w) = self.writes.get_mut(file) {
            *w += 1;
        }
        self.any = true;
        added
    }

    /// A snapshot: its number, what's written, and the files to sync before it may be saved
    /// (their place, and their write count now).
    fn snapshot(&mut self) -> (u64, Vec<Ranges>, Vec<(usize, u64)>) {
        let sync = self.writes.iter().zip(&self.synced).enumerate().filter(|(_, (w, s))| w > s).map(|(i, (w, _))| (i, *w)).collect();
        self.any = false;
        self.seq += 1;
        (self.seq, self.ranges.clone(), sync)
    }

    /// A note covering these writes was saved.
    fn saved(&mut self, sync: &[(usize, u64)]) {
        for &(i, w) in sync {
            if let Some(s) = self.synced.get_mut(i) {
                *s = (*s).max(w);
            }
        }
    }
}

/// Saving a receiver's note: the last snapshot saved, and whether the transfer ended (then its
/// note is gone for good and never saved again).
#[derive(Default)]
struct NoteState {
    saved: u64,
    ended: bool,
}

struct Incoming {
    contact: String,
    name: String,
    files: Vec<OfferFile>,
    total: u64,
    received: Arc<AtomicU64>,
    state: String,
    message: Option<String>,
    cancel: CancellationToken,
    /// Where each file ends up, and the ".pakorecv" file it's written to meanwhile.
    targets: Vec<PathBuf>,
    parts: Vec<PathBuf>,
    handles: Vec<Arc<std::fs::File>>,
    /// The folder or single file shown in the history.
    saved_to: Option<PathBuf>,
    progress: Arc<Notify>,
    meter: Arc<Meter>,
    /// Arrival order of the offer (questions are shown oldest first).
    at: u64,
    /// "All sent" came in: finishing has started (a second "all sent" is ignored).
    finishing: bool,
    /// Why this offer is asked about although the contact may send without asking.
    asked_because: Option<String>,
    /// Kept after a cut (both Pakos can continue it): see `resume.rs`.
    resume: bool,
    got: Arc<Mutex<Got>>,
    note: Arc<Mutex<NoteState>>,
    /// Sent again once already because it didn't match its hash.
    retried: Vec<bool>,
    /// Bytes already there when this round started (the meter counts from here).
    base: u64,
    /// When it last moved or stopped (ms).
    since: u64,
}

#[derive(Default)]
struct Inner {
    contacts: Vec<Contact>,
    history: Vec<HistoryItem>,
    /// Your tags for contacts, in your order (only on this PC).
    tags: Vec<String>,
    live: HashMap<String, Live>,
    /// Contacts we're waiting on for their first "yes".
    waiting: HashMap<String, ()>,
    /// Last problem reaching a contact.
    notes: HashMap<String, String>,
    requests: HashMap<String, Request>,
    outgoing: HashMap<String, Outgoing>,
    incoming: HashMap<String, Incoming>,
    my_name: String,
    /// Tests: addresses to dial instead of looking IDs up.
    hints: HashMap<String, EndpointAddr>,
    /// Contacts being dialled right now.
    connecting: HashSet<String>,
    /// Contacts waiting for their turn in the round of calls.
    queued: HashSet<String>,
    /// When each contact not connected is called next (fewer calls to those who don't answer).
    tries: Tries,
    /// The conversation open in the window (kept connected).
    open_chat: Option<String>,
    /// People you declined or removed, turned away without asking until a date.
    blocked: Vec<Blocked>,
    /// Counter for arrival order.
    seq: u64,
    /// When each contact's offers of the last minute came in.
    offer_times: HashMap<String, VecDeque<std::time::Instant>>,
    /// When each contact's name changes and "my device" messages of the last minute came in.
    small_times: HashMap<String, VecDeque<std::time::Instant>>,
    /// When the last "… wants to connect" questions from people who aren't contacts came in.
    request_times: VecDeque<std::time::Instant>,
    /// My devices: lists, orders, their history.
    dev: devices::DevInner,
    /// Direct relay through your devices.
    relay: relay::RelayInner,
}

impl Inner {
    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }
}

pub struct Node {
    pub ep: Endpoint,
    store: Store,
    data_dir: PathBuf,
    /// Pako's download folder, for "<download folder>\From <name>".
    downloads: Box<dyn Fn() -> PathBuf + Send + Sync>,
    inner: Mutex<Inner>,
    changed: Changed,
    notify: Notify2,
    settle: Settle,
    pub stop: CancellationToken,
    /// No relay answers (no internet): only contacts on the same network can be reached.
    offline: AtomicBool,
    /// Where background work runs (the window's commands aren't async).
    rt: tokio::runtime::Handle,
    /// My devices: where their orders act.
    dev: devices::DevNode,
    /// Tests: behave like a Pako from before picking up transfers (never offers or keeps it).
    old_pako: AtomicBool,
    /// Tests: a pause (ms) after each block sent, to cut a transfer half-way.
    pace: AtomicU64,
    /// Starts the round of calls now (the internet came back).
    round_now: Notify,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactView {
    #[serde(flatten)]
    pub contact: Contact,
    pub online: bool,
    pub path: Option<String>,
    /// Waiting for them to accept the first connection.
    pub waiting: bool,
    pub code: String,
    pub note: Option<String>,
    pub folder: String,
    /// Not connected because nothing was happening (Pako closes quiet connections when many are
    /// open): they were there a moment ago, and are called again when you open them.
    pub idle: bool,
    /// How Pako checks them, Automatic resolved ("often" for My devices and people who can
    /// send without asking).
    pub check_as: pace::Check,
    /// "Not checked lately": not called in the background (gone quiet, or only when you ask)
    /// and not connected. Check now, opening them or sending calls them.
    pub unchecked: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestView {
    pub peer: String,
    pub name: String,
    pub code: String,
    /// One of your contacts already has this name (a different Pako: maybe someone pretending).
    pub known_as: Option<String>,
    /// A device pairing (My devices shows it, Direct doesn't).
    #[serde(skip)]
    pub device: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferView {
    pub id: String,
    pub contact: String,
    pub contact_name: String,
    pub name: String,
    pub files: u32,
    pub size: u64,
    pub sample: Vec<String>,
    pub folder: String,
    /// Why you're asked although this contact may send without asking (it's big for the drive).
    pub asked_because: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferView {
    pub id: String,
    pub contact: String,
    pub outgoing: bool,
    pub name: String,
    pub files: u32,
    pub size: u64,
    pub done: u64,
    pub state: String,
    pub path: Option<String>,
    pub message: Option<String>,
    /// A send passed on for another of your PCs: its name ("from Laptop").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    /// The first leg of a relay: who it's for ("for Rado").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_to: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub my_id: String,
    pub my_name: String,
    /// Your tags, in your order.
    pub tags: Vec<String>,
    pub contacts: Vec<ContactView>,
    pub requests: Vec<RequestView>,
    pub offers: Vec<OfferView>,
    pub transfers: Vec<TransferView>,
    pub history: Vec<HistoryItem>,
    /// The internet answers (a relay is connected). Without it only contacts on the same
    /// network can be reached.
    pub internet: bool,
    /// Direct relays through your devices, both ways (see `relay.rs`).
    pub relays: Vec<relay::RelayView>,
}

impl Node {
    pub fn new(
        ep: Endpoint,
        data_dir: PathBuf,
        my_name: String,
        downloads: Box<dyn Fn() -> PathBuf + Send + Sync>,
        changed: Changed,
        notify: Notify2,
        settle: Settle,
    ) -> Arc<Self> {
        let store = Store::new(data_dir.clone());
        let mut contacts = store.load_contacts();
        // Transfers cut short that both sides keep (Pako closed, the PC turned off): they
        // continue when the contact connects again. Only for contacts still in the list.
        let (ins, outs) = resume::load(&data_dir);
        let history = store.load_history();
        // Contacts from before "Check this contact": their last activity is their last
        // transfer, else the date they were added.
        if store::seed_activity(&mut contacts, &history, now_ms()) {
            store.save_contacts(&contacts);
        }
        // (A note of a transfer that ended is a leftover: its end was recorded.)
        let known = |c: &str, id: &str| contacts.iter().any(|x| x.id == c) && !history.iter().any(|h| h.id == id);
        let (ins, gone): (Vec<InNote>, Vec<InNote>) = ins.into_iter().partition(|n| known(&n.contact, &n.id));
        for n in gone {
            resume::remove_in(&data_dir, &n.id);
        }
        let (outs, gone): (Vec<OutNote>, Vec<OutNote>) = outs.into_iter().partition(|n| known(&n.contact, &n.id));
        for n in gone {
            resume::remove_out(&data_dir, &n.id);
        }
        // Other leftovers of transfers cut short (Pako closed or crashed): unfinished pieces
        // and temporary ZIPs go (the sender sends again).
        let kept_parts: HashSet<PathBuf> = ins.iter().flat_map(|n| n.parts.iter().cloned()).collect();
        for p in store.load_parts() {
            let p = PathBuf::from(p);
            if !kept_parts.contains(&p) && is_own_leftover(&p) {
                let _ = std::fs::remove_file(&p);
            }
        }
        let kept_zips: HashSet<PathBuf> = outs.iter().filter_map(|n| n.temp.clone()).collect();
        if let Ok(rd) = std::fs::read_dir(data_dir.join("direct_tmp")) {
            for e in rd.flatten() {
                if !kept_zips.contains(&e.path()) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let tags = store.load_tags(&contacts);
        let blocked = store.load_blocked(now_ms());
        let mut inner = Inner { contacts, history, tags, my_name, blocked, ..Default::default() };
        inner.dev.log = store.load_device_log();
        inner.dev.sent = store.load_sent();
        for n in ins {
            let received = n.have.iter().map(|r| r.covered()).sum();
            let at = inner.next_seq();
            inner.incoming.insert(
                n.id.clone(),
                Incoming {
                    contact: n.contact,
                    name: n.name,
                    total: n.files.iter().map(|f| f.size).sum(),
                    received: Arc::new(AtomicU64::new(received)),
                    state: "waiting".into(),
                    message: None,
                    cancel: CancellationToken::new(),
                    targets: n.targets,
                    parts: n.parts,
                    handles: Vec::new(),
                    saved_to: n.saved_to,
                    progress: Arc::new(Notify::new()),
                    meter: Arc::new(Meter::default()),
                    at,
                    finishing: false,
                    asked_because: None,
                    resume: true,
                    got: Arc::new(Mutex::new(Got::new(n.have))),
                    note: Arc::default(),
                    retried: n.retried,
                    base: received,
                    since: n.since,
                    files: n.files,
                },
            );
        }
        for n in outs {
            let (etx, _) = mpsc::unbounded_channel();
            inner.outgoing.insert(
                n.id.clone(),
                Outgoing {
                    contact: n.contact,
                    name: n.name,
                    total: n.files.iter().map(|f| f.file.size).sum(),
                    mtimes: n.files.iter().map(|f| f.mtime).collect(),
                    files: n.files.into_iter().map(|f| (f.path, f.file)).collect(),
                    sent: Arc::new(AtomicU64::new(n.done)),
                    state: "waiting".into(),
                    message: None,
                    cancel: CancellationToken::new(),
                    events: etx,
                    temp: n.temp,
                    meter: Arc::new(Meter::default()),
                    resume: true,
                    base: n.done,
                    since: n.since,
                },
            );
        }
        let list: Vec<String> = inner.incoming.values().flat_map(|t| t.parts.iter().map(|p| p.display().to_string())).collect();
        store.save_parts(&list);
        let node = Arc::new(Self {
            ep,
            store,
            data_dir,
            downloads,
            inner: Mutex::new(inner),
            changed,
            notify,
            settle,
            stop: CancellationToken::new(),
            offline: AtomicBool::new(false),
            rt: tokio::runtime::Handle::current(),
            dev: devices::DevNode::default(),
            old_pako: AtomicBool::new(false),
            pace: AtomicU64::new(0),
            round_now: Notify::new(),
        });
        // Relays cut by Pako closing go on; the PC that asked hears how they go.
        node.relay_load();
        node.rt.spawn(node.clone().relay_loop());
        // Waited more than 7 days (Pako wasn't running meanwhile): cleared now.
        node.expire(now_ms());
        node
    }

    /// This Pako offers and keeps transfers to continue after a cut (tests turn it off to
    /// play an older Pako).
    fn resume_on(&self) -> bool {
        !self.old_pako.load(Ordering::Relaxed)
    }

    pub fn my_id(&self) -> String {
        self.ep.id().to_string()
    }

    fn changed(&self) {
        (self.changed)();
    }

    fn save_contacts(&self, inner: &Inner) {
        self.store.save_contacts(&inner.contacts);
    }

    /// Remembers the unfinished pieces of running transfers (cleaned up at the next start if
    /// Pako closes before they finish).
    fn save_parts(&self, inner: &Inner) {
        let list: Vec<String> = inner.incoming.values().flat_map(|t| t.parts.iter().map(|p| p.display().to_string())).collect();
        self.store.save_parts(&list);
    }

    /// Turns this person away without asking you for a week (you declined or removed them).
    fn block(&self, inner: &mut Inner, id: &str) {
        let now = now_ms();
        inner.blocked.retain(|b| b.id != id && b.until > now);
        inner.blocked.push(Blocked { id: id.to_string(), until: now + BLOCK_FOR_MS });
        self.store.save_blocked(&inner.blocked);
    }

    fn unblock(&self, inner: &mut Inner, id: &str) {
        if inner.blocked.iter().any(|b| b.id == id) {
            inner.blocked.retain(|b| b.id != id);
            self.store.save_blocked(&inner.blocked);
        }
    }

    /// Where a contact's files go by default.
    fn folder_for(&self, c: &Contact) -> PathBuf {
        match &c.save_folder {
            Some(f) if !f.is_empty() => PathBuf::from(f),
            _ => (self.downloads)().join(format!("From {}", proto::safe_part(&c.name).unwrap_or_else(|| "a contact".into()))),
        }
    }

    // ---------- the window's view ----------

    /// Direct's view: people only (your devices are in My devices).
    pub fn state(&self) -> State {
        let mut s = self.full_state();
        let devices: HashSet<String> = s.contacts.iter().filter(|c| c.contact.device).map(|c| c.contact.id.clone()).collect();
        s.contacts.retain(|c| !c.contact.device);
        s.requests.retain(|r| !r.device);
        s.transfers.retain(|t| !devices.contains(&t.contact));
        s.history.retain(|h| !devices.contains(&h.contact));
        s
    }

    /// Everything, devices too.
    fn full_state(&self) -> State {
        let me = self.my_id();
        let inner = self.inner.lock().unwrap();
        let now = now_ms();
        let contacts = inner
            .contacts
            .iter()
            .map(|c| {
                let live = inner.live.get(&c.id);
                ContactView {
                    contact: c.clone(),
                    online: live.is_some(),
                    path: live.and_then(|l| path_of(&l.conn)),
                    waiting: inner.waiting.contains_key(&c.id),
                    code: proto::safety_code(&me, &c.id),
                    note: inner.notes.get(&c.id).cloned(),
                    folder: self.folder_for(c).display().to_string(),
                    idle: live.is_none() && inner.tries.is_idle(&c.id),
                    check_as: pace::check_of(c, now),
                    unchecked: live.is_none() && !c.declined && !inner.waiting.contains_key(&c.id) && !inner.connecting.contains(&c.id) && !in_round(&inner, c, now),
                }
            })
            .collect();
        // Oldest question first.
        let mut asked: Vec<(&String, &Request)> = inner.requests.iter().collect();
        asked.sort_by_key(|(_, r)| r.at);
        let fold = |s: &str| s.trim().to_lowercase();
        let requests = asked
            .into_iter()
            .map(|(peer, r)| RequestView {
                peer: peer.clone(),
                name: r.name.clone(),
                code: proto::safety_code(&me, peer),
                known_as: inner
                    .contacts
                    .iter()
                    .find(|c| fold(&c.name) == fold(&r.name) || c.their_name.as_deref().is_some_and(|t| fold(t) == fold(&r.name)))
                    .map(|c| c.name.clone()),
                device: r.device,
            })
            .collect();
        let name_of = |id: &str| inner.contacts.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default();
        let mut waiting: Vec<(&String, &Incoming)> = inner.incoming.iter().filter(|(_, t)| t.state == "asking").collect();
        waiting.sort_by_key(|(_, t)| t.at);
        let offers = waiting
            .into_iter()
            .map(|(id, t)| OfferView {
                id: id.clone(),
                contact: t.contact.clone(),
                contact_name: name_of(&t.contact),
                name: t.name.clone(),
                files: t.files.len() as u32,
                size: t.total,
                // Shown as sent, minus invisible characters that could disguise a name.
                sample: t.files.iter().take(5).map(|f| f.path.chars().filter(|c| !proto::is_hidden(*c) && !c.is_control()).collect()).collect(),
                folder: inner
                    .contacts
                    .iter()
                    .find(|c| c.id == t.contact)
                    .map(|c| self.folder_for(c).display().to_string())
                    .unwrap_or_default(),
                asked_because: t.asked_because.clone(),
            })
            .collect();
        let path_for = |id: &str| inner.live.get(id).and_then(|l| path_of(&l.conn));
        let mut transfers: Vec<TransferView> = inner
            .outgoing
            .iter()
            .map(|(id, t)| TransferView {
                id: id.clone(),
                contact: t.contact.clone(),
                outgoing: true,
                name: t.name.clone(),
                files: t.files.len() as u32,
                size: t.total,
                done: t.sent.load(Ordering::Relaxed),
                state: t.state.clone(),
                path: path_for(&t.contact),
                message: t.message.clone(),
                via: self.via_of(&inner, id),
                relay_to: Self::relay_to_of(&inner, id),
            })
            .collect();
        transfers.extend(inner.incoming.iter().filter(|(_, t)| t.state != "asking").map(|(id, t)| TransferView {
            id: id.clone(),
            contact: t.contact.clone(),
            outgoing: false,
            name: t.name.clone(),
            files: t.files.len() as u32,
            size: t.total,
            done: t.received.load(Ordering::Relaxed),
            state: t.state.clone(),
            path: path_for(&t.contact),
            message: t.message.clone(),
            via: None,
            relay_to: Self::relay_to_of(&inner, id),
        }));
        State {
            my_id: me,
            my_name: inner.my_name.clone(),
            tags: inner.tags.clone(),
            contacts,
            requests,
            offers,
            transfers,
            history: inner.history.clone(),
            internet: !self.offline.load(Ordering::Relaxed),
            relays: Self::relay_views(&inner),
        }
    }

    /// Files are moving (a transfer waiting for the other PC to come back isn't).
    pub fn busy(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.outgoing.values().any(|t| t.state != "waiting") || inner.incoming.values().any(|t| t.state != "asking" && t.state != "waiting")
    }

    // ---------- contacts ----------

    /// Your name is set (what contacts see; asked before adding or accepting anyone).
    pub fn has_name(&self) -> bool {
        !self.inner.lock().unwrap().my_name.trim().is_empty()
    }

    pub fn set_my_name(&self, name: &str) {
        let name: String = name.trim().chars().take(60).collect();
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.my_name == name {
                return;
            }
            inner.my_name = name.clone();
            // Connected contacts see the new name at once (the others at the next connection).
            for live in inner.live.values() {
                let _ = live.tx.send(Msg::Name { name: name.clone() });
            }
        }
        // Your devices show it in their "From" menu.
        self.tell_devices();
        self.changed();
    }

    pub fn add_contact(self: &Arc<Self>, id: &str, name: &str) -> Result<(), String> {
        let id = id.trim();
        let parsed: EndpointId = id.parse().map_err(|_| "That isn't a Pako ID. Ask your contact to copy theirs again.".to_string())?;
        let id = parsed.to_string();
        if id == self.my_id() {
            return Err("That's your own Pako ID.".into());
        }
        let name = clean_name(name).ok_or("Give this contact a name.")?;
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.contacts.iter().any(|c| c.id == id) {
                return Err("This contact is already in your list.".into());
            }
            // You added them yourself: they may connect again.
            self.unblock(&mut inner, &id);
            inner.contacts.push(Contact {
                id: id.clone(),
                name,
                trusted: false,
                auto_accept: false,
                save_folder: None,
                last_seen: None,
                added: now_ms(),
                tags: Vec::new(),
                favourite: false,
                their_name: None,
                declined: false,
                ..Default::default()
            });
            self.save_contacts(&inner);
        }
        self.tell_devices();
        self.changed();
        self.reach(&id);
        Ok(())
    }

    pub fn edit_contact(&self, id: &str, name: Option<String>, auto_accept: Option<bool>, folder: Option<Option<String>>) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        let c = inner.contacts.iter_mut().find(|c| c.id == id).ok_or("Contact not found.")?;
        if let Some(n) = name {
            c.name = clean_name(&n).ok_or("Give this contact a name.")?;
        }
        if let Some(a) = auto_accept {
            c.auto_accept = a;
        }
        if let Some(f) = folder {
            c.save_folder = f.filter(|s| !s.trim().is_empty());
        }
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
        Ok(())
    }

    // ---------- tags and favourites (only on this PC) ----------

    fn save_tags(&self, inner: &Inner) {
        self.store.save_tags(&inner.tags);
    }

    pub fn set_favourite(&self, id: &str, on: bool) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        let c = inner.contacts.iter_mut().find(|c| c.id == id).ok_or("Contact not found.")?;
        c.favourite = on;
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
        Ok(())
    }

    /// Mute: nothing from them pops up, chimes or goes in the bell (their offers still wait in
    /// their conversation). Not for My devices.
    pub fn set_muted(&self, id: &str, on: bool) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        let c = inner.contacts.iter_mut().find(|c| c.id == id).ok_or("Contact not found.")?;
        if on && c.device {
            return Err("Your own devices can't be muted.".into());
        }
        c.muted = on;
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
        Ok(())
    }

    /// "Check this contact": Often for a while (`span`), Automatic or Only when I ask. Choosing
    /// counts as something happening with them (Automatic starts its 30 days over); Often is
    /// called at the next round, and goes back to the choice it had before once its time is up
    /// (When I ask stays When I ask; Often picked again while it runs keeps the earlier choice).
    pub fn set_check(&self, id: &str, check: pace::Check, span: Option<pace::OftenFor>) -> Result<(), String> {
        let now = now_ms();
        let until = match (check, span) {
            (pace::Check::Often, Some(span)) => Some(pace::often_until(span, now, &chrono::Local)),
            (pace::Check::Often, None) => return Err("Pick how long.".into()),
            _ => None,
        };
        let mut inner = self.inner.lock().unwrap();
        let c = inner.contacts.iter_mut().find(|c| c.id == id).ok_or("Contact not found.")?;
        c.check_before = match check {
            // The choice to go back to: the one before Often (Often picked again while it runs
            // keeps it). Automatic is the default, so it isn't kept.
            pace::Check::Often => {
                let before = if c.check == pace::Check::Often { store::back_from_often(c.check_before) } else { c.check };
                Some(before).filter(|b| *b == pace::Check::Ask)
            }
            _ => None,
        };
        c.check = check;
        c.check_until = until;
        c.last_activity = Some(now);
        inner.tries.reset(id);
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
        self.round_now.notify_one();
        Ok(())
    }

    /// Something happened with them (a transfer, you sending, them calling us, Check now):
    /// Automatic calls them in the background again for the next 30 days. Not saved here.
    fn touch(inner: &mut Inner, id: &str) {
        if let Some(c) = inner.contacts.iter_mut().find(|c| c.id == id) {
            c.last_activity = Some(now_ms());
        }
    }

    /// A notice about a contact (pop-up, chime and bell line), unless they're muted. (Never
    /// silent for one of My devices, also one muted before it was paired.)
    fn tell(&self, peer: &str, kind: &str, title: &str, body: &str, go: Go) {
        if self.inner.lock().unwrap().contacts.iter().any(|c| c.id == peer && c.muted && !c.device) {
            return;
        }
        (self.notify)(kind, title, body, go);
    }

    pub fn set_tags(&self, id: &str, wanted: Vec<String>) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        let Inner { tags, contacts, .. } = &mut *inner;
        let c = contacts.iter_mut().find(|c| c.id == id).ok_or("Contact not found.")?;
        store::set_contact_tags(tags, c, &wanted)?;
        self.save_tags(&inner);
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
        Ok(())
    }

    pub fn add_tag(&self, name: &str) -> Result<String, String> {
        let mut inner = self.inner.lock().unwrap();
        let t = store::add_tag(&mut inner.tags, name)?;
        self.save_tags(&inner);
        drop(inner);
        self.changed();
        Ok(t)
    }

    pub fn rename_tag(&self, old: &str, new: &str) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        let Inner { tags, contacts, .. } = &mut *inner;
        store::rename_tag(tags, contacts, old, new)?;
        self.save_tags(&inner);
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
        Ok(())
    }

    pub fn delete_tag(&self, name: &str) {
        let mut inner = self.inner.lock().unwrap();
        let Inner { tags, contacts, .. } = &mut *inner;
        store::delete_tag(tags, contacts, name);
        self.save_tags(&inner);
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
    }

    /// Removes a contact: what's moving with them stops, the trust ends, the connection closes,
    /// and for a week they're turned away without asking you.
    pub fn remove_contact(&self, id: &str) {
        let (outs, ins): (Vec<String>, Vec<String>) = {
            let inner = self.inner.lock().unwrap();
            (
                inner.outgoing.iter().filter(|(_, t)| t.contact == id).map(|(k, _)| k.clone()).collect(),
                inner.incoming.iter().filter(|(_, t)| t.contact == id).map(|(k, _)| k.clone()).collect(),
            )
        };
        for t in outs {
            self.end_out(&t, "cancelled", Some("You removed this contact.".into()), true);
        }
        for t in ins {
            self.end_in(&t, "cancelled", Some("You removed this contact.".into()), true);
        }
        let live = {
            let mut inner = self.inner.lock().unwrap();
            inner.contacts.retain(|c| c.id != id);
            inner.waiting.remove(id);
            inner.notes.remove(id);
            inner.offer_times.remove(id);
            inner.small_times.remove(id);
            self.save_contacts(&inner);
            self.block(&mut inner, id);
            inner.live.remove(id)
        };
        self.device_gone(id);
        // One of your devices removed: what it asked this PC to pass on stops.
        self.relay_sweep();
        // Relays waiting to be passed on to them end (the copies go).
        let relays: Vec<String> = self.inner.lock().unwrap().relay.list.iter().filter(|r| !r.mine && r.to == id && r.is_open()).map(|r| r.id.clone()).collect();
        for r in relays {
            self.relay_end(&r, "cancelled", Some("That contact was removed on the PC passing it on.".into()));
        }
        self.tell_devices();
        if let Some(l) = live {
            let _ = l.tx.send(Msg::Refused);
            let conn = l.conn.clone();
            self.rt.spawn(async move {
                tokio::time::sleep(Duration::from_millis(300)).await;
                conn.close(2u32.into(), b"removed");
            });
        }
        self.changed();
    }

    pub fn clear_history(&self, contact: Option<&str>) {
        let mut inner = self.inner.lock().unwrap();
        match contact {
            Some(c) => inner.history.retain(|h| h.contact != c),
            None => inner.history.clear(),
        }
        let mut h = std::mem::take(&mut inner.history);
        self.store.save_history(&mut h);
        inner.history = h;
        drop(inner);
        self.changed();
    }

    pub fn history_item(&self, id: &str) -> Option<HistoryItem> {
        self.inner.lock().unwrap().history.iter().find(|h| h.id == id).cloned()
    }

    fn record(&self, item: HistoryItem) {
        if item.result == "done" {
            // Your own devices aren't among the Dashboard's people (Direct doesn't list them:
            // they showed as "Removed contact"). Their bytes still count.
            let device = self.inner.lock().unwrap().contacts.iter().any(|c| c.id == item.contact && c.device);
            let e = crate::numbers::direct_event(
                &item.id,
                item.date as i64,
                item.outgoing,
                &item.name,
                item.size,
                item.files,
                item.path.as_deref(),
                if device { "" } else { &item.contact },
                item.duration_ms,
                item.avg_speed,
                item.peak_speed,
            );
            std::thread::spawn(move || crate::numbers::record(e));
        }
        let mut inner = self.inner.lock().unwrap();
        // A transfer either way (also one declined or cut): something happened with them.
        Self::touch(&mut inner, &item.contact);
        self.save_contacts(&inner);
        inner.history.push(item);
        let mut h = std::mem::take(&mut inner.history);
        self.store.save_history(&mut h);
        inner.history = h;
    }

    // ---------- connecting ----------

    /// Tests: dial this address for that ID.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn add_hint(&self, addr: EndpointAddr) {
        self.inner.lock().unwrap().hints.insert(addr.id.to_string(), addr);
    }

    /// Tries to reach a contact now, in the background (adding them, Connect, Check now): also
    /// someone who declined before (they're asked again). Never waits for the round of calls,
    /// their wait between calls starts over, and it counts as something happening with them
    /// (Automatic calls them in the background again for 30 days).
    pub fn reach(self: &Arc<Self>, id: &str) {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.tries.reset(id);
            if inner.contacts.iter().any(|c| c.id == id) {
                inner.contacts.iter_mut().filter(|c| c.id == id).for_each(|c| c.declined = false);
                Self::touch(&mut inner, id);
                self.save_contacts(&inner);
            }
        }
        self.dial(id, true);
    }

    /// Calls now, in the background. `counts`: a failure makes their next call in the round
    /// wait longer (not for the calls a waiting transfer makes every 30 s).
    fn dial(self: &Arc<Self>, id: &str, counts: bool) {
        let me = self.clone();
        let id = id.to_string();
        self.rt.spawn(async move {
            let _ = me.connect_as(&id, counts).await;
        });
    }

    /// Everyone not connected gets their turn in the round at once (the internet came back):
    /// still a few calls at a time, see `round_worker`. Those who declined stay out (only the
    /// Connect button asks them again).
    pub fn reach_all(self: &Arc<Self>) {
        self.inner.lock().unwrap().tries.reset_all();
        self.round_now.notify_one();
    }

    /// The conversation shown in the window (`None`: none): kept connected, and called now if
    /// it isn't (it may have closed for being quiet).
    pub fn set_open(self: &Arc<Self>, id: Option<&str>) {
        let call = {
            let mut inner = self.inner.lock().unwrap();
            // Only one of the contacts (anything else the window sends is ignored).
            let id = id.filter(|id| inner.contacts.iter().any(|c| c.id == *id));
            inner.open_chat = id.map(str::to_string);
            let call = id.filter(|id| !inner.live.contains_key(*id) && inner.contacts.iter().any(|c| c.id == *id && !c.declined)).map(str::to_string);
            if let Some(id) = &call {
                inner.tries.reset(id);
            }
            call
        };
        if let Some(id) = call {
            self.dial(&id, true);
        }
    }

    async fn connect_as(self: &Arc<Self>, id: &str, counts: bool) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().unwrap();
            // Already connected, or being dialled right now.
            if inner.live.contains_key(id) || !inner.connecting.insert(id.to_string()) {
                return Ok(());
            }
        }
        let dialing = Dialing { node: self.clone(), id: id.to_string() };
        let peer: EndpointId = id.parse().map_err(|_| "Bad Pako ID".to_string())?;
        let addr = self.inner.lock().unwrap().hints.get(id).cloned().unwrap_or_else(|| EndpointAddr::new(peer));
        let failed = |me: &Self| {
            if counts {
                let offline = me.is_offline();
                let mut inner = me.inner.lock().unwrap();
                // A transfer waits for them: tried every round (3 min) as before, never pushed
                // out to an hour, also after its first hour of calls every 30 s.
                // Often: tried every round, never less often.
                let uncounted = offline || waits_for(&inner, id) || inner.contacts.iter().any(|c| c.id == id && pace::often(c, now_ms()));
                inner.tries.failed(id, std::time::Instant::now(), uncounted, super::PRESENCE_EVERY);
            }
        };
        let conn = match tokio::time::timeout(CONNECT_TIMEOUT, self.ep.connect(addr, ALPN)).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                failed(self);
                self.note(id, Some(if self.is_offline() { NO_INTERNET.into() } else { format!("Not reachable right now ({e})") }));
                return Err(e.to_string());
            }
            Err(_) => {
                failed(self);
                self.note(id, Some(if self.is_offline() { NO_INTERNET.into() } else { "Not reachable right now".into() }));
                return Err("timed out".into());
            }
        };
        let me = self.clone();
        self.rt.spawn(async move { me.run(conn, Some(dialing)).await });
        Ok(())
    }

    fn note(&self, id: &str, text: Option<String>) {
        let mut inner = self.inner.lock().unwrap();
        match text {
            Some(t) => inner.notes.insert(id.to_string(), t),
            None => inner.notes.remove(id),
        };
        drop(inner);
        self.changed();
    }

    /// Accepts incoming connections until stopped.
    pub async fn accept_loop(self: Arc<Self>) {
        loop {
            let incoming = tokio::select! {
                _ = self.stop.cancelled() => return,
                i = self.ep.accept() => i,
            };
            let Some(incoming) = incoming else { return };
            let me = self.clone();
            tokio::spawn(async move {
                if let Ok(Ok(conn)) = tokio::time::timeout(CONNECT_TIMEOUT, std::future::IntoFuture::into_future(incoming)).await {
                    me.run(conn, None).await;
                }
            });
        }
    }

    fn is_offline(&self) -> bool {
        self.offline.load(Ordering::Relaxed)
    }

    /// Follows whether the internet answers (a relay connected). Without it, contacts that
    /// aren't on the same network show "Not reachable without internet"; when it's back, every
    /// contact gets their turn again at once (a few calls at a time). `settle`: how long a relay
    /// may stay away before that counts (a network change reconnects it within seconds).
    pub async fn internet_loop(self: Arc<Self>, settle: Duration) {
        let mut status = self.ep.home_relay_status();
        loop {
            let mut up = status.get().iter().any(|r| r.is_connected());
            if !up {
                tokio::select! {
                    _ = self.stop.cancelled() => return,
                    _ = tokio::time::sleep(settle) => {}
                }
                up = status.get().iter().any(|r| r.is_connected());
            }
            if self.offline.swap(!up, Ordering::Relaxed) == up {
                self.changed();
                if up {
                    self.reach_all();
                }
            }
            tokio::select! {
                _ = self.stop.cancelled() => return,
                r = status.updated() => if r.is_err() { return },
            }
        }
    }

    /// Follows the Pakos found on the same network: a contact that shows up is connected at
    /// once (its dot turns blue), without waiting for the next round of `presence_loop`.
    /// Anyone else found there is left alone.
    pub async fn local_loop(self: Arc<Self>, mdns: iroh_mdns_address_lookup::MdnsAddressLookup) {
        use futures_util::StreamExt;
        let mut events = mdns.subscribe().await;
        loop {
            let ev = tokio::select! {
                _ = self.stop.cancelled() => return,
                e = events.next() => e,
            };
            match ev {
                Some(iroh_mdns_address_lookup::DiscoveryEvent::Discovered { endpoint_info, .. }) => {
                    let id = endpoint_info.endpoint_id.to_string();
                    // (Only when they're new on the network or their addresses changed: cheap,
                    // however many contacts.) Seen there: their wait between calls starts over.
                    let known = {
                        let mut inner = self.inner.lock().unwrap();
                        // (Not someone only called when you ask, or gone quiet: that would be a
                        // call in the background too.)
                        let now = now_ms();
                        let known = inner.contacts.iter().any(|c| c.id == id && in_round(&inner, c, now)) && !inner.live.contains_key(&id);
                        if known {
                            inner.tries.reset(&id);
                        }
                        known
                    };
                    if known {
                        self.dial(&id, true);
                    }
                }
                Some(_) => {}
                None => return,
            }
        }
    }

    /// Every 30 seconds: calls the contacts whose turn it is (every few minutes, less often for
    /// those who don't answer: see `pace.rs`), a few at a time; the contacts a transfer is
    /// waiting for at once, in its first hour; closes quiet connections when many are open.
    /// Transfers that waited 7 days are cleared.
    pub async fn presence_loop(self: Arc<Self>) {
        // Two lines: My devices, favourites and people used lately go before the others, also
        // when their turn comes while a long round is still being called.
        let (first, first_calls) = mpsc::unbounded_channel::<String>();
        let (rest, rest_calls) = mpsc::unbounded_channel::<String>();
        tokio::spawn(self.clone().round_worker(first_calls, rest_calls));
        loop {
            self.settle_checks();
            self.round(&first, &rest);
            self.reach_waiting();
            self.reach_open();
            self.close_idle();
            self.expire(now_ms());
            tokio::select! {
                _ = self.stop.cancelled() => return,
                _ = self.round_now.notified() => {}
                _ = tokio::time::sleep(RETRY_WAITING) => {}
            }
        }
    }

    /// Often whose time is up goes back to the choice it had before (saved, and the window told).
    fn settle_checks(&self) {
        let mut inner = self.inner.lock().unwrap();
        if !store::settle_checks(&mut inner.contacts, now_ms()) {
            return;
        }
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
    }

    /// Queues the contacts whose turn it is: My devices and Often first, then favourites and
    /// people sent to lately, then the most recently seen. Not those only called when you ask,
    /// nor those gone quiet (see `pace::in_round`).
    fn round(&self, first: &mpsc::UnboundedSender<String>, rest: &mpsc::UnboundedSender<String>) {
        let now = std::time::Instant::now();
        let ids: Vec<(String, bool)> = {
            let mut inner = self.inner.lock().unwrap();
            let ids = {
                let ms = now_ms();
                let recent = recent_use(&inner.history, ms);
                let mut due: Vec<&Contact> = inner
                    .contacts
                    .iter()
                    .filter(|c| in_round(&inner, c, ms) && !inner.live.contains_key(&c.id) && !inner.connecting.contains(&c.id) && !inner.queued.contains(&c.id) && inner.tries.due(&c.id, now))
                    .collect();
                due.sort_by_key(|c| (!pace::often(c, ms), !wanted(c, &recent, ms), std::cmp::Reverse(c.last_seen.unwrap_or(0))));
                due.into_iter().map(|c| (c.id.clone(), wanted(c, &recent, ms))).collect::<Vec<_>>()
            };
            inner.queued.extend(ids.iter().map(|(id, _)| id.clone()));
            ids
        };
        for (id, wanted) in ids {
            let _ = if wanted { first.send(id) } else { rest.send(id) };
        }
    }

    /// Makes the round's calls: at most `DIALS_AT_ONCE` at a time, a moment apart.
    async fn round_worker(self: Arc<Self>, mut first: mpsc::UnboundedReceiver<String>, mut rest: mpsc::UnboundedReceiver<String>) {
        let slots = Arc::new(tokio::sync::Semaphore::new(pace::DIALS_AT_ONCE));
        loop {
            // A free slot first, then who's next (so someone wanted who comes meanwhile goes first).
            let slot = tokio::select! {
                _ = self.stop.cancelled() => return,
                s = slots.clone().acquire_owned() => match s { Ok(s) => s, Err(_) => return },
            };
            let id = tokio::select! {
                biased;
                _ = self.stop.cancelled() => return,
                Some(id) = first.recv() => id,
                Some(id) = rest.recv() => id,
                else => return,
            };
            let go = {
                let mut inner = self.inner.lock().unwrap();
                inner.queued.remove(&id);
                // Still a contact, nobody reached them meanwhile, and room for one more open
                // connection (else they keep their turn for a later round).
                let flying = pace::DIALS_AT_ONCE - slots.available_permits();
                let ms = now_ms();
                inner.contacts.iter().find(|c| c.id == id && in_round(&inner, c, ms)).is_some_and(|c| pace::room_for(inner.live.len() + flying, wanted(c, &recent_use(&inner.history, ms), ms)))
                    && !inner.live.contains_key(&id)
                    && !inner.connecting.contains(&id)
            };
            if !go {
                continue;
            }
            let me = self.clone();
            tokio::spawn(async move {
                let _ = me.connect_as(&id, true).await;
                drop(slot);
            });
            tokio::select! {
                _ = self.stop.cancelled() => return,
                _ = tokio::time::sleep(pace::DIAL_GAP) => {}
            }
        }
    }

    /// Closes quiet connections when many are open (see `pace::close_idle`): never one with a
    /// transfer, the open conversation or one of My devices; Often, favourites and the like only
    /// past the cap.
    fn close_idle(&self) {
        let now = std::time::Instant::now();
        let ms = now_ms();
        let mut inner = self.inner.lock().unwrap();
        let shut: Vec<String> = {
            let recent = recent_use(&inner.history, ms);
            let held: HashSet<&str> = inner.outgoing.values().map(|t| t.contact.as_str()).chain(inner.incoming.values().map(|t| t.contact.as_str())).collect();
            let list: Vec<pace::Open> = inner
                .live
                .iter()
                .map(|(id, l)| {
                    let c = inner.contacts.iter().find(|c| c.id == *id);
                    pace::Open {
                        id,
                        used: l.used,
                        held: held.contains(id.as_str()) || inner.open_chat.as_deref() == Some(id.as_str()) || c.is_none_or(pace::mine),
                        wanted: c.is_some_and(|c| wanted(c, &recent, ms)),
                    }
                })
                .collect();
            pace::close_idle(&list, now).into_iter().map(str::to_string).collect()
        };
        if shut.is_empty() {
            return;
        }
        for id in &shut {
            self.rest(&mut inner, id, now);
        }
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
    }

    /// Closes a connection for being quiet: both sides then call again only after a while (or
    /// at once when needed), and show "seen lately". Out of `live` at once (under the same lock
    /// that chose it), so a send starting now calls them again instead of using a connection
    /// that is closing. (Only ever one with nothing moving: see `close_idle`.)
    fn rest(&self, inner: &mut Inner, id: &str, now: std::time::Instant) {
        if let Some(l) = inner.live.remove(id) {
            l.conn.close(pace::IDLE_CODE.into(), b"idle");
            if let Some(c) = inner.contacts.iter_mut().find(|c| c.id == id) {
                c.last_seen = Some(now_ms());
            }
        }
        inner.tries.rested(id, now);
    }

    /// The conversation open in the window, closed for being quiet by their Pako (ours never
    /// closes it): called again, at most once a round, so it doesn't sit there "offline" with
    /// Send turned off while they're there. (Not there any more: the call fails and it shows
    /// offline, as anyone.)
    fn reach_open(self: &Arc<Self>) {
        let id = {
            let inner = self.inner.lock().unwrap();
            inner
                .open_chat
                .clone()
                .filter(|id| !inner.live.contains_key(id) && !inner.connecting.contains(id) && inner.tries.is_idle(id) && inner.contacts.iter().any(|c| c.id == *id && c.trusted && !c.declined))
        };
        if let Some(id) = id {
            self.dial(&id, true);
        }
    }

    /// Tries the contacts a transfer is waiting for (they're not connected), in its first hour
    /// of waiting: most cuts are short. After that, the usual round every few minutes is
    /// enough (and the other PC dials too when its Pako starts), so a laptop isn't kept dialling
    /// every 30 seconds for a week.
    fn reach_waiting(self: &Arc<Self>) {
        let now = now_ms();
        let fresh = |state: &str, since: u64| state == "waiting" && now.saturating_sub(since) < RETRY_WAITING_FOR;
        let ids: HashSet<String> = {
            let inner = self.inner.lock().unwrap();
            let waits = inner.outgoing.values().filter(|t| fresh(&t.state, t.since)).map(|t| &t.contact).chain(inner.incoming.values().filter(|t| fresh(&t.state, t.since)).map(|t| &t.contact));
            let relays = Self::relay_waits(&inner, now);
            waits.chain(relays.iter()).filter(|c| !inner.live.contains_key(*c) && inner.contacts.iter().any(|x| x.id == **c && !x.declined)).cloned().collect()
        };
        for id in ids {
            self.dial(&id, false);
        }
    }

    pub fn answer_request(&self, peer: &str, accept: bool) {
        let answer = self.inner.lock().unwrap().requests.get_mut(peer).and_then(|r| r.answer.take());
        if let Some(a) = answer {
            let _ = a.send(accept);
        }
    }

    /// One connection, from the handshake until it closes. `dialing`: we opened it.
    async fn run(self: Arc<Self>, conn: Connection, dialing: Option<Dialing>) {
        let initiator = dialing.is_some();
        let peer = conn.remote_id().to_string();
        let (send, recv) = if initiator {
            match conn.open_bi().await {
                Ok(s) => s,
                Err(_) => return,
            }
        } else {
            match tokio::time::timeout(CONNECT_TIMEOUT, conn.accept_bi()).await {
                Ok(Ok(s)) => s,
                _ => return,
            }
        };
        let (tx, rx) = mpsc::unbounded_channel::<Msg>();
        tokio::spawn(write_loop(send, rx));
        let mut recv = recv;
        let ok = if initiator { self.handshake_out(&peer, &tx, &mut recv, &conn).await } else { self.handshake_in(&peer, &tx, &mut recv, &conn).await };
        drop(dialing);
        if !ok {
            // We called and it didn't get through (no answer to the question, turned away): their
            // next call in the round waits as for one unanswered, not just the next 30 s tick.
            if initiator && !self.is_offline() {
                self.inner.lock().unwrap().tries.failed(&peer, std::time::Instant::now(), false, super::PRESENCE_EVERY);
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
            conn.close(1u32.into(), b"no");
            return;
        }
        // Keep one connection per contact: the one opened by the smaller ID.
        let opener = if initiator { self.my_id() } else { peer.clone() };
        let replaced = {
            let mut inner = self.inner.lock().unwrap();
            let mut replaced = false;
            if let Some(old) = inner.live.get(&peer) {
                if !replaces(&old.initiator, old.since.elapsed(), &opener) {
                    drop(inner);
                    conn.close(3u32.into(), b"duplicate");
                    return;
                }
                old.conn.close(3u32.into(), b"duplicate");
                replaced = true;
            }
            let now = std::time::Instant::now();
            inner.live.insert(peer.clone(), Live { conn: conn.clone(), tx: tx.clone(), initiator: opener, since: now, used: now });
            // Reached (they called us, or answered): their wait between calls starts over.
            inner.tries.reset(&peer);
            inner.waiting.remove(&peer);
            inner.notes.remove(&peer);
            if let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) {
                c.trusted = true;
                c.declined = false;
                c.last_seen = Some(now_ms());
            }
            // They reached us: something happened with them.
            if !initiator {
                Self::touch(&mut inner, &peer);
            }
            self.save_contacts(&inner);
            replaced
        };
        // What was moving on the connection this one replaced can't finish: ended now, instead
        // of waiting on a dead connection.
        if replaced {
            self.connection_lost(&peer);
        }
        self.device_hello(&peer);
        // Transfers with them that stopped half-way continue.
        self.pick_up(&peer);
        self.changed();
        // Files arrive on one-way streams.
        let data = {
            let me = self.clone();
            let conn = conn.clone();
            let peer = peer.clone();
            tokio::spawn(async move {
                while let Ok(stream) = conn.accept_uni().await {
                    let me = me.clone();
                    let peer = peer.clone();
                    tokio::spawn(async move { me.take_part(&peer, stream).await });
                }
            })
        };
        // The way it travels changes (a relay at first, then the home network or straight
        // over the internet; someone leaving the Wi-Fi): the window shows it at once.
        let paths = {
            use futures_util::StreamExt;
            let me = self.clone();
            let conn = conn.clone();
            let mut events = conn.path_events();
            let mut last = path_of(&conn);
            tokio::spawn(async move {
                while events.next().await.is_some() {
                    let now = path_of(&conn);
                    if now != last {
                        last = now;
                        me.changed();
                    }
                }
            })
        };
        loop {
            let msg = tokio::select! {
                _ = self.stop.cancelled() => break,
                m = proto::read_frame::<_, Msg>(&mut recv, MAX_MSG) => m,
            };
            match msg {
                Ok(Some(m)) => self.clone().on_message(&peer, &tx, m).await,
                _ => break,
            }
        }
        data.abort();
        paths.abort();
        // Closed for being quiet (by them or by us): called again later, not at the next round.
        let quiet = matches!(conn.close_reason(), Some(iroh::endpoint::ConnectionError::ApplicationClosed(c)) if c.error_code.into_inner() == pace::IDLE_CODE as u64);
        // Gone: whatever was moving with them stops.
        let still_ours = {
            let mut inner = self.inner.lock().unwrap();
            let ours = inner.live.get(&peer).is_some_and(|l| l.conn.stable_id() == conn.stable_id());
            if ours {
                inner.live.remove(&peer);
                let now = std::time::Instant::now();
                // (Also an Often contact: ours never closes it, so their Pako did; it's called
                // again after the same wait, or at once when you open or send.)
                if quiet {
                    inner.tries.rested(&peer, now);
                } else {
                    inner.tries.lost(&peer, now, super::PRESENCE_EVERY);
                }
                if let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) {
                    c.last_seen = Some(now_ms());
                }
                self.save_contacts(&inner);
            }
            ours
        };
        if still_ours {
            self.connection_lost(&peer);
            self.device_gone(&peer);
        }
        conn.close(0u32.into(), b"bye");
        self.changed();
    }

    async fn handshake_out(&self, peer: &str, tx: &mpsc::UnboundedSender<Msg>, recv: &mut RecvStream, conn: &Connection) -> bool {
        let (name, device, pc) = {
            let inner = self.inner.lock().unwrap();
            let device = inner.contacts.iter().any(|c| c.id == peer && c.device);
            // Pairing your own PC: it's asked with this PC's name ("Desk PC"), not yours.
            let pc = device.then(|| inner.dev.pc_name.clone()).filter(|n| !n.is_empty());
            (inner.my_name.clone(), device, pc)
        };
        if tx.send(Msg::Hello { name, device, pc }).is_err() {
            return false;
        }
        // They were asked (else a refusal came without asking: they declined or removed you before).
        let mut asked = false;
        loop {
            let m = tokio::select! {
                _ = conn.closed() => None,
                m = proto::read_frame::<_, Msg>(recv, MAX_MSG) => Some(m),
            };
            match m {
                Some(Ok(Some(Msg::Welcome { name }))) => {
                    self.remember_their_name(peer, &name);
                    return true;
                }
                Some(Ok(Some(Msg::Wait))) => {
                    asked = true;
                    self.inner.lock().unwrap().waiting.insert(peer.to_string(), ());
                    self.changed();
                }
                Some(Ok(Some(Msg::Refused))) => {
                    // Not asked again by itself: only the Connect button asks again.
                    {
                        let mut inner = self.inner.lock().unwrap();
                        inner.waiting.remove(peer);
                        if let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) {
                            c.declined = true;
                        }
                        self.save_contacts(&inner);
                    }
                    self.note(peer, Some("They declined. You can ask again after 7 days.".into()));
                    return false;
                }
                _ => {
                    self.inner.lock().unwrap().waiting.remove(peer);
                    if asked {
                        self.note(peer, Some("No answer yet. Pako asks again later.".into()));
                    } else {
                        self.changed();
                    }
                    return false;
                }
            }
        }
    }

    async fn handshake_in(&self, peer: &str, tx: &mpsc::UnboundedSender<Msg>, recv: &mut RecvStream, conn: &Connection) -> bool {
        let (hello, device, pc) = match tokio::time::timeout(CONNECT_TIMEOUT, proto::read_frame::<_, Msg>(recv, 64 * 1024)).await {
            Ok(Ok(Some(Msg::Hello { name, device, pc }))) => (name, device, pc.as_deref().and_then(clean_name)),
            _ => return false,
        };
        let my_name = self.inner.lock().unwrap().my_name.clone();
        // Someone in your contacts (you added them): connected, no question asked.
        if self.inner.lock().unwrap().contacts.iter().any(|c| c.id == peer) {
            self.remember_their_name(peer, &hello);
            if let (true, Some(pc)) = (device, &pc) {
                self.remember_pc_name(peer, pc);
            }
            return tx.send(Msg::Welcome { name: my_name }).is_ok();
        }
        // Someone you declined or removed lately: turned away without asking you again.
        let now = now_ms();
        if self.inner.lock().unwrap().blocked.iter().any(|b| b.id == peer && b.until > now) {
            let _ = tx.send(Msg::Refused);
            return false;
        }
        // Someone new: asked once. A PC pairing as your device shows its PC's name.
        let pc = pc.filter(|_| device);
        let shown = pc.clone().or_else(|| clean_name(&hello)).unwrap_or_else(|| PLACEHOLDER.into());
        let (atx, arx) = oneshot::channel();
        {
            let mut inner = self.inner.lock().unwrap();
            // Anyone who has your Pako ID can make new keys at will: past a few open questions,
            // more are turned away without a pop-up each.
            if inner.requests.contains_key(peer) || inner.requests.len() >= MAX_REQUESTS {
                return false;
            }
            // Nor one question after the other without end (each closed to make room).
            if !request_ok(&mut inner.request_times, std::time::Instant::now()) {
                return false;
            }
            let at = inner.next_seq();
            inner.requests.insert(peer.to_string(), Request { name: shown.clone(), device, answer: Some(atx), at });
        }
        let _ = tx.send(Msg::Wait);
        self.changed();
        if device {
            (self.notify)(
                "direct-request",
                "Pair a device",
                &format!("{shown} wants to pair with this PC as one of your devices."),
                Go { page: "devices".into(), peer: Some(peer.to_string()), ..Go::none() },
            );
        } else {
            (self.notify)(
                "direct-request",
                "Someone wants to connect",
                &format!("{shown} wants to connect with you in Pako."),
                Go { page: "direct".into(), peer: Some(peer.to_string()), ..Go::none() },
            );
        }
        let answer = tokio::select! {
            _ = conn.closed() => Answer::Silent,
            a = arx => match a {
                Ok(true) => Answer::Yes,
                Ok(false) => Answer::No,
                Err(_) => Answer::Silent,
            },
            _ = tokio::time::sleep(Duration::from_secs(600)) => Answer::Silent,
        };
        self.inner.lock().unwrap().requests.remove(peer);
        // Its pop-up and bell line say how it ended (answered here, in Pako's window, or not at all).
        let how = match answer {
            Answer::Yes => "Accepted",
            Answer::No => "Declined",
            Answer::Silent => "No longer waiting",
        };
        (self.settle)("direct-request", peer, how);
        match answer {
            Answer::Yes => {}
            Answer::No => {
                // Declined: they're told, and not asked about again for a week.
                self.block(&mut self.inner.lock().unwrap(), peer);
                let _ = tx.send(Msg::Refused);
                self.changed();
                return false;
            }
            Answer::Silent => {
                // No answer: the connection just closes (they see "No answer yet").
                self.changed();
                return false;
            }
        }
        {
            let mut inner = self.inner.lock().unwrap();
            if !inner.contacts.iter().any(|c| c.id == peer) {
                inner.contacts.push(Contact {
                    id: peer.to_string(),
                    name: shown.clone(),
                    trusted: true,
                    auto_accept: false,
                    save_folder: None,
                    last_seen: Some(now_ms()),
                    added: now_ms(),
                    tags: Vec::new(),
                    favourite: false,
                    // None when they gave no name: "Someone" is only what you see until they do.
                    their_name: clean_name(&hello),
                    their_pc_name: pc,
                    declined: false,
                    // A device you confirmed: it may control this Pako until you turn that off;
                    // its files go to the folder you chose when pairing.
                    device,
                    control: device,
                    ..Default::default()
                });
                if device {
                    let folder = inner.dev.folder_for_request(peer);
                    if let Some(c) = inner.contacts.last_mut() {
                        c.save_folder = folder;
                    }
                }
                self.save_contacts(&inner);
            }
        }
        // One more contact: your devices' "From" menu says so.
        self.tell_devices();
        tx.send(Msg::Welcome { name: my_name }).is_ok()
    }

    /// Keeps the name a contact chose for themselves up to date. Your name for them stays, unless
    /// it was only theirs or the "Someone" placeholder (then it follows theirs).
    fn remember_their_name(&self, peer: &str, said: &str) {
        let mut inner = self.inner.lock().unwrap();
        let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) else { return };
        let Some((name, theirs)) = updated_names(&c.name, c.their_name.as_deref(), said) else { return };
        // One of your devices that said its PC's name goes by that one, not its owner's. (A
        // plain contact's PC name never replaces the name of the person: see remember_pc_name.)
        if !(c.device && c.their_pc_name.is_some()) {
            c.name = name;
        }
        c.their_name = Some(theirs);
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
    }

    fn connection_lost(&self, peer: &str) {
        let (outs, ins): (Vec<String>, Vec<String>) = {
            let inner = self.inner.lock().unwrap();
            (
                inner.outgoing.iter().filter(|(_, t)| t.contact == peer).map(|(k, _)| k.clone()).collect(),
                inner.incoming.iter().filter(|(_, t)| t.contact == peer).map(|(k, _)| k.clone()).collect(),
            )
        };
        // Kept to continue later when both Pakos can; ended otherwise (as before).
        for id in outs {
            if !self.pause_out(&id) {
                self.end_outgoing(&id, "failed", Some("The connection was lost.".into()));
            }
        }
        for id in ins {
            if !self.pause_in(&id) {
                self.end_incoming(&id, "failed", Some("The connection was lost.".into()));
            }
        }
    }

    async fn on_message(self: Arc<Self>, peer: &str, tx: &mpsc::UnboundedSender<Msg>, m: Msg) {
        if let Some(l) = self.inner.lock().unwrap().live.get_mut(peer) {
            l.used = std::time::Instant::now();
        }
        match m {
            Msg::Offer { id, name, files, resume, relay } => self.on_offer(peer, tx, id, name, files, resume, relay),
            // A transfer waiting to continue that their Pako finished (or turned away) meanwhile.
            Msg::Result { ref id, ok, ref message } if self.inner.lock().unwrap().outgoing.get(id).is_some_and(|t| t.contact == peer && t.state == "waiting") => {
                if ok {
                    self.end_outgoing(id, "done", None);
                } else {
                    self.end_outgoing(id, "failed", peer_message(message.clone()).or(Some("Their Pako couldn't take it.".into())));
                }
            }
            Msg::Accept { .. } | Msg::Decline { .. } | Msg::Result { .. } => {
                let id = match &m {
                    Msg::Accept { id, .. } | Msg::Decline { id } | Msg::Result { id, .. } => id.clone(),
                    _ => return,
                };
                // Only answers about our own transfers to this contact.
                let ev = self.inner.lock().unwrap().outgoing.get(&id).filter(|t| t.contact == peer).map(|t| t.events.clone());
                if let Some(ev) = ev {
                    let _ = ev.send(m);
                }
            }
            Msg::Cancel { id } => {
                let (theirs_out, theirs_in) = {
                    let inner = self.inner.lock().unwrap();
                    (
                        inner.outgoing.get(&id).is_some_and(|t| t.contact == peer),
                        inner.incoming.get(&id).is_some_and(|t| t.contact == peer),
                    )
                };
                if theirs_out {
                    self.end_out(&id, "cancelled", Some("They cancelled it.".into()), false);
                } else if theirs_in {
                    self.end_in(&id, "cancelled", Some("They cancelled it.".into()), false);
                }
            }
            Msg::Done { id, hashes } => {
                // Marked at once, so a second "all sent" never starts a second check.
                let theirs = {
                    let mut inner = self.inner.lock().unwrap();
                    match inner.incoming.get_mut(&id) {
                        Some(t) if t.contact == peer && t.state == "receiving" && !t.finishing => {
                            t.finishing = true;
                            true
                        }
                        _ => false,
                    }
                };
                if theirs {
                    let me = self.clone();
                    let tx = tx.clone();
                    tokio::spawn(async move { me.finish_incoming(&id, hashes, tx).await });
                }
            }
            // Each one rewrites the contacts (and may notify): a contact's Pako sending them
            // without end can't keep the disk and the window busy.
            Msg::Name { .. } | Msg::Device { .. } if !small_ok(self.inner.lock().unwrap().small_times.entry(peer.to_string()).or_default(), std::time::Instant::now()) => {}
            Msg::Name { name } => self.remember_their_name(peer, &name),
            Msg::Device { mine, control, pc_name, relay } => self.on_device(peer, mine, control, pc_name, relay),
            Msg::Look { id, look } => self.on_look(peer, id, look),
            Msg::Seen { id, ok, message, me, contacts, history, moving } => self.on_seen(peer, &id, relay::Seen { ok, message, me, contacts, history, moving }),
            Msg::Relayed { id, state, done, message, .. } => self.on_relayed(peer, &id, &state, done, message),
            Msg::Unrelay { id } => self.on_unrelay(peer, &id),
            Msg::Watch { on } => self.on_watch(peer, on),
            Msg::List { items } => self.on_list(peer, items),
            // Done off the connection's reading (adding links can take a moment).
            Msg::Order { id, order } => {
                let peer = peer.to_string();
                tokio::spawn(self.on_order(peer, id, order));
            }
            Msg::Reply { id, ok, message, failed, items } => self.on_reply(peer, &id, ok, message, failed, items),
            Msg::Resume { id } => self.on_resume(peer, tx, &id),
            Msg::Have { id, full, part } => {
                let known = self.inner.lock().unwrap().outgoing.get(&id).filter(|t| t.contact == peer).map(|t| t.resume && t.state != "offered");
                match known {
                    Some(true) => {
                        let peer = peer.to_string();
                        tokio::spawn(self.resume_out(peer, id, full, part));
                    }
                    // Not accepted yet, or a transfer that can't continue: nothing to do.
                    Some(false) => {}
                    // Not one of ours with them (ended here meanwhile, or never theirs).
                    None => self.unknown_transfer(peer, tx, &id),
                }
            }
            Msg::Continue { id, restart } => {
                let ours = self.inner.lock().unwrap().incoming.get(&id).is_some_and(|t| t.contact == peer && t.resume && t.state == "waiting");
                if ours {
                    let me = self.clone();
                    let peer = peer.to_string();
                    // Opens and sizes files: off the connection's task.
                    self.rt.spawn_blocking(move || me.continue_in(&peer, &id, restart));
                }
            }
            // Refused (they removed you), Hello/Welcome/Wait after the handshake: nothing to do.
            _ => {}
        }
    }

    // ---------- sending ----------

    /// Sends files and folders to a connected contact. `compress`: one compressed ZIP.
    pub async fn send(self: &Arc<Self>, contact: &str, paths: Vec<PathBuf>, compress: bool, name: Option<String>) -> Result<String, String> {
        self.send_as(contact, paths, compress, name, None).await
    }

    /// Like `send`; `relay`: to one of your devices, for it to pass on to one of its contacts
    /// (with that contact's name there).
    pub(super) async fn send_as(self: &Arc<Self>, contact: &str, paths: Vec<PathBuf>, compress: bool, name: Option<String>, relay: Option<(proto::RelayTo, String)>) -> Result<String, String> {
        self.touch_now(contact);
        if !self.inner.lock().unwrap().live.contains_key(contact) {
            self.call_now(contact).await;
        }
        if !self.inner.lock().unwrap().live.contains_key(contact) {
            return Err("They aren't connected right now.".into());
        }
        if paths.is_empty() {
            return Err("Add files first.".into());
        }
        let id = new_id();
        let (files, title, temp) = if compress {
            let (zip, title) = self.make_zip(&id, &paths, name.clone()).await?;
            let size = std::fs::metadata(&zip).map_err(|e| e.to_string())?.len();
            (vec![(zip.clone(), OfferFile { path: title.clone(), size })], title, Some(zip))
        } else {
            let list = tokio::task::spawn_blocking(move || list_files(&paths)).await.map_err(|e| e.to_string())??;
            let title = name.filter(|n| !n.trim().is_empty()).map(|n| n.trim().to_string()).unwrap_or_else(|| default_title(&list));
            (list, title, None)
        };
        self.offer_files(contact, id, files, title, temp, relay).await
    }

    /// You send to them: something happened with them.
    fn touch_now(&self, contact: &str) {
        let mut inner = self.inner.lock().unwrap();
        if inner.contacts.iter().any(|c| c.id == contact) {
            Self::touch(&mut inner, contact);
            self.save_contacts(&inner);
        }
    }

    /// Offers files already listed (a relay's copy, passed on as a normal send of this PC).
    pub(super) async fn offer_list(self: &Arc<Self>, contact: &str, files: Vec<(PathBuf, OfferFile)>, title: String, temp: Option<PathBuf>) -> Result<String, String> {
        self.touch_now(contact);
        self.offer_files(contact, new_id(), files, title, temp, None).await
    }

    async fn offer_files(
        self: &Arc<Self>,
        contact: &str,
        id: String,
        files: Vec<(PathBuf, OfferFile)>,
        title: String,
        temp: Option<PathBuf>,
        relay: Option<(proto::RelayTo, String)>,
    ) -> Result<String, String> {
        // The same checks the other side makes, so a problem shows here with a clear reason.
        let listed: Vec<OfferFile> = files.iter().map(|(_, f)| f.clone()).collect();
        let problem = match proto::check_offer(&title, &listed) {
            Err(e) => Some(format!("This can't be sent as it is: {e}")),
            Ok(_) => {
                let offer = Msg::Offer { id: id.clone(), name: title.clone(), files: listed, resume: true, relay: relay.as_ref().map(|(r, _)| r.clone()) };
                let size = serde_json::to_vec(&offer).map(|b| b.len()).unwrap_or(usize::MAX);
                (size > MAX_MSG).then(|| "Too many files (or very long names) for one send. Turn on Compressed ZIP, or send them in a few goes.".to_string())
            }
        };
        if let Some(e) = problem {
            if let Some(t) = &temp {
                let _ = std::fs::remove_file(t);
            }
            return Err(e);
        }
        let total = files.iter().map(|(_, f)| f.size).sum();
        // As they are now: a file changed by the time a cut transfer continues goes again from zero.
        let mtimes = {
            let paths: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
            tokio::task::spawn_blocking(move || paths.iter().map(|p| std::fs::metadata(p).map(|m| resume::mtime_of(&m)).unwrap_or(0)).collect::<Vec<u64>>())
                .await
                .unwrap_or_default()
        };
        let (etx, erx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        // The connection as it is now: listing or zipping may have taken minutes, and they may
        // have reconnected meanwhile (an offer into the old connection would wait for ever).
        // Closed for being quiet while the files were listed or zipped: called again now.
        if !self.inner.lock().unwrap().live.contains_key(contact) {
            self.call_now(contact).await;
        }
        let tx = {
            let mut inner = self.inner.lock().unwrap();
            let Some(tx) = inner.live.get(contact).map(|l| l.tx.clone()) else {
                if let Some(t) = &temp {
                    let _ = std::fs::remove_file(t);
                }
                return Err("They aren't connected right now.".into());
            };
            // A relay asked of one of your devices: followed here until it ends (both legs).
            if let Some((r, to_name)) = &relay {
                inner.relay.list.push(store::Relay {
                    id: id.clone(),
                    mine: true,
                    device: contact.to_string(),
                    to: r.to.clone(),
                    to_name: to_name.clone(),
                    name: title.clone(),
                    files: files.len() as u32,
                    size: total,
                    date: now_ms(),
                    keep: r.keep,
                    // (Its first leg, to that device: "leg1" until it's there.)
                    state: "leg1".into(),
                    ..Default::default()
                });
                self.save_relays(&mut inner);
            }
            inner.outgoing.insert(
                id.clone(),
                Outgoing {
                    contact: contact.to_string(),
                    name: title.clone(),
                    files: files.clone(),
                    total,
                    sent: Arc::new(AtomicU64::new(0)),
                    state: "offered".into(),
                    message: None,
                    cancel: cancel.clone(),
                    events: etx,
                    temp,
                    meter: Arc::new(Meter::default()),
                    resume: false,
                    mtimes,
                    base: 0,
                    since: now_ms(),
                },
            );
            tx
        };
        let offer = Msg::Offer { id: id.clone(), name: title, files: files.iter().map(|(_, f)| f.clone()).collect(), resume: self.resume_on(), relay: relay.map(|(r, _)| r) };
        if tx.send(offer).is_err() {
            self.end_outgoing(&id, "failed", Some("The connection was lost.".into()));
            return Err("The connection was lost.".into());
        }
        self.changed();
        let me = self.clone();
        let tid = id.clone();
        let contact = contact.to_string();
        self.rt.spawn(async move { me.drive_outgoing(&tid, &contact, erx, cancel).await });
        Ok(id)
    }

    /// A contact you're sending to isn't connected (maybe closed for being quiet): called now,
    /// waiting for them as long as a call may take. Only someone who said yes once already.
    async fn call_now(self: &Arc<Self>, contact: &str) {
        {
            let mut inner = self.inner.lock().unwrap();
            if !inner.contacts.iter().any(|c| c.id == contact && c.trusted && !c.declined) {
                return;
            }
            inner.tries.reset(contact);
        }
        self.dial(contact, true);
        // (The call itself may take up to CONNECT_TIMEOUT.)
        let mut ended = 0;
        for _ in 0..170 {
            {
                let inner = self.inner.lock().unwrap();
                if inner.live.contains_key(contact) {
                    return;
                }
                // The call ended without them (seen a few times in a row: between the call
                // and the connection being kept, it's neither for a moment).
                ended = if inner.connecting.contains(contact) { 0 } else { ended + 1 };
            }
            if ended >= 4 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn make_zip(&self, id: &str, paths: &[PathBuf], name: Option<String>) -> Result<(PathBuf, String), String> {
        let dir = self.data_dir.join("direct_tmp");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let out = dir.join(format!("{id}.zip"));
        let paths = paths.to_vec();
        let target = out.clone();
        let payload_name = tokio::task::spawn_blocking(move || -> Result<String, String> {
            let made = (|| -> Result<String, String> {
                let payload = crate::send::gather(&paths, crate::send::Mode::Compress).map_err(|e| e.to_string())?;
                let pname = payload.name().to_string();
                let entries = match payload {
                    crate::send::Payload::Zip { entries, .. } | crate::send::Payload::Files { entries, .. } => entries,
                    crate::send::Payload::File(e) => vec![e],
                };
                let f = std::fs::File::create(&target).map_err(|e| e.to_string())?;
                let mut w = std::io::BufWriter::new(f);
                crate::send::write_zip(&entries, crate::send::Pack::Max, &mut w, &AtomicBool::new(false), 0xFFFF_0000).map_err(|e| e.to_string())?;
                use std::io::Write;
                w.flush().map_err(|e| e.to_string())?;
                Ok(pname)
            })();
            // A ZIP that couldn't be finished doesn't stay behind.
            if made.is_err() {
                let _ = std::fs::remove_file(&target);
            }
            made
        })
        .await
        .map_err(|e| e.to_string())??;
        let mut title = name.filter(|n| !n.trim().is_empty()).map(|n| n.trim().to_string()).unwrap_or(payload_name);
        if !title.to_lowercase().ends_with(".zip") {
            title.push_str(".zip");
        }
        let title = proto::safe_part(&title).unwrap_or_else(|| "files.zip".into());
        Ok((out, title))
    }

    async fn drive_outgoing(self: Arc<Self>, id: &str, contact: &str, mut events: mpsc::UnboundedReceiver<Msg>, cancel: CancellationToken) {
        // Wait for their answer.
        let answer = tokio::select! {
            _ = cancel.cancelled() => return,
            m = events.recv() => m,
        };
        let resume = match answer {
            Some(Msg::Accept { resume, .. }) => resume && self.resume_on(),
            Some(Msg::Decline { .. }) => {
                self.end_outgoing(id, "declined", Some("They declined it.".into()));
                return;
            }
            // Their Pako turned it away without asking (and says why).
            Some(Msg::Result { ok: false, message, .. }) => {
                self.end_outgoing(id, "failed", peer_message(message).or(Some("Their Pako couldn't take it.".into())));
                return;
            }
            _ => {
                self.end_outgoing(id, "failed", Some("No answer.".into()));
                return;
            }
        };
        // Their Pako keeps what arrives: this side keeps the list, to continue after a cut.
        let (queue, note) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.outgoing.get_mut(id) else { return };
            t.resume = resume;
            let mut queue = VecDeque::new();
            for (i, (_, f)) in t.files.iter().enumerate() {
                parts_for(&mut queue, id, i, 0, f.size);
            }
            (queue, resume.then(|| out_note(id, t)))
        };
        if let Some(n) = note {
            self.save_out_note(&n);
        }
        self.transmit(id, contact, events, cancel, queue).await;
    }

    /// Sends the parts in `queue`, then "all sent" with each file's hash, and waits for the
    /// receiver's verdict.
    async fn transmit(self: Arc<Self>, id: &str, contact: &str, mut events: mpsc::UnboundedReceiver<Msg>, cancel: CancellationToken, queue: VecDeque<Part>) {
        let (files, sent) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.outgoing.get_mut(id) else { return };
            if cancel.is_cancelled() {
                return;
            }
            t.state = "sending".into();
            t.since = now_ms();
            t.meter.start(t.sent.clone(), t.cancel.clone(), &self.rt);
            (t.files.clone(), t.sent.clone())
        };
        self.watch(id, true, sent.clone(), cancel.clone());
        self.changed();
        let Some(conn) = self.inner.lock().unwrap().live.get(contact).map(|l| l.conn.clone()) else {
            if !self.pause_out(id) {
                self.end_outgoing(id, "failed", Some("The connection was lost.".into()));
            }
            return;
        };
        // Hashes are made alongside the sending.
        let hash_paths: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
        let hashing = tokio::task::spawn_blocking(move || hash_files(&hash_paths));
        let queue = Arc::new(Mutex::new(queue));
        let paths: Arc<Vec<PathBuf>> = Arc::new(files.iter().map(|(p, _)| p.clone()).collect());
        let pace = self.pace.load(Ordering::Relaxed);
        let mut workers = Vec::new();
        for _ in 0..STREAMS {
            let (queue, paths, conn, sent, cancel) = (queue.clone(), paths.clone(), conn.clone(), sent.clone(), cancel.clone());
            workers.push(tokio::spawn(async move {
                loop {
                    let part = queue.lock().unwrap().pop_front();
                    let Some(part) = part else { return Ok::<(), String>(()) };
                    tokio::select! {
                        _ = cancel.cancelled() => return Err("cancelled".into()),
                        r = send_part(&conn, &paths[part.file], &part, &sent, pace) => r?,
                    }
                }
            }));
        }
        let mut failed = None;
        for w in workers {
            match w.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => failed = Some(e),
                Err(e) => failed = Some(e.to_string()),
            }
        }
        if cancel.is_cancelled() {
            return;
        }
        if let Some(e) = failed {
            // The connection dropped: kept to continue when they're back (if both Pakos can).
            if conn.close_reason().is_some() && self.pause_out(id) {
                return;
            }
            self.end_outgoing(id, "failed", Some(e));
            return;
        }
        // All sent: the hashes may still be in the making.
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(t) = inner.outgoing.get_mut(id) {
                t.state = "checking".into();
            }
        }
        self.changed();
        let hashes = match hashing.await {
            Ok(Ok(h)) => h,
            Ok(Err(e)) => {
                self.end_outgoing(id, "failed", Some(e));
                return;
            }
            Err(e) => {
                self.end_outgoing(id, "failed", Some(e.to_string()));
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        let tx = self.inner.lock().unwrap().live.get(contact).map(|l| l.tx.clone());
        if tx.and_then(|t| t.send(Msg::Done { id: id.to_string(), hashes }).ok()).is_none() {
            if !self.pause_out(id) {
                self.end_outgoing(id, "failed", Some("The connection was lost.".into()));
            }
            return;
        }
        let verdict = tokio::select! {
            _ = cancel.cancelled() => return,
            m = events.recv() => m,
        };
        match verdict {
            Some(Msg::Result { ok: true, .. }) => self.end_outgoing(id, "done", None),
            Some(Msg::Result { message, .. }) => self.end_outgoing(id, "failed", peer_message(message).or(Some("They couldn't check the files.".into()))),
            _ => self.end_outgoing(id, "failed", Some("No answer.".into())),
        }
    }

    /// The connection dropped (or the receiver starts a new round): a sending transfer both
    /// Pakos keep waits for them to connect again. False if it can't continue (it then ends).
    fn pause_out(&self, id: &str) -> bool {
        let note = {
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.outgoing.get_mut(id) else { return true };
            if !t.resume || !matches!(t.state.as_str(), "sending" | "checking" | "waiting") {
                return false;
            }
            // What runs for it now stops (a new round gets its own).
            t.cancel.cancel();
            t.cancel = CancellationToken::new();
            if t.state == "waiting" {
                None
            } else {
                t.state = "waiting".into();
                t.since = now_ms();
                Some(out_note(id, t))
            }
        };
        if let Some(n) = note {
            self.save_out_note(&n);
            self.changed();
        }
        true
    }

    fn save_out_note(&self, n: &OutNote) {
        if let Err(e) = resume::save_out(&self.data_dir, n) {
            crate::errorlog::note(crate::errorlog::Area::Direct, &format!("Couldn't keep a transfer to continue later ({e})."));
        }
    }

    /// The receiver says what it already has (connected again, or a file to send again): the
    /// rest goes. Files changed since go again from zero; a file gone ends it with the reason.
    async fn resume_out(self: Arc<Self>, peer: String, id: String, full: Vec<[usize; 2]>, part: Vec<FileHave>) {
        // A round still going stops first (a file didn't match its hash there).
        if !self.pause_out(&id) {
            return;
        }
        let (mut files, mut mtimes) = {
            let inner = self.inner.lock().unwrap();
            let Some(t) = inner.outgoing.get(&id).filter(|t| t.contact == peer && t.state == "waiting") else { return };
            (t.files.clone(), t.mtimes.clone())
        };
        let sizes: Vec<u64> = files.iter().map(|(_, f)| f.size).collect();
        let Some(mut have) = resume::decode(&full, &part, &sizes) else {
            self.end_out(&id, "failed", Some("Their Pako sent something this one doesn't understand, so it stopped.".into()), true);
            return;
        };
        let paths: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
        let now = tokio::task::spawn_blocking(move || {
            paths.iter().map(|p| std::fs::metadata(p).ok().filter(|m| m.is_file()).map(|m| (m.len(), resume::mtime_of(&m)))).collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        if now.len() != files.len() {
            return;
        }
        let mut restart = Vec::new();
        for (i, now) in now.into_iter().enumerate() {
            match now {
                None => {
                    let why = format!("{} is gone or can't be read on this PC, so this couldn't be finished. Send it again.", files[i].1.path);
                    self.end_out(&id, "failed", Some(why), true);
                    return;
                }
                Some((len, mtime)) if len != files[i].1.size || mtime != mtimes[i] => {
                    restart.push(Restart { file: i, size: len });
                    have[i] = Ranges::default();
                    files[i].1.size = len;
                    mtimes[i] = mtime;
                }
                Some(_) => {}
            }
        }
        let mut queue = VecDeque::new();
        let mut missing = 0;
        for (i, r) in have.iter().enumerate() {
            for [s, e] in r.missing(files[i].1.size) {
                missing += e - s;
                parts_for(&mut queue, &id, i, s, e);
            }
        }
        let total: u64 = files.iter().map(|(_, f)| f.size).sum();
        let done = total.saturating_sub(missing);
        let (etx, mut erx) = mpsc::unbounded_channel();
        let (tx, cancel, note) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(tx) = inner.live.get(&peer).map(|l| l.tx.clone()) else { return };
            let Some(t) = inner.outgoing.get_mut(&id).filter(|t| t.state == "waiting") else { return };
            t.files = files;
            t.mtimes = mtimes;
            t.total = total;
            t.sent.store(done, Ordering::Relaxed);
            t.base = done;
            t.events = etx;
            t.meter = Arc::new(Meter::default());
            (tx, t.cancel.clone(), out_note(&id, t))
        };
        self.save_out_note(&note);
        self.changed();
        if tx.send(Msg::Continue { id: id.clone(), restart }).is_err() {
            return;
        }
        // Their Pako gets ready (makes room for the files that changed), then says go.
        let answer = tokio::select! {
            _ = cancel.cancelled() => return,
            m = erx.recv() => m,
        };
        if matches!(answer, Some(Msg::Accept { .. })) {
            self.transmit(&id, &peer, erx, cancel, queue).await;
        }
    }

    pub fn cancel(&self, id: &str) {
        let (out_peer, in_peer) = {
            let inner = self.inner.lock().unwrap();
            (inner.outgoing.get(id).map(|t| t.contact.clone()), inner.incoming.get(id).map(|t| t.contact.clone()))
        };
        let peer = out_peer.clone().or(in_peer.clone());
        if let Some(p) = &peer {
            if let Some(tx) = self.inner.lock().unwrap().live.get(p).map(|l| l.tx.clone()) {
                let _ = tx.send(Msg::Cancel { id: id.to_string() });
            }
        }
        if out_peer.is_some() {
            self.end_out(id, "cancelled", None, false);
        } else if in_peer.is_some() {
            self.end_in(id, "cancelled", None, false);
        }
    }

    /// Tells the other side a transfer stopped here, so it doesn't wait for it.
    fn tell_cancel(&self, contact: &str, id: &str) {
        if let Some(tx) = self.inner.lock().unwrap().live.get(contact).map(|l| l.tx.clone()) {
            let _ = tx.send(Msg::Cancel { id: id.to_string() });
        }
    }

    /// Stops a transfer when nothing moves for `STALL` while it should be moving.
    fn watch(self: &Arc<Self>, id: &str, outgoing: bool, counter: Arc<AtomicU64>, cancel: CancellationToken) {
        let me = self.clone();
        let id = id.to_string();
        self.rt.spawn(async move {
            let mut last = counter.load(Ordering::Relaxed);
            let mut since = tokio::time::Instant::now();
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                }
                let now = counter.load(Ordering::Relaxed);
                let moving = {
                    let inner = me.inner.lock().unwrap();
                    if outgoing {
                        inner.outgoing.get(&id).is_some_and(|t| t.state == "sending")
                    } else {
                        inner.incoming.get(&id).is_some_and(|t| t.state == "receiving" && !t.finishing)
                    }
                };
                if now != last || !moving {
                    last = now;
                    since = tokio::time::Instant::now();
                    continue;
                }
                if since.elapsed() >= STALL {
                    let why = Some("Nothing moved for 2 minutes, so it stopped.".to_string());
                    if outgoing {
                        me.end_out(&id, "failed", why, true);
                    } else {
                        me.end_in(&id, "failed", why, true);
                    }
                    return;
                }
            }
        });
    }

    /// Ends a sending transfer. A failure here is told to the other side (so it doesn't wait).
    fn end_outgoing(&self, id: &str, result: &str, message: Option<String>) {
        self.end_out(id, result, message, result == "failed");
    }

    /// Ends a sending transfer; `tell`: send "cancel" to the other side.
    fn end_out(&self, id: &str, result: &str, message: Option<String>, tell: bool) {
        let t = self.inner.lock().unwrap().outgoing.remove(id);
        let Some(t) = t else { return };
        if let Some(m) = message.as_deref().filter(|_| result == "failed") { crate::errorlog::note(crate::errorlog::Area::Direct, m); }
        t.cancel.cancel();
        if t.resume {
            resume::remove_out(&self.data_dir, id);
        }
        if tell {
            self.tell_cancel(&t.contact, id);
        }
        if let Some(tmp) = &t.temp {
            let _ = std::fs::remove_file(tmp);
        }
        let (path, via, relay_to) = {
            let inner = self.inner.lock().unwrap();
            (inner.live.get(&t.contact).and_then(|l| path_of(&l.conn)), self.via_of(&inner, id), Self::relay_to_of(&inner, id))
        };
        let (duration_ms, avg_speed, peak_speed) = t.meter.summary(t.sent.load(Ordering::Relaxed).saturating_sub(t.base));
        self.record(HistoryItem {
            id: id.to_string(),
            contact: t.contact.clone(),
            outgoing: true,
            name: t.name,
            files: t.files.len() as u32,
            size: t.total,
            date: now_ms(),
            path,
            result: result.into(),
            message: message.clone(),
            saved_to: None,
            duration_ms,
            avg_speed,
            peak_speed,
            via,
            relay_to,
        });
        // A relay's leg: the relay goes on (or ends) from here.
        self.relay_leg1_ended(id, result, message.clone(), Vec::new());
        self.relay_leg2_ended(id, result, message);
        self.changed();
    }

    // ---------- receiving ----------

    #[allow(clippy::too_many_arguments)]
    fn on_offer(self: Arc<Self>, peer: &str, tx: &mpsc::UnboundedSender<Msg>, id: String, name: String, files: Vec<OfferFile>, resume: bool, relay: Option<proto::RelayTo>) {
        let contact = self.inner.lock().unwrap().contacts.iter().find(|c| c.id == peer).cloned();
        let Some(contact) = contact else {
            let _ = tx.send(Msg::Decline { id });
            return;
        };
        // Ids are UUIDs made by Pako, never one already used here (it would mix up transfers).
        if !proto::valid_id(&id) {
            return;
        }
        let turned_away = |why: String| {
            let _ = tx.send(Msg::Result { id: id.clone(), ok: false, message: Some(why) });
        };
        let flood = offer_flood(self.inner.lock().unwrap().offer_times.entry(peer.to_string()).or_default(), std::time::Instant::now());
        match flood {
            Flood::Fine => {}
            Flood::TurnAway => {
                turned_away("Too many sends in a short time. Try again in a minute.".into());
                return;
            }
            Flood::Ignore => return,
        }
        {
            let inner = self.inner.lock().unwrap();
            if inner.incoming.contains_key(&id) || inner.outgoing.contains_key(&id) || inner.history.iter().any(|h| h.id == id) {
                return;
            }
            let asking = inner.incoming.values().filter(|t| t.contact == peer && t.state == "asking").count();
            if asking >= MAX_ASKING {
                turned_away(format!("They already have {MAX_ASKING} sends from you waiting for an answer."));
                return;
            }
        }
        let total = match proto::check_offer(&name, &files) {
            Ok(t) => t,
            Err(e) => {
                turned_away(format!("Their Pako couldn't take it: {e}"));
                return;
            }
        };
        let name = proto::safe_part(&name).unwrap_or_else(|| "files".into());
        // For one of this PC's contacts (Direct relay): its own new folder in Pako's data, after
        // the checks (a device that may control this Pako, one of your contacts, the room).
        let relay_dir = match &relay {
            None => None,
            Some(r) => match self.relay_take(peer, &id, &name, &files, total, r) {
                Ok(dir) => Some(dir),
                Err(why) => {
                    turned_away(why);
                    return;
                }
            },
        };
        // "Can send without asking" still asks when the send is big for the drive.
        // Your own devices (paired both ways) send without asking, like "can send without asking".
        let auto = contact.auto_accept || devices::paired(&contact);
        let page = if contact.device { "devices" } else { "direct" };
        let asked_because = if auto && relay_dir.is_none() {
            match crate::system::disk_space(&self.folder_for(&contact)) {
                Some((free, size)) if total.saturating_add(margin_for(size)) > free || total > free / 2 => {
                    Some(format!("Asked because it's big for the drive: {} is free there.", human(free)))
                }
                _ => None,
            }
        } else {
            None
        };
        let ask = !auto || asked_because.is_some();
        {
            let mut inner = self.inner.lock().unwrap();
            let at = inner.next_seq();
            let n = files.len();
            inner.incoming.insert(
                id.clone(),
                Incoming {
                    contact: peer.to_string(),
                    name: name.clone(),
                    files,
                    total,
                    received: Arc::new(AtomicU64::new(0)),
                    state: "asking".into(),
                    message: None,
                    cancel: CancellationToken::new(),
                    targets: Vec::new(),
                    parts: Vec::new(),
                    handles: Vec::new(),
                    saved_to: None,
                    progress: Arc::new(Notify::new()),
                    meter: Arc::new(Meter::default()),
                    at,
                    finishing: false,
                    asked_because,
                    resume: resume && self.resume_on(),
                    got: Arc::new(Mutex::new(Got::new(vec![Ranges::default(); n]))),
                    note: Arc::default(),
                    retried: vec![false; n],
                    base: 0,
                    since: now_ms(),
                },
            );
        }
        if !ask {
            // (A relay is quiet here: the send to the contact shows in Direct.)
            if relay_dir.is_none() {
                self.tell(peer, "direct-incoming", "Files coming in", &format!("{} is sending you {} ({}).", contact.name, name, human(total)), Go::item(page, peer));
            }
            // Making the files takes a moment: off the connection's task.
            let me = self.clone();
            let folder = relay_dir.map(|d| d.display().to_string());
            self.rt.spawn_blocking(move || {
                let _ = me.answer_offer(&id, true, folder);
            });
        } else {
            self.tell(
                peer,
                "direct-offer",
                "Files for you",
                &format!("{} wants to send you {} ({}).", contact.name, name, human(total)),
                Go { page: page.into(), id: Some(peer.to_string()), offer: Some(id.clone()), ..Go::none() },
            );
            self.changed();
        }
    }

    /// Accept (into `folder`, or the contact's folder) or decline an offer. Makes the files on
    /// the disk: call it off the window's thread.
    pub fn answer_offer(self: &Arc<Self>, id: &str, accept: bool, folder: Option<String>) -> Result<(), String> {
        let (peer, files, total, name, tx) = {
            let mut inner = self.inner.lock().unwrap();
            let tx = inner.incoming.get(id).and_then(|t| inner.live.get(&t.contact)).map(|l| l.tx.clone());
            let t = inner.incoming.get_mut(id).ok_or("This offer is gone.")?;
            if t.state != "asking" {
                return Ok(());
            }
            if accept {
                if tx.is_none() {
                    return Err("They aren't connected any more.".into());
                }
                // Taken at once, so a second click doesn't prepare it twice.
                t.state = "preparing".into();
            }
            (t.contact.clone(), t.files.clone(), t.total, t.name.clone(), tx)
        };
        if !accept {
            if let Some(tx) = tx {
                let _ = tx.send(Msg::Decline { id: id.to_string() });
            }
            self.end_incoming(id, "declined", None);
            return Ok(());
        }
        let Some(tx) = tx else { return Err("They aren't connected any more.".into()) };
        self.changed();
        let base = match folder.filter(|f| !f.trim().is_empty()) {
            Some(f) => PathBuf::from(f),
            None => {
                let c = self.inner.lock().unwrap().contacts.iter().find(|c| c.id == peer).cloned();
                let Some(c) = c else {
                    self.end_incoming(id, "failed", Some("Contact not found.".into()));
                    return Err("Contact not found.".into());
                };
                self.folder_for(&c)
            }
        };
        let prepared = prepare_targets(&base, &name, &files, total);
        let (targets, parts, handles, saved_to) = match prepared {
            Ok(p) => p,
            Err(e) => {
                // The reason kept here names folders of this PC: the other side only learns
                // whether it was the room.
                let why = if e.starts_with(NO_ROOM) { "Their Pako couldn't save it: there isn't enough room on their drive." } else { "Their Pako couldn't save it." };
                let _ = tx.send(Msg::Result { id: id.to_string(), ok: false, message: Some(why.into()) });
                self.end_incoming(id, "failed", Some(e.clone()));
                return Err(e);
            }
        };
        let started = {
            let mut inner = self.inner.lock().unwrap();
            match inner.incoming.get_mut(id) {
                Some(t) if t.state == "preparing" => {
                    t.targets = targets;
                    t.parts = parts;
                    t.handles = handles;
                    t.saved_to = Some(saved_to);
                    t.state = "receiving".into();
                    t.meter.start(t.received.clone(), t.cancel.clone(), &self.rt);
                    let watch = (t.received.clone(), t.cancel.clone(), t.resume, snapshot_in(t, id, true));
                    self.save_parts(&inner);
                    Some(watch)
                }
                // Cancelled meanwhile: what was just made goes.
                _ => {
                    drop(handles);
                    for p in &parts {
                        let _ = std::fs::remove_file(p);
                    }
                    if files.len() > 1 || files.first().is_some_and(|f| f.path.contains('/')) {
                        remove_empty_dirs(&saved_to);
                    }
                    None
                }
            }
        };
        let Some((received, cancel, resume, snap)) = started else { return Ok(()) };
        // Kept from the start, so a cut at any moment can continue.
        if let Some(snap) = snap {
            snap.save(&self.data_dir);
            self.keep_notes(id, cancel.clone());
        }
        self.watch(id, false, received, cancel);
        let _ = tx.send(Msg::Accept { id: id.to_string(), resume });
        (self.settle)("direct-offer", id, "Accepted");
        self.changed();
        Ok(())
    }

    /// Every second while files arrive: what was written is synced to the disk, then the
    /// note says so (see `Snapshot::save`).
    fn keep_notes(self: &Arc<Self>, id: &str, cancel: CancellationToken) {
        let me = self.clone();
        let id = id.to_string();
        self.rt.spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(NOTE_EVERY) => {}
                }
                let snap = me.inner.lock().unwrap().incoming.get(&id).and_then(|t| snapshot_in(t, &id, false));
                if let Some(snap) = snap {
                    let dir = me.data_dir.clone();
                    let _ = tokio::task::spawn_blocking(move || snap.save(&dir)).await;
                }
            }
        });
    }

    /// The connection dropped: a transfer coming in that both Pakos keep waits for them to
    /// connect again (what arrived is synced and noted). False if it can't (it then ends).
    fn pause_in(&self, id: &str) -> bool {
        let snap = {
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.incoming.get_mut(id) else { return true };
            if !t.resume {
                return false;
            }
            match t.state.as_str() {
                "receiving" => {}
                // Being checked and saved: it ends by itself (their Pako asks how it went).
                "checking" | "waiting" => return true,
                _ => return false,
            }
            t.cancel.cancel();
            t.cancel = CancellationToken::new();
            t.state = "waiting".into();
            t.finishing = false;
            t.since = now_ms();
            let snap = snapshot_in(t, id, true);
            // Closed while waiting (opened again when it continues).
            t.handles.clear();
            (snap, t.parts.clone())
        };
        let (snap, parts) = snap;
        let dir = self.data_dir.clone();
        self.rt.spawn_blocking(move || {
            if let Some(snap) = snap {
                snap.save(&dir);
            }
            // Out of sight while they wait (they get their name, shown, once finished).
            for p in &parts {
                set_hidden(p, true);
            }
        });
        self.changed();
        true
    }

    /// Connected again: transfers with them that stopped half-way continue. The receiver says
    /// what it has; the sender says it's still keeping its side (a Pako that dropped it answers).
    fn pick_up(&self, peer: &str) {
        let (outs, ins, tx) = {
            let inner = self.inner.lock().unwrap();
            let Some(tx) = inner.live.get(peer).map(|l| l.tx.clone()) else { return };
            let waiting = |c: &str, resume: bool, state: &str| c == peer && resume && state == "waiting";
            (
                inner.outgoing.iter().filter(|(_, t)| waiting(&t.contact, t.resume, &t.state)).map(|(k, _)| k.clone()).collect::<Vec<_>>(),
                inner.incoming.iter().filter(|(_, t)| waiting(&t.contact, t.resume, &t.state)).map(|(k, _)| k.clone()).collect::<Vec<_>>(),
                tx,
            )
        };
        for id in outs {
            let _ = tx.send(Msg::Resume { id });
        }
        for id in ins {
            self.send_have(&id, &tx);
        }
    }

    /// Tells the sender what's here of `id`, so it sends the rest.
    fn send_have(&self, id: &str, tx: &mpsc::UnboundedSender<Msg>) {
        let (full, part) = {
            let inner = self.inner.lock().unwrap();
            let Some(t) = inner.incoming.get(id) else { return };
            let sizes: Vec<u64> = t.files.iter().map(|f| f.size).collect();
            let got = t.got.lock().unwrap();
            resume::encode(&got.ranges, &sizes)
        };
        let m = Msg::Have { id: id.into(), full, part };
        // Very many files partly here: those come again in full rather than not at all.
        let m = match m {
            Msg::Have { id, full, .. } if serde_json::to_vec(&m).map_or(true, |b| b.len() > MAX_MSG) => Msg::Have { id, full, part: Vec::new() },
            m => m,
        };
        let _ = tx.send(m);
    }

    /// The sender asks about a transfer it keeps for us.
    fn on_resume(&self, peer: &str, tx: &mpsc::UnboundedSender<Msg>, id: &str) {
        let (mine, done) = {
            let inner = self.inner.lock().unwrap();
            (
                inner.incoming.get(id).is_some_and(|t| t.contact == peer),
                inner.history.iter().any(|h| h.id == id && h.contact == peer && !h.outgoing && h.result == "done"),
            )
        };
        if mine {
            // Its `Have` went out when connecting (or goes once it's checked).
            return;
        }
        if done {
            // It finished here while the answer couldn't reach them.
            let _ = tx.send(Msg::Result { id: id.to_string(), ok: true, message: None });
            return;
        }
        self.unknown_transfer(peer, tx, id);
    }

    /// They ask to continue a transfer that isn't (or no longer is) one of ours with them:
    /// they're told it's over, so they don't keep waiting. Says nothing about anyone else's.
    fn unknown_transfer(&self, peer: &str, tx: &mpsc::UnboundedSender<Msg>, id: &str) {
        if !proto::valid_id(id) || !small_ok(self.inner.lock().unwrap().small_times.entry(peer.to_string()).or_default(), std::time::Instant::now()) {
            return;
        }
        let _ = tx.send(Msg::Cancel { id: id.to_string() });
    }

    /// The sender is ready to continue: files that changed there start again at their new
    /// size (checked like an offer: real files of this transfer, sizes that fit the drive),
    /// the unfinished files are opened again, and the sender hears "go" (`Accept`).
    /// Blocking: call it off the async threads.
    fn continue_in(self: &Arc<Self>, peer: &str, id: &str, restart: Vec<Restart>) {
        let (files, parts) = {
            let inner = self.inner.lock().unwrap();
            let Some(t) = inner.incoming.get(id).filter(|t| t.contact == peer && t.resume && t.state == "waiting") else { return };
            (t.files.clone(), t.parts.clone())
        };
        let fail = |theirs: &str, here: String| {
            if let Some(tx) = self.inner.lock().unwrap().live.get(peer).map(|l| l.tx.clone()) {
                let _ = tx.send(Msg::Result { id: id.to_string(), ok: false, message: Some(theirs.into()) });
            }
            self.end_incoming(id, "failed", Some(here));
        };
        let odd = "Their Pako sent something this one doesn't understand, so it stopped.";
        let mut sizes: Vec<u64> = files.iter().map(|f| f.size).collect();
        let mut again = HashSet::new();
        for r in &restart {
            if r.file >= files.len() || !again.insert(r.file) {
                fail(odd, odd.into());
                return;
            }
            sizes[r.file] = r.size;
        }
        let Some(total) = sizes.iter().try_fold(0u64, |a, s| a.checked_add(*s)) else {
            fail(odd, odd.into());
            return;
        };
        // Never more than what was accepted, unless this contact may send without asking and an
        // offer of that size wouldn't have been asked about either (as `on_offer` decides):
        // otherwise a sender could grow a small accepted send into one that fills the drive.
        let before: u64 = files.iter().map(|f| f.size).sum();
        if total > before {
            let more = total - before;
            let auto = self.inner.lock().unwrap().contacts.iter().find(|c| c.id == peer).is_some_and(|c| c.auto_accept || devices::paired(c));
            if !auto {
                fail(
                    "Their Pako couldn't take it: the files changed on your PC and are now bigger than what they accepted. Send them again.",
                    format!("The files changed on their PC and are now {} bigger than what you accepted, so it stopped. They can send them again.", human(more)),
                );
                return;
            }
            let dir = parts.first().and_then(|p| p.parent()).map(Path::to_path_buf).unwrap_or_default();
            if let Some((free, size)) = crate::system::disk_space(&dir) {
                if more.saturating_add(margin_for(size)) > free || more > free / 2 {
                    fail(
                        "Their Pako couldn't save it: there isn't enough room on their drive.",
                        format!("{NO_ROOM}: the files changed on their PC and now need {} more, {} is free there.", human(more), human(free)),
                    );
                    return;
                }
            }
        }
        // The unfinished files, opened again (made again if one was deleted meanwhile). A file
        // that changed there, or isn't as it was left here, starts from zero.
        let mut handles = Vec::with_capacity(parts.len());
        let mut fresh = vec![false; parts.len()];
        for (i, p) in parts.iter().enumerate() {
            let opened = match std::fs::OpenOptions::new().read(true).write(true).open(p) {
                Ok(f) => Ok(f),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    fresh[i] = true;
                    std::fs::OpenOptions::new().create_new(true).read(true).write(true).open(p).inspect(|_| mark_received(p))
                }
                Err(e) => Err(e),
            };
            let sized = opened.and_then(|f| {
                let len = f.metadata()?.len();
                if again.contains(&i) || len != files[i].size {
                    fresh[i] = true;
                    f.set_len(0)?;
                }
                f.set_len(sizes[i])?;
                Ok(f)
            });
            match sized {
                Ok(f) => handles.push(Arc::new(f)),
                Err(e) => {
                    fail("Their Pako couldn't save it.", format!("Couldn't save the files ({e})."));
                    return;
                }
            }
        }
        let started = {
            let mut inner = self.inner.lock().unwrap();
            match inner.incoming.get_mut(id) {
                Some(t) if t.state == "waiting" => {
                    let have: u64 = {
                        let mut g = t.got.lock().unwrap();
                        for (i, f) in fresh.iter().enumerate() {
                            if *f {
                                g.ranges[i] = Ranges::default();
                            }
                        }
                        g.ranges.iter().map(|r| r.covered()).sum()
                    };
                    for (f, s) in t.files.iter_mut().zip(&sizes) {
                        f.size = *s;
                    }
                    t.total = total;
                    t.received.store(have, Ordering::Relaxed);
                    t.base = have;
                    t.handles = handles;
                    t.state = "receiving".into();
                    t.since = now_ms();
                    t.meter = Arc::new(Meter::default());
                    t.meter.start(t.received.clone(), t.cancel.clone(), &self.rt);
                    Some((t.received.clone(), t.cancel.clone(), snapshot_in(t, id, true)))
                }
                _ => None,
            }
        };
        let Some((received, cancel, snap)) = started else { return };
        // The note follows the new sizes before anything arrives.
        if let Some(snap) = snap {
            snap.save(&self.data_dir);
        }
        self.watch(id, false, received, cancel.clone());
        self.keep_notes(id, cancel);
        if let Some(tx) = self.inner.lock().unwrap().live.get(peer).map(|l| l.tx.clone()) {
            let _ = tx.send(Msg::Accept { id: id.to_string(), resume: true });
        }
        self.changed();
    }

    /// One data stream: checked against an accepted transfer from this contact, then written.
    async fn take_part(&self, peer: &str, mut stream: RecvStream) {
        let part: Part = match proto::read_frame(&mut stream, MAX_HEADER).await {
            Ok(Some(p)) => p,
            _ => return,
        };
        let target = {
            let inner = self.inner.lock().unwrap();
            inner.incoming.get(&part.id).and_then(|t| {
                // Also after "all sent": pieces still in flight land then.
                if t.contact != peer || t.state != "receiving" {
                    return None;
                }
                let f = t.files.get(part.file)?;
                let end = part.offset.checked_add(part.len)?;
                if end > f.size {
                    return None;
                }
                Some((t.handles.get(part.file)?.clone(), t.received.clone(), t.cancel.clone(), t.progress.clone(), t.got.clone()))
            })
        };
        let Some((file, received, cancel, progress, got)) = target else {
            let _ = stream.stop(1u32.into());
            return;
        };
        let mut buf = vec![0u8; BUF];
        let mut at = part.offset;
        let end = part.offset + part.len;
        while at < end {
            let want = ((end - at) as usize).min(BUF);
            let n = tokio::select! {
                _ = cancel.cancelled() => return,
                r = tokio::io::AsyncReadExt::read(&mut stream, &mut buf[..want]) => match r {
                    Ok(0) | Err(_) => return,
                    Ok(n) => n,
                },
            };
            // The disk write is off the async threads (a slow disk mustn't hold up the rest).
            let f = file.clone();
            let data = std::mem::take(&mut buf);
            let (data, wrote) = match tokio::task::spawn_blocking(move || {
                let r = write_at(&f, &data[..n], at);
                (data, r)
            })
            .await
            {
                Ok(r) => r,
                Err(_) => return,
            };
            buf = data;
            if let Err(e) = wrote {
                self.end_in(&part.id, "failed", Some(format!("Couldn't save the files ({e}).")), true);
                return;
            }
            // Noted as written (synced and saved within a second): only new bytes count.
            let added = got.lock().unwrap().wrote(part.file, at, n as u64);
            at += n as u64;
            received.fetch_add(added, Ordering::Relaxed);
            progress.notify_waiters();
        }
    }

    async fn finish_incoming(self: Arc<Self>, id: &str, hashes: Vec<String>, tx: mpsc::UnboundedSender<Msg>) {
        let (received, total, progress, cancel, contact) = {
            let inner = self.inner.lock().unwrap();
            let Some(t) = inner.incoming.get(id) else { return };
            (t.received.clone(), t.total, t.progress.clone(), t.cancel.clone(), t.contact.clone())
        };
        // Parts still in flight on other streams may land a moment after "done": waited for
        // as long as they keep coming, until nothing arrives for a while.
        let mut last = received.load(Ordering::Relaxed);
        let mut quiet_since = tokio::time::Instant::now();
        while received.load(Ordering::Relaxed) < total {
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = progress.notified() => {}
                _ = tokio::time::sleep(Duration::from_millis(250)) => {}
            }
            let now = received.load(Ordering::Relaxed);
            if now != last {
                last = now;
                quiet_since = tokio::time::Instant::now();
            } else if quiet_since.elapsed() >= LAST_PIECES_QUIET {
                break;
            }
        }
        let (parts, targets, handles, resume, retried, names) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.incoming.get_mut(id) else { return };
            if cancel.is_cancelled() {
                return;
            }
            t.state = "checking".into();
            (t.parts.clone(), t.targets.clone(), std::mem::take(&mut t.handles), t.resume, t.retried.clone(), t.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>())
        };
        drop(handles);
        self.changed();
        // Answers go on the connection as it is now (it may have been made again meanwhile).
        let reply = |m: Msg| {
            let now = self.inner.lock().unwrap().live.get(&contact).map(|l| l.tx.clone());
            let _ = now.unwrap_or_else(|| tx.clone()).send(m);
        };
        if received.load(Ordering::Relaxed) < total || hashes.len() != parts.len() {
            reply(Msg::Result { id: id.into(), ok: false, message: Some("Some parts never arrived.".into()) });
            self.end_incoming(id, "failed", Some("Some parts never arrived.".into()));
            return;
        }
        let check = {
            let parts = parts.clone();
            tokio::task::spawn_blocking(move || hash_files(&parts)).await
        };
        let Ok(Ok(sums)) = check else {
            reply(Msg::Result { id: id.into(), ok: false, message: Some("The files didn't arrive intact.".into()) });
            self.end_incoming(id, "failed", Some("The files didn't arrive intact.".into()));
            return;
        };
        let bad: Vec<usize> = (0..parts.len()).filter(|&i| sums[i] != hashes[i]).collect();
        if !bad.is_empty() {
            // Sent again in full, once, when both Pakos can; a second miss is an error.
            let twice = bad.iter().find(|&&i| retried[i]);
            if resume && twice.is_none() {
                self.retry_in(id, &bad);
                return;
            }
            let why = match twice {
                Some(&i) => format!("{} didn't arrive intact, even when sent again.", names[i]),
                None => "The files didn't arrive intact.".into(),
            };
            reply(Msg::Result { id: id.into(), ok: false, message: Some(why.clone()) });
            self.end_incoming(id, "failed", Some(why));
            return;
        }
        // Never replaces a file: if a name was taken meanwhile, the next free one is used.
        let saved_to = self.inner.lock().unwrap().incoming.get(id).and_then(|t| t.saved_to.clone());
        let mut moved_single = None;
        for (i, (p, t)) in parts.iter().zip(targets.iter()).enumerate() {
            match move_new(p, t) {
                Ok(at) => {
                    set_hidden(&at, false);
                    if saved_to.as_deref() == Some(t.as_path()) && at != *t {
                        moved_single = Some(at);
                    }
                    let mut inner = self.inner.lock().unwrap();
                    if let Some(x) = inner.incoming.get_mut(id) {
                        // This piece is a finished file now: not cleaned up any more.
                        x.parts.retain(|q| q != p);
                        if i + 1 == parts.len() {
                            self.save_parts(&inner);
                        }
                    }
                }
                Err(_) => {
                    reply(Msg::Result { id: id.into(), ok: false, message: Some("Couldn't save the files.".into()) });
                    self.end_incoming(id, "failed", Some("Couldn't save the files.".into()));
                    return;
                }
            }
        }
        if let Some(at) = moved_single {
            if let Some(t) = self.inner.lock().unwrap().incoming.get_mut(id) {
                t.saved_to = Some(at);
            }
        }
        reply(Msg::Result { id: id.into(), ok: true, message: None });
        self.end_incoming(id, "done", None);
    }

    /// Files that didn't match their hash start again from zero (once): the note forgets
    /// them, and the sender hears what's here without them, so it sends them again in full.
    fn retry_in(&self, id: &str, bad: &[usize]) {
        let (snap, contact) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.incoming.get_mut(id) else { return };
            if t.state != "checking" {
                return;
            }
            {
                let mut g = t.got.lock().unwrap();
                for &i in bad {
                    let had = g.ranges[i].covered();
                    g.ranges[i] = Ranges::default();
                    t.received.fetch_sub(had, Ordering::Relaxed);
                    t.retried[i] = true;
                }
            }
            t.state = "waiting".into();
            t.finishing = false;
            t.since = now_ms();
            t.cancel.cancel();
            t.cancel = CancellationToken::new();
            (snapshot_in(t, id, true), t.contact.clone())
        };
        // Saved first, so the note never says those files are here.
        if let Some(snap) = snap {
            snap.save(&self.data_dir);
        }
        let tx = self.inner.lock().unwrap().live.get(&contact).map(|l| l.tx.clone());
        if let Some(tx) = tx {
            self.send_have(id, &tx);
        }
        self.changed();
    }

    /// Transfers that waited 7 days for the other PC are cleared on this side (the unfinished
    /// files that arrived go); their cards say they expired.
    pub(super) fn expire(&self, now: u64) {
        let old = |state: &str, since: u64| state == "waiting" && since.saturating_add(resume::KEEP_MS) <= now;
        let (outs, ins): (Vec<String>, Vec<String>) = {
            let inner = self.inner.lock().unwrap();
            (
                inner.outgoing.iter().filter(|(_, t)| old(&t.state, t.since)).map(|(k, _)| k.clone()).collect(),
                inner.incoming.iter().filter(|(_, t)| old(&t.state, t.since)).map(|(k, _)| k.clone()).collect(),
            )
        };
        for id in outs {
            self.end_out(&id, "failed", Some(EXPIRED_OUT.into()), false);
        }
        for id in ins {
            self.end_in(&id, "failed", Some(EXPIRED_IN.into()), false);
        }
        self.relay_expire(now);
    }

    fn end_incoming(&self, id: &str, result: &str, message: Option<String>) {
        self.end_in(id, result, message, false);
    }

    /// Ends a receiving transfer; `tell`: send "cancel" to the other side.
    fn end_in(&self, id: &str, result: &str, message: Option<String>, tell: bool) {
        let t = self.inner.lock().unwrap().incoming.remove(id);
        let Some(t) = t else { return };
        if let Some(m) = message.as_deref().filter(|_| result == "failed") { crate::errorlog::note(crate::errorlog::Area::Direct, m); }
        t.cancel.cancel();
        if t.resume {
            // Never saved again after this.
            t.note.lock().unwrap().ended = true;
            resume::remove_in(&self.data_dir, id);
        }
        if tell {
            self.tell_cancel(&t.contact, id);
        }
        drop(t.handles);
        // Unfinished pieces go; finished files stay.
        for p in &t.parts {
            let _ = std::fs::remove_file(p);
        }
        if !t.parts.is_empty() {
            self.save_parts(&self.inner.lock().unwrap());
        }
        if result != "done" {
            // Folders made for this transfer that stayed empty go too.
            if let Some(root) = &t.saved_to {
                if root.is_dir() {
                    remove_empty_dirs(root);
                }
            }
        }
        let asked_only = t.state == "asking";
        let path = self.inner.lock().unwrap().live.get(&t.contact).and_then(|l| path_of(&l.conn));
        let (contact_name, page) = self
            .inner
            .lock()
            .unwrap()
            .contacts
            .iter()
            .find(|c| c.id == t.contact)
            .map(|c| (c.name.clone(), if c.device { "devices" } else { "direct" }))
            .unwrap_or(("".into(), "direct"));
        let (duration_ms, avg_speed, peak_speed) = t.meter.summary(t.received.load(Ordering::Relaxed).saturating_sub(t.base));
        // A relay's copy: never offered for anything else (no Open, no Folder).
        let relay_to = Self::relay_to_of(&self.inner.lock().unwrap(), id);
        self.record(HistoryItem {
            id: id.to_string(),
            contact: t.contact.clone(),
            outgoing: false,
            name: t.name.clone(),
            files: t.files.len() as u32,
            size: t.total,
            date: now_ms(),
            path,
            result: result.into(),
            message: message.clone(),
            saved_to: if result == "done" && relay_to.is_none() { t.saved_to.clone().map(|p| p.display().to_string()) } else { None },
            duration_ms,
            avg_speed,
            peak_speed,
            via: None,
            relay_to: relay_to.clone(),
        });
        if relay_to.is_some() {
            let got = if result == "done" { t.targets.iter().cloned().zip(t.files.iter().cloned()).collect() } else { Vec::new() };
            self.relay_leg1_ended(id, result, message, got);
            self.changed();
            return;
        }
        if result == "done" {
            self.tell(&t.contact, "direct-received", "Files received", &format!("{} from {} is saved.", t.name, contact_name), Go::item(page, t.contact.clone()));
        }
        // An offer still waiting for your answer (or one whose accepting failed) ends: its
        // pop-up leaves and its bell line says so. (Accepted ones were settled when accepted.)
        if asked_only || t.state == "preparing" {
            (self.settle)("direct-offer", id, if result == "declined" { "Declined" } else { "No longer waiting" });
        }
        self.changed();
    }
}

/// A transfer with them waits to continue (after a cut).
fn waits_for(inner: &Inner, id: &str) -> bool {
    inner.outgoing.values().any(|t| t.contact == id && t.state == "waiting") || inner.incoming.values().any(|t| t.contact == id && t.state == "waiting")
}

/// Called and kept first: one of My devices, an Often contact, a favourite, someone who can
/// send without asking (`pace::kept_first`), or used lately (`recent`).
fn wanted(c: &Contact, recent: &HashSet<&str>, now: u64) -> bool {
    pace::kept_first(c, now) || recent.contains(c.id.as_str())
}

/// Called by the round (in the background): see `pace::in_round`; a transfer waiting for them
/// is called whatever their setting.
fn in_round(inner: &Inner, c: &Contact, now: u64) -> bool {
    pace::in_round(c, now) || (!c.declined && waits_for(inner, &c.id))
}

/// Contacts a transfer with ended lately (kept connected first).
fn recent_use(history: &[HistoryItem], now: u64) -> HashSet<&str> {
    history.iter().filter(|h| h.date.saturating_add(pace::RECENT_USE_MS) >= now).map(|h| h.contact.as_str()).collect()
}

/// The parts (at most `PIECE` bytes each) carrying [start, end) of file `file`.
fn parts_for(queue: &mut VecDeque<Part>, id: &str, file: usize, start: u64, end: u64) {
    let mut off = start;
    while off < end {
        let len = PIECE.min(end - off);
        queue.push_back(Part { id: id.to_string(), file, offset: off, len });
        off += len;
    }
}

/// What the sending side keeps of a transfer, to continue it after a cut.
fn out_note(id: &str, t: &Outgoing) -> OutNote {
    OutNote {
        id: id.to_string(),
        contact: t.contact.clone(),
        name: t.name.clone(),
        files: t.files.iter().zip(&t.mtimes).map(|((p, f), m)| OutFile { path: p.clone(), file: f.clone(), mtime: *m }).collect(),
        temp: t.temp.clone(),
        done: t.sent.load(Ordering::Relaxed),
        since: t.since,
    }
}

/// A note to save for a transfer coming in: its number, the files to sync first (their place,
/// write count, and the open file if there is one), the note, and its saving states.
struct Snapshot {
    seq: u64,
    sync: Vec<(usize, u64, Option<Arc<std::fs::File>>)>,
    note: InNote,
    state: Arc<Mutex<NoteState>>,
    got: Arc<Mutex<Got>>,
}

/// None when nothing arrived since the last one (unless `always`), or when it isn't kept.
fn snapshot_in(t: &Incoming, id: &str, always: bool) -> Option<Snapshot> {
    if !t.resume {
        return None;
    }
    let mut g = t.got.lock().unwrap();
    if !g.any && !always {
        return None;
    }
    let (seq, have, sync) = g.snapshot();
    drop(g);
    let sync = sync.into_iter().map(|(i, w)| (i, w, t.handles.get(i).cloned())).collect();
    let note = InNote {
        id: id.to_string(),
        contact: t.contact.clone(),
        name: t.name.clone(),
        files: t.files.clone(),
        targets: t.targets.clone(),
        parts: t.parts.clone(),
        saved_to: t.saved_to.clone(),
        have,
        retried: t.retried.clone(),
        since: t.since,
    };
    Some(Snapshot { seq, sync, note, state: t.note.clone(), got: t.got.clone() })
}

impl Snapshot {
    /// Syncs the files written to the disk, then saves the note (so it never claims bytes that
    /// aren't there). An older snapshot never lands after a newer one, and nothing is saved once
    /// the transfer ended. A file no longer open here (between rounds) is opened to sync it.
    /// Blocking.
    fn save(&self, data_dir: &Path) {
        let mut s = self.state.lock().unwrap();
        if s.ended || self.seq < s.saved {
            return;
        }
        for (i, _, f) in &self.sync {
            let synced = match f {
                Some(f) => f.sync_data(),
                None => match self.note.parts.get(*i) {
                    // Gone meanwhile: nothing to sync (it starts again from zero when it continues).
                    Some(p) => match std::fs::OpenOptions::new().write(true).open(p) {
                        Ok(f) => f.sync_data(),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(e) => Err(e),
                    },
                    None => Ok(()),
                },
            };
            if synced.is_err() {
                return;
            }
        }
        if resume::save_in(data_dir, &self.note).is_ok() {
            s.saved = self.seq;
            let done: Vec<(usize, u64)> = self.sync.iter().map(|(i, w, _)| (*i, *w)).collect();
            self.got.lock().unwrap().saved(&done);
        }
    }
}

// ---------- helpers ----------

async fn write_loop(mut send: SendStream, mut rx: mpsc::UnboundedReceiver<Msg>) {
    while let Some(m) = rx.recv().await {
        match proto::write_frame(&mut send, &m, MAX_MSG).await {
            Ok(()) => {}
            // Too big to send: that one message is dropped, the connection stays.
            Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {}
            Err(_) => break,
        }
    }
    let _ = send.finish();
}

/// `pace`: a pause (ms) after each block, for tests (0 = none).
async fn send_part(conn: &Connection, path: &Path, part: &Part, sent: &AtomicU64, pace: u64) -> Result<(), String> {
    let mut stream = conn.open_uni().await.map_err(|e| e.to_string())?;
    proto::write_frame(&mut stream, part, MAX_HEADER).await.map_err(|e| e.to_string())?;
    let file = Arc::new(std::fs::File::open(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?);
    let mut buf = vec![0u8; BUF];
    let mut at = part.offset;
    let end = part.offset + part.len;
    while at < end {
        let want = ((end - at) as usize).min(BUF);
        // The disk read is off the async threads.
        let f = file.clone();
        let mut data = std::mem::take(&mut buf);
        let (data, n) = tokio::task::spawn_blocking(move || {
            let r = read_at(&f, &mut data[..want], at);
            (data, r)
        })
        .await
        .map_err(|e| e.to_string())?;
        buf = data;
        let n = n.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err(format!("{} got shorter while sending.", path.display()));
        }
        tokio::io::AsyncWriteExt::write_all(&mut stream, &buf[..n]).await.map_err(|e| e.to_string())?;
        at += n as u64;
        sent.fetch_add(n as u64, Ordering::Relaxed);
        if pace > 0 {
            tokio::time::sleep(Duration::from_millis(pace)).await;
        }
    }
    stream.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn read_at(f: &std::fs::File, buf: &mut [u8], at: u64) -> std::io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(f, buf, at)
}
#[cfg(not(windows))]
fn read_at(f: &std::fs::File, buf: &mut [u8], at: u64) -> std::io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(f, buf, at)
}

#[cfg(windows)]
fn write_at(f: &std::fs::File, mut buf: &[u8], mut at: u64) -> std::io::Result<()> {
    while !buf.is_empty() {
        let n = std::os::windows::fs::FileExt::seek_write(f, buf, at)?;
        buf = &buf[n..];
        at += n as u64;
    }
    Ok(())
}
#[cfg(not(windows))]
fn write_at(f: &std::fs::File, buf: &[u8], at: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::write_all_at(f, buf, at)
}

fn hash_files(paths: &[PathBuf]) -> Result<Vec<String>, String> {
    paths
        .iter()
        .map(|p| {
            let mut h = blake3::Hasher::new();
            h.update_reader(std::fs::File::open(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            Ok(h.finalize().to_hex().to_string())
        })
        .collect()
}

/// Picked files and folders as (where it is, how it's offered: "Folder/sub/file.txt"). Two
/// picked items with the same name (from different folders) both go: the second is offered as
/// "name (2)".
pub fn list_files(paths: &[PathBuf]) -> Result<Vec<(PathBuf, OfferFile)>, String> {
    let mut out = Vec::new();
    let mut tops = HashSet::new();
    for p in paths {
        let base = p.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("A picked item has no name.")?;
        let name = free_name(&mut tops, &base, p.is_dir());
        if p.is_dir() {
            walk(p, &name, &mut out)?;
        } else if p.is_file() {
            let size = std::fs::metadata(p).map_err(|e| e.to_string())?.len();
            out.push((p.clone(), OfferFile { path: name, size }));
        }
        if out.len() > proto::MAX_FILES {
            return Err("That's too many files for one send.".into());
        }
    }
    if out.is_empty() {
        return Err("There are no files in what was picked.".into());
    }
    Ok(out)
}

/// `name`, or "name (2)", "name (3)"… if already in `taken` (ignoring case, like Windows).
fn free_name(taken: &mut HashSet<String>, name: &str, folder: bool) -> String {
    if taken.insert(name.to_uppercase()) {
        return name.to_string();
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !folder && !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    let mut i = 2;
    loop {
        let n = format!("{stem} ({i}){ext}");
        if taken.insert(n.to_uppercase()) {
            return n;
        }
        i += 1;
    }
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<(PathBuf, OfferFile)>) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let name = format!("{prefix}/{}", e.file_name().to_string_lossy());
        let ft = e.file_type().map_err(|e| e.to_string())?;
        if ft.is_dir() {
            walk(&p, &name, out)?;
        } else if ft.is_file() {
            let size = e.metadata().map_err(|e| e.to_string())?.len();
            out.push((p, OfferFile { path: name, size }));
        }
        if out.len() > proto::MAX_FILES {
            return Err("That's too many files for one send.".into());
        }
    }
    Ok(())
}

fn default_title(list: &[(PathBuf, OfferFile)]) -> String {
    let tops: std::collections::BTreeSet<&str> = list.iter().map(|(_, f)| f.path.split('/').next().unwrap_or("")).collect();
    if tops.len() == 1 {
        tops.into_iter().next().unwrap_or("files").to_string()
    } else {
        format!("{} files", list.len())
    }
}

/// A person's name as shown: no control or hidden characters (which could make one name look
/// like another), at most 60 characters.
/// A contact's names after they said what they're called: (your name for them, theirs), or None
/// when nothing changes. Your own name for them stays unless it was theirs or the placeholder.
fn updated_names(name: &str, their_name: Option<&str>, said: &str) -> Option<(String, String)> {
    let theirs = clean_name(said)?;
    // "Someone" stored as their own name by Pako 1.0.0/1.0.1 when they had none: not a name.
    let before = their_name.filter(|t| *t != PLACEHOLDER);
    if before == Some(theirs.as_str()) && name != PLACEHOLDER {
        return None;
    }
    let follows = name == PLACEHOLDER || their_name == Some(name);
    Some((if follows { theirs.clone() } else { name.to_string() }, theirs))
}

/// A device's names after it said its PC's name: (your name for it, its PC's name), or None
/// when nothing changes. Your name for it follows unless you renamed it here (it was neither
/// what it said before, its owner's name, nor the placeholder): like `updated_names`.
fn updated_pc_name(name: &str, their_name: Option<&str>, their_pc: Option<&str>, said: &str) -> Option<(String, String)> {
    let pc = clean_name(said)?;
    if their_pc == Some(pc.as_str()) && name != PLACEHOLDER {
        return None;
    }
    let before = their_pc.or(their_name);
    let follows = name == PLACEHOLDER || before == Some(name);
    Some((if follows { pc.clone() } else { name.to_string() }, pc))
}

/// What a contact who gave no name is called until they do.
const PLACEHOLDER: &str = "Someone";

fn clean_name(name: &str) -> Option<String> {
    let n: String = name.chars().filter(|c| !c.is_control() && !proto::is_hidden(*c)).collect::<String>().trim().chars().take(60).collect();
    let n = n.trim().to_string();
    if n.is_empty() { None } else { Some(n) }
}

/// The other side's reason, as shown and kept in the history: on one line, without hidden
/// characters, at most 300 characters (their Pako can send any text). None = nothing left.
fn peer_message(m: Option<String>) -> Option<String> {
    let m = m?;
    let one_line: String = m.chars().filter(|c| !proto::is_hidden(*c)).map(|c| if c.is_control() { ' ' } else { c }).collect();
    let words = one_line.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut s: String = words.chars().take(MAX_PEER_MESSAGE).collect();
    if s.len() < words.len() {
        s.push('…');
    }
    (!s.is_empty()).then_some(s)
}

/// The name is taken: by a file or folder, a file arriving through Direct, or a download's
/// unfinished file (".pakopart", or ".zdpart" from an older Pako).
fn taken(p: &Path) -> bool {
    let download_part = || match (p.parent(), p.file_name()) {
        (Some(dir), Some(n)) => crate::names::part_exists(dir, &n.to_string_lossy()),
        _ => false,
    };
    p.exists() || part_of(p).exists() || download_part()
}

/// A name in `dir` that isn't taken yet: "name", "name (2)"…
fn unique(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !taken(&first) {
        return first;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    // "a (2)" taken too: "a (3)", not "a (2) (2)".
    let stem = match stem.strip_suffix(')').and_then(|s| s.rsplit_once(" (")) {
        Some((base, n)) if !base.is_empty() && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => base.to_string(),
        _ => stem,
    };
    for i in 2..10_000 {
        let p = dir.join(format!("{stem} ({i}){ext}"));
        if !taken(&p) {
            return p;
        }
    }
    dir.join(format!("{stem} ({}){ext}", now_ms()))
}

/// The file `p` is written to while it arrives ("name.pakorecv": never a download's ending).
fn part_of(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".");
    s.push(crate::names::RECV_EXT);
    PathBuf::from(s)
}

/// What Pako 0.1.2's Direct wrote arriving files to: today a download's ending.
const OLD_RECV_EXT: &str = ".pakopart";

/// A recorded unfinished file of a transfer cut short that start-up may delete: today's
/// ".pakorecv", or one Pako 0.1.2 left as ".pakopart" when it still carries the "from another
/// computer" mark Direct puts on before a byte lands. A download's unfinished file shares that
/// ending now but never has such a mark (a download gets its own, with the web address, only
/// once finished), so it's never deleted here.
fn is_own_leftover(p: &Path) -> bool {
    let name = p.to_string_lossy();
    if name.ends_with(&format!(".{}", crate::names::RECV_EXT)) {
        return true;
    }
    if !name.ends_with(OLD_RECV_EXT) {
        return false;
    }
    let mut stream = p.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    std::fs::read_to_string(PathBuf::from(stream)).is_ok_and(|z| z.contains("ZoneId=3") && !z.contains("HostUrl="))
}

type Prepared = (Vec<PathBuf>, Vec<PathBuf>, Vec<Arc<std::fs::File>>, PathBuf);

/// How `prepare_targets` starts its "the drive is too full" reason.
const NO_ROOM: &str = "Not enough room";

/// What `prepare_targets` made so far (removed again if it fails half-way).
#[derive(Default)]
struct Made {
    parts: Vec<PathBuf>,
    root: Option<PathBuf>,
}

/// Lays out where an accepted offer goes: one file straight in the folder, several in a
/// sub-folder named after the send. Checks the room, makes the ".pakorecv" files. Nothing is
/// left behind if it fails half-way.
fn prepare_targets(base: &Path, name: &str, files: &[OfferFile], total: u64) -> Result<Prepared, String> {
    std::fs::create_dir_all(base).map_err(|e| format!("Couldn't make the folder {}: {e}", base.display()))?;
    if let Some((free, size)) = crate::system::disk_space(base) {
        let margin = margin_for(size);
        if free < total.saturating_add(margin) {
            return Err(format!(
                "{NO_ROOM}: this needs {}, {} is free there (Pako always leaves {} free).",
                human(total),
                human(free),
                human(margin)
            ));
        }
    }
    let mut made = Made::default();
    let r = lay_out(base, name, files, &mut made);
    if r.is_err() {
        for p in &made.parts {
            let _ = std::fs::remove_file(p);
        }
        if let Some(root) = &made.root {
            remove_empty_dirs(root);
        }
    }
    r
}

fn lay_out(base: &Path, name: &str, files: &[OfferFile], made: &mut Made) -> Result<Prepared, String> {
    let single = files.len() == 1 && !files[0].path.contains('/');
    let root = if single {
        base.to_path_buf()
    } else {
        let r = claim_dir(base, &proto::safe_part(name).unwrap_or_else(|| "files".into()))?;
        made.root = Some(r.clone());
        r
    };
    let mut targets = Vec::new();
    let mut parts = Vec::new();
    let mut handles = Vec::new();
    for f in files {
        let target = if single {
            base.join(proto::safe_part(&f.path).ok_or("A file name isn't safe.")?)
        } else {
            let rel = proto::safe_rel(&f.path).ok_or("A file name isn't safe.")?;
            let mut p = root.clone();
            for r in &rel {
                p.push(r);
            }
            p
        };
        // Never outside the chosen folder.
        if !target.starts_with(base) {
            return Err("A file name isn't safe.".into());
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let (target, part, file) = claim_file(&target)?;
        made.parts.push(part.clone());
        // Marked as from another computer before a byte lands, so the finished file is too.
        mark_received(&part);
        file.set_len(f.size).map_err(|e| e.to_string())?;
        targets.push(target);
        parts.push(part);
        handles.push(Arc::new(file));
    }
    let saved = if single { targets[0].clone() } else { root };
    Ok((targets, parts, handles, saved))
}

/// Makes a new folder `name` in `base` ("name (2)" if taken). Never reuses an existing folder.
fn claim_dir(base: &Path, name: &str) -> Result<PathBuf, String> {
    let mut d = base.join(name);
    for _ in 0..100 {
        match std::fs::create_dir(&d) {
            Ok(()) => return Ok(d),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => d = unique(base, name),
            Err(e) => return Err(format!("Couldn't make the folder {}: {e}", d.display())),
        }
    }
    Err("Couldn't find a free folder name to save to.".into())
}

/// Makes the ".pakorecv" file for `target`, never reusing an existing file: if `target` or its
/// part is taken, the next free name ("name (2)") is used. (target, part, open part).
fn claim_file(target: &Path) -> Result<(PathBuf, PathBuf, std::fs::File), String> {
    let dir = target.parent().ok_or("A file name isn't safe.")?;
    let name = target.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("A file name isn't safe.")?;
    let mut t = target.to_path_buf();
    for _ in 0..100 {
        if !t.exists() {
            let part = part_of(&t);
            match std::fs::OpenOptions::new().create_new(true).read(true).write(true).open(&part) {
                Ok(f) => return Ok((t, part, f)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        t = unique(dir, &name);
    }
    Err("Couldn't find a free name to save to.".into())
}

/// Moves a finished piece to its name without ever replacing a file there: if the name was
/// taken meanwhile, the next free one ("name (2)") is used. Returns where it went.
fn move_new(from: &Path, to: &Path) -> std::io::Result<PathBuf> {
    let dir = to.parent().ok_or_else(|| std::io::Error::other("no folder"))?;
    let name = to.file_name().map(|n| n.to_string_lossy().to_string()).ok_or_else(|| std::io::Error::other("no name"))?;
    let mut t = to.to_path_buf();
    for _ in 0..100 {
        match rename_new(from, &t) {
            Ok(()) => return Ok(t),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => t = unique(dir, &name),
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists))
}

/// Renames, failing (AlreadyExists) if `to` exists. (`std::fs::rename` replaces on Windows.)
#[cfg(windows)]
fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let wide = |p: &Path| long_path(p).as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (a, b) = (wide(from), wide(to));
    // SAFETY: two valid NUL-terminated paths; no flags = fail if `to` exists.
    let ok = unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), 0) };
    if ok != 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}
/// `p` in the form Windows takes past 260 characters ("\\?\C:\..." or "\\?\UNC\server\..."):
/// a folder sent with deep sub-folders often lands that far down. (Rust's own file functions do
/// this by themselves; a direct Windows call doesn't.)
#[cfg(windows)]
fn long_path(p: &Path) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let Ok(full) = std::path::absolute(p) else { return p.to_path_buf() };
    let w: Vec<u16> = full.as_os_str().encode_wide().collect();
    let utf16 = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
    let out: Vec<u16> = if w.starts_with(&utf16(r"\\?\")) || w.starts_with(&utf16(r"\\.\")) {
        return full;
    } else if w.starts_with(&utf16(r"\\")) {
        utf16(r"\\?\UNC\").into_iter().chain(w[2..].iter().copied()).collect()
    } else {
        utf16(r"\\?\").into_iter().chain(w).collect()
    };
    PathBuf::from(std::ffi::OsString::from_wide(&out))
}

#[cfg(not(windows))]
fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
    }
    std::fs::rename(from, to)
}

/// Marks a received file as coming from another computer (Windows' "Mark of the Web"), like a
/// browser download: Windows then warns before running a program, and Office opens documents in
/// Protected View. Not possible on some drives (FAT32, exFAT): skipped there.
fn mark_received(path: &Path) {
    let mut stream = path.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    let _ = std::fs::write(PathBuf::from(stream), "[ZoneTransfer]\r\nZoneId=3\r\n");
}

/// Hides (or shows again) a file in Explorer: an unfinished file waiting to continue.
#[cfg(windows)]
fn set_hidden(p: &Path, on: bool) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, INVALID_FILE_ATTRIBUTES};
    let w: Vec<u16> = long_path(p).as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    // SAFETY: a valid NUL-terminated path.
    unsafe {
        let a = GetFileAttributesW(w.as_ptr());
        if a == INVALID_FILE_ATTRIBUTES || (a & FILE_ATTRIBUTE_HIDDEN != 0) == on {
            return;
        }
        SetFileAttributesW(w.as_ptr(), if on { a | FILE_ATTRIBUTE_HIDDEN } else { a & !FILE_ATTRIBUTE_HIDDEN });
    }
}
#[cfg(not(windows))]
fn set_hidden(_: &Path, _: bool) {}

fn remove_empty_dirs(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                remove_empty_dirs(&e.path());
            }
        }
    }
    let _ = std::fs::remove_dir(dir);
}

#[cfg(test)]
mod tests {
    #[test]
    fn speeds_are_bytes_per_second() {
        assert_eq!(super::speed_of(10_000_000, 2_000), 5_000_000);
        assert_eq!(super::speed_of(123, 0), 0);
        let m = super::Meter::default();
        assert_eq!(m.summary(1_000), (0, 0, 0), "never started: nothing measured");
    }

    use super::*;

    #[test]
    fn a_contacts_new_name_reaches_you() {
        let u = |n: &str, t: Option<&str>, said: &str| updated_names(n, t, said);
        // Your own name for them stays; theirs is noted ("calls themselves …").
        assert_eq!(u("Anthony-3D", Some("Someone"), "Anthony"), Some(("Anthony-3D".into(), "Anthony".into())));
        assert_eq!(u("Mum", Some("Nina"), "Nina R."), Some(("Mum".into(), "Nina R.".into())));
        // A name that was only theirs, or the placeholder, follows theirs.
        assert_eq!(u("Rado", Some("Rado"), "Rado R."), Some(("Rado R.".into(), "Rado R.".into())));
        assert_eq!(u("Someone", Some("Someone"), "Bob"), Some(("Bob".into(), "Bob".into())));
        assert_eq!(u("Someone", None, "Bob"), Some(("Bob".into(), "Bob".into())));
        // Nothing new, or no name given: nothing changes.
        assert_eq!(u("Rado", Some("Rado"), "Rado"), None);
        assert_eq!(u("Someone", None, "  "), None);
        assert_eq!(u("Mum", Some("Nina"), ""), None);
    }

    #[test]
    fn the_name_message_is_understood_and_old_pakos_skip_it() {
        let m = Msg::Name { name: "Anthony".into() };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Msg>(&json).unwrap(), m);
        assert_eq!(serde_json::from_str::<Msg>(r#"{"t":"somethingNew","x":1}"#).unwrap(), Msg::Unknown);
    }
    use iroh::endpoint::presets;

    pub(super) async fn node(dir: &Path, name: &str) -> Arc<Node> {
        keyed_node(dir, name, iroh::SecretKey::generate()).await
    }

    /// A node with this key: the same Pako again after a restart.
    async fn keyed_node(dir: &Path, name: &str, key: iroh::SecretKey) -> Arc<Node> {
        told_node(dir, name, key, Arc::new(|_, _, _, _| {})).await
    }

    /// A node whose notices (pop-up, chime, bell line) go to `notify`.
    async fn told_node(dir: &Path, name: &str, key: iroh::SecretKey, notify: Notify2) -> Arc<Node> {
        let ep = Endpoint::builder(presets::Minimal)
            .secret_key(key)
            .relay_mode(iroh::RelayMode::Disabled)
            .alpns(vec![ALPN.to_vec()])
            .bind_addr("127.0.0.1:0")
            .unwrap()
            .bind()
            .await
            .unwrap();
        std::fs::create_dir_all(dir).unwrap();
        let downloads = dir.join("Downloads");
        let n = Node::new(ep, dir.to_path_buf(), name.into(), Box::new(move || downloads.clone()), Arc::new(|| {}), notify, Arc::new(|_, _, _| {}));
        tokio::spawn(n.clone().accept_loop());
        n
    }

    pub(super) fn addr(n: &Node) -> EndpointAddr {
        let sock = n.ep.bound_sockets().into_iter().find(|s| s.is_ipv4()).unwrap();
        EndpointAddr::new(n.ep.id()).with_ip_addr(sock)
    }

    pub(super) async fn until(mut f: impl FnMut() -> bool) {
        for _ in 0..400 {
            if f() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("timed out");
    }

    #[test]
    fn received_files_never_replace_and_nothing_is_left_on_failure() {
        let base = std::env::temp_dir().join(format!("pako-direct-prep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("a.txt"), b"mine").unwrap();
        // One file: an existing one with that name stays, the new one gets "(2)".
        let (targets, parts, handles, saved) = prepare_targets(&base, "x", &[OfferFile { path: "a.txt".into(), size: 3 }], 3).unwrap();
        assert_eq!(targets[0], base.join("a (2).txt"));
        assert_eq!(saved, targets[0]);
        drop(handles);
        let zone = std::fs::read_to_string(format!("{}:Zone.Identifier", parts[0].display())).unwrap_or_default();
        assert!(zone.contains("ZoneId=3"), "{zone:?}");
        // Something takes the name meanwhile: the move picks the next free one, nothing is replaced.
        std::fs::write(&targets[0], b"other").unwrap();
        let at = move_new(&parts[0], &targets[0]).unwrap();
        assert_eq!(at, base.join("a (3).txt"));
        assert_eq!(std::fs::read(base.join("a.txt")).unwrap(), b"mine");
        assert_eq!(std::fs::read(&targets[0]).unwrap(), b"other");
        // Fails half-way (an unsafe name after a good one): nothing stays behind.
        let bad = [OfferFile { path: "F/ok.txt".into(), size: 1 }, OfferFile { path: "F/../x".into(), size: 1 }];
        assert!(prepare_targets(&base, "Send", &bad, 2).is_err());
        assert!(!base.join("Send").exists());
        // A folder send never reuses an existing folder.
        std::fs::create_dir_all(base.join("Send")).unwrap();
        let (_, _, h, saved) = prepare_targets(&base, "Send", &[OfferFile { path: "F/ok.txt".into(), size: 1 }], 1).unwrap();
        assert_eq!(saved, base.join("Send (2)"));
        drop(h);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A folder with deep sub-folders lands past Windows' old 260-character limit: the finished
    /// file still gets its name (it used to fail at the very end, and everything was deleted).
    #[test]
    fn deep_folders_past_260_characters_finish() {
        let base = std::env::temp_dir().join(format!("pako-direct-deep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let mut deep = base.clone();
        for i in 0..6 {
            deep.push(format!("{i} a rather long folder name, as people give them sometimes"));
        }
        assert!(deep.as_os_str().len() > 300);
        std::fs::create_dir_all(&deep).unwrap();
        let (target, part, file) = claim_file(&deep.join("photo.jpg")).unwrap();
        drop(file);
        std::fs::write(&part, b"abc").unwrap();
        let at = move_new(&part, &target).unwrap();
        assert_eq!(at, target);
        assert_eq!(std::fs::read(&target).unwrap(), b"abc");
        // Never replaces there either.
        let (t2, p2, f2) = claim_file(&deep.join("photo.jpg")).unwrap();
        drop(f2);
        assert_eq!(t2, deep.join("photo (2).jpg"));
        std::fs::write(&p2, b"new").unwrap();
        assert_eq!(rename_new(&p2, &target).unwrap_err().kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(move_new(&p2, &t2).unwrap(), t2);
        assert_eq!(std::fs::read(&target).unwrap(), b"abc");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Direct's unfinished files have their own ending; a download's unfinished file of the
    /// same name is never taken over, and start-up deletes only Direct's own leftovers (Pako
    /// 0.1.2's ".pakopart" ones only with Direct's mark).
    #[test]
    fn direct_never_touches_a_downloads_file() {
        let base = std::env::temp_dir().join(format!("pako-direct-ending-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        assert_eq!(part_of(&base.join("a.zip")), base.join("a.zip.pakorecv"));
        // A download is busy with "b.zip": Direct receives into "b (2).zip".
        std::fs::write(base.join("b.zip.pakopart"), b"download").unwrap();
        std::fs::write(base.join("c.zip.zdpart"), b"download").unwrap();
        assert_eq!(unique(&base, "b.zip"), base.join("b (2).zip"));
        assert_eq!(unique(&base, "c.zip"), base.join("c (2).zip"));
        // Leftovers: today's ending always; 0.1.2's only with Direct's mark.
        assert!(is_own_leftover(&base.join("x.zip.pakorecv")));
        assert!(!is_own_leftover(&base.join("b.zip.pakopart")));
        let old = base.join("d.zip.pakopart");
        std::fs::write(&old, b"direct 0.1.2").unwrap();
        mark_received(&old);
        assert!(is_own_leftover(&old));
        assert!(!is_own_leftover(&base.join("d.zip")));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn same_named_picks_both_go() {
        let root = std::env::temp_dir().join(format!("pako-direct-pick-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (d, f) in [("one/Photos", "a.jpg"), ("two/Photos", "b.jpg")] {
            std::fs::create_dir_all(root.join(d)).unwrap();
            std::fs::write(root.join(d).join(f), b"x").unwrap();
        }
        std::fs::write(root.join("one/x.txt"), b"1").unwrap();
        std::fs::write(root.join("two/X.txt"), b"2").unwrap();
        let list = list_files(&[root.join("one/Photos"), root.join("two/Photos"), root.join("one/x.txt"), root.join("two/X.txt")]).unwrap();
        let names: Vec<&str> = list.iter().map(|(_, f)| f.path.as_str()).collect();
        assert_eq!(names, ["Photos/a.jpg", "Photos (2)/b.jpg", "x.txt", "X (2).txt"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn names_shown_lose_hidden_characters() {
        assert_eq!(clean_name("  Ra\u{202E}do\u{200B} "), Some("Rado".into()));
        assert_eq!(clean_name("\u{2066}\u{2069}"), None);
        // Their Pako's "couldn't take it": one line, no hidden characters, not endless.
        assert_eq!(peer_message(Some("Not enough\nroom\u{202E} here.".into())).as_deref(), Some("Not enough room here."));
        assert_eq!(peer_message(Some(" \u{200B} ".into())), None);
        assert_eq!(peer_message(None), None);
        let long = peer_message(Some("x".repeat(5_000_000))).unwrap();
        assert_eq!(long.chars().count(), MAX_PEER_MESSAGE + 1);
        assert_eq!(margin_for(10 * 1024 * 1024 * 1024), DISK_MARGIN_MIN);
        assert_eq!(margin_for(2_000_000_000_000), 100_000_000_000);
    }

    /// A contact's Pako restarted (or lost the connection) and connects again while this side
    /// still holds the old, dead connection: the new one is kept at once. It used to be turned
    /// away as a duplicate, and the two showed each other offline for up to three minutes.
    #[test]
    fn a_contact_connecting_again_takes_the_place_of_a_dead_connection() {
        let (a, b) = ("aaa", "bbb");
        let (fresh, old) = (Duration::from_secs(2), Duration::from_secs(300));
        // Both dialled at the same moment: the one opened by the smaller ID stays, on both sides.
        assert!(replaces(b, fresh, a));
        assert!(!replaces(a, fresh, b));
        // The same side dials again: it lost its first one.
        assert!(replaces(a, fresh, a) && replaces(b, fresh, b));
        // A connection kept for a while, and they connect again: theirs is gone.
        assert!(replaces(a, old, b) && replaces(b, old, a));
    }

    #[test]
    fn a_flood_of_offers_is_turned_away() {
        let start = std::time::Instant::now();
        let mut times = VecDeque::new();
        for i in 0..MAX_OFFERS_PER_MINUTE {
            assert_eq!(offer_flood(&mut times, start + Duration::from_millis(i as u64)), Flood::Fine);
        }
        for _ in 0..MAX_OFFERS_PER_MINUTE {
            assert_eq!(offer_flood(&mut times, start + Duration::from_secs(1)), Flood::TurnAway, "told why");
        }
        for _ in 0..1000 {
            assert_eq!(offer_flood(&mut times, start + Duration::from_secs(2)), Flood::Ignore, "no answer queued");
        }
        assert_eq!(times.len(), MAX_OFFERS_PER_MINUTE * 2, "nothing piles up");
        // A minute later they can send again.
        assert_eq!(offer_flood(&mut times, start + Duration::from_secs(63)), Flood::Fine);
    }

    /// Someone making new keys to ask "… wants to connect" again and again: after a few
    /// questions in ten minutes the next ones aren't asked; later they are again.
    #[test]
    fn a_flood_of_connection_questions_is_turned_away() {
        let mut times = VecDeque::new();
        let t0 = std::time::Instant::now();
        for i in 0..MAX_NEW_REQUESTS {
            assert!(request_ok(&mut times, t0 + Duration::from_secs(i as u64)), "{i}");
        }
        assert!(!request_ok(&mut times, t0 + Duration::from_secs(60)));
        assert!(!request_ok(&mut times, t0 + Duration::from_secs(599)));
        assert_eq!(times.len(), MAX_NEW_REQUESTS, "the ones turned away aren't counted");
        assert!(request_ok(&mut times, t0 + NEW_REQUESTS_WINDOW), "the first one is ten minutes old");
    }

    /// A contact's Pako sending name changes or "my device" without end: past a few a minute
    /// they're ignored (each one rewrote contacts.json, and could show a notification).
    #[test]
    fn a_flood_of_names_and_device_messages_is_ignored() {
        let start = std::time::Instant::now();
        let mut times = VecDeque::new();
        for i in 0..MAX_SMALL_PER_MINUTE {
            assert!(small_ok(&mut times, start + Duration::from_millis(i as u64)));
        }
        for _ in 0..1000 {
            assert!(!small_ok(&mut times, start + Duration::from_secs(1)));
        }
        assert_eq!(times.len(), MAX_SMALL_PER_MINUTE, "nothing piles up");
        assert!(small_ok(&mut times, start + Duration::from_secs(61)), "a minute later it works again");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn contacts_connect_send_and_strangers_are_refused() {
        let root = std::env::temp_dir().join(format!("pako-direct-it-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let a = node(&root.join("a"), "Alice").await;
        let b = node(&root.join("b"), "Bob").await;
        a.add_hint(addr(&b));
        b.add_hint(addr(&a));

        // Alice adds Bob: Bob is asked once.
        a.add_contact(&b.my_id(), "Bob").unwrap();
        until(|| !b.state().requests.is_empty()).await;
        assert_eq!(b.state().requests[0].code, a.state().contacts[0].code, "same safety code on both sides");
        b.answer_request(&a.my_id(), true);
        until(|| a.state().contacts[0].online && b.state().contacts.iter().any(|c| c.online)).await;
        assert!(b.state().contacts[0].contact.trusted);

        // Bob lets Alice send without asking.
        b.edit_contact(&a.my_id(), None, Some(true), None).unwrap();

        // A few files and a folder (one bigger than a piece, so several streams carry it).
        let src = root.join("src");
        std::fs::create_dir_all(src.join("Folder/sub")).unwrap();
        let big: Vec<u8> = (0..(PIECE as usize * 2 + 12345)).map(|i| (i % 251) as u8).collect();
        std::fs::write(src.join("big.bin"), &big).unwrap();
        std::fs::write(src.join("Folder/a.txt"), b"hello").unwrap();
        std::fs::write(src.join("Folder/sub/b.txt"), b"").unwrap();
        a.send(&b.my_id(), vec![src.join("big.bin"), src.join("Folder")], false, Some("Test send".into())).await.unwrap();
        until(|| a.state().history.iter().any(|h| h.result == "done") && b.state().history.iter().any(|h| h.result == "done")).await;
        let got = root.join("b/Downloads/From Alice/Test send");
        assert_eq!(std::fs::read(got.join("big.bin")).unwrap(), big);
        assert_eq!(std::fs::read(got.join("Folder/a.txt")).unwrap(), b"hello");
        assert_eq!(std::fs::read(got.join("Folder/sub/b.txt")).unwrap(), b"");
        assert!(!std::fs::read_dir(&got).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().ends_with(".pakorecv")));
        // Marked as coming from another computer, like a browser download.
        let zone = std::fs::read_to_string(format!("{}:Zone.Identifier", got.join("big.bin").display())).unwrap_or_default();
        assert!(zone.contains("ZoneId=3"), "{zone:?}");

        // A stranger is asked about, and declined: nothing from them is ever written.
        let c = node(&root.join("c"), "Mallory").await;
        c.add_hint(addr(&b));
        c.add_contact(&b.my_id(), "Bob").unwrap();
        until(|| b.state().requests.iter().any(|r| r.peer == c.my_id())).await;
        assert!(c.send(&b.my_id(), vec![src.join("Folder/a.txt")], false, None).await.is_err(), "not connected before a yes");
        b.answer_request(&c.my_id(), false);
        until(|| c.state().contacts[0].note.is_some()).await;
        assert!(!c.state().contacts[0].online);
        assert!(c.state().contacts[0].contact.declined, "not tried again by itself");
        // Asking again by hand: turned away without asking Bob (he declined lately).
        c.reach(&b.my_id());
        assert!(!c.state().contacts[0].contact.declined);
        until(|| c.state().contacts[0].contact.declined).await;
        assert!(b.state().requests.is_empty(), "Bob isn't asked again");
        assert!(!root.join("b/Downloads/From Mallory").exists());

        a.stop.cancel();
        b.stop.cancel();
        c.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---------- picking up a transfer cut short ----------

    /// Two Pakos that know each other; Bob lets Alice send without asking.
    async fn befriend(a: &Arc<Node>, b: &Arc<Node>) {
        a.add_hint(addr(b));
        b.add_hint(addr(a));
        a.add_contact(&b.my_id(), "Bob").unwrap();
        until(|| !b.state().requests.is_empty()).await;
        b.answer_request(&a.my_id(), true);
        until(|| a.state().contacts[0].online && b.state().contacts.iter().any(|c| c.online)).await;
        b.edit_contact(&a.my_id(), None, Some(true), None).unwrap();
    }

    async fn pair(root: &Path) -> (Arc<Node>, Arc<Node>) {
        let a = node(&root.join("a"), "Alice").await;
        let b = node(&root.join("b"), "Bob").await;
        befriend(&a, &b).await;
        (a, b)
    }

    /// Bytes that differ with `seed`, never 0xFF.
    fn bytes_of(n: usize, seed: usize) -> Vec<u8> {
        (0..n).map(|i| ((i * 7 + seed) % 251) as u8).collect()
    }

    /// Closes their connection, as a network cable pulled out would.
    fn cut(a: &Node, b: &Node) {
        let conn = a.inner.lock().unwrap().live.get(&b.my_id()).map(|l| l.conn.clone()).expect("connected");
        conn.close(0u32.into(), b"cut");
    }

    fn state_of(n: &Node, id: &str) -> Option<String> {
        n.full_state().transfers.into_iter().find(|t| t.id == id).map(|t| t.state)
    }

    fn ended(n: &Node, id: &str) -> Option<HistoryItem> {
        n.history_item(id)
    }

    /// Sends `path` from Alice to Bob slowly and cuts the connection half-way: both sides then
    /// wait for each other. The transfer's id.
    async fn send_and_cut(a: &Arc<Node>, b: &Arc<Node>, path: PathBuf) -> String {
        a.pace.store(60, Ordering::Relaxed);
        let id = a.send(&b.my_id(), vec![path], false, None).await.unwrap();
        until(|| b.full_state().transfers.iter().any(|t| t.id == id && t.done > 2 * 1024 * 1024)).await;
        cut(a, b);
        until(|| state_of(a, &id).as_deref() == Some("waiting") && state_of(b, &id).as_deref() == Some("waiting")).await;
        // Slower than unlimited, so the round after can be seen.
        a.pace.store(20, Ordering::Relaxed);
        id
    }

    /// Connects them again once both noticed the cut.
    async fn reconnect(a: &Arc<Node>, b: &Arc<Node>) {
        until(|| !a.state().contacts[0].online && !b.state().contacts[0].online).await;
        a.reach(&b.my_id());
        until(|| a.state().contacts[0].online).await;
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if e.path().is_dir() {
                    out.extend(leftovers(&e.path()));
                } else if n.ends_with(".pakorecv") || n.ends_with(".json") {
                    out.push(n);
                }
            }
        }
        out
    }

    /// The connection is cut half-way through a big file and comes back: the transfer continues
    /// by itself from where it stopped, and the file is exactly the one sent.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_transfer_cut_half_way_continues_where_it_stopped() {
        let root = std::env::temp_dir().join(format!("pako-resume-cut-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let big = bytes_of(PIECE as usize * 3 + 12345, 1);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/big.bin"), &big).unwrap();
        let id = send_and_cut(&a, &b, root.join("src/big.bin")).await;

        let had = b.full_state().transfers[0].done;
        assert!(had > 0 && had < big.len() as u64, "{had}");
        // Kept on both sides, and not "busy" while waiting.
        assert!(root.join(format!("b/direct_resume/{id}.in.json")).exists());
        assert!(root.join(format!("a/direct_resume/{id}.out.json")).exists());
        assert!(root.join("b/Downloads/From Alice/big.bin.pakorecv").exists());
        assert!(!a.busy() && !b.busy());
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            let hidden = |p: &Path| std::fs::metadata(p).unwrap().file_attributes() & 2 != 0;
            until(|| hidden(&root.join("b/Downloads/From Alice/big.bin.pakorecv"))).await;
        }

        reconnect(&a, &b).await;
        let mut from = 0;
        until(|| {
            if let Some(t) = a.inner.lock().unwrap().outgoing.get(&id) {
                from = from.max(t.base);
            }
            ended(&a, &id).is_some() && ended(&b, &id).is_some()
        })
        .await;
        assert_eq!(ended(&a, &id).unwrap().result, "done");
        assert_eq!(ended(&b, &id).unwrap().result, "done");
        assert!(from >= had, "continued from {from}, not from zero (Bob had {had})");
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert_eq!(std::fs::metadata(root.join("b/Downloads/From Alice/big.bin")).unwrap().file_attributes() & 2, 0, "shown once finished");
        }
        assert_eq!(std::fs::read(root.join("b/Downloads/From Alice/big.bin")).unwrap(), big);
        assert!(leftovers(&root.join("b/Downloads")).is_empty());
        assert!(leftovers(&root.join("a/direct_resume")).is_empty() && leftovers(&root.join("b/direct_resume")).is_empty(), "notes gone");

        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Pako is closed on the receiving PC half-way and started again: it continues there.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_transfer_continues_after_pako_restarts() {
        let root = std::env::temp_dir().join(format!("pako-resume-restart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let a = node(&root.join("a"), "Alice").await;
        let key = iroh::SecretKey::generate();
        let b = keyed_node(&root.join("b"), "Bob", key.clone()).await;
        befriend(&a, &b).await;
        let big = bytes_of(PIECE as usize * 2 + 999, 3);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/big.bin"), &big).unwrap();
        a.pace.store(60, Ordering::Relaxed);
        let id = a.send(&b.my_id(), vec![root.join("src/big.bin")], false, None).await.unwrap();
        until(|| b.full_state().transfers.iter().any(|t| t.id == id && t.done > 2 * 1024 * 1024)).await;
        // Bob's Pako closes.
        b.stop.cancel();
        b.ep.close().await;
        until(|| state_of(&a, &id).as_deref() == Some("waiting")).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        a.pace.store(0, Ordering::Relaxed);

        // And starts again: the transfer is there, waiting, and continues once connected.
        let b2 = keyed_node(&root.join("b"), "Bob", key).await;
        assert_eq!(state_of(&b2, &id).as_deref(), Some("waiting"));
        a.add_hint(addr(&b2));
        b2.add_hint(addr(&a));
        b2.reach(&a.my_id());
        until(|| ended(&a, &id).is_some() && ended(&b2, &id).is_some()).await;
        assert_eq!(ended(&b2, &id).unwrap().result, "done");
        assert_eq!(std::fs::read(root.join("b/Downloads/From Alice/big.bin")).unwrap(), big);
        assert!(leftovers(&root.join("b/Downloads")).is_empty());

        a.stop.cancel();
        b2.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The file changed on the sender's PC while the transfer waited: it's sent again from zero,
    /// at its new size, and Bob gets the new one.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_source_file_changed_meanwhile_is_sent_again() {
        let root = std::env::temp_dir().join(format!("pako-resume-changed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let src = root.join("src/doc.bin");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(&src, bytes_of(PIECE as usize * 2, 5)).unwrap();
        let id = send_and_cut(&a, &b, src.clone()).await;

        let newer = bytes_of(PIECE as usize * 2 + 4321, 9);
        std::fs::write(&src, &newer).unwrap();
        let f = std::fs::File::options().write(true).open(&src).unwrap();
        f.set_modified(std::time::SystemTime::now() + Duration::from_secs(60)).unwrap();
        drop(f);

        reconnect(&a, &b).await;
        until(|| ended(&a, &id).is_some() && ended(&b, &id).is_some()).await;
        assert_eq!(ended(&b, &id).unwrap().result, "done");
        assert_eq!(ended(&b, &id).unwrap().size, newer.len() as u64);
        assert_eq!(std::fs::read(root.join("b/Downloads/From Alice/doc.bin")).unwrap(), newer);

        // Gone meanwhile: the sender says so, and both sides stop.
        std::fs::write(&src, bytes_of(PIECE as usize * 2, 2)).unwrap();
        let id = send_and_cut(&a, &b, src.clone()).await;
        std::fs::remove_file(&src).unwrap();
        reconnect(&a, &b).await;
        until(|| ended(&a, &id).is_some() && ended(&b, &id).is_some()).await;
        let h = ended(&a, &id).unwrap();
        assert_eq!(h.result, "failed");
        assert!(h.message.unwrap().contains("is gone"));
        assert_eq!(ended(&b, &id).unwrap().result, "cancelled");
        assert!(leftovers(&root.join("b/Downloads")).is_empty());

        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Spoils a byte range Bob's note says is written (as a disk could), so the file won't
    /// match its hash.
    fn spoil(b: &Node, id: &str) {
        let (part, at) = {
            let inner = b.inner.lock().unwrap();
            let t = &inner.incoming[id];
            let at = t.got.lock().unwrap().ranges[0].list()[0][0];
            (t.parts[0].clone(), at)
        };
        let f = std::fs::OpenOptions::new().write(true).open(part).unwrap();
        write_at(&f, &[0xFF; 64], at).unwrap();
    }

    /// A file that doesn't match its hash at the end is sent again in full, once; when it
    /// fails again, the transfer shows the error.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_file_that_doesnt_match_is_sent_again_once() {
        let root = std::env::temp_dir().join(format!("pako-resume-hash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let data = bytes_of(PIECE as usize * 2 + 77, 4);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/x.bin"), &data).unwrap();

        let id = send_and_cut(&a, &b, root.join("src/x.bin")).await;
        spoil(&b, &id);
        reconnect(&a, &b).await;
        let mut retried = false;
        until(|| {
            retried |= b.inner.lock().unwrap().incoming.get(&id).is_some_and(|t| t.retried[0]);
            ended(&a, &id).is_some() && ended(&b, &id).is_some()
        })
        .await;
        assert!(retried, "sent again");
        assert_eq!(ended(&b, &id).unwrap().result, "done");
        assert_eq!(std::fs::read(root.join("b/Downloads/From Alice/x.bin")).unwrap(), data);

        // Already sent again once (as if it went wrong before): now it's an error.
        let id = send_and_cut(&a, &b, root.join("src/x.bin")).await;
        spoil(&b, &id);
        b.inner.lock().unwrap().incoming.get_mut(&id).unwrap().retried[0] = true;
        reconnect(&a, &b).await;
        until(|| ended(&a, &id).is_some() && ended(&b, &id).is_some()).await;
        let (ha, hb) = (ended(&a, &id).unwrap(), ended(&b, &id).unwrap());
        assert_eq!((ha.result.as_str(), hb.result.as_str()), ("failed", "failed"));
        assert!(hb.message.unwrap().contains("even when sent again"));
        assert!(ha.message.unwrap().contains("even when sent again"));
        assert!(!root.join("b/Downloads/From Alice/x (2).bin").exists(), "never given its name");
        assert!(leftovers(&root.join("b/Downloads")).is_empty());

        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// After 7 days of waiting, both sides clear it: the part that arrived goes, the notes go,
    /// and the history says it expired.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_transfer_waiting_7_days_expires() {
        let root = std::env::temp_dir().join(format!("pako-resume-expire-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/x.bin"), bytes_of(PIECE as usize * 2, 6)).unwrap();
        let id = send_and_cut(&a, &b, root.join("src/x.bin")).await;
        // Not yet.
        a.expire(now_ms() + resume::KEEP_MS - 60_000);
        b.expire(now_ms() + resume::KEEP_MS - 60_000);
        assert_eq!(state_of(&b, &id).as_deref(), Some("waiting"));
        // A week later.
        a.expire(now_ms() + resume::KEEP_MS + 1);
        b.expire(now_ms() + resume::KEEP_MS + 1);
        assert_eq!(ended(&a, &id).unwrap().message.as_deref(), Some(EXPIRED_OUT));
        assert_eq!(ended(&b, &id).unwrap().message.as_deref(), Some(EXPIRED_IN));
        assert!(state_of(&a, &id).is_none() && state_of(&b, &id).is_none());
        assert!(leftovers(&root.join("b/Downloads")).is_empty(), "the part that arrived is deleted");
        assert!(leftovers(&root.join("a/direct_resume")).is_empty() && leftovers(&root.join("b/direct_resume")).is_empty());

        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---------- many contacts: calls paced, quiet connections closed ----------

    /// A connection closed for being quiet: both show "seen lately" (not offline), neither
    /// calls back at the next round, and sending (either way) or opening the conversation
    /// calls at once: offers still get through.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_quiet_connection_closes_and_comes_back_when_needed() {
        let root = std::env::temp_dir().join(format!("pako-pace-quiet-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let (ai, bi) = (a.my_id(), b.my_id());
        let quiet = |a: &Arc<Node>, b: &str| {
            let mut inner = a.inner.lock().unwrap();
            a.rest(&mut inner, b, std::time::Instant::now());
        };
        quiet(&a, &bi);
        until(|| !a.state().contacts[0].online && !b.state().contacts[0].online).await;
        assert!(a.state().contacts[0].idle && b.state().contacts[0].idle, "seen lately on both sides");
        let later = std::time::Instant::now() + Duration::from_secs(15 * 60);
        assert!(!a.inner.lock().unwrap().tries.due(&bi, later) && !b.inner.lock().unwrap().tries.due(&ai, later), "not called back at the next rounds");

        // Alice sends: Bob is called at once and the files go.
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/x.txt"), b"hello").unwrap();
        a.send(&bi, vec![root.join("src/x.txt")], false, None).await.unwrap();
        until(|| b.state().history.iter().any(|h| h.result == "done")).await;
        assert!(!a.state().contacts[0].idle, "connected again");

        // Bob closes it this time; then he sends to Alice: her Pako takes the call and asks her.
        quiet(&b, &ai);
        until(|| !a.state().contacts[0].online && !b.state().contacts[0].online).await;
        b.send(&ai, vec![root.join("src/x.txt")], false, None).await.unwrap();
        until(|| a.state().offers.len() == 1).await;

        // Opening the conversation calls at once.
        quiet(&a, &bi);
        until(|| !a.state().contacts[0].online).await;
        a.set_open(Some(&bi));
        until(|| a.state().contacts[0].online).await;

        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 300 contacts that never answer: the round calls at most a few at a time, a moment apart,
    /// each failed call waits longer for the next, and pressing Connect for someone else goes
    /// through at once meanwhile.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_round_calls_a_few_at_a_time_and_waits_longer_for_those_who_dont_answer() {
        let root = std::env::temp_dir().join(format!("pako-pace-round-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let fakes: Vec<String> = (0..300).map(|_| iroh::SecretKey::generate().public().to_string()).collect();
        {
            let mut inner = a.inner.lock().unwrap();
            for id in &fakes {
                inner.contacts.push(Contact { id: id.clone(), name: "Away".into(), trusted: true, ..Default::default() });
            }
        }
        cut(&a, &b);
        until(|| !a.state().contacts[0].online && !b.state().contacts[0].online).await;
        let t0 = std::time::Instant::now();
        tokio::spawn(a.clone().presence_loop());
        let started = |a: &Node| {
            let inner = a.inner.lock().unwrap();
            fakes.iter().filter(|f| inner.connecting.contains(*f) || inner.tries.fails(f) > 0).count()
        };
        let mut most = 0;
        while t0.elapsed() < Duration::from_secs(3) {
            most = most.max(a.inner.lock().unwrap().connecting.iter().filter(|c| fakes.contains(c)).count());
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let n = started(&a);
        let most_by_gap = (t0.elapsed().as_millis() / pace::DIAL_GAP.as_millis()) as usize + 1;
        assert!(most <= pace::DIALS_AT_ONCE, "{most} calls at once");
        assert!(n >= 2 && n <= most_by_gap, "{n} called in 3 s: spread, not all 300 at once");
        {
            let inner = a.inner.lock().unwrap();
            let now = std::time::Instant::now();
            for f in fakes.iter().filter(|f| inner.tries.fails(f) > 0) {
                assert_eq!(inner.tries.fails(f), 1, "called once");
                assert!(!inner.tries.due(f, now + super::super::PRESENCE_EVERY / 2), "next call minutes away");
            }
        }
        // Meanwhile the person presses Connect for Bob: no wait behind the round.
        let t1 = std::time::Instant::now();
        a.reach(&b.my_id());
        until(|| a.state().contacts[0].online).await;
        assert!(t1.elapsed() < Duration::from_secs(3), "{:?}", t1.elapsed());

        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Audit 2026-11-18: a connection that closes for being quiet while a send is being
    /// prepared (a ZIP being made: the contact's conversation isn't open, nothing is moving yet)
    /// is called again when the offer goes, instead of the send failing with "They aren't
    /// connected right now." after the wait. A quiet connection is also out of `live` at once.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_send_still_goes_when_the_connection_closed_while_preparing_it() {
        let root = std::env::temp_dir().join(format!("pako-pace-race-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let bi = b.my_id();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/big.bin"), bytes_of(40 * 1024 * 1024, 5)).unwrap();
        let sending = {
            let a = a.clone();
            let (bi, path) = (bi.clone(), root.join("src/big.bin"));
            tokio::spawn(async move { a.send(&bi, vec![path], true, None).await })
        };
        // The ZIP is being made: the connection closes for being quiet meanwhile.
        let tmp = root.join("a").join("direct_tmp");
        until(|| std::fs::read_dir(&tmp).is_ok_and(|mut d| d.next().is_some())).await;
        {
            let mut inner = a.inner.lock().unwrap();
            a.rest(&mut inner, &bi, std::time::Instant::now());
            assert!(!inner.live.contains_key(&bi), "not used any more once closed");
        }
        sending.await.unwrap().unwrap();
        until(|| b.state().history.iter().any(|h| h.result == "done")).await;
        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Audit 2026-11-18: their Pako closes the conversation you have open for being quiet: it is
    /// called again at the next round, not left "offline" (Send turned off) for an hour.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_open_conversation_closed_by_them_is_called_again() {
        let root = std::env::temp_dir().join(format!("pako-pace-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let (ai, bi) = (a.my_id(), b.my_id());
        a.set_open(Some(&bi));
        {
            let mut inner = b.inner.lock().unwrap();
            b.rest(&mut inner, &ai, std::time::Instant::now());
        }
        until(|| !a.state().contacts[0].online && a.state().contacts[0].idle).await;
        a.reach_open();
        until(|| a.state().contacts[0].online).await;
        // Something that isn't a contact is never kept as the open conversation.
        a.set_open(Some("not a contact"));
        assert_eq!(a.inner.lock().unwrap().open_chat, None);
        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Audit 2026-11-18: a call that reaches their Pako but is turned away at the handshake
    /// (here: too many questions open there) waits like an unanswered one, not just 30 s.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_call_turned_away_at_the_handshake_waits_like_an_unanswered_one() {
        let root = std::env::temp_dir().join(format!("pako-pace-turned-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let a = node(&root.join("a"), "Alice").await;
        let b = node(&root.join("b"), "Bob").await;
        {
            let mut inner = b.inner.lock().unwrap();
            for i in 0..MAX_REQUESTS {
                inner.requests.insert(format!("r{i}"), Request { name: "x".into(), device: false, answer: None, at: i as u64 });
            }
        }
        let bi = b.my_id();
        a.add_hint(addr(&b));
        a.add_contact(&bi, "Bob").unwrap();
        until(|| a.inner.lock().unwrap().tries.fails(&bi) == 1).await;
        let next_round = std::time::Instant::now() + RETRY_WAITING + Duration::from_secs(5);
        assert!(!a.inner.lock().unwrap().tries.due(&bi, next_round), "not called again at the next tick");
        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Audit 2026-11-18: My devices, favourites and people used lately go in the first line of
    /// the round, ahead of a long queue of others.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_wanted_go_ahead_of_the_round() {
        let root = std::env::temp_dir().join(format!("pako-pace-first-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let a = node(&root.join("a"), "Alice").await;
        let fakes: Vec<String> = (0..50).map(|_| iroh::SecretKey::generate().public().to_string()).collect();
        {
            let mut inner = a.inner.lock().unwrap();
            for (i, id) in fakes.iter().enumerate() {
                inner.contacts.push(Contact { id: id.clone(), name: "Away".into(), trusted: true, favourite: i == 40, device: i == 45, ..Default::default() });
            }
        }
        let (f, mut fr) = mpsc::unbounded_channel();
        let (r, mut rr) = mpsc::unbounded_channel();
        a.round(&f, &r);
        let mut first = Vec::new();
        while let Ok(id) = fr.try_recv() {
            first.push(id);
        }
        let mut rest = 0;
        while rr.try_recv().is_ok() {
            rest += 1;
        }
        assert_eq!(first, [fakes[45].clone(), fakes[40].clone()], "the device, then the favourite");
        assert_eq!(rest, 48);
        a.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Audit 2026-11-18: a transfer waits for someone: calls that fail aren't counted, so after
    /// its first hour they're still tried every round (3 min), not once an hour.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_waiting_transfers_contact_isnt_pushed_out_to_an_hour() {
        let root = std::env::temp_dir().join(format!("pako-pace-waits-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let bi = b.my_id();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/big.bin"), bytes_of(8 * 1024 * 1024, 3)).unwrap();
        let _id = send_and_cut(&a, &b, root.join("src/big.bin")).await;
        // Bob is gone for good: Alice's calls fail.
        b.stop.cancel();
        b.ep.close().await;
        a.inner.lock().unwrap().hints.insert(bi.clone(), EndpointAddr::new(b.ep.id()).with_ip_addr("127.0.0.1:9".parse().unwrap()));
        until(|| !a.inner.lock().unwrap().live.contains_key(&bi) && !a.inner.lock().unwrap().connecting.contains(&bi)).await;
        let t0 = std::time::Instant::now();
        let _ = a.connect_as(&bi, true).await;
        let inner = a.inner.lock().unwrap();
        assert_eq!(inner.tries.fails(&bi), 0, "not counted");
        assert!(inner.tries.next(&bi).is_some_and(|n| n <= t0 + super::super::PRESENCE_EVERY + CONNECT_TIMEOUT + Duration::from_secs(1)), "next round, as before");
        drop(inner);
        a.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---------- mute and "Check this contact" ----------

    /// Muted: no notice of any kind from them (files coming in, received, waiting for your
    /// answer, a request), but their offer still arrives and waits. Unmuted: notices again.
    /// My devices can't be muted.
    #[tokio::test(flavor = "multi_thread")]
    async fn mute_silences_every_notice_but_the_offer_still_waits() {
        let root = std::env::temp_dir().join(format!("pako-mute-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let log: Arc<Mutex<Vec<String>>> = Arc::default();
        let a = node(&root.join("a"), "Alice").await;
        let l = log.clone();
        let b = told_node(&root.join("b"), "Bob", iroh::SecretKey::generate(), Arc::new(move |kind, _, _, _| l.lock().unwrap().push(kind.to_string()))).await;
        befriend(&a, &b).await;
        let (ai, bi) = (a.my_id(), b.my_id());
        log.lock().unwrap().clear();
        b.set_muted(&ai, true).unwrap();
        assert!(b.state().contacts[0].contact.muted);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/x.txt"), b"hello").unwrap();
        // Sent without asking: "Files coming in" and "Files received" stay silent.
        a.send(&bi, vec![root.join("src/x.txt")], false, None).await.unwrap();
        until(|| b.state().history.iter().any(|h| h.result == "done")).await;
        // Asked: "Files for you" stays silent, the offer waits.
        b.edit_contact(&ai, None, Some(false), None).unwrap();
        a.send(&bi, vec![root.join("src/x.txt")], false, None).await.unwrap();
        until(|| b.state().offers.len() == 1).await;
        // A request from them (a device pairing) goes through the same gate.
        b.tell(&ai, "direct-request", "Pair a device", "x", Go::none());
        assert!(log.lock().unwrap().is_empty(), "{:?}", log.lock().unwrap());
        // Unmuted: notices again.
        b.set_muted(&ai, false).unwrap();
        a.send(&bi, vec![root.join("src/x.txt")], false, None).await.unwrap();
        until(|| b.state().offers.len() == 2).await;
        assert_eq!(*log.lock().unwrap(), ["direct-offer"]);
        // Muted, then paired as one of My devices: not muted any more.
        b.set_muted(&ai, true).unwrap();
        b.add_device(&ai, Some("Laptop"), None).unwrap();
        assert!(!b.inner.lock().unwrap().contacts[0].muted, "pairing ends the mute");
        // My devices can't be muted.
        assert!(b.set_muted(&ai, true).is_err());
        // Even one marked muted (a file from before): its notices still come.
        log.lock().unwrap().clear();
        b.inner.lock().unwrap().contacts[0].muted = true;
        b.tell(&ai, "direct-received", "Files received", "x", Go::none());
        assert_eq!(*log.lock().unwrap(), ["direct-received"]);
        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The round: Often (also devices) first, then favourites and people who can send without
    /// asking; Ask and Automatic gone quiet aren't called and show "Not checked lately"; Check
    /// now and a transfer bring a quiet one back.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_check_modes_steer_the_round() {
        let root = std::env::temp_dir().join(format!("pako-check-round-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let a = node(&root.join("a"), "Alice").await;
        let fakes: Vec<String> = (0..7).map(|_| iroh::SecretKey::generate().public().to_string()).collect();
        let old = now_ms() - 40 * 24 * 3600 * 1000;
        {
            let mut inner = a.inner.lock().unwrap();
            for (i, id) in fakes.iter().enumerate() {
                let mut c = Contact { id: id.clone(), name: format!("C{i}"), trusted: true, last_activity: Some(old), ..Default::default() };
                match i {
                    0 => c.last_activity = Some(now_ms()),
                    1 => {}
                    2 => c.favourite = true,
                    3 => c.auto_accept = true,
                    4 => c.check = pace::Check::Ask,
                    5 => (c.check, c.check_until) = (pace::Check::Often, Some(now_ms() + 3_600_000)),
                    _ => c.device = true,
                }
                inner.contacts.push(c);
            }
        }
        let lines = |a: &Arc<Node>| {
            let (f, mut fr) = mpsc::unbounded_channel();
            let (r, mut rr) = mpsc::unbounded_channel();
            a.round(&f, &r);
            let (mut first, mut rest) = (Vec::new(), Vec::new());
            while let Ok(id) = fr.try_recv() {
                first.push(id);
            }
            while let Ok(id) = rr.try_recv() {
                rest.push(id);
            }
            a.inner.lock().unwrap().queued.clear();
            (first, rest)
        };
        let (first, rest) = lines(&a);
        let pos = |id: &String| first.iter().position(|x| x == id).unwrap();
        assert_eq!(first.len(), 4, "{first:?}");
        assert!(pos(&fakes[5]) < 2 && pos(&fakes[6]) < 2, "Often ones (Often, device) before the favourite and the auto-accept one");
        assert_eq!(rest, [fakes[0].clone()]);
        let view = a.full_state();
        let unchecked: Vec<bool> = fakes.iter().map(|f| view.contacts.iter().find(|c| c.contact.id == *f).unwrap().unchecked).collect();
        assert_eq!(unchecked, [false, true, false, false, true, false, false], "Not checked lately: gone quiet, and ask-only");
        let as_: Vec<pace::Check> = fakes.iter().map(|f| view.contacts.iter().find(|c| c.contact.id == *f).unwrap().check_as).collect();
        assert_eq!((as_[3], as_[5], as_[6]), (pace::Check::Auto, pace::Check::Often, pace::Check::Often), "can send without asking: Automatic, kept first");
        // Check now: in the round again (Automatic), and called at once.
        a.reach(&fakes[1]);
        assert!(a.inner.lock().unwrap().contacts[1].last_activity.unwrap() > old);
        // A transfer (any end) with a quiet one wakes them too.
        a.record(HistoryItem { id: "t".into(), contact: fakes[1].clone(), outgoing: false, name: "x".into(), files: 1, size: 1, date: now_ms(), path: None, result: "declined".into(), message: None, saved_to: None, duration_ms: 0, avg_speed: 0, peak_speed: 0, via: None, relay_to: None });
        {
            let inner = a.inner.lock().unwrap();
            assert!(in_round(&inner, &inner.contacts[1], now_ms()));
            assert!(!in_round(&inner, &inner.contacts[4], now_ms()), "Ask stays out of the round");
        }
        // An Often contact who doesn't answer isn't backed off.
        let _ = a.connect_as(&fakes[5], true).await;
        assert_eq!(a.inner.lock().unwrap().tries.fails(&fakes[5]), 0);
        let _ = a.connect_as(&fakes[0], true).await;
        assert_eq!(a.inner.lock().unwrap().tries.fails(&fakes[0]), 1, "an Automatic one is");
        let _ = a.connect_as(&fakes[3], true).await;
        assert_eq!(a.inner.lock().unwrap().tries.fails(&fakes[3]), 1, "and one who can send without asking too");
        // Choosing Often needs how long; it's saved with its end.
        assert!(a.set_check(&fakes[1], pace::Check::Often, None).is_err());
        a.set_check(&fakes[1], pace::Check::Often, Some(pace::OftenFor::Hour)).unwrap();
        let until = a.inner.lock().unwrap().contacts[1].check_until.unwrap();
        assert!(until.abs_diff(now_ms() + 3_600_000) < 5_000);
        // Its time up: Automatic again, saved and shown.
        a.inner.lock().unwrap().contacts[1].check_until = Some(now_ms() - 1);
        a.settle_checks();
        assert_eq!(a.full_state().contacts.iter().find(|c| c.contact.id == fakes[1]).unwrap().contact.check, pace::Check::Auto);
        a.set_check(&fakes[1], pace::Check::Ask, None).unwrap();
        assert!(a.inner.lock().unwrap().contacts[1].check_until.is_none());
        // When I ask, then Often for 1 hour: back to When I ask once it ends (also when Often
        // is picked again meanwhile), shown as When I ask in the meantime's end.
        a.set_check(&fakes[1], pace::Check::Often, Some(pace::OftenFor::Hour)).unwrap();
        a.set_check(&fakes[1], pace::Check::Often, Some(pace::OftenFor::Today)).unwrap();
        assert_eq!(a.inner.lock().unwrap().contacts[1].check_before, Some(pace::Check::Ask));
        a.inner.lock().unwrap().contacts[1].check_until = Some(now_ms() - 1);
        assert_eq!(pace::check_of(&a.inner.lock().unwrap().contacts[1], now_ms()), pace::Check::Ask, "before it's settled too");
        a.settle_checks();
        {
            let inner = a.inner.lock().unwrap();
            let c = &inner.contacts[1];
            assert_eq!((c.check, c.check_until, c.check_before), (pace::Check::Ask, None, None));
            assert!(!in_round(&inner, c, now_ms()), "When I ask again: out of the round");
        }
        // Automatic, then Often: back to Automatic (nothing kept).
        a.set_check(&fakes[1], pace::Check::Auto, None).unwrap();
        a.set_check(&fakes[1], pace::Check::Often, Some(pace::OftenFor::Hour)).unwrap();
        assert_eq!(a.inner.lock().unwrap().contacts[1].check_before, None);
        a.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Them reaching us counts as activity (Automatic wakes), and a transfer waiting for an
    /// ask-only contact is still called.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_incoming_connection_wakes_a_quiet_contact_and_waiting_transfers_still_call() {
        let root = std::env::temp_dir().join(format!("pako-check-in-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let (ai, bi) = (a.my_id(), b.my_id());
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/big.bin"), bytes_of(8 * 1024 * 1024, 4)).unwrap();
        a.set_check(&bi, pace::Check::Ask, None).unwrap();
        let _id = send_and_cut(&a, &b, root.join("src/big.bin")).await;
        {
            let inner = a.inner.lock().unwrap();
            assert!(in_round(&inner, &inner.contacts[0], now_ms()), "a transfer waits for them: called whatever the setting");
        }
        let old = now_ms() - 40 * 24 * 3600 * 1000;
        // (Bob lets Alice send without asking: she'd never go quiet; back to plain Automatic.)
        b.edit_contact(&ai, None, Some(false), None).unwrap();
        b.inner.lock().unwrap().contacts[0].last_activity = Some(old);
        {
            let inner = b.inner.lock().unwrap();
            assert!(!pace::in_round(&inner.contacts[0], now_ms()), "gone quiet on Bob's side");
        }
        reconnect(&a, &b).await;
        until(|| b.inner.lock().unwrap().contacts.iter().find(|c| c.id == ai).unwrap().last_activity.unwrap() > old).await;
        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// With a Pako from before this on either side, a cut ends the transfer as it always did:
    /// nothing kept, nothing waiting, and the next send works.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_older_pako_ends_a_cut_transfer_as_before() {
        for old_receiver in [true, false] {
            let root = std::env::temp_dir().join(format!("pako-resume-old-{}-{old_receiver}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            let (a, b) = pair(&root).await;
            let old = if old_receiver { &b } else { &a };
            old.old_pako.store(true, Ordering::Relaxed);
            std::fs::create_dir_all(root.join("src")).unwrap();
            std::fs::write(root.join("src/x.bin"), bytes_of(PIECE as usize * 2, 7)).unwrap();
            std::fs::write(root.join("src/small.txt"), b"hello").unwrap();
            a.pace.store(60, Ordering::Relaxed);
            let id = a.send(&b.my_id(), vec![root.join("src/x.bin")], false, None).await.unwrap();
            until(|| b.full_state().transfers.iter().any(|t| t.id == id && t.done > 1024 * 1024)).await;
            assert!(!a.inner.lock().unwrap().outgoing[&id].resume && !b.inner.lock().unwrap().incoming[&id].resume);
            cut(&a, &b);
            until(|| ended(&a, &id).is_some() && ended(&b, &id).is_some()).await;
            assert_eq!(ended(&a, &id).unwrap().result, "failed");
            assert_eq!(ended(&b, &id).unwrap().result, "failed");
            assert!(leftovers(&root.join("b/Downloads")).is_empty());
            assert!(leftovers(&root.join("a/direct_resume")).is_empty() && leftovers(&root.join("b/direct_resume")).is_empty());
            a.pace.store(0, Ordering::Relaxed);
            reconnect(&a, &b).await;
            let id = a.send(&b.my_id(), vec![root.join("src/small.txt")], false, None).await.unwrap();
            until(|| ended(&b, &id).is_some_and(|h| h.result == "done")).await;
            a.stop.cancel();
            b.stop.cancel();
            let _ = std::fs::remove_dir_all(&root);
        }
        // On the wire: an offer without the new field is read as before, and an older Pako
        // reads the new messages as ones it doesn't know.
        let m: Msg = serde_json::from_str(r#"{"t":"offer","id":"x","name":"n","files":[]}"#).unwrap();
        assert_eq!(m, Msg::Offer { id: "x".into(), name: "n".into(), files: vec![], resume: false, relay: None });
        assert_eq!(serde_json::to_string(&Msg::Accept { id: "x".into(), resume: false }).unwrap(), r#"{"t":"accept","id":"x"}"#);
        #[derive(serde::Deserialize, Debug, PartialEq)]
        #[serde(tag = "t", rename_all = "camelCase")]
        enum Old {
            Offer { id: String, name: String, files: Vec<OfferFile> },
            Accept { id: String },
            #[serde(other)]
            Unknown,
        }
        let read = |m: &Msg| serde_json::from_str::<Old>(&serde_json::to_string(m).unwrap()).unwrap();
        assert_eq!(read(&Msg::Offer { id: "x".into(), name: "n".into(), files: vec![], resume: true, relay: None }), Old::Offer { id: "x".into(), name: "n".into(), files: vec![] });
        assert_eq!(read(&Msg::Accept { id: "x".into(), resume: true }), Old::Accept { id: "x".into() });
        assert_eq!(read(&Msg::Resume { id: "x".into() }), Old::Unknown);
        assert_eq!(read(&Msg::Have { id: "x".into(), full: vec![[0, 1]], part: vec![] }), Old::Unknown);
        assert_eq!(read(&Msg::Continue { id: "x".into(), restart: vec![] }), Old::Unknown);
    }

    /// Someone else can't continue (or end, or reshape) a transfer that isn't theirs, and the
    /// right contact can't claim bytes past the end of a file.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_forged_resume_for_another_transfer_is_refused() {
        let root = std::env::temp_dir().join(format!("pako-resume-forged-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let size = PIECE * 2;
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/x.bin"), bytes_of(size as usize, 8)).unwrap();
        let id = send_and_cut(&a, &b, root.join("src/x.bin")).await;

        let mallory = iroh::SecretKey::generate().public().to_string();
        let (tx, mut rx) = mpsc::unbounded_channel();
        // Asks Alice to continue Bob's transfer: told it's unknown, nothing is sent.
        a.clone().on_message(&mallory, &tx, Msg::Have { id: id.clone(), full: vec![], part: vec![] }).await;
        assert_eq!(rx.try_recv().ok(), Some(Msg::Cancel { id: id.clone() }));
        // Asks Bob to take a huge "changed" file for it, to cancel it, or whether he has it.
        b.clone().on_message(&mallory, &tx, Msg::Continue { id: id.clone(), restart: vec![Restart { file: 0, size: 1 << 50 }] }).await;
        b.clone().on_message(&mallory, &tx, Msg::Cancel { id: id.clone() }).await;
        a.clone().on_message(&mallory, &tx, Msg::Cancel { id: id.clone() }).await;
        b.clone().on_message(&mallory, &tx, Msg::Resume { id: id.clone() }).await;
        assert_eq!(rx.try_recv().ok(), Some(Msg::Cancel { id: id.clone() }));
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(rx.try_recv().is_err(), "nothing else said to them");
        assert_eq!(state_of(&a, &id).as_deref(), Some("waiting"));
        assert_eq!(state_of(&b, &id).as_deref(), Some("waiting"));
        assert_eq!(b.inner.lock().unwrap().incoming[&id].files[0].size, size);

        // Bob's own Pako claiming bytes past the end: refused, the transfer stops.
        a.clone().on_message(&b.my_id(), &tx, Msg::Have { id: id.clone(), full: vec![], part: vec![FileHave { f: 0, r: vec![[0, size + 1]] }] }).await;
        until(|| ended(&a, &id).is_some()).await;
        assert_eq!(ended(&a, &id).unwrap().result, "failed");

        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A file that "changed" on the sender's PC can't grow what was accepted: from a contact who
    /// has to be asked, any growth stops the transfer; from one who may send without asking,
    /// growth that would have been asked about (big for the drive) stops it too. Nothing grows
    /// on the disk either way.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_continue_never_grows_past_what_was_accepted() {
        let root = std::env::temp_dir().join(format!("pako-resume-grow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = pair(&root).await;
        let src = root.join("src/doc.bin");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(&src, bytes_of(PIECE as usize * 2, 5)).unwrap();

        // Bob asks before taking files from Alice: a bigger file after the cut stops it.
        let id = send_and_cut(&a, &b, src.clone()).await;
        b.edit_contact(&a.my_id(), None, Some(false), None).unwrap();
        std::fs::write(&src, bytes_of(PIECE as usize * 2 + 10, 9)).unwrap();
        let f = std::fs::File::options().write(true).open(&src).unwrap();
        f.set_modified(std::time::SystemTime::now() + Duration::from_secs(60)).unwrap();
        drop(f);
        reconnect(&a, &b).await;
        until(|| ended(&a, &id).is_some() && ended(&b, &id).is_some()).await;
        let (ha, hb) = (ended(&a, &id).unwrap(), ended(&b, &id).unwrap());
        assert_eq!((ha.result.as_str(), hb.result.as_str()), ("failed", "failed"));
        assert!(hb.message.unwrap().contains("bigger than what you accepted"));
        assert!(ha.message.unwrap().contains("bigger than what they accepted"));
        assert!(leftovers(&root.join("b/Downloads")).is_empty());

        // Bob lets Alice send without asking: a "changed" file the size of a whole drive is
        // still refused, and the part on the disk keeps its size.
        b.edit_contact(&a.my_id(), None, Some(true), None).unwrap();
        std::fs::write(&src, bytes_of(PIECE as usize * 2, 3)).unwrap();
        let id = send_and_cut(&a, &b, src.clone()).await;
        let part = b.inner.lock().unwrap().incoming[&id].parts[0].clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        b.clone().on_message(&a.my_id(), &tx, Msg::Continue { id: id.clone(), restart: vec![Restart { file: 0, size: 1 << 50 }] }).await;
        until(|| ended(&b, &id).is_some()).await;
        let hb = ended(&b, &id).unwrap();
        assert_eq!(hb.result, "failed");
        assert!(hb.message.unwrap().starts_with(NO_ROOM));
        assert!(!part.exists(), "the part that arrived is deleted, never grown");

        a.stop.cancel();
        b.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The receiver's note never claims bytes that weren't synced: a file written to stays "to
    /// sync" until a note that synced it is saved, so a newer snapshot saved first (or a failed
    /// sync) still syncs it.
    #[test]
    fn a_note_syncs_every_file_written_since_the_last_saved_note() {
        let mut g = Got::new(vec![Ranges::default(); 2]);
        assert_eq!(g.wrote(0, 0, 10), 10);
        let (s1, _, sync1) = g.snapshot();
        assert_eq!(sync1, vec![(0, 1)]);
        // A newer snapshot before the first one was saved: file 0 is still to sync.
        assert_eq!(g.wrote(1, 0, 5), 5);
        let (s2, have, sync2) = g.snapshot();
        assert!(s2 > s1);
        assert_eq!(sync2, vec![(0, 1), (1, 1)]);
        assert_eq!(have[0].covered() + have[1].covered(), 15);
        // Saved: nothing left to sync until something is written again.
        g.saved(&sync2);
        assert!(g.snapshot().2.is_empty());
        // The older one saved late changes nothing.
        g.saved(&sync1);
        assert_eq!(g.wrote(0, 5, 10), 5, "only new bytes count");
        assert_eq!(g.snapshot().2, vec![(0, 2)]);
    }
}
