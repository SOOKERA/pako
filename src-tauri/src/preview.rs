// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Video preview before (or while) downloading: a tiny web server on this PC only
//! (127.0.0.1) that the details panel's player reads from.
//!
//! - /raw/<token>: the file itself, piece by piece (the player asks for the parts it
//!   needs). Drive files come through Google's public download address.
//! - /low/<token>: the same video converted on the fly by ffmpeg to a small MP4
//!   (480p by default), so MKV, AVI and the rest play too and little data is used.

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
}

pub struct Preview {
    port: u16,
    http: reqwest::Client,
    entries: Mutex<HashMap<String, Entry>>,
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
        let me = Arc::new(Self { port: listener.local_addr()?.port(), http, entries: Mutex::new(HashMap::new()) });
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

    /// A player address for this file. `convert` = through ffmpeg at `quality` lines.
    pub fn register(&self, source: Source, name: String, quality: u32, convert: bool) -> String {
        let token = uuid::Uuid::new_v4().simple().to_string();
        let mut map = self.entries.lock().unwrap();
        map.retain(|_, e| e.created.elapsed() < Duration::from_secs(6 * 3600));
        map.insert(token.clone(), Entry { source, name, quality: quality.clamp(240, 720), created: Instant::now() });
        let kind = if convert { "low" } else { "raw" };
        format!("http://127.0.0.1:{}/{kind}/{token}", self.port)
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
        let (kind, token) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
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
            _ => {
                sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
                Ok(())
            }
        }
    }

    /// Passes the requested piece of the file through.
    async fn raw(&self, mut sock: TcpStream, source: &Source, name: &str, range: Option<&str>) -> Result<()> {
        let (from, to) = parse_range(range);
        let fetcher = Fetcher::new(self.http.clone(), source);
        let resp = match fetcher.get(from, to).await {
            Ok(r) => r,
            Err(e) => {
                let msg = e.message();
                let head = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", msg.len());
                sock.write_all(head.as_bytes()).await?;
                sock.write_all(msg.as_bytes()).await?;
                return Ok(());
            }
        };
        let partial = resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;
        let h = resp.headers();
        let get = |k: reqwest::header::HeaderName| h.get(k).and_then(|v| v.to_str().ok()).map(String::from);
        let ctype = mime_for(name)
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

    /// Converts on the fly with ffmpeg into a small MP4 the player can show as it arrives.
    async fn low(&self, mut sock: TcpStream, token: &str, quality: u32) -> Result<()> {
        let Some(ffmpeg) = crate::components::ffmpeg() else {
            sock.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
            return Ok(());
        };
        let input = format!("http://127.0.0.1:{}/raw/{token}", self.port);
        let mut cmd = tokio::process::Command::new(ffmpeg);
        // The file comes from any link: ffmpeg may only read it through this proxy, and only
        // as a plain video file. Playlist formats (HLS, DASH, concat...) are refused, since
        // they would make ffmpeg open other addresses, e.g. on the local network.
        cmd.args(["-hide_banner", "-loglevel", "error"])
            .args(["-protocol_whitelist", "http,tcp", "-format_whitelist", VIDEO_DEMUXERS, "-i", &input])
            .args(["-vf", &format!("scale=-2:'min({quality},ih)'")])
            .args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "28"])
            .args(["-c:a", "aac", "-b:a", "128k", "-ac", "2"])
            .args(["-movflags", "frag_keyframe+empty_moov+default_base_moof", "-f", "mp4", "pipe:1"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.as_std_mut().creation_flags(0x0800_0000);
        }
        let mut child = cmd.spawn()?;
        let mut out = child.stdout.take().expect("piped");
        sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").await?;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = out.read(&mut buf).await?;
            if n == 0 || sock.write_all(&buf[..n]).await.is_err() {
                break;
            }
        }
        let _ = child.kill().await;
        Ok(())
    }
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
}
