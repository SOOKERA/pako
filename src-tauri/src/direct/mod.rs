// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Direct: Pako-to-Pako transfers, like a messaging app for files.
//!
//! Each Pako has its own key (its "Pako ID" is the public half). Contacts are other Pakos you
//! added; the first connection asks the other person once ("… wants to connect"), after that
//! known contacts connect by themselves while both Pakos run. Files go straight from one PC to
//! the other over QUIC (iroh): directly when the routers allow it, else through the library's
//! public relays, which only ever see encrypted bytes. Finding each other uses the library's
//! public address lookup, and on the same network the local one (mDNS, see `local.rs`): two
//! Pakos on one Wi-Fi find each other and send PC to PC without the internet. No Sookera
//! server is involved. Contacts and history live only on this PC (contacts.json,
//! direct_history.json).
//!
//! Direct starts with Pako only once it has been used (a contact, a transfer, a name, or the
//! Direct page opened once: see `used_before`). Until then nothing of it runs: no relay
//! connection, no address published, no port asked from the router. Opening the Direct page
//! starts it (`Direct::wake`) and from then on it starts with Pako.

mod local;
mod netkind;
mod node;
mod pace;
mod proto;
mod resume;
mod store;
mod words;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use iroh::endpoint::presets;
use iroh::{Endpoint, SecretKey};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

pub use node::{Node, State};
use node::devices::{self, Added, Host, ToAdd};
use node::relay::{RelayChoice, RemoteBook, RemoteTalk};
pub(crate) use node::devices::open_here_first;
use proto::{LinkOpts, Order, RemoteItem};
pub use proto::runs_code;
/// Also used by the problem list (errorlog.rs): a contact's Pako can send any text.
pub(crate) use proto::is_hidden;

/// How often contacts that aren't connected are tried again.
const PRESENCE_EVERY: Duration = Duration::from_secs(180);
/// How long the relay may stay away before Direct counts as without internet.
const OFFLINE_AFTER: Duration = Duration::from_secs(8);

/// Tauri state: the node once it's running, or why it isn't.
#[derive(Default)]
pub struct Direct {
    node: Mutex<Option<Arc<Node>>>,
    error: Mutex<Option<String>>,
    /// Starting has begun (it begins once).
    started: AtomicBool,
    /// This is the first time Direct starts on this PC (Windows may ask about the firewall).
    first: AtomicBool,
    /// Pako itself, to start Direct later (set by `start` / `start_if_used`).
    app: OnceLock<AppHandle>,
    /// Finding Pakos on the same network, and whether this one is seen there.
    nearby: Mutex<Option<Arc<local::Nearby>>>,
}

impl Direct {
    /// The running node. Never starts Direct by itself (the quit question, the tray and
    /// keep-awake ask here too): the Direct page does, with `wake`.
    pub fn node(&self) -> Result<Arc<Node>, String> {
        self.node.lock().unwrap().clone().ok_or_else(|| {
            if !self.started.load(Ordering::SeqCst) {
                return words::local("Direct isn't running yet. Open the Direct page to start it.");
            }
            words::local(&self.error.lock().unwrap().clone().unwrap_or_else(|| "Direct is still starting. Try again in a moment.".into()))
        })
    }

