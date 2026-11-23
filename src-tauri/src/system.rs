// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Small Windows helpers: free disk space, keeping the PC awake while downloading,
//! starting with Windows, pako:// links, finding the installed browsers, and remembering where
//! the window was.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{OnceLock, mpsc};

use serde::{Deserialize, Serialize};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---------- disk space ----------

/// Free space for this user on the drive holding `path` (the nearest folder that exists).
pub fn free_space(path: &Path) -> Option<u64> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut p: PathBuf = path.to_path_buf();
    while !p.exists() {
        p = p.parent()?.to_path_buf();
    }
    let w = wide(&p.to_string_lossy());
    let mut free: u64 = 0;
    // SAFETY: valid NUL-terminated path; the other outputs are optional (null).
    let ok = unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) };
    (ok != 0).then_some(free)
}

/// (free for this user, size of the drive) for the drive holding `path`.
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut p: PathBuf = path.to_path_buf();
    while !p.exists() {
        p = p.parent()?.to_path_buf();
    }
    let w = wide(&p.to_string_lossy());
    let (mut free, mut total): (u64, u64) = (0, 0);
    // SAFETY: valid NUL-terminated path; the last output is optional (null).
    let ok = unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) };
    (ok != 0).then_some((free, total))
}

/// "D:" for D:\Downloads\x, used to tell whether two downloads share a drive.
pub fn drive_of(path: &Path) -> String {
    match path.components().next() {
        Some(std::path::Component::Prefix(p)) => p.as_os_str().to_string_lossy().to_uppercase(),
        _ => String::new(),
    }
}

// ---------- keep the PC awake ----------

/// Windows ties "don't sleep" to the thread that asked, so one thread of our own holds it.
static AWAKE: OnceLock<mpsc::Sender<bool>> = OnceLock::new();
/// 0 = not asked yet, 1 = may sleep, 2 = kept awake.
static AWAKE_NOW: AtomicU8 = AtomicU8::new(0);

/// While downloads run, Windows doesn't go to sleep (the screen may still turn off).
pub fn keep_awake(on: bool) {
    let want = if on { 2 } else { 1 };
    if AWAKE_NOW.swap(want, Ordering::Relaxed) == want {
        return;
    }
    let tx = AWAKE.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<bool>();
        std::thread::spawn(move || {
            use windows_sys::Win32::System::Power::{ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState};
            for on in rx {
                // SAFETY: plain flags.
                unsafe { SetThreadExecutionState(if on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS }) };
            }
        });
        tx
    });
    let _ = tx.send(on);
}

// ---------- start with Windows ----------

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "Pako";
/// Started by Windows at sign-in, straight to the tray.
pub const TRAY_ARG: &str = "--tray";

/// Adds Pako to (or removes it from) the programs Windows starts at sign-in, for this user only.
pub fn set_autostart(on: bool, in_tray: bool) -> Result<(), String> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW};
    let key = wide(RUN_KEY);
    let name = wide(RUN_NAME);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if !on {
        // Only this Pako's own entry (or one whose Pako is gone): another copy (portable and
        // installed side by side) keeps starting with Windows when this one is set not to.
        let entry = reg_get_value(HKEY_CURRENT_USER, RUN_KEY, Some(RUN_NAME), 0);
        if may_remove_autostart(entry.as_deref(), &exe) {
            // SAFETY: valid NUL-terminated strings. "Not there" is fine.
            unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
        }
        return Ok(());
    }
    let mut cmd = format!("\"{}\"", exe.display());
    if in_tray {
        cmd.push(' ');
        cmd.push_str(TRAY_ARG);
    }
    let data = wide(&cmd);
    // SAFETY: data is a NUL-terminated UTF-16 string; size in bytes includes the NUL.
    let r = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            REG_SZ,
            data.as_ptr().cast(),
            (data.len() * 2) as u32,
        )
    };
    if r == 0 { Ok(()) } else { Err(tr!("Windows refused to add Pako to start-up (error {r}).", "Windows a refusé d’ajouter Pako au démarrage (erreur {r}).")) }
}

/// Windows' start-up entry may go when it starts this Pako, or a Pako that isn't there any more
/// (moved or deleted); never when it starts another copy that is still there.
fn may_remove_autostart(entry: Option<&str>, exe: &Path) -> bool {
    let Some(entry) = entry else { return false };
    if command_starts(entry, exe) {
        return true;
    }
    let target = entry.trim().strip_prefix('"').and_then(|r| r.split_once('"')).map(|(p, _)| PathBuf::from(p));
    target.is_none_or(|t| !t.exists())
}

/// Uninstalling: takes Pako out of the programs Windows starts at sign-in, if that entry starts
/// this copy (a portable Pako elsewhere keeps its own).
pub fn remove_autostart_if_mine(exe: &Path) {
    use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
    if reg_get_value(HKEY_CURRENT_USER, RUN_KEY, Some(RUN_NAME), 0).is_some_and(|c| command_starts(&c, exe)) {
        let _ = set_autostart(false, false);
    }
}

// ---------- pako:// links (the browser extension's "Open Pako" button) ----------

/// What a `pako://` link asks Pako to do. Only these; anything else is ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PakoLink {
    /// Open Pako and connect it to the browsers.
    Connect,
}

