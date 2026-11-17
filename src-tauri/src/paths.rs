// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Where Pako keeps its data.
//!
//! The same Pako.exe runs two ways:
//! - Installed (Pako Setup put a `.pako-installed` file next to it): settings, the download list
//!   and the Google login live in %LOCALAPPDATA%\Pako, so uninstalling or reinstalling never
//!   touches them. Local, not Roaming: lists, cookies and caches don't belong in a roaming profile.
//! - Portable (no such file): they live in a `data` folder next to the .exe. If that folder can't
//!   be written (e.g. the exe was put in Program Files), %LOCALAPPDATA%\Pako is used instead.
//!
//! %LOCALAPPDATA%\Pako was formerly %LOCALAPPDATA%\ZDownloader, which is moved over on first start.
//! Debug builds use `<project>/data` so `cargo clean` never wipes it.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The file Pako Setup writes next to Pako.exe: this copy is installed, not portable.
pub const INSTALLED_MARKER: &str = ".pako-installed";

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

fn has_marker(dir: &Path) -> bool {
    dir.join(INSTALLED_MARKER).is_file()
}

/// This copy was installed with Pako Setup (Settings shows "Installed" rather than "Portable").
pub fn is_installed() -> bool {
    static INSTALLED: OnceLock<bool> = OnceLock::new();
    *INSTALLED.get_or_init(|| exe_dir().is_some_and(|d| has_marker(&d)))
}

/// Where the data folder goes.
#[derive(Debug, PartialEq)]
enum Place {
    /// `data` next to Pako.exe (portable).
    NextToExe,
    /// %LOCALAPPDATA%\Pako.
    LocalAppData,
}

/// Installed copies always use %LOCALAPPDATA%\Pako (and never create a `data` folder next to the
/// exe: `data_writable` isn't even asked). Portable copies use `data` next to the exe when it can
/// be written.
fn data_place(installed: bool, exe_dir_known: bool, data_writable: impl FnOnce() -> bool) -> Place {
    if installed || !exe_dir_known {
        return Place::LocalAppData;
    }
    if data_writable() { Place::NextToExe } else { Place::LocalAppData }
}

fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
}

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn data_dir() -> &'static PathBuf {
    DATA_DIR.get_or_init(|| {
        let dir = pick_data_dir();
        let _ = std::fs::create_dir_all(&dir);
        dir
    })
}

/// The data folder if it is already known (never works it out: safe from a panic hook, even
/// one that fires while `data_dir` is being worked out).
pub fn data_dir_known() -> Option<&'static PathBuf> {
    DATA_DIR.get()
}

/// Test setting: a separate data folder (PAKO_DATA_DIR; ZD_DATA_DIR, its name before the app
/// was called Pako, still works).
pub fn data_dir_override() -> Option<PathBuf> {
    std::env::var_os("PAKO_DATA_DIR").or_else(|| std::env::var_os("ZD_DATA_DIR")).map(PathBuf::from)
}

fn pick_data_dir() -> PathBuf {
    // Test runs can point to a separate folder.
    data_dir_override().unwrap_or_else(default_data_dir)
}

/// The data folder the window uses (ignores PAKO_DATA_DIR). The CLI test mode keeps its own
/// folder inside it.
#[cfg_attr(debug_assertions, allow(unreachable_code))]
pub fn default_data_dir() -> PathBuf {
    // Debug builds only (compiled out of a release, so the project's folder never ends up in a
    // published Pako.exe).
    #[cfg(debug_assertions)]
    return dev_data_dir();
    let dir = exe_dir();
    let installed = dir.as_deref().is_some_and(has_marker);
    let next_to_exe = dir.as_ref().map(|d| d.join("data"));
    match data_place(installed, next_to_exe.is_some(), || next_to_exe.as_ref().is_some_and(is_writable)) {
        Place::NextToExe => next_to_exe.unwrap_or_default(),
        Place::LocalAppData => {
            let base = local_app_data();
            migrate_old_name(&base.join("ZDownloader"), base.join("Pako"))
        }
    }
}

/// Debug builds: `<project>/data`, so `cargo clean` never wipes it.
#[cfg(debug_assertions)]
fn dev_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("data")
}

