// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Sending with your own Google Drive (the "Pako Send" sign-in, which only sees the files Pako
//! uploads itself).
//!
//! Everything goes into one folder of your Drive, "Pako uploads" (or "Pako sends", made by
//! earlier versions, if it is already there). One file (or one ZIP) is
//! uploaded there and shared as "anyone with the link can view"; several files sent as they
//! are go into a folder of their own (named after the send), shared the same way, up to three
//! files at a time. Uploads use Google's resumable upload: the bytes go in 16 MB pieces, and
//! after a cut connection Pako asks Google how much arrived and carries on from there (the
//! piece being sent is kept in memory until Google confirms it).

use super::*;
use crate::google::Google;

/// One piece of an upload: Google wants a multiple of 256 KB (except the last piece).
pub(super) const CHUNK: usize = 16 * 1024 * 1024;
const API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD_API: &str = "https://www.googleapis.com/upload/drive/v3/files";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
/// The folder of your Drive everything Pako uploads goes into.
const UPLOADS_FOLDER: &str = "Pako uploads";
/// Its name before 0.2: still used when it's there, so the links and "Make room" keep working
/// for what was uploaded into it.
const OLD_UPLOADS_FOLDER: &str = "Pako sends";
/// Tries after a cut connection or Google being busy, per piece.
const PIECE_TRIES: u32 = 6;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct DriveFile {
    pub id: String,
    pub web_view_link: Option<String>,
}

/// The `Content-Range` of one piece. `len` 0 = no bytes, only saying the total ("bytes */N").
/// The total is "*" until the last piece, when the size wasn't known up front.
pub(super) fn content_range(start: u64, len: u64, total: Option<u64>) -> String {
    let whole = total.map(|t| t.to_string()).unwrap_or_else(|| "*".into());
    if len == 0 {
        format!("bytes */{whole}")
    } else {
        format!("bytes {start}-{}/{whole}", start + len - 1)
    }
}

/// Where the upload stands from Google's `Range: bytes=0-N` answer: the next byte to send.
/// No `Range` = nothing arrived yet.
pub(super) fn next_offset(range: Option<&str>) -> Option<u64> {
    let Some(r) = range else { return Some(0) };
    let end = r.trim().strip_prefix("bytes=")?.split_once('-')?.1;
    end.trim().parse::<u64>().ok().map(|n| n + 1)
}

/// "Anyone with the link can view".
pub(super) fn share_body() -> serde_json::Value {
    serde_json::json!({ "type": "anyone", "role": "reader" })
}

/// Drive search for Pako's folder at the top of your Drive, under its name or its old one.
pub(super) fn folders_query() -> String {
    format!("(name = '{UPLOADS_FOLDER}' or name = '{OLD_UPLOADS_FOLDER}') and mimeType = '{FOLDER_MIME}' and trashed = false and 'root' in parents")
}

/// Which found folder to use: "Pako uploads" if there is one, else an earlier "Pako sends"
/// (so nothing already uploaded is left behind). None = make "Pako uploads".
pub(super) fn pick_folder(found: &serde_json::Value) -> Option<String> {
    let list = found["files"].as_array()?;
    let named = |want: &str| list.iter().find(|f| f["name"].as_str() == Some(want)).and_then(|f| f["id"].as_str()).map(str::to_string);
    named(UPLOADS_FOLDER).or_else(|| named(OLD_UPLOADS_FOLDER))
}

pub(super) fn folder_body(name: &str, parent: Option<&str>) -> serde_json::Value {
    let mut v = serde_json::json!({ "name": name, "mimeType": FOLDER_MIME });
    if let Some(p) = parent {
        v["parents"] = serde_json::json!([p]);
    }
    v
}

/// Sizes in plain words ("4.2 GB").
pub(super) fn human(n: u64) -> String {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1000.0 && i < UNITS.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 { format!("{n} bytes") } else if v >= 100.0 { format!("{v:.0} {}", UNITS[i]) } else { format!("{v:.1} {}", UNITS[i]) }
}

