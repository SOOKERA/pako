// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Send files with a link (PLAN.md Phase 6, first version: Gofile, no account).
//!
//! One file is uploaded as it is. Several files or a folder are, by default, also sent as they
//! are: the first one makes a Gofile folder, the rest go into it three at a time, and the
//! receiver gets one link to that folder. Packing them into one ZIP is an option: then the ZIP
//! is built *while* uploading (no temporary copy, no "creating archive" wait), the same way
//! every time; stored only (no compression), its exact size is known before the upload starts.
//!
//! The first upload creates a Gofile guest account and returns its token; Pako keeps it
//! (encrypted with Windows, like the Google login) to delete the sender's files later.
//!
//! Sending with your own Google Drive instead is in senddrive.rs (its own sign-in).

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::engine::Engine;
use crate::notices::{Go, Pop};
use crate::google::Google;
use crate::paths::data_dir;

#[path = "senddrive.rs"]
mod drivesend;
#[path = "storage.rs"]
mod storage;
use storage::ExpiryBy;

/// Where a send goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    /// Gofile, no account (the default).
    Gofile,
    /// Your own Google Drive (the "Pako Send" sign-in).
    Drive,
}

impl Host {
    pub fn from_setting(s: &str) -> Host {
        if s == "drive" { Host::Drive } else { Host::Gofile }
    }
}

const UPLOAD_URL: &str = "https://upload.gofile.io/uploadfile";
const CONTENTS_URL: &str = "https://api.gofile.io/contents";
/// What Gofile does with guest files (their FAQ): kept while people download them.
pub const KEEP_NOTE: &str = "Gofile keeps files while people download them. After 10 days without a download, they may be removed.";
/// Gofile's keep rule: this long without a download and a file may be removed.
const KEEP_DAYS: u64 = 10;
/// "May be removed by Gofile soon" this many days before that.
const SOON_DAYS: u64 = 3;
const DAY_MS: u64 = 86_400_000;
/// "Delete after" choices offered (days; 0 = keep), and the usual one.
pub const DELETE_CHOICES: [u32; 4] = [0, 3, 7, 30];
pub const DELETE_DEFAULT: u32 = 7;
/// Files listed per sent item (the details panel); more are counted, not named.
const CONTENTS_MAX: usize = 1000;
/// How often Pako looks for uploads whose "Delete after" date has come.
const SWEEP_EVERY: Duration = Duration::from_secs(3600);
/// Download counts are asked for at most this often (opening Send) …
const CHECK_GAP: Duration = Duration::from_secs(3 * 60);
/// … or this often with the refresh button.
const CHECK_GAP_BUTTON: Duration = Duration::from_secs(60);
/// After Gofile said it doesn't share counts, opening Send doesn't ask again for this long.
const REFUSED_GAP: Duration = Duration::from_secs(6 * 3600);
/// Gofile's own limit on stored items.
const MAX_FILES: usize = 100_000;

fn now_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn sent_path() -> PathBuf {
    data_dir().join("sent.json")
}
fn token_path() -> PathBuf {
    data_dir().join("gofile_guest.bin")
}

// ---------- what is sent ----------

/// One file going into the upload: where it is, its name in the ZIP ("Folder/sub/file.txt").
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub mtime: SystemTime,
    /// An empty folder ("Folder/empty/"): kept in a ZIP so it unpacks the same.
    pub dir: bool,
}

#[derive(Debug)]
pub enum Payload {
    /// One file, uploaded as it is.
    File(Entry),
    /// Several files, each uploaded as it is into one Gofile folder (one link).
    Files { name: String, entries: Vec<Entry> },
    /// Several files or folders, packed into one ZIP on the fly.
    Zip { name: String, entries: Vec<Entry> },
}

impl Payload {
    pub fn name(&self) -> &str {
        match self {
            Payload::File(e) => &e.name,
            Payload::Files { name, .. } | Payload::Zip { name, .. } => name,
        }
    }
    pub fn files(&self) -> usize {
        match self {
            Payload::File(_) => 1,
            Payload::Files { entries, .. } | Payload::Zip { entries, .. } => entries.iter().filter(|e| !e.dir).count(),
        }
    }
    /// Bytes of the files themselves.
    pub fn input_size(&self) -> u64 {
        match self {
            Payload::File(e) => e.size,
            Payload::Files { entries, .. } | Payload::Zip { entries, .. } => entries.iter().map(|e| e.size).sum(),
        }
    }
}

/// Settings › Send › "When you send several files".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Each file as it is, into one folder (default).
    AsTheyAre,
    /// One ZIP, files only stored.
    Zip,
    /// One ZIP, compressed where it helps.
    Compress,
}

impl Mode {
    /// `mode` is the setting (or the one-off choice); without it, the older compression setting
    /// carries over (Auto / Maximum meant packing and compressing).
    pub fn from_settings(mode: Option<&str>, old_compression: &str) -> Mode {
        match mode {
            Some("zip") => Mode::Zip,
            Some("compress") => Mode::Compress,
            Some(_) => Mode::AsTheyAre,
            None if matches!(old_compression, "auto" | "max") => Mode::Compress,
            None => Mode::AsTheyAre,
        }
    }
    fn pack(self) -> Pack {
        match self {
            Mode::Compress => Pack::Max,
            _ => Pack::Off,
        }
    }
    /// The one-off choice that gives this mode again ("Try again" keeps it).
    fn as_str(self) -> &'static str {
        match self {
            Mode::AsTheyAre => "files",
            Mode::Zip => "zip",
            Mode::Compress => "compress",
        }
    }
}

/// "Photos/sub/beach.jpg" → "sub - beach.jpg" (one picked folder: its own name is left out,
/// the link already shows it) or "Photos - sub - beach.jpg". Gofile guest folders are kept flat:
/// whether a guest may make sub-folders isn't confirmed, so the path stays readable in the name.
fn flat_name(name: &str, drop_top: bool) -> String {
    let mut parts: Vec<&str> = name.split('/').filter(|p| !p.is_empty()).collect();
    if drop_top && parts.len() > 1 {
        parts.remove(0);
    }
    parts.join(" - ")
}

/// "a.txt" → "a (2).txt"; a folder ("Photos" or "Photos/") → "Photos (2)" / "Photos (2)/".
fn numbered(name: &str, n: usize, dir: bool) -> String {
    if dir {
        let base = name.strip_suffix('/').unwrap_or(name);
        let slash = if name.ends_with('/') { "/" } else { "" };
        return format!("{base} ({n}){slash}");
    }
    match name.rsplit_once('.') {
        Some((s, x)) if !s.is_empty() && !x.contains('/') => format!("{s} ({n}).{x}"),
        _ => format!("{name} ({n})"),
    }
}

/// Two items with the same name get " (2)", " (3)"… (case-insensitive, like Windows).
fn dedupe(entries: &mut [Entry]) {
    let mut seen = std::collections::HashSet::new();
    for e in entries.iter_mut() {
        let mut name = e.name.clone();
        let mut n = 2;
        while !seen.insert(name.to_lowercase()) {
            name = numbered(&e.name, n, e.dir);
            n += 1;
        }
        e.name = name;
    }
}

/// The names the picked items get at the top of a ZIP. A picked file and a picked folder with
/// the same name (or two folders) would clash inside it: the later one gets " (2)".
fn unique_tops(paths: &[PathBuf]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    paths
        .iter()
        .map(|p| {
            let base = file_name(p);
            let mut name = base.clone();
            let mut n = 2;
            while !seen.insert(name.to_lowercase()) {
                name = numbered(&base, n, p.is_dir());
                n += 1;
            }
            name
        })
        .collect()
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into())
}

/// Turns the picked files and folders into what gets uploaded.
/// A name you gave the send, made safe for Gofile, Google Drive and Windows: no path or reserved
/// characters, no leading/trailing dots or spaces, at most 120 characters. None = empty.
pub fn clean_send_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') { ' ' } else { c })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    let cleaned: String = cleaned.chars().take(120).collect();
    let cleaned = cleaned.trim_end_matches(|c: char| c == '.' || c == ' ').to_string();
    (!cleaned.is_empty()).then_some(cleaned)
}

impl Payload {
    /// Uses the name you gave the send: the folder's name (several files as they are) or the ZIP's
    /// name (".zip" added when missing). A single file keeps its own name.
    pub fn rename(&mut self, name: &str) {
        match self {
            Payload::File(_) => {}
            Payload::Files { name: n, .. } => *n = name.to_string(),
            Payload::Zip { name: n, .. } => {
                *n = if name.to_lowercase().ends_with(".zip") { name.to_string() } else { format!("{name}.zip") };
            }
        }
    }
}

/// One picked file or folder as the Send basket shows it: its size, and for a folder how many
/// files it holds. `error` = it couldn't be read (moved, no access...).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PathSize {
    pub path: String,
    pub dir: bool,
    pub size: u64,
    pub files: usize,
    pub error: Option<String>,
}

/// Sizes of picked files and folders (folders: everything inside, sub-folders included).
pub fn path_sizes(paths: &[String]) -> Vec<PathSize> {
    paths
        .iter()
        .map(|p| {
            let path = PathBuf::from(p);
            let mut out = PathSize { path: p.clone(), dir: path.is_dir(), size: 0, files: 0, error: None };
            if out.dir {
                let mut entries = Vec::new();
                match walk(&path, "", &mut entries) {
                    Ok(()) => {
                        let files: Vec<_> = entries.iter().filter(|e| !e.dir).collect();
                        out.files = files.len();
                        out.size = files.iter().map(|e| e.size).sum();
                    }
                    Err(e) => out.error = Some(format!("Couldn't read this folder ({e}).")),
                }
            } else {
                match std::fs::metadata(&path) {
                    Ok(m) => {
                        out.size = m.len();
                        out.files = 1;
                    }
                    Err(_) => out.error = Some("This file isn't there any more.".into()),
                }
            }
            out
        })
        .collect()
}

pub fn gather(paths: &[PathBuf], mode: Mode) -> Result<Payload> {
    if paths.is_empty() {
        bail!("Nothing was picked.");
    }
    // One file goes as it is, unless "Compressed ZIP" was picked for it (a plain ZIP of one
    // file would only add a step for the receiver).
    if paths.len() == 1 && paths[0].is_file() && mode != Mode::Compress {
        let m = std::fs::metadata(&paths[0])?;
        return Ok(Payload::File(Entry {
            path: paths[0].clone(),
            name: file_name(&paths[0]),
            size: m.len(),
            mtime: m.modified().unwrap_or(UNIX_EPOCH),
            dir: false,
        }));
    }
    let tops: Vec<String> = if mode == Mode::AsTheyAre { paths.iter().map(|p| file_name(p)).collect() } else { unique_tops(paths) };
    let mut entries = Vec::new();
    for (p, top) in paths.iter().zip(tops) {
        if p.is_dir() {
            walk(p, &top, &mut entries)?;
        } else if p.is_file() {
            let m = std::fs::metadata(p)?;
            entries.push(Entry { path: p.clone(), name: top, size: m.len(), mtime: m.modified().unwrap_or(UNIX_EPOCH), dir: false });
        }
        if entries.len() > MAX_FILES {
            bail!("That's more than {MAX_FILES} files: Gofile can't hold that many.");
        }
    }
    if !entries.iter().any(|e| !e.dir) {
        bail!("There are no files in what was picked.");
    }
    // Same order every time; two picked items with the same name get " (2)".
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.name.cmp(&b.name)));
    dedupe(&mut entries);
    if mode == Mode::AsTheyAre {
        // A Gofile guest folder is flat: an empty folder has no place in it.
        entries.retain(|e| !e.dir);
        let one_folder = paths.len() == 1 && paths[0].is_dir();
        for e in entries.iter_mut() {
            e.name = flat_name(&e.name, one_folder);
        }
        dedupe(&mut entries);
        let n = entries.len();
        let name = if one_folder { file_name(&paths[0]) } else { format!("{n} files") };
        return Ok(Payload::Files { name, entries });
    }
    let name = if paths.len() == 1 {
        format!("{}.zip", file_name(&paths[0]))
    } else {
        let first = file_name(&paths[0]);
        let stem = first.rsplit_once('.').map(|(s, _)| s.to_string()).filter(|s| !s.is_empty()).unwrap_or(first);
        format!("{stem} and {} more.zip", paths.len() - 1)
    };
    Ok(Payload::Zip { name, entries })
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<Entry>) -> Result<()> {
    let mut items: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    items.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    let before = out.len();
    for it in items {
        let ft = match it.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let name = format!("{prefix}/{}", it.file_name().to_string_lossy());
        if ft.is_dir() {
            walk(&it.path(), &name, out)?;
        } else if ft.is_file() {
            let m = it.metadata()?;
            out.push(Entry { path: it.path(), name, size: m.len(), mtime: m.modified().unwrap_or(UNIX_EPOCH), dir: false });
        }
        if out.len() > MAX_FILES {
            bail!("That's more than {MAX_FILES} files: Gofile can't hold that many.");
        }
    }
    if out.len() == before {
        // Nothing inside: the folder itself goes in, so it isn't lost.
        let mtime = std::fs::metadata(dir).and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
        out.push(Entry { path: dir.to_path_buf(), name: format!("{prefix}/"), size: 0, mtime, dir: true });
    }
    Ok(())
}

// ---------- compression ----------

/// How a ZIP is made (Settings › Send: "Pack them into one ZIP" = Off, "Pack and compress" = Max).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pack {
    /// Store only (fastest, default).
    Off,
    /// Compress what shrinks (documents, text, raw pictures…), store the rest. (Not offered
    /// at the moment: "Pack and compress" uses Max; kept for a lighter choice later.)
    #[allow(dead_code)]
    Auto,
    /// Compress everything that isn't already compressed, harder.
    Max,
}

/// Already squeezed: compressing these again only costs time.
const COMPRESSED: &[&str] = &[
    "zip", "rar", "7z", "gz", "xz", "bz2", "zst", "tgz", "cab", "jpg", "jpeg", "png", "gif", "webp", "heic", "avif",
    "mp4", "mkv", "mov", "avi", "webm", "m4v", "wmv", "mp3", "m4a", "aac", "ogg", "opus", "flac", "docx", "xlsx",
    "pptx", "odt", "ods", "epub", "apk", "jar", "pdf",
];
/// Shrink well (worth it in Auto).
const SHRINKS: &[&str] = &[
    "txt", "csv", "tsv", "json", "xml", "html", "htm", "css", "js", "ts", "md", "log", "svg", "psd", "ai", "doc",
    "xls", "ppt", "rtf", "sql", "bmp", "tif", "tiff", "dng", "cr2", "cr3", "nef", "arw", "raf", "orf", "wav", "aif",
    "aiff", "iso", "img", "obj", "fbx", "blend", "dll", "exe", "ini", "yaml", "yml", "toml",
];

/// How one file is put in the ZIP: None = stored, Some(level) = deflated.
fn level_for(pack: Pack, name: &str) -> Option<u32> {
    let ext = name.rsplit_once('.').map(|(_, x)| x.to_ascii_lowercase()).unwrap_or_default();
    match pack {
        Pack::Off => None,
        Pack::Auto => SHRINKS.contains(&ext.as_str()).then_some(6),
        Pack::Max => (!COMPRESSED.contains(&ext.as_str())).then_some(9),
    }
}

/// `level_for` for one entry: an empty folder is always just stored.
fn level_of(pack: Pack, e: &Entry) -> Option<u32> {
    if e.dir { None } else { level_for(pack, &e.name) }
}

// ---------- the ZIP, written as a stream ----------
//
// A plain ZIP (opens in Windows / macOS with nothing to install) written front to back: each
// file's checksum goes in a small record right after it, so nothing has to be rewritten.
// ZIP64 records are used only where sizes or positions need them.

/// Above this, a file needs ZIP64 records (4 GB, with a small margin).
const Z64_AT: u64 = 0xFFFF_0000;
const MAX32: u64 = 0xFFFF_FFFF;
const FLAGS: u16 = 0x0008 | 0x0800; // checksum after the data · UTF-8 names

/// The most a deflated file can grow to (zlib's deflateBound, rounded up a little): files that
/// don't shrink come out slightly bigger than they went in.
fn deflate_bound(n: u64) -> u64 {
    n + (n >> 12) + (n >> 14) + (n >> 25) + 64
}

