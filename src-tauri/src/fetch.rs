// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Opening a connection for (part of) a file, and sorting failures into
//! "try again later", "slow down", or "stop".

use reqwest::{Response, StatusCode, header};

use crate::drive;
use crate::model::Source;

pub const BROWSER_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36";

#[derive(Debug)]
pub enum FetchError {
    /// Temporary: network dropped, server busy, Google asked to slow down... Wait and try again.
    Retry(String),
    /// The server says "too many requests / slow down": wait, AND use fewer connections.
    /// (message, seconds the server asked us to wait; 0 = not said)
    Throttled(String, u64),
    /// Google's daily download limit for this shared file.
    Quota(String),
    /// Google sent a web page instead of the file: it's only shared with some people (or
    /// Google stopped the public link). Downloading it in the browser works: Pako catches it.
    Blocked(String),
    /// Retrying won't help.
    Fatal(String),
}

pub const BLOCKED_MESSAGE: &str =
    "Google only gives this file to people it's shared with. Open it in your browser and download it there: Pako catches the download.";

/// Google's daily download limit for a shared file (what the row says).
pub const QUOTA_MESSAGE: &str = "Google's download limit for this shared file is reached for today.";

impl FetchError {
    pub fn message(&self) -> String {
        match self {
            FetchError::Retry(m)
            | FetchError::Throttled(m, _)
            | FetchError::Quota(m)
            | FetchError::Blocked(m)
            | FetchError::Fatal(m) => m.clone(),
        }
    }
}

pub struct Fetcher {
    http: reqwest::Client,
    target: Target,
    /// Plain web links only: the file version noted when the download was first checked,
    /// sent as `If-Range` so the server sends the whole (new) file instead of a piece of
    /// a different one if the file changed meanwhile.
    validator: Option<String>,
    /// Plain web links sent from the browser extension: the page they came from and the
    /// browser's cookies for them (memory only), so files behind a login download.
    browser: Option<crate::extension::BrowserHeaders>,
    /// A video stream: the headers the site wants (from yt-dlp), sent instead of Pako's own.
    headers: Vec<(header::HeaderName, header::HeaderValue)>,
}

enum Target {
    Http { url: String },
    /// A Drive file through Google's public download (or export) address: no sign-in.
    Drive { url: String },
}

impl Fetcher {
    pub fn new(http: reqwest::Client, source: &Source) -> Self {
        let target = match source {
            Source::Http { url } | Source::Video { url, .. } => Target::Http { url: url.clone() },
            Source::GDrive { file_id, resource_key, doc_type } => {
                Target::Drive { url: drive::public_url(file_id, resource_key.as_deref(), doc_type.as_deref()) }
            }
        };
        Self { http, target, validator: None, browser: None, headers: Vec::new() }
    }

    /// One stream of a video, at the address and with the headers yt-dlp found for it. Headers
    /// that aren't valid, or that Pako sets itself (range, encoding...), are left out.
    pub fn for_stream(http: reqwest::Client, url: &str, headers: &[(String, String)]) -> Self {
        const OWN: [&str; 5] = ["range", "accept-encoding", "host", "content-length", "connection"];
        let headers = headers
            .iter()
            .filter(|(k, _)| !OWN.contains(&k.to_ascii_lowercase().as_str()))
            .filter_map(|(k, v)| {
                Some((header::HeaderName::from_bytes(k.as_bytes()).ok()?, header::HeaderValue::from_str(v).ok()?))
            })
            .collect();
        Self { http, target: Target::Http { url: url.to_string() }, validator: None, browser: None, headers }
    }

    /// Remembers the file version to send back on resumed parts. Ignored for Drive files:
    /// Google's own file ids already point at one exact file.
    /// Referrer and cookies from the browser extension (web links only).
    pub fn set_browser_headers(&mut self, h: Option<crate::extension::BrowserHeaders>) {
        if matches!(self.target, Target::Http { .. }) {
            self.browser = h;
        }
    }

    pub fn set_validator(&mut self, validator: Option<String>) {
        if matches!(self.target, Target::Http { .. }) {
            self.validator = validator;
        }
    }

