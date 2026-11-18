// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Outside tools Pako can use, installed from Settings › Features & updates into data/components:
//! 7-Zip to extract split archives; ffmpeg, yt-dlp and Deno for video.
//!
//! Each one comes only from its official source (GitHub releases of the projects, or the
//! Windows builds ffmpeg.org links to), and is checked against the published checksum when
//! the project publishes one.

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::paths::data_dir;

pub fn dir() -> PathBuf {
    data_dir().join("components")
}

/// 7-Zip's command-line program, if installed.
pub fn seven_zip() -> Option<PathBuf> {
    let d = dir().join("7zip");
    ["7z.exe", "7za.exe"].into_iter().map(|n| d.join(n)).find(|p| p.is_file())
}

/// A tool's program, if installed.
pub fn path(kind: Kind) -> Option<PathBuf> {
    restore_interrupted(kind);
    Some(kind.exe()).filter(|p| p.is_file())
}

/// yt-dlp's Windows folder build: yt-dlp.exe with its `_internal` folder next to it. It starts in
/// about a third of a second; the single yt-dlp.exe older Pako versions installed unpacks itself
/// on every run (about a second and a half).
const YTDLP_ZIP: &str = "yt-dlp_win.zip";

/// yt-dlp's folder holds the single yt-dlp.exe of an older Pako (no `_internal` next to it).
fn single_exe_in(folder: &Path) -> bool {
    folder.join("yt-dlp.exe").is_file() && !folder.join("_internal").is_dir()
}

/// The installed copy is an older kind of build, replaced at the next update even when its
/// version is the latest (yt-dlp's single exe -> its folder build). It keeps working until then.
pub fn old_build(kind: Kind) -> bool {
    kind == Kind::YtDlp && single_exe_in(&dir().join(kind.folder()))
}

/// The note a failed move to the new kind of build leaves (the release it tried).
fn rebuild_note(base: &Path, kind: Kind) -> PathBuf {
    base.join(format!(".rebuild-failed-{}", kind.folder()))
}

/// An older kind of build is to be replaced now, at the same version (`latest`): not when moving
/// it already failed for that release (the new build didn't start on this PC, say). Else a
/// failure that stays would download yt-dlp again every hour; the next release tries again.
pub fn rebuild_due(kind: Kind, latest: &str) -> bool {
    rebuild_due_in(&dir(), kind, latest)
}

fn rebuild_due_in(base: &Path, kind: Kind, latest: &str) -> bool {
    kind == Kind::YtDlp
        && single_exe_in(&base.join(kind.folder()))
        && std::fs::read_to_string(rebuild_note(base, kind)).map_or(true, |v| v.trim() != latest)
}

/// A program is running from `folder` (or a folder in it). Windows lets a folder be moved while
/// a program in it runs when nothing else in it is open: yt-dlp's single exe is, and it then
/// stops with "appears to have been moved or deleted". So "in use" isn't left to the move alone.
#[cfg(windows)]
fn running_from(folder: &Path) -> bool {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
    let norm = |p: &Path| p.to_string_lossy().trim_start_matches(r"\\?\").replace('/', "\\").to_lowercase();
    let Ok(real) = std::fs::canonicalize(folder) else { return false };
    let mut prefix = norm(&real);
    if !prefix.ends_with('\\') {
        prefix.push('\\');
    }
    let mut found = false;
    // SAFETY: the snapshot and process handles are checked and closed; the entry's size is set
    // before use; the buffer holds `len` UTF-16 units and Windows writes at most that many.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return false;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut buf = vec![0u16; 32 * 1024];
        let mut ok = Process32FirstW(snap, &mut e);
        while ok != 0 && !found {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, e.th32ProcessID);
            if !h.is_null() {
                let mut len = buf.len() as u32;
                if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) != 0 {
                    let image = PathBuf::from(std::ffi::OsString::from_wide(&buf[..len as usize]));
                    let image = std::fs::canonicalize(&image).unwrap_or(image);
                    found = norm(&image).starts_with(&prefix);
                }
                CloseHandle(h);
            }
            ok = Process32NextW(snap, &mut e);
        }
        CloseHandle(snap);
    }
    found
}

#[cfg(not(windows))]
fn running_from(_folder: &Path) -> bool {
    false
}

/// What an install, an update or a removal may leave next to a tool's folder.
fn leftovers(kind: Kind) -> [String; 5] {
    let f = kind.folder();
    [format!(".new-{f}"), format!(".download-{f}"), format!(".check-{f}"), format!(".prev-{f}"), format!(".old-{f}")]
}

/// An update cut off between its two moves (Pako closed or crashed) leaves the tool's folder
/// aside: it is put back, so the tool keeps working. Never while it is installing.
fn restore_interrupted(kind: Kind) {
    if !is_installing(kind) {
        restore_in(&dir(), kind);
    }
}

/// `restore_interrupted` in a given folder: `.prev-<folder>` (an update's) or `.check-<folder>`
/// (the in-use check's) holding the program goes back in place when the folder is missing.
fn restore_in(base: &Path, kind: Kind) {
    let folder = kind.folder();
    let target = base.join(folder);
    if target.exists() {
        return;
    }
    for aside in [format!(".prev-{folder}"), format!(".check-{folder}")] {
        let aside = base.join(aside);
        if aside.join(kind.exe_name()).is_file() && std::fs::rename(&aside, &target).is_ok() {
            return;
        }
    }
}

/// ffmpeg, if installed (video preview of MKV/AVI, joining video and sound).
pub fn ffmpeg() -> Option<PathBuf> {
    Some(dir().join("ffmpeg").join("ffmpeg.exe")).filter(|p| p.is_file())
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    SevenZip,
    Ffmpeg,
    YtDlp,
    Deno,
}

pub const ALL: [Kind; 4] = [Kind::SevenZip, Kind::Ffmpeg, Kind::YtDlp, Kind::Deno];

