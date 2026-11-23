// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Video preview before (or while) downloading: a tiny web server on this PC only
//! (127.0.0.1) that the details panel's player reads from.
//!
//! - /raw/<token>: the file itself, piece by piece (the player asks for the parts it
//!   needs). Drive files come through Google's public download address.
//! - /low/<token>: the same video converted on the fly by ffmpeg to a small MP4
//!   (480p by default), so MKV, AVI and the rest play too and little data is used.
//! - /site/<token>: a video-site link (YouTube and the others yt-dlp reads): yt-dlp finds a
//!   small stream (480p or lower) and ffmpeg turns its first seconds into a short clip, just
//!   to check it's the right video. Only when the user presses play: it fetches from the site.
//!   /part/<token>/<n> passes a stream's pieces to ffmpeg with the site's headers (cookies
//!   included), so they never appear on a command line.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::fetch::Fetcher;
use crate::model::Source;

struct Entry {
    source: Source,
    name: String,
    quality: u32,
    created: Instant,
    /// The download this preview is for (its browser login, for video sites).
    owner: Option<String>,
    /// Video sites: the streams yt-dlp found (asked once, then kept a while).
    site: Option<Arc<SiteSlot>>,
}

/// Video-site previews: at most this many lines.
const SITE_LINES: u32 = 480;
/// Video-site previews: a clip this many seconds long.
const CLIP_SECONDS: u32 = 20;
/// How long yt-dlp may take to find the streams.
const FIND_LIMIT: Duration = Duration::from_secs(45);
/// Found stream addresses are reused this long (sites keep them valid for hours).
const FOUND_FOR: Duration = Duration::from_secs(20 * 60);
/// What yt-dlp picks for a preview: one small file with picture and sound when there is one,
/// else small picture + sound. Never DASH pieces (ffmpeg can't pick one stream out of them).
const SITE_FORMAT: &str = "b[height<=480][protocol!*=dash]/bv*[height<=480][protocol!*=dash]+ba[protocol!*=dash]/b[protocol!*=dash]/bv*[protocol!*=dash]+ba[protocol!*=dash]";
fn needs_tools() -> String {
    tr!(
        "Previewing this video needs the video tools: Settings › Features & updates › Download videos from YouTube and other sites.",
        "L’aperçu de cette vidéo a besoin des outils vidéo : Paramètres › Fonctions et mises à jour › Télécharger des vidéos depuis YouTube et d’autres sites."
    )
}
fn live() -> String {
    tr!("Live streams can't be previewed.", "Les directs ne peuvent pas avoir d’aperçu.")
}

#[derive(Default)]
struct SiteSlot {
    found: tokio::sync::Mutex<Option<(Instant, Clip)>>,
}

