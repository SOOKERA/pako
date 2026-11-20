// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! The browser extension's line to Pako ("native messaging"): the browser starts Pako.exe with
//! its own arguments, sends a few length-prefixed JSON messages on stdin and reads the answers
//! on stdout. Nothing listens on the network.
//!
//! That short-lived Pako.exe never opens a window: it checks each request and hands it to the
//! running Pako over a named pipe that only this Windows user's Pako owns, then passes Pako's
//! real answer back to the browser. If Pako isn't running, it starts it (`Pako.exe
//! --from-browser`, hidden; outside the browser's process group so closing the browser's line
//! doesn't end it) and waits for the pipe.
//!
//! Requests never travel on command lines. Cookies and the referrer sent with a download are
//! kept for that download only: in memory, and in the data folder encrypted by Windows for this
//! Windows user (so the download still works after a restart). They are dropped once it is
//! finished or removed.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// The host name both browsers know Pako by.
pub const HOST_NAME: &str = "app.pako.host";
/// Pako Light: the Chrome Web Store / Edge Add-ons edition (no video downloading, as those stores
/// require), item made 2026-11-16 by the publisher Sookera. Its own ID, so it can sit next to the
/// full extension.
pub const CHROME_STORE_ID: &str = "cojnnpgiilnbkblckkjnheigiodkdlpl";
/// The full extension (video button included), the copy Pako installs itself ("Load unpacked");
/// fixed by the key in extension/manifest.chromium.json.
pub const CHROMIUM_ID: &str = "fnaplmfbaccplngclnkchlngpkbcbhod";
/// Every Chromium extension ID Pako answers. (An Edge Add-ons item gets its own ID: add it here
/// and to `LIGHT_IDS` before that edition is submitted.)
pub const CHROMIUM_IDS: [&str; 2] = [CHROME_STORE_ID, CHROMIUM_ID];
/// The Pako Light IDs: requests from them never become videos (those stores don't allow
/// video downloading). The browser tells the native host which extension started it.
pub const LIGHT_IDS: [&str; 1] = [CHROME_STORE_ID];
/// Pako's answer to Pako Light on the sites it leaves alone (its own wording).
pub const LIGHT_REFUSED: &str = "Pako Light doesn't work on this site.";
/// "Record this video" from a private window (the extension doesn't offer it there either).
pub const NO_PRIVATE_RECORDING: &str = "Pako doesn't record in private windows.";


/// The browsers' list of extensions allowed to start Pako's native host.
fn chromium_origins() -> Vec<String> {
    CHROMIUM_IDS.iter().map(|id| format!("chrome-extension://{id}/")).collect()
}
/// Firefox extension ID (manifest.firefox.json).
pub const FIREFOX_ID: &str = "extension@pako.sookera.com";
/// Argument for a Pako started by the native host: stay hidden in the tray.
pub const FROM_BROWSER: &str = "--from-browser";

const MAX_MESSAGE: usize = 1024 * 1024;
const MAX_URLS: usize = 2000;
const MAX_URL_LEN: usize = 8 * 1024;
const MAX_COOKIES: usize = 16 * 1024;
/// The pipe's message format (changes only if the format does).
const PIPE_VERSION: u32 = 1;
/// How long the host waits for Pako's answer (Pako itself answers within about 30 s).
const ANSWER_WAIT: Duration = Duration::from_secs(60);
/// How long the host waits for a Pako it started (or one still starting) to take requests.
const START_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Request {
    /// "download" (straight to Downloads), "links" (to Links, to pick from), "video" (the
    /// player's Download button: one video straight to Downloads in `quality`), "videoPrepare"
    /// (that button's quality menu opened: Pako starts reading the video, no quality),
    /// "videoPrepareCancel" (that menu closed without a pick: the read stops), "stream" (a video
    /// caught playing on the page: see `crate::stream`), "record" (Record this video: `record`)
    /// or "ping". "videoPrepare", "videoPrepareCancel", "stream" and "record" never come from
    /// Pako Light.
    pub kind: String,
    pub urls: Vec<String>,
    pub referrer: String,
    pub cookies: String,
    pub filename: String,
    /// A download the browser started and holds paused: Pako answers "ok" only once its own
    /// first request for the file got a good answer (the browser deletes its copy then).
    pub takeover: bool,
    /// "video" only: height in lines (1080, 720, 480), or 0 = audio only.
    pub quality: Option<u32>,
    /// Takeover only: the size of the browser's file, as the browser was told (0 = not known).
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    /// Takeover only: the kind of file the browser got ("application/zip", "text/html"…).
    pub mime: String,
    /// Video sites only: the browser's cookies for that site (so videos that need you to be
    /// logged in work). Given to yt-dlp in a temporary file, never on a command line.
    #[serde(rename = "siteCookies")]
    pub site_cookies: Vec<SiteCookie>,
    /// "record" only: one step of a recording (checked by `crate::stream::validate_record`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<crate::stream::RecordRequest>,
    /// From Pako Light (set by the native host from the browser's own launch arguments, never
    /// read from the extension's message): no videos, no video lookups for its links.
    #[serde(skip)]
    pub light: bool,
    /// Takeover only, from an extension that can wait for a second answer: Pako answers at once
    /// with `Reply::held` (the Download with Pako window opens right away) and checks the file
    /// in the background; the extension then asks for the result with a "held" request. Older
    /// extensions don't send it: Pako answers only once checked, as before.
    pub hold: bool,
    /// "held" only: which takeover's result the extension waits for (`Reply::held`).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub held: String,
    /// "follow" only: a download the browser makes because Pako couldn't get it ("Via
    /// browser"), as the browser sees it now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow: Option<FollowReport>,
    /// From a private browser window (the browser lets the extension work there): no trace in
    /// Pako beyond the file (see `Download::private`). Also on a private takeover's "held"
    /// requests, so the helper notes none of their problems.
    pub private: bool,
    /// Takeover with `hold`: the browser holds its copy paused (a form sent with POST, Pako
    /// Light, a private window in Chrome): Pako checks the file first and only then asks you, so
    /// that paused copy never waits on your answer (it may not continue after a long pause).
    pub paused: bool,
}

/// A browser download Pako follows ("Via browser"), as the extension reports it (at most about
/// once a second, and at once when its state changes).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct FollowReport {
    /// Pako's download it belongs to (`Reply::followed`), empty = not known to Pako yet.
    pub pako: String,
    /// The browser's id for its download.
    pub browser_id: i64,
    /// Where the browser saves it (its full path; may be empty early on).
    pub path: String,
    /// Bytes so far and in all (0 = not known).
    pub bytes: u64,
    pub total: u64,
    /// "in_progress", "paused", "complete" or "interrupted".
    pub state: String,
    /// "interrupted": the browser's reason ("USER_CANCELED", "NETWORK_FAILED"...).
    pub error: String,
    pub can_resume: bool,
}

/// One browser cookie, as yt-dlp's cookie file needs it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SiteCookie {
    /// ".youtube.com" (this site and its subdomains) or "www.youtube.com" (that host only).
    pub domain: String,
    pub path: String,
    pub secure: bool,
    /// Unix seconds; 0 = until the browser closes.
    pub expires: u64,
    pub name: String,
    pub value: String,
}

const MAX_SITE_COOKIES: usize = 300;
const MAX_SITE_COOKIE_BYTES: usize = 64 * 1024;

/// The site a link belongs to for cookies ("youtube.com" for www.youtube.com, youtu.be and
/// music.youtube.com).
fn cookie_site(link: &str) -> Option<String> {
    let host = url::Url::parse(link).ok()?.host_str()?.to_ascii_lowercase();
    let host = host.trim_end_matches('.');
    let site = match host {
        "youtu.be" => "youtube.com".to_string(),
        "fb.watch" => "facebook.com".to_string(),
        "dai.ly" => "dailymotion.com".to_string(),
        _ => {
            let labels: Vec<&str> = host.split('.').collect();
            if labels.len() < 2 {
                return None;
            }
            labels[labels.len() - 2..].join(".")
        }
    };
    Some(site)
}

/// Keeps only well-formed cookies of the video's own site: no control characters or tabs
/// (they would break the cookie file or add headers), sensible sizes, no expired ones.
pub fn clean_site_cookies(link: &str, cookies: Vec<SiteCookie>, now: u64) -> Vec<SiteCookie> {
    let Some(site) = cookie_site(link) else { return Vec::new() };
    let plain = |t: &str| !t.chars().any(|c| c.is_control());
    let mut out = Vec::new();
    let mut bytes = 0usize;
    for mut c in cookies {
        c.domain = c.domain.trim().to_ascii_lowercase();
        let bare = c.domain.trim_start_matches('.');
        let of_site = bare == site || bare.ends_with(&format!(".{site}"));
        let domain_ok = !bare.is_empty() && bare.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
        if c.path.is_empty() {
            c.path = "/".into();
        }
        let ok = of_site
            && domain_ok
            && c.path.starts_with('/')
            && c.path.len() <= 1024
            && !c.name.is_empty()
            && c.name.len() <= 256
            && c.value.len() <= 8192
            && plain(&c.name)
            && plain(&c.value)
            && plain(&c.path)
            && !c.name.contains([';', '='])
            && (c.expires == 0 || c.expires > now);
        if !ok {
            continue;
        }
        bytes += c.domain.len() + c.path.len() + c.name.len() + c.value.len();
        if bytes > MAX_SITE_COOKIE_BYTES || out.len() >= MAX_SITE_COOKIES {
            break;
        }
        out.push(c);
    }
    out
}

fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The qualities the player's Download button offers.
pub const VIDEO_QUALITIES: &[u32] = &[1080, 720, 480, 0];

/// Pako's answer (passed to the browser as it is).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Reply {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Pako is still starting: ask again in a moment.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub retry: bool,
    /// The version of the full extension in Pako's "Load unpacked" folder (added by the native
    /// host, never for Pako Light): an older copy running in the browser reloads itself to
    /// pick up the new one after a Pako update, instead of waiting for the browser to restart.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,
    /// A takeover asked with `hold`: Pako has it and is checking it. The extension asks for the
    /// result with `{kind: "held", held}`. (Older extensions never ask with `hold`.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub held: Option<String>,
    /// This Pako answers takeovers at once when asked with `hold` (added by the native host to
    /// every answer, so the extension knows before its next takeover). Older Pakos don't say it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub holds: bool,
    /// The Pako download a browser download belongs to: in a takeover's failure (the browser
    /// downloads it and reports it with this id) and in answers to "follow".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followed: Option<String>,
    /// "follow" answers: what you asked Pako to do with that browser download ("pause",
    /// "resume", "cancel"), done by the extension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// The browser this helper runs for asks "Download …? Save / Open / Cancel" before its
    /// extensions see a download (Vivaldi): the full extension then catches downloads at the
    /// server's answer. Added by the native host, which knows its browser by its program.
    #[serde(rename = "asksFirst", skip_serializing_if = "std::ops::Not::not")]
    pub asks_first: bool,
    /// This Pako reads a video ahead when the player's quality menu opens ("videoPrepare"; added
    /// by the native host, never for Pako Light). Older Pakos don't say it: the extension then
    /// never asks (they would only answer "Unknown request.").
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub prepares: bool,
}

/// Browsers that ask before their extensions see a download (by `seen::browser_id`).
pub fn asks_first(browser: Option<&str>) -> bool {
    browser == Some("vivaldi")
}

impl Reply {
    pub fn ok() -> Self {
        Reply { ok: true, ..Default::default() }
    }
    pub fn error(e: impl Into<String>) -> Self {
        Reply { ok: false, error: Some(e.into()), ..Default::default() }
    }
    /// Not taken, nothing wrong (the browser keeps its download, no warning).
    pub fn declined() -> Self {
        Reply::default()
    }
    pub fn not_ready() -> Self {
        Reply { ok: false, error: Some("Pako is still starting. Try again in a moment.".into()), retry: true, ..Default::default() }
    }
    /// Taken, still being checked: the result comes with a "held" request (see `HELD`).
    pub fn held(token: &str) -> Self {
        Reply { ok: true, held: Some(token.to_string()), ..Default::default() }
    }
    /// The answer to a "held" request when the result isn't known yet: ask again.
    pub fn still_held(token: &str) -> Self {
        Reply { ok: false, held: Some(token.to_string()), ..Default::default() }
    }
}

/// Started by a browser as its native host? Chrome passes `chrome-extension://<id>/`
/// (plus `--parent-window=…`); Firefox passes the manifest path and the extension ID.
pub fn is_host_launch(args: &[String]) -> bool {
    args.iter().any(|a| a.starts_with("chrome-extension://") || a == FIREFOX_ID)
}

/// Started by a browser for Pako Light? Chrome and Edge pass `chrome-extension://<id>/` of the
/// extension that asked.
pub fn is_light_launch(args: &[String]) -> bool {
    args.iter().any(|a| LIGHT_IDS.iter().any(|id| a.trim_end_matches('/') == format!("chrome-extension://{id}")))
}

/// The sites Pako Light leaves alone, whole sites (any page on them): the same list as
/// `LIGHT_SKIPS_SITES` in extension/background.js (a test keeps the two the same).
const LIGHT_SKIPS_SITES: &[&str] = &[
    "youtube.com", "youtu.be", "youtube-nocookie.com", "vimeo.com", "twitch.tv", "facebook.com", "fb.watch",
    "instagram.com", "tiktok.com", "x.com", "twitter.com", "dailymotion.com", "dai.ly", "reddit.com", "rumble.com",
    "odysee.com", "bitchute.com", "bilibili.com", "streamable.com", "soundcloud.com",
];

/// On a site Pako Light leaves alone (any page of it, not only a video's page).
fn light_skips(link: &str) -> bool {
    let Some(host) = url::Url::parse(link).ok().and_then(|u| u.host_str().map(str::to_ascii_lowercase)) else { return false };
    let host = host.trim_end_matches('.');
    let host = host.strip_prefix("www.").unwrap_or(host);
    LIGHT_SKIPS_SITES.iter().any(|s| host == *s || host.ends_with(&format!(".{s}")))
}

