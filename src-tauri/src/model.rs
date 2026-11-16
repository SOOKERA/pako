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
    /// Show a Windows notification when this one finishes (even if the global setting is off).
    #[serde(default)]
    pub notify: bool,
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
    /// No notification for this one, even when every finished download is announced
    /// ("Notify me" switched off in the Download with Pako window).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quiet: bool,
}

impl Download {
    pub fn final_path(&self) -> PathBuf {
        self.dest_dir.join(&self.name)
    }
    /// The unfinished file while it downloads ("name.pakopart").
    pub fn part_path(&self) -> PathBuf {
        crate::names::part_of(&self.dest_dir, &self.name)
    }
    /// The same file as an older Pako named it ("name.zdpart").
    pub fn old_part_path(&self) -> PathBuf {
        crate::names::old_part_of(&self.dest_dir, &self.name)
    }
    /// A download left unfinished by an older Pako: its "name.zdpart" becomes "name.pakopart"
    /// (when both are there, the one changed last is kept), so it continues from the bytes it has.
    pub fn adopt_old_part(&self) {
        crate::names::adopt_old(&self.old_part_path(), &self.part_path());
    }
    /// Is its unfinished file on disk (under either name)?
    pub fn has_part(&self) -> bool {
        crate::names::part_exists(&self.dest_dir, &self.name)
    }
    /// Deletes its unfinished file, under either name.
    pub fn remove_part(&self) {
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
}

impl FoundItem {
    pub fn new(template: Download, state: FoundState, message: Option<String>, is_folder: bool) -> Self {
        Self { id: template.id.clone(), template, state, message, folder: None, rel_dir: None, is_folder, dest: None, extract: None, no_video: false }
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
    /// A pop-up when any download finishes (each download can also ask for one). It is the
    /// "Download finished" kind's Pop-up switch in Settings › Notifications.
    pub notify_done: bool,
    /// Settings › Notifications: each kind's Pop-up and Bell, where changed from the defaults
    /// (see `notices::pref`).
    pub notices: HashMap<String, NoticePref>,
    /// Do not disturb: no pop-ups (the bell keeps collecting).
    pub quiet: bool,
    /// No pop-ups while Windows says not now (full-screen game or video, presenting, Windows'
    /// own Do not disturb).
    pub follow_windows_quiet: bool,
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
    /// Pako starts when you sign in to Windows.
    pub start_with_windows: bool,
    /// ...straight to the tray (no window).
    pub start_in_tray: bool,
    /// The browser extension can reach Pako (registered with the browsers, per user).
    pub browser_connect: bool,
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
    /// Send, several files: "files" (as they are, default), "zip" or "compress". None = not
    /// chosen yet (then the older `send_compression` decides).
    pub send_mode: Option<String>,
    /// Downloads from the browser open the "Download with Pako" window first.
    pub ask_browser: bool,
    /// That window's "Notify me when it's done" (remembered).
    pub ask_notify: bool,
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
            notices: HashMap::new(),
            quiet: false,
            follow_windows_quiet: true,
            extract_default: false,
            minimize_to_tray: false,
            close_to_tray: false,
            tray_asked: false,
            preview_quality: 480,
            keep_awake: true,
            confirm_quit: true,
            start_with_windows: false,
            start_in_tray: true,
            browser_connect: false,
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
            ask_browser: true,
            ask_notify: true,
            mini_on_browser: true,
            mini_pinned: false,
            mini_pos: None,
            send_host: "gofile".into(),
            upload_delete_days: crate::send::DELETE_DEFAULT,
            whats_new_seen: None,
            tour_offered: false,
            tours_done: Vec::new(),
            theme: "dark".into(),
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
    /// "Notify me when it's done", as last chosen.
    pub notify: bool,
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
    pub notify: bool,
    #[serde(default)]
    pub dont_ask: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
