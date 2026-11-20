// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! File names: making them safe for Windows, reading them from links and server headers,
//! and avoiding overwriting files that already exist.

use std::collections::HashSet;
use std::path::Path;

/// What an unfinished download's file ends with ("file.zip.pakopart").
pub const PART_EXT: &str = "pakopart";
/// What it ended with up to Pako 0.1.2 ("file.zip.zdpart"): still recognised, so a download
/// left unfinished by an older Pako carries on (see `adopt_old_part`).
pub const OLD_PART_EXT: &str = "zdpart";

/// The unfinished file of the download named `name` in `dir`.
pub fn part_of(dir: &Path, name: &str) -> std::path::PathBuf {
    dir.join(format!("{name}.{PART_EXT}"))
}

/// Its name in an older Pako.
pub fn old_part_of(dir: &Path, name: &str) -> std::path::PathBuf {
    dir.join(format!("{name}.{OLD_PART_EXT}"))
}

/// What a file arriving through Direct ends with until it's complete ("file.zip.pakorecv").
/// Never the same as a download's: a download must never write into a file Direct is
/// receiving, nor Direct's clean-up delete a download's file. (Pako 0.1.2's Direct used
/// ".pakopart", today's download ending: see `direct::node`.)
pub const RECV_EXT: &str = "pakorecv";

/// Is there an unfinished download file for `name` in `dir`, under either name?
pub fn part_exists(dir: &Path, name: &str) -> bool {
    part_of(dir, name).exists() || old_part_of(dir, name).exists()
}

/// Is a file named `name` still arriving through Direct in `dir`?
pub fn receiving(dir: &Path, name: &str) -> bool {
    dir.join(format!("{name}.{RECV_EXT}")).exists()
}

/// An unfinished file from an older Pako (`old`) takes the new name (`new`), so the download
/// continues from its bytes. When both are there, the one written last is kept: its bytes go
/// with the parts saved last (an older Pako put back by a failed update wrote `old` after this
/// version wrote `new`; a file this version restarted after a failed rename is newer than the
/// `old` it left behind). Tried again for about 2 seconds if the file is briefly held
/// (antivirus). Best effort: if it still fails, the download lays its file out again.
pub fn adopt_old(old: &Path, new: &Path) {
    let Ok(old_meta) = std::fs::metadata(old) else { return };
    let keep_new = std::fs::metadata(new).is_ok_and(|n| match (n.modified(), old_meta.modified()) {
        (Ok(n), Ok(o)) => n >= o,
        _ => true,
    });
    for attempt in 0..8 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        // (`rename` replaces `new` on Windows.)
        let r = if keep_new { std::fs::remove_file(old) } else { std::fs::rename(old, new) };
        match r {
            Ok(()) => return,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(_) => {}
        }
    }
}

/// Renames, failing with AlreadyExists when `to` exists (`std::fs::rename` replaces it on
/// Windows): a file that appeared under the final name meanwhile is never written over.
#[cfg(windows)]
pub fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let wide = |p: &Path| p.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (a, b) = (wide(from), wide(to));
    // SAFETY: two valid NUL-terminated paths; no flags = fail if `to` exists, and never copy.
    let ok = unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), 0) };
    if ok != 0 {
        return Ok(());
    }
    let e = std::io::Error::last_os_error();
    // ERROR_ALREADY_EXISTS / ERROR_FILE_EXISTS
    if matches!(e.raw_os_error(), Some(183) | Some(80)) {
        return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
    }
    Err(e)
}

#[cfg(not(windows))]
pub fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
    }
    std::fs::rename(from, to)
}

/// Windows file names ignore upper/lower case: compare paths this way.
pub fn path_key(p: &Path) -> String {
    p.to_string_lossy().to_lowercase()
}

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "COM¹", "COM²", "COM³",
    "LPT¹", "LPT²", "LPT³", "CONIN$", "CONOUT$",
];

/// Endings Pako gives its own unfinished files (a download's, an older Pako's, a Direct
/// receive's). A file a server or a shared folder names this way could be taken for another
/// download's unfinished file (and written over, or deleted with it): it gets a "_" added.
const PART_ENDINGS: &[&str] = &[".pakopart", ".zdpart", ".pakorecv"];

/// Windows counts a name's length in UTF-16 units (an emoji is two): the limit Pako keeps to.
const MAX_UNITS: usize = 180;