/// Whether a file gets ZIP64 records: decided before it's written, so a deflated file just under
/// 4 GB that grows past it is still described correctly.
fn needs_z64(size: u64, deflate: bool, z64_at: u64) -> bool {
    size >= z64_at || (deflate && deflate_bound(size) >= z64_at)
}

/// A date and time: year, month, day, hour, minute, second.
type Civil = (i64, i64, i64, i64, i64, i64);

/// The UTC date and time of seconds since 1970.
fn civil_utc(secs: i64) -> Civil {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d, rem / 3600, (rem % 3600) / 60, rem % 60)
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct SysTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    millis: u16,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn SystemTimeToTzSpecificLocalTime(tz: *const std::ffi::c_void, utc: *const SysTime, local: *mut SysTime) -> i32;
}

/// The same moment in this PC's time zone (summer time included), like Explorer shows it.
/// Falls back to UTC if Windows can't say.
fn to_local(c: Civil) -> Civil {
    #[cfg(windows)]
    {
        if (1601..=30_827).contains(&c.0) {
            let utc = SysTime {
                year: c.0 as u16,
                month: c.1 as u16,
                day: c.2 as u16,
                hour: c.3 as u16,
                minute: c.4 as u16,
                second: c.5 as u16,
                ..Default::default()
            };
            let mut local = SysTime::default();
            // SAFETY: both point to valid SYSTEMTIME-shaped values; null = the current time zone.
            if unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) } != 0 {
                return (local.year as i64, local.month as i64, local.day as i64, local.hour as i64, local.minute as i64, local.second as i64);
            }
        }
    }
    c
}

/// ZIP (MS-DOS) time and date. ZIP times have no time zone: they're read as local time, so
/// they're written in local time too.
fn dos_time(t: SystemTime) -> (u16, u16) {
    let secs = t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    dos_pack(to_local(civil_utc(secs)))
}

fn dos_pack((y, m, d, hh, mm, ss): Civil) -> (u16, u16) {
    if y < 1980 {
        return (0, (1 << 5) | 1);
    }
    let y = y.min(2107);
    let date = (((y - 1980) as u16) << 9) | ((m as u16) << 5) | d as u16;
    let time = ((hh as u16) << 11) | ((mm as u16) << 5) | (ss / 2) as u16;
    (time, date)
}

struct Placed {
    name: Vec<u8>,
    time: u16,
    date: u16,
    deflate: bool,
    dir: bool,
    crc: u32,
    csize: u64,
    usize: u64,
    offset: u64,
    z64_local: bool,
}

fn local_len(name_len: usize, z64: bool) -> u64 {
    30 + name_len as u64 + if z64 { 20 } else { 0 }
}
fn descriptor_len(z64: bool) -> u64 {
    if z64 { 24 } else { 16 }
}
fn central_len(name_len: usize, needs64: bool) -> u64 {
    46 + name_len as u64 + if needs64 { 28 } else { 0 }
}
fn central_needs64(csize: u64, usize: u64, offset: u64, z64_local: bool) -> bool {
    z64_local || csize >= MAX32 || usize >= MAX32 || offset >= MAX32
}
fn end_needs64(count: usize, cd_start: u64, cd_size: u64) -> bool {
    count >= 0xFFFF || cd_start >= MAX32 || cd_size >= MAX32
}

/// The exact size of the ZIP, when every file is only stored (else unknown until it's built).
pub fn zip_size(entries: &[Entry], pack: Pack, z64_at: u64) -> Option<u64> {
    let mut off = 0u64;
    let mut placed = Vec::with_capacity(entries.len());
    for e in entries {
        if level_of(pack, e).is_some() {
            return None;
        }
        let z64 = needs_z64(e.size, false, z64_at);
        let start = off;
        off += local_len(e.name.len(), z64) + e.size + descriptor_len(z64);
        placed.push((e.name.len(), e.size, start, z64));
    }
    let cd_start = off;
    for (nl, size, start, z64) in &placed {
        off += central_len(*nl, central_needs64(*size, *size, *start, *z64));
    }
    let cd_size = off - cd_start;
    if end_needs64(placed.len(), cd_start, cd_size) {
        off += 56 + 20;
    }
    Some(off + 22)
}

/// Counts what goes through, so positions in the ZIP are known.
struct Counted<'a, W: Write> {
    inner: &'a mut W,
    n: u64,
}
impl<W: Write> Write for Counted<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let k = self.inner.write(buf)?;
        self.n += k as u64;
        Ok(k)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn changed(name: &str) -> io::Error {
    io::Error::other(format!("\"{name}\" changed while it was being uploaded. Try again."))
}

/// Writes the ZIP of `entries` into `out`. `stop` ends it early (Cancel).
pub fn write_zip<W: Write>(entries: &[Entry], pack: Pack, out: &mut W, stop: &AtomicBool, z64_at: u64) -> io::Result<()> {
    let mut w = Counted { inner: out, n: 0 };
    let mut placed: Vec<Placed> = Vec::with_capacity(entries.len());
    let mut buf = vec![0u8; 256 * 1024];
    for e in entries {
        if stop.load(Ordering::Relaxed) {
            return Err(io::Error::other("cancelled"));
        }
        let level = level_of(pack, e);
        let deflate = level.is_some();
        let z64 = needs_z64(e.size, deflate, z64_at);
        let (time, date) = dos_time(e.mtime);
        let name = e.name.as_bytes().to_vec();
        let offset = w.n;
        // Local header. Stored files carry their (known) size; deflated ones get it afterwards.
        let (c32, u32_) = match (deflate, z64) {
            (_, true) => (MAX32 as u32, MAX32 as u32),
            (false, false) => (e.size as u32, e.size as u32),
            (true, false) => (0, 0),
        };
        let mut h = Vec::with_capacity(64 + name.len());
        h.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        h.extend_from_slice(&(if z64 { 45u16 } else { 20u16 }).to_le_bytes());
        h.extend_from_slice(&FLAGS.to_le_bytes());
        h.extend_from_slice(&(if deflate { 8u16 } else { 0u16 }).to_le_bytes());
        h.extend_from_slice(&time.to_le_bytes());
        h.extend_from_slice(&date.to_le_bytes());
        h.extend_from_slice(&0u32.to_le_bytes()); // checksum: in the record after the data
        h.extend_from_slice(&c32.to_le_bytes());
        h.extend_from_slice(&u32_.to_le_bytes());
        h.extend_from_slice(&(name.len() as u16).to_le_bytes());
        h.extend_from_slice(&(if z64 { 20u16 } else { 0u16 }).to_le_bytes());
        h.extend_from_slice(&name);
        if z64 {
            let known = if deflate { 0 } else { e.size };
            h.extend_from_slice(&1u16.to_le_bytes());
            h.extend_from_slice(&16u16.to_le_bytes());
            h.extend_from_slice(&known.to_le_bytes());
            h.extend_from_slice(&known.to_le_bytes());
        }
        w.write_all(&h)?;

        // The file itself (an empty folder has nothing more).
        let mut crc = crc32fast::Hasher::new();
        let mut read = 0u64;
        let start = w.n;
        if e.dir {
            // Just the name, ending in "/".
        } else if let Some(level) = level {
            let mut src = BufReader::with_capacity(1024 * 1024, File::open(&e.path)?);
            let mut enc = flate2::write::DeflateEncoder::new(&mut w, flate2::Compression::new(level));
            loop {
                if stop.load(Ordering::Relaxed) {
                    return Err(io::Error::other("cancelled"));
                }
                let k = src.read(&mut buf)?;
                if k == 0 {
                    break;
                }
                crc.update(&buf[..k]);
                read += k as u64;
                enc.write_all(&buf[..k])?;
            }
            enc.finish()?;
        } else {
            let mut src = BufReader::with_capacity(1024 * 1024, File::open(&e.path)?);
            loop {
                if stop.load(Ordering::Relaxed) {
                    return Err(io::Error::other("cancelled"));
                }
                let want = (e.size - read).min(buf.len() as u64) as usize;
                if want == 0 {
                    break;
                }
                let k = src.read(&mut buf[..want])?;
                if k == 0 {
                    break;
                }
                crc.update(&buf[..k]);
                read += k as u64;
                w.write_all(&buf[..k])?;
            }
            // The size was promised up front: a file that changed can't be sent as is.
            if read != e.size || src.read(&mut buf[..1])? != 0 {
                return Err(changed(&e.name));
            }
        }
        if read != e.size && !deflate {
            return Err(changed(&e.name));
        }
        let csize = w.n - start;
        let crc = crc.finalize();
        // A deflated file that grew past the ZIP64 limit without ZIP64 records can't be described.
        if !z64 && (csize >= MAX32 || read >= MAX32) {
            return Err(changed(&e.name));
        }
        let mut d = Vec::with_capacity(24);
        d.extend_from_slice(&0x0807_4b50u32.to_le_bytes());
        d.extend_from_slice(&crc.to_le_bytes());
        if z64 {
            d.extend_from_slice(&csize.to_le_bytes());
            d.extend_from_slice(&read.to_le_bytes());
        } else {
            d.extend_from_slice(&(csize as u32).to_le_bytes());
            d.extend_from_slice(&(read as u32).to_le_bytes());
        }
        w.write_all(&d)?;
        placed.push(Placed { name, time, date, deflate, dir: e.dir, crc, csize, usize: read, offset, z64_local: z64 });
    }

    // Table of contents.
    let cd_start = w.n;
    for p in &placed {
        let n64 = central_needs64(p.csize, p.usize, p.offset, p.z64_local);
        let mut c = Vec::with_capacity(80 + p.name.len());
        c.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        c.extend_from_slice(&45u16.to_le_bytes()); // made by (MS-DOS, spec 4.5)
        c.extend_from_slice(&(if n64 { 45u16 } else { 20u16 }).to_le_bytes());
        c.extend_from_slice(&FLAGS.to_le_bytes());
        c.extend_from_slice(&(if p.deflate { 8u16 } else { 0u16 }).to_le_bytes());
        c.extend_from_slice(&p.time.to_le_bytes());
        c.extend_from_slice(&p.date.to_le_bytes());
        c.extend_from_slice(&p.crc.to_le_bytes());
        let (cs, us, of) = if n64 { (MAX32, MAX32, MAX32) } else { (p.csize, p.usize, p.offset) };
        c.extend_from_slice(&(cs as u32).to_le_bytes());
        c.extend_from_slice(&(us as u32).to_le_bytes());
        c.extend_from_slice(&(p.name.len() as u16).to_le_bytes());
        c.extend_from_slice(&(if n64 { 28u16 } else { 0u16 }).to_le_bytes());
        c.extend_from_slice(&0u16.to_le_bytes()); // comment
        c.extend_from_slice(&0u16.to_le_bytes()); // disk
        c.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        c.extend_from_slice(&(if p.dir { 0x10u32 } else { 0 }).to_le_bytes()); // external attributes (folder)
        c.extend_from_slice(&(of as u32).to_le_bytes());
        c.extend_from_slice(&p.name);
        if n64 {
            c.extend_from_slice(&1u16.to_le_bytes());
            c.extend_from_slice(&24u16.to_le_bytes());
            c.extend_from_slice(&p.usize.to_le_bytes());
            c.extend_from_slice(&p.csize.to_le_bytes());
            c.extend_from_slice(&p.offset.to_le_bytes());
        }
        w.write_all(&c)?;
    }
    let cd_size = w.n - cd_start;
    let count = placed.len();
    let big = end_needs64(count, cd_start, cd_size);
    let mut end = Vec::with_capacity(98);
    if big {
        let z64_eocd = w.n;
        end.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        end.extend_from_slice(&44u64.to_le_bytes());
        end.extend_from_slice(&45u16.to_le_bytes());
        end.extend_from_slice(&45u16.to_le_bytes());
        end.extend_from_slice(&0u32.to_le_bytes());
        end.extend_from_slice(&0u32.to_le_bytes());
        end.extend_from_slice(&(count as u64).to_le_bytes());
        end.extend_from_slice(&(count as u64).to_le_bytes());
        end.extend_from_slice(&cd_size.to_le_bytes());
        end.extend_from_slice(&cd_start.to_le_bytes());
        end.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
        end.extend_from_slice(&0u32.to_le_bytes());
        end.extend_from_slice(&z64_eocd.to_le_bytes());
        end.extend_from_slice(&1u32.to_le_bytes());
    }
    let n16 = if big { 0xFFFF } else { count as u16 };
    end.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    end.extend_from_slice(&0u16.to_le_bytes());
    end.extend_from_slice(&0u16.to_le_bytes());
    end.extend_from_slice(&n16.to_le_bytes());
    end.extend_from_slice(&n16.to_le_bytes());
    end.extend_from_slice(&(if big { MAX32 as u32 } else { cd_size as u32 }).to_le_bytes());
    end.extend_from_slice(&(if big { MAX32 as u32 } else { cd_start as u32 }).to_le_bytes());
    end.extend_from_slice(&0u16.to_le_bytes());
    w.write_all(&end)?;
    w.flush()
}

// ---------- streaming into the upload ----------

/// Hands bytes to the upload in 256 KB pieces. The queue is short, so packing never runs far
/// ahead of the upload (and memory stays small). Stops when the upload is gone.
struct ToUpload {
    tx: mpsc::Sender<io::Result<Bytes>>,
    buf: Vec<u8>,
    produced: Arc<AtomicU64>,
    /// This attempt's bytes only (taken back off `produced` if the attempt fails).
    mine: Arc<AtomicU64>,
}
impl ToUpload {
    fn send(&mut self) -> io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let chunk = Bytes::from(std::mem::replace(&mut self.buf, Vec::with_capacity(256 * 1024)));
        let n = chunk.len() as u64;
        self.tx.blocking_send(Ok(chunk)).map_err(|_| io::Error::other("upload stopped"))?;
        self.produced.fetch_add(n, Ordering::Relaxed);
        self.mine.fetch_add(n, Ordering::Relaxed);
        Ok(())
    }
}
impl Write for ToUpload {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        if self.buf.len() >= 256 * 1024 {
            self.send()?;
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.send()
    }
}

// ---------- Gofile ----------

