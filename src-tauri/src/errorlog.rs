// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! A small private list of the problems Pako ran into (`data/problems.log`), for Help › Report a
//! problem. It never leaves the PC by itself: the user reads it in the report and sends it with
//! their own email app, or not at all.
//!
//! Each line is one problem (JSON): when, where in Pako (download, video, link check, …), the
//! message the user was shown, and Pako's version. What could say who you are or what you
//! download is trimmed before it is written: links keep only their site (`https://example.com`),
//! cookies, tokens and long IDs are removed, email addresses too, folder and file names in
//! paths read `…` (Windows' own folders and the file type stay), and your home folder reads
//! `%USERPROFILE%`.
//!
//! Writing is a plain append (several threads take turns; the browser helper, a separate
//! Pako.exe, appends too). When the file gets big, only the newest problems are kept. Nothing
//! here can stop Pako: every failure is ignored, and the trimming never panics (a release
//! Pako aborts on a panic; `nothing_panics` throws odd text at it).

use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// How many problems are kept (and shown at most).
pub const KEEP: usize = 300;
/// The file is trimmed to the newest `KEEP` problems once it is bigger than this.
const MAX_BYTES: u64 = 128 * 1024;
/// At most this much of the file's end is read (a huge or damaged file never loads whole).
const READ_MAX: u64 = 4 * MAX_BYTES;
/// A message longer than this is cut (Pako's own messages are a line or two).
const MAX_MESSAGE: usize = 500;
/// Longer text (a contact's Pako can send any) is cut before trimming, at a word.
const PRE_CUT: usize = 4000;
/// The same problem again within this time is the same event (ten downloads failing together
/// when the connection drops): written once.
const REPEAT_WINDOW: Duration = Duration::from_secs(60);
/// At most this many problems are written per `REPEAT_WINDOW` (a burst of failures doesn't push
/// every older problem out of the list).
const BURST: usize = 30;

/// Where in Pako the problem happened.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Area {
    Download,
    Video,
    LinkCheck,
    Upload,
    Direct,
    Update,
    Extension,
    #[serde(other)]
    Other,
}