    /// Starts Direct if it isn't running yet (the Direct page opened). Returns at once: the
    /// window gets "direct-changed" when it's ready. From then on Direct starts with Pako.
    pub fn wake(&self) {
        // Never before the terms of use are accepted (no node, nothing said on the network).
        if !crate::consent::ok() {
            return;
        }
        let Some(app) = self.app.get() else { return };
        if !self.started.swap(true, Ordering::SeqCst) {
            begin(app.clone());
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub ready: bool,
    pub error: Option<String>,
    /// Starting for the first time on this PC.
    pub first_start: bool,
    /// Seen by Pakos on the same network: "visible", or why not: "off" (the setting),
    /// "public" (Windows marks a network this PC is on as Public), "unknown" (Windows didn't
    /// say). None when Direct can't look on the network at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nearby: Option<&'static str>,
    #[serde(flatten)]
    pub state: Option<State>,
}

/// Direct's view, in this PC's language (see `words.rs`).
pub fn view(d: &Direct) -> View {
    match d.node.lock().unwrap().clone() {
        Some(n) => View { ready: true, error: None, first_start: false, nearby: d.nearby.lock().unwrap().as_ref().map(|l| l.status()), state: Some(words::in_state(n.state())) },
        None => View { ready: false, error: d.error.lock().unwrap().as_deref().map(words::local), first_start: d.first.load(Ordering::SeqCst), nearby: None, state: None },
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Profile {
    /// Your name for Direct's people.
    #[serde(default)]
    name: String,
    /// This PC's name for your devices ("Desk PC"), kept apart from `name`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pc_name: String,
}

fn profile_path() -> PathBuf {
    crate::paths::data_dir().join("direct_profile.json")
}

fn load_profile() -> Profile {
    std::fs::read(profile_path()).ok().and_then(|b| serde_json::from_slice::<Profile>(&b).ok()).unwrap_or_default()
}

fn save_profile(p: &Profile) {
    if let Ok(b) = serde_json::to_vec_pretty(p) {
        let _ = crate::paths::write_atomic(&profile_path(), &b);
    }
}

pub fn save_name(name: &str) {
    save_profile(&Profile { name: name.trim().chars().take(60).collect(), ..load_profile() });
}

/// This Pako's key, kept encrypted for this Windows user. A copy of the data folder on another
/// PC can't use it: that PC gets its own key (and its own Pako ID).
///
/// A new key is made only when there is none, or the one there can't be decrypted by this
/// Windows user (it's kept aside, never overwritten). If the file can't be read for any other
/// reason (locked, a disk error), or a new key can't be saved, Direct doesn't start: running
/// with a key that isn't saved would give this Pako a new ID every time and break every contact.
fn load_key() -> Result<SecretKey, String> {
    let path = crate::paths::data_dir().join("direct_key.bin");
    match std::fs::read(&path) {
        Ok(sealed) => {
            if let Ok(raw) = crate::secure::unprotect(&sealed) {
                if let Ok(bytes) = <[u8; 32]>::try_from(raw.as_slice()) {
                    return Ok(SecretKey::from_bytes(&bytes));
                }
            }
            // Made by another PC or Windows user: kept aside, a new one is made.
            let aside = path.with_extension(format!("other-{}.bin", crate::model::now_secs()));
            std::fs::rename(&path, &aside).map_err(|e| format!("Direct couldn't set its old key aside ({e})."))?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Direct couldn't read its key ({e}). Restart Pako to try again.")),
    }
    let key = SecretKey::generate();
    let sealed = crate::secure::protect(&key.to_bytes()).map_err(|e| format!("Direct couldn't protect its new key ({e})."))?;
    // Never replaces a key file that appeared meanwhile.
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("Direct couldn't save its new key ({e}). Restart Pako to try again."))?;
    f.write_all(&sealed).and_then(|_| f.sync_all()).map_err(|e| {
        let _ = std::fs::remove_file(&path);
        format!("Direct couldn't save its new key ({e}).")
    })?;
    Ok(key)
}

/// The main window is up and in front (else news also shows as a pop-up).
fn window_in_view(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false) && w.is_focused().unwrap_or(false))
}

/// The file that says Direct was used on this PC (made the first time it starts).
const USED_FLAG: &str = "direct_used";

/// A saved list (contacts, history, blocked people) that holds something. One that can't be
/// read counts as holding something: Direct then starts and deals with it as before.
fn list_has_items(path: &Path) -> bool {
    match std::fs::read(path) {
        Ok(b) => serde_json::from_slice::<Vec<serde_json::Value>>(&b).map(|l| !l.is_empty()).unwrap_or(true),
        Err(e) => e.kind() != std::io::ErrorKind::NotFound,
    }
}

/// Direct was used on this PC: it started once since this version (the flag), or an earlier
/// Pako left contacts, transfers, people turned away or your name for Direct.
fn used_before(dir: &Path) -> bool {
    dir.join(USED_FLAG).exists()
        || ["contacts.json", "direct_history.json", "direct_blocked.json"].iter().any(|f| list_has_items(&dir.join(f)))
        || std::fs::read(dir.join("direct_profile.json")).ok().and_then(|b| serde_json::from_slice::<Profile>(&b).ok()).is_some_and(|p| !p.name.trim().is_empty())
}

/// At Pako's start: Direct starts only if it was used before. Otherwise it waits for the
/// Direct page (`Direct::wake`), and nothing of it touches the network until then.
pub fn start_if_used(app: &AppHandle) {
    let direct = Direct::default();
    let _ = direct.app.set(app.clone());
    app.manage(direct);
    follow_language(app);
    if used_before(crate::paths::data_dir()) {
        // Once the terms of use are accepted (straight away when they already are).
        let app = app.clone();
        crate::consent::after(move || app.state::<Direct>().wake());
    }
}

/// Starts Direct now, used before or not.
#[allow(dead_code)]
pub fn start(app: &AppHandle) {
    let direct = Direct::default();
    let _ = direct.app.set(app.clone());
    app.manage(direct);
    follow_language(app);
    app.state::<Direct>().wake();
}

/// Another language: Direct and My devices ask for their views again (each line is worded
/// when it's shown, see `words.rs`).
fn follow_language(app: &AppHandle) {
    use tauri::Listener;
    let a = app.clone();
    app.listen("language-changed", move |_| {
        let _ = a.emit("direct-changed", ());
        let _ = a.emit("devices-changed", ());
    });
}

/// Starts Direct in the background: the window works meanwhile and shows it as starting.
fn begin(app: AppHandle) {
    // Used from now on: it starts with Pako (someone given this Pako ID can then reach it).
    let flag = crate::paths::data_dir().join(USED_FLAG);
    if !flag.exists() {
        app.state::<Direct>().first.store(true, Ordering::SeqCst);
        let _ = std::fs::write(&flag, b"1");
    }
    let _ = app.emit("direct-changed", ());
    tauri::async_runtime::spawn(async move {
        let key = match load_key() {
            Ok(k) => k,
            Err(e) => {
                *app.state::<Direct>().error.lock().unwrap() = Some(e);
                let _ = app.emit("direct-changed", ());
                return;
            }
        };
        let ep = Endpoint::builder(presets::N0).secret_key(key).alpns(vec![proto::ALPN.to_vec()]).bind().await;
        let ep = match ep {
            Ok(ep) => ep,
            Err(e) => {
                *app.state::<Direct>().error.lock().unwrap() = Some(format!("Direct couldn't start ({e})."));
                let _ = app.emit("direct-changed", ());
                return;
            }
        };
        let profile = load_profile();
        let name = profile.name;
        let dl = app.clone();
        let downloads = Box::new(move || dl.state::<Arc<crate::engine::Engine>>().settings().download_dir.clone());
        let em = app.clone();
        let changed: node::Changed = Arc::new(move || {
            let _ = em.emit("direct-changed", ());
        });
        let na = app.clone();
        // Pako's window in front on the page the news is about (Direct, My devices) shows it
        // itself: no pop-up then (the bell still gets its kinds). On another page, it pops up,
        // except Direct's questions, which the window asks on every page.
        // Files offered to you or received: the arrived chime too (also with the window in
        // front; never for a muted contact, whose notices don't come here).
        let notify: node::Notify2 = Arc::new(move |kind: &str, title: &str, body: &str, go: crate::notices::Go| {
            if let Some(s) = crate::notices::sound_of_notice(kind) {
                crate::notices::sound(s);
            }
            if let Some(e) = na.try_state::<Arc<crate::engine::Engine>>() {
                let pop = crate::notices::pop_unless_on_page(window_in_view(&na), kind, &go);
                e.notify_pop(kind, title, body, go, pop);
            }
        });
        // A question settled anywhere: its pop-up leaves and its bell line says how it ended.
        let sa = app.clone();
        let settle: node::Settle = Arc::new(move |kind: &str, key: &str, text: &str| crate::notices::resolve(&sa, kind, key, text));
        let node = Node::new(ep, crate::paths::data_dir().clone(), name, downloads, changed.clone(), notify, settle);
        // None saved yet: "Name this PC" until you give one.
        let _ = node.set_pc_name(&profile.pc_name);
        // My devices: their orders act on this Pako's downloads.
        let la = app.clone();
        node.set_host(Arc::new(EngineHost(app.clone())), Arc::new(move || {
            let _ = la.emit("devices-changed", ());
        }));
        *app.state::<Direct>().node.lock().unwrap() = Some(node.clone());
        tokio::spawn(node.clone().accept_loop());
        // Contacts on the same network: found and connected without the internet. This Pako
        // is seen there only as "Be visible to Pakos on this network" allows (by default only
        // on networks Windows marks as Private or Domain); it always looks for its contacts.
        if let Some(near) = local::start(&node.ep) {
            *app.state::<Direct>().nearby.lock().unwrap() = Some(near.clone());
            let sa = app.clone();
            let setting = move || sa.try_state::<Arc<crate::engine::Engine>>().map(|e| e.settings().direct_visible).unwrap_or_default();
            let ch = changed.clone();
            tokio::spawn(near.clone().watch(setting, node.stop.clone(), move || ch()));
            tokio::spawn(node.clone().local_loop(near.mdns()));
        }
        // Wait to be online (relay chosen, address published) before calling contacts. Without
        // internet that ends after 20 s and contacts are called anyway: those on the same
        // network answer (the ones found there are called at once, above).
        let _ = tokio::time::timeout(Duration::from_secs(20), node.ep.online()).await;
        tokio::spawn(node.clone().internet_loop(OFFLINE_AFTER));
        tokio::spawn(node.clone().presence_loop());
        let _ = app.emit("direct-changed", ());
        // Progress to the window twice a second while files move.
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if node.stop.is_cancelled() {
                return;
            }
            if node.busy() {
                let _ = app.emit("direct-changed", ());
            }
        }
    });
}

// ---------- My devices: what their orders reach (the engine) ----------

/// The download list as your devices see it, and the only things they may do with it. Links
/// go in like links pasted here (this Pako's own download folder and settings); removing never
/// deletes a file; a file is sent only for a finished download of the list.
struct EngineHost(AppHandle);

impl EngineHost {
    fn engine(&self) -> Option<Arc<crate::engine::Engine>> {
        self.0.try_state::<Arc<crate::engine::Engine>>().map(|e| e.inner().clone())
    }

