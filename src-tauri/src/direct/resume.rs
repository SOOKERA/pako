// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Picking up a Direct transfer where it stopped: which bytes of each file are on the disk,
//! and the small notes both sides keep so a transfer cut short (Wi-Fi gone, a PC asleep, Pako
//! closed) continues when the two PCs connect again.
//!
//! The receiver keeps its unfinished ".pakorecv" files and a note of the byte ranges written
//! and flushed to the disk (the data is synced before the note is saved, so the note never
//! claims bytes that aren't there). The sender keeps the file list with each file's size and
//! last-modified time. Both notes live in Pako's own data folder (`direct_resume`), never in
//! the receiving folder. Unfinished transfers are kept 7 days.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::proto::{FileHave, OfferFile};

/// How long a transfer waits for the other PC before it's cleared on this side.
pub const KEEP_MS: u64 = 7 * 24 * 3600 * 1000;

/// The byte ranges of one file that are here: [start, end), in order, never touching.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ranges(Vec<[u64; 2]>);

impl Ranges {
    /// Adds [start, end); returns how many of those bytes are new.
    pub fn insert(&mut self, start: u64, end: u64) -> u64 {
        if start >= end {
            return 0;
        }
        let v = &mut self.0;
        // The first range that ends at or after `start` (touching ones merge too).
        let i = v.partition_point(|r| r[1] < start);
        let (mut s, mut e, mut overlap) = (start, end, 0);
        let mut j = i;
        while j < v.len() && v[j][0] <= end {
            let r = v[j];
            overlap += r[1].min(end).saturating_sub(r[0].max(start));
            s = s.min(r[0]);
            e = e.max(r[1]);
            j += 1;
        }
        v.splice(i..j, [[s, e]]);
        (end - start) - overlap
    }

    /// Bytes here.
    pub fn covered(&self) -> u64 {
        self.0.iter().map(|r| r[1] - r[0]).sum()
    }

    /// What's still missing of a file of `size` bytes.
    pub fn missing(&self, size: u64) -> Vec<[u64; 2]> {
        let mut out = Vec::new();
        let mut at = 0;
        for r in &self.0 {
            if r[0] >= size {
                break;
            }
            if r[0] > at {
                out.push([at, r[0]]);
            }
            at = at.max(r[1]);
        }
        if at < size {
            out.push([at, size]);
        }
        out
    }

    pub fn is_full(&self, size: u64) -> bool {
        size == 0 || self.0.len() == 1 && self.0[0] == [0, size]
    }

    #[cfg(test)]
    pub fn list(&self) -> &[[u64; 2]] {
        &self.0
    }

    /// Ranges from the other side, checked against the file's size: None if any is empty,
    /// backwards or past the end. Put in order first, so each one joins at the end: a long list
    /// sent backwards (a few hundred thousand fit in one message) can't make this slow.
    pub fn checked(list: &[[u64; 2]], size: u64) -> Option<Self> {
        let mut list = list.to_vec();
        list.sort_unstable_by_key(|r| r[0]);
        let mut r = Ranges::default();
        for [s, e] in list {
            if s >= e || e > size {
                return None;
            }
            r.insert(s, e);
        }
        Some(r)
    }
}

/// What the receiver says it has, in a `Have` message: runs of complete files, and the
/// files partly here.
pub fn encode(have: &[Ranges], sizes: &[u64]) -> (Vec<[usize; 2]>, Vec<FileHave>) {
    let mut full: Vec<[usize; 2]> = Vec::new();
    let mut part = Vec::new();
    for (i, (r, &size)) in have.iter().zip(sizes).enumerate() {
        if r.is_full(size) {
            match full.last_mut() {
                Some(run) if run[1] == i => run[1] = i + 1,
                _ => full.push([i, i + 1]),
            }
        } else if !r.0.is_empty() {
            part.push(FileHave { f: i, r: r.0.clone() });
        }
    }
    (full, part)
}

/// A `Have` from the receiver, checked against the files as offered: None when anything is
/// off (a file that doesn't exist, a range past its end, a file listed twice).
pub fn decode(full: &[[usize; 2]], part: &[FileHave], sizes: &[u64]) -> Option<Vec<Ranges>> {
    let n = sizes.len();
    let mut out = vec![Ranges::default(); n];
    let mut seen = vec![false; n];
    for &[a, b] in full {
        if a >= b || b > n {
            return None;
        }
        for i in a..b {
            if std::mem::replace(&mut seen[i], true) {
                return None;
            }
            if sizes[i] > 0 {
                out[i].insert(0, sizes[i]);
            }
        }
    }
    for p in part {
        if p.f >= n || std::mem::replace(&mut seen[p.f], true) {
            return None;
        }
        out[p.f] = Ranges::checked(&p.r, sizes[p.f])?;
    }
    Some(out)
}