/// Reads a `pako://` link. Strict: the exact action, nothing after it.
pub fn parse_pako_link(s: &str) -> Option<PakoLink> {
    let s = s.trim();
    if s.len() > 64 {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    let rest = lower.strip_prefix("pako://")?;
    match rest.strip_suffix('/').unwrap_or(rest) {
        "connect" => Some(PakoLink::Connect),
        _ => None,
    }
}

/// The `pako://` link Pako was started with (Windows passes it as an argument).
pub fn pako_link_in(args: &[String]) -> Option<PakoLink> {
    args.iter().find_map(|a| parse_pako_link(a))
}

/// Sets one value under HKEY_CURRENT_USER (`name` None = the key's default value).
pub(crate) fn reg_set_hkcu(key: &str, name: Option<&str>, value: &str) -> Result<(), u32> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW};
    let k = wide(key);
    let n = name.map(wide);
    let data = wide(value);
    // SAFETY: NUL-terminated strings; a null value name = the key's default value. The key is
    // created when missing.
    let r = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            k.as_ptr(),
            n.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
            REG_SZ,
            data.as_ptr().cast(),
            (data.len() * 2) as u32,
        )
    };
    if r == 0 { Ok(()) } else { Err(r) }
}

const PROTOCOL_KEY: &str = r"Software\Classes\pako";

/// Lets the browser open Pako with a `pako://` link, for this Windows user only (no admin).
/// Done at every start, so it always points at this Pako (it may have moved).
pub fn register_url_protocol() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.display().to_string();
    let open = format!("\"{exe}\" \"%1\"");
    let icon = format!("\"{exe}\",0");
    let command = format!(r"{PROTOCOL_KEY}\shell\open\command");
    let icon_key = format!(r"{PROTOCOL_KEY}\DefaultIcon");
    [
        (PROTOCOL_KEY, None, "URL:Pako"),
        (PROTOCOL_KEY, Some("URL Protocol"), ""),
        (icon_key.as_str(), None, icon.as_str()),
        (command.as_str(), None, open.as_str()),
    ]
    .into_iter()
    .try_for_each(|(k, n, v)| reg_set_hkcu(k, n, v))
    .map_err(|r| tr!("Windows refused to register pako:// links (error {r}).", "Windows a refusé d’enregistrer les liens pako:// (erreur {r})."))
}

/// Uninstalling: removes pako:// links, if they open this copy (a portable Pako elsewhere
/// registers them again at its next start anyway).
pub fn remove_url_protocol_if_mine(exe: &Path) {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteTreeW};
    let command = format!(r"{PROTOCOL_KEY}\shell\open\command");
    if reg_get_value(HKEY_CURRENT_USER, &command, None, 0).is_some_and(|c| command_starts(&c, exe)) {
        let k = wide(PROTOCOL_KEY);
        // SAFETY: NUL-terminated string. "Not there" is fine.
        unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, k.as_ptr()) };
    }
}

// ---------- installed browsers (Settings › Browser extension › Add Pako to your browser) ----------

/// One browser Pako can add its extension to.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Browser {
    /// "chrome", "edge", "brave", "vivaldi", "opera", "operagx", "firefox", "zen", "librewolf",
    /// "waterfox" or "floorp".
    pub id: &'static str,
    pub name: &'static str,
    /// "chromium" (Load unpacked) or "firefox" (signed .xpi).
    pub family: &'static str,
    #[serde(skip)]
    pub exe: PathBuf,
}

struct Known {
    id: &'static str,
    name: &'static str,
    family: &'static str,
    /// Its name in Windows' "App Paths" list (None = don't look there).
    app_path: Option<&'static str>,
    /// The files that start it, best first. Opera: `launcher.exe` in its folder starts the newest
    /// version (`opera.exe` sits in a folder named after the version, which changes with updates).
    files: &'static [&'static str],
    /// Usual places, under a folder from the environment.
    folders: &'static [(&'static str, &'static str)],
}

