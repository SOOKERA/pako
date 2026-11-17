// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! My devices: your own PCs, paired like Direct contacts but both ways as "this is my device".
//!
//! A device is a contact with `device` set (you said so, after comparing the safety code), so it
//! reuses Direct's connection, encryption and trust. It isn't shown among Direct's people.
//! Paired both ways (`device` here, `their_device` said by their Pako):
//! - files between them need no Accept (into that device's usual Direct folder);
//! - with "This device can control my Pako" (`control`, per device, on by default when pairing),
//!   it may see this Pako's download list and give orders. Orders are only: add http(s) links
//!   (into this Pako's own download folder), pause / resume / remove a download of the list, and
//!   send a finished download of the list back. Never a path on disk, never anything run or
//!   opened. Every order (done or refused) goes into the device's history.
//!
//! The permission is looked up at every message, so taking it away works at once. A normal
//! contact can only offer files, as before.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::oneshot;

use super::proto::{self, LinkItems, LinkOpts, Msg, Order, RemoteItem};
use crate::model::Priority;
use super::store::{Contact, DeviceLog, HistoryItem, SentCard, SentLink};
use super::{now_ms, ContactView, Node, RequestView, TransferView};
use crate::notices::Go;

/// Most downloads sent in one list (newest first).
pub const MAX_LIST: usize = 300;
/// Most links in one order.
pub const MAX_LINKS: usize = 200;
/// Longest link taken.
const MAX_LINK: usize = 4096;
/// Orders one device may give in a minute.
const MAX_ORDERS_PER_MINUTE: usize = 60;
/// How long an order may wait for its answer.
const ORDER_TIMEOUT: Duration = Duration::from_secs(60);
/// How often a watched list is looked at (sent only when it changed).
const LIST_EVERY: Duration = Duration::from_millis(1000);
/// A contact saying again and again "this PC is one of my devices" is notified once per this.
const PAIR_NOTICE_EVERY: Duration = Duration::from_secs(600);

/// What a device's orders act on: this Pako's downloads (the engine, see `direct/mod.rs`).
pub trait Host: Send + Sync {
    /// The download list, newest first (speed left at 0: it's measured here).
    fn list(&self) -> Vec<RemoteItem>;
    /// Adds checked http(s) links (own folder, own settings): a file's link to the downloads
    /// with its priority; a video's at once in the quality the sending PC picked (a playlist
    /// to Links); a link page's, or a video's from an older Pako, to Links (opened there like
    /// a pasted link). Says what was taken and which links weren't.
    fn add(&self, links: Vec<ToAdd>) -> Pin<Box<dyn Future<Output = Added> + Send>>;
    fn pause(&self, item: &str);
    /// First, Normal or Later, through the same path as the Downloads menu.
    fn set_priority(&self, item: &str, priority: Priority);
    fn resume(&self, item: &str);
    /// Off the list only: a finished file stays on the disk.
    fn remove(&self, item: &str);
    /// A finished download's file, if it's finished and still there.
    fn file_of(&self, item: &str) -> Option<PathBuf>;
}

/// One link of an order, as this PC takes it.
#[derive(Debug, Clone, PartialEq)]
pub struct ToAdd {
    pub link: String,
    /// A video's height in px (0 = audio only); None = not said (this PC's default).
    pub quality: Option<u32>,
    pub priority: Priority,
    /// Sent by a Pako that picks videos' quality: a video's link downloads at once.
    pub newer: bool,
    /// A subfolder of this PC's own download folder, as sent (checked by `clean_subfolder`
    /// and `folder_inside` before use; anything unsafe means the download folder itself).
    pub folder: Option<String>,
}

/// Longest subfolder name taken from an order (characters, all levels).
pub const MAX_FOLDER: usize = 200;
/// Deepest subfolder taken from an order ("Playlist/Part 2").
pub const MAX_FOLDER_LEVELS: usize = 2;

/// A subfolder name from another PC, cleaned like a file name, or None when it isn't safe:
/// relative only ("/" between levels, at most 2), no "..", no drive letter, no "\\server",
/// no leading slash, not too long. Reserved names (CON, NUL…) get a "_" and trailing dots and
/// spaces go, as for any received name.
pub fn clean_subfolder(raw: &str) -> Option<Vec<String>> {
    let raw = raw.trim();
    if raw.is_empty() || raw.chars().count() > MAX_FOLDER || raw.starts_with('/') || raw.starts_with('\\') || raw.contains('\\') || raw.contains(':') {
        return None;
    }
    let parts = proto::safe_rel(raw)?;
    (parts.len() <= MAX_FOLDER_LEVELS).then_some(parts)
}

/// A subfolder name as this PC sends it (typed in **Into folder**, or a playlist's or Drive
/// folder's name): "\" counts as "/" (Windows habit), each level is kept short enough for the
/// two to fit, and anything the other PC would refuse is refused here, saying why, instead of
/// the files silently landing in its download folder itself. None: no folder.
pub fn send_folder(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = raw.map(str::trim).filter(|r| !r.is_empty()) else { return Ok(None) };
    let refuse = || {
        Err(format!(
            "This folder name can't be used there: {}. Give a folder inside its download folder, like Course or Course/Week 3 (no \"..\", no drive or \":\", two levels at most).",
            short(raw)
        ))
    };
    if raw.starts_with('/') || raw.starts_with('\\') {
        return refuse();
    }
    let per_level = MAX_FOLDER / MAX_FOLDER_LEVELS - 1;
    let unified = raw.replace('\\', "/");
    let parts: Vec<String> = unified.split('/').map(str::trim).filter(|p| !p.is_empty()).map(|p| p.chars().take(per_level).collect()).collect();
    match clean_subfolder(&parts.join("/")) {
        Some(p) => Ok(Some(p.join("/"))),
        None => refuse(),
    }
}

/// Makes the subfolder inside `base` (this PC's download folder), one level at a time, and
/// checks each level once it exists: its real place must still be inside `base` (a junction
/// or a link could lead elsewhere; nothing is made past one). None: not safe, use `base`.
pub fn folder_inside(base: &std::path::Path, parts: &[String]) -> Option<PathBuf> {
    if parts.is_empty() {
        return None;
    }
    std::fs::create_dir_all(base).ok()?;
    let real_base = std::fs::canonicalize(base).ok()?;
    let mut dir = base.to_path_buf();
    for p in parts {
        dir.push(p);
        if !dir.is_dir() {
            std::fs::create_dir(&dir).ok()?;
        }
        let real = std::fs::canonicalize(&dir).ok()?;
        if real == real_base || !real.starts_with(&real_base) {
            return None;
        }
    }
    Some(dir)
}

/// Where a link from another PC is saved: `base`, or the subfolder it named inside it. The
/// bool says a folder was named but wasn't safe (the history notes it).
pub fn place_for(base: &std::path::Path, folder: Option<&str>) -> (PathBuf, bool) {
    let Some(raw) = folder.filter(|f| !f.trim().is_empty()) else { return (base.to_path_buf(), false) };
    match clean_subfolder(raw).and_then(|parts| folder_inside(base, &parts)) {
        Some(dir) => (dir, false),
        None => (base.to_path_buf(), true),
    }
}

/// Highest video quality taken from an order (8K); anything above is ignored.
pub const MAX_QUALITY: u32 = 4320;

/// "first", "normal", "later" as an order says it (anything else: None).
pub fn parse_priority(s: &str) -> Option<Priority> {
    match s {
        "first" => Some(Priority::First),
        "normal" => Some(Priority::Normal),
        "later" => Some(Priority::Later),
        _ => None,
    }
}

/// A priority as an order says it; None for Normal (left out).
pub fn priority_word(p: Priority) -> Option<String> {
    match p {
        Priority::First => Some("first".into()),
        Priority::Normal => None,
        Priority::Later => Some("later".into()),
    }
}

fn priority_label(p: Priority) -> &'static str {
    match p {
        Priority::First => "First",
        Priority::Normal => "Normal",
        Priority::Later => "Later",
    }
}

/// The checked links of an order, each with what the sending PC picked for it (matched by
/// the link). A quality out of range or an unknown priority is ignored, not refused.
pub fn to_add(links: Vec<String>, opts: &[LinkOpts]) -> Vec<ToAdd> {
    let newer = !opts.is_empty();
    let opts = &opts[..opts.len().min(MAX_LINKS)];
    links
        .into_iter()
        .map(|link| {
            let o = opts.iter().find(|o| o.link.trim() == link);
            ToAdd {
                folder: o.and_then(|o| o.folder.clone()).filter(|f| !f.trim().is_empty()),
                quality: o.and_then(|o| o.quality).filter(|q| *q <= MAX_QUALITY),
                priority: o.and_then(|o| o.priority.as_deref()).and_then(parse_priority).unwrap_or_default(),
                newer,
                link,
            }
        })
        .collect()
}

/// What goes with checked links in an order: only for links that were kept, only when it
/// says something (an empty list is left out of the order).
fn opts_for(checked: &[String], opts: Vec<LinkOpts>) -> Vec<LinkOpts> {
    let mut out: Vec<LinkOpts> = Vec::new();
    for o in opts {
        let link = o.link.trim().to_string();
        if !checked.contains(&link) || out.iter().any(|x| x.link == link) {
            continue;
        }
        let quality = o.quality.filter(|q| *q <= MAX_QUALITY);
        let priority = o.priority.as_deref().and_then(parse_priority).and_then(priority_word);
        // Only a subfolder name travels, never a path of this PC (checked there too).
        let folder = o.folder.as_deref().and_then(clean_subfolder).map(|p| p.join("/"));
        if quality.is_some() || priority.is_some() || folder.is_some() {
            out.push(LinkOpts { link, quality, priority, folder });
        }
    }
    out
}

/// A link that is never sent to another PC as it is: a playlist (its videos are picked here
/// first) or a link page Pako must click through (it's opened here first). Says why.
pub fn open_here_first(link: &str) -> Option<&'static str> {
    if crate::video::is_playlist_link(link) {
        return Some("A playlist: open it in Links first to pick the videos.");
    }
    let gate = url::Url::parse(link).ok().and_then(|u| u.host_str().map(crate::gates::is_known_gate)).unwrap_or(false);
    gate.then_some("A link page: Pako opens it in Links first, then sends the file.")
}

/// What adding a device's links on this PC gave.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Added {
    /// Downloads added (a Drive folder's link adds one per file).
    pub downloads: usize,
    /// Links put in Links instead: a video site's page (its quality is picked there) or a link
    /// page Pako works through first.
    pub to_links: usize,
    /// The links not taken, with why.
    pub failed: Vec<(String, String)>,
    /// Links whose folder name wasn't safe: saved in the download folder itself.
    pub unsafe_folders: usize,
    /// The downloads each link became (their ids), for the PC that sent them.
    pub items: Vec<(String, Vec<String>)>,
}

