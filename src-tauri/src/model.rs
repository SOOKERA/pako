// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Data saved to disk and sent to the window.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Source {
    /// A plain web link.
    Http { url: String },
    /// A Google Drive file, downloaded through Google's public download address (no sign-in).
    /// Lists saved by older versions may hold `exportMime` / `copyId` too: they are skipped.
    #[serde(rename_all = "camelCase")]
    GDrive {
        file_id: String,
        resource_key: Option<String>,
        /// A Google Doc/Sheet/Slides/Drawing: "document", "spreadsheets", "presentation" or
        /// "drawings", exported through Google's public export link (Word, Excel, PowerPoint, PNG).
        #[serde(default)]
        doc_type: Option<String>,
    },
    /// A video from YouTube or another site, downloaded with yt-dlp.
    #[serde(rename_all = "camelCase")]
    Video {
        url: String,
        /// Height in lines (1080...); None = the default from Settings; 0 = audio only.
        #[serde(default)]
        quality: Option<u32>,
        /// Audio only: "m4a" or "mp3".
        #[serde(default)]
        audio: Option<String>,
        /// Qualities the video has, with their sizes (best first).
        #[serde(default)]
        options: Vec<VideoOption>,
        /// Best sound-only size.
        #[serde(default)]
        audio_size: Option<u64>,
        /// Length in seconds.
        #[serde(default)]
        duration: Option<f64>,
        /// Streams Pako downloads itself (in parallel parts) and how far each one is.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        streams: Vec<StreamPart>,
        /// The page a caught stream played on, sent as the referrer (the browser's own rule:
        /// only "https://site/" when the stream is on another site). None for other videos.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        referrer: Option<String>,
        /// A stream the browser extension caught playing on a page ("Videos on this page"):
        /// when yt-dlp can't get it, Pako's own HLS download tries.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        caught: bool,
        /// "Record this video": the folder holding what the page played (one file per
        /// track), joined into the video when it runs.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recording: Option<PathBuf>,
    },
}

/// One stream of a video that Pako downloads itself, and how far it is (saved, so a pause or a
/// restart continues where it stopped).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamPart {
    pub format_id: String,
    pub ext: String,
    pub size: u64,
    #[serde(default)]
    pub has_video: bool,
    #[serde(default)]
    pub has_audio: bool,
    pub segments: Vec<Segment>,
}

/// One quality a video comes in.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VideoOption {
    pub height: u32,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Queued,
    Starting,
    Downloading,
    /// Connection problem: waiting before trying again. Never gives up by itself.
    Retrying,
    Paused,
    Verifying,
    Completed,
    Failed,
    /// Google refused because too many people downloaded this shared file today.
    QuotaExceeded,
    /// Google only gives the file to people it's shared with (or stopped the public link).
    /// Downloading it in the browser works: Pako catches that download.
    NeedsAccount,
}

impl Status {
    pub fn is_active(&self) -> bool {
        matches!(self, Status::Starting | Status::Downloading | Status::Retrying | Status::Verifying)
    }
}

/// Where a download stands in the queue. The queue runs First, then Normal, then Later.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Priority {
    /// Starts before the others; takes the spot of a lower one if all spots are busy.
    First,
    #[default]
    Normal,
    /// Only runs when nothing else is waiting.
    Later,
}

/// How a finished download tells you: **Off** (nothing), **Pop-up** (the card at the bottom
/// right and the arrived chime) or **Important** (a big card in the middle of the screen that
/// stays until clicked, the chime, through Do not disturb but never over something full
/// screen). Ordered: a folder or set says it as loud as its loudest file.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Alert {
    Off,
    #[default]
    Pop,
    Important,
}

/// Settings › Notifications › "Chime when a download finishes".
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DoneChime {
    /// Pop-up and Important.
    #[default]
    All,
    /// Important only.
    Important,
    Never,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    pub start: u64,
    /// Exclusive end. Can shrink when another connection takes over half of this part.
    pub end: u64,
    /// Bytes already written from `start`.
    pub done: u64,
}