/// Characters that flip the direction text is shown in: a server could make "invoice", a
/// right-to-left mark, then "fdp.exe" look like "invoiceexe.pdf". Left out of file names.
fn direction_mark(c: char) -> bool {
    matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

pub fn sanitize(name: &str) -> String {
    let mut s: String = name
        .chars()
        .filter(|c| !direction_mark(*c))
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect();
    s = s.trim().trim_end_matches(['.', ' ']).to_string();
    if s.is_empty() {
        s = "download".into();
    }
    let stem = s.split('.').next().unwrap_or("").trim_end().to_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        s = format!("_{s}");
    }
    let lower = s.to_lowercase();
    if PART_ENDINGS.iter().any(|e| lower.ends_with(e)) {
        s.push('_');
    }
    // Keep names a sensible length (Windows paths are limited), preserving the extension.
    if s.encode_utf16().count() > MAX_UNITS {
        let (base, ext) = split_ext(&s);
        let room = MAX_UNITS.saturating_sub(10 + ext.encode_utf16().count());
        let mut base = base.to_string();
        while base.encode_utf16().count() > room {
            base.pop();
        }
        // Cut before a dot or a space: Windows drops those at the end of a name, and the file
        // would then not be where Pako looks for it.
        s = format!("{base}{ext}").trim_end_matches(['.', ' ']).to_string();
        if s.is_empty() {
            s = "download".into();
        }
    }
    s
}

fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 && name.len() - i <= 10 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// Last part of the link's path, decoded. "https://x.com/a/file%20one.zip?x=1" -> "file one.zip"
pub fn from_url(link: &str) -> String {
    let name = url::Url::parse(link)
        .ok()
        .and_then(|u| u.path_segments().and_then(|mut s| s.next_back().map(String::from)))
        .filter(|s| !s.is_empty())
        .map(|s| percent_encoding::percent_decode_str(&s).decode_utf8_lossy().to_string())
        .unwrap_or_else(|| "download".into());
    sanitize(&name)
}

/// File name from a Content-Disposition header (supports filename*=UTF-8''...).
pub fn from_content_disposition(header: &str) -> Option<String> {
    let mut plain = None;
    for part in header.split(';').map(str::trim) {
        let lower = part.to_ascii_lowercase();
        if lower.starts_with("filename*=") {
            let v = &part["filename*=".len()..];
            let v = v.trim_matches('"');
            let encoded = v.splitn(3, '\'').nth(2).unwrap_or(v);
            let decoded = percent_encoding::percent_decode_str(encoded).decode_utf8_lossy().to_string();
            if !decoded.is_empty() {
                return Some(sanitize(&decoded));
            }
        } else if lower.starts_with("filename=") {
            let v = part["filename=".len()..].trim().trim_matches('"');
            if !v.is_empty() {
                plain = Some(sanitize(v));
            }
        }
    }
    plain
}

/// Final name when a download finishes: only checks finished files (its own unfinished file is expected).
pub fn unique_final(dir: &Path, name: &str, taken: &HashSet<String>) -> String {
    let free = |n: &str| {
        let p = dir.join(n);
        !p.exists() && !taken.contains(&path_key(&p))
    };
    if free(name) {
        return name.to_string();
    }
    let (base, ext) = split_ext(name);
    (1..10_000)
        .map(|i| format!("{base} ({i}){ext}"))
        .find(|c| free(c))
        .unwrap_or_else(|| format!("{base} ({}){ext}", uuid::Uuid::new_v4()))
}

