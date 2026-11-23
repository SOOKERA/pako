// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Pako's language: English or French. Asked, never guessed: Pako Setup's English / Français
//! choice (handed over with `--install-choices --lang=en|fr`), a portable Pako's first-start
//! setup, and Settings › Window › Look › Language later. English until one of them says French.
//!
//! The windows translate themselves (`src/lib/i18n.svelte.ts`); the words Rust shows
//! (notifications and their buttons, the tray menu, dialogs, messages) are written in both
//! languages where they are used: `tr!("Open Pako", "Ouvrir Pako")`, with `format!`'s
//! placeholders (`tr!("{n} files", "{n} fichiers")`). French typography (a non-breaking space
//! before : and inside « », a narrow one before ; ? !) is written in the French itself.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::engine::Engine;

static FRENCH: AtomicBool = AtomicBool::new(false);

// Tests run side by side: each test thread keeps its own language (English unless it says).
#[cfg(test)]
thread_local! {
    static TEST_FRENCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Pako speaks French right now?
pub fn is_fr() -> bool {
    #[cfg(test)]
    {
        TEST_FRENCH.with(|f| f.get())
    }
    #[cfg(not(test))]
    {
        FRENCH.load(Ordering::Relaxed)
    }
}

/// This test thread speaks French (`true`) or English.
#[cfg(test)]
pub fn test_french(on: bool) {
    TEST_FRENCH.with(|f| f.set(on));
}

/// One of the two known languages (anything else, from an older or edited file: English).
pub fn choice(saved: &str) -> &'static str {
    if saved.eq_ignore_ascii_case("fr") { "fr" } else { "en" }
}

/// The language Rust's words follow from now on.
pub fn apply(lang: &str) {
    FRENCH.store(choice(lang) == "fr", Ordering::Relaxed);
}

/// The words in the language of the moment: `tr!("English", "Français")`, or with `format!`'s
/// placeholders and arguments: `tr!("Sent to {}", "Envoyé à {}", name)`.
#[macro_export]
macro_rules! tr {
    ($en:literal, $fr:literal $(,)?) => {
        if $crate::i18n::is_fr() { format!($fr) } else { format!($en) }
    };
    ($en:literal, $fr:literal, $($arg:expr),+ $(,)?) => {
        if $crate::i18n::is_fr() { format!($fr, $($arg),+) } else { format!($en, $($arg),+) }
    };
}

/// The saved language, for the windows (each reads it once, then follows `language-changed`).
#[tauri::command]
pub fn language_get(engine: State<'_, Arc<Engine>>) -> String {
    choice(&engine.settings().language).into()
}

/// Changes the language everywhere: every window is told at once (each fades its words to the
/// new ones), Rust's own words follow, the tray menu is made again, and the choice is saved.
/// Settings' Save never changes it (an older copy can't put the previous one back).
#[tauri::command]
pub async fn language_set(app: AppHandle, lang: String) -> Result<(), String> {
    let lang = choice(&lang);
    apply(lang);
    let _ = app.emit("language-changed", lang);
    crate::tray::refresh_words(&app);
    tell_setup(lang);
    let engine = app.state::<Arc<Engine>>().inner().clone();
    let saved = tauri::async_runtime::spawn_blocking(move || engine.change_settings(|s| s.language = lang.into())).await;
    // Shown even if saving failed (it then lasts until Pako closes).
    match saved {
        Ok(r) => r.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

// ---------- Pako Setup and the uninstaller follow Pako's language ----------
//
// Pako Setup keeps the language it ran in as "Installer Language" (Windows' language number as
// text, the way NSIS's MUI writes it: 1033 English, 1036 French) under its key for Pako. The
// uninstaller speaks that language, and Setup opens in it over an installed Pako (installer/ui.nsh:
// PAKO_UN_LANG_INIT, PakoGuiInit). So that both follow a language changed in Pako later, the
// installed Pako writes it there too: when the language changes, and once at start if it differs.
// Only the copy Setup installed and its key points at (a portable Pako never writes it).

/// Pako Setup's key for Pako (installer.nsi: MANUPRODUCTKEY, Software\<publisher>\<product>).
const SETUP_KEY: &str = r"Software\Sookera\Pako";
const SETUP_VALUE: &str = "Installer Language";

/// The value for a language: 1036 for French, 1033 for English.
fn setup_lcid(lang: &str) -> &'static str {
    if choice(lang) == "fr" { "1036" } else { "1033" }
}

/// What to write, if anything: for the installed copy only (`installed_here`), and only when the
/// saved value says something else (also when it is missing: an older Pako Setup's install).
fn setup_value_to_write(lang: &str, saved: Option<&str>, installed_here: bool) -> Option<&'static str> {
    let want = setup_lcid(lang);
    (installed_here && saved.map(str::trim) != Some(want)).then_some(want)
}