/// Google's reason in an error answer ("storageQuotaExceeded", …).
fn reason(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let e = v.get("error")?;
    e.get("errors")
        .and_then(|l| l.get(0))
        .and_then(|x| x.get("reason"))
        .or_else(|| e.get("status"))
        .and_then(|r| r.as_str())
        .map(str::to_string)
}

/// Google asks to slow down (429, or 403 with a rate-limit reason): worth trying again later.
fn rate_limited(code: u16, body: &str) -> bool {
    code == 429 || (code == 403 && matches!(reason(body).as_deref(), Some("userRateLimitExceeded" | "rateLimitExceeded")))
}

/// A refused call. "Slow down" carries 429 whatever the code, so it's retried like one.
fn drive_fail(code: u16, body: &str) -> Fail {
    Fail { msg: drive_error(code, body), code: Some(if rate_limited(code, body) { 429 } else { code }) }
}

/// Google's answer in plain words.
pub(super) fn drive_error(code: u16, body: &str) -> String {
    match (code, reason(body).as_deref()) {
        (_, Some("storageQuotaExceeded" | "quotaExceeded")) => {
            "Your Google Drive is full. Make room (delete older uploads), then try again.".into()
        }
        (403, Some("userRateLimitExceeded" | "rateLimitExceeded")) | (429, _) => {
            "Google asks to slow down. Try again in a few minutes.".into()
        }
        (401, _) => "Your Google Drive login expired. Connect Google Drive again.".into(),
        (403, _) => "Google refused this. Connect Google Drive again, then try again.".into(),
        (404, _) => "Google Drive doesn't have it any more.".into(),
        (c, _) if c >= 500 => "Google Drive is busy right now. Try again in a few minutes.".into(),
        (c, _) => format!("Google Drive refused it ({c})."),
    }
}

fn drive_net_error(e: &reqwest::Error) -> String {
    if e.is_connect() {
        "No connection to Google Drive. Check your internet, then try again.".into()
    } else if e.is_timeout() {
        "Google Drive stopped answering. Try again in a few minutes.".into()
    } else {
        "The upload was cut off. Try again.".into()
    }
}

/// What one piece's request came back with.
enum Put {
    Done(DriveFile),
    /// Google has the bytes up to here (the next byte to send).
    More(u64),
    /// Cut connection, Google busy, login to renew: ask where it stands, then go on.
    Retry(String),
    Fail(Fail),
}

pub(super) struct Drive<'a> {
    pub http: &'a reqwest::Client,
    pub google: &'a Google,
}