/// The streams of a video-site preview.
#[derive(Clone, Debug, PartialEq)]
struct Clip {
    inputs: Vec<Input>,
    /// Where the clip starts (seconds): a few seconds in for longer videos (past a dark intro).
    start: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct Input {
    url: String,
    /// The headers the site wants (User-Agent, Referer, cookies...).
    headers: Vec<(String, String)>,
    /// Small pieces (HLS): ffmpeg reads them itself. Else one plain file, through /part.
    pieces: bool,
    video: bool,
    audio: bool,
}

pub struct Preview {
    port: u16,
    http: reqwest::Client,
    entries: Mutex<HashMap<String, Entry>>,
    /// Private mode's videos: the vault file and its name, read only while Private mode is
    /// open (decrypted as the player asks; nothing readable is written anywhere).
    vault: Mutex<HashMap<String, (std::path::PathBuf, String)>>,
}

/// Formats the player in Pako's window plays as they are.
pub fn plays_directly(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    matches!(ext.as_str(), "mp4" | "m4v" | "webm")
}

pub fn is_video(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    matches!(ext.as_str(), "mp4" | "m4v" | "webm" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "ts" | "mpg" | "mpeg" | "3gp")
}

impl Preview {
    pub async fn start(http: reqwest::Client) -> Result<Arc<Self>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let me = Arc::new(Self { port: listener.local_addr()?.port(), http, entries: Mutex::new(HashMap::new()), vault: Mutex::new(HashMap::new()) });
        let server = me.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((sock, _)) => {
                        let s = server.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = s.serve(sock).await;
                        });
                    }
                    // A failed accept (e.g. too many open connections) is passing: keep serving.
                    Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
                }
            }
        });
        Ok(me)
    }

    /// A player address for a download or Links item. `convert` = through ffmpeg at `quality`
    /// lines; `owner`: the item's id (`owner`: its id, for the browser login a
    /// video-site preview may need). A video-site link always gets a short clip at 480p or
    /// lower, whatever `quality` and `convert` say.
    pub fn register_item(&self, owner: Option<&str>, source: Source, name: String, quality: u32, convert: bool) -> String {
        let token = uuid::Uuid::new_v4().simple().to_string();
        let site = matches!(source, Source::Video { .. });
        let mut map = self.entries.lock().unwrap();
        map.retain(|_, e| e.created.elapsed() < Duration::from_secs(6 * 3600));
        map.insert(
            token.clone(),
            Entry {
                source,
                name,
                quality: if site { SITE_LINES } else { quality.clamp(240, 720) },
                created: Instant::now(),
                owner: owner.map(String::from),
                site: site.then(Default::default),
            },
        );
        let kind = if site { "site" } else if convert { "low" } else { "raw" };
        format!("http://127.0.0.1:{}/{kind}/{token}", self.port)
    }

    /// A player address for a video in Private mode's vault. `convert`: through ffmpeg (for
    /// kinds the player can't show as they are), reading the decrypted bytes from this same
    /// server, never from a file.
    pub fn register_vault(&self, path: std::path::PathBuf, name: String, quality: u32, convert: bool) -> String {
        let token = uuid::Uuid::new_v4().simple().to_string();
        self.vault.lock().unwrap().insert(token.clone(), (path, name));
        let kind = if convert { format!("vaultlow/{}", quality.clamp(240, 720)) } else { "vault".into() };
        format!("http://127.0.0.1:{}/{kind}/{token}", self.port)
    }

    /// Private mode locked: its player addresses stop working.
    pub fn forget_vault(&self) {
        self.vault.lock().unwrap().clear();
    }

    /// A video-site preview: asks yt-dlp for its streams (once; later calls reuse them), so the
    /// window can say plainly why a video can't be previewed before the player starts.
    pub async fn check(&self, url: &str) -> Result<(), String> {
        let token = url.rsplit_once("/site/").map(|(_, t)| t).unwrap_or_default();
        self.clip(token).await.map(|_| ())
    }

    async fn clip(&self, token: &str) -> Result<Clip, String> {
        let entry = {
            let map = self.entries.lock().unwrap();
            map.get(token).and_then(|e| Some((e.source.clone(), e.owner.clone(), e.site.clone()?)))
        };
        let Some((source, owner, slot)) = entry else { return Err(tr!("This preview is gone. Press play again.", "Cet aperçu n’est plus là. Appuyez à nouveau sur lecture.")) };
        // One question to yt-dlp at a time per preview; the others wait for its answer.
        let mut found = slot.found.lock().await;
        if let Some((at, clip)) = &*found {
            if at.elapsed() < FOUND_FOR {
                return Ok(clip.clone());
            }
        }
        // Errors aren't kept: pressing play again asks again.
        let clip = find_clip(&source, owner.as_deref()).await?;
        *found = Some((Instant::now(), clip.clone()));
        Ok(clip)
    }

    async fn serve(self: Arc<Self>, mut sock: TcpStream) -> Result<()> {
        // Read the request head (the player only sends small GET requests).
        let mut buf = Vec::with_capacity(2048);
        let mut chunk = [0u8; 1024];
        while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = sock.read(&mut chunk).await?;
            if n == 0 || buf.len() > 16 * 1024 {
                return Ok(());
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        let head = String::from_utf8_lossy(&buf).to_string();
        let mut lines = head.lines();
        let first = lines.next().unwrap_or_default();
        let path = first.split_whitespace().nth(1).unwrap_or_default().to_string();
        let range = lines
            .filter_map(|l| l.split_once(':'))
            .find(|(k, _)| k.trim().eq_ignore_ascii_case("range"))
            .map(|(_, v)| v.trim().to_string());
        let mut parts = path.trim_start_matches('/').splitn(2, '/');
        let (kind, mut token) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
        // Private mode's videos.
        if kind == "vault" || kind == "vaultlow" {
            let (quality, t) = if kind == "vaultlow" {
                let (q, t) = token.split_once('/').unwrap_or(("480", token));
                (q.parse().unwrap_or(480u32).clamp(240, 720), t)
            } else {
                (0, token)
            };
            let found = self.vault.lock().unwrap().get(t).cloned();
            let Some((file, name)) = found.filter(|_| crate::vault::get().is_unlocked()) else {
                sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
                return Ok(());
            };
            return if kind == "vault" { serve_vault(sock, file, &name, range.as_deref()).await } else { self.vault_low(sock, t, quality).await };
        }
        // /part/<token>/<n>: stream n of a video-site preview.
        let mut part = usize::MAX;
        if kind == "part" {
            let (t, n) = token.split_once('/').unwrap_or((token, ""));
            token = t;
            part = n.parse().unwrap_or(usize::MAX);
        }
        let entry = {
            let map = self.entries.lock().unwrap();
            map.get(token).map(|e| (e.source.clone(), e.name.clone(), e.quality))
        };
        let Some((source, name, quality)) = entry else {
            sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
            return Ok(());
        };
        match kind {
            "raw" => self.raw(sock, &source, &name, range.as_deref()).await,
            "low" => self.low(sock, token, quality).await,
            "site" => self.site(sock, token).await,
            "part" => self.part(sock, token, part, range.as_deref()).await,
            _ => {
                sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
                Ok(())
            }
        }
    }

    /// Passes the requested piece of the file through.
    async fn raw(&self, sock: TcpStream, source: &Source, name: &str, range: Option<&str>) -> Result<()> {
        let (from, to) = parse_range(range);
        let fetcher = Fetcher::new(self.http.clone(), source);
        let resp = match fetcher.get(from, to).await {
            Ok(r) => r,
            Err(e) => return refuse(sock, &e.message()).await,
        };
        pass_through(sock, resp, mime_for(name)).await
    }

    /// Converts on the fly with ffmpeg into a small MP4 the player can show as it arrives.
    async fn low(&self, mut sock: TcpStream, token: &str, quality: u32) -> Result<()> {
        let Some(ffmpeg) = crate::components::ffmpeg() else {
            sock.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
            return Ok(());
        };
        let input = format!("http://127.0.0.1:{}/raw/{token}", self.port);
        let mut cmd = tokio::process::Command::new(&ffmpeg);
        // From its own folder, never Pako's working folder (DLLs placed there aren't loaded).
        crate::components::in_own_folder_async(&mut cmd, AsRef::<std::path::Path>::as_ref(&ffmpeg));
        // The file comes from any link: ffmpeg may only read it through this proxy, and only
        // as a plain video file. Playlist formats (HLS, DASH, concat...) are refused, since
        // they would make ffmpeg open other addresses, e.g. on the local network.
        cmd.args(["-hide_banner", "-loglevel", "error"])
            .args(["-protocol_whitelist", "http,tcp", "-format_whitelist", VIDEO_DEMUXERS, "-i", &input])
            .args(small_mp4(quality));
        stream_ffmpeg(sock, cmd, false).await
    }

    /// A short clip of a video-site link, converted on the fly like /low.
    async fn site(&self, sock: TcpStream, token: &str) -> Result<()> {
        let clip = match self.clip(token).await {
            Ok(c) => c,
            Err(m) => return refuse(sock, &m).await,
        };
        let Some(ffmpeg) = crate::components::ffmpeg() else { return refuse(sock, &needs_tools()).await };
        let mut cmd = tokio::process::Command::new(&ffmpeg);
        // From its own folder, never Pako's working folder (DLLs placed there aren't loaded).
        crate::components::in_own_folder_async(&mut cmd, AsRef::<std::path::Path>::as_ref(&ffmpeg));
        cmd.args(site_args(&clip, self.port, token));
        // In Pako's job (it ends with Pako), without a window.
        stream_ffmpeg(sock, cmd, true).await
    }

    /// One stream of a video-site preview, piece by piece, with the site's headers.
    async fn part(&self, sock: TcpStream, token: &str, n: usize, range: Option<&str>) -> Result<()> {
        let clip = match self.clip(token).await {
            Ok(c) => c,
            Err(m) => return refuse(sock, &m).await,
        };
        let Some(input) = clip.inputs.get(n).filter(|i| !i.pieces) else { return refuse(sock, &tr!("No such stream.", "Ce flux n’existe pas.")).await };
        let mut req = self.http.get(&input.url);
        for (k, v) in &input.headers {
            req = req.header(k.as_str(), v.as_str());
        }
        if let Some(r) = range {
            req = req.header(reqwest::header::RANGE, r);
        }
        match req.send().await {
            Ok(resp) if resp.status().is_success() => pass_through(sock, resp, None).await,
            Ok(resp) => refuse(sock, &tr!("The site refused the preview ({}).", "Le site a refusé l’aperçu ({}).", resp.status().as_u16())).await,
            Err(_) => refuse(sock, &crate::model::say(crate::video::NO_SITE)).await,
        }
    }
}

