// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Pako Setup's side of things (the installed copy; the portable one never needs these).
//!
//! - `Pako.exe --uninstall-cleanup [--delete-data]`: run by the uninstaller before it removes
//!   Pako's files. Removes what Pako registered with Windows for this copy: the browser
//!   connection, pako:// links and start with Windows. No window. With `--delete-data` (the
//!   uninstaller's "Also delete Pako's settings and lists" box), it also deletes Pako's data.
//!   What changes from one version to the next is done here, not in the uninstaller: a Pako
//!   that updated itself keeps the uninstaller of the version first installed, but this exe is
//!   always the current one.
//! - The Windows Apps entry shows a version: Pako's own updates replace Pako.exe without the
//!   installer, so each start of an installed copy brings that number up to date.

use std::path::Path;

/// The argument the uninstaller starts Pako.exe with.
pub const UNINSTALL_CLEANUP: &str = "--uninstall-cleanup";
/// With it: also delete Pako's settings, lists and window data.
pub const DELETE_DATA: &str = "--delete-data";
/// Exit code: another Pako is open (a portable copy), so its data (which may be the same
/// folders) was kept. The uninstaller then keeps the rest too and says why.
pub const KEPT_FOR_OTHER_PAKO: i32 = 2;
/// Exit code: some of the data couldn't be deleted (the uninstaller tries again itself).
pub const NOT_ALL_DELETED: i32 = 3;

/// The uninstall entry Pako Setup writes (per user: HKEY_CURRENT_USER only).
const APPS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Pako";

/// `args`: the arguments after the exe (the first is `UNINSTALL_CLEANUP`).
pub fn uninstall_cleanup(args: &[String]) -> i32 {
    let Ok(exe) = std::env::current_exe() else { return 1 };
    crate::extension::unregister_if_mine(&crate::paths::data_dir_as_is());
    crate::system::remove_url_protocol_if_mine(&exe);
    crate::system::remove_autostart_if_mine(&exe);
    let other_pako = other_pako_open();
    if !other_pako {
        // A browser login a Pako that didn't close normally left in %TEMP%\Pako: no Pako may
        // start again to sweep it.
        remove_cookie_files_in(&crate::paths::temp_folder());
    }
    if args.iter().any(|a| a == DELETE_DATA) {
        if other_pako {
            return KEPT_FOR_OTHER_PAKO;
        }
        let mut all = true;
        // %TEMP%\Pako too: recordings not joined yet belong to the lists that go.
        let temp = crate::paths::temp_folder();
        for dir in crate::paths::installed_data_folders().into_iter().chain(std::iter::once(temp)) {
            if dir.exists() && std::fs::remove_dir_all(&dir).is_err() {
                all = false;
            }
        }
        return if all { 0 } else { NOT_ALL_DELETED };
    }
    // Data kept: the window's data goes back under Pako 0.1.2's name, so an uninstaller from
    // 0.1.2 (kept by a copy that updated itself) deletes it when its box is ticked.
    if !other_pako {
        crate::paths::hand_webview_data_back();
    }
    0
}

/// Deletes yt-dlp's cookie files ("yt-cookies-<id>.txt", as `video::cookie_file` names them)
/// in `dir`; nothing else there is touched.
fn remove_cookie_files_in(dir: &Path) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("yt-cookies-") && name.ends_with(".txt") {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Another Pako runs (a portable copy). The installed one was just closed by the uninstaller:
/// its lock goes with its process, so this waits a moment for that.
fn other_pako_open() -> bool {
    for _ in 0..12 {
        if !crate::updates::another_instance_running() {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    true
}

/// Same folder, whatever the case, quotes or a trailing backslash.
fn same_folder(a: &str, b: &Path) -> bool {
    let norm = |s: &str| s.trim().trim_matches('"').trim_end_matches(['\\', '/']).to_lowercase();
    norm(a) == norm(&b.to_string_lossy())
}

/// The Apps entry belongs to this copy (its InstallLocation is this exe's folder) and shows
/// another version.
fn needs_new_version(install_location: Option<&str>, shown: Option<&str>, exe_dir: &Path, current: &str) -> bool {
    install_location.is_some_and(|l| same_folder(l, exe_dir)) && shown.map(str::trim) != Some(current)
}

/// At start, installed copies only: Windows Apps shows the version that is really installed.
pub fn sync_apps_entry() {
    use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
    if !crate::paths::is_installed() {
        return;
    }
    let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else { return };
    let location = crate::system::reg_get_value(HKEY_CURRENT_USER, APPS_KEY, Some("InstallLocation"), 0);
    let shown = crate::system::reg_get_value(HKEY_CURRENT_USER, APPS_KEY, Some("DisplayVersion"), 0);
    let current = env!("CARGO_PKG_VERSION");
    if needs_new_version(location.as_deref(), shown.as_deref(), &dir, current) {
        let _ = crate::system::reg_set_hkcu(APPS_KEY, Some("DisplayVersion"), current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pako Setup names its Apps entry after the product name (Tauri's UNINSTKEY).
    #[test]
    fn apps_entry_follows_the_product_name() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let product = conf["productName"].as_str().unwrap();
        assert_eq!(APPS_KEY, format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{product}"));
    }

    /// Only yt-dlp's cookie files go: a recording's folder and other files stay.
    #[test]
    fn uninstall_removes_left_cookie_files_only() {
        let dir = std::env::temp_dir().join(format!("pako-uninstall-jars-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(dir.join("recordings").join("rec-1")).unwrap();
        std::fs::write(dir.join("yt-cookies-abc.txt"), b"# Netscape HTTP Cookie File\n").unwrap();
        std::fs::write(dir.join("other.txt"), b"x").unwrap();
        remove_cookie_files_in(&dir);
        assert!(!dir.join("yt-cookies-abc.txt").exists());
        assert!(dir.join("other.txt").exists() && dir.join("recordings").join("rec-1").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The cookie files' name, as `video::cookie_file` makes it.
    #[test]
    fn cookie_file_names_match_the_video_tool() {
        let src = include_str!("video.rs");
        assert!(src.contains(r#"const JAR_PREFIX: &str = "yt-cookies-";"#));
        assert!(src.contains(r#"format!("{JAR_PREFIX}{}.txt""#));
        // And the same temporary folder.
        let stream = include_str!("stream.rs");
        assert!(stream.contains(r#"if cfg!(debug_assertions) { "Pako-dev" } else { "Pako" }"#));
        assert!(src.contains(r#"if cfg!(debug_assertions) { "Pako-dev" } else { "Pako" }"#));
    }

    #[test]
    fn updates_only_this_copys_apps_entry() {
        let dir = Path::new(r"C:\Users\me\AppData\Local\Programs\Pako");
        // Pako Setup writes the folder in quotes.
        assert!(needs_new_version(Some(r#""C:\Users\me\AppData\Local\Programs\Pako""#), Some("0.1.1"), dir, "0.2.0"));
        assert!(needs_new_version(Some(r"c:\users\me\appdata\local\programs\pako\"), None, dir, "0.2.0"));
        // Already right.
        assert!(!needs_new_version(Some(r#""C:\Users\me\AppData\Local\Programs\Pako""#), Some("0.2.0"), dir, "0.2.0"));
        // Another install (or none): leave it alone.
        assert!(!needs_new_version(Some(r#""D:\Apps\Pako""#), Some("0.1.1"), dir, "0.2.0"));
        assert!(!needs_new_version(None, Some("0.1.1"), dir, "0.2.0"));
    }
}
