// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Direct's own files, kept only on this PC: the contacts and the history of transfers.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Most history entries kept (oldest dropped first).
const MAX_HISTORY: usize = 2000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    /// Their Pako ID (public key).
    pub id: String,
    /// The name you gave them.
    pub name: String,
    /// The first handshake went through (both sides said yes).
    #[serde(default)]
    pub trusted: bool,
    /// "Can send to me without asking".
    #[serde(default)]
    pub auto_accept: bool,
    /// Where their files go (None = Downloads\From <name>).
    #[serde(default)]
    pub save_folder: Option<String>,
    /// Last time their Pako was reachable (ms).
    #[serde(default)]
    pub last_seen: Option<u64>,
    #[serde(default)]
    pub added: u64,
    /// Your tags for them ("Friends", "Work"…). Only on this PC, never sent to them.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Shown under Favourites.
    #[serde(default)]
    pub favourite: bool,
    /// The name they chose for themselves (sent when you connect). Your own name for them
    /// (`name`) is never changed by it.
    #[serde(default)]
    pub their_name: Option<String>,
    /// They declined connecting (or removed you): Pako stops trying by itself, and only asks
    /// again when you press Connect.
    #[serde(default)]
    pub declined: bool,
    /// You said "this is my device" (with the safety code): shown in My devices, not in Direct's
    /// people.
    #[serde(default)]
    pub device: bool,
    /// Their Pako says you're one of its devices too (what it said last).
    #[serde(default)]
    pub their_device: bool,
    /// "This device can control my Pako": it may see your downloads and give orders. Only counts
    /// for a device paired both ways.
    #[serde(default)]
    pub control: bool,
    /// They let you control their Pako (what their Pako said last).
    #[serde(default)]
    pub their_control: bool,
    /// A device's name for its PC ("Desk PC"), as it said last. Your own name for it (`name`)
    /// follows it until you rename the device here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub their_pc_name: Option<String>,
    /// "Mute": no pop-up, chime or bell line for what they do (their offers still wait in
    /// their conversation). Never for My devices.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub muted: bool,
    /// "Check this contact": how Pako calls them in the background (see `pace.rs`).
    #[serde(default, skip_serializing_if = "Check::is_auto")]
    pub check: Check,
    /// Often ends here (ms) and they're Automatic again. Often never lasts for good: Often
    /// without an end (from a development build before 1.2) reads as Automatic (`settle_checks`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_until: Option<u64>,
    /// While Often runs: the choice it goes back to when it ends (None = Automatic). Left out
    /// when empty; older files have none (back to Automatic, as before).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_before: Option<Check>,
    /// One of your devices that lets you control it and can relay to its Direct contacts (it
    /// said so last): the name its contacts see and how many it has. None: it can't (an older
    /// Pako) or doesn't let this PC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub their_relay: Option<super::proto::RelayFrom>,
    /// Last thing that happened with them (ms): a transfer either way, you sending, them
    /// calling us, Check now. Automatic stops calling a contact after 30 days without any.
    /// Seeded for older files from the history, else the date they were added
    /// (`seed_activity`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<u64>,
}

/// "Check this contact".
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Check {
    /// For a while (until `check_until`): called every round, ahead of the others, never
    /// backed off, and their connection isn't closed for being quiet (still within the
    /// connections' cap).
    Often,
    /// Only when you ask: never called in the background.
    Ask,
    /// Called less and less often when they don't answer; not at all after 30 days with
    /// nothing happening (My devices count as Often; favourites and people who can send
    /// without asking never stop). Also a choice from a later Pako this one doesn't know (else
    /// the whole contacts file would be unreadable here and set aside). (Last: serde wants
    /// that for "other".)
    #[default]
    #[serde(other)]
    Auto,
}

impl Check {
    pub fn is_auto(&self) -> bool {
        *self == Check::Auto
    }
}

/// Often that ran out (or never had an end: an older file) goes back to the choice it had before
/// (`check_before`: When I ask, else Automatic). True when
/// something changed. An end more than a week away (the PC's clock was set back, or a file
/// edited by hand) is brought back to a week from now: Often never lasts longer than that.
pub fn settle_checks(contacts: &mut [Contact], now: u64) -> bool {
    let mut changed = false;
    let latest = now.saturating_add(super::pace::OFTEN_LONGEST_MS);
    for c in contacts.iter_mut() {
        if c.check == Check::Often && c.check_until.is_some_and(|t| now < t) {
            if c.check_until.is_some_and(|t| t > latest) {
                c.check_until = Some(latest);
                changed = true;
            }
            continue;
        }
        if c.check == Check::Often {
            c.check = back_from_often(c.check_before);
            changed = true;
        }
        changed |= c.check_until.take().is_some();
        changed |= c.check_before.take().is_some();
    }
    changed
}

/// The choice Often goes back to when it ends: the one before it (never Often itself), else
/// Automatic.
pub fn back_from_often(before: Option<Check>) -> Check {
    before.filter(|b| *b != Check::Often).unwrap_or(Check::Auto)
}

/// Gives contacts from an older file (no `last_activity`) one: their last transfer, else the
/// date they were added (`now` when that isn't known either).
pub fn seed_activity(contacts: &mut [Contact], history: &[HistoryItem], now: u64) -> bool {
    let mut changed = false;
    for c in contacts.iter_mut().filter(|c| c.last_activity.is_none()) {
        let last = history.iter().filter(|h| h.contact == c.id).map(|h| h.date).max();
        c.last_activity = Some(last.unwrap_or(if c.added > 0 { c.added } else { now }));
        changed = true;
    }
    changed
}

