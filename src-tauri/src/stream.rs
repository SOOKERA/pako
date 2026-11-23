// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! "The video playing on this page" (full extension only, never Pako Light): checking what the
//! extension sends, and keeping what "Record this video" sends until it is joined.
//!
//! - A caught stream (request kind "stream"): the stream's address (http or https), the page
//!   it played on (sent as the referrer, the browser's own way), the page's title (the file's
//!   name), and the browser's cookies for the stream's own server only.
//! - "Record this video" (kind "record"): the extension sends what the page's player is given,
//!   one track at a time (picture and sound apart), in numbered chunks small enough for the
//!   browser's message limit. Pako writes each track to a file; at the end the download joins
//!   them (see `Engine::record_end`). DRM videos are never recorded (the extension stops
//!   before; nothing here could open them anyway).

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::extension::{LIGHT_REFUSED, Request, SiteCookie};

const MAX_URL_LEN: usize = 8 * 1024;
const MAX_TITLE: usize = 300;

/// A web link, written the standard way; None if it isn't one.
fn web_link(u: &str) -> Option<String> {
    if u.len() > MAX_URL_LEN {
        return None;
    }
    let x = url::Url::parse(u.trim()).ok()?;
    (matches!(x.scheme(), "http" | "https") && x.host_str().is_some()).then(|| x.to_string()).filter(|s| s.len() <= MAX_URL_LEN)
}

/// The page as the browser sends it to the stream's server ("strict-origin-when-cross-origin"):
/// the whole address on the same site, "https://site/" to another site, nothing from an https
/// page to an http stream. Never the "#…" part or a login in the address.
pub fn referrer_for(page: &str, stream: &str) -> String {
    let (Ok(mut page), Ok(to)) = (url::Url::parse(page), url::Url::parse(stream)) else { return String::new() };
    if !matches!(page.scheme(), "http" | "https") || page.host_str().is_none() {
        return String::new();
    }
    if page.scheme() == "https" && to.scheme() != "https" {
        return String::new();
    }
    page.set_fragment(None);
    let _ = page.set_username("");
    let _ = page.set_password(None);
    if to.origin() != page.origin() {
        page.set_path("/");
        page.set_query(None);
    }
    page.to_string()
}

/// The page's title as a file name's start: one line, no control characters, not too long
/// (the file name itself is made safe for Windows later). Empty = "Video".
pub fn clean_title(title: &str) -> String {
    let t: String = title.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let t: String = t.chars().take(MAX_TITLE).collect();
    if t.is_empty() { "Video".into() } else { t }
}

/// Does a cookie go to `host` (the browser's rule: its own host, or the domain and its
/// subdomains for a ".domain" cookie)?
fn cookie_for_host(c: &SiteCookie, host: &str) -> bool {
    match c.domain.strip_prefix('.') {
        Some(bare) => bare.contains('.') && (host == bare || host.ends_with(&format!(".{bare}"))),
        None => host == c.domain,
    }
}

/// Only well-formed cookies the browser would send to the stream's own server.
pub fn stream_cookies(stream: &str, cookies: Vec<SiteCookie>) -> Vec<SiteCookie> {
    let Some(host) = url::Url::parse(stream).ok().and_then(|u| u.host_str().map(|h| h.trim_end_matches('.').to_ascii_lowercase())) else {
        return Vec::new();
    };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    crate::extension::clean_site_cookies(stream, cookies, now).into_iter().filter(|c| cookie_for_host(c, &host)).collect()
}

/// Checks a caught stream ("Download with Pako" in the list of videos on this page): exactly one
/// http(s) stream, the page as referrer, the title cleaned, the stream's own server's cookies.
/// Pako Light never sends one (and is refused if it does).
pub fn validate_stream(r: Request) -> Result<Request, String> {
    if r.light {
        return Err(LIGHT_REFUSED.into());
    }
    let [url] = r.urls.as_slice() else { return Err(tr!("Send one video at a time.", "Envoyez une vidéo à la fois.")) };
    let url = web_link(url).ok_or_else(|| tr!("Pako only takes http and https links.", "Pako n’accepte que les liens http et https."))?;
    let page = web_link(&r.referrer).unwrap_or_default();
    let referrer = referrer_for(&page, &url);
    // A private window's: no cookies (the extension sends none either), no trace in Pako.
    let site_cookies = if r.private { Vec::new() } else { stream_cookies(&url, r.site_cookies) };
    Ok(Request { kind: "stream".into(), urls: vec![url], referrer, filename: clean_title(&r.filename), site_cookies, private: r.private, ..Default::default() })

}

// ---------- "Record this video" ----------