impl Added {
    /// What happened, for the history and the PC that sent them ("Added 2 links to the
    /// downloads, 1 to Links"), or why nothing was taken.
    fn text(&self) -> Result<String, String> {
        let why = |f: &(String, String)| format!("{}: {}", short(&f.0), short_line(&f.1, 200));
        if self.downloads + self.to_links == 0 {
            return Err(match self.failed.as_slice() {
                [] => "Nothing was added.".into(),
                [one] => why(one),
                [first, rest @ ..] => format!("{} (and {} more)", why(first), rest.len()),
            });
        }
        let mut t = match (self.downloads, self.to_links) {
            (d, 0) => format!("Added {} to the downloads", plural(d, "link")),
            (0, l) => format!("Added {} to Links", plural(l, "link")),
            (d, l) => format!("Added {} to the downloads, {l} to Links", plural(d, "link")),
        };
        if let Some(first) = self.failed.first() {
            t.push_str(&format!(". Not taken: {} ({})", self.failed.len(), why(first)));
        }
        if self.unsafe_folders > 0 {
            t.push_str(". A folder name wasn't safe: saved in the download folder itself");
        }
        Ok(t)
    }
}

/// An order's answer: done or not, what the other PC said, and (for links) the ones it didn't
/// take and the downloads the others became.
type Answer = (bool, Option<String>, Vec<String>, Vec<LinkItems>);

/// Most download ids taken back from one answer (a Drive folder's link makes one per file).
const MAX_REPLY_IDS: usize = 5000;

/// The downloads each sent link became, as the other PC said: only links that were sent, only
/// ids as Pako makes them, not too many.
pub fn clean_items(sent: &[String], items: Vec<LinkItems>) -> Vec<LinkItems> {
    let mut n = 0;
    let mut out: Vec<LinkItems> = Vec::new();
    for it in items {
        if !sent.contains(&it.link) || out.iter().any(|o| o.link == it.link) {
            continue;
        }
        // Each id once: the card lists a link's downloads by their id.
        let mut seen = std::collections::HashSet::new();
        let ids: Vec<String> = it.ids.into_iter().filter(|i| valid_item(i) && seen.insert(i.clone())).take(MAX_REPLY_IDS.saturating_sub(n)).collect();
        n += ids.len();
        out.push(LinkItems { link: it.link, ids });
    }
    out
}

/// Device parts of the node's shared state.
#[derive(Default)]
pub struct DevInner {
    /// Devices watching this Pako's list, and the last list sent to each (a hash).
    watchers: HashMap<String, u64>,
    /// Devices whose list this Pako wants (re-asked when they reconnect).
    watching: HashSet<String>,
    /// The last list each device sent, and when.
    remote: HashMap<String, (Vec<RemoteItem>, u64)>,
    /// Orders sent, waiting for their answer: (device, answer).
    orders: HashMap<String, (String, oneshot::Sender<Answer>)>,
    /// When each device's orders of the last minute came in.
    order_times: HashMap<String, std::collections::VecDeque<Instant>>,
    pub log: Vec<DeviceLog>,
    /// Links sent to your devices (My devices › Links), oldest first.
    pub sent: Vec<SentCard>,
    /// The receiving folder picked while answering a pairing request, until it's kept.
    request_folders: HashMap<String, String>,
    /// When "… says this PC is one of its devices" was last notified, per contact.
    pair_noticed: HashMap<String, Instant>,
    /// This PC's name for your devices ("Desk PC"), apart from your name for Direct's people.
    /// Empty until you set it.
    pub pc_name: String,
}

impl DevInner {
    /// The folder picked for a device whose pairing request was just accepted.
    pub fn folder_for_request(&mut self, peer: &str) -> Option<String> {
        self.request_folders.remove(peer)
    }
}

/// Device parts of the node that are set once.
#[derive(Default)]
pub struct DevNode {
    host: OnceLock<Arc<dyn Host>>,
    /// Tells the window a watched list changed ("devices-changed").
    listed: OnceLock<super::Changed>,
}

// ---------- the checks (pure, so they can be tested) ----------

/// Paired both ways: files need no Accept.
pub fn paired(c: &Contact) -> bool {
    c.device && c.their_device
}

/// May this contact (None = not a contact) see this Pako's downloads and give orders?
pub fn may_control(c: Option<&Contact>) -> Result<(), &'static str> {
    let Some(c) = c else { return Err("Only your own devices can do that.") };
    if !c.device {
        return Err("Only your own devices can do that.");
    }
    if !c.their_device {
        return Err("This device isn't paired both ways.");
    }
    if !c.control {
        return Err("This device isn't allowed to control that Pako.");
    }
    Ok(())
}

/// Links from an order, checked like pasted links: http(s) only, each one a whole link as the
/// link finder reads it. Any bad one refuses the order (nothing half done).
pub fn check_links(links: &[String]) -> Result<Vec<String>, String> {
    if links.is_empty() {
        return Err("Paste at least one link.".into());
    }
    if links.len() > MAX_LINKS {
        return Err(format!("Up to {MAX_LINKS} links at once."));
    }
    let mut out: Vec<String> = Vec::new();
    for l in links {
        let l = l.trim();
        let bad = || format!("Not a web link: {}", short(l));
        if l.len() > MAX_LINK {
            return Err("A link is far too long.".into());
        }
        let url = url::Url::parse(l).map_err(|_| bad())?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none_or(|h| h.is_empty()) {
            return Err(bad());
        }
        // The same reading as pasted text: exactly this one link.
        if crate::engine::extract_links(l) != [l.to_string()] {
            return Err(bad());
        }
        // Never this PC itself: another PC's order can't reach what only listens here
        // (a local program's page), then take it with "Get this file".
        if is_this_pc(&url) {
            return Err(format!("Links to the PC itself (localhost) aren't taken from another PC: {}", short(l)));
        }
        if !out.iter().any(|o| o == l) {
            out.push(l.to_string());
        }
    }
    Ok(out)
}

// A link that names the PC it's opened on. One that only leads there (a redirect, a name
// that stands for 127.0.0.1) is refused when its answer comes: see `fetch::from_this_pc`.
use crate::fetch::is_this_pc;

/// Every link found in pasted text (the window's "Download on this device" box).
pub fn links_in(text: &str) -> Vec<String> {
    crate::engine::extract_links(text)
}