impl Segment {
    pub fn remaining(&self) -> u64 {
        self.end.saturating_sub(self.start + self.done)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    pub id: String,
    /// The link as the user pasted it.
    pub link: String,
    pub source: Source,
    pub name: String,
    pub dest_dir: PathBuf,
    pub size: Option<u64>,
    pub md5: Option<String>,
    pub status: Status,
    /// Plain-language reason for Retrying / Failed / QuotaExceeded.
    pub message: Option<String>,
    pub resumable: bool,
    pub segments: Vec<Segment>,
    pub downloaded: u64,
    /// Queue order (moving a download in the queue changes it). Not a date to show.
    pub added_at: u64,
    /// When it was added to the downloads (ms). Never changes.
    #[serde(default)]
    pub date: u64,
    pub finished_at: Option<u64>,
    /// Name of the Drive folder this file came from, for grouping in the list.
    pub group: Option<String>,
    /// Whether metadata (name, size) has been fetched.
    pub resolved: bool,
    /// Old "Start now" flag, read from older lists only (it now sets priority First).
    #[serde(default, skip_serializing)]
    pub force: bool,
    #[serde(default)]
    pub priority: Priority,
    /// The old "Notify when done" (read from older lists: it is **Pop-up** now, see `alert_of`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub notify: bool,
    /// Off · Pop-up · Important for this download. None = the default in Settings (or what an
    /// older list's `notify` / `quiet` said).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert: Option<Alert>,
    /// Part of a split archive: extract the set once every part is downloaded.
    #[serde(default)]
    pub extract: bool,
    /// What identifies this version of the file on the server (its strong ETag, else its
    /// Last-Modified date), noted when the download is first checked. Resumed parts send it
    /// back, so a file that changed on the server is never stitched onto the old bytes.
    #[serde(default)]
    pub validator: Option<String>,
    /// Came from the browser and waits for the "Download with Pako" window's answer (not
    /// started until then).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ask: bool,
    /// The old "Notify me" switched off in the Download with Pako window (read from older
    /// lists: it is **Off** now, see `alert_of`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quiet: bool,
    /// The same file on other hosts (grouped in Links), in the order they are tried when the
    /// host in use fails for good. Lists saved before mirrors existed have none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mirrors: Vec<Mirror>,
    /// Hosts that already failed for good, in the order they were tried (with why).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tried: Vec<Mirror>,
    /// Just moved to another host with the same size: the parts on disk came from the host
    /// before, so a few samples from this host are compared with them before it goes on (else
    /// it starts over). Set until that check has answered.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub spot_check: bool,
    /// The file holds bytes from two hosts (kept after a host change): if the final checksum
    /// fails, it starts over from zero on this host instead of stopping.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub spliced: bool,
    /// Its link came from another of your PCs (My devices): an answer from this PC itself
    /// (localhost) is never taken for it, so that PC can't read what only listens here.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_device: bool,
    /// Pako couldn't get this file itself (a site that only lets the browser have it): the
    /// browser downloads it and Pako follows it ("Via browser"), then moves it to `dest_dir`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via_browser: Option<ViaBrowser>,
    /// Its name was picked by you (Download with Pako window, Rename): kept when a file the
    /// browser downloaded comes in under another name.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub named: bool,
    /// From a private browser window: never in the Dashboard's numbers or the error log, no
    /// line kept in the bell, never saved in the list on disk, and off the list soon after it
    /// finishes (see `Settings::private_keep` and engine `private_keep_for`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub private: bool,
    /// Kept in Private mode's vault: its file is encrypted as it's written (see `vault`), its
    /// line is only in the private list (sealed), never in the list on disk. Always `private`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub vault: bool,
    /// In Private mode, a file of a folder download (a Drive folder, a split set, a playlist):
    /// its place inside that folder ("Holiday\day 2"), the folder's own name first. Its file
    /// waits in the vault, so this keeps the layout for the private list and for Decrypt….
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_folder: Option<String>,
}

/// A download the browser makes, followed by Pako (see `Download::via_browser`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ViaBrowser {
    /// The browser's own id for it (-1 = not reported yet).
    pub browser_id: i64,
    /// Where the browser saves it (known once it reports it).
    pub path: String,
    /// The browser can pause and resume it (its server allows continuing).
    pub can_resume: bool,
    /// Reported by Pako Light.
    pub light: bool,
}

/// One host offering a file: a backup of a download (or a host it already tried).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mirror {
    pub link: String,
    pub source: Source,
    /// What the Links check found on this host.
    #[serde(default)]
    pub size: Option<u64>,
    /// Google Drive's MD5 of the file.
    #[serde(default)]
    pub md5: Option<String>,
    /// Strong ETag (else Last-Modified) the host sent.
    #[serde(default)]
    pub validator: Option<String>,
    #[serde(default)]
    pub resumable: bool,
    /// Why this host was left (tried hosts only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

impl Download {
    /// How this download tells you when it finishes: its own choice, else what an older list
    /// said ("Notify when done" = Pop-up, the window's "Notify me" off = Off), else `default`
    /// (Settings › Notifications › New downloads).
    pub fn alert_of(&self, default: Alert) -> Alert {
        self.alert.unwrap_or(if self.notify {
            Alert::Pop
        } else if self.quiet {
            Alert::Off
        } else {
            default
        })
    }

    /// Sets its choice. The older flags follow it, so an older Pako reading the list does the
    /// same (its "Notify when done" for Pop-up and Important, its "Notify me" off for Off).
    pub fn set_alert(&mut self, a: Alert) {
        self.alert = Some(a);
        self.notify = a != Alert::Off;
        self.quiet = a == Alert::Off;
    }

    pub fn final_path(&self) -> PathBuf {
        if self.vault {
            return crate::vault::get().file_path(&self.id);
        }
        self.dest_dir.join(&self.name)
    }
    /// The unfinished file while it downloads ("name.pakopart"; in the vault, its own file).
    pub fn part_path(&self) -> PathBuf {
        if self.vault {
            return crate::vault::get().file_path(&self.id);
        }
        crate::names::part_of(&self.dest_dir, &self.name)
    }
    /// The same file as an older Pako named it ("name.zdpart").
    pub fn old_part_path(&self) -> PathBuf {
        crate::names::old_part_of(&self.dest_dir, &self.name)
    }
    /// A download left unfinished by an older Pako: its "name.zdpart" becomes "name.pakopart"
    /// (when both are there, the one changed last is kept), so it continues from the bytes it has.
    pub fn adopt_old_part(&self) {
        if self.vault {
            return;
        }
        crate::names::adopt_old(&self.old_part_path(), &self.part_path());
    }
    /// Is its unfinished file on disk (under either name)?
    pub fn has_part(&self) -> bool {
        if self.vault {
            return self.part_path().exists();
        }
        crate::names::part_exists(&self.dest_dir, &self.name)
    }
    /// Deletes its unfinished file, under either name.
    pub fn remove_part(&self) {
        if self.vault {
            crate::vault::blocks::remove(&self.part_path());
            return;
        }
        let _ = std::fs::remove_file(self.part_path());
        let _ = std::fs::remove_file(self.old_part_path());
    }
}