/// What one "record" message carries (as the extension sends it).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RecordRequest {
    /// "start", "chunk", "end" or "cancel".
    pub action: String,
    /// Made by the extension for one recording.
    pub session: String,
    /// start: the page and its title.
    pub page: String,
    pub title: String,
    /// chunk: which of the player's tracks (0-3), its kind ("video/mp4; codecs=…"), its
    /// number in that track (from 0), and the data (base64).
    pub track: u32,
    pub mime: String,
    pub seq: u64,
    pub data: String,
}

/// A checked "record" message.
#[derive(Debug, PartialEq)]
pub enum Record {
    Start { session: String, page: String, title: String },
    Chunk { session: String, track: u32, mime: String, seq: u64, data: Vec<u8> },
    End { session: String },
    Cancel { session: String },
}

/// Largest chunk (well under the browser's message limit once in base64).
pub const MAX_CHUNK: usize = 512 * 1024;
/// Tracks one player may have (picture, sound, and a spare each).
pub const MAX_TRACKS: u32 = 4;
/// Largest recording, all tracks together.
const MAX_RECORDING: u64 = 32 * 1024 * 1024 * 1024;
/// Recordings at the same time.
const MAX_SESSIONS: usize = 3;
/// A recording nothing arrived for this long is dropped (the tab closed, the browser crashed).
const IDLE: Duration = Duration::from_secs(10 * 60);

fn clean_session(s: &str) -> Result<String, String> {
    let ok = (8..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    ok.then(|| s.to_ascii_lowercase()).ok_or_else(|| "Unknown recording.".into())
}

/// A media type as the player names it: `video/mp4; codecs="avc1.64001f, mp4a.40.2"`.
fn clean_mime(m: &str) -> Option<String> {
    let m = m.trim();
    let plain = !m.is_empty() && m.len() <= 200 && m.bytes().all(|b| b.is_ascii_alphanumeric() || b" /.+-_;=,\"".contains(&b));
    (plain && ext_of(m).is_some()).then(|| m.to_string())
}

/// The file kind a track's data makes (None = a kind Pako can't record).
pub fn ext_of(mime: &str) -> Option<&'static str> {
    let base = mime.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    Some(match base.as_str() {
        "video/mp4" => "mp4",
        "audio/mp4" => "m4a",
        "video/webm" | "audio/webm" => "webm",
        "video/mp2t" => "ts",
        "audio/mpeg" => "mp3",
        "audio/aac" => "aac",
        _ => return None,
    })
}

/// Picture and sound in a track, from its media type (a picture track whose codecs aren't
/// named may hold sound too: joining treats it as "maybe").
fn track_kind(mime: &str) -> (bool, bool) {
    let m = mime.to_ascii_lowercase();
    if m.starts_with("audio/") {
        return (false, true);
    }
    let codecs = m.split_once("codecs=").map(|(_, c)| c).unwrap_or("");
    let sound = codecs.is_empty() || ["mp4a", "opus", "vorbis", "ac-3", "ec-3", "flac", "mp3"].iter().any(|a| codecs.contains(a));
    (true, sound)
}

/// Checks a "record" message: a known action, a sensible recording id, and for chunks a track
/// number, a media type Pako can record, and data no bigger than `MAX_CHUNK`. Never from Pako
/// Light.
pub fn validate_record(r: RecordRequest, light: bool) -> Result<Record, String> {
    if light {
        return Err(LIGHT_REFUSED.into());
    }
    let session = clean_session(&r.session)?;
    match r.action.as_str() {
        "start" => {
            let page = web_link(&r.page).ok_or_else(|| tr!("Pako only takes http and https links.", "Pako n’accepte que les liens http et https."))?;
            Ok(Record::Start { session, page, title: clean_title(&r.title) })
        }
        "chunk" => {
            if r.track >= MAX_TRACKS {
                return Err("Unknown track.".into());
            }
            let mime = clean_mime(&r.mime).ok_or_else(|| tr!("Pako can't record this kind of video.", "Pako ne peut pas enregistrer ce type de vidéo."))?;
            if r.data.len() > MAX_CHUNK.div_ceil(3) * 4 + 4 {
                return Err(tr!("This part of the recording is too big.", "Cette partie de l’enregistrement est trop grande."));
            }
            let data = base64::engine::general_purpose::STANDARD.decode(r.data.as_bytes()).map_err(|_| tr!("This part of the recording is damaged.", "Cette partie de l’enregistrement est endommagée."))?;
            if data.is_empty() || data.len() > MAX_CHUNK {
                return Err(tr!("This part of the recording is damaged.", "Cette partie de l’enregistrement est endommagée."));
            }
            Ok(Record::Chunk { session, track: r.track, mime, seq: r.seq, data })
        }
        "end" => Ok(Record::End { session }),
        "cancel" => Ok(Record::Cancel { session }),
        _ => Err("Unknown request.".into()),
    }
}

