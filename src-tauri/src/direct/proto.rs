// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! What two Pakos say to each other over a Direct connection, and the checks on what comes in.
//!
//! One control stream per connection carries small JSON messages, each prefixed with its length
//! (4 bytes, big endian). File bytes travel on separate one-way streams, each starting with a
//! small header saying which file and which part.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The protocol name both sides must speak.
pub const ALPN: &[u8] = b"pako/direct/1";

/// Biggest control message (an offer listing many files).
pub const MAX_MSG: usize = 4 * 1024 * 1024;
/// Biggest header of a data stream.
pub const MAX_HEADER: usize = 4 * 1024;
/// Most files in one offer.
pub const MAX_FILES: usize = 100_000;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum Msg {
    /// First message from the side that connects: its name as it introduces itself. `device`:
    /// "I'm pairing you as my own device" (older Pakos don't send it, and ignore it).
    Hello {
        name: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        device: bool,
        /// With `device`: this PC's own name ("Desk PC"), shown on the pairing question.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pc: Option<String>,
    },
    /// "You're one of my contacts": the connection is open for transfers.
    Welcome { name: String },
    /// "Asking my user": the first time only.
    Wait,
    /// "My user declined" (or removed you): the connection closes.
    Refused,
    /// "My user changed the name I go by" (sent to connected contacts at once; older Pakos ignore it
    /// and learn the name at the next connection).
    Name { name: String },
    /// Files I'd like to send you. `resume`: "if we're cut off, I can pick this up where it
    /// stopped" (older Pakos don't send it, and ignore it).
    Offer {
        id: String,
        name: String,
        files: Vec<OfferFile>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        resume: bool,
        /// Only between your own devices: "pass these files on to your contact `to`" (Direct
        /// relay). Older Pakos don't send it (and the switch never offers them). Never a path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relay: Option<RelayTo>,
    },
    /// `resume`: "I keep what arrives, so we can pick it up after a cut" (only when the offer
    /// said it can; older Pakos don't send it). Also the answer to `Continue`: ready again.
    Accept {
        id: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        resume: bool,
    },
    Decline { id: String },
    /// Either side stops a transfer.
    Cancel { id: String },
    /// All parts sent; one BLAKE3 hash per file (hex), in offer order.
    Done { id: String, hashes: Vec<String> },
    /// The receiver's verdict after checking.
    Result { id: String, ok: bool, message: Option<String> },
    // ---------- My devices (older Pakos read these as `Unknown` and ignore them) ----------
    /// "You're one of my devices" (`mine`), and "you may control my Pako" (`control`). Sent after
    /// connecting and whenever it changes. `pc_name`: this PC's name ("Desk PC"), when set; sent
    /// again when it's renamed (Pakos without it keep the name they had).
    Device {
        mine: bool,
        control: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pc_name: Option<String>,
        /// Only with `control`: "I can show you my Direct contacts and pass files on to them",
        /// with the name my contacts see and how many there are. Older Pakos don't send it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        relay: Option<RelayFrom>,
    },
    // ---------- Direct relay between your devices (older Pakos read these as `Unknown`) ----------
    /// "Show me your Direct contacts, or your history with one of them" (needs control);
    /// answered with `Seen` (same id).
    Look { id: String, look: Look },
    /// The answer to `Look`. `ok: false` with why (no permission, too many asks…).
    Seen {
        id: String,
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        me: Option<RemoteMe>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        contacts: Vec<RemoteContact>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        history: Vec<super::store::HistoryItem>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        moving: Vec<RemoteTransfer>,
    },
    /// The relaying PC tells the PC that asked how a relay is doing (`id`: the relay, the first
    /// leg's transfer id): `state` waiting / offered / sending / delivered / declined / failed /
    /// cancelled / expired, and how far the second leg is.
    Relayed {
        id: String,
        state: String,
        #[serde(default)]
        done: u64,
        #[serde(default)]
        size: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// The PC that asked: "stop this relay" (the copy is deleted there).
    Unrelay { id: String },
    /// "Send me your download list while I look at it" (or stop).
    Watch { on: bool },
    /// The download list, a summary (newest first).
    List { items: Vec<RemoteItem> },
    /// An order from one of your devices; answered with `Reply` (same id).
    Order { id: String, order: Order },
    /// `failed`: for an `Add`, the links that PC couldn't take (the others it took), so the PC
    /// that sent them keeps those. Left out when empty.
    Reply {
        id: String,
        ok: bool,
        message: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        failed: Vec<String>,
        /// For an `Add`: the downloads each link became there (their ids in its list), so the
        /// PC that sent them can follow them. Older Pakos don't send it. Left out when empty.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        items: Vec<LinkItems>,
    },
    // ---------- Picking up a transfer cut short (older Pakos read these as `Unknown`) ----------
    /// Sender, on connecting again: "I'm still keeping this transfer for you." A receiver that
    /// has it answers with `Have` (it sends that by itself when connecting); one that finished
    /// it answers `Result`; one that dropped it answers `Cancel`.
    Resume { id: String },
    /// Receiver: "Continue this one: here's what I already have." `full`: runs of files that
    /// are all here ([first, end) by their place in the offer); `part`: files partly here,
    /// with their byte ranges. Sent when connecting again, and when a file didn't match its
    /// hash (it's then left out, so it comes again in full).
    Have {
        id: String,
        #[serde(default)]
        full: Vec<[usize; 2]>,
        #[serde(default)]
        part: Vec<FileHave>,
    },
    /// Sender, answering `Have`: these files changed since, so they start again from zero at
    /// this size. The receiver answers `Accept` once ready, then the missing parts come.
    Continue {
        id: String,
        #[serde(default)]
        restart: Vec<Restart>,
    },
    /// A kind of message this Pako doesn't know (a newer Pako's): ignored, the connection stays.
    #[serde(other)]
    Unknown,
}

/// Pass the files of this offer on to `to` (a Pako ID among that PC's Direct contacts), as a
/// normal send from that PC. `keep`: keep a copy there once delivered (in the folder of the PC
/// that sent them), else the copy is deleted.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RelayTo {
    pub to: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keep: bool,
}