impl Area {
    /// A download that stopped: a video's own area when it is one.
    pub fn download(video: bool) -> Self {
        if video { Area::Video } else { Area::Download }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// When (ms since 1970, UTC).
    pub time: u64,
    pub area: Area,
    /// The message the user saw, trimmed (see `clean`).
    pub message: String,
    /// The Pako version that had it.
    pub version: String,
}

fn log_path() -> Option<PathBuf> {
    // Tests never write into the developer's data folder.
    if cfg!(test) {
        return None;
    }
    Some(crate::paths::data_dir().join("problems.log"))
}

/// The problems written lately (to skip repeats and bursts).
static TURN: Mutex<Vec<(Area, String, Instant)>> = Mutex::new(Vec::new());

/// Writes down a problem the user was just told about (the same words). Never fails.
pub fn note(area: Area, message: &str) {
    let message = clean(message);
    if message.is_empty() {
        return;
    }
    let Some(path) = log_path() else { return };
    // One writer at a time in this Pako; also remembers the last problems to skip repeats.
    let mut recent = TURN.lock().unwrap_or_else(|e| e.into_inner());
    if !worth_writing(&mut recent, area, &message, Instant::now()) {
        return;
    }
    let time = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let p = Problem { time, area, message, version: env!("CARGO_PKG_VERSION").to_string() };
    let _ = append(&path, &p, MAX_BYTES);
}

/// Pako is stopping because of an error it can't recover from (called by the panic hook, just
/// before the process ends): one line, written without any lock (the panic may have come from
/// inside `note`), trimmed like every other. The next `note` keeps the file small.
pub fn note_crash(message: &str) {
    let message = clean(&code_quotes(message));
    if cfg!(test) || message.is_empty() {
        return;
    }
    let Some(path) = crate::paths::data_dir_known().map(|d| d.join("problems.log")) else { return };
    let time = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let p = Problem { time, area: Area::Other, message, version: env!("CARGO_PKG_VERSION").to_string() };
    if let Ok(mut line) = serde_json::to_string(&p) {
        line.push('\n');
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

/// Rust's own panic messages quote text in backticks, which can be a file's name ("… inside 'é'
/// of `Holiday photos.zip`"): only code names (`Result::unwrap()`) stay, anything else reads `…`.
fn code_quotes(s: &str) -> String {
    let code = |q: &str| !q.is_empty() && q.chars().all(|c| c.is_ascii_alphanumeric() || "_:()<>&".contains(c));
    let mut out = String::with_capacity(s.len());
    let mut parts = s.split('`');
    out.push_str(parts.next().unwrap_or_default());
    let mut inside = true;
    for p in parts {
        out.push('`');
        out.push_str(if inside && !code(p) { "…" } else { p });
        inside = !inside;
    }
    out
}

/// Not the same problem as one written in the last minute, and not past `BURST` in that minute.
/// Remembers it when it is worth writing.
fn worth_writing(recent: &mut Vec<(Area, String, Instant)>, area: Area, message: &str, now: Instant) -> bool {
    recent.retain(|(_, _, at)| now.saturating_duration_since(*at) < REPEAT_WINDOW);
    if recent.len() >= BURST || recent.iter().any(|(a, m, _)| *a == area && m == message) {
        return false;
    }
    recent.push((area, message.to_string(), now));
    true
}

/// Adds one line; trims the file to the newest `KEEP` problems once it is over `max_bytes`.
fn append(path: &Path, p: &Problem, max_bytes: u64) -> std::io::Result<()> {
    let mut line = serde_json::to_string(p).map_err(std::io::Error::other)?;
    line.push('\n');
    // One write call per line, in append mode: lines from two Pako processes never mix.
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line.as_bytes())?;
    let size = f.metadata()?.len();
    drop(f);
    if size > max_bytes {
        let mut kept = read(path);
        // Still too big with `KEEP` long lines: fewer, so the next lines don't trim again at once.
        while kept.len() > 1 && kept.iter().map(|p| p.message.len() as u64 + 120).sum::<u64>() > max_bytes / 2 {
            kept.remove(0);
        }
        let mut out = String::new();
        for p in &kept {
            if let Ok(l) = serde_json::to_string(p) {
                out.push_str(&l);
                out.push('\n');
            }
        }
        let pb = path.to_path_buf();
        crate::paths::write_atomic(&pb, out.as_bytes())?;
        // write_atomic keeps the previous file as .bak: not wanted for this one.
        let _ = std::fs::remove_file(pb.with_extension("bak"));
    }
    Ok(())
}

/// The newest `KEEP` problems, oldest first. Lines that can't be read are skipped; of a huge
/// file only the end is read, and text that isn't UTF-8 doesn't hide the rest.
fn read(path: &Path) -> Vec<Problem> {
    let Ok(mut f) = std::fs::File::open(path) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let from = len.saturating_sub(READ_MAX);
    if from > 0 && f.seek(std::io::SeekFrom::Start(from)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if f.take(READ_MAX).read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    // Started in the middle: the first line is only a piece.
    let skip = usize::from(from > 0);
    let mut all: Vec<Problem> = text
        .lines()
        .skip(skip)
        .filter_map(|l| serde_json::from_str::<Problem>(l.trim()).ok())
        .map(|mut p| {
            // A file edited by hand still shows short lines.
            if p.message.chars().count() > MAX_MESSAGE {
                p.message = p.message.chars().take(MAX_MESSAGE - 1).collect::<String>() + "…";
            }
            p.version = p.version.chars().take(24).collect();
            p
        })
        .collect();
    if all.len() > KEEP {
        all.drain(..all.len() - KEEP);
    }
    all
}

// ---------- trimming ----------

/// The message without what could say who you are or what you download: links keep their
/// site only, cookies / tokens / passwords and long IDs are removed, email addresses too,
/// folder and file names in paths read `…` and the home folder reads `%USERPROFILE%`. Cut to
/// `MAX_MESSAGE` characters, on one line.
pub fn clean(message: &str) -> String {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    clean_with(message, &home)
}

fn clean_with(message: &str, home: &str) -> String {
    // Very long text is cut first, at a word: a word cut in half (an address, a name) goes.
    let mut text: String = message.chars().take(PRE_CUT).collect();
    if text.len() < message.len() {
        match text.rfind(char::is_whitespace) {
            Some(i) => text.truncate(i),
            None => text.clear(),
        }
    }
    // Invisible characters (they can disguise text) out, control characters read as spaces.
    let visible: String =
        text.chars().filter(|c| !crate::direct::is_hidden(*c)).map(|c| if c.is_control() { ' ' } else { c }).collect();
    let one_line: String = visible.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut s = header_values(&one_line);
    s = links(&s);
    s = bare_links(&s);
    s = secret_pairs(&s);
    s = emails(&s);
    s = paths(&s, home);
    s = quoted_names(&s);
    s = video_ids(&s);
    s = long_ids(&s);
    s = safety_codes(&s);
    if s.chars().count() > MAX_MESSAGE {
        s = s.chars().take(MAX_MESSAGE - 1).collect::<String>() + "…";
    }
    s
}

const REMOVED: &str = "[removed]";

/// `Cookie: …`, `Authorization: …` (and the like): everything after them goes.
fn header_values(s: &str) -> String {
    const HEADERS: [&str; 5] = ["set-cookie:", "cookie:", "proxy-authorization:", "authorization:", "x-api-key:"];
    let lower = s.to_ascii_lowercase();
    let cut = HEADERS
        .iter()
        .filter_map(|h| {
            lower.match_indices(h).find(|(i, _)| *i == 0 || !lower.as_bytes()[i - 1].is_ascii_alphanumeric() && lower.as_bytes()[i - 1] != b'-').map(|(i, _)| i + h.len())
        })
        .min();
    match cut {
        Some(at) => format!("{} {REMOVED}", s[..at].trim_end()),
        None => s.to_string(),
    }
}

fn url_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '"' | '\'' | '<' | '>' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\')
}

/// How long the link after `scheme://` is: up to a space or a quote, with an IPv6 host's
/// brackets (`http://[2001:db8::1]:8080/…`) kept inside.
fn link_len(tail: &str) -> usize {
    let mut in_authority = true;
    let mut bracket = false;
    for (i, c) in tail.char_indices() {
        if in_authority && matches!(c, '/' | '?' | '#') {
            in_authority = false;
        }
        let ok = url_char(c) || (in_authority && (c == '[' || (c == ']' && bracket)));
        if c == '[' && in_authority {
            bracket = true;
        }
        if !ok {
            return i;
        }
    }
    tail.len()
}

/// Every `scheme://…` link becomes `scheme://host` (web and FTP links) or `scheme://…`.
fn links(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(sep) = rest.find("://") {
        // The scheme: letters (then letters, digits, + . -) right before "://".
        let head = &rest[..sep];
        let start = head
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
            .last()
            .map(|(i, _)| i)
            .unwrap_or(sep);
        let start = head[start..].char_indices().find(|(_, c)| c.is_ascii_alphabetic()).map(|(i, _)| start + i).unwrap_or(sep);
        if start == sep {
            out.push_str(&rest[..sep + 3]);
            rest = &rest[sep + 3..];
            continue;
        }
        let tail = &rest[sep + 3..];
        let len = link_len(tail);
        let mut link = &rest[start..sep + 3 + len];
        // A full stop or comma after a link is the sentence's.
        while link.ends_with(['.', ',', ';', ':', '!', '?']) {
            link = &link[..link.len() - 1];
        }
        out.push_str(&rest[..start]);
        out.push_str(&site_only(link));
        rest = &rest[start + link.len()..];
    }
    out.push_str(rest);
    out
}

fn site_only(link: &str) -> String {
    let scheme = link.split("://").next().unwrap_or("").to_ascii_lowercase();
    let host = url::Url::parse(link).ok().and_then(|u| {
        u.host().map(|h| match h {
            url::Host::Domain(d) => short_host(d),
            other => other.to_string(),
        })
    });
    match host {
        Some(h) if !h.is_empty() && matches!(scheme.as_str(), "http" | "https" | "ftp" | "ftps" | "ws" | "wss") => format!("{scheme}://{h}"),
        _ => format!("{scheme}://…"),
    }
}

/// The site, without a server's own long name in front (`rr3---sn-4g5e6nsz.googlevideo.com`
/// says which video server near you): `….googlevideo.com`.
fn short_host(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').collect();
    let keep_from = labels.len().saturating_sub(2);
    labels
        .iter()
        .enumerate()
        .map(|(i, l)| if i < keep_from && l.len() >= 8 && l.chars().any(|c| c.is_ascii_digit()) { "…" } else { l })
        .collect::<Vec<_>>()
        .join(".")
}

/// A site's name: `example.com`, `www.files.example.co.uk` (labels of letters, digits and -,
/// ending in letters).
fn looks_like_host(h: &str) -> bool {
    let labels: Vec<&str> = h.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|l| !l.is_empty() && l.len() <= 63 && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        && labels.last().is_some_and(|t| (2..=24).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphabetic()))
}

/// Links written without `https://` (`www.example.com/a/b?token=…`, `//cdn.example.com/x`):
/// the site only.
fn bare_links(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            if w.contains("://") || w.contains('\\') {
                return w.to_string();
            }
            let lead = w.len() - w.trim_start_matches(['(', '[', '<', '"', '\'']).len();
            let body = &w[lead..];
            let (slashes, rest) = match body.strip_prefix("//") {
                Some(r) => ("//", r),
                None => ("", body),
            };
            let Some(end) = rest.find(['/', '?', '#']) else { return w.to_string() };
            let hostport = &rest[..end];
            let (host, port) = match hostport.split_once(':') {
                Some((h, p)) => (h, Some(p)),
                None => (hostport, None),
            };
            if !looks_like_host(host) || port.is_some_and(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit())) {
                return w.to_string();
            }
            // The sentence's punctuation after the link stays.
            let kept_end = w.trim_end_matches([')', '.', ',', ';', ':', '!', '?', ']', '>', '"', '\'']).len();
            let site_end = lead + slashes.len() + hostport.len();
            let trail = if kept_end > site_end { &w[kept_end..] } else { "" };
            format!("{}{slashes}{}{trail}", &w[..lead], short_host(host))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A name whose value is secret: `token=`, `"password":`, `X-Amz-Signature=`, `lsig=`, `code=`.
/// After `=` (links' and cookies' way) more names count than after `:` (where "code: 5" or
/// "key: …" are often plain words).
fn secret_name(name: &str, eq: bool) -> bool {
    const EXACT: [&str; 13] =
        ["auth", "apikey", "api_key", "key", "pwd", "session", "sid", "sig", "cookie", "secret", "token", "password", "passwd"];
    const ENDS: [&str; 11] =
        ["token", "secret", "password", "passwd", "signature", "credential", "credentials", "cookie", "api_key", "api-key", "apikey"];
    let k = name.trim_matches(['?', '&', '"', '\'', '{', '[', '(', ',']).to_ascii_lowercase();
    if k.is_empty() {
        return false;
    }
    EXACT.contains(&k.as_str())
        || ENDS.iter().any(|e| k.ends_with(e))
        || matches!(k.as_str(), "sessionid" | "session_id" | "client_secret")
        || (eq && (k == "code" || k.ends_with("sig") || k.ends_with("key") || k.ends_with("session") || k.ends_with("sessionid")))
}

/// `token=…`, `password: …`, `Bearer …`, `"token":"…"`: the value goes, the name stays.
fn secret_pairs(s: &str) -> String {
    let words: Vec<&str> = s.split(' ').collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut hide_next = false;
    for w in words {
        if hide_next && !w.is_empty() {
            out.push(REMOVED.to_string());
            hide_next = false;
            continue;
        }
        let lower = w.to_ascii_lowercase();
        if lower == "bearer" || lower == "basic" {
            hide_next = true;
            out.push(w.to_string());
            continue;
        }
        // name=value or name:value inside the word (several joined by & or ; or ,).
        let mut piece = String::new();
        let mut start = 0;
        for end in w.match_indices(['&', ';', ',']).map(|(i, _)| i).chain(std::iter::once(w.len())) {
            let part = &w[start..end];
            match part.find(['=', ':']) {
                Some(at) if secret_name(&part[..at], part.as_bytes()[at] == b'=') => {
                    piece.push_str(&part[..=at]);
                    if part[at + 1..].is_empty() {
                        // "password: hunter2": the value is the next word.
                        hide_next = true;
                    } else {
                        piece.push_str(REMOVED);
                    }
                }
                _ => piece.push_str(part),
            }
            // The & ; or , that ended this part.
            piece.push_str(&w[end..(end + 1).min(w.len())]);
            start = end + 1;
        }
        out.push(piece);
    }
    out.join(" ")
}

/// Email addresses (an account's, a contact's), also written `name%40example.com`: `[email]`.
fn emails(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let core = w.trim_matches(|c: char| !c.is_alphanumeric());
            let plain = core.replace("%40", "@");
            match plain.split_once('@') {
                Some((user, domain)) if !user.is_empty() && domain.contains('.') && !domain.starts_with('.') => w.replacen(core, "[email]", 1),
                _ => w.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Windows' own folders (and Pako's): their names say nothing about you, and help.
const KNOWN_FOLDERS: &[&str] = &[
    "users", "public", "default", "downloads", "documents", "desktop", "videos", "music", "pictures", "onedrive",
    "appdata", "local", "locallow", "roaming", "temp", "tmp", "program files", "program files (x86)", "programdata",
    "windows", "system32", "pako", "data", "tools", "?", ".", "..", "…",
];
/// Pako's own files and tools.
const KNOWN_FILES: &[&str] = &[
    "pako.exe", "ffmpeg.exe", "ffprobe.exe", "yt-dlp.exe", "deno.exe", "7z.exe", "7za.exe", "problems.log",
    "settings.json", "downloads.json",
];

fn is_sep(c: char) -> bool {
    c == '\\' || c == '/'
}

/// A file type at the end of a name (`.zip`, `.f137`, `.part`): 1 to 8 letters and digits,
/// with a letter.
fn file_ext(name: &str) -> Option<&str> {
    let (stem, ext) = name.rsplit_once('.')?;
    let ok = !stem.trim().is_empty()
        && (1..=8).contains(&ext.len())
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
        && ext.chars().any(|c| c.is_ascii_alphabetic());
    ok.then_some(ext)
}

/// Where a path starts at `i`: `C:\` / `C:/` (the root is "C:"), `\\server` (no root of its
/// own) or `%USERPROFILE%\`. The length of its root, in characters.
fn path_root(c: &[char], i: usize) -> Option<usize> {
    let prev_ok = i == 0 || !(c[i - 1].is_alphanumeric() || is_sep(c[i - 1]) || matches!(c[i - 1], '%' | '_' | '-' | '.' | ':'));
    if !prev_ok {
        return None;
    }
    let at = |k: usize| c.get(k).copied();
    if c[i].is_ascii_alphabetic() && at(i + 1) == Some(':') && at(i + 2).is_some_and(is_sep) {
        return Some(2);
    }
    if c[i] == '\\' && at(i + 1) == Some('\\') {
        let mut k = i;
        while at(k) == Some('\\') {
            k += 1;
        }
        return at(k).filter(|n| !n.is_whitespace() && !is_sep(*n)).map(|_| 0);
    }
    if c[i] == '%' {
        let close = c[i + 1..].iter().take(32).position(|&x| x == '%')?;
        let name = &c[i + 1..i + 1 + close];
        if !name.is_empty() && name.iter().all(|x| x.is_ascii_alphanumeric() || *x == '_') && at(i + close + 2).is_some_and(is_sep) {
            return Some(close + 2);
        }
    }
    None
}

/// Where the path part starting at `j` ends, and whether it is the path's last part. A part
/// runs to the next \ or /; the last one stops at a quote, `: `, `, `, `; `, an unmatched `)`
/// or the end (see `name_end` for spaces inside it).
fn part_end(c: &[char], j: usize) -> (usize, bool) {
    let mut k = j;
    let mut depth = 0i32;
    while k < c.len() {
        let ch = c[k];
        let next = c.get(k + 1).copied();
        if is_sep(ch) {
            return (k, false);
        }
        let stop = match ch {
            '"' | '<' | '>' | '|' | '*' | '`' | '“' | '”' => true,
            '?' => !next.is_some_and(is_sep),
            '\'' | '’' | '‘' => !next.is_some_and(|n| n.is_alphabetic()),
            ':' | ',' | ';' => next.is_none_or(|n| n == ' '),
            '(' | '[' => {
                depth += 1;
                false
            }
            ')' | ']' => {
                depth -= 1;
                depth < 0
            }
            _ => false,
        };
        if stop {
            break;
        }
        k += 1;
    }
    (name_end(c, j, k), true)
}

/// The end of the last part's name in `c[j..k]`, where the sentence may go on after it:
/// "Taxes 2025.pdf got shorter" ends after the file type, "Downloads is full" after a known
/// folder, "x (denied)" before the bracket; else all of it is the name (masking a few words
/// too many is better than showing a name). A full stop at the end is the sentence's.
fn name_end(c: &[char], j: usize, k: usize) -> usize {
    let mut e = k;
    while e > j && c[e - 1] == '.' {
        e -= 1;
    }
    if !c[j..e].contains(&' ') {
        return e;
    }
    let ends_with_ext = |p: usize| {
        let word_start = c[j..p].iter().rposition(|&x| x == ' ').map(|x| j + x + 1).unwrap_or(j);
        let word: String = c[word_start..p].iter().collect();
        file_ext(&word).is_some()
    };
    if let Some(p) = (j + 1..=e).rev().find(|&p| (p == e || c[p] == ' ') && ends_with_ext(p)) {
        return p;
    }
    if let Some(p) = (j..e.saturating_sub(1)).find(|&p| c[p] == ' ' && c[p + 1] == '(') {
        return p;
    }
    let first = c[j..e].iter().position(|&x| x == ' ').map(|x| j + x).unwrap_or(e);
    let word: String = c[j..first].iter().collect();
    if KNOWN_FOLDERS.contains(&word.to_lowercase().as_str()) {
        return first;
    }
    e
}

/// One part of a path as the report shows it.
fn mask_part(part: &str, last: bool) -> String {
    let lower = part.to_lowercase();
    let drive = part.len() == 2 && part.ends_with(':') && part.starts_with(|c: char| c.is_ascii_alphabetic());
    if part.is_empty() || part.starts_with('…') || drive || KNOWN_FOLDERS.contains(&lower.as_str()) || (last && KNOWN_FILES.contains(&lower.as_str())) {
        return part.to_string();
    }
    if lower.starts_with("onedrive") {
        // "OneDrive - Contoso": the company's name goes.
        return "OneDrive…".into();
    }
    match file_ext(part).filter(|_| last) {
        Some(ext) => format!("….{ext}"),
        None => "…".into(),
    }
}

/// The home folder's parts, lower case: `C:\Users\Sam` → ["c:", "users", "sam"].
fn home_parts(home: &str) -> Vec<String> {
    let h = home.trim_end_matches(['\\', '/']);
    let parts: Vec<String> = h.split(['\\', '/']).filter(|p| !p.is_empty()).map(str::to_lowercase).collect();
    let drive = parts.first().is_some_and(|d| d.len() == 2 && d.ends_with(':'));
    if drive && parts.len() >= 2 { parts } else { Vec::new() }
}

/// Paths (`C:\…`, `D:/…`, `\\server\share\…`, `%USERPROFILE%\…`, also with doubled \\ as a
/// tool writes them): your home folder reads `%USERPROFILE%`, Windows' own folders stay, every
/// other folder and file name reads `…` (a file keeps its type: `….zip`). A folder can carry
/// a name (`D:\Users\Rado`, `From Mia`, `OneDrive - Contoso`, a download folder named after
/// you), and file names say what you download or send.
fn paths(s: &str, home: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let home = home_parts(home);
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < c.len() {
        let Some(root_len) = path_root(&c, i) else {
            out.push(c[i]);
            i += 1;
            continue;
        };
        let root: String = c[i..i + root_len].iter().collect();
        // (separators as written, the part, last?)
        let mut parts: Vec<(String, String, bool)> = Vec::new();
        let mut j = i + root_len;
        loop {
            let from = j;
            while j < c.len() && is_sep(c[j]) {
                j += 1;
            }
            if j == from {
                break;
            }
            let seps: String = c[from..j].iter().collect();
            let start = j;
            let (end, last) = part_end(&c, start);
            parts.push((seps, c[start..end].iter().collect(), last || end == start));
            j = end;
            if last || end == start {
                break;
            }
        }
        if j == i {
            // Nothing read (can't happen: a root is always followed by a separator).
            out.push(c[i]);
            i += 1;
            continue;
        }
        let at_home = !home.is_empty()
            && root.eq_ignore_ascii_case(&home[0])
            && parts.len() + 1 >= home.len()
            && home[1..].iter().zip(&parts).all(|(h, (_, p, _))| p.to_lowercase() == *h);
        let skip = if at_home {
            out.push_str("%USERPROFILE%");
            home.len() - 1
        } else {
            out.push_str(&root);
            0
        };
        for (seps, part, last) in &parts[skip..] {
            out.push_str(seps);
            out.push_str(&mask_part(part, *last));
        }
        i = j;
    }
    out
}

/// A file name in quotes (`"Holiday.zip" changed while it was being uploaded`): `"….zip"`.
fn quoted_names(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < c.len() {
        let close = match c[i] {
            '"' => Some('"'),
            '\'' => Some('\''),
            '“' => Some('”'),
            '‘' => Some('’'),
            _ => None,
        };
        if let Some(close) = close.filter(|_| i == 0 || !c[i - 1].is_alphanumeric()) {
            // The closing quote: for ' one not inside a word ("Rado's").
            let found = (i + 1..c.len().min(i + 262)).find(|&k| c[k] == close && !c.get(k + 1).is_some_and(|n| n.is_alphanumeric()));
            if let Some(k) = found {
                let inner: String = c[i + 1..k].iter().collect();
                let plain = !inner.contains(['\\', '/', '…', '[']) && !KNOWN_FILES.contains(&inner.to_lowercase().as_str());
                if let Some(ext) = file_ext(&inner).filter(|_| plain) {
                    out.push(c[i]);
                    out.push_str(&format!("….{ext}"));
                    out.push(close);
                    i = k + 1;
                    continue;
                }
            }
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// The video tool's `[youtube] dQw4w9WgXcQ: …`: the video's ID says which video, `[youtube] …:`.
fn video_ids(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else { break };
        let name = &after[..close];
        let tail = &after[close + 1..];
        let extractor = !name.is_empty()
            && name.len() <= 40
            && name.starts_with(|c: char| c.is_ascii_alphabetic())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-' | '.' | ' '))
            && !matches!(name, "removed" | "id" | "email" | "code");
        if let Some(t) = tail.strip_prefix(' ').filter(|_| extractor) {
            let word_end = t.find(' ').unwrap_or(t.len());
            let word = &t[..word_end];
            if word.len() > 1 && word.ends_with(':') {
                out.push_str(&rest[..open + 1 + close + 2]);
                out.push_str("…:");
                rest = &t[word_end..];
                continue;
            }
        }
        out.push_str(&rest[..open + 1]);
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Long IDs and keys (a Pako ID, a file ID, a token): `[id]`. A run of letters, digits and
/// _ - + = with letters and digits mixed in its pieces, as names people write never are, or a
/// long one in mixed case with digits (a Drive file or folder ID).
fn long_ids(s: &str) -> String {
    let is_run = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '=');
    let mut out = String::with_capacity(s.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        let mixed = |p: &str| p.chars().any(|c| c.is_ascii_digit()) && p.chars().any(|c| c.is_ascii_alphabetic());
        let pieces: Vec<&str> = run.split(['_', '-', '+', '=']).filter(|p| mixed(p)).collect();
        // Capitals in the middle of a word (aB, 3C), several, plus digits: not "Lofi_rain_3".
        let b = run.as_bytes();
        let inner_caps = (1..b.len()).filter(|&i| b[i].is_ascii_uppercase() && b[i - 1].is_ascii_alphanumeric()).count();
        let random = run.len() >= 25 && inner_caps >= 3 && b.iter().any(|c| c.is_ascii_digit());
        let token = run.len() >= 20 && (pieces.iter().filter(|p| p.len() >= 4).count() >= 2 || pieces.iter().any(|p| p.len() >= 12) || random);
        out.push_str(if token { "[id]" } else { run });
        run.clear();
    };
    for c in s.chars() {
        if is_run(c) {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// A Direct safety code ("4821 0937 5512 0386 7741"): `[code]`.
fn safety_codes(s: &str) -> String {
    let words: Vec<&str> = s.split(' ').collect();
    let four = |w: &str| {
        let core = w.trim_end_matches(|c: char| !c.is_ascii_digit());
        core.len() == 4 && core.bytes().all(|b| b.is_ascii_digit())
    };
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        let mut n = 0;
        while i + n < words.len() && four(words[i + n]) && (n == 0 || words[i + n - 1].len() == 4) {
            n += 1;
        }
        if n >= 4 {
            let last = words[i + n - 1];
            out.push(format!("[code]{}", &last[last.trim_end_matches(|c: char| !c.is_ascii_digit()).len()..]));
            i += n;
        } else {
            out.push(words[i].to_string());
            i += 1;
        }
    }
    out.join(" ")
}

// ---------- the report (Help › Report a problem) ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportInfo {
    version: String,
    /// Installed with Pako Setup (false = the portable Pako.exe).
    installed: bool,
    /// "Windows 11 Pro 24H2 (build 26100)".
    windows: String,
    /// Oldest first.
    problems: Vec<Problem>,
}

/// What a report would carry (the window adds the description and the theme). The window asks
/// for it through lib.rs's `report_info` (off its thread).
pub fn report_info() -> ReportInfo {
    ReportInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        installed: crate::paths::is_installed(),
        windows: windows_version(),
        problems: log_path().map(|p| read(&p)).unwrap_or_default(),
    }
}

/// Where a report is saved: the place picked in the Save window, always a .txt file (so this
/// can't write over a program or a setting).
fn report_path(path: &str) -> Option<PathBuf> {
    let p = PathBuf::from(path);
    if !p.is_absolute() || p.file_name().is_none() {
        return None;
    }
    if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("txt")) {
        Some(p)
    } else {
        let mut s = p.into_os_string();
        s.push(".txt");
        Some(s.into())
    }
}

/// Saves the report as a text file (Windows line ends, so any editor shows it right).
#[tauri::command]
pub fn report_save(path: String, text: String) -> Result<(), String> {
    let Some(path) = report_path(&path) else { return Err(crate::tr!("Couldn't save the file there. Try another folder.", "Impossible d’enregistrer le fichier à cet endroit. Essayez un autre dossier.")) };
    let text = text.replace("\r\n", "\n").replace('\n', "\r\n");
    std::fs::write(&path, text.as_bytes()).map_err(|e| crate::tr!("Couldn't save the file ({e}). Try another folder.", "Impossible d’enregistrer le fichier ({e}). Essayez un autre dossier."))
}

/// The address reports go to.
const SUPPORT: &str = "mailto:support@sookera.com";
/// Longer than any report link (the window keeps them under 2000 characters).
const MAIL_MAX: usize = 8000;

/// Only a report: `mailto:support@sookera.com?subject=…&body=…`, both encoded the way the
/// window encodes them (encodeURIComponent), so nothing can add a recipient (`cc=`, `bcc=`,
/// `to=`, `,someone@…`) or a header, and the subject has no line breaks.
fn report_link_ok(url: &str) -> bool {
    let Some(q) = url.strip_prefix(SUPPORT).and_then(|r| r.strip_prefix("?subject=")) else { return false };
    let Some((subject, body)) = q.split_once("&body=") else { return false };
    // Letters, digits, - _ . ! ~ * ' ( ) and %XX; `allow` says which control characters may be
    // encoded (%0D%0A line ends and %09 tabs in the body only).
    let encoded = |p: &str, breaks: bool| {
        let b = p.as_bytes();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'%' => {
                    let Some(hex) = p.get(i + 1..i + 3).filter(|h| h.bytes().all(|x| x.is_ascii_hexdigit())) else { return false };
                    let v = u8::from_str_radix(hex, 16).unwrap_or(0);
                    if (v < 0x20 || v == 0x7f) && !(breaks && matches!(v, 0x09 | 0x0a | 0x0d)) {
                        return false;
                    }
                    i += 3;
                }
                x if x.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&x) => i += 1,
                _ => return false,
            }
        }
        true
    };
    url.len() <= MAIL_MAX && encoded(subject, false) && encoded(body, true)
}

/// Opens the user's email app on a new message to Sookera (nothing is sent by Pako).
#[tauri::command]
pub fn report_mail(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    if !report_link_ok(&url) {
        return Err(crate::tr!("Not a report", "Ce n’est pas un rapport"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| crate::tr!("Couldn't open your email app. Use Copy details and paste them into an email to support@sookera.com.", "Impossible d’ouvrir votre messagerie. Utilisez Copier et collez les détails dans un e-mail à support@sookera.com."))
}

// ---------- Send report: one click, to Sookera's report service ----------

/// Sookera's report service (a Cloudflare Worker on the studio's account): it files each report
/// as an issue in a private GitHub repository. What it takes and answers: `report_body` and
/// `service_answer` below.
const REPORT_URL: &str = "https://reports.sookera.com/v1/report";
/// The service refuses anything bigger.
const REPORT_MAX: usize = 32 * 1024;
const DESCRIPTION_MAX: usize = 5000;
/// The newest problems a report carries (the window's REPORT_PROBLEMS).
const REPORT_PROBLEMS: usize = 20;

/// One problem as the window shows it: the time with the PC's offset
/// ("2026-11-16T16:21:00+03:00"), the area, the trimmed message, the Pako version that had it.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SentProblem {
    time: String,
    area: String,
    message: String,
    version: String,
}

/// Why a report didn't go (the window says what to do instead).
#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SendFail {
    /// No connection, or the service didn't answer in time.
    Offline,
    /// The service is down or answered something unexpected.
    Server,
    /// Too many reports from this connection: try again after this many seconds.
    #[serde(rename_all = "camelCase")]
    Busy { retry_after: u64 },
    /// The report itself can't go (too long, empty…): `message` says why, for the user.
    Refused { message: String },
}

/// "2026-11-16T16:21:00+03:00" (or …Z): the only time form the service takes.
fn is_time(t: &str) -> bool {
    let b = t.as_bytes();
    let digits = |r: std::ops::Range<usize>| b.get(r).is_some_and(|x| x.iter().all(u8::is_ascii_digit));
    let at = |i: usize, c: u8| b.get(i) == Some(&c);
    let head = digits(0..4)
        && at(4, b'-')
        && digits(5..7)
        && at(7, b'-')
        && digits(8..10)
        && at(10, b'T')
        && digits(11..13)
        && at(13, b':')
        && digits(14..16)
        && at(16, b':')
        && digits(17..19);
    let tail: &[u8] = b.get(19..).unwrap_or_default();
    let offset = tail.len() == 6
        && matches!(tail[0], b'+' | b'-')
        && tail[1..3].iter().all(u8::is_ascii_digit)
        && tail[3] == b':'
        && tail[4..].iter().all(u8::is_ascii_digit);
    head && (tail == b"Z" || offset)
}

/// The service's name for an area ("links" for the link check; anything unknown is "other").
fn service_area(area: &str) -> &'static str {
    match area {
        "download" => "download",
        "video" => "video",
        "linkCheck" | "links" => "links",
        "upload" => "upload",
        "direct" => "direct",
        "update" => "update",
        "extension" => "extension",
        _ => "other",
    }
}