const BROWSERS: &[Known] = &[
    Known {
        id: "chrome",
        name: "Google Chrome",
        family: "chromium",
        app_path: Some("chrome.exe"),
        files: &["chrome.exe"],
        folders: &[
            ("ProgramFiles", r"Google\Chrome\Application"),
            ("ProgramFiles(x86)", r"Google\Chrome\Application"),
            ("LOCALAPPDATA", r"Google\Chrome\Application"),
        ],
    },
    Known {
        id: "edge",
        name: "Microsoft Edge",
        family: "chromium",
        app_path: Some("msedge.exe"),
        files: &["msedge.exe"],
        folders: &[("ProgramFiles(x86)", r"Microsoft\Edge\Application"), ("ProgramFiles", r"Microsoft\Edge\Application")],
    },
    Known {
        id: "brave",
        name: "Brave",
        family: "chromium",
        app_path: Some("brave.exe"),
        files: &["brave.exe"],
        folders: &[
            ("ProgramFiles", r"BraveSoftware\Brave-Browser\Application"),
            ("ProgramFiles(x86)", r"BraveSoftware\Brave-Browser\Application"),
            ("LOCALAPPDATA", r"BraveSoftware\Brave-Browser\Application"),
        ],
    },
    Known {
        id: "vivaldi",
        name: "Vivaldi",
        family: "chromium",
        app_path: Some("vivaldi.exe"),
        files: &["vivaldi.exe"],
        folders: &[("LOCALAPPDATA", r"Vivaldi\Application"), ("ProgramFiles", r"Vivaldi\Application")],
    },
    Known {
        id: "opera",
        name: "Opera",
        family: "chromium",
        // Points at launcher.exe (or, on older installs, opera.exe).
        app_path: Some("opera.exe"),
        files: &["launcher.exe", "opera.exe"],
        folders: &[("LOCALAPPDATA", r"Programs\Opera"), ("ProgramFiles", "Opera"), ("ProgramFiles(x86)", "Opera")],
    },
    Known {
        id: "operagx",
        name: "Opera GX",
        family: "chromium",
        // Its own folders only: Windows' "opera.exe" entry may belong to Opera.
        app_path: None,
        files: &["launcher.exe"],
        folders: &[("LOCALAPPDATA", r"Programs\Opera GX"), ("ProgramFiles", "Opera GX"), ("ProgramFiles(x86)", "Opera GX")],
    },
    Known {
        id: "firefox",
        name: "Firefox",
        family: "firefox",
        app_path: Some("firefox.exe"),
        files: &["firefox.exe"],
        folders: &[
            ("ProgramFiles", "Mozilla Firefox"),
            ("ProgramFiles(x86)", "Mozilla Firefox"),
            ("LOCALAPPDATA", "Mozilla Firefox"),
        ],
    },
    // Firefox-based browsers: they take the same signed extension as Firefox. Their installers
    // (Mozilla's, renamed) put them in "<Program Files or LocalAppData>\<their name>".
    Known {
        id: "zen",
        name: "Zen",
        family: "firefox",
        app_path: Some("zen.exe"),
        files: &["zen.exe"],
        folders: &[
            ("ProgramFiles", "Zen Browser"),
            ("LOCALAPPDATA", "Zen Browser"),
            ("ProgramFiles", "Zen Twilight"),
            ("LOCALAPPDATA", "Zen Twilight"),
        ],
    },
    Known {
        id: "librewolf",
        name: "LibreWolf",
        family: "firefox",
        // Its own installer doesn't fill in App Paths; looked up anyway in case another one did.
        app_path: Some("librewolf.exe"),
        files: &["librewolf.exe"],
        folders: &[("ProgramFiles", "LibreWolf"), ("LOCALAPPDATA", "LibreWolf"), ("LOCALAPPDATA", r"Programs\LibreWolf")],
    },
    Known {
        id: "waterfox",
        name: "Waterfox",
        family: "firefox",
        app_path: Some("waterfox.exe"),
        files: &["waterfox.exe"],
        folders: &[("ProgramFiles", "Waterfox"), ("ProgramFiles(x86)", "Waterfox"), ("LOCALAPPDATA", "Waterfox")],
    },
    Known {
        id: "floorp",
        name: "Floorp",
        family: "firefox",
        app_path: Some("floorp.exe"),
        files: &["floorp.exe"],
        folders: &[
            ("ProgramFiles", "Ablaze Floorp"),
            ("ProgramFiles(x86)", "Ablaze Floorp"),
            ("LOCALAPPDATA", "Ablaze Floorp"),
            ("ProgramFiles", "Floorp"),
        ],
    },
];

/// A text value from the registry (expanded), or None.
fn reg_get(root: windows_sys::Win32::System::Registry::HKEY, key: &str, flags: u32) -> Option<String> {
    let s = reg_get_value(root, key, None, flags)?;
    let s = s.trim().trim_matches('"').to_string();
    (!s.is_empty()).then_some(s)
}

/// A text value from the registry as it is written (`name` None = the key's default value).
pub(crate) fn reg_get_value(root: windows_sys::Win32::System::Registry::HKEY, key: &str, name: Option<&str>, flags: u32) -> Option<String> {
    use windows_sys::Win32::System::Registry::{RRF_RT_REG_SZ, RegGetValueW};
    let k = wide(key);
    let n = name.map(wide);
    let mut buf = vec![0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    // SAFETY: NUL-terminated key and name; the buffer and its size in bytes match. A null value
    // name = the key's default value. REG_EXPAND_SZ values are expanded by Windows.
    let r = unsafe {
        RegGetValueW(
            root,
            k.as_ptr(),
            n.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
            RRF_RT_REG_SZ | flags,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if r != 0 {
        return None;
    }
    let n = (len as usize / 2).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..n]).trim_end_matches('\0').to_string())
}

/// A command written by Pako (`"C:\…\Pako.exe" --tray`, `"C:\…\Pako.exe" "%1"`) starts `exe`.
pub fn command_starts(command: &str, exe: &Path) -> bool {
    let quoted = format!("\"{}\"", exe.display()).to_lowercase();
    command.trim().to_lowercase().starts_with(&quoted)
}

/// Can `p` be this browser's program? One of its file names, and for Opera and Opera GX (both
/// started by a `launcher.exe`) the right one: Opera GX's is in a folder named "Opera GX".
#[cfg(test)]
fn fits(b: &Known, p: &Path) -> bool {
    fits_as(b, p, in_opera_gx(p))
}

fn in_opera_gx(p: &Path) -> bool {
    p.components().any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("Opera GX"))
}

/// `fits`, with whether `p` is known to be Opera GX's (`gx`).
fn fits_as(b: &Known, p: &Path, gx: bool) -> bool {
    let named = p.file_name().is_some_and(|n| b.files.iter().any(|f| n.to_string_lossy().eq_ignore_ascii_case(f)));
    named
        && match b.id {
            "opera" => !gx,
            "operagx" => gx,
            _ => true,
        }
}

/// Can a browser Windows lists (its key `name` under StartMenuInternet, its program `p`) be this
/// one? As `fits`; Opera GX is also known by its key ("Opera GXStable"), wherever it was put.
#[cfg(test)]
fn fits_registered(b: &Known, name: &str, p: &Path) -> bool {
    fits_as(b, p, in_opera_gx(p) || key_is_opera_gx(name))
}

fn key_is_opera_gx(name: &str) -> bool {
    name.replace(' ', "").to_ascii_lowercase().contains("operagx")
}

