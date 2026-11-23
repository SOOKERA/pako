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
//! - `Pako.exe --install-choices --start-with-windows=0|1 --browsers=0|1 [--terms-accepted=1] [--lang=en|fr]`:
//!   run by Pako Setup's window once Pako is installed (never by a silent install). Saves Setup's
//!   choices in Pako's settings and does them now: Start with Windows, the browser connection,
//!   and the language (only when Setup sends it). No window.
//! - The Windows Apps entry shows a version: Pako's own updates replace Pako.exe without the
//!   installer, so each start of an installed copy brings that number up to date.

use std::path::Path;

/// The argument the uninstaller starts Pako.exe with.
pub const UNINSTALL_CLEANUP: &str = "--uninstall-cleanup";
/// With it: also delete Pako's settings, lists and window data.
pub const DELETE_DATA: &str = "--delete-data";
/// "Also delete Private mode's files": its own switch in the uninstaller. Without it, Private
/// mode's folder (its encrypted files and key) is kept even when the settings and lists go (an
/// uninstaller from before 1.3.0 only knows `--delete-data`: it never deletes them either).
pub const DELETE_PRIVATE: &str = "--delete-private";

/// Private mode's folder inside the data folder, and its readable temporary copies in %TEMP%\Pako
/// (a copy waiting to be saved back belongs to it).
const PRIVATE_DIRS: [&str; 2] = ["private", "private-open"];
/// Exit code: another Pako is open (a portable copy), so its data (which may be the same
/// folders) was kept. The uninstaller then keeps the rest too and says why.
pub const KEPT_FOR_OTHER_PAKO: i32 = 2;
/// Exit code: some of the data couldn't be deleted (the uninstaller tries again itself).
pub const NOT_ALL_DELETED: i32 = 3;

/// The argument Pako Setup starts Pako.exe with for its choices.
pub const INSTALL_CHOICES: &str = "--install-choices";

/// `--name=1` → Some(true), `--name=0` → Some(false), missing → None.
fn choice(args: &[String], name: &str) -> Option<bool> {
    let prefix = format!("{name}=");
    args.iter().find_map(|a| a.strip_prefix(&prefix)).map(|v| v.trim() == "1")
}

/// `--lang=en|fr` → the language picked in Pako Setup (sent only when it was picked there, or on
/// a first install, where English is the choice; missing → the saved one stays). Anything else
/// than "fr" is English.
fn language(args: &[String]) -> Option<&'static str> {
    args.iter().find_map(|a| a.strip_prefix("--lang=")).map(crate::i18n::choice)
}