    /// True if requests carry the file version (If-Range).
    pub fn sends_version(&self) -> bool {
        self.validator.is_some()
    }

    pub fn is_drive(&self) -> bool {
        matches!(self.target, Target::Drive { .. })
    }

    /// The server this file comes from, for the per-server connection budget.
    pub fn host(&self) -> String {
        let url = match &self.target {
            Target::Http { url } | Target::Drive { url } => url,
        };
        url::Url::parse(url).ok().and_then(|u| u.host_str().map(String::from)).unwrap_or_default()
    }

    /// Most connections Auto may use for one file from this kind of source.
    pub fn auto_ceiling(&self) -> u32 {
        match self.target {
            Target::Drive { .. } => 8,
            _ => 16,
        }
    }

    /// GET from byte `from` to `to` (inclusive). `to = None` means "until the end".
    pub async fn get(&self, from: u64, to: Option<u64>) -> Result<Response, FetchError> {
        let range = match to {
            Some(to) => format!("bytes={from}-{to}"),
            None => format!("bytes={from}-"),
        };
        let req = match &self.target {
            Target::Http { url } if !self.headers.is_empty() => {
                let mut r = self.http.get(url);
                if !self.headers.iter().any(|(k, _)| k == header::USER_AGENT) {
                    r = r.header(header::USER_AGENT, BROWSER_UA);
                }
                for (k, v) in &self.headers {
                    r = r.header(k, v);
                }
                r
            }
            Target::Http { url } => {
                let mut r = self.http.get(url).header(header::USER_AGENT, BROWSER_UA);
                if let Some(b) = &self.browser {
                    if !b.referrer.is_empty() {
                        r = r.header(header::REFERER, &b.referrer);
                    }
                    if !b.cookies.is_empty() {
                        r = r.header(header::COOKIE, &b.cookies);
                    }
                }
                r
            }
            Target::Drive { url } => self.http.get(url).header(header::USER_AGENT, BROWSER_UA),
        };
        let req = match &self.validator {
            Some(v) => req.header(header::IF_RANGE, v),
            None => req,
        };
        let resp = req
            .header(header::RANGE, range)
            .header(header::ACCEPT_ENCODING, "identity")
            .send()
            .await
            .map_err(|e| FetchError::Retry(network_message(&e)))?;
        self.check(resp).await
    }

    async fn check(&self, resp: Response) -> Result<Response, FetchError> {
        let status = resp.status();
        // "Retry-After: 30": how long the server wants us to wait (seconds form only).
        let retry_after: Option<u64> = resp
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok())
            .map(|s: u64| s.min(3600));
        let wait = retry_after.unwrap_or(0);
        if status.is_success() {
            // The public Drive link returns an HTML page (not the file) when it's blocked.
            if self.is_drive() && is_html(&resp) {
                let body = resp.text().await.unwrap_or_default().to_lowercase();
                return Err(drive_page_error(&body));
            }
            return Ok(resp);
        }
        if self.is_drive() {
            let code = status.as_u16();
            // "Not allowed" means the file is only shared with some people (or today's limit).
            if matches!(code, 401 | 403) {
                let body = resp.text().await.unwrap_or_default().to_lowercase();
                return Err(drive_page_error(&body));
            }
            let body = resp.text().await.unwrap_or_default();
            let (reason, message) = drive::error_reason(&body);
            let msg = drive::friendly_api_error(code, reason.as_deref(), message.as_deref());
            return Err(match reason.as_deref() {
                Some("downloadQuotaExceeded") => FetchError::Quota(msg),
                Some("rateLimitExceeded" | "userRateLimitExceeded" | "sharingRateLimitExceeded") => {
                    FetchError::Throttled(msg, wait)
                }
                _ if code == 429 => FetchError::Throttled(msg, wait),
                _ if code >= 500 => FetchError::Retry(msg),
                _ => FetchError::Fatal(msg),
            });
        }
        Err(match status {
            StatusCode::TOO_MANY_REQUESTS => FetchError::Throttled(
                match retry_after {
                    Some(s) => format!("The server asked to slow down for {s}s (429)"),
                    None => "The server asked to slow down (429)".into(),
                },
                wait,
            ),
            StatusCode::REQUEST_TIMEOUT => FetchError::Retry(format!("Server busy ({})", status.as_u16())),
            s if s.is_server_error() => FetchError::Retry(format!("Server error ({})", s.as_u16())),
            StatusCode::NOT_FOUND | StatusCode::GONE => FetchError::Fatal("File not found on the server (404).".into()),
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => FetchError::Fatal(format!(
                "The server refused access ({}). The link may have expired.",
                status.as_u16()
            )),
            s => FetchError::Fatal(format!("The server answered {}", s.as_u16())),
        })
    }
}