/// A link in the Found list: what it is, before you pick it and press Download.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FoundState {
    /// Asking the server for the name and size.
    Checking,
    /// A link page (ad-gate shortener) Pako is working through to reach the real file.
    Opening,
    Ready,
    /// The server says the file isn't there (or refuses it).
    Broken,
    /// A Drive file or folder Google only shows to people it's shared with (opened in the
    /// browser instead).
    NeedsAccount,
    /// A video, but the video tools (yt-dlp, Deno, ffmpeg) aren't installed.
    NeedsTools,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundItem {
    pub id: String,
    /// The download it becomes (its folder is set when you press Download).
    pub template: Download,
    pub state: FoundState,
    /// Plain-language reason for Broken / NeedsAccount, or a note (Google's daily limit).
    pub message: Option<String>,
    /// The Drive folder it came from, and the subfolder inside it.
    pub folder: Option<String>,
    pub rel_dir: Option<PathBuf>,
    /// A Drive folder Google won't list for Pako (opened in the browser instead).
    pub is_folder: bool,
    /// Its own save folder (set in the details panel); None = the Found tab's Save to.
    #[serde(default)]
    pub dest: Option<PathBuf>,
    /// Split archive: extract after the last part. None = the default from Settings.
    #[serde(default)]
    pub extract: Option<bool>,
    /// From Pako Light: never asked to yt-dlp, never turned into a video.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_video: bool,
    /// The host the user picked for this file when the same file is on several hosts.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub host_pick: bool,
    /// "Download on": the paired PC (its Pako ID) this link is sent to when you press
    /// Download. None = this PC (left out, so older lists load as they were).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Why it wasn't sent to its PC (offline, refused, no longer allowed), shown on the row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_note: Option<String>,
}

impl FoundItem {
    pub fn new(template: Download, state: FoundState, message: Option<String>, is_folder: bool) -> Self {
        Self {
            id: template.id.clone(),
            template,
            state,
            message,
            folder: None,
            rel_dir: None,
            is_folder,
            dest: None,
            extract: None,
            no_video: false,
            host_pick: false,
            device: None,
            device_note: None,
        }
    }
    /// Ready, or still being checked (the download checks again anyway).
    pub fn can_download(&self) -> bool {
        !self.is_folder && matches!(self.state, FoundState::Ready | FoundState::Checking)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub download_dir: PathBuf,
    /// Auto: the app finds the best number of connections per file by itself (default).
    pub auto_connections: bool,
    /// Connections per file when Auto is off.
    pub connections: u32,
    /// Files downloading at the same time.
    pub max_active: u32,
    /// Total speed limit in KB/s. 0 = no limit.
    pub speed_limit_kb: u64,
    /// Check Drive files against Google's MD5 checksum when finished.
    pub verify_checksum: bool,
    /// The old "Download finished › Pop-up" switch: read only to pick `done_alert`'s starting
    /// value for settings saved before it (off: new downloads start at Off).
    pub notify_done: bool,
    /// Settings › Notifications › "New downloads": Off · Pop-up · Important. None = from
    /// `notify_done` (see `done_alert`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_alert: Option<Alert>,
    /// "Chime when a download finishes". None = from `play_done_sound` (see `done_chime`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_chime: Option<DoneChime>,
    /// The older Download with Pako window's remembered "Notify me" (read only, to start
    /// `ask_off`).
    #[serde(skip_serializing)]
    pub ask_notify: Option<bool>,
    /// That window had "Notify me" off: its downloads start at Off, until a level is picked in
    /// the window or New downloads changes in Settings.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ask_off: bool,
    /// Settings › Notifications: each kind's Pop-up and Bell, where changed from the defaults
    /// (see `notices::pref`).
    pub notices: HashMap<String, NoticePref>,
    /// Do not disturb: no pop-ups (the bell keeps collecting).
    pub quiet: bool,
    /// No pop-ups while Windows says not now (full-screen game or video, presenting, Windows'
    /// own Do not disturb).
    pub follow_windows_quiet: bool,
    /// "Play a sound when files arrive": the arrived chime when files arrive in Direct or My
    /// devices, or someone offers you files (not during Do not disturb, nor while Windows says
    /// not now when that is followed). Finished downloads have their own choice (`done_chime`).
    /// The key is the one from before Direct had sounds (`playDoneSound`).
    pub play_done_sound: bool,
    /// "Play a sound when files are sent": the sent sound when an upload finishes or a Direct
    /// or My devices send is delivered (the same quiet times).
    pub play_sent_sound: bool,
    /// Split archives: extract when every part is downloaded (each set can change it).
    pub extract_default: bool,
    /// Minimize hides Pako in the taskbar tray (it keeps downloading).
    pub minimize_to_tray: bool,
    /// Closing the window hides Pako in the tray instead of quitting.
    pub close_to_tray: bool,
    /// The first-start question about the tray was answered.
    pub tray_asked: bool,
    /// Video preview height in lines (360, 480 or 720).
    pub preview_quality: u32,
    /// Windows doesn't go to sleep while downloads run.
    pub keep_awake: bool,
    /// Quitting while downloads run asks first.
    pub confirm_quit: bool,
    /// Downloads that were running when Pako closed continue when it starts again (default).
    /// Off: they're paused, and wait for Resume.
    pub resume_on_start: bool,
    /// When a finished private download ("Done · private") leaves the list by itself: "10m"
    /// (default, 10 minutes after it finished), "1h", or "close" (when Pako closes). Opening it
    /// from Pako, or its ✕, takes it off at once. Only the choice is saved, never the rows.
    pub private_keep: String,
    /// Private mode locks itself after this long with no keyboard or mouse anywhere on the PC:
    /// "1m", "5m" (default), "15m", "30m" or "never".
    pub private_lock_after: String,
    /// Private mode's list clears itself (rows and their files) this long after a download
    /// finished: "never" (default), "3h", "12h", "3d", or a number of hours ("36h").
    pub private_clear_after: String,
    /// "Show real names when opening private files" (off: PrivateFile_<date>_<time>.ext).
    pub private_real_names: bool,
    /// The one-time tip about Private mode on the first "Done · private" row was shown.
    pub private_tip_seen: bool,
    /// Pako starts when you sign in to Windows.
    pub start_with_windows: bool,
    /// ...straight to the tray (no window).
    pub start_in_tray: bool,
    /// The browser extension can reach Pako (registered with the browsers, per user).
    pub browser_connect: bool,
    /// Pako Setup's window chose Start with Windows and the browser connection (installed copy):
    /// first-start setup shows those choices as they are.
    pub installer_choices: bool,
    /// The first-start setup was done (or skipped).
    pub setup_done: bool,
    /// Video downloading was offered (asked once when it arrives).
    pub video_asked: bool,
    /// Pako and its tools update themselves (checked once a day when Pako starts).
    pub auto_update: bool,
    /// Only download between these times (minutes after midnight, local time).
    pub schedule_on: bool,
    pub schedule_start: u32,
    pub schedule_end: u32,
    /// Links copied anywhere go to Links by themselves.
    pub catch_clipboard: bool,
    /// Video quality picked by default (height in lines; 0 = audio only).
    pub video_quality: u32,
    /// Send (older setting, read only to carry it over): "off", "auto" or "max".
    pub send_compression: String,
    /// Send, several files: "files" (as they are, default), "zip", "compress" or "parts" (one ZIP
    /// cut into numbered parts). None = not chosen yet (then the older `send_compression` decides).
    pub send_mode: Option<String>,
    /// "ZIP in parts": the size of each part, in MB (500 by default; 50 MB to 10 GB).
    pub send_part_mb: u32,
    /// The part size was typed in (Custom), so 1024 shows as Custom, not as "1 GB".
    pub send_part_custom: bool,
    /// An upload in parts that stopped (Pako closed, the PC restarted) carries on by itself when
    /// Pako starts. Off: it waits for "Try again".
    pub resume_uploads_on_start: bool,
    /// Downloads from the browser open the "Download with Pako" window first.
    pub ask_browser: bool,
    /// The small window (the tray's list) opens by itself when a browser download starts.
    pub mini_on_browser: bool,
    /// The small window stays open and on top (doesn't close when you click elsewhere).
    pub mini_pinned: bool,
    /// Where the pinned small window was moved to (screen pixels); None = next to the tray.
    pub mini_pos: Option<(i32, i32)>,
    /// Where Send uploads to: "gofile" (default, no account) or "drive" (your Google Drive).
    pub send_host: String,
    /// "Delete after" in the New upload basket (days; 0 = keep). The last choice is remembered.
    pub upload_delete_days: u32,
    /// The last Pako version whose "What's new" was shown (None = never: shown once after an update).
    pub whats_new_seen: Option<String>,
    /// "Take a quick tour?" was offered once (after first-start setup).
    pub tour_offered: bool,
    /// The page tutorials gone through once (their Tutorial button stops calling for attention).
    pub tours_done: Vec<String>,
    /// Light or dark: "dark" (default), "light" or "system" (follow Windows' app mode). Changed
    /// only through `theme::theme_set` (Settings › Window › Theme and the title bar's button).
    pub theme: String,
    /// Pako's language: "en" (default) or "fr". Asked, never guessed (Pako Setup, a portable
    /// Pako's first-start setup, Settings › General › Language). Changed only through
    /// `i18n::language_set`, so an older copy of the settings can't put the previous one back.
    pub language: String,
    /// Direct › "Be visible to Pakos on this network": "home" (default: only while every network
    /// this PC is on is one Windows marks as Private or Domain), "always" or "never". Pako
    /// always looks for its contacts on the network; this only decides whether it answers.
    pub direct_visible: String,
    /// Same file on several hosts: the hosts picked in Links, most recent first (the next
    /// group picks the first of them that is ready).
    pub mirror_hosts: Vec<String>,
    /// The date of the terms of use accepted ("2026-11-18", TERMS.md's "Last updated"). None =
    /// not accepted yet (settings from before Pako had terms too): Pako asks before anything
    /// runs (see consent.rs). Once set, never asked again. Not written while unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_accepted: Option<String>,
}

impl Settings {
    /// "New downloads": as chosen, else from the old Download finished pop-up switch.
    pub fn done_alert(&self) -> Alert {
        self.done_alert.unwrap_or(if self.notify_done { Alert::Pop } else { Alert::Off })
    }