/// One track of a recording on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordedTrack {
    /// File name in the recording's folder.
    pub file: String,
    pub ext: String,
    pub has_video: bool,
    pub has_audio: bool,
    /// The files that follow `file` when the player changed quality while recording (each
    /// starts with its own "init" part, so each is a whole video file on its own).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub more: Vec<String>,
    /// The largest picture size the parts name (0 = not known).
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

/// Parts of one track at most (a quality change each).
const MAX_PARTS: usize = 1000;
/// An init part kept for comparing (they are a few kB).
const INIT_MAX: usize = 1024 * 1024;

/// Does this chunk start an "init" part (what a player gives first, and again when the quality
/// changes)? MP4: an `ftyp` or `moov` box; WebM: the EBML header. Other kinds have none.
fn starts_init(ext: &str, data: &[u8]) -> bool {
    match ext {
        "mp4" | "m4a" => data.len() >= 8 && matches!(&data[4..8], b"ftyp" | b"moov"),
        "webm" => data.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
        _ => false,
    }
}

/// The largest picture size an MP4 init part names (its tracks' `tkhd` boxes end with it).
pub(crate) fn mp4_size(init: &[u8]) -> Option<(u32, u32)> {
    let mut best: Option<(u32, u32)> = None;
    let mut from = 4;
    while let Some(p) = init.get(from..).and_then(|rest| rest.windows(4).position(|w| w == b"tkhd")) {
        let at = from + p;
        let start = at - 4;
        let size = u32::from_be_bytes([init[start], init[start + 1], init[start + 2], init[start + 3]]) as usize;
        if size >= 92 && start + size <= init.len() {
            let end = start + size;
            let w = u32::from_be_bytes([init[end - 8], init[end - 7], init[end - 6], init[end - 5]]) >> 16;
            let h = u32::from_be_bytes([init[end - 4], init[end - 3], init[end - 2], init[end - 1]]) >> 16;
            if w > 0 && h > 0 && best.is_none_or(|(_, bh)| h > bh) {
                best = Some((w, h));
            }
        }
        from = at + 4;
    }
    best
}

/// A finished recording, ready to be joined.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    pub dir: PathBuf,
    pub page: String,
    pub title: String,
    pub tracks: Vec<RecordedTrack>,
}

struct TrackFile {
    mime: String,
    file: std::io::BufWriter<std::fs::File>,
    next: u64,
    /// The track's files: one per init part.
    files: Vec<String>,
    /// The current file's init part, and whether media followed it yet.
    init: Vec<u8>,
    media: bool,
    /// The same init sent again (the player re-sent it): how much of it matched so far.
    same_at: Option<usize>,
    bytes: u64,
    size: Option<(u32, u32)>,
}

struct Session {
    dir: PathBuf,
    page: String,
    title: String,
    tracks: HashMap<u32, TrackFile>,
    bytes: u64,
    last: Instant,
}

static SESSIONS: LazyLock<Mutex<HashMap<String, Session>>> = LazyLock::new(Default::default);

/// Where recordings are kept until joined: Windows' temporary folder for this user.
pub fn recordings_dir() -> PathBuf {
    let name = if cfg!(debug_assertions) { "Pako-dev" } else { "Pako" };
    std::env::temp_dir().join(name).join("recordings")
}

/// The list of tracks, written when a recording ends (a join after a restart reads it).
const TRACKS_FILE: &str = "tracks.json";

/// Drops recordings nothing arrived for in a while (their folders too).
fn drop_idle(map: &mut HashMap<String, Session>) {
    let idle: Vec<String> = map.iter().filter(|(_, s)| s.last.elapsed() > IDLE).map(|(k, _)| k.clone()).collect();
    for k in idle {
        if let Some(s) = map.remove(&k) {
            drop(s.tracks);
            let _ = std::fs::remove_dir_all(&s.dir);
        }
    }
}

/// A new recording (`root`: where its folder goes).
pub fn start_in(root: &Path, session: &str, page: &str, title: &str) -> Result<(), String> {
    let mut map = SESSIONS.lock().unwrap();
    drop_idle(&mut map);
    if map.contains_key(session) {
        return Ok(());
    }
    if map.len() >= MAX_SESSIONS {
        return Err(tr!("Pako is already recording other videos. Stop one first.", "Pako enregistre déjà d’autres vidéos. Arrêtez-en une d’abord."));
    }
    let dir = root.join(format!("rec-{session}"));
    std::fs::create_dir_all(&dir).map_err(|e| tr!("Couldn't start recording: {e}", "Impossible de commencer l’enregistrement : {e}"))?;
    map.insert(
        session.to_string(),
        Session { dir, page: page.to_string(), title: title.to_string(), tracks: HashMap::new(), bytes: 0, last: Instant::now() },
    );
    Ok(())
}