    /// The engine, when `item` is a download your devices may see and touch: not one waiting
    /// for the browser's answer, and never a private window's (it stays on this PC, with no
    /// trace in your devices' lists or histories).
    fn engine_for(&self, item: &str) -> Option<Arc<crate::engine::Engine>> {
        let e = self.engine()?;
        let d = e.find(item)?;
        shown_to_devices(&d).then_some(e)
    }
}

/// What your devices see of the download list: not what waits for the browser's answer, and
/// never a private window's download.
fn shown_to_devices(d: &crate::model::Download) -> bool {
    !d.ask && !d.private
}

/// The downloads a link from another PC just became: new since `before`, with that link, and
/// never a private window's that the browser added meanwhile (a Drive link waits on Google).
fn made_for_device(downloads: &[crate::model::Download], link: &str, before: &std::collections::HashSet<String>) -> Vec<String> {
    downloads.iter().filter(|d| d.link == link && !before.contains(&d.id) && shown_to_devices(d)).map(|d| d.id.clone()).collect()
}

/// A private window's download listed here that has this very file (a finished download's file
/// was deleted, then the browser saved a private one under its name): never sent to a device.
fn private_file(downloads: &[crate::model::Download], p: &std::path::Path) -> bool {
    let key = crate::names::path_key(p);
    downloads.iter().any(|d| d.private && crate::names::path_key(&d.final_path()) == key)
}

impl Host for EngineHost {
    fn list(&self) -> Vec<RemoteItem> {
        let Some(e) = self.engine() else { return Vec::new() };
        let mut list = e.downloads();
        // Newest first; what waits for the browser's answer, and a private window's download,
        // aren't shown.
        list.retain(shown_to_devices);
        list.sort_by(|a, b| b.date.cmp(&a.date).then(b.added_at.cmp(&a.added_at)));
        list.truncate(devices::MAX_LIST);
        list.into_iter()
            .map(|d| RemoteItem {
                status: serde_json::to_value(&d.status).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default(),
                name: if d.name.is_empty() { d.link.clone() } else { d.name.clone() },
                size: d.size.unwrap_or(0),
                done: d.downloaded,
                speed: 0,
                // In English when it's one of the words that travel (the other PC words it).
                message: d.message.as_deref().map(crate::model::in_english),
                priority: devices::priority_word(d.priority),
                id: d.id,
            })
            .collect()
    }

