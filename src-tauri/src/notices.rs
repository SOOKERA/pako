// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Pako's own notifications: pop-ups (small cards at the bottom right of the screen, in a
//! window that never takes the focus) and the bell (a history in the top bar, kept 30 days in
//! `data/notices.json`). Each kind has its own Pop-up and Bell switches in Settings ›
//! Notifications. Everything Pako tells you goes through `post`.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::model::{Alert, DoneChime, Settings};

/// One kind of notification: its id, its name in Settings (plain words), and whether the bell
/// keeps it by default. Every kind gets a pop-up by default.
/// The window has the same table, with icons, in `src/lib/notices.ts` (a test checks they match).
pub struct Kind {
    pub id: &'static str,
    /// Its name in Settings › Notifications (shown by the window from its copy of this table).
    #[cfg_attr(not(test), allow(dead_code))]
    pub label: &'static str,
    pub bell: bool,
}

pub const KINDS: &[Kind] = &[
    Kind { id: "download-done", label: "Download finished", bell: false },
    Kind { id: "download-failed", label: "Download stopped", bell: false },
    Kind { id: "browser", label: "Sent from your browser", bell: false },
    // Bell too: the player's button said "Sent" before Pako knew (see `video_from_browser`).
    Kind { id: "video-failed", label: "Couldn't get a video", bell: true },
    Kind { id: "upload-done", label: "Upload finished", bell: false },
    Kind { id: "upload-failed", label: "Couldn't upload", bell: false },
    Kind { id: "upload-deleted", label: "Upload deleted on its date", bell: false },
    Kind { id: "direct-request", label: "Someone wants to connect", bell: true },
    Kind { id: "direct-offer", label: "Files waiting for your answer", bell: true },
    Kind { id: "direct-incoming", label: "Files coming in", bell: false },
    Kind { id: "direct-received", label: "Files received", bell: false },
    Kind { id: "direct-sent", label: "Files delivered", bell: false },
    Kind { id: "needs-you", label: "Pako needs you", bell: true },
    Kind { id: "power", label: "Sleep and shut down", bell: false },
    Kind { id: "update-ready", label: "New Pako ready", bell: true },
    Kind { id: "other", label: "Other messages", bell: false },
];

/// Pop-ups of these kinds wait for an answer: they stay until answered or closed.
const STAYS: &[&str] = &["direct-offer", "direct-request", "needs-you"];

/// The bell keeps this long, and at most this many.
const KEEP_MS: u64 = 30 * 24 * 3600 * 1000;
const KEEP_MAX: usize = 300;

/// A kind's two switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pref {
    pub popup: bool,
    pub bell: bool,
}

fn kind_of(id: &str) -> &'static Kind {
    KINDS.iter().find(|k| k.id == id).unwrap_or_else(|| KINDS.iter().find(|k| k.id == "other").unwrap())
}

/// A kind's switches: the defaults, with what was changed in Settings › Notifications on top.
/// "Download finished" has no Pop-up switch: each download says how it tells you (Off ·
/// Pop-up · Important), new ones as Settings › Notifications › New downloads says; its pop-up
/// "switch" here is that default (for what has no download of its own, like an extraction).
pub fn pref(s: &Settings, kind: &str) -> Pref {
    let k = kind_of(kind);
    let changed = s.notices.get(k.id).copied().unwrap_or_default();
    let popup = if k.id == "download-done" { s.done_alert() != Alert::Off } else { changed.popup.unwrap_or(true) };
    Pref { popup, bell: changed.bell.unwrap_or(k.bell) }
}

/// Where a click on a notification leads: a page of Pako's window and, when there is one, the
/// item to show there (a download, a link, an upload or a contact). Direct's questions also
/// carry what they answer: the offer (Accept / Decline) or the person asking to connect.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Go {
    /// "downloads", "found", "send", "direct", "devices", "dashboard", or "" (just open Pako).
    pub page: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer: Option<String>,
}

impl Go {
    /// Nowhere in particular: a click just opens Pako.
    pub fn none() -> Self {
        Self::default()
    }
    pub fn page(page: &str) -> Self {
        Self { page: page.into(), ..Self::default() }
    }
    pub fn item(page: &str, id: impl Into<String>) -> Self {
        Self { page: page.into(), id: Some(id.into()), ..Self::default() }
    }
    /// A page, and an item on it when there is exactly one.
    pub fn maybe(page: &str, id: Option<String>) -> Self {
        Self { page: page.into(), id, ..Self::default() }
    }
}

/// One line in the bell.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    /// When it came (ms since 1970).
    pub at: u64,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub go: Go,
    /// A question once settled ("Accepted", "Declined", "Done"...), see `resolve`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered: Option<String>,
}

/// One card for the pop-up window.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Popup {
    /// The same id as its line in the bell, if the bell keeps it.
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub go: Go,
    /// Waits for an answer: no leaving by itself.
    pub stays: bool,
    /// The Important card for several downloads at once ("3 downloads finished"): how many.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub many: Option<u32>,
}

/// Whether a pop-up shows: as the kind's switch says, always (a download set to Pop-up), never
/// (Direct while Pako's window is in front: its question is on screen; a download set to Off),
/// or as the big card in the middle of the screen (a download set to Important: through Do not
/// disturb, but never over something full screen).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pop {
    Pref,
    Force,
    Skip,
    Important,
}

impl Pop {
    fn name(self) -> &'static str {
        match self {
            Pop::Pref => "pref",
            Pop::Force => "force",
            Pop::Skip => "skip",
            Pop::Important => "important",
        }
    }
}

/// The engine sends its notifications as an event ("notify") with this payload.
pub fn payload(kind: &str, title: &str, body: &str, go: &Go, pop: Pop) -> serde_json::Value {
    serde_json::json!({ "kind": kind, "title": title, "body": body, "go": go, "pop": pop.name() })
}

/// The same for a private window's download: a pop-up only (as Settings say), never a line in
/// the bell (nothing about it is kept).
pub fn payload_private(kind: &str, title: &str, body: &str, go: &Go, pop: Pop) -> serde_json::Value {
    let mut p = payload(kind, title, body, go, pop);
    p["noBell"] = serde_json::Value::Bool(true);
    p
}

/// Posts what the engine sent (see `payload`).
pub fn post_payload(app: &AppHandle, p: &serde_json::Value) {
    let text = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let go = p.get("go").cloned().and_then(|g| serde_json::from_value(g).ok()).unwrap_or_default();
    let pop = match p.get("pop").and_then(|v| v.as_str()) {
        Some("force") => Pop::Force,
        Some("skip") => Pop::Skip,
        Some("important") => Pop::Important,
        _ => Pop::Pref,
    };
    let kind = text("kind");
    let bell = p.get("noBell").and_then(|v| v.as_bool()) != Some(true);
    let many = p.get("many").and_then(|v| v.as_u64()).map(|n| n as u32);
    post_as(app, if kind.is_empty() { "other" } else { &kind }, &text("title"), &text("body"), go, pop, bell, many);
}

// ---------- the settings in use ----------

/// The notification settings in use, kept here by the engine whenever settings load or change,
/// so posting never waits for the engine (it may be busy, or not started yet).
static LIVE: Mutex<Option<Settings>> = Mutex::new(None);

