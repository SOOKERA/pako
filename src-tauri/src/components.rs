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
    Some(kind.exe()).filter(|p| p.is_file())
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
    fn exe(self) -> PathBuf {
        let name = match self {
            Kind::SevenZip => "7z.exe",
            Kind::Ffmpeg => "ffmpeg.exe",
            Kind::YtDlp => "yt-dlp.exe",
            Kind::Deno => "deno.exe",
        };
        dir().join(self.folder()).join(name)
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
    if let Some(k) = kinds.iter().find(|k| is_installing(**k)) {
        bail!("{} is being installed right now.", k.label());
    }
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for &kind in kinds {
        let target = dir().join(kind.folder());
        if !target.exists() {
            continue;
        }
        // Moving it aside fails while the tool runs: nothing is half-deleted then.
        let old = dir().join(format!(".old-{}", kind.folder()));
        let _ = std::fs::remove_dir_all(&old);
        if std::fs::rename(&target, &old).is_err() {
            for (target, old) in moved.iter().rev() {
                let _ = std::fs::rename(old, target);
            }
            bail!("{} is in use (a preview, a video or an extraction is running). Close it, then try again.", kind.label());
        }
        moved.push((target, old));
    }
    for (_, old) in moved {
        let _ = std::fs::remove_dir_all(&old);
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
    if std::fs::rename(&target, &probe).is_err() {
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
        // A newer version than the one installed (compared the way updates compare).
        let update = match (&self.installed, &self.latest) {
            (Some(have), Some(latest)) => have != "installed" && !same_version(self.kind, have, latest),
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

/// Runs a tool once to read its version (hidden, no console window).
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
        Kind::YtDlp | Kind::Deno => run_version(&exe, &["--version"]),
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

    let result: Result<()> = async {
        let asset_name: String;
        match kind {
            Kind::YtDlp => {
                let a = asset(&rel, "yt-dlp.exe")?;
                asset_name = a.name.clone();
                let got = fetch(http, &a.browser_download_url, &tmp.join("yt-dlp.exe"), progress).await?;
                progress("Checking the file", 0, None);
                let want = published_sha256(http, &asset(&rel, "SHA2-256SUMS")?.browser_download_url, "yt-dlp.exe").await?;
                if got != want {
                    bail!("The downloaded file doesn't match yt-dlp's published checksum");
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
        return Err(e);
    }
    // Swap safely: move the old copy aside first. If that fails, the tool is in use and
    // nothing has been touched; the new files only replace the old ones once moved.
    let target = base.join(kind.folder());
    let old = base.join(format!(".old-{}", kind.folder()));
    let _ = std::fs::remove_dir_all(&old);
    if target.exists() && std::fs::rename(&target, &old).is_err() {
        let _ = std::fs::remove_dir_all(&tmp);
        bail!("{} is in use (a preview, a video or an extraction is running). Close it, then try again.", kind.label());
    }
    if let Err(e) = std::fs::rename(&tmp, &target) {
        // Put the old copy back so the tool keeps working.
        let _ = std::fs::rename(&old, &target);
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(anyhow!("Couldn't put the new files in place: {e}"));
    }
    let _ = std::fs::remove_dir_all(&old);
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

    #[test]
    fn the_install_note_reads_back() {
        let r = Record { version: "9.0".into(), asset: "ffmpeg-n9.0-latest-win64-gpl-9.0.zip".into() };
        let back: Record = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }
}
