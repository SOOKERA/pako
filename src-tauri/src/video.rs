// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Videos from YouTube and other sites, through yt-dlp (with Deno for YouTube and ffmpeg to
//! join video and sound). The tools live in data/components and are installed from Settings
//! › Features & updates or the first-start setup.
//!
//! - `probe` asks yt-dlp what a link is (title, length, qualities and their sizes, or the
//!   videos of a playlist) without downloading anything.
//! - `download_args` / `parse_line` run and follow the actual download for the engine.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::components::{self, Kind};
use crate::model::VideoOption;

/// Shown on a Links row when the tools aren't installed.
pub const NEEDS_TOOLS: &str = "Video downloads need the video tools.";
/// Shown while yt-dlp looks at a link.
pub const CHECKING: &str = "Checking video…";
/// yt-dlp's errors that mean "the site changed": a newer yt-dlp usually fixes them.
pub const SITE_CHANGED: &str = "The site changed something. Pako updates its video tool automatically: try again in a little while.";
/// The site locks its videos (DRM): nothing can download them, and Pako never tries to get around it.
pub const DRM: &str = "This site protects its videos (DRM), so they can't be downloaded.";
/// Connections Pako asks yt-dlp to use for videos that come in small pieces (HLS / DASH).
pub const FRAGMENTS_AT_ONCE: u32 = 16;

pub struct Tools {
    pub yt_dlp: PathBuf,
    pub deno: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
}

/// The installed video tools (None when yt-dlp itself is missing).
pub fn tools() -> Option<Tools> {
    let yt_dlp = components::path(Kind::YtDlp)?;
    Some(Tools { yt_dlp, deno: components::path(Kind::Deno), ffmpeg: components::ffmpeg() })
}

/// All three video tools are installed (yt-dlp, Deno for YouTube, ffmpeg to join).
pub fn ready() -> bool {
    components::path(Kind::YtDlp).is_some() && components::path(Kind::Deno).is_some() && components::ffmpeg().is_some()
}

fn host_of(url: &url::Url) -> String {
    url.host_str().unwrap_or_default().trim_start_matches("www.").trim_start_matches("m.").to_ascii_lowercase()
}

/// Sites that are clearly video sites: their links always go the video route. Other links are
/// asked to yt-dlp only when they turn out to be a web page (see `found.rs`).
pub fn is_video_site(link: &str) -> bool {
    let Ok(url) = url::Url::parse(link) else { return false };
    let host = host_of(&url);
    let path = url.path();
    let on = |d: &str| host == d || host.ends_with(&format!(".{d}"));
    if on("youtu.be") {
        return path.len() > 1;
    }
    if on("youtube.com") || on("youtube-nocookie.com") || on("music.youtube.com") {
        return path.starts_with("/watch")
            || path.starts_with("/shorts/")
            || path.starts_with("/playlist")
            || path.starts_with("/live/")
            || path.starts_with("/embed/");
    }
    if on("vimeo.com") {
        return path.trim_matches('/').split('/').any(|p| p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty());
    }
    if on("dailymotion.com") || on("dai.ly") {
        return path.starts_with("/video/") || on("dai.ly");
    }
    if on("twitch.tv") {
        return path.contains("/videos/") || path.contains("/clip/") || on("clips.twitch.tv");
    }
    if on("tiktok.com") {
        return path.contains("/video/");
    }
    if on("x.com") || on("twitter.com") {
        return path.contains("/status/");
    }
    if on("instagram.com") {
        return path.starts_with("/reel/") || path.starts_with("/p/") || path.starts_with("/tv/");
    }
    if on("facebook.com") || on("fb.watch") {
        return path.contains("/videos/") || path.starts_with("/watch") || path.starts_with("/reel/") || on("fb.watch");
    }
    if on("reddit.com") {
        return path.contains("/comments/");
    }
    if on("bilibili.com") {
        return path.starts_with("/video/");
    }
    if on("streamable.com") || on("rumble.com") || on("odysee.com") || on("bitchute.com") || on("soundcloud.com") {
        return path.len() > 1;
    }
    false
}

/// What yt-dlp says a link is.
#[derive(Debug, Default)]
pub struct Probe {
    pub title: String,
    pub duration: Option<f64>,
    /// Qualities (height in lines) with their size, best first.
    pub options: Vec<VideoOption>,
    /// Best sound-only size (for "Audio only", and added to video-only qualities).
    pub audio_size: Option<u64>,
    /// yt-dlp's extractor ("Youtube", "Generic"...).
    pub extractor: String,
    /// A playlist: its videos (link, title, length).
    pub entries: Vec<(String, String, Option<f64>)>,
    pub is_playlist: bool,
}

#[derive(Deserialize)]
struct RawInfo {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    formats: Option<Vec<RawFormat>>,
    #[serde(default)]
    entries: Option<Vec<RawEntry>>,
    #[serde(default, rename = "_type")]
    kind: Option<String>,
    #[serde(default)]
    extractor_key: Option<String>,
    #[serde(default)]
    filesize: Option<f64>,
    #[serde(default)]
    filesize_approx: Option<f64>,
    #[serde(default)]
    height: Option<u32>,
}

#[derive(Deserialize)]
struct RawFormat {
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    vcodec: Option<String>,
    #[serde(default)]
    acodec: Option<String>,
    #[serde(default)]
    filesize: Option<f64>,
    #[serde(default)]
    filesize_approx: Option<f64>,
    #[serde(default)]
    tbr: Option<f64>,
    #[serde(default)]
    ext: Option<String>,
}

#[derive(Deserialize)]
struct RawEntry {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    webpage_url: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    ie_key: Option<String>,
}

fn has(codec: &Option<String>) -> bool {
    codec.as_deref().is_some_and(|c| c != "none")
}

/// Whether a stream to join may have picture (or sound): only yt-dlp's "none" says it hasn't.
/// A codec the site doesn't name counts as "maybe": joining maps it optionally, so a stream
/// that turns out to have none of it doesn't stop the join, and none is left out.
fn may_have(codec: &Option<String>) -> bool {
    codec.as_deref() != Some("none")
}

/// Reads `yt-dlp -J` output.
pub fn parse_info(json: &str) -> anyhow::Result<Probe> {
    let raw: RawInfo = serde_json::from_str(json)?;
    let mut p = Probe {
        title: raw.title.clone().unwrap_or_else(|| "Video".into()),
        duration: raw.duration,
        extractor: raw.extractor_key.clone().unwrap_or_default(),
        ..Default::default()
    };
    if raw.kind.as_deref() == Some("playlist") || raw.entries.is_some() {
        p.is_playlist = true;
        for e in raw.entries.unwrap_or_default() {
            let link = e.webpage_url.or(e.url).or_else(|| {
                // Flat YouTube entries sometimes only carry the id.
                let id = e.id?;
                (e.ie_key.as_deref() == Some("Youtube")).then(|| format!("https://www.youtube.com/watch?v={id}"))
            });
            if let Some(link) = link.filter(|l| l.starts_with("http")) {
                p.entries.push((link, e.title.unwrap_or_else(|| "Video".into()), e.duration));
            }
        }
        return Ok(p);
    }
    let duration = raw.duration.unwrap_or(0.0);
    let size_of = |f: &RawFormat| -> Option<u64> {
        f.filesize
            .or(f.filesize_approx)
            .or_else(|| f.tbr.filter(|_| duration > 0.0).map(|t| t * 125.0 * duration))
            .map(|s| s as u64)
    };
    let formats = raw.formats.unwrap_or_default();
    // Best sound-only stream (M4A preferred, like the download asks for).
    let audio = formats
        .iter()
        .filter(|f| has(&f.acodec) && !has(&f.vcodec))
        .max_by_key(|f| (f.ext.as_deref() == Some("m4a"), (f.tbr.unwrap_or(0.0) * 10.0) as u64));
    p.audio_size = audio.and_then(size_of);
    let mut by_height: std::collections::BTreeMap<u32, (bool, u64, Option<u64>)> = Default::default();
    for f in formats.iter().filter(|f| has(&f.vcodec)) {
        let Some(h) = f.height.filter(|h| *h > 0) else { continue };
        let combined = has(&f.acodec);
        let mut size = size_of(f);
        if !combined {
            size = size.map(|s| s + p.audio_size.unwrap_or(0));
        }
        // The stream the download prefers at this height: MP4 first, then the biggest.
        let rank = (f.ext.as_deref() == Some("mp4"), size.unwrap_or(0));
        let better = by_height.get(&h).is_none_or(|(mp4, s, _)| (rank.0, rank.1) > (*mp4, *s));
        if better {
            by_height.insert(h, (rank.0, rank.1, size));
        }
    }
    p.options = by_height.into_iter().rev().map(|(height, (_, _, size))| VideoOption { height, size }).collect();
    // A single file without a formats list (some sites).
    if p.options.is_empty() {
        if let Some(h) = raw.height {
            p.options.push(VideoOption { height: h, size: raw.filesize.or(raw.filesize_approx).map(|s| s as u64) });
        }
    }
    Ok(p)
}

/// The quality the download will take: the chosen one, else the closest lower one,
/// else the smallest there is. 0 = audio only.
pub fn pick(options: &[VideoOption], want: u32) -> Option<&VideoOption> {
    if want == 0 {
        return None;
    }
    options.iter().find(|o| o.height <= want).or_else(|| options.last())
}

/// Size estimate for a quality (0 = audio only).
pub fn estimate(options: &[VideoOption], audio_size: Option<u64>, want: u32) -> Option<u64> {
    if want == 0 {
        return audio_size;
    }
    pick(options, want).and_then(|o| o.size)
}