/// Pako Light's requests: never a video, nothing from or to the sites it leaves alone (Light
/// checks the same on its side), and its links are marked so Links never turns them into videos
/// (`light`). Links to those sites are left out.
pub fn light_rules(mut r: Request) -> Result<Request, String> {
    match r.kind.as_str() {
        "ping" => return Ok(r),
        "held" => {
            r.light = true;
            return Ok(r);
        }
        "follow" => {
            if r.urls.iter().any(|u| crate::video::is_video_site(u) || light_skips(u)) {
                return Err(LIGHT_REFUSED.into());
            }
            r.light = true;
            return Ok(r);
        }
        "video" | "videoPrepare" | "videoPrepareCancel" | "stream" | "record" => return Err(LIGHT_REFUSED.into()),
        _ => {}
    }
    r.urls.retain(|u| !crate::video::is_video_site(u) && !light_skips(u));
    // The referrer is often only "https://site/" (see `referrer_for`): the whole site counts.
    if r.urls.is_empty() || crate::video::is_video_site(&r.referrer) || light_skips(&r.referrer) {
        return Err(LIGHT_REFUSED.into());
    }
    r.site_cookies.clear();
    r.light = true;
    Ok(r)
}

/// Started by the native host (to take a browser request): no window.
pub fn started_for_browser(args: &[String]) -> bool {
    args.iter().any(|a| a == FROM_BROWSER)
}

/// A web link, written the standard way; None if it isn't one.
fn web_link(u: &str) -> Option<String> {
    let x = url::Url::parse(u.trim()).ok()?;
    (matches!(x.scheme(), "http" | "https") && x.host_str().is_some()).then(|| x.to_string()).filter(|s| s.len() <= MAX_URL_LEN)
}

/// Checks a request: web links only, sensible sizes. Returns the cleaned request.
pub fn validate(mut r: Request) -> Result<Request, String> {
    match r.kind.as_str() {
        "ping" => return Ok(Request { kind: "ping".into(), ..Default::default() }),
        "held" => return validate_held(r),
        "follow" => return validate_follow(r),
        "video" => return validate_video(r),
        "videoPrepare" => return validate_video_prepare(r),
        "videoPrepareCancel" => return validate_video_prepare_cancel(r),
        "stream" => return crate::stream::validate_stream(r),
        "record" => return validate_record(r),
        "download" | "links" => {}
        _ => return Err("Unknown request.".into()),
    }
    r.quality = None;
    // Pako works with the link as the URL parser read it, not the raw text it came in.
    r.urls = r.urls.iter().filter(|u| u.len() <= MAX_URL_LEN).filter_map(|u| web_link(u)).collect();
    r.urls.dedup();
    r.urls.truncate(MAX_URLS);
    if r.urls.is_empty() {
        return Err("Pako only takes http and https links.".into());
    }
    r.referrer = if r.referrer.len() <= MAX_URL_LEN { web_link(&r.referrer).unwrap_or_default() } else { String::new() };
    r.referrer = referrer_for(&r.referrer, &r.urls);
    // A cookie header with line breaks or other control characters could add headers: refused.
    if r.cookies.len() > MAX_COOKIES || r.cookies.chars().any(|c| c.is_control()) {
        r.cookies.clear();
    }
    // A video site's cookies travel only with one link to that video, never from a private
    // window (no cookie file for yt-dlp).
    r.site_cookies = match r.urls.as_slice() {
        [one] if crate::video::is_video_site(one) && !r.private => clean_site_cookies(one, std::mem::take(&mut r.site_cookies), unix_now()),
        _ => Vec::new(),
    };
    // Only a plain file name, never a path. No name stays no name (the link's own name is used
    // then): cleaning an empty one would call the file "download".
    r.filename = clean_filename(&r.filename);
    // Only single browser downloads are held by the browser.
    r.takeover = r.takeover && r.kind == "download" && r.urls.len() == 1;
    r.hold = r.hold && r.takeover;
    r.paused = r.paused && r.hold;
    r.held.clear();
    if !r.takeover {
        r.total_bytes = 0;
    }
    // Only a plain media type ("type/subtype", nothing after it).
    r.mime = if r.takeover { clean_mime(&r.mime) } else { String::new() };
    Ok(r)
}

/// The page a download came from, as a browser sends it to the file's server (its default rule,
/// "strict-origin-when-cross-origin"): the whole address to the same site, only "https://site/"
/// to another site, nothing from an https page to an http link. Never the "#…" part or a login
/// in the address. So a page's private address (a token in it) doesn't reach another site.
fn referrer_for(referrer: &str, urls: &[String]) -> String {
    let Ok(mut page) = url::Url::parse(referrer) else { return String::new() };
    if !matches!(page.scheme(), "http" | "https") || page.host_str().is_none() {
        return String::new();
    }
    page.set_fragment(None);
    let _ = page.set_username("");
    let _ = page.set_password(None);
    let mut other_site = false;
    for u in urls {
        let Ok(to) = url::Url::parse(u) else { return String::new() };
        if page.scheme() == "https" && to.scheme() != "https" {
            return String::new();
        }
        other_site |= to.origin() != page.origin();
    }
    if other_site {
        page.set_path("/");
        page.set_query(None);
    }
    page.to_string()
}

/// The browser's name for a file: its last part only (never a path), cleaned for Windows. Empty
/// when the browser gave none (right-click "Download with Pako" sends none).
fn clean_filename(name: &str) -> String {
    let last = name.rsplit(['/', '\\']).next().unwrap_or_default().trim();
    if last.is_empty() { String::new() } else { crate::names::sanitize(last) }
}

fn clean_mime(m: &str) -> String {
    let m = m.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
    let plain = m.len() <= 127 && m.bytes().all(|b| b.is_ascii_alphanumeric() || b"/.+-_".contains(&b));
    if plain { m } else { String::new() }
}

fn is_html_mime(m: &str) -> bool {
    matches!(m, "text/html" | "application/xhtml+xml")
}

/// A taken-over download: does Pako's first answer from the server look like the file the
/// browser got? The sizes must be the same when both are known, and a web page is refused
/// unless the browser's file was a web page too. `size` and `html` = what Pako's own first
/// answer said (None = not known).
pub fn same_as_browser(r: &Request, size: Option<u64>, html: Option<bool>) -> Result<(), String> {
    if let Some(size) = size.filter(|_| r.total_bytes > 0) {
        if size != r.total_bytes {
            return Err(format!(
                "the server gave Pako a different file ({} bytes, the browser's has {})",
                size, r.total_bytes
            ));
        }
    }
    if html == Some(true) && !is_html_mime(&r.mime) {
        return Err("the server gave Pako a web page instead of the file".into());
    }
    Ok(())
}

/// "follow": one web link (the file's), and the browser's report in sensible bounds.
fn validate_follow(r: Request) -> Result<Request, String> {
    let Some(mut f) = r.follow else { return Err("Unknown request.".into()) };
    let url = r.urls.first().and_then(|u| web_link(u)).ok_or("Pako only takes http and https links.")?;
    if !matches!(f.state.as_str(), "in_progress" | "paused" | "complete" | "interrupted") {
        return Err("Unknown request.".into());
    }
    let token_ok = f.pako.len() <= 64 && f.pako.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !token_ok {
        return Err("Unknown request.".into());
    }
    if f.path.len() > 4096 || f.path.chars().any(|c| c.is_control()) {
        f.path.clear();
    }
    f.error.truncate(64);
    f.error.retain(|c| c.is_ascii_alphanumeric() || c == '_');
    Ok(Request { kind: "follow".into(), urls: vec![url], filename: clean_filename(&r.filename), follow: Some(f), private: r.private, ..Default::default() })
}

/// "held": only the takeover's token (a download id: letters, digits and dashes).
fn validate_held(r: Request) -> Result<Request, String> {
    let ok = !r.held.is_empty() && r.held.len() <= 64 && r.held.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err("Unknown request.".into());
    }
    Ok(Request { kind: "held".into(), held: r.held, private: r.private, ..Default::default() })
}

/// The player's Download button: exactly one link to a video site Pako knows, and one of the
/// offered qualities. Only the site's own cookies travel with it (no referrer or name).
fn validate_video(r: Request) -> Result<Request, String> {
    let quality = r.quality;
    let (url, site_cookies, private) = one_video(r)?;
    let quality = quality.filter(|q| VIDEO_QUALITIES.contains(q)).ok_or("Unknown video quality.")?;
    Ok(Request { kind: "video".into(), urls: vec![url], quality: Some(quality), site_cookies, private, ..Default::default() })
}

/// The player's quality menu opened: the same checks as its "video", with no quality (the read
/// doesn't depend on it).
fn validate_video_prepare(r: Request) -> Result<Request, String> {
    let (url, site_cookies, private) = one_video(r)?;
    Ok(Request { kind: "videoPrepare".into(), urls: vec![url], site_cookies, private, ..Default::default() })
}

/// That menu closed without a pick: the same link checks, and nothing else (no cookies).
fn validate_video_prepare_cancel(r: Request) -> Result<Request, String> {
    let (url, _, private) = one_video(Request { site_cookies: Vec::new(), ..r })?;
    Ok(Request { kind: "videoPrepareCancel".into(), urls: vec![url], private, ..Default::default() })
}

/// A read ahead or its cancel: a menu opening or closing. It never starts Pako (the pick does)
/// and says nothing: no problem noted (the pick says what went wrong).
fn read_ahead(kind: &str) -> bool {
    matches!(kind, "videoPrepare" | "videoPrepareCancel")
}

/// Exactly one link to a video site Pako knows, with only that site's cookies (none at all from
/// a private window: no cookie file for yt-dlp, no trace in Pako).
fn one_video(r: Request) -> Result<(String, Vec<SiteCookie>, bool), String> {
    let [url] = r.urls.as_slice() else { return Err("Send one video at a time.".into()) };
    let url = (url.len() <= MAX_URL_LEN).then(|| web_link(url)).flatten().ok_or("Pako only takes http and https links.")?;
    if !crate::video::is_video_site(&url) {
        return Err("Pako can't download videos from this page.".into());
    }
    let site_cookies = if r.private { Vec::new() } else { clean_site_cookies(&url, r.site_cookies, unix_now()) };
    Ok((url, site_cookies, r.private))
}

/// One step of "Record this video": checked (the checked form is made again where it is used),
/// nothing else travels with it. Never from a private window (the extension doesn't offer it
/// there either).
fn validate_record(r: Request) -> Result<Request, String> {
    if r.private {
        return Err(NO_PRIVATE_RECORDING.into());
    }
    let rec = r.record.ok_or("Unknown request.")?;
    crate::stream::validate_record(rec.clone(), r.light)?;
    Ok(Request { kind: "record".into(), record: Some(rec), ..Default::default() })
}

// ---------- message framing (4-byte little-endian length, then JSON) ----------

fn frame_len(len: [u8; 4]) -> std::io::Result<usize> {
    let n = u32::from_le_bytes(len) as usize;
    if n > MAX_MESSAGE {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "message too big"));
    }
    Ok(n)
}

pub fn read_message(input: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut len = [0u8; 4];
    match input.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let mut buf = vec![0u8; frame_len(len)?];
    input.read_exact(&mut buf)?;
    Ok(Some(buf))
}

pub fn write_message(out: &mut impl Write, json: &[u8]) -> std::io::Result<()> {
    if json.len() > MAX_MESSAGE {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "message too big"));
    }
    out.write_all(&(json.len() as u32).to_le_bytes())?;
    out.write_all(json)?;
    out.flush()
}

async fn read_message_async(input: &mut (impl tokio::io::AsyncRead + Unpin)) -> std::io::Result<Option<Vec<u8>>> {
    use tokio::io::AsyncReadExt;
    let mut len = [0u8; 4];
    match input.read_exact(&mut len).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let mut buf = vec![0u8; frame_len(len)?];
    input.read_exact(&mut buf).await?;
    Ok(Some(buf))
}

async fn write_message_async(out: &mut (impl tokio::io::AsyncWrite + Unpin), json: &[u8]) -> std::io::Result<()> {
    use tokio::io::AsyncWriteExt;
    out.write_all(&(json.len() as u32).to_le_bytes()).await?;
    out.write_all(json).await?;
    out.flush().await
}

/// What travels on the pipe from the host to Pako. `light`: the host was started for Pako Light
/// (the host sets it from the browser's launch arguments; the extension can't).
#[derive(Debug, Serialize, Deserialize)]
struct PipeMessage {
    pako: u32,
    request: Request,
    #[serde(default)]
    light: bool,
}

fn pipe_message(r: &Request) -> Vec<u8> {
    serde_json::to_vec(&PipeMessage { pako: PIPE_VERSION, request: r.clone(), light: r.light }).unwrap_or_default()
}

/// Reads and checks a request that came through the pipe (checked again on Pako's side).
fn parse_pipe_message(bytes: &[u8]) -> Result<Request, String> {
    let m: PipeMessage = serde_json::from_slice(bytes).map_err(|_| "Unreadable request.".to_string())?;
    if m.pako != PIPE_VERSION {
        return Err("This Pako and its browser helper don't match. Restart Pako.".into());
    }
    let r = validate(m.request)?;
    if m.light { light_rules(r) } else { Ok(r) }
}

// ---------- the native host (runs in the short-lived Pako.exe the browser starts) ----------