impl Preview {
    /// A private video converted on the fly like /low, from the decrypted /vault address.
    async fn vault_low(&self, mut sock: TcpStream, token: &str, quality: u32) -> Result<()> {
        let Some(ffmpeg) = crate::components::ffmpeg() else {
            sock.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
            return Ok(());
        };
        let input = format!("http://127.0.0.1:{}/vault/{token}", self.port);
        let mut cmd = tokio::process::Command::new(&ffmpeg);
        // From its own folder, never Pako's working folder (DLLs placed there aren't loaded).
        crate::components::in_own_folder_async(&mut cmd, AsRef::<std::path::Path>::as_ref(&ffmpeg));
        cmd.args(["-hide_banner", "-loglevel", "error"])
            .args(["-protocol_whitelist", "http,tcp", "-format_whitelist", VIDEO_DEMUXERS, "-i", &input])
            .args(small_mp4(quality));
        stream_ffmpeg(sock, cmd, true).await
    }
}

/// Serves the asked piece of a private video, decrypted block by block.
async fn serve_vault(mut sock: TcpStream, file: std::path::PathBuf, name: &str, range: Option<&str>) -> Result<()> {
    let reader = match tauri::async_runtime::spawn_blocking(move || crate::vault::get().reader(&file)).await {
        Ok(Ok(r)) => Arc::new(r),
        _ => return refuse(sock, &tr!("This private video can't be read now.", "Cette vidéo privée ne peut pas être lue pour le moment.")).await,
    };
    let len = reader.len();
    let (from, to) = parse_range(range);
    if len == 0 || from >= len {
        sock.write_all(format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await?;
        return Ok(());
    }
    let to = to.unwrap_or(len - 1).min(len - 1);
    let ctype = mime_for(name).unwrap_or("application/octet-stream");
    let head = if range.is_some() {
        format!("HTTP/1.1 206 Partial Content\r\nContent-Type: {ctype}\r\nAccept-Ranges: bytes\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{to}/{len}\r\n\r\n", to - from + 1)
    } else {
        format!("HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nAccept-Ranges: bytes\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {len}\r\n\r\n")
    };
    sock.write_all(head.as_bytes()).await?;
    let mut at = from;
    while at <= to {
        // Private mode locked meanwhile: not one more decrypted byte (the player, or ffmpeg
        // converting it for /vault-low, stops here).
        if !crate::vault::get().is_unlocked() {
            break;
        }
        let n = ((to - at + 1) as usize).min(256 * 1024);
        let r = reader.clone();
        let chunk = tauri::async_runtime::spawn_blocking(move || {
            let mut buf = zeroize::Zeroizing::new(vec![0u8; n]);
            let got = r.read_at(&mut buf, at)?;
            buf.truncate(got);
            Ok::<_, std::io::Error>(buf)
        })
        .await;
        let Ok(Ok(buf)) = chunk else { break };
        if buf.is_empty() {
            break;
        }
        // The player closed this piece (seeked elsewhere): stop quietly.
        if sock.write_all(&buf).await.is_err() {
            break;
        }
        at += buf.len() as u64;
    }
    Ok(())
}

/// A plain-text error for the player (or for ffmpeg reading /part).
async fn refuse(mut sock: TcpStream, msg: &str) -> Result<()> {
    let head = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", msg.len());
    sock.write_all(head.as_bytes()).await?;
    sock.write_all(msg.as_bytes()).await?;
    Ok(())
}

/// Sends a web answer on to the player as it comes (its type from `ctype`, else the server's).
async fn pass_through(mut sock: TcpStream, resp: reqwest::Response, ctype: Option<&str>) -> Result<()> {
    let partial = resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let h = resp.headers();
    let get = |k: reqwest::header::HeaderName| h.get(k).and_then(|v| v.to_str().ok()).map(String::from);
    let ctype = ctype
        .map(String::from)
        .or_else(|| get(reqwest::header::CONTENT_TYPE).filter(|t| !is_playlist_type(t)))
        .unwrap_or_else(|| "application/octet-stream".into());
    let mut head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {ctype}\r\nAccept-Ranges: bytes\r\nCache-Control: no-store\r\nConnection: close\r\n",
        if partial { "206 Partial Content" } else { "200 OK" }
    );
    if let Some(len) = get(reqwest::header::CONTENT_LENGTH) {
        head.push_str(&format!("Content-Length: {len}\r\n"));
    }
    if let Some(cr) = get(reqwest::header::CONTENT_RANGE) {
        head.push_str(&format!("Content-Range: {cr}\r\n"));
    }
    head.push_str("\r\n");
    sock.write_all(head.as_bytes()).await?;
    let mut body = resp.bytes_stream();
    while let Some(chunk) = body.next().await {
        // The player closed this piece (seeked elsewhere): stop quietly.
        if sock.write_all(&chunk?).await.is_err() {
            break;
        }
    }
    Ok(())
}