/// A download id as Pako makes them: anything else is refused before looking.
pub fn valid_item(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn short(s: &str) -> String {
    let s: String = s.chars().filter(|c| !proto::is_hidden(*c) && !c.is_control()).collect();
    if s.chars().count() > 60 { format!("{}…", s.chars().take(59).collect::<String>()) } else { s }
}

/// A list from another device, made safe to show: not too long, names without hidden characters.
pub fn clean_list(mut items: Vec<RemoteItem>) -> Vec<RemoteItem> {
    items.truncate(MAX_LIST);
    let line = |s: &str, max: usize| -> String { s.chars().filter(|c| !proto::is_hidden(*c) && !c.is_control()).take(max).collect() };
    items
        .into_iter()
        .filter(|i| valid_item(&i.id))
        .map(|i| RemoteItem {
            name: line(&i.name, 300),
            status: line(&i.status, 20),
            message: i.message.map(|m| line(&m, 300)).filter(|m| !m.trim().is_empty()),
            done: i.done.min(i.size.max(i.done)),
            ..i
        })
        .collect()
}

fn list_hash(items: &[RemoteItem]) -> u64 {
    let b = serde_json::to_vec(items).unwrap_or_default();
    let h = blake3::hash(&b);
    u64::from_le_bytes(h.as_bytes()[..8].try_into().expect("8 bytes"))
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") }
}

// ---------- the window's view ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceView {
    #[serde(flatten)]
    pub view: ContactView,
    /// Both sides said "this is my device".
    pub paired: bool,
    /// One of your contacts says this PC is its device: confirm it here (or not).
    pub asks_to_pair: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteView {
    pub items: Vec<RemoteItem>,
    /// When it came (ms).
    pub at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicesState {
    pub my_id: String,
    pub my_name: String,
    /// This PC's name, as your devices see it (empty until set).
    pub pc_name: String,
    pub internet: bool,
    pub devices: Vec<DeviceView>,
    /// Someone new asks to pair as your device (answered like a connection request).
    pub requests: Vec<RequestView>,
    pub transfers: Vec<TransferView>,
    pub history: Vec<HistoryItem>,
    pub log: Vec<DeviceLog>,
    /// Links sent to your devices from this PC, a card per send (oldest first).
    pub sent: Vec<SentCard>,
    /// The download list each device sent (only while you look and it lets you).
    pub remote: HashMap<String, RemoteView>,
    /// Direct relays through your devices, both ways ("Relaying 2 files to Rado").
    pub relays: Vec<super::relay::RelayView>,
}

impl Node {
    /// Where device orders act (Pako's engine). Starts sending lists to devices that watch.
    pub fn set_host(self: &Arc<Self>, host: Arc<dyn Host>, listed: super::Changed) {
        if self.dev.host.set(host).is_err() {
            return;
        }
        let _ = self.dev.listed.set(listed);
        let me = self.clone();
        self.rt.spawn(async move { me.list_loop().await });
    }

    fn listed(&self) {
        match self.dev.listed.get() {
            Some(f) => f(),
            None => self.changed(),
        }
    }

    pub fn devices_state(&self) -> DevicesState {
        let full = self.full_state();
        let inner = self.inner.lock().unwrap();
        let is_dev = |id: &str| inner.contacts.iter().any(|c| c.id == id && c.device);
        let devices = full
            .contacts
            .into_iter()
            .filter(|v| v.contact.device || v.contact.their_device)
            .map(|v| DeviceView { paired: paired(&v.contact), asks_to_pair: !v.contact.device && v.contact.their_device, view: v })
            .collect();
        DevicesState {
            my_id: full.my_id,
            my_name: full.my_name,
            pc_name: inner.dev.pc_name.clone(),
            internet: full.internet,
            devices,
            requests: full.requests.into_iter().filter(|r| r.device).collect(),
            transfers: full.transfers.into_iter().filter(|t| is_dev(&t.contact)).collect(),
            history: full.history.into_iter().filter(|h| is_dev(&h.contact)).collect(),
            log: inner.dev.log.clone(),
            sent: inner.dev.sent.iter().filter(|c| is_dev(&c.device)).cloned().collect(),
            remote: inner.dev.remote.iter().map(|(k, (items, at))| (k.clone(), RemoteView { items: items.clone(), at: *at })).collect(),
            relays: full.relays,
        }
    }

    pub(super) fn log(&self, device: &str, by_me: bool, text: String, ok: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.dev.log.push(DeviceLog { device: device.to_string(), date: now_ms(), by_me, text, ok });
        let mut l = std::mem::take(&mut inner.dev.log);
        self.store.save_device_log(&mut l);
        inner.dev.log = l;
    }

    pub fn clear_device_log(&self, device: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.dev.log.retain(|l| l.device != device);
        let mut l = std::mem::take(&mut inner.dev.log);
        self.store.save_device_log(&mut l);
        inner.dev.log = l;
        drop(inner);
        self.changed();
    }

    /// Tells a connected contact how this Pako counts them (device, control).
    fn tell_device(&self, id: &str) {
        let inner = self.inner.lock().unwrap();
        let Some(c) = inner.contacts.iter().find(|c| c.id == id) else { return };
        // Plain contacts on both sides: nothing to say (older Pakos would ignore it anyway).
        if !c.device && !c.their_device {
            return;
        }
        let pc_name = Some(inner.dev.pc_name.clone()).filter(|n| !n.is_empty());
        let control = c.device && c.control;
        // Only to a device this PC lets control it: Direct's name and how many contacts (it
        // can then show them and send through this PC).
        let relay = control.then(|| self.relay_from(&inner));
        let m = Msg::Device { mine: c.device, control, pc_name, relay };
        if let Some(l) = inner.live.get(id) {
            let _ = l.tx.send(m);
        }
    }

    /// Tells every connected device how this PC counts it (after Direct's name or the number
    /// of contacts changed: the "From" menu there shows them).
    pub(super) fn tell_devices(&self) {
        let ids: Vec<String> = self.inner.lock().unwrap().contacts.iter().filter(|c| c.device).map(|c| c.id.clone()).collect();
        for id in ids {
            self.tell_device(&id);
        }
    }

    // ---------- this PC's name ----------

    /// This PC has a name for your devices.
    pub fn has_pc_name(&self) -> bool {
        !self.inner.lock().unwrap().dev.pc_name.is_empty()
    }

    /// Names this PC for your devices ("Desk PC"); connected devices see it at once, the others
    /// when they next connect. Direct's name for people doesn't change. Returns the name kept;
    /// a name with nothing left once cleaned changes nothing.
    pub fn set_pc_name(&self, name: &str) -> Result<String, String> {
        let name = super::clean_name(name).ok_or("Give this PC a name (like Desk PC).")?;
        let ids: Vec<String> = {
            let mut inner = self.inner.lock().unwrap();
            if inner.dev.pc_name == name {
                return Ok(name);
            }
            inner.dev.pc_name = name.clone();
            inner.contacts.iter().filter(|c| c.device || c.their_device).map(|c| c.id.clone()).collect()
        };
        for id in ids {
            self.tell_device(&id);
        }
        self.changed();
        Ok(name)
    }

    /// A device said its PC's name: your name for it follows, unless you renamed it here. A
    /// contact you didn't pair as your device keeps the name of the person (only the PC's name
    /// is kept, for when you pair it): a contact can't take a device's name on the question
    /// "… says this PC is one of its devices".
    pub(super) fn remember_pc_name(&self, peer: &str, said: &str) {
        let mut inner = self.inner.lock().unwrap();
        let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) else { return };
        if !c.device {
            let Some(pc) = super::clean_name(said).filter(|pc| c.their_pc_name.as_ref() != Some(pc)) else { return };
            c.their_pc_name = Some(pc);
            self.save_contacts(&inner);
            return;
        }
        let Some((name, pc)) = super::updated_pc_name(&c.name, c.their_name.as_deref(), c.their_pc_name.as_deref(), said) else { return };
        c.name = name;
        c.their_pc_name = Some(pc);
        self.save_contacts(&inner);
        drop(inner);
        self.changed();
    }

    /// Your devices this PC can send links to now: paired both ways, letting this PC control
    /// them, and connected. (id, name), by name.
    pub fn device_targets(&self) -> Vec<(String, String)> {
        let inner = self.inner.lock().unwrap();
        let mut out: Vec<(String, String)> =
            inner.contacts.iter().filter(|c| paired(c) && c.their_control && inner.live.contains_key(&c.id)).map(|c| (c.id.clone(), c.name.clone())).collect();
        out.sort_by_key(|(_, n)| n.to_lowercase());
        out
    }

    /// Your devices a link can be set to download on (Links › "Download on"): paired both
    /// ways and letting this PC control them, online or not. By name.
    pub fn device_choices(&self) -> Vec<crate::engine::DeviceChoice> {
        let inner = self.inner.lock().unwrap();
        let mut out: Vec<_> = inner
            .contacts
            .iter()
            .filter(|c| paired(c) && c.their_control)
            .map(|c| crate::engine::DeviceChoice { id: c.id.clone(), name: c.name.clone(), online: inner.live.contains_key(&c.id) })
            .collect();
        out.sort_by_key(|d| d.name.to_lowercase());
        out
    }

    // ---------- pairing ----------

    /// "This is my device": pairs a PC by its Pako ID (new, or a contact you already have, or
    /// one asking to pair). It may control this Pako until you turn that off. The other PC
    /// confirms too, with the same safety code.
    /// `folder`: where its files are saved, and only there (None = `<download folder>\From <name>`).
    pub fn add_device(self: &Arc<Self>, id: &str, name: Option<&str>, folder: Option<String>) -> Result<(), String> {
        let id = id.trim();
        let parsed: iroh::EndpointId = id.parse().map_err(|_| "That isn't a Pako ID. Copy it again from the other PC (My devices › Add a device).".to_string())?;
        let id = parsed.to_string();
        if id == self.my_id() {
            return Err("That's this PC's own Pako ID. Enter the other PC's.".into());
        }
        let name = name.and_then(super::clean_name);
        {
            let mut inner = self.inner.lock().unwrap();
            self.unblock(&mut inner, &id);
            match inner.contacts.iter_mut().find(|c| c.id == id) {
                Some(c) if c.device => return Err("This PC is already one of your devices.".into()),
                Some(c) => {
                    c.device = true;
                    c.control = true;
                    c.declined = false;
                    // My devices can't be muted (and My devices has no Unmute).
                    c.muted = false;
                    if let Some(n) = name {
                        c.name = n;
                    } else if let Some(pc) = c.their_pc_name.clone() {
                        // A contact that said it's your PC goes by its PC's name now, unless
                        // you had renamed it.
                        if c.name == super::PLACEHOLDER || c.their_name.as_deref() == Some(c.name.as_str()) {
                            c.name = pc;
                        }
                    }
                }
                None => {
                    let name = name.ok_or("Give this device a name (like Laptop or Home PC).")?;
                    inner.contacts.push(Contact { id: id.clone(), name, added: now_ms(), device: true, control: true, ..Default::default() });
                }
            }
            // Its receiving folder, kept as a path (renaming the device later doesn't move it).
            if let Some(c) = inner.contacts.iter_mut().find(|c| c.id == id) {
                let f = folder.filter(|f| !f.trim().is_empty()).unwrap_or_else(|| self.folder_for(&Contact { save_folder: None, ..c.clone() }).display().to_string());
                c.save_folder = Some(f);
            }
            self.save_contacts(&inner);
        }
        self.log(&id, true, "Paired as your device".into(), true);
        // (A contact that became a device: one contact fewer in Direct.)
        self.tell_devices();
        self.changed();
        self.reach(&id);
        Ok(())
    }

    /// The suggested receiving folder for a device of this name: `<download folder>\From <name>`.
    pub fn default_folder(&self, name: &str) -> String {
        let name = super::clean_name(name).unwrap_or_else(|| "my device".into());
        self.folder_for(&Contact { name, ..Default::default() }).display().to_string()
    }

    /// Answers "… wants to pair as your device" (in Pako's window, after comparing the code).
    /// `folder`: where its files will be saved (None = the suggested one).
    pub fn answer_device_request(&self, peer: &str, accept: bool, folder: Option<String>) {
        if accept {
            let mut inner = self.inner.lock().unwrap();
            let name = inner.requests.get(peer).map(|r| r.name.clone()).unwrap_or_default();
            let f = folder.filter(|f| !f.trim().is_empty()).unwrap_or_else(|| self.default_folder(&name));
            inner.dev.request_folders.insert(peer.to_string(), f);
        }
        self.answer_request(peer, accept);
    }

    /// Where a device's files are saved (and only there). Never set by the other device.
    pub fn set_device_folder(&self, id: &str, folder: &str) -> Result<(), String> {
        let folder = folder.trim();
        if folder.is_empty() || !std::path::Path::new(folder).is_absolute() {
            return Err("Pick a folder.".into());
        }
        {
            let mut inner = self.inner.lock().unwrap();
            let c = inner.contacts.iter_mut().find(|c| c.id == id && c.device).ok_or("Device not found.")?;
            c.save_folder = Some(folder.to_string());
            self.save_contacts(&inner);
        }
        self.changed();
        Ok(())
    }

    /// "This device can control my Pako", per device. Off takes effect at once: its next
    /// message is refused and it stops getting the list.
    pub fn set_control(&self, id: &str, on: bool) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().unwrap();
            let c = inner.contacts.iter_mut().find(|c| c.id == id && c.device).ok_or("Device not found.")?;
            if c.control == on {
                return Ok(());
            }
            c.control = on;
            self.save_contacts(&inner);
            if !on {
                inner.dev.watchers.remove(id);
            }
        }
        self.log(id, true, if on { "Allowed to control this Pako".into() } else { "No longer allowed to control this Pako".into() }, true);
        self.tell_device(id);
        // Direct relays it asked of this PC stop at once (none of its files go on).
        if !on {
            self.relay_sweep();
        }
        self.changed();
        Ok(())
    }

    /// Called once a connection is kept: say how we count them, and ask for their list again
    /// if the window looks at it.
    pub(super) fn device_hello(self: &Arc<Self>, id: &str) {
        self.tell_device(id);
        self.relay_hello(id);
        let inner = self.inner.lock().unwrap();
        if inner.dev.watching.contains(id) {
            if let Some(l) = inner.live.get(id) {
                let _ = l.tx.send(Msg::Watch { on: true });
            }
        }
    }

    /// The connection ended (or the contact was removed): their list and orders go.
    pub(super) fn device_gone(&self, id: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.dev.watchers.remove(id);
        inner.dev.remote.remove(id);
        let ids: Vec<String> = inner.dev.orders.iter().filter(|(_, (d, _))| d == id).map(|(k, _)| k.clone()).collect();
        for k in ids {
            if let Some((_, a)) = inner.dev.orders.remove(&k) {
                let _ = a.send((false, Some("The connection was lost.".into()), Vec::new(), Vec::new()));
            }
        }
    }

    /// "You're one of my devices" (or no longer), from their Pako.
    pub(super) fn on_device(&self, peer: &str, mine: bool, control: bool, pc_name: Option<String>, relay: Option<proto::RelayFrom>) {
        if let (true, Some(pc)) = (mine, pc_name.as_deref()) {
            self.remember_pc_name(peer, pc);
        }
        let asks = {
            let mut inner = self.inner.lock().unwrap();
            let Some(c) = inner.contacts.iter_mut().find(|c| c.id == peer) else { return };
            let newly = mine && !c.their_device;
            // What it says of its Direct side (shown in the "From" menu), cleaned.
            let relay = relay.filter(|_| mine && control).map(|r| proto::RelayFrom { name: super::clean_name(&r.name).unwrap_or_default(), contacts: r.contacts.min(1_000_000) });
            let changed = c.their_device != mine || c.their_control != (mine && control) || c.their_relay != relay;
            c.their_device = mine;
            c.their_control = mine && control;
            c.their_relay = relay;
            let asks = newly && !c.device;
            let name = c.name.clone();
            if changed {
                self.save_contacts(&inner);
            }
            if !mine || !control {
                inner.dev.remote.remove(peer);
            }
            if !mine {
                inner.dev.watchers.remove(peer);
            }
            // Said, taken back and said again: notified once in a while, not each time.
            let quiet = inner.dev.pair_noticed.get(peer).is_some_and(|t| t.elapsed() < PAIR_NOTICE_EVERY);
            if asks && !quiet {
                inner.dev.pair_noticed.insert(peer.to_string(), Instant::now());
            }
            (asks && !quiet).then_some(name)
        };
        // (Not from a contact you muted: it still shows in My devices.)
        if let Some(name) = asks {
            self.tell(
                peer,
                "direct-request",
                "Pair a device",
                &format!("{name} says this PC is one of its devices. Confirm it in My devices."),
                Go::item("devices", peer),
            );
        }
        self.changed();
    }

    // ---------- the download list ----------

    /// The window looks at a device's downloads (or stops).
    pub fn device_watch(&self, id: &str, on: bool) {
        let mut inner = self.inner.lock().unwrap();
        if on {
            inner.dev.watching.insert(id.to_string());
        } else {
            inner.dev.watching.remove(id);
        }
        if let Some(l) = inner.live.get(id) {
            let _ = l.tx.send(Msg::Watch { on });
        }
    }

    pub(super) fn on_watch(&self, peer: &str, on: bool) {
        let mut inner = self.inner.lock().unwrap();
        let ok = may_control(inner.contacts.iter().find(|c| c.id == peer)).is_ok();
        if on && ok {
            // 0: the next round sends it at once.
            inner.dev.watchers.insert(peer.to_string(), 0);
        } else {
            inner.dev.watchers.remove(peer);
        }
    }

    pub(super) fn on_list(&self, peer: &str, items: Vec<RemoteItem>) {
        {
            let mut inner = self.inner.lock().unwrap();
            // Only from your devices that let you see it, and only while you asked.
            let ok = inner.contacts.iter().any(|c| c.id == peer && paired(c) && c.their_control) && inner.dev.watching.contains(peer);
            if !ok {
                return;
            }
            inner.dev.remote.insert(peer.to_string(), (clean_list(items), now_ms()));
        }
        self.listed();
    }

    /// Sends the list to the devices watching it, when it changed.
    async fn list_loop(self: Arc<Self>) {
        let mut seen: HashMap<String, (u64, Instant)> = HashMap::new();
        let mut speeds: HashMap<String, u64> = HashMap::new();
        loop {
            tokio::select! {
                _ = self.stop.cancelled() => return,
                _ = tokio::time::sleep(LIST_EVERY) => {}
            }
            let watchers: Vec<String> = {
                let mut inner = self.inner.lock().unwrap();
                // The permission is checked again every round: taken away, nothing more goes.
                let allowed: Vec<String> =
                    inner.dev.watchers.keys().filter(|w| inner.live.contains_key(*w) && may_control(inner.contacts.iter().find(|c| &c.id == *w)).is_ok()).cloned().collect();
                inner.dev.watchers.retain(|w, _| allowed.contains(w));
                allowed
            };
            if watchers.is_empty() {
                seen.clear();
                continue;
            }
            let Some(host) = self.dev.host.get().cloned() else { continue };
            let mut items = match tokio::task::spawn_blocking(move || host.list()).await {
                Ok(l) => l,
                Err(_) => continue,
            };
            items.truncate(MAX_LIST);
            // Speed: what came since the last look.
            let now = Instant::now();
            let mut next = HashMap::new();
            for i in items.iter_mut() {
                if let Some((before, at)) = seen.get(&i.id) {
                    let dt = now.duration_since(*at).as_secs_f64();
                    if i.status == "downloading" && dt > 0.2 {
                        let fresh = (i.done.saturating_sub(*before) as f64 / dt) as u64;
                        // Smoothed a little, like the window's own speeds.
                        let old = speeds.get(&i.id).copied().unwrap_or(fresh);
                        i.speed = (old + fresh * 2) / 3;
                    }
                }
                next.insert(i.id.clone(), (i.done, now));
            }
            speeds = items.iter().map(|i| (i.id.clone(), i.speed)).collect();
            seen = next;
            let hash = list_hash(&items);
            let inner = self.inner.lock().unwrap();
            let sends: Vec<_> = watchers
                .iter()
                .filter(|w| inner.dev.watchers.get(*w).is_some_and(|h| *h != hash))
                .filter_map(|w| inner.live.get(w).map(|l| (w.clone(), l.tx.clone())))
                .collect();
            drop(inner);
            for (w, tx) in sends {
                if tx.send(Msg::List { items: items.clone() }).is_ok() {
                    if let Some(h) = self.inner.lock().unwrap().dev.watchers.get_mut(&w) {
                        *h = hash;
                    }
                }
            }
        }
    }

    // ---------- orders ----------

    /// Gives an order to one of your devices and waits for its answer (what happened there).
    pub async fn device_order(self: &Arc<Self>, id: &str, order: Order) -> Result<String, String> {
        let order = match order {
            Order::Add { links, opts } => {
                let links = check_links(&links)?;
                Order::Add { opts: opts_for(&links, opts), links }
            }
            o => o,
        };
        self.device_order_answer(id, order).await.map(|(text, _, _)| text)
    }

    /// "Download on <device>": these links go to one of your devices, each with its quality
    /// and priority. What happened there, and the links it didn't take (it took the others),
    /// as they were sent.
    pub async fn device_add(self: &Arc<Self>, id: &str, links: Vec<LinkOpts>) -> Result<(String, Vec<String>), String> {
        self.device_add_named(id, links, &HashMap::new()).await
    }

    /// Like `device_add`, with the names known here (a playlist's videos, files) for the card
    /// My devices › Links keeps of this send.
    pub async fn device_add_named(self: &Arc<Self>, id: &str, links: Vec<LinkOpts>, names: &HashMap<String, String>) -> Result<(String, Vec<String>), String> {
        let checked = check_links(&links.iter().map(|l| l.link.clone()).collect::<Vec<_>>())?;
        let opts = opts_for(&checked, links);
        let (text, failed, items) = match self.device_order_answer(id, Order::Add { links: checked.clone(), opts: opts.clone() }).await {
            Ok(a) => a,
            Err(e) => return Err(e),
        };
        // Only links that were sent (the other PC can say anything).
        let failed: Vec<String> = failed.into_iter().filter(|f| checked.contains(f)).collect();
        let items = clean_items(&checked, items);
        let card = SentCard {
            id: super::new_id(),
            device: id.to_string(),
            date: now_ms(),
            text: text.clone(),
            links: checked
                .iter()
                .map(|l| {
                    let o = opts.iter().find(|o| &o.link == l);
                    SentLink {
                        link: l.clone(),
                        name: names.get(l).map(|n| short_line(n, 300)).filter(|n| !n.is_empty()),
                        quality: o.and_then(|o| o.quality),
                        priority: o.and_then(|o| o.priority.clone()),
                        folder: o.and_then(|o| o.folder.clone()),
                        ids: items.iter().find(|i| &i.link == l).map(|i| i.ids.clone()).unwrap_or_default(),
                        failed: failed.contains(l),
                    }
                })
                .collect(),
        };
        {
            let mut inner = self.inner.lock().unwrap();
            inner.dev.sent.push(card);
            let mut l = std::mem::take(&mut inner.dev.sent);
            self.store.save_sent(&mut l);
            inner.dev.sent = l;
        }
        self.changed();
        Ok((text, failed))
    }

    /// Clears My devices › Links for one device (what was sent stays on that PC).
    pub fn clear_device_sent(&self, device: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.dev.sent.retain(|c| c.device != device);
        let mut l = std::mem::take(&mut inner.dev.sent);
        self.store.save_sent(&mut l);
        inner.dev.sent = l;
        drop(inner);
        self.changed();
    }

    /// Like `device_order`, with the links the other PC didn't take.
    async fn device_order_answer(self: &Arc<Self>, id: &str, order: Order) -> Result<(String, Vec<String>, Vec<LinkItems>), String> {
        let (name, ok) = {
            let inner = self.inner.lock().unwrap();
            let c = inner.contacts.iter().find(|c| c.id == id && c.device).ok_or("Device not found.")?;
            (c.name.clone(), paired(c) && c.their_control)
        };
        if !ok {
            return Err(format!("{name} doesn't let this PC control it. On {name}, turn on \"This device can control my Pako\" for this PC."));
        }
        self.order_answer(id, order).await
    }

    /// Sends an order as it is (tests also send ones the other side must refuse).
    #[cfg(test)]
    pub(super) async fn order_raw(self: &Arc<Self>, id: &str, order: Order) -> Result<String, String> {
        self.order_answer(id, order).await.map(|(text, _, _)| text)
    }

    /// Sends an order as it is and waits for its answer.
    async fn order_answer(self: &Arc<Self>, id: &str, order: Order) -> Result<(String, Vec<String>, Vec<LinkItems>), String> {
        let what = describe(&order);
        let oid = super::new_id();
        let (atx, arx) = oneshot::channel();
        {
            let mut inner = self.inner.lock().unwrap();
            let tx = inner.live.get(id).map(|l| l.tx.clone()).ok_or("That device isn't connected right now.")?;
            inner.dev.orders.insert(oid.clone(), (id.to_string(), atx));
            if tx.send(Msg::Order { id: oid.clone(), order }).is_err() {
                inner.dev.orders.remove(&oid);
                return Err("The connection was lost.".into());
            }
        }
        let answer = tokio::time::timeout(ORDER_TIMEOUT, arx).await;
        self.inner.lock().unwrap().dev.orders.remove(&oid);
        let (ok, message, failed, items) = match answer {
            Ok(Ok(a)) => a,
            _ => (false, Some("No answer.".into()), Vec::new(), Vec::new()),
        };
        let message = message.map(|m| short_line(&m, 300));
        let text = match &message {
            Some(m) if ok => m.clone(),
            Some(m) => format!("{what}: {m}"),
            None => what,
        };
        self.log(id, true, text.clone(), ok);
        self.changed();
        if ok { Ok((text, failed, items)) } else { Err(message.unwrap_or_else(|| "It didn't work.".into())) }
    }

    pub(super) fn on_reply(&self, peer: &str, id: &str, ok: bool, message: Option<String>, failed: Vec<String>, items: Vec<LinkItems>) {
        let mut inner = self.inner.lock().unwrap();
        if inner.dev.orders.get(id).is_some_and(|(d, _)| d == peer) {
            if let Some((_, a)) = inner.dev.orders.remove(id) {
                let _ = a.send((ok, message, failed, items));
            }
        }
    }

    /// An order from another Pako: checked (who, how often, what), done, logged, answered.
    pub(super) async fn on_order(self: Arc<Self>, peer: String, id: String, order: Order) {
        if !proto::valid_id(&id) {
            return;
        }
        let reply_with = |ok: bool, message: Option<String>, failed: Vec<String>, items: Vec<LinkItems>| {
            if let Some(l) = self.inner.lock().unwrap().live.get(&peer) {
                let _ = l.tx.send(Msg::Reply { id: id.clone(), ok, message, failed, items });
            }
        };
        let reply = |ok: bool, message: Option<String>| reply_with(ok, message, Vec::new(), Vec::new());
        let flood = {
            let mut inner = self.inner.lock().unwrap();
            let times = inner.dev.order_times.entry(peer.clone()).or_default();
            while times.front().is_some_and(|t| t.elapsed() >= Duration::from_secs(60)) {
                times.pop_front();
            }
            if times.len() >= MAX_ORDERS_PER_MINUTE * 2 {
                return;
            }
            times.push_back(Instant::now());
            times.len() > MAX_ORDERS_PER_MINUTE
        };
        let what = describe(&order);
        if flood {
            reply(false, Some("Too many orders in a short time. Try again in a minute.".into()));
            return;
        }
        // Who: one of your devices, paired both ways, with the permission (looked up now).
        let allowed = may_control(self.inner.lock().unwrap().contacts.iter().find(|c| c.id == peer));
        if let Err(e) = allowed {
            // A stranger or a plain contact leaves no trace here; a device of yours does.
            if self.inner.lock().unwrap().contacts.iter().any(|c| c.id == peer && c.device) {
                self.log(&peer, false, format!("Refused: {what} ({e})"), false);
                self.changed();
            }
            reply(false, Some(e.into()));
            return;
        }
        let Some(host) = self.dev.host.get().cloned() else {
            reply(false, Some("That Pako can't take orders right now.".into()));
            return;
        };
        let name_of = |item: &str| host.list().into_iter().find(|i| i.id == item).map(|i| i.name);
        // For links: the ones not taken (the PC that sent them keeps those).
        let mut failed: Vec<String> = Vec::new();
        let mut items: Vec<LinkItems> = Vec::new();
        let result: Result<String, String> = match order {
            Order::Add { links, opts } => match check_links(&links) {
                Err(e) => Err(e),
                Ok(links) => {
                    let added = host.add(to_add(links.clone(), &opts)).await;
                    failed = added.failed.iter().map(|(l, _)| l.clone()).collect();
                    items = clean_items(&links, added.items.iter().map(|(link, ids)| LinkItems { link: link.clone(), ids: ids.clone() }).collect());
                    added.text()
                }
            },
            Order::Pause { item } | Order::Resume { item } | Order::Remove { item } | Order::Get { item } | Order::Priority { item, .. }
                if !valid_item(&item) || name_of(&item).is_none() =>
            {
                Err("That download isn't in the list any more.".into())
            }
            Order::Priority { item, priority } => match parse_priority(&priority) {
                None => Err("Unknown priority.".into()),
                Some(p) => {
                    host.set_priority(&item, p);
                    Ok(format!("Priority of {}: {}", name_of(&item).unwrap_or_default(), priority_label(p)))
                }
            },
            Order::Pause { item } => {
                host.pause(&item);
                Ok(format!("Paused {}", name_of(&item).unwrap_or_default()))
            }
            Order::Resume { item } => {
                host.resume(&item);
                Ok(format!("Resumed {}", name_of(&item).unwrap_or_default()))
            }
            Order::Remove { item } => {
                let name = name_of(&item).unwrap_or_default();
                host.remove(&item);
                Ok(format!("Removed {name} from the list"))
            }
            Order::Get { item } => {
                let name = name_of(&item).unwrap_or_default();
                match host.file_of(&item) {
                    None => Err(format!("{name} isn't finished, or isn't there any more.")),
                    Some(path) => match self.send(&peer, vec![path], false, None).await {
                        Ok(_) => Ok(format!("Sending {name}")),
                        Err(e) => Err(e),
                    },
                }
            }
            Order::Unknown => Err("That Pako doesn't know this order. Update Pako there.".into()),
        };
        match result {
            Ok(text) => {
                self.log(&peer, false, text.clone(), true);
                reply_with(true, Some(text), failed, items);
            }
            Err(e) => {
                self.log(&peer, false, format!("{what}: {e}"), false);
                reply_with(false, Some(e), failed, Vec::new());
            }
        }
        self.changed();
    }
}

