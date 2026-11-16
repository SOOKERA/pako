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
use iroh::{Endpoint, EndpointAddr, EndpointId, TransportAddr};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot, Notify};
use tokio_util::sync::CancellationToken;

use super::proto::{self, Msg, OfferFile, Part, ALPN, MAX_HEADER, MAX_MSG};
use crate::notices::Go;
use super::store::{self, Blocked, Contact, HistoryItem, Store};

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
/// How long someone you declined or removed is turned away without asking you again.
const BLOCK_FOR_MS: u64 = 7 * 24 * 3600 * 1000;

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

/// How a connection travels right now: "direct6", "direct" or "relay".
fn path_of(conn: &Connection) -> Option<String> {
    let paths = conn.paths();
    let sel = paths.iter().find(|p| p.is_selected())?;
    Some(match sel.remote_addr() {
        TransportAddr::Ip(a) if a.is_ipv6() => "direct6".into(),
        TransportAddr::Ip(_) => "direct".into(),
        _ => "relay".into(),
    })
}

struct Live {
    conn: Connection,
    tx: mpsc::UnboundedSender<Msg>,
    /// Who opened this connection (the same pair keeps the one opened by the smaller ID).
    initiator: String,
}

struct Request {
    name: String,
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
    /// People you declined or removed, turned away without asking until a date.
    blocked: Vec<Blocked>,
    /// Counter for arrival order.
    seq: u64,
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
    /// Where background work runs (the window's commands aren't async).
    rt: tokio::runtime::Handle,
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestView {
    pub peer: String,
    pub name: String,
    pub code: String,
    /// One of your contacts already has this name (a different Pako: maybe someone pretending).
    pub known_as: Option<String>,
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
        // Left over from transfers cut short (Pako closed or crashed): unfinished pieces and
        // temporary ZIPs go (the sender sends again).
        for p in store.load_parts() {
            if is_own_leftover(Path::new(&p)) {
                let _ = std::fs::remove_file(&p);
            }
        }
        store.save_parts(&[]);
        let _ = std::fs::remove_dir_all(data_dir.join("direct_tmp"));
        let contacts = store.load_contacts();
        let tags = store.load_tags(&contacts);
        let blocked = store.load_blocked(now_ms());
        let inner = Inner { contacts, history: store.load_history(), tags, my_name, blocked, ..Default::default() };
        Arc::new(Self {
            ep,
            store,
            data_dir,
            downloads,
            inner: Mutex::new(inner),
            changed,
            notify,
            settle,
            stop: CancellationToken::new(),
            rt: tokio::runtime::Handle::current(),
        })
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

    pub fn state(&self) -> State {
        let me = self.my_id();
        let inner = self.inner.lock().unwrap();
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
        }
    }

    pub fn busy(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        !inner.outgoing.is_empty() || inner.incoming.values().any(|t| t.state != "asking")
    }

    // ---------- contacts ----------