/// What the receiving side keeps about a transfer cut short.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InNote {
    pub id: String,
    pub contact: String,
    pub name: String,
    pub files: Vec<OfferFile>,
    /// Where each file ends up, and the ".pakorecv" file it's written to meanwhile.
    pub targets: Vec<PathBuf>,
    pub parts: Vec<PathBuf>,
    pub saved_to: Option<PathBuf>,
    /// Written and synced to the disk, per file.
    pub have: Vec<Ranges>,
    /// Sent again once already because it didn't match its hash.
    pub retried: Vec<bool>,
    /// When it last moved or stopped (ms).
    pub since: u64,
}

/// One file the sending side keeps for a transfer cut short.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutFile {
    pub path: PathBuf,
    pub file: OfferFile,
    /// Last-modified time when offered (ms).
    pub mtime: u64,
}

/// What the sending side keeps about a transfer cut short.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutNote {
    pub id: String,
    pub contact: String,
    pub name: String,
    pub files: Vec<OutFile>,
    /// A temporary ZIP made for "Compressed ZIP" (kept until it ends).
    pub temp: Option<PathBuf>,
    /// Bytes the receiver had, as last known.
    pub done: u64,
    pub since: u64,
    /// Sent because another of your devices asked for it (no sent sound here). Left out when
    /// not (and in notes from before).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ordered: bool,
    /// A send of files from Private mode: their names (and the send's) are kept sealed for this
    /// Windows user (DPAPI) instead of written out, so the note never names them on disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed: Option<String>,
}

/// The names a note keeps sealed: the send's, then each file's (empty for files not private).
#[derive(Serialize, Deserialize)]
struct Names {
    name: String,
    files: Vec<Option<String>>,
}

/// The note as written to disk: a private file's name (and the send's) sealed. Not private:
/// as it is.
fn sealed_out(note: &OutNote) -> std::io::Result<OutNote> {
    let private: Vec<bool> = note.files.iter().map(|f| crate::private_send::is_private(&f.path)).collect();
    if !private.contains(&true) {
        return Ok(note.clone());
    }
    let names = Names { name: note.name.clone(), files: note.files.iter().zip(&private).map(|(f, &p)| p.then(|| f.file.path.clone())).collect() };
    let bytes = serde_json::to_vec(&names).map_err(std::io::Error::other)?;
    let sealed = crate::secure::protect(&bytes).map_err(std::io::Error::other)?;
    let mut out = note.clone();
    out.name = String::new();
    for (f, &p) in out.files.iter_mut().zip(&private) {
        if p {
            f.file.path = String::new();
        }
    }
    use base64::Engine as _;
    out.sealed = Some(base64::engine::general_purpose::STANDARD.encode(sealed));
    Ok(out)
}

/// Its names back (None: they can't be read, e.g. another Windows user's copy).
fn unsealed_out(mut note: OutNote) -> Option<OutNote> {
    let Some(sealed) = note.sealed.take() else { return Some(note) };
    use base64::Engine as _;
    let raw = base64::engine::general_purpose::STANDARD.decode(sealed).ok()?;
    let names: Names = serde_json::from_slice(&crate::secure::unprotect(&raw).ok()?).ok()?;
    if names.files.len() != note.files.len() {
        return None;
    }
    note.name = names.name;
    for (f, n) in note.files.iter_mut().zip(names.files) {
        if let Some(n) = n {
            f.file.path = n;
        }
    }
    Some(note)
}

fn dir(data: &Path) -> PathBuf {
    data.join("direct_resume")
}

fn in_path(data: &Path, id: &str) -> PathBuf {
    dir(data).join(format!("{id}.in.json"))
}

/// How far a transfer coming in got (saved every few seconds while it moves, apart from the
/// rest of its note, which is written once when it's accepted or continues).
fn have_path(data: &Path, id: &str) -> PathBuf {
    dir(data).join(format!("{id}.have.json"))
}

/// What changes while files arrive: the bytes written and synced, the files sent again, when
/// it last moved. Small next to the full note of a big folder (every file, every place).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InProgress {
    pub have: Vec<Ranges>,
    pub retried: Vec<bool>,
    pub since: u64,
}