#[derive(Deserialize)]
struct UploadReply {
    status: String,
    data: Option<UploadData>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadData {
    download_page: Option<String>,
    code: Option<String>,
    parent_folder: Option<String>,
    file_id: Option<String>,
    guest_token: Option<String>,
}

fn net_error(e: &reqwest::Error) -> String {
    if e.is_connect() {
        "No connection to Gofile. Check your internet, then try again.".into()
    } else if e.is_timeout() {
        "Gofile stopped answering. Try again in a few minutes.".into()
    } else {
        "The upload was cut off. Try again.".into()
    }
}

fn status_error(code: u16, body: &str) -> String {
    match code {
        413 => "That's too big for Gofile.".into(),
        429 => "Gofile asks to slow down. Try again in a few minutes.".into(),
        401 | 403 => "Gofile refused the upload. Try again; if it keeps happening, Gofile may have changed its rules.".into(),
        c if c >= 500 => "Gofile is busy right now. Try again in a few minutes.".into(),
        c => {
            let status = serde_json::from_str::<UploadReply>(body).map(|r| r.status).unwrap_or_default();
            if status.is_empty() { format!("Gofile refused the upload ({c}).") } else { format!("Gofile refused the upload ({status}).") }
        }
    }
}

/// The upload request itself: one multipart form with the file streamed from `rx`. With the
/// exact size known, the request says so up front (Content-Length); otherwise it is sent in chunks.
async fn post_upload(
    http: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    folder: Option<&str>,
    name: &str,
    mut rx: mpsc::Receiver<io::Result<Bytes>>,
    total: Option<u64>,
) -> std::result::Result<(u16, String), reqwest::Error> {
    let stream = futures_util::stream::poll_fn(move |cx| rx.poll_recv(cx));
    let body = reqwest::Body::wrap_stream(stream);
    let part = match total {
        Some(len) => reqwest::multipart::Part::stream_with_length(body, len),
        None => reqwest::multipart::Part::stream(body),
    }
    .file_name(name.to_string())
    .mime_str("application/octet-stream")
    .expect("a fixed, valid type");
    let mut form = reqwest::multipart::Form::new();
    // Into an existing folder (the rest of a several-files send): needs the same guest key.
    if let Some(f) = folder {
        form = form.text("folderId", f.to_string());
    }
    let form = form.part("file", part);
    let mut req = http.post(url).multipart(form);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await?;
    let code = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    Ok((code, text))
}

// ---------- the Sender ----------

/// Files sent before, with their link (data/sent.json).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentItem {
    pub id: String,
    pub name: String,
    /// Bytes uploaded.
    pub size: u64,
    pub files: usize,
    /// When it was sent (ms).
    pub date: u64,
    pub link: String,
    pub folder_id: Option<String>,
    pub file_id: Option<String>,
    pub compressed: bool,
    /// Several files sent as they are (one Gofile folder), not one ZIP.
    #[serde(default)]
    pub as_files: bool,
    /// Not all files are up: the send was cancelled, failed or Pako closed during it. The files
    /// that are up are on Gofile all the same, so they stay on the list (and can be deleted).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub incomplete: bool,
    /// The Gofile guest key it was sent with (encrypted with Windows), to delete it later.
    /// Older items have none: the key Pako keeps for new sends is used for them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// How many times it was downloaded, as Gofile said at the last check (None = not known).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<u64>,
    /// When Pako saw that count go up (ms): the last download was about then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloaded_at: Option<u64>,
    /// Gofile says it's no longer there (deleted or expired).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gone: bool,
    /// Where it was sent: "drive" for your Google Drive; none = Gofile (older items too).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// When it gets deleted ("Delete after"), ms. None = kept (older items too).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_at: Option<u64>,
    /// The service deletes it itself on time (Gofile folders), even with the PC off.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub expiry_on_host: bool,
    /// Why Pako couldn't delete it yet (tried again every hour).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_error: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub delete_tries: u32,
    /// Your note ("for the client, v2 renders").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The files inside (names, sizes), up to CONTENTS_MAX.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contents: Vec<SentFile>,
    /// Seconds spent uploading and the top speed (bytes/s), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secs: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peak: Option<f64>,
    /// Download counts seen over time: (when, count), each change once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub count_history: Vec<(u64, u64)>,
    /// The Google account a Drive upload went into: deleting it needs that account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

/// One file inside a sent item.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SentFile {
    pub name: String,
    pub size: u64,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// When an upload gets deleted: `days` after `from` (0 = kept).
pub fn delete_at(from: u64, days: u32) -> Option<u64> {
    (days > 0).then(|| from + days as u64 * DAY_MS)
}

/// The files of a send, for its details (names relative to what was sent).
fn contents_of(payload: &Payload) -> Vec<SentFile> {
    let list: Vec<&Entry> = match payload {
        Payload::File(e) => vec![e],
        Payload::Files { entries, .. } | Payload::Zip { entries, .. } => entries.iter().filter(|e| !e.dir).collect(),
    };
    list.into_iter().take(CONTENTS_MAX).map(|e| SentFile { name: e.name.clone(), size: e.size }).collect()
}

impl SentItem {
    pub fn on_drive(&self) -> bool {
        self.host.as_deref() == Some("drive")
    }
}

/// A Sent item as the window sees it (without its key).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SentView {
    #[serde(flatten)]
    item: SentItem,
    /// Pako has the Gofile key for it, so it can be deleted from Gofile.
    can_delete: bool,
    /// Close to Gofile's keep limit: "May be removed by Gofile soon".
    soon: bool,
}

/// Close to Gofile's limit (10 days without a download): counted from the last download Pako
/// saw, else from when it was sent. Only when the count is known.
fn removal_soon(s: &SentItem, now: u64) -> bool {
    if s.gone || s.downloads.is_none() {
        return false;
    }
    let since = s.downloaded_at.unwrap_or(s.date).max(s.date);
    now.saturating_sub(since) >= (KEEP_DAYS - SOON_DAYS) * DAY_MS
}

// ---------- download counts ----------

/// What Gofile said, on one Sent item.
fn apply_to(s: &mut SentItem, remote: Remote, now: u64) {
    match remote {
        Remote::Found(n) => {
            // Only a count Pako saw go up says when the last download was.
            if s.downloads.is_some_and(|old| n > old) {
                s.downloaded_at = Some(now);
            }
            if s.downloads != Some(n) {
                s.count_history.push((now, n));
                if s.count_history.len() > 200 {
                    s.count_history.remove(0);
                }
            }
            s.downloads = Some(n);
            s.gone = false;
        }
        Remote::Gone => s.gone = true,
        Remote::Refused | Remote::Unknown => {}
    }
}

/// What Gofile says about one sent item.
#[derive(Debug, PartialEq, Eq)]
enum Remote {
    /// Still there, downloaded this many times (all its files together).
    Found(u64),
    /// Deleted or expired.
    Gone,
    /// Gofile doesn't share this with a free (guest) account.
    Refused,
    /// Busy, slow down, cut off, or an answer Pako doesn't understand: try another time.
    Unknown,
}

/// Download count of one content item: a file's own `downloadCount`, or for a folder the sum
/// over its `children` (a map by id, or a list), or its `totalDownloadCount` when that's bigger.
fn count_downloads(v: &serde_json::Value) -> u64 {
    let own = v.get("downloadCount").and_then(|n| n.as_u64()).unwrap_or(0);
    let children: Vec<&serde_json::Value> = match v.get("children") {
        Some(serde_json::Value::Object(m)) => m.values().collect(),
        Some(serde_json::Value::Array(a)) => a.iter().collect(),
        _ => Vec::new(),
    };
    let sum: u64 = children.iter().map(|c| count_downloads(c)).sum();
    let total = v.get("totalDownloadCount").and_then(|n| n.as_u64()).unwrap_or(0);
    own.max(sum).max(total)
}

/// Reads Gofile's answer to `GET /contents/{id}`.
fn parse_contents(code: u16, body: &str) -> Remote {
    let v: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let status = v.as_ref().and_then(|v| v.get("status")).and_then(|s| s.as_str()).unwrap_or("");
    match status {
        "ok" => {
            return match v.as_ref().and_then(|v| v.get("data")) {
                Some(d) if d.is_object() => Remote::Found(count_downloads(d)),
                _ => Remote::Unknown,
            };
        }
        "error-notFound" => return Remote::Gone,
        "error-notPremium" | "error-notAuthorized" | "error-notPublic" => return Remote::Refused,
        _ => {}
    }
    match code {
        404 => Remote::Gone,
        401 | 403 => Remote::Refused,
        _ => Remote::Unknown,
    }
}

/// Asks Gofile about one item (`base` = the contents address; a local stand-in in tests).
async fn fetch_remote(http: &reqwest::Client, base: &str, id: &str, token: &str) -> Remote {
    let req = http.get(format!("{base}/{id}")).bearer_auth(token).timeout(Duration::from_secs(20));
    match req.send().await {
        Ok(resp) => {
            let code = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            parse_contents(code, &text)
        }
        Err(_) => Remote::Unknown,
    }
}

/// The Gofile key a sent item was sent with (older items: the key Pako keeps).
fn token_for(item: &SentItem, shared: Option<String>) -> Option<String> {
    item.key.as_deref().and_then(unprotect_key).or(shared)
}

fn protect_key(key: &str) -> Option<String> {
    use base64::Engine as _;
    crate::secure::protect(key.as_bytes()).ok().map(|b| base64::engine::general_purpose::STANDARD.encode(b))
}
fn unprotect_key(s: &str) -> Option<String> {
    use base64::Engine as _;
    let b = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
    String::from_utf8(crate::secure::unprotect(&b).ok()?).ok().filter(|t| !t.is_empty())
}

enum Phase {
    Uploading,
    Failed(String),
}

/// Link, folder id, file id.
type Done = (String, Option<String>, Option<String>);

struct Job {
    id: String,
    name: String,
    files: usize,
    input: u64,
    /// Exact upload size when known (one file, or a ZIP of stored files).
    total: Option<u64>,
    produced: Arc<AtomicU64>,
    phase: Mutex<Phase>,
    cancel: CancellationToken,
    paths: Vec<String>,
    /// How several files were sent ("Try again" sends them the same way).
    mode: Mode,
    /// (when, bytes) of the last speed sample, and the smoothed speed.
    sample: Mutex<(Instant, u64, f64)>,
    /// Seconds spent uploading and the top (smoothed) speed, for the Dashboard.
    timing: Mutex<(f64, f64)>,
    /// Several files sent as they are.
    as_files: bool,
    files_done: AtomicUsize,
    /// The Gofile folder they go into, and its link (known after the first file).
    folder: Mutex<Option<(String, String)>>,
    /// Files that didn't upload: "Try again" sends just these into the same folder.
    left: Mutex<Vec<Entry>>,
    /// This send's own Gofile guest key (known after the first file is up).
    key: Mutex<Option<String>>,
    host: Host,
    /// "Delete after" for this send (days; 0 = keep).
    delete_days: u32,
    /// The files inside, for the details panel.
    contents: Vec<SentFile>,
    /// The Google account a Drive send goes into (known once it starts).
    account: Mutex<Option<String>>,
    /// Carried on with "Try again": Cancel then keeps what the earlier runs put up.
    resumed: AtomicBool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub id: String,
    pub name: String,
    pub files: usize,
    /// Bytes of the files themselves.
    pub size: u64,
    pub total: Option<u64>,
    pub sent: u64,
    pub speed: u64,
    /// "uploading" or "failed".
    pub status: &'static str,
    pub message: Option<String>,
    pub paths: Vec<String>,
    pub as_files: bool,
    pub files_done: usize,
    /// The folder's link, once the first file is up.
    pub link: Option<String>,
    /// "Try again" sends only the files that didn't upload.
    pub can_resume: bool,
    /// "files" / "zip" / "compress": how "Try again" sends it again.
    pub mode: &'static str,
    /// "gofile" or "drive".
    pub host: &'static str,
    /// Its "Delete after" (days; 0 = keep): "Try again" keeps it.
    pub delete_days: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendState {
    pub jobs: Vec<JobView>,
    pub sent: Vec<SentView>,
    /// Pako holds a Gofile guest key, so sent files can be deleted from Gofile.
    pub can_delete: bool,
    pub keep_note: &'static str,
    /// Download counts are being asked for right now.
    pub checking: bool,
    /// When they were last asked for (ms).
    pub checked_at: Option<u64>,
    /// Set when Gofile didn't share the counts at the last check.
    pub counts_note: Option<&'static str>,
    /// What Pako's sends take in your Google Drive (items still on the Sent list), and how many.
    pub drive_used: u64,
    pub drive_sends: usize,
    /// Something in the list of uploads couldn't be read (said once in the Upload tab).
    pub list_note: Option<String>,
    /// The Google accounts the Drive uploads on the list went into.
    pub drive_accounts: Vec<String>,
}

/// Your Google Drive for sending, as the Send tab shows it.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DriveSendStatus {
    /// This PC has the "Pako Send" registration (else connecting can't work).
    pub configured: bool,
    /// Said when it isn't.
    pub missing_note: Option<&'static str>,
    pub signed_in: bool,
    pub email: Option<String>,
    /// None = no limit (some Workspace accounts) or not known.
    pub limit: Option<u64>,
    pub used: Option<u64>,
    /// Google couldn't be asked, in plain words.
    pub error: Option<String>,
}

const REFUSED_NOTE: &str = "Gofile didn't share download counts with Pako this time.";

#[derive(Default)]
struct Checks {
    running: bool,
    last: Option<Instant>,
    last_ms: Option<u64>,
    refused: bool,
}

struct Inner {
    jobs: Vec<Arc<Job>>,
    sent: Vec<SentItem>,
    /// The guest key new sends start with (also the key of older Sent items without their own).
    token: Option<String>,
    ticking: bool,
}

pub struct Sender {
    app: AppHandle,
    http: reqwest::Client,
    /// The "Pako Send" Google sign-in (separate from the download one).
    pub send_google: Arc<Google>,
    inner: Mutex<Inner>,
    /// Held across copy, write and rename of sent.json, so two saves can't undo each other.
    save: Mutex<()>,
    /// sent.json couldn't be read at all: it isn't written over.
    locked: AtomicBool,
    list_note: Mutex<Option<String>>,
    /// Uploads being deleted or getting a new date right now.
    changing: Claims,
    checks: Mutex<Checks>,
}

/// One change at a time per upload (deleting it, giving it a new date): a second one is turned
/// away while the first is out, so neither undoes the other.
#[derive(Default)]
struct Claims(Mutex<HashSet<String>>);

/// Held while an upload is being changed; letting go frees it.
struct Claim<'a>(&'a Claims, String);

impl Claims {
    fn take(&self, id: &str) -> Option<Claim<'_>> {
        self.0.lock().unwrap().insert(id.to_string()).then(|| Claim(self, id.to_string()))
    }
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        self.0.0.lock().unwrap().remove(&self.1);
    }
}

const BUSY_ITEM: &str = "Pako is changing this upload right now. Try again in a moment.";

/// What reading sent.json gave.
#[derive(Debug, Default)]
struct Loaded {
    items: Vec<SentItem>,
    /// Said in the Upload tab when something couldn't be read.
    note: Option<String>,
    /// The file couldn't be read at all: Pako leaves it as it is.
    locked: bool,
}

/// Reads the list of uploads item by item, so one damaged entry doesn't lose the others. A file
/// with something unreadable is copied aside first (sent.json.bad-<time>), so saving the list
/// again loses nothing; a file that can't be opened at all isn't written over.
fn load_sent(path: &Path) -> Loaded {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Loaded::default(),
        Err(_) => {
            return Loaded {
                locked: true,
                note: Some("Pako couldn't open your list of uploads, so it shows none and leaves the list as it is. Restart Pako to try again.".into()),
                ..Default::default()
            };
        }
    };
    if text.trim().is_empty() {
        return Loaded::default();
    }
    let (items, bad) = match serde_json::from_str::<Vec<serde_json::Value>>(&text) {
        Ok(list) => {
            let n = list.len();
            let items: Vec<SentItem> = list.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect();
            let bad = n - items.len();
            (items, Some(bad))
        }
        Err(_) => (Vec::new(), None),
    };
    if bad == Some(0) {
        return Loaded { items, ..Default::default() };
    }
    let aside = path.with_extension(format!("json.bad-{}", now_millis()));
    if std::fs::copy(path, &aside).is_err() {
        return Loaded {
            items,
            locked: true,
            note: Some("Part of your list of uploads couldn't be read, and Pako couldn't keep a copy of it, so it leaves the list as it is. Restart Pako to try again.".into()),
        };
    }
    let kept = aside.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let note = match bad {
        None => format!("Pako couldn't read your list of uploads, so it starts a new one. The old list is kept as {kept} in Pako's data folder."),
        Some(1) => format!("1 upload in your list couldn't be read, so it isn't shown. The old list is kept as {kept} in Pako's data folder."),
        Some(n) => format!("{n} uploads in your list couldn't be read, so they aren't shown. The old list is kept as {kept} in Pako's data folder."),
    };
    Loaded { items, note: Some(note), locked: false }
}

/// The Google accounts the Drive uploads still on the list went into (each once).
fn drive_accounts(sent: &[SentItem]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for a in sent.iter().filter(|s| s.on_drive() && !s.gone).filter_map(|s| s.account.as_ref()) {
        if !out.iter().any(|o| o.eq_ignore_ascii_case(a)) {
            out.push(a.clone());
        }
    }
    out
}

/// Where an upload's bytes come from.
enum Source {
    File(Entry),
    Zip(Vec<Entry>, Pack),
}

/// Why one upload attempt didn't work ("cancelled" when the send was cancelled).
struct Fail {
    msg: String,
    /// Gofile's answer, when it answered.
    code: Option<u16>,
}

impl Fail {
    fn msg(msg: impl Into<String>) -> Self {
        Fail { msg: msg.into(), code: None }
    }
    fn cancelled() -> Self {
        Fail::msg("cancelled")
    }
    fn is_cancelled(&self) -> bool {
        self.msg == "cancelled"
    }
}

/// One file straight from disk; it must keep the size it had when it was picked.
fn copy_file<W: Write>(e: &Entry, out: &mut W, stop: &AtomicBool) -> io::Result<()> {
    let mut src = BufReader::with_capacity(1024 * 1024, File::open(&e.path)?);
    let mut buf = vec![0u8; 256 * 1024];
    let mut n = 0u64;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Err(io::Error::other("cancelled"));
        }
        let k = src.read(&mut buf)?;
        if k == 0 {
            break;
        }
        n += k as u64;
        if n > e.size {
            return Err(changed(&e.name));
        }
        out.write_all(&buf[..k])?;
    }
    if n != e.size {
        return Err(changed(&e.name));
    }
    out.flush()
}

