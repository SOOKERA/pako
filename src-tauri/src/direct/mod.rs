// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Direct: Pako-to-Pako transfers, like a messaging app for files.
//!
//! Each Pako has its own key (its "Pako ID" is the public half). Contacts are other Pakos you
//! added; the first connection asks the other person once ("… wants to connect"), after that
//! known contacts connect by themselves while both Pakos run. Files go straight from one PC to
//! the other over QUIC (iroh): directly when the routers allow it, else through the library's
//! public relays, which only ever see encrypted bytes. Finding each other uses the library's
//! public address lookup. No Sookera server is involved. Contacts and history live only on
//! this PC (contacts.json, direct_history.json).
//!
//! Direct starts with Pako only once it has been used (a contact, a transfer, a name, or the
//! Direct page opened once: see `used_before`). Until then nothing of it runs: no relay
//! connection, no address published, no port asked from the router. Opening the Direct page
//! starts it (`Direct::wake`) and from then on it starts with Pako.

mod node;
mod proto;
mod store;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use iroh::endpoint::presets;
use iroh::{Endpoint, SecretKey};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

pub use node::{Node, State};
pub use proto::runs_code;
/// Also used by the problem list (errorlog.rs): a contact's Pako can send any text.
pub(crate) use proto::is_hidden;

/// How often contacts that aren't connected are tried again.
const PRESENCE_EVERY: Duration = Duration::from_secs(180);

/// Tauri state: the node once it's running, or why it isn't.
#[derive(Default)]
pub struct Direct {
    node: Mutex<Option<Arc<Node>>>,
    error: Mutex<Option<String>>,
    /// Starting has begun (it begins once).
    started: AtomicBool,
    /// Pako itself, to start Direct later (set by `start` / `start_if_used`).
    app: OnceLock<AppHandle>,
}

impl Direct {
    /// The running node. Never starts Direct by itself (the quit question, the tray and
    /// keep-awake ask here too): the Direct page does, with `wake`.
    pub fn node(&self) -> Result<Arc<Node>, String> {
        self.node.lock().unwrap().clone().ok_or_else(|| {
            if !self.started.load(Ordering::SeqCst) {
                return "Direct isn't running yet. Open the Direct page to start it.".to_string();
            }
            self.error.lock().unwrap().clone().unwrap_or_else(|| "Direct is still starting. Try again in a moment.".into())
        })
    }

    /// Starts Direct if it isn't running yet (the Direct page opened). Returns at once: the
    /// window gets "direct-changed" when it's ready. From then on Direct starts with Pako.
    pub fn wake(&self) {
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
    #[serde(flatten)]
    pub state: Option<State>,
}

pub fn view(d: &Direct) -> View {
    match d.node.lock().unwrap().clone() {
        Some(n) => View { ready: true, error: None, state: Some(n.state()) },
        None => View { ready: false, error: d.error.lock().unwrap().clone(), state: None },
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Profile {
    #[serde(default)]
    name: String,
}

fn profile_path() -> PathBuf {
    crate::paths::data_dir().join("direct_profile.json")
}

pub fn save_name(name: &str) {
    if let Ok(b) = serde_json::to_vec_pretty(&Profile { name: name.trim().chars().take(60).collect() }) {
        let _ = crate::paths::write_atomic(&profile_path(), &b);
    }
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
    if used_before(crate::paths::data_dir()) {
        app.state::<Direct>().wake();
    }
}

/// Starts Direct now, used before or not.
#[allow(dead_code)]
pub fn start(app: &AppHandle) {
    let direct = Direct::default();
    let _ = direct.app.set(app.clone());
    app.manage(direct);
    app.state::<Direct>().wake();
}

/// Starts Direct in the background: the window works meanwhile and shows it as starting.
fn begin(app: AppHandle) {
    // Used from now on: it starts with Pako (someone given this Pako ID can then reach it).
    let flag = crate::paths::data_dir().join(USED_FLAG);
    if !flag.exists() {
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
        let name = std::fs::read(profile_path()).ok().and_then(|b| serde_json::from_slice::<Profile>(&b).ok()).unwrap_or_default().name;
        let dl = app.clone();
        let downloads = Box::new(move || dl.state::<Arc<crate::engine::Engine>>().settings().download_dir.clone());
        let em = app.clone();
        let changed: node::Changed = Arc::new(move || {
            let _ = em.emit("direct-changed", ());
        });
        let na = app.clone();
        // Pako's window in front shows Direct's news itself: no pop-up then (the bell still
        // gets its kinds).
        let notify: node::Notify2 = Arc::new(move |kind: &str, title: &str, body: &str, go: crate::notices::Go| {
            if let Some(e) = na.try_state::<Arc<crate::engine::Engine>>() {
                let pop = if window_in_view(&na) { crate::notices::Pop::Skip } else { crate::notices::Pop::Pref };
                e.notify_pop(kind, title, body, go, pop);
            }
        });
        // A question settled anywhere: its pop-up leaves and its bell line says how it ended.
        let sa = app.clone();
        let settle: node::Settle = Arc::new(move |kind: &str, key: &str, text: &str| crate::notices::resolve(&sa, kind, key, text));
        let node = Node::new(ep, crate::paths::data_dir().clone(), name, downloads, changed, notify, settle);
        *app.state::<Direct>().node.lock().unwrap() = Some(node.clone());
        tokio::spawn(node.clone().accept_loop());
        // Wait to be online (relay chosen, address published) before calling contacts.
        let _ = tokio::time::timeout(Duration::from_secs(20), node.ep.online()).await;
        tokio::spawn(node.clone().presence_loop(PRESENCE_EVERY));
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

#[cfg(test)]
mod tests {
    use super::*;

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