/// What a device that can relay says about its Direct side: the name its contacts see, and how
/// many contacts it has.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RelayFrom {
    pub name: String,
    pub contacts: u32,
}

/// What a `Look` asks for.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "what", rename_all = "camelCase")]
pub enum Look {
    Contacts,
    Talk { contact: String },
    /// A newer Pako's: refused.
    #[serde(other)]
    Unknown,
}

/// The relaying PC's Direct side: its Pako ID and the name its contacts see.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RemoteMe {
    pub id: String,
    pub name: String,
}

/// One of that PC's Direct contacts, as your other PC sees it (shown only, nothing to change).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteContact {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub their_name: Option<String>,
    #[serde(default)]
    pub online: bool,
    /// How it's connected ("local", "direct6", "direct", "relay").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<u64>,
    #[serde(default)]
    pub favourite: bool,
    /// Said yes once (a contact still waiting for that can't be sent to).
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub declined: bool,
}

/// A transfer moving on that PC with one of its contacts.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTransfer {
    pub id: String,
    pub outgoing: bool,
    pub name: String,
    pub files: u32,
    pub size: u64,
    pub done: u64,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Relayed for another of your PCs: its name there ("Laptop").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
}

/// One file partly received: which byte ranges ([start, end)) are on the disk there.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FileHave {
    pub f: usize,
    pub r: Vec<[u64; 2]>,
}

/// A file that changed on the sender's PC since it was offered: sent again from zero, at this size.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Restart {
    pub file: usize,
    pub size: u64,
}

/// What a device may ask another of your devices to do. Nothing else exists: no path on disk,
/// no program, nothing opened.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "do", rename_all = "camelCase")]
pub enum Order {
    /// Download these http(s) links, into that Pako's own download folder. `opts`: a link's
    /// video quality, priority and subfolder, chosen on the PC that sends it (older Pakos don't send it,
    /// and ignore it). Left out when empty.
    Add {
        links: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        opts: Vec<LinkOpts>,
    },
    Pause { item: String },
    Resume { item: String },
    /// Off the list (the file, if finished, stays on that PC).
    Remove { item: String },
    /// Send me this finished download (a normal Direct transfer).
    Get { item: String },
    /// Change a download's priority ("first", "normal" or "later"). An older Pako answers it
    /// as an order it doesn't know.
    Priority { item: String, priority: String },
    /// An order from a newer Pako: refused.
    #[serde(other)]
    Unknown,
}

/// What goes with one link of an `Add` order: the quality (a height in px; 0 = audio only),
/// the priority ("first", "normal", "later") and a subfolder name, picked on the PC that
/// sends it. Never a path of that PC, never cookies or headers. Matched to its link by the
/// link itself.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LinkOpts {
    pub link: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// A subfolder of that PC's own download folder ("Playlist name"): a relative name only,
    /// at most 2 levels, cleaned and checked there. Never a path of this PC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

/// The downloads one link of an `Add` became on the PC that took it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct LinkItems {
    pub link: String,
    pub ids: Vec<String>,
}