/// Counts one send as uploading for as long as it lives (Quit, "Restart now", "when done" and
/// keep-awake look at the count). Dropped however the send ends: done, failed, cancelled, or a
/// panic in it.
struct Uploading(&'static AtomicUsize);

impl Uploading {
    fn start(count: &'static AtomicUsize) -> Self {
        count.fetch_add(1, Ordering::SeqCst);
        Uploading(count)
    }
}

impl Drop for Uploading {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// What "Try again" on a send can do.
enum Resume {
    /// It's uploading already.
    Running,
    /// Nothing to resume (no files left, no folder, or its Sent item is gone).
    Nothing,
    /// Now uploading again: the files left.
    Go(Arc<Job>, Vec<Entry>),
}

impl Inner {
    /// Starts sending again the files a several-files send missed, into the same folder. Only
    /// while its Sent item is on the list: once that is deleted from Gofile (or taken off the
    /// list), the folder may be gone and the files would land nowhere anyone can reach.
    /// Decided under the `inner` lock, the same lock `take_failed` works under.
    fn begin_resume(&self, id: &str) -> Resume {
        let Some(job) = self.jobs.iter().find(|j| j.id == id).cloned() else { return Resume::Nothing };
        let mut phase = job.phase.lock().unwrap();
        if matches!(*phase, Phase::Uploading) {
            return Resume::Running;
        }
        if job.folder.lock().unwrap().is_none() || !self.sent.iter().any(|s| s.id == id) {
            return Resume::Nothing;
        }
        let left: Vec<Entry> = std::mem::take(&mut *job.left.lock().unwrap());
        if left.is_empty() {
            return Resume::Nothing;
        }
        *phase = Phase::Uploading;
        drop(phase);
        Resume::Go(job, left)
    }

    /// Before a Sent item is deleted from Gofile: takes its failed send off the list ("Try
    /// again" would upload into the deleted folder). Err while it is still uploading.
    fn take_failed(&mut self, id: &str) -> Result<Option<(usize, Arc<Job>)>, ()> {
        let Some(i) = self.jobs.iter().position(|j| j.id == id) else { return Ok(None) };
        if self.jobs[i].uploading() {
            return Err(());
        }
        Ok(Some((i, self.jobs.remove(i))))
    }
}

fn link_from(data: &UploadData) -> Result<String, String> {
    data.download_page
        .clone()
        .or_else(|| data.code.clone().map(|c| format!("https://gofile.io/d/{c}")))
        .ok_or_else(|| "Gofile didn't send the link back.".to_string())
}

/// Why packing / reading stopped, when it failed for its own reason (not because the upload
/// ended or this attempt was stopped). Never waits long: Cancel, or 15 seconds, ends the wait.
async fn writer_error(cancel: &CancellationToken, writer: tauri::async_runtime::JoinHandle<io::Result<()>>) -> Option<String> {
    tokio::select! {
        _ = cancel.cancelled() => None,
        _ = tokio::time::sleep(Duration::from_secs(15)) => None,
        r = writer => match r {
            Ok(Err(e)) if !matches!(e.to_string().as_str(), "upload stopped" | "cancelled") => Some(e.to_string()),
            _ => None,
        },
    }
}

impl Job {
    fn new(name: String, files: usize, input: u64, total: Option<u64>, paths: Vec<String>, mode: Mode, as_files: bool, host: Host) -> Self {
        Job {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            files,
            input,
            total,
            produced: Arc::new(AtomicU64::new(0)),
            phase: Mutex::new(Phase::Uploading),
            cancel: CancellationToken::new(),
            paths,
            mode,
            sample: Mutex::new((Instant::now(), 0, 0.0)),
            timing: Mutex::new((0.0, 0.0)),
            as_files,
            files_done: AtomicUsize::new(0),
            folder: Mutex::new(None),
            left: Mutex::new(Vec::new()),
            key: Mutex::new(None),
            host,
            delete_days: 0,
            contents: Vec::new(),
            account: Mutex::new(None),
            resumed: AtomicBool::new(false),
        }
    }

    fn view(&self) -> JobView {
        let (status, message) = match &*self.phase.lock().unwrap() {
            Phase::Uploading => ("uploading", None),
            Phase::Failed(m) => ("failed", Some(m.clone())),
        };
        // Each lock taken once, on its own line: two guards of the same lock in one expression
        // would wait for each other forever.
        let link = self.folder.lock().unwrap().as_ref().map(|f| f.1.clone());
        let any_left = !self.left.lock().unwrap().is_empty();
        let speed = self.sample.lock().unwrap().2 as u64;
        JobView {
            id: self.id.clone(),
            name: self.name.clone(),
            files: self.files,
            size: self.input,
            total: self.total,
            sent: self.produced.load(Ordering::Relaxed),
            speed,
            status,
            message,
            paths: self.paths.clone(),
            as_files: self.as_files,
            files_done: self.files_done.load(Ordering::Relaxed),
            can_resume: any_left && link.is_some(),
            link,
            mode: self.mode.as_str(),
            host: if self.host == Host::Drive { "drive" } else { "gofile" },
            delete_days: self.delete_days,
        }
    }

    fn uploading(&self) -> bool {
        matches!(*self.phase.lock().unwrap(), Phase::Uploading)
    }
}

impl Sender {
    pub fn new(app: AppHandle) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("Pako/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        let loaded = load_sent(&sent_path());
        let token = std::fs::read(token_path())
            .ok()
            .and_then(|b| crate::secure::unprotect(&b).ok())
            .and_then(|b| String::from_utf8(b).ok())
            .filter(|t| !t.is_empty());
        let send_google = Arc::new(Google::for_account(http.clone(), crate::google::SENDING));
        Arc::new(Self {
            app,
            http,
            send_google,
            inner: Mutex::new(Inner { jobs: Vec::new(), sent: loaded.items, token, ticking: false }),
            save: Mutex::new(()),
            locked: AtomicBool::new(loaded.locked),
            list_note: Mutex::new(loaded.note),
            changing: Claims::default(),
            checks: Mutex::new(Checks::default()),
        })
    }

    pub fn state(&self) -> SendState {
        let (checking, checked_at, refused) = {
            let c = self.checks.lock().unwrap();
            (c.running, c.last_ms, c.refused)
        };
        let now = now_millis();
        let inner = self.inner.lock().unwrap();
        let shared = inner.token.is_some();
        SendState {
            jobs: inner.jobs.iter().map(|j| j.view()).collect(),
            sent: inner
                .sent
                .iter()
                .map(|s| SentView {
                    can_delete: !s.gone && (s.on_drive() || s.key.is_some() || shared) && (s.folder_id.is_some() || s.file_id.is_some()),
                    soon: removal_soon(s, now),
                    item: SentItem { key: None, ..s.clone() },
                })
                .collect(),
            can_delete: shared || inner.sent.iter().any(|s| s.key.is_some()),
            keep_note: KEEP_NOTE,
            checking,
            checked_at,
            counts_note: refused.then_some(REFUSED_NOTE),
            drive_used: inner.sent.iter().filter(|s| s.on_drive() && !s.gone).map(|s| s.size).sum(),
            drive_sends: inner.sent.iter().filter(|s| s.on_drive() && !s.gone).count(),
            list_note: self.list_note.lock().unwrap().clone(),
            drive_accounts: drive_accounts(&inner.sent),
        }
    }

    /// Your Google Drive for sending: connected or not, and its room.
    pub async fn drive_status(&self) -> DriveSendStatus {
        let g = &self.send_google;
        if !g.configured() {
            return DriveSendStatus { missing_note: Some(g.missing_note()), ..Default::default() };
        }
        let mut out = DriveSendStatus { configured: true, signed_in: g.signed_in().await, email: g.email().await, ..Default::default() };
        if out.signed_in {
            match self.drive().quota().await {
                Ok((limit, used)) => {
                    out.limit = limit;
                    out.used = Some(used);
                }
                Err(f) => out.error = Some(f.msg),
            }
        }
        out
    }

    /// Where new sends go (the setting).
    fn host(&self) -> Host {
        self.app.try_state::<Arc<Engine>>().map(|e| Host::from_setting(&e.settings().send_host)).unwrap_or(Host::Gofile)
    }

    /// How much a send of these files would upload (to check it fits before sending).
    pub fn measure(&self, paths: &[String], choice: Option<&str>) -> Result<u64> {
        let picked: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        let mode = self.mode(choice);
        let payload = gather(&picked, mode)?;
        Ok(match &payload {
            Payload::Zip { entries, .. } => zip_size(entries, mode.pack(), Z64_AT).unwrap_or_else(|| payload.input_size()),
            _ => payload.input_size(),
        })
    }

    fn changed(&self) {
        let _ = self.app.emit("send-changed", ());
    }

    fn save_sent(&self) {
        let _saving = self.save.lock().unwrap();
        // The file on disk couldn't be read: writing would replace it with what's shown.
        if self.locked.load(Ordering::Relaxed) {
            return;
        }
        let list = self.inner.lock().unwrap().sent.clone();
        if let Ok(s) = serde_json::to_string_pretty(&list) {
            let tmp = sent_path().with_extension("json.tmp");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(&tmp, sent_path());
            }
        }
    }

    /// Remembers a guest key for the next sends. The key already kept stays (older Sent items
    /// without their own key need it to be deleted), unless Gofile just refused it.
    fn keep_token(&self, token: &str, refused: Option<&str>) {
        let mut inner = self.inner.lock().unwrap();
        if inner.token.as_deref() == Some(token) {
            return;
        }
        if inner.token.is_some() && inner.token.as_deref() != refused {
            return;
        }
        inner.token = Some(token.to_string());
        if let Ok(b) = crate::secure::protect(token.as_bytes()) {
            let _ = std::fs::write(token_path(), b);
        }
    }

    /// How several files are sent: the one-off choice, else the setting.
    fn mode(&self, choice: Option<&str>) -> Mode {
        let s = self.app.try_state::<Arc<Engine>>().map(|e| e.settings());
        match choice {
            Some(c) => Mode::from_settings(Some(c), ""),
            None => s.map(|s| Mode::from_settings(s.send_mode.as_deref(), &s.send_compression)).unwrap_or(Mode::AsTheyAre),
        }
    }

    /// Starts sending the picked files and folders. Returns at once; progress comes as events.
    /// `choice` = "files" / "zip" / "compress" for this send only (None = the setting). `name` =
    /// the name you gave it (the folder or ZIP name; None or empty = Pako's usual name).
    /// `host` = "gofile" / "drive" for this send only ("Try again" keeps where it went; None = the setting).
    pub fn start(self: &Arc<Self>, paths: Vec<String>, choice: Option<String>, name: Option<String>, delete_days: Option<u32>, host: Option<String>) -> Result<String> {
        let picked: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        let mode = self.mode(choice.as_deref());
        let mut payload = gather(&picked, mode)?;
        if let Some(n) = name.as_deref().and_then(clean_send_name) {
            payload.rename(&n);
        }
        let pack = mode.pack();
        let total = match &payload {
            Payload::File(e) => Some(e.size),
            Payload::Files { entries, .. } => Some(entries.iter().map(|e| e.size).sum()),
            Payload::Zip { entries, .. } => zip_size(entries, pack, Z64_AT),
        };
        let compressed = matches!(&payload, Payload::Zip { entries, .. } if entries.iter().any(|e| level_of(pack, e).is_some()));
        let as_files = matches!(payload, Payload::Files { .. });
        let mut job = Job::new(payload.name().to_string(), payload.files(), payload.input_size(), total, paths, mode, as_files, host.as_deref().map(Host::from_setting).unwrap_or_else(|| self.host()));
        job.delete_days = delete_days.filter(|d| DELETE_CHOICES.contains(d)).unwrap_or_else(|| self.delete_days());
        job.contents = contents_of(&payload);
        let job = Arc::new(job);
        let id = job.id.clone();
        self.inner.lock().unwrap().jobs.push(job.clone());
        self.changed();
        self.tick_while_busy();
        let me = self.clone();
        let up = Uploading::start(&crate::engine::UPLOADS_RUNNING);
        tauri::async_runtime::spawn(async move {
            let _up = up;
            let result = me.run(&job, payload, pack).await;
            me.finished(&job, result, compressed);
        });
        Ok(id)
    }

    async fn run(&self, job: &Arc<Job>, payload: Payload, pack: Pack) -> Result<Done, String> {
        self.store(job.host).upload(job, payload, pack).await
    }

    /// The usual "Delete after" (days; 0 = keep): the setting, else 7 days.
    fn delete_days(&self) -> u32 {
        let days = self.app.try_state::<Arc<Engine>>().map(|e| e.settings().upload_delete_days).unwrap_or(DELETE_DEFAULT);
        if DELETE_CHOICES.contains(&days) { days } else { DELETE_DEFAULT }
    }

    /// One upload request, its bytes packed or read on their own thread straight into it.
    async fn upload_once(
        &self,
        job: &Arc<Job>,
        src: &Arc<Source>,
        name: &str,
        total: Option<u64>,
        folder: Option<&str>,
        token: Option<&str>,
    ) -> Result<UploadData, Fail> {
        let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(8);
        let mine = Arc::new(AtomicU64::new(0));
        // Stops this attempt's packing / reading only (other files of the send go on).
        let stop = Arc::new(AtomicBool::new(false));
        let writer = {
            let (produced, mine, stop, src) = (job.produced.clone(), mine.clone(), stop.clone(), src.clone());
            tauri::async_runtime::spawn_blocking(move || {
                let mut out = ToUpload { tx: tx.clone(), buf: Vec::with_capacity(256 * 1024), produced, mine };
                let r = match &*src {
                    Source::File(e) => copy_file(e, &mut out, &stop),
                    Source::Zip(entries, pack) => write_zip(entries, *pack, &mut out, &stop, Z64_AT),
                };
                if let Err(e) = &r {
                    // The upload learns the stream broke (and doesn't finish a half file).
                    let _ = tx.blocking_send(Err(io::Error::other(e.to_string())));
                }
                r
            })
        };
        let request = post_upload(&self.http, UPLOAD_URL, token, folder, name, rx, total);
        let reply = tokio::select! {
            _ = job.cancel.cancelled() => {
                stop.store(true, Ordering::Relaxed);
                return Err(Fail::cancelled());
            }
            r = request => r,
        };
        let outcome = async {
            // If packing failed, say why (a changed file is clearer than "the upload was cut off").
            let (code, text) = match reply {
                Ok(r) => r,
                Err(err) => {
                    stop.store(true, Ordering::Relaxed);
                    return Err(Fail::msg(writer_error(&job.cancel, writer).await.unwrap_or_else(|| net_error(&err))));
                }
            };
            let ok = (200..300).contains(&code);
            if !ok {
                stop.store(true, Ordering::Relaxed);
            }
            if let Some(m) = writer_error(&job.cancel, writer).await {
                return Err(Fail::msg(m));
            }
            if !ok {
                return Err(Fail { msg: status_error(code, &text), code: Some(code) });
            }
            let reply: UploadReply = serde_json::from_str(&text).map_err(|_| Fail::msg("Gofile answered something Pako doesn't understand."))?;
            if reply.status != "ok" {
                return Err(Fail::msg(format!("Gofile refused the upload ({}).", reply.status)));
            }
            reply.data.ok_or_else(|| Fail::msg("Gofile didn't send the link back."))
        }
        .await;
        if outcome.is_err() {
            // That attempt's bytes don't count: the file is sent again from the start.
            job.produced.fetch_sub(mine.load(Ordering::Relaxed).min(job.produced.load(Ordering::Relaxed)), Ordering::Relaxed);
        }
        outcome
    }

    /// One upload with the right Gofile key: the send's own key once it has one, else the key
    /// Pako keeps (none = Gofile makes a new guest account). If Gofile refuses that key, the
    /// first upload of a send goes once more without one.
    async fn upload(&self, job: &Arc<Job>, src: &Arc<Source>, name: &str, total: Option<u64>, folder: Option<&str>) -> Result<UploadData, Fail> {
        let own = job.key.lock().unwrap().clone();
        let mut token = own.or_else(|| if folder.is_none() { self.inner.lock().unwrap().token.clone() } else { None });
        let mut refused = None;
        let mut r = self.upload_once(job, src, name, total, folder, token.as_deref()).await;
        if folder.is_none() && token.is_some() && matches!(&r, Err(f) if matches!(f.code, Some(401 | 403))) {
            refused = token.take();
            r = self.upload_once(job, src, name, total, None, None).await;
        }
        let data = r?;
        // The key this send (and its Sent item) uses from now on.
        if let Some(key) = data.guest_token.clone().or(token) {
            let mut own = job.key.lock().unwrap();
            if own.is_none() {
                *own = Some(key);
            }
        }
        if let Some(t) = &data.guest_token {
            self.keep_token(t, refused.as_deref());
        }
        Ok(data)
    }

    /// One file with two more tries after a short wait (a cut connection, Gofile busy). Not
    /// when Gofile said it's too big or refused it: that would send the whole file again for
    /// the same answer.
    async fn upload_retrying(&self, job: &Arc<Job>, e: &Entry, folder: Option<&str>) -> Result<UploadData, String> {
        let src = Arc::new(Source::File(e.clone()));
        let mut last = String::new();
        for wait in [0u64, 3, 8] {
            if wait > 0 {
                tokio::select! {
                    _ = job.cancel.cancelled() => return Err("cancelled".into()),
                    _ = tokio::time::sleep(Duration::from_secs(wait)) => {}
                }
            }
            match self.upload(job, &src, &e.name, Some(e.size), folder).await {
                Ok(d) => return Ok(d),
                Err(f) if f.is_cancelled() || matches!(f.code, Some(413 | 401 | 403)) => return Err(f.msg),
                Err(f) => last = f.msg,
            }
        }
        Err(last)
    }

    /// Several files as they are: the first makes the folder (and its link), the rest go into it
    /// three at a time. Files that still fail are kept for "Try again".
    async fn run_files(&self, job: &Arc<Job>, mut entries: Vec<Entry>) -> Result<Done, String> {
        use futures_util::StreamExt;
        let mut folder = job.folder.lock().unwrap().clone();
        if folder.is_none() {
            if entries.is_empty() {
                return Err("There are no files to upload.".into());
            }
            let first = entries.remove(0);
            // Nothing is up if this fails: "Try again" starts the whole send over.
            let data = self.upload_retrying(job, &first, None).await?;
            let id = data.parent_folder.clone().ok_or_else(|| "Gofile didn't say which folder it made.".to_string())?;
            let link = link_from(&data)?;
            *job.folder.lock().unwrap() = Some((id.clone(), link.clone()));
            job.files_done.fetch_add(1, Ordering::Relaxed);
            // Files are on Gofile from now on: the send goes on the Sent list at once (marked
            // not finished), so Cancel, Dismiss or closing Pako never leaves files online that
            // Pako can't delete.
            self.record(job, &link, Some(id.clone()), None, false, true);
            self.changed();
            folder = Some((id, link));
        }
        let (folder_id, link) = folder.expect("set above");
        let failed: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
        futures_util::stream::iter(entries)
            .map(|e| {
                let folder_id = folder_id.clone();
                let failed = &failed;
                async move {
                    match self.upload_retrying(job, &e, Some(&folder_id)).await {
                        Ok(_) => {
                            job.files_done.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(m) if m == "cancelled" => {}
                        Err(_) => failed.lock().unwrap().push(e),
                    }
                }
            })
            .buffer_unordered(3)
            .collect::<Vec<()>>()
            .await;
        if job.cancel.is_cancelled() {
            return Err("cancelled".into());
        }
        let failed = failed.into_inner().unwrap();
        if !failed.is_empty() {
            let n = failed.len();
            *job.left.lock().unwrap() = failed;
            return Err(if n == 1 {
                "1 file didn't upload. Try again to upload just that one (the link stays the same).".into()
            } else {
                format!("{n} files didn't upload. Try again to upload just those (the link stays the same).")
            });
        }
        Ok((link, Some(folder_id), None))
    }

    /// "Try again" on a several-files send that partly failed: only the missing files, into the
    /// same folder (the link people already have keeps working). Returns false when there's
    /// nothing to resume, or its Sent item is gone (then the whole send is started again by the
    /// window). A second click while it's already going again does nothing (true).
    pub fn resume(self: &Arc<Self>, id: &str) -> bool {
        let go = self.inner.lock().unwrap().begin_resume(id);
        let (job, left) = match go {
            Resume::Running => return true,
            Resume::Nothing => return false,
            Resume::Go(job, left) => (job, left),
        };
        job.resumed.store(true, Ordering::Relaxed);
        let me = self.clone();
        let up = Uploading::start(&crate::engine::UPLOADS_RUNNING);
        tauri::async_runtime::spawn(async move {
            let _up = up;
            let result = me.store(job.host).upload_rest(&job, left).await;
            me.finished(&job, result, false);
        });
        self.changed();
        self.tick_while_busy();
        true
    }

    /// Puts a send on the Sent list, or brings its entry up to date (first on the list).
    fn record(&self, job: &Job, link: &str, folder_id: Option<String>, file_id: Option<String>, compressed: bool, incomplete: bool) {
        let key = job.key.lock().unwrap().as_deref().and_then(protect_key);
        let account = job.account.lock().unwrap().clone();
        let now = now_millis();
        let (busy, peak) = *job.timing.lock().unwrap();
        {
            let mut inner = self.inner.lock().unwrap();
            let old = inner.sent.iter().position(|s| s.id == job.id).map(|i| inner.sent.remove(i));
            inner.sent.insert(
                0,
                SentItem {
                    id: job.id.clone(),
                    name: job.name.clone(),
                    size: job.produced.load(Ordering::Relaxed),
                    // Not finished: what is on Gofile so far.
                    files: if incomplete { job.files_done.load(Ordering::Relaxed) } else { job.files },
                    date: now,
                    link: link.to_string(),
                    folder_id,
                    file_id,
                    compressed,
                    as_files: job.as_files,
                    incomplete,
                    key: key.or(old.as_ref().and_then(|o| o.key.clone())),
                    downloads: old.as_ref().and_then(|o| o.downloads),
                    downloaded_at: old.as_ref().and_then(|o| o.downloaded_at),
                    gone: false,
                    host: (job.host == Host::Drive).then(|| "drive".to_string()),
                    delete_at: delete_at(now, job.delete_days),
                    expiry_on_host: false,
                    delete_error: None,
                    delete_tries: 0,
                    note: old.as_ref().and_then(|o| o.note.clone()),
                    contents: job.contents.clone(),
                    secs: (busy > 0.0).then_some(busy),
                    peak: (peak > 0.0).then_some(peak),
                    count_history: old.as_ref().map(|o| o.count_history.clone()).unwrap_or_default(),
                    account: account.or(old.as_ref().and_then(|o| o.account.clone())),
                },
            );
        }
        self.save_sent();
    }

    /// The Sent entry of a several-files send that stopped part-way, with what is up by now.
    fn record_partial(&self, job: &Job) {
        let folder = job.folder.lock().unwrap().clone();
        if let Some((id, link)) = folder {
            self.record(job, &link, Some(id), None, false, true);
        }
    }

    fn finished(self: &Arc<Self>, job: &Arc<Job>, result: Result<Done, String>, compressed: bool) {
        match result {
            Ok((link, folder_id, file_id)) => {
                self.inner.lock().unwrap().jobs.retain(|j| j.id != job.id);
                self.record(job, &link, folder_id, file_id, compressed, false);
                if job.delete_days > 0 {
                    let (me, id) = (self.clone(), job.id.clone());
                    tauri::async_runtime::spawn(async move { me.push_expiry(&id).await });
                }
                let (busy, peak) = *job.timing.lock().unwrap();
                let bytes = job.produced.load(Ordering::Relaxed);
                let mut e = crate::numbers::Event::new(format!("up:{}", job.id), crate::numbers::now_ms(), crate::numbers::UPLOAD).timed(bytes, busy, peak);
                e.name = job.name.clone();
                e.bytes = bytes;
                e.files = job.files.max(1) as u32;
                e.route = if job.host == Host::Drive { "drive" } else { "gofile" }.into();
                e.ftype = if job.files > 1 || compressed { "archive" } else { crate::numbers::file_type(&job.name) }.into();
                std::thread::spawn(move || crate::numbers::record(e));
                crate::system::set_clipboard_text(&link);
                let _ = self.app.emit("send-done", serde_json::json!({ "name": job.name, "link": link }));
                // Pako's window in view shows it already (a message at the bottom): no pop-up.
                if let Some(e) = self.app.try_state::<Arc<Engine>>() {
                    let pop = if self.window_in_view() { Pop::Skip } else { Pop::Pref };
                    e.notify_pop("upload-done", "Upload finished, link copied", &format!("{}: paste the link to anyone.", job.name), Go::item("send", job.id.clone()), pop);
                }
            }
            Err(msg) if msg == "cancelled" => {
                self.inner.lock().unwrap().jobs.retain(|j| j.id != job.id);
                // You cancelled: what already went up is deleted too, and the entry goes with it.
                // Recorded first, so if deleting fails it stays on the list (unfinished) to retry.
                // A send carried on with "Try again" keeps what its earlier runs put up: it
                // stays on the list, not finished, for you to delete or keep.
                self.record_partial(job);
                let folder = job.folder.lock().unwrap().is_some();
                if folder && !job.resumed.load(Ordering::Relaxed) {
                    let (me, id) = (self.clone(), job.id.clone());
                    tauri::async_runtime::spawn(async move {
                        if me.remove(&id, true).await.is_err() {
                            me.changed();
                        }
                    });
                }
            }
            Err(msg) => {
                crate::errorlog::note(crate::errorlog::Area::Upload, &msg);
                *job.phase.lock().unwrap() = Phase::Failed(msg.clone());
                self.record_partial(job);
                if let Some(e) = self.app.try_state::<Arc<Engine>>() {
                    let pop = if self.window_in_view() { Pop::Skip } else { Pop::Pref };
                    e.notify_pop("upload-failed", "Couldn't upload a file", &format!("{}: {msg}", job.name), Go::item("send", job.id.clone()), pop);
                }
            }
        }
        self.changed();
    }

    fn window_in_view(&self) -> bool {
        self.app
            .get_webview_window("main")
            .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
    }

    /// Progress to the windows twice a second while something uploads.
    fn tick_while_busy(self: &Arc<Self>) {
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.ticking {
                return;
            }
            inner.ticking = true;
        }
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let jobs: Vec<Arc<Job>> = me.inner.lock().unwrap().jobs.clone();
                for j in jobs.iter().filter(|j| j.uploading()) {
                    let now = Instant::now();
                    let n = j.produced.load(Ordering::Relaxed);
                    let mut s = j.sample.lock().unwrap();
                    let dt = now.duration_since(s.0).as_secs_f64();
                    if dt >= 0.4 {
                        let rate = n.saturating_sub(s.1) as f64 / dt;
                        s.2 = if s.2 == 0.0 { rate } else { s.2 * 0.7 + rate * 0.3 };
                        *s = (now, n, s.2);
                        let mut t = j.timing.lock().unwrap();
                        t.0 += dt;
                        t.1 = t.1.max(s.2);
                    }
                }
                let views: Vec<JobView> = jobs.iter().map(|j| j.view()).collect();
                let _ = me.app.emit("send-progress", views);
                // Decided under the same lock a new send is added with: a send added right now
                // is either seen here, or starts the ticking again itself.
                let mut inner = me.inner.lock().unwrap();
                if !inner.jobs.iter().any(|j| j.uploading()) {
                    inner.ticking = false;
                    return;
                }
            }
        });
    }

    /// The link of something sent (for "Open the download page").
    pub fn link_of(&self, id: &str) -> Option<String> {
        self.inner.lock().unwrap().sent.iter().find(|s| s.id == id).map(|s| s.link.clone())
    }

    pub fn cancel(&self, id: &str) {
        let job = self.inner.lock().unwrap().jobs.iter().find(|j| j.id == id).cloned();
        if let Some(j) = job {
            j.cancel.cancel();
        }
    }

    /// Takes a failed send off the list (files already up stay on the Sent list).
    pub fn dismiss(&self, id: &str) {
        self.inner.lock().unwrap().jobs.retain(|j| j.id != id || j.uploading());
        self.changed();
    }

    /// Asks Gofile how often each sent item was downloaded, one after the other, in the
    /// background (the window gets "send-changed" when it starts and when it's done). Not more
    /// often than every few minutes; `button` = the refresh button (a shorter wait). Returns
    /// false when it didn't start (already running, or asked a moment ago).
    pub fn check(self: &Arc<Self>, button: bool) -> bool {
        {
            let mut c = self.checks.lock().unwrap();
            let gap = if button {
                CHECK_GAP_BUTTON
            } else if c.refused {
                REFUSED_GAP
            } else {
                CHECK_GAP
            };
            if c.running || c.last.is_some_and(|t| t.elapsed() < gap) {
                return false;
            }
            c.running = true;
        }
        self.changed();
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let refused = me.check_all(CONTENTS_URL).await;
            {
                let mut c = me.checks.lock().unwrap();
                c.running = false;
                c.last = Some(Instant::now());
                c.last_ms = Some(now_millis());
                if let Some(r) = refused {
                    c.refused = r;
                }
            }
            me.changed();
        });
        true
    }

    /// Each sent item in turn. Some(true) = Gofile didn't share the counts, Some(false) = it
    /// did, None = nothing could be asked.
    async fn check_all(&self, base: &str) -> Option<bool> {
        let (items, shared): (Vec<SentItem>, Option<String>) = {
            let inner = self.inner.lock().unwrap();
            let uploading: Vec<&str> = inner.jobs.iter().filter(|j| j.uploading()).map(|j| j.id.as_str()).collect();
            // Gofile's counts only: Google Drive doesn't say how often a file was downloaded.
            let items = inner.sent.iter().filter(|s| !s.gone && !s.on_drive() && !uploading.contains(&s.id.as_str())).cloned().collect();
            (items, inner.token.clone())
        };
        let mut refused = None;
        let mut asked = false;
        for item in &items {
            let Some(target) = item.folder_id.clone().or(item.file_id.clone()) else { continue };
            let Some(token) = token_for(item, shared.clone()) else { continue };
            if asked {
                // One at a time, without hurrying Gofile.
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            asked = true;
            let remote = fetch_remote(&self.http, base, &target, &token).await;
            match remote {
                Remote::Refused => {
                    // The same answer would come for every item: stop here.
                    refused = Some(true);
                    break;
                }
                Remote::Unknown => continue,
                Remote::Found(_) | Remote::Gone => refused = Some(false),
            }
            self.apply_remote(&item.id, remote, now_millis());
        }
        self.save_sent();
        refused
    }

    /// Notes what Gofile said about one item on the Sent list.
    fn apply_remote(&self, id: &str, remote: Remote, now: u64) {
        let mut inner = self.inner.lock().unwrap();
        let Some(s) = inner.sent.iter_mut().find(|s| s.id == id) else { return };
        apply_to(s, remote, now);
    }

    /// Takes a sent item off the list; `online` also deletes it from Gofile (permanent).
    pub async fn remove(&self, id: &str, online: bool) -> Result<()> {
        let Some(_claim) = self.changing.take(id) else { bail!(BUSY_ITEM) };
        self.remove_claimed(id, online).await
    }

    /// `remove`, once this upload is claimed.
    async fn remove_claimed(&self, id: &str, online: bool) -> Result<()> {
        let (item, shared) = {
            let inner = self.inner.lock().unwrap();
            (inner.sent.iter().find(|s| s.id == id).cloned(), inner.token.clone())
        };
        let Some(item) = item else { bail!("It's no longer on the list.") };
        if online {
            // Its failed send (if any) comes off the list first, so "Try again" can't upload into
            // the folder being deleted; it comes back if the deleting doesn't work.
            let parked = self
                .inner
                .lock()
                .unwrap()
                .take_failed(id)
                .map_err(|_| anyhow!("It's still uploading. Cancel it first, then delete it."))?;
            if let Err(msg) = self.store_of(&item).delete(&item, shared).await {
                if let Some((i, job)) = parked {
                    let mut inner = self.inner.lock().unwrap();
                    let i = i.min(inner.jobs.len());
                    inner.jobs.insert(i, job);
                }
                bail!("{msg} It's still on the list.");
            }
        }
        self.inner.lock().unwrap().sent.retain(|s| s.id != id);
        self.save_sent();
        self.changed();
        Ok(())
    }
}