/// Is `p` Opera GX's: in a folder named "Opera GX", or in (or below) the folder of a program
/// Windows lists under an Opera GX key. Opera GX put in another folder ("D:\Games\OGX") can also
/// be what Windows' "App Paths" entry for opera.exe names: without this, it was listed as Opera
/// too.
fn is_opera_gx(p: &Path, registered: &[(String, PathBuf)]) -> bool {
    in_opera_gx(p) || registered.iter().any(|(name, q)| key_is_opera_gx(name) && q.parent().is_some_and(|dir| under(p, dir)))
}

/// Is `p` below `dir` (never a drive's own folder: too wide)? Windows ignores case.
fn under(p: &Path, dir: &Path) -> bool {
    let parts = |x: &Path| x.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect::<Vec<_>>();
    let (p, d) = (parts(p), parts(dir));
    d.len() >= 3 && p.len() > d.len() && p[..d.len()] == d[..]
}

/// The program a registered command starts: `"C:\…\launcher.exe" --single-argument %1` gives
/// `C:\…\launcher.exe`. Unquoted, a path with spaces ends at its ".exe". `%Name%` is replaced by
/// what `env` has for it (left as it is when nothing).
fn command_exe(command: &str, env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let c = expand_env(command.trim(), env);
    let c = c.trim();
    let path = if let Some(rest) = c.strip_prefix('"') {
        rest.split('"').next().unwrap_or_default()
    } else {
        let lower = c.to_ascii_lowercase(); // the same byte positions as `c`
        let mut from = 0;
        let mut end = None;
        while let Some(i) = lower[from..].find(".exe") {
            let j = from + i + 4;
            if c[j..].chars().next().is_none_or(char::is_whitespace) {
                end = Some(j);
                break;
            }
            from = j;
        }
        match end {
            Some(j) => &c[..j],
            None => c.split_whitespace().next().unwrap_or_default(),
        }
    };
    let path = path.trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// `%Name%` replaced by `env(Name)`; unknown names (and a lone `%`, as in `%1`) stay as they are.
fn expand_env(s: &str, env: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let value = after
            .find('%')
            .filter(|&j| j > 0 && !after[..j].contains(char::is_whitespace))
            .and_then(|j| env(&after[..j]).map(|v| (j, v)));
        match value {
            Some((j, v)) => {
                out.push_str(&v);
                rest = &after[j + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The names of a registry key's subkeys (`view`: KEY_WOW64_32KEY for the 32-bit one, or 0).
fn reg_subkeys(root: windows_sys::Win32::System::Registry::HKEY, key: &str, view: u32) -> Vec<String> {
    use windows_sys::Win32::System::Registry::{HKEY, KEY_READ, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW};
    let k = wide(key);
    let mut h: HKEY = std::ptr::null_mut();
    // SAFETY: NUL-terminated key name; `h` receives the opened key, closed below.
    if unsafe { RegOpenKeyExW(root, k.as_ptr(), 0, KEY_READ | view, &mut h) } != 0 {
        return Vec::new();
    }
    let mut names = Vec::new();
    // A key name is at most 255 characters; no PC has hundreds of browsers.
    for i in 0..512 {
        let mut buf = [0u16; 256];
        let mut len = buf.len() as u32;
        // SAFETY: `h` is open; the buffer and its length in characters match; the rest is optional.
        let r = unsafe {
            RegEnumKeyExW(
                h,
                i,
                buf.as_mut_ptr(),
                &mut len,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if r != 0 {
            break;
        }
        names.push(String::from_utf16_lossy(&buf[..(len as usize).min(buf.len())]));
    }
    // SAFETY: opened above, closed once.
    unsafe { RegCloseKey(h) };
    names
}

/// The browsers Windows lists (Settings › Apps › Default apps), for this user and for the PC:
/// each one's key name and program, from
/// `SOFTWARE\Clients\StartMenuInternet\<name>\shell\open\command`. They find a browser installed
/// outside its usual folders (on another drive, for example).
fn registered_browsers() -> Vec<(String, PathBuf)> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY, RRF_SUBKEY_WOW6432KEY};
    const KEY: &str = r"SOFTWARE\Clients\StartMenuInternet";
    let mut out = Vec::new();
    for (root, view, flags) in [(HKEY_CURRENT_USER, 0, 0), (HKEY_LOCAL_MACHINE, 0, 0), (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY, RRF_SUBKEY_WOW6432KEY)] {
        for name in reg_subkeys(root, KEY, view) {
            let command = reg_get_value(root, &format!(r"{KEY}\{name}\shell\open\command"), None, flags);
            if let Some(exe) = command.and_then(|c| command_exe(&c, |v| std::env::var(v).ok())) {
                out.push((name, exe));
            }
        }
    }
    out
}

/// Where a browser's program is: Windows' "App Paths" list first, then the browsers Windows
/// lists (`registered`), then the usual folders.
fn find_exe(b: &Known, registered: &[(String, PathBuf)]) -> Option<PathBuf> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_SUBKEY_WOW6432KEY};
    let key = b.app_path.map(|exe| format!(r"Software\Microsoft\Windows\CurrentVersion\App Paths\{exe}"));
    let app_paths = [(HKEY_CURRENT_USER, 0), (HKEY_LOCAL_MACHINE, 0), (HKEY_LOCAL_MACHINE, RRF_SUBKEY_WOW6432KEY)]
        .into_iter()
        .filter_map(|(root, flags)| reg_get(root, key.as_deref()?, flags))
        .map(PathBuf::from);
    let usual = b
        .folders
        .iter()
        .filter_map(|(env, sub)| std::env::var_os(env).map(|d| PathBuf::from(d).join(sub)))
        .flat_map(|dir| b.files.iter().map(move |f| dir.join(f)));
    pick_exe(b, app_paths, registered, usual, |p| p.is_file())
}

/// `find_exe` once the places are read: the first that is this browser's program and is there
/// (`exists`), from App Paths, then the browsers Windows lists, then the usual folders.
fn pick_exe(
    b: &Known,
    app_paths: impl IntoIterator<Item = PathBuf>,
    registered: &[(String, PathBuf)],
    usual: impl IntoIterator<Item = PathBuf>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let fits_here = |p: &PathBuf| fits_as(b, p, is_opera_gx(p, registered));
    let listed = registered
        .iter()
        .filter(|(name, p)| fits_as(b, p, is_opera_gx(p, registered) || key_is_opera_gx(name)))
        .map(|(_, p)| p.clone());
    app_paths.into_iter().filter(fits_here).chain(listed).chain(usual.into_iter().filter(fits_here)).find(|p| exists(p))
}

/// The browsers installed on this PC, in a fixed order.
pub fn installed_browsers() -> Vec<Browser> {
    let registered = registered_browsers();
    BROWSERS
        .iter()
        .filter_map(|b| find_exe(b, &registered).map(|exe| Browser { id: b.id, name: b.name, family: b.family, exe }))
        .collect()
}

/// The page listing a Chromium browser's extensions.
pub fn extensions_page(id: &str) -> &'static str {
    match id {
        "edge" => "edge://extensions",
        "brave" => "brave://extensions",
        "vivaldi" => "vivaldi://extensions",
        "opera" | "operagx" => "opera://extensions",
        _ => "chrome://extensions",
    }
}

/// Starts a browser with one argument (a page or a file). False = Windows couldn't start it.
pub fn launch_browser(exe: &Path, arg: &str) -> bool {
    std::process::Command::new(exe)
        .arg(arg)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

// ---------- the window's place ----------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct WindowPlace {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub maximized: bool,
}

fn place_path() -> PathBuf {
    crate::paths::data_dir().join("window.json")
}

pub fn load_place() -> Option<WindowPlace> {
    serde_json::from_str(&std::fs::read_to_string(place_path()).ok()?).ok()
}

pub fn save_place(p: &WindowPlace) {
    if let Ok(s) = serde_json::to_string(p) {
        let _ = std::fs::write(place_path(), s);
    }
}

// ---------- time, power ----------

/// Minutes after midnight, local time.
pub fn local_minutes() -> u32 {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: GetLocalTime fills the struct.
    let mut t = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    t.wHour as u32 * 60 + t.wMinute as u32
}

/// Is `now` inside start..end? Wraps past midnight (22:00–07:00). Same start and end = always.
pub fn in_window(now: u32, start: u32, end: u32) -> bool {
    if start == end {
        true
    } else if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

/// "22:00" from minutes after midnight.
pub fn hhmm(m: u32) -> String {
    format!("{:02}:{:02}", (m / 60) % 24, m % 60)
}

/// Puts the PC to sleep (not hibernate). False = Windows refused.
pub fn sleep_now() -> bool {
    use windows_sys::Win32::System::Power::SetSuspendState;
    // Sleeping needs the "shut down" right switched on for Pako (users have it, but it's off).
    enable_shutdown_right();
    // SAFETY: plain flags (no hibernate, no force, wake events allowed).
    unsafe { SetSuspendState(0, 0, 0) != 0 }
}

/// Switches on SeShutdownPrivilege for Pako's own process (needed by SetSuspendState).
fn enable_shutdown_right() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, LUID};
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES,
        TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    // SAFETY: the token handle is checked and closed; the structs are filled before use.
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut luid = LUID { LowPart: 0, HighPart: 0 };
        let mut ok = LookupPrivilegeValueW(std::ptr::null(), SE_SHUTDOWN_NAME, &mut luid) != 0;
        if ok {
            let tp = TOKEN_PRIVILEGES { PrivilegeCount: 1, Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }] };
            // It can "succeed" without the right being given (ERROR_NOT_ALL_ASSIGNED = 1300).
            ok = AdjustTokenPrivileges(token, 0, &tp, 0, std::ptr::null_mut(), std::ptr::null_mut()) != 0 && GetLastError() != 1300;
        }
        CloseHandle(token);
        ok
    }
}