fn is_version(v: &str) -> bool {
    !v.is_empty() && v.len() <= 32 && v.chars().all(|c| c.is_ascii_alphanumeric() || ".-+".contains(c))
}

/// The JSON the service gets: what the window showed, checked again here (a message is trimmed
/// again, in case it didn't come from the problem list; trimming twice changes nothing).
fn report_body(description: &str, theme: &str, problems: &[SentProblem], version: &str, installed: bool, windows: &str) -> Result<String, SendFail> {
    let refused = |m: String| SendFail::Refused { message: m };
    let description = description.trim();
    if description.is_empty() {
        return Err(refused(crate::tr!("Say what happened first.", "Dites d’abord ce qui s’est passé.")));
    }
    if description.chars().count() > DESCRIPTION_MAX {
        return Err(refused(crate::tr!("What happened is too long: keep it under 5000 characters.", "Ce qui s’est passé est trop long : restez sous 5000 caractères.")));
    }
    if !matches!(theme, "dark" | "light" | "system") {
        return Err(refused(crate::tr!("Pako couldn't tell its theme. Try again.", "Pako n’a pas pu dire quel est son thème. Réessayez.")));
    }
    if problems.len() > REPORT_PROBLEMS {
        return Err(refused(crate::tr!("Too many problems attached. Try again.", "Trop de problèmes joints. Réessayez.")));
    }
    let mut list = Vec::with_capacity(problems.len());
    for p in problems {
        if !is_time(&p.time) || !is_version(&p.version) {
            return Err(refused(crate::tr!("A problem in the list can't be sent. Turn off Include recent problems, then try again.", "Un problème de la liste ne peut pas être envoyé. Désactivez Joindre les problèmes récents, puis réessayez.")));
        }
        list.push(serde_json::json!({
            "time": p.time,
            "area": service_area(&p.area),
            "message": clean(&p.message),
            "version": p.version,
        }));
    }
    let body = serde_json::json!({
        "app": "pako",
        "version": version,
        "install": if installed { "setup" } else { "portable" },
        "windows": windows.chars().take(120).collect::<String>(),
        "theme": theme,
        "description": description,
        "problems": list,
    })
    .to_string();
    if body.len() > REPORT_MAX {
        return Err(refused(crate::tr!("The report is too long. Shorten what happened, or turn off Include recent problems.", "Le rapport est trop long. Raccourcissez ce qui s’est passé, ou désactivez Joindre les problèmes récents.")));
    }
    Ok(body)
}