pub fn use_settings(s: &Settings) {
    *LIVE.lock().unwrap() = Some(s.clone());
}

fn live_pref(kind: &str) -> (Pref, bool) {
    let s = LIVE.lock().unwrap().clone().unwrap_or_default();
    (pref(&s, kind), quiet_for(&s))
}

/// No pop-ups now: Pako's Do not disturb, or Windows says not now (when that is followed).
/// The finished chime keeps quiet then too.
fn quiet_for(s: &Settings) -> bool {
    quiet_when(s.quiet, s.follow_windows_quiet, windows_says_not_now)
}

/// `quiet_for` with Windows' answer passed in (only asked when it is followed).
fn quiet_when(do_not_disturb: bool, follow_windows: bool, windows_not_now: impl FnOnce() -> bool) -> bool {
    do_not_disturb || (follow_windows && windows_not_now())
}

fn quiet_now() -> bool {
    let s = LIVE.lock().unwrap().clone().unwrap_or_default();
    quiet_for(&s)
}

// ---------- the page Pako's window shows ----------

/// The page Pako's main window shows ("downloads", "direct", "devices"…), as it says each time
/// it changes (`window_page`).
static WINDOW_PAGE: Mutex<String> = Mutex::new(String::new());

/// Pako's window says which page it shows.
#[tauri::command]
pub fn window_page(page: String) {
    *WINDOW_PAGE.lock().unwrap() = page;
}

/// News that leads to `go`'s page (Direct's, My devices'): no pop-up while Pako's window is in
/// front (`in_front`) showing that very page, where you see it already; on any other page, or
/// with the window behind another app, its pop-up comes as its switch says. Direct's questions
/// ("Files for you", "Someone wants to connect") are asked inside the window on every page
/// (DirectPrompts), so with the window in front they never pop up as well; only pairing a
/// device isn't asked there.
pub fn pop_unless_on_page(in_front: bool, kind: &str, go: &Go) -> Pop {
    pop_for(in_front, &WINDOW_PAGE.lock().unwrap(), kind, &go.page)
}

fn pop_for(in_front: bool, shown: &str, kind: &str, page: &str) -> Pop {
    let asked_in_window = kind == "direct-offer" || (kind == "direct-request" && page != "devices");
    if in_front && (asked_in_window || (!page.is_empty() && shown == page)) { Pop::Skip } else { Pop::Pref }
}

// ---------- posting ----------

/// Pop-ups waiting for the pop-up window to take them.
static WAITING: Mutex<Vec<Popup>> = Mutex::new(Vec::new());
/// Question cards given to the pop-up window (id, kind, go), so that settling a question
/// (`resolve`) can tell the window which cards to drop. The newest few are enough.
static ASKING: Mutex<Vec<(String, String, Go)>> = Mutex::new(Vec::new());
const ASKING_MAX: usize = 50;

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// What a question is about, so it can be settled later: the link's row ("Pako needs you"),
/// the offer, or the person asking to connect. Other kinds aren't questions.
fn about<'a>(kind: &str, go: &'a Go) -> Option<&'a str> {
    match kind {
        "needs-you" => go.id.as_deref(),
        "direct-offer" => go.offer.as_deref(),
        "direct-request" => go.peer.as_deref(),
        _ => None,
    }
}

/// Whether a pop-up shows: as `pop` and the kind's Pop-up switch say, never at a quiet time,
/// and no "Download finished" while the small window is on screen (it shows the download
/// finishing itself; a download's own "Notify when done" included).
fn popup_shows(kind: &str, pop: Pop, kind_popup: bool, quiet: bool, small_window_open: bool) -> bool {
    let wanted = match pop {
        Pop::Force => true,
        // The big card is shown on its own (`post_as`), never in the corner.
        Pop::Skip | Pop::Important => false,
        Pop::Pref => kind_popup,
    };
    wanted && !quiet && !(kind == "download-done" && small_window_open)
}

/// Tells you something: into the bell if its kind goes there, and as a pop-up if its kind
/// (or `pop`) says so, unless Do not disturb is on or Windows says not now (see
/// `popup_shows`). A question about the same thing as an earlier one (the same link asking
/// again) takes its place.
pub fn post(app: &AppHandle, kind: &str, title: &str, body: &str, go: Go, pop: Pop) {
    post_as(app, kind, title, body, go, pop, true, None);
}

/// `bell` = false: never a line in the bell (a private window's download). `many`: an
/// Important card for several downloads.
#[allow(clippy::too_many_arguments)]
fn post_as(app: &AppHandle, kind: &str, title: &str, body: &str, go: Go, pop: Pop, bell: bool, many: Option<u32>) {
    let kind = kind_of(kind).id;
    let (p, quiet) = live_pref(kind);
    let id = uuid::Uuid::new_v4().simple().to_string();
    let key = about(kind, &go).map(str::to_string);
    let same = |k: &str, g: &Go| key.is_some() && k == kind && about(k, g) == key.as_deref();
    let in_bell = p.bell && bell;
    if in_bell {
        let n = Notice { id: id.clone(), kind: kind.into(), title: title.into(), body: body.into(), at: now_ms(), read: false, go: go.clone(), answered: None };
        change(app, |list| {
            list.retain(|o| !same(&o.kind, &o.go));
            list.push(n);
            prune(list, now_ms());
            true
        });
    }
    // The big card, in its own window; without that window (it couldn't be made), the corner
    // card instead, whatever Settings say.
    let pop = if pop == Pop::Important && app.get_webview_window("important").is_none() { Pop::Force } else { pop };
    if pop == Pop::Important {
        let card = Popup { id, kind: kind.into(), title: title.into(), body: body.into(), go, stays: true, many };
        let dropped = hold_important(card, in_bell, bell);
        for (c, ..) in dropped {
            bell_line(app, &c);
        }
        let _ = app.emit_to("important", "important-new", ());
        return;
    }
    let small_window_open = kind == "download-done" && crate::tray::mini_showing(app);
    if popup_shows(kind, pop, p.popup, quiet, small_window_open) {
        drop_cards(app, &same);
        let stays = STAYS.contains(&kind);
        if stays && key.is_some() {
            let mut asking = ASKING.lock().unwrap();
            asking.push((id.clone(), kind.into(), go.clone()));
            if asking.len() > ASKING_MAX {
                let extra = asking.len() - ASKING_MAX;
                asking.drain(..extra);
            }
        }
        WAITING.lock().unwrap().push(Popup { id, kind: kind.into(), title: title.into(), body: body.into(), go, stays, many: None });
        let _ = app.emit_to("popup", "popup-new", ());
    }
}

/// Takes back the question cards `is_it` picks: those not taken yet, and those on screen (the
/// pop-up window is told to drop them).
fn drop_cards(app: &AppHandle, is_it: &dyn Fn(&str, &Go) -> bool) {
    WAITING.lock().unwrap().retain(|p| !is_it(&p.kind, &p.go));
    let gone: Vec<String> = {
        let mut asking = ASKING.lock().unwrap();
        let gone = asking.iter().filter(|(_, k, g)| is_it(k, g)).map(|(id, _, _)| id.clone()).collect();
        asking.retain(|(_, k, g)| !is_it(k, g));
        gone
    };
    if !gone.is_empty() {
        let _ = app.emit_to("popup", "popup-resolve", gone);
    }
}

