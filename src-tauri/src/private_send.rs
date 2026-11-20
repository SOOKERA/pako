// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Sending a private file with Direct or Upload: Pako reads it decrypted as it goes, so no
//! readable copy is written. The send gets the encrypted file's path; this module knows its real
//! name and size, and reads it block by block (only while Private mode is open: locking ends
//! every read). Upload's password-protected archives are made by 7-Zip, which can only read
//! files on disk: those get readable copies in Pako's owner-only private folder, wiped right
//! after (and at the next start, if Pako stopped meanwhile).

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use crate::vault::{self, blocks::Reader};

/// A private file handed to a send: its real name (with its folder, "Trip/Maps/Route.gpx") and
/// size, and its reader once opened.
struct Sending {
    name: String,
    size: u64,
    reader: Option<Arc<Reader>>,
}

static SENDING: LazyLock<Mutex<HashMap<PathBuf, Sending>>> = LazyLock::new(Default::default);
/// Goes up each time Private mode locks: a reader opened before stops at its next read.
static LOCKS: AtomicU64 = AtomicU64::new(0);

fn locked() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "Private mode is locked")
}

/// Pako's vault (a vault of its own in tests: never the real data folder's).
#[cfg(not(test))]
fn the_vault() -> &'static vault::Vault {
    vault::get()
}
#[cfg(test)]
fn the_vault() -> &'static vault::Vault {
    static TEST: LazyLock<vault::Vault> = LazyLock::new(|| {
        let v = vault::Vault::new(std::env::temp_dir().join(format!("pako-psend-vault-{}", uuid::Uuid::new_v4().simple())));
        v.setup("2468", "pin").unwrap();
        v
    });
    &TEST
}

/// Tests with private files take turns (one of them locks Private mode for everyone).
#[cfg(test)]
pub static TEST_TURN: Mutex<()> = Mutex::new(());

/// Tests elsewhere (Direct's): a finished private file holding `data`, handed over as `name`.
#[cfg(test)]
pub fn test_private_file(data: &[u8], name: &str) -> PathBuf {
    let v = the_vault();
    let target = v.file_path(&format!("t-{}", uuid::Uuid::new_v4().simple()));
    let w = v.create_file(&target, Some(data.len() as u64)).unwrap();
    w.write_at(data, 0).unwrap();
    w.finish(data.len() as u64).unwrap();
    drop(w);
    register(&target, name, data.len() as u64);
    target
}

/// Private mode is open (private files can be read).
pub fn is_open() -> bool {
    the_vault().is_unlocked()
}

/// How many times Private mode has locked in this run: a read that started before and ends
/// after a change read bytes that may have been wiped meanwhile (never used).
pub fn lock_count() -> u64 {
    LOCKS.load(Ordering::SeqCst)
}

/// A file in Private mode's vault (encrypted): it's only ever read decrypted, never as it is.
pub fn is_private(path: &Path) -> bool {
    path.parent() == Some(the_vault().files_dir().as_path())
}

/// Hands a private file to Direct or Upload under its real name.
pub fn register(path: &Path, name: &str, size: u64) {
    SENDING.lock().unwrap().insert(path.to_path_buf(), Sending { name: name.to_string(), size, reader: None });
}

/// Its real name and size, when `path` is a private file handed to a send.
pub fn known(path: &Path) -> Option<(String, u64)> {
    SENDING.lock().unwrap().get(path).map(|s| (s.name.clone(), s.size))
}

