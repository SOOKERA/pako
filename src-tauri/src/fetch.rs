// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Opening a connection for (part of) a file, and sorting failures into
//! "try again later", "slow down", or "stop".

use reqwest::{Response, StatusCode, header};

use crate::drive;
use crate::model::Source;

pub const BROWSER_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36";

/// The client for downloads, link checks, previews and caught streams.
///
/// No `Referer` of its own on redirects: reqwest would otherwise send the whole previous
/// address (its query's tokens and signatures included) to the server it is sent on to,
/// even another site, in place of the page referrer the browser's rule gave
/// (`stream::referrer_for`, the extension's). A referrer Pako sets itself stays as it is, as in
/// a browser. Cookies Pako sets itself are dropped by reqwest when a redirect leaves for another
/// host.
pub fn download_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(20))
        // A connection that receives nothing for 45s is considered dead and reopened.
        .read_timeout(std::time::Duration::from_secs(45))
        .pool_idle_timeout(std::time::Duration::from_secs(30))
        .referer(false)
        .build()
        .expect("http client")
}

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

// The words below are compared again later (to know what happened), in either language: see
// `crate::model::Said`.
pub const BLOCKED_MESSAGE: crate::model::Said = [
    "Google only gives this file to people it's shared with. Open it in your browser and download it there: Pako catches the download.",
    "Google ne donne ce fichier qu’aux personnes avec qui il est partagé. Ouvrez-le dans votre navigateur et téléchargez-le là : Pako récupère le téléchargement.",
];

/// Google's daily download limit for a shared file (what the row says).
pub const QUOTA_MESSAGE: crate::model::Said = [
    "Google's download limit for this shared file is reached for today.",
    "La limite de téléchargement de Google pour ce fichier partagé est atteinte pour aujourd’hui.",
];

/// The file isn't there (404, 410): the start of what the row says.
pub const NOT_FOUND: crate::model::Said = [
    "File not found on the server",
    "Fichier introuvable sur le serveur",
];
/// The server refused (401, 403): the start of what the row says.
pub const REFUSED: crate::model::Said = [
    "The server refused access",
    "Le serveur a refusé l’accès",
];
/// Another final answer: the start of what the row says.
pub const ANSWERED: crate::model::Said = [
    "The server answered",
    "Le serveur a répondu",
];

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
    /// A link another of your PCs sent (My devices): an answer that comes from this PC itself
    /// (localhost) is refused, however the link got there (see `from_this_pc`).
    not_this_pc: bool,
}

/// What a download sent by another PC says when its link leads to this PC itself.
pub fn this_pc_message() -> String {
    tr!("Links to the PC itself (localhost) aren't taken from another PC.", "Les liens vers le PC lui-même (localhost) ne sont pas acceptés depuis un autre PC.")
}

/// A link to the PC it's opened on: localhost, 127.x, ::1, 0.0.0.0 (as the link reader writes
/// them: "http://127.1" and "http://0x7f000001" are 127.0.0.1 too).
pub fn is_this_pc(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(d)) => {
            let d = d.trim_end_matches('.').to_ascii_lowercase();
            d == "localhost" || d.ends_with(".localhost")
        }
        Some(url::Host::Ipv4(ip)) => is_this_pc_ip(ip.into()),
        Some(url::Host::Ipv6(ip)) => is_this_pc_ip(ip.into()),
        None => false,
    }
}

/// An address of the PC itself: 127.x, ::1, 0.0.0.0, :: (also written as IPv4 inside IPv6).
pub fn is_this_pc_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_loopback() || v4.is_unspecified(),
        std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || v6.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback() || v4.is_unspecified()),
    }
}

/// Windows (or the environment) sends web requests through a proxy: Pako's connection then
/// goes to the proxy, often a program on this PC, whatever the link's server is.
fn proxy_in_use() -> bool {
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        let env = ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"];
        env.iter().any(|k| std::env::var(k).is_ok_and(|v| !v.trim().is_empty())) || windows_proxy_on()
    });
    *ON
}

/// Windows' own proxy setting for this user (Settings › Network › Proxy) is switched on.
#[cfg(windows)]
fn windows_proxy_on() -> bool {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (key, name) = (wide(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings"), wide("ProxyEnable"));
    let mut on = 0u32;
    let mut len = 4u32;
    // SAFETY: NUL-terminated key and name; the buffer is one DWORD and `len` says so.
    let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), (&mut on as *mut u32).cast(), &mut len) };
    r == 0 && on != 0
}
#[cfg(not(windows))]
fn windows_proxy_on() -> bool {
    false
}