/// What the service's answer means: the report's reference, or why it didn't go.
fn service_answer(status: u16, retry_header: Option<u64>, body: &str) -> Result<String, SendFail> {
    let json: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    match status {
        200 | 201 => {
            let r = match &json["ref"] {
                serde_json::Value::String(s) => s.trim_start_matches('#').to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            };
            // A short, plain reference (it is shown to the user as it is).
            let plain = !r.is_empty() && r.len() <= 40 && r.chars().all(|c| c.is_ascii_alphanumeric() || "-_#".contains(c));
            if json["ok"] == serde_json::Value::Bool(true) && plain { Ok(r) } else { Err(SendFail::Server) }
        }
        429 => Err(SendFail::Busy { retry_after: json["retryAfter"].as_u64().or(retry_header).unwrap_or(3600).clamp(30, 86_400) }),
        413 => Err(SendFail::Refused { message: crate::tr!("The report is too long. Shorten what happened, or turn off Include recent problems.", "Le rapport est trop long. Raccourcissez ce qui s’est passé, ou désactivez Joindre les problèmes récents.") }),
        400 => Err(SendFail::Refused { message: crate::tr!("Sookera's report service didn't accept this report. Use Send by email instead.", "Le service de rapports de Sookera n’a pas accepté ce rapport. Utilisez plutôt Par e-mail.") }),
        _ => Err(SendFail::Server),
    }
}