impl Kind {
    fn folder(self) -> &'static str {
        match self {
            Kind::SevenZip => "7zip",
            Kind::Ffmpeg => "ffmpeg",
            Kind::YtDlp => "yt-dlp",
            Kind::Deno => "deno",
        }
    }
    fn exe_name(self) -> &'static str {
        match self {
            Kind::SevenZip => "7z.exe",
            Kind::Ffmpeg => "ffmpeg.exe",
            Kind::YtDlp => "yt-dlp.exe",
            Kind::Deno => "deno.exe",
        }
    }
    fn exe(self) -> PathBuf {
        dir().join(self.folder()).join(self.exe_name())
    }
    /// The tool's own name.
    pub fn label(self) -> &'static str {
        match self {
            Kind::SevenZip => "7-Zip",
            Kind::Ffmpeg => "ffmpeg",
            Kind::YtDlp => "yt-dlp",
            Kind::Deno => "Deno",
        }
    }
    fn repo(self) -> &'static str {
        match self {
            Kind::SevenZip => "ip7z/7zip",
            Kind::Ffmpeg => "BtbN/FFmpeg-Builds",
            Kind::YtDlp => "yt-dlp/yt-dlp",
            Kind::Deno => "denoland/deno",
        }
    }
}

/// What Settings shows for one tool. Sent as { kind, installed, latest, installing }:
/// `installing` is read when it is sent, so the window sees an install that is running.
#[derive(Clone, Debug)]
pub struct Status {
    pub kind: Kind,
    pub installed: Option<String>,
    /// Known after "Check for new versions".
    pub latest: Option<String>,
}

/// Disk space a tool takes (its folder).
pub fn size_on_disk(kind: Kind) -> u64 {
    fn walk(p: &Path) -> u64 {
        std::fs::read_dir(p)
            .map(|rd| {
                rd.flatten()
                    .map(|e| match e.metadata() {
                        Ok(m) if m.is_dir() => walk(&e.path()),
                        Ok(m) => m.len(),
                        Err(_) => 0,
                    })
                    .sum()
            })
            .unwrap_or(0)
    }
    walk(&dir().join(kind.folder()))
}

/// Removes a tool (frees its space). Refused while it is installing or in use.
pub fn remove(kind: Kind) -> Result<()> {
    remove_all(&[kind])
}

/// Removes several tools together (a feature's tools): every folder is moved aside first, and
/// if one of them is in use, the others are put back, so a feature is never half removed.
pub fn remove_all(kinds: &[Kind]) -> Result<()> {
    // Held while removing: no install (daily, "site changed") starts in between.
    let _guards = kinds
        .iter()
        .map(|k| InstallGuard::take(*k).map_err(|_| anyhow!("{} is being installed right now.", k.label())))
        .collect::<Result<Vec<_>>>()?;
    remove_in(&dir(), kinds)
}

/// `remove_all` in a given folder.
fn remove_in(base: &Path, kinds: &[Kind]) -> Result<()> {
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for &kind in kinds {
        let target = base.join(kind.folder());
        if !target.exists() {
            continue;
        }
        // Moving it aside fails while the tool runs (and is never tried then): nothing is
        // half-deleted.
        let old = base.join(format!(".old-{}", kind.folder()));
        let _ = std::fs::remove_dir_all(&old);
        if running_from(&target) || std::fs::rename(&target, &old).is_err() {
            for (target, old) in moved.iter().rev() {
                let _ = std::fs::rename(old, target);
            }
            bail!("{} is in use (a preview, a video or an extraction is running). Close it, then try again.", kind.label());
        }
        moved.push((target, old));
    }
    // What an install or update cut off earlier left goes too: it would take space, and an
    // update's (or the in-use check's) copy aside must not bring a removed tool back.
    for &kind in kinds {
        for name in leftovers(kind) {
            let _ = std::fs::remove_dir_all(base.join(name));
        }
        let _ = std::fs::remove_file(rebuild_note(base, kind));
    }
    Ok(())
}

/// The note Pako leaves in a tool's folder: which release it installed. ffmpeg's own version
/// ("n9.0.2-57-g…") never reads like the release's ("9.0"), so updates compare to this note.
const RECORD: &str = "pako-installed.json";

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
struct Record {
    /// The release's version, as `latest_version` reads it.
    version: String,
    /// The file that was downloaded.
    #[serde(default)]
    asset: String,
}

fn read_record(kind: Kind) -> Option<Record> {
    let r: Record = serde_json::from_str(&std::fs::read_to_string(dir().join(kind.folder()).join(RECORD)).ok()?).ok()?;
    (!r.version.is_empty()).then_some(r)
}

/// The installed copy is the latest one. Without Pako's note (installed by an older Pako), an
/// ffmpeg "9.0.2" is a build of the "9.0" release line.
pub fn same_version(kind: Kind, installed: &str, latest: &str) -> bool {
    installed == latest || (kind == Kind::Ffmpeg && installed.strip_prefix(latest).is_some_and(|rest| rest.starts_with('.')))
}

/// Checks that a tool's folder can be replaced (not in use) before anything is downloaded.
fn can_swap(kind: Kind) -> Result<()> {
    let target = dir().join(kind.folder());
    if !target.exists() {
        return Ok(());
    }
    let probe = dir().join(format!(".check-{}", kind.folder()));
    let _ = std::fs::remove_dir_all(&probe);
    if running_from(&target) || std::fs::rename(&target, &probe).is_err() {
        bail!("{} is in use (a preview, a video or an extraction is running). Close it, then try again.", kind.label());
    }
    std::fs::rename(&probe, &target).map_err(|e| anyhow!("Couldn't put {} back in place: {e}", kind.label()))
}

impl Serialize for Status {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("Status", 8)?;
        st.serialize_field("kind", &self.kind)?;
        st.serialize_field("installed", &self.installed)?;
        st.serialize_field("latest", &self.latest)?;
        // A newer version than the one installed (compared the way updates compare), or the
        // same one in the kind of build Pako now installs.
        let update = match (&self.installed, &self.latest) {
            (Some(have), Some(latest)) => rebuild_due(self.kind, latest) || (have != "installed" && !same_version(self.kind, have, latest)),
            _ => false,
        };
        st.serialize_field("update", &update)?;
        st.serialize_field("installing", &is_installing(self.kind))?;
        st.serialize_field("name", self.kind.label())?;
        st.serialize_field("source", &format!("github.com/{}", self.kind.repo()))?;
        st.serialize_field("size", &self.installed.as_ref().map(|_| size_on_disk(self.kind)))?;
        st.end()
    }
}