/// The data folder as it is now, creating and moving nothing (for the uninstall cleanup).
#[cfg_attr(debug_assertions, allow(unreachable_code))]
pub fn data_dir_as_is() -> PathBuf {
    if let Some(dir) = data_dir_override() {
        return dir;
    }
    #[cfg(debug_assertions)]
    return dev_data_dir();
    let dir = exe_dir();
    let installed = dir.as_deref().is_some_and(has_marker);
    let next_to_exe = dir.as_ref().map(|d| d.join("data"));
    match data_place(installed, next_to_exe.is_some(), || next_to_exe.as_ref().is_some_and(|d| d.is_dir())) {
        Place::NextToExe => next_to_exe.unwrap_or_default(),
        Place::LocalAppData => local_app_data().join("Pako"),
    }
}

/// The window's own data (WebView2's storage, cache and window preferences) is kept by Tauri in
/// %LOCALAPPDATA%\<identifier>. The identifier was "app.zdownloader" up to Pako 0.1.2.
pub const WEBVIEW_DIR: &str = "com.sookera.pako";
const OLD_WEBVIEW_DIR: &str = "app.zdownloader";

/// Left in a fresh window-data folder when the old one couldn't be moved into it (a file in
/// it still open): a later start puts the old one in its place and tries again.
const MOVE_PENDING: &str = ".move-pending";

/// At the very start, before any window: the window's data from Pako 0.1.2 and earlier moves
/// to the new identifier's folder. Tried for about 5 seconds (right after "Restart now", the
/// older Pako's WebView2 may still be closing, or Defender may hold a file). If it still can't
/// be moved, Pako starts with a fresh folder marked `MOVE_PENDING`, and the next start
/// replaces that fresh folder with the old one.
pub fn migrate_webview_data() {
    migrate_webview_data_in(&local_app_data(), 20, std::time::Duration::from_millis(250));
}

fn migrate_webview_data_in(base: &Path, tries: u32, wait: std::time::Duration) {
    let (old, new) = (base.join(OLD_WEBVIEW_DIR), base.join(WEBVIEW_DIR));
    let pending = new.join(MOVE_PENDING);
    sweep_set_aside(base);
    if !old.is_dir() {
        // Nothing (left) to move.
        let _ = std::fs::remove_file(&pending);
        return;
    }
    if new.exists() {
        // A new folder is never replaced, except the fresh one made after a move failed. Set
        // aside first: that fails while a file in it is open, and is tried again next time.
        if !pending.is_file() {
            return;
        }
        let aside = base.join(format!("{SET_ASIDE}{}", uuid::Uuid::new_v4().simple()));
        if std::fs::rename(&new, &aside).is_err() {
            return;
        }
        let _ = std::fs::remove_dir_all(&aside);
    }
    for attempt in 0..tries {
        if attempt > 0 {
            std::thread::sleep(wait);
        }
        if std::fs::rename(&old, &new).is_ok() {
            // (A folder an uninstall handed back may still carry the mark.)
            let _ = std::fs::remove_file(&pending);
            return;
        }
    }
    let _ = std::fs::create_dir_all(&new);
    let _ = std::fs::write(&pending, b"");
}

/// A fresh window-data folder set aside to be deleted: "com.sookera.pako.fresh-<id>".
const SET_ASIDE: &str = "com.sookera.pako.fresh-";