/// Someone you declined or removed: until `until` (ms), they're turned away without asking you.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Blocked {
    pub id: String,
    pub until: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    /// The transfer's id.
    pub id: String,
    /// The contact's Pako ID.
    pub contact: String,
    /// You sent it (else you received it).
    pub outgoing: bool,
    pub name: String,
    pub files: u32,
    pub size: u64,
    /// When it ended (ms).
    pub date: u64,
    /// "local" (the home network), "direct6", "direct" or "relay" (how it travelled), if known.
    #[serde(default)]
    pub path: Option<String>,
    /// "done", "declined", "cancelled" or "failed".
    pub result: String,
    #[serde(default)]
    pub message: Option<String>,
    /// Where received files were saved (a file or a folder).
    #[serde(default)]
    pub saved_to: Option<String>,
    /// How long the bytes were moving (ms; 0 = never started).
    #[serde(default)]
    pub duration_ms: u64,
    /// Average speed while moving (bytes per second).
    #[serde(default)]
    pub avg_speed: u64,
    /// Best speed over about one second (bytes per second).
    #[serde(default)]
    pub peak_speed: u64,
    /// A send passed on for another of your PCs: its name ("from Laptop"). Left out otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    /// The first leg of a relay (between your devices): who it's for ("for Rado"). Left out
    /// otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay_to: Option<String>,
    /// What was in it: its files and folders as they were sent (the first `MAX_ITEMS`; `more`
    /// counts the rest). Empty in an older history.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<TransferItem>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub more: u32,
}

impl HistoryItem {
    /// Where a received transfer was saved (`item`: one of its items, else all of it). Only
    /// ever `saved_to` or a name right inside it, never anywhere else.
    pub fn place_of(&self, item: Option<usize>) -> Option<PathBuf> {
        let root = PathBuf::from(self.saved_to.as_ref()?);
        let Some(i) = item else { return Some(root) };
        let it = self.items.get(i)?;
        match &it.at {
            // (One name, as Pako itself made it: no folder, no "..".)
            Some(at) if super::proto::safe_part(at).as_deref() == Some(at.as_str()) => Some(root.join(at)),
            Some(_) => None,
            // A single file: `saved_to` is the file itself.
            None if self.items.len() == 1 && !it.dir => Some(root),
            None => None,
        }
    }
}

/// Most items of one transfer kept in its history entry (the rest are only counted).
pub const MAX_ITEMS: usize = 200;

/// One file or folder of a transfer, as it was sent (a folder with everything in it).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransferItem {
    pub name: String,
    pub size: u64,
    /// Files in it (1 for a file).
    pub files: u32,
    /// A folder (else a file).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dir: bool,
    /// Received: where it was saved, inside the transfer's `saved_to` folder (one name, never a
    /// path). Left out for a single file (`saved_to` is the file itself) and for sends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// A top-level name of a transfer as it's kept and shown: the name it gets on this PC (hidden
/// characters such as right-to-left overrides gone, at most 180 characters, never a reserved
/// device name), so a list never shows a name disguised by the other side.
pub fn item_name(top: &str) -> String {
    super::proto::safe_part(top).unwrap_or_else(|| "file".into())
}

/// The key an item is found by: Windows ignores case, so "Photos/a" and "photos/b" land in one
/// folder.
fn item_key(top: &str) -> String {
    item_name(top).to_uppercase()
}

fn top_of(path: &str) -> (&str, bool) {
    match path.split_once('/') {
        Some((top, _)) => (top, true),
        None => (path, false),
    }
}

/// The files and folders of a transfer, from its files ("Folder/sub/a.txt" counts in the
/// item "Folder"), in the order they first come: the first `MAX_ITEMS`, and how many more.
pub fn items_of<'a>(files: impl IntoIterator<Item = &'a super::proto::OfferFile>) -> (Vec<TransferItem>, u32) {
    let mut items: Vec<TransferItem> = Vec::new();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut extra: std::collections::HashSet<String> = std::collections::HashSet::new();
    for f in files {
        let (top, dir) = top_of(&f.path);
        let key = item_key(top);
        if let Some(&i) = seen.get(&key) {
            let it = &mut items[i];
            it.size = it.size.saturating_add(f.size);
            it.files = it.files.saturating_add(1);
            it.dir |= dir;
        } else if items.len() < MAX_ITEMS {
            seen.insert(key, items.len());
            items.push(TransferItem { name: item_name(top), size: f.size, files: 1, dir, at: None });
        } else {
            extra.insert(key);
        }
    }
    (items, extra.len().min(u32::MAX as usize) as u32)
}

/// Received into the folder `root`: each item's place in it (the first part of where its first
/// file went), so it can be opened from the list.
pub fn place_items(items: &mut [TransferItem], files: &[super::proto::OfferFile], targets: &[PathBuf], root: &std::path::Path) {
    let mut place: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (f, target) in files.iter().zip(targets.iter()) {
        let key = item_key(top_of(&f.path).0);
        if place.contains_key(&key) {
            continue;
        }
        if let Some(std::path::Component::Normal(first)) = target.strip_prefix(root).ok().and_then(|r| r.components().next()) {
            place.insert(key, first.to_string_lossy().into_owned());
        }
    }
    for it in items {
        it.at = place.get(&it.name.to_uppercase()).cloned();
    }
}

/// Most items kept across the whole history: the newest transfers keep their lists, older
/// ones only their counts (as before lists were kept). The history goes to the window twice a
/// second while files move, and is written whole after each transfer.
pub const MAX_HISTORY_ITEMS: usize = 2000;