/// "Title.mp4", or "Title.m4a" / "Title.mp3" for audio only.
pub fn file_name(title: &str, audio: Option<&str>) -> String {
    let ext = match audio {
        Some("mp3") => "mp3",
        Some(_) => "m4a",
        None => "mp4",
    };
    let stem = crate::names::sanitize(title);
    let stem = if stem.trim().is_empty() { "Video".to_string() } else { stem };
    // Long titles: keep the name reasonable (Windows paths have limits).
    let stem: String = stem.chars().take(150).collect();
    format!("{}.{ext}", stem.trim())
}

// ---------- the browser's login, for yt-dlp ----------

/// The browser's cookies as yt-dlp's cookie file ("Netscape" format, one cookie per line).
/// Cookies with tabs or line breaks (which would break the file) are left out.
pub fn netscape_cookies(cookies: &[crate::extension::SiteCookie]) -> String {
    let mut out = String::from("# Netscape HTTP Cookie File\n");
    for c in cookies {
        let fields = [&c.domain, &c.path, &c.name, &c.value];
        if c.domain.is_empty() || c.name.is_empty() || fields.iter().any(|f| f.chars().any(|ch| ch.is_control())) {
            continue;
        }
        let sub = if c.domain.starts_with('.') { "TRUE" } else { "FALSE" };
        let secure = if c.secure { "TRUE" } else { "FALSE" };
        let path = if c.path.is_empty() { "/" } else { c.path.as_str() };
        out.push_str(&format!("{}\t{sub}\t{path}\t{secure}\t{}\t{}\t{}\n", c.domain, c.expires, c.name, c.value));
    }
    out
}

/// A cookie file for one yt-dlp run, readable only by this Windows user; deleted as soon as
/// it is dropped (the run ended, was stopped or failed), and by `remove_cookie_files` when
/// Pako closes (a run that is still going then never gets to drop it).
pub struct CookieFile {
    path: PathBuf,
}

impl CookieFile {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for CookieFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        LIVE_JARS.lock().unwrap_or_else(|e| e.into_inner()).retain(|p| p != &self.path);
    }
}

/// The cookie files that exist right now (removed when Pako closes).
static LIVE_JARS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// Cookie files start with this; anything else in their folder is left alone.
const JAR_PREFIX: &str = "yt-cookies-";

/// Where cookie files go: Windows' temporary folder for this user (on the system disk, so the
/// owner-only lock always holds; a data folder on a USB stick may not support it).
fn jar_dir() -> PathBuf {
    // A test or development run never touches the files of the Pako in daily use.
    let name = if cfg!(debug_assertions) { "Pako-dev" } else { "Pako" };
    std::env::temp_dir().join(name)
}

/// Writes the cookie file (None when there are no cookies).
pub fn cookie_file(cookies: &[crate::extension::SiteCookie]) -> std::io::Result<Option<CookieFile>> {
    if cookies.is_empty() {
        return Ok(None);
    }
    let dir = jar_dir();
    make_private_dir(&dir)?;
    cookie_file_in(&dir, cookies)
}

fn cookie_file_in(dir: &Path, cookies: &[crate::extension::SiteCookie]) -> std::io::Result<Option<CookieFile>> {
    if cookies.is_empty() {
        return Ok(None);
    }
    let path = dir.join(format!("{JAR_PREFIX}{}.txt", uuid::Uuid::new_v4().simple()));
    write_private(&path, netscape_cookies(cookies).as_bytes())?;
    LIVE_JARS.lock().unwrap_or_else(|e| e.into_inner()).push(path.clone());
    Ok(Some(CookieFile { path }))
}

/// The cookies of a download that came from the browser (none for pasted links).
pub fn cookie_file_for(download_id: &str) -> Option<CookieFile> {
    let cookies = crate::extension::headers_for(download_id)?.site_cookies;
    cookie_file(&cookies).ok().flatten()
}

/// Pako is closing (after `kill_all`): deletes the cookie files of runs that were still going.
/// yt-dlp may take a moment to let go of one, so each is tried a few times (half a second at most).
pub fn remove_cookie_files() {
    remove_live_jars(|_| true);
}

fn remove_live_jars(which: impl Fn(&Path) -> bool) {
    let paths: Vec<PathBuf> = {
        let mut live = LIVE_JARS.lock().unwrap_or_else(|e| e.into_inner());
        let (gone, keep) = std::mem::take(&mut *live).into_iter().partition(|p| which(p));
        *live = keep;
        gone
    };
    for path in paths {
        for _ in 0..10 {
            if std::fs::remove_file(&path).is_ok() || !path.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// At start: deletes cookie files left behind by a Pako that didn't close normally (in the
/// temporary folder, and in the data folder where older versions put them).
pub fn sweep_cookie_files() {
    sweep_jars_in(&jar_dir());
    sweep_jars_in(crate::paths::data_dir());
}

fn sweep_jars_in(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let live = LIVE_JARS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with(JAR_PREFIX) && name.ends_with(".txt") && !live.contains(&e.path()) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// The folder for cookie files, made so only this Windows user can open it (when it is new).
#[cfg(windows)]
fn make_private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::CreateDirectoryW;
    if dir.is_dir() {
        return Ok(());
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    with_owner_only(|sa| {
        // SAFETY: valid NUL-terminated path and attributes.
        if unsafe { CreateDirectoryW(wide.as_ptr(), sa) } == 0 {
            let err = std::io::Error::last_os_error();
            // Made at the same moment by another run: fine.
            if !dir.is_dir() {
                return Err(err);
            }
        }
        Ok(())
    })
}

#[cfg(not(windows))]
fn make_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// Runs `f` with security attributes that give only the owner (this Windows user) access,
/// nothing inherited from the folder above.
#[cfg(windows)]
fn with_owner_only<T>(f: impl FnOnce(*const windows_sys::Win32::Security::SECURITY_ATTRIBUTES) -> std::io::Result<T>) -> std::io::Result<T> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1};
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    // A protected list with one entry: the owner, full access (files and folders made inside
    // a folder made this way get the same).
    let sddl: Vec<u16> = "D:P(A;OICI;FA;;;OW)\0".encode_utf16().collect();
    let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: valid NUL-terminated string; `sd` is freed with LocalFree below.
    if unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), SDDL_REVISION_1, &mut sd, std::ptr::null_mut()) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let sa = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: sd, bInheritHandle: 0 };
    let r = f(&sa);
    // SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW.
    unsafe { LocalFree(sd as _) };
    r
}

/// A new file only its owner (this Windows user) can open, written in one go.
#[cfg(windows)]
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_TEMPORARY};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut f = with_owner_only(|sa| {
        // SAFETY: valid path and attributes; the handle is owned by the File below.
        let h = unsafe { CreateFileW(wide.as_ptr(), GENERIC_WRITE, 0, sa, CREATE_NEW, FILE_ATTRIBUTE_TEMPORARY, std::ptr::null_mut()) };
        if h == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: a fresh, valid file handle.
        Ok(unsafe { std::fs::File::from_raw_handle(h as _) })
    })?;
    let written = f.write_all(data).and_then(|_| f.flush());
    drop(f);
    if written.is_err() {
        let _ = std::fs::remove_file(path);
    }
    written
}

#[cfg(not(windows))]
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, data)
}

fn base_command(t: &Tools) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(&t.yt_dlp);
    cmd.env("PYTHONIOENCODING", "utf-8").env("PYTHONUTF8", "1");
    cmd.arg("--no-config").arg("--encoding").arg("utf-8");
    if let Some(deno) = &t.deno {
        cmd.arg("--js-runtimes").arg(format!("deno:{}", deno.display()));
    }
    if let Some(ff) = &t.ffmpeg {
        cmd.arg("--ffmpeg-location").arg(ff);
    }
    cmd.kill_on_drop(true);
    cmd
}

/// Starts yt-dlp inside Pako's job (see `job`): whatever it starts (its Python part, Deno,
/// ffmpeg) ends with Pako, even after a crash or "Restart now".
pub fn spawn(cmd: &mut tokio::process::Command) -> std::io::Result<tokio::process::Child> {
    job::spawn(cmd)
}

/// Ends every yt-dlp Pako started, and what they started, right away (Pako is closing).
pub fn kill_all() {
    job::kill_all();
}

#[cfg(windows)]
mod job {
    use std::sync::OnceLock;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_SUSPENDED: u32 = 0x0000_0004;

    /// One job for all of Pako's yt-dlp runs, made once and never closed: Windows closes it when
    /// Pako's process ends (however it ends), and "kill on close" then ends everything in it.
    /// 0 = Windows wouldn't make one (yt-dlp still runs, it just isn't tied to Pako).
    static JOB: OnceLock<usize> = OnceLock::new();

    fn job() -> HANDLE {
        *JOB.get_or_init(|| {
            // SAFETY: plain calls; the limit struct is zeroed and only its flags are set.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if h.is_null() {
                    return 0;
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ok == 0 {
                    CloseHandle(h);
                    return 0;
                }
                h as usize
            }
        }) as HANDLE
    }

    /// Starts the process paused, puts it in the job, then lets it run: it can't start
    /// anything of its own before it is in the job.
    pub fn spawn(cmd: &mut tokio::process::Command) -> std::io::Result<tokio::process::Child> {
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
        let mut child = cmd.spawn()?;
        let job = job();
        if !job.is_null() {
            if let Some(h) = child.raw_handle() {
                // SAFETY: both handles are valid; a failure only means it isn't tied to Pako.
                unsafe { AssignProcessToJobObject(job, h as HANDLE) };
            }
        }
        let resumed = child.id().is_some_and(resume);
        if !resumed {
            let _ = child.start_kill();
            return Err(std::io::Error::other("Windows didn't let it start"));
        }
        Ok(child)
    }

    /// Lets the (only) thread of a process started paused run.
    fn resume(pid: u32) -> bool {
        let mut done = false;
        // SAFETY: the snapshot and thread handles are checked and always closed.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snap == INVALID_HANDLE_VALUE || snap.is_null() {
                return false;
            }
            let mut te: THREADENTRY32 = std::mem::zeroed();
            te.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            let mut more = Thread32First(snap, &mut te) != 0;
            while more {
                if te.th32OwnerProcessID == pid {
                    let t = OpenThread(THREAD_SUSPEND_RESUME, 0, te.th32ThreadID);
                    if !t.is_null() {
                        if ResumeThread(t) != u32::MAX {
                            done = true;
                        }
                        CloseHandle(t);
                    }
                }
                more = Thread32Next(snap, &mut te) != 0;
            }
            CloseHandle(snap);
        }
        done
    }

    pub fn kill_all() {
        if let Some(&h) = JOB.get() {
            if h != 0 {
                // SAFETY: the job handle stays open for the whole run.
                unsafe { TerminateJobObject(h as HANDLE, 1) };
            }
        }
    }
}