/// The native host: answers the browser until it closes the line. `args`: the browser's launch
/// arguments (they say which extension asked).
pub fn run_host(args: &[String]) -> i32 {
    let started = Instant::now();
    let light = is_light_launch(args);
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    // Which browser this is, for "Connected" in Settings (noted after each answer is sent).
    let mut seen = crate::seen::HostNote::default();
    // Found while the first message is read and handed over (its answer says whether the
    // browser asks first: `asks_first`).
    seen.look_ahead();
    loop {
        let asked = Instant::now();
        let msg = match read_host_message(&mut input) {
            Ok(HostMessage::Request(m)) => Ok(m),
            // Too big to take: said to the browser, and the line stays open.
            Ok(HostMessage::TooBig) => Err("Too many links at once. Pako takes up to about 1 MB of links per request.".to_string()),
            Ok(HostMessage::Closed) => return 0,
            Err(_) => return 1,
        };
        let read = Instant::now();
        let version = msg.as_deref().ok().and_then(crate::seen::version_in);
        let parsed = msg.and_then(|m| serde_json::from_slice::<Request>(&m).map_err(|_| "Unreadable request.".to_string()));
        // A private window's: none of its problems in the problem list. Nor a read ahead's or
        // its cancel's (they say nothing; the send that follows says what went wrong), also
        // when its checks fail.
        let quiet = parsed.as_ref().is_ok_and(|r| r.private || read_ahead(&r.kind));
        let checked = parsed.and_then(validate);
        let private = checked.as_ref().is_ok_and(|r| r.private);
        let mut reply = match checked.and_then(|r| if light { light_rules(r) } else { Ok(r) }) {
            // "Connected": the browser reached this helper (Pako itself starts when needed).
            Ok(r) if r.kind == "ping" => Reply::ok(),
            Ok(r) => hand_over(&r),
            Err(e) => Reply::error(e),
        };
        if !light {
            reply.ext = folder_version();
        }
        // This Pako answers takeovers at once (see `Request::hold`).
        reply.holds = true;
        if !light {
            reply.asks_first = asks_first(seen.browser());
            reply.prepares = true;
        }
        if let Some(e) = reply.error.as_deref().filter(|e| *e != LIGHT_REFUSED && !quiet) { crate::errorlog::note(crate::errorlog::Area::Extension, e); }
        let json = serde_json::to_vec(&reply).unwrap_or_default();
        if write_message(&mut output, &json).is_err() {
            return 1;
        }
        // (A private window's request: not in the timing log either.)
        if !private {
            timing(&format!(
                "host: reading from {} ms after start, answered {} ms after the message (ok {}, held {})",
                asked.duration_since(started).as_millis(),
                read.elapsed().as_millis(),
                reply.ok,
                reply.held.is_some()
            ));
        }
        // A private window's request leaves no "last seen" time on disk either.
        if !private {
            seen.note(light, version, unix_now());
        }
    }
}

enum HostMessage {
    Request(Vec<u8>),
    /// Bigger than Pako takes: skipped, so the next message reads normally.
    TooBig,
    Closed,
}

/// The browsers send at most 64 MB to a native host.
const MAX_FROM_BROWSER: u64 = 64 * 1024 * 1024;

fn read_host_message(input: &mut impl Read) -> std::io::Result<HostMessage> {
    let mut len = [0u8; 4];
    match input.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(HostMessage::Closed),
        Err(e) => return Err(e),
    }
    let n = u32::from_le_bytes(len) as u64;
    if n > MAX_MESSAGE as u64 {
        if n > MAX_FROM_BROWSER {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "message too big"));
        }
        let skipped = std::io::copy(&mut input.by_ref().take(n),&mut std::io::sink())?;
        if skipped != n {
            return Ok(HostMessage::Closed);
        }
        return Ok(HostMessage::TooBig);
    }
    let mut buf = vec![0u8; n as usize];
    input.read_exact(&mut buf)?;
    Ok(HostMessage::Request(buf))
}

enum Connect {
    NotRunning,
    Busy,
    /// The pipe belongs to another Windows user: never talk to it.
    Stranger,
    Failed(String),
}

/// Hands a request to the running Pako (starting it if needed) and returns its answer.
fn hand_over(r: &Request) -> Reply {
    let begun = Instant::now();
    let deadline = begun + START_WAIT;
    let mut started = false;
    loop {
        match connect() {
            Ok(pipe) => {
                let connected = Instant::now();
                match exchange(pipe, r) {
                    Ok(reply) if reply.retry && Instant::now() < deadline => {}
                    Ok(reply) => {
                        if !r.private {
                            timing(&format!(
                                "host: {} pipe reached after {} ms (Pako started: {started}), answer {} ms later",
                                r.kind,
                                connected.duration_since(begun).as_millis(),
                                connected.elapsed().as_millis()
                            ));
                        }
                        return reply;
                    }

                    Err(e) => return Reply::error(format!("Pako didn't answer ({e}). The browser keeps the download.")),
                }
            }
            // The result of a takeover only the running Pako knows: a Pako started now can't
            // tell it (the browser gets its download back).
            Err(Connect::NotRunning) if r.kind == "held" => {
                return Reply::error("Pako closed before it finished checking this download.");
            }
            // A browser download's progress never starts Pako (you may have closed it on
            // purpose): the extension keeps it and reports again once Pako runs.
            Err(Connect::NotRunning) if r.kind == "follow" => {
                return Reply::error("Pako isn't running.");
            }
            // Nor does a quality menu opening or closing (the pick starts Pako, as before).
            Err(Connect::NotRunning) if read_ahead(&r.kind) => {
                return Reply::error("Pako isn't running.");
            }
            Err(Connect::NotRunning) if !started => {
                if let Err(e) = start_pako() {
                    return Reply::error(e);
                }
                started = true;
            }
            Err(Connect::NotRunning | Connect::Busy) => {}
            Err(Connect::Stranger) => return Reply::error("Something other than your Pako answered, so nothing was sent."),
            Err(Connect::Failed(e)) => return Reply::error(format!("Couldn't reach Pako: {e}")),
        }
        if Instant::now() >= deadline {
            return Reply::error("Pako didn't start in time. Open Pako, then try again.");
        }
        std::thread::sleep(poll_wait(begun.elapsed()));
    }
}

/// How long the host waits before looking for Pako's pipe again: often in the first second
/// (Pako is usually just finishing its start), then every 250 ms.
fn poll_wait(waited: Duration) -> Duration {
    if waited < Duration::from_secs(1) { Duration::from_millis(50) } else { Duration::from_millis(250) }
}

/// Timings of the browser's requests, for measuring only: written to `pako-timing.log` in the
/// temp folder when PAKO_TIMING is set, otherwise nothing at all.
pub fn timing(line: &str) {
    static ON: LazyLock<bool> = LazyLock::new(|| std::env::var_os("PAKO_TIMING").is_some());
    if !*ON {
        return;
    }
    let file = std::env::temp_dir().join("pako-timing.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
        let _ = writeln!(f, "{} [{}] {line}", unix_now(), std::process::id());
    }
}

fn connect() -> Result<std::fs::File, Connect> {
    const ERROR_PIPE_BUSY: i32 = 231;
    let name = pipe_name().ok_or_else(|| Connect::Failed("can't tell who is signed in to Windows".into()))?;
    let pipe = match std::fs::OpenOptions::new().read(true).write(true).open(&name) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Connect::NotRunning),
        Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => return Err(Connect::Busy),
        Err(e) => return Err(Connect::Failed(e.to_string())),
    };
    if !server_is_this_user(&pipe) {
        return Err(Connect::Stranger);
    }
    Ok(pipe)
}

/// Sends the request and waits (a limited time) for the answer.
fn exchange(mut pipe: std::fs::File, r: &Request) -> Result<Reply, String> {
    let msg = pipe_message(r);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let answer = write_message(&mut pipe, &msg)
            .and_then(|_| read_message(&mut pipe))
            .map_err(|e| e.to_string())
            .and_then(|m| m.ok_or_else(|| "the line closed".to_string()))
            .and_then(|m| serde_json::from_slice::<Reply>(&m).map_err(|_| "unreadable answer".to_string()));
        let _ = tx.send(answer);
    });
    rx.recv_timeout(ANSWER_WAIT).unwrap_or_else(|_| Err("no answer in time".into()))
}

/// Starts Pako hidden, outside the browser's job (Firefox ends the host's whole process group
/// when it closes the line). Windows refuses that when the job doesn't allow it: then normally.
fn start_pako() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_BREAKAWAY_FROM_JOB;
    let exe = std::env::current_exe().map_err(|e| format!("Couldn't find Pako: {e}"))?;
    let spawn = |flags: u32| {
        std::process::Command::new(&exe)
            .arg(FROM_BROWSER)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(flags)
            .spawn()
    };
    spawn(CREATE_BREAKAWAY_FROM_JOB).or_else(|_| spawn(0)).map(|_| ()).map_err(|e| format!("Couldn't start Pako: {e}"))
}

// ---------- the pipe ----------

/// This Windows user's id (SID), as bytes.
fn user_sid(process: windows_sys::Win32::Foundation::HANDLE) -> Option<Vec<u8>> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{GetLengthSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows_sys::Win32::System::Threading::OpenProcessToken;
    // SAFETY: the token handle is closed before returning; the buffer is sized by the first
    // call, aligned for TOKEN_USER (u64 storage), and the SID read stays inside it.
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut len = 0u32;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
        let ok = GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), (buf.len() * 8) as u32, &mut len);
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let user = &*(buf.as_ptr() as *const TOKEN_USER);
        let sid = user.User.Sid;
        let n = GetLengthSid(sid) as usize;
        Some(std::slice::from_raw_parts(sid as *const u8, n).to_vec())
    }
}

fn my_sid() -> Option<Vec<u8>> {
    static SID: LazyLock<Option<Vec<u8>>> =
        LazyLock::new(|| user_sid(unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() }));
    SID.clone()
}

/// The pipe's name: one per Windows user.
fn pipe_name() -> Option<String> {
    let hex: String = my_sid()?.iter().map(|b| format!("{b:02x}")).collect();
    Some(format!(r"\\.\pipe\pako-browser-{hex}"))
}

/// How trusted a process is in Windows (its integrity level: low, medium, high…).
fn integrity_level(process: windows_sys::Win32::Foundation::HANDLE) -> Option<u32> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenIntegrityLevel,
    };
    use windows_sys::Win32::System::Threading::OpenProcessToken;
    // SAFETY: as in `user_sid`: the token is closed, the buffer is sized by the first call and
    // aligned for TOKEN_MANDATORY_LABEL; the SID read stays inside it.
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut len = 0u32;
        GetTokenInformation(token, TokenIntegrityLevel, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
        let ok = GetTokenInformation(token, TokenIntegrityLevel, buf.as_mut_ptr().cast(), (buf.len() * 8) as u32, &mut len);
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let label = &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL);
        let sid = label.Label.Sid;
        let count = *GetSidSubAuthorityCount(sid);
        if count == 0 {
            return None;
        }
        Some(*GetSidSubAuthority(sid, (count - 1) as u32))
    }
}

/// The program file a process runs.
pub(crate) fn process_image(process: windows_sys::Win32::Foundation::HANDLE) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Threading::{PROCESS_NAME_WIN32, QueryFullProcessImageNameW};
    let mut buf = vec![0u16; 32 * 1024];
    let mut len = buf.len() as u32;
    // SAFETY: the buffer holds `len` UTF-16 units; Windows writes at most that many.
    let ok = unsafe { QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) };
    (ok != 0).then(|| PathBuf::from(std::ffi::OsString::from_wide(&buf[..len as usize])))
}

/// The same file (paths written differently, short names and letter case don't matter).
fn same_program(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a.as_os_str().to_string_lossy().to_lowercase() == b.as_os_str().to_string_lossy().to_lowercase(),
        _ => false,
    }
}

/// The program at the other end of the pipe is this Pako.exe, runs as this Windows user, and
/// isn't less trusted by Windows than this helper (a sandboxed program can't pose as Pako).
fn server_is_this_user(pipe: &std::fs::File) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let Some(mine) = my_sid() else { return false };
    let Ok(my_exe) = std::env::current_exe() else { return false };
    // SAFETY: the pipe handle is valid while `pipe` lives; the process handle is closed.
    unsafe {
        let mut pid = 0u32;
        if GetNamedPipeServerProcessId(pipe.as_raw_handle() as _, &mut pid) == 0 {
            return false;
        }
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let theirs = user_sid(h);
        let their_level = integrity_level(h);
        let their_exe = process_image(h);
        CloseHandle(h);
        let my_level = integrity_level(GetCurrentProcess());
        theirs.as_deref() == Some(mine.as_slice())
            && matches!((their_level, my_level), (Some(t), Some(m)) if t >= m)
            && their_exe.is_some_and(|e| same_program(&e, &my_exe))
    }
}

/// How long the pipe waits before it is opened again after a problem (`tries` in a row): a
/// little longer each time, at most 30 s.
fn reopen_wait(tries: u32) -> Duration {
    Duration::from_millis(500u64.saturating_mul(1u64 << tries.min(6))).min(Duration::from_secs(30))
}

/// In Pako: takes requests from the native host until Pako quits. `handle` gives each request's
/// answer. Only the first Pako can own the pipe (another program holding the name makes opening
/// it fail, and the host then refuses to talk to that program). After a problem the pipe is
/// opened again, a little later each time, so the browser's line comes back by itself.
pub async fn serve<F, Fut>(handle: F) -> std::io::Result<()>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Reply> + Send + 'static,
{
    let name = pipe_name().ok_or_else(|| std::io::Error::other("can't tell who is signed in to Windows"))?;
    let handle = Arc::new(handle);
    let mut tries = 0u32;
    loop {
        let served = std::sync::atomic::AtomicBool::new(false);
        let _ = serve_pipe(&name, &handle, &served).await;
        if served.load(std::sync::atomic::Ordering::Relaxed) {
            tries = 0;
        }
        tokio::time::sleep(reopen_wait(tries)).await;
        tries = tries.saturating_add(1);
    }
}

/// Serves the pipe until opening a fresh end fails. `served` = at least one caller was taken.
async fn serve_pipe<F, Fut>(name: &str, handle: &Arc<F>, served: &std::sync::atomic::AtomicBool) -> std::io::Result<()>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Reply> + Send + 'static,
{
    use tokio::net::windows::named_pipe::ServerOptions;
    let mut server = ServerOptions::new().first_pipe_instance(true).reject_remote_clients(true).create(name)?;
    loop {
        if server.connect().await.is_err() {
            // That caller left early: wait for the next one on a fresh end.
            tokio::time::sleep(Duration::from_millis(50)).await;
            server = ServerOptions::new().reject_remote_clients(true).create(name)?;
            continue;
        }
        served.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut client = server;
        let h = handle.clone();
        // This caller is answered even if the next end can't be opened.
        tokio::spawn(async move {
            let Ok(Ok(Some(msg))) = tokio::time::timeout(Duration::from_secs(10), read_message_async(&mut client)).await else { return };
            let reply = match parse_pipe_message(&msg) {
                Ok(r) => h(r).await,
                Err(e) => Reply::error(e),
            };
            let _ = write_message_async(&mut client, &serde_json::to_vec(&reply).unwrap_or_default()).await;
        });
        server = ServerOptions::new().reject_remote_clients(true).create(name)?;
    }
}