/// Lets go of the lists of older transfers past `MAX_HISTORY_ITEMS` (`list`: oldest first).
pub fn keep_recent_items(list: &mut [HistoryItem]) {
    let mut total = 0usize;
    let mut full = false;
    for h in list.iter_mut().rev() {
        if !full && total + h.items.len() <= MAX_HISTORY_ITEMS {
            total += h.items.len();
            continue;
        }
        full = true;
        h.items = Vec::new();
        h.more = 0;
    }
}

/// Most relays kept (ended ones go first).
const MAX_RELAYS: usize = 200;

/// A Direct relay through one of your devices. On the PC that asked (`mine`), how it's going;
/// on the PC that passes the files on, also where its copy is (`dir`, made by that PC, never
/// named by the other) and its files.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Relay {
    /// The first leg's transfer id (also the relay's).
    pub id: String,
    /// This PC asked for it (else this PC passes the files on).
    pub mine: bool,
    /// The other PC (a device).
    pub device: String,
    /// The contact it's for (their Pako ID on the relaying PC) and their name there.
    pub to: String,
    pub to_name: String,
    pub name: String,
    pub files: u32,
    pub size: u64,
    /// When it started, and when it ended (ms).
    pub date: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended: Option<u64>,
    /// Keep the copy on the relaying PC once delivered.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub keep: bool,
    /// leg1 (on its way to the relaying PC; "receiving" there), waiting (for the contact),
    /// offered, sending, paused (the second leg), delivered, declined, failed, cancelled,
    /// expired.
    pub state: String,
    /// The second leg: how far it is.
    pub done: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Relaying PC: the second leg's transfer id, the relay folder, the files there (each
    /// with its place in the send), and offers tried.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leg2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub list: Vec<RelayFile>,
    #[serde(skip_serializing_if = "is_zero")]
    pub tries: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// One file of a relay's copy: where it is on the relaying PC, and its place in the send.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RelayFile {
    pub at: String,
    pub file: super::proto::OfferFile,
}

impl Relay {
    pub fn is_open(&self) -> bool {
        self.ended.is_none()
    }
}

/// Most device history lines kept (oldest dropped first).
const MAX_DEVICE_LOG: usize = 1000;

/// One line of a device's history: something one of your devices did on another.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLog {
    /// The other device's Pako ID.
    pub device: String,
    /// ms
    pub date: u64,
    /// You did it from this PC on that device (else that device did it here).
    pub by_me: bool,
    /// What happened, in plain words ("Added 2 links", "Paused Movie.mkv").
    pub text: String,
    pub ok: bool,
}

/// Sends kept per device in My devices › Links (oldest dropped first).
pub const MAX_SENT: usize = 200;

/// One link of a send to one of your devices, and the downloads it became there.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentLink {
    pub link: String,
    /// Its name as known here (a playlist's video, a file), if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// The downloads it became on that PC (its list's ids). Empty: an older Pako there (no
    /// live status), or it went to Links there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// That PC didn't take it (the row stayed here).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub failed: bool,
}

/// One send of links to one of your devices (a card in My devices › Links).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentCard {
    pub id: String,
    pub device: String,
    /// ms
    pub date: u64,
    pub links: Vec<SentLink>,
    /// What that PC said ("Added 3 links to the downloads").
    pub text: String,
}