// ---------- Delete after ----------

/// Sent items whose "Delete after" date has come (not ones still uploading).
fn due_ids(sent: &[SentItem], now: u64, busy: &[String]) -> Vec<String> {
    sent.iter().filter(|s| s.delete_at.is_some_and(|at| at <= now) && !busy.contains(&s.id)).map(|s| s.id.clone()).collect()
}

/// Notes a delete that didn't work: it's tried again at the next sweep.
fn mark_failed(sent: &mut [SentItem], id: &str, msg: &str) -> u32 {
    let Some(s) = sent.iter_mut().find(|s| s.id == id) else { return 0 };
    s.delete_error = Some(msg.to_string());
    s.delete_tries += 1;
    s.delete_tries
}

/// What one sweep did.
#[derive(Debug, Default)]
struct Swept {
    deleted: Vec<SentItem>,
    /// (id, why, tries so far)
    failed: Vec<(String, String, u32)>,
}

/// Its "Delete after" date has come.
fn still_due(s: &SentItem, now: u64) -> bool {
    s.delete_at.is_some_and(|at| at <= now)
}

/// Deletes each due item with `del`; failures stay on the list with their reason. `del` checks
/// again right before deleting and answers Ok(false) when the item changed meanwhile (a new
/// date, deleted by you): then it's left alone.
async fn sweep_list<F, Fut>(list: &Mutex<Vec<SentItem>>, now: u64, busy: &[String], mut del: F) -> Swept
where
    F: FnMut(SentItem) -> Fut,
    Fut: std::future::Future<Output = Result<bool, String>>,
{
    let mut out = Swept::default();
    let due = due_ids(&list.lock().unwrap(), now, busy);
    for id in due {
        let Some(item) = list.lock().unwrap().iter().find(|s| s.id == id && still_due(s, now)).cloned() else { continue };
        match del(item.clone()).await {
            Ok(true) => out.deleted.push(item),
            Ok(false) => {}
            Err(msg) => {
                let tries = mark_failed(&mut list.lock().unwrap(), &id, &msg);
                out.failed.push((id, msg, tries));
            }
        }
    }
    out
}