/// One chunk, in order. A chunk sent again (the extension retried) is ignored; a gap stops.
pub fn chunk(session: &str, track: u32, mime: &str, seq: u64, data: &[u8]) -> Result<(), String> {
    let mut map = SESSIONS.lock().unwrap();
    let s = map.get_mut(session).ok_or_else(|| tr!("This recording stopped. Start it again.", "Cet enregistrement s’est arrêté. Recommencez-le."))?;
    s.last = Instant::now();
    if s.bytes + data.len() as u64 > MAX_RECORDING {
        return Err(tr!("This recording is too big.", "Cet enregistrement est trop grand."));
    }
    if !s.tracks.contains_key(&track) {
        if seq != 0 {
            return Err(tr!("Part of the recording is missing. Start it again.", "Une partie de l’enregistrement manque. Recommencez-le."));
        }
        let ext = ext_of(mime).ok_or_else(|| tr!("Pako can't record this kind of video.", "Pako ne peut pas enregistrer ce type de vidéo."))?;
        let name = format!("t{track}.{ext}");
        let file = std::fs::File::create(s.dir.join(&name)).map_err(|e| tr!("Couldn't save the recording: {e}", "Impossible de sauvegarder l’enregistrement : {e}"))?;
        s.tracks.insert(
            track,
            TrackFile {
                mime: mime.to_string(),
                file: std::io::BufWriter::new(file),
                next: 0,
                files: vec![name],
                init: Vec::new(),
                media: false,
                same_at: None,
                bytes: 0,
                size: None,
            },
        );
    }
    let t = s.tracks.get_mut(&track).ok_or("Unknown track.")?;
    if ext_of(&t.mime) != ext_of(mime) {
        return Err(tr!("The video changed kind while recording. Start it again.", "La vidéo a changé de type pendant l’enregistrement. Recommencez-le."));
    }
    if seq < t.next {
        return Ok(());
    }
    if seq > t.next {
        return Err(tr!("Part of the recording is missing. Start it again.", "Une partie de l’enregistrement manque. Recommencez-le."));
    }
    let saving = |e: std::io::Error| tr!("Couldn't save the recording: {e}", "Impossible de sauvegarder l’enregistrement : {e}");
    let ext = ext_of(mime).unwrap_or("mp4");
    let init = starts_init(ext, data);
    if init && t.media {
        let off = t.same_at.unwrap_or(0);
        if t.init.get(off..).is_some_and(|rest| rest.starts_with(data)) {
            // The same init sent again: nothing changed, nothing to keep.
            t.same_at = Some(off + data.len());
            t.next += 1;
            return Ok(());
        }
        // Another quality: its own file (one file holds one init part; players read only the
        // first, and the picture after a change would be broken).
        if t.files.len() >= MAX_PARTS {
            return Err(tr!("The video changed quality too often while recording.", "La vidéo a changé de qualité trop souvent pendant l’enregistrement."));
        }
        // (What matched the old init so far, e.g. the same `ftyp` box sent on its own, belongs
        // to the new one too.)
        let matched = t.init[..off.min(t.init.len())].to_vec();
        t.file.flush().map_err(saving)?;
        let name = format!("t{track}-{}.{ext}", t.files.len());
        let file = std::fs::File::create(s.dir.join(&name)).map_err(saving)?;
        t.file = std::io::BufWriter::new(file);
        t.files.push(name);
        t.file.write_all(&matched).map_err(saving)?;
        t.init = matched;
        t.media = false;
        t.same_at = None;
    }
    t.file.write_all(data).map_err(saving)?;
    if init {
        if t.init.len() < INIT_MAX {
            t.init.extend_from_slice(data);
        }
        if let Some((w, h)) = mp4_size(&t.init).filter(|&(_, h)| ext != "webm" && t.size.is_none_or(|(_, bh)| h > bh)) {
            t.size = Some((w, h));
        }
    } else {
        t.media = true;
        t.same_at = None;
    }
    t.next += 1;
    t.bytes += data.len() as u64;
    s.bytes += data.len() as u64;
    Ok(())
}

/// The recording is complete: its files are closed and listed, ready to be joined.
pub fn end(session: &str) -> Result<Recorded, String> {
    let s = SESSIONS.lock().unwrap().remove(session).ok_or_else(|| tr!("This recording stopped. Start it again.", "Cet enregistrement s’est arrêté. Recommencez-le."))?;
    let dir = s.dir.clone();
    let r = finish(s);
    if r.is_err() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    r
}

