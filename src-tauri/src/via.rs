// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! "Via browser": a file only the browser could get (a site that refuses Pako's own request,
//! a proxy or VPN in the browser, a one-time link). The browser downloads it, the extension
//! reports its progress, and Pako shows it as one of its downloads. Once the browser has
//! finished, Pako moves the file to the folder chosen for it (never over another file).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The browser's file Pako may move: an absolute path to a file (not a folder) the browser
/// wrote recently (a report from the extension can't make Pako move an old file of yours).
pub fn usable_file(path: &str) -> Option<PathBuf> {
    let p = PathBuf::from(path);
    if path.len() > 4096 || !p.is_absolute() {
        return None;
    }
    let meta = std::fs::symlink_metadata(&p).ok()?;
    if !meta.is_file() {
        return None;
    }
    let recent = meta.modified().ok().and_then(|m| SystemTime::now().duration_since(m).ok()).is_none_or(|age| age < Duration::from_secs(24 * 3600));
    recent.then_some(p)
}

/// The same folder, however it is written (letter case, "..", short 8.3 names).
pub fn same_folder(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => crate::names::path_key(&a) == crate::names::path_key(&b),
        _ => crate::names::path_key(a) == crate::names::path_key(b),
    }
}

/// Where the browser's finished file ends up, and how:
/// - same folder and same name: left as it is (no copy, no "(2)"), unless that path is in
///   `taken` (an older row of the list has it, its file deleted since): then renamed in place
///   to a free name, so that row never points at this file;
/// - same folder, another name (`wanted`, picked by you): renamed in place, never over another
///   file;
/// - another folder: moved there (copied then deleted between drives), never over another file.
/// `wanted` = None keeps the browser's name (with any " (1)" the browser added). `taken`: names
/// other downloads will use (`names::path_key` of their paths).
pub fn place(from: &Path, dest_dir: &Path, wanted: Option<&str>, taken: &HashSet<String>) -> Result<PathBuf, String> {
    let from_name = from.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("the browser's file has no name")?;
    let name = wanted.map(crate::names::sanitize).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| from_name.clone());
    let from_dir = from.parent().ok_or("the browser's file has no folder")?;
    let same_dir = same_folder(from_dir, dest_dir);
    let listed = |p: &Path| taken.contains(&crate::names::path_key(p));
    if same_dir && name.to_lowercase() == from_name.to_lowercase() && !listed(from) && !listed(&dest_dir.join(&name)) {
        if name != from_name {
            // Only the letter case differs: Windows renames it in place.
            std::fs::rename(from, from_dir.join(&name)).map_err(|e| e.to_string())?;
            return Ok(from_dir.join(name));
        }
        return Ok(from.to_path_buf());
    }
    std::fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    let target_dir = if same_dir { from_dir } else { dest_dir };
    let mut tries = 0;
    loop {
        let target = target_dir.join(crate::names::unique_final(target_dir, &name, taken));
        match rename_new(from, &target) {
            Ok(()) => return Ok(target),
            // Taken meanwhile: the next free name.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && tries < 20 => {}
            // Another drive: copied, then the browser's copy goes.
            Err(e) if other_drive(&e) => return copy_then_remove(from, &target),
            // The browser or an antivirus still has the file open for a moment after it
            // finished: tried again a few times.
            Err(e) if e.raw_os_error() == Some(SHARING_VIOLATION) && tries < 10 => std::thread::sleep(std::time::Duration::from_millis(500)),
            Err(e) => return Err(e.to_string()),
        }
        tries += 1;
    }
}

#[cfg(windows)]
const SHARING_VIOLATION: i32 = 32;
#[cfg(not(windows))]
const SHARING_VIOLATION: i32 = -1;

/// The move crosses drives (it must be copied).
fn other_drive(e: &std::io::Error) -> bool {
    // ERROR_NOT_SAME_DEVICE on Windows, EXDEV elsewhere.
    e.raw_os_error() == Some(if cfg!(windows) { 17 } else { 18 })
}

