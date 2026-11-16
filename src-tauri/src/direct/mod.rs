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

mod node;
mod proto;
mod store;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
}

impl Direct {
    pub fn node(&self) -> Result<Arc<Node>, String> {
        self.node.lock().unwrap().clone().ok_or_else(|| {
            self.error.lock().unwrap().clone().unwrap_or_else(|| "Direct is still starting. Try again in a moment.".into())
        })
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

/// Starts Direct in the background: the window works meanwhile and shows it as starting.
pub fn start(app: &AppHandle) {
    app.manage(Direct::default());
    let app = app.clone();
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