/// One download in a device's list, as another of your devices sees it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteItem {
    pub id: String,
    pub name: String,
    /// 0 = not known yet.
    pub size: u64,
    pub done: u64,
    /// Bytes per second (0 when not moving).
    pub speed: u64,
    /// queued, starting, downloading, retrying, paused, verifying, completed, failed...
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    /// "first" or "later" (left out for normal, and by older Pakos).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OfferFile {
    /// "Folder/sub/file.txt" (always "/", relative).
    pub path: String,
    pub size: u64,
}

/// What starts a data stream: this stream carries `len` bytes of file `file` from `offset`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Part {
    pub id: String,
    pub file: usize,
    pub offset: u64,
    pub len: u64,
}

pub async fn write_frame<W: AsyncWrite + Unpin, T: Serialize>(w: &mut W, value: &T, max: usize) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    if body.len() > max {
        // Nothing was written: the stream is still usable.
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "message too big"));
    }
    w.write_all(&(body.len() as u32).to_be_bytes()).await?;
    w.write_all(&body).await
}

/// One message, or None at a clean end of the stream.
pub async fn read_frame<R: AsyncRead + Unpin, T: for<'de> Deserialize<'de>>(r: &mut R, max: usize) -> std::io::Result<Option<T>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let n = u32::from_be_bytes(len) as usize;
    if n > max {
        return Err(std::io::Error::other("message too big"));
    }
    let mut body = vec![0u8; n];
    r.read_exact(&mut body).await?;
    serde_json::from_slice(&body).map(Some).map_err(std::io::Error::other)
}

/// A code both people can compare once ("4821 0937 5512 0386 7741"): the same on both PCs,
/// different for any other pair of IDs. 20 digits (about 66 bits), so nobody can make a key that
/// gives the same code as someone else's.
pub fn safety_code(a: &str, b: &str) -> String {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let mut h = blake3::Hasher::new();
    h.update(b"pako-direct-safety-code");
    h.update(lo.as_bytes());
    h.update(&[0]);
    h.update(hi.as_bytes());
    let d = h.finalize();
    let n = u128::from_be_bytes(d.as_bytes()[..16].try_into().expect("16 bytes")) % 100_000_000_000_000_000_000;
    let s = format!("{n:020}");
    format!("{} {} {} {} {}", &s[0..4], &s[4..8], &s[8..12], &s[12..16], &s[16..20])
}

/// A transfer id as Pako makes them (a UUID): anything else from the other side is refused.
pub fn valid_id(id: &str) -> bool {
    id.len() == 36 && uuid::Uuid::parse_str(id).is_ok()
}

/// Invisible characters that can disguise a name: right-to-left overrides and isolates
/// ("photo\u{202E}gpj.exe" shows as "photoexe.jpg"), zero-width spaces and joiners, and the
/// other Unicode "format" characters.
pub fn is_hidden(c: char) -> bool {
    matches!(c as u32,
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 | 0x180B..=0x180F | 0x200B..=0x200F
        | 0x202A..=0x202E | 0x2060..=0x206F | 0x3164 | 0xFE00..=0xFE0F | 0xFEFF | 0xFFA0 | 0xFFF0..=0xFFFB
        | 0x1D173..=0x1D17A | 0xE0000..=0xE0FFF)
}

/// Kinds of file that run code (or open something that does) when opened. Pako shows these in
/// their folder instead of opening them, so a received program never starts with one click.
const RUNS_CODE: &[&str] = &[
    "exe", "msi", "msp", "msix", "msixbundle", "appx", "appxbundle", "appref-ms", "bat", "cmd", "com", "cpl", "scr",
    "pif", "ps1", "psm1", "psd1", "ps1xml", "vbs", "vbe", "js", "jse", "wsf", "wsh", "wsc", "hta", "jar", "lnk",
    "url", "reg", "inf", "msc", "gadget", "application", "chm", "dll", "sys", "drv", "ocx", "vb", "sct", "shs",
    "scf", "website", "settingcontent-ms", "library-ms", "searchconnector-ms", "xll", "xlam", "ppam", "docm",
    "xlsm", "pptm", "iso", "img", "vhd", "vhdx", "py", "pyw", "pl", "rb", "sh", "diagcab", "mht", "mhtml",
    "theme", "themepack", "desktopthemepack", "rdp", "cab", "ins", "isp", "job", "mde", "mdb", "accde",
    "msh", "msh1", "msh2", "mshxml", "msh1xml", "msh2xml", "ps2", "ps2xml", "psc1", "psc2", "ws", "appinstaller",
    "msu", "mst", "hlp", "jnlp", "xbap", "ade", "adp", "pyz", "pyzw", "pyc", "dotm", "xltm", "xlsb", "xla", "potm",
    "ppsm", "sldm", "wsb", "search-ms",
];