/// Shuts Windows down (other apps get the usual chance to save).
pub fn shut_down_now() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("shutdown.exe")
        .args(["/s", "/t", "0"])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .is_ok()
}

// ---------- clipboard ----------

/// Changes every time anything is copied.
pub fn clipboard_seq() -> u32 {
    use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
    // SAFETY: no arguments.
    unsafe { GetClipboardSequenceNumber() }
}

/// The copied text, if it is text (at most 1 MB).
pub fn clipboard_text() -> Option<String> {
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    // SAFETY: the clipboard is opened, read through a locked handle (never past the size
    // Windows gives for it), and always closed again.
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let h = GetClipboardData(CF_UNICODETEXT);
        let mut out = None;
        if !h.is_null() {
            let max = (GlobalSize(h) / 2).min(512 * 1024);
            let p = GlobalLock(h) as *const u16;
            if !p.is_null() {
                let mut len = 0usize;
                while len < max && *p.add(len) != 0 {
                    len += 1;
                }
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, len)));
                GlobalUnlock(h);
            }
        }
        CloseClipboard();
        out
    }
}

/// The clipboard change made by Pako's own copy (so catching copied links skips it).
static OWN_COPY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn own_copy_seq() -> u32 {
    OWN_COPY.load(Ordering::Relaxed)
}