/// What a Drive refusal page means: today's download limit, or a file only some people get.
fn drive_page_error(lower_body: &str) -> FetchError {
    if lower_body.contains("quota") || lower_body.contains("too many users") {
        FetchError::Quota(QUOTA_MESSAGE.into())
    } else {
        FetchError::Blocked(BLOCKED_MESSAGE.into())
    }
}

pub fn is_html(resp: &Response) -> bool {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("text/html"))
        .unwrap_or(false)
}

pub fn network_message(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "Connection timed out".into()
    } else if e.is_connect() {
        "No connection to the server".into()
    } else {
        "Connection interrupted".into()
    }
}

/// What a first request tells us about the file.
pub struct ProbeInfo {
    pub size: Option<u64>,
    pub resumable: bool,
    pub name: Option<String>,
    /// Strong ETag, else Last-Modified (a weak ETag can't be used with If-Range).
    pub validator: Option<String>,
}

pub fn probe_info(resp: &Response) -> ProbeInfo {
    let h = resp.headers();
    let name = h
        .get(header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(crate::names::from_content_disposition);
    let header_text = |k: header::HeaderName| {
        h.get(k).and_then(|v| v.to_str().ok()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    };
    let validator = header_text(header::ETAG)
        .filter(|e| !e.starts_with("W/"))
        .or_else(|| header_text(header::LAST_MODIFIED));
    if resp.status() == StatusCode::PARTIAL_CONTENT {
        let total = content_range(resp).and_then(|(_, total)| total);
        ProbeInfo { size: total, resumable: total.is_some(), name, validator }
    } else {
        ProbeInfo { size: resp.content_length(), resumable: false, name, validator }
    }
}

/// `Content-Range: bytes 1000-1999/12345` -> (first byte sent, total size if known).
/// None = header missing or not understood.
pub fn content_range(resp: &Response) -> Option<(u64, Option<u64>)> {
    let v = resp.headers().get(header::CONTENT_RANGE)?.to_str().ok()?.trim();
    let rest = v.strip_prefix("bytes")?.trim_start();
    let (range, total) = rest.split_once('/')?;
    let start = range.split_once('-')?.0.trim().parse::<u64>().ok()?;
    let total = total.trim().parse::<u64>().ok(); // "*" = total not said
    Some((start, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A video stream's own headers are sent, except the ones Pako sets itself or that aren't valid.
    #[test]
    fn stream_headers_are_kept_and_checked() {
        let http = reqwest::Client::new();
        let headers = vec![
            ("User-Agent".to_string(), "UA-1".to_string()),
            ("Referer".to_string(), "https://site/".to_string()),
            ("Range".to_string(), "bytes=5-".to_string()),
            ("Accept-Encoding".to_string(), "gzip".to_string()),
            ("Bad Name".to_string(), "x".to_string()),
            ("X-Bad".to_string(), "a\nb".to_string()),
        ];
        let f = Fetcher::for_stream(http, "https://cdn/v.mp4", &headers);
        let names: Vec<String> = f.headers.iter().map(|(k, _)| k.as_str().to_string()).collect();
        assert_eq!(names, vec!["user-agent", "referer"]);
        assert_eq!(f.host(), "cdn");
        assert!(!f.is_drive());
    }
}