/// Sends the report as the window shows it. Returns the report's reference.
#[tauri::command]
pub async fn report_send(description: String, theme: String, problems: Vec<SentProblem>) -> Result<String, SendFail> {
    let version = env!("CARGO_PKG_VERSION");
    let installed = crate::paths::is_installed();
    let windows = tauri::async_runtime::spawn_blocking(windows_version).await.unwrap_or_else(|_| "Windows".into());
    let body = report_body(&description, &theme, &problems, version, installed, &windows)?;
    let http = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent(format!("Pako/{version}"))
        // The report goes to this address only: a redirect (which would carry it elsewhere)
        // is an unexpected answer.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| SendFail::Offline)?;
    let res = http
        .post(REPORT_URL)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .map_err(|_| SendFail::Offline)?;
    let status = res.status().as_u16();
    let retry = res.headers().get(reqwest::header::RETRY_AFTER).and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse().ok());
    // The answer is small; anything bigger isn't the service's.
    let text = res.text().await.unwrap_or_default();
    service_answer(status, retry, if text.len() > 4096 { "" } else { &text })
}

/// "Windows 11 Pro 24H2 (build 26100)", read from the registry.
fn windows_version() -> String {
    use windows_sys::Win32::System::Registry::HKEY_LOCAL_MACHINE;
    const KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let get = |name: &str| crate::system::reg_get_value(HKEY_LOCAL_MACHINE, KEY, Some(name), 0).filter(|v| !v.trim().is_empty());
    let build = get("CurrentBuild").or_else(|| get("CurrentBuildNumber"));
    let product = get("ProductName").unwrap_or_else(|| "Windows".into());
    windows_label(&product, get("DisplayVersion").as_deref(), build.as_deref())
}