/// Keeps the newest `MAX_SENT` sends of each device.
pub fn trim_sent(list: &mut Vec<SentCard>) {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut keep = vec![false; list.len()];
    for (i, c) in list.iter().enumerate().rev() {
        let n = seen.entry(c.device.clone()).or_default();
        *n += 1;
        keep[i] = *n <= MAX_SENT;
    }
    let mut i = 0;
    list.retain(|_| {
        i += 1;
        keep[i - 1]
    });
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn contacts_path(&self) -> PathBuf {
        self.dir.join("contacts.json")
    }

    fn history_path(&self) -> PathBuf {
        self.dir.join("direct_history.json")
    }

    fn tags_path(&self) -> PathBuf {
        self.dir.join("direct_tags.json")
    }

    fn blocked_path(&self) -> PathBuf {
        self.dir.join("direct_blocked.json")
    }

    fn device_log_path(&self) -> PathBuf {
        self.dir.join("device_log.json")
    }

    /// What your devices did with each other (orders given and taken, permission changes).
    pub fn load_device_log(&self) -> Vec<DeviceLog> {
        read(&self.device_log_path())
    }

    pub fn save_device_log(&self, list: &mut Vec<DeviceLog>) {
        if list.len() > MAX_DEVICE_LOG {
            let extra = list.len() - MAX_DEVICE_LOG;
            list.drain(..extra);
        }
        write(&self.device_log_path(), list);
    }

    fn sent_path(&self) -> PathBuf {
        self.dir.join("device_sent.json")
    }

    /// Links sent to your devices (My devices › Links), oldest first.
    pub fn load_sent(&self) -> Vec<SentCard> {
        read(&self.sent_path())
    }

    pub fn save_sent(&self, list: &mut Vec<SentCard>) {
        trim_sent(list);
        write(&self.sent_path(), list);
    }

    fn relays_path(&self) -> PathBuf {
        self.dir.join("direct_relays.json")
    }

    /// Relays through your devices (open ones, and ended ones for 7 days).
    pub fn load_relays(&self) -> Vec<Relay> {
        read(&self.relays_path())
    }

    pub fn save_relays(&self, list: &mut Vec<Relay>) {
        while list.len() > MAX_RELAYS {
            match list.iter().position(|r| !r.is_open()) {
                Some(i) => {
                    list.remove(i);
                }
                None => {
                    list.remove(0);
                }
            }
        }
        write(&self.relays_path(), list);
    }

    fn parts_path(&self) -> PathBuf {
        self.dir.join("direct_parts.json")
    }

    /// People turned away without asking, still in effect at `now` (ms).
    pub fn load_blocked(&self, now: u64) -> Vec<Blocked> {
        let mut list: Vec<Blocked> = read(&self.blocked_path());
        list.retain(|b| b.until > now);
        list
    }

    pub fn save_blocked(&self, list: &[Blocked]) {
        write(&self.blocked_path(), list);
    }

    /// Unfinished ".pakorecv" files (".pakopart" up to Pako 0.1.2) of transfers that were
    /// running (to clean up after a crash).
    pub fn load_parts(&self) -> Vec<String> {
        read(&self.parts_path())
    }

    pub fn save_parts(&self, list: &[String]) {
        write(&self.parts_path(), list);
    }

    /// Your tags, in your order; tags found on contacts but missing from the file are added.
    pub fn load_tags(&self, contacts: &[Contact]) -> Vec<String> {
        let mut tags: Vec<String> = read(&self.tags_path());
        for c in contacts {
            for t in &c.tags {
                if find_tag(&tags, t).is_none() {
                    tags.push(t.clone());
                }
            }
        }
        tags
    }

    pub fn save_tags(&self, tags: &[String]) {
        write(&self.tags_path(), tags);
    }

    /// The contacts, Often that ran out (or an older file's Often for good) read as Automatic.
    pub fn load_contacts(&self) -> Vec<Contact> {
        let mut list: Vec<Contact> = read(&self.contacts_path());
        settle_checks(&mut list, super::node::now_ms());
        list
    }

    pub fn save_contacts(&self, list: &[Contact]) {
        write(&self.contacts_path(), list);
    }

    pub fn load_history(&self) -> Vec<HistoryItem> {
        let mut list: Vec<HistoryItem> = read(&self.history_path());
        keep_recent_items(&mut list);
        list
    }

    pub fn save_history(&self, list: &mut Vec<HistoryItem>) {
        if list.len() > MAX_HISTORY {
            let extra = list.len() - MAX_HISTORY;
            list.drain(..extra);
        }
        keep_recent_items(list);
        write(&self.history_path(), list);
    }
}

// ---------- tags (pure, so they can be tested) ----------

/// Longest tag name kept.
const MAX_TAG: usize = 30;
/// Most tags kept.
const MAX_TAGS: usize = 100;

/// A tag name as typed: trimmed, inner spaces collapsed, at most 30 characters. None = empty.
pub fn clean_tag(name: &str) -> Option<String> {
    let s: String = name.split_whitespace().collect::<Vec<_>>().join(" ").chars().filter(|c| !c.is_control()).take(MAX_TAG).collect();
    (!s.is_empty()).then_some(s)
}

/// The existing tag with this name, ignoring case.
pub fn find_tag(tags: &[String], name: &str) -> Option<usize> {
    tags.iter().position(|t| t.eq_ignore_ascii_case(name) || t.to_lowercase() == name.to_lowercase())
}

/// Adds a tag (or returns the existing one with the same name).
pub fn add_tag(tags: &mut Vec<String>, name: &str) -> Result<String, String> {
    let name = clean_tag(name).ok_or_else(|| crate::tr!("Give the tag a name.", "Donnez un nom à l’étiquette."))?;
    if let Some(i) = find_tag(tags, &name) {
        return Ok(tags[i].clone());
    }
    if tags.len() >= MAX_TAGS {
        return Err(crate::tr!("You can have up to {MAX_TAGS} tags.", "Vous pouvez avoir jusqu’à {MAX_TAGS} étiquettes."));
    }
    tags.push(name.clone());
    Ok(name)
}

/// Sets a contact's tags (any new name is added to the list, in the list's spelling).
pub fn set_contact_tags(tags: &mut Vec<String>, contact: &mut Contact, wanted: &[String]) -> Result<(), String> {
    let mut out: Vec<String> = Vec::new();
    for w in wanted {
        let t = add_tag(tags, w)?;
        if find_tag(&out, &t).is_none() {
            out.push(t);
        }
    }
    contact.tags = out;
    Ok(())
}

/// Renames a tag everywhere; renaming onto an existing tag merges the two.
pub fn rename_tag(tags: &mut Vec<String>, contacts: &mut [Contact], old: &str, new: &str) -> Result<(), String> {
    let i = find_tag(tags, old).ok_or_else(|| crate::tr!("That tag doesn't exist any more.", "Cette étiquette n’existe plus."))?;
    let new = clean_tag(new).ok_or_else(|| crate::tr!("Give the tag a name.", "Donnez un nom à l’étiquette."))?;
    let from = tags[i].clone();
    let into = match find_tag(tags, &new) {
        Some(j) if j != i => {
            tags.remove(i);
            tags[if j > i { j - 1 } else { j }].clone()
        }
        _ => {
            tags[i] = new.clone();
            new
        }
    };
    for c in contacts.iter_mut() {
        let had = c.tags.iter().any(|t| t.eq_ignore_ascii_case(&from));
        c.tags.retain(|t| !t.eq_ignore_ascii_case(&from));
        if had && find_tag(&c.tags, &into).is_none() {
            c.tags.push(into.clone());
        }
    }
    Ok(())
}