/// Runs ffmpeg and sends what it makes to the player until it ends or the player goes
/// (another item picked, the panel closed): ffmpeg is then stopped.
async fn stream_ffmpeg(mut sock: TcpStream, mut cmd: tokio::process::Command, in_job: bool) -> Result<()> {
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = if in_job {
        crate::video::spawn(&mut cmd)?
    } else {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.as_std_mut().creation_flags(0x0800_0000);
        }
        cmd.spawn()?
    };
    let Some(mut out) = child.stdout.take() else {
        let _ = child.kill().await;
        return Ok(());
    };
    sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").await?;
    let (mut from_player, mut to_player) = sock.split();
    let mut buf = vec![0u8; 64 * 1024];
    let mut ignored = [0u8; 1024];
    let started = Instant::now();
    loop {
        tokio::select! {
            n = out.read(&mut buf) => {
                let n = n.unwrap_or(0);
                if n == 0 || to_player.write_all(&buf[..n]).await.is_err() {
                    break;
                }
            }
            // The player went (another item picked, the panel closed) while ffmpeg was still
            // waiting for the site: it stops now, not whenever the site answers.
            n = from_player.read(&mut ignored) => {
                if n.unwrap_or(0) == 0 {
                    break;
                }
            }
            // A video-site clip never takes longer than this (it takes seconds): a site that
            // stops answering half-way doesn't keep ffmpeg. (/low plays a whole file: as
            // long as the player wants it.)
            _ = tokio::time::sleep(CLIP_LIMIT.saturating_sub(started.elapsed())), if in_job => break,
        }
    }
    let _ = child.kill().await;
    Ok(())
}