/// A question was settled, wherever that happened (answered in Pako, in a pop-up or in the
/// bell; the link page finished or closed; the other person gave up): its bell line says how
/// (`text`, e.g. "Accepted", "Done") and counts as read, and its pop-up leaves.
/// `kind` and `key` say which question (see `about`).
pub fn resolve(app: &AppHandle, kind: &str, key: &str, text: &str) {
    let is_it = |k: &str, g: &Go| k == kind && about(k, g) == Some(key);
    change(app, |list| {
        let mut any = false;
        for n in list.iter_mut().filter(|n| n.answered.is_none() && is_it(&n.kind, &n.go)) {
            n.answered = Some(text.into());
            n.read = true;
            any = true;
        }
        any
    });
    drop_cards(app, &is_it);
}

// ---------- the two sounds ----------

/// Pako's two sounds, by direction: files coming to you, and files you sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// "Arrived" (`done.wav`): a download you asked about finished, files arrived in Direct or
    /// My devices, or someone offers you files and waits for your answer.
    Arrived,
    /// "Sent" (`sent.wav`): an upload finished, or a Direct or My devices send was delivered.
    Sent,
}

/// The arrived chime: two soft rising notes, about 0.42 s (made by `scripts/make-done-sound.mjs`).
static DONE_SOUND: &[u8] = include_bytes!("../sounds/done.wav");
/// The sent sound: three bright rising notes, about 0.55 s (made by `scripts/make-sent-sound.mjs`).
static SENT_SOUND: &[u8] = include_bytes!("../sounds/sent.wav");

/// The same sound this close together plays once (several downloads finishing together).
const CHIME_GAP_MS: u64 = 2000;

/// When each sound last played (ms since 1970): arrived, sent.
static LAST_CHIME: Mutex<[Option<u64>; 2]> = Mutex::new([None, None]);

impl Sound {
    fn wav(self) -> &'static [u8] {
        match self {
            Sound::Arrived => DONE_SOUND,
            Sound::Sent => SENT_SOUND,
        }
    }

    fn slot(self) -> usize {
        match self {
            Sound::Arrived => 0,
            Sound::Sent => 1,
        }
    }

    /// Its switch in Settings › Notifications ("Play a sound when files arrive" / "… are sent").
    fn on(self, s: &Settings) -> bool {
        match self {
            Sound::Arrived => s.play_done_sound,
            Sound::Sent => s.play_sent_sound,
        }
    }
}

/// The sound a Direct or My devices notice brings: files offered and waiting for your answer,
/// and files received. "Files coming in" (sent without asking) has none: those chime once,
/// when they have arrived. So an offer you answer chimes twice (it needed you, then it's
/// here), one sent without asking once. A muted contact's notices never come, nor their sound.
pub fn sound_of_notice(kind: &str) -> Option<Sound> {
    match kind {
        "direct-offer" | "direct-received" => Some(Sound::Arrived),
        _ => None,
    }
}

/// Whether a sound plays: it is wanted (`wanted`, e.g. the download asked "Notify when done"),
/// its switch is on, it isn't a quiet time (as for pop-ups: Do not disturb, or Windows says not
/// now), and the same sound didn't play within 2 seconds. The small window being open doesn't
/// matter (it only stops the pop-up).
fn chime_plays(wanted: bool, sound_on: bool, quiet: bool, last: Option<u64>, now: u64) -> bool {
    wanted && sound_on && !quiet && last.is_none_or(|l| now < l || now - l >= CHIME_GAP_MS)
}

/// Whether a finished download at `alert` chimes, as "Chime when a download finishes" says.
pub fn done_chimes(alert: Alert, chime: DoneChime) -> bool {
    match (alert, chime) {
        (Alert::Off, _) | (_, DoneChime::Never) => false,
        (Alert::Pop, DoneChime::Important) => false,
        _ => true,
    }
}

/// A download (or a whole folder or set) finished, at `alert`: the arrived chime if "Chime when
/// a download finishes" wants it. Pop-up keeps quiet at quiet times like its card; Important
/// (called when its card shows) gets through Do not disturb like its card. Never waits.
pub fn chime_done(alert: Alert) {
    let s = LIVE.lock().unwrap().clone().unwrap_or_default();
    if !done_chimes(alert, s.done_chime()) {
        return;
    }
    let quiet = alert != Alert::Important && quiet_for(&s);
    play_now(Sound::Arrived, true, quiet);
}

/// Plays `which` if its switch is on and `chime_plays` says so. Never waits for the sound.
pub fn sound(which: Sound) {
    let s = LIVE.lock().unwrap().clone().unwrap_or_default();
    // Windows is only asked when the sound is on.
    let on = which.on(&s);
    let quiet = on && quiet_for(&s);
    play_now(which, on, quiet);
}

/// Plays `which` when it's `on`, not a quiet time and it didn't just play (`chime_plays`).
fn play_now(which: Sound, on: bool, quiet: bool) {
    let now = now_ms();
    if !sound_now(which, on, quiet, &mut LAST_CHIME.lock().unwrap(), now) {
        return;
    }
    // Windows plays one sound at a time (a new one cuts the one playing): the other sound,
    // still playing, finishes first.
    let len = length_ms(which.wav());
    let wait = take_turn(&mut PLAYING_UNTIL.lock().unwrap(), now, len);
    if wait == 0 {
        play(which.wav());
    } else {
        // (The thread stays while it plays, so its sound is never tied to a thread gone.)
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(wait));
            play(which.wav());
            std::thread::sleep(std::time::Duration::from_millis(len));
        });
    }
}

/// Until when (ms since 1970) the last sound started plays.
static PLAYING_UNTIL: Mutex<u64> = Mutex::new(0);

/// A short breath between two sounds played one after the other.
const SOUND_GAP_MS: u64 = 80;
/// The longest a sound waits for another (a clock set back can't make it wait longer).
const MAX_SOUND_WAIT_MS: u64 = 1500;

/// How long a sound starting `now` waits (ms) for the one playing to end (`until`), and keeps
/// its own end in `until`.
fn take_turn(until: &mut u64, now: u64, len: u64) -> u64 {
    let start = if *until > now { (*until + SOUND_GAP_MS).min(now + MAX_SOUND_WAIT_MS) } else { now };
    *until = start + len;
    start - now
}

/// How long one of Pako's WAVs plays (ms), from its header (data size / bytes per second).
fn length_ms(wav: &[u8]) -> u64 {
    let u32_at = |i: usize| wav.get(i..i + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64);
    let rate = u32_at(28);
    if rate == 0 {
        return 0;
    }
    u32_at(40) * 1000 / rate
}

/// Whether `which` plays now (`chime_plays` with its own switch and its own last time, kept in
/// `last` when it plays): each sound is held back only by itself, so a file sent right after
/// one arrived still sounds.
fn sound_now(which: Sound, on: bool, quiet: bool, last: &mut [Option<u64>; 2], now: u64) -> bool {
    let at = &mut last[which.slot()];
    let plays = chime_plays(true, on, quiet, *at, now);
    if plays {
        *at = Some(now);
    }
    plays
}