/// Did this answer come from the PC itself? `named`: the address it came from names this PC
/// (after redirects); `peer`: where the connection went; `proxied`: requests go through a
/// proxy (the connection then goes to the proxy, so `peer` says nothing by itself);
/// `resolved`: what the address's host name stands for, looked up again (only with a proxy).
/// So a link that only looks like another server's (a name that stands for 127.0.0.1, a
/// redirect to localhost, a name that changes what it stands for) is caught too.
fn from_this_pc(named: bool, peer: Option<std::net::IpAddr>, proxied: bool, resolved: &[std::net::IpAddr]) -> bool {
    if named {
        return true;
    }
    match peer {
        Some(ip) if is_this_pc_ip(ip) => !proxied || resolved.iter().any(|r| is_this_pc_ip(*r)),
        _ => false,
    }
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
        Self { http, target, validator: None, browser: None, headers: Vec::new(), not_this_pc: false }
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
        Self { http, target: Target::Http { url: url.to_string() }, validator: None, browser: None, headers, not_this_pc: false }
    }

    /// Remembers the file version to send back on resumed parts. Ignored for Drive files:
    /// Google's own file ids already point at one exact file.
    /// Referrer and cookies from the browser extension (web links only).
    pub fn set_browser_headers(&mut self, h: Option<crate::extension::BrowserHeaders>) {
        if matches!(self.target, Target::Http { .. }) {
            self.browser = h;
        }
    }

    /// For a link another of your PCs sent: never an answer from this PC itself.
    pub fn refuse_this_pc(&mut self, on: bool) {
        self.not_this_pc = on;
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
            .map_err(|e| send_error(&e))?;
        if self.not_this_pc && self.answered_by_this_pc(&resp).await {
            return Err(FetchError::Fatal(this_pc_message()));
        }
        self.check(resp).await
    }

    /// See `from_this_pc`.
    async fn answered_by_this_pc(&self, resp: &Response) -> bool {
        let peer = resp.remote_addr().map(|a| a.ip());
        let proxied = proxy_in_use();
        let mut resolved = Vec::new();
        if proxied && peer.is_some_and(is_this_pc_ip) {
            if let Some(host) = resp.url().host_str() {
                // No answer from the lookup: not taken (it can't be told apart).
                match tokio::net::lookup_host((host.trim_matches(['[', ']']), resp.url().port_or_known_default().unwrap_or(80))).await {
                    Ok(found) => resolved.extend(found.map(|a| a.ip())),
                    Err(_) => return true,
                }
            }
        }
        from_this_pc(is_this_pc(resp.url()), peer, proxied, &resolved)
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
                    Some(s) => tr!("The server asked to slow down for {s}s (429)", "Le serveur a demandé de ralentir pendant {s} s (429)"),
                    None => tr!("The server asked to slow down (429)", "Le serveur a demandé de ralentir (429)"),
                },
                wait,
            ),
            StatusCode::REQUEST_TIMEOUT => FetchError::Retry(tr!("Server busy ({})", "Serveur occupé ({})", status.as_u16())),
            s if s.is_server_error() => FetchError::Retry(tr!("Server error ({})", "Erreur du serveur ({})", s.as_u16())),
            StatusCode::NOT_FOUND | StatusCode::GONE => FetchError::Fatal(format!("{} (404).", crate::model::say(NOT_FOUND))),
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => FetchError::Fatal(tr!(
                "{} ({}). The link may have expired.",
                "{} ({}). Le lien a peut-être expiré.",
                crate::model::say(REFUSED),
                status.as_u16()
            )),
            s => FetchError::Fatal(format!("{} {}", crate::model::say(ANSWERED), s.as_u16())),
        })
    }
}

/// What a Drive refusal page means: today's download limit, or a file only some people get.
fn drive_page_error(lower_body: &str) -> FetchError {
    if lower_body.contains("quota") || lower_body.contains("too many users") {
        FetchError::Quota(crate::model::say(QUOTA_MESSAGE))
    } else {
        FetchError::Blocked(crate::model::say(BLOCKED_MESSAGE))
    }
}

pub fn is_html(resp: &Response) -> bool {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("text/html"))
        .unwrap_or(false)
}