impl Drive<'_> {
    async fn token(&self, force: bool) -> Result<String, Fail> {
        match self.google.access_token(force).await {
            Ok(Some(t)) => Ok(t),
            Ok(None) => Err(Fail::msg("Connect your Google Drive first (Upload › Google Drive).")),
            Err(e) => Err(Fail::msg(e.to_string())),
        }
    }

    /// One API call with a JSON body (or none); a refused login is renewed once.
    async fn call(&self, method: reqwest::Method, url: &str, body: Option<&serde_json::Value>) -> Result<(u16, String), Fail> {
        let mut force = false;
        loop {
            let token = self.token(force).await?;
            let mut req = self.http.request(method.clone(), url).bearer_auth(token).timeout(Duration::from_secs(60));
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await.map_err(|e| Fail::msg(drive_net_error(&e)))?;
            let code = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            if code == 401 && !force {
                force = true;
                continue;
            }
            return Ok((code, text));
        }
    }

    async fn call_ok(&self, method: reqwest::Method, url: &str, body: Option<&serde_json::Value>) -> Result<String, Fail> {
        let (code, text) = self.call(method, url, body).await?;
        if (200..300).contains(&code) { Ok(text) } else { Err(drive_fail(code, &text)) }
    }

    /// Your Drive's storage: (limit, used). No limit = some Workspace accounts.
    pub async fn quota(&self) -> Result<(Option<u64>, u64), Fail> {
        let text = self.call_ok(reqwest::Method::GET, &format!("{API}/about?fields=storageQuota(limit,usage)"), None).await?;
        let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| Fail::msg("Google Drive answered something Pako doesn't understand."))?;
        let num = |k: &str| v["storageQuota"][k].as_str().and_then(|s| s.parse::<u64>().ok());
        Ok((num("limit"), num("usage").unwrap_or(0)))
    }

    /// The "Pako uploads" folder (made the first time), or the "Pako sends" folder an earlier
    /// version made. Pako only sees folders it made itself.
    pub async fn sends_folder(&self) -> Result<String, Fail> {
        let q = folders_query();
        let q = percent_encoding::utf8_percent_encode(&q, percent_encoding::NON_ALPHANUMERIC).to_string();
        let text = self.call_ok(reqwest::Method::GET, &format!("{API}/files?q={q}&fields=files(id,name)&pageSize=10"), None).await?;
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        if let Some(id) = pick_folder(&v) {
            return Ok(id);
        }
        Ok(self.create_folder(UPLOADS_FOLDER, None).await?.id)
    }

    pub async fn create_folder(&self, name: &str, parent: Option<&str>) -> Result<DriveFile, Fail> {
        let text = self.call_ok(reqwest::Method::POST, &format!("{API}/files?fields=id,webViewLink"), Some(&folder_body(name, parent))).await?;
        serde_json::from_str(&text).map_err(|_| Fail::msg("Google Drive didn't say which folder it made."))
    }

    /// Anyone with the link can view it.
    pub async fn share(&self, id: &str) -> Result<(), Fail> {
        self.call_ok(reqwest::Method::POST, &format!("{API}/files/{id}/permissions?fields=id"), Some(&share_body())).await.map(|_| ())
    }

    /// Shares a file or folder Pako just made and returns its link. If that doesn't work it is
    /// deleted again, so nothing is left in your Drive that isn't on the Sent list.
    pub async fn share_and_link(&self, file: &DriveFile) -> Result<String, String> {
        let shared = match self.share(&file.id).await {
            Ok(()) => self.link(file).await,
            Err(f) => Err(f),
        };
        match shared {
            Ok(link) => Ok(link),
            Err(f) if self.delete(&file.id).await.is_ok() => Err(f.msg),
            Err(f) => Err(format!("{} It stays in Pako's folder of your Google Drive (\"{UPLOADS_FOLDER}\", or \"{OLD_UPLOADS_FOLDER}\" from an earlier Pako): delete it there if you don't need it.", f.msg)),
        }
    }

    /// The link people open (Google's own page for the file or folder).
    pub async fn link(&self, file: &DriveFile) -> Result<String, Fail> {
        if let Some(l) = &file.web_view_link {
            return Ok(l.clone());
        }
        let text = self.call_ok(reqwest::Method::GET, &format!("{API}/files/{}?fields=webViewLink", file.id), None).await?;
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        v["webViewLink"].as_str().map(str::to_string).ok_or_else(|| Fail::msg("Google Drive didn't send the link back."))
    }

    /// Deletes it for good (a folder with everything in it). Already gone is fine.
    pub async fn delete(&self, id: &str) -> Result<(), Fail> {
        let (code, text) = self.call(reqwest::Method::DELETE, &format!("{API}/files/{id}"), None).await?;
        // Drive answers a delete with no body: the code says it all.
        if super::storage::deleted(code, true) { Ok(()) } else { Err(drive_fail(code, &text)) }
    }

    /// Opens a resumable upload; returns where the bytes go.
    pub async fn start_upload(&self, name: &str, parent: &str, total: Option<u64>) -> Result<String, Fail> {
        let meta = serde_json::json!({ "name": name, "parents": [parent] });
        let mut force = false;
        loop {
            let token = self.token(force).await?;
            let mut req = self
                .http
                .post(format!("{UPLOAD_API}?uploadType=resumable&fields=id,webViewLink"))
                .bearer_auth(token)
                .header("X-Upload-Content-Type", "application/octet-stream")
                .timeout(Duration::from_secs(60))
                .json(&meta);
            if let Some(t) = total {
                req = req.header("X-Upload-Content-Length", t.to_string());
            }
            let resp = req.send().await.map_err(|e| Fail::msg(drive_net_error(&e)))?;
            let code = resp.status().as_u16();
            if code == 401 && !force {
                force = true;
                continue;
            }
            let location = resp.headers().get(reqwest::header::LOCATION).and_then(|v| v.to_str().ok()).map(str::to_string);
            let text = resp.text().await.unwrap_or_default();
            if !(200..300).contains(&code) {
                return Err(drive_fail(code, &text));
            }
            return location.ok_or_else(|| Fail::msg("Google Drive didn't open the upload."));
        }
    }

    /// Sends one piece (or, with no bytes, asks where the upload stands). `sent` counts the bytes
    /// as they go out to the network (what the speed shows), slice by slice.
    async fn put(&self, session: &str, start: u64, piece: &[u8], total: Option<u64>, sent: &[Arc<AtomicU64>]) -> Put {
        // Renewing the login can fail on a short network cut too: waited out like the upload
        // itself, so a big upload doesn't start over from the first byte. (A login Google
        // refused for good is said again on every try, then the upload stops with it.)
        let token = match self.google.access_token(false).await {
            Ok(Some(t)) => t,
            Ok(None) => return Put::Fail(Fail::msg("Connect your Google Drive first (Upload › Google Drive).")),
            Err(e) => return Put::Retry(e.to_string()),
        };
        let resp = self
            .http
            .put(session)
            .bearer_auth(token)
            .header(reqwest::header::CONTENT_LENGTH, piece.len())
            .header(reqwest::header::CONTENT_RANGE, content_range(start, piece.len() as u64, total))
            .timeout(Duration::from_secs(600))
            .body(counted_body(Bytes::copy_from_slice(piece), sent))
            .send()
            .await;
        let resp = match resp {
            Ok(r) => r,
            Err(e) => return Put::Retry(drive_net_error(&e)),
        };
        let code = resp.status().as_u16();
        let range = resp.headers().get(reqwest::header::RANGE).and_then(|v| v.to_str().ok()).map(str::to_string);
        let text = resp.text().await.unwrap_or_default();
        match code {
            200 | 201 => match serde_json::from_str::<DriveFile>(&text) {
                Ok(f) => Put::Done(f),
                Err(_) => Put::Fail(Fail::msg("Google Drive didn't say which file it made.")),
            },
            308 => match next_offset(range.as_deref()) {
                Some(n) => Put::More(n),
                None => Put::Retry("Google Drive's answer was unclear.".into()),
            },
            401 => {
                // Renewed before the next try.
                let _ = self.google.access_token(true).await;
                Put::Retry(drive_error(code, &text))
            }
            404 | 410 => Put::Fail(Fail::msg("The upload was open too long and Google closed it. Try again.")),
            c if rate_limited(c, &text) || c >= 500 => Put::Retry(drive_error(code, &text)),
            _ => Put::Fail(drive_fail(code, &text)),
        }
    }

    /// Uploads what comes from `rx` into the opened upload, piece by piece. A cut connection or
    /// a busy Google is waited out (a few times per piece), carrying on from what arrived.
    pub async fn put_stream(
        &self,
        session: &str,
        mut rx: mpsc::Receiver<io::Result<Bytes>>,
        total: Option<u64>,
        cancel: &CancellationToken,
        sent: &[Arc<AtomicU64>],
    ) -> Result<DriveFile, Fail> {
        // What this piece has put on the counters so far; replaced by what Google confirmed.
        let take_back = |n: u64| {
            for c in sent {
                c.fetch_sub(n.min(c.load(Ordering::Relaxed)), Ordering::Relaxed);
            }
        };
        let add = |n: u64| {
            for c in sent {
                c.fetch_add(n, Ordering::Relaxed);
            }
        };
        let mine = |c: &[Arc<AtomicU64>]| c.first().map(|a| a.load(Ordering::Relaxed)).unwrap_or(0);
        let mut start = 0u64;
        let mut pending: Vec<u8> = Vec::with_capacity(CHUNK + 512 * 1024);
        let mut ended = false;
        let mut stalled = 0u32;
        loop {
            while !ended && pending.len() < CHUNK {
                tokio::select! {
                    _ = cancel.cancelled() => return Err(Fail::cancelled()),
                    m = rx.recv() => match m {
                        Some(Ok(b)) => pending.extend_from_slice(&b),
                        Some(Err(e)) => return Err(Fail::msg(e.to_string())),
                        None => ended = true,
                    },
                }
            }
            let len = if ended { pending.len() } else { CHUNK };
            let whole = if ended { Some(start + len as u64) } else { total };
            let mut tries = 0u32;
            let before = mine(sent);
            // Cancel stops a piece mid-way (not only between pieces).
            let mut put = tokio::select! {
                _ = cancel.cancelled() => {
                    take_back(mine(sent).saturating_sub(before));
                    return Err(Fail::cancelled());
                }
                p = self.put(session, start, &pending[..len], whole, sent) => p,
            };
            // Bytes counted while sending are swapped for the bytes Google says it has.
            take_back(mine(sent).saturating_sub(before));
            let next = loop {
                match put {
                    Put::Done(f) => {
                        add(len as u64);
                        return Ok(f);
                    }
                    Put::More(n) => break n,
                    Put::Fail(f) => return Err(f),
                    Put::Retry(msg) => {
                        tries += 1;
                        if tries >= PIECE_TRIES {
                            return Err(Fail::msg(msg));
                        }
                        let wait = Duration::from_secs(1 << tries.min(5));
                        tokio::select! {
                            _ = cancel.cancelled() => return Err(Fail::cancelled()),
                            _ = tokio::time::sleep(wait) => {}
                        }
                        // Where it stands; the rest is sent from there.
                        put = tokio::select! {
                            _ = cancel.cancelled() => return Err(Fail::cancelled()),
                            p = self.put(session, start, &[], whole, &[]) => p,
                        };
                    }
                }
            };
            if next < start || next > start + pending.len() as u64 {
                return Err(Fail::msg("Google Drive lost track of the upload. Try again."));
            }
            let confirmed = (next - start) as usize;
            add(confirmed as u64);
            if confirmed == 0 {
                stalled += 1;
                if stalled >= PIECE_TRIES {
                    return Err(Fail::msg("Google Drive isn't taking the upload. Try again in a few minutes."));
                }
            } else {
                stalled = 0;
            }
            pending.drain(..confirmed);
            start = next;
        }
    }
}