/// "file.zip" -> "file (1).zip" if "file.zip" (or its unfinished file, .pakopart or an older
/// Pako's .zdpart, or a file of that name arriving through Direct) exists or is taken by
/// another download.
pub fn unique(dir: &Path, name: &str, taken: &HashSet<String>) -> String {
    let free = |n: &str| {
        let p = dir.join(n);
        !p.exists() && !part_exists(dir, n) && !receiving(dir, n) && !taken.contains(&path_key(&p))
    };
    if free(name) {
        return name.to_string();
    }
    let (base, ext) = split_ext(name);
    for i in 1..10_000 {
        let candidate = format!("{base} ({i}){ext}");
        if free(&candidate) {
            return candidate;
        }
    }
    format!("{base} ({}){ext}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    /// A name that looks like Pako's own unfinished file never is one; the other reserved
    /// device names and long names counted the way Windows counts them.
    #[test]
    fn names_never_pose_as_unfinished_files() {
        assert_eq!(super::sanitize("movie.mkv.pakopart"), "movie.mkv.pakopart_");
        assert_eq!(super::sanitize("a.ZDPART"), "a.ZDPART_");
        assert_eq!(super::sanitize("x.pakorecv"), "x.pakorecv_");
        assert_eq!(super::sanitize("movie.mkv"), "movie.mkv");
        assert_eq!(super::sanitize("COM¹.txt"), "_COM¹.txt");
        assert_eq!(super::sanitize("conout$"), "_conout$");
        let emoji = "😀".repeat(150) + ".zip";
        let s = super::sanitize(&emoji);
        assert!(s.encode_utf16().count() <= 180 && s.ends_with(".zip"), "{}", s.encode_utf16().count());
    }

    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pako-names-{tag}-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Direction marks (which can disguise "x.exe" as "exe.pdf") go; long names cut before a
    /// dot or a space don't end with one (Windows would drop it).
    #[test]
    fn sanitize_drops_direction_marks_and_trailing_dots() {
        assert_eq!(sanitize("invoice\u{202E}fdp.exe"), "invoicefdp.exe");
        assert_eq!(sanitize("a\u{2066}b\u{2069}.zip"), "ab.zip");
        let long = format!("{}. {}", "a".repeat(168), "b".repeat(20));
        let s = sanitize(&long);
        assert!(!s.ends_with('.') && !s.ends_with(' '), "{s:?}");
        assert_eq!(s, "a".repeat(168));
        assert_eq!(sanitize(&"x".repeat(200)).chars().count(), 170);
    }

    /// A name whose unfinished file exists (new or older Pako's) is taken.
    #[test]
    fn unique_sees_both_unfinished_names() {
        let dir = temp_dir("unique");
        let none = HashSet::new();
        assert_eq!(unique(&dir, "a.zip", &none), "a.zip");
        std::fs::write(dir.join("a.zip.pakopart"), b"x").unwrap();
        assert_eq!(unique(&dir, "a.zip", &none), "a (1).zip");
        std::fs::write(dir.join("b.zip.zdpart"), b"x").unwrap();
        assert_eq!(unique(&dir, "b.zip", &none), "b (1).zip");
        assert!(part_exists(&dir, "a.zip") && part_exists(&dir, "b.zip") && !part_exists(&dir, "c.zip"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Sets a file's "changed" time `secs` seconds in the past.
    pub(crate) fn age(path: &Path, secs: u64) {
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(secs)).unwrap();
    }

    /// An older Pako's unfinished file takes the new name when it's the only one.
    #[test]
    fn adopt_old_renames_when_new_is_missing() {
        let dir = temp_dir("adopt");
        let (old, new) = (old_part_of(&dir, "a.zip"), part_of(&dir, "a.zip"));
        assert_eq!(new.file_name().unwrap().to_string_lossy(), "a.zip.pakopart");
        std::fs::write(&old, b"old bytes").unwrap();
        adopt_old(&old, &new);
        assert!(!old.exists());
        assert_eq!(std::fs::read(&new).unwrap(), b"old bytes");
        // Neither there: nothing happens.
        adopt_old(&old_part_of(&dir, "z.zip"), &part_of(&dir, "z.zip"));
        assert!(!part_of(&dir, "z.zip").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Both there (an older Pako put back by a failed update wrote its own file meanwhile): the
    /// file written last is kept, under the new name, and the other one goes.
    #[test]
    fn adopt_old_prefers_the_file_written_last() {
        let dir = temp_dir("rollback");
        let (old, new) = (old_part_of(&dir, "a.zip"), part_of(&dir, "a.zip"));
        std::fs::write(&new, b"stale from 0.1.3").unwrap();
        age(&new, 3600);
        std::fs::write(&old, b"fresh from 0.1.2").unwrap();
        adopt_old(&old, &new);
        assert_eq!(std::fs::read(&new).unwrap(), b"fresh from 0.1.2");
        assert!(!old.exists());
        // The other way round (this version restarted the file after a failed rename): the
        // older Pako's stale file goes.
        std::fs::write(&old, b"stale from 0.1.2").unwrap();
        age(&old, 3600);
        std::fs::write(&new, b"fresh from 0.1.3").unwrap();
        adopt_old(&old, &new);
        assert_eq!(std::fs::read(&new).unwrap(), b"fresh from 0.1.3");
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Downloads and Direct never share an unfinished file: different endings, and each one's
    /// name check sees the other's unfinished files.
    #[test]
    fn direct_has_its_own_ending() {
        assert_ne!(RECV_EXT, PART_EXT);
        assert_ne!(RECV_EXT, OLD_PART_EXT);
        let dir = temp_dir("recv");
        std::fs::write(dir.join("x.zip.pakorecv"), b"x").unwrap();
        assert!(receiving(&dir, "x.zip") && !part_exists(&dir, "x.zip"));
        assert_eq!(unique(&dir, "x.zip", &HashSet::new()), "x (1).zip");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