/// Tools being installed right now (two installs of one tool would share its temporary folders).
static INSTALLING: LazyLock<Mutex<HashSet<Kind>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn is_installing(kind: Kind) -> bool {
    INSTALLING.lock().unwrap().contains(&kind)
}

/// Marks a tool as installing until dropped (also when the install fails or is cancelled).
struct InstallGuard(Kind);

impl InstallGuard {
    fn take(kind: Kind) -> Result<Self> {
        if !INSTALLING.lock().unwrap().insert(kind) {
            bail!("{} is already being installed.", kind.label());
        }
        Ok(Self(kind))
    }
}

impl Drop for InstallGuard {
    fn drop(&mut self) {
        INSTALLING.lock().unwrap().remove(&self.0);
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize, Clone)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Runs a tool once to read its version (hidden, no console window). Never yt-dlp's own `-U`:
/// Pako updates its tools itself.
fn run_version(exe: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok()?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    Some(text)
}

/// The installed version: the release Pako noted when it installed the tool, else read from the
/// tool itself.
pub fn installed_version(kind: Kind) -> Option<String> {
    restore_interrupted(kind);
    let exe = kind.exe();
    if !exe.is_file() {
        return None;
    }
    if let Some(r) = read_record(kind) {
        return Some(r.version);
    }
    let text = match kind {
        Kind::SevenZip => run_version(&exe, &[]),
        Kind::Ffmpeg => run_version(&exe, &["-version"]),
        Kind::YtDlp => run_version(&exe, &["--no-config", "--no-update", "--version"]),
        Kind::Deno => run_version(&exe, &["--version"]),
    }?;
    Some(parse_version(kind, &text).unwrap_or_else(|| "installed".into()))
}

fn parse_version(kind: Kind, text: &str) -> Option<String> {
    let first = text.lines().find(|l| !l.trim().is_empty())?.trim();
    let word_after = |prefix: &str| -> Option<String> {
        let i = first.find(prefix)? + prefix.len();
        first[i..].split_whitespace().next().map(String::from)
    };
    match kind {
        // "7-Zip 26.06 (x64) : Copyright ..."
        Kind::SevenZip => word_after("7-Zip "),
        // "ffmpeg version n9.0-12-g... Copyright" -> "9.0"
        Kind::Ffmpeg => word_after("version ").map(|v| {
            let v = v.trim_start_matches('n');
            v.split(['-', '_']).next().unwrap_or(v).to_string()
        }),
        // "2026.10.26"
        Kind::YtDlp => Some(first.to_string()),
        // "deno 2.9.8 (stable, release, x86_64-pc-windows-msvc)"
        Kind::Deno => word_after("deno "),
    }
}

async fn latest_release(http: &reqwest::Client, kind: Kind) -> Result<Release> {
    // ffmpeg: BtbN keeps the stable builds in the release tagged "latest"; what GitHub calls the
    // latest release is the day's automatic build (other file names). That one only if the tag
    // has no Windows build.
    if kind == Kind::Ffmpeg {
        if let Ok(rel) = release_at(http, kind, "tags/latest").await {
            if pick_ffmpeg(&rel.assets).is_some() {
                return Ok(rel);
            }
        }
    }
    release_at(http, kind, "latest").await
}

async fn release_at(http: &reqwest::Client, kind: Kind, which: &str) -> Result<Release> {
    let url = format!("https://api.github.com/repos/{}/releases/{which}", kind.repo());
    let r = http
        .get(url)
        .header(reqwest::header::USER_AGENT, "Pako")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .context("Couldn't reach GitHub")?;
    if !r.status().is_success() {
        bail!("GitHub answered {}", r.status().as_u16());
    }
    Ok(r.json().await?)
}

/// The newest stable ffmpeg build: "ffmpeg-n9.0-latest-win64-gpl-9.0.zip" (the "latest" tag) or
/// "ffmpeg-n9.0.2-57-gd838f9f9ae-win64-gpl-9.0.zip" (a day's build) -> (asset, "9.0"). Never the
/// "shared" or "lgpl" builds, or master ("ffmpeg-master-…", "ffmpeg-N-…").
fn pick_ffmpeg(assets: &[Asset]) -> Option<(Asset, String)> {
    assets
        .iter()
        .filter_map(|a| {
            let rest = a.name.strip_prefix("ffmpeg-n")?.strip_suffix(".zip")?;
            let (_, ver) = rest.rsplit_once("-win64-gpl-")?;
            if ver.is_empty() || !ver.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return None;
            }
            let ver = ver.to_string();
            let key: Vec<u32> = ver.split('.').filter_map(|p| p.parse().ok()).collect();
            Some((key, a.clone(), ver))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, a, v)| (a, v))
}

/// The version a release offers.
fn release_version(kind: Kind, rel: &Release) -> Option<String> {
    match kind {
        Kind::Ffmpeg => pick_ffmpeg(&rel.assets).map(|(_, v)| v),
        _ => Some(rel.tag_name.trim_start_matches('v').to_string()),
    }
}

pub async fn latest_version(http: &reqwest::Client, kind: Kind) -> Result<String> {
    let rel = latest_release(http, kind).await?;
    release_version(kind, &rel).ok_or_else(|| anyhow!("No Windows build in the latest release"))
}

/// Progress callback: (stage, bytes done, bytes total).
pub type Progress<'a> = &'a (dyn Fn(&str, u64, Option<u64>) + Send + Sync);

/// Downloads a file, reporting progress; returns its SHA-256 (hex).
async fn fetch(http: &reqwest::Client, url: &str, to: &Path, progress: Progress<'_>) -> Result<String> {
    let resp = http.get(url).header(reqwest::header::USER_AGENT, "Pako").send().await.context("Download failed")?;
    if !resp.status().is_success() {
        bail!("The download server answered {}", resp.status().as_u16());
    }
    let total = resp.content_length();
    let mut file = std::fs::File::create(to)?;
    let mut hash = Sha256::new();
    let mut done = 0u64;
    let mut last = Instant::now();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("The download was cut")?;
        file.write_all(&chunk)?;
        hash.update(&chunk);
        done += chunk.len() as u64;
        if last.elapsed() >= Duration::from_millis(250) {
            last = Instant::now();
            progress("Downloading", done, total);
        }
    }
    file.sync_all()?;
    Ok(format!("{:x}", hash.finalize()))
}