    /// "Chime when a download finishes": as chosen, else from the old arrived-sound switch
    /// (which used to cover finished downloads too).
    pub fn done_chime(&self) -> DoneChime {
        self.done_chime.unwrap_or(if self.play_done_sound { DoneChime::All } else { DoneChime::Never })
    }

    /// Settles the finished-download choices once settings are read and before each save: New
    /// downloads and the chime are pinned from the older switches the first time (so "Play a
    /// sound when files arrive" changes only Direct's sound from then on), the old window's
    /// "Notify me" off becomes `ask_off`, and the old Download finished switch is kept in step
    /// for an older Pako.
    pub fn settle_notify(&mut self) {
        if self.done_alert.is_none() && self.ask_notify == Some(false) {
            self.ask_off = true;
        }
        self.done_alert = Some(self.done_alert());
        self.done_chime = Some(self.done_chime());
        self.notify_done = self.done_alert() != Alert::Off;
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: crate::paths::default_download_dir(),
            auto_connections: true,
            connections: 8,
            max_active: 3,
            speed_limit_kb: 0,
            verify_checksum: true,
            notify_done: true,
            done_alert: None,
            done_chime: None,
            ask_notify: None,
            ask_off: false,
            notices: HashMap::new(),
            quiet: false,
            follow_windows_quiet: true,
            play_done_sound: true,
            play_sent_sound: true,
            extract_default: false,
            minimize_to_tray: false,
            close_to_tray: false,
            tray_asked: false,
            preview_quality: 480,
            keep_awake: true,
            confirm_quit: true,
            resume_on_start: true,
            private_keep: "10m".into(),
            private_lock_after: "5m".into(),
            private_clear_after: "never".into(),
            private_real_names: false,
            private_tip_seen: false,
            start_with_windows: false,
            start_in_tray: true,
            browser_connect: false,
            installer_choices: false,
            setup_done: false,
            video_asked: false,
            auto_update: true,
            schedule_on: false,
            schedule_start: 22 * 60,
            schedule_end: 7 * 60,
            catch_clipboard: false,
            video_quality: 1080,
            send_compression: "off".into(),
            send_mode: None,
            send_part_mb: crate::send::PART_DEFAULT_MB,
            send_part_custom: false,
            resume_uploads_on_start: true,
            ask_browser: true,
            mini_on_browser: true,
            mini_pinned: false,
            mini_pos: None,
            send_host: "gofile".into(),
            upload_delete_days: crate::send::DELETE_DEFAULT,
            whats_new_seen: None,
            tour_offered: false,
            tours_done: Vec::new(),
            theme: "dark".into(),
            language: "en".into(),
            direct_visible: "home".into(),
            mirror_hosts: Vec::new(),
            terms_accepted: None,
        }
    }
}

