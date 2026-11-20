// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Direct relay through your own PCs: Direct's "From: Home PC ▾". Each PC keeps its own
//! contacts; one of your devices that lets this PC control it can show you its contacts and
//! its history with them, and pass files on to one of them as a normal send from itself.
//!
//! Two legs. This PC (the laptop) sends the files to that PC (Home PC) as a My devices
//! transfer, marked "for Rado" (`Offer.relay`), with resume. Home PC keeps them in a relay
//! folder inside its own Pako data (`direct_relay/<made here>`), never the user's folders, then
//! offers them to Rado as a normal send of its own: Rado's Accept, auto-accept, size questions
//! and rate limits apply as always, and Rado never learns about the hop. Home PC tells the
//! laptop how the second leg goes (`Relayed`); the copy is deleted once delivered (unless the
//! laptop asked to keep it: then it goes into Home PC's folder for files from that laptop),
//! when Rado declines, and after 7 days.
//!
//! Hard rules (checked on the PC that passes files on, at every message):
//! - only paired devices that this PC lets control it may look or relay;
//! - only files that arrived in that one relay's first leg are ever sent on: no message names
//!   a path or a file already on the disk; the folder is made here, with a new name each time;
//! - the copy is never offered for anything else (no Open, no Folder, not in the downloads);
//! - only to one of this PC's Direct contacts who said yes once (never a device, never someone
//!   who declined).

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::oneshot;

use super::proto::{self, Look, Msg, OfferFile, RelayFrom, RelayTo, RemoteContact, RemoteMe, RemoteTransfer};
use super::store::{Contact, HistoryItem, Relay, RelayFile};
use super::{devices, now_ms, Node};

/// Folder in Pako's data where relayed files wait for the contact (one new folder per relay).
pub const RELAY_DIR: &str = "direct_relay";
/// Relays one device may have open at once on this PC.
const MAX_OPEN: usize = 20;
/// Looks (contacts, a history) one device may ask for in a minute (the window asks every few
/// seconds while it shows them).
const MAX_LOOKS_PER_MINUTE: usize = 120;
/// How long a look may wait for its answer.
const LOOK_TIMEOUT: Duration = Duration::from_secs(15);
/// A relay not taken by its contact is offered again after this (while they're connected).
const TRY_AGAIN: Duration = Duration::from_secs(60);
/// Offers to the contact tried before the relay gives up (their Pako turned it away, no answer).
const MAX_TRIES: u32 = 5;
/// Most contacts and history items taken from one answer.
const MAX_REMOTE_CONTACTS: usize = 5000;
const MAX_REMOTE_HISTORY: usize = 300;
const MAX_REMOTE_MOVING: usize = 50;
/// Ended relays are kept this long (ms), for the PC that asked to learn how they ended.
const KEEP_ENDED_MS: u64 = super::resume::KEEP_MS;
/// The PC that asked gives up on a relay its other PC never ended (it never came back): a day
/// after that PC's own 7 days.
const GIVE_UP_MS: u64 = super::resume::KEEP_MS + 24 * 3600 * 1000;
/// Why a relay stopped when the PC passing it on no longer lets the other control it (shown
/// on the PC that asked).
const STOPPED: &str = "That PC no longer lets this one send through it.";

/// Relay parts of the node's shared state.
#[derive(Default)]
pub struct RelayInner {
    /// Relays through your devices, both ways (saved).
    pub list: Vec<Relay>,
    /// Looks sent, waiting for their answer: (device, answer).
    looks: HashMap<String, (String, oneshot::Sender<Seen>)>,
    /// When each device's looks of the last minute came in.
    look_times: HashMap<String, VecDeque<Instant>>,
    /// What each relay's PC that asked was last told (state, done), to tell only changes.
    told: HashMap<String, (String, u64)>,
    /// When each relay was last offered to its contact (offered again at most once a minute,
    /// or at once when they connect).
    tried: HashMap<String, Instant>,
    /// How a relay being stopped ends (state, why), whichever path ends it first: stopping a
    /// leg ends the relay on its own, before the caller could say why.
    ending: HashMap<String, (String, Option<String>)>,
}

/// The answer to a look, as it came.
#[derive(Debug, Default)]
pub struct Seen {
    pub ok: bool,
    pub message: Option<String>,
    pub me: Option<RemoteMe>,
    pub contacts: Vec<RemoteContact>,
    pub history: Vec<HistoryItem>,
    pub moving: Vec<RemoteTransfer>,
}

/// One of your devices Direct can send from (the "From" menu).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelayChoice {
    pub id: String,
    /// Your name for it (My devices).
    pub name: String,
    /// The name it gives itself, when different.
    pub pc_name: Option<String>,
    /// The name its Direct contacts see.
    pub direct_name: String,
    pub contacts: u32,
    pub online: bool,
}

/// That PC's Direct side: its name and ID there, and its contacts.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBook {
    pub me: RemoteMe,
    pub contacts: Vec<RemoteContact>,
}

/// That PC's history with one of its contacts, and what's moving with them now.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTalk {
    pub history: Vec<HistoryItem>,
    pub moving: Vec<RemoteTransfer>,
}

/// A relay as the window shows it (both legs).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelayView {
    pub id: String,
    pub mine: bool,
    pub device: String,
    pub device_name: String,
    pub to: String,
    pub to_name: String,
    pub name: String,
    pub files: u32,
    pub size: u64,
    pub date: u64,
    pub ended: Option<u64>,
    pub state: String,
    pub message: Option<String>,
    /// The first leg (to the relaying PC): how far, and its state while it runs.
    pub leg1_done: u64,
    pub leg1_state: Option<String>,
    /// The second leg (to the contact): how far.
    pub done: u64,
    pub keep: bool,
    /// The first leg got there (the relaying PC has the files).
    pub reached: bool,
    /// Its files and folders (from its first leg), and how many more.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<super::store::TransferItem>,
    #[serde(skip_serializing_if = "is_zero")]
    pub more: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A relay's end as a few words ("delivered", "Rado declined"…), for the history lines.
fn end_words(r: &Relay) -> String {
    match r.state.as_str() {
        "delivered" => "delivered".into(),
        "declined" => format!("{} declined", r.to_name),
        "cancelled" => "cancelled".into(),
        "expired" => "expired after 7 days".into(),
        _ => match &r.message {
            Some(m) => format!("didn't go through: {m}"),
            None => "didn't go through".into(),
        },
    }
}

fn plural(n: u32, one: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") }
}

/// A state that ends a relay.
fn is_final(state: &str) -> bool {
    matches!(state, "delivered" | "declined" | "failed" | "cancelled" | "expired")
}

/// A line from the other PC, made safe to show (one line, no hidden characters, not too long).
fn line(s: &str, max: usize) -> String {
    s.chars().filter(|c| !proto::is_hidden(*c) && !c.is_control()).take(max).collect::<String>().trim().to_string()
}

/// A message told to another PC, without a place on this one: some say which file or folder
/// they were about ("Couldn't read C:\Users\…"). Those become a few plain words.
fn no_places(m: &str) -> String {
    let has_place = m.contains(":\\") || m.contains("\\\\") || m.contains(":/") || m.contains(" /");
    if !has_place {
        return m.to_string();
    }
    if m.starts_with("Couldn't read") {
        "Couldn't read one of the files.".into()
    } else if m.contains("got shorter") {
        "A file got shorter while it was being sent.".into()
    } else if m.starts_with("Couldn't make the folder") {
        "Couldn't make the folder to save to.".into()
    } else {
        "Something went wrong on that PC.".into()
    }
}