/// Plays a WAV held in memory, in the background. Any failure is ignored (no sound).
#[cfg(all(windows, not(test)))]
fn play(wav: &'static [u8]) {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
    // SAFETY: with SND_MEMORY the first argument is the WAV's bytes, not a name; they are
    // 'static, so they outlive the sound playing on after the call (SND_ASYNC).
    unsafe {
        PlaySoundW(wav.as_ptr() as _, std::ptr::null_mut(), SND_MEMORY | SND_ASYNC | SND_NODEFAULT);
    }
}

/// No sound outside Windows, nor while testing.
#[cfg(any(not(windows), test))]
fn play(_wav: &'static [u8]) {}

// ---------- the bell's history ----------

static STORE: Mutex<Option<Vec<Notice>>> = Mutex::new(None);

fn store_path() -> PathBuf {
    crate::paths::data_dir().join("notices.json")
}

fn load() -> Vec<Notice> {
    let mut list = load_from(&store_path());
    list.sort_by_key(|n| n.at);
    prune(&mut list, now_ms());
    list
}

/// The saved history. A file that can't be read as one is kept aside as `notices.json.bad`
/// (the next notice would otherwise overwrite it) and the bell starts empty.
fn load_from(path: &std::path::Path) -> Vec<Notice> {
    let Ok(bytes) = std::fs::read(path) else { return Vec::new() };
    match serde_json::from_slice(&bytes) {
        Ok(list) => list,
        Err(_) => {
            let mut bad = path.as_os_str().to_owned();
            bad.push(".bad");
            let bad = PathBuf::from(bad);
            let _ = std::fs::remove_file(&bad);
            let _ = std::fs::rename(path, &bad);
            Vec::new()
        }
    }
}

/// Older than 30 days goes, and only the newest 300 stay (the list is oldest first).
fn prune(list: &mut Vec<Notice>, now: u64) {
    list.retain(|n| n.at.saturating_add(KEEP_MS) >= now);
    if list.len() > KEEP_MAX {
        let extra = list.len() - KEEP_MAX;
        list.drain(..extra);
    }
}

fn with_store<R>(f: impl FnOnce(&mut Vec<Notice>) -> R) -> R {
    let mut g = STORE.lock().unwrap();
    let list = g.get_or_insert_with(load);
    f(list)
}

/// Changes the history; when `f` says something changed, it is saved and the window and the
/// tray icon's dot follow.
fn change(app: &AppHandle, f: impl FnOnce(&mut Vec<Notice>) -> bool) {
    let changed = with_store(|list| {
        if !f(list) {
            return false;
        }
        if let Ok(bytes) = serde_json::to_vec(list) {
            let _ = crate::paths::write_atomic(&store_path(), &bytes);
        }
        // Noted under the history's lock, so the last change always decides the dot (the
        // tray itself is only touched later, on the main thread).
        crate::tray::want_dot(list.iter().any(|n| !n.read));
        true
    });
    if changed {
        let _ = app.emit_to("main", "notices-changed", ());
        crate::tray::show_dot(app);
    }
}

/// At start: the tray icon gets its dot if the bell has something unread. Links that were
/// still waiting for you when Pako closed aren't any more (their link pages closed with it).
pub fn start(app: &AppHandle) {
    change(app, |list| {
        let mut any = false;
        for n in list.iter_mut().filter(|n| n.kind == "needs-you" && n.answered.is_none()) {
            n.answered = Some("Not finished".into());
            n.read = true;
            any = true;
        }
        any
    });
    with_store(|list| crate::tray::want_dot(list.iter().any(|n| !n.read)));
    crate::tray::show_dot(app);
}

/// Lines about this download go from the bell, and from its saved history and safety copy
/// (Move to Private: its name mustn't stay there).
pub fn forget_about(id: &str) {
    let changed = with_store(|list| {
        let before = list.len();
        list.retain(|n| n.go.id.as_deref() != Some(id));
        if before == list.len() {
            return false;
        }
        if let Ok(bytes) = serde_json::to_vec(list) {
            let _ = crate::paths::write_atomic(&store_path(), &bytes);
        }
        crate::tray::want_dot(list.iter().any(|n| !n.read));
        true
    });
    if changed {
        crate::paths::bak_like_now(&store_path());
    }
}

#[tauri::command]
pub fn notices_list() -> Vec<Notice> {
    with_store(|list| list.iter().rev().cloned().collect())
}

/// Marks these as read (or all of them, with no ids).
/// (Off the window's thread: saving the history writes to disk.)
#[tauri::command]
pub async fn notices_read(app: AppHandle, ids: Option<Vec<String>>) {
    let _ = tauri::async_runtime::spawn_blocking(move || mark_read(&app, ids)).await;
}

fn mark_read(app: &AppHandle, ids: Option<Vec<String>>) {
    change(app, |list| {
        let mut any = false;
        for n in list.iter_mut().filter(|n| !n.read && ids.as_ref().is_none_or(|ids| ids.contains(&n.id))) {
            n.read = true;
            any = true;
        }
        any
    });
}

#[tauri::command]
pub async fn notices_clear(app: AppHandle) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        change(&app, |list| {
            let any = !list.is_empty();
            list.clear();
            any
        })
    })
    .await;
}

// ---------- the pop-up window ----------

/// The pop-up window's width in logical pixels: the cards (340) and a see-through margin around
/// them for their shadow. Its height follows the cards (`popup_fit`).
const POP_W: f64 = 368.0;

/// Made once at start (hidden), so the first pop-up shows at once.
pub fn make_window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(w) = app.get_webview_window("popup") {
        return Some(w);
    }
    WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("popup".into()))
        .title("Pako")
        .inner_size(POP_W, 100.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        // Never takes the focus, even when clicked: you stay in what you were doing.
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()
        .ok()
        // A window manager (PowerToys FancyZones) never snaps it into a zone.
        .inspect(crate::tray::keep_out_of_zones)
}

/// The pop-up window takes the pop-ups waiting for it.
#[tauri::command]
pub fn popup_take() -> Vec<Popup> {
    std::mem::take(&mut *WAITING.lock().unwrap())
}

/// The pop-up window's cards are this tall (logical pixels, margin included): the window takes
/// that height, its bottom right corner on the bottom right of the main screen's free area
/// (above the taskbar). 0 = no cards: hidden. Also hidden while it isn't the time for
/// pop-ups (`popup_quiet`): the cards wait in it.
#[tauri::command]
pub fn popup_fit(app: AppHandle, height: f64) {
    let Some(w) = app.get_webview_window("popup") else { return };
    if height < 1.0 || quiet_now() {
        crate::tray::hide_window(&w);
        return;
    }
    let Some(m) = app.primary_monitor().ok().flatten() else { return };
    let scale = m.scale_factor();
    let area = m.work_area();
    let pw = (POP_W * scale).round() as i32;
    let ph = ((height * scale).ceil() as i32).min(area.size.height as i32);
    let x = area.position.x + area.size.width as i32 - pw;
    let y = area.position.y + area.size.height as i32 - ph;
    place(&w, x, y, pw, ph);
    crate::tray::show_without_focus(&w);
}

/// Moves and sizes the window in one go (separately, it would grow down first, then jump up).
#[cfg(windows)]
fn place(w: &WebviewWindow, x: i32, y: i32, width: i32, height: i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOCOPYBITS, SetWindowPos};
    match w.hwnd() {
        // SAFETY: the window's own handle, valid while `w` lives.
        Ok(h) => unsafe {
            SetWindowPos(h.0 as _, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE | SWP_NOCOPYBITS);
        },
        Err(_) => {
            let _ = w.set_size(tauri::PhysicalSize::new(width as u32, height as u32));
            let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
        }
    }
}