/// An order in a few words, for the history.
fn describe(o: &Order) -> String {
    match o {
        Order::Add { links, .. } => format!("Add {}", plural(links.len(), "link")),
        Order::Priority { .. } => "Change the priority".into(),
        Order::Pause { .. } => "Pause".into(),
        Order::Resume { .. } => "Resume".into(),
        Order::Remove { .. } => "Remove".into(),
        Order::Get { .. } => "Get a file".into(),
        Order::Unknown => "An unknown order".into(),
    }
}

fn short_line(s: &str, max: usize) -> String {
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ").chars().filter(|c| !proto::is_hidden(*c)).take(max).collect();
    s
}

#[cfg(test)]
mod tests {
    use super::super::tests::{addr, node, until};
    use super::*;
    use std::sync::Mutex;

    fn lo(link: &str) -> LinkOpts {
        LinkOpts { link: link.into(), ..Default::default() }
    }

    /// Each link's quality, priority and subfolder in an Add order: written only when said,
    /// read back the same; an order from an older Pako (plain links) still reads; an older
    /// Pako still reads the new one (its links), and the new Priority order as unknown.
    #[test]
    fn link_options_travel_and_older_pakos_still_read_the_order() {
        let o = Msg::Order {
            id: "o".into(),
            order: Order::Add {
                links: vec!["https://www.youtube.com/watch?v=a".into(), "https://a.com/x.zip".into()],
                opts: vec![LinkOpts { link: "https://www.youtube.com/watch?v=a".into(), quality: Some(720), priority: Some("first".into()), folder: Some("Mix".into()) }],
            },
        };
        let json = serde_json::to_string(&o).unwrap();
        assert_eq!(
            json,
            r#"{"t":"order","id":"o","order":{"do":"add","links":["https://www.youtube.com/watch?v=a","https://a.com/x.zip"],"opts":[{"link":"https://www.youtube.com/watch?v=a","quality":720,"priority":"first","folder":"Mix"}]}}"#
        );
        assert_eq!(serde_json::from_str::<Msg>(&json).unwrap(), o);
        // Plain links: written as they always were.
        let plain = Order::Add { links: vec!["https://a.com/x".into()], opts: Vec::new() };
        assert_eq!(serde_json::to_string(&plain).unwrap(), r#"{"do":"add","links":["https://a.com/x"]}"#);
        assert_eq!(serde_json::from_str::<Order>(r#"{"do":"add","links":["https://a.com/x"]}"#).unwrap(), plain);
        // A Pako from before reads the links and skips the rest; a priority order is unknown.
        #[derive(serde::Deserialize, Debug, PartialEq)]
        #[serde(tag = "do", rename_all = "camelCase")]
        enum Before {
            Add { links: Vec<String> },
            #[serde(other)]
            Unknown,
        }
        let Msg::Order { order, .. } = o else { unreachable!() };
        let order = serde_json::to_string(&order).unwrap();
        assert_eq!(serde_json::from_str::<Before>(&order).unwrap(), Before::Add { links: vec!["https://www.youtube.com/watch?v=a".into(), "https://a.com/x.zip".into()] });
        let pr = serde_json::to_string(&Order::Priority { item: "d1".into(), priority: "first".into() }).unwrap();
        assert_eq!(pr, r#"{"do":"priority","item":"d1","priority":"first"}"#);
        assert_eq!(serde_json::from_str::<Before>(&pr).unwrap(), Before::Unknown);
        // The list's priority: left out for Normal and by older Pakos.
        let it = RemoteItem { id: "a".into(), name: "x".into(), size: 1, done: 0, speed: 0, status: "queued".into(), message: None, priority: None };
        assert!(!serde_json::to_string(&it).unwrap().contains("priority"));
        let first = RemoteItem { priority: Some("first".into()), ..it.clone() };
        assert_eq!(serde_json::from_str::<RemoteItem>(&serde_json::to_string(&first).unwrap()).unwrap(), first);
        assert_eq!(serde_json::from_str::<RemoteItem>(r#"{"id":"a","name":"x","size":1,"done":0,"speed":0,"status":"queued"}"#).unwrap(), it);
    }

    /// The downloads each link became, in the answer to an Add: written only when there are
    /// some; a Pako from before reads the answer as always (and sends none: no live status).
    #[test]
    fn the_answer_says_which_downloads_each_link_became() {
        let r = Msg::Reply { id: "o".into(), ok: true, message: Some("Added 1".into()), failed: Vec::new(), items: vec![LinkItems { link: "https://a.com/x".into(), ids: vec!["d1".into(), "d2".into()] }] };
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"t":"reply","id":"o","ok":true,"message":"Added 1","items":[{"link":"https://a.com/x","ids":["d1","d2"]}]}"#);
        assert_eq!(serde_json::from_str::<Msg>(&json).unwrap(), r);
        #[derive(serde::Deserialize, Debug, PartialEq)]
        #[serde(tag = "t", rename_all = "camelCase")]
        enum Before {
            Reply { id: String, ok: bool, message: Option<String> },
        }
        assert_eq!(serde_json::from_str::<Before>(&json).unwrap(), Before::Reply { id: "o".into(), ok: true, message: Some("Added 1".into()) });
        let Msg::Reply { items, .. } = serde_json::from_str::<Msg>(r#"{"t":"reply","id":"o","ok":true,"message":"Added 1"}"#).unwrap() else { panic!() };
        assert!(items.is_empty());
        // Only links that were sent, only ids as Pako makes them.
        let got = clean_items(
            &["https://a.com/x".to_string()],
            vec![
                LinkItems { link: "https://a.com/x".into(), ids: vec!["d1".into(), "..\\x".into(), "d1".into()] },
                LinkItems { link: "https://evil.com/y".into(), ids: vec!["d9".into()] },
            ],
        );
        assert_eq!(got, vec![LinkItems { link: "https://a.com/x".into(), ids: vec!["d1".into()] }]);
    }

    /// What a receiving PC takes from an order: each link with its own options, matched by
    /// the link; nonsense ignored; an older Pako's links come plain (not "newer").
    #[test]
    fn link_options_are_read_safely() {
        let links = vec!["https://a.com/v".to_string(), "https://a.com/f.zip".into()];
        let old = to_add(links.clone(), &[]);
        assert!(old.iter().all(|t| !t.newer && t.quality.is_none() && t.priority == Priority::Normal && t.folder.is_none()));
        let opts = vec![
            LinkOpts { link: " https://a.com/v ".into(), quality: Some(480), priority: Some("later".into()), folder: Some("Mix".into()) },
            LinkOpts { link: "https://a.com/f.zip".into(), quality: Some(99_999), priority: Some("asap".into()), folder: Some("  ".into()) },
            LinkOpts { link: "https://elsewhere.com/x".into(), quality: Some(720), ..Default::default() },
        ];
        let t = to_add(links, &opts);
        assert_eq!(t.len(), 2, "only the order's own links");
        assert_eq!((t[0].quality, t[0].priority, t[0].folder.as_deref(), t[0].newer), (Some(480), Priority::Later, Some("Mix"), true));
        assert_eq!((t[1].quality, t[1].priority, t[1].folder.as_deref()), (None, Priority::Normal, None));
        // Sending: only kept links, only what says something, Normal left out.
        let kept = vec!["https://a.com/v".to_string()];
        let o = opts_for(&kept, vec![
            LinkOpts { link: "https://a.com/v".into(), priority: Some("normal".into()), ..Default::default() },
            LinkOpts { link: "https://a.com/gone".into(), quality: Some(720), ..Default::default() },
        ]);
        assert!(o.is_empty(), "{o:?}");
        assert_eq!(parse_priority("first"), Some(Priority::First));
        assert_eq!(priority_word(Priority::Normal), None);
    }

    /// A subfolder from another PC: cleaned like a file name, 2 levels at most, never out of
    /// this PC's download folder; anything unsafe means the download folder itself.
    #[test]
    fn a_folder_from_another_pc_stays_inside_the_download_folder() {
        assert_eq!(clean_subfolder("Mix"), Some(vec!["Mix".to_string()]));
        assert_eq!(clean_subfolder(" Course/Week 3 "), Some(vec!["Course".to_string(), "Week 3".into()]));
        assert_eq!(clean_subfolder("CON"), Some(vec!["_CON".to_string()]), "reserved names get a _");
        assert_eq!(clean_subfolder("nul.txt/a"), Some(vec!["_nul.txt".to_string(), "a".into()]));
        assert_eq!(clean_subfolder("Notes.. "), Some(vec!["Notes".to_string()]), "no trailing dots or spaces");
        assert_eq!(clean_subfolder("a\u{202E}b"), Some(vec!["ab".to_string()]), "no hidden characters");
        for bad in [
            "", "   ", "..", "../x", "a/../b", "a/..", "./a", "/abs", "\\abs", "\\\\server\\share", "//server/share", "C:\\Windows", "C:/Windows", "C:x",
            "a\\b", "a//b", "a/b/c", "...", "a/",
        ] {
            assert_eq!(clean_subfolder(bad), None, "{bad:?}");
        }
        assert_eq!(clean_subfolder(&"x".repeat(MAX_FOLDER + 1)), None, "too long");
        // Sending: "\" taken as "/", long levels shortened to fit, the rest refused with why
        // (never silently dropped).
        assert_eq!(send_folder(None), Ok(None));
        assert_eq!(send_folder(Some("  ")), Ok(None));
        assert_eq!(send_folder(Some(" Work\\Reports ")), Ok(Some("Work/Reports".into())));
        assert_eq!(send_folder(Some("Course/Week 3/")), Ok(Some("Course/Week 3".into())));
        let long = format!("{}/{}", "a".repeat(180), "b".repeat(180));
        let got = send_folder(Some(&long)).unwrap().unwrap();
        assert!(got.chars().count() <= MAX_FOLDER && clean_subfolder(&got).is_some(), "{got}");
        for bad in ["Best of: 2020", "..", "a/../b", "C:\\Windows", "\\\\server\\share", "/abs", "a/b/c", "\\\\?\\C:\\x", "a:b"] {
            let e = send_folder(Some(bad)).unwrap_err();
            assert!(e.starts_with("This folder name can't be used there"), "{bad:?}: {e}");
        }
        let base = std::env::temp_dir().join(format!("pako-subfolder-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&base).unwrap();
        let (dir, bad) = place_for(&base, Some("Mix/Part 2"));
        assert!(!bad && dir == base.join("Mix").join("Part 2") && dir.is_dir());
        assert_eq!(place_for(&base, None), (base.clone(), false));
        assert_eq!(place_for(&base, Some("..\\..\\Windows")), (base.clone(), true));
        assert_eq!(place_for(&base, Some("C:/Windows")), (base.clone(), true));
        // A junction inside the download folder that leads out of it: refused, and nothing is
        // made past it.
        let outside = std::env::temp_dir().join(format!("pako-outside-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&outside).unwrap();
        #[cfg(windows)]
        {
            let link = base.join("Trap");
            let made = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&link).arg(&outside).output().map(|o| o.status.success()).unwrap_or(false);
            assert!(made, "a junction could be made");
            assert_eq!(place_for(&base, Some("Trap")), (base.clone(), true));
            assert_eq!(place_for(&base, Some("Trap/inner")), (base.clone(), true));
            assert!(!outside.join("inner").exists(), "nothing made outside");
            let _ = std::fs::remove_dir(&link);
        }
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&outside);
    }

    /// A playlist or a link page is never sent as it is: it's opened here first (a picker card in
    /// the device's Links tab, or its own row on the Links page).
    #[test]
    fn playlists_and_link_pages_are_opened_here_first() {
        assert!(open_here_first("https://www.youtube.com/playlist?list=PL1").is_some());
        assert!(open_here_first("https://exe.io/abc").is_some());
        assert!(open_here_first("https://www.youtube.com/watch?v=abc&list=PL1").is_none(), "one video");
        assert!(open_here_first("https://example.com/a.zip").is_none());
    }

    fn dev(device: bool, their: bool, control: bool) -> Contact {
        Contact { id: "x".into(), name: "Laptop".into(), device, their_device: their, control, ..Default::default() }
    }

    #[test]
    fn only_devices_paired_both_ways_with_the_permission_may_control() {
        assert!(may_control(None).is_err(), "a stranger");
        assert!(may_control(Some(&dev(false, false, false))).is_err(), "a contact");
        assert!(may_control(Some(&dev(false, true, true))).is_err(), "a contact who says you're its device");
        assert!(may_control(Some(&dev(true, false, true))).is_err(), "paired one way only");
        assert!(may_control(Some(&dev(true, true, false))).is_err(), "control taken away");
        assert!(may_control(Some(&dev(true, true, true))).is_ok());
        assert!(paired(&dev(true, true, false)) && !paired(&dev(true, false, true)));
    }

    #[test]
    fn orders_take_only_web_links() {
        assert_eq!(check_links(&["https://example.com/a.zip".into(), " http://x.org/b ".into()]).unwrap(), vec!["https://example.com/a.zip", "http://x.org/b"]);
        assert_eq!(check_links(&["https://a.com/x".into(), "https://a.com/x".into()]).unwrap().len(), 1, "no duplicates");
        for bad in ["file:///C:/Windows/system32/calc.exe", "C:\\Users\\me\\x.exe", "\\\\server\\share\\x", "ftp://a.com/x", "javascript:alert(1)", "https://", "https://a.com/x y", "https://a.com/x\nhttps://b.com/y", ""] {
            assert!(check_links(&[bad.into()]).is_err(), "{bad:?}");
        }
        // One bad link refuses the whole order.
        assert!(check_links(&["https://a.com/x".into(), "file:///etc/passwd".into()]).is_err());
        assert!(check_links(&[]).is_err());
        assert!(check_links(&vec!["https://a.com/x".to_string(); MAX_LINKS + 1]).is_err());
        assert!(check_links(&[format!("https://a.com/{}", "x".repeat(5000))]).is_err());
        // Never the PC itself (a local program's page, then "Get this file").
        for here in ["http://localhost:8080/x", "http://LOCALHOST./x", "http://app.localhost/x", "http://127.0.0.1/x", "http://127.1/x", "http://0x7f000001/x", "http://0.0.0.0/x", "http://[::1]/x", "http://[::ffff:127.0.0.1]/x"] {
            assert!(check_links(&[here.into()]).is_err(), "{here}");
        }
        // The home network stays allowed (a NAS, another PC).
        assert!(check_links(&["http://192.168.1.10/movie.mkv".into()]).is_ok());
        assert_eq!(links_in("look: https://a.com/x.zip, and http://b.org/y."), vec!["https://a.com/x.zip", "http://b.org/y"]);
        assert!(valid_item("4f1c-ab_9") && !valid_item("../x") && !valid_item("C:\\x") && !valid_item(""));
    }

    #[test]
    fn lists_from_a_device_are_made_safe() {
        let it = |id: &str, name: &str| RemoteItem { id: id.into(), name: name.into(), size: 10, done: 50, speed: 0, status: "downloading".into(), message: Some(" ".into()), priority: None };
        let l = clean_list(vec![it("a", "photo\u{202E}gpj.exe"), it("../b", "x")]);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].name, "photogpj.exe");
        assert!(l[0].message.is_none());
        assert_eq!(clean_list((0..1000).map(|i| it(&i.to_string(), "x")).collect()).len(), MAX_LIST);
    }

    #[test]
    fn the_new_messages_round_trip_and_older_pakos_skip_them() {
        let msgs = vec![
            Msg::Hello { name: "Zak".into(), device: true, pc: Some("Laptop".into()) },
            Msg::Device { mine: true, control: false, pc_name: None, relay: None },
            Msg::Device { mine: true, control: true, pc_name: Some("Desk PC".into()), relay: None },
            Msg::Watch { on: true },
            Msg::List { items: vec![RemoteItem { id: "a".into(), name: "x.zip".into(), size: 5, done: 2, speed: 1, status: "paused".into(), message: None, priority: None }] },
            Msg::Order { id: "o".into(), order: Order::Add { links: vec!["https://a.com/x".into()], opts: Vec::new() } },
            Msg::Order { id: "o".into(), order: Order::Get { item: "a".into() } },
            Msg::Reply { id: "o".into(), ok: true, message: Some("Added".into()), failed: Vec::new(), items: Vec::new() },
            Msg::Reply { id: "o".into(), ok: true, message: Some("Added".into()), failed: vec!["https://a.com/x".into()], items: Vec::new() },
            Msg::Reply { id: "o".into(), ok: true, message: Some("Added".into()), failed: Vec::new(), items: vec![LinkItems { link: "https://a.com/y".into(), ids: vec!["d1".into()] }] },
        ];
        // A reply without the links not taken (written before them) reads as "none".
        assert_eq!(
            serde_json::from_str::<Msg>(r#"{"t":"reply","id":"o","ok":true,"message":"Added"}"#).unwrap(),
            Msg::Reply { id: "o".into(), ok: true, message: Some("Added".into()), failed: Vec::new(), items: Vec::new() }
        );
        for m in &msgs {
            let json = serde_json::to_string(m).unwrap();
            assert_eq!(&serde_json::from_str::<Msg>(&json).unwrap(), m, "{json}");
        }
        // A plain hello looks as it always did (older Pakos read it).
        assert_eq!(serde_json::to_string(&Msg::Hello { name: "A".into(), device: false, pc: None }).unwrap(), r#"{"t":"hello","name":"A"}"#);
        // Pako 1.0's messages, as it reads them: the new ones are "unknown" (ignored), and a
        // pairing hello is a plain hello.
        #[derive(serde::Deserialize, Debug, PartialEq)]
        #[serde(tag = "t", rename_all = "camelCase")]
        enum Old {
            Hello { name: String },
            Accept { id: String },
            #[serde(other)]
            Unknown,
        }
        for m in &msgs[1..] {
            assert_eq!(serde_json::from_str::<Old>(&serde_json::to_string(m).unwrap()).unwrap(), Old::Unknown);
        }
        assert_eq!(serde_json::from_str::<Old>(&serde_json::to_string(&msgs[0]).unwrap()).unwrap(), Old::Hello { name: "Zak".into() });
        // An order kind from a newer Pako is refused, not a broken connection.
        let m: Msg = serde_json::from_str(r#"{"t":"order","id":"o","order":{"do":"formatDisk","path":"C:\\"}}"#).unwrap();
        assert_eq!(m, Msg::Order { id: "o".into(), order: Order::Unknown });
    }

    #[test]
    fn this_pcs_name_travels_with_the_device_message_and_older_pakos_skip_it() {
        // Sent only when set; a Pako from before it reads the message as it always did.
        assert_eq!(serde_json::to_string(&Msg::Device { mine: true, control: true, pc_name: None, relay: None }).unwrap(), r#"{"t":"device","mine":true,"control":true}"#);
        let named = serde_json::to_string(&Msg::Device { mine: true, control: false, pc_name: Some("Desk PC".into()), relay: None }).unwrap();
        assert_eq!(named, r#"{"t":"device","mine":true,"control":false,"pc_name":"Desk PC"}"#);
        #[derive(serde::Deserialize, Debug, PartialEq)]
        #[serde(tag = "t", rename_all = "camelCase")]
        enum Before {
            Device { mine: bool, control: bool },
        }
        assert_eq!(serde_json::from_str::<Before>(&named).unwrap(), Before::Device { mine: true, control: false });
        // And a message from such a Pako reads here, without a name.
        assert_eq!(serde_json::from_str::<Msg>(r#"{"t":"device","mine":true,"control":true}"#).unwrap(), Msg::Device { mine: true, control: true, pc_name: None, relay: None });
        // A plain hello stays as it was; a pairing one carries the PC's name.
        assert_eq!(
            serde_json::to_string(&Msg::Hello { name: "Zak".into(), device: true, pc: Some("Desk PC".into()) }).unwrap(),
            r#"{"t":"hello","name":"Zak","device":true,"pc":"Desk PC"}"#
        );
    }

    #[test]
    fn your_name_for_a_device_follows_its_pc_name_until_you_rename_it() {
        use super::super::{updated_pc_name, PLACEHOLDER};
        // Paired before PC names: it went by its owner's name, which now gives way.
        assert_eq!(updated_pc_name("Zak", Some("Zak"), None, "Desk PC"), Some(("Desk PC".into(), "Desk PC".into())));
        // Renamed on its side: followed.
        assert_eq!(updated_pc_name("Desk PC", Some("Zak"), Some("Desk PC"), "Office PC"), Some(("Office PC".into(), "Office PC".into())));
        // Renamed here (Rename in its Settings): yours stays, what it calls itself is kept.
        assert_eq!(updated_pc_name("Work laptop", Some("Zak"), Some("Desk PC"), "Office PC"), Some(("Work laptop".into(), "Office PC".into())));
        // Nothing new, or nothing usable.
        assert_eq!(updated_pc_name("Desk PC", None, Some("Desk PC"), " Desk PC "), None);
        assert_eq!(updated_pc_name("Desk PC", None, Some("Desk PC"), "\u{202E}  "), None);
        assert_eq!(updated_pc_name(PLACEHOLDER, None, None, "Laptop"), Some(("Laptop".into(), "Laptop".into())));
        // Cleaned like any name: no hidden characters, at most 60 characters.
        let (_, pc) = updated_pc_name("x", None, None, &format!("Desk\u{200B}PC{}", "y".repeat(80))).unwrap();
        assert!(pc.starts_with("DeskPC") && pc.chars().count() == 60);
    }

    /// The engine, pretended: a list and what was asked of it.
    #[derive(Default)]
    struct FakeHost {
        did: Mutex<Vec<String>>,
        /// The links of each Add, with what came with them.
        added: Mutex<Vec<ToAdd>>,
        file: Mutex<Option<PathBuf>>,
    }
    impl Host for FakeHost {
        fn list(&self) -> Vec<RemoteItem> {
            vec![
                RemoteItem { id: "d1".into(), name: "Movie.mkv".into(), size: 100, done: 40, speed: 0, status: "downloading".into(), message: None, priority: None },
                RemoteItem { id: "d2".into(), name: "report.pdf".into(), size: 3, done: 3, speed: 0, status: "completed".into(), message: None, priority: None },
            ]
        }
        /// A video site's link goes to Links; a link with "broken" in it isn't taken.
        fn add(&self, links: Vec<ToAdd>) -> Pin<Box<dyn Future<Output = Added> + Send>> {
            self.did.lock().unwrap().push(format!("add {}", links.iter().map(|l| l.link.as_str()).collect::<Vec<_>>().join(" ")));
            self.added.lock().unwrap().extend(links.iter().cloned());
            let mut a = Added::default();
            for t in links {
                let l = t.link;
                if l.contains("broken") {
                    a.failed.push((l, "the server said no".into()));
                } else if l.contains("youtube.com") && t.quality.is_none() && !t.newer {
                    // An older Pako's video: Links there, as before.
                    a.to_links += 1;
                } else {
                    a.downloads += 1;
                    a.items.push((l.clone(), vec![format!("n{}", a.downloads)]));
                }
            }
            Box::pin(async move { a })
        }
        fn pause(&self, item: &str) {
            self.did.lock().unwrap().push(format!("pause {item}"));
        }
        fn set_priority(&self, item: &str, priority: Priority) {
            self.did.lock().unwrap().push(format!("priority {item} {}", priority_label(priority)));
        }
        fn resume(&self, item: &str) {
            self.did.lock().unwrap().push(format!("resume {item}"));
        }
        fn remove(&self, item: &str) {
            self.did.lock().unwrap().push(format!("remove {item}"));
        }
        fn file_of(&self, item: &str) -> Option<PathBuf> {
            (item == "d2").then(|| self.file.lock().unwrap().clone()).flatten()
        }
    }

    /// Two Pakos on this PC: paired as devices (both sides confirm), files with no Accept, the
    /// list and orders; a plain contact can't order, and taking the permission away refuses
    /// the very next order.
    #[tokio::test(flavor = "multi_thread")]
    async fn two_pcs_pair_send_and_obey_only_with_permission() {
        let root = std::env::temp_dir().join(format!("pako-devices-it-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = node(&root.join("home"), "Home PC").await;
        let laptop = node(&root.join("laptop"), "Laptop").await;
        home.add_hint(addr(&laptop));
        laptop.add_hint(addr(&home));
        let host = Arc::new(FakeHost::default());
        home.set_host(host.clone(), Arc::new(|| {}));
        // Each PC's own name for your devices, apart from the name people see in Direct.
        assert!(!laptop.has_pc_name());
        assert_eq!(laptop.set_pc_name("  Laptop\u{200B} ").unwrap(), "Laptop");
        // Nothing left once cleaned: refused, the name stays.
        assert!(laptop.set_pc_name(" \u{200B}\u{202E} ").is_err());
        assert_eq!(laptop.devices_state().pc_name, "Laptop");
        home.set_pc_name("Home PC").unwrap();
        assert!(laptop.has_pc_name() && laptop.devices_state().pc_name == "Laptop");

        // The laptop pairs Home PC: Home PC is asked, as a device, with the same safety code.
        laptop.add_device(&home.my_id(), Some("Home PC"), None).unwrap();
        assert!(laptop.devices_state().devices[0].view.folder.ends_with("From Home PC"));
        until(|| !home.devices_state().requests.is_empty()).await;
        assert!(home.state().requests.is_empty(), "not among Direct's questions");
        assert_eq!(home.devices_state().requests[0].code, laptop.devices_state().devices[0].view.code);
        // Asked with the laptop's PC name.
        assert_eq!(home.devices_state().requests[0].name, "Laptop");
        let inbox = root.join("home/Inbox from Laptop");
        home.answer_device_request(&laptop.my_id(), true, Some(inbox.display().to_string()));
        until(|| home.devices_state().devices.first().is_some_and(|d| d.paired) && laptop.devices_state().devices.first().is_some_and(|d| d.paired)).await;
        assert!(home.state().contacts.is_empty() && laptop.state().contacts.is_empty(), "devices aren't in Direct's people");
        until(|| laptop.devices_state().devices[0].view.contact.their_control).await;

        // Files between them: no Accept, into that device's folder.
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("notes.txt"), b"hello").unwrap();
        laptop.send(&home.my_id(), vec![src.join("notes.txt")], false, None).await.unwrap();
        until(|| home.devices_state().history.iter().any(|h| h.result == "done")).await;
        // Only into the folder picked for that device.
        assert_eq!(std::fs::read(inbox.join("notes.txt")).unwrap(), b"hello");
        assert!(!root.join("home/Downloads/From Laptop").exists());

        // The laptop renames itself: Home PC follows at once; Direct's name for people stays.
        let direct_name = laptop.devices_state().my_name;
        laptop.set_pc_name("Zak's laptop").unwrap();
        until(|| home.devices_state().devices[0].view.contact.name == "Zak's laptop").await;
        assert_eq!(home.devices_state().devices[0].view.contact.their_pc_name.as_deref(), Some("Zak's laptop"));
        assert_eq!(laptop.devices_state().my_name, direct_name, "Direct's name didn't change");
        // Renamed on Home PC (its Settings › Rename): that name wins over what it calls itself.
        home.edit_contact(&laptop.my_id(), Some("Travel laptop".into()), None, None).unwrap();
        laptop.set_pc_name("Laptop 2").unwrap();
        until(|| home.devices_state().devices[0].view.contact.their_pc_name.as_deref() == Some("Laptop 2")).await;
        assert_eq!(home.devices_state().devices[0].view.contact.name, "Travel laptop");

        // The list, while the laptop looks.
        laptop.device_watch(&home.my_id(), true);
        until(|| laptop.devices_state().remote.get(&home.my_id()).is_some_and(|r| r.items.len() == 2)).await;

        // "Download on" (Links, the Download with Pako window): only devices that let this PC
        // control them and are connected.
        until(|| laptop.device_targets() == vec![(home.my_id(), "Home PC".to_string())]).await;
        // Links sent from Links: their original links, as they are (a video page, a file).
        let links = vec!["https://www.youtube.com/watch?v=abc123".to_string(), "https://example.com/files/b.zip".into(), "https://example.com/files/b.zip".into()];
        let sent = laptop.device_order(&home.my_id(), Order::Add { links, opts: Vec::new() }).await.unwrap();
        assert_eq!(sent, "Added 1 link to the downloads, 1 to Links");
        assert_eq!(host.did.lock().unwrap().pop().as_deref(), Some("add https://www.youtube.com/watch?v=abc123 https://example.com/files/b.zip"));
        // A link that PC couldn't take comes back: the PC that sent it keeps it (the others go).
        let links = vec!["https://example.com/ok.zip".to_string(), "https://example.com/broken.zip".into()];
        let (text, failed) = laptop.device_add(&home.my_id(), links.into_iter().map(|l| lo(&l)).collect()).await.unwrap();
        assert_eq!(failed, vec!["https://example.com/broken.zip".to_string()]);
        assert!(text.starts_with("Added 1 link to the downloads. Not taken: 1"), "{text}");
        // None taken: an error, with why (every link stays).
        let e = laptop.device_add(&home.my_id(), vec![lo("https://example.com/broken.zip")]).await.unwrap_err();
        assert!(e.contains("the server said no"), "{e}");
        // Quality, priority and a subfolder go with each link (only the link and these).
        host.added.lock().unwrap().clear();
        let yt = "https://www.youtube.com/watch?v=abc123";
        let sent = vec![
            LinkOpts { link: yt.into(), quality: Some(720), priority: Some("first".into()), folder: Some("Mix/Part 2".into()) },
            LinkOpts { link: "https://example.com/c.zip".into(), quality: None, priority: Some("later".into()), folder: Some("..\\..\\Windows".into()) },
            LinkOpts { link: "https://example.com/d.zip".into(), ..Default::default() },
        ];
        let (text, failed) = laptop.device_add(&home.my_id(), sent).await.unwrap();
        assert_eq!((text.as_str(), failed.len()), ("Added 3 links to the downloads", 0));
        let got = host.added.lock().unwrap().clone();
        assert_eq!(got[0], ToAdd { link: yt.into(), quality: Some(720), priority: Priority::First, newer: true, folder: Some("Mix/Part 2".into()) });
        assert_eq!((got[1].priority, got[1].folder.clone()), (Priority::Later, None), "an unsafe folder isn't even sent");
        assert_eq!((got[2].quality, got[2].priority, got[2].newer), (None, Priority::Normal, true));
        // The send is kept as a card on the laptop, with the downloads each link became there.
        let card = laptop.devices_state().sent.last().cloned().unwrap();
        assert_eq!(card.device, home.my_id());
        assert_eq!(card.links.len(), 3);
        assert_eq!((card.links[0].quality, card.links[0].priority.as_deref(), card.links[0].folder.as_deref()), (Some(720), Some("first"), Some("Mix/Part 2")));
        assert_eq!(card.links[0].ids, vec!["n1".to_string()]);
        assert_eq!(card.links[2].ids, vec!["n3".to_string()]);
        assert!(home.devices_state().sent.is_empty(), "kept only on the PC that sent them");
        host.did.lock().unwrap().clear();

        // Orders.
        let h = home.my_id();
        assert!(laptop.device_order(&h, Order::Add { links: vec!["file:///C:/x.exe".into()], opts: Vec::new() }).await.is_err(), "checked before it goes");
        let ok = laptop.order_raw(&h, Order::Add { links: vec!["https://example.com/a.zip".into()], opts: Vec::new() }).await.unwrap();
        assert!(ok.starts_with("Added"), "{ok}");
        assert!(laptop.order_raw(&h, Order::Add { links: vec!["file:///C:/Windows/x.exe".into()], opts: Vec::new() }).await.is_err(), "checked there too");
        laptop.order_raw(&h, Order::Pause { item: "d1".into() }).await.unwrap();
        assert!(laptop.order_raw(&h, Order::Remove { item: "..\\..\\Windows".into() }).await.is_err());
        assert!(laptop.order_raw(&h, Order::Get { item: "d1".into() }).await.is_err(), "not finished");
        // "Get this file": a normal Direct transfer, no Accept.
        std::fs::write(src.join("report.pdf"), b"pdf").unwrap();
        *host.file.lock().unwrap() = Some(src.join("report.pdf"));
        laptop.order_raw(&h, Order::Get { item: "d2".into() }).await.unwrap();
        until(|| std::fs::read(root.join("laptop/Downloads/From Home PC/report.pdf")).is_ok_and(|b| b == b"pdf")).await;
        assert_eq!(*host.did.lock().unwrap(), vec!["add https://example.com/a.zip", "pause d1"]);
        // The priority, changed from the other PC (My devices › Downloads).
        assert_eq!(laptop.order_raw(&h, Order::Priority { item: "d1".into(), priority: "first".into() }).await.unwrap(), "Priority of Movie.mkv: First");
        assert!(laptop.order_raw(&h, Order::Priority { item: "d1".into(), priority: "urgent".into() }).await.is_err());
        assert!(laptop.order_raw(&h, Order::Priority { item: "..\\x".into(), priority: "first".into() }).await.is_err());
        assert_eq!(host.did.lock().unwrap().last().map(String::as_str), Some("priority d1 First"));

        // Control taken away on Home PC: the very next order is refused, the list stops.
        home.set_control(&laptop.my_id(), false).unwrap();
        let e = laptop.order_raw(&h, Order::Resume { item: "d1".into() }).await.unwrap_err();
        assert!(e.contains("isn't allowed"), "{e}");
        let e = laptop.order_raw(&h, Order::Priority { item: "d1".into(), priority: "later".into() }).await.unwrap_err();
        assert!(e.contains("isn't allowed"), "the priority needs the same permission: {e}");
        until(|| !laptop.devices_state().devices[0].view.contact.their_control).await;
        assert!(laptop.device_order(&h, Order::Resume { item: "d1".into() }).await.is_err());
        assert!(laptop.device_targets().is_empty(), "no longer offered under Download on");
        assert_eq!(host.did.lock().unwrap().len(), 3, "nothing more was done");
        // Every action is in the history, on both sides.
        let log = home.devices_state().log;
        assert!(log.iter().any(|l| !l.by_me && l.ok && l.text.starts_with("Paused")));
        assert!(log.iter().any(|l| !l.by_me && !l.ok && l.text.starts_with("Refused")));
        assert!(laptop.devices_state().log.iter().any(|l| l.by_me && l.text.starts_with("Added")));

        // A plain contact can't give orders or see the list.
        let friend = node(&root.join("friend"), "Rado").await;
        friend.add_hint(addr(&home));
        friend.add_contact(&h, "Home").unwrap();
        until(|| !home.state().requests.is_empty()).await;
        home.answer_request(&friend.my_id(), true);
        until(|| friend.state().contacts.first().is_some_and(|c| c.online)).await;
        assert!(friend.device_order(&h, Order::Pause { item: "d1".into() }).await.is_err(), "not a device here");
        let e = friend.order_raw(&h, Order::Pause { item: "d1".into() }).await.unwrap_err();
        assert!(e.contains("Only your own devices"), "{e}");
        friend.device_watch(&h, true);
        tokio::time::sleep(Duration::from_millis(1500)).await;
        assert!(friend.devices_state().remote.is_empty());
        assert_eq!(host.did.lock().unwrap().len(), 3);
        let e = friend.order_raw(&h, Order::Priority { item: "d1".into(), priority: "first".into() }).await.unwrap_err();
        assert!(e.contains("Only your own devices"), "{e}");

        // The contact says Home PC is its device, under a PC name: Home PC still shows the
        // person's name on that question (a contact can't take a device's name), and only
        // once paired here does it go by its PC's name.
        friend.set_pc_name("Laptop").unwrap();
        friend.add_device(&h, None, None).unwrap();
        let fid = friend.my_id();
        let seen = |n: &Arc<Node>| n.devices_state().devices.into_iter().find(|d| d.view.contact.id == fid).map(|d| (d.asks_to_pair, d.view.contact.name, d.view.contact.their_pc_name));
        until(|| seen(&home).is_some_and(|(asks, _, pc)| asks && pc.as_deref() == Some("Laptop"))).await;
        assert_eq!(seen(&home).unwrap().1, "Rado");
        home.add_device(&fid, None, None).unwrap();
        assert_eq!(seen(&home).unwrap().1, "Laptop");

        home.stop.cancel();
        laptop.stop.cancel();
        friend.stop.cancel();
        let _ = std::fs::remove_dir_all(&root);
    }
}