/// Deletes a tag from the list and from every contact (the contacts stay).
pub fn delete_tag(tags: &mut Vec<String>, contacts: &mut [Contact], name: &str) {
    if let Some(i) = find_tag(tags, name) {
        let gone = tags.remove(i);
        for c in contacts.iter_mut() {
            c.tags.retain(|t| !t.eq_ignore_ascii_case(&gone));
        }
    }
}

/// A list file, read as Pako's other lists are (`paths::read_list_checked`): entries that
/// can't be read are kept aside, a damaged file is moved aside and its ".bak" copy (a save
/// older at most) is used. Still locked after a moment, with no whole ".bak": the file is set
/// aside (never overwritten) and the list starts empty.
fn read<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Vec<T> {
    match crate::paths::read_list_checked::<T>(path) {
        Ok(r) => r.items,
        Err(_) => {
            let mut aside = path.clone();
            aside.set_extension(format!("unreadable-{}.json", crate::model::now_secs()));
            let _ = std::fs::rename(path, aside);
            Vec::new()
        }
    }
}

fn write<T: Serialize>(path: &PathBuf, list: &[T]) {
    if let Ok(bytes) = serde_json::to_vec_pretty(list) {
        let _ = crate::paths::write_atomic(path, &bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str, tags: &[&str]) -> Contact {
        Contact {
            id: name.to_lowercase(),
            name: name.into(),
            trusted: true,
            auto_accept: false,
            save_folder: None,
            last_seen: None,
            added: 0,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            favourite: false,
            their_name: None,
            declined: false,
            ..Default::default()
        }
    }

    #[test]
    fn tags_are_cleaned_and_not_doubled() {
        let mut tags = vec!["Friends".to_string()];
        assert_eq!(add_tag(&mut tags, "  friends ").unwrap(), "Friends");
        assert_eq!(add_tag(&mut tags, " Work   team ").unwrap(), "Work team");
        assert!(add_tag(&mut tags, "   ").is_err());
        assert_eq!(clean_tag(&"x".repeat(80)).unwrap().len(), MAX_TAG);
        assert_eq!(tags, vec!["Friends", "Work team"]);
        let mut c = person("Rado", &[]);
        set_contact_tags(&mut tags, &mut c, &["work TEAM".into(), "Family".into(), "family".into()]).unwrap();
        assert_eq!(c.tags, vec!["Work team", "Family"]);
        assert_eq!(tags, vec!["Friends", "Work team", "Family"]);
    }

    #[test]
    fn renaming_and_deleting_tags_reach_every_contact() {
        let mut tags = vec!["Friends".to_string(), "Work".to_string(), "Clients".to_string()];
        let mut people = vec![person("Rado", &["Friends", "Work"]), person("Mia", &["Clients"])];
        rename_tag(&mut tags, &mut people, "work", "Job").unwrap();
        assert_eq!(tags, vec!["Friends", "Job", "Clients"]);
        assert_eq!(people[0].tags, vec!["Friends", "Job"]);
        // Renaming onto an existing tag merges them.
        rename_tag(&mut tags, &mut people, "Clients", "job").unwrap();
        assert_eq!(tags, vec!["Friends", "Job"]);
        assert_eq!(people[1].tags, vec!["Job"]);
        delete_tag(&mut tags, &mut people, "JOB");
        assert_eq!(tags, vec!["Friends"]);
        assert_eq!(people[0].tags, vec!["Friends"]);
        assert!(people[1].tags.is_empty());
    }

    /// The sends to each device are kept, the newest 200 per device, and read back as saved.
    #[test]
    fn sent_links_are_kept_per_device_and_bounded() {
        let dir = std::env::temp_dir().join(format!("pako-sent-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::new(dir.clone());
        assert!(store.load_sent().is_empty(), "nothing yet");
        let card = |device: &str, n: u64| SentCard {
            id: format!("{device}-{n}"),
            device: device.into(),
            date: n,
            links: vec![SentLink { link: "https://a.com/x.zip".into(), ids: vec![format!("d{n}")], ..Default::default() }],
            text: "Added 1 link to the downloads".into(),
        };
        let mut list: Vec<SentCard> = (0..(MAX_SENT as u64 + 10)).map(|n| card("home", n)).collect();
        list.push(card("laptop", 1));
        store.save_sent(&mut list);
        let back = store.load_sent();
        assert_eq!(back.iter().filter(|c| c.device == "home").count(), MAX_SENT);
        assert_eq!(back.iter().find(|c| c.device == "home").unwrap().date, 10, "the oldest went first");
        assert_eq!(back.last().unwrap(), &card("laptop", 1));
        // Written compactly: what isn't said is left out, and reads back the same.
        let json = serde_json::to_string(&back[0].links[0]).unwrap();
        assert_eq!(json, r#"{"link":"https://a.com/x.zip","ids":["d10"]}"#);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_older_history_file_has_no_speeds() {
        let old: Vec<HistoryItem> = serde_json::from_str(
            r#"[{"id":"t1","contact":"a","outgoing":true,"name":"x","files":1,"size":10,"date":5,"result":"done"}]"#,
        )
        .unwrap();
        assert_eq!((old[0].duration_ms, old[0].avg_speed, old[0].peak_speed), (0, 0, 0));
        assert!(old[0].path.is_none());
    }

    #[test]
    fn an_older_contacts_file_has_no_tags_and_no_favourite() {
        let old: Vec<Contact> = serde_json::from_str(r#"[{"id":"a","name":"Rado","trusted":true}]"#).unwrap();
        assert!(old[0].tags.is_empty() && !old[0].favourite && old[0].their_name.is_none());
    }

    /// An older contacts file: not muted, checked automatically, and given a last activity
    /// from the history (else the date added); what's at its default isn't written back.
    #[test]
    fn an_older_contacts_file_loads_with_the_check_defaults() {
        let mut old: Vec<Contact> = serde_json::from_str(
            r#"[{"id":"a","name":"Rado","trusted":true,"added":100},{"id":"b","name":"Mia","trusted":true,"added":200},{"id":"c","name":"Old","trusted":true}]"#,
        )
        .unwrap();
        assert!(old.iter().all(|c| !c.muted && c.check == Check::Auto && c.last_activity.is_none()));
        let hist: Vec<HistoryItem> = serde_json::from_str(
            r#"[{"id":"t1","contact":"a","outgoing":true,"name":"x","files":1,"size":10,"date":5000,"result":"done"},
                {"id":"t2","contact":"a","outgoing":false,"name":"y","files":1,"size":10,"date":3000,"result":"declined"}]"#,
        )
        .unwrap();
        assert!(seed_activity(&mut old, &hist, 9999));
        let seeded: Vec<Option<u64>> = old.iter().map(|c| c.last_activity).collect();
        assert_eq!(seeded, [Some(5000), Some(200), Some(9999)], "the last transfer, else added, else now");
        assert!(!seed_activity(&mut old, &hist, 1), "only once");
        let json = serde_json::to_string(&Contact { id: "x".into(), name: "X".into(), ..Default::default() }).unwrap();
        assert!(!json.contains("muted") && !json.contains("check") && !json.contains("lastActivity"), "{json}");
        let json = serde_json::to_string(&Contact { muted: true, check: Check::Often, check_until: Some(42), ..Default::default() }).unwrap();
        assert!(json.contains(r#""muted":true"#) && json.contains(r#""check":"often""#) && json.contains(r#""checkUntil":42"#), "{json}");
        // A choice from a later Pako: read as Automatic, the rest of the file still loads.
        let later: Vec<Contact> = serde_json::from_str(r#"[{"id":"a","name":"Rado","check":"rarely"},{"id":"b","name":"Mia","check":"ask"}]"#).unwrap();
        assert_eq!(later.iter().map(|c| c.check).collect::<Vec<_>>(), [Check::Auto, Check::Ask]);
    }

    /// Often for good (development builds before 1.2) no longer exists: an older file's "often" with no end loads as
    /// Automatic, as does an Often whose time is up; one still running stays.
    #[test]
    fn an_older_often_loads_as_automatic() {
        let dir = std::env::temp_dir().join(format!("pako-often-old-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::new(dir.clone());
        let later = super::super::node::now_ms() + 3_600_000;
        std::fs::write(
            dir.join("contacts.json"),
            format!(
                r#"[{{"id":"a","name":"Rado","check":"often"}},{{"id":"b","name":"Mia","check":"often","checkUntil":1000}},{{"id":"c","name":"Jo","check":"often","checkUntil":{later}}},{{"id":"d","name":"Leo","check":"ask","checkUntil":{later}}}]"#
            ),
        )
        .unwrap();
        let back = store.load_contacts();
        let got: Vec<(Check, Option<u64>)> = back.iter().map(|c| (c.check, c.check_until)).collect();
        assert_eq!(got, [(Check::Auto, None), (Check::Auto, None), (Check::Often, Some(later)), (Check::Ask, None)]);
        // An end more than a week away (the clock was set back, or the file edited): a week
        // from now at most, so Often still ends by itself.
        let now = super::super::node::now_ms();
        let week = super::super::pace::OFTEN_LONGEST_MS;
        let mut far = vec![Contact { check: Check::Often, check_until: Some(now + 400 * week), ..Default::default() }];
        assert!(settle_checks(&mut far, now));
        assert_eq!((far[0].check, far[0].check_until), (Check::Often, Some(now + week)));
        assert!(!settle_checks(&mut far, now + 1000), "settled once, not on every round");
        assert!(settle_checks(&mut far, now + week) && far[0].check == Check::Auto);
        // Often given to a When I ask contact goes back to When I ask; an older file (no
        // checkBefore) to Automatic, as before.
        let mut ask = vec![Contact { check: Check::Often, check_until: Some(now + 1000), check_before: Some(Check::Ask), ..Default::default() }];
        let json = serde_json::to_string(&ask[0]).unwrap();
        assert!(json.contains(r#""checkBefore":"ask""#), "{json}");
        assert_eq!(serde_json::from_str::<Contact>(&json).unwrap(), ask[0]);
        assert!(!settle_checks(&mut ask, now));
        assert!(settle_checks(&mut ask, now + 1000));
        assert_eq!((ask[0].check, ask[0].check_until, ask[0].check_before), (Check::Ask, None, None));
        assert_eq!(back_from_often(Some(Check::Often)), Check::Auto, "never back to Often");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn contacts_and_history_round_trip() {
        let dir = std::env::temp_dir().join(format!("pako-direct-store-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::new(dir.clone());
        assert!(store.load_contacts().is_empty());
        let c = Contact {
            id: "abc".into(),
            name: "Rado".into(),
            trusted: true,
            auto_accept: false,
            save_folder: None,
            last_seen: Some(5),
            added: 1,
            tags: vec!["Friends".into()],
            favourite: true,
            their_name: Some("Rado R.".into()),
            declined: true,
            device: true,
            their_device: true,
            control: true,
            their_control: false,
            their_pc_name: Some("Desk PC".into()),
            muted: true,
            check: Check::Often,
            // (An hour from now: still running when read back.)
            check_until: Some(super::super::node::now_ms() + 3_600_000),
            check_before: Some(Check::Ask),
            last_activity: Some(7),
            their_relay: Some(super::super::proto::RelayFrom { name: "Sam".into(), contacts: 4 }),
        };
        store.save_contacts(&[c.clone()]);
        assert_eq!(store.load_contacts(), vec![c.clone()]);
        // An older file without the newer fields still reads.
        std::fs::write(dir.join("contacts.json"), r#"[{"id":"x","name":"Old"}]"#).unwrap();
        let old = store.load_contacts();
        assert_eq!(old[0].name, "Old");
        assert!(!old[0].trusted && !old[0].auto_accept);
        // A damaged file is set aside, not lost, and its ".bak" copy (the save before) is used.
        store.save_contacts(&[c.clone()]);
        store.save_contacts(&[c.clone()]);
        std::fs::write(dir.join("contacts.json"), b"{not json").unwrap();
        assert_eq!(store.load_contacts(), vec![c.clone()], "the .bak copy");
        assert!(std::fs::read_dir(&dir).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains("damaged")));
        // No .bak either: the list starts empty.
        let _ = std::fs::remove_file(dir.join("contacts.json.bak"));
        let _ = std::fs::remove_file(dir.join("contacts.bak"));
        std::fs::write(dir.join("contacts.json"), b"{not json").unwrap();
        let _ = std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".bak")).for_each(|e| {
            let _ = std::fs::remove_file(e.path());
        });
        assert!(store.load_contacts().is_empty());
        let mut h: Vec<HistoryItem> = (0..(MAX_HISTORY + 5))
            .map(|i| HistoryItem {
                id: i.to_string(),
                contact: "abc".into(),
                outgoing: i % 2 == 0,
                name: "x".into(),
                files: 1,
                size: 1,
                date: i as u64,
                path: None,
                result: "done".into(),
                message: None,
                saved_to: None,
                duration_ms: 0,
                avg_speed: 0,
                peak_speed: 0,
                via: None,
                relay_to: None,
                items: Vec::new(),
                more: 0,
            })
            .collect();
        store.save_history(&mut h);
        let back = store.load_history();
        assert_eq!(back.len(), MAX_HISTORY);
        assert_eq!(back[0].id, "5", "the oldest go first");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn file(path: &str, size: u64) -> super::super::proto::OfferFile {
        super::super::proto::OfferFile { path: path.into(), size }
    }

    /// A transfer's items: a folder with everything in it, a file on its own, in the order
    /// they first come.
    #[test]
    fn a_transfers_items_are_its_files_and_folders() {
        let files = [file("Photos/a.jpg", 10), file("notes.txt", 3), file("Photos/sub/b.jpg", 20), file("Photos/c.jpg", 5), file("Song.mp3", 7)];
        let (items, more) = items_of(&files);
        assert_eq!(more, 0);
        let got: Vec<(&str, u64, u32, bool)> = items.iter().map(|i| (i.name.as_str(), i.size, i.files, i.dir)).collect();
        assert_eq!(got, [("Photos", 35, 3, true), ("notes.txt", 3, 1, false), ("Song.mp3", 7, 1, false)]);
        // A folder holding a single file is still a folder.
        assert!(items_of(&[file("F/only.txt", 1)]).0[0].dir);
        assert_eq!(items_of(&[]).0, Vec::new());
    }

    /// A huge send keeps its first items only, and counts the rest (each once).
    #[test]
    fn a_transfers_items_are_capped() {
        let mut files: Vec<super::super::proto::OfferFile> = (0..MAX_ITEMS + 50).map(|i| file(&format!("f{i}.txt"), 1)).collect();
        files.push(file("f0.txt/x", 1)); // (counts in an item already kept)
        files.push(file("late/a", 1));
        files.push(file("late/b", 1));
        let (items, more) = items_of(&files);
        assert_eq!(items.len(), MAX_ITEMS);
        assert_eq!(more, 51, "50 files and one folder past the cap");
        assert_eq!((items[0].files, items[0].dir), (2, true));
    }

    /// The items go to the history file and back; an entry without them (an older file, or a
    /// transfer before this) loads as before and is written without them.
    #[test]
    fn history_items_round_trip() {
        let dir = std::env::temp_dir().join(format!("pako-history-items-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::new(dir.clone());
        let (mut items, _) = items_of(&[file("Photos/a.jpg", 10), file("notes.txt", 3)]);
        items[0].at = Some("Photos".into());
        items[1].at = Some("notes.txt".into());
        let with = HistoryItem {
            id: "t1".into(),
            contact: "a".into(),
            outgoing: false,
            name: "Holiday".into(),
            files: 2,
            size: 13,
            date: 5,
            path: None,
            result: "done".into(),
            message: None,
            saved_to: Some("D:/Downloads/Holiday".into()),
            duration_ms: 0,
            avg_speed: 0,
            peak_speed: 0,
            via: None,
            relay_to: None,
            items,
            more: 4,
        };
        // Each item opens where it was saved, never outside that folder.
        assert_eq!(with.place_of(Some(0)), Some(PathBuf::from("D:/Downloads/Holiday").join("Photos")));
        assert_eq!(with.place_of(None), Some(PathBuf::from("D:/Downloads/Holiday")));
        assert_eq!(with.place_of(Some(9)), None);
        let mut bad = with.clone();
        bad.items[0].at = Some("../../Windows".into());
        assert_eq!(bad.place_of(Some(0)), None);
        bad.items[0].at = Some("a/b".into());
        assert_eq!(bad.place_of(Some(0)), None);
        bad.saved_to = None;
        assert_eq!(bad.place_of(Some(1)), None, "a send: nothing to open");
        let without = HistoryItem { id: "t2".into(), items: Vec::new(), more: 0, ..with.clone() };
        store.save_history(&mut vec![with.clone(), without.clone()]);
        assert_eq!(store.load_history(), vec![with, without.clone()]);
        let json = serde_json::to_string(&without).unwrap();
        assert!(!json.contains("items") && !json.contains("more"), "{json}");
        // An older history file (no items) still loads.
        std::fs::write(dir.join("direct_history.json"), r#"[{"id":"t1","contact":"a","outgoing":true,"name":"x","files":2,"size":10,"date":5,"result":"done"}]"#).unwrap();
        let old = store.load_history();
        assert_eq!(old.len(), 1);
        assert!(old[0].items.is_empty() && old[0].more == 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The other side's names are kept and shown as they're saved here: a right-to-left override
    /// can't make "photo<RLO>gpj.exe" show as "photoexe.jpg", a name is never longer than a saved
    /// one, and names Windows treats as one ("a?" and "a*", "Photos" and "photos") are one item.
    #[test]
    fn a_transfers_item_names_are_the_saved_ones() {
        let long = "x".repeat(5000);
        let files = [
            file("photo\u{202E}gpj.exe", 1),
            file("a\u{200B}b\u{0007}.txt", 1),
            file(&format!("{long}/a"), 1),
            file("CON", 1),
            file("Pics?/a.jpg", 2),
            file("Pics*/b.jpg", 3),
            file("pics_/c.jpg", 4),
        ];
        let (items, more) = items_of(&files);
        assert_eq!(more, 0);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names[..2], ["photogpj.exe", "ab_.txt"]);
        assert!(items[2].name.chars().count() <= 180 && items[2].dir);
        assert_eq!(names[3], "_CON");
        assert_eq!((names[4], items[4].files, items[4].size), ("Pics_", 3, 9));
        assert_eq!(items.len(), 5);
    }

    /// Each received item's place is found by the name it's kept under (not the other side's).
    #[test]
    fn received_items_find_their_place() {
        let root = PathBuf::from("D:/Downloads/From Rado/Holiday");
        let files = [file("Be\u{200B}ach/a.jpg", 1), file("notes\u{202E}.txt", 1), file("Beach/b.jpg", 1)];
        let targets = [root.join("Beach").join("a.jpg"), root.join("notes (2).txt"), root.join("Beach").join("b.jpg")];
        let (mut items, _) = items_of(&files);
        place_items(&mut items, &files, &targets, &root);
        let got: Vec<(&str, Option<&str>)> = items.iter().map(|i| (i.name.as_str(), i.at.as_deref())).collect();
        assert_eq!(got, [("Beach", Some("Beach")), ("notes.txt", Some("notes (2).txt"))]);
    }

    /// A long history keeps the lists of its newest transfers only (it goes to the window twice
    /// a second while files move): older ones keep their counts, as before lists were kept.
    #[test]
    fn history_items_have_a_total_budget() {
        let dir = std::env::temp_dir().join(format!("pako-history-budget-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::new(dir.clone());
        let big: Vec<TransferItem> = (0..MAX_ITEMS).map(|i| TransferItem { name: format!("{i}{}", "\u{4E00}".repeat(170)), size: 1, files: 1, ..Default::default() }).collect();
        let entry = |i: usize| HistoryItem {
            id: i.to_string(),
            contact: "a".into(),
            outgoing: true,
            name: "x".into(),
            files: 300,
            size: 1,
            date: i as u64,
            path: None,
            result: "declined".into(),
            message: None,
            saved_to: None,
            duration_ms: 0,
            avg_speed: 0,
            peak_speed: 0,
            via: None,
            relay_to: None,
            items: big.clone(),
            more: 100,
        };
        let mut h: Vec<HistoryItem> = (0..MAX_HISTORY).map(entry).collect();
        store.save_history(&mut h);
        let kept: usize = h.iter().map(|x| x.items.len()).sum();
        assert!(kept <= MAX_HISTORY_ITEMS && kept >= MAX_HISTORY_ITEMS - MAX_ITEMS, "{kept}");
        // The newest keep theirs; once one is let go, every older one is too (and its "more").
        let last = h.iter().rposition(|x| x.items.is_empty()).unwrap();
        assert!(h[..=last].iter().all(|x| x.items.is_empty() && x.more == 0));
        assert!(h[last + 1..].iter().all(|x| x.items.len() == MAX_ITEMS && x.more == 100));
        assert_eq!(h.len(), MAX_HISTORY, "every transfer is still there");
        // What's written (and so what the window gets) stays small.
        let bytes = std::fs::metadata(dir.join("direct_history.json")).unwrap().len();
        assert!(bytes < 8 * 1024 * 1024, "{bytes}");
        // A file written without the budget is held to it when read.
        std::fs::write(dir.join("direct_history.json"), serde_json::to_vec(&(0..40).map(entry).collect::<Vec<_>>()).unwrap()).unwrap();
        let back = store.load_history();
        assert!(back.iter().map(|x| x.items.len()).sum::<usize>() <= MAX_HISTORY_ITEMS);
        assert_eq!(back.last().unwrap().items.len(), MAX_ITEMS);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