/// Change time in ms (as Direct's resume notes keep it).
pub fn mtime_ms(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Size and change time as a send sees them: the real size for a private file (its time is the
/// encrypted file's), else the file's own.
pub fn info(path: &Path) -> Option<(u64, SystemTime)> {
    // A private file not handed over (Private mode locked since): never its encrypted size.
    if is_private(path) && known(path).is_none() {
        return None;
    }
    let m = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    let len = known(path).map(|(_, s)| s).unwrap_or(m.len());
    Some((len, m.modified().unwrap_or(SystemTime::UNIX_EPOCH)))
}

/// Private mode locked: every reader goes (a send reading one stops with "Private mode is
/// locked"), and so does what was handed over.
pub fn forget_all() {
    LOCKS.fetch_add(1, Ordering::SeqCst);
    SENDING.lock().unwrap().clear();
}

fn reader(path: &Path) -> io::Result<Arc<Reader>> {
    let mut all = SENDING.lock().unwrap();
    let s = all.get_mut(path).ok_or_else(locked)?;
    if let Some(r) = &s.reader {
        return Ok(r.clone());
    }
    let r = Arc::new(the_vault().reader(path)?);
    s.reader = Some(r.clone());
    Ok(r)
}

/// Reads at `at`: decrypted for a private file (an error once Private mode locked: never its
/// encrypted bytes), else from the file itself.
pub fn read_at(path: &Path, buf: &mut [u8], at: u64) -> io::Result<usize> {
    if is_private(path) {
        return reader(path)?.read_at(buf, at);
    }
    let f = std::fs::File::open(path)?;
    #[cfg(windows)]
    return std::os::windows::fs::FileExt::seek_read(&f, buf, at);
    #[cfg(not(windows))]
    return std::os::unix::fs::FileExt::read_at(&f, buf, at);
}

/// Reads from the start: decrypted for a private file, else the file itself.
pub fn open(path: &Path) -> io::Result<Box<dyn Read + Send>> {
    if is_private(path) {
        return Ok(Box::new(Plain { r: reader(path)?, at: 0, since: LOCKS.load(Ordering::SeqCst) }));
    }
    Ok(Box::new(std::fs::File::open(path)?))
}

struct Plain {
    r: Arc<Reader>,
    at: u64,
    /// The lock count when it was opened: Private mode locking since ends it.
    since: u64,
}

impl Read for Plain {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if LOCKS.load(Ordering::SeqCst) != self.since {
            return Err(locked());
        }
        let n = self.r.read_at(buf, self.at)?;
        // Locked while it read: those bytes are dropped, never used.
        if LOCKS.load(Ordering::SeqCst) != self.since {
            buf[..n].fill(0);
            return Err(locked());
        }
        self.at += n as u64;
        Ok(n)
    }
}

/// For 7-Zip (password-protected uploads): readable copies of the private files among `paths`,
/// under their real names, in an owner-only folder of their own; the others stay as they are.
/// Returns the paths to give 7-Zip and that folder (wipe it with `wipe_copies` when done).
pub fn copies_for_7zip(job: &str, paths: &[PathBuf]) -> io::Result<(Vec<PathBuf>, Option<PathBuf>)> {
    if !paths.iter().any(|p| is_private(p)) {
        return Ok((paths.to_vec(), None));
    }
    // Private mode locked since the hand-over: never the encrypted files themselves.
    if paths.iter().any(|p| is_private(p) && known(p).is_none()) {
        return Err(locked());
    }
    // Inside the private copies' folder: only this Windows user can open it, and a folder there
    // with no note is wiped at the next start (crate::private_files::sweep).
    let root = crate::private_files::open_dir();
    crate::video::make_private_dir(&root)?;
    let dir = root.join(format!("send-{job}"));
    std::fs::create_dir_all(&dir)?;
    let mut out: Vec<PathBuf> = Vec::new();
    let mut tops = std::collections::HashSet::new();
    let mut taken = std::collections::HashSet::new();
    let made = (|| -> io::Result<()> {
        for p in paths {
            let Some((name, _)) = known(p) else {
                out.push(p.clone());
                continue;
            };
            // A file of a private folder keeps its folder: 7-Zip stores it with that path. Two
            // with the same name get " (2)" (never one over the other).
            let parts: Vec<String> = name.split('/').map(crate::names::sanitize).filter(|c| !c.is_empty() && c != "." && c != "..").collect();
            let mut rel: PathBuf = parts.iter().collect();
            let mut n = 2;
            while !taken.insert(rel.to_string_lossy().to_lowercase()) {
                let last = parts.last().cloned().unwrap_or_else(|| "file".into());
                let (stem, ext) = match last.rsplit_once('.') {
                    Some((st, e)) if !st.is_empty() => (st.to_string(), format!(".{e}")),
                    _ => (last.clone(), String::new()),
                };
                rel = parts[..parts.len().saturating_sub(1)].iter().collect::<PathBuf>().join(format!("{stem} ({n}){ext}"));
                n += 1;
            }
            let to = dir.join(&rel);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut src = open(p)?;
            let mut dst = std::fs::File::create(&to)?;
            io::copy(&mut src, &mut dst)?;
            dst.sync_all()?;
            // 7-Zip gets each top folder (or file) once.
            let top = dir.join(rel.components().next().map(|c| c.as_os_str().to_owned()).unwrap_or_default());
            if tops.insert(top.clone()) {
                out.push(top);
            }
        }
        Ok(())
    })();
    if let Err(e) = made {
        wipe_copies(&dir);
        return Err(e);
    }
    Ok((out, Some(dir)))
}

/// The readable copies made for 7-Zip: gone (overwritten first), now.
pub fn wipe_copies(dir: &Path) {
    crate::private_files::wipe_dir(dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A private file reads back whole, decrypted, at any place; its real name and size are what a
    /// send sees; locking ends every read; copies for 7-Zip are wiped.
    #[test]
    fn private_files_read_as_they_are_and_leave_nothing_readable() {
        let _turn = TEST_TURN.lock().unwrap_or_else(|e| e.into_inner());
        let data: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let target = test_private_file(&data, "Trip/Report.bin");
        assert!(!std::fs::read(&target).unwrap().windows(64).any(|x| x == &data[1000..1064]), "encrypted on disk");

        assert_eq!(known(&target), Some(("Trip/Report.bin".to_string(), data.len() as u64)));
        assert_eq!(info(&target).unwrap().0, data.len() as u64);
        let mut all = Vec::new();
        open(&target).unwrap().read_to_end(&mut all).unwrap();
        assert_eq!(all, data, "the bytes sent are the original's");
        let mut part = vec![0u8; 5000];
        let n = read_at(&target, &mut part, 70_000).unwrap();
        assert_eq!(&part[..n], &data[70_000..70_000 + n]);

        let (paths, dir) = copies_for_7zip("t1", std::slice::from_ref(&target)).unwrap();
        let dir = dir.unwrap();
        assert_eq!(std::fs::read(dir.join("Trip").join("Report.bin")).unwrap(), data);
        assert_eq!(paths, vec![dir.join("Trip")]);
        wipe_copies(&dir);
        assert!(!dir.exists(), "nothing readable left");

        // Making the copies fails partway (a private file that can't be read): nothing is left.
        let broken = the_vault().file_path("broken-one");
        std::fs::write(&broken, b"not a private file").unwrap();
        register(&broken, "Broken.bin", 18);
        assert!(copies_for_7zip("t2", &[target.clone(), broken.clone()]).is_err());
        assert!(!crate::private_files::open_dir().join("send-t2").exists(), "no copy left after a failure");
        let _ = std::fs::remove_file(&broken);

        // Two files with the same name and a folder's files: none written over, each top once.
        let second = test_private_file(&data[..1000], "Trip/Report.bin");
        let third = test_private_file(b"hi", "Trip/Other.txt");
        let (paths, dir) = copies_for_7zip("t3", &[target.clone(), second.clone(), third.clone()]).unwrap();
        let dir = dir.unwrap();
        assert_eq!(paths, vec![dir.join("Trip")], "the folder once");
        assert_eq!(std::fs::read(dir.join("Trip").join("Report.bin")).unwrap(), data);
        assert_eq!(std::fs::read(dir.join("Trip").join("Report (2).bin")).unwrap(), &data[..1000]);
        assert_eq!(std::fs::read(dir.join("Trip").join("Other.txt")).unwrap(), b"hi");
        wipe_copies(&dir);

        // Private mode locks while a send reads: its next read stops.
        let mut reading = open(&target).unwrap();
        let mut some = vec![0u8; 1000];
        reading.read_exact(&mut some).unwrap();
        // Locked (or forgotten): an error, never the encrypted bytes passed off as the file.
        forget_all();
        assert_eq!(reading.read(&mut some).unwrap_err().kind(), io::ErrorKind::PermissionDenied, "a read under way stops");
        assert_eq!(copies_for_7zip("t4", std::slice::from_ref(&target)).unwrap_err().kind(), io::ErrorKind::PermissionDenied, "never the raw file for 7-Zip");
        assert!(known(&target).is_none());
        assert!(open(&target).is_err());
        assert!(read_at(&target, &mut part, 0).is_err());
        assert!(info(&target).is_none(), "never its encrypted size");
        let _ = std::fs::remove_file(&target);
    }
}