#[cfg(not(windows))]
mod job {
    pub fn spawn(cmd: &mut tokio::process::Command) -> std::io::Result<tokio::process::Child> {
        cmd.spawn()
    }
    pub fn kill_all() {}
}

/// Asks yt-dlp what a link is (nothing is downloaded). Errors are plain sentences;
/// `true` in the error means a newer yt-dlp may fix it.
pub async fn probe(link: &str, limit: Duration) -> Result<Probe, (String, bool)> {
    probe_with(link, limit, &[]).await
}

/// `probe` with the browser's cookies for the site (videos that need you to be logged in).
pub async fn probe_with(link: &str, limit: Duration, cookies: &[crate::extension::SiteCookie]) -> Result<Probe, (String, bool)> {
    let Some(t) = tools() else { return Err((NEEDS_TOOLS.into(), false)) };
    let jar = cookie_file(cookies).map_err(|e| (format!("Couldn't prepare your browser login for the video tool: {e}"), false))?;
    let mut cmd = base_command(&t);
    if let Some(j) = &jar {
        cmd.arg("--cookies").arg(j.path());
    }
    cmd.args(["-J", "--flat-playlist", "--no-playlist", "--no-warnings", "--skip-download"]).arg(link);
    let json = run_json(cmd, limit).await;
    drop(jar);
    let json = json?;
    parse_info(&json).map_err(|_| ("The video tool gave an answer Pako couldn't read.".into(), true))
}

/// Runs yt-dlp for a JSON answer (nothing is downloaded), within `limit`.
async fn run_json(mut cmd: tokio::process::Command, limit: Duration) -> Result<String, (String, bool)> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = spawn(&mut cmd).map_err(|e| (format!("Couldn't start the video tool: {e}"), false))?;
    let pid = child.id();
    let out = match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err((format!("The video tool stopped: {e}"), false)),
        Err(_) => {
            // Not just yt-dlp's launcher: what it started too (its Python part, Deno).
            if let Some(pid) = pid {
                let _ = tokio::task::spawn_blocking(move || kill_tree(pid)).await;
            }
            return Err(("The site took too long to answer.".into(), false));
        }
    };
    if !out.status.success() {
        return Err(friendly_error(&String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// yt-dlp's own words for DRM ("This video is DRM protected", "known to use DRM protection"),
/// not any "drm" in a video id or a site name ("hdrmovies").
fn is_drm(lower: &str) -> bool {
    lower.contains("drm protected") || lower.contains("drm protection") || lower.contains("drm-protected")
}

/// yt-dlp's error text in plain words, and whether a newer yt-dlp may fix it.
pub fn friendly_error(stderr: &str) -> (String, bool) {
    let last = stderr.lines().rev().find(|l| l.contains("ERROR")).unwrap_or(stderr.lines().last().unwrap_or("")).to_string();
    let l = last.to_lowercase();
    let msg = |s: &str| (s.to_string(), false);
    // "This video is DRM protected", "known to use DRM protection": never worked around.
    if is_drm(&l) {
        return msg(DRM);
    }
    if l.contains("private video") || l.contains("this video is private") {
        return msg("This video is private.");
    }
    if l.contains("confirm your age") || l.contains("age-restricted") || l.contains("age restricted") || l.contains("inappropriate for some users") {
        return msg("Age-restricted: it needs a signed-in browser, which Pako doesn't support yet.");
    }
    if l.contains("not a bot") {
        return msg("YouTube asks to sign in to prove you're not a bot. Try again later.");
    }
    if l.contains("members-only") || l.contains("join this channel") {
        return msg("Only the channel's members can watch this video.");
    }
    if l.contains("premiere") || l.contains("live event will begin") || l.contains("is upcoming") {
        return msg("This video hasn't started yet (premiere or upcoming live).");
    }
    if l.contains("video unavailable") || l.contains("has been removed") || l.contains("no longer available") || l.contains("not available in your country") {
        return msg("This video isn't available (removed, or blocked where you are).");
    }
    if l.contains("unsupported url") {
        return msg("Not a video Pako can download.");
    }
    if l.contains("timed out") || l.contains("getaddrinfo") || l.contains("unable to download webpage") || l.contains("connection") || l.contains("network") {
        return msg("No connection to the site.");
    }
    if l.contains("requested format is not available") || l.contains("unable to extract") || l.contains("nsig") || l.contains("signature") || l.contains("http error 403") || l.contains("player") {
        return (SITE_CHANGED.into(), true);
    }
    if last.trim().is_empty() {
        return ("The video tool stopped without saying why.".into(), false);
    }
    let text = last.trim().trim_start_matches("ERROR:").trim();
    (format!("The video tool said: {}", text.chars().take(160).collect::<String>()), false)
}

/// A network-type failure: worth waiting and trying again (the download continues).
pub fn is_network(msg: &str) -> bool {
    msg == "No connection to the site."
}

/// Arguments for downloading one video into `dest` as `name` (the extension may change to
/// .mkv when the streams can't go into MP4).
/// Which streams yt-dlp picks for a quality (the same choice for Pako's own download and for
/// yt-dlp's): the best video up to that height (MP4 preferred) plus the best sound, or the
/// sound alone (M4A preferred).
pub fn format_args(quality: u32, audio: Option<&str>) -> Vec<String> {
    match audio {
        Some(_) => vec!["-f".into(), "ba[ext=m4a]/ba/b".into()],
        None => {
            let q = if quality == 0 { 1080 } else { quality };
            // Height, like Pako's qualities ("res" is the smaller side: wrong for vertical videos).
            vec!["-f".into(), "bv*+ba/b".into(), "-S".into(), format!("height:{q},ext:mp4:m4a")]
        }
    }
}

/// `fragments` > 0: that many pieces at once for videos that come in small pieces (HLS /
/// DASH); it changes nothing for other videos. `rate`: at most this many bytes per second
/// (this video's share of the speed limit).
#[allow(clippy::too_many_arguments)]
pub fn download_command(
    t: &Tools,
    link: &str,
    quality: u32,
    audio: Option<&str>,
    dest: &Path,
    name: &str,
    fragments: u32,
    cookies: Option<&Path>,
    rate: Option<u64>,
) -> tokio::process::Command {
    let mut cmd = base_command(t);
    if let Some(c) = cookies {
        cmd.arg("--cookies").arg(c);
        // yt-dlp hands cookies to ffmpeg on its command line when ffmpeg downloads a stream
        // (where other programs can read them): with the browser's login, its own downloader
        // fetches HLS streams. (Live streams are the exception: yt-dlp's own downloader can't
        // follow them, so it still uses ffmpeg there.)
        cmd.args(["--downloader", "m3u8:native"]);
    }
    if let Some(r) = rate.filter(|r| *r > 0) {
        cmd.arg("--limit-rate").arg(r.max(1024).to_string());
    }
    let stem = Path::new(name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Video".into());
    let out = dest.join(out_template(&stem));
    cmd.args(["--newline", "--progress", "--no-playlist", "--no-mtime", "--windows-filenames", "--continue", "--no-overwrites"]);
    cmd.args(["--progress-template", "download:PAKO %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s %(progress.speed)s %(info.format_id)s"]);
    cmd.args(["--print", "before_dl:PAKO_FORMAT %(format_id)s"]);
    cmd.args(["--print", "after_move:PAKO_FILE %(filepath)s"]);
    if fragments > 0 {
        cmd.arg("-N").arg(fragments.to_string());
    }
    cmd.args(format_args(quality, audio));
    match audio {
        Some(fmt) => {
            cmd.args(["-x", "--audio-format"]).arg(if fmt == "mp3" { "mp3" } else { "m4a" });
        }
        None => {
            cmd.args(["--merge-output-format", "mp4/mkv"]);
        }
    }
    cmd.arg("-o").arg(out).arg(link);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}

/// yt-dlp's output name for a file name stem: "%" starts a field and "$" an environment
/// variable in its template, so both are doubled to stay as they are.
pub fn out_template(stem: &str) -> String {
    format!("{}.%(ext)s", stem.replace('%', "%%").replace('$', "$$"))
}

/// One line of yt-dlp's output during a download.
#[derive(Debug, PartialEq)]
pub enum Line {
    /// downloaded, total (exact or estimate), speed (bytes/s), format id
    Progress { downloaded: u64, total: Option<u64>, speed: Option<u64>, format: String },
    /// The formats being downloaded ("137+140" = video then sound).
    Formats(String),
    /// Where the finished file is.
    File(PathBuf),
    Other,
}

fn num(s: &str) -> Option<u64> {
    if s == "NA" || s == "None" {
        return None;
    }
    s.parse::<f64>().ok().map(|v| v as u64)
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim_end_matches(['\r', '\n']);
    if let Some(rest) = line.strip_prefix("PAKO_FILE ") {
        return Line::File(PathBuf::from(rest.trim()));
    }
    if let Some(rest) = line.strip_prefix("PAKO_FORMAT ") {
        return Line::Formats(rest.trim().to_string());
    }
    if let Some(rest) = line.strip_prefix("PAKO ") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() >= 4 {
            return Line::Progress {
                downloaded: num(parts[0]).unwrap_or(0),
                total: num(parts[1]).or_else(|| num(parts[2])),
                speed: num(parts[3]),
                format: parts.get(4).unwrap_or(&"").to_string(),
            };
        }
    }
    Line::Other
}

/// Reads lines from a child's output until it ends. Read as bytes: a line that isn't valid
/// UTF-8 (an odd title) is shown with replacement marks instead of stopping the reading (which
/// would fill the pipe and freeze yt-dlp).
pub async fn lines<R: tokio::io::AsyncRead + Unpin>(r: R, mut each: impl FnMut(String)) {
    let mut reader = BufReader::new(r);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let l = String::from_utf8_lossy(&buf);
                each(l.trim_end_matches(['\r', '\n']).to_string());
            }
        }
    }
}