/// The folder Setup's key says Pako is installed in is this Pako's own (the same folder, written
/// either way: case, a slash at the end).
fn same_folder(a: &std::path::Path, b: &str) -> bool {
    let norm = |s: &str| s.trim().trim_end_matches(['\\', '/']).replace('/', "\\").to_lowercase();
    !b.trim().is_empty() && norm(&a.to_string_lossy()) == norm(b)
}

/// Tells Pako Setup's key the language (see above). Never in tests (no registry writes there).
pub fn tell_setup(lang: &str) {
    #[cfg(not(test))]
    {
        use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW};
        if !crate::paths::is_installed() {
            return;
        }
        let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(std::path::Path::to_path_buf)) else { return };
        let here = crate::system::reg_get_value(HKEY_CURRENT_USER, SETUP_KEY, None, 0).is_some_and(|d| same_folder(&dir, &d));
        let saved = crate::system::reg_get_value(HKEY_CURRENT_USER, SETUP_KEY, Some(SETUP_VALUE), 0);
        let Some(value) = setup_value_to_write(lang, saved.as_deref(), here) else { return };
        let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (k, n, v) = (wide(SETUP_KEY), wide(SETUP_VALUE), wide(value));
        // SAFETY: NUL-terminated key, name and text; the size in bytes includes the NUL. The key
        // exists (Setup's own default value was just read from it).
        unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), REG_SZ, v.as_ptr().cast(), (v.len() * 2) as u32) };
    }
    #[cfg(test)]
    let _ = lang;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_follows_pakos_language() {
        assert_eq!(setup_lcid("fr"), "1036");
        assert_eq!(setup_lcid("en"), "1033");
        assert_eq!(setup_lcid("de"), "1033");
        // Only the installed copy, only when it differs (or is missing).
        assert_eq!(setup_value_to_write("fr", Some("1033"), true), Some("1036"));
        assert_eq!(setup_value_to_write("en", Some("1036"), true), Some("1033"));
        assert_eq!(setup_value_to_write("fr", None, true), Some("1036"));
        assert_eq!(setup_value_to_write("fr", Some("1036"), true), None);
        assert_eq!(setup_value_to_write("en", Some("1033"), true), None);
        assert_eq!(setup_value_to_write("fr", Some("1033"), false), None);
        assert_eq!(setup_value_to_write("fr", None, false), None);
        let dir = std::path::Path::new(r"C:\Users\x\AppData\Local\Pako");
        assert!(same_folder(dir, r"c:\users\x\appdata\local\pako\"));
        assert!(same_folder(dir, "C:/Users/x/AppData/Local/Pako"));
        assert!(!same_folder(dir, r"D:\Portable\Pako"));
        assert!(!same_folder(dir, ""));
    }

    #[test]
    fn only_french_is_french() {
        assert_eq!(choice("fr"), "fr");
        assert_eq!(choice("FR"), "fr");
        for other in ["en", "", "de", "system", "français"] {
            assert_eq!(choice(other), "en", "{other}");
        }
    }

    #[test]
    fn words_in_both_languages() {
        let n = 3;
        test_french(false);
        assert_eq!(tr!("Open Pako", "Ouvrir Pako"), "Open Pako");
        assert_eq!(tr!("{} files", "{} fichiers", n), "3 files");
        assert_eq!(tr!("{n} files", "{n} fichiers"), "3 files");
        test_french(true);
        assert_eq!(tr!("Open Pako", "Ouvrir Pako"), "Ouvrir Pako");
        assert_eq!(tr!("{} files", "{} fichiers", n), "3 fichiers");
        assert_eq!(tr!("{n} files", "{n} fichiers"), "3 fichiers");
        test_french(false);
    }

    /// The string literal at the start of `s` (after spaces), unescaped, and what follows it.
    fn literal(s: &str) -> Option<(String, &str)> {
        let s = s.trim_start();
        let rest = s.strip_prefix('"')?;
        let mut out = String::new();
        let mut chars = rest.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return Some((out, &rest[i + 1..])),
                '\\' => match chars.next()?.1 {
                    'n' => out.push('\n'),
                    'u' => {
                        let hex: String = chars.by_ref().map(|(_, c)| c).skip(1).take_while(|c| *c != '}').collect();
                        out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                    }
                    '\n' => {
                        // A line continuation: the next line's leading spaces are skipped.
                        while chars.clone().next().is_some_and(|(_, c)| c.is_whitespace()) {
                            chars.next();
                        }
                    }
                    other => out.push(other),
                },
                c => out.push(c),
            }
        }
        None
    }

    /// Every French literal in Pako's Rust: the second words of each `tr!` and of each `Said`
    /// pair, and the `_FR` words.
    fn french_literals() -> Vec<(String, String)> {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        let mut found = Vec::new();
        for f in files {
            let text = std::fs::read_to_string(&f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            // This file's own examples (and the macro's definition) aside.
            if name == "i18n.rs" {
                continue;
            }
            let mut push = |after: &str| {
                let Some((_, rest)) = literal(after) else { return };
                let Some(rest) = rest.trim_start().strip_prefix(',') else { return };
                if let Some((fr, _)) = literal(rest) {
                    found.push((name.clone(), fr));
                }
            };
            for (at, _) in text.match_indices("tr!(") {
                push(&text[at + 4..]);
            }
            for (at, m) in text.match_indices("Said = [") {
                push(&text[at + m.len()..]);
            }
            // `match (…, crate::i18n::is_fr()) {`: the arms ending in `true) =>` are French.
            for (at, _) in text.match_indices("crate::i18n::is_fr()) {") {
                for line in text[at..].lines().skip(1).take(12).take_while(|l| !l.trim_start().starts_with('}')) {
                    let Some((arm, words)) = line.split_once("=>") else { continue };
                    if arm.trim_end().ends_with("true)") {
                        if let Some(start) = words.find('"') {
                            if let Some((fr, _)) = literal(&words[start..]) {
                                found.push((name.clone(), fr));
                            }
                        }
                    }
                }
            }
            for (at, m) in text.match_indices("_FR: &str =") {
                if let Some((fr, _)) = literal(&text[at + m.len()..]) {
                    found.push((name.clone(), fr));
                }
            }
        }
        found
    }

    #[test]
    fn french_typography_everywhere() {
        let all = french_literals();
        assert!(all.len() > 300, "the French literals were found ({})", all.len());
        for (file, fr) in all {
            let bad = fr.contains('\'')
                || fr.contains("...")
                || [" :", " ;", " ?", " !", "« ", " »"].iter().any(|p| fr.contains(p));
            assert!(!bad, "French typography in {file}: {fr:?} (’ not ', … not ..., a no-break space before : and inside « », a narrow one before ; ? !)");
        }
    }
}