/// `args`: the arguments after the exe (the first is `INSTALL_CHOICES`). 0 = all done; 1 = some
/// of it couldn't be done (Setup goes on: Settings can still change both).
pub fn install_choices(args: &[String]) -> i32 {
    let start = choice(args, "--start-with-windows");
    let browsers = choice(args, "--browsers");
    let lang = language(args);
    // Setup's window says "By installing (updating), you accept the terms of use": that counts,
    // so the first start doesn't ask again. An acceptance already saved is kept as it is.
    let terms = (choice(args, "--terms-accepted") == Some(true)).then(crate::terms::version).flatten();
    let mut in_tray = true;
    let saved = crate::engine::change_saved_settings(|s| {
        if let Some(date) = &terms {
            if crate::consent::needed(s) {
                s.terms_accepted = Some(date.clone());
            }
        }
        if let Some(v) = start {
            s.start_with_windows = v;
        }
        if let Some(v) = browsers {
            s.browser_connect = v;
        }
        if let Some(l) = lang {
            s.language = l.into();
        }
        s.installer_choices = true;
        in_tray = s.start_in_tray;
    });
    if saved.is_err() {
        return 1;
    }
    let mut code = 0;
    if let Some(on) = start {
        if crate::system::set_autostart(on, in_tray).is_err() {
            code = 1;
        }
    }
    match browsers {
        Some(true) => {
            if crate::extension::set_registered(true).is_err() {
                code = 1;
            }
        }
        // Only this copy's line to the browsers: a portable Pako's stays.
        Some(false) => crate::extension::unregister_if_mine(&crate::paths::data_dir_as_is()),
        None => {}
    }
    code
}

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
    let data = args.iter().any(|a| a == DELETE_DATA);
    let private = args.iter().any(|a| a == DELETE_PRIVATE);
    if data || private {
        if other_pako {
            return KEPT_FOR_OTHER_PAKO;
        }
        let mut all = true;
        // %TEMP%\Pako too: recordings not joined yet belong to the lists that go.
        let temp = crate::paths::temp_folder();
        for dir in crate::paths::installed_data_folders().into_iter().chain(std::iter::once(temp)) {
            if dir.exists() && !delete_data_in(&dir, data, private) {
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

/// Deletes what the uninstaller's switches ask for in one of Pako's folders: `data`, everything
/// but Private mode's folders; `private`, only those. Both: the whole folder. True when all of
/// it went.
fn delete_data_in(dir: &Path, data: bool, private: bool) -> bool {
    if data && private {
        return std::fs::remove_dir_all(dir).is_ok();
    }
    let mut all = true;
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_lowercase();
        let is_private = PRIVATE_DIRS.contains(&name.as_str());
        if is_private != private {
            continue;
        }
        let p = e.path();
        let gone = if p.is_dir() { std::fs::remove_dir_all(&p).is_ok() } else { std::fs::remove_file(&p).is_ok() };
        all &= gone;
    }
    // Nothing left in it (everything asked for went, and nothing was kept): the folder too.
    let _ = std::fs::remove_dir(dir);
    all
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
    /// "Also delete Pako's settings and lists" keeps Private mode's folder (its encrypted files
    /// and key); "Also delete Private mode's files" removes only that; both remove everything.
    #[test]
    fn the_uninstall_switches_keep_private_mode_apart() {
        let root = std::env::temp_dir().join(format!("pako-uninstall-{}", std::process::id()));
        let make = |dir: &std::path::Path| {
            let _ = std::fs::remove_dir_all(dir);
            std::fs::create_dir_all(dir.join("private").join("files")).unwrap();
            std::fs::create_dir_all(dir.join("private-open")).unwrap();
            std::fs::write(dir.join("private").join("files").join("a.pkv"), b"x").unwrap();
            std::fs::write(dir.join("settings.json"), b"{}").unwrap();
            std::fs::create_dir_all(dir.join("direct")).unwrap();
        };
        let d = root.join("data");
        make(&d);
        assert!(super::delete_data_in(&d, true, false));
        assert!(d.join("private").join("files").join("a.pkv").exists() && d.join("private-open").exists());
        assert!(!d.join("settings.json").exists() && !d.join("direct").exists());
        make(&d);
        assert!(super::delete_data_in(&d, false, true));
        assert!(!d.join("private").exists() && !d.join("private-open").exists());
        assert!(d.join("settings.json").exists());
        make(&d);
        assert!(super::delete_data_in(&d, true, true));
        assert!(!d.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reads_setups_choices() {
        let a: Vec<String> = ["--install-choices", "--start-with-windows=0", "--browsers=1"].map(String::from).to_vec();
        assert_eq!(super::choice(&a, "--start-with-windows"), Some(false));
        assert_eq!(super::choice(&a, "--browsers"), Some(true));
        assert_eq!(super::choice(&a, "--other"), None);
        assert_eq!(super::choice(&a, "--terms-accepted"), None, "silent installs: not accepted");
        assert_eq!(super::language(&a), None, "no --lang: the saved language stays");
        let fr: Vec<String> = ["--install-choices", "--lang=fr"].map(String::from).to_vec();
        assert_eq!(super::language(&fr), Some("fr"));
        let en: Vec<String> = ["--install-choices", "--lang=en"].map(String::from).to_vec();
        assert_eq!(super::language(&en), Some("en"));
        let odd: Vec<String> = ["--install-choices", "--lang=de"].map(String::from).to_vec();
        assert_eq!(super::language(&odd), Some("en"), "anything but French is English");
    }

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