/// Copies a file or a folder (with all it holds) to `to`, never over something already there
/// (the next free name, "name (2)", instead). Marked as from another computer, like any file
/// received. Where it went.
fn copy_new(from: &Path, to: &Path) -> std::io::Result<PathBuf> {
    use std::io::ErrorKind;
    let dir = to.parent().ok_or_else(|| std::io::Error::other("no folder"))?;
    let name = to.file_name().map(|n| n.to_string_lossy().to_string()).ok_or_else(|| std::io::Error::other("no name"))?;
    let mut t = to.to_path_buf();
    for _ in 0..100 {
        if from.is_dir() {
            match std::fs::create_dir(&t) {
                Ok(()) => {
                    for e in std::fs::read_dir(from)?.flatten() {
                        copy_new(&e.path(), &t.join(e.file_name()))?;
                    }
                    return Ok(t);
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => t = super::unique(dir, &name),
                Err(e) => return Err(e),
            }
        } else {
            match std::fs::OpenOptions::new().write(true).create_new(true).open(&t) {
                Ok(mut f) => {
                    let mut src = std::fs::File::open(from)?;
                    std::io::copy(&mut src, &mut f)?;
                    drop(f);
                    super::mark_received(&t);
                    return Ok(t);
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => t = super::unique(dir, &name),
                Err(e) => return Err(e),
            }
        }
    }
    Err(std::io::Error::from(ErrorKind::AlreadyExists))
}

/// May this PC look at, and relay through, this device (its own view of the permission)?
pub fn can_relay_through(c: &Contact) -> bool {
    devices::paired(c) && c.their_control && c.their_relay.is_some()
}

/// Can this contact be relayed to from this PC: a contact (not a device) who said yes once and
/// didn't decline.
fn relayable(c: &Contact) -> bool {
    !c.device && !c.their_device && c.trusted && !c.declined
}

/// A remote list made safe: only valid IDs and transfer ids, names on one line, not too many.
fn clean_seen(mut s: Seen) -> Seen {
    s.contacts.truncate(MAX_REMOTE_CONTACTS);
    s.contacts.retain(|c| c.id.parse::<iroh::EndpointId>().is_ok());
    for c in &mut s.contacts {
        c.name = line(&c.name, 60);
        if c.name.is_empty() {
            c.name = super::PLACEHOLDER.into();
        }
        c.their_name = c.their_name.as_deref().map(|n| line(n, 60)).filter(|n| !n.is_empty());
        c.path = c.path.take().filter(|p| matches!(p.as_str(), "local" | "direct6" | "direct" | "relay"));
    }
    if let Some(me) = &mut s.me {
        me.name = line(&me.name, 60);
        if me.id.parse::<iroh::EndpointId>().is_err() {
            me.id.clear();
        }
    }
    let n = s.history.len();
    if n > MAX_REMOTE_HISTORY {
        s.history.drain(..n - MAX_REMOTE_HISTORY);
    }
    s.history.retain(|h| proto::valid_id(&h.id));
    for h in &mut s.history {
        h.name = line(&h.name, 300);
        h.message = h.message.as_deref().map(|m| line(m, 300)).filter(|m| !m.is_empty());
        h.via = h.via.as_deref().map(|m| line(m, 60)).filter(|m| !m.is_empty());
        h.relay_to = None;
        // Never a place on that PC; what was in each transfer stays there.
        h.saved_to = None;
        h.items.clear();
        h.more = 0;
        h.path = h.path.take().filter(|p| matches!(p.as_str(), "local" | "direct6" | "direct" | "relay"));
    }
    s.moving.truncate(MAX_REMOTE_MOVING);
    s.moving.retain(|t| proto::valid_id(&t.id));
    for t in &mut s.moving {
        t.name = line(&t.name, 300);
        t.state = line(&t.state, 20);
        t.message = t.message.as_deref().map(|m| line(m, 300)).filter(|m| !m.is_empty());
        t.via = t.via.as_deref().map(|m| line(m, 60)).filter(|m| !m.is_empty());
        t.done = t.done.min(t.size);
        t.path = t.path.take().filter(|p| matches!(p.as_str(), "local" | "direct6" | "direct" | "relay"));
    }
    s
}

impl Node {
    pub(super) fn save_relays(&self, inner: &mut super::Inner) {
        let mut l = std::mem::take(&mut inner.relay.list);
        self.store.save_relays(&mut l);
        inner.relay.list = l;
    }

    /// The relay folder root in Pako's data.
    fn relay_root(&self) -> PathBuf {
        self.data_dir.join(RELAY_DIR)
    }

    /// What this PC tells one of its devices about its Direct side (only one it lets control
    /// it): the name its contacts see, and how many contacts it has.
    pub(super) fn relay_from(&self, inner: &super::Inner) -> RelayFrom {
        RelayFrom { name: inner.my_name.clone(), contacts: inner.contacts.iter().filter(|c| !c.device && !c.their_device).count() as u32 }
    }

    // ---------- the window: the "From" menu, looking ----------

    /// Your devices Direct can send from: paired both ways, letting this PC control them, with
    /// a Pako that can relay. Offline ones too (shown greyed). By name.
    pub fn relay_choices(&self) -> Vec<RelayChoice> {
        let inner = self.inner.lock().unwrap();
        let mut out: Vec<RelayChoice> = inner
            .contacts
            .iter()
            .filter(|c| can_relay_through(c))
            .map(|c| {
                let r = c.their_relay.clone().unwrap_or_default();
                RelayChoice {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    pc_name: c.their_pc_name.clone().filter(|p| p.trim().to_lowercase() != c.name.trim().to_lowercase()),
                    direct_name: line(&r.name, 60),
                    contacts: r.contacts,
                    online: inner.live.contains_key(&c.id),
                }
            })
            .collect();
        out.sort_by_key(|d| d.name.to_lowercase());
        out
    }

    /// That PC's Direct contacts, read now.
    pub async fn remote_book(self: &Arc<Self>, device: &str) -> Result<RemoteBook, String> {
        let s = self.look(device, Look::Contacts).await?;
        Ok(RemoteBook { me: s.me.unwrap_or_default(), contacts: s.contacts })
    }

    /// That PC's history with one of its contacts, read now.
    pub async fn remote_talk(self: &Arc<Self>, device: &str, contact: &str) -> Result<RemoteTalk, String> {
        let s = self.look(device, Look::Talk { contact: contact.to_string() }).await?;
        Ok(RemoteTalk { history: s.history, moving: s.moving })
    }

    async fn look(self: &Arc<Self>, device: &str, look: Look) -> Result<Seen, String> {
        {
            let inner = self.inner.lock().unwrap();
            let c = inner.contacts.iter().find(|c| c.id == device && c.device).ok_or("Device not found.")?;
            if !can_relay_through(c) {
                let name = c.name.clone();
                return Err(format!("{name} doesn't let this PC see its Direct contacts. On {name}, turn on \"This device can control my Pako\" for this PC (and update Pako there)."));
            }
        }
        self.look_raw(device, look).await
    }

    /// Asks as it is (tests also ask what the other side must refuse).
    async fn look_raw(self: &Arc<Self>, device: &str, look: Look) -> Result<Seen, String> {
        let id = super::new_id();
        let (atx, arx) = oneshot::channel();
        {
            let mut inner = self.inner.lock().unwrap();
            let name = inner.contacts.iter().find(|c| c.id == device).map(|c| c.name.clone()).unwrap_or_else(|| "That PC".into());
            let tx = inner.live.get(device).map(|l| l.tx.clone()).ok_or(format!("{name} isn't connected right now."))?;
            inner.relay.looks.insert(id.clone(), (device.to_string(), atx));
            if tx.send(Msg::Look { id: id.clone(), look }).is_err() {
                inner.relay.looks.remove(&id);
                return Err("The connection was lost.".into());
            }
        }
        let answer = tokio::time::timeout(LOOK_TIMEOUT, arx).await;
        self.inner.lock().unwrap().relay.looks.remove(&id);
        match answer {
            Ok(Ok(s)) if s.ok => Ok(clean_seen(s)),
            Ok(Ok(s)) => Err(s.message.map(|m| line(&m, 300)).unwrap_or_else(|| "That PC said no.".into())),
            _ => Err("No answer.".into()),
        }
    }

    /// A look from another Pako: only your devices that may control this one, not too often.
    /// Shows only this PC's Direct contacts (not your devices) and its history with one of them.
    pub(super) fn on_look(&self, peer: &str, id: String, look: Look) {
        if !proto::valid_id(&id) {
            return;
        }
        let mut inner = self.inner.lock().unwrap();
        let Some(tx) = inner.live.get(peer).map(|l| l.tx.clone()) else { return };
        let no = |why: &str| Msg::Seen { id: id.clone(), ok: false, message: Some(why.into()), me: None, contacts: Vec::new(), history: Vec::new(), moving: Vec::new() };
        let times = inner.relay.look_times.entry(peer.to_string()).or_default();
        while times.front().is_some_and(|t| t.elapsed() >= Duration::from_secs(60)) {
            times.pop_front();
        }
        if times.len() >= MAX_LOOKS_PER_MINUTE * 2 {
            return;
        }
        times.push_back(Instant::now());
        if times.len() > MAX_LOOKS_PER_MINUTE {
            let _ = tx.send(no("Too many asks in a short time. Try again in a minute."));
            return;
        }
        if let Err(e) = devices::may_control(inner.contacts.iter().find(|c| c.id == peer)) {
            let _ = tx.send(no(e));
            return;
        }
        let path_for = |id: &str| inner.live.get(id).and_then(|l| super::path_of(&l.conn));
        let m = match look {
            Look::Contacts => Msg::Seen {
                id: id.clone(),
                ok: true,
                message: None,
                me: Some(RemoteMe { id: self.my_id(), name: inner.my_name.clone() }),
                contacts: inner
                    .contacts
                    .iter()
                    .filter(|c| !c.device && !c.their_device)
                    .take(MAX_REMOTE_CONTACTS)
                    .map(|c| RemoteContact {
                        id: c.id.clone(),
                        name: c.name.clone(),
                        their_name: c.their_name.clone(),
                        online: inner.live.contains_key(&c.id),
                        path: path_for(&c.id),
                        last_seen: c.last_seen,
                        favourite: c.favourite,
                        trusted: c.trusted,
                        declined: c.declined,
                    })
                    .collect(),
                history: Vec::new(),
                moving: Vec::new(),
            },
            Look::Talk { contact } => {
                if !inner.contacts.iter().any(|c| c.id == contact && !c.device && !c.their_device) {
                    no("That contact isn't on this PC any more.")
                } else {
                    let mut history: Vec<HistoryItem> = inner.history.iter().filter(|h| h.contact == contact).cloned().collect();
                    let n = history.len();
                    if n > MAX_REMOTE_HISTORY {
                        history.drain(..n - MAX_REMOTE_HISTORY);
                    }
                    for h in &mut history {
                        // Never a place on this PC (also not inside a reason), nor what was
                        // in each transfer: those names stay on this PC.
                        h.saved_to = None;
                        h.items.clear();
                        h.more = 0;
                        h.message = h.message.as_deref().map(no_places);
                    }
                    let via_of = |tid: &str| self.via_of(&inner, tid);
                    let mut moving: Vec<RemoteTransfer> = inner
                        .outgoing
                        .iter()
                        .filter(|(_, t)| t.contact == contact)
                        .map(|(tid, t)| RemoteTransfer {
                            id: tid.clone(),
                            outgoing: true,
                            name: t.name.clone(),
                            files: t.files.len() as u32,
                            size: t.total,
                            done: t.sent.load(Ordering::Relaxed),
                            state: t.state.clone(),
                            path: path_for(&contact),
                            message: t.message.as_deref().map(no_places),
                            via: via_of(tid),
                        })
                        .collect();
                    moving.extend(inner.incoming.iter().filter(|(_, t)| t.contact == contact && t.state != "asking").map(|(tid, t)| RemoteTransfer {
                        id: tid.clone(),
                        outgoing: false,
                        name: t.name.clone(),
                        files: t.files.len() as u32,
                        size: t.total,
                        done: t.received.load(Ordering::Relaxed),
                        state: t.state.clone(),
                        path: path_for(&contact),
                        message: t.message.as_deref().map(no_places),
                        via: None,
                    }));
                    moving.truncate(MAX_REMOTE_MOVING);
                    Msg::Seen { id: id.clone(), ok: true, message: None, me: None, contacts: Vec::new(), history, moving }
                }
            }
            Look::Unknown => no("That Pako doesn't know this. Update Pako there."),
        };
        drop(inner);
        // Too big for one message (thousands of contacts with long names): fewer.
        let m = match m {
            Msg::Seen { id, ok, message, me, mut contacts, mut history, moving } if serde_json::to_vec(&(&contacts, &history)).map_or(true, |b| b.len() > proto::MAX_MSG - 64 * 1024) => {
                contacts.truncate(1000);
                let n = history.len();
                history.drain(..n.saturating_sub(50));
                Msg::Seen { id, ok, message, me, contacts, history, moving }
            }
            m => m,
        };
        let _ = tx.send(m);
    }

    pub(super) fn on_seen(&self, peer: &str, id: &str, seen: Seen) {
        let mut inner = self.inner.lock().unwrap();
        if inner.relay.looks.get(id).is_some_and(|(d, _)| d == peer) {
            if let Some((_, a)) = inner.relay.looks.remove(id) {
                let _ = a.send(seen);
            }
        }
    }

    /// The device a relay's second leg is for ("from Laptop"), by that leg's transfer id.
    pub(super) fn via_of(&self, inner: &super::Inner, transfer: &str) -> Option<String> {
        let r = inner.relay.list.iter().find(|r| !r.mine && r.leg2.as_deref() == Some(transfer))?;
        inner.contacts.iter().find(|c| c.id == r.device).map(|c| c.name.clone()).or(Some("another PC".into()))
    }

    /// Who a relay's first leg is for ("for Rado"), by its transfer id.
    pub(super) fn relay_to_of(inner: &super::Inner, transfer: &str) -> Option<String> {
        inner.relay.list.iter().find(|r| r.id == transfer).map(|r| r.to_name.clone())
    }

    pub(super) fn relay_views(inner: &super::Inner) -> Vec<RelayView> {
        let name_of = |id: &str| inner.contacts.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default();
        inner
            .relay
            .list
            .iter()
            .map(|r| {
                let leg1 = if r.mine { inner.outgoing.get(&r.id).map(|t| (t.sent.load(Ordering::Relaxed), t.state.clone())) } else { inner.incoming.get(&r.id).map(|t| (t.received.load(Ordering::Relaxed), t.state.clone())) };
                let leg2 = r.leg2.as_ref().and_then(|l| inner.outgoing.get(l)).map(|t| t.sent.load(Ordering::Relaxed));
                // What's in it: from its first leg (moving, else its history line).
                let (items, more) = inner
                    .outgoing
                    .get(&r.id)
                    .map(|t| t.items.clone())
                    .or_else(|| inner.incoming.get(&r.id).map(|t| t.items.clone()))
                    .or_else(|| inner.history.iter().rev().find(|h| h.id == r.id).map(|h| (h.items.clone(), h.more)))
                    .unwrap_or_default();
                RelayView {
                    id: r.id.clone(),
                    mine: r.mine,
                    device: r.device.clone(),
                    device_name: name_of(&r.device),
                    to: r.to.clone(),
                    to_name: r.to_name.clone(),
                    name: r.name.clone(),
                    files: r.files,
                    size: r.size,
                    date: r.date,
                    ended: r.ended,
                    state: r.state.clone(),
                    message: r.message.clone(),
                    leg1_done: match &leg1 {
                        Some((d, _)) => *d,
                        None if matches!(r.state.as_str(), "leg1" | "receiving") => 0,
                        None => r.size,
                    },
                    leg1_state: leg1.map(|(_, s)| s),
                    done: leg2.unwrap_or(r.done).min(r.size),
                    keep: r.keep,
                    reached: !matches!(r.state.as_str(), "leg1" | "receiving") && inner.history.iter().any(|h| h.id == r.id && h.result == "done"),
                    items: items.into_iter().map(|it| super::store::TransferItem { at: None, ..it }).collect(),
                    more,
                }
            })
            .collect()
    }

    // ---------- the PC that asks (the laptop) ----------

    /// Sends files to one of a device's Direct contacts through that device: they go to the
    /// device first (a My devices transfer, with resume), which then offers them to the contact
    /// as a normal send of its own. `to_name`: the contact's name as that PC showed it.
    pub async fn relay_send(self: &Arc<Self>, device: &str, to: &str, to_name: &str, paths: Vec<PathBuf>, compress: bool, name: Option<String>, keep: bool) -> Result<String, String> {
        let to: iroh::EndpointId = to.trim().parse().map_err(|_| "That contact can't be reached this way.".to_string())?;
        let to = to.to_string();
        {
            let inner = self.inner.lock().unwrap();
            let c = inner.contacts.iter().find(|c| c.id == device && c.device).ok_or("Device not found.")?;
            if !can_relay_through(c) {
                return Err(format!("{} doesn't let this PC send from it. Turn on \"This device can control my Pako\" there for this PC.", c.name));
            }
            if to == device || to == self.my_id() {
                return Err("That contact can't be reached this way.".into());
            }
        }
        let to_name = super::clean_name(to_name).unwrap_or_else(|| super::PLACEHOLDER.into());
        self.send_as(device, paths, compress, name, Some((RelayTo { to, keep }, to_name))).await
    }

    /// Stops a relay: one this PC asked for (its first leg is cancelled as usual, or the other
    /// PC is told to stop and delete its copy), or one this PC passes on (its copy goes, the
    /// PC that asked is told).
    pub fn relay_cancel(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let (mine, device, leg1, leg2) = {
            let inner = self.inner.lock().unwrap();
            let r = inner.relay.list.iter().find(|r| r.id == id && r.is_open()).ok_or("This relay is over.")?;
            let leg1 = if r.mine { inner.outgoing.contains_key(id) } else { inner.incoming.contains_key(id) };
            (r.mine, r.device.clone(), leg1, r.leg2.clone().filter(|l| inner.outgoing.contains_key(l)))
        };
        if leg1 {
            self.cancel(id);
        }
        if let Some(l) = leg2 {
            self.cancel(&l);
        }
        if mine && !leg1 {
            if let Some(tx) = self.inner.lock().unwrap().live.get(&device).map(|l| l.tx.clone()) {
                let _ = tx.send(Msg::Unrelay { id: id.to_string() });
            }
        }
        self.relay_end(id, "cancelled", None);
        Ok(())
    }

    /// The relaying PC says how it goes (only about a relay this PC asked of it).
    pub(super) fn on_relayed(&self, peer: &str, id: &str, state: &str, done: u64, message: Option<String>) {
        let state = line(state, 20);
        let message = message.as_deref().map(|m| line(m, 300)).filter(|m| !m.is_empty());
        {
            let mut inner = self.inner.lock().unwrap();
            let Some(r) = inner.relay.list.iter_mut().find(|r| r.id == id && r.mine && r.device == peer) else { return };
            if !r.is_open() {
                // Cancelled here while that PC was away (it never heard): it still has the
                // relay going, so it's told again now. Else the files would still go.
                if r.state == "cancelled" && !is_final(&state) {
                    if let Some(tx) = inner.live.get(peer).map(|l| l.tx.clone()) {
                        let _ = tx.send(Msg::Unrelay { id: id.to_string() });
                    }
                }
                return;
            }
            if !is_final(&state) {
                // Still arriving there: this PC's own first leg shows it.
                if state == "receiving" {
                    return;
                }
                r.done = done.min(r.size);
                let changed = r.state != state || r.message != message;
                r.state = state;
                r.message = message;
                if changed {
                    self.save_relays(&mut inner);
                }
                drop(inner);
                self.changed();
                return;
            }
        }
        self.relay_end(id, &state, message);
    }

    // ---------- the PC that passes files on (Home PC) ----------

    /// A device offers files "for one of your contacts": checks who asks (a device that may
    /// control this Pako), for whom (one of this PC's contacts), the room, and makes the relay's
    /// folder (a new one, named here). The folder to receive into, or why not.
    pub(super) fn relay_take(&self, peer: &str, id: &str, name: &str, files: &[OfferFile], total: u64, r: &RelayTo) -> Result<PathBuf, String> {
        let dir = {
            let mut inner = self.inner.lock().unwrap();
            devices::may_control(inner.contacts.iter().find(|c| c.id == peer)).map_err(|e| e.to_string())?;
            // (A relay id is the first leg's, a new one each time: never one this PC had.)
            if inner.relay.list.iter().any(|x| x.id == id) {
                return Err("That PC already had this send.".into());
            }
            let to = inner.contacts.iter().find(|c| c.id == r.to).ok_or("That contact isn't on this PC any more.")?;
            if !relayable(to) {
                return Err(if to.declined { format!("{} declined this PC.", to.name) } else { "That contact can't get files from this PC yet.".into() });
            }
            let to_name = to.name.clone();
            if inner.relay.list.iter().filter(|x| !x.mine && x.device == peer && x.is_open()).count() >= MAX_OPEN {
                return Err(format!("This PC is already passing on {MAX_OPEN} sends for you. Try again once they're delivered."));
            }
            let root = self.relay_root();
            if let Some((free, size)) = crate::system::disk_space(&self.data_dir) {
                if total.saturating_add(super::margin_for(size)) > free {
                    return Err("There isn't enough room on that PC to pass this on.".into());
                }
            }
            // A new folder each time, named here: never one that exists, never a name from them.
            let dir = root.join(uuid::Uuid::new_v4().simple().to_string());
            std::fs::create_dir_all(&dir).map_err(|_| "That PC couldn't make room for it.".to_string())?;
            inner.relay.list.push(Relay {
                id: id.to_string(),
                mine: false,
                device: peer.to_string(),
                to: r.to.clone(),
                to_name,
                name: name.to_string(),
                files: files.len() as u32,
                size: total,
                date: now_ms(),
                keep: r.keep,
                state: "receiving".into(),
                dir: Some(dir.display().to_string()),
                ..Default::default()
            });
            self.save_relays(&mut inner);
            dir
        };
        Ok(dir)
    }

    /// A relay's first leg ended (this PC sent it, or received it to pass on). `got`: on the
    /// relaying PC, the files as they were saved, each with its place in the send.
    pub(super) fn relay_leg1_ended(&self, id: &str, result: &str, message: Option<String>, got: Vec<(PathBuf, OfferFile)>) {
        let mine = {
            let inner = self.inner.lock().unwrap();
            match inner.relay.list.iter().find(|r| r.id == id && r.is_open()) {
                Some(r) => r.mine,
                None => return,
            }
        };
        if result != "done" {
            let state = if result == "cancelled" { "cancelled" } else { "failed" };
            let msg = if result == "declined" { Some("That PC didn't take it.".into()) } else { message };
            self.relay_end(id, state, msg);
            return;
        }
        {
            let mut inner = self.inner.lock().unwrap();
            let Some(r) = inner.relay.list.iter_mut().find(|r| r.id == id) else { return };
            // (The other PC may already have said how its leg goes.)
            if matches!(r.state.as_str(), "leg1" | "receiving") {
                r.state = "waiting".into();
            }
            r.date = now_ms();
            if !mine {
                r.list = got.into_iter().map(|(p, f)| RelayFile { at: p.display().to_string(), file: f }).collect();
            }
            self.save_relays(&mut inner);
        }
        if !mine {
            // Offered to the contact by the relay loop within a second (or when they connect).
            self.relay_tell(id);
        }
        self.changed();
    }

    /// Offers a relay's copy to its contact, as a normal send from this PC (when they're
    /// connected; else when they next are).
    pub(super) async fn relay_offer(self: Arc<Self>, id: &str) {
        let taken = {
            let mut guard = self.inner.lock().unwrap();
            let inner = &mut *guard;
            let Some(r) = inner.relay.list.iter_mut().find(|r| r.id == id && !r.mine && r.is_open() && r.state == "waiting" && r.leg2.is_none()) else { return };
            // Checked again now, as at the start: the device may still control this Pako, and
            // the contact may still get files from it (they may have declined meanwhile).
            if devices::may_control(inner.contacts.iter().find(|c| c.id == r.device)).is_err() {
                Err(STOPPED.to_string())
            } else if !inner.contacts.iter().find(|c| c.id == r.to).is_some_and(relayable) {
                Err(format!("{} can't get files from that PC any more.", r.to_name))
            } else {
                // Taken now, so two connections at once don't offer it twice.
                r.state = "offering".into();
                let out = (r.to.clone(), r.list.clone(), r.name.clone(), r.dir.clone());
                inner.relay.tried.insert(id.to_string(), Instant::now());
                Ok(out)
            }
        };
        let (to, files, name, dir) = match taken {
            Ok(t) => t,
            Err(why) => {
                self.relay_end(id, "failed", Some(why));
                return;
            }
        };
        let back = |me: &Arc<Self>, why: Option<String>| {
            let mut inner = me.inner.lock().unwrap();
            if let Some(r) = inner.relay.list.iter_mut().find(|r| r.id == id && r.state == "offering") {
                r.state = "waiting".into();
                if why.is_some() {
                    r.message = why;
                }
            }
        };
        // Only the files that arrived for this relay, still inside its own folder.
        let root = self.relay_root();
        let inside = dir.as_deref().map(PathBuf::from).filter(|d| d.starts_with(&root) && d != &root);
        let Some(dir) = inside else {
            self.relay_end(id, "failed", Some("The copy on that PC is gone.".into()));
            return;
        };
        let list: Vec<(PathBuf, OfferFile)> = files.into_iter().map(|f| (PathBuf::from(f.at), f.file)).collect();
        if list.is_empty() || list.iter().any(|(p, _)| !p.starts_with(&dir) || !p.is_file()) {
            self.relay_end(id, "failed", Some("The copy on that PC is gone.".into()));
            return;
        }
        let live = self.inner.lock().unwrap().live.contains_key(&to);
        if !live {
            self.call_now(&to).await;
            if !self.inner.lock().unwrap().live.contains_key(&to) {
                back(&self, None);
                self.relay_tell(id);
                self.changed();
                return;
            }
        }
        match self.offer_list(&to, list, name, None).await {
            Ok(leg2) => {
                {
                    let mut inner = self.inner.lock().unwrap();
                    match inner.relay.list.iter_mut().find(|r| r.id == id && r.state == "offering") {
                        Some(r) => {
                            r.leg2 = Some(leg2.clone());
                            r.state = "offered".into();
                            r.message = None;
                            r.tries += 1;
                        }
                        // Ended meanwhile (cancelled, expired): the offer goes too.
                        None => {
                            drop(inner);
                            self.cancel(&leg2);
                            return;
                        }
                    }
                    self.save_relays(&mut inner);
                }
                self.relay_tell(id);
                self.changed();
            }
            Err(e) => {
                back(&self, Some(e));
                self.relay_tell(id);
                self.changed();
            }
        }
    }

    /// A relay's second leg (this PC to the contact) ended.
    pub(super) fn relay_leg2_ended(&self, leg2: &str, result: &str, message: Option<String>) {
        let (id, tries) = {
            let inner = self.inner.lock().unwrap();
            let Some(r) = inner.relay.list.iter().find(|r| !r.mine && r.leg2.as_deref() == Some(leg2) && r.is_open()) else { return };
            (r.id.clone(), r.tries)
        };
        match result {
            "done" => self.relay_end(&id, "delivered", None),
            "declined" => self.relay_end(&id, "declined", None),
            "cancelled" => self.relay_end(&id, "cancelled", message),
            _ if message.as_deref() == Some(super::EXPIRED_OUT) => self.relay_end(&id, "expired", None),
            // Not taken this time (no answer, turned away, the connection went before they
            // answered): offered again when they're next connected, a few times.
            _ if tries < MAX_TRIES => {
                {
                    let mut inner = self.inner.lock().unwrap();
                    if let Some(r) = inner.relay.list.iter_mut().find(|r| r.id == id) {
                        r.leg2 = None;
                        r.done = 0;
                        r.state = "waiting".into();
                        r.message = message;
                    }
                    self.save_relays(&mut inner);
                }
                self.relay_tell(&id);
                self.changed();
            }
            _ => self.relay_end(&id, "failed", message),
        }
    }

    /// The PC that asked says "stop": its relay ends here, the copy goes.
    pub(super) fn on_unrelay(self: &Arc<Self>, peer: &str, id: &str) {
        let mine = self.inner.lock().unwrap().relay.list.iter().any(|r| r.id == id && !r.mine && r.device == peer && r.is_open());
        if mine {
            self.relay_stop(id, "cancelled", Some("Cancelled on the PC that sent it.".into()));
        }
    }

    /// Stops a relay this PC passes on: what's moving stops (Rado is told his send was
    /// cancelled, as for any send), the copy goes, and it ends as `state` with `why`.
    fn relay_stop(&self, id: &str, state: &str, why: Option<String>) {
        let (leg1, leg2) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(r) = inner.relay.list.iter().find(|r| r.id == id && !r.mine && r.is_open()) else { return };
            let legs = (inner.incoming.contains_key(id), r.leg2.clone().filter(|l| inner.outgoing.contains_key(l)));
            // (Stopping a leg ends the relay on its way: it ends as said here.)
            inner.relay.ending.insert(id.to_string(), (state.to_string(), why.clone()));
            legs
        };
        if leg1 {
            self.cancel(id);
        }
        if let Some(l) = leg2 {
            self.cancel(&l);
        }
        self.relay_end(id, state, why);
        self.inner.lock().unwrap().relay.ending.remove(id);
    }

    /// Relays this PC passes on for a device that may no longer control it (control turned
    /// off here, the device removed or no longer paired): they stop at once, the copy goes.
    /// Checked every second and right after such a change.
    pub(super) fn relay_sweep(&self) {
        let stop: Vec<String> = {
            let inner = self.inner.lock().unwrap();
            inner.relay.list.iter().filter(|r| !r.mine && r.is_open() && devices::may_control(inner.contacts.iter().find(|c| c.id == r.device)).is_err()).map(|r| r.id.clone()).collect()
        };
        for id in stop {
            self.relay_stop(&id, "failed", Some(STOPPED.into()));
        }
    }

    /// Ends a relay: saved, the copy deleted (or kept, if asked: into this PC's folder for that
    /// device), a line in that device's history, and the other PC told.
    pub(super) fn relay_end(&self, id: &str, state: &str, message: Option<String>) {
        // Being stopped: it ends as the stop says, whichever leg ended first.
        let stop = self.inner.lock().unwrap().relay.ending.get(id).cloned();
        let (state, message) = match stop {
            Some((s, m)) => (s, m),
            None => (state.to_string(), message),
        };
        let state = state.as_str();
        let r = {
            let mut inner = self.inner.lock().unwrap();
            let Some(r) = inner.relay.list.iter_mut().find(|r| r.id == id && r.is_open()) else { return };
            r.state = state.into();
            r.ended = Some(now_ms());
            r.message = message;
            if state == "delivered" {
                r.done = r.size;
            }
            let r = r.clone();
            self.save_relays(&mut inner);
            r
        };
        if !r.mine {
            if let Some(dir) = r.dir.as_deref().map(PathBuf::from) {
                let keep_in = (state == "delivered" && r.keep).then(|| self.inner.lock().unwrap().contacts.iter().find(|c| c.id == r.device).map(|c| self.folder_for(c))).flatten();
                self.relay_clear(&dir, keep_in.as_deref());
            }
        }
        let what = plural(r.files, "file");
        let text = if r.mine {
            format!("Relayed {what} to {} · {}", r.to_name, end_words(&r))
        } else {
            format!("Relayed {what} to {} for this device · {}", r.to_name, end_words(&r))
        };
        self.log(&r.device, r.mine, text, state == "delivered");
        // The PC that asked: its files reached the contact, the sent sound.
        if super::sent_sound_plays(state, !r.mine) {
            self.sent_sound();
            self.relay_delivered(&r);
        }
        if !r.mine {
            self.relay_tell(id);
        }
        self.changed();
    }

    /// "Files delivered" on the PC that asked, once its relay reached the contact: their
    /// conversation here if this PC knows them too, else the PC that passed it on.
    fn relay_delivered(&self, r: &Relay) {
        let known = self.inner.lock().unwrap().contacts.iter().any(|c| c.id == r.to && !c.device);
        let go = if known { crate::notices::Go::item("direct", &r.to) } else { crate::notices::Go::item("devices", &r.device) };
        self.tell(&r.device, "direct-sent", "Files delivered", &format!("{} has received {}.", r.to_name, r.name), go);
    }

    /// Deletes a relay's copy (only inside the relay folder), or moves what's in it to `keep_in`.
    fn relay_clear(&self, dir: &Path, keep_in: Option<&Path>) {
        let root = self.relay_root();
        if !dir.starts_with(&root) || dir == root {
            return;
        }
        let Some(to) = keep_in else {
            let _ = std::fs::remove_dir_all(dir);
            return;
        };
        // Kept: moved (or copied, when that folder is on another drive than Pako's data, which
        // a move can't cross), then the relay folder goes. Off the caller: a copy takes time.
        let (dir, to) = (dir.to_path_buf(), to.to_path_buf());
        std::thread::spawn(move || {
            if std::fs::create_dir_all(&to).is_ok() {
                if let Ok(rd) = std::fs::read_dir(&dir) {
                    for e in rd.flatten() {
                        let target = to.join(e.file_name());
                        if super::move_new(&e.path(), &target).is_err() {
                            let _ = copy_new(&e.path(), &target);
                        }
                    }
                }
            }
            let _ = std::fs::remove_dir_all(&dir);
        });
    }

    /// How a relay goes as the PC that asked is told: its state (the second leg's, while it
    /// runs) and how far.
    fn relay_status(inner: &super::Inner, r: &Relay) -> (String, u64) {
        let leg2 = r.leg2.as_ref().and_then(|l| inner.outgoing.get(l));
        let done = leg2.map(|t| t.sent.load(Ordering::Relaxed)).unwrap_or(r.done).min(r.size);
        let state = match leg2.map(|t| t.state.as_str()) {
            Some("offered") | Some("preparing") => "offered",
            Some("sending") | Some("checking") => "sending",
            Some("waiting") => "paused",
            _ if r.state == "offering" => "waiting",
            _ => r.state.as_str(),
        };
        (state.to_string(), done)
    }

    /// Tells the PC that asked how a relay goes (when connected).
    pub(super) fn relay_tell(&self, id: &str) {
        let mut inner = self.inner.lock().unwrap();
        let Some(r) = inner.relay.list.iter().find(|r| r.id == id && !r.mine) else { return };
        let (state, done) = Self::relay_status(&inner, r);
        let m = Msg::Relayed { id: id.to_string(), state: state.clone(), done, size: r.size, message: r.message.as_deref().map(no_places) };
        let device = r.device.clone();
        if let Some(tx) = inner.live.get(&device).map(|l| l.tx.clone()) {
            if tx.send(m).is_ok() {
                inner.relay.told.insert(id.to_string(), (state, done));
            }
        }
    }

    /// Connected to someone: a device hears how its relays go; a contact a relay waits for is
    /// offered it.
    pub(super) fn relay_hello(self: &Arc<Self>, peer: &str) {
        let (told, offer): (Vec<String>, Vec<String>) = {
            let inner = self.inner.lock().unwrap();
            (
                inner.relay.list.iter().filter(|r| !r.mine && r.device == peer).map(|r| r.id.clone()).collect(),
                inner.relay.list.iter().filter(|r| !r.mine && r.to == peer && r.is_open() && r.state == "waiting").map(|r| r.id.clone()).collect(),
            )
        };
        for id in told {
            self.relay_tell(&id);
        }
        for id in offer {
            let me = self.clone();
            self.rt.spawn(async move { me.relay_offer(&id).await });
        }
    }

    /// Contacts a relay waits for (in its first hour: they're called every 30 seconds).
    pub(super) fn relay_waits(inner: &super::Inner, now: u64) -> Vec<String> {
        inner.relay.list.iter().filter(|r| !r.mine && r.is_open() && r.state == "waiting" && now.saturating_sub(r.date) < super::RETRY_WAITING_FOR).map(|r| r.to.clone()).collect()
    }

    /// Every second: the PC that asked hears how the second leg goes (only changes).
    pub(super) async fn relay_loop(self: Arc<Self>) {
        loop {
            tokio::select! {
                _ = self.stop.cancelled() => return,
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            }
            // A device that may no longer control this Pako (however that came about): its
            // relays stop.
            self.relay_sweep();
            let ids: Vec<String> = {
                let inner = self.inner.lock().unwrap();
                inner
                    .relay
                    .list
                    .iter()
                    .filter(|r| !r.mine && r.is_open() && inner.live.contains_key(&r.device))
                    .filter(|r| inner.relay.told.get(&r.id) != Some(&Self::relay_status(&inner, r)))
                    .map(|r| r.id.clone())
                    .collect()
            };
            for id in ids {
                self.relay_tell(&id);
            }
            // Waiting for a contact who is connected (the first leg just arrived, or it wasn't
            // taken a minute ago): offered now.
            let offer: Vec<String> = {
                let inner = self.inner.lock().unwrap();
                inner
                    .relay
                    .list
                    .iter()
                    .filter(|r| !r.mine && r.is_open() && r.state == "waiting" && inner.live.contains_key(&r.to))
                    .filter(|r| inner.relay.tried.get(&r.id).is_none_or(|t| t.elapsed() >= TRY_AGAIN))
                    .map(|r| r.id.clone())
                    .collect()
            };
            for id in offer {
                let me = self.clone();
                self.rt.spawn(async move { me.relay_offer(&id).await });
            }
        }
    }

    /// Relays past their 7 days end ("expired"; the copy goes); ended ones are forgotten
    /// after 7 days.
    pub(super) fn relay_expire(&self, now: u64) {
        let (leg2s, ends): (Vec<String>, Vec<String>) = {
            let inner = self.inner.lock().unwrap();
            let old = |r: &Relay| r.is_open() && r.date.saturating_add(if r.mine { GIVE_UP_MS } else { super::resume::KEEP_MS }) <= now;
            (
                inner.relay.list.iter().filter(|r| old(r) && !r.mine).filter_map(|r| r.leg2.clone().filter(|l| inner.outgoing.contains_key(l))).collect(),
                inner
                    .relay
                    .list
                    .iter()
                    .filter(|r| old(r) && !inner.outgoing.contains_key(&r.id) && !inner.incoming.contains_key(&r.id))
                    .map(|r| r.id.clone())
                    .collect(),
            )
        };
        for l in leg2s {
            // Rado is told, as for any send cancelled.
            self.end_out(&l, "failed", Some(super::EXPIRED_OUT.into()), true);
        }
        for id in ends {
            self.relay_end(&id, "expired", None);
        }
        let mut inner = self.inner.lock().unwrap();
        let before = inner.relay.list.len();
        inner.relay.list.retain(|r| r.ended.is_none_or(|e| e.saturating_add(KEEP_ENDED_MS) > now));
        if inner.relay.list.len() != before {
            self.save_relays(&mut inner);
        }
    }

    /// At start: relays cut by Pako closing are put back in a state they can go on from. Copies
    /// of relays this PC no longer knows are deleted.
    pub(super) fn relay_load(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.relay.list = self.store.load_relays();
        let now = now_ms();
        let mut ends = Vec::new();
        let super::Inner { relay, outgoing, incoming, history, .. } = &mut *inner;
        for r in relay.list.iter_mut().filter(|r| r.is_open()) {
            let leg1_here = if r.mine { outgoing.contains_key(&r.id) } else { incoming.contains_key(&r.id) };
            match r.state.as_str() {
                "leg1" | "receiving" if !leg1_here => {
                    let done = history.iter().any(|h| h.id == r.id && h.result == "done");
                    if done && r.mine {
                        r.state = "waiting".into();
                    } else {
                        ends.push(r.id.clone());
                    }
                }
                "offering" => r.state = "waiting".into(),
                "offered" | "sending" | "paused" if !r.mine && r.leg2.as_ref().is_none_or(|l| !outgoing.contains_key(l)) => {
                    r.state = "waiting".into();
                    r.leg2 = None;
                    r.done = 0;
                }
                _ => {}
            }
        }
        let keep: Vec<PathBuf> = relay.list.iter().filter(|r| r.is_open()).filter_map(|r| r.dir.as_deref().map(PathBuf::from)).collect();
        drop(inner);
        for id in ends {
            self.relay_end(&id, "failed", Some("Pako closed before the files got to that PC.".into()));
        }
        if let Ok(rd) = std::fs::read_dir(self.relay_root()) {
            for e in rd.flatten() {
                if !keep.contains(&e.path()) {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
        let _ = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The new messages round trip; an older Pako reads them as unknown (and ignores them), and
    /// still reads an offer or a device message that carries the relay parts.
    #[test]
    fn the_relay_messages_round_trip_and_older_pakos_skip_them() {
        let msgs = vec![
            Msg::Offer { id: "x".into(), name: "n".into(), files: vec![], resume: true, relay: Some(RelayTo { to: "rado".into(), keep: true }) },
            Msg::Device { mine: true, control: true, pc_name: Some("Home PC".into()), relay: Some(RelayFrom { name: "Sam".into(), contacts: 3 }) },
            Msg::Look { id: "l".into(), look: Look::Contacts },
            Msg::Look { id: "l".into(), look: Look::Talk { contact: "rado".into() } },
            Msg::Seen { id: "l".into(), ok: true, message: None, me: Some(RemoteMe { id: "h".into(), name: "Sam".into() }), contacts: vec![RemoteContact { id: "r".into(), name: "Rado".into(), ..Default::default() }], history: Vec::new(), moving: Vec::new() },
            Msg::Relayed { id: "x".into(), state: "sending".into(), done: 4, size: 9, message: None },
            Msg::Unrelay { id: "x".into() },
        ];
        for m in &msgs {
            let json = serde_json::to_string(m).unwrap();
            assert_eq!(&serde_json::from_str::<Msg>(&json).unwrap(), m, "{json}");
        }
        // Without the relay parts, written as before.
        assert_eq!(
            serde_json::to_string(&Msg::Offer { id: "x".into(), name: "n".into(), files: vec![], resume: false, relay: None }).unwrap(),
            r#"{"t":"offer","id":"x","name":"n","files":[]}"#
        );
        assert_eq!(serde_json::to_string(&Msg::Device { mine: true, control: false, pc_name: None, relay: None }).unwrap(), r#"{"t":"device","mine":true,"control":false}"#);
        // A Pako from before.
        #[derive(serde::Deserialize, Debug, PartialEq)]
        #[serde(tag = "t", rename_all = "camelCase")]
        enum Before {
            Offer { id: String, name: String, files: Vec<OfferFile> },
            Device { mine: bool, control: bool },
            #[serde(other)]
            Unknown,
        }
        let read = |m: &Msg| serde_json::from_str::<Before>(&serde_json::to_string(m).unwrap()).unwrap();
        assert_eq!(read(&msgs[0]), Before::Offer { id: "x".into(), name: "n".into(), files: vec![] });
        assert_eq!(read(&msgs[1]), Before::Device { mine: true, control: true });
        for m in &msgs[2..] {
            assert_eq!(read(m), Before::Unknown);
        }
        // A look from a newer Pako: refused, not a broken connection.
        assert_eq!(serde_json::from_str::<Msg>(r#"{"t":"look","id":"l","look":{"what":"everything"}}"#).unwrap(), Msg::Look { id: "l".into(), look: Look::Unknown });
    }

    /// Old saved files (contacts, history, without the relay parts) still load, and a relay
    /// row from a later Pako with fields this one doesn't know still reads.
    #[test]
    fn old_files_still_load() {
        let c: Contact = serde_json::from_str(r#"{"id":"a","name":"Home PC","device":true}"#).unwrap();
        assert!(c.their_relay.is_none());
        let h: HistoryItem = serde_json::from_str(r#"{"id":"t","contact":"a","outgoing":true,"name":"x","files":1,"size":1,"date":1,"result":"done"}"#).unwrap();
        assert!(h.via.is_none() && h.relay_to.is_none());
        let json = serde_json::to_string(&h).unwrap();
        assert!(!json.contains("via") && !json.contains("relayTo"), "{json}");
        let r: Relay = serde_json::from_str(r#"{"id":"t","mine":true,"state":"waiting","later":42}"#).unwrap();
        assert_eq!((r.id.as_str(), r.mine, r.state.as_str(), r.is_open()), ("t", true, "waiting", true));
    }

    /// What comes from the other PC is made safe to show: no hidden characters, no place on
    /// that PC, only real IDs.
    #[test]
    fn what_another_pc_shows_is_made_safe() {
        let id = iroh::SecretKey::generate().public().to_string();
        let tid = uuid::Uuid::new_v4().to_string();
        let s = clean_seen(Seen {
            ok: true,
            contacts: vec![RemoteContact { id: id.clone(), name: "Ra\u{202E}do".into(), path: Some("C:\\x".into()), ..Default::default() }, RemoteContact { id: "nope".into(), name: "x".into(), ..Default::default() }],
            history: vec![HistoryItem {
                id: tid.clone(),
                contact: id.clone(),
                outgoing: true,
                name: "a".into(),
                files: 1,
                size: 1,
                date: 1,
                path: Some("relay".into()),
                result: "done".into(),
                message: None,
                saved_to: Some("C:\\Users\\x".into()),
                duration_ms: 0,
                avg_speed: 0,
                peak_speed: 0,
                via: Some("Lap\u{200B}top".into()),
                relay_to: Some("x".into()),
                items: vec![super::super::store::TransferItem { name: "a".into(), size: 1, files: 1, dir: false, at: Some("a".into()) }],
                more: 3,
            }],
            ..Default::default()
        });
        assert_eq!(s.contacts.len(), 1);
        assert_eq!((s.contacts[0].name.as_str(), s.contacts[0].path.as_deref()), ("Rado", None));
        assert_eq!((s.history[0].saved_to.as_deref(), s.history[0].via.as_deref(), s.history[0].path.as_deref()), (None, Some("Laptop"), Some("relay")));
        // What was in each transfer stays on that PC (never sent, never taken from it).
        assert!(s.history[0].items.is_empty() && s.history[0].more == 0);
    }

    // ---------- three PCs: the laptop, Home PC (passes files on) and Rado (Home PC's contact) ----------

    use super::super::tests::{addr, node, until};
    use std::sync::atomic::Ordering;

    struct Three {
        root: PathBuf,
        laptop: Arc<Node>,
        home: Arc<Node>,
        rado: Arc<Node>,
    }

    impl Three {
        fn stop(self) {
            self.laptop.stop.cancel();
            self.home.stop.cancel();
            self.rado.stop.cancel();
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// The laptop and Home PC paired as devices (Home PC lets the laptop control it); Rado one
    /// of Home PC's Direct contacts. The laptop doesn't know Rado.
    async fn three(tag: &str) -> Three {
        let root = std::env::temp_dir().join(format!("pako-relay-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let laptop = node(&root.join("laptop"), "Sam").await;
        let home = node(&root.join("home"), "Sam at home").await;
        let rado = node(&root.join("rado"), "Rado").await;
        for (a, b) in [(&laptop, &home), (&home, &rado), (&laptop, &rado)] {
            a.add_hint(addr(b));
            b.add_hint(addr(a));
        }
        laptop.set_pc_name("Laptop").unwrap();
        home.set_pc_name("Desk PC").unwrap();
        laptop.add_device(&home.my_id(), Some("Home PC"), None).unwrap();
        until(|| !home.devices_state().requests.is_empty()).await;
        home.answer_device_request(&laptop.my_id(), true, None);
        until(|| home.devices_state().devices.first().is_some_and(|d| d.paired) && laptop.devices_state().devices.first().is_some_and(|d| d.paired)).await;
        // Rado, a contact of Home PC only.
        home.add_contact(&rado.my_id(), "Rado").unwrap();
        until(|| !rado.state().requests.is_empty()).await;
        rado.answer_request(&home.my_id(), true);
        until(|| home.state().contacts.first().is_some_and(|c| c.online) && rado.state().contacts.first().is_some_and(|c| c.online)).await;
        // The laptop hears Home PC can relay, with one contact.
        until(|| laptop.relay_choices().first().is_some_and(|c| c.contacts == 1 && c.online)).await;
        Three { root, laptop, home, rado }
    }

    fn relay_of(n: &Node, id: &str) -> Option<RelayView> {
        n.full_state().relays.into_iter().find(|r| r.id == id)
    }

    fn state(n: &Node, id: &str) -> String {
        relay_of(n, id).map(|r| r.state).unwrap_or_default()
    }

    fn bytes_of(n: usize, seed: usize) -> Vec<u8> {
        (0..n).map(|i| ((i * 7 + seed) % 251) as u8).collect()
    }

    /// Closes their connection, as a network cable pulled out would.
    fn cut(a: &Node, b: &Node) {
        let conn = a.inner.lock().unwrap().live.get(&b.my_id()).map(|l| l.conn.clone()).expect("connected");
        conn.close(0u32.into(), b"cut");
    }

    fn relay_dirs(n: &Node) -> usize {
        std::fs::read_dir(n.relay_root()).map(|rd| rd.count()).unwrap_or(0)
    }

    /// Rado's offer from Home PC, once it's there.
    async fn offer_at(rado: &Arc<Node>) -> super::super::OfferView {
        until(|| !rado.state().offers.is_empty()).await;
        rado.state().offers.into_iter().next().unwrap()
    }

    /// A relay from start to end: the laptop sees Home PC in its "From" menu, reads Home PC's
    /// contacts, and sends to Rado through it. Rado is asked as for any send from Home PC (and
    /// never hears of the laptop); both legs show; the copy goes once delivered; each PC's
    /// history says it once, tagged; the conversation on Home PC, read live from the laptop,
    /// shows the relayed card "from Laptop".
    #[tokio::test(flavor = "multi_thread")]
    async fn a_relay_goes_from_the_laptop_through_home_pc_to_rado() {
        let t = three("e2e").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        let h = home.my_id();
        // The menu: your name for it, what it calls itself, its Direct name and contact count.
        let c = laptop.relay_choices();
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].name.as_str(), c[0].pc_name.as_deref(), c[0].direct_name.as_str(), c[0].contacts), ("Home PC", Some("Desk PC"), "Sam at home", 1));
        // Its contacts, read live: Rado (never the laptop, a device).
        let book = laptop.remote_book(&h).await.unwrap();
        assert_eq!((book.me.id.as_str(), book.me.name.as_str()), (h.as_str(), "Sam at home"));
        assert_eq!(book.contacts.len(), 1);
        assert_eq!((book.contacts[0].id.as_str(), book.contacts[0].name.as_str(), book.contacts[0].online), (rado.my_id().as_str(), "Rado", true));

        let src = t.root.join("src/Holiday");
        std::fs::create_dir_all(src.join("day 2")).unwrap();
        std::fs::write(src.join("a.jpg"), b"aaa").unwrap();
        std::fs::write(src.join("day 2/b.jpg"), b"bbbb").unwrap();
        let id = laptop.relay_send(&h, &rado.my_id(), "Rado", vec![src.clone()], false, None, false).await.unwrap();
        // Rado is asked, as for a send from Home PC: same name, no trace of the laptop.
        let offer = offer_at(rado).await;
        assert_eq!((offer.contact.as_str(), offer.name.as_str(), offer.files), (h.as_str(), "Holiday", 2));
        assert!(rado.state().contacts.iter().all(|c| c.contact.id != laptop.my_id()));
        until(|| state(laptop, &id) == "offered").await;
        let v = relay_of(laptop, &id).unwrap();
        assert_eq!((v.to_name.as_str(), v.device_name.as_str(), v.leg1_done, v.size), ("Rado", "Home PC", 7, 7), "the first leg is through");
        // Home PC's copy is in its own Pako data, in a folder of its own.
        assert_eq!(relay_dirs(home), 1);
        assert!(!t.root.join("home/Downloads").join("From Laptop").exists(), "never the user's receiving folder");
        // Its Direct page shows the send to Rado, tagged with the PC it's for.
        let moving = home.state().transfers.into_iter().find(|x| x.contact == rado.my_id()).unwrap();
        assert_eq!(moving.via.as_deref(), Some("Laptop"));

        let r2 = rado.clone();
        let oid = offer.id.clone();
        tokio::task::spawn_blocking(move || r2.answer_offer(&oid, true, None)).await.unwrap().unwrap();
        until(|| state(laptop, &id) == "delivered").await;
        // Laid out as any send of a folder from Home PC would be.
        let got = t.root.join("rado/Downloads/From Sam at home/Holiday/Holiday");
        assert_eq!(std::fs::read(got.join("a.jpg")).unwrap(), b"aaa");
        assert_eq!(std::fs::read(got.join("day 2/b.jpg")).unwrap(), b"bbbb");
        until(|| relay_dirs(home) == 0).await;
        assert_eq!(relay_of(laptop, &id).unwrap().done, 7);

        // History: Home PC's send to Rado is tagged "from Laptop" (once); the legs between the
        // two PCs say who they were for; the copy can't be opened on Home PC.
        let hh = home.full_state().history;
        let to_rado: Vec<_> = hh.iter().filter(|x| x.contact == rado.my_id()).collect();
        assert_eq!(to_rado.len(), 1);
        assert_eq!((to_rado[0].via.as_deref(), to_rado[0].result.as_str(), to_rado[0].outgoing), (Some("Laptop"), "done", true));
        let leg1_home = hh.iter().find(|x| x.id == id).unwrap();
        assert_eq!((leg1_home.relay_to.as_deref(), leg1_home.saved_to.as_deref()), (Some("Rado"), None));
        until(|| laptop.history_item(&id).is_some()).await;
        assert_eq!(laptop.history_item(&id).unwrap().relay_to.as_deref(), Some("Rado"));
        assert!(rado.full_state().history.iter().all(|x| x.via.is_none()), "Rado never learns about the hop");
        // Read live from the laptop: the conversation on Home PC with Rado.
        let talk = laptop.remote_talk(&h, &rado.my_id()).await.unwrap();
        assert_eq!(talk.history.len(), 1);
        assert_eq!((talk.history[0].via.as_deref(), talk.history[0].saved_to.as_deref()), (Some("Laptop"), None));
        // My devices › History: one line per relay, on both PCs.
        let line = |n: &Node| n.devices_state().log.into_iter().filter(|l| l.text.starts_with("Relayed")).map(|l| l.text).collect::<Vec<_>>();
        assert_eq!(line(laptop), vec!["Relayed 2 files to Rado · delivered".to_string()]);
        assert_eq!(line(home), vec!["Relayed 2 files to Rado for this device · delivered".to_string()]);
        t.stop();
    }

    /// Without "This device can control my Pako" on Home PC: not in the menu, nothing listed,
    /// and a relay sent anyway is refused there (nothing kept). A plain contact (Rado) can
    /// neither look nor relay through Home PC.
    #[tokio::test(flavor = "multi_thread")]
    async fn without_control_nothing_is_shown_or_relayed() {
        let t = three("control").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        let h = home.my_id();
        home.set_control(&laptop.my_id(), false).unwrap();
        until(|| laptop.relay_choices().is_empty()).await;
        assert!(laptop.remote_book(&h).await.is_err());
        assert!(laptop.remote_talk(&h, &rado.my_id()).await.is_err());
        // Asked anyway (the laptop's own check skipped): refused there.
        let e = laptop.look_raw(&h, Look::Contacts).await.unwrap_err();
        assert!(e.contains("isn't allowed"), "{e}");
        let e = laptop.look_raw(&h, Look::Talk { contact: rado.my_id() }).await.unwrap_err();
        assert!(e.contains("isn't allowed"), "{e}");
        let src = t.root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("x.txt"), b"x").unwrap();
        assert!(laptop.relay_send(&h, &rado.my_id(), "Rado", vec![src.join("x.txt")], false, None, false).await.is_err());
        let id = laptop.send_as(&h, vec![src.join("x.txt")], false, None, Some((RelayTo { to: rado.my_id(), keep: false }, "Rado".into()))).await.unwrap();
        until(|| state(laptop, &id) == "failed").await;
        assert!(relay_of(laptop, &id).unwrap().message.unwrap_or_default().contains("isn't allowed"));
        assert_eq!(relay_dirs(home), 0);
        assert!(rado.state().offers.is_empty());
        assert!(home.full_state().relays.is_empty());
        // Rado, one of Home PC's contacts but not a device: no look, no relay.
        let e = rado.look_raw(&h, Look::Contacts).await.unwrap_err();
        assert!(e.contains("Only your own devices"), "{e}");
        std::fs::write(src.join("y.txt"), b"y").unwrap();
        let id = rado.send_as(&h, vec![src.join("y.txt")], false, None, Some((RelayTo { to: rado.my_id(), keep: false }, "x".into()))).await.unwrap();
        until(|| state(rado, &id) == "failed").await;
        assert!(home.full_state().relays.is_empty() && relay_dirs(home) == 0);
        assert!(home.full_state().history.iter().all(|x| x.id != id || x.result != "done"));
        t.stop();
    }

    /// A relay can only pass on what arrived in it: names from the laptop never reach a file
    /// already on Home PC (the folder is new, made there), unsafe names are refused, and the
    /// relay messages that name nothing on the disk can't be used to reach anything else.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_relay_never_reaches_a_file_already_on_that_pc() {
        let t = three("existing").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        let h = home.my_id();
        rado.edit_contact(&h, None, Some(true), None).unwrap();
        std::fs::write(t.root.join("home/secret.txt"), b"home only").unwrap();
        // Files named like Home PC's own: they go into a new folder there, and Rado gets the
        // laptop's bytes.
        let src = t.root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("secret.txt"), b"from the laptop").unwrap();
        std::fs::write(src.join("contacts.json"), b"[]").unwrap();
        let id = laptop.relay_send(&h, &rado.my_id(), "Rado", vec![src.join("secret.txt"), src.join("contacts.json")], false, Some("..".into()), false).await.unwrap();
        until(|| state(laptop, &id) == "delivered").await;
        assert_eq!(std::fs::read(t.root.join("home/secret.txt")).unwrap(), b"home only");
        // Home PC's own contacts file is still its own (only its dates moved).
        assert_ne!(std::fs::read(t.root.join("home/contacts.json")).unwrap(), b"[]");
        assert_eq!(home.store.load_contacts().len(), 2);
        let got = std::fs::read_dir(t.root.join("rado/Downloads/From Sam at home")).unwrap().flatten().map(|e| e.path()).find(|p| p.is_dir()).unwrap();
        assert_eq!(std::fs::read(got.join("secret.txt")).unwrap(), b"from the laptop");
        // Unsafe names never get as far as Home PC's disk.
        let tx = laptop.inner.lock().unwrap().live.get(&h).map(|l| l.tx.clone()).unwrap();
        let bad = uuid::Uuid::new_v4().to_string();
        tx.send(Msg::Offer { id: bad.clone(), name: "x".into(), files: vec![OfferFile { path: "../secret.txt".into(), size: 1 }], resume: true, relay: Some(RelayTo { to: rado.my_id(), keep: false }) }).unwrap();
        // A relay to someone who isn't Home PC's contact (the laptop itself, or Home PC).
        for to in [laptop.my_id(), h.clone()] {
            let x = uuid::Uuid::new_v4().to_string();
            tx.send(Msg::Offer { id: x.clone(), name: "x".into(), files: vec![OfferFile { path: "a.txt".into(), size: 1 }], resume: true, relay: Some(RelayTo { to, keep: false }) }).unwrap();
        }
        // Stopping or telling about relays it never had: nothing happens.
        tx.send(Msg::Unrelay { id: id.clone() }).unwrap();
        tx.send(Msg::Unrelay { id: uuid::Uuid::new_v4().to_string() }).unwrap();
        tx.send(Msg::Relayed { id: id.clone(), state: "delivered".into(), done: 0, size: 0, message: None }).unwrap();
        tokio::time::sleep(Duration::from_millis(800)).await;
        let rs = home.full_state().relays;
        assert_eq!(rs.len(), 1, "only the one that went through");
        assert_eq!(rs[0].state, "delivered");
        assert_eq!(relay_dirs(home), 0);
        assert_eq!(std::fs::read(t.root.join("home/secret.txt")).unwrap(), b"home only");
        t.stop();
    }

    /// Rado declines: the laptop sees it declined, the copy goes, and the history says so.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_decline_cleans_up() {
        let t = three("decline").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        let src = t.root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("x.txt"), b"x").unwrap();
        let id = laptop.relay_send(&home.my_id(), &rado.my_id(), "Rado", vec![src.join("x.txt")], false, None, true).await.unwrap();
        let offer = offer_at(rado).await;
        assert_eq!(relay_dirs(home), 1);
        let r2 = rado.clone();
        tokio::task::spawn_blocking(move || r2.answer_offer(&offer.id, false, None)).await.unwrap().unwrap();
        until(|| state(laptop, &id) == "declined").await;
        until(|| relay_dirs(home) == 0).await;
        assert!(!t.root.join("home/Downloads/From Laptop/x.txt").exists(), "a copy is kept only once delivered");
        assert!(laptop.devices_state().log.iter().any(|l| l.text == "Relayed 1 file to Rado · Rado declined"));
        assert!(relay_of(laptop, &id).unwrap().ended.is_some());
        t.stop();
    }

    /// Never accepted: after 7 days the relay expires on Home PC (Rado's question goes, the copy
    /// goes) and the laptop hears it. A relay waiting for a contact who never comes back too.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_relay_never_accepted_expires_after_7_days() {
        let t = three("expire").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        let src = t.root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("x.txt"), b"x").unwrap();
        let id = laptop.relay_send(&home.my_id(), &rado.my_id(), "Rado", vec![src.join("x.txt")], false, None, false).await.unwrap();
        offer_at(rado).await;
        home.expire(now_ms() + super::super::resume::KEEP_MS - 60_000);
        assert_eq!(state(home, &id), "offered", "not yet");
        home.expire(now_ms() + super::super::resume::KEEP_MS + 1);
        assert_eq!(state(home, &id), "expired");
        until(|| state(laptop, &id) == "expired").await;
        until(|| rado.state().offers.is_empty()).await;
        assert_eq!(relay_dirs(home), 0);
        // Waiting for someone who isn't there: expires the same way.
        let away = iroh::SecretKey::generate().public().to_string();
        home.inner.lock().unwrap().contacts.push(Contact { id: away.clone(), name: "Away".into(), trusted: true, ..Default::default() });
        let id = laptop.relay_send(&home.my_id(), &away, "Away", vec![src.join("x.txt")], false, None, false).await.unwrap();
        until(|| state(laptop, &id) == "waiting").await;
        assert_eq!(relay_dirs(home), 1);
        home.expire(now_ms() + super::super::resume::KEEP_MS + 1);
        until(|| state(laptop, &id) == "expired").await;
        assert_eq!(relay_dirs(home), 0);
        // Ended relays are forgotten a week later.
        home.expire(now_ms() + 3 * super::super::resume::KEEP_MS);
        assert!(home.full_state().relays.is_empty());
        t.stop();
    }

    /// A cut in the middle of either leg: it waits, then continues from where it stopped, and
    /// Rado gets the whole file.
    #[tokio::test(flavor = "multi_thread")]
    async fn each_leg_continues_after_a_cut() {
        let t = three("resume").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        rado.edit_contact(&home.my_id(), None, Some(true), None).unwrap();
        let piece = super::super::PIECE as usize;
        let big = bytes_of(piece * 3 + 4321, 3);
        let src = t.root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("big.bin"), &big).unwrap();
        // Leg 1 cut half-way.
        laptop.pace.store(60, Ordering::Relaxed);
        let id = laptop.relay_send(&home.my_id(), &rado.my_id(), "Rado", vec![src.join("big.bin")], false, None, false).await.unwrap();
        until(|| home.full_state().transfers.iter().any(|x| x.id == id && x.done > 2 * 1024 * 1024)).await;
        cut(laptop, home);
        let st = |n: &Node, tid: &str| n.full_state().transfers.into_iter().find(|x| x.id == tid).map(|x| x.state);
        until(|| st(laptop, &id).as_deref() == Some("waiting") && st(home, &id).as_deref() == Some("waiting")).await;
        let had = home.full_state().transfers.into_iter().find(|x| x.id == id).unwrap().done;
        laptop.pace.store(0, Ordering::Relaxed);
        // Leg 2 will be slow, to cut it too.
        home.pace.store(60, Ordering::Relaxed);
        until(|| !home.inner.lock().unwrap().live.contains_key(&laptop.my_id())).await;
        laptop.reach(&home.my_id());
        let mut from = 0;
        until(|| {
            if let Some(o) = laptop.inner.lock().unwrap().outgoing.get(&id) {
                from = from.max(o.base);
            }
            state(home, &id) == "offered" || state(home, &id) == "sending"
        })
        .await;
        assert!(from >= had, "the first leg continued from {from}, not from zero ({had} there)");
        // Leg 2 cut half-way.
        let leg2 = || home.inner.lock().unwrap().relay.list.iter().find(|r| r.id == id).and_then(|r| r.leg2.clone());
        until(|| leg2().is_some_and(|l| rado.full_state().transfers.iter().any(|x| x.id == l && x.done > 2 * 1024 * 1024))).await;
        let l2 = leg2().unwrap();
        cut(home, rado);
        until(|| st(home, &l2).as_deref() == Some("waiting") && st(rado, &l2).as_deref() == Some("waiting")).await;
        let had2 = rado.full_state().transfers.into_iter().find(|x| x.id == l2).unwrap().done;
        home.pace.store(0, Ordering::Relaxed);
        until(|| state(laptop, &id) == "paused" || state(laptop, &id) == "sending").await;
        until(|| !rado.inner.lock().unwrap().live.contains_key(&home.my_id())).await;
        home.reach(&rado.my_id());
        let mut from2 = 0;
        until(|| {
            if let Some(o) = home.inner.lock().unwrap().outgoing.get(&l2) {
                from2 = from2.max(o.base);
            }
            state(laptop, &id) == "delivered"
        })
        .await;
        assert!(from2 >= had2, "the second leg continued from {from2}, not from zero ({had2} there)");
        assert_eq!(std::fs::read(t.root.join("rado/Downloads/From Sam at home/big.bin")).unwrap(), big);
        until(|| relay_dirs(home) == 0).await;
        t.stop();
    }

    /// Control turned off on Home PC mid-relay (a stolen laptop, say): what it asked to pass on
    /// stops at once: Rado's question goes, the copy goes, and the laptop hears why. Removing
    /// the device does the same for one still waiting for its contact.
    #[tokio::test(flavor = "multi_thread")]
    async fn turning_control_off_stops_what_that_pc_asked_to_pass_on() {
        let t = three("revoke").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        let src = t.root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("x.txt"), b"x").unwrap();
        let id = laptop.relay_send(&home.my_id(), &rado.my_id(), "Rado", vec![src.join("x.txt")], false, None, false).await.unwrap();
        offer_at(rado).await;
        until(|| state(laptop, &id) == "offered").await;
        home.set_control(&laptop.my_id(), false).unwrap();
        assert_eq!(state(home, &id), "failed", "stopped at once");
        until(|| state(laptop, &id) == "failed").await;
        assert_eq!(relay_of(laptop, &id).unwrap().message.as_deref(), Some(STOPPED));
        until(|| rado.state().offers.is_empty()).await;
        until(|| relay_dirs(home) == 0).await;
        // Allowed again; a relay waiting for someone away; then the laptop is removed there.
        home.set_control(&laptop.my_id(), true).unwrap();
        until(|| !laptop.relay_choices().is_empty()).await;
        let away = iroh::SecretKey::generate().public().to_string();
        home.inner.lock().unwrap().contacts.push(Contact { id: away.clone(), name: "Away".into(), trusted: true, ..Default::default() });
        let id = laptop.relay_send(&home.my_id(), &away, "Away", vec![src.join("x.txt")], false, None, false).await.unwrap();
        until(|| state(home, &id) == "waiting").await;
        assert_eq!(relay_dirs(home), 1);
        home.remove_contact(&laptop.my_id());
        assert_eq!(state(home, &id), "failed");
        until(|| relay_dirs(home) == 0).await;
        t.stop();
    }

    /// Cancelled on the laptop while Home PC wasn't connected (the "stop" never got there):
    /// when Home PC says it still has it, the laptop says stop again, and nothing goes on.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_cancel_while_that_pc_was_away_still_stops_it() {
        let t = three("cancel-away").await;
        let (laptop, home) = (&t.laptop, &t.home);
        let src = t.root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("x.txt"), b"x").unwrap();
        let away = iroh::SecretKey::generate().public().to_string();
        home.inner.lock().unwrap().contacts.push(Contact { id: away.clone(), name: "Away".into(), trusted: true, ..Default::default() });
        let id = laptop.relay_send(&home.my_id(), &away, "Away", vec![src.join("x.txt")], false, None, false).await.unwrap();
        until(|| state(laptop, &id) == "waiting" && state(home, &id) == "waiting").await;
        // What relay_cancel does when Home PC isn't connected: ended here, nothing sent.
        laptop.relay_end(&id, "cancelled", None);
        // Home PC connects again and says how it goes.
        home.relay_hello(&laptop.my_id());
        until(|| state(home, &id) == "cancelled").await;
        until(|| relay_dirs(home) == 0).await;
        t.stop();
    }

    /// Home PC's history read from the laptop never names a place on Home PC, also not in a
    /// reason ("Couldn't read C:\Users\…").
    #[tokio::test(flavor = "multi_thread")]
    async fn a_conversation_read_from_another_pc_names_no_place_there() {
        let t = three("places").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        let item = |id: &str, message: &str| HistoryItem {
            id: id.into(),
            contact: rado.my_id(),
            outgoing: true,
            name: "plan.txt".into(),
            files: 1,
            size: 1,
            date: now_ms(),
            path: None,
            result: "failed".into(),
            message: Some(message.into()),
            saved_to: None,
            duration_ms: 0,
            avg_speed: 0,
            peak_speed: 0,
            via: None,
            relay_to: None,
            items: vec![super::super::store::TransferItem { name: "plan.txt".into(), size: 1, files: 1, ..Default::default() }],
            more: 0,
        };
        {
            let mut inner = home.inner.lock().unwrap();
            inner.history.push(item(&uuid::Uuid::new_v4().to_string(), "Couldn't read C:\\Users\\Sam\\Secret\\plan.txt: Access is denied. (os error 5)"));
            inner.history.push(item(&uuid::Uuid::new_v4().to_string(), "Couldn't make the folder D:\\Private\\From Rado: Access is denied."));
            inner.history.push(item(&uuid::Uuid::new_v4().to_string(), "The connection was lost."));
        }
        let talk = laptop.remote_talk(&home.my_id(), &rado.my_id()).await.unwrap();
        let said: Vec<String> = talk.history.iter().filter_map(|h| h.message.clone()).collect();
        assert_eq!(said, vec!["Couldn't read one of the files.".to_string(), "Couldn't make the folder to save to.".into(), "The connection was lost.".into()]);
        t.stop();
    }

    /// "Keep a copy" when Home PC's folder for the laptop is on another drive than Pako's data
    /// (a move can't go there): copied instead, folders and all, never over a file there.
    #[test]
    fn a_kept_copy_is_copied_when_it_cannot_be_moved() {
        let root = std::env::temp_dir().join(format!("pako-relay-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let src = root.join("relay/Holiday");
        std::fs::create_dir_all(src.join("day 2")).unwrap();
        std::fs::write(src.join("a.jpg"), b"new a").unwrap();
        std::fs::write(src.join("day 2/b.jpg"), b"new b").unwrap();
        std::fs::write(root.join("relay/x.txt"), b"new x").unwrap();
        let to = root.join("From Laptop");
        std::fs::create_dir_all(to.join("Holiday")).unwrap();
        std::fs::write(to.join("Holiday/a.jpg"), b"old a").unwrap();
        std::fs::write(to.join("x.txt"), b"old x").unwrap();
        let got = copy_new(&src, &to.join("Holiday")).unwrap();
        assert_eq!(got, to.join("Holiday (2)"));
        assert_eq!(std::fs::read(got.join("a.jpg")).unwrap(), b"new a");
        assert_eq!(std::fs::read(got.join("day 2/b.jpg")).unwrap(), b"new b");
        assert_eq!(std::fs::read(to.join("Holiday/a.jpg")).unwrap(), b"old a");
        let got = copy_new(&root.join("relay/x.txt"), &to.join("x.txt")).unwrap();
        assert_eq!(got, to.join("x (2).txt"));
        assert_eq!((std::fs::read(&got).unwrap(), std::fs::read(to.join("x.txt")).unwrap()), (b"new x".to_vec(), b"old x".to_vec()));
        // Messages without a place stay as they are.
        assert_eq!(no_places("Too many sends in a short time. Try again in a minute."), "Too many sends in a short time. Try again in a minute.");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Older Pakos: a Home PC that doesn't say it can relay isn't in the menu and can't be
    /// sent through; Rado on an older Pako gets a normal send (without resume), as always.
    #[tokio::test(flavor = "multi_thread")]
    async fn older_pakos() {
        let t = three("older").await;
        let (laptop, home, rado) = (&t.laptop, &t.home, &t.rado);
        let h = home.my_id();
        // Rado's Pako is older: the relayed send is a plain send to it.
        rado.old_pako.store(true, Ordering::Relaxed);
        rado.edit_contact(&h, None, Some(true), None).unwrap();
        let src = t.root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("x.txt"), b"old").unwrap();
        let id = laptop.relay_send(&h, &rado.my_id(), "Rado", vec![src.join("x.txt")], false, None, true).await.unwrap();
        until(|| state(laptop, &id) == "delivered").await;
        assert_eq!(std::fs::read(t.root.join("rado/Downloads/From Sam at home/x.txt")).unwrap(), b"old");
        // Kept on Home PC, as asked: in its folder for files from the laptop.
        until(|| std::fs::read(t.root.join("home/Downloads/From Laptop/x.txt")).is_ok_and(|b| b == b"old")).await;
        assert_eq!(relay_dirs(home), 0);
        // Home PC's Pako doesn't say it can relay (an older Pako): not offered, refused here.
        laptop.on_device(&h, true, true, Some("Desk PC".into()), None);
        assert!(laptop.relay_choices().is_empty());
        assert!(laptop.remote_book(&h).await.is_err());
        assert!(laptop.relay_send(&h, &rado.my_id(), "Rado", vec![src.join("x.txt")], false, None, false).await.is_err());
        t.stop();
    }
}