/// This file would run code if opened (by its last extension).
pub fn runs_code(path: &std::path::Path) -> bool {
    path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).is_some_and(|e| RUNS_CODE.contains(&e.as_str()))
}

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "COM\u{B9}", "COM\u{B2}", "COM\u{B3}", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
    "LPT7", "LPT8", "LPT9", "LPT\u{B9}", "LPT\u{B2}", "LPT\u{B3}",
];

/// Longest name part kept (characters).
const MAX_PART: usize = 180;

/// A received name made safe to use as part of a path on Windows, or None if it can't be.
/// Never lets a name climb out of the folder it's saved in.
pub fn safe_part(part: &str) -> Option<String> {
    if part.is_empty() || part == "." || part == ".." {
        return None;
    }
    // Hidden characters go; cut to length first, so the trimming below sees the real end.
    let mut s: String = part
        .chars()
        .filter(|c| !is_hidden(*c))
        .map(|c| if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') { '_' } else { c })
        .take(MAX_PART)
        .collect();
    // Windows drops trailing dots and spaces, which could turn "a." into "a" behind our back.
    while s.ends_with('.') || s.ends_with(' ') {
        s.pop();
    }
    let s = s.trim_start().to_string();
    if s.is_empty() || s.chars().all(|c| c == '.' || c == '_') && s.contains('.') {
        return None;
    }
    let stem = s.split('.').next().unwrap_or("").trim_end().to_uppercase();
    let s = if RESERVED.contains(&stem.as_str()) { format!("_{s}") } else { s };
    Some(s)
}

/// "Folder/sub/file.txt" from the other side, as safe path parts (None = refuse the offer).
pub fn safe_rel(path: &str) -> Option<Vec<String>> {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') || path.contains('\\') || path.contains(':') {
        return None;
    }
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() > 32 {
        return None;
    }
    let mut out = Vec::with_capacity(parts.len());
    for p in parts {
        if p == ".." || p == "." || p.is_empty() {
            return None;
        }
        out.push(safe_part(p)?);
    }
    Some(out)
}

/// Longest file path in an offer (bytes).
const MAX_PATH_BYTES: usize = 2048;