/// The text Pako copied last: the same text seen again on the clipboard (another app copied it
/// back, or the change number was missed) is still Pako's own.
static OWN_TEXT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub fn is_own_copy(text: &str) -> bool {
    OWN_TEXT.lock().unwrap().as_deref().is_some_and(|own| own.trim() == text.trim())
}

/// Puts text on the clipboard (e.g. a link that was just sent). Works with Pako hidden.
pub fn set_clipboard_text(text: &str) -> bool {
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    let w: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    // Remembered before it lands, so the clipboard watcher can't see it first.
    *OWN_TEXT.lock().unwrap() = Some(text.to_string());
    // SAFETY: the memory is allocated, filled and handed to the clipboard (which then owns it);
    // freed only if the clipboard didn't take it. The clipboard is always closed again.
    let ok = unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return false;
        }
        EmptyClipboard();
        let h = GlobalAlloc(GMEM_MOVEABLE, w.len() * 2);
        let mut ok = false;
        if !h.is_null() {
            let p = GlobalLock(h) as *mut u16;
            if !p.is_null() {
                std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
                GlobalUnlock(h);
                ok = !SetClipboardData(CF_UNICODETEXT, h).is_null();
            }
            if !ok {
                GlobalFree(h);
            }
        }
        CloseClipboard();
        ok
    };
    if ok {
        OWN_COPY.store(clipboard_seq(), Ordering::Relaxed);
    }
    ok
}