/// One kind of notification changed from its defaults in Settings › Notifications (a missing
/// switch keeps the default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticePref {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popup: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bell: Option<bool>,
}

/// Live numbers sent to the window twice a second.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub status: Status,
    pub message: Option<String>,
    pub downloaded: u64,
    pub size: Option<u64>,
    /// Bytes per second.
    pub speed: u64,
    pub connections: u32,
    pub name: String,
}

/// Words Pako recognises again later (a message compared to know what happened, such as a
/// download's reason for stopping): written in English and French, so a message made before a
/// language switch is still recognised. Shown with `say`, compared with `is_said`/`starts_said`.
pub type Said = [&'static str; 2];

/// The words in the language of the moment.
pub fn say(w: Said) -> String {
    w[usize::from(crate::i18n::is_fr())].to_string()
}

/// `msg` is these words, in either language.
pub fn is_said(msg: &str, w: Said) -> bool {
    w.contains(&msg)
}

/// `msg` starts with these words, in either language.
pub fn starts_said(msg: &str, w: Said) -> bool {
    w.iter().any(|p| msg.starts_with(p))
}

/// Why a link isn't added (a paste, or a link sent by another of your PCs).
pub const NOT_WEB_LINK: Said = ["not a web link", "ce n’est pas un lien web"];
pub const ONLY_HTTP: Said = ["only http and https links are supported", "seuls les liens http et https sont pris en charge"];
pub const FOLDER_EMPTY: Said = ["the folder is empty", "le dossier est vide"];
pub const GOOGLE_KIND: Said = [
    "this kind of Google file (form, site...) can't be downloaded",
    "ce type de fichier Google (formulaire, site…) ne peut pas être téléchargé",
];

/// The words that may travel to another of your PCs (why a link it sent wasn't added here, a
/// download's message in its list): each PC sends them in English (`in_english`) and shows
/// them in its own language (`direct::words`), like Direct's own lines.
const TRAVELLING: &[Said] = &[
    NOT_WEB_LINK,
    ONLY_HTTP,
    FOLDER_EMPTY,
    GOOGLE_KIND,
    crate::engine::NEEDS_VIDEO_TOOLS,
    crate::engine::NO_ANSWER,
    crate::engine::SPLICE_FAILED,
    [crate::drive::FOLDER_NOT_PUBLIC, crate::drive::FOLDER_NOT_PUBLIC_FR],
    [crate::drive::CANT_LIST_NOW, crate::drive::CANT_LIST_NOW_FR],
    [crate::drive::NO_KEY, crate::drive::NO_KEY_FR],
    crate::engine::FILE_CHANGED,
    crate::engine::WEB_PAGE,
    crate::engine::RESUME_STOPPED,
    crate::engine::NEEDS_PASSWORD,
    crate::engine::EXTRACTED,
    crate::fetch::BLOCKED_MESSAGE,
    crate::fetch::QUOTA_MESSAGE,
    crate::fetch::NOT_FOUND,
    crate::fetch::REFUSED,
    crate::fetch::ANSWERED,
    crate::fetch::NAME_NOT_FOUND,
    crate::video::NEEDS_TOOLS,
    crate::video::SITE_CHANGED,
    crate::video::DRM,
    crate::video::UNREADABLE,
    crate::video::NO_SITE,
];

/// Travelling words that START a message whose rest is only a code ("File not found on the
/// server (404).", "Server error (502)"), or a code and one of `TAILS`.
const TRAVELLING_STARTS: &[Said] = &[
    crate::fetch::NOT_FOUND,
    crate::fetch::REFUSED,
    crate::fetch::ANSWERED,
    ["Server error", "Erreur du serveur"],
    ["Server busy", "Serveur occupé"],
];

/// Endings that follow a code in those messages (fetch.rs).
const TAILS: &[Said] = &[[". The link may have expired.", ". Le lien a peut-être expiré."]];

/// `msg` with its travelling start (and ending) said in the other language: `from`/`to` are 0
/// (English) or 1 (French). None when it isn't such a message, or what's left isn't a code.
fn swap_start(msg: &str, from: usize, to: usize) -> Option<String> {
    for w in TRAVELLING_STARTS {
        let Some(rest) = msg.strip_prefix(w[from]) else { continue };
        let (code, tail) = match TAILS.iter().find(|t| rest.ends_with(t[from])) {
            Some(t) => (&rest[..rest.len() - t[from].len()], t[to]),
            None => (rest, ""),
        };
        // Only a code is left ("(404).", " 418", " (403)"): numbers, brackets, spaces, a full stop.
        if code.chars().all(|c| c.is_ascii_digit() || " ().\u{a0}".contains(c)) {
            return Some(format!("{}{code}{tail}", w[to]));
        }
    }
    None
}

/// One of the travelling words in English (as it is when it's none of them, or English already).
pub fn in_english(msg: &str) -> String {
    if let Some(w) = TRAVELLING.iter().find(|w| w[1] == msg) {
        return w[0].to_string();
    }
    swap_start(msg, 1, 0).unwrap_or_else(|| msg.to_string())
}

/// The French of one of the travelling words given in English.
pub fn french_of(msg: &str) -> Option<String> {
    if let Some(w) = TRAVELLING.iter().find(|w| w[0] == msg) {
        return Some(w[1].to_string());
    }
    swap_start(msg, 0, 1)
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// What the "Download with Pako" window shows for one download.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskView {
    pub id: String,
    /// The name without its extension (that part can be edited).
    pub stem: String,
    /// "zip", "mp4"... (stays as it is; a video's follows its quality).
    pub ext: String,
    pub site: String,
    pub size: Option<u64>,
    pub dir: String,
    pub video: bool,
    /// Videos: the picked height (0 = audio only).
    pub quality: u32,
    /// Videos: each offered quality and its likely size.
    pub sizes: Vec<(u32, Option<u64>)>,
    /// "1 of 3" when several downloads wait.
    pub position: usize,
    pub total: usize,
    /// Downloads running now (then the window offers "Download now" and "Add to queue").
    pub running: usize,
    /// Off · Pop-up · Important, starting at Settings' "New downloads".
    pub alert: Alert,
    /// A video caught on a page or a recording: only this PC can make it (no "Download on").
    pub here_only: bool,
}

/// The window's answer.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskAnswer {
    pub id: String,
    /// "now" (start now), "queue" (add to the queue) or "cancel".
    pub action: String,
    #[serde(default)]
    pub stem: String,
    #[serde(default)]
    pub dir: String,
    #[serde(default)]
    pub quality: Option<u32>,
    #[serde(default)]
    pub alert: Option<Alert>,
    #[serde(default)]
    pub dont_ask: bool,
    /// "Download privately": straight into Private mode (only while it's on).
    #[serde(default)]
    pub private: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Settings from before the terms of use load (nothing accepted yet) and write nothing new
    /// for it; an accepted date survives a save.
    #[test]
    fn old_settings_load_without_terms() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"connections":4,"whatsNewSeen":"1.0.1"}"#).expect("loads");
        assert!(s.terms_accepted.is_none() && s.setup_done && s.connections == 4);
        assert!(!serde_json::to_string(&s).unwrap().contains("termsAccepted"));
        let s = Settings { terms_accepted: Some("2026-11-18".into()), ..s };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.terms_accepted.as_deref(), Some("2026-11-18"));
    }

    /// Lists and settings saved before mirrors existed still load (no backups, no host picked,
    /// no preferred hosts), and nothing new is written for them.
    #[test]
    fn old_lists_load_without_mirrors() {
        let old = r#"{"id":"a","link":"https://a.example/x.zip","source":{"kind":"http","url":"https://a.example/x.zip"},"name":"x.zip","destDir":"C:/D","size":10,"md5":null,"status":"queued","message":null,"resumable":true,"segments":[],"downloaded":0,"addedAt":1,"finishedAt":null,"group":null,"resolved":false}"#;
        let d: Download = serde_json::from_str(old).expect("old entry loads");
        assert!(d.mirrors.is_empty() && d.tried.is_empty());
        let json = serde_json::to_string(&d).unwrap();
        assert!(!d.spot_check && !d.spliced);
        assert!(!json.contains("mirrors") && !json.contains("tried"), "{json}");
        assert!(!json.contains("spotCheck") && !json.contains("spliced"), "{json}");
        let f: FoundItem = serde_json::from_str(&format!(r#"{{"id":"a","template":{old},"state":"ready","message":null,"folder":null,"relDir":null,"isFolder":false}}"#)).expect("old Links item loads");
        assert!(!f.host_pick);
        let s: Settings = serde_json::from_str(r#"{"setupDone":true}"#).expect("loads");
        assert!(s.mirror_hosts.is_empty());
        // A backup survives a save.
        let mut d = d;
        d.mirrors.push(Mirror { link: "https://b.example/x.zip".into(), source: Source::Http { url: "https://b.example/x.zip".into() }, size: Some(10), md5: None, validator: Some("\"e1\"".into()), resumable: true, why: None });
        let back: Download = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(back.mirrors, d.mirrors);
    }

    /// Videos saved by older versions (no referrer, not caught, no recording) still load, and
    /// a caught stream keeps its page and recording across a save.
    #[test]
    fn video_sources_old_and_caught() {
        let old: Source = serde_json::from_str(r#"{"kind":"video","url":"https://e.example/v","quality":720}"#).unwrap();
        let Source::Video { referrer, caught, recording, .. } = &old else { panic!() };
        assert_eq!((referrer, caught, recording), (&None, &false, &None));
        // Nothing new is written for them either.
        let json = serde_json::to_string(&old).unwrap();
        assert!(!json.contains("referrer") && !json.contains("caught") && !json.contains("recording"));
        let caught = Source::Video {
            url: "https://cdn.example/m.m3u8".into(),
            quality: None,
            audio: None,
            options: Vec::new(),
            audio_size: None,
            duration: None,
            streams: Vec::new(),
            referrer: Some("https://page.example/".into()),
            caught: true,
            recording: Some(PathBuf::from(r"C:\t\rec-1")),
        };
        let back: Source = serde_json::from_str(&serde_json::to_string(&caught).unwrap()).unwrap();
        assert_eq!(back, caught);
    }

    /// Lists saved before downloads stopped signing in to Google still load: their Drive
    /// entries may carry `exportMime` and `copyId` (the old temporary copy), which are skipped.
    #[test]
    fn old_drive_entries_still_load() {
        let old = r#"{"id":"a","link":"https://drive.google.com/file/d/ID1234567890/view","source":{"kind":"gDrive","fileId":"ID1234567890","resourceKey":null,"exportMime":null,"copyId":"COPY123456789"},"name":"x.zip","destDir":"C:/D","size":10,"md5":null,"status":"quotaExceeded","message":null,"resumable":true,"segments":[],"downloaded":0,"addedAt":1,"finishedAt":null,"group":null,"resolved":true}"#;
        let d: Download = serde_json::from_str(old).expect("old entry loads");
        assert_eq!(d.status, Status::QuotaExceeded);
        match &d.source {
            Source::GDrive { file_id, resource_key, doc_type } => {
                assert_eq!(file_id, "ID1234567890");
                assert!(resource_key.is_none() && doc_type.is_none());
            }
            other => panic!("not a Drive entry: {other:?}"),
        }
        // Saved again without the old fields.
        let again = serde_json::to_string(&d.source).unwrap();
        assert!(!again.contains("copyId") && !again.contains("exportMime"), "{again}");
    }

    /// Settings saved before "ZIP in parts" existed load as they were: the same way of sending
    /// several files, parts of 500 MB if it's picked later; a picked part size survives a save.
    #[test]
    fn old_settings_get_part_size_default() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"sendMode":"zip","sendCompression":"off"}"#).expect("loads");
        assert_eq!(s.send_mode.as_deref(), Some("zip"));
        assert_eq!((s.send_part_mb, s.send_part_custom), (500, false));
        assert_eq!(crate::send::Mode::from_settings(s.send_mode.as_deref(), &s.send_compression), crate::send::Mode::Zip);
        let s = Settings { send_mode: Some("parts".into()), send_part_mb: 1024, send_part_custom: true, ..s };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!((back.send_mode.as_deref(), back.send_part_mb, back.send_part_custom), (Some("parts"), 1024, true));
        assert_eq!(crate::send::Mode::from_settings(back.send_mode.as_deref(), ""), crate::send::Mode::Parts);
    }

    /// Settings saved before "Continue unfinished uploads when Pako starts" existed: it's on;
    /// turned off, it stays off.
    #[test]
    fn old_settings_continue_uploads_at_start() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"resumeOnStart":false}"#).expect("loads");
        assert!(s.resume_uploads_on_start && !s.resume_on_start);
        let off: Settings = serde_json::from_str(r#"{"resumeUploadsOnStart":false}"#).expect("loads");
        assert!(!off.resume_uploads_on_start);
    }

    /// Settings saved before "Continue downloads when Pako starts" existed: it's on.
    #[test]
    fn old_settings_continue_downloads_at_start() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true}"#).expect("loads");
        assert!(s.resume_on_start);
        let off: Settings = serde_json::from_str(r#"{"resumeOnStart":false}"#).expect("loads");
        assert!(!off.resume_on_start);
    }

    /// Settings saved before Help existed still load: What's new is shown once (never seen),
    /// and the quick tour wasn't offered yet.
    #[test]
    fn old_settings_get_help_defaults() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"connections":4}"#).expect("loads");
        assert!(s.setup_done && s.connections == 4);
        assert_eq!(s.whats_new_seen, None);
        assert!(!s.tour_offered);
        assert!(s.tours_done.is_empty());
        let again = serde_json::to_string(&Settings { whats_new_seen: Some("0.2.0".into()), ..s }).unwrap();
        assert!(again.contains(r#""whatsNewSeen":"0.2.0""#) && again.contains(r#""tourOffered":false"#), "{again}");
    }

    /// Settings saved before the light look existed still load, dark; the choice is saved.
    #[test]
    fn old_settings_get_theme_default() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"connections":4}"#).expect("loads");
        assert_eq!(s.theme, "dark");
        let again = serde_json::to_string(&Settings { theme: "system".into(), ..s }).unwrap();
        assert!(again.contains(r#""theme":"system""#), "{again}");
        let back: Settings = serde_json::from_str(&again).expect("loads");
        assert_eq!(back.theme, "system");
        assert_eq!(crate::theme::choice("sepia"), "dark");
    }

    /// Settings saved before Settings › Notifications existed still load: no changed kinds,
    /// Do not disturb off, Windows' Do not disturb followed, and the old "Notify when a download
    /// finishes" kept (it is the Download finished pop-up now).
    #[test]
    fn old_settings_get_notice_defaults() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"notifyDone":false}"#).expect("loads");
        assert!(s.notices.is_empty());
        assert!(!s.quiet && s.follow_windows_quiet && !s.notify_done);
        // A kind with only one switch changed keeps the other at its default.
        let s: Settings = serde_json::from_str(r#"{"notices":{"browser":{"popup":false}}}"#).expect("loads");
        assert_eq!(s.notices["browser"], NoticePref { popup: Some(false), bell: None });
        let again = serde_json::to_string(&s).unwrap();
        assert!(again.contains(r#""notices":{"browser":{"popup":false}}"#) && again.contains(r#""followWindowsQuiet":true"#), "{again}");
    }

    /// Settings saved before the finished chime existed load with it on; off is kept.
    #[test]
    fn old_settings_play_the_done_sound() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"notifyDone":true}"#).expect("loads");
        assert!(s.play_done_sound);
        let off = serde_json::to_string(&Settings { play_done_sound: false, ..s }).unwrap();
        assert!(off.contains(r#""playDoneSound":false"#), "{off}");
        assert!(!serde_json::from_str::<Settings>(&off).expect("loads").play_done_sound);
    }

    /// Settings saved before the sent sound existed load with it on (and the arrived one as
    /// saved); off is kept, each switch on its own.
    #[test]
    fn old_settings_play_the_sent_sound() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"playDoneSound":false}"#).expect("loads");
        assert!(s.play_sent_sound && !s.play_done_sound);
        let again = serde_json::to_string(&Settings { play_sent_sound: false, play_done_sound: true, ..s }).unwrap();
        assert!(again.contains(r#""playSentSound":false"#) && again.contains(r#""playDoneSound":true"#), "{again}");
        let back: Settings = serde_json::from_str(&again).expect("loads");
        assert!(!back.play_sent_sound && back.play_done_sound);
    }

    /// Settings saved before "Remove private downloads from the list" existed: 10 minutes; a
    /// choice is kept.
    #[test]
    fn old_settings_keep_private_rows_ten_minutes() {
        let s: Settings = serde_json::from_str(r#"{"setupDone":true}"#).expect("loads");
        assert_eq!(s.private_keep, "10m");
        let again = serde_json::to_string(&Settings { private_keep: "close".into(), ..s }).unwrap();
        assert!(again.contains(r#""privateKeep":"close""#), "{again}");
        assert_eq!(serde_json::from_str::<Settings>(&again).expect("loads").private_keep, "close");
    }

    /// Off · Pop-up · Important: older lists and settings carry over. A row's old "Notify when
    /// done" is Pop-up, the Download with Pako window's "Notify me" off is Off, any other row
    /// follows the default; the old Download finished pop-up switch off makes the default Off
    /// (nobody gets pop-ups and chimes they had turned off), the old arrived sound off makes
    /// finished downloads silent.
    #[test]
    fn notify_levels_carry_over_from_older_files() {
        let old = r#"{"id":"a","link":"https://a.example/x.zip","source":{"kind":"http","url":"https://a.example/x.zip"},"name":"x.zip","destDir":"C:/D","size":10,"md5":null,"status":"queued","message":null,"resumable":true,"segments":[],"downloaded":0,"addedAt":1,"finishedAt":null,"group":null,"resolved":false,"notify":true}"#;
        let d: Download = serde_json::from_str(old).expect("loads");
        assert_eq!(d.alert, None);
        assert_eq!(d.alert_of(Alert::Off), Alert::Pop, "Notify when done = Pop-up");
        let quiet = Download { notify: false, quiet: true, ..d.clone() };
        assert_eq!(quiet.alert_of(Alert::Important), Alert::Off, "Notify me off = Off");
        let plain = Download { notify: false, ..d.clone() };
        assert_eq!(plain.alert_of(Alert::Important), Alert::Important, "the default");
        // A choice wins, and is what's saved, with the old flags in step for an older Pako.
        let mut chosen = d.clone();
        chosen.set_alert(Alert::Important);
        assert_eq!(chosen.alert_of(Alert::Off), Alert::Important);
        let saved = serde_json::to_value(&chosen).unwrap();
        assert_eq!(saved["alert"], "important");
        assert!(saved["notify"] == true && saved.get("quiet").is_none(), "{saved}");
        chosen.set_alert(Alert::Off);
        let saved = serde_json::to_value(&chosen).unwrap();
        assert!(saved.get("notify").is_none() && saved["quiet"] == true, "{saved}");
        chosen.set_alert(Alert::Pop);
        assert_eq!(serde_json::to_value(&chosen).unwrap()["notify"], true);
        assert!(serde_json::to_value(&plain).unwrap().get("alert").is_none(), "no choice: nothing written");

        // Settings.
        let s: Settings = serde_json::from_str(r#"{"setupDone":true}"#).expect("loads");
        assert_eq!((s.done_alert(), s.done_chime()), (Alert::Pop, DoneChime::All), "new: Pop-up, chime with both");
        let s: Settings = serde_json::from_str(r#"{"setupDone":true,"notifyDone":false,"playDoneSound":false}"#).expect("loads");
        assert_eq!((s.done_alert(), s.done_chime()), (Alert::Off, DoneChime::Never));
        let s: Settings = serde_json::from_str(r#"{"notifyDone":false,"doneAlert":"important","doneChime":"important"}"#).expect("loads");
        assert_eq!((s.done_alert(), s.done_chime()), (Alert::Important, DoneChime::Important), "a choice wins");
        let again = serde_json::to_string(&s).unwrap();
        assert!(again.contains(r#""doneAlert":"important""#) && again.contains(r#""doneChime":"important""#), "{again}");
        assert!(!serde_json::to_string(&Settings::default()).unwrap().contains("doneAlert"), "not chosen: not written");
        // Ordered: a folder says it as loud as its loudest file.
        assert!(Alert::Off < Alert::Pop && Alert::Pop < Alert::Important);
    }

    /// Settled once read (and before each save): New downloads and the chime are pinned from the
    /// older switches, so turning "Play a sound when files arrive" off later only quiets Direct;
    /// the old window's "Notify me" off starts its downloads at Off; the old Download finished
    /// switch follows New downloads for an older Pako.
    #[test]
    fn notify_choices_settle_once() {
        let mut s: Settings = serde_json::from_str(r#"{"setupDone":true,"playDoneSound":true,"askNotify":false}"#).expect("loads");
        s.settle_notify();
        assert_eq!((s.done_alert, s.done_chime), (Some(Alert::Pop), Some(DoneChime::All)));
        assert!(s.ask_off, "the window's Notify me was off");
        s.play_done_sound = false;
        s.settle_notify();
        assert_eq!(s.done_chime(), DoneChime::All, "Direct's sound switch no longer moves it");
        s.done_alert = Some(Alert::Off);
        s.settle_notify();
        assert!(!s.notify_done, "an older Pako sees Download finished off");
        let again = serde_json::to_string(&s).unwrap();
        assert!(again.contains(r#""askOff":true"#) && !again.contains("askNotify"), "{again}");
        // Already chosen before: the old window value doesn't come back.
        let mut chosen: Settings = serde_json::from_str(r#"{"doneAlert":"pop","askNotify":false}"#).expect("loads");
        chosen.settle_notify();
        assert!(!chosen.ask_off);
    }
}