// ---------- one answer for the same takeover asked twice ----------

/// How long a takeover counts as "the same one" when it comes again (the full extension and
/// Pako Light both seeing one browser download, or a double click).
const SAME_TAKEOVER: Duration = Duration::from_secs(30);
/// How long a second request waits for the first one's answer.
const SAME_TAKEOVER_WAIT: Duration = Duration::from_secs(50);
/// Once the first one has its answer, how long a repeat still counts as the same takeover (the
/// other extension a moment later, a double click). Later, it's another click: a new takeover,
/// unless the first is still being checked (held).
const SAME_TAKEOVER_AFTER: Duration = Duration::from_secs(5);

/// Takeovers in flight, by (link, size the browser was told): the first one is worked out, the
/// same one arriving again within `window` waits for it and gets the same answer (ok stays ok,
/// anything else becomes "declined", so the browser keeps its copy without a second warning).
/// A first one answered "held" (still being checked) hands its token on: a second request that
/// can wait too gets the same token, an older one waits for the check's result.
pub struct TakeoverGate {
    window: Duration,
    seen: Mutex<HashMap<(String, u64), Seen>>,
}

/// One takeover the gate answers repeats of.
struct Seen {
    since: Instant,
    rx: tokio::sync::watch::Receiver<Option<(bool, Option<String>)>>,
    /// When the first one got its answer (None: still running).
    done: Option<Instant>,
}

impl Seen {
    /// Still "the same takeover" for a repeat: running (within the window); or answered a moment
    /// ago; or answered "held" and still being checked. A takeover cancelled or removed in
    /// Pako is forgotten (`forget`), so clicking the link again takes it again.
    fn alive(&self, window: Duration) -> bool {
        match self.done {
            None => self.since.elapsed() < window,
            Some(done) => {
                done.elapsed() < SAME_TAKEOVER_AFTER.min(window)
                    || self.rx.borrow().as_ref().and_then(|(_, t)| t.as_deref()).is_some_and(|t| HELD.checking(t))
            }
        }
    }
}

impl TakeoverGate {
    pub fn new(window: Duration) -> Self {
        TakeoverGate { window, seen: Mutex::new(HashMap::new()) }
    }

    /// A takeover cancelled or removed in Pako (its held token, or the download's id): a repeat
    /// of it is a new takeover, never the old answer replayed (the browser already dropped its
    /// copy, so a replayed "ok" would lose the download).
    pub fn forget(&self, token: &str) {
        self.seen.lock().unwrap().retain(|_, s| s.rx.borrow().as_ref().and_then(|(_, t)| t.as_deref()) != Some(token));
    }

    /// Runs `work` for the first request with this key; an identical one meanwhile waits for it.
    /// `hold`: this request can take a "held" answer (see `Request::hold`).
    pub async fn run(&self, key: (String, u64), hold: bool, work: impl std::future::Future<Output = Reply>) -> Reply {
        let (first, mut follow) = {
            let mut seen = self.seen.lock().unwrap();
            let window = self.window;
            seen.retain(|_, s| s.alive(window));
            match seen.get(&key) {
                Some(s) => (None, s.rx.clone()),
                None => {
                    let (tx, rx) = tokio::sync::watch::channel(None);
                    seen.insert(key.clone(), Seen { since: Instant::now(), rx: rx.clone(), done: None });
                    (Some(tx), rx)
                }
            }
        };
        if let Some(tx) = first {
            let reply = work.await;
            let _ = tx.send(Some((reply.ok, reply.held.clone())));
            if let Some(s) = self.seen.lock().unwrap().get_mut(&key) {
                s.done = Some(Instant::now());
            }
            return reply;
        }
        let answer = match tokio::time::timeout(SAME_TAKEOVER_WAIT, follow.wait_for(|v| v.is_some())).await {
            Ok(Ok(v)) => v.clone(),
            _ => None,
        };
        match answer {
            Some((true, Some(token))) if hold => Reply::held(&token),
            Some((true, Some(token))) => match HELD.wait(&token, HELD_WAIT).await {
                r if r.ok => Reply::ok(),
                _ => Reply::declined(),
            },
            Some((true, None)) => Reply::ok(),
            _ => Reply::declined(),
        }
    }
}

/// Pako's takeovers from every extension in every browser.
pub static TAKEOVERS: LazyLock<TakeoverGate> = LazyLock::new(|| TakeoverGate::new(SAME_TAKEOVER));

// ---------- takeovers answered at once, checked in the background ----------

/// How long a takeover from an older extension, waiting behind the same held takeover, waits
/// for its check (see `TakeoverGate`; the native host waits 60 s for any answer).
pub const HELD_WAIT: Duration = Duration::from_secs(50);
/// How long the extension's own "held" request waits before it is told "ask again": well under
/// the 30 s a browser lets an idle extension background run, so the extension is never stopped
/// while it waits (and with it the browser's fallback).
pub const HELD_ASK_WAIT: Duration = Duration::from_secs(20);
/// A finished check's result is kept this long for a late "held" request.
const HELD_KEEP: Duration = Duration::from_secs(300);
/// The extension asks again at least every `HELD_ASK_WAIT`: nobody asked for this long = no
/// extension waits for it any more (the browser closed, or stopped the extension).
const HELD_LISTEN: Duration = Duration::from_secs(90);

/// A check's result: Ok = Pako has the file (the browser drops its copy), Err = what to tell the
/// user (the browser downloads it itself).
pub type HeldOutcome = Result<(), String>;

struct Held {
    /// When it was opened, then when its result came (what `HELD_KEEP` counts from).
    since: Instant,
    /// When the extension last asked for it.
    asked: Instant,
    tx: tokio::sync::watch::Sender<Option<HeldOutcome>>,
}

/// Takeovers Pako answered "held": the result of each one's check, by token. The first result
/// counts (the check's, or the window's Cancel, or the wait running out); later ones change
/// nothing. A check still waiting (your answer in the window, however long) is never dropped.
pub struct HeldChecks {
    map: Mutex<HashMap<String, Held>>,
}

impl HeldChecks {
    pub fn new() -> Self {
        HeldChecks { map: Mutex::new(HashMap::new()) }
    }

    /// A takeover answered "held": its check starts now.
    pub fn open(&self, token: &str) {
        let mut map = self.map.lock().unwrap();
        map.retain(|_, h| h.tx.borrow().is_none() || h.since.elapsed() < HELD_KEEP);
        let now = Instant::now();
        map.insert(token.to_string(), Held { since: now, asked: now, tx: tokio::sync::watch::channel(None).0 });
    }

    /// The result, unless one came first. Returns the result that counts (None = not a held
    /// takeover).
    pub fn finish(&self, token: &str, outcome: HeldOutcome) -> Option<HeldOutcome> {
        let mut map = self.map.lock().unwrap();
        let h = map.get_mut(token)?;
        let first = h.tx.send_if_modified(|v| {
            if v.is_none() {
                *v = Some(outcome);
                true
            } else {
                false
            }
        });
        if first {
            h.since = Instant::now();
        }
        let fin = h.tx.borrow().clone();
        fin
    }

    /// Still being checked.
    pub fn checking(&self, token: &str) -> bool {
        self.map.lock().unwrap().get(token).is_some_and(|h| h.tx.borrow().is_none())
    }

    /// An extension still waits for this one's result (it asked lately).
    pub fn listening(&self, token: &str) -> bool {
        self.map.lock().unwrap().get(token).is_some_and(|h| h.asked.elapsed() < HELD_LISTEN)
    }

    /// The extension's "held" request: waits (at most `limit`) for the result. Not known yet
    /// (it still waits for your answer in the Download with Pako window): `Reply::still_held`,
    /// and the extension asks again.
    pub async fn wait(&self, token: &str, limit: Duration) -> Reply {
        let rx = self.map.lock().unwrap().get_mut(token).map(|h| {
            h.asked = Instant::now();
            h.tx.subscribe()
        });
        let Some(mut rx) = rx else {
            return Reply::error("Pako lost track of this download (was it restarted?), so your browser downloads it.");
        };
        let got = match tokio::time::timeout(limit, rx.wait_for(|v| v.is_some())).await {
            Ok(Ok(v)) => v.clone(),
            _ => None,
        };
        match got {
            Some(Ok(())) => Reply::ok(),
            Some(Err(e)) => Reply::error(e),
            None => Reply::still_held(token),
        }
    }

    /// For the tests: as if it was opened (or finished) and last asked for `ago`.
    #[cfg(test)]
    fn age(&self, token: &str, ago: Duration) {
        if let Some(h) = self.map.lock().unwrap().get_mut(token) {
            h.since = Instant::now() - ago;
            h.asked = Instant::now() - ago;
        }
    }
}

impl Default for HeldChecks {
    fn default() -> Self {
        Self::new()
    }
}

/// Every held takeover in this Pako.
pub static HELD: LazyLock<HeldChecks> = LazyLock::new(HeldChecks::new);

/// The server's first answer to a browser download's own first request (its size, and whether
/// it is a web page), by download id: what a takeover compares with the browser's file.
static FIRST_ANSWERS: LazyLock<Mutex<HashMap<String, (Instant, Option<u64>, bool)>>> = LazyLock::new(Default::default);

/// Noted by the engine for downloads that came from the browser (kept a few minutes).
pub fn note_first_answer(download_id: &str, size: Option<u64>, html: bool) {
    let mut map = FIRST_ANSWERS.lock().unwrap();
    map.retain(|_, (since, _, _)| since.elapsed() < HELD_KEEP);
    map.insert(download_id.to_string(), (Instant::now(), size, html));
}

/// Takes that first answer, once it came.
pub fn first_answer(download_id: &str) -> Option<(Option<u64>, bool)> {
    FIRST_ANSWERS.lock().unwrap().remove(download_id).map(|(_, size, html)| (size, html))
}

// ---------- per-download browser headers (kept encrypted for a restart) ----------

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BrowserHeaders {
    pub referrer: String,
    pub cookies: String,
    /// Videos: the site's cookies for yt-dlp (see `SiteCookie`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub site_cookies: Vec<SiteCookie>,
    /// A private window's: kept in memory only, never in the file.
    #[serde(skip)]
    pub private: bool,
}

/// A download's headers are kept at least this long after they arrive, even before the download
/// shows in the list (it is added to the list just after its headers).
const HEADERS_GRACE: Duration = Duration::from_secs(120);

struct Kept {
    headers: BrowserHeaders,
    since: Instant,
}

/// The headers by download id, and the file they are kept in (None = memory only: the browser's
/// helper, tests, or before Pako has read the file at start).
#[derive(Default)]
struct HeaderStore {
    map: HashMap<String, Kept>,
    file: Option<PathBuf>,
}

impl HeaderStore {
    fn insert(&mut self, id: &str, headers: BrowserHeaders) {
        if headers.referrer.is_empty() && headers.cookies.is_empty() && headers.site_cookies.is_empty() {
            return;
        }
        self.map.insert(id.to_string(), Kept { headers, since: Instant::now() });
        self.save();
    }

    fn remove(&mut self, id: &str) {
        if self.map.remove(id).is_some() {
            self.save();
        }
    }

    /// Kept in memory only from now on (the download went into Private mode): out of the file.
    fn make_private(&mut self, id: &str) {
        if let Some(k) = self.map.get_mut(id) {
            if !k.headers.private {
                k.headers.private = true;
                self.save();
            }
        }
    }

    fn retain(&mut self, grace: Duration, keep: impl Fn(&str) -> bool) {
        let before = self.map.len();
        self.map.retain(|id, k| keep(id) || k.since.elapsed() < grace);
        if self.map.len() != before {
            self.save();
        }
    }

    /// Reads the file (headers already in memory win) and keeps it up to date from now on.
    fn open(&mut self, file: PathBuf) {
        let now = Instant::now();
        for (id, headers) in read_kept_headers(&file) {
            self.map.entry(id).or_insert(Kept { headers, since: now });
        }
        self.file = Some(file);
        self.save();
    }

    /// Encrypted by Windows for this Windows user; no file at all when nothing is kept.
    fn save(&self) {
        let Some(file) = &self.file else { return };
        if self.map.is_empty() {
            let _ = std::fs::remove_file(file);
            return;
        }
        let plain: HashMap<&str, &BrowserHeaders> =
            self.map.iter().filter(|(_, k)| !k.headers.private).map(|(id, k)| (id.as_str(), &k.headers)).collect();
        let Ok(json) = serde_json::to_vec(&plain) else { return };
        let Ok(sealed) = crate::secure::protect(&json) else { return };
        let tmp = file.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));
        if std::fs::write(&tmp, sealed).is_err() || std::fs::rename(&tmp, file).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

fn read_kept_headers(file: &std::path::Path) -> HashMap<String, BrowserHeaders> {
    std::fs::read(file)
        .ok()
        .and_then(|b| crate::secure::unprotect(&b).ok())
        .and_then(|j| serde_json::from_slice(&j).ok())
        .unwrap_or_default()
}

static HEADERS: LazyLock<Mutex<HeaderStore>> = LazyLock::new(Default::default);

/// At Pako's start, before any download runs: the headers kept from last time, and keeping them
/// in the data folder from now on.
pub fn load_headers() {
    HEADERS.lock().unwrap().open(crate::paths::data_dir().join("browser-headers.dat"));
}

pub fn remember_headers(download_id: &str, h: BrowserHeaders) {
    HEADERS.lock().unwrap().insert(download_id, h);
}

pub fn headers_for(download_id: &str) -> Option<BrowserHeaders> {
    HEADERS.lock().unwrap().map.get(download_id).map(|k| k.headers.clone())
}

pub fn forget_headers(download_id: &str) {
    HEADERS.lock().unwrap().remove(download_id);
}

/// A download sent into Private mode (Download privately): its browser login and page stay in
/// memory only, never in browser-headers.dat (taken out of it now).
pub fn mark_headers_private(download_id: &str) {
    HEADERS.lock().unwrap().make_private(download_id);
}

/// Drops the headers of downloads that finished or are gone (`keep` = still needs them). Headers
/// that just arrived stay a little while: their download may not be in the list yet.
pub fn keep_headers(keep: impl Fn(&str) -> bool) {
    HEADERS.lock().unwrap().retain(HEADERS_GRACE, keep);
}

// ---------- registering Pako with the browsers ----------