/// The tracks to join: the page may have had other players too (an advert before the video,
/// a preview): the picture track that got the most data, and the sound track that did (unless
/// the picture track names its own sound). The others' files go.
fn finish(s: Session) -> Result<Recorded, String> {
    let mut all: Vec<(u32, TrackFile)> = s.tracks.into_iter().collect();
    all.sort_unstable_by_key(|(id, _)| *id);
    for (_, t) in all.iter_mut() {
        t.file.flush().map_err(|e| tr!("Couldn't save the recording: {e}", "Impossible de sauvegarder l’enregistrement : {e}"))?;
    }
    let best = |video: bool| all.iter().filter(|(_, t)| track_kind(&t.mime).0 == video && t.bytes > 0).max_by_key(|(_, t)| t.bytes).map(|(id, _)| *id);
    let video = best(true);
    let own_sound = video.and_then(|v| all.iter().find(|(id, _)| *id == v)).is_some_and(|(_, t)| names_sound(&t.mime));
    let sound = if own_sound { None } else { best(false) };
    let mut tracks = Vec::new();
    for (id, t) in all {
        if Some(id) != video && Some(id) != sound {
            drop(t.file);
            for f in &t.files {
                let _ = std::fs::remove_file(s.dir.join(f));
            }
            continue;
        }
        let ext = ext_of(&t.mime).unwrap_or("mp4");
        let (has_video, has_audio) = track_kind(&t.mime);
        let (width, height) = t.size.unwrap_or((0, 0));
        let mut files = t.files.into_iter();
        let file = files.next().unwrap_or_else(|| format!("t{id}.{ext}"));
        tracks.push(RecordedTrack { file, ext: ext.into(), has_video, has_audio, more: files.collect(), width, height });
    }
    if tracks.is_empty() {
        return Err(tr!("Nothing was recorded: the video didn't play.", "Rien n’a été enregistré : la vidéo ne s’est pas lue."));
    }
    std::fs::write(s.dir.join(TRACKS_FILE), serde_json::to_vec(&tracks).unwrap_or_default()).map_err(|e| tr!("Couldn't save the recording: {e}", "Impossible de sauvegarder l’enregistrement : {e}"))?;
    Ok(Recorded { dir: s.dir, page: s.page, title: s.title, tracks })
}

/// A picture track whose media type names a sound codec (picture and sound in one).
fn names_sound(mime: &str) -> bool {
    let m = mime.to_ascii_lowercase();
    m.starts_with("video/") && m.split_once("codecs=").is_some_and(|(_, c)| ["mp4a", "opus", "vorbis", "ac-3", "ec-3", "flac", "mp3"].iter().any(|a| c.contains(a)))
}

/// ffmpeg's arguments to make one file of a track that comes in several parts, each with its
/// own init part (the player changed quality while recording, or an HLS stream's quality
/// changes part-way): the parts one after the other (`list`: ffmpeg's list of them, next to
/// them), made again at the largest size they had (`size`, 0 = the first part's), since copied
/// as they are they would show a broken picture after the first change. Picture: H.264; sound: AAC.
pub fn rejoin_args(list: &Path, has_video: bool, has_audio: bool, size: (u32, u32), out: &Path) -> Vec<std::ffi::OsString> {
    use std::ffi::OsString;
    let mut a: Vec<OsString> = ["-hide_banner", "-nostdin", "-loglevel", "error", "-y", "-f", "concat", "-i"].iter().map(OsString::from).collect();
    a.push(list.as_os_str().to_owned());
    if has_video {
        a.extend(["-map", "0:v:0"].iter().map(OsString::from));
        if size.0 > 0 && size.1 > 0 {
            let (w, h) = (size.0.div_ceil(2) * 2, size.1.div_ceil(2) * 2);
            a.push("-vf".into());
            a.push(format!("scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1").into());
        }
        a.extend(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"].iter().map(OsString::from));
        if has_audio {
            a.extend(["-map", "0:a:0?", "-c:a", "aac", "-b:a", "192k"].iter().map(OsString::from));
        }
    } else {
        a.extend(["-map", "0:a:0", "-vn", "-c:a", "aac", "-b:a", "192k"].iter().map(OsString::from));
    }
    a.extend(["-movflags", "+faststart", "-f", "mp4"].iter().map(OsString::from));
    a.push(out.as_os_str().to_owned());
    a
}

/// ffmpeg's list of a track's parts (plain file names, Pako's own: they are next to the list).
pub fn parts_list<'a>(files: impl IntoIterator<Item = &'a str>) -> String {
    files.into_iter().map(|f| format!("file '{f}'\n")).collect()
}

