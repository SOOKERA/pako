// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.

//! Which browsers have Pako's extension talking to Pako: the native host (started by the
//! browser) notes the browser, the edition and the date in `browsers_seen.json`, and Settings
//! shows "Connected" for them. Only that is kept, on this PC; nothing else from the browser.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One browser and edition the extension talked to Pako from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Seen {
    /// "chrome", "edge", "brave", "vivaldi", "opera", "operagx" or "firefox" (as `browsers_found`).
    pub browser: String,
    /// "full" or "light" (Pako Light, from a store).
    pub edition: String,
    /// The extension's version, when its messages say it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Unix seconds.
    pub last_seen: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SeenFile {
    #[serde(default)]
    browsers: Vec<Seen>,
}

const IDS: &[&str] = &["chrome", "edge", "brave", "vivaldi", "opera", "operagx", "firefox"];
const EDITIONS: &[&str] = &["full", "light"];
/// A busy page sends many messages: the file is written at most once a minute per browser.
const WRITE_EVERY: u64 = 60;
/// How many levels up the host looks for the browser (Chrome starts it through cmd.exe).
const LEVELS: usize = 3;

pub fn file() -> PathBuf {
    crate::paths::data_dir().join("browsers_seen.json")
}

/// The browser a program file is, by its name (`None`: not one Pako's extension works in).
pub fn browser_id(exe: &Path) -> Option<&'static str> {
    let name = exe.file_name()?.to_string_lossy().to_lowercase();
    Some(match name.as_str() {
        "chrome.exe" => "chrome",
        "msedge.exe" => "edge",
        "brave.exe" => "brave",
        "vivaldi.exe" => "vivaldi",
        "firefox.exe" => "firefox",
        "opera.exe" if exe.to_string_lossy().to_lowercase().contains("opera gx") => "operagx",
        "opera.exe" => "opera",
        _ => return None,
    })
}

/// Walks up from `pid` (at most `LEVELS` parents) to the first known browser.
fn find_browser(pid: u32, parent: impl Fn(u32) -> Option<u32>, image: impl Fn(u32) -> Option<PathBuf>) -> Option<&'static str> {
    let mut at = pid;
    for _ in 0..LEVELS {
        at = parent(at).filter(|p| *p != 0 && *p != at)?;
        if let Some(id) = image(at).as_deref().and_then(browser_id) {
            return Some(id);
        }
    }
    None
}

/// The browser that started this process (the native host), if it is one Pako knows.
pub fn browser_of_this_process() -> Option<&'static str> {
    let parents = parent_pids();
    find_browser(std::process::id(), |p| parents.iter().find(|(c, _)| *c == p).map(|(_, pp)| *pp), image_of)
}

/// (process, its parent) for every process running now.
fn parent_pids() -> Vec<(u32, u32)> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
    let mut out = Vec::new();
    // SAFETY: the snapshot handle is checked and closed; the entry's size is set before use.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return out;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut e);
        while ok != 0 {
            out.push((e.th32ProcessID, e.th32ParentProcessID));
            ok = Process32NextW(snap, &mut e);
        }
        CloseHandle(snap);
    }
    out
}

fn image_of(pid: u32) -> Option<PathBuf> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    // SAFETY: the handle is checked and closed.
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let path = crate::extension::process_image(h);
        CloseHandle(h);
        path
    }
}

/// A version as extensions write them ("1.4.2"); anything else is left out.
fn clean_version(v: &str) -> Option<String> {
    let ok = !v.is_empty() && v.len() <= 32 && v.chars().all(|c| c.is_ascii_digit() || c == '.');
    ok.then(|| v.to_string())
}

/// The extension's version in a message from the browser, if it says one.
pub fn version_in(message: &[u8]) -> Option<String> {
    #[derive(Deserialize)]
    struct V {
        version: Option<String>,
    }
    serde_json::from_slice::<V>(message).ok()?.version.as_deref().and_then(clean_version)
}

/// The list with this browser seen `now`, or `None` when nothing needs writing (seen less than
/// a minute ago, same version). Only known browsers and editions, one entry each.
fn updated(mut list: Vec<Seen>, browser: &str, edition: &str, version: Option<String>, now: u64) -> Option<Vec<Seen>> {
    list.retain(|s| IDS.contains(&s.browser.as_str()) && EDITIONS.contains(&s.edition.as_str()));
    let mut kept: Vec<Seen> = Vec::new();
    for s in list {
        if !kept.iter().any(|k| k.browser == s.browser && k.edition == s.edition) {
            kept.push(s);
        }
    }
    match kept.iter_mut().find(|s| s.browser == browser && s.edition == edition) {
        Some(s) => {
            let new_version = version.is_some() && version != s.version;
            if !new_version && now.saturating_sub(s.last_seen) < WRITE_EVERY && now >= s.last_seen {
                return None;
            }
            s.last_seen = now;
            if version.is_some() {
                s.version = version;
            }
        }
        None => kept.push(Seen { browser: browser.into(), edition: edition.into(), version, last_seen: now }),
    }
    Some(kept)
}