/// Windows 11 still calls itself "Windows 10" in the registry: its build number tells.
fn windows_label(product: &str, display: Option<&str>, build: Option<&str>) -> String {
    let eleven = build.and_then(|b| b.trim().parse::<u32>().ok()).is_some_and(|b| b >= 22000);
    let mut s = if eleven { product.replacen("Windows 10", "Windows 11", 1) } else { product.to_string() };
    if let Some(d) = display {
        s.push(' ');
        s.push_str(d.trim());
    }
    if let Some(b) = build {
        s.push_str(&format!(" (build {})", b.trim()));
    }
    s
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_window_timing_line_survives_cleaning() {
        let home = r"C:\Users\x";
        let m = "Window: Tutorial private took 1012 ms to show: start 3 ms, first frame 950 ms, found 2 ms, shown 40 ms; 9 frames, longest 948 ms";
        assert_eq!(clean_with(m, home), m);
        let m2 = "Window: Tutorial step private-list waited 900 ms; moving (dx/dy/dw/dh px): div.list 0/0/0.5/0; div.body 0/0/0.5/0";
        assert_eq!(clean_with(m2, home), m2);
    }

    use super::*;

    const HOME: &str = r"C:\Users\Sam";

    fn c(s: &str) -> String {
        clean_with(s, HOME)
    }

    #[test]
    fn links_keep_only_their_site() {
        assert_eq!(c("Couldn't reach https://files.example.com/a/b.zip?token=abc#x."), "Couldn't reach https://files.example.com.");
        assert_eq!(c("Got 403 from http://user:pw@cdn.host.net:8080/path"), "Got 403 from http://cdn.host.net");
        assert_eq!(c("(see https://drive.google.com/file/d/1AbC2dEf3GhI4jKl5MnO6pQr/view)"), "(see https://drive.google.com)");
        assert_eq!(c("Opened file:///C:/Users/Sam/x.txt and pako://add?link=x"), "Opened file://… and pako://…");
        assert_eq!(c("two links: https://a.com/1, https://b.org/2"), "two links: https://a.com, https://b.org");
        // Not a link: left as it is.
        assert_eq!(c("Pako 1.0 :// odd"), "Pako 1.0 :// odd");
    }

    #[test]
    fn links_in_odd_forms() {
        // IPv6 with a port: the address stays whole, the path goes.
        assert_eq!(c("Failed: http://[2001:db8::1]:8080/private/file.zip?k=1 now"), "Failed: http://[2001:db8::1] now");
        assert_eq!(c("at http://u:p@[::1]/secret"), "at http://[::1]");
        // Upper case, percent-encoded host and user name.
        assert_eq!(c("HTTPS://Me%40x.com:pw@EXAMPLE.com/a"), "https://example.com");
        assert_eq!(c("got http://%65xample.com/a"), "got http://example.com");
        // Without a scheme.
        assert_eq!(c("Open www.example.com/files/report.pdf?token=abc."), "Open www.example.com.");
        assert_eq!(c("from files.example.org:8443/x/y and (cdn.host.net/z)"), "from files.example.org and (cdn.host.net)");
        assert_eq!(c("see //cdn.example.com/a/b"), "see //cdn.example.com");
        // Gofile and Drive links, signed links.
        assert_eq!(c("https://gofile.io/d/aB3xYz gone"), "https://gofile.io gone");
        assert_eq!(c("https://drive.google.com/drive/folders/1AbCdEfGh-IjKlMnOpQ_RsTuVwXyZaBcD?usp=sharing"), "https://drive.google.com");
        assert_eq!(
            c("403 at https://bucket.s3.amazonaws.com/f.zip?X-Amz-Credential=AKIA/x&X-Amz-Signature=abc123"),
            "403 at https://bucket.s3.amazonaws.com"
        );
        // A video server's own name says where you are: only the site.
        assert_eq!(c("https://rr3---sn-4g5e6nsz.googlevideo.com/videoplayback?sig=AOq0QJ8&lsig=x"), "https://….googlevideo.com");
        assert_eq!(c("https://store12.gofile.io/x"), "https://store12.gofile.io");
        // Not links: versions, file names, plain words with a slash.
        assert_eq!(c("Pako 1.0.0 and/or yt-dlp 2026.11.01"), "Pako 1.0.0 and/or yt-dlp 2026.11.01");
    }

    #[test]
    fn signed_values_outside_links_go() {
        assert_eq!(c("X-Amz-Signature=deadbeef&X-Amz-Date=20261116"), "X-Amz-Signature=[removed]&X-Amz-Date=20261116");
        assert_eq!(c("sig=AOq0QJ8wRQIgb lsig=AG3C_xAwRgIh"), "sig=[removed] lsig=[removed]");
        assert_eq!(c("got code=4/0AeanS0b&scope=drive"), "got code=[removed]&scope=drive");
        assert_eq!(c(r#"{"access_token":"ya29.a0Af","expires_in":3599}"#), r#"{"access_token":[removed],"expires_in":3599}"#);
        assert_eq!(c(r#"{"refresh_token": "1//0gAb", "x": 1}"#), r#"{"refresh_token": [removed] "x": 1}"#);
        assert_eq!(c("X-Goog-Api-Key: AIzaSyA1"), "X-Goog-Api-Key: [removed]");
        assert_eq!(c("Set-Cookie: __Secure-3PSID=abc; Path=/"), "Set-Cookie: [removed]");
        // Plain words stay.
        assert_eq!(c("exit code: 1"), "exit code: 1");
    }

    #[test]
    fn home_folder_becomes_userprofile() {
        assert_eq!(c(r"Can't write C:\Users\Sam\Downloads\a.zip"), r"Can't write %USERPROFILE%\Downloads\….zip");
        assert_eq!(c(r"Can't write c:/users/sam/Videos"), "Can't write %USERPROFILE%/Videos");
        assert_eq!(c(r"In C:\Users\Sam."), "In %USERPROFILE%.");
        // Another folder that only starts the same way isn't the home folder (it is someone
        // else's, so its name goes too).
        assert_eq!(c(r"C:\Users\Samuel\x"), r"C:\Users\…\…");
        assert_eq!(c(r"C:\Users\Sam_2\x"), r"C:\Users\…\…");
        // A contact's Pako said where it couldn't save: their name goes.
        assert_eq!(
            c(r"Their Pako couldn't save it: D:\Users\Rado\Downloads\x (denied)."),
            r"Their Pako couldn't save it: D:\Users\…\Downloads\… (denied)."
        );
        assert_eq!(c(r"Saved in C:\Users\Mia."), r"Saved in C:\Users\….");
        assert_eq!(c(r"C:\Users\Public\Videos"), r"C:\Users\Public\Videos");
    }

    #[test]
    fn names_in_paths_go() {
        // The video tool writes paths with doubled \\ (Python): still your home folder.
        assert_eq!(
            c(r"The video tool said: Permission denied: 'C:\\Users\\Sam\\Downloads\\My holiday [dQw4w9WgXcQ].mp4.part'"),
            r"The video tool said: Permission denied: '%USERPROFILE%\\Downloads\\….part'"
        );
        // OneDrive with a company's name, a download folder with your name, a network share.
        assert_eq!(c(r"Can't write C:\Users\Sam\OneDrive - Contoso Ltd\Films\a.mkv"), r"Can't write %USERPROFILE%\OneDrive…\…\….mkv");
        assert_eq!(c(r"Can't write E:\Rado Rakoto films\Kids\x.mp4: denied"), r"Can't write E:\…\…\….mp4: denied");
        assert_eq!(c(r"No access to \\NAS-RADO\Family\Photos 2026\a.zip, try again"), r"No access to \\…\…\…\….zip, try again");
        assert_eq!(c(r"at \\?\C:\Users\Sam\Downloads\big.iso"), r"at \\?\C:\Users\…\Downloads\….iso");
        // Direct: a send's folder is named after the contact; file names with spaces.
        assert_eq!(
            c(r"Couldn't make the folder D:\Downloads\From Mia Rakoto: Access is denied. (os error 5)"),
            r"Couldn't make the folder D:\Downloads\…: Access is denied. (os error 5)"
        );
        assert_eq!(c(r"C:\Users\Sam\Documents\Taxes 2025 final.pdf got shorter while sending."), r"%USERPROFILE%\Documents\….pdf got shorter while sending.");
        assert_eq!(c(r"Couldn't read C:\Users\Sam\Documents\Rado's CV.docx: busy"), r"Couldn't read %USERPROFILE%\Documents\….docx: busy");
        assert_eq!(c(r"Their Pako couldn't save it: D:\Users\…\Downloads is full."), r"Their Pako couldn't save it: D:\Users\…\Downloads is full.");
        // Forward slashes, Pako's own tools, %USERPROFILE% already there.
        assert_eq!(c("Couldn't start C:/Program Files/Pako/tools/ffmpeg.exe"), "Couldn't start C:/Program Files/Pako/tools/ffmpeg.exe");
        assert_eq!(c(r"Can't write to %USERPROFILE%\Downloads\Videos: access denied."), r"Can't write to %USERPROFILE%\Downloads\Videos: access denied.");
        // Not paths.
        assert_eq!(c("Server busy (408). 80% done / 20% left"), "Server busy (408). 80% done / 20% left");
        assert_eq!(c("Time 14:02/15:00, ratio 3:4"), "Time 14:02/15:00, ratio 3:4");
    }

    #[test]
    fn quoted_file_names_and_video_ids_go() {
        assert_eq!(c("\"Holiday photos.zip\" changed while it was being uploaded. Try again."), "\"….zip\" changed while it was being uploaded. Try again.");
        assert_eq!(c("“Rado's CV.pdf” is open"), "“….pdf” is open");
        // Pako's own quoted words stay.
        assert_eq!(c("It stays in \"Pako uploads\" (or \"Pako sends\")."), "It stays in \"Pako uploads\" (or \"Pako sends\").");
        assert_eq!(c("Pako's helper 'ffmpeg.exe' stopped"), "Pako's helper 'ffmpeg.exe' stopped");
        assert_eq!(c("The video tool said: [youtube] dQw4w9WgXcQ: Something odd"), "The video tool said: [youtube] …: Something odd");
        assert_eq!(c("[vimeo] 123456789: gone, [youtube:tab] UCabc: x"), "[vimeo] …: gone, [youtube:tab] …: x");
    }

    #[test]
    fn cookies_tokens_and_ids_go() {
        assert_eq!(c("Sent Cookie: SID=31d4d96e407aad42; lang=en"), "Sent Cookie: [removed]");
        assert_eq!(c("Authorization: Bearer abc.def.ghi"), "Authorization: [removed]");
        assert_eq!(c("Answer: bearer eyJhbGciOiJIUzI1NiJ9 was refused"), "Answer: bearer [removed] was refused");
        assert_eq!(c("Bad token=abc123&lang=en here"), "Bad token=[removed]&lang=en here");
        assert_eq!(c("password: hunter2 was wrong"), "password: [removed] was wrong");
        assert_eq!(c("Signed in as me.name@gmail.com, then"), "Signed in as [email], then");
        // A Pako ID / file ID / UUID.
        assert_eq!(c("Contact k3f8a9d0q2w7e5r1t6y4u8i0o3p2a5s7d9f1g4h6 is offline"), "Contact [id] is offline");
        assert_eq!(c("Item 550e8400-e29b-41d4-a716-446655440000 failed"), "Item [id] failed");
        // A Drive ID whose pieces have few digits.
        assert_eq!(c("File 1AbCdEfGh-IjKlMnOpQ_RsTuVwXyZaBcD not found"), "File [id] not found");
        // Ordinary names stay.
        assert_eq!(c("Lofi_rain_cafe_3_hours_of_calm_music.mp4 failed"), "Lofi_rain_cafe_3_hours_of_calm_music.mp4 failed");
        assert_eq!(c("backup_v2_final_2026_11_16.zip"), "backup_v2_final_2026_11_16.zip");
        // Words that only look like keys are left alone.
        assert_eq!(c("Couldn't reach Pako: The pipe is busy"), "Couldn't reach Pako: The pipe is busy");
    }

    #[test]
    fn emails_in_odd_forms_go() {
        assert_eq!(c("Drive of <Sam.Smith+pako@Gmail.COM> is full"), "Drive of <[email]> is full");
        assert_eq!(c("login_hint=sam%40gmail.com&x=1"), "[email]");
        assert_eq!(c("(mailto:sam@x.org)"), "([email])");
        assert_eq!(c(r#"{"email":"sam@x.org"}"#), r#"{"[email]"}"#);
        assert_eq!(c("Pako 1.0 @ home"), "Pako 1.0 @ home");
    }

    #[test]
    fn direct_codes_and_hidden_characters_go() {
        assert_eq!(c("Code 4821 0937 5512 0386 7741. Check it"), "Code [code]. Check it");
        assert_eq!(c("Sizes 1024 2048"), "Sizes 1024 2048");
        // A contact's Pako can send any text: no hidden right-to-left tricks, no control characters.
        assert_eq!(c("photo\u{202E}gpj.exe\u{0007} done\u{200B}"), "photogpj.exe done");
    }

    #[test]
    fn messages_stay_one_short_line() {
        assert_eq!(c("line one\nline   two"), "line one line two");
        let long = c(&"word ".repeat(400));
        assert_eq!(long.chars().count(), MAX_MESSAGE);
        assert!(long.ends_with('…'));
        // Huge text is cut at a word first: an address cut in half doesn't show.
        let huge = format!("{} sam@gmail.com", "a ".repeat(PRE_CUT / 2 - 3));
        assert!(!c(&huge).contains("sam"));
        assert!(c(&"x".repeat(PRE_CUT * 3)).is_empty());
    }

    /// A release Pako aborts on a panic, and `note` runs inside the download engine: odd text
    /// of every kind must go through.
    fn sp(time: &str, area: &str, message: &str) -> SentProblem {
        SentProblem { time: time.into(), area: area.into(), message: message.into(), version: "1.0.0".into() }
    }

    #[test]
    fn send_times() {
        assert!(is_time("2026-11-16T16:21:00+03:00"));
        assert!(is_time("2026-11-16T16:21:00-05:30"));
        assert!(is_time("2026-11-16T16:21:00Z"));
        assert!(!is_time("2026-11-16 16:21"));
        assert!(!is_time("2026-11-16T16:21:00+0300"));
        assert!(!is_time(""));
        assert!(!is_time("2026-11-16T16:21:00+03:00x"));
    }

    #[test]
    fn send_body() {
        let ps = [sp("2026-11-16T16:21:00+03:00", "linkCheck", "File not found (https://cdn.example.com/a/b?token=x)")];
        let b = report_body("  It stopped.  ", "system", &ps, "1.0.0", true, "Windows 11 Pro").unwrap();
        let v: serde_json::Value = serde_json::from_str(&b).unwrap();
        assert_eq!(v["app"], "pako");
        assert_eq!(v["install"], "setup");
        assert_eq!(v["description"], "It stopped.");
        assert_eq!(v["problems"][0]["area"], "links");
        // Trimmed again here: only the site.
        assert!(!v["problems"][0]["message"].as_str().unwrap().contains("token"));
        assert_eq!(v.as_object().unwrap().len(), 7);
        assert_eq!(v["problems"][0].as_object().unwrap().len(), 4);

        let refused = |r: Result<String, SendFail>| matches!(r, Err(SendFail::Refused { .. }));
        assert!(refused(report_body("   ", "dark", &[], "1.0.0", false, "W")));
        assert!(refused(report_body(&"a".repeat(5001), "dark", &[], "1.0.0", false, "W")));
        assert!(report_body(&"\u{e9}".repeat(5000), "dark", &[], "1.0.0", false, "W").is_ok());
        assert!(refused(report_body("x", "blue", &[], "1.0.0", false, "W")));
        assert!(refused(report_body("x", "dark", &[sp("yesterday", "download", "m")], "1.0.0", false, "W")));
        let many: Vec<_> = (0..21).map(|_| sp("2026-11-16T16:21:00Z", "download", "m")).collect();
        assert!(refused(report_body("x", "dark", &many, "1.0.0", false, "W")));
    }

    #[test]
    fn send_answers() {
        assert_eq!(service_answer(201, None, r#"{"ok":true,"ref":"12"}"#), Ok("12".into()));
        assert_eq!(service_answer(201, None, r#"{"ok":true,"ref":12}"#), Ok("12".into()));
        assert_eq!(service_answer(201, None, r#"{"ok":true,"ref":"<b>x</b>"}"#), Err(SendFail::Server));
        assert_eq!(service_answer(201, None, "not json"), Err(SendFail::Server));
        assert_eq!(service_answer(429, None, r#"{"ok":false,"retryAfter":600}"#), Err(SendFail::Busy { retry_after: 600 }));
        assert_eq!(service_answer(429, Some(120), ""), Err(SendFail::Busy { retry_after: 120 }));
        assert_eq!(service_answer(429, None, ""), Err(SendFail::Busy { retry_after: 3600 }));
        assert!(matches!(service_answer(400, None, "{}"), Err(SendFail::Refused { .. })));
        assert!(matches!(service_answer(413, None, ""), Err(SendFail::Refused { .. })));
        assert_eq!(service_answer(502, None, ""), Err(SendFail::Server));
        assert_eq!(service_answer(404, None, ""), Err(SendFail::Server));
    }

    /// The service gets what the window showed: trimming a trimmed message changes nothing.
    #[test]
    fn clean_twice_is_clean_once() {
        for m in [
            r"Can't write to C:\Users\Sam\Downloads\Videos\holiday.mp4: access denied.",
            "Reached the file link, but: the server answered 403 (https://cdn.example.com/x/y?sig=abc).",
            r"Their Pako couldn't save it: D:\Users\Rado\Downloads\From Nina is full.",
            "[youtube] dQw4w9WgXcQ: This video is private.",
            "Couldn't reach www.example.org/files/a.zip (timed out). Mail me at sam@example.com",
            r#"Couldn't rename "Holiday photos 2025.zip": it is open in another app."#,
        ] {
            let once = clean_with(m, HOME);
            assert_eq!(clean_with(&once, HOME), once, "{m}");
        }
    }

    #[test]
    fn nothing_panics() {
        use rand::{Rng, SeedableRng};
        const BITS: &[&str] = &[
            "\\", "\\\\", "/", "//", ":", "://", "C:", "c:\\", "%", "%USERPROFILE%", "…", "é", "😀", "\u{202E}", "'", "\"", "“", "”",
            "‘", "’", "(", ")", "[", "]", "?", "&", "=", ";", ",", ".", " ", "  ", "@", "%40", "http", "https://", "[::1]", "www.",
            ".com", "token", "cookie:", "Bearer", "a", "Z", "0", "9", "-", "_", "+", "Users", "Sam", "\n", "\u{0}", "[youtube] ",
            "x.zip", "1234 ", "OneDrive",
        ];
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        for _ in 0..20_000 {
            let n = rng.gen_range(0..40);
            let s: String = (0..n).map(|_| BITS[rng.gen_range(0..BITS.len())]).collect();
            let out = clean_with(&s, HOME);
            assert!(out.chars().count() <= MAX_MESSAGE, "{s:?}");
            let _ = clean_with(&s, "");
            let _ = clean_with(&s, "C:");
        }
    }

    #[test]
    fn report_links_only_to_sookera() {
        let ok = "mailto:support@sookera.com?subject=Pako%201.0.0%3A%20problem%20report&body=What%20happened%3A%0D%0Ait%20broke%20(again)!";
        assert!(report_link_ok(ok));
        for bad in [
            "mailto:support@sookera.com?subject=x&body=y&cc=a%40b.c",
            "mailto:support@sookera.com?subject=x&bcc=a@b.c&body=y",
            "mailto:support@sookera.com,evil@x.com?subject=x&body=y",
            "mailto:support@sookera.com?to=evil@x.com&subject=x&body=y",
            "mailto:support@sookera.com?subject=x%0D%0ABcc:%20a@b.c&body=y",
            "mailto:support@sookera.com?subject=x&body=y%0",
            "mailto:support@sookera.com?subject=x&body=y%00",
            "mailto:support@sookera.com.evil.com?subject=x&body=y",
            "mailto:evil@x.com?subject=x&body=y",
            "https://example.com/?subject=x&body=y",
        ] {
            assert!(!report_link_ok(bad), "{bad}");
        }
        assert!(!report_link_ok(&format!("mailto:support@sookera.com?subject=x&body={}", "a".repeat(MAIL_MAX))));
    }

    #[test]
    fn reports_are_saved_as_text_files() {
        assert_eq!(report_path(r"C:\Users\Sam\Desktop\report.txt"), Some(PathBuf::from(r"C:\Users\Sam\Desktop\report.txt")));
        assert_eq!(report_path(r"C:\Users\Sam\Desktop\report.TXT"), Some(PathBuf::from(r"C:\Users\Sam\Desktop\report.TXT")));
        assert_eq!(report_path(r"C:\Windows\notepad.exe"), Some(PathBuf::from(r"C:\Windows\notepad.exe.txt")));
        assert_eq!(report_path(r"C:\x\report"), Some(PathBuf::from(r"C:\x\report.txt")));
        assert_eq!(report_path("report.txt"), None);
        assert_eq!(report_path(""), None);
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pako-errorlog-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("problems.log")
    }

    fn problem(i: usize, len: usize) -> Problem {
        Problem { time: i as u64, area: Area::Download, message: format!("{i:05} {}", "x".repeat(len)), version: "1.0.0".into() }
    }

    #[test]
    fn the_file_keeps_the_newest_and_stays_small() {
        let path = temp("cap");
        let max = 64 * 1024;
        for i in 0..2000 {
            append(&path, &problem(i, 200), max).unwrap();
            assert!(std::fs::metadata(&path).unwrap().len() <= max + 400, "grew past the cap at {i}");
        }
        let kept = read(&path);
        assert!(!kept.is_empty() && kept.len() <= KEEP);
        // Newest last, in order, and the last one written is there.
        assert_eq!(kept.last().unwrap().time, 1999);
        assert!(kept.windows(2).all(|w| w[0].time < w[1].time));
        assert!(!path.with_extension("bak").exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn reading_shows_at_most_keep_and_skips_bad_lines() {
        let path = temp("read");
        for i in 0..KEEP + 50 {
            append(&path, &problem(i, 5), u64::MAX).unwrap();
        }
        std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"not json\n{\"time\":1}\n").unwrap();
        let kept = read(&path);
        assert_eq!(kept.len(), KEEP);
        assert_eq!(kept.first().unwrap().time, 50);
        // An area a later Pako added reads as "other".
        std::fs::write(&path, "{\"time\":1,\"area\":\"teleport\",\"message\":\"m\",\"version\":\"9\"}\n").unwrap();
        assert_eq!(read(&path)[0].area, Area::Other);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_huge_or_damaged_file_is_read_from_its_end() {
        let path = temp("huge");
        // 3 MB of junk (not even UTF-8), then good lines: only the end is read, the lines show.
        let mut junk = vec![0xffu8; 3 * 1024 * 1024];
        junk.push(b'\n');
        std::fs::write(&path, &junk).unwrap();
        for i in 0..10 {
            append(&path, &problem(i, 5), u64::MAX).unwrap();
        }
        let kept = read(&path);
        assert_eq!(kept.len(), 10);
        // A huge line written by hand shows cut.
        let long = Problem { message: "y".repeat(10_000), version: "v".repeat(100), ..problem(1, 0) };
        std::fs::write(&path, serde_json::to_string(&long).unwrap()).unwrap();
        let p = &read(&path)[0];
        assert_eq!(p.message.chars().count(), MAX_MESSAGE);
        assert!(p.version.len() <= 24);
        // And the next problem written trims the huge file back under the cap.
        std::fs::write(&path, &junk).unwrap();
        append(&path, &problem(99, 5), MAX_BYTES).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() < MAX_BYTES);
        assert_eq!(read(&path).last().unwrap().time, 99);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn repeats_and_bursts_are_written_once() {
        let mut recent = Vec::new();
        let t = Instant::now();
        assert!(worth_writing(&mut recent, Area::Download, "a", t));
        assert!(worth_writing(&mut recent, Area::Download, "b", t));
        // "a" again while "b" came between: still the same event.
        assert!(!worth_writing(&mut recent, Area::Download, "a", t));
        assert!(worth_writing(&mut recent, Area::Video, "a", t));
        for i in 0..100 {
            worth_writing(&mut recent, Area::Download, &format!("m{i}"), t);
        }
        assert_eq!(recent.len(), BURST);
        // A minute later it is news again.
        assert!(worth_writing(&mut recent, Area::Download, "a", t + REPEAT_WINDOW));
    }

    #[test]
    fn several_threads_append_whole_lines() {
        let path = temp("threads");
        let handles: Vec<_> = (0..8)
            .map(|t| {
                let p = path.clone();
                std::thread::spawn(move || {
                    for i in 0..30 {
                        append(&p, &problem(t * 100 + i, 40), u64::MAX).unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(read(&path).len(), 240);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn windows_label_reads_right() {
        assert_eq!(windows_label("Windows 10 Pro", Some("24H2"), Some("26100")), "Windows 11 Pro 24H2 (build 26100)");
        assert_eq!(windows_label("Windows 10 Home", Some("22H2"), Some("19045")), "Windows 10 Home 22H2 (build 19045)");
        assert_eq!(windows_label("Windows", None, None), "Windows");
    }

    /// A panic's message keeps the code it names, never the text it quotes.
    #[test]
    fn crash_messages_hide_quoted_text() {
        assert_eq!(
            code_quotes("byte index 5 is not a char boundary; it is inside 'é' (bytes 4..6) of `Résumé Sam.pdf`"),
            "byte index 5 is not a char boundary; it is inside 'é' (bytes 4..6) of `…`"
        );
        assert_eq!(code_quotes("called `Result::unwrap()` on an `Err` value: x"), "called `Result::unwrap()` on an `Err` value: x");
        assert_eq!(code_quotes("odd ` count"), "odd `…");
        assert_eq!(code_quotes("plain"), "plain");
        assert_eq!(code_quotes(""), "");
    }
}