/// A piece handed to the network in 256 KB slices, each counted as it leaves.
fn counted_body(piece: Bytes, sent: &[Arc<AtomicU64>]) -> reqwest::Body {
    const SLICE: usize = 256 * 1024;
    let counters: Vec<Arc<AtomicU64>> = sent.to_vec();
    let slices: Vec<Bytes> = (0..piece.len()).step_by(SLICE).map(|i| piece.slice(i..(i + SLICE).min(piece.len()))).collect();
    let stream = futures_util::stream::iter(slices.into_iter().map(move |b| {
        for c in &counters {
            c.fetch_add(b.len() as u64, Ordering::Relaxed);
        }
        Ok::<Bytes, std::io::Error>(b)
    }));
    reqwest::Body::wrap_stream(stream)
}

impl Sender {
    pub(super) fn drive(&self) -> Drive<'_> {
        Drive { http: &self.http, google: &self.send_google }
    }

    /// One upload into your Drive, its bytes packed or read on their own thread straight into it.
    async fn drive_upload_once(&self, job: &Arc<Job>, src: &Arc<Source>, name: &str, total: Option<u64>, parent: &str) -> Result<DriveFile, Fail> {
        let drive = self.drive();
        let session = drive.start_upload(name, parent, total).await?;
        let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(8);
        let mine = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let writer = {
            // Reading ahead isn't progress: these counters stay local, the upload counts what leaves.
            let (produced, read, stop, src) = (Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0)), stop.clone(), src.clone());
            tauri::async_runtime::spawn_blocking(move || {
                let mut out = ToUpload { tx: tx.clone(), buf: Vec::with_capacity(256 * 1024), produced, mine: read };
                let r = match &*src {
                    Source::File(e) => copy_file(e, &mut out, &stop),
                    Source::Zip(entries, pack) => write_zip(entries, *pack, &mut out, &stop, Z64_AT),
                };
                if let Err(e) = &r {
                    let _ = tx.blocking_send(Err(io::Error::other(e.to_string())));
                }
                r
            })
        };
        let r = drive.put_stream(&session, rx, total, &job.cancel, &[mine.clone(), job.produced.clone()]).await;
        if r.is_err() {
            stop.store(true, Ordering::Relaxed);
            // A changed file is clearer than "the upload was cut off".
            if let Some(m) = writer_error(&job.cancel, writer).await {
                job.produced.fetch_sub(mine.load(Ordering::Relaxed).min(job.produced.load(Ordering::Relaxed)), Ordering::Relaxed);
                return Err(Fail::msg(m));
            }
            job.produced.fetch_sub(mine.load(Ordering::Relaxed).min(job.produced.load(Ordering::Relaxed)), Ordering::Relaxed);
        }
        r
    }

    /// One upload with two more tries after a short wait. Not when the Drive is full or the
    /// login was refused: the same answer would come again.
    async fn drive_upload_retrying(&self, job: &Arc<Job>, src: Arc<Source>, name: &str, total: Option<u64>, parent: &str) -> Result<DriveFile, String> {
        let mut last = String::new();
        for wait in [0u64, 3, 8] {
            if wait > 0 {
                tokio::select! {
                    _ = job.cancel.cancelled() => return Err("cancelled".into()),
                    _ = tokio::time::sleep(Duration::from_secs(wait)) => {}
                }
            }
            match self.drive_upload_once(job, &src, name, total, parent).await {
                Ok(f) => return Ok(f),
                Err(f) if f.is_cancelled() || matches!(f.code, Some(400 | 401 | 403)) => return Err(f.msg),
                Err(f) => last = f.msg,
            }
        }
        Err(last)
    }

    /// Refuses a send that can't fit in your Drive (said before anything uploads).
    async fn check_room(&self, need: u64) -> Result<(), String> {
        if let Some(free) = self.store(Host::Drive).free_space().await? {
            if need > free {
                return Err(format!(
                    "This upload needs {}, and your Google Drive has {} free. Make room (delete older uploads), then try again.",
                    human(need),
                    human(free)
                ));
            }
        }
        Ok(())
    }

    /// A whole send into your Drive.
    pub(super) async fn run_drive(&self, job: &Arc<Job>, payload: Payload, pack: Pack) -> Result<Done, String> {
        if !self.store(Host::Drive).connected().await {
            return Err("Connect your Google Drive first (Upload › Google Drive).".into());
        }
        self.check_room(job.total.unwrap_or(job.input)).await?;
        *job.account.lock().unwrap() = self.send_google.email().await;
        let parent = self.drive().sends_folder().await.map_err(|f| f.msg)?;
        let src = match payload {
            Payload::Files { entries, .. } => return self.run_drive_files(job, entries, Some(&parent)).await,
            Payload::File(e) => Source::File(e),
            Payload::Zip { entries, .. } => Source::Zip(entries, pack),
        };
        let file = self.drive_upload_retrying(job, Arc::new(src), &job.name, job.total, &parent).await?;
        let link = self.drive().share_and_link(&file).await?;
        Ok((link, None, Some(file.id)))
    }

    /// Several files as they are: a folder named after the send (shared at once, so its link
    /// works as soon as files arrive), the files three at a time. Files that still fail are kept
    /// for "Try again" (into the same folder).
    pub(super) async fn run_drive_files(&self, job: &Arc<Job>, entries: Vec<Entry>, parent: Option<&str>) -> Result<Done, String> {
        use futures_util::StreamExt;
        let mut folder = job.folder.lock().unwrap().clone();
        if folder.is_some() {
            // "Try again" goes into the same folder: only with the account it is in.
            let owner = job.account.lock().unwrap().clone();
            if let Some(msg) = super::storage::other_account(owner.as_deref(), self.send_google.email().await.as_deref()) {
                // Still to upload: "Try again" (with the right account) carries on with them.
                *job.left.lock().unwrap() = entries;
                return Err(msg);
            }
        } else {
            if entries.is_empty() {
                return Err("There are no files to upload.".into());
            }
            let (id, link) = self.store(Host::Drive).create_folder(&job.name, parent).await?;
            *job.folder.lock().unwrap() = Some((id.clone(), link.clone()));
            // On the Sent list at once (marked not finished): Cancel, Dismiss or closing Pako
            // never leaves files in your Drive that Pako can't delete.
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
                    let src = Arc::new(Source::File(e.clone()));
                    match self.drive_upload_retrying(job, src, &e.name, Some(e.size), &folder_id).await {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_are_a_multiple_of_256_kb() {
        assert_eq!(CHUNK % (256 * 1024), 0);
        assert!((8..=32).contains(&(CHUNK / (1024 * 1024))));
    }

    #[test]
    fn content_ranges() {
        assert_eq!(content_range(0, 16, Some(100)), "bytes 0-15/100");
        assert_eq!(content_range(16, 16, None), "bytes 16-31/*");
        assert_eq!(content_range(96, 4, Some(100)), "bytes 96-99/100");
        // Asking where it stands.
        assert_eq!(content_range(50, 0, None), "bytes */*");
        assert_eq!(content_range(100, 0, Some(100)), "bytes */100");
    }

    #[test]
    fn where_the_upload_stands() {
        assert_eq!(next_offset(Some("bytes=0-16777215")), Some(16_777_216));
        assert_eq!(next_offset(Some(" bytes=0-0 ")), Some(1));
        assert_eq!(next_offset(None), Some(0), "nothing arrived yet");
        assert_eq!(next_offset(Some("bytes=oops")), None);
        assert_eq!(next_offset(Some("items=0-5")), None);
    }

    #[test]
    fn shared_with_anyone_who_has_the_link() {
        assert_eq!(share_body(), serde_json::json!({ "type": "anyone", "role": "reader" }));
        let f = folder_body("Holiday", Some("p1"));
        assert_eq!(f["mimeType"], FOLDER_MIME);
        assert_eq!(f["parents"][0], "p1");
        assert!(folder_body("Pako uploads", None).get("parents").is_none());
    }

    #[test]
    fn uploads_folder_new_name_and_old_one() {
        use serde_json::json;
        let q = folders_query();
        assert!(q.contains("name = 'Pako uploads'") && q.contains("name = 'Pako sends'"), "{q}");
        assert!(q.contains("'root' in parents") && q.contains("trashed = false"));
        // Only the old folder: keep using it, so earlier uploads stay where they are.
        assert_eq!(pick_folder(&json!({ "files": [{ "id": "old", "name": "Pako sends" }] })).as_deref(), Some("old"));
        // Both: the new name wins, whatever order Drive lists them in.
        let both = json!({ "files": [{ "id": "old", "name": "Pako sends" }, { "id": "new", "name": "Pako uploads" }] });
        assert_eq!(pick_folder(&both).as_deref(), Some("new"));
        assert_eq!(pick_folder(&json!({ "files": [{ "id": "new", "name": "Pako uploads" }] })).as_deref(), Some("new"));
        // None (or an unexpected answer): Pako makes "Pako uploads".
        assert_eq!(pick_folder(&json!({ "files": [] })), None);
        assert_eq!(pick_folder(&json!({ "files": [{ "id": "x", "name": "Something else" }] })), None);
        assert_eq!(pick_folder(&json!({})), None);
    }

    #[test]
    fn plain_drive_errors() {
        let full = r#"{"error":{"errors":[{"reason":"storageQuotaExceeded"}],"code":403}}"#;
        assert!(drive_error(403, full).contains("full"));
        assert!(drive_error(429, "").contains("slow down"));
        assert!(drive_error(503, "").contains("busy"));
        assert!(drive_error(401, "").contains("Connect Google Drive again"));
        assert_eq!(human(4_200_000_000), "4.2 GB");
        assert_eq!(human(512), "512 bytes");
    }

    /// A stand-in for Google: takes pieces, says "308, bytes=0-N" until the last one.
    #[tokio::test]
    async fn uploads_in_pieces_and_carries_on() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let data: Vec<u8> = (0..(CHUNK + 1000)).map(|i| (i % 251) as u8).collect();
        let want = data.clone();
        let server = tokio::spawn(async move {
            let mut got: Vec<u8> = Vec::new();
            let mut ranges = Vec::new();
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut buf = Vec::new();
                let mut tmp = vec![0u8; 256 * 1024];
                let (head, body_start, len) = loop {
                    let n = sock.read(&mut tmp).await.unwrap();
                    buf.extend_from_slice(&tmp[..n]);
                    if let Some(h) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..h]).to_lowercase();
                        let len: usize = head.lines().find_map(|l| l.strip_prefix("content-length: ")).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
                        break (head, h + 4, len);
                    }
                };
                while buf.len() < body_start + len {
                    let n = sock.read(&mut tmp).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                }
                let range = head.lines().find_map(|l| l.strip_prefix("content-range: ")).unwrap_or("").to_string();
                ranges.push(range.clone());
                got.extend_from_slice(&buf[body_start..body_start + len]);
                let last = !range.ends_with("/*");
                let resp = if last {
                    let body = r#"{"id":"f1","webViewLink":"https://drive.google.com/file/d/f1/view"}"#;
                    format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len())
                } else {
                    format!("HTTP/1.1 308 Resume Incomplete\r\nrange: bytes=0-{}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n", got.len() - 1)
                };
                sock.write_all(resp.as_bytes()).await.unwrap();
                if last {
                    return (got, ranges);
                }
            }
        });
        let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(8);
        tokio::spawn(async move {
            for piece in data.chunks(256 * 1024) {
                tx.send(Ok(Bytes::copy_from_slice(piece))).await.unwrap();
            }
        });
        let http = reqwest::Client::new();
        // Signed in with a stand-in login: the token is only sent along.
        let google = Google::for_account(http.clone(), crate::google::SENDING);
        google.test_token("tok").await;
        let drive = Drive { http: &http, google: &google };
        let counted = Arc::new(AtomicU64::new(0));
        let f = drive
            .put_stream(&format!("http://{addr}/upload"), rx, None, &CancellationToken::new(), &[counted.clone()])
            .await
            .ok()
            .expect("uploaded");
        assert_eq!(f.id, "f1");
        // Progress ends at exactly what Google took, once.
        assert_eq!(counted.load(Ordering::Relaxed), (CHUNK + 1000) as u64);
        let (got, ranges) = server.await.unwrap();
        assert_eq!(got, want, "every byte arrived once, in order");
        assert_eq!(ranges[0], format!("bytes 0-{}/*", CHUNK - 1));
        assert_eq!(ranges[1], format!("bytes {}-{}/{}", CHUNK, CHUNK + 999, CHUNK + 1000));
    }

    /// "Slow down" comes as 403 too: it's waited out like 429, not taken as a refusal.
    #[test]
    fn slow_down_is_retried() {
        let slow = r#"{"error":{"errors":[{"reason":"userRateLimitExceeded"}],"code":403}}"#;
        let denied = r#"{"error":{"errors":[{"reason":"insufficientPermissions"}],"code":403}}"#;
        assert!(rate_limited(403, slow) && rate_limited(429, ""));
        assert!(!rate_limited(403, denied) && !rate_limited(500, slow));
        assert_eq!(drive_fail(403, slow).code, Some(429));
        assert_eq!(drive_fail(403, denied).code, Some(403));
        assert!(drive_fail(403, slow).msg.contains("slow down"));
    }
}
