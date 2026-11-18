// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
pub mod cli;
mod components;
mod drive;
mod engine;
mod errorlog;
mod fetch;
mod gates;
mod google;
mod limiter;
mod model;
mod names;
mod numbers;
mod net;
mod paths;
mod secure;
mod preview;
mod split;
mod stats;
mod system;
mod tray;
mod updates;
pub mod extension;
mod seen;
mod video;
mod prepared;
mod hls;
mod stream;
mod send;
mod filedrop;
mod ask;
mod direct;
mod notices;
mod theme;
mod terms;
mod consent;
mod via;
pub mod release;
pub mod install;

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use engine::{AddResult, Engine};
use model::{Download, FoundItem, Priority, Settings};

type Eng<'a> = State<'a, Arc<Engine>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    downloads: Vec<Download>,
    /// Links waiting to be picked (the Found tab).
    found: Vec<FoundItem>,
    /// Split archives among the Found links and among the downloads.
    found_sets: Vec<split::SetView>,
    download_sets: Vec<split::SetView>,
    /// The same file on several hosts among the Found links.
    found_mirrors: Vec<engine::MirrorView>,
    settings: Settings,
    data_dir: String,
    /// 7-Zip is installed (extracting split archives works).
    seven_zip: bool,
    /// ffmpeg is installed (MKV/AVI previews).
    ffmpeg: bool,
    /// Pako's own version.
    version: String,
    /// Installed with Pako Setup (false = the portable Pako.exe).
    installed: bool,
    /// The terms of use wait to be accepted: the window shows only them (see consent.rs).
    terms_needed: bool,
}

// ---------- Send (files with a link) ----------

type Snd<'a> = State<'a, Arc<send::Sender>>;
type Dir<'a> = State<'a, direct::Direct>;

// ---------- Direct (Pako to Pako) ----------

#[tauri::command]
fn direct_state(d: Dir<'_>) -> direct::View {
    direct::view(&d)
}

/// The Direct page is open: start Direct if it isn't running yet (and from then on with Pako).
#[tauri::command]
fn direct_wake(d: Dir<'_>) {
    d.wake();
}

/// Your name, as contacts see it when you connect.
#[tauri::command]
fn direct_set_name(d: Dir<'_>, name: String) -> Result<(), String> {
    direct::save_name(&name);
    d.node()?.set_my_name(&name);
    Ok(())
}

/// Contacts see your name: without one, everyone would be "someone" to the other side.
const NAME_FIRST: &str = "Add your name first: it's what your contacts see.";

#[tauri::command]
fn direct_add_contact(d: Dir<'_>, id: String, name: String) -> Result<(), String> {
    let node = d.node()?;
    if !node.has_name() {
        return Err(NAME_FIRST.into());
    }
    node.add_contact(&id, &name)
}

/// Rename, "can send to me without asking", or their save folder (`folder` "" = the default).
#[tauri::command]
fn direct_edit_contact(d: Dir<'_>, id: String, name: Option<String>, auto_accept: Option<bool>, folder: Option<String>) -> Result<(), String> {
    d.node()?.edit_contact(&id, name, auto_accept, folder.map(Some))
}

/// Favourite on or off (only on this PC).
#[tauri::command]
fn direct_set_favourite(d: Dir<'_>, id: String, on: bool) -> Result<(), String> {
    d.node()?.set_favourite(&id, on)
}

/// Mute or unmute a contact: no pop-up, chime or bell line from them (not for My devices).
#[tauri::command]
fn direct_set_muted(d: Dir<'_>, id: String, on: bool) -> Result<(), String> {
    d.node()?.set_muted(&id, on)
}

/// "Check this contact": "often" (with `span` "hour", "today" or "week"), "auto" or "ask".
#[tauri::command]
fn direct_set_check(d: Dir<'_>, id: String, check: String, span: Option<String>) -> Result<(), String> {
    let (check, span) = direct::parse_check(&check, span.as_deref())?;
    d.node()?.set_check(&id, check, span)
}

/// A contact's tags (only on this PC; new names are added to your tags).
#[tauri::command]
fn direct_set_tags(d: Dir<'_>, id: String, tags: Vec<String>) -> Result<(), String> {
    d.node()?.set_tags(&id, tags)
}

#[tauri::command]
fn direct_add_tag(d: Dir<'_>, name: String) -> Result<String, String> {
    d.node()?.add_tag(&name)
}

#[tauri::command]
fn direct_rename_tag(d: Dir<'_>, old: String, new: String) -> Result<(), String> {
    d.node()?.rename_tag(&old, &new)
}

#[tauri::command]
fn direct_delete_tag(d: Dir<'_>, name: String) -> Result<(), String> {
    d.node()?.delete_tag(&name);
    Ok(())
}

#[tauri::command]
fn direct_remove_contact(d: Dir<'_>, id: String) -> Result<(), String> {
    d.node()?.remove_contact(&id);
    Ok(())
}

#[tauri::command]
fn direct_connect(d: Dir<'_>, id: String) -> Result<(), String> {
    d.node()?.reach(&id);
    Ok(())
}

/// The conversation shown on the Direct page (None: none shown): kept connected, and called
/// at once if it isn't.
#[tauri::command]
fn direct_shown(d: Dir<'_>, id: Option<String>) -> Result<(), String> {
    d.node()?.set_open(id.as_deref());
    Ok(())
}

/// A connection request is answered only in Pako's own window, where its safety code is
/// compared (never from a pop-up or another small window). Its pop-up and bell line follow
/// (the node settles them, see notices.rs `resolve`).
#[tauri::command]
fn direct_answer_request(window: tauri::Window, d: Dir<'_>, peer: String, accept: bool) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Answer this in Pako's window.".into());
    }
    let node = d.node()?;
    if accept && !node.has_name() {
        return Err(NAME_FIRST.into());
    }
    node.answer_request(&peer, accept);
    Ok(())
}

/// Making the files of an accepted offer takes a moment: off the window's thread. Its pop-up
/// and bell line follow once it is really accepted or declined (the node settles them).
#[tauri::command]
async fn direct_answer_offer(d: Dir<'_>, id: String, accept: bool, folder: Option<String>) -> Result<(), String> {
    let node = d.node()?;
    tauri::async_runtime::spawn_blocking(move || node.answer_offer(&id, accept, folder)).await.map_err(|e| e.to_string())??;
    Ok(())
}

#[tauri::command]
async fn direct_send(d: Dir<'_>, contact: String, paths: Vec<String>, compress: bool, name: Option<String>) -> Result<String, String> {
    let node = d.node()?;
    node.send(&contact, paths.into_iter().map(std::path::PathBuf::from).collect(), compress, name).await
}

#[tauri::command]
fn direct_cancel(d: Dir<'_>, id: String) -> Result<(), String> {
    d.node()?.cancel(&id);
    Ok(())
}

#[tauri::command]
fn direct_clear_history(d: Dir<'_>, contact: Option<String>) -> Result<(), String> {
    d.node()?.clear_history(contact.as_deref());
    Ok(())
}

/// Opens what was received (or shows it in its folder). A program or script is never started
/// from here: it's shown in its folder instead (true = that happened).
#[tauri::command]
fn direct_open(app: tauri::AppHandle, d: Dir<'_>, id: String, reveal: bool) -> Result<bool, String> {
    let item = d.node()?.history_item(&id).ok_or("Not in the history any more.")?;
    let path = std::path::PathBuf::from(item.saved_to.ok_or("Nothing was saved for this one.")?);
    if !path.exists() {
        return Err("It's not there any more (moved or deleted).".into());
    }
    if path.is_dir() {
        app.opener().open_path(path.display().to_string(), None::<&str>).map_err(|e| e.to_string())?;
        return Ok(false);
    }
    let runs = direct::runs_code(&path);
    if reveal || runs {
        app.opener().reveal_item_in_dir(&path).map_err(|e| e.to_string())?;
    } else {
        app.opener().open_path(path.display().to_string(), None::<&str>).map_err(|e| e.to_string())?;
    }
    Ok(runs && !reveal)
}

#[tauri::command]
fn send_state(sender: Snd<'_>) -> send::SendState {
    sender.state()
}

/// Starts sending these files / folders. Looking through a big folder can take a moment: off
/// the window's thread.
#[tauri::command]
/// `part_mb`: "ZIP in parts"' part size for this send (None = the setting).
async fn send_start(sender: Snd<'_>, paths: Vec<String>, mode: Option<String>, name: Option<String>, delete_days: Option<u32>, host: Option<String>, part_mb: Option<u32>) -> Result<String, String> {
    let s = sender.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.start(paths, mode, name, delete_days, host, part_mb)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

/// Sizes of the files and folders in the Send basket (folders: all their files), off the window's
/// thread (a big folder takes a moment).
#[tauri::command]
async fn send_sizes(paths: Vec<String>) -> Result<Vec<send::PathSize>, String> {
    tauri::async_runtime::spawn_blocking(move || send::path_sizes(&paths)).await.map_err(|e| e.to_string())
}

/// "Try again" on a several-files send: only the files that didn't upload. False = nothing to
/// resume (the window then starts the whole send again).
#[tauri::command]
fn send_resume(sender: Snd<'_>, id: String) -> bool {
    sender.inner().resume(&id)
}

#[tauri::command]
fn send_cancel(sender: Snd<'_>, id: String) {
    sender.cancel(&id);
}

#[tauri::command]
fn send_dismiss(sender: Snd<'_>, id: String) {
    sender.dismiss(&id);
}

/// Takes a sent item off the list; `online` also deletes it from Gofile.
#[tauri::command]
async fn sent_remove(sender: Snd<'_>, id: String, online: bool) -> Result<(), String> {
    sender.remove(&id, online).await.map_err(|e| e.to_string())
}

/// Changes when an upload gets deleted ("Delete after"); None = keep it.
#[tauri::command]
async fn sent_set_expiry(sender: Snd<'_>, id: String, at: Option<u64>) -> Result<(), String> {
    sender.set_expiry(&id, at).await.map_err(|e| e.to_string())
}

/// Your note on an upload.
#[tauri::command]
fn sent_set_note(sender: Snd<'_>, id: String, note: String) {
    sender.set_note(&id, &note);
}

/// An upload's link as a QR code (SVG), made on this PC.
#[tauri::command]
fn sent_qr(sender: Snd<'_>, id: String) -> Result<String, String> {
    sender.qr(&id).map_err(|e| e.to_string())
}

/// Asks Gofile, in the background, how often each sent item was downloaded. False = not now
/// (already asking, or asked a moment ago).
#[tauri::command]
fn sent_check(sender: Snd<'_>, button: bool) -> bool {
    sender.inner().check(button)
}

/// Your Google Drive for sending: connected or not, and its room.
#[tauri::command]
async fn send_drive_status(sender: Snd<'_>) -> Result<send::DriveSendStatus, String> {
    let s = sender.inner().clone();
    Ok(s.drive_status().await)
}

/// Connects your Google Drive for sending (Google's sign-in page opens in the browser). Once
/// connected, uploads go to Drive by default (the user can still pick Gofile in the basket).
#[tauri::command]
async fn send_drive_sign_in(app: tauri::AppHandle, sender: Snd<'_>, engine: Eng<'_>) -> Result<String, String> {
    let opener = app.clone();
    let s = sender.inner().clone();
    let email = s
        .send_google
        .sign_in(move |url| {
            let _ = opener.opener().open_url(url, None::<&str>);
        })
        .await
        .map_err(|e| e.to_string())?;
    let _ = engine.change_settings(|s| s.send_host = "drive".into());
    let _ = app.emit("send-changed", ());
    Ok(email)
}

/// Disconnects your Google Drive for sending (what was sent stays in your Drive). Uploads go back
/// to Gofile, which needs no account.
#[tauri::command]
async fn send_drive_sign_out(app: tauri::AppHandle, sender: Snd<'_>, engine: Eng<'_>) -> Result<(), String> {
    let s = sender.inner().clone();
    s.send_google.sign_out().await;
    let _ = engine.change_settings(|s| {
        if s.send_host == "drive" {
            s.send_host = "gofile".into();
        }
    });
    let _ = app.emit("send-changed", ());
    Ok(())
}

/// How much a send of these files would upload (checked against your Drive's room first).
#[tauri::command]
async fn send_measure(sender: Snd<'_>, paths: Vec<String>, mode: Option<String>) -> Result<u64, String> {
    let s = sender.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.measure(&paths, mode.as_deref()))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Opens a sent file's download page in the browser.
#[tauri::command]
fn sent_open(app: tauri::AppHandle, sender: Snd<'_>, id: String) -> Result<(), String> {
    let link = sender.link_of(&id).ok_or("Not on the list any more")?;
    if !link.starts_with("https://") {
        return Err("Not a web link".into());
    }
    app.opener().open_url(link, None::<&str>).map_err(|e| e.to_string())
}

/// Copies text (a link) to the clipboard.
#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    if system::set_clipboard_text(&text) { Ok(()) } else { Err("Couldn't copy: another app is using the clipboard. Try again.".into()) }
}