/// Checks a whole offer before it's shown: sane names, no duplicates, sizes add up.
pub fn check_offer(name: &str, files: &[OfferFile]) -> Result<u64, String> {
    if files.is_empty() || files.len() > MAX_FILES {
        return Err("The offer has no files, or far too many.".into());
    }
    if name.chars().count() > 200 {
        return Err("The offer's name is too long.".into());
    }

    let mut seen = std::collections::HashSet::new();
    let mut total: u64 = 0;
    for f in files {
        if f.path.len() > MAX_PATH_BYTES || f.path.split('/').any(|p| p.chars().count() > MAX_PART) {
            return Err("A file name is too long.".into());
        }

        let parts = safe_rel(&f.path).ok_or_else(|| format!("A file name isn't safe: {}", f.path))?;
        // Windows ignores case, and two names can end up the same once made safe ("a?" and "a*").
        if !seen.insert(parts.join("/").to_uppercase()) {
            return Err("The offer lists the same file twice.".into());
        }
        total = total.checked_add(f.size).ok_or("The sizes don't add up.")?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_never_climb_out() {
        assert_eq!(safe_rel("a/b.txt"), Some(vec!["a".into(), "b.txt".into()]));
        assert_eq!(safe_rel("../x"), None);
        assert_eq!(safe_rel("a/../x"), None);
        assert_eq!(safe_rel("/etc/x"), None);
        assert_eq!(safe_rel("C:/x"), None);
        assert_eq!(safe_rel("a\\b"), None);
        assert_eq!(safe_rel("a//b"), None);
        assert_eq!(safe_rel("."), None);
        assert_eq!(safe_part("CON"), Some("_CON".into()));
        assert_eq!(safe_part("con.txt"), Some("_con.txt".into()));
        assert_eq!(safe_part("report.  "), Some("report".into()));
        assert_eq!(safe_part("a<b>c?.txt"), Some("a_b_c_.txt".into()));
        assert_eq!(safe_part("..."), None);
        assert_eq!(safe_part("bad\u{0}name"), Some("bad_name".into()));
        // Hidden characters can't disguise a type.
        assert_eq!(safe_part("photo\u{202E}gpj.exe"), Some("photogpj.exe".into()));
        assert_eq!(safe_part("a\u{200B}b\u{2066}.txt\u{FEFF}"), Some("ab.txt".into()));
        // More reserved names.
        for r in ["COM0", "lpt0.txt", "CONIN$", "conout$.log", "COM\u{B9}", "lpt\u{B3}.x"] {
            assert!(safe_part(r).unwrap().starts_with('_'), "{r}");
        }
        // Cut first, then trimmed: a long name never ends with a dot or a space.
        let long = format!("{}. .txt", "a".repeat(178));
        let s = safe_part(&long).unwrap();
        assert!(s.chars().count() <= 180 && !s.ends_with('.') && !s.ends_with(' '), "{s:?}");
    }

    #[test]
    fn ids_and_programs() {
        assert!(valid_id(&uuid::Uuid::new_v4().to_string()));
        assert!(!valid_id("t1"));
        assert!(!valid_id(&"a".repeat(36)));
        assert!(runs_code(std::path::Path::new("C:/x/setup.EXE")));
        assert!(runs_code(std::path::Path::new("a.tar.ps1")));
        assert!(runs_code(std::path::Path::new("link.lnk")));
        assert!(runs_code(std::path::Path::new("Update.appinstaller")));
        assert!(runs_code(std::path::Path::new("tool.pyz")));
        assert!(!runs_code(std::path::Path::new("photo.jpg")));
        assert!(!runs_code(std::path::Path::new("folder")));
    }

    #[test]
    fn offers_are_checked() {
        let ok = vec![OfferFile { path: "A/x.bin".into(), size: 5 }, OfferFile { path: "A/y.bin".into(), size: 7 }];
        assert_eq!(check_offer("A", &ok), Ok(12));
        let dup = vec![OfferFile { path: "a.txt".into(), size: 1 }, OfferFile { path: "A.TXT".into(), size: 1 }];
        assert!(check_offer("x", &dup).is_err());
        assert!(check_offer("x", &[OfferFile { path: "../a".into(), size: 1 }]).is_err());
        assert!(check_offer("x", &[]).is_err());
        let huge = vec![OfferFile { path: "a".into(), size: u64::MAX }, OfferFile { path: "b".into(), size: 1 }];
        assert!(check_offer("x", &huge).is_err());
        // Two names that become the same once made safe.
        let same = vec![OfferFile { path: "a?".into(), size: 1 }, OfferFile { path: "a*".into(), size: 1 }];
        assert!(check_offer("x", &same).is_err());

        assert!(check_offer("x", &[OfferFile { path: "a".repeat(300), size: 1 }]).is_err());
    }

    #[test]
    fn safety_code_is_symmetric_and_specific() {
        let ab = safety_code("alice", "bob");
        assert_eq!(ab, safety_code("bob", "alice"));
        assert_ne!(ab, safety_code("alice", "carol"));
        assert_eq!(ab.len(), 24);
        assert_eq!(ab.split(' ').count(), 5);
        assert!(ab.chars().all(|c| c.is_ascii_digit() || c == ' '));
    }

    #[tokio::test]
    async fn frames_round_trip_and_cap() {
        let (mut a, mut b) = tokio::io::duplex(1 << 16);
        let m = Msg::Offer { id: "t1".into(), name: "Photos".into(), files: vec![OfferFile { path: "a.jpg".into(), size: 3 }], resume: false, relay: None };
        write_frame(&mut a, &m, MAX_MSG).await.unwrap();
        write_frame(&mut a, &Msg::Accept { id: "t1".into(), resume: false }, MAX_MSG).await.unwrap();
        drop(a);
        assert_eq!(read_frame::<_, Msg>(&mut b, MAX_MSG).await.unwrap(), Some(m));
        assert_eq!(read_frame::<_, Msg>(&mut b, MAX_MSG).await.unwrap(), Some(Msg::Accept { id: "t1".into(), resume: false }));
        assert_eq!(read_frame::<_, Msg>(&mut b, MAX_MSG).await.unwrap(), None);
        // A message kind from a newer Pako is read as "unknown", not as a broken connection.
        let (mut a, mut b) = tokio::io::duplex(1 << 16);
        let newer = br#"{"t":"somethingNew","id":"x","more":[1,2]}"#;
        a.write_all(&(newer.len() as u32).to_be_bytes()).await.unwrap();
        a.write_all(newer).await.unwrap();
        assert_eq!(read_frame::<_, Msg>(&mut b, MAX_MSG).await.unwrap(), Some(Msg::Unknown));
        let (mut a, mut b) = tokio::io::duplex(1 << 16);
        a.write_all(&(10_000u32).to_be_bytes()).await.unwrap();
        assert!(read_frame::<_, Msg>(&mut b, 100).await.is_err());
    }
}