/// The SHA-256 published for `file_name` in a checksum file.
async fn published_sha256(http: &reqwest::Client, url: &str, file_name: &str) -> Result<String> {
    let text = http.get(url).header(reqwest::header::USER_AGENT, "Pako").send().await?.text().await?;
    hash_for(&text, file_name).ok_or_else(|| anyhow!("The checksum file doesn't list {file_name}"))
}

/// Finds the hash for `file_name` in the formats projects publish: "<hash>  <name>" lines
/// (yt-dlp, ffmpeg builds), a lone hash, or PowerShell's "Hash : <HASH>" block (Deno).
/// A hash on a line naming the file wins; otherwise the file's only hash is used.
fn hash_for(text: &str, file_name: &str) -> Option<String> {
    let is_hash = |w: &str| w.len() == 64 && w.chars().all(|c| c.is_ascii_hexdigit());
    let mut all = Vec::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split(|c: char| c.is_whitespace() || c == ':').filter(|w| !w.is_empty()).collect();
        // The exact file name, possibly after a folder path ("C:\a\deno\x.zip").
        let names_file = words.iter().any(|w| w.trim_start_matches('*').rsplit(['/', '\\']).next() == Some(file_name));
        for w in words.iter().filter(|w| is_hash(w)) {
            if names_file {
                return Some(w.to_lowercase());
            }
            all.push(w.to_lowercase());
        }
    }
    all.dedup();
    (all.len() == 1).then(|| all.remove(0))
}

fn asset<'a>(rel: &'a Release, name: &str) -> Result<&'a Asset> {
    rel.assets.iter().find(|a| a.name == name).ok_or_else(|| anyhow!("{name} is missing from the release"))
}

/// Unpacks a whole zip into `out`, folders included. A name that would land outside `out`
/// (`..`, an absolute path, a drive) refuses the whole zip.
fn unzip_tree(zip_path: &Path, out: &Path) -> Result<usize> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip_path)?)?;
    let mut n = 0;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i)?;
        let raw = f.name().to_string();
        // A "\" in a zip name is a folder too on Windows.
        let parts: Vec<&str> = raw.trim_end_matches(['/', '\\']).split(['/', '\\']).collect();
        if parts.iter().any(|p| p.is_empty() || *p == "." || *p == ".." || p.contains(':')) {
            bail!("The download holds a file that would land outside its folder ({raw})");
        }
        let dest = parts.iter().fold(out.to_path_buf(), |d, p| d.join(p));
        if f.is_dir() {
            std::fs::create_dir_all(&dest)?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(&dest)?;
        std::io::copy(&mut f, &mut file)?;
        // On the disk before the folder is swapped in: a power cut right after must not leave
        // an empty yt-dlp in place of the old one.
        file.sync_all()?;
        n += 1;
    }
    Ok(n)
}

/// yt-dlp's folder build, once downloaded: checked against the published checksum before
/// anything is unpacked, then unpacked whole into `out` (yt-dlp.exe and `_internal`).
fn unpack_yt_dlp(zip_path: &Path, got: &str, want: &str, out: &Path) -> Result<()> {
    if !got.eq_ignore_ascii_case(want) {
        bail!("The downloaded file doesn't match yt-dlp's published checksum");
    }
    unzip_tree(zip_path, out)?;
    if !out.join("yt-dlp.exe").is_file() || !out.join("_internal").is_dir() {
        bail!("yt-dlp.exe wasn't in the download");
    }
    Ok(())
}

/// The unpacked yt-dlp runs and says its version, before it replaces the working one.
fn yt_dlp_runs(exe: &Path) -> bool {
    run_version(exe, &["--no-config", "--no-update", "--version"])
        .and_then(|t| parse_version(Kind::YtDlp, &t))
        .is_some_and(|v| v.chars().any(|c| c.is_ascii_digit()) && v.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

/// A rename, tried a few times: right after unpacking or a run, an antivirus may still hold a
/// file for a moment.
fn rename_patiently(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut tries = 0;
    loop {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if tries >= 3 || !from.exists() => return Err(e),
            Err(_) => {
                tries += 1;
                std::thread::sleep(Duration::from_millis(250));
            }
        }
    }
}

/// Puts the folder `new` in place of `target` (`name` = the tool, for errors). The old copy is
/// moved aside to `prev` first: if that fails, the tool is in use and nothing has been touched.
/// If the new one then can't be moved in, the old copy is put back, so the tool keeps working.
/// The old copy is deleted last. (Pako closing between the two moves: `restore_in`.)
fn swap_in(new: &Path, target: &Path, prev: &Path, name: &str) -> Result<()> {
    let _ = std::fs::remove_dir_all(prev);
    if target.exists() && (running_from(target) || rename_patiently(target, prev).is_err()) {
        let _ = std::fs::remove_dir_all(new);
        bail!("{name} is in use (a preview, a video or an extraction is running). Close it, then try again.");
    }
    if let Err(e) = rename_patiently(new, target) {
        if prev.exists() {
            let _ = rename_patiently(prev, target);
        }
        let _ = std::fs::remove_dir_all(new);
        return Err(anyhow!("Couldn't put the new files in place: {e}"));
    }
    let _ = std::fs::remove_dir_all(prev);
    Ok(())
}

/// Unpacks the files of a zip whose name matches `keep` into `out` (flat).
fn unzip(zip_path: &Path, out: &Path, keep: impl Fn(&str) -> bool) -> Result<usize> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip_path)?)?;
    let mut n = 0;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i)?;
        if f.is_dir() {
            continue;
        }
        // Only the file's own name (a "\" in a zip name is a folder too on Windows), never "..".
        let name = f.name().rsplit(['/', '\\']).next().unwrap_or_default().to_string();
        if name.is_empty() || name == ".." || !keep(f.name()) {
            continue;
        }
        let mut dest = std::fs::File::create(out.join(&name))?;
        std::io::copy(&mut f, &mut dest)?;
        n += 1;
    }
    Ok(n)
}