/// Where each browser looks for native hosts (per user, no admin needed). Opera and Opera GX
/// have no key of their own: like Chromium, they read Chrome's.
const CHROMIUM_KEYS: &[&str] = &[
    r"Software\Google\Chrome\NativeMessagingHosts",
    r"Software\Chromium\NativeMessagingHosts",
    r"Software\Microsoft\Edge\NativeMessagingHosts",
    r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts",
    r"Software\Vivaldi\NativeMessagingHosts",
];
/// Firefox and the browsers built on it (Zen, LibreWolf, Waterfox, Floorp) read Mozilla's key on
/// Windows. LibreWolf builds from 2022 read their own instead; an unused key does no harm.
const FIREFOX_KEYS: &[&str] = &[r"Software\Mozilla\NativeMessagingHosts", r"Software\LibreWolf\NativeMessagingHosts"];

/// Every key Pako writes its host into (and removes it from).
fn all_keys() -> impl Iterator<Item = &'static &'static str> {
    CHROMIUM_KEYS.iter().chain(FIREFOX_KEYS)
}

fn host_dir() -> PathBuf {
    crate::paths::data_dir().join("browser")
}

/// Connects Pako to the browsers (or disconnects it): writes the host files and the
/// per-user registry entries pointing at them.
pub fn set_registered(on: bool) -> Result<(), String> {
    if !on {
        for k in all_keys() {
            reg_delete(&format!(r"{k}\{HOST_NAME}"));
        }
        let _ = std::fs::remove_dir_all(host_dir());
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = host_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let chrome = dir.join("chromium-host.json");
    let firefox = dir.join("firefox-host.json");
    let base = |extra: (&str, serde_json::Value)| {
        let mut m = serde_json::json!({
            "name": HOST_NAME,
            "description": "Pako download manager",
            "path": exe.to_string_lossy(),
            "type": "stdio",
        });
        m[extra.0] = extra.1;
        serde_json::to_vec_pretty(&m).unwrap_or_default()
    };
    std::fs::write(&chrome, base(("allowed_origins", serde_json::json!(chromium_origins()))))
        .map_err(|e| e.to_string())?;
    std::fs::write(&firefox, base(("allowed_extensions", serde_json::json!([FIREFOX_ID])))).map_err(|e| e.to_string())?;
    for k in CHROMIUM_KEYS {
        reg_set_default(&format!(r"{k}\{HOST_NAME}"), &chrome.to_string_lossy())?;
    }
    for k in FIREFOX_KEYS {
        reg_set_default(&format!(r"{k}\{HOST_NAME}"), &firefox.to_string_lossy())?;
    }
    Ok(())
}

/// Uninstalling: disconnects the browsers from this copy only. Each browser's entry is removed
/// when it points at this copy's host files (in `data_dir`); an entry made by another Pako (a
/// portable one) stays. The host files go too: they only point at the exe being removed.
/// A portable Pako that can't write next to itself keeps its data in the same folder as the
/// installed one: host files there that start another Pako.exe are that copy's, and stay.
pub fn unregister_if_mine(data_dir: &std::path::Path) {
    use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
    let dir = data_dir.join("browser");
    let host_exe = std::fs::read(dir.join("chromium-host.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["path"].as_str().map(str::to_lowercase));
    let this_exe = std::env::current_exe().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if host_exe.is_some_and(|p| p != this_exe) {
        return;
    }
    let mine = [dir.join("chromium-host.json"), dir.join("firefox-host.json")].map(|p| p.to_string_lossy().to_lowercase());
    for k in all_keys() {
        let key = format!(r"{k}\{HOST_NAME}");
        let points_here = crate::system::reg_get_value(HKEY_CURRENT_USER, &key, None, 0)
            .is_some_and(|v| mine.contains(&v.trim().trim_matches('"').to_lowercase()));
        if points_here {
            reg_delete(&key);
        }
    }
    for f in ["chromium-host.json", "firefox-host.json"] {
        let _ = std::fs::remove_file(dir.join(f));
    }
    let _ = std::fs::remove_dir(&dir);
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn reg_set_default(key: &str, value: &str) -> Result<(), String> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW};
    let k = wide(key);
    let data = wide(value);
    // SAFETY: NUL-terminated strings; a null value name = the key's default value. The key is
    // created when missing.
    let r = unsafe {
        RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), std::ptr::null(), REG_SZ, data.as_ptr().cast(), (data.len() * 2) as u32)
    };
    if r == 0 { Ok(()) } else { Err(format!("Windows refused to connect Pako to the browsers (error {r}).")) }
}

fn reg_delete(key: &str) {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteTreeW};
    let k = wide(key);
    // SAFETY: NUL-terminated string. "Not there" is fine.
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, k.as_ptr()) };
}

// ---------- the extension's files, for loading into a browser ----------

const FILES: &[(&str, &[u8])] = &[
    ("background.js", include_bytes!("../../extension/background.js")),
    // The licence and its additional terms travel with every copy.
    ("LICENSE", include_bytes!("../../LICENSE")),
    ("NOTICE.md", include_bytes!("../../NOTICE.md")),
    ("popup.html", include_bytes!("../../extension/popup.html")),
    ("popup.css", include_bytes!("../../extension/popup.css")),
    ("popup.js", include_bytes!("../../extension/popup.js")),
    ("pill.js", include_bytes!("../../extension/pill.js")),
    ("player.js", include_bytes!("../../extension/player.js")),
    ("caught.js", include_bytes!("../../extension/caught.js")),
    ("recorder.js", include_bytes!("../../extension/recorder.js")),
    ("recorder-bridge.js", include_bytes!("../../extension/recorder-bridge.js")),
    ("pako-sent.html", include_bytes!("../../extension/pako-sent.html")),
    ("pako-sent.js", include_bytes!("../../extension/pako-sent.js")),
    ("download-links.js", include_bytes!("../../extension/download-links.js")),
    ("icons/icon16.png", include_bytes!("../../extension/icons/icon16.png")),
    ("icons/icon32.png", include_bytes!("../../extension/icons/icon32.png")),
    ("icons/icon48.png", include_bytes!("../../extension/icons/icon48.png")),
    ("icons/icon128.png", include_bytes!("../../extension/icons/icon128.png")),
];
const CHROMIUM_MANIFEST: &[u8] = include_bytes!("../../extension/manifest.chromium.json");
/// The Firefox extension signed by Mozilla, built in by build.rs from
/// dist/extension/signed/pako-firefox-<version>.xpi (scripts/sign-firefox.ps1). Empty in a build
/// made without it: Pako then says Firefox isn't available.
const FIREFOX_XPI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pako-firefox.xpi"));
const XPI_NAME: &str = "pako-firefox.xpi";

/// After a Pako update: the extension folder a browser already loads from is brought up to date
/// (the browser picks it up at its next start, or with the reload button). Nothing is written
/// when the folder was never made or already matches this Pako.
pub fn refresh_if_outdated() {
    let manifest = crate::paths::data_dir().join("extension").join("chromium").join("manifest.json");
    match std::fs::read(&manifest) {
        Ok(on_disk) if on_disk != CHROMIUM_MANIFEST || !files_match() => {
            let _ = write_files();
        }
        _ => {}
    }
}

/// The folder's scripts and pages (and the Firefox file) are the ones built into this Pako.
fn files_match() -> bool {
    let root = crate::paths::data_dir().join("extension");
    let chromium = root.join("chromium");
    FILES.iter().all(|(name, bytes)| std::fs::read(chromium.join(name)).is_ok_and(|b| b == *bytes))
        && (FIREFOX_XPI.is_empty() || std::fs::read(root.join(XPI_NAME)).is_ok_and(|b| b == FIREFOX_XPI))
}

/// The version of the extension in Pako's "Load unpacked" folder, as written on disk (not the
/// one built into this exe: the browser reloads from the folder, so only its version counts).
fn folder_version() -> Option<String> {
    let path = crate::paths::data_dir().join("extension").join("chromium").join("manifest.json");
    manifest_version(&std::fs::read(path).ok()?)
}

fn manifest_version(bytes: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    v.get("version")?.as_str().map(String::from).filter(|s| !s.is_empty() && s.len() <= 32)
}

/// Writes the extension next to Pako's data (a folder for Chromium browsers' "Load unpacked",
/// and the signed file Firefox adds, when this Pako has it) and returns that folder.
pub fn write_files() -> Result<PathBuf, String> {
    let root = crate::paths::data_dir().join("extension");
    let chromium = root.join("chromium");
    let _ = std::fs::remove_dir_all(&chromium);
    std::fs::create_dir_all(chromium.join("icons")).map_err(|e| e.to_string())?;
    for (name, bytes) in FILES {
        std::fs::write(chromium.join(name), bytes).map_err(|e| e.to_string())?;
    }
    std::fs::write(chromium.join("manifest.json"), CHROMIUM_MANIFEST).map_err(|e| e.to_string())?;

    // Firefox: only the copy signed by Mozilla (Firefox refuses any other). A file left by an
    // older Pako (or the unsigned zip Pako 1.0 wrote for signing by hand) goes.
    let xpi = root.join(XPI_NAME);
    if FIREFOX_XPI.is_empty() {
        let _ = std::fs::remove_file(&xpi);
    } else {
        std::fs::write(&xpi, FIREFOX_XPI).map_err(|e| e.to_string())?;
    }
    let _ = std::fs::remove_file(root.join("pako-firefox.zip"));
    // How to add it, next to the files it talks about.
    std::fs::write(root.join("README.md"), include_bytes!("../../extension/README.md")).map_err(|e| e.to_string())?;
    Ok(root)
}