fn out_path(data: &Path, id: &str) -> PathBuf {
    dir(data).join(format!("{id}.out.json"))
}

fn save<T: Serialize>(path: PathBuf, note: &T) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let bytes = serde_json::to_vec(note).map_err(std::io::Error::other)?;
    crate::paths::write_atomic(&path, &bytes)
}

/// The full note (when a transfer is accepted, continues, pauses or ends): it holds the
/// progress too, so an older progress file is dropped.
pub fn save_in(data: &Path, note: &InNote) -> std::io::Result<()> {
    save(in_path(data, &note.id), note)?;
    let _ = std::fs::remove_file(have_path(data, &note.id));
    Ok(())
}

/// Only how far it got (every few seconds while it moves): written whole and replaced in one
/// step (a cut leaves the last one), with no ".bak" copy; its note must be saved already.
pub fn save_in_progress(data: &Path, id: &str, p: &InProgress) -> std::io::Result<()> {
    if !in_path(data, id).exists() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no note yet"));
    }
    let path = have_path(data, id);
    let bytes = serde_json::to_vec(p).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, &path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

pub fn save_out(data: &Path, note: &OutNote) -> std::io::Result<()> {
    save(out_path(data, &note.id), &sealed_out(note)?)
}

fn remove(path: PathBuf) {
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("bak"));
}

pub fn remove_in(data: &Path, id: &str) {
    remove(in_path(data, id));
    let _ = std::fs::remove_file(have_path(data, id));
}

pub fn remove_out(data: &Path, id: &str) {
    remove(out_path(data, id));
}