/// Installs (or updates) one tool. Files land in a temporary folder first and replace the
/// old ones only when everything worked.
pub async fn install(http: &reqwest::Client, kind: Kind, progress: Progress<'_>) -> Result<String> {
    let _guard = InstallGuard::take(kind)?;
    // An update cut off earlier: its folder goes back first (the guard keeps `path` from it).
    restore_in(&dir(), kind);
    let _ = std::fs::remove_dir_all(dir().join(format!(".old-{}", kind.folder())));
    // Moving an older kind of build to the new one (a failure is noted, see `rebuild_due`).
    let rebuilding = old_build(kind);
    // In use: say so now, not after a 150 MB download.
    can_swap(kind)?;
    progress("Checking the latest version", 0, None);
    let rel = latest_release(http, kind).await?;
    let base = dir();
    std::fs::create_dir_all(&base)?;
    let tmp = base.join(format!(".new-{}", kind.folder()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp)?;
    let work = base.join(format!(".download-{}", kind.folder()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;

    // The new files arrived whole: a failure from here on (unpacking, the new build not
    // starting) is not a cut line, and stops an older build's move for this release.
    let mut arrived = false;
    let result: Result<()> = async {
        let asset_name: String;
        match kind {
            Kind::YtDlp => {
                // The folder build: it starts much faster than the single yt-dlp.exe.
                let a = asset(&rel, YTDLP_ZIP)?;
                asset_name = a.name.clone();
                let zip_path = work.join(YTDLP_ZIP);
                let got = fetch(http, &a.browser_download_url, &zip_path, progress).await?;
                progress("Checking the file", 0, None);
                let want = published_sha256(http, &asset(&rel, "SHA2-256SUMS")?.browser_download_url, YTDLP_ZIP).await?;
                arrived = got.eq_ignore_ascii_case(&want);
                progress("Unpacking", 0, None);
                let (z, t) = (zip_path.clone(), tmp.clone());
                tauri::async_runtime::spawn_blocking(move || unpack_yt_dlp(&z, &got, &want, &t))
                    .await
                    .map_err(|e| anyhow!("Couldn't unpack yt-dlp: {e}"))??;
                let exe = tmp.join("yt-dlp.exe");
                if !tauri::async_runtime::spawn_blocking(move || yt_dlp_runs(&exe)).await.unwrap_or(false) {
                    bail!("The new yt-dlp didn't start");
                }
            }
            Kind::Deno => {
                let name = "deno-x86_64-pc-windows-msvc.zip";
                asset_name = name.into();
                let zip_path = work.join(name);
                let got = fetch(http, &asset(&rel, name)?.browser_download_url, &zip_path, progress).await?;
                progress("Checking the file", 0, None);
                let sums = asset(&rel, &format!("{name}.sha256sum"))?;
                if got != published_sha256(http, &sums.browser_download_url, name).await? {
                    bail!("The downloaded file doesn't match Deno's published checksum");
                }
                progress("Unpacking", 0, None);
                if unzip(&zip_path, &tmp, |n| n.ends_with("deno.exe"))? == 0 {
                    bail!("deno.exe wasn't in the download");
                }
            }
            Kind::Ffmpeg => {
                let (a, _) = pick_ffmpeg(&rel.assets).ok_or_else(|| anyhow!("No Windows build in the latest release"))?;
                let sums = &asset(&rel, "checksums.sha256")?.browser_download_url;
                let zip_path = work.join(&a.name);
                asset_name = a.name.clone();
                // These builds are redone every day under the same names: read the checksum
                // before the big download, and if a rebuild lands in between, try once more.
                let mut tries = 0;
                loop {
                    tries += 1;
                    let want = published_sha256(http, sums, &a.name).await?;
                    let got = fetch(http, &a.browser_download_url, &zip_path, progress).await?;
                    progress("Checking the file", 0, None);
                    if got == want {
                        break;
                    }
                    if tries == 2 {
                        bail!("The downloaded file doesn't match the published checksum");
                    }
                }
                progress("Unpacking", 0, None);
                let n = unzip(&zip_path, &tmp, |p| p.contains("/bin/") && p.ends_with(".exe"))?;
                if n == 0 {
                    bail!("ffmpeg.exe wasn't in the download");
                }
            }
            Kind::SevenZip => {
                // 7-Zip's own small extractor (7zr) unpacks the full 64-bit 7-Zip, whose
                // 7z.exe + 7z.dll also open RAR. (7-Zip doesn't publish checksums.)
                let installer = rel
                    .assets
                    .iter()
                    .find(|a| a.name.starts_with("7z") && a.name.ends_with("-x64.exe"))
                    .ok_or_else(|| anyhow!("No 64-bit 7-Zip in the latest release"))?;
                let zr = work.join("7zr.exe");
                fetch(http, &asset(&rel, "7zr.exe")?.browser_download_url, &zr, progress).await?;
                let inst = work.join(&installer.name);
                asset_name = installer.name.clone();
                fetch(http, &installer.browser_download_url, &inst, progress).await?;
                progress("Unpacking", 0, None);
                let mut cmd = Command::new(&zr);
                cmd.arg("x").arg("-y").arg(format!("-o{}", work.join("full").display())).arg(&inst);
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    cmd.creation_flags(0x0800_0000);
                }
                let out = cmd.output().context("Couldn't run 7-Zip's extractor")?;
                if !out.status.success() {
                    bail!("Couldn't unpack 7-Zip");
                }
                for f in ["7z.exe", "7z.dll", "License.txt"] {
                    let from = work.join("full").join(f);
                    if from.is_file() {
                        std::fs::copy(&from, tmp.join(f))?;
                    }
                }
                if !tmp.join("7z.exe").is_file() || !tmp.join("7z.dll").is_file() {
                    bail!("7z.exe wasn't in the download");
                }
            }
        }
        // What was installed, for the next update check.
        let record = Record { version: release_version(kind, &rel).unwrap_or_default(), asset: asset_name };
        std::fs::write(tmp.join(RECORD), serde_json::to_vec_pretty(&record)?)?;
        Ok(())
    }
    .await;

    let _ = std::fs::remove_dir_all(&work);
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&tmp);
        if rebuilding && arrived {
            let _ = std::fs::write(rebuild_note(&base, kind), release_version(kind, &rel).unwrap_or_default());
        }
        return Err(e);
    }
    // The whole folder is replaced (from an older kind of build too, such as yt-dlp's single
    // exe), and only once everything worked. (Off the async threads: it may wait a moment.)
    let (target, prev) = (base.join(kind.folder()), base.join(format!(".prev-{}", kind.folder())));
    let new = tmp.clone();
    tauri::async_runtime::spawn_blocking(move || swap_in(&new, &target, &prev, kind.label()))
        .await
        .map_err(|e| anyhow!("Couldn't put the new files in place: {e}"))??;
    let _ = std::fs::remove_file(rebuild_note(&base, kind));
    progress("Done", 0, None);
    Ok(installed_version(kind).unwrap_or_else(|| release_version(kind, &rel).unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_versions() {
        assert_eq!(parse_version(Kind::SevenZip, "\n7-Zip 26.06 (x64) : Copyright (c) 1999-2026").as_deref(), Some("26.06"));
        assert_eq!(parse_version(Kind::Ffmpeg, "ffmpeg version n9.0-12-gabc Copyright").as_deref(), Some("9.0"));
        assert_eq!(parse_version(Kind::Deno, "deno 2.9.8 (stable, release)").as_deref(), Some("2.9.8"));
        assert_eq!(parse_version(Kind::YtDlp, "2026.10.26\n").as_deref(), Some("2026.10.26"));
    }

    #[test]
    fn reads_every_published_checksum_format() {
        let h = "cba84b26e0df5ae7f74fc66b86a95b866872a86292ebd84fa1e959927edd93bf";
        // Deno (PowerShell Get-FileHash)
        let deno = format!("\r\nAlgorithm : SHA256\r\nHash      : {}\r\nPath      : C:\\a\\deno-x86_64-pc-windows-msvc.zip\r\n", h.to_uppercase());
        assert_eq!(hash_for(&deno, "deno-x86_64-pc-windows-msvc.zip").as_deref(), Some(h));
        // ffmpeg builds / yt-dlp: one line per file
        let list = format!("{}  other.zip\n{h}  ffmpeg-n9.0-latest-win64-gpl-9.0.zip\n", "a".repeat(64));
        assert_eq!(hash_for(&list, "ffmpeg-n9.0-latest-win64-gpl-9.0.zip").as_deref(), Some(h));
        // A list that doesn't name the file: no guess between several hashes.
        assert_eq!(hash_for(&list, "missing.zip"), None);
        // A lone hash
        assert_eq!(hash_for(&format!("{h}\n"), "x.zip").as_deref(), Some(h));
    }

    #[test]
    fn one_install_per_tool_at_a_time() {
        let first = InstallGuard::take(Kind::Deno).unwrap();
        assert!(is_installing(Kind::Deno));
        assert!(InstallGuard::take(Kind::Deno).is_err());
        let other = InstallGuard::take(Kind::YtDlp).unwrap();
        drop(first);
        assert!(!is_installing(Kind::Deno));
        let json = serde_json::to_value(Status { kind: Kind::YtDlp, installed: None, latest: None }).unwrap();
        assert_eq!(json["installing"], true);
        assert_eq!(json["kind"], "ytDlp");
        drop(other);
        assert!(InstallGuard::take(Kind::Deno).is_ok());
    }

    /// Against GitHub today (network): ffmpeg's stable Windows build is found and its checksum is
    /// listed. `cargo test --lib -- --ignored ffmpeg_on_github`.
    #[test]
    #[ignore]
    fn ffmpeg_on_github() {
        tauri::async_runtime::block_on(async {
            let http = reqwest::Client::new();
            let rel = latest_release(&http, Kind::Ffmpeg).await.unwrap();
            let (a, v) = pick_ffmpeg(&rel.assets).expect("a Windows build");
            let sums = asset(&rel, "checksums.sha256").unwrap().browser_download_url.clone();
            let text = http.get(sums).header(reqwest::header::USER_AGENT, "Pako").send().await.unwrap().text().await.unwrap();
            assert!(hash_for(&text, &a.name).is_some(), "{} has no checksum", a.name);
            println!("ffmpeg {v}: {} (release {})", a.name, rel.tag_name);
        });
    }

    /// Against GitHub today (network): yt-dlp's folder build downloads, matches SHA2-256SUMS,
    /// unpacks and runs (in a temporary folder). `cargo test --lib -- --ignored yt_dlp_on_github`.
    #[test]
    #[ignore]
    fn yt_dlp_on_github() {
        tauri::async_runtime::block_on(async {
            let http = reqwest::Client::new();
            let rel = latest_release(&http, Kind::YtDlp).await.unwrap();
            let d = scratch("github");
            let zip = d.join(YTDLP_ZIP);
            let quiet = |_: &str, _: u64, _: Option<u64>| {};
            let got = fetch(&http, &asset(&rel, YTDLP_ZIP).unwrap().browser_download_url, &zip, &quiet).await.unwrap();
            let want = published_sha256(&http, &asset(&rel, "SHA2-256SUMS").unwrap().browser_download_url, YTDLP_ZIP).await.unwrap();
            let out = d.join("yt-dlp");
            std::fs::create_dir_all(&out).unwrap();
            unpack_yt_dlp(&zip, &got, &want, &out).unwrap();
            assert!(yt_dlp_runs(&out.join("yt-dlp.exe")));
            println!("yt-dlp {} ({YTDLP_ZIP})", rel.tag_name);
            let _ = std::fs::remove_dir_all(&d);
        });
    }

    #[test]
    fn picks_the_newest_stable_ffmpeg() {
        let a = |n: &str| Asset { name: n.into(), browser_download_url: String::new() };
        let assets = vec![
            a("ffmpeg-master-latest-win64-gpl.zip"),
            a("ffmpeg-n9.0-latest-win64-gpl-9.0.zip"),
            a("ffmpeg-n9.0-latest-win64-gpl-shared-9.0.zip"),
            a("ffmpeg-n8.1-latest-win64-gpl-8.1.zip"),
        ];
        let (pick, ver) = pick_ffmpeg(&assets).unwrap();
        assert_eq!(pick.name, "ffmpeg-n9.0-latest-win64-gpl-9.0.zip");
        // A day's automatic build (BtbN's "latest" release since late 2026).
        let day = [
            a("ffmpeg-N-128959-g4dbee3b241-win64-gpl.zip"),
            a("ffmpeg-n8.1.3-35-gf1acb0a266-win64-gpl-8.1.zip"),
            a("ffmpeg-n9.0.2-57-gd838f9f9ae-win64-gpl-shared-9.0.zip"),
            a("ffmpeg-n9.0.2-57-gd838f9f9ae-win64-lgpl-9.0.zip"),
            a("ffmpeg-n9.0.2-57-gd838f9f9ae-win64-gpl-9.0.zip"),
            a("ffmpeg-n9.0.2-57-gd838f9f9ae-winarm64-gpl-9.0.zip"),
        ];
        let (pick, v) = pick_ffmpeg(&day).unwrap();
        assert_eq!((pick.name.as_str(), v.as_str()), ("ffmpeg-n9.0.2-57-gd838f9f9ae-win64-gpl-9.0.zip", "9.0"));
        assert_eq!(ver, "9.0");
    }

    #[test]
    fn compares_installed_ffmpeg_to_the_release() {
        // Noted at install: the release's own version, equal to what the next check reads.
        assert!(same_version(Kind::Ffmpeg, "9.0", "9.0"));
        // No note (installed by an older Pako): ffmpeg's own "9.0.2" is a build of "9.0".
        let own = parse_version(Kind::Ffmpeg, "ffmpeg version n9.0.2-57-gabc Copyright").unwrap();
        assert_eq!(own, "9.0.2");
        assert!(same_version(Kind::Ffmpeg, &own, "9.0"));
        // A new release line is an update.
        assert!(!same_version(Kind::Ffmpeg, "9.0.2", "9.1"));
        assert!(!same_version(Kind::Ffmpeg, "9.0", "9.1"));
        assert!(!same_version(Kind::Ffmpeg, "19.0", "9.0"));
        // Other tools compare exactly.
        assert!(!same_version(Kind::YtDlp, "2026.10.26.1", "2026.10.26"));
        let status = |installed: &str, latest: &str| {
            serde_json::to_value(Status { kind: Kind::Ffmpeg, installed: Some(installed.into()), latest: Some(latest.into()) }).unwrap()
        };
        assert_eq!(status("9.0.2", "9.0")["update"], false);
        assert_eq!(status("9.0", "9.1")["update"], true);
    }

    /// A fresh empty folder for one test.
    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pako-components-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A zip with these (name, content) entries; a name ending in "/" is a folder.
    fn make_zip(path: &Path, entries: &[(&str, &str)]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, body) in entries {
            if name.ends_with('/') {
                w.add_directory(*name, opts).unwrap();
            } else {
                w.start_file(*name, opts).unwrap();
                w.write_all(body.as_bytes()).unwrap();
            }
        }
        w.finish().unwrap();
    }

    fn files_in(d: &Path) -> usize {
        std::fs::read_dir(d).map(|rd| rd.count()).unwrap_or(0)
    }

    #[test]
    fn yt_dlp_folder_build_unpacks_whole() {
        let d = scratch("unpack");
        let zip = d.join("yt-dlp_win.zip");
        make_zip(&zip, &[("yt-dlp.exe", "exe"), ("_internal/", ""), ("_internal/python314.dll", "dll"), ("_internal/yt_dlp\\x.pyc", "pyc")]);
        let out = d.join("out");
        std::fs::create_dir_all(&out).unwrap();
        unpack_yt_dlp(&zip, "ABC", "abc", &out).unwrap();
        assert_eq!(std::fs::read_to_string(out.join("yt-dlp.exe")).unwrap(), "exe");
        assert!(out.join("_internal").join("python314.dll").is_file());
        assert!(out.join("_internal").join("yt_dlp").join("x.pyc").is_file());
        assert!(!single_exe_in(&out));
        // A zip without `_internal` isn't the folder build.
        let bare = d.join("bare.zip");
        make_zip(&bare, &[("yt-dlp.exe", "exe")]);
        assert!(unpack_yt_dlp(&bare, "a", "a", &d.join("out2")).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_wrong_checksum_unpacks_nothing() {
        let d = scratch("hash");
        let zip = d.join("yt-dlp_win.zip");
        make_zip(&zip, &[("yt-dlp.exe", "exe"), ("_internal/a.dll", "dll")]);
        let out = d.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let err = unpack_yt_dlp(&zip, &"a".repeat(64), &"b".repeat(64), &out).unwrap_err();
        assert!(err.to_string().contains("checksum"), "{err}");
        assert_eq!(files_in(&out), 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_zip_reaching_outside_its_folder_is_refused() {
        let d = scratch("slip");
        let out = d.join("out");
        for (i, bad) in ["../evil.txt", "_internal/../../evil.txt", "..\\evil.txt", "/abs.txt", "\\abs.txt", "C:/evil.txt", "C:evil.txt", "a/./b.txt"]
            .into_iter()
            .enumerate()
        {
            let zip = d.join(format!("bad{i}.zip"));
            make_zip(&zip, &[("yt-dlp.exe", "exe"), ("_internal/a.dll", "dll"), (bad, "x")]);
            let _ = std::fs::remove_dir_all(&out);
            std::fs::create_dir_all(&out).unwrap();
            assert!(unzip_tree(&zip, &out).is_err(), "{bad} was unpacked");
            assert!(unpack_yt_dlp(&zip, "h", "h", &out).is_err(), "{bad} was unpacked");
            assert!(!d.join("evil.txt").exists() && !std::env::temp_dir().join("evil.txt").exists());
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_failed_swap_keeps_the_old_copy() {
        let d = scratch("swap");
        let target = d.join("yt-dlp");
        let prev = d.join(".prev-yt-dlp");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("yt-dlp.exe"), "old").unwrap();
        // The new folder can't be moved in (here: it's gone): the old copy goes back in place.
        let err = swap_in(&d.join(".new-yt-dlp"), &target, &prev, "yt-dlp").unwrap_err();
        assert!(err.to_string().contains("new files"), "{err}");
        assert_eq!(std::fs::read_to_string(target.join("yt-dlp.exe")).unwrap(), "old");
        assert!(!prev.exists());
        // Pako closed between the two moves: the old copy is put back when it's next looked for.
        std::fs::rename(&target, &prev).unwrap();
        restore_in(&d, Kind::YtDlp);
        assert_eq!(std::fs::read_to_string(target.join("yt-dlp.exe")).unwrap(), "old");
        assert!(!prev.exists());
        // ... but never over a copy that is in place.
        std::fs::create_dir_all(&prev).unwrap();
        std::fs::write(prev.join("yt-dlp.exe"), "older").unwrap();
        restore_in(&d, Kind::YtDlp);
        assert_eq!(std::fs::read_to_string(target.join("yt-dlp.exe")).unwrap(), "old");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_single_exe_moves_to_the_folder_build() {
        let d = scratch("migrate");
        // What an older Pako installed: the single yt-dlp.exe and Pako's note.
        let target = d.join("yt-dlp");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("yt-dlp.exe"), "single").unwrap();
        std::fs::write(target.join(RECORD), r#"{"version":"2026.10.26","asset":"yt-dlp.exe"}"#).unwrap();
        assert!(single_exe_in(&target));
        // The update: the folder build unpacked next to it, then swapped in whole.
        let zip = d.join("yt-dlp_win.zip");
        make_zip(&zip, &[("yt-dlp.exe", "folder"), ("_internal/python314.dll", "dll")]);
        let new = d.join(".new-yt-dlp");
        std::fs::create_dir_all(&new).unwrap();
        unpack_yt_dlp(&zip, "h", "h", &new).unwrap();
        std::fs::write(new.join(RECORD), r#"{"version":"2026.10.26","asset":"yt-dlp_win.zip"}"#).unwrap();
        swap_in(&new, &target, &d.join(".prev-yt-dlp"), "yt-dlp").unwrap();
        assert!(!single_exe_in(&target));
        assert_eq!(std::fs::read_to_string(target.join("yt-dlp.exe")).unwrap(), "folder");
        assert!(target.join("_internal").join("python314.dll").is_file());
        assert!(std::fs::read_to_string(target.join(RECORD)).unwrap().contains("yt-dlp_win.zip"));
        assert!(!new.exists() && !d.join(".prev-yt-dlp").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A program running from the tool's folder: Windows would let the folder move (as with
    /// yt-dlp's single exe, which then stops), so the update and Remove must refuse by themselves.
    #[cfg(windows)]
    #[test]
    fn a_running_tool_is_never_moved() {
        use std::os::windows::process::CommandExt;
        let d = scratch("running");
        let target = d.join("yt-dlp");
        std::fs::create_dir_all(&target).unwrap();
        let exe = target.join("yt-dlp.exe");
        std::fs::copy(std::env::var("SystemRoot").map(PathBuf::from).unwrap_or("C:\\Windows".into()).join("System32").join("PING.EXE"), &exe).unwrap();
        let mut child = Command::new(&exe)
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .unwrap();
        let start = Instant::now();
        while !running_from(&target) && start.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(running_from(&target));
        std::fs::create_dir_all(d.join("yt-dl")).unwrap();
        assert!(!running_from(&d.join("yt-dl")), "only the folder itself and what is in it");
        // An update: refused, the running copy untouched, the new one dropped.
        let new = d.join(".new-yt-dlp");
        std::fs::create_dir_all(new.join("_internal")).unwrap();
        std::fs::write(new.join("yt-dlp.exe"), "new").unwrap();
        let err = swap_in(&new, &target, &d.join(".prev-yt-dlp"), "yt-dlp").unwrap_err();
        assert!(err.to_string().contains("in use"), "{err}");
        assert!(exe.is_file() && !new.exists() && !d.join(".prev-yt-dlp").exists());
        // Remove: refused too.
        assert!(remove_in(&d, &[Kind::YtDlp]).is_err());
        assert!(exe.is_file());
        let _ = child.kill();
        let _ = child.wait();
        assert!(!running_from(&target));
        assert!(remove_in(&d, &[Kind::YtDlp]).is_ok());
        assert!(!target.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn remove_takes_the_leftovers_too() {
        let d = scratch("leftovers");
        std::fs::create_dir_all(d.join("yt-dlp")).unwrap();
        std::fs::write(d.join("yt-dlp").join("yt-dlp.exe"), "exe").unwrap();
        for name in leftovers(Kind::YtDlp) {
            std::fs::create_dir_all(d.join(&name)).unwrap();
            std::fs::write(d.join(&name).join("yt-dlp.exe"), "left").unwrap();
        }
        std::fs::write(rebuild_note(&d, Kind::YtDlp), "2026.11.01").unwrap();
        // Another tool's folders stay.
        std::fs::create_dir_all(d.join(".new-deno")).unwrap();
        remove_in(&d, &[Kind::YtDlp]).unwrap();
        for name in leftovers(Kind::YtDlp) {
            assert!(!d.join(&name).exists(), "{name} was left");
        }
        assert!(!rebuild_note(&d, Kind::YtDlp).exists());
        assert!(d.join(".new-deno").exists());
        // Nothing brings the removed tool back.
        restore_in(&d, Kind::YtDlp);
        assert!(!d.join("yt-dlp").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_failed_move_to_the_folder_build_waits_for_the_next_release() {
        let d = scratch("rebuild");
        let target = d.join("yt-dlp");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("yt-dlp.exe"), "single").unwrap();
        assert!(rebuild_due_in(&d, Kind::YtDlp, "2026.11.01"));
        // The folder build didn't start on this PC: not downloaded again for this release...
        std::fs::write(rebuild_note(&d, Kind::YtDlp), "2026.11.01").unwrap();
        assert!(!rebuild_due_in(&d, Kind::YtDlp, "2026.11.01"));
        // ... tried again with the next one.
        assert!(rebuild_due_in(&d, Kind::YtDlp, "2026.11.09"));
        // The folder build in place: nothing to move.
        std::fs::create_dir_all(target.join("_internal")).unwrap();
        assert!(!rebuild_due_in(&d, Kind::YtDlp, "2026.11.09"));
        assert!(!rebuild_due_in(&d, Kind::Deno, "2026.11.09"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_install_note_reads_back() {
        let r = Record { version: "9.0".into(), asset: "ffmpeg-n9.0-latest-win64-gpl-9.0.zip".into() };
        let back: Record = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }
}