/// Is one of Pako's own windows in front? (Then a copy came from Pako itself.)
pub fn pako_in_front() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    // SAFETY: plain window queries.
    unsafe {
        let w = GetForegroundWindow();
        if w.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(w, &mut pid);
        pid == std::process::id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_start_up_off_leaves_another_copys_entry() {
        let dir = std::env::temp_dir().join(format!("pako-run-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::create_dir_all(dir.join("b")).unwrap();
        let me = dir.join("a").join("Pako.exe");
        let other = dir.join("b").join("Pako.exe");
        std::fs::write(&me, b"x").unwrap();
        std::fs::write(&other, b"x").unwrap();
        let q = |p: &Path, tray: bool| format!("\"{}\"{}", p.display(), if tray { " --tray" } else { "" });
        assert!(may_remove_autostart(Some(&q(&me, true)), &me), "this Pako's own entry");
        assert!(!may_remove_autostart(Some(&q(&other, false)), &me), "another copy that is there");
        assert!(may_remove_autostart(Some(&q(&dir.join("gone").join("Pako.exe"), false)), &me), "a Pako that is gone");
        assert!(!may_remove_autostart(None, &me));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn knows_which_copy_a_command_starts() {
        let exe = Path::new(r"C:\Users\me\AppData\Local\Programs\Pako\Pako.exe");
        assert!(command_starts(r#""C:\Users\me\AppData\Local\Programs\Pako\Pako.exe" --tray"#, exe));
        assert!(command_starts(r#""c:\users\ME\appdata\local\programs\pako\pako.exe" "%1""#, exe));
        // A portable copy elsewhere, or a copy in a folder whose name only starts the same.
        assert!(!command_starts(r#""D:\Tools\Pako\Pako.exe" --tray"#, exe));
        assert!(!command_starts(r#""C:\Users\me\AppData\Local\Programs\Pako\Pako.exe.old" --tray"#, exe));
        assert!(!command_starts("", exe));
    }

    #[test]
    fn schedule_windows() {
        // 22:00–07:00 wraps past midnight.
        assert!(in_window(23 * 60, 22 * 60, 7 * 60));
        assert!(in_window(3 * 60, 22 * 60, 7 * 60));
        assert!(!in_window(12 * 60, 22 * 60, 7 * 60));
        assert!(!in_window(7 * 60, 22 * 60, 7 * 60));
        // 09:00–17:00 doesn't.
        assert!(in_window(9 * 60, 9 * 60, 17 * 60));
        assert!(!in_window(17 * 60, 9 * 60, 17 * 60));
        // Same start and end: always.
        assert!(in_window(5, 600, 600));
        assert_eq!(hhmm(22 * 60 + 5), "22:05");
    }

    #[test]
    fn reads_pako_links_strictly() {
        assert_eq!(parse_pako_link("pako://connect"), Some(PakoLink::Connect));
        assert_eq!(parse_pako_link("PAKO://Connect/"), Some(PakoLink::Connect));
        assert_eq!(parse_pako_link(" pako://connect "), Some(PakoLink::Connect));
        for bad in [
            "",
            "pako://",
            "pako:connect",
            "pako:///connect",
            "pako://connect?x=1",
            "pako://connect#a",
            "pako://connect/x",
            "pako://connect//",
            "pako://download?url=https://e.org/a.zip",
            "pako://user@connect",
            "pako://connect:80",
            "https://connect",
            "connect",
            "--tray",
        ] {
            assert_eq!(parse_pako_link(bad), None, "{bad}");
        }
        assert_eq!(parse_pako_link(&format!("pako://{}", "c".repeat(80))), None);
        // Windows passes the link after the program's path.
        let args = [r"C:\Pako\Pako.exe".to_string(), "pako://connect".to_string()];
        assert_eq!(pako_link_in(&args), Some(PakoLink::Connect));
        assert_eq!(pako_link_in(&[r"C:\Pako\Pako.exe".to_string(), "--tray".to_string()]), None);
    }

    #[test]
    fn extensions_pages() {
        assert_eq!(extensions_page("edge"), "edge://extensions");
        assert_eq!(extensions_page("vivaldi"), "vivaldi://extensions");
        assert_eq!(extensions_page("chrome"), "chrome://extensions");
        assert_eq!(extensions_page("opera"), "opera://extensions");
        assert_eq!(extensions_page("operagx"), "opera://extensions");
    }

    fn known(id: &str) -> &'static Known {
        BROWSERS.iter().find(|b| b.id == id).unwrap()
    }

    #[test]
    fn browser_list() {
        let ids: Vec<_> = BROWSERS.iter().map(|b| b.id).collect();
        assert_eq!(ids, ["chrome", "edge", "brave", "vivaldi", "opera", "operagx", "firefox", "zen", "librewolf", "waterfox", "floorp"]);
        let opera = known("opera");
        assert_eq!((opera.name, opera.family, opera.app_path), ("Opera", "chromium", Some("opera.exe")));
        assert!(opera.folders.contains(&("LOCALAPPDATA", r"Programs\Opera")));
        let gx = known("operagx");
        assert_eq!((gx.name, gx.family, gx.app_path), ("Opera GX", "chromium", None));
        assert!(gx.folders.contains(&("LOCALAPPDATA", r"Programs\Opera GX")));
        // Every page the window names for a Chromium browser is the one Pako opens.
        for b in BROWSERS.iter().filter(|b| b.family == "chromium") {
            assert!(extensions_page(b.id).ends_with("://extensions"), "{}", b.id);
        }
    }

    #[test]
    fn opera_programs() {
        let opera = known("opera");
        let gx = known("operagx");
        let p = |s: &str| PathBuf::from(s);
        assert!(fits(opera, &p(r"C:\Users\me\AppData\Local\Programs\Opera\launcher.exe")));
        assert!(fits(opera, &p(r"C:\Program Files\Opera\120.0.5543.61\opera.exe")));
        // Windows' "opera.exe" entry written by Opera GX isn't Opera.
        assert!(!fits(opera, &p(r"C:\Users\me\AppData\Local\Programs\Opera GX\launcher.exe")));
        assert!(fits(gx, &p(r"C:\Users\me\AppData\Local\Programs\Opera GX\Launcher.exe")));
        assert!(!fits(gx, &p(r"C:\Users\me\AppData\Local\Programs\Opera GX\opera.exe")));
        assert!(!fits(opera, &p(r"C:\Opera\chrome.exe")));
        // The others: only their own program.
        assert!(fits(known("vivaldi"), &p(r"C:\x\Vivaldi\Application\vivaldi.exe")));
        assert!(!fits(known("vivaldi"), &p(r"C:\x\Vivaldi\Application\launcher.exe")));
    }

    #[test]
    fn firefox_based_browsers() {
        let p = |s: &str| PathBuf::from(s);
        for (id, name, exe, folder) in [
            ("zen", "Zen", "zen.exe", "Zen Browser"),
            ("librewolf", "LibreWolf", "librewolf.exe", "LibreWolf"),
            ("waterfox", "Waterfox", "waterfox.exe", "Waterfox"),
            ("floorp", "Floorp", "floorp.exe", "Ablaze Floorp"),
        ] {
            let b = known(id);
            assert_eq!((b.name, b.family, b.app_path), (name, "firefox", Some(exe)));
            assert!(b.folders.contains(&("ProgramFiles", folder)), "{id}");
            assert!(fits(b, &p(&format!(r"C:\Program Files\{folder}\{exe}"))), "{id}");
            assert!(fits(b, &p(&format!(r"C:\Program Files\{folder}\{}", exe.to_uppercase()))), "{id}");
            // Not Firefox's program, and Firefox doesn't take theirs.
            assert!(!fits(b, &p(r"C:\Program Files\Mozilla Firefox\firefox.exe")), "{id}");
            assert!(!fits(known("firefox"), &p(&format!(r"C:\Program Files\{folder}\{exe}"))), "{id}");
        }
        assert!(fits(known("zen"), &p(r"C:\Users\me\AppData\Local\Zen Twilight\zen.exe")));
    }

    #[test]
    fn registered_browser_commands() {
        let env = |v: &str| match v.to_ascii_lowercase().as_str() {
            "programfiles" => Some(r"C:\Program Files".to_string()),
            "localappdata" => Some(r"C:\Users\me\AppData\Local".to_string()),
            _ => None,
        };
        let exe = |c: &str| command_exe(c, env).map(|p| p.to_string_lossy().into_owned());
        let s = |x: &str| Some(x.to_string());
        // Quoted, with arguments or without.
        assert_eq!(
            exe(r#""C:\Program Files\Google\Chrome\Application\chrome.exe" --single-argument %1"#),
            s(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
        );
        assert_eq!(exe(r#""D:\Opera GX\launcher.exe""#), s(r"D:\Opera GX\launcher.exe"));
        assert_eq!(exe(r#"  "D:\Opera GX\launcher.exe"  "#), s(r"D:\Opera GX\launcher.exe"));
        // Not quoted, with spaces: up to ".exe".
        assert_eq!(exe(r#"C:\Program Files\Mozilla Firefox\firefox.exe -osint -url "%1""#), s(r"C:\Program Files\Mozilla Firefox\firefox.exe"));
        assert_eq!(exe(r"D:\My Apps\Opera GX\LAUNCHER.EXE"), s(r"D:\My Apps\Opera GX\LAUNCHER.EXE"));
        assert_eq!(exe(r"C:\Apps\Waterfox.exe.d\waterfox.exe --new"), s(r"C:\Apps\Waterfox.exe.d\waterfox.exe"));
        assert_eq!(exe(r"C:\Browsers\zen"), s(r"C:\Browsers\zen"));
        // Environment variables, quoted or not; one Pako doesn't know stays as it is.
        assert_eq!(exe(r"%ProgramFiles%\Waterfox\waterfox.exe"), s(r"C:\Program Files\Waterfox\waterfox.exe"));
        assert_eq!(
            exe(r#""%LOCALAPPDATA%\Programs\Opera GX\launcher.exe" --single-argument %1"#),
            s(r"C:\Users\me\AppData\Local\Programs\Opera GX\launcher.exe")
        );
        assert_eq!(exe(r#"%NOPE%\x.exe "%1""#), s(r"%NOPE%\x.exe"));
        assert_eq!(expand_env("100% sure %1 %", env), "100% sure %1 %");
        assert_eq!(expand_env("%ProgramFiles%%LOCALAPPDATA%", env), r"C:\Program FilesC:\Users\me\AppData\Local");
        // Nothing to start.
        assert_eq!(exe(""), None);
        assert_eq!(exe("   "), None);
        assert_eq!(exe(r#""""#), None);
    }

    #[test]
    fn registered_browsers_found_by_their_program() {
        let which = |name: &str, path: &str| -> Vec<&str> {
            let exe = command_exe(&format!("\"{path}\" --single-argument %1"), |_| None).unwrap();
            BROWSERS.iter().filter(|b| fits_registered(b, name, &exe)).map(|b| b.id).collect()
        };
        // Opera GX on another drive is Opera GX, not Opera: by its folder, or by its key.
        assert_eq!(which("Opera GXStable", r"D:\Opera GX\launcher.exe"), ["operagx"]);
        assert_eq!(which("OperaStable", r"D:\Opera GX\launcher.exe"), ["operagx"]);
        assert_eq!(which("Opera GXStable", r"D:\Games\OGX\launcher.exe"), ["operagx"]);
        assert_eq!(which("OperaStable", r"D:\Browsers\Opera\launcher.exe"), ["opera"]);
        // The others by their program's name; Firefox's forks aren't Firefox.
        assert_eq!(which("FIREFOX.EXE", r"E:\Mozilla Firefox\firefox.exe"), ["firefox"]);
        assert_eq!(which("LibreWolf", r"E:\Tools\LibreWolf\librewolf.exe"), ["librewolf"]);
        assert_eq!(which("Zen", r"E:\Zen Browser\zen.exe"), ["zen"]);
        assert_eq!(which("Google Chrome", r"E:\Chrome\Application\chrome.exe"), ["chrome"]);
        assert_eq!(which("Vivaldi.ABC", r"E:\Vivaldi\Application\vivaldi.exe"), ["vivaldi"]);
        assert!(which("IEXPLORE.EXE", r"C:\Program Files\Internet Explorer\iexplore.exe").is_empty());
        // fits() itself now tells Opera GX's launcher from Opera's too.
        assert!(!fits(known("operagx"), &PathBuf::from(r"C:\Users\me\AppData\Local\Programs\Opera\launcher.exe")));
        // Edge is listed under "Microsoft Edge".
        assert_eq!(which("Microsoft Edge", r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"), ["edge"]);
    }

    #[test]
    fn opera_gx_in_its_own_folder_is_never_listed_as_opera() {
        let p = |s: &str| PathBuf::from(s);
        // Opera GX put in D:\Games\OGX; Windows' App Paths entry for opera.exe names its program.
        let registered = vec![("Opera GXStable".to_string(), p(r"D:\Games\OGX\launcher.exe"))];
        let app_paths = [p(r"D:\Games\OGX\opera.exe")];
        let there = |_: &Path| true;
        let none: [PathBuf; 0] = [];
        assert_eq!(pick_exe(known("opera"), app_paths.clone(), &registered, none.clone(), there), None);
        assert_eq!(pick_exe(known("operagx"), app_paths.clone(), &registered, none.clone(), there), Some(p(r"D:\Games\OGX\launcher.exe")));
        // A version folder below it too (Opera keeps its opera.exe there).
        let deeper = [p(r"d:\games\ogx\120.0.5543.61\opera.exe")];
        assert_eq!(pick_exe(known("opera"), deeper, &registered, none.clone(), there), None);
        // Opera itself, next to it, is still Opera.
        let both = vec![registered[0].clone(), ("OperaStable".to_string(), p(r"D:\Games\Opera\launcher.exe"))];
        assert_eq!(pick_exe(known("opera"), app_paths.clone(), &both, none.clone(), there), Some(p(r"D:\Games\Opera\launcher.exe")));
        // A GX program at a drive's own folder doesn't make the whole drive Opera GX.
        let at_root = vec![("Opera GXStable".to_string(), p(r"D:\launcher.exe"))];
        assert_eq!(pick_exe(known("opera"), [p(r"D:\Opera\opera.exe")], &at_root, none.clone(), there), Some(p(r"D:\Opera\opera.exe")));
        // A listed program that isn't there is skipped; the usual folder is next.
        let gone = |q: &Path| !q.starts_with(r"D:\Games");
        assert_eq!(
            pick_exe(known("operagx"), none.clone(), &registered, [p(r"C:\Users\me\AppData\Local\Programs\Opera GX\launcher.exe")], gone),
            Some(p(r"C:\Users\me\AppData\Local\Programs\Opera GX\launcher.exe"))
        );
    }
}