/// Every note kept (unreadable ones are dropped).
pub fn load(data: &Path) -> (Vec<InNote>, Vec<OutNote>) {
    let (mut ins, mut outs) = (Vec::new(), Vec::new());
    let Ok(rd) = std::fs::read_dir(dir(data)) else { return (ins, outs) };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        let Some(id) = name.strip_suffix(".in.json").or_else(|| name.strip_suffix(".out.json")) else {
            // Left from a save cut short (".tmp", ".bak"): its note is complete or gone.
            if name.ends_with(".tmp") {
                let _ = std::fs::remove_file(&p);
            }
            continue;
        };
        if !super::proto::valid_id(id) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&p) else { continue };
        if name.ends_with(".in.json") {
            match serde_json::from_slice::<InNote>(&bytes) {
                Ok(mut n) if n.id == id && [n.parts.len(), n.targets.len(), n.have.len(), n.retried.len()].iter().all(|&l| l == n.files.len()) => {
                    // Newer progress saved apart (one that doesn't fit the note is ignored).
                    if let Some(g) = std::fs::read(have_path(data, id)).ok().and_then(|b| serde_json::from_slice::<InProgress>(&b).ok()) {
                        if g.have.len() == n.files.len() && g.retried.len() == n.files.len() && g.have.iter().zip(&n.files).all(|(r, f)| r.missing(f.size).iter().all(|m| m[1] <= f.size) && r.covered() <= f.size) {
                            n.have = g.have;
                            n.retried = g.retried;
                            n.since = n.since.max(g.since);
                        }
                    }
                    ins.push(n)
                }
                _ => remove(p),
            }
        } else {
            match serde_json::from_slice::<OutNote>(&bytes).ok().filter(|n| n.id == id).and_then(unsealed_out) {
                Some(n) => outs.push(n),
                None => remove(p),
            }
        }
    }
    (ins, outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_merge_and_say_what_is_missing() {
        let mut r = Ranges::default();
        assert_eq!(r.missing(10), vec![[0, 10]]);
        assert_eq!(r.insert(2, 4), 2);
        assert_eq!(r.insert(6, 8), 2);
        assert_eq!(r.list(), &[[2, 4], [6, 8]]);
        assert_eq!(r.missing(10), vec![[0, 2], [4, 6], [8, 10]]);
        // Overlapping and touching pieces merge; only new bytes count.
        assert_eq!(r.insert(3, 7), 2);
        assert_eq!(r.list(), &[[2, 8]]);
        assert_eq!(r.insert(8, 9), 1);
        assert_eq!(r.insert(0, 2), 2);
        assert_eq!(r.list(), &[[0, 9]]);
        assert_eq!(r.insert(1, 5), 0, "nothing new");
        assert_eq!(r.insert(5, 5), 0, "empty");
        assert_eq!(r.covered(), 9);
        assert!(!r.is_full(10));
        assert_eq!(r.insert(0, 10), 1);
        assert!(r.is_full(10));
        assert!(r.missing(10).is_empty());
        // Out of order, many pieces.
        let mut r = Ranges::default();
        for i in (0..100u64).rev().step_by(2) {
            r.insert(i * 10, i * 10 + 10);
        }
        assert_eq!(r.list().len(), 50);
        for i in (0..100u64).step_by(2) {
            r.insert(i * 10, i * 10 + 10);
        }
        assert_eq!(r.list(), &[[0, 1000]]);
        assert!(Ranges::default().is_full(0));
    }

    #[test]
    fn what_the_receiver_has_goes_and_comes_back_checked() {
        let sizes = [5, 0, 10, 10, 7];
        let mut have = vec![Ranges::default(); 5];
        have[0].insert(0, 5);
        have[2].insert(0, 10);
        have[3].insert(2, 6);
        let (full, part) = encode(&have, &sizes);
        assert_eq!(full, vec![[0, 3]], "complete and empty files in one run");
        assert_eq!(part, vec![FileHave { f: 3, r: vec![[2, 6]] }]);
        let back = decode(&full, &part, &sizes).unwrap();
        assert_eq!(back[0], have[0]);
        assert_eq!(back[3], have[3]);
        assert!(back[4].list().is_empty());
        // Anything past what was offered is refused.
        assert!(decode(&[[0, 6]], &[], &sizes).is_none(), "a file that doesn't exist");
        assert!(decode(&[[2, 2]], &[], &sizes).is_none());
        assert!(decode(&[], &[FileHave { f: 9, r: vec![] }], &sizes).is_none());
        assert!(decode(&[], &[FileHave { f: 3, r: vec![[0, 11]] }], &sizes).is_none(), "past the end");
        assert!(decode(&[], &[FileHave { f: 3, r: vec![[4, 4]] }], &sizes).is_none());
        assert!(decode(&[[3, 4]], &[FileHave { f: 3, r: vec![[0, 1]] }], &sizes).is_none(), "listed twice");
        assert!(decode(&[], &[FileHave { f: 3, r: vec![[0, u64::MAX]] }], &sizes).is_none());
    }

    /// A receiver listing a few hundred thousand ranges backwards (about what one message holds)
    /// is read at once, not in minutes.
    #[test]
    fn a_long_backwards_range_list_is_quick() {
        let n = 300_000u64;
        let list: Vec<[u64; 2]> = (0..n).rev().map(|i| [i * 2, i * 2 + 1]).collect();
        let at = std::time::Instant::now();
        let r = Ranges::checked(&list, n * 2).unwrap();
        assert_eq!(r.list().len(), n as usize);
        assert_eq!(r.covered(), n);
        assert!(at.elapsed() < std::time::Duration::from_secs(2), "{:?}", at.elapsed());
        // Overlapping and touching ones still merge.
        assert_eq!(Ranges::checked(&[[5, 9], [0, 3], [2, 6]], 10).unwrap().list(), &[[0, 9]]);
    }

    #[test]
    fn notes_are_saved_and_loaded() {
        let data = std::env::temp_dir().join(format!("pako-resume-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let id = uuid::Uuid::new_v4().to_string();
        let mut have = vec![Ranges::default()];
        have[0].insert(0, 3);
        let n = InNote {
            id: id.clone(),
            contact: "c".into(),
            name: "x".into(),
            files: vec![OfferFile { path: "a.bin".into(), size: 9 }],
            targets: vec![data.join("a.bin")],
            parts: vec![data.join("a.bin.pakorecv")],
            saved_to: Some(data.join("a.bin")),
            have,
            retried: vec![false],
            since: 5,
        };
        save_in(&data, &n).unwrap();
        let o = OutNote { id: id.clone(), contact: "c".into(), name: "x".into(), files: vec![], temp: None, done: 3, since: 5, ordered: false, sealed: None };
        save_out(&data, &o).unwrap();
        // Something else in the folder, and a note with a made-up name: left out.
        std::fs::write(dir(&data).join("junk.in.json"), b"{}").unwrap();
        std::fs::write(dir(&data).join(format!("{}.in.json", uuid::Uuid::new_v4())), b"not json").unwrap();
        let (ins, outs) = load(&data);
        assert_eq!(ins, vec![n]);
        assert_eq!(outs, vec![o]);
        remove_in(&data, &id);
        remove_out(&data, &id);
        let (ins, outs) = load(&data);
        assert!(ins.is_empty() && outs.is_empty());
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A send another of your devices asked for keeps that after a restart (no sent sound when
    /// it ends); notes from before load as not asked for, and a plain one doesn't write it.
    #[test]
    fn an_ordered_send_stays_ordered() {
        let o = OutNote { id: "i".into(), contact: "c".into(), name: "x".into(), files: vec![], temp: None, done: 3, since: 5, ordered: true, sealed: None };
        let back: OutNote = serde_json::from_slice(&serde_json::to_vec(&o).unwrap()).unwrap();
        assert!(back.ordered);
        let old: OutNote = serde_json::from_str(r#"{"id":"i","contact":"c","name":"x","files":[],"temp":null,"done":3,"since":5}"#).unwrap();
        assert!(!old.ordered);
        assert!(!serde_json::to_string(&OutNote { ordered: false, ..o }).unwrap().contains("ordered"));
    }

    /// While files arrive only their progress is saved, apart and without a ".bak" copy; the
    /// note read back has it. Saving the full note again drops the separate progress.
    #[test]
    fn progress_is_saved_apart_and_read_back() {
        let data = std::env::temp_dir().join(format!("pako-resume-progress-{}", uuid::Uuid::new_v4()));
        let id = uuid::Uuid::new_v4().to_string();
        let n = InNote {
            id: id.clone(),
            contact: "c".into(),
            name: "x".into(),
            files: vec![OfferFile { path: "a.bin".into(), size: 9 }, OfferFile { path: "b.bin".into(), size: 4 }],
            targets: vec![data.join("a.bin"), data.join("b.bin")],
            parts: vec![data.join("a.bin.pakorecv"), data.join("b.bin.pakorecv")],
            saved_to: None,
            have: vec![Ranges::default(), Ranges::default()],
            retried: vec![false, false],
            since: 5,
        };
        // No note yet: no progress either.
        let mut a = Ranges::default();
        a.insert(0, 6);
        let p = InProgress { have: vec![a.clone(), Ranges::default()], retried: vec![false, true], since: 9 };
        assert!(save_in_progress(&data, &id, &p).is_err());
        save_in(&data, &n).unwrap();
        save_in_progress(&data, &id, &p).unwrap();
        save_in_progress(&data, &id, &p).unwrap();
        assert!(!dir(&data).join(format!("{id}.have.bak")).exists() && !dir(&data).join(format!("{id}.have.json.bak")).exists(), "no .bak for progress");
        let (ins, _) = load(&data);
        assert_eq!(ins.len(), 1);
        assert_eq!((ins[0].have.clone(), ins[0].retried.clone(), ins[0].since), (p.have.clone(), p.retried.clone(), 9));
        // Progress that doesn't fit the note (another number of files) is ignored.
        save_in_progress(&data, &id, &InProgress { have: vec![a.clone()], retried: vec![false], since: 11 }).unwrap();
        assert_eq!(load(&data).0[0].have, n.have);
        // A full save again: the separate progress goes.
        save_in(&data, &n).unwrap();
        assert!(!have_path(&data, &id).exists());
        remove_in(&data, &id);
        assert!(load(&data).0.is_empty());
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A send of files from Private mode never writes their names in its note: they're sealed
    /// for this Windows user, and back when the note is read.
    #[test]
    fn private_names_are_sealed_in_the_note() {
        let _turn = crate::private_send::TEST_TURN.lock().unwrap_or_else(|e| e.into_inner());
        let data = std::env::temp_dir().join(format!("pako-resume-sealed-{}", uuid::Uuid::new_v4()));
        let id = uuid::Uuid::new_v4().to_string();
        let vault = crate::private_send::test_private_file(b"secret bytes", "Trip/Secret report.pdf");
        let o = OutNote {
            id: id.clone(),
            contact: "c".into(),
            name: "Secret report.pdf".into(),
            files: vec![OutFile { path: vault.clone(), file: OfferFile { path: "Trip/Secret report.pdf".into(), size: 12 }, mtime: 1 }],
            temp: None,
            done: 0,
            since: 5,
            ordered: false,
            sealed: None,
        };
        save_out(&data, &o).unwrap();
        let raw = std::fs::read(out_path(&data, &id)).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("Secret"), "the name isn't on disk");
        let (_, outs) = load(&data);
        assert_eq!(outs, vec![o]);
        let _ = std::fs::remove_dir_all(&data);
    }
}
