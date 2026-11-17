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

/// A file's last-modified time (ms), to see whether it changed since it was offered.
pub fn mtime_of(m: &std::fs::Metadata) -> u64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0)
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
}

fn dir(data: &Path) -> PathBuf {
    data.join("direct_resume")
}

fn in_path(data: &Path, id: &str) -> PathBuf {
    dir(data).join(format!("{id}.in.json"))
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

pub fn save_in(data: &Path, note: &InNote) -> std::io::Result<()> {
    save(in_path(data, &note.id), note)
}

pub fn save_out(data: &Path, note: &OutNote) -> std::io::Result<()> {
    save(out_path(data, &note.id), note)
}

fn remove(path: PathBuf) {
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("bak"));
}

pub fn remove_in(data: &Path, id: &str) {
    remove(in_path(data, id));
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
                Ok(n) if n.id == id && [n.parts.len(), n.targets.len(), n.have.len(), n.retried.len()].iter().all(|&l| l == n.files.len()) => ins.push(n),
                _ => remove(p),
            }
        } else {
            match serde_json::from_slice::<OutNote>(&bytes) {
                Ok(n) if n.id == id => outs.push(n),
                _ => remove(p),
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
        let o = OutNote { id: id.clone(), contact: "c".into(), name: "x".into(), files: vec![], temp: None, done: 3, since: 5 };
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
}