#[cfg(not(windows))]
fn place(w: &WebviewWindow, x: i32, y: i32, width: i32, height: i32) {
    let _ = w.set_size(tauri::PhysicalSize::new(width as u32, height as u32));
    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Asked every few seconds by the pop-up window while it has cards (and at once when settings
/// are saved): true = not the time for pop-ups (Do not disturb, a full-screen game, a
/// presentation...). The window is then hidden, its cards kept; it shows them again
/// (`popup_fit`) once this says false.
#[tauri::command]
pub fn popup_quiet(app: AppHandle) -> bool {
    let quiet = quiet_now();
    if quiet {
        if let Some(w) = app.get_webview_window("popup") {
            crate::tray::hide_window(&w);
        }
    }
    quiet
}

/// A click on a card: Pako opens on the right page (and its bell line counts as read).
#[tauri::command]
pub fn popup_open(app: AppHandle, go: Go, id: Option<String>) {
    if let Some(id) = id {
        mark_read(&app, Some(vec![id]));
    }
    crate::tray::show_main(&app);
    let _ = app.emit_to("main", "notice-go", go);
}

// ---------- the Important card ----------

/// Important cards waiting for their window to take them.
static IMPORTANT: Mutex<Vec<Popup>> = Mutex::new(Vec::new());
/// Important cards not shown yet (taken or not), with whether their line is in the bell and
/// whether they may go there at all (a private window's never does).
static UNSHOWN: Mutex<Vec<(Popup, bool, bool)>> = Mutex::new(Vec::new());
/// At most this many Important cards wait; older ones go to the bell instead.
const IMPORTANT_MAX: usize = 20;

/// Keeps a new Important card waiting; returns the ones pushed out by the limit that still need
/// their bell line (`bell_line`).
fn hold_important(card: Popup, in_bell: bool, bell_ok: bool) -> Vec<(Popup, bool, bool)> {
    let mut unshown = UNSHOWN.lock().unwrap();
    let mut waiting = IMPORTANT.lock().unwrap();
    unshown.push((card.clone(), in_bell, bell_ok));
    waiting.push(card);
    let over = unshown.len().saturating_sub(IMPORTANT_MAX);
    let out: Vec<(Popup, bool, bool)> = unshown.drain(..over).collect();
    waiting.retain(|c| !out.iter().any(|(o, ..)| o.id == c.id));
    out.into_iter().filter(|(_, in_bell, bell_ok)| !in_bell && *bell_ok).collect()
}

/// An Important card that never showed: its line in the bell (also when Download finished's
/// Bell switch is off), so it isn't lost.
fn bell_line(app: &AppHandle, c: &Popup) {
    let n = Notice { id: c.id.clone(), kind: c.kind.clone(), title: c.title.clone(), body: c.body.clone(), at: now_ms(), read: false, go: c.go.clone(), answered: None };
    change(app, |list| {
        list.retain(|o| o.id != n.id);
        list.push(n);
        prune(list, now_ms());
        true
    });
}

/// Pako quits: Important cards that never showed go to the bell.
pub fn important_at_quit(app: &AppHandle) {
    let left: Vec<(Popup, bool, bool)> = std::mem::take(&mut *UNSHOWN.lock().unwrap());
    for (c, in_bell, bell_ok) in left {
        if !in_bell && bell_ok {
            bell_line(app, &c);
        }
    }
}

/// The Important window's width (logical pixels): the card (420) and a see-through margin for
/// its shadow. Its height follows the card (`important_fit`).
const BIG_W: f64 = 452.0;

/// Made once at start (hidden), so an Important card shows at once.
pub fn make_important_window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(w) = app.get_webview_window("important") {
        return Some(w);
    }
    WebviewWindowBuilder::new(app, "important", WebviewUrl::App("important".into()))
        .title("Pako")
        .inner_size(BIG_W, 200.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        // Never takes the focus: what you were typing keeps going where it was.
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()
        .ok()
        .inspect(crate::tray::keep_out_of_zones)
}

/// The Important window takes the cards waiting for it.
#[tauri::command]
pub fn important_take() -> Vec<Popup> {
    std::mem::take(&mut *IMPORTANT.lock().unwrap())
}

/// Whether the big card waits: never over something full screen (a game, a video, a
/// presentation). Do not disturb doesn't hold it back.
fn important_waits(full_screen: bool) -> bool {
    full_screen
}

/// The Important window's card is this tall (logical pixels, margin included): the window takes
/// that height, in the middle of the screen the mouse is on. 0 = no card: hidden. While
/// something is full screen it stays hidden and says so (true = waiting): the window asks again
/// every couple of seconds.
#[tauri::command]
pub fn important_fit(app: AppHandle, height: f64) -> bool {
    let Some(w) = app.get_webview_window("important") else { return false };
    if height < 1.0 {
        crate::tray::hide_window(&w);
        return false;
    }
    if important_waits(full_screen_now()) {
        crate::tray::hide_window(&w);
        return true;
    }
    let Some(at) = crate::tray::middle_of_pointer_screen(&app, BIG_W, height) else { return false };
    let scale = app
        .monitor_from_point(at.x, at.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let (bw, bh) = ((BIG_W * scale).round() as i32, (height * scale).ceil() as i32);
    if !w.is_visible().unwrap_or(false) {
        place(&w, at.x as i32, at.y as i32, bw, bh);
        crate::tray::show_without_focus(&w);
    } else if let Ok(size) = w.outer_size() {
        // Already showing: only its height follows, the top stays put.
        if size.height as i32 != bh {
            if let Ok(pos) = w.outer_position() {
                place(&w, pos.x, pos.y, bw, bh);
            }
        }
    }
    false
}

/// An Important card came into view: its chime (as "Chime when a download finishes" says),
/// once; it's no longer one that never showed.
#[tauri::command]
pub fn important_shown(id: String) {
    let mut unshown = UNSHOWN.lock().unwrap();
    let before = unshown.len();
    unshown.retain(|(c, ..)| c.id != id);
    let first_time = unshown.len() != before;
    drop(unshown);
    if first_time {
        chime_done(Alert::Important);
    }
}

#[cfg(windows)]
fn full_screen_now() -> bool {
    busy_full_screen()
}

#[cfg(not(windows))]
fn full_screen_now() -> bool {
    false
}

// ---------- Windows says "not now" ----------

/// Windows says not now: a full-screen game or video, a presentation, or Windows' own Do not
/// disturb (Focus). Best effort: any failure counts as "fine to show".
#[cfg(windows)]
fn windows_says_not_now() -> bool {
    busy_full_screen() || windows_do_not_disturb()
}

#[cfg(not(windows))]
fn windows_says_not_now() -> bool {
    false
}

/// Windows' documented check (SHQueryUserNotificationState): something full screen or a
/// presentation.
#[cfg(windows)]
fn busy_full_screen() -> bool {
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHQueryUserNotificationState(state: *mut i32) -> i32;
    }
    // QUERY_USER_NOTIFICATION_STATE: 2 = busy (a full-screen app), 3 = a full-screen Direct3D
    // game or video, 4 = presentation mode.
    const QUNS_BUSY: i32 = 2;
    const QUNS_RUNNING_D3D_FULL_SCREEN: i32 = 3;
    const QUNS_PRESENTATION_MODE: i32 = 4;
    let mut state = 0i32;
    // SAFETY: a plain out-parameter that lives through the call.
    let ok = unsafe { SHQueryUserNotificationState(&mut state) } == 0;
    ok && matches!(state, QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
}

/// Windows' Do not disturb / Focus assist.
///
/// UNDOCUMENTED: Windows has no public way to ask this. This reads the shell's "quiet hours
/// profile" state (WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED) through ntdll's
/// NtQueryWnfStateData, the way other apps do: 0 = off, anything else (1 = priority only,
/// 2 = alarms only, ...) = on. Microsoft can change it in any update, and on Windows 11 it must
/// be checked on the developer's PC (turn Do not disturb on, post something, no pop-up should
/// show; the bell still gets its kinds). If it fails or reads nothing, pop-ups show.
#[cfg(windows)]
fn windows_do_not_disturb() -> bool {
    use std::ffi::c_void;
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryWnfStateData(
            state_name: *const u64,
            type_id: *const c_void,
            explicit_scope: *const c_void,
            change_stamp: *mut u32,
            buffer: *mut c_void,
            buffer_size: *mut u32,
        ) -> i32;
    }
    const WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED: u64 = 0x0D83_063E_A3BF_1C75;
    let name = WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED;
    let mut stamp = 0u32;
    let mut profile = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: every pointer is to a local that lives through the call; the buffer is 4 bytes
    // and its size says so (a bigger state is refused, not written past).
    let status = unsafe {
        NtQueryWnfStateData(&name, std::ptr::null(), std::ptr::null(), &mut stamp, &mut profile as *mut u32 as *mut c_void, &mut size)
    };
    status == 0 && size >= 4 && profile != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::NoticePref;

    #[test]
    fn defaults_popup_everywhere_bell_for_what_waits() {
        let s = Settings::default();
        for k in KINDS {
            let p = pref(&s, k.id);
            assert!(p.popup, "{} pops up by default", k.id);
            // Questions, a new Pako waiting for a restart, and a video the button said was sent.
            let bell = matches!(k.id, "direct-request" | "direct-offer" | "needs-you" | "update-ready" | "video-failed");
            assert_eq!(p.bell, bell, "{} bell default", k.id);
        }
        // Unknown kinds are "other".
        assert_eq!(pref(&s, "something-new"), pref(&s, "other"));
    }

    #[test]
    fn changed_switches_merge_with_defaults() {
        let mut s = Settings::default();
        s.notices.insert("browser".into(), NoticePref { popup: Some(false), bell: None });
        s.notices.insert("needs-you".into(), NoticePref { popup: None, bell: Some(false) });
        s.notices.insert("upload-done".into(), NoticePref { popup: None, bell: Some(true) });
        assert_eq!(pref(&s, "browser"), Pref { popup: false, bell: false });
        assert_eq!(pref(&s, "needs-you"), Pref { popup: true, bell: false });
        assert_eq!(pref(&s, "upload-done"), Pref { popup: true, bell: true });
        assert_eq!(pref(&s, "direct-offer"), Pref { popup: true, bell: true });
    }

    #[test]
    fn download_finished_popup_is_the_old_setting() {
        let mut s = Settings { notify_done: false, ..Settings::default() };
        // Before New downloads is chosen, the old switch decides it (Off when it was off).
        s.notices.insert("download-done".into(), NoticePref { popup: Some(true), bell: Some(true) });
        assert_eq!(pref(&s, "download-done"), Pref { popup: false, bell: true });
        s.notify_done = true;
        assert!(pref(&s, "download-done").popup);
    }

    fn notice(at: u64) -> Notice {
        Notice { id: at.to_string(), kind: "other".into(), title: String::new(), body: String::new(), at, read: false, go: Go::none(), answered: None }
    }

    #[test]
    fn store_keeps_30_days_and_300_lines() {
        let now = 100 * 24 * 3600 * 1000;
        let day = 24 * 3600 * 1000;
        let mut list = vec![notice(now - 31 * day), notice(now - 30 * day), notice(now - day), notice(now)];
        prune(&mut list, now);
        assert_eq!(list.iter().map(|n| n.at).collect::<Vec<_>>(), vec![now - 30 * day, now - day, now]);

        let mut many: Vec<Notice> = (0..350).map(|i| notice(now - 350 + i)).collect();
        prune(&mut many, now);
        assert_eq!(many.len(), KEEP_MAX);
        // The newest stay.
        assert_eq!(many.first().unwrap().at, now - 300);
        assert_eq!(many.last().unwrap().at, now - 1);
    }

    #[test]
    fn saved_lines_load_with_missing_fields() {
        let n: Notice = serde_json::from_str(r#"{"id":"a","kind":"direct-offer","title":"t","body":"b","at":5}"#).unwrap();
        assert!(!n.read && n.go == Go::none() && n.answered.is_none());
        let again = serde_json::to_string(&Notice { go: Go { page: "direct".into(), offer: Some("o1".into()), ..Go::none() }, ..n }).unwrap();
        assert!(again.contains(r#""go":{"page":"direct","offer":"o1"}"#), "{again}");
    }

    #[test]
    fn engine_payload_round_trips() {
        let go = Go::item("downloads", "d1");
        let p = payload("download-done", "Download finished", "a.zip", &go, Pop::Force);
        assert_eq!(p["pop"], "force");
        assert_eq!(serde_json::from_value::<Go>(p["go"].clone()).unwrap(), go);
    }

    /// Direct's news pops up unless Pako's window is in front on that very page.
    #[test]
    fn direct_news_pops_up_unless_its_page_is_in_front() {
        assert_eq!(pop_for(true, "direct", "direct-received", "direct"), Pop::Skip, "you see it there");
        assert_eq!(pop_for(true, "downloads", "direct-received", "direct"), Pop::Pref, "on another page: a pop-up");
        assert_eq!(pop_for(true, "direct", "direct-sent", "devices"), Pop::Pref);
        assert_eq!(pop_for(false, "direct", "direct-received", "direct"), Pop::Pref, "window behind another app");
        assert_eq!(pop_for(true, "", "direct-received", ""), Pop::Pref, "no page told yet / news without a page");
        // Questions the window asks on every page: never a pop-up as well while it's in front.
        assert_eq!(pop_for(true, "downloads", "direct-offer", "direct"), Pop::Skip, "offer asked in the window");
        assert_eq!(pop_for(true, "downloads", "direct-request", "direct"), Pop::Skip, "request asked in the window");
        assert_eq!(pop_for(false, "downloads", "direct-offer", "direct"), Pop::Pref, "window behind: a pop-up");
        assert_eq!(pop_for(true, "downloads", "direct-request", "devices"), Pop::Pref, "pairing isn't asked in the window");
        // "Files delivered" brings no sound of its own: the sent sound plays with it.
        assert_eq!(sound_of_notice("direct-sent"), None);
    }

    /// The window's table (src/lib/notices.ts) has the same kinds, row for row: the same id,
    /// name and bell default on one line, and no other rows.
    #[test]
    fn window_table_matches() {
        let ts = include_str!("../../src/lib/notices.ts");
        let rows: Vec<&str> = ts.lines().map(str::trim).filter(|l| l.starts_with("{ id: \"")).collect();
        assert_eq!(rows.len(), KINDS.len(), "notices.ts has {} kinds, notices.rs {}", rows.len(), KINDS.len());
        for Kind { id, label, bell } in KINDS {
            let start = format!("{{ id: \"{id}\", label: \"{label}\", ");
            let row = rows.iter().find(|l| l.starts_with(&start)).unwrap_or_else(|| panic!("{id} / {label} missing in notices.ts"));
            assert!(row.ends_with(&format!("bell: {bell} }},")), "{id}: bell default differs in notices.ts: {row}");
        }
    }

    #[test]
    fn prune_survives_a_far_future_time() {
        let mut list = vec![notice(u64::MAX)];
        prune(&mut list, 1_000);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn unreadable_history_is_kept_aside() {
        let dir = std::env::temp_dir().join(format!("pako-notices-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("notices.json");
        assert!(load_from(&path).is_empty(), "no file: empty");
        std::fs::write(&path, b"[{\"id\":").unwrap();
        assert!(load_from(&path).is_empty());
        assert!(!path.exists(), "the bad file is moved away");
        assert_eq!(std::fs::read(dir.join("notices.json.bad")).unwrap(), b"[{\"id\":");
        std::fs::write(&path, br#"[{"id":"a","kind":"other","title":"t","body":"b","at":5}]"#).unwrap();
        assert_eq!(load_from(&path).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_download_finished_popup_while_the_small_window_shows() {
        for pop in [Pop::Pref, Pop::Force] {
            assert!(popup_shows("download-done", pop, true, false, false));
            // Its own "Notify when done" (Force) included.
            assert!(!popup_shows("download-done", pop, true, false, true), "{pop:?}");
        }
        // Other kinds still pop up with the small window open.
        for kind in ["download-failed", "upload-done", "browser", "needs-you"] {
            assert!(popup_shows(kind, Pop::Pref, true, false, true), "{kind}");
        }
        // The rest as before: the switch, Force, Skip, quiet.
        assert!(!popup_shows("download-done", Pop::Pref, false, false, false));
        assert!(popup_shows("download-done", Pop::Force, false, false, false));
        assert!(!popup_shows("download-failed", Pop::Skip, true, false, false));
        assert!(!popup_shows("download-failed", Pop::Force, true, true, false));
    }

    #[test]
    fn quiet_follows_windows_only_when_asked() {
        assert!(quiet_when(true, false, || false));
        assert!(quiet_when(false, true, || true));
        assert!(!quiet_when(false, false, || panic!("Windows isn't asked when not followed")));
        assert!(!quiet_when(false, true, || false));
    }

    #[test]
    fn chime_only_for_notify_when_done_and_not_when_quiet() {
        let now = 1_000_000;
        assert!(chime_plays(true, true, false, None, now));
        // Not for downloads without "Notify when done", nor with the sound off.
        assert!(!chime_plays(false, true, false, None, now));
        assert!(!chime_plays(true, false, false, None, now));
        // Do not disturb, and Windows saying not now (when followed): the same check as pop-ups.
        assert!(!chime_plays(true, true, quiet_when(true, true, || false), None, now));
        assert!(!chime_plays(true, true, quiet_when(false, true, || true), None, now));
        assert!(chime_plays(true, true, quiet_when(false, false, || true), None, now));
        // The small window open: no pop-up, the chime still plays.
        assert!(!popup_shows("download-done", Pop::Force, true, false, true));
        assert!(chime_plays(true, true, false, None, now));
    }

    #[test]
    fn two_finishing_within_2_seconds_chime_once() {
        let first = 1_000_000;
        assert!(!chime_plays(true, true, false, Some(first), first + 500));
        assert!(!chime_plays(true, true, false, Some(first), first + 1_999));
        assert!(chime_plays(true, true, false, Some(first), first + 2_000));
        // A clock set back never silences it.
        assert!(chime_plays(true, true, false, Some(first), first - 10_000));
    }

    #[test]
    fn each_sound_has_its_own_switch_and_its_own_2_seconds() {
        let now = 1_000_000;
        let on = Settings::default();
        assert!(on.play_done_sound && on.play_sent_sound, "both on at first");
        let mut last = [None, None];
        assert!(sound_now(Sound::Arrived, Sound::Arrived.on(&on), false, &mut last, now));
        // A send delivered right after a file arrived still sounds; each again within 2 s doesn't.
        assert!(sound_now(Sound::Sent, Sound::Sent.on(&on), false, &mut last, now + 100));
        assert!(!sound_now(Sound::Arrived, Sound::Arrived.on(&on), false, &mut last, now + 1_000));
        assert!(!sound_now(Sound::Sent, Sound::Sent.on(&on), false, &mut last, now + 1_500));
        assert!(sound_now(Sound::Arrived, Sound::Arrived.on(&on), false, &mut last, now + 2_000));
        assert!(sound_now(Sound::Sent, Sound::Sent.on(&on), false, &mut last, now + 2_100));
        // One switch off: only that sound is silent.
        let no_sent = Settings { play_sent_sound: false, ..Settings::default() };
        let mut last = [None, None];
        assert!(!sound_now(Sound::Sent, Sound::Sent.on(&no_sent), false, &mut last, now));
        assert_eq!(last, [None, None], "a sound that didn't play doesn't hold the next one back");
        assert!(sound_now(Sound::Arrived, Sound::Arrived.on(&no_sent), false, &mut last, now));
        let no_arrived = Settings { play_done_sound: false, ..Settings::default() };
        let mut last = [None, None];
        assert!(!sound_now(Sound::Arrived, Sound::Arrived.on(&no_arrived), false, &mut last, now));
        assert!(sound_now(Sound::Sent, Sound::Sent.on(&no_arrived), false, &mut last, now));
        // Do not disturb, or Windows saying not now (when followed): neither plays.
        let mut last = [None, None];
        for which in [Sound::Arrived, Sound::Sent] {
            assert!(!sound_now(which, which.on(&on), quiet_when(true, true, || false), &mut last, now));
            assert!(!sound_now(which, which.on(&on), quiet_when(false, true, || true), &mut last, now));
            assert!(sound_now(which, which.on(&on), quiet_when(false, false, || true), &mut last, now));
        }
    }

    /// Windows plays one sound at a time: a sent sound right after an arrived one (or the other
    /// way round) waits for it to end instead of cutting it off; apart, neither waits.
    #[test]
    fn a_sound_never_cuts_the_other_one_off() {
        let (arrived, sent) = (length_ms(Sound::Arrived.wav()), length_ms(Sound::Sent.wav()));
        assert!((400..=440).contains(&arrived), "{arrived} ms");
        assert!((540..=560).contains(&sent), "{sent} ms");
        let now = 1_000_000;
        let mut until = 0;
        assert_eq!(take_turn(&mut until, now, arrived), 0);
        // 100 ms later: the arrived chime still plays, so the sent one starts after it.
        let wait = take_turn(&mut until, now + 100, sent);
        assert_eq!(wait, arrived - 100 + SOUND_GAP_MS);
        assert_eq!(until, now + arrived + SOUND_GAP_MS + sent);
        // A third one waits for both.
        assert_eq!(take_turn(&mut until, now + 200, arrived), arrived + SOUND_GAP_MS + sent + SOUND_GAP_MS - 200);
        // Long after: at once.
        let later = until + 5_000;
        assert_eq!(take_turn(&mut until, later, sent), 0);
        // A clock set back never holds a sound for long.
        let mut until = now + 3_600_000;
        assert_eq!(take_turn(&mut until, now, sent), MAX_SOUND_WAIT_MS);
        assert_eq!(length_ms(b"RIFF"), 0, "no header: no wait");
    }

    /// Direct and My devices: an offer waiting for your answer and files received chime
    /// (arrived); nothing else does. A send you had to accept chimes twice (it needed you, then
    /// it's here), one sent without asking only once, when it has arrived.
    #[test]
    fn direct_notices_chime_once_unless_you_had_to_answer() {
        assert_eq!(sound_of_notice("direct-offer"), Some(Sound::Arrived));
        assert_eq!(sound_of_notice("direct-received"), Some(Sound::Arrived));
        for kind in ["direct-incoming", "direct-request", "download-done", "upload-done", "needs-you", "other"] {
            assert_eq!(sound_of_notice(kind), None, "{kind}");
        }
        let chimes = |kinds: &[&str]| kinds.iter().filter(|k| sound_of_notice(k).is_some()).count();
        // Sent without asking (a contact who can, or one of My devices).
        assert_eq!(chimes(&["direct-incoming", "direct-received"]), 1);
        // Asked: the offer, then (accepted) the files.
        assert_eq!(chimes(&["direct-offer", "direct-received"]), 2);
        // Declined or given up: the offer's chime only.
        assert_eq!(chimes(&["direct-offer"]), 1);
    }

    /// The sent sound: 16-bit mono at 22.05 kHz, about 0.55 s, not the arrived one.
    #[test]
    fn sent_sound_is_a_small_wav() {
        let w = SENT_SOUND;
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        let u16_at = |i: usize| u16::from_le_bytes([w[i], w[i + 1]]);
        let u32_at = |i: usize| u32::from_le_bytes([w[i], w[i + 1], w[i + 2], w[i + 3]]);
        assert_eq!(u16_at(20), 1, "PCM");
        assert_eq!(u16_at(22), 1, "mono");
        assert_eq!(u32_at(24), 22_050);
        assert_eq!(u16_at(34), 16);
        assert_eq!(&w[36..40], b"data");
        let secs = u32_at(40) as f64 / (22_050.0 * 2.0);
        assert!((0.5..=0.6).contains(&secs), "{secs} s");
        assert!(w.len() < 30_000);
        assert_ne!(Sound::Sent.wav(), Sound::Arrived.wav());
    }

    /// The chime is a small 16-bit mono WAV at 22.05 kHz, about 0.42 s.
    #[test]
    fn done_sound_is_a_small_wav() {
        let w = DONE_SOUND;
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        let u16_at = |i: usize| u16::from_le_bytes([w[i], w[i + 1]]);
        let u32_at = |i: usize| u32::from_le_bytes([w[i], w[i + 1], w[i + 2], w[i + 3]]);
        assert_eq!(u16_at(20), 1, "PCM");
        assert_eq!(u16_at(22), 1, "mono");
        assert_eq!(u32_at(24), 22_050);
        assert_eq!(u16_at(34), 16);
        assert_eq!(&w[36..40], b"data");
        let secs = u32_at(40) as f64 / (22_050.0 * 2.0);
        assert!((0.3..=0.5).contains(&secs), "{secs} s");
        assert!(w.len() < 20_000);
    }

    #[test]
    fn questions_are_about_their_link_offer_or_person() {
        assert_eq!(about("needs-you", &Go::item("found", "f1")), Some("f1"));
        let offer = Go { page: "direct".into(), id: Some("peer".into()), offer: Some("o1".into()), ..Go::none() };
        assert_eq!(about("direct-offer", &offer), Some("o1"));
        assert_eq!(about("direct-request", &Go { page: "direct".into(), peer: Some("p1".into()), ..Go::none() }), Some("p1"));
        assert_eq!(about("download-done", &Go::item("downloads", "d1")), None);
    }

    /// "Chime when a download finishes": Pop-up and Important / Important only / Never; Off
    /// never chimes.
    #[test]
    fn which_finished_downloads_chime() {
        use DoneChime::*;
        for (alert, chime, plays) in [
            (Alert::Off, All, false),
            (Alert::Pop, All, true),
            (Alert::Important, All, true),
            (Alert::Off, Important, false),
            (Alert::Pop, Important, false),
            (Alert::Important, Important, true),
            (Alert::Pop, Never, false),
            (Alert::Important, Never, false),
        ] {
            assert_eq!(done_chimes(alert, chime), plays, "{alert:?} {chime:?}");
        }
    }

    /// The big card: never in the corner, waits only for something full screen (Do not disturb
    /// doesn't hold it back), and travels from the engine as "important".
    #[test]
    fn the_important_card_waits_only_for_full_screen() {
        assert!(!popup_shows("download-done", Pop::Important, true, false, false), "not a corner card");
        assert!(important_waits(true));
        assert!(!important_waits(false));
        let p = payload("download-done", "Download finished", "a.zip", &Go::item("downloads", "d1"), Pop::Important);
        assert_eq!(p["pop"], "important");
    }

    /// Download finished's pop-up "switch" is New downloads (for what has no download of its own).
    #[test]
    fn download_finished_follows_new_downloads() {
        let mut s = Settings { done_alert: Some(Alert::Off), ..Settings::default() };
        assert!(!pref(&s, "download-done").popup);
        s.done_alert = Some(Alert::Important);
        assert!(pref(&s, "download-done").popup);
    }

    /// Important cards wait 20 at most: older ones leave for the bell (when they may go there
    /// and aren't there already).
    #[test]
    fn important_cards_wait_twenty_at_most() {
        let card = |i: u32| Popup { id: format!("c{i}"), kind: "download-done".into(), title: "t".into(), body: "b".into(), go: Go::none(), stays: true, many: None };
        UNSHOWN.lock().unwrap().clear();
        IMPORTANT.lock().unwrap().clear();
        for i in 0..20 {
            assert!(hold_important(card(i), false, true).is_empty());
        }
        let out = hold_important(card(20), false, true);
        assert_eq!(out.iter().map(|(c, ..)| c.id.as_str()).collect::<Vec<_>>(), ["c0"], "the oldest, to the bell");
        assert_eq!(hold_important(card(21), true, false).len(), 1, "c1 leaves for the bell");
        // Those already in the bell (or never allowed there, a private window's) don't go again.
        for i in 22..60 {
            hold_important(card(i), i % 2 == 0, i % 2 == 0);
        }
        assert!(hold_important(card(60), true, true).is_empty(), "c40: in the bell already");
        assert!(hold_important(card(61), true, true).is_empty(), "c41: a private window's");
        assert_eq!(UNSHOWN.lock().unwrap().len(), 20);
        assert!(!IMPORTANT.lock().unwrap().iter().any(|c| c.id == "c0" || c.id == "c1"));
        UNSHOWN.lock().unwrap().clear();
        IMPORTANT.lock().unwrap().clear();
    }
}