/// The Firefox version signed by Mozilla, in the folder `write_files` returns (`root`), when this
/// Pako has one. Firefox only adds signed extensions.
pub fn firefox_xpi(root: &std::path::Path) -> Option<PathBuf> {
    Some(root.join(XPI_NAME)).filter(|p| !FIREFOX_XPI.is_empty() && p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vivaldi_is_said_to_ask_first_and_the_answer_names_it() {
        assert!(asks_first(Some("vivaldi")));
        for b in [None, Some("chrome"), Some("edge"), Some("brave"), Some("opera"), Some("firefox")] {
            assert!(!asks_first(b), "{b:?}");
        }
        let mut r = Reply::ok();
        assert!(!serde_json::to_string(&r).unwrap().contains("asksFirst"));
        r.asks_first = true;
        assert!(serde_json::to_string(&r).unwrap().contains("\"asksFirst\":true"));
    }

    #[test]
    fn framing_round_trip() {
        let mut buf = Vec::new();
        write_message(&mut buf, br#"{"kind":"ping"}"#).unwrap();
        assert_eq!(&buf[..4], &15u32.to_le_bytes());
        let mut r = &buf[..];
        assert_eq!(read_message(&mut r).unwrap().unwrap(), br#"{"kind":"ping"}"#.to_vec());
        assert!(read_message(&mut r).unwrap().is_none()); // closed line = done
    }

    #[test]
    fn refuses_oversized_messages() {
        let mut buf = ((MAX_MESSAGE as u32) + 1).to_le_bytes().to_vec();
        buf.extend_from_slice(b"{}");
        assert!(read_message(&mut &buf[..]).is_err());
    }

    #[test]
    fn validates_requests() {
        let r = validate(Request {
            kind: "download".into(),
            urls: vec!["javascript:alert(1)".into(), "https://example.org/a.zip".into(), "file:///C:/x".into()],
            referrer: "chrome://settings".into(),
            cookies: "a=1\r\nX-Evil: 1".into(),
            filename: r"..\..\Windows\evil.exe".into(),
            takeover: false,
            quality: Some(720),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(r.quality, None);
        assert_eq!(r.urls, vec!["https://example.org/a.zip".to_string()]);
        assert!(r.referrer.is_empty());
        assert!(r.cookies.is_empty());
        assert!(!r.filename.contains(['\\', '/']));
        assert!(validate(Request { kind: "download".into(), urls: vec!["ftp://x".into()], ..Default::default() }).is_err());
        assert!(validate(Request { kind: "rm -rf".into(), ..Default::default() }).is_err());
        assert_eq!(validate(Request { kind: "ping".into(), ..Default::default() }).unwrap().kind, "ping");
    }

    /// A request without a file name keeps none (the link's own name is used then); a name is
    /// only ever its last part.
    #[test]
    fn no_file_name_stays_no_file_name() {
        assert_eq!(clean_filename(""), "");
        assert_eq!(clean_filename("   "), "");
        assert_eq!(clean_filename(r"C:\Users\me\"), "");
        assert_eq!(clean_filename(r"..\..\Windows\evil.exe"), "evil.exe");
        assert_eq!(clean_filename("a/b/report.pdf"), "report.pdf");
        let r = validate(Request { kind: "download".into(), urls: vec!["https://example.org/a.zip".into()], ..Default::default() }).unwrap();
        assert_eq!(r.filename, "");
    }

    #[test]
    fn pipe_message_round_trip() {
        let r = Request {
            kind: "download".into(),
            urls: vec!["https://example.org/a.zip".into()],
            cookies: "s=1".into(),
            takeover: true,
            ..Default::default()
        };
        // Framed on the pipe like on stdin.
        let mut buf = Vec::new();
        write_message(&mut buf, &pipe_message(&r)).unwrap();
        let back = read_message(&mut &buf[..]).unwrap().unwrap();
        assert_eq!(parse_pipe_message(&back), Ok(validate(r).unwrap()));
        // The async side reads the same frames.
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let mut slice = &buf[..];
        assert_eq!(rt.block_on(read_message_async(&mut slice)).unwrap().unwrap(), back);
    }

    #[test]
    fn refuses_bad_pipe_messages() {
        assert!(parse_pipe_message(b"not json").is_err());
        // Another format version.
        assert!(parse_pipe_message(br#"{"pako":2,"request":{"kind":"ping"}}"#).is_err());
        // Checked on Pako's side too.
        assert!(parse_pipe_message(br#"{"pako":1,"request":{"kind":"download","urls":["file:///C:/Windows/win.ini"]}}"#).is_err());
        // An oversized frame is refused before reading it.
        let mut big = ((MAX_MESSAGE as u32) + 1).to_le_bytes().to_vec();
        big.extend_from_slice(b"{}");
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        assert!(rt.block_on(read_message_async(&mut &big[..])).is_err());
        assert!(write_message(&mut Vec::new(), &vec![b' '; MAX_MESSAGE + 1]).is_err());
    }

    #[test]
    fn cleans_links_referrers_and_cookies() {
        let r = validate(Request {
            kind: "download".into(),
            urls: vec!["  HTTPS://Example.org/a b.zip".into()],
            referrer: format!("https://example.org/{}", "x".repeat(MAX_URL_LEN)),
            cookies: "a=1\u{0}b".into(),
            takeover: true,
            ..Default::default()
        })
        .unwrap();
        // The link as the parser read it, not the raw text.
        assert_eq!(r.urls, vec!["https://example.org/a%20b.zip".to_string()]);
        assert!(r.referrer.is_empty());
        assert!(r.cookies.is_empty());
        assert!(r.takeover);
        let link = |cookies: &str, referrer: &str| Request {
            kind: "download".into(),
            urls: vec!["https://e.org/".into()],
            cookies: cookies.into(),
            referrer: referrer.into(),
            ..Default::default()
        };
        assert!(validate(link("a=1\tb=2", "")).unwrap().cookies.is_empty());
        let ok = validate(link("a=1; b=2", "https://e.org/p")).unwrap();
        assert_eq!(ok.cookies, "a=1; b=2");
        assert_eq!(ok.referrer, "https://e.org/p");
        // Several links are never held by the browser.
        let many = validate(Request {
            kind: "links".into(),
            urls: vec!["https://e.org/1".into(), "https://e.org/2".into()],
            takeover: true,
            ..Default::default()
        })
        .unwrap();
        assert!(!many.takeover);
    }

    #[test]
    fn validates_video_requests() {
        let video = |urls: &[&str], quality: Option<u32>| Request {
            kind: "video".into(),
            urls: urls.iter().map(|u| u.to_string()).collect(),
            quality,
            cookies: "s=1".into(),
            referrer: "https://www.youtube.com/".into(),
            filename: "x.exe".into(),
            takeover: true,
            ..Default::default()
        };
        let ok = validate(video(&["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], Some(720))).unwrap();
        assert_eq!(ok.kind, "video");
        assert_eq!(ok.urls, vec!["https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()]);
        assert_eq!(ok.quality, Some(720));
        // Nothing else travels with a video.
        assert!(ok.cookies.is_empty() && ok.referrer.is_empty() && ok.filename.is_empty() && !ok.takeover);
        for q in VIDEO_QUALITIES {
            assert!(validate(video(&["https://www.youtube.com/shorts/abcdefghijk"], Some(*q))).is_ok());
        }
        assert!(validate(video(&["https://vimeo.com/123456"], Some(0))).is_ok());
        // Only the offered qualities.
        for q in [None, Some(1), Some(360), Some(2160), Some(u32::MAX)] {
            assert!(validate(video(&["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], q)).is_err(), "{q:?}");
        }
        // One web link to a video site Pako knows.
        assert!(validate(video(&[], Some(720))).is_err());
        assert!(validate(video(&["https://www.youtube.com/watch?v=a", "https://www.youtube.com/watch?v=b"], Some(720))).is_err());
        assert!(validate(video(&["https://example.org/movie.mp4"], Some(720))).is_err());
        assert!(validate(video(&["https://www.youtube.com/"], Some(720))).is_err());
        assert!(validate(video(&["javascript:alert(1)"], Some(720))).is_err());
        assert!(validate(video(&["file:///C:/Windows/win.ini"], Some(720))).is_err());
        let long = format!("https://www.youtube.com/watch?v={}", "a".repeat(MAX_URL_LEN));
        assert!(validate(video(&[long.as_str()], Some(720))).is_err());
        // Checked again on Pako's side of the pipe.
        assert!(parse_pipe_message(br#"{"pako":1,"request":{"kind":"video","urls":["https://www.youtube.com/watch?v=x"],"quality":144}}"#).is_err());
        let back = parse_pipe_message(br#"{"pako":1,"request":{"kind":"video","urls":["https://www.youtube.com/watch?v=x"],"quality":0}}"#).unwrap();
        assert_eq!(back.quality, Some(0));
    }

    /// The quality menu opened: the same checks as "video", with no quality and nothing else.
    #[test]
    fn validates_video_prepare_requests() {
        let prepare = |urls: &[&str], private: bool| Request {
            kind: "videoPrepare".into(),
            urls: urls.iter().map(|u| u.to_string()).collect(),
            quality: Some(720),
            cookies: "s=1".into(),
            referrer: "https://www.youtube.com/".into(),
            filename: "x.exe".into(),
            takeover: true,
            site_cookies: vec![site_cookie(".youtube.com", "SID", "x", 0), site_cookie(".other.example", "a", "b", 0)],
            private,
            ..Default::default()
        };
        let ok = validate(prepare(&["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], false)).unwrap();
        assert_eq!(ok.kind, "videoPrepare");
        assert_eq!(ok.urls, vec!["https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()]);
        assert_eq!(ok.quality, None, "a read ahead has no quality");
        assert!(ok.cookies.is_empty() && ok.referrer.is_empty() && ok.filename.is_empty() && !ok.takeover);
        // Only the video site's own cookies; none at all from a private window.
        assert_eq!(ok.site_cookies.len(), 1);
        assert_eq!(ok.site_cookies[0].domain, ".youtube.com");
        let private = validate(prepare(&["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], true)).unwrap();
        assert!(private.private && private.site_cookies.is_empty());
        // One web link to a video site Pako knows.
        assert!(validate(prepare(&[], false)).is_err());
        assert!(validate(prepare(&["https://www.youtube.com/watch?v=a", "https://www.youtube.com/watch?v=b"], false)).is_err());
        assert!(validate(prepare(&["https://example.org/movie.mp4"], false)).is_err());
        assert!(validate(prepare(&["javascript:alert(1)"], false)).is_err());
        // Never from Pako Light.
        let mut buf = Vec::new();
        let light = Request { light: true, ..prepare(&["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], false) };
        write_message(&mut buf, &pipe_message(&light)).unwrap();
        assert_eq!(parse_pipe_message(&read_message(&mut &buf[..]).unwrap().unwrap()), Err(LIGHT_REFUSED.to_string()));
        assert_eq!(light_rules(ok), Err(LIGHT_REFUSED.to_string()));
        // The extension asks only a Pako that says it reads ahead.
        let said = Reply { prepares: true, ..Reply::ok() };
        assert_eq!(serde_json::to_string(&said).unwrap(), r#"{"ok":true,"prepares":true}"#);
    }

    /// The quality menu closed without a pick: the link checked like "video", nothing else kept
    /// (no cookies); never from Pako Light; and, like the read ahead, it never starts Pako.
    #[test]
    fn validates_video_prepare_cancels() {
        let cancel = |urls: &[&str], private: bool| Request {
            kind: "videoPrepareCancel".into(),
            urls: urls.iter().map(|u| u.to_string()).collect(),
            quality: Some(720),
            site_cookies: vec![site_cookie(".youtube.com", "SID", "x", 0)],
            private,
            ..Default::default()
        };
        let ok = validate(cancel(&["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], false)).unwrap();
        assert_eq!(ok.kind, "videoPrepareCancel");
        assert_eq!(ok.urls, vec!["https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()]);
        assert!(ok.site_cookies.is_empty() && ok.quality.is_none() && !ok.private);
        assert!(validate(cancel(&["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], true)).unwrap().private);
        assert!(validate(cancel(&[], false)).is_err());
        assert!(validate(cancel(&["https://example.org/movie.mp4"], false)).is_err());
        assert!(validate(cancel(&["file:///C:/x"], false)).is_err());
        assert_eq!(light_rules(ok), Err(LIGHT_REFUSED.to_string()));
        let mut buf = Vec::new();
        let light = Request { light: true, ..cancel(&["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], false) };
        write_message(&mut buf, &pipe_message(&light)).unwrap();
        assert_eq!(parse_pipe_message(&read_message(&mut &buf[..]).unwrap().unwrap()), Err(LIGHT_REFUSED.to_string()));
        // A menu opening or closing never starts Pako and notes nothing; a pick does as before.
        assert!(read_ahead("videoPrepare") && read_ahead("videoPrepareCancel"));
        assert!(!read_ahead("video") && !read_ahead("download"));
    }

    #[test]
    fn replies_read_back() {
        let r: Reply = serde_json::from_slice(&serde_json::to_vec(&Reply::not_ready()).unwrap()).unwrap();
        assert!(r.retry && !r.ok);
        assert_eq!(serde_json::to_string(&Reply::ok()).unwrap(), r#"{"ok":true}"#);
        assert_eq!(serde_json::to_string(&Reply::declined()).unwrap(), r#"{"ok":false}"#);
    }

    #[test]
    fn forgets_headers_of_finished_downloads() {
        let cookie = |c: &str| BrowserHeaders { referrer: String::new(), cookies: c.into(), ..Default::default() };
        let mut s = HeaderStore::default();
        s.insert("keep-me", cookie("a=1"));
        s.insert("drop-me", cookie("b=2"));
        s.insert("empty", BrowserHeaders::default());
        assert!(!s.map.contains_key("empty"));
        // Just arrived: kept even though its download isn't listed yet.
        s.retain(Duration::from_secs(60), |id| id == "keep-me");
        assert!(s.map.contains_key("drop-me"));
        s.retain(Duration::ZERO, |id| id != "drop-me");
        assert!(s.map.contains_key("keep-me"));
        assert!(!s.map.contains_key("drop-me"));
        s.remove("keep-me");
        assert!(s.map.is_empty());
        // The shared store works the same way.
        remember_headers("shared", cookie("c=3"));
        assert_eq!(headers_for("shared"), Some(cookie("c=3")));
        forget_headers("shared");
        assert!(headers_for("shared").is_none());
    }

    #[test]
    fn keeps_headers_encrypted_for_a_restart() {
        let dir = std::env::temp_dir().join(format!("pako-headers-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("browser-headers.dat");
        let h = BrowserHeaders { referrer: "https://e.org/p".into(), cookies: "secret=1".into(), ..Default::default() };
        let mut s = HeaderStore::default();
        s.open(file.clone());
        assert!(!file.exists()); // nothing kept, no file
        s.insert("d1", h.clone());
        let raw = std::fs::read(&file).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("secret"));
        // The next start reads it back.
        let mut next = HeaderStore::default();
        next.open(file.clone());
        assert_eq!(next.map.get("d1").map(|k| k.headers.clone()), Some(h));
        // Finished or removed: gone from the file too.
        next.remove("d1");
        assert!(!file.exists());
        assert!(read_kept_headers(&file).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pipe_reopens_a_little_later_each_time() {
        assert_eq!(reopen_wait(0), Duration::from_millis(500));
        assert_eq!(reopen_wait(1), Duration::from_secs(1));
        assert!(reopen_wait(3) > reopen_wait(2));
        assert_eq!(reopen_wait(50), Duration::from_secs(30));
        assert_eq!(reopen_wait(u32::MAX), Duration::from_secs(30));
    }

    #[test]
    fn host_skips_messages_too_big_and_carries_on() {
        let mut buf = ((MAX_MESSAGE as u32) + 1).to_le_bytes().to_vec();
        buf.extend(std::iter::repeat_n(b' ', MAX_MESSAGE + 1));
        write_message(&mut buf, br#"{"kind":"ping"}"#).unwrap();
        let mut r = &buf[..];
        assert!(matches!(read_host_message(&mut r).unwrap(), HostMessage::TooBig));
        assert!(matches!(read_host_message(&mut r).unwrap(), HostMessage::Request(m) if m == br#"{"kind":"ping"}"#));
        assert!(matches!(read_host_message(&mut r).unwrap(), HostMessage::Closed));
        // Cut short: the line closed.
        let mut short = ((MAX_MESSAGE as u32) + 1).to_le_bytes().to_vec();
        short.extend_from_slice(b"{}");
        assert!(matches!(read_host_message(&mut &short[..]).unwrap(), HostMessage::Closed));
    }

    #[test]
    fn takeover_matches_the_browsers_file() {
        let req = |total: u64, mime: &str| {
            validate(Request {
                kind: "download".into(),
                urls: vec!["https://e.org/a.zip".into()],
                takeover: true,
                total_bytes: total,
                mime: mime.into(),
                ..Default::default()
            })
            .unwrap()
        };
        let zip = req(5_000_000, "application/zip");
        assert!(same_as_browser(&zip, Some(5_000_000), Some(false)).is_ok());
        assert!(same_as_browser(&zip, Some(5_000_000), None).is_ok());
        // Another size: another file.
        assert!(same_as_browser(&zip, Some(4_999_999), Some(false)).is_err());
        // A web page instead of the file.
        assert!(same_as_browser(&zip, None, Some(true)).is_err());
        assert!(same_as_browser(&zip, Some(5_000_000), Some(true)).is_err());
        // Sizes not known on one side: only the kind of file is compared.
        assert!(same_as_browser(&req(0, "application/zip"), Some(123), Some(false)).is_ok());
        assert!(same_as_browser(&zip, None, Some(false)).is_ok());
        // The browser's file was a web page too.
        let page = req(0, "text/html; charset=utf-8");
        assert_eq!(page.mime, "text/html");
        assert!(same_as_browser(&page, Some(10), Some(true)).is_ok());
        assert!(same_as_browser(&req(0, ""), None, Some(true)).is_err());
        // Odd media types are dropped; nothing of this travels without a takeover.
        assert_eq!(req(1, "text/html\r\nX: y").mime, "");
        let plain = validate(Request {
            kind: "download".into(),
            urls: vec!["https://e.org/a.zip".into()],
            total_bytes: 9,
            mime: "application/zip".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!((plain.total_bytes, plain.mime.as_str()), (0, ""));
    }

    #[test]
    fn the_pipe_belongs_to_this_user() {
        let name = pipe_name().unwrap();
        let prefix = r"\\.\pipe\pako-browser-";
        assert!(name.starts_with(prefix));
        assert!(name.len() > prefix.len() + 10);
    }

    #[test]
    fn answers_the_store_copy_and_its_own() {
        let o = chromium_origins();
        assert!(o.contains(&"chrome-extension://cojnnpgiilnbkblckkjnheigiodkdlpl/".to_string()));
        assert!(o.contains(&"chrome-extension://fnaplmfbaccplngclnkchlngpkbcbhod/".to_string()));
        assert!(o.iter().all(|x| x.len() == "chrome-extension://".len() + 32 + 1));
    }

    #[test]
    fn registers_where_browsers_look() {
        // Chrome's key is also the one Chromium-based browsers without their own read (Opera, Opera GX).
        assert_eq!(CHROMIUM_KEYS[0], r"Software\Google\Chrome\NativeMessagingHosts");
        assert!(CHROMIUM_KEYS.contains(&r"Software\Microsoft\Edge\NativeMessagingHosts"));
        assert!(CHROMIUM_KEYS.iter().all(|k| k.starts_with(r"Software\") && k.ends_with(r"\NativeMessagingHosts")));
        // Firefox, Zen, Waterfox, Floorp and LibreWolf read Mozilla's; old LibreWolf its own.
        assert_eq!(FIREFOX_KEYS[0], r"Software\Mozilla\NativeMessagingHosts");
        assert!(FIREFOX_KEYS.contains(&r"Software\LibreWolf\NativeMessagingHosts"));
        assert!(FIREFOX_KEYS.iter().all(|k| k.starts_with(r"Software\") && k.ends_with(r"\NativeMessagingHosts")));
        // Turning off and uninstalling go through every key written.
        let all: Vec<_> = all_keys().copied().collect();
        assert_eq!(all.len(), CHROMIUM_KEYS.len() + FIREFOX_KEYS.len());
        assert!(FIREFOX_KEYS.iter().chain(CHROMIUM_KEYS).all(|k| all.contains(k)));
    }

    #[test]
    fn spots_browser_launches() {
        assert!(is_host_launch(&["chrome-extension://fnaplmfbaccplngclnkchlngpkbcbhod/".into(), "--parent-window=0".into()]));
        assert!(is_host_launch(&[r"C:\x\firefox-host.json".into(), FIREFOX_ID.into()]));
        assert!(!is_host_launch(&["--tray".into()]));
        assert!(started_for_browser(&[FROM_BROWSER.into()]));
        assert!(!started_for_browser(&["--tray".into()]));
    }

    fn site_cookie(domain: &str, name: &str, value: &str, expires: u64) -> SiteCookie {
        SiteCookie { domain: domain.into(), path: "/".into(), secure: true, expires, name: name.into(), value: value.into() }
    }

    #[test]
    fn site_cookies_only_of_the_video_site() {
        let now = 1_800_000_000;
        let kept = clean_site_cookies(
            "https://www.youtube.com/watch?v=abc",
            vec![
                site_cookie(".youtube.com", "SID", "1", 0),
                site_cookie("www.youtube.com", "PREF", "2", now + 10),
                site_cookie(".google.com", "NID", "3", 0),
                site_cookie(".youtube.com", "OLD", "4", now - 10),
                site_cookie(".youtube.com", "BAD", "a\r\nX-Evil: 1", 0),
                site_cookie(".notyoutube.com", "X", "5", 0),
                site_cookie(".youtube.com", "", "6", 0),
            ],
            now,
        );
        let names: Vec<&str> = kept.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["SID", "PREF"]);
        // Short links count as their site.
        assert_eq!(clean_site_cookies("https://youtu.be/abc", vec![site_cookie(".youtube.com", "SID", "1", 0)], now).len(), 1);
        assert_eq!(clean_site_cookies("https://fb.watch/x", vec![site_cookie(".facebook.com", "c_user", "1", 0)], now).len(), 1);
    }

    #[test]
    fn video_requests_keep_site_cookies() {
        let r = Request {
            kind: "video".into(),
            urls: vec!["https://www.instagram.com/reel/abc123/".into()],
            quality: Some(720),
            site_cookies: vec![site_cookie(".instagram.com", "sessionid", "s", 0)],
            ..Default::default()
        };
        assert_eq!(validate(r).unwrap().site_cookies.len(), 1);
        // A plain file download never carries them.
        let r = Request {
            kind: "download".into(),
            urls: vec!["https://e.org/file.zip".into()],
            site_cookies: vec![site_cookie(".e.org", "a", "b", 0)],
            ..Default::default()
        };
        assert!(validate(r).unwrap().site_cookies.is_empty());
    }

    #[test]
    fn spots_pako_light() {
        assert!(is_light_launch(&["chrome-extension://cojnnpgiilnbkblckkjnheigiodkdlpl/".into(), "--parent-window=0".into()]));
        assert!(!is_light_launch(&["chrome-extension://fnaplmfbaccplngclnkchlngpkbcbhod/".into()]));
        assert!(!is_light_launch(&[r"C:\x\firefox-host.json".into(), FIREFOX_ID.into()]));
        // An ID inside another argument doesn't count.
        assert!(!is_light_launch(&["--x=chrome-extension://cojnnpgiilnbkblckkjnheigiodkdlpl/".into()]));
    }

    #[test]
    fn pako_light_never_asks_for_videos() {
        let light = |kind: &str, urls: &[&str], quality: Option<u32>| {
            let r = Request { kind: kind.into(), urls: urls.iter().map(|u| u.to_string()).collect(), quality, light: true, ..Default::default() };
            let mut buf = Vec::new();
            write_message(&mut buf, &pipe_message(&r)).unwrap();
            parse_pipe_message(&read_message(&mut &buf[..]).unwrap().unwrap())
        };
        // The player's Download button, and one video link: refused.
        assert_eq!(light("video", &["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], Some(720)), Err(LIGHT_REFUSED.to_string()));
        assert_eq!(light("download", &["https://www.youtube.com/watch?v=dQw4w9WgXcQ"], None), Err(LIGHT_REFUSED.to_string()));
        assert_eq!(light("links", &["https://youtu.be/dQw4w9WgXcQ", "https://vimeo.com/123456"], None), Err(LIGHT_REFUSED.to_string()));
        // "All links": the video links are left out, the rest is marked as Light's.
        let links = light("links", &["https://www.youtube.com/watch?v=dQw4w9WgXcQ", "https://e.org/a.zip", "https://youtu.be/x1"], None).unwrap();
        assert_eq!(links.urls, vec!["https://e.org/a.zip".to_string()]);
        assert!(links.light);
        assert!(light("ping", &[], None).is_ok());
        // The same requests from the full extension are videos as before.
        let full = parse_pipe_message(br#"{"pako":1,"request":{"kind":"video","urls":["https://www.youtube.com/watch?v=x"],"quality":0}}"#).unwrap();
        assert!(!full.light);
        // The extension can't say it is Light (nor that it isn't): only the host sets it.
        let r: Request = serde_json::from_str(r#"{"kind":"links","urls":["https://e.org/"],"light":true}"#).unwrap();
        assert!(!r.light);
        let open = parse_pipe_message(br#"{"pako":1,"request":{"kind":"download","urls":["https://www.youtube.com/watch?v=x"],"light":false},"light":true}"#);
        assert_eq!(open, Err(LIGHT_REFUSED.to_string()));
    }

    #[test]
    fn referrer_as_a_browser_sends_it() {
        let sent = |referrer: &str, url: &str| {
            validate(Request { kind: "download".into(), urls: vec![url.into()], referrer: referrer.into(), ..Default::default() })
                .unwrap()
                .referrer
        };
        // Same site: the whole address, without its "#…" part or a login.
        assert_eq!(sent("https://u:p@app.example/inbox?id=7#token=abc", "https://app.example/f.zip"), "https://app.example/inbox?id=7");
        // Another site: only the site.
        assert_eq!(sent("https://mail.example/inbox?id=7&token=abc#x", "https://cdn.other/f.zip"), "https://mail.example/");
        assert_eq!(sent("https://app.example:8443/p?q=1", "https://app.example/f.zip"), "https://app.example:8443/");
        // From https to http: nothing.
        assert_eq!(sent("https://app.example/p", "http://app.example/f.zip"), "");
        assert_eq!(sent("http://app.example/p?q=1", "https://cdn.other/f.zip"), "http://app.example/");
        // Several links: only the site as soon as one is elsewhere.
        let links = validate(Request {
            kind: "links".into(),
            urls: vec!["https://e.org/1".into(), "https://f.org/2".into()],
            referrer: "https://e.org/page?x=1".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(links.referrer, "https://e.org/");
        // Checked again on Pako's side: the same answer.
        let r = validate(Request {
            kind: "download".into(),
            urls: vec!["https://cdn.other/f.zip".into()],
            referrer: "https://mail.example/inbox#t".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(validate(r.clone()).unwrap().referrer, r.referrer);
    }

    #[test]
    fn pako_light_skips_whole_sites_like_the_extension() {
        // The same list as background.js.
        let js = include_str!("../../extension/background.js");
        let start = js.find("const LIGHT_SKIPS_SITES = [").expect("LIGHT_SKIPS_SITES in background.js");
        let list = &js[start..start + js[start..].find("];").unwrap()];
        let mut theirs: Vec<&str> = list.split('"').skip(1).step_by(2).collect();
        let mut ours = LIGHT_SKIPS_SITES.to_vec();
        theirs.sort_unstable();
        ours.sort_unstable();
        assert_eq!(ours, theirs);
        // Any page of those sites, also when only the site is known (the referrer).
        let light = |urls: &[&str], referrer: &str| {
            let r = validate(Request {
                kind: "links".into(),
                urls: urls.iter().map(|u| u.to_string()).collect(),
                referrer: referrer.into(),
                ..Default::default()
            })
            .unwrap();
            light_rules(r)
        };
        assert_eq!(light(&["https://cdn.example/a.zip"], "https://www.youtube.com/watch?v=x"), Err(LIGHT_REFUSED.to_string()));
        assert_eq!(light(&["https://cdn.example/a.zip"], "https://m.youtube.com/"), Err(LIGHT_REFUSED.to_string()));
        assert_eq!(light(&["https://www.youtube.com/@someone"], ""), Err(LIGHT_REFUSED.to_string()));
        let kept = light(&["https://www.reddit.com/r/x/", "https://cdn.example/a.zip"], "https://news.example/").unwrap();
        assert_eq!(kept.urls, vec!["https://cdn.example/a.zip".to_string()]);
        // Not a lookalike.
        assert!(light(&["https://notyoutube.com/a.zip"], "https://youtube.com.example/").is_ok());
    }

    #[test]
    fn the_same_takeover_twice_is_taken_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
        let gate = TakeoverGate::new(Duration::from_secs(30));
        let added = AtomicUsize::new(0);
        let work = |reply: Reply| {
            let added = &added;
            async move {
                added.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(30)).await;
                reply
            }
        };
        let key = || ("https://e.org/big.zip".to_string(), 50_000_000u64);
        // Two at once: one add, the same answer.
        let (a, b) = rt.block_on(async { tokio::join!(gate.run(key(), false, work(Reply::ok())), gate.run(key(), false, work(Reply::ok()))) });
        assert_eq!((a, b), (Reply::ok(), Reply::ok()));
        assert_eq!(added.load(Ordering::SeqCst), 1);
        // Shortly after: still the same one.
        assert_eq!(rt.block_on(gate.run(key(), false, work(Reply::error("x")))), Reply::ok());
        assert_eq!(added.load(Ordering::SeqCst), 1);
        // The first one failed: the second one is declined (no second warning in the browser).
        let other = || ("https://e.org/other.zip".to_string(), 0u64);
        let (a, b) = rt.block_on(async { tokio::join!(gate.run(other(), false, work(Reply::error("no"))), gate.run(other(), false, work(Reply::ok()))) });
        assert_eq!((a, b), (Reply::error("no"), Reply::declined()));
        assert_eq!(added.load(Ordering::SeqCst), 2);
        // Another size is another file.
        rt.block_on(gate.run(("https://e.org/big.zip".to_string(), 1), false, work(Reply::ok())));
        assert_eq!(added.load(Ordering::SeqCst), 3);
        // Cancelled or removed in Pako (forgotten): clicking again takes it again.
        let held = || ("https://e.org/held-again.zip".to_string(), 9u64);
        HELD.open("tok-forget");
        assert_eq!(rt.block_on(gate.run(held(), true, work(Reply::held("tok-forget")))), Reply::held("tok-forget"));
        assert_eq!(added.load(Ordering::SeqCst), 4);
        gate.forget("tok-forget");
        HELD.finish("tok-forget", Ok(()));
        assert_eq!(rt.block_on(gate.run(held(), true, work(Reply::held("tok-new")))), Reply::held("tok-new"));
        assert_eq!(added.load(Ordering::SeqCst), 5);
        // Answered a while ago (not held): a repeat is a new click, taken again.
        if let Some(s) = gate.seen.lock().unwrap().get_mut(&key()) {
            s.done = Some(Instant::now() - SAME_TAKEOVER_AFTER - Duration::from_secs(1));
        }
        rt.block_on(gate.run(key(), false, work(Reply::ok())));
        assert_eq!(added.load(Ordering::SeqCst), 6);
        // After the window, the same file is taken again.
        let quick = TakeoverGate::new(Duration::ZERO);
        rt.block_on(quick.run(key(), false, work(Reply::ok())));
        rt.block_on(quick.run(key(), false, work(Reply::ok())));
        assert_eq!(added.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn the_licence_goes_with_the_extension() {
        for name in ["LICENSE", "NOTICE.md"] {
            assert!(FILES.iter().any(|(n, b)| *n == name && !b.is_empty()), "{name}");
        }
    }

    /// The Firefox file built in (when there is one) is a signed extension of the current version,
    /// for the ID Pako's native host lets in.
    #[test]
    fn the_built_in_firefox_extension_is_this_version() {
        if FIREFOX_XPI.is_empty() {
            return; // a build without it: Pako says Firefox isn't available
        }
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(FIREFOX_XPI)).expect("the .xpi is a zip");
        let mut text = String::new();
        zip.by_name("manifest.json").expect("manifest.json").read_to_string(&mut text).unwrap();
        let signed: serde_json::Value = serde_json::from_str(&text).unwrap();
        let source: serde_json::Value = serde_json::from_slice(include_bytes!("../../extension/manifest.firefox.json")).unwrap();
        assert_eq!(signed["version"], source["version"]);
        assert_eq!(signed["browser_specific_settings"]["gecko"]["id"], FIREFOX_ID);
        // Mozilla's signature.
        assert!(zip.by_name("META-INF/mozilla.rsa").is_ok() || zip.by_name("META-INF/cose.sig").is_ok());
    }

    #[test]
    fn caught_streams_and_recordings_are_checked_and_never_from_light() {
        // A caught stream: checked the same in the host and again in Pako (through the pipe).
        let raw = br#"{"kind":"stream","urls":["https://cdn.example.net/v/master.m3u8"],"referrer":"https://watch.example.org/v/1#t","filename":"My clip","siteCookies":[{"domain":"cdn.example.net","path":"/","name":"a","value":"1"},{"domain":"other.example","path":"/","name":"b","value":"2"}]}"#;
        let r = validate(serde_json::from_slice(raw).unwrap()).unwrap();
        assert_eq!(r.kind, "stream");
        assert_eq!(r.referrer, "https://watch.example.org/");
        assert_eq!(r.filename, "My clip");
        assert_eq!(r.site_cookies.len(), 1);
        assert_eq!(parse_pipe_message(&pipe_message(&r)), Ok(r.clone()));
        assert!(validate(Request { kind: "stream".into(), urls: vec!["ftp://x.example/a.m3u8".into()], ..Default::default() }).is_err());
        // Record this video: the step travels as it came, once checked.
        let chunk = br#"{"kind":"record","record":{"action":"chunk","session":"abcdef12-3456","track":0,"mime":"video/mp4","seq":0,"data":"AQID"}}"#;
        let r = validate(serde_json::from_slice(chunk).unwrap()).unwrap();
        assert_eq!(r.kind, "record");
        assert!(r.urls.is_empty() && r.site_cookies.is_empty());
        assert_eq!(parse_pipe_message(&pipe_message(&r)), Ok(r.clone()));
        let bad = br#"{"kind":"record","record":{"action":"chunk","session":"abcdef12-3456","track":9,"mime":"video/mp4","seq":0,"data":"AQID"}}"#;
        assert!(validate(serde_json::from_slice(bad).unwrap()).is_err());
        assert!(validate(Request { kind: "record".into(), ..Default::default() }).is_err());
        // Pako Light: neither, whatever it sends.
        let stream = validate(serde_json::from_slice(raw).unwrap()).unwrap();
        assert_eq!(light_rules(stream), Err(LIGHT_REFUSED.to_string()));
        assert_eq!(light_rules(r), Err(LIGHT_REFUSED.to_string()));
        let open = parse_pipe_message(br#"{"pako":1,"request":{"kind":"stream","urls":["https://cdn.example.net/v.m3u8"]},"light":true}"#);
        assert_eq!(open, Err(LIGHT_REFUSED.to_string()));
    }

    /// The folder's version is read from its manifest (what the browser would load).
    #[test]
    fn the_folder_version_comes_from_its_manifest() {
        assert_eq!(manifest_version(br#"{"name":"Pako","version":"1.2.5"}"#).as_deref(), Some("1.2.5"));
        assert_eq!(manifest_version(CHROMIUM_MANIFEST).is_some(), true);
        assert_eq!(manifest_version(b"not json"), None);
        assert_eq!(manifest_version(br#"{"version":""}"#), None);
        let r = serde_json::to_string(&Reply::ok()).unwrap();
        assert!(!r.contains("ext"), "no field unless the host adds it: {r}");
    }
    /// A takeover answered at once: the extension's "held" request gets the check's result;
    /// the first result counts (the window's Cancel before a failed check means: drop it).
    #[test]
    fn held_takeovers_answer_with_the_checks_result() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
        let held = HeldChecks::new();
        // Unknown (Pako restarted): an error, so the browser keeps or restarts its download.
        let r = rt.block_on(held.wait("nope", Duration::from_millis(10)));
        assert!(!r.ok && r.error.is_some() && r.held.is_none());
        // Still waiting (your answer in the window): "ask again".
        held.open("a");
        assert!(held.checking("a"));
        assert_eq!(rt.block_on(held.wait("a", Duration::from_millis(20))), Reply::still_held("a"));
        // The check's result, also for a request already waiting.
        let r = rt.block_on(async {
            let waiting = held.wait("a", Duration::from_secs(5));
            let finish = async {
                tokio::time::sleep(Duration::from_millis(30)).await;
                held.finish("a", Ok(()));
            };
            tokio::join!(waiting, finish).0
        });
        assert_eq!(r, Reply::ok());
        assert!(!held.checking("a"));
        // A later result changes nothing.
        assert_eq!(held.finish("a", Err("late".into())), Some(Ok(())));
        assert_eq!(rt.block_on(held.wait("a", Duration::from_millis(10))), Reply::ok());
        // A failed check: its words reach the browser.
        held.open("b");
        held.finish("b", Err("Pako couldn't get this download itself (x).".into()));
        assert_eq!(rt.block_on(held.wait("b", Duration::from_millis(10))), Reply::error("Pako couldn't get this download itself (x)."));
        // Cancelled in the window first: the failure that follows doesn't count.
        held.open("c");
        held.finish("c", Ok(()));
        assert_eq!(held.finish("c", Err("removed".into())), Some(Ok(())));
        assert_eq!(held.finish("zzz", Ok(())), None, "not a held takeover");
    }

    /// The same takeover twice while it is held: a second extension that can wait gets the same
    /// token; an older one waits for the check's result.
    #[test]
    fn the_same_held_takeover_twice() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
        let gate = TakeoverGate::new(Duration::from_secs(30));
        let key = || ("https://e.org/held.zip".to_string(), 7u64);
        HELD.open("tok-gate");
        let (a, b) = rt.block_on(async {
            tokio::join!(
                gate.run(key(), true, async {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    Reply::held("tok-gate")
                }),
                gate.run(key(), true, async { Reply::error("never run") })
            )
        });
        assert_eq!((a, b), (Reply::held("tok-gate"), Reply::held("tok-gate")));
        let older = rt.block_on(async {
            let older = gate.run(key(), false, async { Reply::error("never run") });
            let finish = async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                HELD.finish("tok-gate", Ok(()));
            };
            tokio::join!(older, finish).0
        });
        assert_eq!(older, Reply::ok());
    }

    /// Older extensions and older Pakos: the new fields only appear when used, and a request
    /// without `hold` stays an ordinary takeover.
    #[test]
    fn hold_follow_and_old_messages() {
        assert_eq!(serde_json::to_string(&Reply::ok()).unwrap(), r#"{"ok":true}"#);
        assert_eq!(serde_json::to_string(&Reply::held("t1")).unwrap(), r#"{"ok":true,"held":"t1"}"#);
        // An old extension's takeover: no hold.
        let old: Request = serde_json::from_str(r#"{"kind":"download","urls":["https://e.org/a.zip"],"takeover":true,"totalBytes":5}"#).unwrap();
        let old = validate(old).unwrap();
        assert!(old.takeover && !old.hold && !old.private);
        // hold only for a takeover.
        let r: Request = serde_json::from_str(r#"{"kind":"download","urls":["https://e.org/a.zip"],"hold":true}"#).unwrap();
        assert!(!validate(r).unwrap().hold);
        let r: Request = serde_json::from_str(r#"{"kind":"download","urls":["https://e.org/a.zip"],"takeover":true,"hold":true,"private":true}"#).unwrap();
        let r = validate(r).unwrap();
        assert!(r.hold && r.private);
        // "held": only a plain token.
        let h = |t: &str| validate(Request { kind: "held".into(), held: t.into(), urls: vec!["https://x.org/".into()], ..Default::default() });
        let ok = h("0b5e-7a_Z").unwrap();
        assert_eq!((ok.kind.as_str(), ok.held.as_str(), ok.urls.len()), ("held", "0b5e-7a_Z", 0));
        assert!(h("").is_err() && h("a/b").is_err() && h(&"x".repeat(65)).is_err());
        // "follow": one web link, a known state, a plain token, the browser's reason cleaned.
        let f = |state: &str, pako: &str, url: &str| {
            validate(Request {
                kind: "follow".into(),
                urls: vec![url.into()],
                follow: Some(FollowReport {
                    pako: pako.into(),
                    state: state.into(),
                    error: "NETWORK_FAILED\r\nX: y".into(),
                    path: "C:\\D\\a.zip".into(),
                    ..Default::default()
                }),
                ..Default::default()
            })
        };
        let ok = f("complete", "abc-1", "https://e.org/a.zip").unwrap();
        assert_eq!(ok.follow.as_ref().unwrap().error, "NETWORK_FAILEDXy");
        assert_eq!(ok.follow.as_ref().unwrap().path, "C:\\D\\a.zip");
        assert!(f("done", "", "https://e.org/a.zip").is_err());
        assert!(f("paused", "../x", "https://e.org/a.zip").is_err());
        assert!(f("paused", "", "file:///C:/a.zip").is_err());
        assert!(validate(Request { kind: "follow".into(), urls: vec!["https://e.org/a".into()], ..Default::default() }).is_err());
        // Pako Light may report its own, never one of the sites it leaves alone.
        let light = |u: &str| light_rules(f("paused", "", u).unwrap());
        assert!(light("https://e.org/a.zip").unwrap().light);
        assert!(light("https://www.youtube.com/a").is_err());
    }

    /// Pako just starting: the host looks for its pipe often at first, then less.
    #[test]
    fn the_host_looks_for_a_starting_pako_often_at_first() {
        assert_eq!(poll_wait(Duration::ZERO), Duration::from_millis(50));
        assert_eq!(poll_wait(Duration::from_millis(999)), Duration::from_millis(50));
        assert_eq!(poll_wait(Duration::from_secs(1)), Duration::from_millis(250));
    }

    /// A private window's cookies are never written to the file kept for a restart.
    #[test]
    fn private_headers_stay_in_memory() {
        let dir = std::env::temp_dir().join(format!("pako-headers-private-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("browser-headers.dat");
        let mut s = HeaderStore::default();
        s.open(file.clone());
        s.insert("pub", BrowserHeaders { cookies: "normal=1".into(), ..Default::default() });
        s.insert("priv", BrowserHeaders { cookies: "incognito=1".into(), private: true, ..Default::default() });
        assert!(s.map.contains_key("priv"), "used while it runs");
        let kept = read_kept_headers(&file);
        assert!(kept.contains_key("pub"));
        assert!(!kept.contains_key("priv"));
        // Sent into Private mode afterwards (Download privately): out of the file, still usable.
        s.make_private("pub");
        assert!(!read_kept_headers(&file).contains_key("pub"));
        assert_eq!(s.map.get("pub").unwrap().headers.cookies, "normal=1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A check still waiting (your answer in the window) is never dropped, however long it
    /// takes and however many takeovers come meanwhile; finished ones go after a while.
    #[test]
    fn a_waiting_held_check_is_never_dropped() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
        let held = HeldChecks::new();
        held.open("slow");
        held.open("done");
        held.finish("done", Ok(()));
        held.age("slow", Duration::from_secs(3600));
        held.age("done", Duration::from_secs(3600));
        assert!(!held.listening("slow"), "nobody asked for an hour");
        held.open("next");
        assert!(held.checking("slow"), "still waiting for your answer");
        assert_eq!(rt.block_on(held.wait("slow", Duration::from_millis(5))), Reply::still_held("slow"));
        assert!(held.listening("slow"), "asked just now");
        assert_eq!(held.finish("done", Err("x".into())), None, "a finished one goes after a while");
        assert_eq!(held.finish("slow", Ok(())), Some(Ok(())));
    }

    /// "paused" only with "hold"; a private takeover's "held" requests stay private.
    #[test]
    fn paused_and_private_held_requests() {
        let r: Request = serde_json::from_str(r#"{"kind":"download","urls":["https://e.org/a.zip"],"takeover":true,"paused":true}"#).unwrap();
        assert!(!validate(r).unwrap().paused, "not without hold");
        let r: Request = serde_json::from_str(r#"{"kind":"download","urls":["https://e.org/a.zip"],"takeover":true,"hold":true,"paused":true}"#).unwrap();
        assert!(validate(r).unwrap().paused);
        let h: Request = serde_json::from_str(r#"{"kind":"held","held":"abc","private":true}"#).unwrap();
        assert!(validate(h).unwrap().private);
    }

    /// Every kind of request from a private window stays private once checked (not only a
    /// takeover), a video from one never keeps cookies (no cookie file for yt-dlp), and Record
    /// this video is refused there. Messages without the field are as before.
    #[test]
    fn every_kind_of_request_keeps_private() {
        let cookie = r#"[{"domain":".youtube.com","path":"/","name":"SID","value":"x"}]"#;
        let yt = "https://www.youtube.com/watch?v=abcdefghijk";
        let check = |json: String| validate(serde_json::from_str::<Request>(&json).unwrap());
        for private in [true, false] {
            let p = if private { r#","private":true"# } else { "" };
            let download = check(format!(r#"{{"kind":"download","urls":["https://e.org/a.zip"]{p}}}"#)).unwrap();
            let links = check(format!(r#"{{"kind":"links","urls":["https://e.org/a.zip","https://e.org/b.zip"]{p}}}"#)).unwrap();
            let video = check(format!(r#"{{"kind":"video","urls":["{yt}"],"quality":720,"siteCookies":{cookie}{p}}}"#)).unwrap();
            let page = check(format!(r#"{{"kind":"download","urls":["{yt}"],"siteCookies":{cookie}{p}}}"#)).unwrap();
            let stream = check(format!(
                r#"{{"kind":"stream","urls":["https://cdn.e.org/a.m3u8"],"siteCookies":[{{"domain":"cdn.e.org","path":"/","name":"k","value":"v"}}]{p}}}"#
            ))
            .unwrap();
            for r in [&download, &links, &video, &page, &stream] {
                assert_eq!(r.private, private, "{} keeps private = {private}", r.kind);
            }
            // A private window's video or stream carries no cookies; a normal one keeps them.
            assert_eq!(video.site_cookies.is_empty(), private);
            assert_eq!(page.site_cookies.is_empty(), private);
            assert_eq!(stream.site_cookies.is_empty(), private);
        }
        let rec = r#"{"kind":"record","record":{"action":"cancel","session":"0b4e1a2c-1111-4222-8333-944455556666"},"private":true}"#;
        assert_eq!(validate(serde_json::from_str(rec).unwrap()).unwrap_err(), NO_PRIVATE_RECORDING);
    }
}