#[tauri::command]
async fn snapshot(window: tauri::WebviewWindow, engine: Eng<'_>) -> Result<Snapshot, String> {
    // Pako's window has loaded and reached the engine: a new version on trial is up.
    if window.label() == "main" {
        updates::app_ready();
    }
    Ok(Snapshot {
        downloads: engine.downloads(),
        found: engine.found(),
        found_sets: engine.found_sets(),
        download_sets: engine.download_sets(),
        found_mirrors: engine.found_mirrors(),
        settings: engine.settings(),
        data_dir: paths::data_dir().display().to_string(),
        seven_zip: components::seven_zip().is_some(),
        ffmpeg: components::ffmpeg().is_some(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        installed: paths::is_installed(),
        terms_needed: !consent::ok(),
    })
}

#[tauri::command]
async fn add_links(engine: Eng<'_>, text: String, dest: Option<String>) -> Result<AddResult, String> {
    // Adding opens and checks the links (network): not before the terms of use are accepted.
    consent::check()?;
    let dest = dest.filter(|d| !d.trim().is_empty()).map(PathBuf::from);
    Ok(engine.inner().add_links(&text, dest).await)
}

/// Pasted links go to the Found list first.
#[tauri::command]
async fn found_add(engine: Eng<'_>, text: String) -> Result<AddResult, String> {
    // Links dropped on the window while it shows the terms: nothing is opened or checked.
    consent::check()?;
    Ok(engine.inner().found_add(&text).await)
}

/// The picked Found links become downloads (`first` = Download first).
#[tauri::command]
async fn found_download(engine: Eng<'_>, ids: Vec<String>, first: bool, dest: Option<String>, notify: bool) -> Result<usize, String> {
    let dest = dest.filter(|d| !d.trim().is_empty()).map(PathBuf::from);
    // Off the window's thread: thousands of files or a slow drive must not freeze it.
    let e = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || e.found_download(&ids, first, dest, notify))
        .await
        .map_err(|e| e.to_string())
}

/// Details panel settings for Found items. `set_dest` with no `dest` = back to the tab's Save to.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn found_set(
    engine: Eng<'_>,
    ids: Vec<String>,
    set_dest: bool,
    dest: Option<String>,
    priority: Option<Priority>,
    notify: Option<bool>,
    extract: Option<bool>,
) {
    let dest = set_dest.then(|| dest.filter(|d| !d.trim().is_empty()).map(PathBuf::from));
    engine.found_set(&ids, dest, priority, notify, extract);
}

/// Video quality for Links rows (details panel): `quality` None = the default from Settings,
/// 0 = audio only (`audio` = "m4a" or "mp3").
#[tauri::command]
fn found_set_video(engine: Eng<'_>, ids: Vec<String>, quality: Option<u32>, audio: Option<String>) {
    engine.found_set_video(&ids, quality, audio);
}

/// Same file on several hosts: download these rows' hosts (the others become backups).
#[tauri::command]
fn found_pick_host(engine: Eng<'_>, ids: Vec<String>) {
    engine.found_pick_host(&ids);
}

/// Checks Links rows again (after the video tools were installed).
#[tauri::command]
fn found_recheck(engine: Eng<'_>, ids: Vec<String>) {
    engine.inner().found_recheck(&ids);
}

#[tauri::command]
fn set_extract(engine: Eng<'_>, ids: Vec<String>, on: bool) {
    engine.inner().set_extract(&ids, on);
}

#[tauri::command]
fn set_notify(engine: Eng<'_>, ids: Vec<String>, on: bool) {
    engine.set_notify(&ids, on);
}

#[tauri::command]
fn set_folder(engine: Eng<'_>, ids: Vec<String>, dir: String) -> Result<(), String> {
    engine.set_folder(&ids, PathBuf::from(dir)).map_err(|e| e.to_string())
}

/// "Show" on a Found row waiting for a check: brings that small window to the front.
#[tauri::command]
fn found_restore(engine: Eng<'_>, items: Vec<FoundItem>, at: usize) {
    engine.inner().found_restore(items, at);
}

#[tauri::command]
fn restore_downloads(engine: Eng<'_>, list: Vec<Download>) {
    engine.restore_downloads(list);
}

/// Drag and drop in the list: these downloads go just before `target` (none = at the end).
#[tauri::command]
fn move_before(engine: Eng<'_>, ids: Vec<String>, target: Option<String>) {
    engine.inner().move_before(&ids, target.as_deref());
}

/// "When all downloads finish": nothing / sleep / shutDown (this session only).
#[tauri::command]
fn set_when_done(engine: Eng<'_>, action: String) {
    engine.set_when_done(&action);
}

/// "Cancel" in the sleep / shut-down countdown.
#[tauri::command]
fn cancel_when_done(engine: Eng<'_>) {
    engine.cancel_when_done();
}

/// "Quit anyway" in the window's question.
#[tauri::command]
fn quit_now(app: tauri::AppHandle) {
    tray::remember_place(&app);
    app.exit(0);
}

/// Links whose file is already in the folder they would be saved to (for "already downloaded?").
#[tauri::command]
async fn found_existing(engine: Eng<'_>, ids: Vec<String>, dest: Option<String>) -> Result<Vec<String>, String> {
    let dest = dest.filter(|d| !d.trim().is_empty()).map(PathBuf::from);
    // A sleeping drive or a network folder can take seconds: never on the window's thread.
    let e = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || e.found_existing(&ids, dest)).await.map_err(|e| e.to_string())
}

/// Settings from the tray's compact list.
#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    tray::open_settings(&app);
}

#[tauri::command]
fn gate_focus(id: String) {
    gates::focus_waiting(&id);
}

#[tauri::command]
fn found_remove(engine: Eng<'_>, ids: Vec<String>) {
    engine.found_remove(&ids);
}

/// Latest versions found by the last "Check for new versions".
static LATEST: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, String>>> =
    std::sync::LazyLock::new(Default::default);

fn kind_key(k: components::Kind) -> String {
    format!("{k:?}")
}

/// For component downloads: give up when the connection stalls, never because a big file
/// takes long on a slow line (ffmpeg is ~200 MB).
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(20))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_default()
}