/// The server's name isn't known (a misspelled or gone site): said plainly, and the engine stops
/// trying once the internet is known to work (`engine.rs` resolve).
pub const NAME_NOT_FOUND: crate::model::Said = [
    "The server's name wasn't found. Check the link",
    "Le nom du serveur est introuvable. Vérifiez le lien",
];
/// The site's security certificate was refused (expired, for another site, not trusted).
pub fn bad_certificate() -> String {
    tr!("The site's security certificate isn't valid, so Pako didn't connect", "Le certificat de sécurité du site n’est pas valide : Pako ne s’est donc pas connecté")
}

/// What went wrong sending a request, from the whole chain of causes (reqwest says "connect" for
/// a name that can't be found and for a refused certificate too).
#[derive(Debug, PartialEq)]
enum Cause {
    NameNotFound,
    BadCertificate,
    Other,
}

fn cause_of(chain: &[String]) -> Cause {
    let text = chain.join(" | ").to_lowercase();
    if text.contains("invalid peer certificate") || text.contains("certificate verify failed") || text.contains("unknownissuer") || text.contains("notvalidforname") {
        Cause::BadCertificate
    } else if text.contains("no such host is known") || text.contains("os error 11001") || text.contains("failed to lookup address") || text.contains("name or service not known") {
        Cause::NameNotFound
    } else {
        Cause::Other
    }
}

fn chain_of(e: &reqwest::Error) -> Vec<String> {
    let mut out = vec![e.to_string()];
    let mut src = std::error::Error::source(e);
    while let Some(s) = src {
        out.push(s.to_string());
        src = s.source();
    }
    out
}

/// A failed request: a refused certificate is final (trying again changes nothing); a name not
/// found is retried with its own words (it may be the internet that's down: the engine decides).
fn send_error(e: &reqwest::Error) -> FetchError {
    match cause_of(&chain_of(e)) {
        Cause::BadCertificate => FetchError::Fatal(bad_certificate()),
        Cause::NameNotFound => FetchError::Retry(crate::model::say(NAME_NOT_FOUND)),
        Cause::Other => FetchError::Retry(network_message(e)),
    }
}