/// The longest a video-site clip is made for.
const CLIP_LIMIT: Duration = Duration::from_secs(5 * 60);

/// ffmpeg's output settings: a small MP4 at most `lines` high that plays as it arrives.
fn small_mp4(lines: u32) -> Vec<String> {
    let scale = format!("scale=-2:'min({lines},ih)'");
    [
        "-vf", scale.as_str(),
        "-c:v", "libx264", "-preset", "veryfast", "-crf", "28",
        "-c:a", "aac", "-b:a", "128k", "-ac", "2",
        "-movflags", "frag_keyframe+empty_moov+default_base_moof", "-f", "mp4", "pipe:1",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// ffmpeg's arguments for a video-site clip: each stream from `start` (plain files through
/// /part, HLS pieces read directly with the site's headers), `CLIP_SECONDS` long.
fn site_args(clip: &Clip, port: u16, token: &str) -> Vec<String> {
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error"].iter().map(|s| s.to_string()).collect();
    for (n, input) in clip.inputs.iter().enumerate() {
        if clip.start > 0 {
            a.extend(["-ss".into(), clip.start.to_string()]);
        }
        // A site (or this PC's /part) that stops answering: ffmpeg gives up after 20 s instead
        // of waiting for ever.
        a.extend(["-rw_timeout".into(), "20000000".into()]);
        if input.pieces {
            // Only web addresses, nothing on this PC. Never a cookie on the command line
            // (`parse_clip` refuses those).
            a.extend(["-protocol_whitelist".into(), "http,https,tls,tcp,crypto".into()]);
            let headers: String = input.headers.iter().map(|(k, v)| format!("{}: {}\r\n", clean(k), clean(v))).collect();
            if !headers.is_empty() {
                a.extend(["-headers".into(), headers]);
            }
            a.extend(["-i".into(), input.url.clone()]);
        } else {
            a.extend(["-protocol_whitelist".into(), "http,tcp".into(), "-format_whitelist".into(), format!("{VIDEO_DEMUXERS},mp3,aac")]);
            a.extend(["-i".into(), format!("http://127.0.0.1:{port}/part/{token}/{n}")]);
        }
    }
    // Picture from the first stream that has it; sound from a sound-only stream when there is
    // one (picture + sound picked separately), else from the picture's.
    let v = clip.inputs.iter().position(|i| i.video).unwrap_or(0);
    let s = clip.inputs.iter().position(|i| i.audio && !i.video).unwrap_or(v);
    a.extend(["-map".into(), format!("{v}:v:0"), "-map".into(), format!("{s}:a:0?")]);
    a.extend(["-t".into(), CLIP_SECONDS.to_string()]);
    a.extend(small_mp4(SITE_LINES));
    a
}

/// A header name or value without line breaks (they would start another header).
fn clean(s: &str) -> String {
    s.chars().filter(|c| *c != '\r' && *c != '\n').collect()
}

fn cant(why: &str) -> String {
    tr!("This video can't be previewed. {why}", "Pas d’aperçu possible pour cette vidéo. {why}")
}

/// Asks yt-dlp for a small stream of a video-site link (nothing is downloaded).
async fn find_clip(source: &Source, owner: Option<&str>) -> Result<Clip, String> {
    let Source::Video { url, referrer, recording, .. } = source else { return Err(cant(&tr!("It isn't a video-site link.", "Ce n’est pas un lien de site vidéo."))) };
    if recording.is_some() {
        return Err(cant(&tr!("It is recorded from the page as it plays.", "Elle est enregistrée depuis la page pendant la lecture.")));
    }
    let (Some(t), Some(_)) = (crate::video::tools(), crate::components::ffmpeg()) else { return Err(needs_tools()) };
    // The browser's login for this video, when it came from the browser (deleted right after).
    let jar = owner.and_then(crate::video::cookie_file_for);
    let mut cmd = tokio::process::Command::new(&t.yt_dlp);
    // From its own folder, never Pako's working folder (DLLs placed there aren't loaded).
    crate::components::in_own_folder_async(&mut cmd, AsRef::<std::path::Path>::as_ref(&t.yt_dlp));
    cmd.env("PYTHONIOENCODING", "utf-8").env("PYTHONUTF8", "1");
    cmd.args(["--no-config", "--no-update", "--encoding", "utf-8"]);
    if let Some(deno) = &t.deno {
        cmd.arg("--js-runtimes").arg(format!("deno:{}", deno.display()));
        // Deno's cache next to Pako's tools (else %LOCALAPPDATA%\deno, left after uninstalling).
        cmd.env("DENO_DIR", crate::components::dir().join("deno-cache"));
    }
    if let Some(ff) = &t.ffmpeg {
        cmd.arg("--ffmpeg-location").arg(ff);
    }
    if let Some(r) = referrer.as_deref().filter(|r| r.starts_with("http://") || r.starts_with("https://")) {
        cmd.arg("--referer").arg(r);
    }
    if let Some(j) = &jar {
        cmd.arg("--cookies").arg(j.path());
    }
    cmd.args(["-J", "--no-playlist", "--no-warnings", "-f", SITE_FORMAT]).arg("--").arg(url);
    cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).kill_on_drop(true);
    let child = crate::video::spawn(&mut cmd).map_err(|e| tr!("Couldn't start the video tool: {e}", "Impossible de lancer l’outil vidéo : {e}"))?;
    let pid = child.id();
    // Kept here (not inside a timeout) so that yt-dlp's launcher still runs when the time is up:
    // the tree is ended from it first (its Python part, Deno). Dropping it first would end only
    // the launcher, and what it started would stay behind.
    let wait = child.wait_with_output();
    tokio::pin!(wait);
    let out = tokio::select! {
        r = &mut wait => match r {
            Ok(o) => o,
            Err(e) => return Err(tr!("The video tool stopped: {e}", "L’outil vidéo s’est arrêté : {e}")),
        },
        _ = tokio::time::sleep(FIND_LIMIT) => {
            if let Some(pid) = pid {
                let _ = tokio::task::spawn_blocking(move || crate::video::kill_tree(pid)).await;
            }
            return Err(tr!("The site took too long to answer.", "Le site a mis trop de temps à répondre."));
        }
    };
    drop(jar);
    if !out.status.success() {
        return Err(site_error(&String::from_utf8_lossy(&out.stderr)));
    }
    parse_clip(&String::from_utf8_lossy(&out.stdout))
}

/// yt-dlp's error in plain words: "This video can't be previewed." and its reason (private,
/// age-restricted, DRM...); network trouble and site changes as they are.
fn site_error(stderr: &str) -> String {
    let lower = stderr.to_lowercase();
    if lower.contains("live event will begin") || lower.contains("is upcoming") || lower.contains("premiere") {
        return crate::video::friendly_error(stderr).0;
    }
    if lower.contains("is live") || lower.contains("live stream") || lower.contains("livestream") {
        return live();
    }
    let (msg, _) = crate::video::friendly_error(stderr);
    if crate::video::is_network(&msg) || crate::model::is_said(&msg, crate::video::SITE_CHANGED) {
        msg
    } else {
        cant(&msg)
    }
}

#[derive(serde::Deserialize, Default)]
struct ClipFormat {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    protocol: Option<String>,
    #[serde(default)]
    http_headers: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    cookies: Option<String>,
    #[serde(default)]
    vcodec: Option<String>,
    #[serde(default)]
    acodec: Option<String>,
    #[serde(default)]
    has_drm: Option<serde_json::Value>,
    #[serde(default)]
    fragments: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct ClipInfo {
    #[serde(default)]
    is_live: Option<bool>,
    #[serde(default)]
    live_status: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    requested_formats: Option<Vec<ClipFormat>>,
    #[serde(flatten)]
    top: ClipFormat,
}

/// Reads yt-dlp's answer (`-J -f SITE_FORMAT`) into the streams ffmpeg reads.
fn parse_clip(json: &str) -> Result<Clip, String> {
    let info: ClipInfo = serde_json::from_str(json).map_err(|_| crate::model::say(crate::video::UNREADABLE))?;
    if info.is_live == Some(true) || info.live_status.as_deref() == Some("is_live") {
        return Err(live());
    }
    let formats = match info.requested_formats {
        Some(list) if !list.is_empty() => list,
        _ => vec![info.top],
    };
    if formats.iter().any(|f| f.has_drm.as_ref().is_some_and(|v| v.as_bool() == Some(true))) {
        return Err(cant(&crate::model::say(crate::video::DRM)));
    }
    let unreadable = || cant(&tr!("The site sends it in a way the preview can't read.", "Le site l’envoie d’une façon que l’aperçu ne sait pas lire."));
    let mut inputs = Vec::new();
    for f in formats {
        let Some(url) = f.url.filter(|u| u.starts_with("https://") || u.starts_with("http://")) else { return Err(unreadable()) };
        let pieces = match f.protocol.as_deref().unwrap_or("https") {
            "http" | "https" if f.fragments.is_none() => false,
            "m3u8" | "m3u8_native" => true,
            _ => return Err(unreadable()),
        };
        let mut headers: Vec<(String, String)> = f.http_headers.unwrap_or_default().into_iter().collect();
        if let Some(c) = f.cookies.as_deref().map(crate::video::cookie_header).filter(|c| !c.is_empty()) {
            headers.retain(|(k, _)| !k.eq_ignore_ascii_case("cookie"));
            headers.push(("Cookie".into(), c));
        }
        // ffmpeg reads HLS pieces itself: a login would have to go on its command line, where
        // other programs can read it. Not done.
        if pieces && headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("cookie")) {
            return Err(cant(&tr!("It needs your browser login, which the preview doesn't pass on.", "Elle a besoin de la connexion de votre navigateur, que l’aperçu ne transmet pas.")));
        }
        let has = |c: &Option<String>| c.as_deref() != Some("none");
        inputs.push(Input { url, headers, pieces, video: has(&f.vcodec), audio: has(&f.acodec) });
    }
    if !inputs.iter().any(|i| i.video) {
        return Err(cant(&tr!("It has no picture.", "Elle n’a pas d’image.")));
    }
    // Longer videos: a few seconds in, past a dark intro.
    let start = if info.duration.is_some_and(|d| d >= 60.0) { 5 } else { 0 };
    Ok(Clip { inputs, start })
}

/// "bytes=100-" -> (100, None); "bytes=100-199" -> (100, Some(199)); none -> whole file.
fn parse_range(h: Option<&str>) -> (u64, Option<u64>) {
    let Some(spec) = h.and_then(|v| v.strip_prefix("bytes=")) else { return (0, None) };
    let (a, b) = spec.split_once('-').unwrap_or((spec, ""));
    (a.trim().parse().unwrap_or(0), b.trim().parse().ok())
}

/// ffmpeg's readers for the formats `is_video` accepts (MP4/MOV/3GP, MKV/WebM, AVI, WMV,
/// FLV, MPEG-TS, MPEG-PS); none of them opens other files or addresses.
const VIDEO_DEMUXERS: &str = "mov,matroska,avi,asf,flv,mpegts,mpeg";

/// Playlist types (HLS, DASH) that name other addresses to fetch.
fn is_playlist_type(ctype: &str) -> bool {
    let t = ctype.to_ascii_lowercase();
    t.contains("mpegurl") || t.contains("dash+xml")
}

fn mime_for(name: &str) -> Option<&'static str> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ranges_and_formats() {
        assert_eq!(parse_range(Some("bytes=100-")), (100, None));
        assert_eq!(parse_range(Some("bytes=0-499")), (0, Some(499)));
        assert_eq!(parse_range(None), (0, None));
        assert!(plays_directly("clip.MP4"));
        assert!(!plays_directly("film.mkv"));
        assert!(is_video("film.mkv"));
        assert!(!is_video("notes.txt"));
        assert!(is_playlist_type("application/vnd.apple.mpegURL"));
        assert!(is_playlist_type("audio/x-mpegurl; charset=utf-8"));
        assert!(is_playlist_type("application/dash+xml"));
        assert!(!is_playlist_type("video/mp4"));
    }

    fn video(url: &str) -> Source {
        serde_json::from_value(serde_json::json!({ "kind": "video", "url": url })).unwrap()
    }

    #[tokio::test]
    async fn video_sites_get_a_short_clip_address() {
        let p = Preview { port: 1, http: reqwest::Client::new(), entries: Mutex::new(HashMap::new()), vault: Mutex::new(HashMap::new()) };
        let site = p.register_item(Some("d1"), video("https://www.youtube.com/watch?v=x"), "Song.mp4".into(), 720, true);
        assert!(site.contains("/site/"), "{site}");
        let low = p.register_item(None, Source::Http { url: "https://example.com/a.mkv".into() }, "a.mkv".into(), 720, true);
        assert!(low.contains("/low/"));
        let entries = p.entries.lock().unwrap();
        let token = site.rsplit('/').next().unwrap();
        let e = entries.get(token).unwrap();
        // Fixed at 480p, whatever the setting says; the download's id kept for its login.
        assert_eq!(e.quality, 480);
        assert_eq!(e.owner.as_deref(), Some("d1"));
        assert!(e.site.is_some());
        drop(entries);
        // A gone preview says so (nothing is asked to yt-dlp).
        assert!(p.check("http://127.0.0.1:1/site/nope").await.unwrap_err().contains("gone"));
    }

    #[test]
    fn reads_the_streams_yt_dlp_found() {
        // One small file with picture and sound.
        let clip = parse_clip(r#"{"duration": 212, "url": "https://rr1.example/v.mp4", "protocol": "https",
            "http_headers": {"User-Agent": "UA"}, "vcodec": "avc1", "acodec": "mp4a"}"#)
        .unwrap();
        assert_eq!(clip.start, 5);
        assert_eq!(clip.inputs.len(), 1);
        assert!(!clip.inputs[0].pieces && clip.inputs[0].video && clip.inputs[0].audio);
        // Picture and sound apart, with the browser's login (kept for /part, never ffmpeg's).
        let clip = parse_clip(r#"{"duration": 20, "requested_formats": [
            {"url": "https://x/v", "protocol": "https", "vcodec": "avc1", "acodec": "none", "cookies": "SID=a; Domain=.x; Path=/"},
            {"url": "https://x/a", "protocol": "https", "vcodec": "none", "acodec": "opus"}]}"#)
        .unwrap();
        assert_eq!(clip.start, 0);
        assert!(clip.inputs[0].headers.contains(&("Cookie".into(), "SID=a".into())));
        let args = site_args(&clip, 4000, "tok");
        assert!(args.contains(&"http://127.0.0.1:4000/part/tok/0".to_string()));
        assert!(args.contains(&"http://127.0.0.1:4000/part/tok/1".to_string()));
        assert!(!args.iter().any(|a| a.contains("SID")));
        assert!(args.windows(2).any(|w| w == ["-map", "0:v:0"]));
        assert!(args.windows(2).any(|w| w == ["-map", "1:a:0?"]));
        assert!(args.windows(2).any(|w| w == ["-t", "20"]));
        assert!(args.iter().any(|a| a.contains("min(480,ih)")));
        // Each input gives up on a site that stops answering.
        assert_eq!(args.windows(2).filter(|w| w == &["-rw_timeout", "20000000"]).count(), 2);
        // HLS: ffmpeg reads the pieces with the site's headers.
        let clip = parse_clip(r#"{"url": "https://x/index.m3u8", "protocol": "m3u8_native", "http_headers": {"Referer": "https://x/\r\nEvil: 1"}}"#).unwrap();
        let args = site_args(&clip, 4000, "tok");
        assert!(args.contains(&"https://x/index.m3u8".to_string()));
        assert!(args.contains(&"Referer: https://x/Evil: 1\r\n".to_string()));
        assert!(!args.contains(&"-ss".to_string()));
    }

    /// The player going away stops the converter at once, even while it is still waiting for
    /// the site (nothing to send yet).
    #[cfg(windows)]
    #[tokio::test]
    async fn the_player_leaving_stops_a_converter_that_sends_nothing() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let player = tokio::spawn(async move {
            let mut s = TcpStream::connect(addr).await.unwrap();
            let mut head = [0u8; 64];
            let _ = s.read(&mut head).await;
            // Gone (another item picked).
        });
        let (sock, _) = listener.accept().await.unwrap();
        // Stands for an ffmpeg waiting on a site that doesn't answer: no output for a long time.
        let mut cmd = tokio::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"]);
        let started = Instant::now();
        tokio::time::timeout(Duration::from_secs(20), stream_ffmpeg(sock, cmd, false)).await.expect("stopped when the player left").unwrap();
        assert!(started.elapsed() < Duration::from_secs(20));
        player.await.unwrap();
    }

    #[test]
    fn says_why_a_video_cant_be_previewed() {
        assert_eq!(parse_clip(r#"{"is_live": true, "url": "https://x/l.m3u8", "protocol": "m3u8"}"#).unwrap_err(), live());
        assert!(parse_clip(r#"{"url": "https://x/v", "protocol": "https", "has_drm": true}"#).unwrap_err().starts_with("This video can't be previewed."));
        assert!(parse_clip(r#"{"url": "https://x/m.mpd", "protocol": "http_dash_segments"}"#).unwrap_err().contains("can't read"));
        assert!(parse_clip(r#"{"url": "https://x/i.m3u8", "protocol": "m3u8", "cookies": "a=1"}"#).unwrap_err().contains("login"));
        assert!(parse_clip(r#"{"url": "file:///c/x.mp4", "protocol": "file"}"#).is_err());
        assert!(parse_clip(r#"{"url": "https://x/a", "protocol": "https", "vcodec": "none", "acodec": "mp4a"}"#).unwrap_err().contains("no picture"));
        assert_eq!(site_error("ERROR: [youtube] abc: Private video. Sign in"), "This video can't be previewed. This video is private.");
        assert!(site_error("ERROR: [youtube] abc: Sign in to confirm your age").starts_with("This video can't be previewed. Age-restricted"));
        assert_eq!(site_error("ERROR: [twitch:stream] x: The channel is live stream only"), live());
        assert_eq!(site_error("ERROR: Unable to download webpage: timed out"), "No connection to the site.");
    }
}