/// Ends yt-dlp and what it started (ffmpeg joining the streams).
pub fn kill_tree(pid: u32) {
    let mut cmd = std::process::Command::new("taskkill");
    cmd.args(["/PID", &pid.to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let _ = cmd.status();
}

fn stem_of(name: &str) -> String {
    Path::new(name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

/// Is `file` one of the pieces yt-dlp makes for the output name `stem.%(ext)s`? Only its exact
/// patterns: "stem.f137.mp4" (a finished stream waiting to be joined), the same or "stem.mp4"
/// with ".part", ".ytdl" or ".part-Frag3", "stem.temp.mp4" (joining) and "stem.part".
/// "Song (2).mp4.part" or "Song remix.mp4.part" are not pieces of "Song".
pub fn is_leftover(file: &str, stem: &str) -> bool {
    let Some(rest) = file.strip_prefix(stem).and_then(|r| r.strip_prefix('.')) else { return false };
    if stem.is_empty() {
        return false;
    }
    let ext_ok = |e: &str| (1..=6).contains(&e.len()) && e.chars().all(|c| c.is_ascii_alphanumeric());
    let id_ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_+=".contains(c));
    if rest == "part" {
        return true;
    }
    // Pako's own stream pieces ("stem.p1a2b3c4d.f137.mp4.pakopart"; formerly without the
    // download's mark, and ending in ".zdpart" up to Pako 0.1.2): only with a format id.
    if let Some(b) = rest.strip_suffix(".pakopart").or_else(|| rest.strip_suffix(".zdpart")) {
        let b = match b.split_once('.') {
            Some((tag, more)) if tag.len() > 1 && tag.starts_with('p') && tag.chars().all(|c| c.is_ascii_alphanumeric()) => more,
            _ => b,
        };
        return b.split_once('.').is_some_and(|(first, ext)| ext_ok(ext) && first.strip_prefix('f').is_some_and(id_ok));
    }
    // What a piece still being written ends with.
    let (base, unfinished) = if let Some(b) = rest.strip_suffix(".ytdl") {
        (b, true)
    } else if let Some((b, frag)) = rest.split_once(".part-Frag") {
        let n = frag.strip_suffix(".part").unwrap_or(frag);
        if n.is_empty() || !n.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        (b, true)
    } else if let Some(b) = rest.strip_suffix(".part") {
        (b, true)
    } else {
        (rest, false)
    };
    match base.split_once('.') {
        // "stem.mp4.part"
        None => unfinished && ext_ok(base),
        // "stem.temp.mp4" (joining), "stem.f137.mp4" (one stream). A finished stream needs a
        // digit in its format id (they all have one), so "Song.final.mp4" stays.
        Some((first, ext)) => {
            ext_ok(ext)
                && (first == "temp"
                    || first.strip_prefix('f').is_some_and(|id| id_ok(id) && (unfinished || id.chars().any(|c| c.is_ascii_digit()))))
        }
    }
}

/// yt-dlp's unfinished pieces for a download (deleted when the download is removed or started
/// over). `others` = names of the other downloads in that folder: their pieces are never
/// touched, even when one name starts like this one ("Song" and "Song.f1").
/// `download_id`: Pako's own pieces marked for another download (same title, same folder) stay.
pub fn leftovers(dest: &Path, name: &str, download_id: &str, others: &[String]) -> Vec<PathBuf> {
    let stem = stem_of(name);
    let own = format!("{stem}.{}.", piece_tag(download_id));
    let anothers_piece = |n: &str| {
        (n.ends_with(".pakopart") || n.ends_with(".zdpart"))
            && n.strip_prefix(&format!("{stem}.p")).is_some_and(|r| r.split_once('.').is_some_and(|(t, _)| t.chars().all(|c| c.is_ascii_alphanumeric())))
            && !n.starts_with(&own)
    };
    if stem.is_empty() {
        return Vec::new();
    }
    let other_stems: Vec<String> = others.iter().map(|o| stem_of(o)).filter(|o| *o != stem).collect();
    std::fs::read_dir(dest)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    is_leftover(&n, &stem) && !anothers_piece(&n) && !others.contains(&n) && !other_stems.iter().any(|o| is_leftover(&n, o))
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------- fast downloads: Pako downloads the streams itself ----------

/// One stream of the chosen quality (picture, sound, or both), as yt-dlp found it.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub format_id: String,
    pub url: String,
    pub ext: String,
    pub protocol: String,
    /// The headers the site wants for this stream (User-Agent, Referer, cookies...).
    pub headers: Vec<(String, String)>,
    pub size: Option<u64>,
    /// Ask for at most this many bytes per request (YouTube slows down bigger requests).
    pub chunk: Option<u64>,
    pub has_video: bool,
    pub has_audio: bool,
}

/// How a video is downloaded.
#[derive(Debug, PartialEq)]
pub enum Plan {
    /// Plain files on a web server: Pako downloads them itself, in parallel parts, all at once.
    Direct(Vec<Stream>),
    /// Small pieces (HLS / DASH): yt-dlp, many pieces at a time.
    Fragments,
    /// Anything else (live streams, other kinds): yt-dlp as it is.
    Plain,
}

#[derive(Deserialize, Default)]
struct SelFormat {
    #[serde(default)]
    format_id: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    ext: Option<String>,
    #[serde(default)]
    protocol: Option<String>,
    #[serde(default)]
    http_headers: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    cookies: Option<String>,
    #[serde(default)]
    filesize: Option<f64>,
    #[serde(default)]
    vcodec: Option<String>,
    #[serde(default)]
    acodec: Option<String>,
    #[serde(default)]
    downloader_options: Option<SelOptions>,
    /// true, false or "maybe".
    #[serde(default)]
    has_drm: Option<serde_json::Value>,
    #[serde(default)]
    fragments: Option<serde_json::Value>,
}

#[derive(Deserialize, Default)]
struct SelOptions {
    #[serde(default)]
    http_chunk_size: Option<f64>,
}

#[derive(Deserialize)]
struct SelInfo {
    #[serde(default)]
    is_live: Option<bool>,
    #[serde(default)]
    requested_formats: Option<Vec<SelFormat>>,
    #[serde(flatten)]
    top: SelFormat,
}

/// yt-dlp's cookie text for a stream ("a=1; Domain=.x.com; Path=/; Secure; b=2") as a Cookie
/// header ("a=1; b=2").
pub fn cookie_header(text: &str) -> String {
    const ATTRS: [&str; 8] = ["domain", "path", "secure", "expires", "version", "httponly", "max-age", "samesite"];
    text.split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .filter(|p| {
            let key = p.split('=').next().unwrap_or("").trim().to_ascii_lowercase();
            p.contains('=') && !ATTRS.contains(&key.as_str())
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Protocols where the stream is one plain file Pako can download in parallel parts.
fn is_direct(protocol: &str) -> bool {
    matches!(protocol, "https" | "http")
}

/// Protocols where the stream comes in small pieces (yt-dlp downloads many at once with -N).
fn is_fragmented(protocol: &str) -> bool {
    matches!(protocol, "m3u8" | "m3u8_native" | "http_dash_segments" | "http_dash_segments_generator")
}

fn stream_of(f: SelFormat) -> Option<Stream> {
    let url = f.url?;
    let mut headers: Vec<(String, String)> = f.http_headers.unwrap_or_default().into_iter().collect();
    if let Some(c) = f.cookies.as_deref().map(cookie_header).filter(|c| !c.is_empty()) {
        headers.retain(|(k, _)| !k.eq_ignore_ascii_case("cookie"));
        headers.push(("Cookie".into(), c));
    }
    Some(Stream {
        format_id: f.format_id.unwrap_or_default(),
        url,
        ext: f.ext.unwrap_or_default(),
        protocol: f.protocol.unwrap_or_default(),
        headers,
        size: f.filesize.filter(|s| *s > 0.0).map(|s| s as u64),
        chunk: f.downloader_options.and_then(|o| o.http_chunk_size).filter(|c| *c >= 1024.0 * 1024.0).map(|c| c as u64),
        has_video: may_have(&f.vcodec),
        has_audio: may_have(&f.acodec),
    })
}

/// Reads `yt-dlp -J -f ...` (the chosen streams) and decides how to download them.
/// The inner Err is DRM (never worked around).
pub fn classify(json: &str) -> anyhow::Result<Result<Plan, String>> {
    let info: SelInfo = serde_json::from_str(json)?;
    let mut formats = match info.requested_formats {
        Some(list) if !list.is_empty() => list,
        _ => vec![info.top],
    };
    if formats.iter().any(|f| f.has_drm.as_ref().is_some_and(|v| v.as_bool() == Some(true))) {
        return Ok(Err(DRM.into()));
    }
    if info.is_live == Some(true) {
        return Ok(Ok(Plan::Plain));
    }
    let protocols: Vec<String> = formats.iter().map(|f| f.protocol.clone().unwrap_or_default()).collect();
    let direct = protocols.iter().all(|p| is_direct(p))
        && formats.iter().all(|f| f.fragments.is_none() && f.url.as_deref().is_some_and(|u| u.starts_with("http")));
    if direct {
        let streams: Vec<Stream> = formats.drain(..).filter_map(stream_of).collect();
        if !streams.is_empty() {
            return Ok(Ok(Plan::Direct(streams)));
        }
    }
    if protocols.iter().any(|p| is_fragmented(p)) && protocols.iter().all(|p| is_fragmented(p) || is_direct(p)) {
        return Ok(Ok(Plan::Fragments));
    }
    Ok(Ok(Plan::Plain))
}

/// Asks yt-dlp which streams make up this quality, with their addresses (nothing downloaded).
/// `cookies`: the download's cookie file, if it came from the browser.
pub async fn select(link: &str, quality: u32, audio: Option<&str>, limit: Duration, cookies: Option<&Path>) -> Result<Plan, (String, bool)> {
    let Some(t) = tools() else { return Err((NEEDS_TOOLS.into(), false)) };
    let mut cmd = base_command(&t);
    if let Some(c) = cookies {
        cmd.arg("--cookies").arg(c);
    }
    cmd.args(["-J", "--no-playlist", "--no-warnings"]).args(format_args(quality, audio)).arg(link);
    let json = run_json(cmd, limit).await?;
    match classify(&json) {
        Ok(Ok(plan)) => Ok(plan),
        Ok(Err(drm)) => Err((drm, false)),
        Err(_) => Err(("The video tool gave an answer Pako couldn't read.".into(), true)),
    }
}

/// Where Pako keeps one stream while it downloads: "Title.p1a2b3c4d.f137.mp4.pakopart" (never
/// yt-dlp's own names, so yt-dlp can't mistake it for one of its pieces).
pub fn stream_part(dest: &Path, name: &str, download_id: &str, format_id: &str, ext: &str) -> PathBuf {
    dest.join(piece_name(name, Some(&piece_tag(download_id)), format_id, ext, crate::names::PART_EXT))
}

/// The same piece as Pako 0.1.2 and earlier named it ("...f137.mp4.zdpart").
pub fn old_stream_part(dest: &Path, name: &str, download_id: &str, format_id: &str, ext: &str) -> PathBuf {
    dest.join(piece_name(name, Some(&piece_tag(download_id)), format_id, ext, crate::names::OLD_PART_EXT))
}

/// The piece to continue: one an older Pako left (".zdpart", and with `legacy` also one named
/// before pieces carried the download's mark) takes the new name; when both are there, the one
/// written last is kept (see `names::adopt_old`). Returns the new name.
pub fn adopt_old_pieces(dest: &Path, name: &str, download_id: &str, format_id: &str, ext: &str, legacy: bool) -> PathBuf {
    let part = stream_part(dest, name, download_id, format_id, ext);
    crate::names::adopt_old(&old_stream_part(dest, name, download_id, format_id, ext), &part);
    if legacy {
        crate::names::adopt_old(&legacy_stream_part(dest, name, format_id, ext), &part);
    }
    part
}

fn piece_name(name: &str, tag: Option<&str>, format_id: &str, ext: &str, part_ext: &str) -> String {
    let id: String = format_id.chars().map(|c| if c.is_ascii_alphanumeric() || "-_+=".contains(c) { c } else { '_' }).collect();
    let id = if id.is_empty() { "0".to_string() } else { id };
    let ext: String = ext.chars().filter(|c| c.is_ascii_alphanumeric()).take(6).collect();
    let ext = if ext.is_empty() { "bin".to_string() } else { ext };
    match tag {
        Some(t) => format!("{}.{t}.f{id}.{ext}.{part_ext}", stem_of(name)),
        None => format!("{}.f{id}.{ext}.{part_ext}", stem_of(name)),
    }
}

/// The download's mark in its piece names ("p" and the start of its id): two downloads with
/// the same title in the same folder (the video and its sound) never share a piece.
pub fn piece_tag(download_id: &str) -> String {
    let id: String = download_id.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect::<String>().to_ascii_lowercase();
    format!("p{id}")
}

/// Pieces named before they carried the download's mark ("stem.f137.mp4.zdpart", only ever
/// with the old ending): a download in progress from an older Pako continues them under the
/// new name.
pub fn legacy_stream_part(dest: &Path, name: &str, format_id: &str, ext: &str) -> PathBuf {
    dest.join(piece_name(name, None, format_id, ext, crate::names::OLD_PART_EXT))
}

/// One downloaded stream to put in the final file.
#[derive(Debug, Clone)]
pub struct Piece {
    pub path: PathBuf,
    pub ext: String,
    pub has_video: bool,
    pub has_audio: bool,
}

/// What happens after the streams are in: the final extension, and the ffmpeg arguments
/// (None = the single stream already is the final file: it is just renamed).
#[derive(Debug, PartialEq)]
pub struct Finish {
    pub ext: String,
    pub ffmpeg: Option<Vec<std::ffi::OsString>>,
}

fn mp4_video(ext: &str) -> bool {
    matches!(ext, "mp4" | "m4v" | "mov")
}

fn mp4_audio(ext: &str) -> bool {
    matches!(ext, "m4a" | "mp4" | "aac" | "mp3" | "m4b")
}

/// How the pieces become one file, like yt-dlp does it: picture and sound copied as they are
/// into MP4 (MKV when they don't fit in MP4); audio only converted to M4A or MP3 when needed.
pub fn finish_plan(pieces: &[Piece], audio: Option<&str>, out: &Path) -> Finish {
    use std::ffi::OsString;
    let mut args: Vec<OsString> = ["-hide_banner", "-nostdin", "-loglevel", "error", "-y"].iter().map(OsString::from).collect();
    for p in pieces {
        args.push("-i".into());
        args.push(p.path.clone().into_os_string());
    }
    let map = |args: &mut Vec<OsString>, what: String| {
        args.push("-map".into());
        args.push(what.into());
    };
    // The piece the sound comes from when only sound is wanted (or there is no picture).
    let sound_piece = pieces.iter().position(|p| p.has_audio).unwrap_or(0);
    if let Some(fmt) = audio {
        let target = if fmt == "mp3" { "mp3" } else { "m4a" };
        let p = &pieces[sound_piece];
        if pieces.len() == 1 && !p.has_video && p.ext == target {
            return Finish { ext: target.into(), ffmpeg: None };
        }
        map(&mut args, format!("{sound_piece}:a:0"));
        args.push("-vn".into());
        if target == "mp3" {
            args.extend(["-c:a", "libmp3lame", "-q:a", "5", "-f", "mp3"].iter().map(OsString::from));
        } else if matches!(p.ext.as_str(), "m4a" | "mp4" | "aac") {
            args.extend(["-c:a", "copy", "-f", "mp4"].iter().map(OsString::from));
        } else {
            args.extend(["-c:a", "aac", "-b:a", "192k", "-f", "mp4"].iter().map(OsString::from));
        }
        args.push(out.as_os_str().to_owned());
        return Finish { ext: target.into(), ffmpeg: Some(args) };
    }
    // No piece has a picture (the site only had sound): an audio file, its sound copied as is.
    // (A sound file's own kind says it: an .mp3 never holds a picture, whatever the site says.)
    let Some(video) = pieces.iter().position(|p| p.has_video && !sound_file(&p.ext)) else {
        let p = &pieces[sound_piece];
        let (ext, format) = audio_container(&p.ext);
        if pieces.len() == 1 && p.ext == ext {
            return Finish { ext: ext.into(), ffmpeg: None };
        }
        map(&mut args, format!("{sound_piece}:a:0"));
        args.extend(["-vn", "-c", "copy", "-f", format].iter().map(OsString::from));
        args.push(out.as_os_str().to_owned());
        return Finish { ext: ext.into(), ffmpeg: Some(args) };
    };
    let sound = pieces.iter().enumerate().position(|(i, p)| i != video && p.has_audio);
    let fits_mp4 = mp4_video(&pieces[video].ext) && sound.is_none_or(|a| mp4_audio(&pieces[a].ext));
    let ext = if fits_mp4 { "mp4" } else { "mkv" };
    if pieces.len() == 1 && pieces[0].ext == ext {
        return Finish { ext: ext.into(), ffmpeg: None };
    }
    // Optional maps ("?"): a piece whose picture or sound was only "maybe" doesn't stop the
    // join. The picture piece's own sound is kept too when it has some (one sound track is
    // enough: it is only used when the sound piece turns out to have none).
    map(&mut args, format!("{video}:v:0?"));
    match sound {
        Some(a) => {
            map(&mut args, format!("{a}:a:0?"));
            if pieces[video].has_audio {
                // (Players play the first sound track: the sound piece's.)
                map(&mut args, format!("{video}:a:0?"));
            }
        }
        None => map(&mut args, format!("{video}:a:0?")),
    }
    args.extend(["-c", "copy"].iter().map(OsString::from));
    if ext == "mp4" {
        args.extend(["-movflags", "+faststart", "-f", "mp4"].iter().map(OsString::from));
    } else {
        args.extend(["-f", "matroska"].iter().map(OsString::from));
    }
    args.push(out.as_os_str().to_owned());
    Finish { ext: ext.into(), ffmpeg: Some(args) }
}

/// Kinds of file that only ever hold sound.
fn sound_file(ext: &str) -> bool {
    matches!(ext, "mp3" | "m4a" | "aac" | "opus" | "ogg" | "oga" | "wav" | "flac" | "m4b")
}

/// For sound without a picture: the file's extension and ffmpeg's name for that kind of file.
fn audio_container(ext: &str) -> (&'static str, &'static str) {
    match ext {
        "m4a" | "mp4" | "aac" | "m4b" => ("m4a", "mp4"),
        "mp3" => ("mp3", "mp3"),
        "webm" => ("webm", "webm"),
        "ogg" => ("ogg", "ogg"),
        "opus" => ("opus", "ogg"),
        _ => ("mka", "matroska"),
    }
}

/// A download error that new stream addresses may fix (they expire, or the site moved them).
pub fn worth_new_links(msg: &str) -> bool {
    msg.starts_with("The server refused access")
        || msg.starts_with("File not found on the server")
        || msg.starts_with("The server stopped allowing resumed downloads")
        || msg.contains("changed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_video_sites() {
        assert!(is_video_site("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_video_site("https://youtu.be/dQw4w9WgXcQ"));
        assert!(is_video_site("https://www.youtube.com/shorts/abc123"));
        assert!(is_video_site("https://www.youtube.com/playlist?list=PL123"));
        assert!(is_video_site("https://vimeo.com/123456"));
        assert!(is_video_site("https://x.com/user/status/123"));
        assert!(!is_video_site("https://www.youtube.com/@channel"));
        assert!(!is_video_site("https://drive.google.com/file/d/abc/view"));
        assert!(!is_video_site("https://example.com/file.zip"));
    }

    #[test]
    fn reads_formats_and_picks_quality() {
        let json = r#"{"title":"My video","duration":100,"extractor_key":"Youtube","formats":[
            {"format_id":"140","ext":"m4a","vcodec":"none","acodec":"mp4a.40.2","filesize":1600000,"tbr":128},
            {"format_id":"251","ext":"webm","vcodec":"none","acodec":"opus","filesize":1500000,"tbr":130},
            {"format_id":"137","ext":"mp4","height":1080,"vcodec":"avc1","acodec":"none","filesize":50000000},
            {"format_id":"248","ext":"webm","height":1080,"vcodec":"vp9","acodec":"none","filesize":40000000},
            {"format_id":"136","ext":"mp4","height":720,"vcodec":"avc1","acodec":"none","filesize":25000000},
            {"format_id":"18","ext":"mp4","height":360,"vcodec":"avc1","acodec":"mp4a","filesize":8000000}
        ]}"#;
        let p = parse_info(json).unwrap();
        assert_eq!(p.title, "My video");
        assert_eq!(p.audio_size, Some(1_600_000));
        let heights: Vec<u32> = p.options.iter().map(|o| o.height).collect();
        assert_eq!(heights, vec![1080, 720, 360]);
        // MP4 preferred at 1080, plus the sound.
        assert_eq!(p.options[0].size, Some(51_600_000));
        // Combined format: no sound added.
        assert_eq!(p.options[2].size, Some(8_000_000));
        assert_eq!(pick(&p.options, 1080).unwrap().height, 1080);
        assert_eq!(pick(&p.options, 900).unwrap().height, 720);
        assert_eq!(pick(&p.options, 240).unwrap().height, 360);
        assert_eq!(estimate(&p.options, p.audio_size, 0), Some(1_600_000));
    }

    #[test]
    fn reads_playlists() {
        let json = r#"{"_type":"playlist","title":"Mix","entries":[
            {"url":"https://www.youtube.com/watch?v=a","title":"One","duration":60},
            {"id":"b","ie_key":"Youtube","title":"Two"}
        ]}"#;
        let p = parse_info(json).unwrap();
        assert!(p.is_playlist);
        assert_eq!(p.entries.len(), 2);
        assert_eq!(p.entries[1].0, "https://www.youtube.com/watch?v=b");
    }

    #[test]
    fn reads_progress_lines() {
        assert_eq!(
            parse_line("PAKO 1048576 10485760 NA 524288.5 137"),
            Line::Progress { downloaded: 1_048_576, total: Some(10_485_760), speed: Some(524_288), format: "137".into() }
        );
        assert_eq!(
            parse_line("PAKO 100 NA 2000 NA 140"),
            Line::Progress { downloaded: 100, total: Some(2000), speed: None, format: "140".into() }
        );
        assert_eq!(parse_line("PAKO_FORMAT 137+140"), Line::Formats("137+140".into()));
        assert_eq!(parse_line("PAKO_FILE C:\\Videos\\My video.mp4"), Line::File(PathBuf::from("C:\\Videos\\My video.mp4")));
        assert_eq!(parse_line("[youtube] Extracting URL"), Line::Other);
    }

    #[test]
    fn plain_errors() {
        assert_eq!(friendly_error("ERROR: [youtube] abc: Private video. Sign in").0, "This video is private.");
        assert!(friendly_error("ERROR: [youtube] abc: Sign in to confirm your age").0.starts_with("Age-restricted"));
        assert!(friendly_error("ERROR: [youtube] abc: Requested format is not available").1);
        assert!(is_network(&friendly_error("ERROR: Unable to download webpage: timed out").0));
    }

    #[test]
    fn leftover_patterns() {
        for yes in [
            "Song.f137.mp4",
            "Song.f140.m4a",
            "Song.f137.mp4.part",
            "Song.mp4.part",
            "Song.f137.mp4.ytdl",
            "Song.f137.mp4.part-Frag3",
            "Song.f137.mp4.part-Frag12.part",
            "Song.temp.mp4",
            "Song.part",
            "Song.fhls-1080p.mp4",
            "Song.fsd.mp4.part",
        ] {
            assert!(is_leftover(yes, "Song"), "{yes}");
        }
        for no in [
            "Song.mp4",
            "Song.webm",
            "Song (2).f137.mp4.part",
            "Song (2).mp4.part",
            "Song remix.mp4.part",
            "Songbird.mp4.part",
            "Song.final.mp4",
            "Song.final cut.mp4.part",
            "Song.v2.mp4.part",
            "Song.f137.mp4.part-Fragx",
            "Song.txt",
        ] {
            assert!(!is_leftover(no, "Song"), "{no}");
        }
        assert!(!is_leftover("x.mp4.part", ""));
    }

    #[test]
    fn leftovers_skip_other_downloads() {
        let dir = std::env::temp_dir().join(format!("pako-leftovers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["Song.f137.mp4.part", "Song.f140.m4a", "Song.f1.mp4.part", "Song (2).mp4.part", "Song.mp4"] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let mut got: Vec<String> = leftovers(&dir, "Song.mp4", "abc", &["Song.f1.mp4".to_string()])
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        got.sort();
        assert_eq!(got, vec!["Song.f137.mp4.part", "Song.f140.m4a"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The video and its sound-only twin (same title, same folder): each keeps its own pieces.
    #[test]
    fn same_title_downloads_keep_their_own_pieces() {
        let dir = std::env::temp_dir().join(format!("pako-twins-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let mine = stream_part(&dir, "Song.mp4", "aaaa1111", "140", "m4a");
        let theirs = stream_part(&dir, "Song.m4a", "bbbb2222", "140", "m4a");
        assert_ne!(mine, theirs);
        std::fs::write(&mine, b"x").unwrap();
        std::fs::write(&theirs, b"x").unwrap();
        std::fs::write(dir.join("Song.f140.m4a.zdpart"), b"x").unwrap();
        // The same, as Pako 0.1.2 named them (".zdpart").
        let (my_old, their_old) = (
            old_stream_part(&dir, "Song.mp4", "aaaa1111", "137", "mp4"),
            old_stream_part(&dir, "Song.m4a", "bbbb2222", "137", "mp4"),
        );
        std::fs::write(&my_old, b"x").unwrap();
        std::fs::write(&their_old, b"x").unwrap();
        let got = leftovers(&dir, "Song.mp4", "aaaa1111", &["Song.m4a".to_string()]);
        assert!(got.contains(&mine));
        assert!(!got.contains(&theirs));
        assert!(got.contains(&my_old));
        assert!(!got.contains(&their_old));
        // An unmarked one from an older Pako still goes with it.
        assert!(got.contains(&dir.join("Song.f140.m4a.zdpart")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A process started in Pako's job runs normally (it is started paused, then let go).
    #[cfg(windows)]
    #[tokio::test]
    async fn spawned_in_the_job_runs() {
        let mut cmd = tokio::process::Command::new("cmd.exe");
        cmd.args(["/C", "echo pako"]).stdout(Stdio::piped());
        let child = spawn(&mut cmd).unwrap();
        let out = tokio::time::timeout(Duration::from_secs(20), child.wait_with_output()).await.unwrap().unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("pako"));
    }

    #[tokio::test]
    async fn reads_lines_that_are_not_utf8() {
        let data: &[u8] = b"PAKO 1 2 NA NA 137\r\nbad \xff\xfe title\nPAKO_FILE C:\\x.mp4\n";
        let mut got = Vec::new();
        lines(data, |l| got.push(l)).await;
        assert_eq!(got.len(), 3);
        assert_eq!(got[0], "PAKO 1 2 NA NA 137");
        assert_eq!(got[2], "PAKO_FILE C:\\x.mp4");
    }

    #[test]
    fn output_template_keeps_dollar_and_percent() {
        assert_eq!(out_template("Price $HOME 100%"), "Price $$HOME 100%%.%(ext)s");
        assert_eq!(out_template("Plain"), "Plain.%(ext)s");
    }

    #[test]
    fn sorts_by_height() {
        let t = Tools { yt_dlp: PathBuf::from("yt-dlp.exe"), deno: None, ffmpeg: None };
        let cmd = download_command(&t, "https://x.com/v", 720, None, Path::new("C:\\D"), "V.mp4", 0, None, None);
        let args: Vec<String> = cmd.as_std().get_args().map(|a| a.to_string_lossy().to_string()).collect();
        assert!(args.iter().any(|a| a == "height:720,ext:mp4:m4a"));
        assert!(!args.iter().any(|a| a == "--limit-rate" || a == "--downloader"));
        // A speed limit share, and the browser's login kept off ffmpeg's command line.
        let cmd = download_command(&t, "https://x.com/v", 720, None, Path::new("C:\\D"), "V.mp4", 0, Some(Path::new("j.txt")), Some(250_000));
        let args: Vec<String> = cmd.as_std().get_args().map(|a| a.to_string_lossy().to_string()).collect();
        assert!(args.windows(2).any(|w| w == ["--limit-rate", "250000"]));
        assert!(args.windows(2).any(|w| w == ["--downloader", "m3u8:native"]));
        assert!(args.windows(2).any(|w| w == ["--cookies", "j.txt"]));
    }

    // ---------- fast downloads ----------

    #[test]
    fn plain_files_are_downloaded_by_pako() {
        // YouTube: picture and sound as two plain files, with the site's headers and cookies.
        let json = r#"{"title":"T","is_live":false,"protocol":"https+https","requested_formats":[
            {"format_id":"137","url":"https://rr1.googlevideo.com/v","ext":"mp4","protocol":"https","vcodec":"avc1.640028","acodec":"none",
             "filesize":50000000,"http_headers":{"User-Agent":"UA-1","Accept":"*/*"},"downloader_options":{"http_chunk_size":10485760},
             "cookies":"SID=abc; Domain=.youtube.com; Path=/; Secure; Expires=1999999999; PREF=x"},
            {"format_id":"140","url":"https://rr1.googlevideo.com/a","ext":"m4a","protocol":"https","vcodec":"none","acodec":"mp4a.40.2",
             "filesize":1600000,"http_headers":{"User-Agent":"UA-1"}}
        ]}"#;
        let Plan::Direct(streams) = classify(json).unwrap().unwrap() else { panic!("not direct") };
        assert_eq!(streams.len(), 2);
        let v = &streams[0];
        assert_eq!(v.format_id, "137");
        assert_eq!(v.url, "https://rr1.googlevideo.com/v");
        assert_eq!(v.chunk, Some(10_485_760));
        assert_eq!(v.size, Some(50_000_000));
        assert!(v.has_video && !v.has_audio);
        assert!(v.headers.contains(&("User-Agent".into(), "UA-1".into())));
        assert!(v.headers.contains(&("Cookie".into(), "SID=abc; PREF=x".into())));
        assert!(!streams[1].has_video && streams[1].has_audio);
        assert_eq!(streams[1].chunk, None);
    }

    #[test]
    fn one_plain_file_is_direct_too() {
        let json = r#"{"format_id":"http-720p","url":"https://cdn.example.com/v.mp4","ext":"mp4","protocol":"https",
            "vcodec":"h264","acodec":"aac","http_headers":{"Referer":"https://example.com/"}}"#;
        let Plan::Direct(streams) = classify(json).unwrap().unwrap() else { panic!("not direct") };
        assert_eq!(streams.len(), 1);
        assert!(streams[0].has_video && streams[0].has_audio);
        assert_eq!(streams[0].headers, vec![("Referer".to_string(), "https://example.com/".to_string())]);
    }

    #[test]
    fn small_pieces_and_live_go_to_yt_dlp() {
        let hls = r#"{"format_id":"hls-1080","url":"https://x/m.m3u8","ext":"mp4","protocol":"m3u8_native"}"#;
        assert_eq!(classify(hls).unwrap().unwrap(), Plan::Fragments);
        let dash = r#"{"requested_formats":[
            {"format_id":"v","url":"https://x/v","protocol":"http_dash_segments","fragments":[{"path":"a"}]},
            {"format_id":"a","url":"https://x/a","protocol":"https"}]}"#;
        assert_eq!(classify(dash).unwrap().unwrap(), Plan::Fragments);
        let live = r#"{"is_live":true,"format_id":"1","url":"https://x/v","protocol":"https"}"#;
        assert_eq!(classify(live).unwrap().unwrap(), Plan::Plain);
        let rtmp = r#"{"format_id":"1","url":"rtmp://x/v","protocol":"rtmp"}"#;
        assert_eq!(classify(rtmp).unwrap().unwrap(), Plan::Plain);
        // An https stream that still comes in pieces is not one plain file.
        let pieces = r#"{"format_id":"1","url":"https://x/v","protocol":"https","fragments":[{"url":"https://x/1"}]}"#;
        assert_eq!(classify(pieces).unwrap().unwrap(), Plan::Plain);
    }

    #[test]
    fn drm_is_said_plainly() {
        let json = r#"{"format_id":"1","url":"https://x/v","protocol":"https","has_drm":true}"#;
        assert_eq!(classify(json).unwrap().unwrap_err(), DRM);
        let maybe = r#"{"format_id":"1","url":"https://x/v","protocol":"https","has_drm":"maybe"}"#;
        assert!(classify(maybe).unwrap().is_ok());
        assert_eq!(friendly_error("ERROR: [Netflix] 123: This video is DRM protected").0, DRM);
        assert_eq!(friendly_error("ERROR: The requested site is known to use DRM protection. It will NOT be supported.").0, DRM);
        assert!(!friendly_error("ERROR: This video is DRM protected").1);
    }

    #[test]
    fn cookie_text_becomes_a_header() {
        assert_eq!(cookie_header("a=1; Domain=.x.com; Path=/; Secure; Expires=123; b=2; HttpOnly"), "a=1; b=2");
        assert_eq!(cookie_header(""), "");
    }

    #[test]
    fn new_links_only_for_refused_or_moved_streams() {
        assert!(worth_new_links("The server refused access (403). The link may have expired."));
        assert!(worth_new_links("File not found on the server (404)."));
        assert!(worth_new_links("The file on the server changed since this download started. Use Start over to download it again."));
        assert!(!worth_new_links("Not enough free space on D:"));
        assert!(!worth_new_links("Connection timed out"));
    }

    fn piece(path: &str, ext: &str, v: bool, a: bool) -> Piece {
        Piece { path: PathBuf::from(path), ext: ext.into(), has_video: v, has_audio: a }
    }

    fn args_of(f: &Finish) -> Vec<String> {
        f.ffmpeg.as_ref().unwrap().iter().map(|a| a.to_string_lossy().to_string()).collect()
    }

    #[test]
    fn joins_picture_and_sound_without_reencoding() {
        let out = Path::new("C:\\D\\T.temp.mp4");
        let f = finish_plan(&[piece("v.pakopart", "mp4", true, false), piece("a.pakopart", "m4a", false, true)], None, out);
        assert_eq!(f.ext, "mp4");
        let a = args_of(&f);
        let at = |x: &str| a.iter().position(|y| y == x).unwrap();
        assert_eq!(a[at("-i") + 1], "v.pakopart");
        assert_eq!(&a[at("-c")..at("-c") + 2], &["-c".to_string(), "copy".to_string()]);
        assert!(a.windows(2).any(|w| w == ["-map", "0:v:0?"]));
        assert!(a.windows(2).any(|w| w == ["-map", "1:a:0?"]));
        assert!(!a.windows(2).any(|w| w == ["-map", "0:a:0?"]));
        assert!(a.windows(2).any(|w| w == ["-f", "mp4"]));
        assert_eq!(a.last().unwrap(), "C:\\D\\T.temp.mp4");
        // WebM / VP9 + Opus don't fit in MP4: MKV, still copied as they are.
        let f = finish_plan(&[piece("v", "webm", true, false), piece("a", "webm", false, true)], None, out);
        assert_eq!(f.ext, "mkv");
        assert!(args_of(&f).windows(2).any(|w| w == ["-f", "matroska"]));
        // One MP4 with picture and sound: already the final file.
        let f = finish_plan(&[piece("v", "mp4", true, true)], None, out);
        assert_eq!(f, Finish { ext: "mp4".into(), ffmpeg: None });
    }

    /// Streams whose codecs the site doesn't name: nothing is left out, nothing required.
    #[test]
    fn joining_keeps_sound_of_unnamed_streams() {
        assert!(may_have(&None));
        assert!(may_have(&Some("avc1".into())));
        assert!(!may_have(&Some("none".into())));
        let out = Path::new("T.temp.mkv");
        // Both "maybe": the picture piece's sound is kept as well as the other piece's.
        let f = finish_plan(&[piece("v", "mp4", true, true), piece("a", "webm", true, true)], None, out);
        let a = args_of(&f);
        assert!(a.windows(2).any(|w| w == ["-map", "0:v:0?"]));
        assert!(a.windows(2).any(|w| w == ["-map", "1:a:0?"]));
        assert!(a.windows(2).any(|w| w == ["-map", "0:a:0?"]));
        // One piece that isn't MP4: remuxed with its sound.
        let f = finish_plan(&[piece("v", "webm", true, true)], None, out);
        let a = args_of(&f);
        assert!(a.windows(2).any(|w| w == ["-map", "0:a:0?"]));
        assert_eq!(f.ext, "mkv");
    }

    /// No piece has a picture: an audio file with the right extension, not a silent "video".
    #[test]
    fn sound_only_pieces_become_an_audio_file() {
        let out = Path::new("T.temp.x");
        assert_eq!(finish_plan(&[piece("a", "m4a", false, true)], None, out), Finish { ext: "m4a".into(), ffmpeg: None });
        assert_eq!(finish_plan(&[piece("a", "webm", false, true)], None, out), Finish { ext: "webm".into(), ffmpeg: None });
        let f = finish_plan(&[piece("a", "mp4", false, true)], None, out);
        assert_eq!(f.ext, "m4a");
        assert!(args_of(&f).windows(2).any(|w| w == ["-f", "mp4"]));
        // The site named no codecs, but an .mp3 is sound: saved as it is, not put in a video file.
        assert_eq!(finish_plan(&[piece("a", "mp3", true, true)], None, out), Finish { ext: "mp3".into(), ffmpeg: None });
        let f = finish_plan(&[piece("x", "mp4", false, false), piece("a", "opus", false, true)], None, out);
        assert_eq!(f.ext, "opus");
        assert!(args_of(&f).windows(2).any(|w| w == ["-map", "1:a:0"]));
    }

    #[test]
    fn audio_only_is_converted_only_when_needed() {
        let out = Path::new("T.temp.m4a");
        assert_eq!(finish_plan(&[piece("a", "m4a", false, true)], Some("m4a"), out), Finish { ext: "m4a".into(), ffmpeg: None });
        let f = finish_plan(&[piece("a", "webm", false, true)], Some("m4a"), out);
        assert!(args_of(&f).windows(2).any(|w| w == ["-c:a", "aac"]));
        let f = finish_plan(&[piece("a", "m4a", false, true)], Some("mp3"), Path::new("T.temp.mp3"));
        assert_eq!(f.ext, "mp3");
        let a = args_of(&f);
        assert!(a.windows(2).any(|w| w == ["-c:a", "libmp3lame"]));
        assert!(a.iter().any(|x| x == "-vn"));
        // A video picked because there was no sound-only stream: its sound is copied out.
        let f = finish_plan(&[piece("v", "mp4", true, true)], Some("m4a"), out);
        assert!(args_of(&f).windows(2).any(|w| w == ["-c:a", "copy"]));
    }

    #[test]
    fn stream_pieces_have_their_own_names() {
        let p = stream_part(Path::new("C:\\D"), "My video.mp4", "1A2B3C4D-5e6f", "137", "mp4");
        assert_eq!(p, PathBuf::from("C:\\D\\My video.p1a2b3c4d.f137.mp4.pakopart"));
        // Odd format ids stay one safe word.
        let p = stream_part(Path::new("C:\\D"), "V.mp4", "id", "hls/1080.p", "mp4");
        assert_eq!(p.file_name().unwrap().to_string_lossy(), "V.pid.fhls_1080_p.mp4.pakopart");
        assert!(is_leftover("My video.p1a2b3c4d.f137.mp4.pakopart", "My video"));
        assert!(is_leftover("V.pid.fhls_1080_p.mp4.pakopart", "V"));
        // Named by an older Pako (0.1.2: ".zdpart"; before that, also without the mark).
        let old = old_stream_part(Path::new("C:\\D"), "My video.mp4", "1A2B3C4D-5e6f", "137", "mp4");
        assert_eq!(old, PathBuf::from("C:\\D\\My video.p1a2b3c4d.f137.mp4.zdpart"));
        assert!(is_leftover("My video.p1a2b3c4d.f137.mp4.zdpart", "My video"));
        assert!(is_leftover("My video.f137.mp4.zdpart", "My video"));
        assert_eq!(legacy_stream_part(Path::new("C:\\D"), "My video.mp4", "137", "mp4"), PathBuf::from("C:\\D\\My video.f137.mp4.zdpart"));
        // A normal download's unfinished file is not a video piece.
        for ext in ["pakopart", "zdpart"] {
            assert!(!is_leftover(&format!("Song.mp4.{ext}"), "Song"));
            assert!(!is_leftover(&format!("Song (2).f137.mp4.{ext}"), "Song"));
        }
    }

    /// A video left unfinished by an older Pako continues its pieces under the new name: the
    /// marked ".zdpart" one always, the unmarked one only when asked (it may be another's).
    #[test]
    fn old_pieces_are_adopted() {
        let dir = std::env::temp_dir().join(format!("pako-adopt-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = old_stream_part(&dir, "Song.mp4", "aaaa1111", "137", "mp4");
        std::fs::write(&old, b"video bytes").unwrap();
        let part = adopt_old_pieces(&dir, "Song.mp4", "aaaa1111", "137", "mp4", false);
        assert_eq!(part, stream_part(&dir, "Song.mp4", "aaaa1111", "137", "mp4"));
        assert_eq!(std::fs::read(&part).unwrap(), b"video bytes");
        assert!(!old.exists());

        let legacy = legacy_stream_part(&dir, "Song.mp4", "140", "m4a");
        std::fs::write(&legacy, b"sound bytes").unwrap();
        let part = adopt_old_pieces(&dir, "Song.mp4", "aaaa1111", "140", "m4a", false);
        assert!(!part.exists() && legacy.exists());
        let part = adopt_old_pieces(&dir, "Song.mp4", "aaaa1111", "140", "m4a", true);
        assert_eq!(std::fs::read(&part).unwrap(), b"sound bytes");
        assert!(!legacy.exists());

        let age = |p: &Path| {
            let f = std::fs::File::options().write(true).open(p).unwrap();
            f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600)).unwrap();
        };
        // Both names there: an older piece left by an older Pako goes, the newer one is kept.
        std::fs::write(&old, b"stale").unwrap();
        age(&old);
        let part = adopt_old_pieces(&dir, "Song.mp4", "aaaa1111", "137", "mp4", true);
        assert_eq!(std::fs::read(&part).unwrap(), b"video bytes");
        assert!(!old.exists());
        // An older Pako put back by a failed update wrote its piece last: that one is kept.
        age(&part);
        std::fs::write(&old, b"newer bytes").unwrap();
        let part = adopt_old_pieces(&dir, "Song.mp4", "aaaa1111", "137", "mp4", true);
        assert_eq!(std::fs::read(&part).unwrap(), b"newer bytes");
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn many_pieces_at_once_only_when_asked() {
        let t = Tools { yt_dlp: PathBuf::from("yt-dlp.exe"), deno: None, ffmpeg: None };
        let args = |n: u32| -> Vec<String> {
            download_command(&t, "https://x.com/v", 720, None, Path::new("C:\\D"), "V.mp4", n, None, None)
                .as_std()
                .get_args()
                .map(|a| a.to_string_lossy().to_string())
                .collect()
        };
        assert!(args(16).windows(2).any(|w| w == ["-N", "16"]));
        assert!(!args(0).iter().any(|a| a == "-N"));
        assert_eq!(format_args(0, Some("mp3")), vec!["-f".to_string(), "ba[ext=m4a]/ba/b".to_string()]);
        assert_eq!(format_args(0, None)[3], "height:1080,ext:mp4:m4a");
    }

    #[test]
    fn names() {
        assert_eq!(file_name("My video", None), "My video.mp4");
        assert!(file_name("Song", Some("mp3")).ends_with(".mp3"));
        assert!(file_name("Song", Some("m4a")).ends_with(".m4a"));
    }

    #[test]
    fn drm_only_from_its_own_words() {
        assert!(is_drm("error: [youtube] abc: this video is drm protected"));
        assert!(is_drm("error: [netflix] this site is known to use drm protection"));
        assert!(!is_drm("error: [generic] unable to download https://hdrmovies.example/v"));
        assert!(!is_drm("error: [youtube] xxdrmxx: video unavailable"));
        assert_ne!(friendly_error("ERROR: [youtube] xxDrMxx: Video unavailable").0, DRM);
    }

    fn cookie(domain: &str, name: &str, value: &str) -> crate::extension::SiteCookie {
        crate::extension::SiteCookie { domain: domain.into(), path: "/".into(), secure: true, expires: 1_900_000_000, name: name.into(), value: value.into() }
    }

    #[test]
    fn netscape_cookie_lines() {
        let text = netscape_cookies(&[
            cookie(".youtube.com", "SID", "abc"),
            crate::extension::SiteCookie { secure: false, expires: 0, path: String::new(), ..cookie("www.youtube.com", "PREF", "f1=1") },
            // A tab or line break would break the file: left out.
            cookie(".youtube.com", "BAD", "a\tb"),
            cookie(".youtube.com", "BAD2", "a\nb"),
        ]);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "# Netscape HTTP Cookie File");
        assert_eq!(lines[1], ".youtube.com\tTRUE\t/\tTRUE\t1900000000\tSID\tabc");
        assert_eq!(lines[2], "www.youtube.com\tFALSE\t/\tFALSE\t0\tPREF\tf1=1");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn cookie_file_is_removed_when_dropped() {
        let dir = std::env::temp_dir().join(format!("pako-cookie-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(cookie_file_in(&dir, &[]).unwrap().is_none());
        let jar = cookie_file_in(&dir, &[cookie(".instagram.com", "sessionid", "xyz")]).unwrap().unwrap();
        let path = jar.path().to_path_buf();
        assert!(std::fs::read_to_string(&path).unwrap().contains("sessionid\txyz"));
        assert!(LIVE_JARS.lock().unwrap().contains(&path));
        drop(jar);
        assert!(!path.exists());
        assert!(!LIVE_JARS.lock().unwrap().contains(&path));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Pako closing with a run still going: its cookie file is deleted all the same.
    #[test]
    fn cookie_files_are_removed_when_pako_closes() {
        let dir = std::env::temp_dir().join(format!("pako-cookie-close-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let jar = cookie_file_in(&dir, &[cookie(".x.com", "auth", "1")]).unwrap().unwrap();
        let path = jar.path().to_path_buf();
        std::mem::forget(jar); // a run that never ends
        // (Only this test's folder: other tests run at the same time.)
        remove_live_jars(|p| p.starts_with(&dir));
        assert!(!path.exists());
        assert!(!LIVE_JARS.lock().unwrap().contains(&path));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Leftovers from a crash go at the next start; other files in the folder stay.
    #[test]
    fn leftover_cookie_files_are_swept() {
        let dir = std::env::temp_dir().join(format!("pako-cookie-sweep-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("yt-cookies-old.txt"), "x").unwrap();
        std::fs::write(dir.join("settings.json"), "{}").unwrap();
        sweep_jars_in(&dir);
        assert!(!dir.join("yt-cookies-old.txt").exists());
        assert!(dir.join("settings.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn cookie_folder_is_private() {
        let dir = std::env::temp_dir().join(format!("pako-cookie-dir-{}", uuid::Uuid::new_v4().simple()));
        make_private_dir(&dir).unwrap();
        assert!(dir.is_dir());
        make_private_dir(&dir).unwrap(); // already there: fine
        let _ = std::fs::remove_dir_all(&dir);
    }
}