/// The browsers seen, as written (none when there's no file or it can't be read).
pub fn read(path: &PathBuf) -> Vec<Seen> {
    let list = crate::paths::read_json_checked::<SeenFile>(path).ok().flatten().unwrap_or_default().browsers;
    list.into_iter().filter(|s| IDS.contains(&s.browser.as_str()) && EDITIONS.contains(&s.edition.as_str())).collect()
}

/// Notes that `browser` talked to Pako (from the native host). Skipped when the file is there
/// but can't be read right now (it's never written over then).
pub fn record(path: &PathBuf, browser: &str, light: bool, version: Option<String>, now: u64) {
    let Ok(file) = crate::paths::read_json_checked::<SeenFile>(path) else { return };
    let edition = if light { "light" } else { "full" };
    if let Some(browsers) = updated(file.unwrap_or_default().browsers, browser, edition, version, now) {
        if let Ok(bytes) = serde_json::to_vec_pretty(&SeenFile { browsers }) {
            let _ = crate::paths::write_atomic(path, &bytes);
        }
    }
}

/// The host's own memory: a host kept open by the browser doesn't even read the file more than
/// once a minute.
#[derive(Default)]
pub struct HostNote {
    browser: Option<Option<&'static str>>,
    last: Option<u64>,
}

impl HostNote {
    /// `version`: the extension's version, when the message said it (`version_in`).
    pub fn note(&mut self, light: bool, version: Option<String>, now: u64) {
        let browser = *self.browser.get_or_insert_with(browser_of_this_process);
        let Some(browser) = browser else { return };
        if version.is_none() && self.last.is_some_and(|t| now >= t && now - t < WRITE_EVERY) {
            return;
        }
        self.last = Some(now);
        record(&file(), browser, light, version, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_browsers_by_their_program() {
        let id = |p: &str| browser_id(Path::new(p));
        assert_eq!(id(r"C:\Program Files\Google\Chrome\Application\chrome.exe"), Some("chrome"));
        assert_eq!(id(r"C:\Program Files (x86)\Microsoft\Edge\Application\MSEDGE.EXE"), Some("edge"));
        assert_eq!(id(r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"), Some("brave"));
        assert_eq!(id(r"C:\Users\a\AppData\Local\Vivaldi\Application\vivaldi.exe"), Some("vivaldi"));
        assert_eq!(id(r"C:\Program Files\Mozilla Firefox\firefox.exe"), Some("firefox"));
        assert_eq!(id(r"C:\Users\a\AppData\Local\Programs\Opera GX\opera.exe"), Some("operagx"));
        assert_eq!(id(r"C:\Users\a\AppData\Local\Programs\Opera GX\120.0\opera.exe"), Some("operagx"));
        assert_eq!(id(r"C:\Users\a\AppData\Local\Programs\Opera\opera.exe"), Some("opera"));
        assert_eq!(id(r"C:\Windows\System32\cmd.exe"), None);
        assert_eq!(id(r"C:\Windows\explorer.exe"), None);
        assert_eq!(id(r"C:\Tools\notchrome.exe"), None);
    }

    #[test]
    fn walks_up_past_helpers_at_most_three_levels() {
        // 10 = the host, 9 = cmd.exe, 8 = chrome.exe.
        let tree = |chain: &'static [(u32, u32, &'static str)]| {
            (
                move |p: u32| chain.iter().find(|(c, _, _)| *c == p).map(|(_, pp, _)| *pp),
                move |p: u32| chain.iter().find(|(c, _, _)| *c == p).map(|(_, _, exe)| PathBuf::from(*exe)),
            )
        };
        let (parent, image) = tree(&[(10, 9, "pako.exe"), (9, 8, r"C:\Windows\cmd.exe"), (8, 1, r"C:\x\chrome.exe")]);
        assert_eq!(find_browser(10, parent, image), Some("chrome"));
        let (parent, image) = tree(&[(10, 9, "pako.exe"), (9, 1, r"C:\x\firefox.exe")]);
        assert_eq!(find_browser(10, parent, image), Some("firefox"));
        // Four levels up: too far.
        let (parent, image) = tree(&[(10, 9, "pako.exe"), (9, 8, "a.exe"), (8, 7, "b.exe"), (7, 6, "c.exe"), (6, 1, "chrome.exe")]);
        assert_eq!(find_browser(10, parent, image), None);
        // Started by hand: no browser.
        let (parent, image) = tree(&[(10, 9, "pako.exe"), (9, 0, r"C:\Windows\explorer.exe")]);
        assert_eq!(find_browser(10, parent, image), None);
    }

    #[test]
    fn reads_versions_only_when_they_look_like_one() {
        assert_eq!(version_in(br#"{"kind":"ping","version":"1.4.2"}"#), Some("1.4.2".into()));
        assert_eq!(version_in(br#"{"kind":"ping"}"#), None);
        assert_eq!(version_in(br#"{"kind":"ping","version":"<b>x</b>"}"#), None);
        assert_eq!(version_in(br#"{"kind":"ping","version":3}"#), None);
        assert_eq!(version_in(b"not json"), None);
    }

    #[test]
    fn writes_at_most_once_a_minute_per_browser() {
        let first = updated(vec![], "chrome", "full", None, 1000).unwrap();
        assert_eq!(first, vec![Seen { browser: "chrome".into(), edition: "full".into(), version: None, last_seen: 1000 }]);
        // 30 s later: nothing to write.
        assert!(updated(first.clone(), "chrome", "full", None, 1030).is_none());
        // Another browser or edition is written right away.
        assert_eq!(updated(first.clone(), "edge", "full", None, 1030).unwrap().len(), 2);
        assert_eq!(updated(first.clone(), "chrome", "light", None, 1030).unwrap().len(), 2);
        // A new version is written right away; no version keeps the one known.
        let v = updated(first.clone(), "chrome", "full", Some("1.2.0".into()), 1030).unwrap();
        assert_eq!(v[0].version.as_deref(), Some("1.2.0"));
        let later = updated(v, "chrome", "full", None, 1100).unwrap();
        assert_eq!((later[0].last_seen, later[0].version.as_deref()), (1100, Some("1.2.0")));
        // A clock set back doesn't freeze it.
        assert_eq!(updated(first, "chrome", "full", None, 500).unwrap()[0].last_seen, 500);
    }

    #[test]
    fn stays_small_and_drops_what_it_doesnt_know() {
        let junk = vec![
            Seen { browser: "chrome".into(), edition: "full".into(), version: None, last_seen: 1 },
            Seen { browser: "chrome".into(), edition: "full".into(), version: None, last_seen: 2 },
            Seen { browser: "netscape".into(), edition: "full".into(), version: None, last_seen: 3 },
            Seen { browser: "edge".into(), edition: "gold".into(), version: None, last_seen: 4 },
        ];
        let list = updated(junk, "firefox", "full", None, 5000).unwrap();
        assert_eq!(list.iter().map(|s| s.browser.as_str()).collect::<Vec<_>>(), ["chrome", "firefox"]);
        let mut all = vec![];
        for id in IDS {
            for ed in EDITIONS {
                all = updated(all, id, ed, None, 9000).unwrap();
            }
        }
        assert_eq!(all.len(), IDS.len() * EDITIONS.len());
        assert!(updated(all, "chrome", "full", None, 9001).is_none());
    }

    #[test]
    fn file_round_trip_and_old_or_missing_files() {
        let dir = std::env::temp_dir().join(format!("pako-seen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("browsers_seen.json");
        // No file yet.
        assert!(read(&path).is_empty());
        record(&path, "edge", true, Some("1.0.3".into()), 1_800_000_000);
        record(&path, "chrome", false, None, 1_800_000_010);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(r#""browser": "edge""#) && text.contains(r#""edition": "light""#) && text.contains(r#""lastSeen": 1800000000"#));
        assert_eq!(read(&path).len(), 2);
        // Throttled: same minute, nothing changes.
        record(&path, "chrome", false, None, 1_800_000_020);
        assert_eq!(read(&path).iter().find(|s| s.browser == "chrome").unwrap().last_seen, 1_800_000_010);
        // An older or unknown shape: extra fields are ignored, unknown browsers left out.
        std::fs::write(&path, r#"{"browsers":[{"browser":"brave","edition":"full","lastSeen":5,"extra":1},{"browser":"ie","edition":"full","lastSeen":6}],"v":0}"#).unwrap();
        assert_eq!(read(&path).iter().map(|s| s.browser.as_str()).collect::<Vec<_>>(), ["brave"]);
        // A damaged file reads as empty (or its .bak), and the next note starts over.
        let _ = std::fs::remove_file(path.with_extension("bak"));
        std::fs::write(&path, b"{oops").unwrap();
        assert!(read(&path).is_empty());
        record(&path, "firefox", false, None, 1_800_000_100);
        assert_eq!(read(&path).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