    /// One link at a time, so each one not taken is known (the PC that sent it keeps it).
    /// See `route`: a video downloads at once in the quality picked on the other PC; a link
    /// page, or a video from an older Pako, goes to Links, as if pasted there. Each one in this
    /// PC's own download folder, or a checked subfolder of it (`devices::place_for`).
    fn add(&self, links: Vec<ToAdd>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Added> + Send>> {
        let e = self.engine();
        Box::pin(async move {
            let mut out = Added::default();
            let Some(e) = e else {
                out.failed = links.into_iter().map(|l| (l.link, "Pako isn't ready yet.".to_string())).collect();
                return out;
            };
            let settings = e.settings();
            // Why a link wasn't added, in English (the PC that sent it words it in its own
            // language, `words`), without the link the engine puts first (`Added::text` does).
            let why = |link: &str, errors: Vec<String>| errors.into_iter().next().map_or_else(|| "not added".into(), |e| reason(link, &e));
            // Videos are read by yt-dlp side by side; the answer waits for them up to here
            // (the other PC gives up on an answer after a minute). One still being read then
            // counts as taken: it goes on here.
            let deadline = tokio::time::Instant::now() + VIDEO_WAIT;
            let mut videos = Vec::new();
            for l in links {
                let route = route(&l, settings.video_quality);
                // Files are added one after the other (a Drive link asks Google first): past
                // the deadline, the rest aren't added and go back to the PC that sent them,
                // which then knows exactly what was taken (no "No answer." for links that
                // were in fact added, and no second copy when they're sent again).
                if !matches!(route, Route::Video(_)) && tokio::time::Instant::now() >= deadline {
                    out.failed.push((l.link, TOO_SLOW.into()));
                    continue;
                }
                // The folder only for what is saved here now (a link opened in Links isn't).
                let place = |out: &mut Added| {
                    let (dir, bad_folder) = devices::place_for(&settings.download_dir, l.folder.as_deref());
                    if bad_folder {
                        out.unsafe_folders += 1;
                    }
                    dir
                };
                match route {
                    Route::Links => {
                        let r = e.found_add_from_device(&l.link).await;
                        // Already in Links there counts as taken (never a private window's row:
                        // that one goes at quit, so the link gets a row of its own).
                        if r.added + r.skipped > 0 {
                            out.to_links += 1;
                        } else {
                            out.failed.push((l.link.clone(), why(&l.link, r.errors)));
                        }
                    }
                    Route::Video(q) => {
                        let dir = place(&mut out);
                        let id = uuid::Uuid::new_v4().to_string();
                        videos.push((l.link.clone(), id.clone(), e.video_from_device(l.link, q, l.priority, dir, id)));
                    }
                    Route::File => {
                        let dir = place(&mut out);
                        let before: std::collections::HashSet<String> = e.downloads().into_iter().map(|d| d.id).collect();
                        let r = e.add_links_from_device(&l.link, Some(dir)).await;
                        if r.added > 0 {
                            out.downloads += r.added;
                            let made = made_for_device(&e.downloads(), &l.link, &before);
                            if l.priority != crate::model::Priority::Normal {
                                e.set_priority(&made, l.priority);
                            }
                            out.items.push((l.link, made));
                        } else {
                            out.failed.push((l.link.clone(), why(&l.link, r.errors)));
                        }
                    }
                }
            }
            for (link, id, answer) in videos {
                match tokio::time::timeout_at(deadline, answer).await {
                    Ok(Ok(Ok(crate::engine::DeviceVideo::ToLinks))) => out.to_links += 1,
                    Ok(Ok(Ok(crate::engine::DeviceVideo::Downloading(have)))) => {
                        out.downloads += 1;
                        out.items.push((link, vec![have]));
                    }
                    Ok(Ok(Err(msg))) => {
                        let msg = reason(&link, &msg);
                        out.failed.push((link, msg));
                    }
                    // Still being read: it goes on here, under the id it was given.
                    _ => {
                        out.downloads += 1;
                        out.items.push((link, vec![id]));
                    }
                }
            }
            out
        })
    }

    fn set_priority(&self, item: &str, priority: crate::model::Priority) {
        if let Some(e) = self.engine_for(item) {
            e.set_priority(&[item.to_string()], priority);
        }
    }

    fn pause(&self, item: &str) {
        if let Some(e) = self.engine_for(item) {
            e.pause(&[item.to_string()]);
        }
    }

    fn resume(&self, item: &str) {
        if let Some(e) = self.engine_for(item) {
            e.resume(&[item.to_string()]);
        }
    }

    fn remove(&self, item: &str) {
        if let Some(e) = self.engine_for(item) {
            e.remove(&[item.to_string()], false);
        }
    }

    fn file_of(&self, item: &str) -> Option<PathBuf> {
        let e = self.engine_for(item)?;
        let d = e.find(item)?;
        let p = d.final_path();
        (d.status == crate::model::Status::Completed && p.is_file() && !private_file(&e.downloads(), &p)).then_some(p)
    }
}

/// How long an order's answer waits for its videos to be read (the other PC waits a minute).
const VIDEO_WAIT: Duration = Duration::from_secs(40);
/// A link not added before then (the others took the time): the PC that sent it keeps it.
const TOO_SLOW: &str = "Pako there was still busy adding the others: send it again";

/// Why a link another PC sent wasn't added, as it goes back: without the "link: " the engine
/// puts before a paste's errors (the answer names the link itself), and in English when Pako
/// knows the words (`model::in_english`; the PC that shows it words it in its language).
fn reason(link: &str, e: &str) -> String {
    let short = crate::engine::short(link);
    let e = [format!("{short}: "), format!("{short}\u{a0}: ")].iter().find_map(|p| e.strip_prefix(p.as_str())).unwrap_or(e);
    crate::model::in_english(e)
}

/// Where a link from another of your PCs goes here.
#[derive(Debug, PartialEq)]
enum Route {
    /// Links, as if pasted here: a known link page (Pako works through its pages to the
    /// file), or a video site's page from an older Pako (its quality is picked here).
    Links,
    /// Downloaded at once in this quality (a playlist still turns out to go to Links).
    Video(u32),
    /// Straight to the downloads.
    File,
}

fn route(l: &ToAdd, default_quality: u32) -> Route {
    let gate = url::Url::parse(&l.link).ok().and_then(|u| u.host_str().map(crate::gates::is_known_gate)).unwrap_or(false);
    if gate {
        return Route::Links;
    }
    if crate::video::is_video_site(&l.link) {
        if crate::video::is_playlist_link(&l.link) {
            return Route::Links;
        }
        return match (l.quality, l.newer) {
            (Some(q), _) => Route::Video(q),
            (None, true) => Route::Video(default_quality),
            (None, false) => Route::Links,
        };
    }
    Route::File
}

/// A video site's link this PC sends with a quality: the one picked, else this PC's default
/// (0 = audio only). Other links carry none.
fn with_quality(mut l: LinkOpts, default_quality: u32) -> LinkOpts {
    if crate::video::is_video_site(l.link.trim()) {
        l.quality = l.quality.or(Some(default_quality));
    } else {
        l.quality = None;
    }
    l
}

/// "Check this contact" as the window says it: "often" with how long ("hour", "today",
/// "week"), "auto" or "ask".
pub fn parse_check(s: &str, span: Option<&str>) -> Result<(pace::Check, Option<pace::OftenFor>), String> {
    match (s, span) {
        ("often", Some("hour")) => Ok((pace::Check::Often, Some(pace::OftenFor::Hour))),
        ("often", Some("today")) => Ok((pace::Check::Often, Some(pace::OftenFor::Today))),
        ("often", Some("week")) => Ok((pace::Check::Often, Some(pace::OftenFor::Week))),
        ("auto", _) => Ok((pace::Check::Auto, None)),
        ("ask", _) => Ok((pace::Check::Ask, None)),
        _ => Err(crate::tr!("Unknown choice.", "Choix inconnu.")),
    }
}

fn engine_of(app: &AppHandle) -> Option<Arc<crate::engine::Engine>> {
    app.try_state::<Arc<crate::engine::Engine>>().map(|e| e.inner().clone())
}

// ---------- My devices: the window's commands ----------

type Dir<'a> = tauri::State<'a, Direct>;

#[tauri::command]
pub async fn devices_state(app: AppHandle) -> Result<devices::DevicesState, String> {
    // Built off the window's thread (the tray window asks for it while files move).
    tauri::async_runtime::spawn_blocking(move || Ok(words::in_devices(app.state::<Direct>().node()?.devices_state()))).await.map_err(|e| e.to_string())?
}

/// The suggested receiving folder for a device named so.
#[tauri::command]
pub fn devices_default_folder(d: Dir<'_>, name: String) -> Result<String, String> {
    Ok(d.node()?.default_folder(&name))
}

/// Names this PC for your devices (they see it at once); Direct's name for people stays.
#[tauri::command]
pub async fn devices_set_pc_name(d: Dir<'_>, name: String) -> Result<String, String> {
    let node = d.node()?;
    off(move || {
        let kept = node.set_pc_name(&name).map_err(words::say)?;
        save_profile(&Profile { pc_name: kept.clone(), ..load_profile() });
        Ok(kept)
    })
    .await
}

/// Saving work (contacts, the profile, the history: written and synced to the disk) is done
/// off the window's thread, so a click never makes the window wait for the disk.
async fn off<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?.map_err(words::say)
}

/// Pairs a PC as yours ("This is my device"), with its receiving folder.
#[tauri::command]
pub fn devices_add(d: Dir<'_>, id: String, name: Option<String>, folder: Option<String>) -> Result<(), String> {
    let node = d.node()?;
    if !node.has_pc_name() {
        return Err(words::local("Name this PC first: your other PC sees it."));
    }
    node.add_device(&id, name.as_deref(), folder).map_err(words::say)
}

/// "… wants to pair as your device": answered only in Pako's own window (the safety code).
#[tauri::command]
pub fn devices_answer(window: tauri::Window, d: Dir<'_>, peer: String, accept: bool, folder: Option<String>) -> Result<(), String> {
    if window.label() != "main" {
        return Err(words::local("Answer this in Pako's window."));
    }
    let node = d.node()?;
    if accept && !node.has_pc_name() {
        return Err(words::local("Name this PC first: your other PC sees it."));
    }
    node.answer_device_request(&peer, accept, folder);
    Ok(())
}

#[tauri::command]
pub async fn devices_set_control(d: Dir<'_>, id: String, on: bool) -> Result<(), String> {
    let node = d.node()?;
    off(move || node.set_control(&id, on)).await
}

#[tauri::command]
pub async fn devices_set_folder(d: Dir<'_>, id: String, folder: String) -> Result<(), String> {
    let node = d.node()?;
    off(move || node.set_device_folder(&id, &folder)).await
}

#[tauri::command]
pub async fn devices_rename(d: Dir<'_>, id: String, name: String) -> Result<(), String> {
    let node = d.node()?;
    off(move || node.edit_contact(&id, Some(name), None, None)).await
}

#[tauri::command]
pub fn devices_watch(d: Dir<'_>, id: String, on: bool) -> Result<(), String> {
    d.node()?.device_watch(&id, on);
    Ok(())
}

#[tauri::command]
pub async fn devices_clear_log(d: Dir<'_>, id: String) -> Result<(), String> {
    let node = d.node()?;
    off(move || {
        node.clear_device_log(&id);
        Ok(())
    })
    .await
}

#[derive(Serialize)]
pub struct DownloadThere {
    /// What happened there ("Added 2 links to the downloads"), or None if nothing was sent.
    text: Option<String>,
    /// The links picked here first (a playlist, a Drive folder, a link page): each one gets a
    /// picker card in My devices › Links.
    pick: Vec<String>,
}

/// Links that are picked on this PC before anything is sent: a playlist, a Drive folder or a
/// link page. Everything else (a file, one video) goes as it is.
fn picked_here(link: &str) -> bool {
    open_here_first(link).is_some() || crate::drive::parse_link(link).is_some_and(|l| l.kind == crate::drive::LinkKind::Folder)
}

/// My devices › Links › Send: the links found in the pasted text, each file and single video
/// sent at once with the quality (videos), priority and subfolder picked under the box. A
/// playlist, a Drive folder or a link page is never sent as it is: it comes back in `pick`,
/// to be read here and its videos or files picked (`devices_pick`).
#[tauri::command]
pub async fn devices_download(
    app: AppHandle,
    d: Dir<'_>,
    id: String,
    text: String,
    quality: Option<u32>,
    priority: Option<String>,
    folder: Option<String>,
) -> Result<DownloadThere, String> {
    let links = devices::links_in(&text);
    if links.is_empty() {
        return Err(words::local("No web link in what you pasted."));
    }
    devices::check_links(&links).map_err(words::say)?;
    let folder = devices::send_folder(folder.as_deref()).map_err(words::say)?;
    let node = d.node()?;
    let default_q = engine_of(&app).map(|e| e.settings().video_quality).unwrap_or(1080);
    let (pick, send): (Vec<String>, Vec<String>) = links.into_iter().partition(|l| picked_here(l));
    let mut out = DownloadThere { text: None, pick };
    if !send.is_empty() {
        let opts = send
            .into_iter()
            .map(|link| with_quality(LinkOpts { link, quality, priority: priority.clone(), folder: folder.clone() }, default_q))
            .collect();
        let (text, _) = node.device_add(&id, opts).await.map_err(words::say)?;
        out.text = Some(words::local(&text));
    }
    Ok(out)
}

#[derive(Serialize)]
pub struct Picked {
    title: String,
    items: Vec<crate::engine::PickItem>,
}

/// My devices › Links: a playlist, a Drive folder or a link page read on this PC, its videos
/// or files listed to pick.
#[tauri::command]
pub async fn devices_pick(app: AppHandle, link: String) -> Result<Picked, String> {
    let link = devices::check_links(&[link]).map_err(words::say)?.remove(0);
    let engine = engine_of(&app).ok_or_else(|| words::local("Pako is still starting."))?;
    let (title, items) = engine.pick_items(&link).await?;
    Ok(Picked { title, items })
}

/// Clears My devices › Links for one device.
#[tauri::command]
pub fn devices_clear_sent(d: Dir<'_>, id: String) -> Result<(), String> {
    d.node()?.clear_device_sent(&id);
    Ok(())
}

#[derive(Serialize)]
pub struct Target {
    id: String,
    name: String,
}

/// "Download on ▾": your devices that let this PC start downloads on them and are connected.
/// Empty while Direct isn't running.
#[tauri::command]
pub fn devices_targets(d: Dir<'_>) -> Vec<Target> {
    d.node().map(|n| n.device_targets().into_iter().map(|(id, name)| Target { id, name }).collect()).unwrap_or_default()
}

/// Links › "Download on" (per link): your devices that let this PC control them, online or
/// not. Links set to a device that's no longer among them go back to This PC (noted). Empty,
/// and nothing changed, while Direct isn't running.
#[tauri::command]
pub fn devices_choices(app: AppHandle, d: Dir<'_>) -> Vec<crate::engine::DeviceChoice> {
    let Ok(node) = d.node() else { return Vec::new() };
    let list = node.device_choices();
    if let Some(e) = engine_of(&app) {
        e.found_device_fallback(&list.iter().map(|c| c.id.clone()).collect::<Vec<_>>());
    }
    list
}

/// Pressing Download in Links: where each picked link goes (this PC, or the PC it's set to).
/// Without Direct running, links set to a device stay (noted as offline).
#[tauri::command]
pub fn devices_found_split(app: AppHandle, d: Dir<'_>, ids: Vec<String>) -> Result<crate::engine::DeviceSplit, String> {
    let engine = engine_of(&app).ok_or_else(|| words::local("Pako is still starting."))?;
    let devices = match d.node() {
        Ok(n) => n.device_choices(),
        // Not running: every device counts as offline (nothing set back to This PC).
        Err(_) => engine.found().iter().filter_map(|f| f.device.clone()).map(|id| crate::engine::DeviceChoice { name: crate::tr!("That PC", "Ce PC"), id, online: false }).collect(),
    };
    Ok(engine.found_split(&ids, &devices))
}

/// Links › details › "Download on": None = This PC.
#[tauri::command]
pub fn found_set_device(app: AppHandle, ids: Vec<String>, device: Option<String>) -> Result<(), String> {
    engine_of(&app).ok_or_else(|| words::local("Pako is still starting."))?.found_set_device(&ids, device);
    Ok(())
}

/// The note on rows a device didn't take (None clears it).
#[tauri::command]
pub fn found_device_note(app: AppHandle, ids: Vec<String>, note: Option<String>) -> Result<(), String> {
    engine_of(&app).ok_or_else(|| words::local("Pako is still starting."))?.found_device_note(&ids, note);
    Ok(())
}

#[derive(Serialize)]
pub struct Sent {
    /// What happened there ("Added 2 links to the downloads").
    text: String,
    /// The links it didn't take: their rows stay in Links here.
    failed: Vec<String>,
    /// Links not sent because they're opened here first (a playlist, a link page), with why.
    held: Vec<(String, String)>,
}

/// "Download on <device>" in Links: these links, each with its quality (a video's: the one
/// picked on its row, else this PC's default), priority and subfolder, downloaded there. A
/// playlist or a link page is never sent as it is (held: its row says why).
/// `names`: each link's name as known here, for the card in My devices › Links.
#[tauri::command]
pub async fn devices_add_links(app: AppHandle, d: Dir<'_>, id: String, links: Vec<LinkOpts>, names: Option<std::collections::HashMap<String, String>>) -> Result<Sent, String> {
    let default_q = engine_of(&app).map(|e| e.settings().video_quality).unwrap_or(1080);
    // A private window's links in Links stay on this PC (held: their rows say why).
    let private = engine_of(&app).map(|e| e.found_private_links()).unwrap_or_default();
    let (held, send): (Vec<LinkOpts>, Vec<LinkOpts>) = links.into_iter().partition(|l| held_why(l.link.trim(), &private).is_some());
    // Trimmed, as the window compares them.
    let held: Vec<(String, String)> = held.into_iter().map(|l| (l.link.trim().to_string(), held_why(l.link.trim(), &private).unwrap_or_default())).collect();
    if send.is_empty() {
        return Err(held.first().map(|(_, w)| w.clone()).unwrap_or_else(|| words::local("Nothing to send.")));
    }
    // A folder name the other PC would refuse is refused here, saying why (not silently dropped).
    let send = send
        .into_iter()
        .map(|mut l| {
            l.folder = devices::send_folder(l.folder.as_deref()).map_err(words::say)?;
            Ok(with_quality(l, default_q))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // A large folder goes in orders of up to MAX_LINKS (the other PC takes no more at once).
    // An order that fails after another went keeps its links here as not taken, saying why.
    let names = names.unwrap_or_default();
    let node = d.node()?;
    let (mut texts, mut failed, mut first_err, mut any_ok) = (Vec::new(), Vec::new(), None::<String>, false);
    for part in send.chunks(devices::MAX_LINKS) {
        match node.device_add_named(&id, part.to_vec(), &names).await {
            Ok((t, f)) => {
                any_ok = true;
                texts.push(words::local(&t));
                failed.extend(f);
            }
            Err(e) => {
                let e = words::local(&e);
                failed.extend(part.iter().map(|l| l.link.trim().to_string()));
                texts.push(crate::tr!("Not taken: {} ({e})", "Non accepté : {} ({e})", part.len()));
                first_err.get_or_insert(e);
            }
        }
    }
    if let (Some(e), false) = (first_err, any_ok) {
        return Err(e);
    }
    Ok(Sent { text: texts.join(". "), failed, held })
}

/// Why a link isn't sent to another PC as it is: opened here first (a playlist, a link page),
/// or a private window's (`private`: those links in Links). None = it is sent.
fn held_why(link: &str, private: &std::collections::HashSet<String>) -> Option<String> {
    if private.contains(link) {
        return Some(crate::engine::private_here());
    }
    open_here_first(link)
}

/// "Download on <device>" in the Download with Pako window: the waiting download's link goes
/// to that device and, once it took it, leaves this PC (as Cancel does). A video carries the
/// quality chosen in the window; it goes as First, like the window's "Download now".
#[tauri::command]
pub async fn devices_send_ask(app: AppHandle, d: Dir<'_>, id: String, device: String, quality: Option<u32>) -> Result<String, String> {
    let engine = engine_of(&app).ok_or_else(|| words::local("Pako is still starting."))?;
    let item = engine.find(&id).filter(|x| x.ask).ok_or_else(|| words::local("This download is no longer waiting."))?;
    // A private window's download stays on this PC.
    if item.private {
        return Err(crate::engine::private_here());
    }

    if crate::engine::here_only(&item) {
        return Err(words::local("A video caught on a page downloads on this PC only."));
    }
    if let Some(why) = open_here_first(&item.link) {
        return Err(why);
    }
    let default_q = engine.settings().video_quality;
    // Only the link goes: never its cookies or the browser's headers.
    let quality = quality.filter(|q| crate::extension::VIDEO_QUALITIES.contains(q));
    let link = with_quality(LinkOpts { link: item.link.clone(), quality, priority: Some("first".into()), folder: None }, default_q);
    let names = std::collections::HashMap::from([(item.link.clone(), item.name.clone())]);
    let (text, _) = d.node()?.device_add_named(&device, vec![link], &names).await.map_err(words::say)?;
    let text = words::local(&text);
    // It went: whatever happens here now, the window must not say "Not sent". (Answered
    // meanwhile, it stays as it was answered.)
    let _ = crate::ask::answer(crate::model::AskAnswer { id, action: "cancel".into(), stem: String::new(), dir: String::new(), quality: None, alert: None, dont_ask: false, private: false });
    Ok(text)
}

/// Pause, resume, remove or get one download of a device's list, or change its priority
/// ("first", "normal", "later").
#[tauri::command]
pub async fn devices_item(d: Dir<'_>, id: String, item: String, action: String) -> Result<String, String> {
    let order = match action.as_str() {
        "pause" => Order::Pause { item },
        "resume" => Order::Resume { item },
        "remove" => Order::Remove { item },
        "get" => Order::Get { item },
        "first" | "normal" | "later" => Order::Priority { item, priority: action },
        _ => return Err(words::local("Unknown action.")),
    };
    d.node()?.device_order(&id, order).await.map(|t| words::local(&t)).map_err(words::say)
}

// ---------- Direct relay: sending from another of your PCs ----------

/// Your devices Direct can send from (the "From" menu): online or not, by name.
#[tauri::command]
pub fn direct_from_choices(d: Dir<'_>) -> Vec<RelayChoice> {
    d.node().map(|n| n.relay_choices()).unwrap_or_default()
}

/// That PC's Direct contacts, read live.
#[tauri::command]
pub async fn direct_remote_contacts(d: Dir<'_>, device: String) -> Result<RemoteBook, String> {
    d.node()?.remote_book(&device).await.map_err(words::say)
}

/// That PC's history with one of its contacts, read live.
#[tauri::command]
pub async fn direct_remote_talk(d: Dir<'_>, device: String, contact: String) -> Result<RemoteTalk, String> {
    d.node()?.remote_talk(&device, &contact).await.map(words::in_talk).map_err(words::say)
}

/// Sends files to one of that PC's contacts, through that PC. `keep`: keep a copy there.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn direct_relay_send(d: Dir<'_>, device: String, contact: String, contact_name: String, paths: Vec<String>, compress: bool, name: Option<String>, keep: bool) -> Result<String, String> {
    let node = d.node()?;
    node.relay_send(&device, &contact, &contact_name, paths.into_iter().map(PathBuf::from).collect(), compress, name, keep).await.map_err(words::say)
}

/// Stops a relay this PC asked for.
#[tauri::command]
pub fn direct_relay_cancel(d: Dir<'_>, id: String) -> Result<(), String> {
    d.node()?.relay_cancel(&id).map_err(words::say)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(link: &str, quality: Option<u32>, newer: bool) -> ToAdd {
        ToAdd { link: link.into(), quality, priority: crate::model::Priority::Normal, newer, folder: None }
    }

    #[test]
    fn your_devices_never_see_a_private_windows_download() {
        let mk = |private: bool, ask: bool| {
            let mut d = crate::engine::new_download("https://example.com/a.zip", crate::model::Source::Http { url: "https://example.com/a.zip".into() }, "a.zip".into(), std::path::PathBuf::from("."), None);
            d.private = private;
            d.ask = ask;
            d
        };
        assert!(shown_to_devices(&mk(false, false)));
        assert!(!shown_to_devices(&mk(true, false)), "a private window's download stays on this PC");
        assert!(!shown_to_devices(&mk(false, true)), "a download waiting for the browser's answer isn't shown");
        assert!(!shown_to_devices(&mk(true, true)));
    }

    /// A link from another PC: the downloads it became never include a private window's that
    /// the browser added meanwhile (its id would go to that PC and get its priority).
    #[test]
    fn a_link_from_another_pc_never_takes_a_private_download_added_meanwhile() {
        let link = "https://drive.google.com/file/d/abc/view";
        let mk = |private: bool| {
            let mut d = crate::engine::new_download(link, crate::model::Source::Http { url: link.into() }, "a.zip".into(), std::path::PathBuf::from("."), None);
            d.private = private;
            d
        };
        let (old, mine, private) = (mk(false), mk(false), mk(true));
        let before = std::collections::HashSet::from([old.id.clone()]);
        assert_eq!(made_for_device(&[old, mine.clone(), private], link, &before), vec![mine.id]);
    }

    /// "Get" never sends a file a private window's download now has (a finished download's
    /// file was deleted, then the browser saved a private one under the same name).
    #[test]
    fn get_never_sends_a_file_a_private_download_has() {
        let dir = std::path::PathBuf::from("D:\\Downloads");
        let mk = |name: &str, private: bool| {
            let mut d = crate::engine::new_download("https://e.org/a.zip", crate::model::Source::Http { url: "https://e.org/a.zip".into() }, name.into(), dir.clone(), None);
            d.private = private;
            d
        };
        let listed = mk("a.zip", false);
        let p = listed.final_path();
        assert!(!private_file(&[listed.clone()], &p));
        assert!(!private_file(&[listed.clone(), mk("b.zip", true)], &p));
        assert!(private_file(&[listed.clone(), mk("A.zip", true)], &p), "same file, whatever the letter case");
    }

    /// Links from another of your PCs: a video downloads at once in the quality picked there
    /// (or this PC's default, from a Pako that picks it); from an older Pako it goes to Links,
    /// as before. A link page and a playlist go to Links; a file's link to the downloads.
    #[test]
    fn videos_from_a_device_download_at_once_in_their_quality() {
        let yt = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        assert_eq!(route(&add(yt, Some(720), true), 1080), Route::Video(720));
        assert_eq!(route(&add(yt, Some(0), true), 1080), Route::Video(0), "audio only");
        assert_eq!(route(&add(yt, None, true), 480), Route::Video(480), "a newer Pako, no quality said: this PC's default");
        assert_eq!(route(&add(yt, None, false), 1080), Route::Links, "an older Pako's video: Links, as before");
        assert_eq!(route(&add("https://vimeo.com/123456", Some(1080), true), 1080), Route::Video(1080));
        assert_eq!(route(&add("https://www.youtube.com/playlist?list=PL1", Some(720), true), 1080), Route::Links, "a playlist: picked there");
        assert_eq!(route(&add("https://exe.io/abc", Some(720), true), 1080), Route::Links, "a link page");
        assert_eq!(route(&add("https://example.com/files/b.zip", Some(720), true), 1080), Route::File);
        assert_eq!(route(&add("https://drive.google.com/file/d/abcdefghijk/view", None, false), 1080), Route::File);
        // This PC sends a quality only with a video's link, its default when none was picked.
        let l = |link: &str, q| LinkOpts { link: link.into(), quality: q, ..Default::default() };
        assert_eq!(with_quality(l(yt, None), 720).quality, Some(720));
        assert_eq!(with_quality(l(yt, Some(0)), 720).quality, Some(0));
        assert_eq!(with_quality(l("https://example.com/a.zip", Some(480)), 720).quality, None);
    }

    /// Direct starts with Pako only once it was used: a new install stays quiet, an upgrade
    /// with contacts (or any other trace of Direct) starts as before.
    #[test]
    fn starts_with_pako_only_once_used() {
        let dir = std::env::temp_dir().join(format!("pako-direct-used-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!used_before(&dir), "a new install");
        // Pako 0.1.2 made a key for everyone: that alone isn't use.
        std::fs::write(dir.join("direct_key.bin"), b"x").unwrap();
        std::fs::write(dir.join("contacts.json"), b"[]").unwrap();
        std::fs::write(dir.join("direct_history.json"), b"[]").unwrap();
        std::fs::write(dir.join("direct_profile.json"), br#"{"name":"  "}"#).unwrap();
        assert!(!used_before(&dir), "started by 0.1.2, never used");
        std::fs::write(dir.join("contacts.json"), br#"[{"id":"a","name":"Rado"}]"#).unwrap();
        assert!(used_before(&dir), "an upgrade with contacts");
        std::fs::write(dir.join("contacts.json"), b"[]").unwrap();
        std::fs::write(dir.join("direct_profile.json"), br#"{"name":"Zak"}"#).unwrap();
        assert!(used_before(&dir), "a name for Direct");
        std::fs::remove_file(dir.join("direct_profile.json")).unwrap();
        std::fs::write(dir.join("direct_history.json"), b"{damaged").unwrap();
        assert!(used_before(&dir), "a list that can't be read: start and deal with it as before");
        std::fs::remove_file(dir.join("direct_history.json")).unwrap();
        assert!(!used_before(&dir));
        std::fs::write(dir.join(USED_FLAG), b"1").unwrap();
        assert!(used_before(&dir), "the Direct page was opened once");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