/// The PC's clock is more than a day away from the server's.
fn clock_off(pc: u64, server: u64) -> bool {
    pc.abs_diff(server) > DAY_MS
}

/// An HTTP Date ("Sun, 16 Nov 2026 21:04:05 GMT") in ms.
fn http_date_ms(s: &str) -> Option<u64> {
    let t = chrono::DateTime::parse_from_rfc2822(s.trim()).ok()?;
    u64::try_from(t.timestamp_millis()).ok()
}

impl Sender {
    fn item(&self, id: &str) -> (Option<SentItem>, Option<String>) {
        let inner = self.inner.lock().unwrap();
        (inner.sent.iter().find(|s| s.id == id).cloned(), inner.token.clone())
    }

    fn edit(&self, id: &str, f: impl FnOnce(&mut SentItem)) {
        {
            let mut inner = self.inner.lock().unwrap();
            let Some(s) = inner.sent.iter_mut().find(|s| s.id == id) else { return };
            f(s);
        }
        self.save_sent();
        self.changed();
    }

    /// Right after an upload: asks the service to delete it on its date (Gofile does it
    /// itself, even with the PC off). Otherwise Pako does it when the date comes.
    async fn push_expiry(&self, id: &str) {
        let Some(_claim) = self.changing.take(id) else { return };
        let (Some(item), shared) = self.item(id) else { return };
        let Some(at) = item.delete_at else { return };
        let on_host = matches!(self.store_of(&item).set_expiry(&item, shared, Some(at)).await, Ok(ExpiryBy::Host));
        // Only for the date that was sent: a date changed meanwhile is the one that counts.
        self.edit(id, |s| {
            if s.delete_at == Some(at) {
                s.expiry_on_host = on_host;
            }
        });
    }

    /// Changes when an upload gets deleted (None = keep it).
    pub async fn set_expiry(&self, id: &str, at: Option<u64>) -> Result<()> {
        let Some(_claim) = self.changing.take(id) else { bail!(BUSY_ITEM) };
        let (Some(item), shared) = self.item(id) else { bail!("It's no longer on the list.") };
        if at.is_some_and(|a| a <= now_millis()) {
            bail!("Pick a date that hasn't passed.");
        }
        let on_host = match self.store_of(&item).set_expiry(&item, shared, at).await {
            Ok(by) => by == ExpiryBy::Host && at.is_some(),
            // The service would still delete it on the old date: nothing changes.
            Err(msg) if item.expiry_on_host => bail!("{msg} Nothing changed."),
            // Pako deletes it itself then.
            Err(_) => false,
        };
        self.edit(id, |s| {
            s.delete_at = at;
            s.expiry_on_host = on_host;
            s.delete_error = None;
            s.delete_tries = 0;
        });
        Ok(())
    }

    /// Your note on an upload (empty = none).
    pub fn set_note(&self, id: &str, note: &str) {
        let note = note.trim();
        let note = (!note.is_empty()).then(|| note.chars().take(500).collect::<String>());
        self.edit(id, |s| s.note = note);
    }

    /// The link of an upload as a QR code (SVG), made here: nothing is sent anywhere.
    pub fn qr(&self, id: &str) -> Result<String> {
        let (Some(item), _) = self.item(id) else { bail!("It's no longer on the list.") };
        qr_svg(&item.link)
    }

    /// Deletes the uploads whose "Delete after" date has come. A delete that doesn't work is
    /// tried again at the next sweep.
    pub async fn sweep_expired(&self) {
        let busy: Vec<String> = self.inner.lock().unwrap().jobs.iter().filter(|j| matches!(*j.phase.lock().unwrap(), Phase::Uploading)).map(|j| j.id.clone()).collect();
        let now = now_millis();
        let list = Mutex::new(self.inner.lock().unwrap().sent.clone());
        if due_ids(&list.lock().unwrap(), now, &busy).is_empty() {
            return;
        }
        // A PC clock far ahead would delete uploads days early: then nothing is deleted.
        if let Some(server) = self.server_time().await {
            if clock_off(now, server) {
                return;
            }
        }
        let swept = sweep_list(&list, now, &busy, |item| async move { self.sweep_one(&item.id, now).await }).await;
        if swept.deleted.is_empty() && swept.failed.is_empty() {
            return;
        }
        for (id, msg, tries) in &swept.failed {
            let (msg, tries) = (msg.clone(), *tries);
            self.edit(id, |s| {
                s.delete_error = Some(msg);
                s.delete_tries = tries;
            });
        }
        for item in &swept.deleted {
            let mut e = crate::numbers::Event::new(format!("cleaned:{}", item.id), crate::numbers::now_ms(), crate::numbers::CLEANED);
            e.name = item.name.clone();
            e.bytes = item.size;
            e.files = item.files.max(1) as u32;
            e.route = if item.on_drive() { "drive" } else { "gofile" }.into();
            std::thread::spawn(move || crate::numbers::record(e));
        }
        // One message, only the first time a delete doesn't work (it's tried every hour).
        let new_fails: Vec<_> = swept.failed.iter().filter(|f| f.2 == 1).collect();
        let Some(engine) = self.app.try_state::<Arc<Engine>>() else { return };
        match (swept.deleted.as_slice(), new_fails.as_slice()) {
            ([], []) => {}
            ([one], []) => engine.notify("upload-deleted", "Upload deleted", &format!("{}: its \"Delete after\" date came, so Pako deleted it.", one.name), Go::page("send")),
            (many, []) => engine.notify("upload-deleted", "Uploads deleted", &format!("{} uploads reached their \"Delete after\" date, so Pako deleted them.", many.len()), Go::page("send")),
            (_, [(id, msg, _)]) => engine.notify("upload-deleted", "Pako couldn't delete an upload", &format!("{msg} Pako tries again every hour."), Go::item("send", id.clone())),
            (_, fails) => engine.notify("upload-deleted", "Pako couldn't delete some uploads", &format!("{} uploads are past their \"Delete after\" date. Pako tries again every hour.", fails.len()), Go::page("send")),
        }
    }

    /// Deletes one due upload, unless it changed since the sweep started (a new date, already
    /// deleted, uploading again): then Ok(false).
    async fn sweep_one(&self, id: &str, now: u64) -> Result<bool, String> {
        let Some(_claim) = self.changing.take(id) else { return Ok(false) };
        let due = self.item(id).0.is_some_and(|s| still_due(&s, now));
        let uploading = self.inner.lock().unwrap().jobs.iter().any(|j| j.id == id && j.uploading());
        if !due || uploading {
            return Ok(false);
        }
        match self.remove_claimed(id, true).await {
            Ok(()) => Ok(true),
            Err(e) => Err(e.to_string().trim_end_matches(" It's still on the list.").to_string()),
        }
    }

    /// The time a server says it is (ms), from the Date of its answer. None = no answer.
    async fn server_time(&self) -> Option<u64> {
        let resp = self.http.head("https://api.gofile.io/").timeout(Duration::from_secs(15)).send().await.ok()?;
        let date = resp.headers().get(reqwest::header::DATE)?.to_str().ok()?;
        http_date_ms(date)
    }

    /// Looks for uploads to delete a little after start, then every hour.
    pub fn start_sweeper(self: &Arc<Self>) {
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            loop {
                me.sweep_expired().await;
                tokio::time::sleep(SWEEP_EVERY).await;
            }
        });
    }
}