/// Copied into a new file (never over another one), then the browser's copy goes. Either both
/// steps work or nothing changed: a copy cut short, or a browser's copy that can't be deleted
/// (still open), leaves only the browser's file, never two.
fn copy_then_remove(from: &Path, target: &Path) -> Result<PathBuf, String> {
    let copied = (|| -> std::io::Result<()> {
        let mut src = std::fs::File::open(from)?;
        let mut dst = std::fs::OpenOptions::new().write(true).create_new(true).open(target)?;
        std::io::copy(&mut src, &mut dst)?;
        dst.sync_all()
    })();
    if let Err(e) = copied {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            let _ = std::fs::remove_file(target);
        }
        return Err(e.to_string());
    }
    if let Err(e) = std::fs::remove_file(from) {
        let _ = std::fs::remove_file(target);
        return Err(e.to_string());
    }
    Ok(target.to_path_buf())
}

/// Renames, failing (AlreadyExists) when `to` exists (`std::fs::rename` replaces it on Windows).
#[cfg(windows)]
fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
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
fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
    }
    std::fs::rename(from, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pako-via-{tag}-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn same_folder_same_name_stays_put() {
        let dir = tmp("same");
        let f = dir.join("report (1).pdf");
        std::fs::write(&f, b"x").unwrap();
        // Written another way (letter case, a ".." step): still the same folder.
        let other = PathBuf::from(dir.to_string_lossy().to_uppercase()).join("sub").join("..");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let got = place(&f, &other, None, &HashSet::new()).unwrap();
        assert_eq!(got, f, "no move, no copy, no (2)");
        assert!(f.exists());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "nothing else made");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The browser saved it where an older row of the list had its file (deleted since): it
    /// gets a free name, so that row (and "Get" from another PC through it) never reaches it.
    #[test]
    fn same_folder_same_name_as_an_older_row_gets_a_free_name() {
        let dir = tmp("old-row");
        let f = dir.join("a.zip");
        std::fs::write(&f, b"new").unwrap();
        let mut taken = HashSet::new();
        taken.insert(crate::names::path_key(&dir.join("A.zip")));
        taken.insert(crate::names::path_key(&dir.join("a (1).zip")));
        let got = place(&f, &dir, None, &taken).unwrap();
        assert_eq!(got.parent().unwrap(), dir.as_path());
        assert_eq!(got.file_name().unwrap(), "a (2).zip", "not the old row's path, nor another row's");
        assert_eq!(std::fs::read(&got).unwrap(), b"new");
        assert!(!f.exists());
        // Only the letter case differs from the name asked for: the same.
        let g = dir.join("b.zip");
        std::fs::write(&g, b"x").unwrap();
        let mut taken = HashSet::new();
        taken.insert(crate::names::path_key(&dir.join("B.zip")));
        let got = place(&g, &dir, Some("B.zip"), &taken).unwrap();
        assert_eq!(got.file_name().unwrap(), "B (1).zip");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn same_folder_other_name_renames_in_place_never_over_a_file() {
        let dir = tmp("rename");
        let f = dir.join("download.bin");
        std::fs::write(&f, b"new").unwrap();
        std::fs::write(dir.join("Setup.exe"), b"old").unwrap();
        let got = place(&f, &dir, Some("Setup.exe"), &HashSet::new()).unwrap();
        assert_eq!(got.parent().unwrap(), dir.as_path());
        assert_ne!(got.file_name().unwrap(), "Setup.exe", "the other file stays");
        assert_eq!(std::fs::read(dir.join("Setup.exe")).unwrap(), b"old");
        assert_eq!(std::fs::read(&got).unwrap(), b"new");
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn another_folder_moves_it_keeping_the_browsers_name() {
        let from_dir = tmp("from");
        let to_dir = tmp("to");
        let f = from_dir.join("movie (1).mp4");
        std::fs::write(&f, b"data").unwrap();
        std::fs::write(to_dir.join("movie (1).mp4"), b"other").unwrap();
        let mut taken = HashSet::new();
        taken.insert(crate::names::path_key(&to_dir.join("movie (1) (2).mp4")));
        let got = place(&f, &to_dir, None, &taken).unwrap();
        assert_eq!(got.parent().unwrap(), to_dir.as_path());
        assert!(got.file_name().unwrap().to_string_lossy().starts_with("movie (1)"), "{got:?}");
        assert_ne!(got, to_dir.join("movie (1).mp4"));
        assert_ne!(got, to_dir.join("movie (1) (2).mp4"), "a name another download will use");
        assert_eq!(std::fs::read(&got).unwrap(), b"data");
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(from_dir);
        let _ = std::fs::remove_dir_all(to_dir);
    }

    /// The browser's file still open (an antivirus scanning it, say): never two copies, never a
    /// file replaced; it's said, and the browser's file stays where it is.
    #[cfg(windows)]
    #[test]
    fn a_file_still_open_is_left_where_it_is() {
        use std::os::windows::fs::OpenOptionsExt;
        let from_dir = tmp("open-from");
        let to_dir = tmp("open-to");
        let f = from_dir.join("big.iso");
        std::fs::write(&f, b"data").unwrap();
        // Open without letting anyone delete or move it (FILE_SHARE_READ only).
        let lock = std::fs::OpenOptions::new().read(true).share_mode(1).open(&f).unwrap();
        let got = place(&f, &to_dir, None, &HashSet::new());
        assert!(got.is_err(), "{got:?}");
        assert!(f.exists());
        assert_eq!(std::fs::read_dir(&to_dir).unwrap().count(), 0, "no copy left behind");
        drop(lock);
        // A file already there under the wanted name is never replaced (the move fails instead
        // of replacing, even if it appears between the name check and the move).
        let other = to_dir.join("big.iso");
        std::fs::write(&other, b"mine").unwrap();
        assert_eq!(rename_new(&f, &other).unwrap_err().kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&other).unwrap(), b"mine");
        let got = place(&f, &to_dir, None, &HashSet::new()).unwrap();
        assert_eq!(std::fs::read(&got).unwrap(), b"data");
        assert_eq!(std::fs::read(&other).unwrap(), b"mine");
        let _ = std::fs::remove_dir_all(from_dir);
        let _ = std::fs::remove_dir_all(to_dir);
    }

    /// Between drives: a copy into a new file, then the browser's copy goes; never over a file.
    #[test]
    fn a_copy_between_drives_never_replaces_and_never_doubles() {
        let from_dir = tmp("copy-from");
        let to_dir = tmp("copy-to");
        let f = from_dir.join("a.bin");
        std::fs::write(&f, b"data").unwrap();
        let taken = to_dir.join("a.bin");
        std::fs::write(&taken, b"mine").unwrap();
        assert!(copy_then_remove(&f, &taken).is_err());
        assert_eq!(std::fs::read(&taken).unwrap(), b"mine", "never replaced, never removed");
        assert!(f.exists());
        let got = copy_then_remove(&f, &to_dir.join("a (1).bin")).unwrap();
        assert_eq!(std::fs::read(&got).unwrap(), b"data");
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(from_dir);
        let _ = std::fs::remove_dir_all(to_dir);
    }

    #[test]
    fn only_a_recent_file_is_moved() {
        let dir = tmp("usable");
        let f = dir.join("a.zip");
        std::fs::write(&f, b"x").unwrap();
        assert_eq!(usable_file(&f.to_string_lossy()), Some(f.clone()));
        assert_eq!(usable_file(&dir.to_string_lossy()), None, "a folder");
        assert_eq!(usable_file("a.zip"), None, "not a full path");
        assert_eq!(usable_file(&dir.join("missing.zip").to_string_lossy()), None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
