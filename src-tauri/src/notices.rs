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

use crate::model::Settings;

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
    Kind { id: "video-failed", label: "Couldn't get a video", bell: false },
    Kind { id: "upload-done", label: "Upload finished", bell: false },
    Kind { id: "upload-failed", label: "Couldn't upload", bell: false },
    Kind { id: "upload-deleted", label: "Upload deleted on its date", bell: false },
    Kind { id: "direct-request", label: "Someone wants to connect", bell: true },
    Kind { id: "direct-offer", label: "Files waiting for your answer", bell: true },
    Kind { id: "direct-incoming", label: "Files coming in", bell: false },
    Kind { id: "direct-received", label: "Files received", bell: false },
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
/// "Download finished › Pop-up" is the older "Notify when a download finishes" setting.
pub fn pref(s: &Settings, kind: &str) -> Pref {
    let k = kind_of(kind);
    let changed = s.notices.get(k.id).copied().unwrap_or_default();
    let popup = if k.id == "download-done" { s.notify_done } else { changed.popup.unwrap_or(true) };
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
}

/// Whether a pop-up shows: as the kind's switch says, always (a download's own "Notify when
/// done"), or never (Direct while Pako's window is in front: its question is on screen).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pop {
    Pref,
    Force,
    Skip,
}

impl Pop {
    fn name(self) -> &'static str {
        match self {
            Pop::Pref => "pref",
            Pop::Force => "force",
            Pop::Skip => "skip",
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
        _ => Pop::Pref,
    };
    let kind = text("kind");
    let bell = p.get("noBell").and_then(|v| v.as_bool()) != Some(true);
    post_as(app, if kind.is_empty() { "other" } else { &kind }, &text("title"), &text("body"), go, pop, bell);
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
        Pop::Skip => false,
        Pop::Pref => kind_popup,
    };
    wanted && !quiet && !(kind == "download-done" && small_window_open)
}

/// Tells you something: into the bell if its kind goes there, and as a pop-up if its kind
/// (or `pop`) says so, unless Do not disturb is on or Windows says not now (see
/// `popup_shows`). A question about the same thing as an earlier one (the same link asking
/// again) takes its place.
pub fn post(app: &AppHandle, kind: &str, title: &str, body: &str, go: Go, pop: Pop) {
    post_as(app, kind, title, body, go, pop, true);
}

/// `bell` = false: never a line in the bell (a private window's download).
fn post_as(app: &AppHandle, kind: &str, title: &str, body: &str, go: Go, pop: Pop, bell: bool) {
    let kind = kind_of(kind).id;
    let (p, quiet) = live_pref(kind);
    let id = uuid::Uuid::new_v4().simple().to_string();
    let key = about(kind, &go).map(str::to_string);
    let same = |k: &str, g: &Go| key.is_some() && k == kind && about(k, g) == key.as_deref();
    if p.bell && bell {
        let n = Notice { id: id.clone(), kind: kind.into(), title: title.into(), body: body.into(), at: now_ms(), read: false, go: go.clone(), answered: None };
        change(app, |list| {
            list.retain(|o| !same(&o.kind, &o.go));
            list.push(n);
            prune(list, now_ms());
            true
        });
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
        WAITING.lock().unwrap().push(Popup { id, kind: kind.into(), title: title.into(), body: body.into(), go, stays });
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

// ---------- the finished chime ----------

/// Pako's chime: two soft rising notes, about 0.42 s (made by `scripts/make-done-sound.mjs`).
static DONE_SOUND: &[u8] = include_bytes!("../sounds/done.wav");

/// Several downloads finishing this close together chime once.
const CHIME_GAP_MS: u64 = 2000;

/// When the chime last played (ms since 1970).
static LAST_CHIME: Mutex<Option<u64>> = Mutex::new(None);

/// Whether the chime plays for a finished download: only one with its own "Notify when done"
/// (`notify`), with the sound on, not at a quiet time (as for pop-ups: Do not disturb, or
/// Windows says not now), and not again within 2 seconds of the last one. The small window
/// being open doesn't matter (it only stops the pop-up).
fn chime_plays(notify: bool, sound_on: bool, quiet: bool, last: Option<u64>, now: u64) -> bool {
    notify && sound_on && !quiet && last.is_none_or(|l| now < l || now - l >= CHIME_GAP_MS)
}

/// A download (or a whole folder or set) finished; `notify`: it asked "Notify when done".
/// Plays the chime if `chime_plays` says so. Never waits for the sound.
pub fn chime_if_asked(notify: bool) {
    if !notify {
        return;
    }
    let s = LIVE.lock().unwrap().clone().unwrap_or_default();
    // Windows is only asked when the sound is on.
    let quiet = s.play_done_sound && quiet_for(&s);
    let now = now_ms();
    {
        let mut last = LAST_CHIME.lock().unwrap();
        if !chime_plays(notify, s.play_done_sound, quiet, *last, now) {
            return;
        }
        *last = Some(now);
    }
    play(DONE_SOUND);
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

#[tauri::command]
pub fn notices_list() -> Vec<Notice> {
    with_store(|list| list.iter().rev().cloned().collect())
}

/// Marks these as read (or all of them, with no ids).
#[tauri::command]
pub fn notices_read(app: AppHandle, ids: Option<Vec<String>>) {
    mark_read(&app, ids);
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
pub fn notices_clear(app: AppHandle) {
    change(&app, |list| {
        let any = !list.is_empty();
        list.clear();
        any
    });
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
            // Questions, and a new Pako waiting for a restart.
            let bell = matches!(k.id, "direct-request" | "direct-offer" | "needs-you" | "update-ready");
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
        // Only notify_done decides it (Settings shows that one switch).
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
}
