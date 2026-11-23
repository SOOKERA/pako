// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Pako's own HLS download: the last try for a stream caught on a page when yt-dlp can't get it.
//!
//! It reads the playlist (the quality from Settings when there are several), fetches the
//! pieces with the page as referrer and the browser's cookies for the stream's own server, a
//! few at a time with retries, opens standard AES-128 pieces, drops anything a server puts in
//! front of an MPEG-TS piece, and joins each track into one file (ffmpeg then makes the video).
//! Each piece is saved on its own as it arrives, so a pause or a restart continues from them.
//!
//! Nothing here gets around DRM: a playlist that uses it (SAMPLE-AES, or a key system other
//! than plain AES-128) stops with "protected", before anything is downloaded.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use aes::cipher::{BlockDecrypt, KeyInit, generic_array::GenericArray};
use futures_util::stream::{self, StreamExt};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::extension::SiteCookie;

/// What the row says for a stream that uses DRM.
pub fn protected() -> String {
    tr!("This video is protected, so it can't be downloaded.", "Cette vidéo est protégée : elle ne peut donc pas être téléchargée.")
}
/// What the row says when nothing worked.
pub fn couldnt() -> String {
    tr!("Pako couldn't get this video.", "Pako n’a pas pu récupérer cette vidéo.")
}
/// A live stream (the playlist never ends).
pub fn live() -> String {
    tr!("This is a live stream: Pako can't save it.", "C’est un direct : Pako ne peut pas l’enregistrer.")
}

/// Pieces fetched at the same time.
const AT_ONCE: usize = 4;
/// Tries for one piece before the download stops (it continues from there on Try again).
const TRIES: u32 = 5;
const MAX_PLAYLIST: usize = 8 * 1024 * 1024;
const MAX_PIECE: usize = 256 * 1024 * 1024;
const MAX_SEGMENTS: usize = 200_000;

/// (The details in `Net` and `Refused` are for tests and debugging: the row says it plainly.)
#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    /// DRM: never downloaded.
    Protected,
    /// A live stream.
    Live,
    /// Not an HLS playlist (or one Pako can't read).
    NotHls,
    /// The connection or the server failed several times in a row: worth waiting and going on.
    Net(String),
    /// The server refused (403, 404...).
    Refused(String),
    Disk(std::io::Error),
    Cancelled,
}

impl Error {
    /// The row's words.
    pub fn message(&self) -> String {
        match self {
            Error::Protected => protected(),
            Error::Live => live(),
            Error::Disk(e) => tr!("Couldn't save the video: {e}", "Impossible d’enregistrer la vidéo : {e}"),
            _ => couldnt(),
        }
    }
}