/// A QR code of `text` as SVG (dark modules on a transparent background).
pub fn qr_svg(text: &str) -> Result<String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::new(text.as_bytes()).map_err(|_| anyhow!("This link is too long for a QR code."))?;
    Ok(code.render::<svg::Color>().min_dimensions(160, 160).quiet_zone(false).dark_color(svg::Color("#000")).light_color(svg::Color("#fff")).build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn item(id: &str, delete_at: Option<u64>) -> SentItem {
        SentItem { id: id.into(), name: format!("{id}.zip"), size: 10, files: 1, link: format!("https://gofile.io/d/{id}"), delete_at, ..Default::default() }
    }

    #[test]
    fn delete_after_dates() {
        let sent = 1_700_000_000_000;
        assert_eq!(delete_at(sent, 0), None);
        assert_eq!(delete_at(sent, 3), Some(sent + 3 * DAY_MS));
        assert_eq!(delete_at(sent, 7), Some(sent + 7 * 86_400_000));
        assert_eq!(delete_at(sent, 30), Some(sent + 30 * DAY_MS));
        assert!(DELETE_CHOICES.contains(&DELETE_DEFAULT) && DELETE_DEFAULT == 7);
    }

    #[test]
    fn due_only_when_the_date_came_and_not_uploading() {
        let now = 10 * DAY_MS;
        let list = vec![item("past", Some(now - 1)), item("now", Some(now)), item("later", Some(now + 1)), item("kept", None), item("busy", Some(1))];
        assert_eq!(due_ids(&list, now, &["busy".to_string()]), vec!["past", "now"]);
    }

    /// One delete fails: it stays on the list with the reason, and goes at the next sweep.
    #[tokio::test]
    async fn sweep_retries_what_didnt_delete() {
        let now = 10 * DAY_MS;
        let list = Mutex::new(vec![item("a", Some(now - 5)), item("b", Some(now - 5)), item("c", Some(now + DAY_MS))]);
        let down = AtomicBool::new(true);
        let del = |it: SentItem| {
            let fail = it.id == "b" && down.load(Ordering::SeqCst);
            let list = &list;
            async move {
                if fail {
                    return Err("Gofile is busy right now.".to_string());
                }
                list.lock().unwrap().retain(|s| s.id != it.id);
                Ok(true)
            }
        };
        let first = sweep_list(&list, now, &[], del).await;
        assert_eq!(first.deleted.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["a"]);
        assert_eq!(first.failed, vec![("b".to_string(), "Gofile is busy right now.".to_string(), 1)]);
        {
            let l = list.lock().unwrap();
            let b = l.iter().find(|s| s.id == "b").unwrap();
            assert_eq!((b.delete_error.as_deref(), b.delete_tries), (Some("Gofile is busy right now."), 1));
        }
        // Still failing an hour later: counted, not lost.
        let again = sweep_list(&list, now + 3_600_000, &[], del).await;
        assert_eq!(again.failed[0].2, 2);
        // Works next time; "c" isn't due yet.
        down.store(false, Ordering::SeqCst);
        let last = sweep_list(&list, now + 7_200_000, &[], del).await;
        assert_eq!(last.deleted.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["b"]);
        assert!(last.failed.is_empty());
        assert_eq!(list.lock().unwrap().iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["c"]);
    }

    /// sent.json from before "Delete after": read as kept, and saved the same way.
    #[test]
    fn older_sent_items_are_kept() {
        let old = r#"[{"id":"x","name":"a.zip","size":5,"files":1,"date":1,"link":"https://gofile.io/d/x","folderId":"f","fileId":null,"compressed":false,"downloads":3}]"#;
        let list: Vec<SentItem> = serde_json::from_str(old).unwrap();
        let s = &list[0];
        assert_eq!((s.delete_at, s.expiry_on_host, s.delete_tries), (None, false, 0));
        assert!(s.note.is_none() && s.contents.is_empty() && s.count_history.is_empty() && s.secs.is_none());
        let saved = serde_json::to_string(s).unwrap();
        for key in ["deleteAt", "expiryOnHost", "deleteError", "deleteTries", "note", "contents", "secs", "peak", "countHistory"] {
            assert!(!saved.contains(key), "{key} in {saved}");
        }
        // With a date, a note and its files, they come back.
        let mut s = s.clone();
        s.delete_at = Some(99);
        s.expiry_on_host = true;
        s.note = Some("for the client".into());
        s.contents = vec![SentFile { name: "a/b.jpg".into(), size: 4 }];
        let back: SentItem = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!((back.delete_at, back.expiry_on_host, back.note.as_deref()), (Some(99), true, Some("for the client")));
        assert_eq!(back.contents, s.contents);
    }

    #[test]
    fn download_counts_kept_over_time() {
        let mut s = item("x", None);
        apply_to(&mut s, Remote::Found(0), 1);
        apply_to(&mut s, Remote::Found(0), 2);
        apply_to(&mut s, Remote::Found(2), 3);
        assert_eq!(s.count_history, vec![(1, 0), (3, 2)]);
    }

    #[test]
    fn qr_code_is_an_svg() {
        let svg = qr_svg("https://gofile.io/d/abc123").unwrap();
        assert!(svg.contains("<svg") && svg.contains("</svg>"));
    }

    #[test]
    fn files_inside_are_listed() {
        let d = tmp_dir("contents");
        let payload = gather(&[d.join("Photos")], Mode::Zip).unwrap();
        let names: Vec<String> = contents_of(&payload).into_iter().map(|f| f.name).collect();
        assert!(names.iter().any(|n| n.ends_with("a.txt")) && names.iter().any(|n| n.ends_with("b.jpg")));
        assert!(!names.iter().any(|n| n.ends_with("sub") || n.ends_with("sub/")));
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pako-send-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("Photos/sub")).unwrap();
        std::fs::write(d.join("Photos/a.txt"), b"hello hello hello hello hello hello").unwrap();
        std::fs::write(d.join("Photos/sub/b.jpg"), vec![7u8; 70_000]).unwrap();
        std::fs::write(d.join("Photos/sub/empty.log"), b"").unwrap();
        std::fs::write(d.join("note.txt"), "café ☕ notes".repeat(100)).unwrap();
        d
    }

    fn read_back(zip: Vec<u8>) -> Vec<(String, Vec<u8>)> {
        let mut a = zip::ZipArchive::new(Cursor::new(zip)).expect("a valid zip");
        let mut out = Vec::new();
        for i in 0..a.len() {
            let mut f = a.by_index(i).unwrap();
            let mut v = Vec::new();
            f.read_to_end(&mut v).expect("checksum ok");
            out.push((f.name().to_string(), v));
        }
        out
    }

    #[test]
    fn gathers_in_a_fixed_order_with_names() {
        let d = tmp_dir("gather");
        let p = gather(&[d.join("Photos"), d.join("note.txt")], Mode::Zip).unwrap();
        let Payload::Zip { name, entries } = p else { panic!("zip expected") };
        assert_eq!(name, "Photos and 1 more.zip");
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["note.txt", "Photos/a.txt", "Photos/sub/b.jpg", "Photos/sub/empty.log"]);
        let one = gather(&[d.join("note.txt")], Mode::Zip).unwrap();
        assert!(matches!(one, Payload::File(_)));
        let folder = gather(&[d.join("Photos")], Mode::Zip).unwrap();
        assert_eq!(folder.name(), "Photos.zip");
    }

    #[test]
    fn a_send_name_is_made_safe() {
        assert_eq!(clean_send_name("  Holiday  photos  "), Some("Holiday photos".into()));
        assert_eq!(clean_send_name("a/b\\c:d*e?f\"g<h>i|j"), Some("a b c d e f g h i j".into()));
        assert_eq!(clean_send_name(" ... "), None);
        assert_eq!(clean_send_name(""), None);
        assert_eq!(clean_send_name("..secret."), Some("secret".into()));
        assert_eq!(clean_send_name(&"x".repeat(300)).unwrap().chars().count(), 120);
    }

    #[test]
    fn the_given_name_names_the_folder_or_zip() {
        let d = tmp_dir("rename");
        let mut files = gather(&[d.join("Photos"), d.join("note.txt")], Mode::AsTheyAre).unwrap();
        files.rename("For Sam");
        assert_eq!(files.name(), "For Sam");
        let mut zip = gather(&[d.join("Photos"), d.join("note.txt")], Mode::Zip).unwrap();
        zip.rename("For Sam");
        assert_eq!(zip.name(), "For Sam.zip");
        zip.rename("Already.ZIP");
        assert_eq!(zip.name(), "Already.ZIP");
        let mut one = gather(&[d.join("note.txt")], Mode::Zip).unwrap();
        one.rename("ignored");
        assert_eq!(one.name(), "note.txt");
    }

    #[test]
    fn sizes_of_picked_files_and_folders() {
        let d = tmp_dir("sizes");
        let got = path_sizes(&[
            d.join("Photos").to_string_lossy().into_owned(),
            d.join("note.txt").to_string_lossy().into_owned(),
            d.join("gone.bin").to_string_lossy().into_owned(),
        ]);
        let photos = &got[0];
        assert!(photos.dir && photos.error.is_none());
        let expected: u64 = ["a.txt", "sub/b.jpg", "sub/empty.log"]
            .iter()
            .map(|f| std::fs::metadata(d.join("Photos").join(f)).unwrap().len())
            .sum();
        assert_eq!((photos.files, photos.size), (3, expected));
        assert_eq!((got[1].dir, got[1].files, got[1].size), (false, 1, std::fs::metadata(d.join("note.txt")).unwrap().len()));
        assert!(got[2].error.is_some());
    }

    #[test]
    fn stored_zip_has_the_size_promised_and_opens() {
        let d = tmp_dir("stored");
        let Payload::Zip { entries, .. } = gather(&[d.join("Photos"), d.join("note.txt")], Mode::Zip).unwrap() else { panic!() };
        for z64_at in [Z64_AT, 10] {
            let mut out = Vec::new();
            write_zip(&entries, Pack::Off, &mut out, &AtomicBool::new(false), z64_at).unwrap();
            assert_eq!(Some(out.len() as u64), zip_size(&entries, Pack::Off, z64_at), "size for z64_at={z64_at}");
            let files = read_back(out);
            assert_eq!(files.len(), 4);
            for (name, data) in files {
                let e = entries.iter().find(|e| e.name == name).unwrap();
                assert_eq!(data, std::fs::read(&e.path).unwrap(), "{name}");
            }
        }
    }

    #[test]
    fn same_zip_every_time() {
        let d = tmp_dir("same");
        let Payload::Zip { entries, .. } = gather(&[d.join("Photos")], Mode::Zip).unwrap() else { panic!() };
        let mut a = Vec::new();
        let mut b = Vec::new();
        write_zip(&entries, Pack::Off, &mut a, &AtomicBool::new(false), Z64_AT).unwrap();
        write_zip(&entries, Pack::Off, &mut b, &AtomicBool::new(false), Z64_AT).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn compressed_zip_opens_and_shrinks_text() {
        let d = tmp_dir("deflate");
        let Payload::Zip { entries, .. } = gather(&[d.join("Photos"), d.join("note.txt")], Mode::Zip).unwrap() else { panic!() };
        assert_eq!(zip_size(&entries, Pack::Auto, Z64_AT), None);
        for (pack, z64_at) in [(Pack::Auto, Z64_AT), (Pack::Max, Z64_AT), (Pack::Max, 10)] {
            let mut out = Vec::new();
            write_zip(&entries, pack, &mut out, &AtomicBool::new(false), z64_at).unwrap();
            let stored = zip_size(&entries, Pack::Off, Z64_AT).unwrap();
            if z64_at == Z64_AT {
                assert!((out.len() as u64) < stored, "{pack:?} should be smaller");
            }
            for (name, data) in read_back(out) {
                let e = entries.iter().find(|e| e.name == name).unwrap();
                assert_eq!(data, std::fs::read(&e.path).unwrap(), "{name} with {pack:?}");
            }
        }
    }

    #[test]
    fn what_gets_compressed() {
        assert_eq!(level_for(Pack::Off, "a.txt"), None);
        assert_eq!(level_for(Pack::Auto, "a.TXT"), Some(6));
        assert_eq!(level_for(Pack::Auto, "a.mp4"), None);
        assert_eq!(level_for(Pack::Auto, "a.unknownext"), None);
        assert_eq!(level_for(Pack::Max, "a.unknownext"), Some(9));
        assert_eq!(level_for(Pack::Max, "a.zip"), None);
    }

    #[test]
    fn dos_dates() {
        // 2026-11-15 13:20:10 UTC
        let secs = 1_794_748_810;
        let (time, date) = dos_pack(civil_utc(secs));
        assert_eq!(date >> 9, 2026 - 1980);
        assert_eq!((date >> 5) & 0xF, 11);
        assert_eq!(date & 0x1F, 15);
        assert_eq!(time >> 11, 13);
        assert_eq!((time >> 5) & 0x3F, 20);
        assert_eq!(dos_time(UNIX_EPOCH).1, (1 << 5) | 1);
        // In the ZIP: this PC's local time (never more than a day away from UTC).
        let t = UNIX_EPOCH + Duration::from_secs(secs as u64);
        assert_eq!(dos_time(t), dos_pack(to_local(civil_utc(secs))));
        let (y, m, d, hh, mm, _) = to_local(civil_utc(secs));
        assert_eq!((y, m), (2026, 11));
        let minutes = (d * 24 + hh) * 60 + mm;
        assert!((minutes - (15 * 24 + 13) * 60 - 20).abs() <= 24 * 60);
    }

    #[test]
    fn job_view_with_files_left_does_not_freeze() {
        let job = Arc::new(Job::new("3 files".into(), 3, 30, Some(30), vec![], Mode::AsTheyAre, true, Host::Gofile));
        *job.folder.lock().unwrap() = Some(("f1".into(), "https://gofile.io/d/AbC".into()));
        job.left.lock().unwrap().push(Entry { path: "x".into(), name: "x".into(), size: 1, mtime: UNIX_EPOCH, dir: false });
        *job.phase.lock().unwrap() = Phase::Failed("1 file didn't upload.".into());
        let (tx, rx) = std::sync::mpsc::channel();
        let j = job.clone();
        std::thread::spawn(move || {
            let _ = tx.send(j.view());
        });
        let v = rx.recv_timeout(Duration::from_secs(5)).expect("view() returned (no lock waiting on itself)");
        assert!(v.can_resume);
        assert_eq!(v.link.as_deref(), Some("https://gofile.io/d/AbC"));
        assert_eq!((v.status, v.mode), ("failed", "files"));
    }

    #[test]
    fn deflated_files_near_4gb_get_zip64() {
        let gb4 = 0xFFFF_FFFFu64;
        assert!(deflate_bound(gb4 - 100_000) > gb4, "a file that doesn't shrink grows a little");
        // Stored: the size is known exactly.
        assert!(!needs_z64(0xFFFE_0000, false, Z64_AT));
        assert!(needs_z64(Z64_AT, false, Z64_AT));
        // Deflated: just under 4 GB could grow past it, so ZIP64 is set up front.
        assert!(needs_z64(0xFFFE_0000, true, Z64_AT));
        assert!(needs_z64(0xFFF0_0000, true, Z64_AT));
        assert!(!needs_z64(0xF000_0000, true, Z64_AT));
        assert!(!needs_z64(1_000_000, true, Z64_AT));
    }

    #[test]
    fn file_and_folder_with_the_same_name_dont_clash() {
        let d = std::env::temp_dir().join(format!("pako-send-test-clash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("a")).unwrap();
        std::fs::create_dir_all(d.join("b/Photos")).unwrap();
        std::fs::write(d.join("a/Photos"), b"a file called Photos").unwrap();
        std::fs::write(d.join("b/Photos/x.txt"), b"inside the folder").unwrap();
        let Payload::Zip { entries, .. } = gather(&[d.join("a/Photos"), d.join("b/Photos")], Mode::Zip).unwrap() else { panic!() };
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Photos", "Photos (2)/x.txt"]);
        // Nothing is both a file and a folder in the ZIP.
        for e in &entries {
            let as_folder = format!("{}/", e.name.to_lowercase());
            assert!(!entries.iter().any(|o| o.name.to_lowercase().starts_with(&as_folder)), "{} clashes", e.name);
        }
        let mut out = Vec::new();
        write_zip(&entries, Pack::Off, &mut out, &AtomicBool::new(false), Z64_AT).unwrap();
        assert_eq!(read_back(out).len(), 2);
        // Two folders with the same name stay apart too.
        std::fs::create_dir_all(d.join("c/Photos")).unwrap();
        std::fs::write(d.join("c/Photos/x.txt"), b"another").unwrap();
        let Payload::Zip { entries, .. } = gather(&[d.join("b/Photos"), d.join("c/Photos")], Mode::Zip).unwrap() else { panic!() };
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Photos (2)/x.txt", "Photos/x.txt"]);
        assert_eq!(numbered("a.txt", 2, false), "a (2).txt");
        assert_eq!(numbered("My.Folder", 3, true), "My.Folder (3)");
        assert_eq!(numbered("Photos/", 2, true), "Photos (2)/");
    }

    #[test]
    fn empty_folders_stay_in_a_zip() {
        let d = std::env::temp_dir().join(format!("pako-send-test-emptydir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("Proj/empty")).unwrap();
        std::fs::create_dir_all(d.join("Proj/deep/er")).unwrap();
        std::fs::write(d.join("Proj/x.txt"), b"x").unwrap();
        let p = gather(&[d.join("Proj")], Mode::Zip).unwrap();
        assert_eq!(p.files(), 1);
        let Payload::Zip { entries, .. } = p else { panic!() };
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Proj/deep/er/", "Proj/empty/", "Proj/x.txt"]);
        for pack in [Pack::Off, Pack::Max] {
            let mut out = Vec::new();
            write_zip(&entries, pack, &mut out, &AtomicBool::new(false), Z64_AT).unwrap();
            if pack == Pack::Off {
                assert_eq!(Some(out.len() as u64), zip_size(&entries, pack, Z64_AT));
            }
            let mut a = zip::ZipArchive::new(Cursor::new(out)).unwrap();
            assert!(a.by_name("Proj/empty/").unwrap().is_dir());
            assert!(a.by_name("Proj/deep/er/").unwrap().is_dir());
        }
        // As they are: a Gofile folder is flat, so only the file goes.
        let Payload::Files { entries, .. } = gather(&[d.join("Proj")], Mode::AsTheyAre).unwrap() else { panic!() };
        assert_eq!(entries.len(), 1);
        // Only empty folders picked: nothing to send.
        assert!(gather(&[d.join("Proj/empty"), d.join("Proj/deep")], Mode::Zip).is_err());
    }

    #[test]
    fn sent_items_keep_their_own_key_out_of_the_window() {
        // An older sent.json (no key, no "incomplete") still reads.
        let old = r#"[{"id":"a","name":"n","size":1,"files":1,"date":1,"link":"https://gofile.io/d/x","folderId":"f","fileId":null,"compressed":false}]"#;
        let list: Vec<SentItem> = serde_json::from_str(old).unwrap();
        assert!(!list[0].incomplete && list[0].key.is_none());
        let saved = serde_json::to_string(&list[0]).unwrap();
        assert!(!saved.contains("incomplete") && !saved.contains("\"key\""));
        let item = SentItem { key: Some("secret".into()), incomplete: true, ..list[0].clone() };
        assert!(serde_json::to_string(&item).unwrap().contains("\"key\":\"secret\""));
        let view = serde_json::to_string(&SentView { item: SentItem { key: None, ..item }, can_delete: true, soon: false }).unwrap();
        assert!(!view.contains("secret") && view.contains("\"incomplete\":true") && view.contains("\"canDelete\":true"));
    }

    #[test]
    fn sent_items_say_where_they_went() {
        // Older items (and Gofile ones) have no host and stay Gofile; Drive ones say "drive".
        let old = r#"[{"id":"a","name":"n","size":1,"files":1,"date":1,"link":"https://gofile.io/d/x","folderId":"f","fileId":null,"compressed":false}]"#;
        let list: Vec<SentItem> = serde_json::from_str(old).unwrap();
        assert!(!list[0].on_drive());
        assert!(!serde_json::to_string(&list[0]).unwrap().contains("host"), "Gofile items are saved as before");
        let d = SentItem { host: Some("drive".into()), ..list[0].clone() };
        let back: SentItem = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert!(back.on_drive());
        assert_eq!(Host::from_setting("drive"), Host::Drive);
        assert_eq!(Host::from_setting("gofile"), Host::Gofile);
        assert_eq!(Host::from_setting(""), Host::Gofile, "no setting yet: Gofile");
    }

    /// A local stand-in for Gofile: checks the upload says its exact size up front, carries the
    /// whole ZIP in a "file" field, and that the reply is read.
    #[tokio::test]
    async fn upload_request_has_size_and_body() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let d = tmp_dir("upload");
        let Payload::Zip { entries, .. } = gather(&[d.join("Photos")], Mode::Zip).unwrap() else { panic!() };
        let mut zip = Vec::new();
        write_zip(&entries, Pack::Off, &mut zip, &AtomicBool::new(false), Z64_AT).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut got = Vec::new();
            let mut buf = vec![0u8; 64 * 1024];
            // Read until the end of the multipart body.
            loop {
                let n = sock.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
                if let Some(h) = got.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&got[..h]).to_lowercase();
                    let len: usize = head.lines().find_map(|l| l.strip_prefix("content-length: ")).and_then(|v| v.trim().parse().ok()).unwrap_or(usize::MAX);
                    if got.len() >= h + 4 + len {
                        break;
                    }
                }
            }
            let reply = r#"{"status":"ok","data":{"downloadPage":"https://gofile.io/d/AbC","code":"AbC","parentFolder":"f1","fileId":"x1","guestToken":"tok"}}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                reply.len(),
                reply
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
            got
        });
        let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(8);
        let total = zip_size(&entries, Pack::Off, Z64_AT);
        let feeder = tokio::task::spawn_blocking({
            let entries = entries.clone();
            move || {
                let mut out = ToUpload { tx, buf: Vec::new(), produced: Arc::new(AtomicU64::new(0)), mine: Arc::new(AtomicU64::new(0)) };
                write_zip(&entries, Pack::Off, &mut out, &AtomicBool::new(false), Z64_AT)
            }
        });
        let http = reqwest::Client::new();
        let (code, text) = post_upload(&http, &format!("http://{addr}/uploadfile"), Some("tok"), Some("f1"), "Photos.zip", rx, total).await.unwrap();
        feeder.await.unwrap().unwrap();
        let got = server.await.unwrap();
        assert_eq!(code, 200);
        let reply: UploadReply = serde_json::from_str(&text).unwrap();
        assert_eq!(reply.data.unwrap().download_page.as_deref(), Some("https://gofile.io/d/AbC"));
        let h = got.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&got[..h]).to_lowercase();
        assert!(head.contains("content-length: "), "size said up front:\n{head}");
        assert!(!head.contains("transfer-encoding: chunked"));
        assert!(head.contains("authorization: bearer tok"));
        let body = &got[h + 4..];
        assert!(String::from_utf8_lossy(body).contains(r#"name="file"; filename="Photos.zip""#));
        assert!(String::from_utf8_lossy(body).contains("name=\"folderId\"\r\n\r\nf1"), "goes into the folder");
        assert!(body.windows(zip.len()).any(|w| w == zip.as_slice()), "the whole ZIP is in the upload");
    }

    #[test]
    fn several_files_as_they_are_by_default() {
        let d = tmp_dir("asis");
        let p = gather(&[d.join("Photos"), d.join("note.txt")], Mode::AsTheyAre).unwrap();
        let Payload::Files { name, entries } = p else { panic!("files expected") };
        assert_eq!(name, "4 files");
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["note.txt", "Photos - a.txt", "Photos - sub - b.jpg", "Photos - sub - empty.log"]);
        // One folder picked: its own name is left out (the link shows it).
        let Payload::Files { name, entries } = gather(&[d.join("Photos")], Mode::AsTheyAre).unwrap() else { panic!() };
        assert_eq!(name, "Photos");
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["a.txt", "sub - b.jpg", "sub - empty.log"]);
        assert!(matches!(gather(&[d.join("note.txt")], Mode::AsTheyAre).unwrap(), Payload::File(_)));
    }

    #[test]
    fn send_setting_carries_over() {
        assert_eq!(Mode::from_settings(None, "off"), Mode::AsTheyAre);
        assert_eq!(Mode::from_settings(None, "auto"), Mode::Compress);
        assert_eq!(Mode::from_settings(None, "max"), Mode::Compress);
        assert_eq!(Mode::from_settings(Some("zip"), "max"), Mode::Zip);
        assert_eq!(Mode::from_settings(Some("files"), "max"), Mode::AsTheyAre);
        assert_eq!(flat_name("a/b/c.txt", false), "a - b - c.txt");
        assert_eq!(flat_name("a/c.txt", true), "c.txt");
        assert_eq!(flat_name("c.txt", true), "c.txt");
    }

    #[test]
    fn reads_download_counts() {
        // A folder (several files sent as they are): children by id, counts added up.
        let folder = r#"{"status":"ok","data":{"id":"f1","type":"folder","name":"Photos","childrenCount":3,
            "children":{"a":{"id":"a","type":"file","name":"a.jpg","size":10,"downloadCount":2},
                        "b":{"id":"b","type":"file","name":"b.jpg","size":10,"downloadCount":0},
                        "c":{"id":"c","type":"file","name":"c.jpg","size":10,"downloadCount":3}}}}"#;
        assert_eq!(parse_contents(200, folder), Remote::Found(5));
        // One file's folder, never downloaded.
        let fresh = r#"{"status":"ok","data":{"type":"folder","children":{"a":{"type":"file","downloadCount":0}}}}"#;
        assert_eq!(parse_contents(200, fresh), Remote::Found(0));
        // No count given at all: not downloaded as far as Gofile says.
        assert_eq!(parse_contents(200, r#"{"status":"ok","data":{"type":"folder","children":{}}}"#), Remote::Found(0));
        // The total on the folder, when that's all there is (children as ids only).
        let total = r#"{"status":"ok","data":{"type":"folder","childrenIds":["a"],"totalDownloadCount":7}}"#;
        assert_eq!(parse_contents(200, total), Remote::Found(7));
        // A file asked for directly, and children as a list.
        assert_eq!(parse_contents(200, r#"{"status":"ok","data":{"type":"file","downloadCount":4}}"#), Remote::Found(4));
        assert_eq!(parse_contents(200, r#"{"status":"ok","data":{"type":"folder","children":[{"downloadCount":1},{"downloadCount":1}]}}"#), Remote::Found(2));
        // Deleted or expired.
        assert_eq!(parse_contents(404, r#"{"status":"error-notFound","data":{}}"#), Remote::Gone);
        assert_eq!(parse_contents(200, r#"{"status":"error-notFound"}"#), Remote::Gone);
        assert_eq!(parse_contents(404, "Not Found"), Remote::Gone);
        // Not shared with a free account.
        assert_eq!(parse_contents(401, r#"{"status":"error-notPremium","data":{}}"#), Remote::Refused);
        assert_eq!(parse_contents(200, r#"{"status":"error-notPremium"}"#), Remote::Refused);
        assert_eq!(parse_contents(403, ""), Remote::Refused);
        // Busy, slow down, garbage: try another time (never "gone").
        assert_eq!(parse_contents(429, r#"{"status":"error-rateLimit"}"#), Remote::Unknown);
        assert_eq!(parse_contents(503, "<html>"), Remote::Unknown);
        assert_eq!(parse_contents(200, r#"{"status":"ok"}"#), Remote::Unknown);
    }

    #[test]
    fn counts_and_the_keep_limit() {
        let now = 100 * DAY_MS;
        let mut s: SentItem = serde_json::from_str(
            r#"{"id":"a","name":"n","size":1,"files":1,"date":0,"link":"https://gofile.io/d/x","folderId":"f","fileId":null,"compressed":false}"#,
        )
        .unwrap();
        s.date = now - 8 * DAY_MS;
        // Count not known: no hint.
        assert!(!removal_soon(&s, now));
        // First count: 0, sent 8 days ago → close to the 10 days.
        apply_to(&mut s, Remote::Found(0), now);
        assert_eq!((s.downloads, s.downloaded_at), (Some(0), None));
        assert!(removal_soon(&s, now));
        // The count went up: downloaded about now, so far from the limit.
        apply_to(&mut s, Remote::Found(2), now);
        assert_eq!((s.downloads, s.downloaded_at), (Some(2), Some(now)));
        assert!(!removal_soon(&s, now));
        assert!(!removal_soon(&s, now + 6 * DAY_MS));
        assert!(removal_soon(&s, now + 7 * DAY_MS));
        // Same count later: the last download stays when it was.
        apply_to(&mut s, Remote::Found(2), now + DAY_MS);
        assert_eq!(s.downloaded_at, Some(now));
        // Unknown answers change nothing; gone is kept (and has no hint).
        apply_to(&mut s, Remote::Unknown, now);
        apply_to(&mut s, Remote::Refused, now);
        assert_eq!(s.downloads, Some(2));
        apply_to(&mut s, Remote::Gone, now);
        assert!(s.gone && !removal_soon(&s, now + 9 * DAY_MS));
        // Saved with the item.
        let saved = serde_json::to_string(&s).unwrap();
        assert!(saved.contains("\"downloads\":2") && saved.contains("\"downloadedAt\"") && saved.contains("\"gone\":true"));
    }

    /// A local stand-in for Gofile's contents address: the key goes as "Bearer", the id in the path.
    #[tokio::test]
    async fn asks_with_the_items_own_key() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut got = Vec::new();
            let mut buf = vec![0u8; 8 * 1024];
            while !got.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = sock.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
            }
            let reply = r#"{"status":"ok","data":{"type":"folder","children":{"a":{"type":"file","downloadCount":3}}}}"#;
            let resp = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}", reply.len(), reply);
            sock.write_all(resp.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&got).to_lowercase()
        });
        let http = reqwest::Client::new();
        let r = fetch_remote(&http, &format!("http://{addr}/contents"), "f1", "tok").await;
        assert_eq!(r, Remote::Found(3));
        let head = server.await.unwrap();
        assert!(head.starts_with("get /contents/f1 "), "{head}");
        assert!(head.contains("authorization: bearer tok"));
        // Nobody answering: unknown, not gone.
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let dead = closed.local_addr().unwrap();
        drop(closed);
        assert_eq!(fetch_remote(&http, &format!("http://{dead}/contents"), "f1", "tok").await, Remote::Unknown);
    }

    #[test]
    fn plain_errors() {
        assert!(status_error(503, "").contains("busy"));
        assert!(status_error(413, "").contains("too big"));
        assert!(status_error(400, r#"{"status":"error-notPremium"}"#).contains("error-notPremium"));
    }

    #[test]
    fn upload_count_goes_back_down_however_a_send_ends() {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        {
            let _a = Uploading::start(&COUNT);
            let _b = Uploading::start(&COUNT);
            assert_eq!(COUNT.load(Ordering::SeqCst), 2);
        }
        assert_eq!(COUNT.load(Ordering::SeqCst), 0);
        // Moved into the task that runs the send, like start() does.
        let up = Uploading::start(&COUNT);
        std::thread::spawn(move || {
            let _up = up;
        })
        .join()
        .unwrap();
        assert_eq!(COUNT.load(Ordering::SeqCst), 0);
        // A send that panics still counts down.
        let r = std::panic::catch_unwind(|| {
            let _up = Uploading::start(&COUNT);
            panic!("send broke");
        });
        assert!(r.is_err());
        assert_eq!(COUNT.load(Ordering::SeqCst), 0);
    }

    /// A several-files send that failed part-way, with one file left and its Sent item listed.
    fn failed_send(listed: bool) -> Inner {
        let job = Arc::new(Job::new("Photos".into(), 2, 2, Some(2), vec![], Mode::AsTheyAre, true, Host::Gofile));
        *job.phase.lock().unwrap() = Phase::Failed("1 file didn't upload.".into());
        *job.folder.lock().unwrap() = Some(("folder1".into(), "https://gofile.io/d/x".into()));
        let e = Entry { path: PathBuf::from("b.jpg"), name: "b.jpg".into(), size: 1, mtime: SystemTime::now(), dir: false };
        *job.left.lock().unwrap() = vec![e];
        let item = SentItem {
            id: job.id.clone(),
            name: "Photos".into(),
            size: 1,
            files: 1,
            date: 1,
            link: "https://gofile.io/d/x".into(),
            folder_id: Some("folder1".into()),
            file_id: None,
            compressed: false,
            as_files: true,
            incomplete: true,
            key: None,
            downloads: None,
            downloaded_at: None,
            gone: false,
            host: None,
            ..Default::default()
        };
        Inner { jobs: vec![job], sent: if listed { vec![item] } else { vec![] }, token: None, ticking: false }
    }

    #[test]
    fn try_again_only_while_its_sent_item_is_listed() {
        // Listed: the missing file goes again, and a second click does nothing more.
        let inner = failed_send(true);
        let id = inner.jobs[0].id.clone();
        assert!(matches!(inner.begin_resume(&id), Resume::Go(_, ref left) if left.len() == 1));
        assert!(inner.jobs[0].uploading());
        assert!(matches!(inner.begin_resume(&id), Resume::Running));
        // Sent item deleted (or taken off the list): nothing is uploaded, the files left stay.
        let inner = failed_send(false);
        let id = inner.jobs[0].id.clone();
        assert!(matches!(inner.begin_resume(&id), Resume::Nothing));
        assert!(!inner.jobs[0].uploading());
        assert_eq!(inner.jobs[0].left.lock().unwrap().len(), 1);
        assert!(matches!(inner.begin_resume("other"), Resume::Nothing));
    }

    #[test]
    fn deleting_from_gofile_takes_its_failed_send_off() {
        let mut inner = failed_send(true);
        let id = inner.jobs[0].id.clone();
        let (at, job) = inner.take_failed(&id).unwrap().expect("its failed send");
        assert_eq!((at, job.id.as_str()), (0, id.as_str()));
        assert!(inner.jobs.is_empty());
        // Gone from the jobs: "Try again" can't send into the deleted folder.
        assert!(matches!(inner.begin_resume(&id), Resume::Nothing));
        assert!(matches!(inner.take_failed(&id), Ok(None)));
        // Still uploading: not taken off (deleting now would only make the rest fail).
        let mut inner = failed_send(true);
        let id = inner.jobs[0].id.clone();
        assert!(matches!(inner.begin_resume(&id), Resume::Go(..)));
        assert!(inner.take_failed(&id).is_err());
        assert_eq!(inner.jobs.len(), 1);
    }

    /// A date changed while the sweep runs (here: by the delete before it) wins: that upload
    /// stays. One that the check right before deleting turns away isn't a failure either.
    #[tokio::test]
    async fn sweep_leaves_what_changed_meanwhile() {
        let now = 10 * DAY_MS;
        let list = Mutex::new(vec![item("a", Some(now - 5)), item("b", Some(now - 5)), item("c", Some(now - 5))]);
        let del = |it: SentItem| {
            let list = &list;
            async move {
                if it.id == "a" {
                    // You pressed "Keep 7 more days" on "b" meanwhile.
                    list.lock().unwrap().iter_mut().find(|s| s.id == "b").unwrap().delete_at = Some(now + 7 * DAY_MS);
                    list.lock().unwrap().retain(|s| s.id != "a");
                    return Ok(true);
                }
                // "c" changed in Pako's own list (deleted by you): turned away.
                Ok(false)
            }
        };
        let swept = sweep_list(&list, now, &[], del).await;
        assert_eq!(swept.deleted.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["a"]);
        assert!(swept.failed.is_empty());
        let l = list.lock().unwrap();
        assert_eq!(l.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["b", "c"]);
        assert_eq!(l.iter().find(|s| s.id == "c").unwrap().delete_tries, 0);
    }

    /// One change at a time per upload; letting go frees it.
    #[test]
    fn one_change_at_a_time() {
        let c = Claims::default();
        let first = c.take("a");
        assert!(first.is_some());
        assert!(c.take("a").is_none(), "a second change waits");
        assert!(c.take("b").is_some(), "other uploads aren't held up");
        drop(first);
        assert!(c.take("a").is_some());
    }

    /// A damaged entry doesn't lose the others, and the file is kept aside before anything is
    /// saved over it.
    #[test]
    fn sent_list_survives_a_bad_entry() {
        let d = std::env::temp_dir().join(format!("pako-send-test-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("sent.json");
        // Missing: an empty list, nothing said.
        let l = load_sent(&p);
        assert!(l.items.is_empty() && l.note.is_none() && !l.locked);
        // All good.
        let good = serde_json::to_string(&vec![item("a", None), item("b", Some(5))]).unwrap();
        std::fs::write(&p, &good).unwrap();
        let l = load_sent(&p);
        assert_eq!(l.items.len(), 2);
        assert!(l.note.is_none());
        // One entry damaged (size isn't a number): the other two stay, a copy is kept.
        let text = format!(r#"[{}, {{"id":"x","name":"x.zip","size":"big"}}, {}]"#, serde_json::to_string(&item("a", None)).unwrap(), serde_json::to_string(&item("b", None)).unwrap());
        std::fs::write(&p, &text).unwrap();
        let l = load_sent(&p);
        assert_eq!(l.items.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
        assert!(l.note.as_deref().unwrap().starts_with("1 upload in your list couldn't be read"));
        assert!(!l.locked);
        let kept: Vec<_> = std::fs::read_dir(&d).unwrap().filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().starts_with("sent.json.bad-")).collect();
        assert_eq!(kept.len(), 1);
        assert_eq!(std::fs::read_to_string(kept[0].path()).unwrap(), text);
        // Not a list at all: starts again, the old file kept.
        std::fs::write(&p, "{ not json").unwrap();
        let l = load_sent(&p);
        assert!(l.items.is_empty() && !l.locked);
        assert!(l.note.as_deref().unwrap().starts_with("Pako couldn't read your list of uploads"));
        // Can't be opened (a folder in its place): nothing is written over it.
        let q = d.join("folder.json");
        std::fs::create_dir_all(&q).unwrap();
        assert!(load_sent(&q).locked);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn clock_check() {
        assert_eq!(http_date_ms("Sun, 06 Nov 1994 08:49:37 GMT"), Some(784_111_777_000));
        assert_eq!(http_date_ms("yesterday"), None);
        let server = 1_800_000_000_000;
        assert!(!clock_off(server + 3_600_000, server));
        assert!(!clock_off(server - DAY_MS, server));
        assert!(clock_off(server + DAY_MS + 1, server), "a PC clock days ahead deletes nothing");
        assert!(clock_off(server - 2 * DAY_MS, server));
    }

    #[test]
    fn drive_accounts_are_listed_once() {
        let drive = |id: &str, account: Option<&str>, gone: bool| SentItem { host: Some("drive".into()), account: account.map(str::to_string), gone, ..item(id, None) };
        let list = vec![
            drive("a", Some("me@gmail.com"), false),
            drive("b", Some("Me@Gmail.com"), false),
            drive("c", Some("work@sookera.com"), false),
            drive("d", Some("old@gmail.com"), true),
            drive("e", None, false),
            SentItem { account: Some("x@gmail.com".into()), ..item("f", None) },
        ];
        assert_eq!(drive_accounts(&list), vec!["me@gmail.com", "work@sookera.com"]);
    }
}