/// Settings › Features & updates: what's installed (and the latest versions, once checked).
#[tauri::command]
async fn components_status() -> Vec<components::Status> {
    tauri::async_runtime::spawn_blocking(|| {
        let latest = LATEST.lock().unwrap().clone();
        components::ALL
            .iter()
            .map(|&k| components::Status { kind: k, installed: components::installed_version(k), latest: latest.get(&kind_key(k)).cloned() })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// Settings › Features & updates › Check for new versions (asks GitHub for each tool's latest release).
#[tauri::command]
async fn components_check() -> Result<Vec<components::Status>, String> {
    consent::check()?;
    let http = http_client();
    let mut errors = Vec::new();
    for k in components::ALL {
        match components::latest_version(&http, k).await {
            Ok(v) => {
                LATEST.lock().unwrap().insert(kind_key(k), v);
            }
            Err(e) => errors.push(e.to_string()),
        }
    }
    if errors.len() == components::ALL.len() {
        return Err(format!("Couldn't check for new versions: {}", errors[0]));
    }
    Ok(components_status().await)
}

/// Removes a tool (Settings › Features & updates › Remove).
#[tauri::command]
async fn component_remove(kind: components::Kind) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || components::remove(kind)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

/// Removes a feature's tools together (all or none).
#[tauri::command]
async fn components_remove(kinds: Vec<components::Kind>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || components::remove_all(&kinds))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Pako's own update: where it stands.
#[tauri::command]
fn update_status() -> updates::View {
    updates::view()
}

/// "Check for updates" (`get` = also download it, ready for a restart).
#[tauri::command]
async fn update_check(get: bool) -> updates::View {
    // Not before the terms of use are accepted: what is known already, nothing asked.
    if !consent::ok() {
        return updates::view();
    }
    updates::mark_checked();
    updates::check(&http_client(), get).await
}

/// "Restart now": quits and starts the new version.
#[tauri::command]
fn update_restart(app: tauri::AppHandle) -> Result<(), String> {
    // From the pop-up or the bell with the window hidden in the tray: the new Pako stays there.
    let hidden = app.get_webview_window("main").map(|w| !w.is_visible().unwrap_or(true)).unwrap_or(true);
    updates::restart(hidden).map_err(|e| e.to_string())?;
    tray::remember_place(&app);
    app.exit(0);
    Ok(())
}

/// Installs or updates one tool; progress arrives as "component-progress" events.
#[tauri::command]
async fn component_install(app: tauri::AppHandle, kind: components::Kind) -> Result<String, String> {
    consent::check()?;
    let http = http_client();
    let emitter = app.clone();
    let progress = move |stage: &str, done: u64, total: Option<u64>| {
        let _ = emitter.emit("component-progress", serde_json::json!({ "kind": kind, "stage": stage, "done": done, "total": total }));
    };
    components::install(&http, kind, &progress).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewInfo {
    url: String,
    /// Converted by ffmpeg to `quality` lines (else the file plays as it is).
    converted: bool,
    quality: u32,
}

/// A player address for a video (a download or a Found link).
#[tauri::command]
fn preview_url(
    engine: Eng<'_>,
    preview: State<'_, Arc<preview::Preview>>,
    id: String,
    quality: Option<u32>,
) -> Result<PreviewInfo, String> {
    let (source, name) = engine.source_of(&id).ok_or("This item is gone.")?;
    if !preview::is_video(&name) {
        return Err("Only videos can be previewed.".into());
    }
    let quality = quality.unwrap_or(engine.settings().preview_quality).clamp(240, 720);
    // A video-site link: a short clip from the site (yt-dlp + ffmpeg), always 480p or lower.
    if matches!(source, model::Source::Video { .. }) {
        let url = preview.register_item(Some(&id), source, name, 480, true);
        return Ok(PreviewInfo { url, converted: true, quality: 480 });
    }
    if components::ffmpeg().is_some() {
        let url = preview.register_item(Some(&id), source, name, quality, true);
        return Ok(PreviewInfo { url, converted: true, quality });
    }
    if preview::plays_directly(&name) {
        let url = preview.register_item(Some(&id), source, name, quality, false);
        return Ok(PreviewInfo { url, converted: false, quality });
    }
    Err("Install ffmpeg (Settings › Updates) to preview this kind of video.".into())
}

/// A video-site preview: asks the site for a small stream first, so the window can say why it can't play.
#[tauri::command]
async fn preview_check(preview: State<'_, Arc<preview::Preview>>, url: String) -> Result<(), String> {
    preview.check(&url).await
}

/// Dashboard history.
#[tauri::command]
fn stats(engine: Eng<'_>) -> stats::StatsView {
    engine.stats()
}

/// The Dashboard's numbers for "today", "week", "month" or "all".
#[tauri::command]
fn numbers_view(period: String) -> numbers::NumbersView {
    numbers::view(&period)
}

/// Everything with one Direct contact (Direct's details panel).
#[tauri::command]
fn numbers_contact(id: String) -> numbers::ContactNumbers {
    numbers::contact(&id)
}

/// Saves every kept event as a CSV file; returns how many rows.
#[tauri::command]
fn numbers_export(path: String) -> Result<usize, String> {
    numbers::export_csv(std::path::Path::new(&path))
}

/// Forgets all statistics (the Dashboard's numbers and the speed history).
#[tauri::command]
fn numbers_reset(engine: Eng<'_>) -> Result<(), String> {
    numbers::reset()?;
    engine.reset_stats();
    Ok(())
}

/// Hide in the taskbar tray (Pako keeps downloading).
#[tauri::command]
fn hide_to_tray(app: tauri::AppHandle) {
    tray::hide_main(&app);
}

/// From the compact tray list: open the full window.
#[tauri::command]
fn show_main(app: tauri::AppHandle) {
    tray::show_main(&app);
}

#[tauri::command]
fn pause_all(engine: Eng<'_>) {
    engine.pause_all();
}

#[tauri::command]
fn resume_all(engine: Eng<'_>) {
    engine.inner().resume_all();
}

#[tauri::command]
fn pause(engine: Eng<'_>, ids: Vec<String>) {
    engine.pause(&ids);
}

#[tauri::command]
fn resume(engine: Eng<'_>, ids: Vec<String>) {
    engine.resume(&ids);
}

#[tauri::command]
async fn remove(engine: Eng<'_>, ids: Vec<String>, delete_files: bool) -> Result<(), String> {
    // Moving files to the Recycle Bin can take a while: not on the window's thread.
    let e = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        e.remove(&ids, delete_files);
        // The browser's cookies for these go now (not at the next clean-up).
        for id in &ids {
            extension::forget_headers(id);
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_completed(engine: Eng<'_>) {
    engine.clear_completed();
}

#[tauri::command]
fn download_first(engine: Eng<'_>, ids: Vec<String>) {
    engine.download_first(&ids);
}

#[tauri::command]
fn set_priority(engine: Eng<'_>, ids: Vec<String>, priority: Priority) {
    engine.set_priority(&ids, priority);
}

#[tauri::command]
fn start_over(engine: Eng<'_>, ids: Vec<String>) {
    engine.start_over(&ids);
}

#[tauri::command]
fn rename(engine: Eng<'_>, id: String, name: String) -> Result<String, String> {
    engine.rename(&id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn move_in_queue(engine: Eng<'_>, ids: Vec<String>, top: bool) {
    engine.move_in_queue(&ids, top);
}

#[tauri::command]
fn parts(engine: Eng<'_>, id: String) -> Vec<engine::PartView> {
    engine.parts(&id)
}

/// Opens the download's original link in the browser.
#[tauri::command]
fn open_link(app: tauri::AppHandle, engine: Eng<'_>, id: String) -> Result<(), String> {
    let d = engine.find(&id).ok_or("Download not found")?;
    if !(d.link.starts_with("http://") || d.link.starts_with("https://")) {
        return Err("Not a web link".into());
    }
    app.opener().open_url(d.link, None::<&str>).map_err(|e| e.to_string())
}

/// The Drive page to open for an item: the folder itself for a folder Google wouldn't list,
/// else the file's own page (also for a file that came from a folder link).
fn drive_page(link: &str, source: &model::Source) -> String {
    match source {
        model::Source::GDrive { file_id, resource_key, .. } => {
            let is_the_folder = drive::parse_link(link).is_some_and(|dl| dl.kind == drive::LinkKind::Folder && dl.id == *file_id);
            if is_the_folder { link.to_string() } else { drive::view_url(file_id, resource_key.as_deref()) }
        }
        _ => link.to_string(),
    }
}

/// A Drive file's own page in the browser (download it there, or make a copy in your Drive):
/// a download's file, or a Links item's file or folder.
#[tauri::command]
fn open_in_drive(app: tauri::AppHandle, engine: Eng<'_>, id: String) -> Result<(), String> {
    let (link, source) = match engine.find(&id) {
        Some(d) => (d.link, d.source),
        None => {
            let f = engine.found().into_iter().find(|f| f.id == id).ok_or("Not found")?;
            (f.template.link, f.template.source)
        }
    };
    let url = drive_page(&link, &source);
    if !(url.starts_with("https://drive.google.com/") || url.starts_with("https://docs.google.com/")) {
        return Err("Not a Google Drive link".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_settings(app: tauri::AppHandle, engine: Eng<'_>, settings: Settings) -> Result<(), String> {
    // Checks the download folder (a sleeping drive can take seconds): not on the window's thread.
    let e = engine.inner().clone();
    let start = (settings.start_with_windows, settings.start_in_tray);
    let browsers = settings.browser_connect;
    tauri::async_runtime::spawn_blocking(move || e.set_settings(settings))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    // Do not disturb turned on (or off): pop-ups already on screen follow at once.
    let _ = app.emit_to("popup", "popup-quiet-check", ());
    // Start with Windows lives in Windows' own start-up list, not in Pako's files; the
    // extension's line to Pako in the browsers' own lists. Both are tried; each problem is said.
    let errors: Vec<String> = [system::set_autostart(start.0, start.1), extension::set_registered(browsers)]
        .into_iter()
        .filter_map(Result::err)
        .collect();
    if errors.is_empty() { Ok(()) } else { Err(errors.join(" ")) }
}

/// What first-start setup saves (missing = not asked this time).
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SetupChoices {
    auto_update: Option<bool>,
    minimize_to_tray: Option<bool>,
    close_to_tray: Option<bool>,
    start_with_windows: Option<bool>,
    start_in_tray: Option<bool>,
    /// "Add Pako to my browsers": the extension's line to Pako.
    browser_connect: Option<bool>,
}

/// The terms of use, accepted (Agree and continue): their date is saved at once, and
/// everything that waited for them starts now, no restart (see consent.rs). Settings that can't
/// be saved (unreadable, or the disk refusing) still let Pako run this time; the error says so.
#[tauri::command]
async fn accept_terms(engine: Eng<'_>) -> Result<(), String> {
    let date = terms::version().ok_or("Pako's terms of use have no date.")?;
    let saved = engine.change_settings(|s| s.terms_accepted = Some(date));
    // Off the window's thread: what starts may open windows (the Download with Pako window).
    tauri::async_runtime::spawn_blocking(consent::open).await.map_err(|e| e.to_string())?;
    saved.map_err(|e| e.to_string())
}

/// First-start setup: saves only its own answers, without checking the download folder (so it
/// closes even with that drive unplugged).
#[tauri::command]
fn save_setup(engine: Eng<'_>, choices: SetupChoices) -> Result<(), String> {
    let mut start = None;
    engine
        .change_settings(|s| {
            s.setup_done = true;
            s.tray_asked = true;
            s.video_asked = true;
            // A new install has nothing "new" to show: What's new starts with the next update.
            s.whats_new_seen = Some(env!("CARGO_PKG_VERSION").to_string());
            if let Some(v) = choices.auto_update {
                s.auto_update = v;
            }
            if let Some(v) = choices.minimize_to_tray {
                s.minimize_to_tray = v;
            }
            if let Some(v) = choices.close_to_tray {
                s.close_to_tray = v;
            }
            if let Some(v) = choices.start_with_windows {
                s.start_with_windows = v;
            }
            if let Some(v) = choices.start_in_tray {
                s.start_in_tray = v;
            }
            if choices.start_with_windows.is_some() || choices.start_in_tray.is_some() {
                start = Some((s.start_with_windows, s.start_in_tray));
            }
            if let Some(v) = choices.browser_connect {
                s.browser_connect = v;
            }
        })
        .map_err(|e| e.to_string())?;
    // Each is tried; each problem is said.
    let errors: Vec<String> = [
        start.map(|(on, tray)| system::set_autostart(on, tray)),
        choices.browser_connect.map(extension::set_registered),
    ]
    .into_iter()
    .flatten()
    .filter_map(Result::err)
    .collect();
    if errors.is_empty() { Ok(()) } else { Err(errors.join(" ")) }
}

/// Writes the extension's files (a folder for Chromium browsers, the signed file for Firefox)
/// next to Pako's data and returns that folder.
#[tauri::command]
fn extension_files() -> Result<String, String> {
    extension::write_files().map(|p| p.to_string_lossy().to_string())
}

/// The browsers installed on this PC that Pako's extension works in.
#[tauri::command]
async fn browsers_found() -> Vec<system::Browser> {
    tauri::async_runtime::spawn_blocking(system::installed_browsers).await.unwrap_or_default()
}

/// The browsers Pako's extension talked to Pako from (Settings shows them as "Connected").
#[tauri::command]
fn browsers_connected() -> Vec<seen::Seen> {
    seen::read(&seen::file())
}

/// What "Add to <browser>" did, for the steps shown under the button.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AddedToBrowser {
    /// Chromium: the extension's folder (its path is on the clipboard when `copied`).
    folder: String,
    copied: bool,
    /// "extensions" = the browser's extensions page opened; "folder" = the browser refused, so
    /// the folder opened instead; "firefox" = Firefox offers to add the signed extension;
    /// "firefoxFolder" = Firefox didn't start, so the folder opened with the file selected;
    /// "unavailable" = this Pako was built without the Firefox version.
    opened: String,
}

/// "Add to <browser>": writes the extension's files and opens the browser where it's added.
/// Chromium browsers: the folder's path goes on the clipboard and the extensions page opens.
/// Firefox: the signed .xpi built into Pako opens in Firefox, which offers to add it.
#[tauri::command]
async fn add_to_browser(app: tauri::AppHandle, id: String) -> Result<AddedToBrowser, String> {
    let (browser, root) = tauri::async_runtime::spawn_blocking(move || {
        let browser = system::installed_browsers().into_iter().find(|b| b.id == id).ok_or("That browser isn't installed.")?;
        extension::write_files().map(|root| (browser, root))
    })
    .await
    .map_err(|e| e.to_string())??;
    if browser.family == "firefox" {
        let opened = match extension::firefox_xpi(&root) {
            Some(xpi) if system::launch_browser(&browser.exe, &xpi.to_string_lossy()) => "firefox",
            Some(xpi) => {
                app.opener().reveal_item_in_dir(&xpi).map_err(|e| e.to_string())?;
                "firefoxFolder"
            }
            None => "unavailable",
        };
        return Ok(AddedToBrowser { folder: root.to_string_lossy().to_string(), opened: opened.into(), ..Default::default() });
    }
    let folder = root.join("chromium");
    let path = folder.to_string_lossy().to_string();
    let copied = system::set_clipboard_text(&path);
    let opened = if system::launch_browser(&browser.exe, system::extensions_page(browser.id)) {
        "extensions"
    } else {
        app.opener().open_path(path.clone(), None::<&str>).map_err(|e| e.to_string())?;
        "folder"
    };
    Ok(AddedToBrowser { folder: path, copied, opened: opened.into() })
}

// ---------- pako:// links ----------

/// What `pako://connect` did, waiting for the window to show it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LinkResult {
    action: &'static str,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

static PENDING_LINK: std::sync::Mutex<Option<LinkResult>> = std::sync::Mutex::new(None);

/// A `pako://` link (Pako started with it, or started again with it): does what it asks, shows
/// the window and tells it (the window picks the result up even if it wasn't loaded yet).
fn handle_pako_link(app: &tauri::AppHandle, link: system::PakoLink) {
    // The terms of use wait: Pako only opens (on them), nothing is changed.
    if !consent::ok() {
        tray::show_main(app);
        return;
    }
    let result = match link {
        system::PakoLink::Connect => {
            let saved = app
                .try_state::<Arc<Engine>>()
                .ok_or_else(|| "Pako is still starting. Try again in a moment.".to_string())
                .and_then(|e| e.change_settings(|s| s.browser_connect = true).map_err(|e| e.to_string()));
            let done = saved.and_then(|_| extension::set_registered(true));
            LinkResult { action: "connect", ok: done.is_ok(), error: done.err() }
        }
    };
    *PENDING_LINK.lock().unwrap() = Some(result);
    tray::show_main(app);
    let _ = app.emit_to("main", "pako-link", serde_json::Value::Null);
}

/// The window takes the last `pako://` result (once).
#[tauri::command]
fn take_pako_link() -> Option<LinkResult> {
    PENDING_LINK.lock().unwrap().take()
}

/// A request from the browser extension (through the native host and the pipe), and Pako's real
/// answer. Videos and "all links" open the Links tab. A download the browser holds paused is
/// confirmed only once Pako's own first request for it got a good answer for the same file (same
/// size, not a web page instead of the file); otherwise it is taken off Pako's list again and the
/// browser keeps its download.
async fn browser_request(app: tauri::AppHandle, r: extension::Request) -> extension::Reply {
    use extension::Reply;
    // The engine starts with the window: not there yet = the host asks again in a moment.
    let Some(engine) = app.try_state::<Arc<Engine>>().map(|e| e.inner().clone()) else { return Reply::not_ready() };
    // Nothing from the browser before the terms of use are accepted (see consent.rs).
    if !consent::ok() {
        // Something was asked of Pako (not just the toolbar window checking the line): Pako
        // opens on its terms, even when it was started hidden for this request or sits in the
        // tray, so the answer isn't only a "!" on the browser's button. Not for a read ahead or
        // its cancel (a menu opening or closing): the pick that follows opens it.
        if !matches!(r.kind.as_str(), "ping" | "videoPrepare" | "videoPrepareCancel") {
            tray::show_main(&app);
        }
        return Reply::error(consent::NOT_YET);
    }
    if r.kind == "ping" {
        return Reply::ok();
    }
    // The result of a takeover answered "held" (see take_over).
    if r.kind == "held" {
        let mut reply = extension::HELD.wait(&r.held, extension::HELD_ASK_WAIT).await;
        // Pako couldn't get it: the browser downloads it, reported to this row ("Via browser").
        if reply.error.is_some() && engine.find(&r.held).is_some_and(|d| d.via_browser.is_some()) {
            reply.followed = Some(r.held.clone());
        }
        return reply;
    }
    // A download the browser makes because Pako couldn't get it: its progress ("Via browser").
    if r.kind == "follow" {
        return engine.follow(&r);
    }
    // A video caught playing on the page (checked by stream::validate_stream). Never from
    // Pako Light (checked when the request came in, and here again).
    if r.kind == "stream" {
        if r.light {
            return Reply::error(extension::LIGHT_REFUSED);
        }
        let Some(url) = r.urls.first().cloned() else { return Reply::error("Unknown request.") };
        return match engine.stream_from_browser(url, r.referrer.clone(), r.filename.clone(), r.site_cookies.clone(), r.private).await {
            Ok(()) => Reply::ok(),
            Err(e) => Reply::error(e),
        };
    }
    // Record this video: one step (start, a part, end or cancel), checked once more here.
    if r.kind == "record" {
        // Never from a private window (refused when it came in, and here again).
        if r.private {
            return Reply::error(extension::NO_PRIVATE_RECORDING);
        }
        let checked = r.record.clone().ok_or_else(|| "Unknown request.".to_string()).and_then(|rec| stream::validate_record(rec, r.light));
        return match checked.and_then(|rec| engine.record(rec)) {
            Ok(()) => Reply::ok(),
            Err(e) => Reply::error(e),
        };
    }
    // The player's Download button opened its quality menu: Pako starts reading the video (the
    // quality picked next uses that read). Answered at once, never with a problem; never from
    // Pako Light (refused when it came in, and here again).
    if r.kind == "videoPrepare" {
        if r.light {
            return Reply::error(extension::LIGHT_REFUSED);
        }
        if let Some(url) = r.urls.first().cloned() {
            engine.video_prepare(url, r.site_cookies.clone(), r.private);
        }
        return Reply::ok();
    }
    // That menu closed without a pick (or its tab closed): the read ahead stops at once.
    if r.kind == "videoPrepareCancel" {
        if r.light {
            return Reply::error(extension::LIGHT_REFUSED);
        }
        if let Some(url) = r.urls.first() {
            engine.video_prepare_cancel(url, r.private);
        }
        return Reply::ok();
    }
    // The player's Download button (in the chosen quality), and "Download video with Pako" on
    // a video page or link (in the usual quality): one video, with the site's login.
    let one_video = r.kind == "download" && !r.takeover && r.urls.len() == 1 && video::is_video_site(&r.urls[0]);
    // Pako Light never asks for a video (checked when the request came in, and here again).
    if r.light && (r.kind == "video" || one_video) {
        return Reply::error(extension::LIGHT_REFUSED);
    }
    if r.kind == "video" || one_video {
        let quality = r.quality.unwrap_or_else(|| engine.settings().video_quality);
        let Some(url) = r.urls.first().cloned() else { return Reply::error("Unknown request.") };
        return match engine.video_from_browser(url, quality, r.site_cookies.clone(), r.private).await {
            Ok(()) => Reply::ok(),
            Err(e) => Reply::error(e),
        };
    }
    // A video page: its quality is picked in Links, so it never replaces the browser's download.
    if r.takeover && r.urls.iter().any(|u| video::is_video_site(u)) {
        return Reply::declined();
    }
    if !r.takeover {
        // Answered at once: a Drive link or a page of links can take Google a while to
        // describe, and nothing about it changes the answer. Added in the order they came.
        static ADDING: std::sync::LazyLock<tokio::sync::Mutex<()>> = std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));
        tauri::async_runtime::spawn(async move {
            let _turn = ADDING.lock().await;
            let since = std::time::Instant::now();
            let (kind, private) = (r.kind.clone(), r.private);
            if engine.add_from_browser(r).await.open_links {
                tray::show_main(&app);
                let _ = app.emit_to("main", "open-view", "found");
            }
            // (A private window's request: not in the timing log.)
            if !private {
                extension::timing(&format!("pako: {kind} added {} ms after the answer", since.elapsed().as_millis()));
            }
        });
        return Reply::ok();
    }
    // The same browser download asked twice at once (the full extension and Pako Light, or a
    // double click): Pako takes it once, and both get that answer.
    let key = (r.urls.first().cloned().unwrap_or_default(), r.total_bytes);
    let hold = r.hold;
    extension::TAKEOVERS.run(key, hold, take_over(app, engine, r)).await
}

/// A download the browser holds paused: added, then confirmed once Pako's own first request got
/// the same file (see browser_request).
async fn take_over(app: tauri::AppHandle, engine: Arc<Engine>, r: extension::Request) -> extension::Reply {
    use extension::Reply;
    let since = std::time::Instant::now();
    let asked = r.clone();
    let added = engine.add_from_browser(r).await;
    if !asked.private {
        extension::timing(&format!("pako: takeover added in {} ms (hold {})", since.elapsed().as_millis(), asked.hold));
    }
    if added.open_links {
        tray::show_main(&app);
        let _ = app.emit_to("main", "open-view", "found");
    }
    // The downloads this request made (never another request's, even for the same link).
    let added = added.ids;
    let take_back = |ids: Vec<String>| {
        for id in &ids {
            extension::forget_headers(id);
        }
        let e2 = engine.clone();
        tauri::async_runtime::spawn_blocking(move || e2.remove(&ids, true))
    };
    let id = match added.len() {
        0 => return Reply::error("Pako couldn't add this download, so the browser keeps it."),
        1 => added[0].clone(),
        // A Drive folder: several files in Pako, one in the browser. They can't be compared, so
        // the browser keeps its download.
        _ => {
            let _ = take_back(added).await;
            return Reply::declined();
        }
    };
    if asked.hold {
        return hold_takeover(engine, id, asked);
    }
    let reply = first_response(&engine, &id, &asked).await;
    if !asked.private {
        extension::timing(&format!("pako: takeover checked {} ms after it came ({})", since.elapsed().as_millis(), if reply.is_ok() { "ok" } else { "refused" }));
    }
    match reply {
        Ok(()) => {
            // Pako has the same file: the browser drops its copy. Then the Download with Pako
            // window (it waits paused meanwhile), or it carries on right away.
            let done = engine.find(&id).is_some_and(|d| d.status == model::Status::Completed);
            if !done && ask::wanted() {
                engine.hold_for_ask(&id);
                ask::enqueue(&id);
            } else {
                ask::started();
            }
            Reply::ok()
        }
        Err(e) => {
            let _ = take_back(vec![id]).await;
            Reply::error(format!("Pako couldn't start this download ({}), so the browser keeps it.", e.trim_end_matches('.')))
        }
    }
}

/// A takeover from an extension that can wait for a second answer: answered "held" at once,
/// with no request to the server first. The Download with Pako window opens now with what the
/// browser knows (the download waits there, not started) or, with the window switched off, it
/// starts right away. Pako's first request is the real download; its first answer is compared
/// with the browser's (see `first_response`). Not the same file, or no file: taken off Pako's
/// list again, a notification says why, and the browser downloads it.
/// When the browser holds its copy paused (`asked.paused`: a form sent with POST, Pako Light, a
/// private window in Chrome), Pako checks first and asks you after, as an older extension's
/// takeover: that paused copy can't wait long for your answer.
fn hold_takeover(engine: Arc<Engine>, id: String, asked: extension::Request) -> extension::Reply {
    let since = std::time::Instant::now();
    extension::HELD.open(&id);
    if asked.paused {
        // (Asked about once checked, below.)
    } else if engine.find(&id).is_some_and(|d| d.ask) {
        ask::enqueue_from_browser(&id, asked.total_bytes);
    } else {
        ask::started();
    }
    let reply = extension::Reply::held(&id);
    tauri::async_runtime::spawn(async move {
        let checked = first_response(&engine, &id, &asked).await;
        if !asked.private {
            extension::timing(&format!(
                "pako: held takeover checked {} ms after the answer ({})",
                since.elapsed().as_millis(),
                if checked.is_ok() { "ok" } else { "refused" }
            ));
        }

        // Removed from Pako meanwhile: you don't want it (the browser drops it too).
        let checked = if engine.find(&id).is_none() { Ok(()) } else { checked };
        if asked.paused && checked.is_ok() {
            // Pako has the same file: the Download with Pako window now (it waits paused).
            let done = engine.find(&id).is_none_or(|d| d.status == model::Status::Completed);
            if !done && ask::wanted() {
                engine.hold_for_ask(&id);
                ask::enqueue(&id);
            } else {
                ask::started();
            }
        }
        let checked = checked.map_err(|e| format!("Pako couldn't get this download itself ({}), so your browser downloads it and Pako follows it.", e.trim_end_matches('.')));
        if let Some(Err(_)) = extension::HELD.finish(&id, checked) {
            // The row stays (your name and folder), marked "Via browser"; the extension
            // reports the browser's download to it. Unless no extension waits for the result
            // any more (the browser closed meanwhile): nobody would download it then, so the
            // row stays Pako's own, with what went wrong (Try again works).
            if extension::HELD.listening(&id) {
                engine.to_via_browser(&id);
            }
        }
    });
    reply
}

/// Waits (a limited time) for Pako's first answer from the server for a new download: Ok once
/// it is the same file the browser got (`asked`: its size and kind of file). That answer is the
/// download's own first request (kept as its first part, see engine `resolve`), so the server
/// is asked once. A download waiting in the Download with Pako window isn't started (or timed)
/// before your answer; one waiting for a free spot or download hours is asked about separately.
async fn first_response(engine: &Arc<Engine>, id: &str, asked: &extension::Request) -> Result<(), String> {
    use model::Status;
    const LIMIT: std::time::Duration = std::time::Duration::from_secs(25);
    let mut start = std::time::Instant::now();
    loop {
        let Some(d) = engine.find(id) else { return Err("it was removed in Pako".into()) };
        if let Some((size, html)) = extension::first_answer(id) {
            return extension::same_as_browser(asked, size, Some(html));
        }
        if d.resolved || d.status == Status::Completed {
            // Started without that first answer (a Google Drive file…): both sizes known and
            // the same, or Pako asks the server itself, to see the kind of file it gives.
            if d.size.is_some() && asked.total_bytes > 0 {
                return extension::same_as_browser(asked, d.size, None);
            }
            let (size, html) = probe(engine, &d).await?;
            return extension::same_as_browser(asked, size.or(d.size), Some(html));
        }
        match d.status {
            Status::Failed | Status::QuotaExceeded | Status::NeedsAccount => {
                return Err(d.message.unwrap_or_else(|| "the server refused it".into()));
            }
            Status::Paused => return Err("it was paused in Pako before it started".into()),
            // In the Download with Pako window: nothing is asked before your answer.
            Status::Queued if d.ask => start = std::time::Instant::now(),
            // Waiting for a free spot or for download hours: Pako asks the server itself now.
            Status::Queued if start.elapsed() >= std::time::Duration::from_secs(2) => {
                let (size, html) = probe(engine, &d).await?;
                return extension::same_as_browser(asked, size, Some(html));
            }
            _ => {}
        }
        if start.elapsed() >= LIMIT {
            return Err("the server didn't answer in time".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// One request for a download, with the browser's referrer and cookies, to see that the server
/// gives the file: its size (when the server says it) and whether it is a web page.
async fn probe(engine: &Arc<Engine>, d: &Download) -> Result<(Option<u64>, bool), String> {
    let mut f = fetch::Fetcher::new(engine.http_client(), &d.source);
    f.set_browser_headers(extension::headers_for(&d.id));
    match tokio::time::timeout(std::time::Duration::from_secs(20), f.get(0, None)).await {
        Ok(Ok(resp)) => {
            let seen = (fetch::probe_info(&resp).size, fetch::is_html(&resp));
            drop(resp);
            Ok(seen)
        }
        Ok(Err(e)) => Err(e.message()),
        Err(_) => Err("the server didn't answer in time".into()),
    }
}

/// Drops the browser's cookies of downloads that finished or are gone.
fn sweep_headers(engine: &Engine) {
    let open: std::collections::HashSet<String> =
        engine.downloads().into_iter().filter(|d| d.status != model::Status::Completed).map(|d| d.id).collect();
    extension::keep_headers(|id| open.contains(id));
}

/// After the list changes: a sweep a moment later (one for a burst of changes).
fn sweep_headers_soon(app: &tauri::AppHandle) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static GEN: AtomicU64 = AtomicU64::new(0);
    let mine = GEN.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if GEN.load(Ordering::Relaxed) != mine {
            return;
        }
        if let Some(engine) = app.try_state::<Arc<Engine>>().map(|e| e.inner().clone()) {
            let _ = tauri::async_runtime::spawn_blocking(move || sweep_headers(&engine)).await;
        }
    });
}

/// The ✕ on a "Done · private" row: off the list now (the file stays). Only a finished private
/// download; anything else is left as it is.
#[tauri::command]
fn forget_private(engine: Eng<'_>, id: String) {
    engine.forget_private(&id);
}

/// Opens the finished file, or shows it in its folder.
#[tauri::command]
fn open_item(app: tauri::AppHandle, engine: Eng<'_>, id: String, reveal: bool) -> Result<(), String> {
    let d = engine.find(&id).ok_or("Download not found")?;
    let done = d.status == model::Status::Completed;
    if !done {
        // Left unfinished by an older Pako: shown under its new name.
        d.adopt_old_part();
    }
    let path = if done { d.final_path() } else { d.part_path() };
    // A private window's download leaves the list once you've opened it (the file stays).
    if done && d.private {
        engine.forget_private(&id);
    }
    if reveal || !done {
        if path.exists() {
            app.opener().reveal_item_in_dir(&path).map_err(|e| e.to_string())
        } else {
            app.opener().open_path(d.dest_dir.display().to_string(), None::<&str>).map_err(|e| e.to_string())
        }
    } else {
        app.opener().open_path(path.display().to_string(), None::<&str>).map_err(|e| e.to_string())
    }
}

/// What finished rows drag out to Explorer or another app: the files (or, for a finished
/// folder, the folder itself), only those still on disk.
#[tauri::command]
fn drag_paths(engine: Eng<'_>, ids: Vec<String>, folder: bool) -> Vec<String> {
    engine.drag_paths(&ids, folder)
}

#[tauri::command]
fn open_folder(app: tauri::AppHandle, path: String) -> Result<(), String> {
    // Only ever opens a folder, never runs a file.
    if !std::path::Path::new(&path).is_dir() {
        return Err("That folder doesn't exist.".into());
    }
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
}

/// The Download with Pako window: the download it asks about now (None = nothing waits).
#[tauri::command]
fn ask_current() -> Option<model::AskView> {
    ask::current()
}

#[tauri::command]
fn ask_answer(answer: model::AskAnswer) -> Result<(), String> {
    ask::answer(answer)
}

/// Help: "What's new" was shown for this version, "Take a quick tour?" was offered, and/or a
/// page's tutorial was gone through.
#[tauri::command]
fn help_seen(engine: Eng<'_>, whats_new: Option<String>, tour_offered: Option<bool>, tour_done: Option<String>) -> Result<(), String> {
    engine
        .change_settings(|s| {
            if let Some(v) = whats_new {
                s.whats_new_seen = Some(v);
            }
            if let Some(v) = tour_offered {
                s.tour_offered = v;
            }
            if let Some(id) = tour_done.filter(|id| !id.is_empty() && id.len() <= 32) {
                if !s.tours_done.contains(&id) {
                    s.tours_done.push(id);
                }
            }
        })
        .map_err(|e| e.to_string())
}

/// The small window's pin: stays open and on top (or closes when you click elsewhere again).
#[tauri::command]
fn mini_pin(app: tauri::AppHandle, engine: Eng<'_>, on: bool) -> Result<(), String> {
    engine.change_settings(|s| s.mini_pinned = on).map_err(|e| e.to_string())?;
    tray::mini_pinned(&app, on);
    Ok(())
}

/// The small window's minimise: out of the way (one click on the tray icon brings it back).
#[tauri::command]
fn mini_hide(app: tauri::AppHandle) {
    tray::hide_mini_now(&app);
}

/// Whether Pako's window opens at start (a pako:// link aside). Not when started by the browser
/// extension's helper (`browser_request` opens it when something is asked while the terms wait).
/// Started by Windows at sign-in "to the tray" (also right after an update put in place at that
/// start): hidden, also while the terms of use wait (user, 2026-11-18: a Pako just updated from
/// the notification mustn't pop up). The terms show when the user opens Pako, or when the browser
/// sends it something (`browser_request`), and the tray's tooltip says so meanwhile.
fn shows_window_at_start(args: &[String], _terms_ok: bool) -> bool {
    !extension::started_for_browser(args) && !args.iter().any(|a| a == system::TRAY_ARG)
}

/// Pako was started again while it runs (by this Pako's single-instance check, or handed over
/// by an older Pako's, see `updates::answer_as_older`). `args`: the exe first, then its arguments.
fn second_start(app: &tauri::AppHandle, args: Vec<String>) {
    // The extension's "Open Pako" button (a pako:// link) while Pako already runs.
    if let Some(link) = system::pako_link_in(&args) {
        handle_pako_link(app, link);
        return;
    }
    // Started by the browser extension's helper while Pako already runs: the request comes
    // through the pipe, nothing to show.
    if extension::started_for_browser(&args) {
        return;
    }
    tray::show_main(app);
}

/// A release Pako ends at once on a panic (`panic = "abort"`): without this it would vanish
/// without a word, and Help › Report a problem would know nothing of it. The problem is noted
/// first, then Pako says so. Its window not opening at all (Tauri's "Failed to setup app",
/// usually a missing or damaged WebView2) says what to do about that.
/// Pako is quitting (its exit has started).
static QUITTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn say_when_pako_stops() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static SAID: AtomicBool = AtomicBool::new(false);
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        if SAID.swap(true, Ordering::SeqCst) {
            return;
        }
        let payload = info.payload();
        let what = payload.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| payload.downcast_ref::<String>().cloned()).unwrap_or_default();
        let at = info.location().map(|l| format!(" ({}:{})", l.file(), l.line())).unwrap_or_default();
        errorlog::note_crash(&format!("Pako stopped: {what}{at}"));
        // A debug build keeps running after a caught panic (and has its console). While Pako is
        // quitting it is noted only: "has to close" on top of a Pako you just closed says nothing.
        if cfg!(not(debug_assertions)) && !QUITTING.load(Ordering::SeqCst) {
            let text = if what.starts_with("Failed to setup app") {
                "Pako couldn't open its window.\n\nStart Pako again. If its window still doesn't open, install Microsoft Edge WebView2 Runtime (from microsoft.com), then start Pako again."
            } else {
                "Pako ran into a problem it couldn't get past and has to close.\n\nStart Pako again: your downloads carry on. If it happens again, use Help › Report a problem (this problem is in its list)."
            };
            message_box("Pako", text);
        }
    }));
}

/// A plain Windows message with an OK button, on top of everything (Pako may have no window).
fn message_box(title: &str, text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW};
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (t, c) = (wide(text), wide(title));
    // SAFETY: NUL-terminated strings that outlive the (blocking) call; no owner window.
    unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), MB_OK | MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST) };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // From here on, a panic is noted and said (see the function).
    say_when_pako_stops();
    // A downloaded update goes in place before anything else starts (then it starts instead).
    if updates::apply_at_start() {
        return;
    }
    // Pako 0.1.2 or earlier is running (its single-instance lock has another name): this start
    // goes to it instead of running a second Pako.
    if updates::step_aside_for_older() {
        return;
    }
    // From here on, a Pako 0.1.2 or earlier started meanwhile sees a Pako running (released at
    // exit, see `RunEvent::Exit`).
    updates::hold_old_lock();
    // Before any window: the window's own data follows the new identifier.
    paths::migrate_webview_data();
    let app = tauri::Builder::default()
        // Only one Pako at a time (two would fight over the same files).
        // Opening it again just brings the existing window to the front.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| second_start(app, args)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_drag::init())
        .setup(|app| {
            // The only Pako running now (single-instance check passed): an older Pako started
            // meanwhile hands its start to this one, like a second start of this Pako.
            let older = app.handle().clone();
            updates::answer_as_older(move |args| second_start(&older, args));
            let handle = app.handle().clone();
            let emit: engine::Emit = Arc::new(move |event, payload| {
                // "notify" = Pako's own notification: a pop-up and/or a line in the bell (see
                // notices.rs; works with the window hidden in the tray).
                // The countdown before sleeping / shutting down shows in Pako's window.
                if event == "show-main" {
                    tray::show_main(&handle);
                    return;
                }
                if event == "notify" {
                    notices::post_payload(&handle, &payload);
                    return;
                }
                if event == "list-changed" {
                    sweep_headers_soon(&handle);
                }
                let _ = handle.emit(event, payload);
            });
            // The browser's cookies for unfinished downloads, kept from last time (before any
            // download starts).
            extension::load_headers();
            // The Dashboard's numbers (the first time, read from what earlier versions kept).
            numbers::open(&paths::data_dir());
            // Cookie files left by a Pako that didn't close normally (before any video starts).
            crate::video::sweep_cookie_files();
            let engine = Engine::new(emit);
            engine.start();
            // Video preview: a tiny server on 127.0.0.1 only.
            if let Ok(p) = tauri::async_runtime::block_on(preview::Preview::start(engine.http_client())) {
                app.manage(p);
            }
            app.manage(engine.clone());
            // The browser extension's requests, through its helper (a pipe only this Windows
            // user's Pako owns).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = extension::serve(move |r| browser_request(handle.clone(), r)).await;
            });
            // The browser's cookies are kept only while their download needs them.
            let sweeper = engine.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    let e = sweeper.clone();
                    let _ = tauri::async_runtime::spawn_blocking(move || sweep_headers(&e)).await;
                }
            });
            // A new version on trial is kept once its window has loaded (see `snapshot`) and it
            // has run a little while.
            let sender = send::Sender::new(app.handle().clone());
            // "Delete after": uploads whose date came are deleted a little after start, then hourly
            // (once the terms of use are accepted, like everything below that runs by itself).
            let sweeper = sender.clone();
            consent::after(move || sweeper.start_sweeper());
            // Uploads in parts that Pako was closed during carry on by themselves ("Continue
            // unfinished uploads when Pako starts"), also once the terms of use are accepted.
            let carry = sender.clone();
            consent::after(move || carry.continue_restored());
            app.manage(sender);
            // Direct (Pako to Pako) starts in the background.
            // Only once Direct was used (contacts, or the Direct page opened before): someone who
            // never uses it makes no Direct connection. The Direct page starts it (direct_wake).
            direct::start_if_used(app.handle());
            // Direct transfers count as busy too: keep-awake and "when all downloads finish" wait for them.
            let h = app.handle().clone();
            engine::also_busy_when(move || h.try_state::<direct::Direct>().and_then(|d| d.node().ok()).is_some_and(|n| n.busy()));
            gates::init(app.handle());
            tray::setup(app)?;
            // Start with Windows: keep Windows' entry pointing at this Pako (it may have moved).
            let s = app.state::<Arc<Engine>>().settings();
            if s.start_with_windows {
                let _ = system::set_autostart(true, s.start_in_tray);
            }
            // Once a day: updates for the tools and for Pako itself. First a little after start,
            // then (for a Pako left running in the tray for days) whenever a day has passed,
            // looked at every hour.
            let handle = app.handle().clone();
            consent::after(move || {
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                let http = http_client();
                let mut first = true;
                loop {
                    if updates::due() {
                        let auto = handle.try_state::<Arc<Engine>>().map(|e| e.settings().auto_update).unwrap_or(false);
                        let mut tools_done = true;
                        if auto {
                            let (names, complete) = updates::update_tools(&http).await;
                            tools_done = complete;
                            if !names.is_empty() {
                                let _ = handle.emit("tools-updated", names);
                            }
                        }
                        let was_ready = updates::ready_now();
                        let (view, worked) = updates::auto_check(&http, auto).await;
                        // Ready now (downloaded by this check): said once per version, also with
                        // Pako in the tray (the window's own message would go unseen there). A
                        // newer one replacing a waiting one is said again.
                        let now_ready = updates::ready_now().filter(|_| view.ready);
                        if now_ready.is_some() && now_ready != was_ready {
                            if let Some(v) = now_ready.as_deref() {
                                notices::post(
                                    &handle,
                                    "update-ready",
                                    &format!("Pako {v} is ready"),
                                    "Restart Pako to start using it. Your downloads and lists stay.",
                                    notices::Go { page: "update".into(), ..Default::default() },
                                    notices::Pop::Pref,
                                );
                            }
                        }
                        // Only a check that worked counts for the day: with no network yet after
                        // the PC woke up, or a tool in use, it's tried again the next hour.
                        if worked && tools_done {
                            updates::mark_checked();
                        }
                        let _ = handle.emit("update-status", view);
                    } else if first {
                        // Already checked today: still tell the window about an update waiting.
                        let _ = handle.emit("update-status", updates::view());
                    }
                    first = false;
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            });
            });
            // The extension's line to Pako: keep the browsers pointing at this Pako (it may have moved).
            if s.browser_connect {
                let _ = extension::set_registered(true);
            }
            // After an update, the extension folder the browser loads from gets this Pako's copy.
            std::thread::spawn(extension::refresh_if_outdated);
            // pako:// links (the extension's "Open Pako" button) open this Pako (it may have moved).
            let _ = system::register_url_protocol();
            // Installed copy: Windows Apps shows the version really installed (Pako's own
            // updates replace Pako.exe without the installer).
            install::sync_apps_entry();
            // Back where it was; started by Windows at sign-in with "to the tray", or by the
            // browser extension for a download: stay hidden (a notification says what happened).
            // Files and folders dropped on the Send tab.
            filedrop::listen(app.handle());
            // The Download with Pako window (and downloads still waiting for it).
            let h = app.handle().clone();
            consent::after(move || ask::init(&h));
            // Light or dark: Pako's window gets the matching background before it shows.
            theme::paint_start(app.handle());
            tray::restore_place(app.handle());
            let args: Vec<String> = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
            if let Some(link) = system::pako_link_in(&args) {
                handle_pako_link(app.handle(), link);
            } else if shows_window_at_start(&args, consent::ok()) {
                tray::show_main(app.handle());
            }
            Ok(())
        })
        // Closing the window: hide in the tray if asked, else quit (the tray list window
        // must not keep a hidden Pako running).
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            // Minimized by Windows (Win+Down, the taskbar button, Show desktop): with
            // "Minimize to the tray" on, hide in the tray like Pako's own button does.
            if let tauri::WindowEvent::Resized(_) = event {
                let to_tray = window.app_handle().try_state::<Arc<Engine>>().map(|e| e.settings().minimize_to_tray).unwrap_or(false);
                if to_tray && window.is_minimized().unwrap_or(false) {
                    let _ = window.unminimize();
                    let _ = window.hide();
                }
            }
            if matches!(event, tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)) {
                tray::remember_place_soon(window.app_handle());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let to_tray = app.try_state::<Arc<Engine>>().map(|e| e.settings().close_to_tray).unwrap_or(false);
                api.prevent_close();
                tray::remember_place(app);
                if to_tray {
                    let _ = window.hide();
                } else {
                    tray::request_quit(app);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            direct_state,
            direct_wake,
            direct_set_name,
            direct_add_contact,
            direct_edit_contact,
            direct_set_favourite,
            direct_set_muted,
            direct_set_check,
            direct_set_tags,
            direct_add_tag,
            direct_rename_tag,
            direct_delete_tag,
            direct_remove_contact,
            direct_connect,
            direct_shown,
            direct_answer_request,
            direct_answer_offer,
            direct_send,
            direct_cancel,
            direct_clear_history,
            direct_open,
            direct::devices_state,
            direct::devices_default_folder,
            direct::devices_add,
            direct::devices_answer,
            direct::devices_set_control,
            direct::devices_set_folder,
            direct::devices_rename,
            direct::devices_watch,
            direct::devices_clear_log,
            direct::devices_download,
            direct::devices_item,
            direct::devices_set_pc_name,
            direct::devices_targets,
            direct::devices_choices,
            direct::devices_pick,
            direct::devices_clear_sent,
            direct::devices_found_split,
            direct::found_set_device,
            direct::found_device_note,
            direct::devices_add_links,
            direct::devices_send_ask,
            direct::direct_from_choices,
            direct::direct_remote_contacts,
            direct::direct_remote_talk,
            direct::direct_relay_send,
            direct::direct_relay_cancel,
            snapshot,
            add_links,
            found_add,
            found_download,
            found_remove,
            gate_focus,
            found_restore,
            restore_downloads,
            move_before,
            open_settings,
            ask_current,
            ask_answer,
            mini_pin,
            help_seen,
            mini_hide,
            send_state,
            send_drive_status,
            send_drive_sign_in,
            send_drive_sign_out,
            send_measure,
            send_start,
            send_sizes,
            send_resume,
            send_cancel,
            send_dismiss,
            sent_remove,
            sent_set_expiry,
            sent_set_note,
            sent_qr,
            sent_check,
            sent_open,
            copy_text,
            quit_now,
            extension_files,
            browsers_found,
            browsers_connected,
            add_to_browser,
            take_pako_link,
            found_set_video,
            found_recheck,
            found_pick_host,
            set_when_done,
            cancel_when_done,
            found_existing,
            found_set,
            set_notify,
            set_folder,
            theme::theme_get,
            theme::theme_set,
            theme::theme_shown,
            set_extract,
            hide_to_tray,
            preview_url,
            preview_check,
            components_status,
            components_check,
            component_install,
            component_remove,
            components_remove,
            save_setup,
            accept_terms,
            update_status,
            update_check,
            update_restart,
            stats,
            numbers_view,
            numbers_contact,
            numbers_export,
            numbers_reset,
            show_main,
            pause_all,
            resume_all,
            pause,
            resume,
            remove,
            clear_completed,
            download_first,
            set_priority,
            start_over,
            rename,
            move_in_queue,
            parts,
            open_link,
            open_in_drive,
            set_settings,
            open_item,
            forget_private,
            open_folder,
            drag_paths,
            notices::notices_list,
            notices::notices_read,
            notices::notices_clear,
            notices::popup_take,
            notices::popup_fit,
            notices::popup_quiet,
            notices::popup_open,
            errorlog::report_info,
            errorlog::report_save,
            errorlog::report_mail,
            errorlog::report_send
        ])
        .build(tauri::generate_context!())
        .expect("error while starting Pako");

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            QUITTING.store(true, std::sync::atomic::Ordering::SeqCst);
            // Quit normally after its window loaded: a new version on trial worked.
            updates::quit_normally();
            if let Some(engine) = handle.try_state::<Arc<Engine>>() {
                engine.shutdown();
            }
            // Nothing left to write: the older Pakos' lock goes now, with today's (released by
            // the single-instance plugin right after this), not when the process is gone.
            updates::release_old_lock();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Pako started in the tray at sign-in (also a 1.0.x one just updated) shows its terms of
    /// use while they wait; otherwise the tray start stays hidden, and a start by the browser's
    /// helper is never shown at start.
    #[test]
    fn terms_waiting_show_the_window_at_start() {
        let args = |a: &[&str]| -> Vec<String> { std::iter::once("Pako.exe").chain(a.iter().copied()).map(String::from).collect() };
        assert!(shows_window_at_start(&args(&[]), true));
        assert!(shows_window_at_start(&args(&[]), false));
        assert!(!shows_window_at_start(&args(&[system::TRAY_ARG]), true));
        assert!(!shows_window_at_start(&args(&[system::TRAY_ARG]), false), "terms waiting: a tray start stays quiet until Pako is used");
        assert!(!shows_window_at_start(&args(&[extension::FROM_BROWSER]), false));
        assert!(!shows_window_at_start(&args(&[extension::FROM_BROWSER]), true));
    }

    /// "Open in browser": a folder Google wouldn't list opens as the folder; a file (also one
    /// listed from a folder link) opens its own page.
    #[test]
    fn drive_page_opens_the_right_thing() {
        let folder = "https://drive.google.com/drive/folders/FOLDER12345?usp=sharing";
        let gdrive = |id: &str, rk: Option<&str>| model::Source::GDrive { file_id: id.into(), resource_key: rk.map(String::from), doc_type: None };
        assert_eq!(drive_page(folder, &gdrive("FOLDER12345", None)), folder);
        assert_eq!(drive_page(folder, &gdrive("FILE1234567", Some("0-k"))), "https://drive.google.com/file/d/FILE1234567/view?resourcekey=0-k");
        let file = "https://drive.google.com/file/d/FILE1234567/view";
        assert_eq!(drive_page(file, &gdrive("FILE1234567", None)), "https://drive.google.com/file/d/FILE1234567/view");
        let web = model::Source::Http { url: "https://x.example/a.zip".into() };
        assert_eq!(drive_page("https://x.example/a.zip", &web), "https://x.example/a.zip");
    }
    // ---------- browser takeovers with a real engine (in a process of their own) ----------

    /// The engine keeps its lists in the data folder, so these run in a child test process
    /// with its own empty one (PAKO_DATA_DIR), never in the developer's.
    #[test]
    fn browser_takeovers_with_a_real_engine() {
        if std::env::var_os("PAKO_ENGINE_CHILD").is_some() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("pako-engine-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::engine_child", "--ignored", "--nocapture", "--test-threads=1"])
            .env("PAKO_DATA_DIR", &dir)
            .env("PAKO_ENGINE_CHILD", "1")
            .output()
            .unwrap();
        let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        for line in text.lines().filter_map(|l| l.find("TIMING").map(|i| &l[i..])) {
            println!("{line}");
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.status.success() && text.contains("1 passed"), "{text}");
    }

    /// A tiny file server: answers after `DELAY` (a slow server), with ranges. Logs each
    /// request as "path range".
    fn slow_server(log: Arc<std::sync::Mutex<Vec<String>>>) -> u16 {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for sock in listener.incoming() {
                let Ok(mut sock) = sock else { continue };
                let log = log.clone();
                std::thread::spawn(move || {
                    let mut buf = vec![0u8; 8192];
                    let n = sock.read(&mut buf).unwrap_or(0);
                    let head = String::from_utf8_lossy(&buf[..n]).to_string();
                    let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let range = head
                        .lines()
                        .find(|l| l.to_ascii_lowercase().starts_with("range:"))
                        .map(|l| l[6..].trim().to_string())
                        .unwrap_or_default();
                    log.lock().unwrap().push(format!("{path} {range}"));
                    std::thread::sleep(DELAY);
                    if path.contains("missing") {
                        let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        return;
                    }
                    let body = file_bytes();
                    let (from, to) = range
                        .strip_prefix("bytes=")
                        .and_then(|r| r.split_once('-'))
                        .map(|(a, b)| (a.parse::<usize>().unwrap_or(0), b.parse::<usize>().ok().map(|b| b.min(body.len() - 1)).unwrap_or(body.len() - 1)))
                        .unwrap_or((0, body.len() - 1));
                    let part = &body[from..=to];
                    let status = if range.is_empty() { "200 OK".to_string() } else { "206 Partial Content".to_string() };
                    let mut answer = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nConnection: close\r\n",
                        part.len()
                    );
                    if !range.is_empty() {
                        answer.push_str(&format!("Content-Range: bytes {from}-{to}/{}\r\n", body.len()));
                    }
                    answer.push_str("\r\n");
                    let _ = sock.write_all(answer.as_bytes());
                    let _ = sock.write_all(part);
                });
            }
        });
        port
    }

    const DELAY: std::time::Duration = std::time::Duration::from_millis(400);
    const SIZE: usize = 300_000;

    fn file_bytes() -> Vec<u8> {
        (0..SIZE).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    #[ignore]
    fn engine_child() {
        if std::env::var_os("PAKO_ENGINE_CHILD").is_none() {
            return;
        }
        use std::time::{Duration, Instant};
        consent::open();
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let port = slow_server(log.clone());
        let url = |p: &str| format!("http://127.0.0.1:{port}{p}");
        let data = paths::data_dir().clone();
        let dl = data.join("Downloads");
        let browser_dir = data.join("Browser downloads");
        std::fs::create_dir_all(&dl).unwrap();
        std::fs::create_dir_all(&browser_dir).unwrap();
        // The notifications the engine sends (pop-up and bell), as the window would get them.
        let heard: Arc<std::sync::Mutex<Vec<serde_json::Value>>> = Arc::default();
        let hear = heard.clone();
        let engine = Engine::new(Arc::new(move |event: &str, p: serde_json::Value| {
            if event == "notify" {
                hear.lock().unwrap().push(p);
            }
        }));
        engine.change_settings(|s| s.download_dir = dl.clone()).unwrap();
        let takeover = |path: &str, total: u64, hold: bool, private: bool| {
            extension::validate(extension::Request {
                kind: "download".into(),
                urls: vec![url(path)],
                filename: path.trim_start_matches('/').into(),
                takeover: true,
                hold,
                private,
                total_bytes: total,
                mime: "application/octet-stream".into(),
                ..Default::default()
            })
            .unwrap()
        };
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let wait_done = |id: &str| {
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(20) {
                if engine.find(id).is_some_and(|d| d.status == model::Status::Completed) {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        };
        let zero_requests = |path: &str| log.lock().unwrap().iter().filter(|l| l.starts_with(path) && l.ends_with("bytes=0-")).count();

        // 1. A Pako that answers at once: "held" before the server has said anything; the
        //    result once its own (only) first request got the same file.
        rt.block_on(async {
            let r = takeover("/good.bin", SIZE as u64, true, false);
            let t0 = Instant::now();
            let id = engine.add_from_browser(r.clone()).await.ids[0].clone();
            let added = t0.elapsed();
            let reply = hold_takeover(engine.clone(), id.clone(), r);
            let answered = t0.elapsed();
            assert!(reply.ok && reply.held.as_deref() == Some(id.as_str()), "{reply:?}");
            assert!(answered < DELAY / 2, "answered in {answered:?}, before the server");
            let confirm = extension::HELD.wait(&id, Duration::from_secs(20)).await;
            let checked = t0.elapsed();
            assert!(confirm.ok, "{confirm:?}");
            assert!(checked >= DELAY, "checked on the server's answer ({checked:?})");
            println!("TIMING new takeover: added {} ms, answered {} ms, checked {} ms (server answers after {} ms)", added.as_millis(), answered.as_millis(), checked.as_millis(), DELAY.as_millis());
            assert!(wait_done(&id), "finished");
            let d = engine.find(&id).unwrap();
            assert_eq!(std::fs::read(d.final_path()).unwrap(), file_bytes());
            // The server was asked for the start of the file once: the download's own request.
            assert_eq!(zero_requests("/good.bin"), 1, "{:?}", log.lock().unwrap());
        });

        // 2. An older extension (no hold): answered only once checked, as before.
        rt.block_on(async {
            let r = takeover("/old.bin", SIZE as u64, false, false);
            let t0 = Instant::now();
            let id = engine.add_from_browser(r.clone()).await.ids[0].clone();
            assert!(first_response(&engine, &id, &r).await.is_ok());
            let answered = t0.elapsed();
            assert!(answered >= DELAY);
            println!("TIMING older extension: answered {} ms (server answers after {} ms)", answered.as_millis(), DELAY.as_millis());
            assert!(wait_done(&id));
            assert_eq!(zero_requests("/old.bin"), 1, "no separate check request");
        });

        // 3. Not the same file (another size): the browser downloads it, Pako follows it.
        let id = rt.block_on(async {
            let r = takeover("/other.bin", SIZE as u64 + 1, true, false);
            let id = engine.add_from_browser(r.clone()).await.ids[0].clone();
            let _ = hold_takeover(engine.clone(), id.clone(), r);
            let confirm = extension::HELD.wait(&id, Duration::from_secs(20)).await;
            assert!(!confirm.ok && confirm.error.as_deref().unwrap_or_default().contains("different file"), "{confirm:?}");
            id
        });
        let d = engine.find(&id).expect("the row stays");
        assert!(d.via_browser.is_some());
        assert_eq!(d.status, model::Status::Paused);
        // The extension reports the browser's download.
        let follow = |pako: &str, browser_id: i64, state: &str, bytes: u64, path: &str| {
            let r = extension::validate(extension::Request {
                kind: "follow".into(),
                urls: vec![url("/other.bin")],
                follow: Some(extension::FollowReport {
                    pako: pako.into(),
                    browser_id,
                    path: path.into(),
                    bytes,
                    total: SIZE as u64,
                    state: state.into(),
                    can_resume: true,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .unwrap();
            engine.follow(&r)
        };
        let reply = follow(&id, 5, "in_progress", 1000, "");
        assert_eq!(reply.followed.as_deref(), Some(id.as_str()));
        let d = engine.find(&id).unwrap();
        assert_eq!((d.status.clone(), d.downloaded, d.size), (model::Status::Downloading, 1000, Some(SIZE as u64)));
        // Pause, Resume: done by the browser (the extension, with its next report).
        engine.pause(&[id.clone()]);
        assert_eq!(follow(&id, 5, "in_progress", 2000, "").command.as_deref(), Some("pause"));
        assert_eq!(follow(&id, 5, "paused", 2000, "").command, None);
        engine.resume(&[id.clone()]);
        assert_eq!(follow(&id, 5, "paused", 2000, "").command.as_deref(), Some("resume"));
        // Pako's queue doesn't apply to it.
        engine.download_first(&[id.clone()]);
        engine.start_over(&[id.clone()]);
        let d = engine.find(&id).unwrap();
        assert_eq!(d.priority, model::Priority::Normal);
        assert_eq!(d.downloaded, 2000);
        // Finished in another folder: moved to the row's folder, keeping the browser's " (1)".
        let file = browser_dir.join("other (1).bin");
        std::fs::write(&file, file_bytes()).unwrap();
        std::fs::write(dl.join("other (1).bin"), b"someone else's").unwrap();
        let reply = follow(&id, 5, "complete", SIZE as u64, &file.to_string_lossy());
        assert!(reply.ok);
        // Moved on its own: the browser's list keeps it until Pako says it's in place.
        assert_eq!(reply.command, None);
        assert!(wait_done(&id));
        // The same report again (the extension's next one): no second move, and now "forget".
        assert_eq!(follow(&id, 5, "complete", SIZE as u64, &file.to_string_lossy()).command.as_deref(), Some("forget"));
        let d = engine.find(&id).unwrap();
        assert_eq!(d.status, model::Status::Completed);
        assert_eq!(d.dest_dir, dl);
        assert!(d.name.starts_with("other (1)") && d.name != "other (1).bin", "never over another file: {}", d.name);
        assert_eq!(std::fs::read(d.final_path()).unwrap(), file_bytes());
        assert_eq!(std::fs::read(dl.join("other (1).bin")).unwrap(), b"someone else's");
        assert!(!file.exists());

        // 4. Reported while Pako didn't know it (Pako was closed): listed; the same folder and
        //    name as the browser's: left where it is.
        let same = dl.join("same.bin");
        std::fs::write(&same, b"done").unwrap();
        let reply = follow("", 9, "complete", 4, &same.to_string_lossy());
        let id9 = reply.followed.unwrap();
        assert!(wait_done(&id9));
        let d = engine.find(&id9).unwrap();
        assert_eq!((d.status.clone(), d.final_path()), (model::Status::Completed, same.clone()));
        assert_eq!(std::fs::read_dir(&dl).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().starts_with("same")).count(), 1);
        // Removed in Pako while the browser still downloads one: the browser cancels it too.
        let gone = follow("", 11, "in_progress", 1, "").followed.unwrap();
        let ids = vec![gone.clone()];
        engine.remove(&ids, false);
        assert_eq!(follow(&gone, 11, "in_progress", 2, "").command.as_deref(), Some("cancel"));
        // The browser's file can't be found to move it: said in the row, and the browser's list
        // keeps it ("keep").
        let lost = follow("", 12, "complete", 4, &dl.join("not-there.bin").to_string_lossy()).followed.unwrap();
        assert!(wait_done(&lost));
        assert!(engine.find(&lost).unwrap().message.is_some());
        assert_eq!(follow(&lost, 12, "complete", 4, "").command.as_deref(), Some("keep"));

        // 4b. The browser holds its copy paused (a form sent with POST): Pako checks first, and
        //     only then the window (none in the tests): the check doesn't wait on an answer.
        rt.block_on(async {
            let mut r = takeover("/paused.bin", SIZE as u64, true, false);
            r.paused = true;
            let r = extension::validate(r).unwrap();
            assert!(r.paused);
            let id = engine.add_from_browser(r.clone()).await.ids[0].clone();
            assert!(!engine.find(&id).unwrap().ask, "not asked before the check");
            let _ = hold_takeover(engine.clone(), id.clone(), r);
            assert!(extension::HELD.wait(&id, Duration::from_secs(20)).await.ok);
            assert!(wait_done(&id));
        });

        // 5. No file at all (404): the browser downloads it, the row follows it.
        rt.block_on(async {
            let r = takeover("/missing.bin", SIZE as u64, true, false);
            let id = engine.add_from_browser(r.clone()).await.ids[0].clone();
            let _ = hold_takeover(engine.clone(), id.clone(), r);
            let confirm = extension::HELD.wait(&id, Duration::from_secs(30)).await;
            assert!(!confirm.ok, "{confirm:?}");
            assert!(engine.find(&id).unwrap().via_browser.is_some());
        });

        // 6. A private window's: no trace beyond the file.
        rt.block_on(async {
            let r = takeover("/private-file.bin", SIZE as u64, true, true);
            let id = engine.add_from_browser(r.clone()).await.ids[0].clone();
            let _ = hold_takeover(engine.clone(), id.clone(), r);
            assert!(extension::HELD.wait(&id, Duration::from_secs(20)).await.ok);
            assert!(wait_done(&id));
            assert!(engine.find(&id).unwrap().private);
            assert!(extension::headers_for(&id).is_none(), "its cookies are gone");
            let r = takeover("/private-missing.bin", SIZE as u64, true, true);
            let failed = engine.add_from_browser(r.clone()).await.ids[0].clone();
            let _ = hold_takeover(engine.clone(), failed.clone(), r);
            let _ = extension::HELD.wait(&failed, Duration::from_secs(30)).await;
            engine.save_now();
            std::thread::sleep(Duration::from_millis(500));
            // Nothing in the data folder names it: not the list, the numbers or the error log.
            let mut stack = vec![data.clone()];
            while let Some(dir) = stack.pop() {
                for e in std::fs::read_dir(&dir).unwrap().flatten() {
                    let p = e.path();
                    if p.starts_with(&dl) || p.starts_with(&browser_dir) {
                        continue;
                    }
                    if p.is_dir() {
                        stack.push(p);
                    } else if let Ok(bytes) = std::fs::read(&p) {
                        assert!(!String::from_utf8_lossy(&bytes).contains("private-"), "{} names it", p.display());
                    }
                }
            }
            // And it leaves the list (opened, or after a while); the file stays.
            let file = engine.find(&id).unwrap().final_path();
            engine.forget_private(&id);
            assert!(engine.find(&id).is_none());
            assert!(file.exists());
        });

        // 7. Everything else from a private window is private too: a right-click download (not
        //    a takeover), "Send all links" (in Links, in memory only, never sent to another PC)
        //    and such a link downloaded from Links. A normal window's links are kept as before.
        rt.block_on(async {
            let req = |kind: &str, paths: &[&str], private: bool| {
                extension::validate(extension::Request { kind: kind.into(), urls: paths.iter().map(|p| url(p)).collect(), private, ..Default::default() }).unwrap()
            };
            let clicked = engine.add_from_browser(req("download", &["/private-click.bin"], true)).await.ids;
            assert_eq!(clicked.len(), 1);
            assert!(engine.find(&clicked[0]).unwrap().private);
            assert!(wait_done(&clicked[0]));
            assert!(engine.add_from_browser(req("links", &["/private-link-a.bin", "/private-link-b.bin"], true)).await.open_links);
            engine.add_from_browser(req("links", &["/plain-link.bin"], false)).await;
            let found = engine.found();
            let private: Vec<&model::FoundItem> = found.iter().filter(|f| f.template.link.contains("private-link")).collect();
            assert_eq!(private.len(), 2);
            assert!(private.iter().all(|f| f.template.private), "marked private in Links");
            let plain = found.iter().find(|f| f.template.link.contains("plain-link")).unwrap();
            assert!(!plain.template.private);
            // "Download on" another PC: never a private link.
            let ids: Vec<String> = found.iter().map(|f| f.id.clone()).collect();
            engine.found_set_device(&ids, Some("home".into()));
            let devices = vec![crate::engine::DeviceChoice { id: "home".into(), name: "Home PC".into(), online: true }];
            let split = engine.found_split(&ids, &devices);
            let sent: Vec<&String> = split.send.iter().flat_map(|(_, _, rows)| rows).collect();
            assert_eq!(sent, vec![&plain.id], "only the normal window's link goes to the other PC");
            for f in engine.found().iter().filter(|f| f.template.private) {
                assert_eq!((f.device.as_deref(), f.device_note.as_deref()), (None, Some(crate::engine::PRIVATE_HERE)));
            }
            assert!(engine.found_private_links().contains(&private[0].template.link));
            // Downloaded from Links: a private download.
            let made = engine.found_download(&[private[0].id.clone()], false, None, false);
            assert_eq!(made, 1);
            let d = engine.downloads().into_iter().find(|d| d.link.contains("private-link-a")).unwrap();
            assert!(d.private);
            assert!(wait_done(&d.id));
            engine.save_now();
            std::thread::sleep(Duration::from_millis(800));
            // The normal link is in Links' file; nothing in the data folder names a private one.
            let links_file = String::from_utf8_lossy(&std::fs::read(data.join("found.json")).unwrap()).to_string();
            assert!(links_file.contains("plain-link"), "{links_file}");
            let mut stack = vec![data.clone()];
            while let Some(dir) = stack.pop() {
                for e in std::fs::read_dir(&dir).unwrap().flatten() {
                    let p = e.path();
                    if p.starts_with(&dl) || p.starts_with(&browser_dir) {
                        continue;
                    }
                    if p.is_dir() {
                        stack.push(p);
                    } else if let Ok(bytes) = std::fs::read(&p) {
                        assert!(!String::from_utf8_lossy(&bytes).contains("private-"), "{} names it", p.display());
                    }
                }
            }
        });

        // 8. Nothing about a private window's links goes to the bell either: a link page that
        //    needs you, and a split set downloaded from Links (the set's one notice, and its
        //    extraction's). A normal window's row still goes to the bell.
        rt.block_on(async {
            let req = |paths: &[&str], private: bool| {
                extension::validate(extension::Request { kind: "links".into(), urls: paths.iter().map(|p| url(p)).collect(), private, ..Default::default() }).unwrap()
            };
            engine.add_from_browser(req(&["/private-set.part1.rar", "/private-set.part2.rar"], true)).await;
            engine.add_from_browser(req(&["/plain-gate.bin"], false)).await;
            let found = engine.found();
            let rows: Vec<String> = found.iter().filter(|f| f.template.link.contains("private-set")).map(|f| f.id.clone()).collect();
            assert_eq!(rows.len(), 2);
            let plain = found.iter().find(|f| f.template.link.contains("plain-gate")).unwrap().id.clone();
            let no_bell = |p: &serde_json::Value| p.get("noBell").and_then(|v| v.as_bool()) == Some(true);
            heard.lock().unwrap().clear();
            engine.notify_row(&rows[0], "needs-you", "Pako needs you", "Finish opening a link from private-gate.example in the Pako window.", notices::Go::item("found", rows[0].clone()));
            engine.notify_row(&plain, "needs-you", "Pako needs you", "Finish opening a link from gate.example in the Pako window.", notices::Go::item("found", plain.clone()));
            {
                let got = heard.lock().unwrap();
                assert_eq!(got.len(), 2);
                assert!(no_bell(&got[0]), "a private row's link page: a pop-up only");
                assert!(!no_bell(&got[1]), "a normal row's link page: in the bell as before");
            }
            heard.lock().unwrap().clear();
            assert_eq!(engine.found_download(&rows, false, None, true), 2);
            let set: Vec<model::Download> = engine.downloads().into_iter().filter(|d| d.link.contains("private-set")).collect();
            assert_eq!(set.len(), 2);
            assert!(set.iter().all(|d| d.private && d.group.as_deref() == Some("private-set")), "{set:?}");
            for d in &set {
                assert!(wait_done(&d.id));
            }
            // The set's notice comes once nothing else finished for 3 seconds (Pako's timer
            // isn't running here: asked by hand).
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(12) && !heard.lock().unwrap().iter().any(|p| p["title"] == "private-set") {
                engine.flush_notes_now();
                std::thread::sleep(Duration::from_millis(100));
            }
            std::thread::sleep(Duration::from_millis(500));
            let got = heard.lock().unwrap();
            assert!(got.iter().any(|p| p["title"] == "private-set"), "{got:?}");
            for p in got.iter().filter(|p| p.to_string().contains("private-set")) {
                assert!(no_bell(p), "never in the bell: {p}");
            }
        });

        // 9. A download neither Pako nor the browser could get (the browser couldn't start it
        //    again, or stopped it at once) is said like any stopped download, never only in the row.
        {
            heard.lock().unwrap().clear();
            let report = |browser_id: i64, error: &str| extension::Request {
                kind: "follow".into(),
                urls: vec![url("/one-time.glb")],
                filename: "one-time.glb".into(),
                follow: Some(extension::FollowReport { browser_id, state: "interrupted".into(), error: error.into(), ..Default::default() }),
                ..Default::default()
            };
            let reply = engine.follow(&report(41, "RESTART_FAILED"));
            let row = reply.followed.clone().expect("listed");
            assert_eq!(engine.find(&row).unwrap().status, model::Status::Failed);
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(8) && heard.lock().unwrap().is_empty() {
                engine.flush_notes_now();
                std::thread::sleep(Duration::from_millis(100));
            }
            let got = heard.lock().unwrap();
            let text = format!("{got:?}");
            assert!(text.contains("download-failed") && text.contains("Download it once more from the page"), "{text}");
        }
        engine.shutdown();
    }
}