    pub fn set_my_name(&self, name: &str) {
        self.inner.lock().unwrap().my_name = name.trim().chars().take(60).collect();
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
            });
            self.save_contacts(&inner);
        }
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
            self.save_contacts(&inner);
            self.block(&mut inner, id);
            inner.live.remove(id)
        };
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
            let e = crate::numbers::direct_event(
                &item.id,
                item.date as i64,
                item.outgoing,
                &item.name,
                item.size,
                item.files,
                item.path.as_deref(),
                &item.contact,
                item.duration_ms,
                item.avg_speed,
                item.peak_speed,
            );
            std::thread::spawn(move || crate::numbers::record(e));
        }
        let mut inner = self.inner.lock().unwrap();
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

    /// Tries to reach a contact now, in the background (adding them, the Connect button): also
    /// someone who declined before (they're asked again).
    pub fn reach(self: &Arc<Self>, id: &str) {
        {
            let mut inner = self.inner.lock().unwrap();
            let was = inner.contacts.iter_mut().find(|c| c.id == id).is_some_and(|c| std::mem::replace(&mut c.declined, false));
            if was {
                self.save_contacts(&inner);
            }
        }
        self.dial(id);
    }

    fn dial(self: &Arc<Self>, id: &str) {
        let me = self.clone();
        let id = id.to_string();
        self.rt.spawn(async move {
            let _ = me.connect(&id).await;
        });
    }

    /// Tries every contact that isn't connected, except those who declined (only the Connect
    /// button asks them again).
    pub fn reach_all(self: &Arc<Self>) {
        let ids: Vec<String> = {
            let inner = self.inner.lock().unwrap();
            inner.contacts.iter().filter(|c| !c.declined && !inner.live.contains_key(&c.id)).map(|c| c.id.clone()).collect()
        };
        for id in ids {
            self.dial(&id);
        }
    }

    pub async fn connect(self: &Arc<Self>, id: &str) -> Result<(), String> {
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
        let conn = match tokio::time::timeout(CONNECT_TIMEOUT, self.ep.connect(addr, ALPN)).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                self.note(id, Some(format!("Not reachable right now ({e})")));
                return Err(e.to_string());
            }
            Err(_) => {
                self.note(id, Some("Not reachable right now".into()));
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

    /// Checks every few minutes who's reachable.
    pub async fn presence_loop(self: Arc<Self>, every: Duration) {
        loop {
            self.reach_all();
            tokio::select! {
                _ = self.stop.cancelled() => return,
                _ = tokio::time::sleep(every) => {}
            }
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
            tokio::time::sleep(Duration::from_millis(300)).await;
            conn.close(1u32.into(), b"no");
            return;
        }
        // Keep one connection per contact: the one opened by the smaller ID.
        let opener = if initiator { self.my_id() } else { peer.clone() };
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(old) = inner.live.get(&peer) {
                if old.initiator <= opener {
                    drop(inner);
                    conn.close(3u32.into(), b"duplicate");
                    return;
                }
                old.conn.close(3u32.into(), b"duplicate");
            }
            inner.live.insert(peer.clone(), Live { conn: conn.clone(), tx: tx.clone(), initiator: opener });
            inner.waiting.remove(&peer);
            inner.notes.remove(&peer);
            if let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) {
                c.trusted = true;
                c.declined = false;
                c.last_seen = Some(now_ms());
            }
            self.save_contacts(&inner);
        }
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
        // Gone: whatever was moving with them stops.
        let still_ours = {
            let mut inner = self.inner.lock().unwrap();
            let ours = inner.live.get(&peer).is_some_and(|l| l.conn.stable_id() == conn.stable_id());
            if ours {
                inner.live.remove(&peer);
                if let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) {
                    c.last_seen = Some(now_ms());
                }
                self.save_contacts(&inner);
            }
            ours
        };
        if still_ours {
            self.connection_lost(&peer);
        }
        conn.close(0u32.into(), b"bye");
        self.changed();
    }

    async fn handshake_out(&self, peer: &str, tx: &mpsc::UnboundedSender<Msg>, recv: &mut RecvStream, conn: &Connection) -> bool {
        let name = self.inner.lock().unwrap().my_name.clone();
        if tx.send(Msg::Hello { name }).is_err() {
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
        let hello = match tokio::time::timeout(CONNECT_TIMEOUT, proto::read_frame::<_, Msg>(recv, 64 * 1024)).await {
            Ok(Ok(Some(Msg::Hello { name }))) => name,
            _ => return false,
        };
        let my_name = self.inner.lock().unwrap().my_name.clone();
        // Someone in your contacts (you added them): connected, no question asked.
        if self.inner.lock().unwrap().contacts.iter().any(|c| c.id == peer) {
            self.remember_their_name(peer, &hello);
            return tx.send(Msg::Welcome { name: my_name }).is_ok();
        }
        // Someone you declined or removed lately: turned away without asking you again.
        let now = now_ms();
        if self.inner.lock().unwrap().blocked.iter().any(|b| b.id == peer && b.until > now) {
            let _ = tx.send(Msg::Refused);
            return false;
        }
        // Someone new: asked once.
        let shown = clean_name(&hello).unwrap_or_else(|| "Someone".into());
        let (atx, arx) = oneshot::channel();
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.requests.contains_key(peer) {
                return false;
            }
            let at = inner.next_seq();
            inner.requests.insert(peer.to_string(), Request { name: shown.clone(), answer: Some(atx), at });
        }
        let _ = tx.send(Msg::Wait);
        self.changed();
        (self.notify)(
            "direct-request",
            "Someone wants to connect",
            &format!("{shown} wants to connect with you in Pako."),
            Go { page: "direct".into(), peer: Some(peer.to_string()), ..Go::none() },
        );
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
                    their_name: Some(shown.clone()),
                    declined: false,
                });
                self.save_contacts(&inner);
            }
        }
        tx.send(Msg::Welcome { name: my_name }).is_ok()
    }

    /// Keeps the name a contact chose for themselves up to date (your name for them stays).
    fn remember_their_name(&self, peer: &str, said: &str) {
        let theirs = clean_name(said);
        let mut inner = self.inner.lock().unwrap();
        let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) else { return };
        if theirs.is_none() || c.their_name == theirs {
            return;
        }
        c.their_name = theirs;
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
        for id in outs {
            self.end_outgoing(&id, "failed", Some("The connection was lost.".into()));
        }
        for id in ins {
            self.end_incoming(&id, "failed", Some("The connection was lost.".into()));
        }
    }

    async fn on_message(self: Arc<Self>, peer: &str, tx: &mpsc::UnboundedSender<Msg>, m: Msg) {
        match m {
            Msg::Offer { id, name, files } => self.on_offer(peer, tx, id, name, files),
            Msg::Accept { .. } | Msg::Decline { .. } | Msg::Result { .. } => {
                let id = match &m {
                    Msg::Accept { id } | Msg::Decline { id } | Msg::Result { id, .. } => id.clone(),
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
            // Refused (they removed you), Hello/Welcome/Wait after the handshake: nothing to do.
            _ => {}
        }
    }

    // ---------- sending ----------

    /// Sends files and folders to a connected contact. `compress`: one compressed ZIP.
    pub async fn send(self: &Arc<Self>, contact: &str, paths: Vec<PathBuf>, compress: bool, name: Option<String>) -> Result<String, String> {
        let tx = self.inner.lock().unwrap().live.get(contact).map(|l| l.tx.clone()).ok_or("They aren't connected right now.")?;
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
        // The same checks the other side makes, so a problem shows here with a clear reason.
        let listed: Vec<OfferFile> = files.iter().map(|(_, f)| f.clone()).collect();
        let problem = match proto::check_offer(&title, &listed) {
            Err(e) => Some(format!("This can't be sent as it is: {e}")),
            Ok(_) => {
                let offer = Msg::Offer { id: id.clone(), name: title.clone(), files: listed };
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
        let (etx, erx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        {
            let mut inner = self.inner.lock().unwrap();
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
                },
            );
        }
        let offer = Msg::Offer { id: id.clone(), name: title, files: files.iter().map(|(_, f)| f.clone()).collect() };
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
        match answer {
            Some(Msg::Accept { .. }) => {}
            Some(Msg::Decline { .. }) => {
                self.end_outgoing(id, "declined", Some("They declined it.".into()));
                return;
            }
            // Their Pako turned it away without asking (and says why).
            Some(Msg::Result { ok: false, message, .. }) => {
                self.end_outgoing(id, "failed", message.or(Some("Their Pako couldn't take it.".into())));
                return;
            }
            _ => {
                self.end_outgoing(id, "failed", Some("No answer.".into()));
                return;
            }
        }
        let (files, sent) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.outgoing.get_mut(id) else { return };
            t.state = "sending".into();
            t.meter.start(t.sent.clone(), t.cancel.clone(), &self.rt);
            (t.files.clone(), t.sent.clone())
        };
        self.watch(id, true, sent.clone(), cancel.clone());
        self.changed();
        let Some(conn) = self.inner.lock().unwrap().live.get(contact).map(|l| l.conn.clone()) else {
            self.end_outgoing(id, "failed", Some("The connection was lost.".into()));
            return;
        };
        // Hashes are made alongside the sending.
        let hash_paths: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
        let hashing = tokio::task::spawn_blocking(move || hash_files(&hash_paths));
        let mut queue = VecDeque::new();
        for (i, (_, f)) in files.iter().enumerate() {
            let mut off = 0;
            while off < f.size {
                let len = PIECE.min(f.size - off);
                queue.push_back(Part { id: id.to_string(), file: i, offset: off, len });
                off += len;
            }
        }
        let queue = Arc::new(Mutex::new(queue));
        let paths: Arc<Vec<PathBuf>> = Arc::new(files.iter().map(|(p, _)| p.clone()).collect());
        let mut workers = Vec::new();
        for _ in 0..STREAMS {
            let (queue, paths, conn, sent, cancel) = (queue.clone(), paths.clone(), conn.clone(), sent.clone(), cancel.clone());
            workers.push(tokio::spawn(async move {
                loop {
                    let part = queue.lock().unwrap().pop_front();
                    let Some(part) = part else { return Ok::<(), String>(()) };
                    tokio::select! {
                        _ = cancel.cancelled() => return Err("cancelled".into()),
                        r = send_part(&conn, &paths[part.file], &part, &sent) => r?,
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
        let tx = self.inner.lock().unwrap().live.get(contact).map(|l| l.tx.clone());
        if tx.and_then(|t| t.send(Msg::Done { id: id.to_string(), hashes }).ok()).is_none() {
            self.end_outgoing(id, "failed", Some("The connection was lost.".into()));
            return;
        }
        let verdict = tokio::select! {
            _ = cancel.cancelled() => return,
            m = events.recv() => m,
        };
        match verdict {
            Some(Msg::Result { ok: true, .. }) => self.end_outgoing(id, "done", None),
            Some(Msg::Result { message, .. }) => self.end_outgoing(id, "failed", message.or(Some("They couldn't check the files.".into()))),
            _ => self.end_outgoing(id, "failed", Some("No answer.".into())),
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
        if tell {
            self.tell_cancel(&t.contact, id);
        }
        if let Some(tmp) = &t.temp {
            let _ = std::fs::remove_file(tmp);
        }
        let path = self.inner.lock().unwrap().live.get(&t.contact).and_then(|l| path_of(&l.conn));
        let (duration_ms, avg_speed, peak_speed) = t.meter.summary(t.sent.load(Ordering::Relaxed));
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
            message,
            saved_to: None,
            duration_ms,
            avg_speed,
            peak_speed,
        });
        self.changed();
    }

    // ---------- receiving ----------

    fn on_offer(self: Arc<Self>, peer: &str, tx: &mpsc::UnboundedSender<Msg>, id: String, name: String, files: Vec<OfferFile>) {
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
        // "Can send without asking" still asks when the send is big for the drive.
        let asked_because = if contact.auto_accept {
            match crate::system::disk_space(&self.folder_for(&contact)) {
                Some((free, size)) if total.saturating_add(margin_for(size)) > free || total > free / 2 => {
                    Some(format!("Asked because it's big for the drive: {} is free there.", human(free)))
                }
                _ => None,
            }
        } else {
            None
        };
        let ask = !contact.auto_accept || asked_because.is_some();
        {
            let mut inner = self.inner.lock().unwrap();
            let at = inner.next_seq();
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
                },
            );
        }
        if !ask {
            (self.notify)("direct-incoming", "Files coming in", &format!("{} is sending you {} ({}).", contact.name, name, human(total)), Go::item("direct", peer));
            // Making the files takes a moment: off the connection's task.
            let me = self.clone();
            self.rt.spawn_blocking(move || {
                let _ = me.answer_offer(&id, true, None);
            });
        } else {
            (self.notify)(
                "direct-offer",
                "Files for you",
                &format!("{} wants to send you {} ({}).", contact.name, name, human(total)),
                Go { page: "direct".into(), id: Some(peer.to_string()), offer: Some(id.clone()), ..Go::none() },
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
                let _ = tx.send(Msg::Result { id: id.to_string(), ok: false, message: Some(format!("Their Pako couldn't save it: {e}")) });
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
                    let watch = (t.received.clone(), t.cancel.clone());
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
        let Some((received, cancel)) = started else { return Ok(()) };
        self.watch(id, false, received, cancel);
        let _ = tx.send(Msg::Accept { id: id.to_string() });
        (self.settle)("direct-offer", id, "Accepted");
        self.changed();
        Ok(())
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
                Some((t.handles.get(part.file)?.clone(), t.received.clone(), t.cancel.clone(), t.progress.clone()))
            })
        };
        let Some((file, received, cancel, progress)) = target else {
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
            at += n as u64;
            received.fetch_add(n as u64, Ordering::Relaxed);
            progress.notify_waiters();
        }
    }

    async fn finish_incoming(self: Arc<Self>, id: &str, hashes: Vec<String>, tx: mpsc::UnboundedSender<Msg>) {
        let (received, total, progress, cancel) = {
            let inner = self.inner.lock().unwrap();
            let Some(t) = inner.incoming.get(id) else { return };
            (t.received.clone(), t.total, t.progress.clone(), t.cancel.clone())
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
        let (parts, targets, handles) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.incoming.get_mut(id) else { return };
            t.state = "checking".into();
            (t.parts.clone(), t.targets.clone(), std::mem::take(&mut t.handles))
        };
        drop(handles);
        self.changed();
        if received.load(Ordering::Relaxed) < total || hashes.len() != parts.len() {
            let _ = tx.send(Msg::Result { id: id.into(), ok: false, message: Some("Some parts never arrived.".into()) });
            self.end_incoming(id, "failed", Some("Some parts never arrived.".into()));
            return;
        }
        let check = {
            let parts = parts.clone();
            tokio::task::spawn_blocking(move || hash_files(&parts)).await
        };
        let ok = matches!(&check, Ok(Ok(h)) if *h == hashes);
        if !ok {
            let _ = tx.send(Msg::Result { id: id.into(), ok: false, message: Some("The files didn't arrive intact.".into()) });
            self.end_incoming(id, "failed", Some("The files didn't arrive intact.".into()));
            return;
        }
        // Never replaces a file: if a name was taken meanwhile, the next free one is used.
        let saved_to = self.inner.lock().unwrap().incoming.get(id).and_then(|t| t.saved_to.clone());
        let mut moved_single = None;
        for (i, (p, t)) in parts.iter().zip(targets.iter()).enumerate() {
            match move_new(p, t) {
                Ok(at) => {
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
                    let _ = tx.send(Msg::Result { id: id.into(), ok: false, message: Some("Couldn't save the files.".into()) });
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
        let _ = tx.send(Msg::Result { id: id.into(), ok: true, message: None });
        self.end_incoming(id, "done", None);
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
        let contact_name = self.inner.lock().unwrap().contacts.iter().find(|c| c.id == t.contact).map(|c| c.name.clone()).unwrap_or_default();
        let (duration_ms, avg_speed, peak_speed) = t.meter.summary(t.received.load(Ordering::Relaxed));
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
            message,
            saved_to: if result == "done" { t.saved_to.map(|p| p.display().to_string()) } else { None },
            duration_ms,
            avg_speed,
            peak_speed,
        });
        if result == "done" {
            (self.notify)("direct-received", "Files received", &format!("{} from {} is saved.", t.name, contact_name), Go::item("direct", t.contact.clone()));
        }
        // An offer still waiting for your answer (or one whose accepting failed) ends: its
        // pop-up leaves and its bell line says so. (Accepted ones were settled when accepted.)
        if asked_only || t.state == "preparing" {
            (self.settle)("direct-offer", id, if result == "declined" { "Declined" } else { "No longer waiting" });
        }
        self.changed();
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

async fn send_part(conn: &Connection, path: &Path, part: &Part, sent: &AtomicU64) -> Result<(), String> {
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
fn clean_name(name: &str) -> Option<String> {
    let n: String = name.chars().filter(|c| !c.is_control() && !proto::is_hidden(*c)).collect::<String>().trim().chars().take(60).collect();
    let n = n.trim().to_string();
    if n.is_empty() { None } else { Some(n) }
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
                "Not enough room: this needs {}, {} is free there (Pako always leaves {} free).",
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
    let wide = |p: &Path| p.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (a, b) = (wide(from), wide(to));
    // SAFETY: two valid NUL-terminated paths; no flags = fail if `to` exists.
    let ok = unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), 0) };
    if ok != 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
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
    use iroh::endpoint::presets;

    async fn node(dir: &Path, name: &str) -> Arc<Node> {
        let ep = Endpoint::builder(presets::Minimal)
            .relay_mode(iroh::RelayMode::Disabled)
            .alpns(vec![ALPN.to_vec()])
            .bind_addr("127.0.0.1:0")
            .unwrap()
            .bind()
            .await
            .unwrap();
        std::fs::create_dir_all(dir).unwrap();
        let downloads = dir.join("Downloads");
        let n = Node::new(ep, dir.to_path_buf(), name.into(), Box::new(move || downloads.clone()), Arc::new(|| {}), Arc::new(|_, _, _, _| {}), Arc::new(|_, _, _| {}));
        tokio::spawn(n.clone().accept_loop());
        n
    }

    fn addr(n: &Node) -> EndpointAddr {
        let sock = n.ep.bound_sockets().into_iter().find(|s| s.is_ipv4()).unwrap();
        EndpointAddr::new(n.ep.id()).with_ip_addr(sock)
    }

    async fn until(mut f: impl FnMut() -> bool) {
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
        assert_eq!(margin_for(10 * 1024 * 1024 * 1024), DISK_MARGIN_MIN);
        assert_eq!(margin_for(2_000_000_000_000), 100_000_000_000);
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
}