// ---------- playlists ----------

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub uri: String,
    pub bandwidth: u64,
    pub height: Option<u32>,
    pub audio_group: Option<String>,
    pub codecs: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rendition {
    pub group: String,
    pub uri: Option<String>,
    pub default: bool,
    pub autoselect: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Master {
    pub variants: Vec<Variant>,
    pub audio: Vec<Rendition>,
    /// An `#EXT-X-SESSION-KEY` that uses DRM.
    pub protected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub uri: String,
    pub iv: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitSection {
    pub uri: String,
    /// (length, offset)
    pub range: Option<(u64, u64)>,
    pub key: Option<Key>,
    /// The media sequence number of the first piece it belongs to (the IV when none is given).
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub uri: String,
    pub range: Option<(u64, u64)>,
    pub key: Option<Key>,
    pub seq: u64,
    /// Index in `Media::maps`.
    pub map: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Media {
    pub segments: Vec<Segment>,
    pub maps: Vec<InitSection>,
    pub ended: bool,
    /// A key that uses DRM.
    pub protected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Playlist {
    Master(Master),
    Media(Media),
}

/// `KEY=VALUE,KEY="quoted, value"` into a map (keys in capitals as the standard writes them).
pub fn attributes(s: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut rest = s.trim();
    while !rest.is_empty() {
        let Some(eq) = rest.find('=') else { break };
        let key = rest[..eq].trim().trim_start_matches(',').trim().to_ascii_uppercase();
        rest = &rest[eq + 1..];
        let value;
        if let Some(q) = rest.strip_prefix('"') {
            let end = q.find('"').unwrap_or(q.len());
            value = q[..end].to_string();
            rest = q.get(end + 1..).unwrap_or("");
            rest = rest.trim_start().strip_prefix(',').unwrap_or(rest);
        } else {
            let end = rest.find(',').unwrap_or(rest.len());
            value = rest[..end].trim().to_string();
            rest = rest.get(end + 1..).unwrap_or("");
        }
        if !key.is_empty() {
            out.insert(key, value);
        }
    }
    out
}

fn resolve(base: &Url, uri: &str) -> Option<String> {
    let u = base.join(uri.trim()).ok()?;
    matches!(u.scheme(), "http" | "https").then(|| u.to_string())
}

/// "1000@200" (length, then offset) or "1000" (right after the previous range). A range whose
/// end doesn't fit in a number (a playlist made to break the sums that follow) isn't one.
fn byte_range(s: &str, next: u64) -> Option<(u64, u64)> {
    let (len, off): (u64, u64) = match s.trim().split_once('@') {
        Some((l, o)) => (l.trim().parse().ok()?, o.trim().parse().ok()?),
        None => (s.trim().parse().ok()?, next),
    };
    (len > 0 && off.checked_add(len).is_some()).then_some((len, off))
}

fn parse_iv(s: &str) -> Option<[u8; 16]> {
    let hex = s.trim().strip_prefix("0x").or_else(|| s.trim().strip_prefix("0X"))?;
    if hex.is_empty() || hex.len() > 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let padded = format!("{hex:0>32}");
    let mut iv = [0u8; 16];
    for (i, b) in iv.iter_mut().enumerate() {
        *b = u8::from_str_radix(&padded[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(iv)
}

/// What an `#EXT-X-KEY` means: Ok(None) = no encryption, Ok(Some) = standard AES-128, Err = DRM
/// (SAMPLE-AES, or a key system other than the plain key file).
fn key_of(a: &HashMap<String, String>, base: &Url) -> Result<Option<Key>, Error> {
    let method = a.get("METHOD").map(|m| m.to_ascii_uppercase()).unwrap_or_default();
    if method == "NONE" {
        return Ok(None);
    }
    let format = a.get("KEYFORMAT").map(|f| f.trim().to_ascii_lowercase()).unwrap_or_else(|| "identity".into());
    if method != "AES-128" || format != "identity" {
        return Err(Error::Protected);
    }
    let uri = a.get("URI").and_then(|u| resolve(base, u)).ok_or(Error::NotHls)?;
    Ok(Some(Key { uri, iv: a.get("IV").and_then(|v| parse_iv(v)) }))
}

/// Reads a playlist (`base`: its own address, for the links in it).
pub fn parse(text: &str, base: &Url) -> Result<Playlist, Error> {
    let mut lines = text.trim_start_matches('\u{feff}').lines().map(str::trim).filter(|l| !l.is_empty()).peekable();
    if lines.next() != Some("#EXTM3U") {
        return Err(Error::NotHls);
    }
    let lines: Vec<&str> = lines.collect();
    if lines.iter().any(|l| l.starts_with("#EXT-X-STREAM-INF")) {
        let mut m = Master::default();
        let mut pending: Option<HashMap<String, String>> = None;
        for l in lines {
            if let Some(a) = l.strip_prefix("#EXT-X-STREAM-INF:") {
                pending = Some(attributes(a));
            } else if let Some(a) = l.strip_prefix("#EXT-X-MEDIA:") {
                let a = attributes(a);
                if a.get("TYPE").is_some_and(|t| t.eq_ignore_ascii_case("AUDIO")) {
                    m.audio.push(Rendition {
                        group: a.get("GROUP-ID").cloned().unwrap_or_default(),
                        uri: a.get("URI").and_then(|u| resolve(base, u)),
                        default: a.get("DEFAULT").is_some_and(|v| v.eq_ignore_ascii_case("YES")),
                        autoselect: a.get("AUTOSELECT").is_some_and(|v| v.eq_ignore_ascii_case("YES")),
                    });
                }
            } else if let Some(a) = l.strip_prefix("#EXT-X-SESSION-KEY:") {
                if matches!(key_of(&attributes(a), base), Err(Error::Protected)) {
                    m.protected = true;
                }
            } else if !l.starts_with('#') {
                if let (Some(a), Some(uri)) = (pending.take(), resolve(base, l)) {
                    let height = a.get("RESOLUTION").and_then(|r| r.split_once(['x', 'X'])).and_then(|(_, h)| h.trim().parse().ok());
                    m.variants.push(Variant {
                        uri,
                        bandwidth: a.get("BANDWIDTH").and_then(|b| b.parse().ok()).unwrap_or(0),
                        height,
                        audio_group: a.get("AUDIO").cloned(),
                        codecs: a.get("CODECS").cloned(),
                    });
                }
            }
        }
        if m.variants.is_empty() {
            return Err(Error::NotHls);
        }
        return Ok(Playlist::Master(m));
    }
    let mut media = Media::default();
    let mut seq: u64 = 0;
    let mut key: Option<Key> = None;
    let mut map: Option<usize> = None;
    let mut range: Option<(u64, u64)> = None;
    let mut next_offset: u64 = 0;
    let mut in_segment = false;
    for l in lines {
        if let Some(n) = l.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            seq = n.trim().parse().unwrap_or(0);
        } else if l.starts_with("#EXTINF") {
            in_segment = true;
        } else if let Some(r) = l.strip_prefix("#EXT-X-BYTERANGE:") {
            range = byte_range(r, next_offset);
        } else if let Some(a) = l.strip_prefix("#EXT-X-KEY:") {
            match key_of(&attributes(a), base) {
                Ok(k) => key = k,
                Err(Error::Protected) => media.protected = true,
                Err(e) => return Err(e),
            }
        } else if let Some(a) = l.strip_prefix("#EXT-X-MAP:") {
            let a = attributes(a);
            let uri = a.get("URI").and_then(|u| resolve(base, u)).ok_or(Error::NotHls)?;
            let range = a.get("BYTERANGE").and_then(|r| byte_range(r, 0));
            media.maps.push(InitSection { uri, range, key: key.clone(), seq });
            map = Some(media.maps.len() - 1);
        } else if l == "#EXT-X-ENDLIST" || l.eq_ignore_ascii_case("#EXT-X-PLAYLIST-TYPE:VOD") {
            media.ended = true;
        } else if !l.starts_with('#') {
            if !in_segment {
                continue;
            }
            in_segment = false;
            let uri = resolve(base, l).ok_or(Error::NotHls)?;
            if let Some((len, off)) = range {
                next_offset = off + len;
            }
            media.segments.push(Segment { uri, range: range.take(), key: key.clone(), seq, map });
            seq += 1;
            if media.segments.len() > MAX_SEGMENTS {
                return Err(Error::NotHls);
            }
        }
    }
    if media.protected {
        return Err(Error::Protected);
    }
    if media.segments.is_empty() {
        return Err(Error::NotHls);
    }
    Ok(Playlist::Media(media))
}

/// A video codec in a variant's CODECS (a variant without one is sound only).
fn has_video_codec(codecs: &str) -> bool {
    let c = codecs.to_ascii_lowercase();
    ["avc", "hvc", "hev", "vp08", "vp09", "vp8", "vp9", "av01", "dvh", "dva", "mp4v"].iter().any(|v| c.contains(v))
}

/// What to download from a master playlist.
#[derive(Debug, PartialEq)]
pub struct Picked {
    pub video: Option<String>,
    pub audio: Option<String>,
}

/// The variant for the quality (the highest at most that tall, else the smallest one; the
/// best one when the playlist doesn't say heights), and its separate sound when it has one.
/// Sound only: a sound rendition when there is one, else the smallest variant.
pub fn pick(m: &Master, quality: u32, audio_only: bool) -> Option<Picked> {
    let rendition = |group: Option<&str>| -> Option<String> {
        let list: Vec<&Rendition> = m.audio.iter().filter(|r| group.is_none_or(|g| r.group == g)).collect();
        let r = list.iter().find(|r| r.default).or_else(|| list.iter().find(|r| r.autoselect)).or(list.first())?;
        r.uri.clone()
    };
    if audio_only {
        if let Some(a) = rendition(None) {
            return Some(Picked { video: None, audio: Some(a) });
        }
        let v = m.variants.iter().min_by_key(|v| v.bandwidth)?;
        return Some(Picked { video: Some(v.uri.clone()), audio: None });
    }
    let with_video: Vec<&Variant> = m.variants.iter().filter(|v| v.codecs.as_deref().is_none_or(has_video_codec)).collect();
    let pool = if with_video.is_empty() { m.variants.iter().collect() } else { with_video };
    let want = if quality == 0 { 1080 } else { quality };
    let tall = pool.iter().filter(|v| v.height.is_some());
    let chosen = if pool.iter().any(|v| v.height.is_some()) {
        tall.clone()
            .filter(|v| v.height.unwrap_or(0) <= want)
            .max_by_key(|v| (v.height, v.bandwidth))
            .or_else(|| tall.min_by_key(|v| (v.height, v.bandwidth)))
    } else {
        pool.iter().max_by_key(|v| v.bandwidth)
    }?;
    let audio = chosen.audio_group.as_deref().and_then(|g| rendition(Some(g)));
    Some(Picked { video: Some(chosen.uri.clone()), audio })
}

// ---------- pieces ----------

/// Opens a standard AES-128 piece (CBC, PKCS#7 padding). A length that isn't whole blocks
/// isn't an AES-128 piece.
pub fn decrypt(data: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Option<Vec<u8>> {
    if data.is_empty() || !data.len().is_multiple_of(16) {
        return None;
    }
    let cipher = aes::Aes128::new(GenericArray::from_slice(key));
    let mut out = Vec::with_capacity(data.len());
    let mut prev = *iv;
    for chunk in data.chunks_exact(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        for (b, p) in block.iter_mut().zip(prev.iter()) {
            *b ^= p;
        }
        out.extend_from_slice(&block);
        prev.copy_from_slice(chunk);
    }
    let pad = *out.last()? as usize;
    if (1..=16).contains(&pad) && out[out.len() - pad..].iter().all(|&b| b as usize == pad) {
        out.truncate(out.len() - pad);
    }
    Some(out)
}

/// The IV of a piece: the playlist's own, else its media sequence number (big-endian).
pub fn iv_for(key: &Key, seq: u64) -> [u8; 16] {
    key.iv.unwrap_or_else(|| (seq as u128).to_be_bytes())
}

const TS_PACKET: usize = 188;

/// Drops whatever comes before the MPEG-TS data in a piece (some servers put a picture or other
/// bytes in front): the start is where the sync byte 0x47 repeats every 188 bytes. A piece
/// with no such pattern is left as it is.
pub fn strip_to_sync(data: &mut Vec<u8>) {
    let synced_at = |i: usize| -> bool {
        data[i] == 0x47 && i + TS_PACKET <= data.len() && (1..=3).all(|k| i + k * TS_PACKET >= data.len() || data[i + k * TS_PACKET] == 0x47)
    };
    if data.is_empty() || synced_at(0) {
        return;
    }
    let look = data.len().min(1024 * 1024);
    if let Some(i) = (1..look).find(|&i| synced_at(i)) {
        data.drain(..i);
    }
}

/// The kind of file a track's pieces make once joined.
fn track_ext(media: &Media) -> &'static str {
    if !media.maps.is_empty() {
        return "mp4";
    }
    let path = media.segments.first().and_then(|s| Url::parse(&s.uri).ok()).map(|u| u.path().to_ascii_lowercase()).unwrap_or_default();
    if path.ends_with(".aac") {
        "aac"
    } else if path.ends_with(".mp3") {
        "mp3"
    } else {
        "ts"
    }
}

/// The browser's cookies a request to `url` would carry (domain, path and secure rules). The
/// request checked them already: only the stream's own server's are there at all.
pub fn cookie_header_for(url: &str, cookies: &[SiteCookie]) -> Option<String> {
    let u = Url::parse(url).ok()?;
    let host = u.host_str()?.to_ascii_lowercase();
    let path = u.path();
    let parts: Vec<String> = cookies
        .iter()
        .filter(|c| {
            let domain = c.domain.to_ascii_lowercase();
            let host_ok = match domain.strip_prefix('.') {
                Some(bare) => !bare.is_empty() && (host == bare || host.ends_with(&format!(".{bare}"))),
                None => host == domain,
            };
            let p = if c.path.is_empty() { "/" } else { c.path.as_str() };
            let path_ok = path == p || (path.starts_with(p) && (p.ends_with('/') || path[p.len()..].starts_with('/')));
            host_ok && path_ok && (!c.secure || u.scheme() == "https")
        })
        .map(|c| format!("{}={}", c.name, c.value))
        .collect();
    (!parts.is_empty()).then(|| parts.join("; "))
}

// ---------- downloading ----------

/// What every request carries, and the stop button.
pub struct Ctx {
    pub http: reqwest::Client,
    pub referrer: Option<String>,
    pub cookies: Vec<SiteCookie>,
    pub cancel: CancellationToken,
}

impl Ctx {
    /// One request (no retries): the body, at most `limit` bytes, and the address it finally
    /// came from (after the server's redirects).
    async fn get_once(&self, url: &str, range: Option<(u64, u64)>, limit: usize) -> Result<(Vec<u8>, String), Error> {
        use reqwest::header;
        let mut req = self.http.get(url).header(header::USER_AGENT, crate::fetch::BROWSER_UA).header(header::ACCEPT, "*/*");
        // The page as the browser sends it to this request's own server: pieces and keys often
        // come from another site than the playlist, which then gets "https://site/" only.
        let referrer = self.referrer.as_deref().map(|p| crate::stream::referrer_for(p, url)).unwrap_or_default();
        if !referrer.is_empty() {
            req = req.header(header::REFERER, referrer);
        }
        if let Some(c) = cookie_header_for(url, &self.cookies) {
            req = req.header(header::COOKIE, c);
        }
        if let Some((len, off)) = range {
            req = req.header(header::RANGE, format!("bytes={}-{}", off, off + len - 1));
        }
        let mut resp = req.send().await.map_err(|e| Error::Net(e.to_string()))?;
        let status = resp.status();
        let final_url = resp.url().to_string();
        if !status.is_success() {
            let msg = format!("The server answered {status}");
            return Err(if status.is_server_error() || status.as_u16() == 429 || status.as_u16() == 408 {
                Error::Net(msg)
            } else {
                Error::Refused(msg)
            });
        }
        if resp.content_length().is_some_and(|n| n as usize > limit) {
            return Err(Error::NotHls);
        }
        let mut body = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(|e| Error::Net(e.to_string()))? {
            body.extend_from_slice(&chunk);
            if body.len() > limit {
                return Err(Error::NotHls);
            }
        }
        // A server that sent the whole file for a range: the part asked for.
        if let Some((len, off)) = range.filter(|_| status == reqwest::StatusCode::OK) {
            let (start, end) = (off as usize, off.saturating_add(len) as usize);
            if let Some(part) = body.get(start..end) {
                body = part.to_vec();
            }
        }
        Ok((body, final_url))
    }

    /// A request, tried again after a failed connection or a busy server (1, 2, 4, 8 s).
    async fn get(&self, url: &str, range: Option<(u64, u64)>, limit: usize) -> Result<Vec<u8>, Error> {
        self.get_from(url, range, limit).await.map(|(body, _)| body)
    }

    /// `get`, with the address the answer finally came from.
    async fn get_from(&self, url: &str, range: Option<(u64, u64)>, limit: usize) -> Result<(Vec<u8>, String), Error> {
        let mut attempt = 0u32;
        loop {
            let r = tokio::select! {
                _ = self.cancel.cancelled() => return Err(Error::Cancelled),
                r = self.get_once(url, range, limit) => r,
            };
            match r {
                Err(Error::Net(_)) if attempt + 1 < TRIES => {
                    tokio::select! {
                        _ = self.cancel.cancelled() => return Err(Error::Cancelled),
                        _ = tokio::time::sleep(Duration::from_secs(1 << attempt)) => {}
                    }
                    attempt += 1;
                }
                other => return other,
            }
        }
    }

    async fn playlist(&self, url: &str) -> Result<Playlist, Error> {
        let (body, from) = self.get_from(url, None, MAX_PLAYLIST).await?;
        let text = String::from_utf8_lossy(&body);
        // Its links are relative to where it really came from (a playlist address often
        // redirects to the server that has the pieces), as players read them.
        let base = Url::parse(&from).or_else(|_| Url::parse(url)).map_err(|_| Error::NotHls)?;
        parse(&text, &base)
    }
}

/// One track ready to be joined.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub path: PathBuf,
    pub ext: String,
    pub has_video: bool,
    pub has_audio: bool,
    /// The files that follow `path` when the init section changes part-way (each one whole on
    /// its own; ffmpeg joins them, see `stream::rejoin_args`).
    pub more: Vec<PathBuf>,
    /// The largest picture size the init sections name (0 = not known).
    pub size: (u32, u32),
}

/// Where a download keeps its pieces: "Title.p1a2b3c4d.fhls.dir.pakopart", a folder named
/// like Pako's other video pieces (so removing, starting over or renaming the download takes
/// it along).
pub fn work_dir(dest: &Path, name: &str, download_id: &str) -> PathBuf {
    crate::video::stream_part(dest, name, download_id, "hls", "dir")
}

/// What was planned last time: which tracks, and how many pieces each (a playlist that changed
/// starts over, so pieces of two versions never mix).
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
struct Plan {
    tracks: Vec<(String, usize)>,
}

/// A playlist's address without what changes between visits (the query's tokens).
fn stable(url: &str) -> String {
    Url::parse(url).map(|u| format!("{}{}", u.host_str().unwrap_or(""), u.path())).unwrap_or_default()
}

struct Job {
    track: usize,
    /// None = the track's init section `map`.
    piece: Piece,
}

enum Piece {
    Init(usize),
    Segment(usize),
}

fn piece_path(work: &Path, track: usize, piece: &Piece) -> PathBuf {
    match piece {
        Piece::Init(k) => work.join(format!("t{track}-init{k}.seg")),
        Piece::Segment(i) => work.join(format!("t{track}-{i:06}.seg")),
    }
}

/// Downloads the stream at `url` into `work` and joins each track into one file there. `quality`
/// = height wanted (0 or `audio_only` = sound only). `progress(bytes so far, likely total)`.
pub async fn download(
    ctx: &Ctx,
    url: &str,
    quality: u32,
    audio_only: bool,
    work: &Path,
    progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<Vec<Track>, Error> {
    // 1. The playlists: the chosen quality's, and its separate sound.
    let mut tracks: Vec<(String, Media, bool, bool)> = Vec::new();
    match ctx.playlist(url).await? {
        Playlist::Media(m) => tracks.push((url.to_string(), m, !audio_only, true)),
        Playlist::Master(m) => {
            if m.protected {
                return Err(Error::Protected);
            }
            let picked = pick(&m, quality, audio_only).ok_or(Error::NotHls)?;
            if let Some(v) = &picked.video {
                let Playlist::Media(media) = ctx.playlist(v).await? else { return Err(Error::NotHls) };
                tracks.push((v.clone(), media, !audio_only, picked.audio.is_none()));
            }
            if let Some(a) = &picked.audio {
                let Playlist::Media(media) = ctx.playlist(a).await? else { return Err(Error::NotHls) };
                tracks.push((a.clone(), media, false, true));
            }
        }
    }
    if tracks.iter().any(|t| !t.1.ended) {
        return Err(Error::Live);
    }

    // 2. Continue from the pieces saved last time, unless the playlist changed.
    std::fs::create_dir_all(work).map_err(Error::Disk)?;
    let plan = Plan { tracks: tracks.iter().map(|t| (stable(&t.0), t.1.segments.len())).collect() };
    let plan_file = work.join("plan.json");
    let before: Option<Plan> = std::fs::read(&plan_file).ok().and_then(|b| serde_json::from_slice(&b).ok());
    if before.as_ref() != Some(&plan) {
        for e in std::fs::read_dir(work).map_err(Error::Disk)?.flatten() {
            let _ = std::fs::remove_file(e.path());
        }
        std::fs::write(&plan_file, serde_json::to_vec(&plan).unwrap_or_default()).map_err(Error::Disk)?;
    }
    let mut jobs: Vec<Job> = Vec::new();
    let mut have_bytes = 0u64;
    let mut have_count = 0u64;
    let mut count = 0u64;
    for (t, (_, media, _, _)) in tracks.iter().enumerate() {
        let pieces = (0..media.maps.len()).map(Piece::Init).chain((0..media.segments.len()).map(Piece::Segment));
        for piece in pieces {
            count += 1;
            match std::fs::metadata(piece_path(work, t, &piece)) {
                Ok(m) => {
                    have_bytes += m.len();
                    have_count += 1;
                }
                Err(_) => jobs.push(Job { track: t, piece }),
            }
        }
    }

    // 3. The missing pieces, a few at a time.
    let done_bytes = AtomicU64::new(have_bytes);
    let done_count = AtomicU64::new(have_count);
    let report = || {
        let (b, n) = (done_bytes.load(Ordering::Relaxed), done_count.load(Ordering::Relaxed));
        let total = (n > 0).then(|| (b as f64 / n as f64 * count as f64) as u64).map(|t| t.max(b));
        progress(b, total);
    };
    report();
    let keys: Mutex<HashMap<String, [u8; 16]>> = Mutex::new(HashMap::new());
    let fetch = |job: Job| {
        let (keys, media, done_bytes, done_count, report) = (&keys, &tracks[job.track].1, &done_bytes, &done_count, &report);
        let ext = track_ext(media);
        async move {
            let (uri, range, key, seq) = match job.piece {
                Piece::Init(k) => {
                    let m = &media.maps[k];
                    (&m.uri, m.range, &m.key, m.seq)
                }
                Piece::Segment(i) => {
                    let s = &media.segments[i];
                    (&s.uri, s.range, &s.key, s.seq)
                }
            };
            let mut data = ctx.get(uri, range, MAX_PIECE).await?;
            if let Some(k) = key {
                let cached = keys.lock().unwrap().get(&k.uri).copied();
                let secret = match cached {
                    Some(s) => s,
                    None => {
                        let raw = ctx.get(&k.uri, None, 64).await?;
                        let s: [u8; 16] = raw.as_slice().try_into().map_err(|_| Error::Protected)?;
                        keys.lock().unwrap().insert(k.uri.clone(), s);
                        s
                    }
                };
                data = decrypt(&data, &secret, &iv_for(k, seq)).ok_or(Error::NotHls)?;
            }
            if ext == "ts" {
                strip_to_sync(&mut data);
            }
            let path = piece_path(work, job.track, &job.piece);
            let tmp = path.with_extension("tmp");
            tokio::fs::write(&tmp, &data).await.map_err(Error::Disk)?;
            tokio::fs::rename(&tmp, &path).await.map_err(Error::Disk)?;
            done_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
            done_count.fetch_add(1, Ordering::Relaxed);
            report();
            Ok::<(), Error>(())
        }
    };
    let mut runs = stream::iter(jobs).map(fetch).buffer_unordered(AT_ONCE);
    while let Some(r) = runs.next().await {
        r?;
    }
    drop(runs);
    if ctx.cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }

    // 4. Each track in one file (off the async threads: a long video takes a while), or in
    // several when its init section changes part-way (see `join_track`).
    let mut out = Vec::new();
    for (t, (_, media, has_video, has_audio)) in tracks.iter().enumerate() {
        let (work, media, cancel) = (work.to_path_buf(), media.clone(), ctx.cancel.clone());
        let joined = tokio::task::spawn_blocking(move || join_track(&work, t, &media, &cancel)).await;
        let (files, size) = match joined {
            Ok(Ok(Some(j))) => j,
            Ok(Ok(None)) => return Err(Error::Cancelled),
            Ok(Err(e)) => return Err(Error::Disk(e)),
            Err(e) => return Err(Error::Disk(std::io::Error::other(e.to_string()))),
        };
        let mut files = files.into_iter();
        let path = files.next().ok_or(Error::NotHls)?;
        out.push(Track { path, ext: track_ext(&tracks[t].1).into(), has_video: *has_video, has_audio: *has_audio, more: files.collect(), size });
    }
    Ok(out)
}

/// Joins one track's pieces: its init section, then its pieces, in "track`t`.ext". When the
/// init section changes part-way to a different one (another quality, an advert), what follows
/// goes in "track`t`-1.ext" and so on: one file can only hold one (players read the first and
/// the picture after the change would be broken). Returns the files and the largest picture
/// size the init sections name; None = stopped.
#[allow(clippy::type_complexity)]
fn join_track(work: &Path, t: usize, media: &Media, cancel: &CancellationToken) -> std::io::Result<Option<(Vec<PathBuf>, (u32, u32))>> {
    use std::io::Write;
    let ext = track_ext(media);
    let mut files = vec![work.join(format!("track{t}.{ext}"))];
    let mut f = std::io::BufWriter::new(std::fs::File::create(&files[0])?);
    let mut last_map = None;
    let mut last_init: Option<Vec<u8>> = None;
    let mut size = (0u32, 0u32);
    for (i, s) in media.segments.iter().enumerate() {
        if cancel.is_cancelled() {
            return Ok(None);
        }
        if let Some(k) = s.map.filter(|_| s.map != last_map) {
            last_map = s.map;
            let init = std::fs::read(piece_path(work, t, &Piece::Init(k)))?;
            if last_init.as_ref() != Some(&init) {
                if last_init.is_some() {
                    f.flush()?;
                    files.push(work.join(format!("track{t}-{}.{ext}", files.len())));
                    f = std::io::BufWriter::new(std::fs::File::create(files.last().unwrap_or(&files[0]))?);
                }
                if let Some((w, h)) = crate::stream::mp4_size(&init).filter(|&(_, h)| h > size.1) {
                    size = (w, h);
                }
                f.write_all(&init)?;
                last_init = Some(init);
            }
        }
        f.write_all(&std::fs::read(piece_path(work, t, &Piece::Segment(i)))?)?;
    }
    f.flush()?;
    Ok(Some((files, size)))
}

/// ffmpeg's arguments to make the video from the joined tracks, copied as they are: MP4 (or
/// MKV with `mkv`, when the streams don't fit in MP4).
pub fn join_args(tracks: &[Track], out: &Path, mkv: bool) -> Vec<std::ffi::OsString> {
    use std::ffi::OsString;
    let mut args: Vec<OsString> = ["-hide_banner", "-nostdin", "-loglevel", "error", "-y"].iter().map(OsString::from).collect();
    for t in tracks {
        args.push("-i".into());
        args.push(t.path.clone().into_os_string());
    }
    let mut map = |m: String| {
        args.push("-map".into());
        args.push(m.into());
    };
    let video = tracks.iter().position(|t| t.has_video).unwrap_or(0);
    map(format!("{video}:v:0?"));
    // The separate sound when there is one, else the video's own.
    let sound = tracks.iter().enumerate().position(|(i, t)| i != video && t.has_audio).unwrap_or(video);
    map(format!("{sound}:a:0?"));
    args.extend(["-c", "copy"].iter().map(OsString::from));
    if mkv {
        args.extend(["-f", "matroska"].iter().map(OsString::from));
    } else {
        args.extend(["-movflags", "+faststart", "-f", "mp4"].iter().map(OsString::from));
    }
    args.push(out.as_os_str().to_owned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockEncrypt;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn base() -> Url {
        Url::parse("https://cdn.example/v/master.m3u8?token=1").unwrap()
    }

    /// A byte range whose end doesn't fit in a number is dropped (the sums on it would wrap,
    /// and cutting the answer with them would stop Pako), so the piece is asked for whole.
    #[test]
    fn a_byte_range_past_the_largest_number_is_ignored() {
        assert_eq!(byte_range("1000@200", 0), Some((1000, 200)));
        assert_eq!(byte_range("500", 1200), Some((500, 1200)));
        assert_eq!(byte_range("10@18446744073709551610", 0), None);
        assert_eq!(byte_range("18446744073709551615", 1), None);
        let text = "#EXTM3U
#EXTINF:6.0,
#EXT-X-BYTERANGE:10@18446744073709551610
main.ts
#EXT-X-ENDLIST
";
        let Ok(Playlist::Media(m)) = parse(text, &base()) else { panic!("not read") };
        assert_eq!(m.segments[0].range, None);
    }

    fn encrypt(data: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Vec<u8> {
        let cipher = aes::Aes128::new(GenericArray::from_slice(key));
        let pad = 16 - data.len() % 16;
        let mut plain = data.to_vec();
        plain.extend(std::iter::repeat_n(pad as u8, pad));
        let mut prev = *iv;
        let mut out = Vec::new();
        for chunk in plain.chunks(16) {
            let mut block = GenericArray::clone_from_slice(chunk);
            for (b, p) in block.iter_mut().zip(prev.iter()) {
                *b ^= p;
            }
            cipher.encrypt_block(&mut block);
            prev.copy_from_slice(&block);
            out.extend_from_slice(&block);
        }
        out
    }

    /// `n` MPEG-TS packets (sync byte, then the piece's number).
    fn ts(n: usize, mark: u8) -> Vec<u8> {
        let mut v = Vec::new();
        for _ in 0..n {
            let mut p = vec![mark; TS_PACKET];
            p[0] = 0x47;
            v.extend(p);
        }
        v
    }

    #[test]
    fn reads_attributes_with_quoted_commas() {
        let a = attributes(r#"BANDWIDTH=800000,CODECS="avc1.4d401f,mp4a.40.2",RESOLUTION=1280x720,AUDIO="aud""#);
        assert_eq!(a["BANDWIDTH"], "800000");
        assert_eq!(a["CODECS"], "avc1.4d401f,mp4a.40.2");
        assert_eq!(a["RESOLUTION"], "1280x720");
        assert_eq!(a["AUDIO"], "aud");
    }

    #[test]
    fn picks_the_quality_and_its_sound() {
        let text = "#EXTM3U\n\
            #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"en\",DEFAULT=YES,URI=\"audio/en.m3u8\"\n\
            #EXT-X-STREAM-INF:BANDWIDTH=500000,RESOLUTION=640x360,CODECS=\"avc1.4d401e,mp4a.40.2\",AUDIO=\"aud\"\n360.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720,AUDIO=\"aud\"\n720.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=5000000,RESOLUTION=1920x1080,AUDIO=\"aud\"\nhttps://other.example/1080.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=64000,CODECS=\"mp4a.40.2\"\naudio-only.m3u8\n";
        let Playlist::Master(m) = parse(text, &base()).unwrap() else { panic!() };
        assert_eq!(m.variants.len(), 4);
        let p = pick(&m, 720, false).unwrap();
        assert_eq!(p.video.as_deref(), Some("https://cdn.example/v/720.m3u8"));
        assert_eq!(p.audio.as_deref(), Some("https://cdn.example/v/audio/en.m3u8"));
        assert_eq!(pick(&m, 2160, false).unwrap().video.as_deref(), Some("https://other.example/1080.m3u8"));
        // Smaller than any: the smallest one.
        assert_eq!(pick(&m, 240, false).unwrap().video.as_deref(), Some("https://cdn.example/v/360.m3u8"));
        // Sound only: the sound rendition.
        assert_eq!(pick(&m, 0, true).unwrap(), Picked { video: None, audio: Some("https://cdn.example/v/audio/en.m3u8".into()) });
    }

    #[test]
    fn playlists_without_heights_take_the_best() {
        let text = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=100\na.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=900\nb.m3u8\n";
        let Playlist::Master(m) = parse(text, &base()).unwrap() else { panic!() };
        assert_eq!(pick(&m, 480, false).unwrap().video.as_deref(), Some("https://cdn.example/v/b.m3u8"));
        assert_eq!(pick(&m, 0, true).unwrap().video.as_deref(), Some("https://cdn.example/v/a.m3u8"));
    }

    #[test]
    fn reads_keys_ranges_and_init_sections() {
        let text = "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXT-X-MEDIA-SEQUENCE:7\n\
            #EXT-X-MAP:URI=\"init.mp4\",BYTERANGE=\"720@0\"\n\
            #EXT-X-KEY:METHOD=AES-128,URI=\"/k/key.bin\",IV=0x0000000000000000000000000000000A\n\
            #EXTINF:6.0,\n#EXT-X-BYTERANGE:1000@720\nmain.mp4\n\
            #EXTINF:6.0,\n#EXT-X-BYTERANGE:500\nmain.mp4\n\
            #EXT-X-KEY:METHOD=NONE\n#EXTINF:4.0,\nlast.m4s\n#EXT-X-ENDLIST\n";
        let Playlist::Media(m) = parse(text, &base()).unwrap() else { panic!() };
        assert!(m.ended);
        assert_eq!(m.maps.len(), 1);
        assert_eq!(m.maps[0].range, Some((720, 0)));
        assert_eq!(m.segments.len(), 3);
        assert_eq!(m.segments[0].seq, 7);
        assert_eq!(m.segments[0].range, Some((1000, 720)));
        assert_eq!(m.segments[1].range, Some((500, 1720)));
        let k = m.segments[0].key.as_ref().unwrap();
        assert_eq!(k.uri, "https://cdn.example/k/key.bin");
        assert_eq!(iv_for(k, 7)[15], 0x0a);
        assert!(m.segments[2].key.is_none());
        assert_eq!(m.segments[2].map, Some(0));
        assert_eq!(track_ext(&m), "mp4");
        // Without an IV: the media sequence number.
        assert_eq!(iv_for(&Key { uri: String::new(), iv: None }, 0x0102), {
            let mut iv = [0u8; 16];
            iv[14] = 1;
            iv[15] = 2;
            iv
        });
    }

    #[test]
    fn drm_is_refused_before_anything_downloads() {
        for key in [
            "#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://x\",KEYFORMAT=\"com.apple.streamingkeydelivery\"",
            "#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI=\"data:x\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\"",
            "#EXT-X-KEY:METHOD=AES-128,URI=\"k\",KEYFORMAT=\"com.microsoft.playready\"",
        ] {
            let text = format!("#EXTM3U\n{key}\n#EXTINF:6,\na.ts\n#EXT-X-ENDLIST\n");
            assert!(matches!(parse(&text, &base()), Err(Error::Protected)), "{key}");
        }
        let master = "#EXTM3U\n#EXT-X-SESSION-KEY:METHOD=SAMPLE-AES,URI=\"skd://x\",KEYFORMAT=\"com.apple.streamingkeydelivery\"\n#EXT-X-STREAM-INF:BANDWIDTH=1\na.m3u8\n";
        let Playlist::Master(m) = parse(master, &base()).unwrap() else { panic!() };
        assert!(m.protected);
        assert_eq!(Error::Protected.message(), protected());
        // Plain AES-128 (the standard key file) is fine, its key format named or not.
        let ok = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\",KEYFORMAT=\"identity\"\n#EXTINF:6,\na.ts\n#EXT-X-ENDLIST\n";
        assert!(parse(ok, &base()).is_ok());
    }

    #[test]
    fn not_a_playlist() {
        assert!(matches!(parse("<html>", &base()), Err(Error::NotHls)));
        assert!(matches!(parse("#EXTM3U\n", &base()), Err(Error::NotHls)));
    }

    #[test]
    fn opens_aes_128_pieces() {
        let key = [7u8; 16];
        let iv = iv_for(&Key { uri: String::new(), iv: None }, 3);
        let plain = ts(3, 1);
        let enc = encrypt(&plain, &key, &iv);
        assert_ne!(enc, plain);
        assert_eq!(decrypt(&enc, &key, &iv).unwrap(), plain);
        assert!(decrypt(&enc[..enc.len() - 1], &key, &iv).is_none());
    }

    #[test]
    fn drops_junk_before_the_ts_data() {
        let mut junk = b"\x89PNG\r\n\x1a\nnot a video at all, G G G".to_vec();
        let real = ts(5, 2);
        junk.extend(&real);
        strip_to_sync(&mut junk);
        assert_eq!(junk, real);
        // Already clean, or not TS at all: left as it is.
        let mut clean = ts(2, 3);
        strip_to_sync(&mut clean);
        assert_eq!(clean, ts(2, 3));
        let mut other = vec![1u8, 2, 3, 0x47, 5];
        strip_to_sync(&mut other);
        assert_eq!(other, vec![1u8, 2, 3, 0x47, 5]);
    }

    #[test]
    fn cookies_only_for_their_own_server() {
        let c = |domain: &str, path: &str, secure: bool| SiteCookie {
            domain: domain.into(),
            path: path.into(),
            secure,
            name: format!("n{domain}{path}"),
            value: "v".into(),
            ..Default::default()
        };
        let jar = vec![c("cdn.example", "/", false), c(".example", "/", false), c("other.example", "/", false), c("cdn.example", "/hls", true)];
        let h = cookie_header_for("https://cdn.example/hls/a.ts", &jar).unwrap();
        assert!(h.contains("ncdn.example/") && h.contains("n.example/") && h.contains("ncdn.example/hls"));
        assert!(!h.contains("nother.example"));
        // Over http: no secure cookie. Another path: not the /hls one.
        let h = cookie_header_for("http://cdn.example/hlsx/a.ts", &jar).unwrap();
        assert!(!h.contains("/hls"));
        assert_eq!(cookie_header_for("https://elsewhere.test/a.ts", &jar), None);
    }

    #[test]
    fn joins_into_mp4_with_the_separate_sound() {
        let t = |p: &str, v: bool, a: bool| Track { path: PathBuf::from(p), ext: "ts".into(), has_video: v, has_audio: a, more: Vec::new(), size: (0, 0) };
        let args: Vec<String> =
            join_args(&[t("v.ts", true, false), t("a.aac", false, true)], Path::new("out.mp4"), false).iter().map(|a| a.to_string_lossy().into()).collect();
        assert!(args.windows(2).any(|w| w == ["-map", "0:v:0?"]));
        assert!(args.windows(2).any(|w| w == ["-map", "1:a:0?"]));
        assert!(args.windows(2).any(|w| w == ["-f", "mp4"]));
        let args: Vec<String> = join_args(&[t("v.ts", true, true)], Path::new("out.mkv"), true).iter().map(|a| a.to_string_lossy().into()).collect();
        assert!(args.windows(2).any(|w| w == ["-map", "0:a:0?"]));
        assert!(args.windows(2).any(|w| w == ["-f", "matroska"]));
    }

    // ---------- a small HLS server on this computer ----------

    type Routes = Arc<HashMap<String, Vec<u8>>>;

    /// Serves `routes` (path → body) over plain HTTP, noting each request's headers. Paths in
    /// `fail_once` answer 503 the first time.
    async fn serve(routes: Routes, seen: Arc<Mutex<Vec<String>>>, fail_once: Vec<String>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let failed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { return };
                let (routes, seen, fail_once, failed) = (routes.clone(), seen.clone(), fail_once.clone(), failed.clone());
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut tmp = [0u8; 4096];
                    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        let Ok(n) = sock.read(&mut tmp).await else { return };
                        if n == 0 {
                            return;
                        }
                        buf.extend_from_slice(&tmp[..n]);
                    }
                    let head = String::from_utf8_lossy(&buf).to_string();
                    seen.lock().unwrap().push(head.clone());
                    let path = head.split_whitespace().nth(1).unwrap_or("/").split('?').next().unwrap_or("/").to_string();
                    let first_fail = fail_once.contains(&path) && !failed.lock().unwrap().contains(&path);
                    let resp = match routes.get(&path) {
                        Some(body) if body.starts_with(b"REDIRECT ") => {
                            let to = String::from_utf8_lossy(&body[9..]).to_string();
                            format!("HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").into_bytes()
                        }
                        _ if first_fail => {
                            failed.lock().unwrap().push(path.clone());
                            b"HTTP/1.1 503 Busy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
                        }
                        Some(body) => {
                            let mut r = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
                            r.extend_from_slice(body);
                            r
                        }
                        None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                    };
                    let _ = sock.write_all(&resp).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        format!("http://{addr}")
    }

    fn ctx(cookies: Vec<SiteCookie>) -> Ctx {
        Ctx {
            http: reqwest::Client::new(),
            referrer: Some("http://page.example/watch".into()),
            cookies,
            cancel: CancellationToken::new(),
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pako-hls-{tag}-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn downloads_an_encrypted_stream_with_junk_and_continues_after_a_stop() {
        let key = [9u8; 16];
        let mut routes: HashMap<String, Vec<u8>> = HashMap::new();
        routes.insert(
            "/master.m3u8".into(),
            b"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=900000,RESOLUTION=1280x720\nhi/index.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=300000,RESOLUTION=640x360\nlo/index.m3u8\n".to_vec(),
        );
        let mut media = String::from("#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-KEY:METHOD=AES-128,URI=\"/key\"\n");
        let mut expected: Vec<u8> = Vec::new();
        for i in 0..6u8 {
            media.push_str(&format!("#EXTINF:4.0,\nseg{i}.ts?t=abc\n"));
            let plain = ts(4, i);
            expected.extend(&plain);
            // Junk in front of every other piece (the server's disguise), then encrypted.
            let mut body = if i % 2 == 0 { b"\x89PNG junk".to_vec() } else { Vec::new() };
            body.extend(&plain);
            routes.insert(format!("/hi/seg{i}.ts"), encrypt(&body, &key, &iv_for(&Key { uri: String::new(), iv: None }, i as u64)));
        }
        media.push_str("#EXT-X-ENDLIST\n");
        routes.insert("/hi/index.m3u8".into(), media.into_bytes());
        routes.insert("/key".into(), key.to_vec());
        routes.insert("/lo/index.m3u8".into(), b"#EXTM3U\n#EXTINF:4,\nx.ts\n#EXT-X-ENDLIST\n".to_vec());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let host = serve(Arc::new(routes), seen.clone(), vec!["/hi/seg2.ts".into()]).await;
        let cookies = vec![SiteCookie { domain: "127.0.0.1".into(), path: "/".into(), name: "sid".into(), value: "42".into(), ..Default::default() }];
        let work = temp_dir("enc");

        // A first run stopped part-way: some pieces stay saved.
        let c = ctx(cookies.clone());
        let stop = c.cancel.clone();
        let calls = AtomicU64::new(0);
        let r = download(&c, &format!("{host}/master.m3u8"), 720, false, &work, &|_, _| {
            if calls.fetch_add(1, Ordering::Relaxed) == 2 {
                stop.cancel();
            }
        })
        .await;
        assert!(matches!(r, Err(Error::Cancelled)), "{r:?}");
        let saved = std::fs::read_dir(&work).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".seg")).count();
        assert!((2..6).contains(&saved), "{saved}");

        // Then on: only what is missing is fetched, and the track is whole.
        let before = seen.lock().unwrap().iter().filter(|h| h.contains(".ts")).count();
        let last = Mutex::new((0u64, None));
        let tracks = download(&ctx(cookies), &format!("{host}/master.m3u8"), 720, false, &work, &|b, t| *last.lock().unwrap() = (b, t))
            .await
            .unwrap();
        let after = seen.lock().unwrap().iter().filter(|h| h.contains(".ts")).count();
        assert!(after - before <= 6 - saved + 1, "fetched again: {}", after - before);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].ext, "ts");
        assert!(tracks[0].has_video && tracks[0].has_audio);
        assert_eq!(std::fs::read(&tracks[0].path).unwrap(), expected);
        let (bytes, total) = *last.lock().unwrap();
        assert_eq!(bytes, expected.len() as u64);
        assert_eq!(total, Some(expected.len() as u64));
        // Every request carried the page (only its site: this server is another one, as a
        // browser sends it) and the browser's cookie for this server.
        for h in seen.lock().unwrap().iter() {
            let lower = h.to_ascii_lowercase();
            assert!(lower.contains("referer: http://page.example/\r\n"), "{h}");
            assert!(lower.contains("cookie: sid=42"), "{h}");
        }
        let _ = std::fs::remove_dir_all(&work);
    }

    #[tokio::test]
    async fn separate_sound_and_init_sections() {
        let mut routes: HashMap<String, Vec<u8>> = HashMap::new();
        routes.insert(
            "/m.m3u8".into(),
            b"#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",DEFAULT=YES,URI=\"a.m3u8\"\n#EXT-X-STREAM-INF:BANDWIDTH=1,RESOLUTION=1x480,AUDIO=\"a\"\nv.m3u8\n".to_vec(),
        );
        routes.insert("/v.m3u8".into(), b"#EXTM3U\n#EXT-X-MAP:URI=\"vinit.mp4\"\n#EXTINF:2,\nv1.m4s\n#EXTINF:2,\nv2.m4s\n#EXT-X-ENDLIST\n".to_vec());
        routes.insert("/a.m3u8".into(), b"#EXTM3U\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXTINF:2,\na1.aac\n".to_vec());
        for (p, b) in [("/vinit.mp4", "I"), ("/v1.m4s", "1"), ("/v2.m4s", "2"), ("/a1.aac", "A")] {
            routes.insert(p.into(), b.as_bytes().to_vec());
        }
        let host = serve(Arc::new(routes), Arc::new(Mutex::new(Vec::new())), Vec::new()).await;
        let work = temp_dir("sep");
        let tracks = download(&ctx(Vec::new()), &format!("{host}/m.m3u8"), 1080, false, &work, &|_, _| {}).await.unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!((tracks[0].ext.as_str(), tracks[0].has_video, tracks[0].has_audio), ("mp4", true, false));
        assert_eq!(std::fs::read(&tracks[0].path).unwrap(), b"I12");
        assert_eq!((tracks[1].ext.as_str(), tracks[1].has_video, tracks[1].has_audio), ("aac", false, true));
        assert_eq!(std::fs::read(&tracks[1].path).unwrap(), b"A");
        let _ = std::fs::remove_dir_all(&work);
    }

    #[tokio::test]
    async fn follows_a_redirect_and_splits_where_the_init_section_changes() {
        let mut routes: HashMap<String, Vec<u8>> = HashMap::new();
        // The playlist's address sends to another folder: its pieces are only there.
        routes.insert("/start.m3u8".into(), b"REDIRECT /deep/real.m3u8".to_vec());
        routes.insert(
            "/deep/real.m3u8".into(),
            b"#EXTM3U\n#EXT-X-MAP:URI=\"a.mp4\"\n#EXTINF:2,\na1.m4s\n#EXTINF:2,\na2.m4s\n\
              #EXT-X-DISCONTINUITY\n#EXT-X-MAP:URI=\"b.mp4\"\n#EXTINF:2,\nb1.m4s\n\
              #EXT-X-MAP:URI=\"b-again.mp4\"\n#EXTINF:2,\nb2.m4s\n#EXT-X-ENDLIST\n"
                .to_vec(),
        );
        for (p, b) in [("/deep/a.mp4", "A"), ("/deep/a1.m4s", "1"), ("/deep/a2.m4s", "2"), ("/deep/b.mp4", "B"), ("/deep/b-again.mp4", "B"), ("/deep/b1.m4s", "3"), ("/deep/b2.m4s", "4")] {
            routes.insert(p.into(), b.as_bytes().to_vec());
        }
        let host = serve(Arc::new(routes), Arc::new(Mutex::new(Vec::new())), Vec::new()).await;
        let work = temp_dir("redir");
        let tracks = download(&ctx(Vec::new()), &format!("{host}/start.m3u8"), 720, false, &work, &|_, _| {}).await.unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(std::fs::read(&tracks[0].path).unwrap(), b"A12");
        // Another init section: its own file (the same one again isn't).
        assert_eq!(tracks[0].more.len(), 1);
        assert_eq!(std::fs::read(&tracks[0].more[0]).unwrap(), b"B34");
        let _ = std::fs::remove_dir_all(&work);
    }

    #[tokio::test]
    async fn live_missing_and_protected_streams_stop() {
        let mut routes: HashMap<String, Vec<u8>> = HashMap::new();
        routes.insert("/live.m3u8".into(), b"#EXTM3U\n#EXTINF:2,\na.ts\n".to_vec());
        routes.insert("/drm.m3u8".into(), b"#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://k\"\n#EXTINF:2,\na.ts\n#EXT-X-ENDLIST\n".to_vec());
        routes.insert("/gone.m3u8".into(), b"#EXTM3U\n#EXTINF:2,\nmissing.ts\n#EXT-X-ENDLIST\n".to_vec());
        let host = serve(Arc::new(routes), Arc::new(Mutex::new(Vec::new())), Vec::new()).await;
        let work = temp_dir("stops");
        let run = |p: &str| {
            let url = format!("{host}{p}");
            let work = work.clone();
            async move { download(&ctx(Vec::new()), &url, 720, false, &work, &|_, _| {}).await }
        };
        assert!(matches!(run("/live.m3u8").await, Err(Error::Live)));
        assert!(matches!(run("/drm.m3u8").await, Err(Error::Protected)));
        let gone = run("/gone.m3u8").await;
        assert!(matches!(gone, Err(Error::Refused(_))), "{gone:?}");
        assert_eq!(gone.unwrap_err().message(), couldnt());
        assert!(matches!(run("/nothing.m3u8").await, Err(Error::Refused(_))));
        let _ = std::fs::remove_dir_all(&work);
    }
}