/// Stops a recording: what it saved is deleted.
pub fn cancel(session: &str) {
    let s = SESSIONS.lock().unwrap().remove(session);
    if let Some(s) = s {
        drop(s.tracks);
        let _ = std::fs::remove_dir_all(&s.dir);
    }
}

/// The tracks of a finished recording's folder (for joining it, also after a restart).
pub fn recorded_tracks(dir: &Path) -> Option<Vec<RecordedTrack>> {
    let list: Vec<RecordedTrack> = serde_json::from_slice(&std::fs::read(dir.join(TRACKS_FILE)).ok()?).ok()?;
    // Only plain file names in that folder.
    let plain = |f: &str| !f.is_empty() && !f.starts_with('.') && f.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
    (!list.is_empty() && list.iter().all(|t| plain(&t.file) && t.more.iter().all(|f| plain(f)))).then_some(list)
}

/// At start: recordings left by a Pako that closed meanwhile, which no download uses, go.
pub fn sweep(root: &Path, in_use: &[PathBuf]) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    let live: Vec<PathBuf> = SESSIONS.lock().unwrap().values().map(|s| s.dir.clone()).collect();
    for e in rd.flatten() {
        let p = e.path();
        let ours = e.file_name().to_string_lossy().starts_with("rec-");
        if ours && p.is_dir() && !in_use.contains(&p) && !live.contains(&p) {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(urls: &[&str], page: &str, title: &str, cookies: Vec<SiteCookie>) -> Request {
        Request {
            kind: "stream".into(),
            urls: urls.iter().map(|u| u.to_string()).collect(),
            referrer: page.into(),
            filename: title.into(),
            site_cookies: cookies,
            ..Default::default()
        }
    }

    fn cookie(domain: &str, name: &str) -> SiteCookie {
        SiteCookie { domain: domain.into(), path: "/".into(), name: name.into(), value: "v".into(), ..Default::default() }
    }

    #[test]
    fn checks_a_caught_stream() {
        let r = validate_stream(req(
            &["https://cdn.example.net/hls/master.m3u8?t=1"],
            "https://watch.example.org/v/12?x=1#t=30",
            "  A  title\nwith\tbreaks ",
            vec![cookie("cdn.example.net", "a"), cookie(".example.net", "b"), cookie("www.example.net", "c"), cookie("watch.example.org", "d")],
        ))
        .unwrap();
        assert_eq!(r.kind, "stream");
        assert_eq!(r.urls, vec!["https://cdn.example.net/hls/master.m3u8?t=1"]);
        // Another site: only "https://site/" goes along.
        assert_eq!(r.referrer, "https://watch.example.org/");
        assert_eq!(r.filename, "A title with breaks");
        let names: Vec<&str> = r.site_cookies.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
        // The same site: the whole page, never its "#…" part.
        let r = validate_stream(req(&["https://e.example/v.m3u8"], "https://e.example/watch?id=3#x", "", vec![])).unwrap();
        assert_eq!(r.referrer, "https://e.example/watch?id=3");
        assert_eq!(r.filename, "Video");
        // An https page sends nothing to an http stream.
        assert_eq!(validate_stream(req(&["http://e.example/v.m3u8"], "https://e.example/", "", vec![])).unwrap().referrer, "");
    }

    #[test]
    fn refuses_bad_streams_and_pako_light() {
        assert!(validate_stream(req(&["file:///C:/x.m3u8"], "", "", vec![])).is_err());
        assert!(validate_stream(req(&["blob:https://e.example/1"], "", "", vec![])).is_err());
        assert!(validate_stream(req(&["https://a.example/1", "https://a.example/2"], "", "", vec![])).is_err());
        assert!(validate_stream(req(&[], "", "", vec![])).is_err());
        let mut light = req(&["https://e.example/v.m3u8"], "https://e.example/", "", vec![]);
        light.light = true;
        assert_eq!(validate_stream(light), Err(LIGHT_REFUSED.to_string()));
        // A page that isn't a web page isn't sent at all.
        assert_eq!(validate_stream(req(&["https://e.example/v.m3u8"], "javascript:alert(1)", "", vec![])).unwrap().referrer, "");
    }

    fn rec(action: &str) -> RecordRequest {
        RecordRequest { action: action.into(), session: "abcd1234-ef".into(), ..Default::default() }
    }

    #[test]
    fn checks_record_messages() {
        let mut start = rec("start");
        start.page = "https://e.example/watch".into();
        start.title = "My video".into();
        assert_eq!(
            validate_record(start.clone(), false),
            Ok(Record::Start { session: "abcd1234-ef".into(), page: "https://e.example/watch".into(), title: "My video".into() })
        );
        assert_eq!(validate_record(start, true), Err(LIGHT_REFUSED.to_string()));
        let mut c = rec("chunk");
        c.mime = r#"video/mp4; codecs="avc1.64001f""#.into();
        c.data = base64::engine::general_purpose::STANDARD.encode([1u8, 2, 3]);
        c.seq = 4;
        assert!(matches!(validate_record(c.clone(), false), Ok(Record::Chunk { seq: 4, ref data, .. }) if data == &[1, 2, 3]));
        // Too big, damaged, an unknown kind or track, a bad recording id: refused.
        let mut big = c.clone();
        big.data = base64::engine::general_purpose::STANDARD.encode(vec![0u8; MAX_CHUNK + 1]);
        assert!(validate_record(big, false).is_err());
        let mut bad = c.clone();
        bad.data = "%%%".into();
        assert!(validate_record(bad, false).is_err());
        let mut kind = c.clone();
        kind.mime = "application/x-mpegurl".into();
        assert!(validate_record(kind, false).is_err());
        let mut track = c.clone();
        track.track = MAX_TRACKS;
        assert!(validate_record(track, false).is_err());
        let mut id = c;
        id.session = "../../x".into();
        assert!(validate_record(id, false).is_err());
        assert!(validate_record(rec("explode"), false).is_err());
        assert_eq!(validate_record(rec("cancel"), false), Ok(Record::Cancel { session: "abcd1234-ef".into() }));
    }

    #[test]
    fn records_tracks_in_order() {
        let root = std::env::temp_dir().join(format!("pako-rec-test-{}", uuid::Uuid::new_v4().simple()));
        let id = format!("t{}", uuid::Uuid::new_v4().simple());
        start_in(&root, &id, "https://e.example/w", "Clip").unwrap();
        let v = r#"video/mp4; codecs="avc1.64001f""#;
        let a = r#"audio/mp4; codecs="mp4a.40.2""#;
        chunk(&id, 0, v, 0, b"init").unwrap();
        chunk(&id, 1, a, 0, b"ai").unwrap();
        chunk(&id, 0, v, 1, b"-one").unwrap();
        // Sent again: ignored. A gap or another kind: refused.
        chunk(&id, 0, v, 1, b"-one").unwrap();
        assert!(chunk(&id, 0, v, 3, b"x").is_err());
        assert!(chunk(&id, 0, "video/webm", 2, b"x").is_err());
        assert!(chunk(&id, 2, v, 5, b"x").is_err());
        let done = end(&id).unwrap();
        assert_eq!(done.title, "Clip");
        assert_eq!(done.tracks.len(), 2);
        let plain = |file: &str, ext: &str, has_video, has_audio| RecordedTrack {
            file: file.into(),
            ext: ext.into(),
            has_video,
            has_audio,
            more: Vec::new(),
            width: 0,
            height: 0,
        };
        assert_eq!(done.tracks[0], plain("t0.mp4", "mp4", true, false));
        assert_eq!(done.tracks[1], plain("t1.m4a", "m4a", false, true));
        assert_eq!(std::fs::read(done.dir.join("t0.mp4")).unwrap(), b"init-one");
        assert_eq!(recorded_tracks(&done.dir), Some(done.tracks.clone()));
        // Ended: no more chunks.
        assert!(chunk(&id, 0, v, 2, b"x").is_err());
        // Swept at start unless a download still uses it.
        sweep(&root, std::slice::from_ref(&done.dir));
        assert!(done.dir.exists());
        sweep(&root, &[]);
        assert!(!done.dir.exists());

        // Cancelled: its folder goes.
        let id2 = format!("c{}", uuid::Uuid::new_v4().simple());
        start_in(&root, &id2, "https://e.example/w", "Clip").unwrap();
        chunk(&id2, 0, v, 0, b"x").unwrap();
        cancel(&id2);
        assert!(!root.join(format!("rec-{id2}")).exists());
        // Nothing played: nothing to join.
        let id3 = format!("e{}", uuid::Uuid::new_v4().simple());
        start_in(&root, &id3, "https://e.example/w", "Clip").unwrap();
        assert!(end(&id3).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// An MP4 init part: `ftyp`, then a `moov` with one track of `w`×`h` (a `tkhd`, version 0).
    fn mp4_init(w: u32, h: u32) -> Vec<u8> {
        let mut tkhd = vec![0u8; 92];
        tkhd[0..4].copy_from_slice(&92u32.to_be_bytes());
        tkhd[4..8].copy_from_slice(b"tkhd");
        tkhd[84..88].copy_from_slice(&(w << 16).to_be_bytes());
        tkhd[88..92].copy_from_slice(&(h << 16).to_be_bytes());
        let mut moov = ((8 + tkhd.len()) as u32).to_be_bytes().to_vec();
        moov.extend(b"moov");
        moov.extend(tkhd);
        let mut v = 16u32.to_be_bytes().to_vec();
        v.extend(b"ftypisom\0\0\0\0");
        v.extend(moov);
        v
    }

    #[test]
    fn a_quality_change_starts_another_file_and_adverts_are_left_out() {
        let root = std::env::temp_dir().join(format!("pako-rec-test-{}", uuid::Uuid::new_v4().simple()));
        let id = format!("q{}", uuid::Uuid::new_v4().simple());
        start_in(&root, &id, "https://e.example/w", "Clip").unwrap();
        let v = r#"video/mp4; codecs="avc1.64001f""#;
        let a = r#"audio/mp4; codecs="mp4a.40.2""#;
        // Track 0/1: an advert's picture and sound (small). Track 2/3: the video.
        chunk(&id, 0, v, 0, &mp4_init(320, 180)).unwrap();
        chunk(&id, 0, v, 1, b"ad").unwrap();
        chunk(&id, 1, a, 0, b"ad sound").unwrap();
        let low = mp4_init(640, 360);
        let high = mp4_init(1280, 720);
        let mut seq = 0;
        let mut send = |data: &[u8]| {
            chunk(&id, 2, v, seq, data).unwrap();
            seq += 1;
        };
        send(&low);
        send(b"low media 1");
        // The same init again: nothing new.
        send(&low);
        send(b"low media 2");
        // Another quality: another file, starting with its init.
        send(&high[..16]);
        send(&high[16..]);
        send(b"high media");
        chunk(&id, 3, a, 0, b"the video's own sound, longer").unwrap();
        let done = end(&id).unwrap();
        assert_eq!(done.tracks.len(), 2, "{:?}", done.tracks);
        let t = &done.tracks[0];
        assert_eq!((t.file.as_str(), t.more.clone(), t.width, t.height), ("t2.mp4", vec!["t2-1.mp4".to_string()], 1280, 720));
        let mut first = low.clone();
        first.extend(b"low media 1low media 2");
        assert_eq!(std::fs::read(done.dir.join("t2.mp4")).unwrap(), first);
        let mut second = high.clone();
        second.extend(b"high media");
        assert_eq!(std::fs::read(done.dir.join("t2-1.mp4")).unwrap(), second);
        assert_eq!(done.tracks[1].file, "t3.m4a");
        // The advert's files are gone; the list is read back as it was written.
        assert!(!done.dir.join("t0.mp4").exists() && !done.dir.join("t1.m4a").exists());
        assert_eq!(recorded_tracks(&done.dir), Some(done.tracks.clone()));
        assert_eq!(parts_list(std::iter::once(t.file.as_str()).chain(t.more.iter().map(String::as_str))), "file 't2.mp4'\nfile 't2-1.mp4'\n");
        let args: Vec<String> =
            rejoin_args(&done.dir.join("t2.txt"), t.has_video, t.has_audio, (t.width, t.height), Path::new("t2-all.mp4")).iter().map(|s| s.to_string_lossy().into()).collect();
        assert!(args.windows(2).any(|w| w == ["-f", "concat"]));
        assert!(args.iter().any(|s| s.starts_with("scale=1280:720")));
        assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reads_the_picture_size_of_an_init_part() {
        assert_eq!(mp4_size(&mp4_init(1920, 1080)), Some((1920, 1080)));
        assert_eq!(mp4_size(b"\0\0\0\x08ftyp"), None);
        assert!(starts_init("mp4", &mp4_init(1, 1)) && !starts_init("mp4", b"\0\0\0\x08moof") && !starts_init("ts", &mp4_init(1, 1)));
        assert!(starts_init("webm", &[0x1A, 0x45, 0xDF, 0xA3, 1]));
        // A picture track naming its sound: no separate sound track is joined to it.
        assert!(names_sound(r#"video/mp4; codecs="avc1.4d401f,mp4a.40.2""#) && !names_sound("video/mp4") && !names_sound("audio/mp4; codecs=mp4a"));
    }

    #[test]
    fn track_kinds() {
        assert_eq!(track_kind(r#"video/mp4; codecs="avc1.4d401f,mp4a.40.2""#), (true, true));
        assert_eq!(track_kind(r#"video/webm; codecs="vp9""#), (true, false));
        assert_eq!(track_kind("video/mp4"), (true, true));
        assert_eq!(track_kind("audio/webm; codecs=opus"), (false, true));
    }
}