pub fn network_message(e: &reqwest::Error) -> String {
    match cause_of(&chain_of(e)) {
        Cause::BadCertificate => return bad_certificate(),
        Cause::NameNotFound => return crate::model::say(NAME_NOT_FOUND),
        Cause::Other => {}
    }
    if e.is_timeout() {
        tr!("Connection timed out", "La connexion a expiré")
    } else if e.is_connect() {
        tr!("No connection to the server", "Pas de connexion au serveur")
    } else {
        tr!("Connection interrupted", "Connexion interrompue")
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
    /// A misspelled site and a refused certificate get their own words (reqwest reports both
    /// as "connect"); other failures keep the general ones.
    #[test]
    fn names_what_went_wrong() {
        use super::{Cause, cause_of};
        let c = |v: &[&str]| cause_of(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(c(&["error sending request", "client error (Connect)", "dns error: failed to lookup address information: No such host is known. (os error 11001)"]), Cause::NameNotFound);
        assert_eq!(c(&["error sending request", "client error (Connect)", "invalid peer certificate: Expired"]), Cause::BadCertificate);
        assert_eq!(c(&["error sending request", "invalid peer certificate: UnknownIssuer"]), Cause::BadCertificate);
        assert_eq!(c(&["error sending request", "client error (Connect)", "tcp connect error: No connection could be made because the target machine actively refused it. (os error 10061)"]), Cause::Other);
        assert_eq!(c(&["operation timed out"]), Cause::Other);
    }

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

    /// A link another of your PCs sent never brings back what this PC itself serves: not by
    /// naming it, not through a redirect, not through a name that stands for 127.0.0.1.
    #[test]
    fn an_answer_from_this_pc_is_told_apart() {
        let ip = |s: &str| s.parse::<std::net::IpAddr>().unwrap();
        let u = |s: &str| url::Url::parse(s).unwrap();
        for l in ["http://localhost/x", "http://a.localhost./x", "http://127.0.0.1:8384/", "http://127.1/", "http://0x7f000001/", "http://[::1]/", "http://0.0.0.0/", "http://[::ffff:127.0.0.1]/"] {
            assert!(is_this_pc(&u(l)), "{l}");
        }
        for l in ["https://example.com/x", "http://192.168.1.10/", "http://localhost.example.com/"] {
            assert!(!is_this_pc(&u(l)), "{l}");
        }
        // Named (a redirect ended on localhost): always.
        assert!(from_this_pc(true, None, true, &[]));
        // A name that stands for 127.0.0.1, or changed to it: the connection went to this PC.
        assert!(from_this_pc(false, Some(ip("127.0.0.1")), false, &[]));
        assert!(from_this_pc(false, Some(ip("::1")), false, &[]));
        assert!(from_this_pc(false, Some(ip("::ffff:127.0.0.1")), false, &[]));
        // Another server: fine.
        assert!(!from_this_pc(false, Some(ip("93.184.216.34")), false, &[]));
        assert!(!from_this_pc(false, Some(ip("192.168.1.10")), false, &[]));
        assert!(!from_this_pc(false, None, false, &[]));
        // Through a proxy on this PC: only when the name itself stands for this PC.
        assert!(!from_this_pc(false, Some(ip("127.0.0.1")), true, &[ip("93.184.216.34")]));
        assert!(from_this_pc(false, Some(ip("127.0.0.1")), true, &[ip("93.184.216.34"), ip("127.0.0.1")]));
    }

    /// The same, on a real connection: asked for by another PC, a server on this PC is
    /// refused (also behind a redirect); asked for here, it downloads as always.
    #[tokio::test]
    async fn a_link_from_another_pc_never_reads_this_pc() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { return };
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let first = String::from_utf8_lossy(&buf[..n]).lines().next().unwrap_or_default().to_string();
                let answer = if first.contains("/hop") {
                    format!("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nsecret".to_string()
                };
                let _ = sock.write_all(answer.as_bytes()).await;
            }
        });
        for path in ["secret", "hop"] {
            let src = Source::Http { url: format!("http://127.0.0.1:{port}/{path}") };
            let mut f = Fetcher::new(download_client(), &src);
            assert!(f.get(0, None).await.is_ok(), "{path}: asked for here");
            f.refuse_this_pc(true);
            match f.get(0, None).await {
                Err(FetchError::Fatal(m)) => assert_eq!(m, this_pc_message()),
                other => panic!("{path}: {:?}", other.map(|r| r.status())),
            }
        }
    }

    /// A download link that sends Pako on to another server: that server gets the page referrer
    /// the browser gave, never the link itself (its token), and never the site's cookies.
    #[tokio::test]
    async fn a_redirect_carries_no_link_and_no_cookie() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let log = seen.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { return };
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).to_string();
                let first = head.lines().next().unwrap_or_default().to_string();
                log.lock().unwrap().push(head);
                let answer = if first.contains("/a?") {
                    // Another host name for the same server: reqwest counts it as another site.
                    format!("HTTP/1.1 302 Found\r\nLocation: http://localhost:{port}/b\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_string()
                };
                let _ = sock.write_all(answer.as_bytes()).await;
            }
        });
        let src = Source::Http { url: format!("http://127.0.0.1:{port}/a?token=s3cret") };
        let mut f = Fetcher::new(download_client(), &src);
        f.set_browser_headers(Some(crate::extension::BrowserHeaders {
            referrer: "http://page.example/".into(),
            cookies: "sid=42".into(),
            ..Default::default()
        }));
        let resp = f.get(0, None).await.unwrap();
        assert!(resp.status().is_success());
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2, "{seen:?}");
        let first = seen[0].to_ascii_lowercase();
        assert!(first.contains("cookie: sid=42") && first.contains("referer: http://page.example/"), "{first}");
        let second = seen[1].to_ascii_lowercase();
        assert!(!second.contains("s3cret"), "the link's token went on: {second}");
        assert!(!second.contains("sid=42"), "the cookie went on: {second}");
        assert!(second.contains("referer: http://page.example/"), "{second}");
    }

    /// Messages compared later are known in both languages; French ones are said in French.
    #[test]
    fn words_in_french() {
        crate::i18n::test_french(true);
        assert_eq!(crate::model::say(NAME_NOT_FOUND), "Le nom du serveur est introuvable. Vérifiez le lien");
        assert!(crate::model::is_said(NAME_NOT_FOUND[0], NAME_NOT_FOUND), "an English one made before a switch");
        let offline = crate::video::friendly_error("ERROR: Unable to download webpage: timed out").0;
        assert_eq!(offline, "Pas de connexion au site.");
        assert!(crate::video::is_network(&offline));
        assert!(crate::video::worth_new_links("Fichier introuvable sur le serveur (404)."));
        assert!(crate::video::worth_new_links(crate::engine::RESUME_STOPPED[1]));
        crate::i18n::test_french(false);
        assert!(crate::video::is_network("Pas de connexion au site."), "a French one after switching back");
    }
}