/// Deletes fresh folders set aside earlier that couldn't be deleted then.
fn sweep_set_aside(base: &Path) {
    for e in std::fs::read_dir(base).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy().starts_with(SET_ASIDE) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Uninstalling without deleting the data (or with Pako 0.1.2's uninstaller, which knows only
/// the old name): the window's data goes back under the old name when that's free. Pako
/// 0.1.2's uninstaller then deletes it when its box is ticked; otherwise the next Pako moves
/// it forward again.
pub fn hand_webview_data_back() {
    let base = local_app_data();
    let (old, new) = (base.join(OLD_WEBVIEW_DIR), base.join(WEBVIEW_DIR));
    if new.is_dir() && !old.exists() {
        let _ = std::fs::rename(&new, &old);
    }
}

/// Every folder an installed Pako keeps data in, under today's and earlier names: the
/// settings and lists (%LOCALAPPDATA%\Pako, formerly ZDownloader) and the window's own data
/// (today's identifier and Pako 0.1.2's).
pub fn installed_data_folders() -> Vec<PathBuf> {
    let base = local_app_data();
    let mut all: Vec<PathBuf> = ["Pako", "ZDownloader", WEBVIEW_DIR, OLD_WEBVIEW_DIR].iter().map(|n| base.join(n)).collect();
    all.extend(
        std::fs::read_dir(&base)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(SET_ASIDE))
            .map(|e| e.path()),
    );
    all
}

/// Pako's folder in Windows' temporary folder for this user (%TEMP%\Pako): the browser's
/// login for a video while yt-dlp runs (`video::jar_dir`) and "Record this video"'s
/// recordings until joined (`stream::recordings_dir`). A development build uses its own.
pub fn temp_folder() -> PathBuf {
    let name = if cfg!(debug_assertions) { "Pako-dev" } else { "Pako" };
    std::env::temp_dir().join(name)
}

/// The app used to be called ZDownloader. If only the old data folder exists, move it to the
/// new name so the download list, settings and Google login carry over. If the move fails
/// (e.g. a file in it is open), keep using the old folder rather than starting empty.
fn migrate_old_name(old_dir: &PathBuf, new_dir: PathBuf) -> PathBuf {
    if new_dir.exists() || !old_dir.is_dir() {
        return new_dir;
    }
    match std::fs::rename(old_dir, &new_dir) {
        Ok(()) => new_dir,
        Err(_) => old_dir.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        INSTALLED_MARKER, MOVE_PENDING, OLD_WEBVIEW_DIR, Place, WEBVIEW_DIR, data_place, has_marker, migrate_old_name,
        migrate_webview_data_in,
    };
    use std::time::Duration;

    #[test]
    fn webview_data_follows_the_identifier() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"].as_str(), Some(WEBVIEW_DIR));
        let root = std::env::temp_dir().join(format!("pako-webview-{}", uuid::Uuid::new_v4().simple()));
        let old = root.join(OLD_WEBVIEW_DIR);
        std::fs::create_dir_all(old.join("EBWebView")).unwrap();
        std::fs::write(old.join("EBWebView").join("Local State"), b"{}").unwrap();
        let quick = Duration::from_millis(1);
        migrate_webview_data_in(&root, 3, quick);
        assert!(root.join(WEBVIEW_DIR).join("EBWebView").join("Local State").exists());
        assert!(!old.exists());
        // An old folder that shows up later never replaces the new one.
        std::fs::create_dir_all(&old).unwrap();
        migrate_webview_data_in(&root, 3, quick);
        assert!(old.exists() && root.join(WEBVIEW_DIR).join("EBWebView").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The move fails while a file in the old folder is open: Pako starts with a fresh folder,
    /// marked, and a later start puts the old one in its place.
    #[test]
    fn a_failed_window_data_move_is_tried_again() {
        let root = std::env::temp_dir().join(format!("pako-webview-retry-{}", uuid::Uuid::new_v4().simple()));
        let (old, new) = (root.join(OLD_WEBVIEW_DIR), root.join(WEBVIEW_DIR));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("Preferences"), b"mine").unwrap();
        let quick = Duration::from_millis(1);
        {
            let _held = std::fs::File::open(old.join("Preferences")).unwrap();
            migrate_webview_data_in(&root, 2, quick);
        }
        assert!(old.join("Preferences").exists());
        assert!(new.join(MOVE_PENDING).is_file());
        // WebView2 fills the fresh folder meanwhile.
        std::fs::write(new.join("Fresh"), b"new").unwrap();
        migrate_webview_data_in(&root, 2, quick);
        assert_eq!(std::fs::read(new.join("Preferences")).unwrap(), b"mine");
        assert!(!new.join("Fresh").exists() && !new.join(MOVE_PENDING).exists() && !old.exists());
        // Only the old folder and the fresh one are ever in the base folder.
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn installed_copies_keep_data_in_local_app_data() {
        // Installed: %LOCALAPPDATA%\Pako, and the folder next to the exe is never tried (that
        // would create a `data` folder in the install folder).
        assert_eq!(data_place(true, true, || panic!("must not be asked")), Place::LocalAppData);
        // Portable: next to the exe when it can be written, else %LOCALAPPDATA%\Pako.
        assert_eq!(data_place(false, true, || true), Place::NextToExe);
        assert_eq!(data_place(false, true, || false), Place::LocalAppData);
        assert_eq!(data_place(false, false, || true), Place::LocalAppData);
    }

    #[test]
    fn spots_the_installed_marker() {
        let dir = std::env::temp_dir().join(format!("pako-marker-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!has_marker(&dir));
        // A folder with that name isn't the marker.
        std::fs::create_dir_all(dir.join(INSTALLED_MARKER)).unwrap();
        assert!(!has_marker(&dir));
        std::fs::remove_dir_all(dir.join(INSTALLED_MARKER)).unwrap();
        std::fs::write(dir.join(INSTALLED_MARKER), b"Installed by Pako Setup.").unwrap();
        assert!(has_marker(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn moves_old_folder_once_and_keeps_its_files() {
        let root = std::env::temp_dir().join(format!("pako-migrate-{}", uuid::Uuid::new_v4().simple()));
        let old_dir = root.join("ZDownloader");
        let new_dir = root.join("Pako");
        std::fs::create_dir_all(&old_dir).unwrap();
        std::fs::write(old_dir.join("downloads.json"), b"[]").unwrap();

        assert_eq!(migrate_old_name(&old_dir, new_dir.clone()), new_dir);
        assert!(new_dir.join("downloads.json").exists());
        assert!(!old_dir.exists());

        // An old folder that shows up later never replaces the new one.
        std::fs::create_dir_all(&old_dir).unwrap();
        assert_eq!(migrate_old_name(&old_dir, new_dir.clone()), new_dir);
        assert!(old_dir.exists());

        let _ = std::fs::remove_dir_all(&root);
    }
}

fn is_writable(dir: &PathBuf) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".write-test");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

pub fn default_download_dir() -> PathBuf {
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        let d = PathBuf::from(profile).join("Downloads");
        if d.exists() {
            return d;
        }
    }
    data_dir().join("Downloads")
}

/// Write a file safely: write a temp file, flush it to the disk, keep the previous version
/// as `.bak`, then put the new one in place. A power cut can't leave a half-written file.
/// Two saves of the same file at once take turns, each with its own temp file, so they can't
/// mix their bytes.
pub fn write_atomic(path: &PathBuf, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_TMP: AtomicU64 = AtomicU64::new(0);
    let lock = path_lock(path);
    let _turn = lock.lock().unwrap_or_else(|e| e.into_inner());
    let n = NEXT_TMP.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("{}-{n}.tmp", std::process::id()));
    let written = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if path.exists() {
        let _ = std::fs::copy(path, path.with_extension("bak"));
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// One lock per file for `write_atomic`.
fn path_lock(path: &PathBuf) -> std::sync::Arc<std::sync::Mutex<()>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut map = LOCKS.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    map.entry(path.clone()).or_default().clone()
}

/// A file saved in the background (the download list, the Links list). The caller makes the
/// bytes (under its own lock, so they're a true picture); the writing happens on a thread of
/// its own: the app's lock isn't held while the disk works, and no download is held up. Only
/// the newest version is written, and an older one never lands after a newer one.
pub struct Saver {
    path: PathBuf,
    state: std::sync::Mutex<SaverState>,
    changed: std::sync::Condvar,
}

#[derive(Default)]
struct SaverState {
    /// Waiting to be written: (number, bytes). A newer save replaces it.
    queued: Option<(u64, Vec<u8>)>,
    /// Number given to the last save asked.
    last: u64,
    /// Number of the last save written (or that failed: nothing waits for it forever).
    done: u64,
    thread: bool,
}

impl Saver {
    pub fn new(path: PathBuf) -> Self {
        Self { path, state: Default::default(), changed: Default::default() }
    }

    /// Asks to write `bytes`; returns the number to give `wait`.
    pub fn save(&'static self, bytes: Vec<u8>) -> u64 {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        st.last += 1;
        let n = st.last;
        st.queued = Some((n, bytes));
        if !st.thread {
            st.thread = true;
            let started = std::thread::Builder::new().name("pako-save".into()).spawn(move || self.run());
            if started.is_err() {
                // No thread: write here instead.
                st.thread = false;
                let job = st.queued.take();
                drop(st);
                if let Some((n, bytes)) = job {
                    let _ = write_atomic(&self.path, &bytes);
                    self.finished(n);
                }
                return n;
            }
        }
        drop(st);
        self.changed.notify_all();
        n
    }

    /// Waits (at most `limit`) until save number `n` is written.
    pub fn wait(&self, n: u64, limit: std::time::Duration) {
        let until = std::time::Instant::now() + limit;
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while st.done < n {
            let left = until.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return;
            }
            st = self.changed.wait_timeout(st, left).unwrap_or_else(|e| e.into_inner()).0;
        }
    }

    /// Waits (at most `limit`) for every save asked so far.
    pub fn wait_all(&self, limit: std::time::Duration) {
        let last = self.state.lock().unwrap_or_else(|e| e.into_inner()).last;
        self.wait(last, limit);
    }

    fn finished(&self, n: u64) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        st.done = st.done.max(n);
        drop(st);
        self.changed.notify_all();
    }

    fn run(&self) {
        loop {
            let (n, bytes) = {
                let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
                loop {
                    if let Some(job) = st.queued.take() {
                        break job;
                    }
                    st = self.changed.wait(st).unwrap_or_else(|e| e.into_inner());
                }
            };
            let _ = write_atomic(&self.path, &bytes);
            self.finished(n);
        }
    }
}

/// Reads a file, trying again for a moment while something else holds it (antivirus, a
/// backup tool at start-up). Ok(None) = there is no such file.
fn read_retry(path: &PathBuf) -> Result<Option<Vec<u8>>, ()> {
    for i in 0..6 {
        match std::fs::read(path) {
            Ok(b) => return Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) if i < 5 => std::thread::sleep(std::time::Duration::from_millis(400)),
            Err(_) => {}
        }
    }
    Err(())
}

fn read_bak<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
    std::fs::read(path.with_extension("bak")).ok().and_then(|b| serde_json::from_slice::<T>(&b).ok())
}

/// A free name beside `path` for a copy kept aside: "downloads.damaged-1731700000.json".
fn aside_path(path: &PathBuf, what: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut p = path.with_extension(format!("{what}-{stamp}.json"));
    let mut k = 2;
    while p.exists() && k < 1000 {
        p = path.with_extension(format!("{what}-{stamp}-{k}.json"));
        k += 1;
    }
    p
}

/// Reads a JSON file written by `write_atomic`. A damaged file falls back to its `.bak` copy and
/// is moved aside (never overwritten silently). Tells "there's no file yet" (`Ok(None)`) apart
/// from "a file is there but can't be read right now" (`Err`, e.g. locked by antivirus or a
/// backup tool at start-up), so the caller never saves over a real file it couldn't read.
pub fn read_json_checked<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<Option<T>, String> {
    match read_retry(path) {
        // Read once and parse those bytes (reading again could fail and look like "no file").
        Ok(Some(bytes)) => Ok(parse_or_bak(path, &bytes)),
        // A crash between "copy to .bak" and "rename into place" leaves only the .bak.
        Ok(None) => Ok(read_bak(path)),
        // Still locked: the .bak (at most a few seconds older) is the next best thing.
        Err(()) => match read_bak(path) {
            Some(v) => Ok(Some(v)),
            None => Err(format!("{} can't be read right now", path.display())),
        },
    }
}

fn parse_or_bak<T: serde::de::DeserializeOwned>(path: &PathBuf, bytes: &[u8]) -> Option<T> {
    if let Ok(v) = serde_json::from_slice::<T>(bytes) {
        return Some(v);
    }
    let from_bak = read_bak(path);
    let _ = std::fs::rename(path, aside_path(path, "damaged"));
    from_bak
}

/// A list read from disk (the downloads, the Links list).
pub struct ListRead<T> {
    pub items: Vec<T>,
    /// Some entries (or the whole file) couldn't be read: a copy of the file as it was is kept
    /// here, never written over. The entries that could be read are in `items`.
    pub kept_in: Option<PathBuf>,
    /// How many entries couldn't be read (0 when the whole file was damaged).
    pub unread: usize,
    /// The file itself was damaged or missing, and the items come from its .bak copy.
    pub from_backup: bool,
}

/// Reads a JSON list entry by entry: one entry this version can't read (e.g. written by a newer
/// Pako before an update was rolled back) doesn't cost the whole list. The file as it was is
/// copied aside, never written over. Err = the file is there but can't be read right now (the
/// caller must not save over it).
pub fn read_list_checked<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<ListRead<T>, String> {
    let none = || ListRead { items: Vec::new(), kept_in: None, unread: 0, from_backup: false };
    match read_retry(path) {
        Ok(Some(bytes)) => {
            if let Some((items, unread)) = parse_list::<T>(&bytes) {
                let kept_in = if unread > 0 { keep_copy(path) } else { None };
                return Ok(ListRead { items, kept_in, unread, from_backup: false });
            }
            // Not a list at all (damaged): move it aside and use the .bak copy.
            let aside = aside_path(path, "damaged");
            let moved = std::fs::rename(path, &aside).ok().map(|_| aside);
            let mut read = bak_list(path).unwrap_or_else(none);
            read.kept_in = moved.or(read.kept_in);
            Ok(read)
        }
        // A crash between "copy to .bak" and "rename into place" leaves only the .bak.
        Ok(None) => Ok(bak_list(path).unwrap_or_else(none)),
        // Still locked: only a whole .bak (at most a few seconds older) will do.
        Err(()) => match bak_list::<T>(path) {
            Some(r) if r.unread == 0 => Ok(r),
            _ => Err(format!("{} can't be read right now", path.display())),
        },
    }
}

/// (entries read, how many couldn't be). None = not a JSON list.
fn parse_list<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Option<(Vec<T>, usize)> {
    if let Ok(all) = serde_json::from_slice::<Vec<T>>(bytes) {
        return Some((all, 0));
    }
    let raw: Vec<serde_json::Value> = serde_json::from_slice(bytes).ok()?;
    let total = raw.len();
    let items: Vec<T> = raw.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect();
    let unread = total - items.len();
    Some((items, unread))
}

fn bak_list<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<ListRead<T>> {
    let bak = path.with_extension("bak");
    let bytes = std::fs::read(&bak).ok()?;
    let (items, unread) = parse_list::<T>(&bytes)?;
    let kept_in = if unread > 0 { keep_copy(&bak) } else { None };
    Some(ListRead { items, kept_in, unread, from_backup: true })
}

/// Copies a file aside under a new name ("downloads.unread-1731700000.json").
fn keep_copy(path: &PathBuf) -> Option<PathBuf> {
    let base = if path.extension().is_some_and(|e| e == "bak") { path.with_extension("json") } else { path.clone() };
    let dest = aside_path(&base, "unread");
    std::fs::copy(path, &dest).ok().map(|_| dest)
}

#[cfg(test)]
mod save_tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("pako-paths-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Item {
        id: u32,
        kind: Kind,
    }
    #[derive(serde::Deserialize, Debug, PartialEq)]
    enum Kind {
        Old,
    }

    #[test]
    fn one_unreadable_entry_keeps_the_rest() {
        let dir = temp_dir();
        let path = dir.join("downloads.json");
        let text = r#"[{"id":1,"kind":"Old"},{"id":2,"kind":"FromANewerPako"},{"id":3,"kind":"Old"}]"#;
        std::fs::write(&path, text).unwrap();
        let r = read_list_checked::<Item>(&path).unwrap();
        assert_eq!(r.items.iter().map(|i| i.id).collect::<Vec<_>>(), vec![1, 3]);
        assert_eq!(r.unread, 1);
        // The whole file, as it was, is kept aside; the list itself is left in place.
        let kept = r.kept_in.unwrap();
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), text);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        // A second read keeps a second copy, never writes over the first.
        let again = read_list_checked::<Item>(&path).unwrap();
        assert_ne!(again.kept_in.unwrap(), kept);
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), text);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn damaged_list_falls_back_to_bak_and_is_kept() {
        let dir = temp_dir();
        let path = dir.join("found.json");
        std::fs::write(&path, b"[{\"id\":1,").unwrap();
        std::fs::write(path.with_extension("bak"), br#"[{"id":7,"kind":"Old"}]"#).unwrap();
        let r = read_list_checked::<Item>(&path).unwrap();
        assert_eq!(r.items, vec![Item { id: 7, kind: Kind::Old }]);
        let kept = r.kept_in.unwrap();
        assert_eq!(std::fs::read(&kept).unwrap(), b"[{\"id\":1,");
        assert!(!path.exists());
        // No file at all = an empty list, not an error.
        let empty = read_list_checked::<Item>(&dir.join("none.json")).unwrap();
        assert!(empty.items.is_empty() && empty.kept_in.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saves_at_once_never_mix() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let a = vec![b'a'; 300_000];
        let b = vec![b'b'; 200_000];
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let path = path.clone();
                let bytes = if i % 2 == 0 { a.clone() } else { b.clone() };
                std::thread::spawn(move || {
                    for _ in 0..10 {
                        write_atomic(&path, &bytes).unwrap();
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        let got = std::fs::read(&path).unwrap();
        assert!(got == a || got == b);
        let bak = std::fs::read(path.with_extension("bak")).unwrap();
        assert!(bak == a || bak == b);
        // No temp file is left behind.
        let left = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(left, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saver_writes_the_newest_and_waits() {
        let dir = temp_dir();
        let saver: &'static Saver = Box::leak(Box::new(Saver::new(dir.join("list.json"))));
        let mut last = 0;
        for i in 0..200u32 {
            last = saver.save(i.to_string().into_bytes());
        }
        saver.wait(last, std::time::Duration::from_secs(10));
        assert_eq!(std::fs::read_to_string(dir.join("list.json")).unwrap(), "199");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
