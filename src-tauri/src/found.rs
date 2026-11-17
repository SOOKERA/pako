// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! The Found list: links waiting to be picked before they download.
//!
//! Pasted links land here first. Each one is checked in the background (name, size, whether
//! the link works); Drive folders are opened into their files. Nothing downloads until you
//! press Download (or Download first), which turns the picked items into downloads.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use tokio::sync::Semaphore;

use super::{AddResult, Engine, drive_download, extract_links, new_download, short};
use crate::drive::{self, DriveApi, LinkKind};
use crate::fetch::{FetchError, Fetcher, probe_info};
use crate::model::*;
use crate::names;
use crate::paths::data_dir;

/// Links checked at the same time (a pasted page with 200 links doesn't open 200 connections).
static CHECKS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(4));
/// Video checks run yt-dlp (a program of its own): two at a time.
static VIDEO_CHECKS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));

/// What the browser hears when the video tools aren't there yet.
const NEEDS_VIDEO_TOOLS: &str =
    "Pako needs its video tools first. In Pako: Settings › Features & updates › Download videos from YouTube and other sites.";

/// A video row in Links, still to be checked.
fn video_item(link: &str, title: String, duration: Option<f64>) -> FoundItem {
    let src = Source::Video { url: link.into(), quality: None, audio: None, options: Vec::new(), audio_size: None, duration, streams: Vec::new(), referrer: None, caught: false, recording: None };
    let t = new_download(link, src, crate::video::file_name(&title, None), PathBuf::new(), None);
    let ready = crate::video::ready();
    let (state, msg) = if ready {
        (FoundState::Checking, crate::video::CHECKING)
    } else {
        (FoundState::NeedsTools, crate::video::NEEDS_TOOLS)
    };
    FoundItem::new(t, state, Some(msg.into()), false)
}

const FILE_NOT_PUBLIC: &str =
    "Google only gives this file to people it's shared with. Open it in your browser and download it there: Pako catches the download.";
const OPENING: &str = "Opening link…";
/// Why a private window's link isn't sent to another of your PCs.
pub const PRIVATE_HERE: &str = "Private downloads stay on this PC.";

fn found_path() -> PathBuf {
    data_dir().join("found.json")
}

/// The Links list, saved in the background like the download list.
static FOUND_SAVER: LazyLock<crate::paths::Saver> = LazyLock::new(|| crate::paths::Saver::new(found_path()));

/// The Found list saved last time: (items, read fine, what to tell). A damaged file is moved
/// aside, an entry that can't be read is left out (the file as it was is kept), and a file that
/// can't be read right now is never saved over this session.
pub(super) fn load() -> (Vec<FoundItem>, bool, Option<(String, String)>) {
    let read = crate::paths::read_list_checked::<FoundItem>(&found_path());
    let note = super::list_note("Links list", &found_path(), &read);
    let ok = read.is_ok();
    (read.map(|r| r.items).unwrap_or_default(), ok, note)
}

/// Waits (at most `limit`) until every Links save asked so far is on disk.
pub(super) fn wait_saved(limit: Duration) {
    FOUND_SAVER.wait_all(limit);
}

impl Engine {
    pub fn found(&self) -> Vec<FoundItem> {
        self.inner.lock().unwrap().found.clone()
    }

    /// Saves the Links list: the bytes are made under the lock, the disk work happens on the
    /// saver's own thread.
    pub(super) fn save_found(&self) {
        let inner = self.inner.lock().unwrap();
        // Never over a file that couldn't be read at start-up.
        if !inner.found_ok {
            return;
        }
        // A private window's links are kept in memory only (gone when Pako closes).
        let kept: Vec<&FoundItem> = inner.found.iter().filter(|f| !f.template.private).collect();
        if let Ok(bytes) = serde_json::to_vec(&kept) {
            FOUND_SAVER.save(bytes);
        }
    }

    /// At start-up: links that were still being checked when the app closed.
    pub(super) fn recheck_found(self: &Arc<Self>) {
        let ids: Vec<String> = self
            .inner
            .lock()
            .unwrap()
            .found
            .iter()
            .filter(|f| matches!(f.state, FoundState::Checking | FoundState::Opening)
                || (f.state == FoundState::NeedsTools && crate::video::ready()))
            .map(|f| f.id.clone())
            .collect();
        for id in ids {
            let gate = self.inner.lock().unwrap().found.iter().find(|f| f.id == id && f.state == FoundState::Opening).map(|f| f.template.link.clone());
            match gate {
                Some(link) => crate::gates::open(self.clone(), id, link),
                None => self.check_found(id),
            }
        }
    }

    /// Pasted text: every link in it goes to the Found list (links already there are skipped).
    pub async fn found_add(self: &Arc<Self>, text: &str) -> AddResult {
        self.found_add_from(text, false).await
    }

    /// Like `found_add`. `no_video`: links from Pako Light, which never become videos (no yt-dlp
    /// lookup, and a video site's link isn't added).
    pub async fn found_add_from(self: &Arc<Self>, text: &str, no_video: bool) -> AddResult {
        self.found_add_with(text, no_video, false).await
    }

    /// Links from the browser. `private`: from a private window: kept in memory only (never in
    /// Links' file, gone when Pako closes), never sent to another PC, and a private download
    /// once downloaded.
    pub async fn found_add_browser(self: &Arc<Self>, text: &str, no_video: bool, private: bool) -> AddResult {
        self.found_add_with(text, no_video, private).await
    }

    /// A link another of your PCs sent to Links. Like a paste, except that a private window's
    /// row of the same link doesn't count as "already there" (`in_links`): that PC's link gets a
    /// row of its own, kept like any other.
    pub async fn found_add_from_device(self: &Arc<Self>, text: &str) -> AddResult {
        self.found_add_with_opts(text, false, false, true).await
    }

    async fn found_add_with(self: &Arc<Self>, text: &str, no_video: bool, private: bool) -> AddResult {
        self.found_add_with_opts(text, no_video, private, false).await
    }

    async fn found_add_with_opts(self: &Arc<Self>, text: &str, no_video: bool, private: bool, from_device: bool) -> AddResult {
        let mut added = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();
        let mut to_check = Vec::new();
        let mut to_open = Vec::new();
        for link in extract_links(text) {
            if in_links(&self.inner.lock().unwrap().found, &link, from_device) {
                skipped += 1;
                continue;
            }
            match self.found_items(&link, no_video).await {
                Ok(items) => {
                    let mut inner = self.inner.lock().unwrap();
                    // Checked again now: another paste of the same link may have landed while
                    // this one was being opened. (A Drive folder's files all carry its link.)
                    if in_links(&inner.found, &link, from_device) {
                        skipped += 1;
                        continue;
                    }
                    for mut it in items {
                        it.no_video = no_video;
                        it.template.private = private;
                        if it.state == FoundState::Checking {
                            to_check.push(it.id.clone());
                        }
                        if it.state == FoundState::Opening {
                            to_open.push((it.id.clone(), it.template.link.clone()));
                        }
                        inner.found.push(it);
                        added += 1;
                    }
                }
                Err(e) => errors.push(format!("{}: {e}", short(&link))),
            }
        }
        if added > 0 {
            self.save_found();
            self.list_changed();
        }
        for id in to_check {
            self.check_found(id);
        }
        for (id, link) in to_open {
            crate::gates::open(self.clone(), id, link);
        }
        AddResult { added, errors, skipped }
    }

    /// What one link becomes in the Found list: one file, or the files of a Drive folder.
    /// `no_video`: from Pako Light (a video site's link is refused).
    async fn found_items(&self, link: &str, no_video: bool) -> Result<Vec<FoundItem>> {
        let none = PathBuf::new();
        if let Some(dl) = drive::parse_link(link) {
            let src = Source::GDrive { file_id: dl.id.clone(), resource_key: dl.resource_key.clone(), doc_type: dl.doc_type.clone() };
            // A folder Google won't list for Pako: open it in the browser (that download is caught).
            let not_public = |src: Source, why: &str| {
                let t = new_download(link, src, "Google Drive folder".into(), none.clone(), None);
                vec![FoundItem::new(t, FoundState::NeedsAccount, Some(why.into()), true)]
            };
            // A file Google won't describe: checked like any link (its first request says the rest).
            let plain = |src: Source| {
                let t = new_download(link, src, format!("Drive file {}", dl.id), none.clone(), None);
                vec![FoundItem::new(t, FoundState::Checking, None, false)]
            };
            let Some(key) = drive::api_key() else {
                return Ok(if dl.kind == LinkKind::Folder { not_public(src, drive::NO_KEY) } else { plain(src) });
            };
            let api = DriveApi { http: &self.http, key: &key };
            let meta = match api.meta(&dl.id, dl.resource_key.as_deref()).await {
                Ok(m) => m,
                Err(e) if dl.kind == LinkKind::Folder => {
                    return match drive::folder_browser_message(&e) {
                        Some(why) => Ok(not_public(src, why)),
                        None => Err(e),
                    };
                }
                Err(_) => return Ok(plain(src)),
            };
            if meta.is_folder() {
                let folder = names::sanitize(&meta.name);
                let rk = meta.resource_key.as_deref().or(dl.resource_key.as_deref());
                let files = match api.list_recursive(&meta.id, rk).await {
                    Ok(f) => f,
                    Err(e) => match drive::folder_browser_message(&e) {
                        Some(why) => return Ok(not_public(src, why)),
                        None => return Err(e),
                    },
                };
                if files.is_empty() {
                    bail!("the folder is empty");
                }
                return Ok(files
                    .into_iter()
                    .filter_map(|(rel, f)| {
                        let t = drive_download(link, &f, none.clone(), Some(folder.clone()))?;
                        let mut it = FoundItem::new(t, FoundState::Ready, None, false);
                        it.folder = Some(folder.clone());
                        it.rel_dir = Some(rel);
                        Some(it)
                    })
                    .collect());
            }
            let t = drive_download(link, &meta, none, None)
                .ok_or_else(|| anyhow!("this kind of Google file (form, site...) can't be downloaded"))?;
            return Ok(vec![FoundItem::new(t, FoundState::Ready, None, false)]);
        }
        let url = url::Url::parse(link).map_err(|_| anyhow!("not a web link"))?;
        if !matches!(url.scheme(), "http" | "https") {
            bail!("only http and https links are supported");
        }
        if crate::video::is_video_site(link) {
            if no_video {
                bail!("Pako Light doesn't send links to this site");
            }
            return Ok(vec![video_item(link, "Video".into(), None)]);
        }
        let t = new_download(link, Source::Http { url: link.into() }, names::from_url(link), none, None);
        // A known link-gate site: Pako works through its pages to reach the real file.
        if crate::gates::is_known_gate(url.host_str().unwrap_or_default()) {
            return Ok(vec![FoundItem::new(t, FoundState::Opening, Some(OPENING.into()), false)]);
        }
        Ok(vec![FoundItem::new(t, FoundState::Checking, None, false)])
    }

    /// Asks the server for the file's name and size, and whether the link works.
    fn check_found(self: &Arc<Self>, id: String) {
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let Ok(permit) = CHECKS.acquire().await else { return };
            let Some((src, no_video)) = me.inner.lock().unwrap().found.iter().find(|f| f.id == id).map(|f| (f.template.source.clone(), f.no_video))
            else {
                return;
            };
            // Pako Light's links are never asked to yt-dlp: a web page is opened as a link page.
            let videos = crate::video::ready() && !no_video;
            if matches!(src, Source::Video { .. }) {
                drop(permit);
                me.check_video(id).await;
                return;
            }
            let fetcher = Fetcher::new(me.http.clone(), &src);
            let is_web = matches!(src, Source::Http { .. });
            let mut outcome = fetcher.get(0, Some(0)).await;
            // A connection problem gets two more tries before the link counts as broken.
            for wait in [2, 6] {
                if !matches!(outcome, Err(FetchError::Retry(_) | FetchError::Throttled(..))) {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(wait)).await;
                outcome = fetcher.get(0, Some(0)).await;
            }
            let mut gate: Option<String> = None;
            {
                let mut inner = me.inner.lock().unwrap();
                let Some(f) = inner.found.iter_mut().find(|f| f.id == id) else { return };
                match outcome {
                    // A web page instead of a file: a page with a video on it (when the video
                    // tools are there and know the site), or a link page (gate) to work through.
                    Ok(resp) if is_web && crate::fetch::is_html(&resp) => {
                        gate = Some(f.template.link.clone());
                        if videos {
                            f.message = Some(crate::video::CHECKING.into());
                        } else {
                            f.state = FoundState::Opening;
                            f.message = Some(OPENING.into());
                        }
                    }
                    Ok(resp) => {
                        let info = probe_info(&resp);
                        if let Some(name) = info.name {
                            f.template.name = names::sanitize(&name);
                        }
                        if info.size.is_some() {
                            f.template.size = info.size;
                        }
                        // Kept for a file on several hosts: a backup host serving the very same
                        // file (same ETag) lets a download go on from the bytes it has.
                        f.template.validator = info.validator;
                        f.template.resumable = info.resumable;
                        f.state = FoundState::Ready;
                        f.message = None;
                    }
                    Err(FetchError::Blocked(_)) => {
                        f.state = FoundState::NeedsAccount;
                        f.message = Some(FILE_NOT_PUBLIC.into());
                    }
                    // The file is there; today's limit is handled when it downloads.
                    Err(FetchError::Quota(m)) => {
                        f.state = FoundState::Ready;
                        f.message = Some(m);
                    }
                    Err(e) => {
                        f.state = FoundState::Broken;
                        f.message = Some(e.message());
                        // (A private window's link: none of its problems in the problem list.)
                        if !f.template.private {
                            crate::errorlog::note(crate::errorlog::Area::LinkCheck, &e.message());
                        }
                    }
                }
            }
            // The quick check is over: the next link can be checked while this page is looked at.
            drop(permit);
            me.save_found();
            me.list_changed();
            if let Some(link) = gate {
                // A page yt-dlp has its own rules for (not its "any page" guess) is a video.
                if videos {
                    // yt-dlp runs: it counts as a video check (two at a time).
                    let probed = match VIDEO_CHECKS.acquire().await {
                        Ok(_slot) => crate::video::probe(&link, Duration::from_secs(25)).await,
                        Err(_) => return,
                    };
                    if let Ok(p) = probed {
                        if p.extractor != "Generic" && (p.is_playlist || !p.options.is_empty()) {
                            let item = video_item(&link, p.title.clone(), None);
                            let new_id = item.id.clone();
                            {
                                let mut inner = me.inner.lock().unwrap();
                                if let Some(pos) = inner.found.iter().position(|f| f.id == id) {
                                    let old = inner.found[pos].clone();
                                    let mut item = item;
                                    item.dest = old.dest;
                                    item.template.priority = old.template.priority;
                                    item.template.notify = old.template.notify;
                                    item.template.private = old.template.private;
                                    inner.found[pos] = item;
                                }
                            }
                            me.apply_probe(&new_id, Ok(p));
                            return;
                        }
                    }
                    if let Some(f) = me.inner.lock().unwrap().found.iter_mut().find(|f| f.id == id) {
                        f.state = FoundState::Opening;
                        f.message = Some(OPENING.into());
                    }
                    me.list_changed();
                }
                crate::gates::open(me.clone(), id, link);
            }
        });
    }

    // ---------- videos (see video.rs) ----------

    /// Asks yt-dlp what a video link is: its title, length and qualities, or a playlist's videos.
    async fn check_video(self: &Arc<Self>, id: String) {
        let Some(link) = self.inner.lock().unwrap().found.iter().find(|f| f.id == id).map(|f| f.template.link.clone()) else {
            return;
        };
        if !crate::video::ready() {
            if let Some(f) = self.inner.lock().unwrap().found.iter_mut().find(|f| f.id == id) {
                f.state = FoundState::NeedsTools;
                f.message = Some(crate::video::NEEDS_TOOLS.into());
            }
            self.save_found();
            self.list_changed();
            return;
        }
        // yt-dlp takes a moment and some memory: a few at a time, like the other checks.
        let Ok(_permit) = VIDEO_CHECKS.acquire().await else { return };
        let mut result = crate::video::probe(&link, Duration::from_secs(60)).await;
        // One more try after a short wait for a connection problem.
        if let Err((m, _)) = &result {
            if crate::video::is_network(m) {
                tokio::time::sleep(Duration::from_secs(3)).await;
                result = crate::video::probe(&link, Duration::from_secs(60)).await;
            }
        }
        if let Err((_, true)) = &result {
            // The site changed: a newer yt-dlp usually fixes it (row says to try again).
            self.update_video_tool_soon();
        }
        self.apply_probe(&id, result);
    }

    /// Puts what yt-dlp said on the row: a video with its qualities, the videos of a
    /// playlist (as a folder), or why it doesn't work.
    fn apply_probe(self: &Arc<Self>, id: &str, result: Result<crate::video::Probe, (String, bool)>) {
        let mut to_check = Vec::new();
        {
            let mut inner = self.inner.lock().unwrap();
            let default_q = inner.settings.video_quality;
            let Some(pos) = inner.found.iter().position(|f| f.id == id) else { return };
            match result {
                Ok(p) if p.is_playlist => {
                    let old = inner.found[pos].clone();
                    let folder = names::sanitize(&p.title);
                    // Videos already in Links or the downloads (the playlist pasted again) aren't
                    // added twice; nor one listed twice in the playlist.
                    let mut known: HashSet<String> = inner
                        .found
                        .iter()
                        .map(|f| f.template.link.clone())
                        .chain(inner.downloads.iter().map(|d| d.link.clone()))
                        .collect();
                    let had = p.entries.len();
                    let items: Vec<FoundItem> = p
                        .entries
                        .iter()
                        .filter(|(link, _, _)| known.insert(link.clone()))
                        .map(|(link, title, duration)| {
                            let mut it = video_item(link, title.clone(), *duration);
                            it.folder = Some(folder.clone());
                            it.dest = old.dest.clone();
                            it.template.priority = old.template.priority;
                            it.template.notify = old.template.notify;
                            it.template.link = link.clone();
                            it.template.private = old.template.private;
                            it.device = old.device.clone();
                            it
                        })
                        .collect();
                    if items.is_empty() && had > 0 {
                        // Nothing new: the row says why nothing was added.
                        inner.found[pos].state = FoundState::Broken;
                        inner.found[pos].message = Some("All videos of this playlist are already in Links or Downloads.".into());
                    } else if items.is_empty() {
                        inner.found[pos].state = FoundState::Broken;
                        inner.found[pos].message = Some("This playlist is empty (or private).".into());
                    } else {
                        to_check = items.iter().map(|f| f.id.clone()).collect();
                        inner.found.splice(pos..=pos, items);
                    }
                }
                Ok(p) => {
                    let f = &mut inner.found[pos];
                    if let Source::Video { quality, audio, options, audio_size, duration, .. } = &mut f.template.source {
                        *options = p.options.clone();
                        *audio_size = p.audio_size;
                        *duration = p.duration.or(*duration);
                        let q = if audio.is_some() { 0 } else { quality.unwrap_or(default_q) };
                        f.template.size = crate::video::estimate(options, *audio_size, q);
                        f.template.name = crate::video::file_name(&p.title, audio.as_deref().or((q == 0).then_some("m4a")));
                    }
                    f.state = FoundState::Ready;
                    f.message = None;
                }
                Err((msg, _)) => {
                    let f = &mut inner.found[pos];
                    if msg == crate::video::NEEDS_TOOLS {
                        f.state = FoundState::NeedsTools;
                    } else {
                        f.state = FoundState::Broken;
                    }
                    if !f.template.private {
                        crate::errorlog::note(crate::errorlog::Area::Video, &msg);
                    }
                    f.message = Some(msg);
                }
            }
        }
        self.save_found();
        self.list_changed();
        for id in to_check {
            self.check_found(id);
        }
    }

    fn update_video_tool_soon(&self) {
        self.update_video_tool();
    }

    /// The player's Download button in the browser: one video straight to Downloads in the
    /// chosen quality (0 = audio only). Answers right away; yt-dlp then reads the video's title
    /// and qualities in the background and a Windows notification says how it went. A playlist
    /// link goes to Links instead (to pick the videos).
    /// The player's Download button. Waits a little for yt-dlp's answer so the button can say
    /// what went wrong (DRM, private video...); a slow site is answered "sent" and carries on.
    /// `private`: from a private window: a private download (no trace beyond the file), and
    /// never any cookies (no cookie file for yt-dlp).
    pub async fn video_from_browser(self: &Arc<Self>, url: String, quality: u32, cookies: Vec<crate::extension::SiteCookie>, private: bool) -> Result<(), String> {
        let cookies = if private { Vec::new() } else { cookies };
        /// Videos being read right now (a double click doesn't add one twice).
        static READING: LazyLock<std::sync::Mutex<HashSet<String>>> = LazyLock::new(Default::default);
        if !crate::video::ready() {
            return Err(NEEDS_VIDEO_TOOLS.into());
        }
        if self.video_in_list(&url) ||!READING.lock().unwrap().insert(url.clone()) {
            return Err("This video is already in Pako.".into());
        }
        let me = self.clone();
        let (answer, answered) = tokio::sync::oneshot::channel::<Result<(), String>>();
        tauri::async_runtime::spawn(async move {
            let result = {
                let _permit = VIDEO_CHECKS.acquire().await;
                let mut result = crate::video::probe_with(&url, Duration::from_secs(60), &cookies).await;
                if let Err((m, _)) = &result {
                    if crate::video::is_network(m) {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        result = crate::video::probe_with(&url, Duration::from_secs(60), &cookies).await;
                    }
                }
                result
            };
            match result {
                Ok(p) if p.is_playlist => {
                    let _ = answer.send(Ok(()));
                    me.found_add_browser(&url, false, private).await;
                    me.notify_as(private, "browser", "Playlist sent to Pako", "Pick the videos in Links, then press Download.", crate::notices::Go::page("found"));
                }
                Ok(p) => {
                    let audio = (quality == 0).then(|| "m4a".to_string());
                    let name = crate::video::file_name(&p.title, audio.as_deref());
                    let size = crate::video::estimate(&p.options, p.audio_size, quality);
                    let src = Source::Video {
                        url: url.clone(),
                        quality: Some(quality),
                        audio,
                        options: p.options,
                        audio_size: p.audio_size,
                        duration: p.duration,
                        streams: Vec::new(),
                        referrer: None,
                        caught: false,
                        recording: None,
                    };
                    let mut d = new_download(&url, src, name, me.settings().download_dir, None);
                    d.size = size;
                    d.private = private;
                    me.hand_over(d, cookies.clone(), crate::ask::wanted());
                    let _ = answer.send(Ok(()));
                }
                Err((msg, newer)) => {
                    if newer {
                        me.update_video_tool_soon();
                    }
                    // The button says it when it is still waiting; else a notification does.
                    if answer.send(Err(msg.clone())).is_err() {
                        me.notify_as(private, "video-failed", "Pako couldn't get this video", &msg, crate::notices::Go::none());
                    }
                }
            }
            READING.lock().unwrap().remove(&url);
        });
        match tokio::time::timeout(Duration::from_secs(25), answered).await {
            Ok(Ok(result)) => result,
            // Still checking (a slow site), or a playlist: it goes on in Pako.
            _ => Ok(()),
        }
    }

    /// A video another of your PCs sent with the quality picked there (0 = audio only) and its
    /// priority, into `dir` (this PC's download folder, or a checked subfolder of it):
    /// straight to the downloads, like the player's Download button, but with no
    /// cookies and no Download with Pako window (nobody may be at this PC). A playlist goes to
    /// Links (picked here). Without the video tools, it waits in Links (its row says so).
    /// Answers once yt-dlp has read the link; the work goes on if nobody waits for the answer.
    /// `id`: the download's id once made (the PC that sent it follows it by that id).
    pub fn video_from_device(self: &Arc<Self>, url: String, quality: u32, priority: Priority, dir: PathBuf, id: String) -> tokio::sync::oneshot::Receiver<Result<DeviceVideo, String>> {
        let (answer, answered) = tokio::sync::oneshot::channel::<Result<DeviceVideo, String>>();
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            if !crate::video::ready() {
                let r = me.found_add_from_device(&url).await;
                let _ = answer.send(if r.added + r.skipped > 0 { Ok(DeviceVideo::ToLinks) } else { Err(r.errors.into_iter().next().unwrap_or_else(|| NEEDS_VIDEO_TOOLS.into())) });
                return;
            }
            // Already downloading (or waiting to) here: taken (never a private window's).
            let already = || device_video_joins(&me.inner.lock().unwrap().downloads, &url);
            if let Some(have) = already() {
                let _ = answer.send(Ok(DeviceVideo::Downloading(have)));
                return;
            }
            let result = {
                let _permit = VIDEO_CHECKS.acquire().await;
                let mut result = crate::video::probe(&url, Duration::from_secs(60)).await;
                if let Err((m, _)) = &result {
                    if crate::video::is_network(m) {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        result = crate::video::probe(&url, Duration::from_secs(60)).await;
                    }
                }
                result
            };
            match result {
                Ok(p) if p.is_playlist => {
                    me.found_add_from_device(&url).await;
                    let _ = answer.send(Ok(DeviceVideo::ToLinks));
                }
                Ok(p) => {
                    // The same video sent twice while it was being read (two sends, two
                    // devices): one download.
                    if let Some(have) = already() {
                        let _ = answer.send(Ok(DeviceVideo::Downloading(have)));
                        return;
                    }
                    let mut d = device_video(&url, p, quality, priority, dir);
                    d.id = id.clone();
                    let name = d.name.clone();
                    me.push(d);
                    me.save_now();
                    me.list_changed();
                    me.schedule();
                    if answer.send(Ok(DeviceVideo::Downloading(id.clone()))).is_err() {
                        // Nobody waited (a slow site): this PC says it.
                        me.notify("devices", "Downloading in Pako", &name, crate::notices::Go::item("downloads", id));
                    }
                }
                Err((msg, newer)) => {
                    if newer {
                        me.update_video_tool_soon();
                    }
                    if answer.send(Err(msg.clone())).is_err() {
                        // Nobody waited: the PC that sent it was told it was taken, and its row
                        // left Links there. The link stays here as a failed download under the
                        // id it was given (Retry here or from that PC; it sees it failed).
                        me.push(device_video_failed(&url, quality, priority, dir, id.clone(), &msg));
                        me.save_now();
                        me.list_changed();
                        me.notify("video-failed", "Pako couldn't get this video", &msg, crate::notices::Go::item("downloads", id));
                    }
                }
            }
        });
        answered
    }

    /// A video from the browser goes to Downloads: the site's login stays with it (encrypted,
    /// for yt-dlp and Pako's own HLS download), then the Download with Pako window first
    /// (quality, folder, name) when `ask`, else it starts and a notification says so.
    fn hand_over(self: &Arc<Self>, mut d: Download, cookies: Vec<crate::extension::SiteCookie>, ask: bool) {
        // A private window's video never gets a cookie file (see `video_from_browser`).
        if !cookies.is_empty() && !d.private {
            crate::extension::remember_headers(&d.id, crate::extension::BrowserHeaders { site_cookies: cookies, ..Default::default() });
        }
        d.ask = ask;
        if d.ask {
            d.message = Some(crate::engine::ASK_NOTE.into());
        }
        let (id, name, held, private) = (d.id.clone(), d.name.clone(), d.ask, d.private);
        self.push(d);
        self.save_now();
        self.list_changed();
        self.schedule();
        if held {
            crate::ask::enqueue(&id);
        } else {
            self.notify_as(private, "browser", "Downloading in Pako", &name, crate::notices::Go::item("downloads", id.clone()));
            crate::ask::started();
        }
    }

    /// Is this link already downloading (or waiting to)?
    fn video_in_list(&self, url: &str) -> bool {
        self.inner.lock().unwrap().downloads.iter().any(|d| d.link == url && !matches!(d.status, Status::Completed | Status::Failed))
    }

    /// "Download with Pako" on a video the extension caught playing on a page (checked by
    /// `stream::validate_stream`): `url` = the stream, `referrer` = the page (as the browser
    /// sends it), `title` = the page's title (the file's name, never yt-dlp's "index" or
    /// "master"), `cookies` = the browser's cookies for the stream's own server. A quick look
    /// with yt-dlp first (a short wait): DRM is said right away, and the qualities when it
    /// knows them. Anything else it can't read still goes to Downloads, where Pako's own HLS
    /// download may get it.
    pub async fn stream_from_browser(
        self: &Arc<Self>,
        url: String,
        referrer: String,
        title: String,
        cookies: Vec<crate::extension::SiteCookie>,
        private: bool,
    ) -> Result<(), String> {
        /// Streams being looked at right now (a double click doesn't add one twice).
        static READING: LazyLock<std::sync::Mutex<HashSet<String>>> = LazyLock::new(Default::default);
        if !crate::video::ready() {
            return Err(NEEDS_VIDEO_TOOLS.into());
        }
        if self.video_in_list(&url) || !READING.lock().unwrap().insert(url.clone()) {
            return Err("This video is already in Pako.".into());
        }
        // A private window's: a private download, never with cookies.
        let cookies = if private { Vec::new() } else { cookies };
        let r = self.add_caught_stream(&url, referrer, title, cookies, private).await;
        READING.lock().unwrap().remove(&url);
        r
    }

    async fn add_caught_stream(
        self: &Arc<Self>,
        url: &str,
        referrer: String,
        title: String,
        cookies: Vec<crate::extension::SiteCookie>,
        private: bool,
    ) -> Result<(), String> {
        let quality = self.settings().video_quality;
        let audio = (quality == 0).then(|| "m4a".to_string());
        let referrer = (!referrer.is_empty()).then_some(referrer);
        let probe = {
            let _permit = VIDEO_CHECKS.acquire().await;
            tokio::time::timeout(Duration::from_secs(25), crate::video::probe_from_page(url, Duration::from_secs(20), &cookies, referrer.as_deref())).await
        };
        let (options, audio_size, duration) = match probe {
            Ok(Ok(p)) => (p.options, p.audio_size, p.duration),
            Ok(Err((msg, _))) if msg == crate::video::DRM => return Err(crate::hls::PROTECTED.into()),
            Ok(Err((msg, _))) if msg == crate::video::NEEDS_TOOLS => return Err(NEEDS_VIDEO_TOOLS.into()),
            _ => (Vec::new(), None, None),
        };
        let size = crate::video::estimate(&options, audio_size, quality);
        let name = crate::video::file_name(&title, audio.as_deref());
        let src = Source::Video {
            url: url.to_string(),
            quality: Some(quality),
            audio,
            options,
            audio_size,
            duration,
            streams: Vec::new(),
            referrer,
            caught: true,
            recording: None,
        };
        let mut d = new_download(url, src, name, self.settings().download_dir, None);
        d.size = size;
        d.private = private;
        self.hand_over(d, cookies, crate::ask::wanted());
        Ok(())

    }

    /// "Record this video" (checked by `stream::validate_record`): a recording starts, gets its
    /// chunks, ends (then it is joined in Downloads like any video) or is cancelled.
    pub fn record(self: &Arc<Self>, r: crate::stream::Record) -> Result<(), String> {
        use crate::stream::Record;
        match r {
            Record::Start { session, page, title } => {
                if !crate::video::ready() {
                    return Err(NEEDS_VIDEO_TOOLS.into());
                }
                crate::stream::start_in(&crate::stream::recordings_dir(), &session, &page, &title)
            }
            Record::Chunk { session, track, mime, seq, data } => crate::stream::chunk(&session, track, &mime, seq, &data),
            Record::Cancel { session } => {
                crate::stream::cancel(&session);
                Ok(())
            }
            Record::End { session } => {
                let rec = crate::stream::end(&session)?;
                let size: u64 = rec.tracks.iter().filter_map(|t| std::fs::metadata(rec.dir.join(&t.file)).ok()).map(|m| m.len()).sum();
                let src = Source::Video {
                    url: rec.page.clone(),
                    quality: None,
                    audio: None,
                    options: Vec::new(),
                    audio_size: None,
                    duration: None,
                    streams: Vec::new(),
                    referrer: None,
                    caught: true,
                    recording: Some(rec.dir.clone()),
                };
                let mut d = new_download(&rec.page, src, crate::video::file_name(&rec.title, None), self.settings().download_dir, None);
                d.size = Some(size);
                // Played to the end already: joined right away, no questions.
                self.hand_over(d, Vec::new(), false);
                Ok(())
            }
        }
    }

    /// The details panel's quality for video rows. `quality` None = the default from
    /// Settings; 0 = audio only, with `audio` "m4a" or "mp3".
    pub fn found_set_video(&self, ids: &[String], quality: Option<u32>, audio: Option<String>) {
        {
            let mut inner = self.inner.lock().unwrap();
            let default_q = inner.settings.video_quality;
            for f in inner.found.iter_mut().filter(|f| ids.contains(&f.id)) {
                let title = std::path::Path::new(&f.template.name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if let Source::Video { quality: q, audio: a, options, audio_size, .. } = &mut f.template.source {
                    *q = quality;
                    *a = if quality == Some(0) { Some(audio.clone().unwrap_or_else(|| "m4a".into())) } else { None };
                    let want = if a.is_some() { 0 } else { quality.unwrap_or(default_q) };
                    f.template.size = crate::video::estimate(options, *audio_size, want);
                    if f.state == FoundState::Ready {
                        f.template.name = crate::video::file_name(&title, a.as_deref());
                    }
                }
            }
        }
        self.save_found();
        self.list_changed();
    }

    /// Checks rows again (the video tools were just installed, or the user asked).
    pub fn found_recheck(self: &Arc<Self>, ids: &[String]) {
        let mut todo = Vec::new();
        {
            let mut inner = self.inner.lock().unwrap();
            for f in inner.found.iter_mut().filter(|f| ids.contains(&f.id) || (ids.is_empty() && f.state == FoundState::NeedsTools)) {
                if matches!(f.state, FoundState::NeedsTools | FoundState::Broken) {
                    f.state = FoundState::Checking;
                    f.message = matches!(f.template.source, Source::Video { .. }).then(|| crate::video::CHECKING.into());
                    todo.push(f.id.clone());
                }
            }
        }
        self.save_found();
        self.list_changed();
        for id in todo {
            self.check_found(id);
        }
    }

    // ---------- link gates (see gates.rs) ----------

    // (Only the link-gate window uses it: not in the unit tests, see gates::open.)
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn found_has(&self, id: &str) -> bool {
        self.inner.lock().unwrap().found.iter().any(|f| f.id == id)
    }

    /// The row's note while a gate is worked through (None = back to "Opening link…").
    pub(crate) fn gate_status(&self, id: &str, msg: Option<&str>) {
        if let Some(f) = self.inner.lock().unwrap().found.iter_mut().find(|f| f.id == id) {
            f.message = Some(msg.unwrap_or(OPENING).to_string());
        }
        self.list_changed();
    }

    pub(crate) fn gate_failed(&self, id: &str, msg: &str) {
        if let Some(f) = self.inner.lock().unwrap().found.iter_mut().find(|f| f.id == id) {
            if !f.template.private {
                crate::errorlog::note(crate::errorlog::Area::LinkCheck, msg);
            }
            f.state = FoundState::Broken;
            f.message = Some(msg.to_string());
        }
        self.save_found();
        self.list_changed();
    }

    /// The gate led to the real file: the row becomes that file (or a Drive folder's files),
    /// keeping its own settings (folder, priority, notify).
    // (Only the link-gate window uses it: not in the unit tests, see gates::open.)
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn gate_done(self: &Arc<Self>, id: &str, file: String) {
        let me = self.clone();
        let id = id.to_string();
        tauri::async_runtime::spawn(async move {
            let no_video = me.inner.lock().unwrap().found.iter().find(|f| f.id == id).is_some_and(|f| f.no_video);
            let items = me.found_items(&file, no_video).await;
            let mut to_check = Vec::new();
            let mut to_open = Vec::new();
            {
                let mut inner = me.inner.lock().unwrap();
                let Some(pos) = inner.found.iter().position(|f| f.id == id) else { return };
                match items {
                    Ok(mut items) => {
                        let old = inner.found[pos].clone();
                        for it in items.iter_mut() {
                            it.no_video = old.no_video;
                            it.dest = old.dest.clone();
                            it.template.priority = old.template.priority;
                            it.template.notify = old.template.notify;
                            it.template.private = old.template.private;
                            it.device = old.device.clone();
                            match it.state {
                                FoundState::Checking => to_check.push(it.id.clone()),
                                FoundState::Opening => to_open.push((it.id.clone(), it.template.link.clone())),
                                _ => {}
                            }
                        }
                        inner.found.splice(pos..=pos, items);
                    }
                    Err(e) => {
                        inner.found[pos].state = FoundState::Broken;
                        inner.found[pos].message = Some(format!("Reached the file link, but: {e}"));
                        if !inner.found[pos].template.private {
                            crate::errorlog::note(crate::errorlog::Area::LinkCheck, &format!("Reached the file link, but: {e}"));
                        }
                    }
                }
            }
            me.save_found();
            me.list_changed();
            for id in to_check {
                me.check_found(id);
            }
            for (id, link) in to_open {
                crate::gates::open(me.clone(), id, link);
            }
        });
    }

    /// Turns the picked Found items into downloads, saved under `dest` (Drive folders keep
    /// their own folder inside it). `first` = Download first. Returns how many were added.
    pub fn found_download(self: &Arc<Self>, ids: &[String], first: bool, dest: Option<PathBuf>, notify: bool) -> usize {
        let dest = dest.unwrap_or_else(|| self.settings().download_dir);
        let picked: Vec<FoundItem> = {
            let mut inner = self.inner.lock().unwrap();
            // The same file on several hosts: one download, from the chosen host, carrying the
            // other ready hosts as backups. The group's rows all leave Links.
            let mut want: HashSet<String> = ids.iter().cloned().collect();
            let mut backups: std::collections::HashMap<String, Vec<Mirror>> = std::collections::HashMap::new();
            let mut gone: HashSet<String> = HashSet::new();
            // A group with a private window's link in it: its download is private too (its
            // backups' links are never written to disk).
            let mut private_groups: HashSet<String> = HashSet::new();
            let prefs = inner.settings.mirror_hosts.clone();
            for g in mirror_groups(&inner.found) {
                if !g.iter().any(|&i| want.contains(&inner.found[i].id)) {
                    continue;
                }
                for &i in &g {
                    want.remove(&inner.found[i].id);
                }
                let (Some(c), _) = mirror_choice(&inner.found, &g, &prefs) else { continue };
                want.insert(inner.found[c].id.clone());
                backups.insert(inner.found[c].id.clone(), mirror_backups(&inner.found, &g, c, &prefs));
                if g.iter().any(|&i| inner.found[i].template.private) {
                    private_groups.insert(inner.found[c].id.clone());
                }
                gone.extend(g.iter().map(|&i| inner.found[i].id.clone()));
            }
            let mut picked: Vec<FoundItem> =
                inner.found.iter().filter(|f| want.contains(&f.id) && f.can_download()).cloned().collect();
            for f in picked.iter_mut() {
                f.template.mirrors = backups.remove(&f.id).unwrap_or_default();
                f.template.private |= private_groups.contains(&f.id);
            }
            gone.extend(picked.iter().map(|f| f.id.clone()));
            inner.found.retain(|f| !gone.contains(&f.id));
            picked
        };
        // Split archives (not inside a Drive folder): each set goes in a folder named after it.
        let extract_default = self.settings().extract_default;
        let loose: Vec<usize> = (0..picked.len()).filter(|&i| picked[i].folder.is_none()).collect();
        let names: Vec<&str> = loose.iter().map(|&i| picked[i].template.name.as_str()).collect();
        let mut set_of: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
        let mut set_members: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for set in crate::split::find_sets(&names) {
            let folder = names::sanitize(&set.base);
            for m in set.members {
                set_of.insert(loose[m], folder.clone());
                set_members.entry(folder.clone()).or_default().push(names[m].to_string());
            }
        }
        // Each set's folder, chosen once: a fresh one ("Name (2)") if the usual one already holds
        // a file with one of its parts' names. Renaming a single part ("x.part1 (1).rar") would
        // break the set (7-Zip finds parts by their exact names).
        let mut set_dirs: std::collections::HashMap<(PathBuf, String), PathBuf> = std::collections::HashMap::new();
        let taken: HashSet<String> = {
            let inner = self.inner.lock().unwrap();
            inner.downloads.iter().map(|d| names::path_key(&d.final_path())).collect()
        };
        let mut set_dir = |base: &PathBuf, folder: &str| -> PathBuf {
            set_dirs
                .entry((base.clone(), folder.to_string()))
                .or_insert_with(|| {
                    let members = set_members.get(folder).cloned().unwrap_or_default();
                    let clashes = |dir: &PathBuf| {
                        members.iter().any(|m| {
                            let p = dir.join(m);
                            p.exists() || names::part_exists(dir, m) || taken.contains(&names::path_key(&p))
                        })
                    };
                    let mut dir = base.join(folder);
                    let mut n = 2;
                    while clashes(&dir) && n < 1000 {
                        dir = base.join(format!("{folder} ({n})"));
                        n += 1;
                    }
                    dir
                })
                .clone()
        };
        let base = super::now_millis();
        let mut new_ids = Vec::new();
        let mut batch = Vec::new();
        for (k, f) in picked.into_iter().enumerate() {
            let mut d = f.template;
            d.id = uuid::Uuid::new_v4().to_string();
            d.notify |= notify;
            d.added_at = base + k as u64;
            d.date = base;
            let base = f.dest.clone().unwrap_or_else(|| dest.clone());
            // A part of a set that is already in the downloads joins it (same folder, same
            // extract choice), even when picked on its own later.
            let joined = if f.folder.is_none() { self.existing_set_place(&d.name) } else { None };
            d.dest_dir = match (&f.folder, joined, set_of.get(&k)) {
                (Some(folder), _, _) => {
                    // A split set inside a Drive folder follows the same extract default.
                    if crate::split::set_key(&d.name).is_some() {
                        d.extract = f.extract.unwrap_or(extract_default);
                    }
                    base.join(folder).join(f.rel_dir.unwrap_or_default())
                }
                (None, Some((dir, group, extract)), _) => {
                    d.group = Some(group);
                    d.extract = f.extract.unwrap_or(extract);
                    dir
                }
                (None, None, Some(set)) => {
                    d.group = Some(set.clone());
                    d.extract = f.extract.unwrap_or(extract_default);
                    set_dir(&base, set)
                }
                (None, None, None) => base,
            };
            new_ids.push(d.id.clone());
            batch.push(d);
        }
        self.push_many(batch);
        // The downloads first (save_now waits until they're on disk), then Links without them:
        // stopped in between, the picked links are in both lists rather than in neither.
        self.save_now();
        self.save_found();
        if first {
            self.download_first(&new_ids);
        } else {
            self.list_changed();
            self.schedule();
        }
        new_ids.len()
    }

    /// Per-item settings from the details panel: own save folder (`Some(None)` = back to
    /// the tab's Save to), priority, notify when done.
    pub fn found_set(
        &self,
        ids: &[String],
        dest: Option<Option<PathBuf>>,
        priority: Option<Priority>,
        notify: Option<bool>,
        extract: Option<bool>,
    ) {
        {
            let mut inner = self.inner.lock().unwrap();
            for f in inner.found.iter_mut().filter(|f| ids.contains(&f.id)) {
                if let Some(d) = &dest {
                    f.dest = d.clone();
                }
                if let Some(p) = priority {
                    f.template.priority = p;
                }
                if let Some(n) = notify {
                    f.template.notify = n;
                }
                if let Some(x) = extract {
                    f.extract = Some(x);
                }
            }
        }
        self.save_found();
        self.list_changed();
    }

    /// My devices › Links: what a playlist, a Drive folder or a link page holds, read on this
    /// PC (yt-dlp, Drive, the link page worked through here) so its videos or files can be
    /// picked and sent one by one. (title, items).
    pub async fn pick_items(self: &Arc<Self>, link: &str) -> Result<(String, Vec<PickItem>), String> {
        let mut link = link.to_string();
        let host = url::Url::parse(&link).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default();
        if crate::gates::is_known_gate(&host) {
            link = crate::gates::resolve(self.clone(), link).await?;
        }
        if crate::video::is_video_site(&link) {
            if !crate::video::ready() {
                return Err(NEEDS_VIDEO_TOOLS.into());
            }
            let p = {
                let _permit = VIDEO_CHECKS.acquire().await;
                crate::video::probe(&link, Duration::from_secs(60)).await.map_err(|(m, _)| m)?
            };
            if p.is_playlist {
                let folder = names::sanitize(&p.title);
                let items = p
                    .entries
                    .into_iter()
                    .map(|(l, title, duration)| PickItem { link: l, name: title, video: true, duration, size: None, folder: Some(folder.clone()) })
                    .collect::<Vec<_>>();
                let items = once_per_link(items);
                if items.is_empty() {
                    return Err("This playlist is empty (or private).".into());
                }
                return Ok((p.title, items));
            }
            return Ok((p.title.clone(), vec![PickItem { link, name: p.title, video: true, duration: p.duration, size: None, folder: None }]));
        }
        let items = self.found_items(&link, false).await.map_err(|e| e.to_string())?;
        if let Some(f) = items.iter().find(|f| f.state == FoundState::NeedsAccount) {
            return Err(f.message.clone().unwrap_or_else(|| FILE_NOT_PUBLIC.into()));
        }
        let title = items.first().and_then(|f| f.folder.clone()).unwrap_or_else(|| items.first().map(|f| f.template.name.clone()).unwrap_or_default());
        let out: Vec<PickItem> = items
            .iter()
            .map(|f| PickItem {
                link: link_of(f),
                name: f.template.name.clone(),
                video: false,
                duration: None,
                size: f.template.size,
                // A Drive folder's files keep its name (and the first level inside it).
                folder: f.folder.as_ref().map(|g| {
                    let sub = f.rel_dir.as_ref().and_then(|r| r.components().next()).map(|c| c.as_os_str().to_string_lossy().to_string());
                    match sub {
                        Some(s) if !s.is_empty() => format!("{g}/{s}"),
                        _ => g.clone(),
                    }
                }),
            })
            .collect();
        let out = once_per_link(out);
        if out.is_empty() {
            return Err("Nothing to download in this link.".into());
        }
        Ok((title, out))
    }

    /// The per-link "Download on": None = this PC. Its note goes (a new choice, a new start).
    pub fn found_set_device(&self, ids: &[String], device: Option<String>) {
        {
            let mut inner = self.inner.lock().unwrap();
            for f in inner.found.iter_mut().filter(|f| ids.contains(&f.id)) {
                // A private window's link stays on this PC (its row says why).
                if f.template.private {
                    f.device = None;
                    f.device_note = device.is_some().then(|| PRIVATE_HERE.to_string());
                    continue;
                }
                f.device = device.clone();
                f.device_note = None;
            }
        }
        self.save_found();
        self.list_changed();
    }

    /// The note on rows that weren't sent (None = clears it).
    pub fn found_device_note(&self, ids: &[String], note: Option<String>) {
        {
            let mut inner = self.inner.lock().unwrap();
            for f in inner.found.iter_mut().filter(|f| ids.contains(&f.id)) {
                f.device_note = note.clone();
            }
        }
        self.save_found();
        self.list_changed();
    }

    /// Rows set to a PC that is no longer paired, or no longer lets this PC control it, go
    /// back to This PC, with a note. `allowed`: the PCs that may still be picked.
    pub fn found_device_fallback(&self, allowed: &[String]) {
        let changed = {
            let mut inner = self.inner.lock().unwrap();
            fallback_devices(&mut inner.found, allowed)
        };
        if changed {
            self.save_found();
            self.list_changed();
        }
    }

    /// Where the picked links go when you press Download: this PC, or the PC each one is set
    /// to (one order per PC). Rows that can't go now get their note and stay in Links.
    pub fn found_split(&self, ids: &[String], devices: &[DeviceChoice]) -> DeviceSplit {
        let split = {
            let mut inner = self.inner.lock().unwrap();
            let allowed: Vec<String> = devices.iter().map(|d| d.id.clone()).collect();
            fallback_devices(&mut inner.found, &allowed);
            let split = split_by_device(&inner.found, ids, devices);
            for (id, note) in &split.stay {
                if let Some(f) = inner.found.iter_mut().find(|f| &f.id == id) {
                    f.device_note = Some(note.clone());
                }
            }
            split
        };
        self.save_found();
        self.list_changed();
        split
    }

    /// Picked links whose file already exists where it would be saved (a duplicate on disk).
    pub fn found_existing(&self, ids: &[String], dest: Option<PathBuf>) -> Vec<String> {
        // Where each would land, read under the lock; the disk is checked after letting go of it.
        let places: Vec<(String, Vec<PathBuf>)> = {
            let inner = self.inner.lock().unwrap();
            let fallback = dest.unwrap_or_else(|| inner.settings.download_dir.clone());
            let picked: Vec<&FoundItem> = inner.found.iter().filter(|f| ids.contains(&f.id)).collect();
            // Parts of a split archive land in a folder named after their set (as `found_download`
            // places them).
            let loose: Vec<usize> = (0..picked.len()).filter(|&i| picked[i].folder.is_none()).collect();
            let names: Vec<&str> = loose.iter().map(|&i| picked[i].template.name.as_str()).collect();
            let mut set_of: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
            for set in crate::split::find_sets(&names) {
                for m in set.members {
                    set_of.insert(loose[m], names::sanitize(&set.base));
                }
            }
            picked
                .iter()
                .enumerate()
                .map(|(k, f)| {
                    let base = f.dest.clone().unwrap_or_else(|| fallback.clone());
                    let name = &f.template.name;
                    // Plain files land in the folder; Drive folder files inside a folder named after it.
                    let mut paths = vec![base.join(name)];
                    if let Some(g) = &f.folder {
                        paths.push(base.join(g).join(name));
                        if let Some(rel) = f.rel_dir.as_deref().filter(|r| !r.as_os_str().is_empty()) {
                            paths.push(base.join(g).join(rel).join(name));
                        }
                    }
                    if let Some(set) = set_of.get(&k) {
                        paths.push(base.join(set).join(name));
                    }
                    (f.id.clone(), paths)
                })
                .collect()
        };
        places.into_iter().filter(|(_, paths)| paths.iter().any(|p| p.is_file())).map(|(id, _)| id).collect()
    }

    /// Undo for removing Found links: puts them back where they were.
    pub fn found_restore(self: &Arc<Self>, items: Vec<FoundItem>, at: usize) {
        let mut to_open = Vec::new();
        let mut to_check = Vec::new();
        {
            let mut inner = self.inner.lock().unwrap();
            let fresh: Vec<FoundItem> = items.into_iter().filter(|f| !inner.found.iter().any(|x| x.id == f.id)).collect();
            for f in &fresh {
                match f.state {
                    FoundState::Opening => to_open.push((f.id.clone(), f.template.link.clone())),
                    FoundState::Checking => to_check.push(f.id.clone()),
                    _ => {}
                }
            }
            let at = at.min(inner.found.len());
            inner.found.splice(at..at, fresh);
        }
        self.save_found();
        self.list_changed();
        for id in to_check {
            self.check_found(id);
        }
        for (id, link) in to_open {
            crate::gates::open(self.clone(), id, link);
        }
    }

    /// The links of a private window's rows in Links (also as another PC would get them):
    /// never sent to another of your PCs.
    pub fn found_private_links(&self) -> HashSet<String> {
        let inner = self.inner.lock().unwrap();
        inner.found.iter().filter(|f| f.template.private).flat_map(|f| [f.template.link.clone(), link_of(f)]).collect()
    }

    /// `notify` about a Links row (a link page that needs you): a private window's row gets a
    /// pop-up only, never a line in the bell.
    pub(crate) fn notify_row(&self, id: &str, kind: &str, title: &str, body: &str, go: crate::notices::Go) {
        let private = self.inner.lock().unwrap().found.iter().any(|f| f.id == id && f.template.private);
        self.notify_as(private, kind, title, body, go);
    }

    pub fn found_remove(&self, ids: &[String]) {
        self.inner.lock().unwrap().found.retain(|f| !ids.contains(&f.id));
        self.save_found();
        self.list_changed();
    }

    /// The same file on several hosts, among the Links rows (for the window).
    pub fn found_mirrors(&self) -> Vec<MirrorView> {
        let inner = self.inner.lock().unwrap();
        mirror_views(&inner.found, &inner.settings.mirror_hosts)
    }

    /// The user picked these rows' hosts (each in its own group; a split set's parts on one
    /// host): they download, the others become backups. The host is remembered first for the
    /// next groups.
    pub fn found_pick_host(&self, ids: &[String]) {
        {
            let mut inner = self.inner.lock().unwrap();
            let groups = mirror_groups(&inner.found);
            let mut hosts = Vec::new();
            for g in &groups {
                let Some(&pick) = g.iter().find(|&&i| ids.contains(&inner.found[i].id) && inner.found[i].can_download()) else {
                    continue;
                };
                for &i in g {
                    inner.found[i].host_pick = i == pick;
                }
                if let Some(h) = host_key(&inner.found[pick].template.source) {
                    if !hosts.contains(&h) {
                        hosts.push(h);
                    }
                }
            }
            for h in hosts.iter().rev() {
                prefer_host(&mut inner.settings.mirror_hosts, h);
            }
        }
        let prefs = self.settings().mirror_hosts;
        let _ = self.change_settings(|s| s.mirror_hosts = prefs);
        self.save_found();
        self.list_changed();
    }
}

// ---------- the same file on several hosts (mirrors) ----------

/// Most hosts remembered in Settings as picked (most recent first).
const MIRROR_HOSTS_KEPT: usize = 20;

/// A group of Links rows that are the same file on different hosts, as the window gets it.
#[derive(serde::Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MirrorView {
    pub key: String,
    pub name: String,
    pub size: Option<u64>,
    /// Its rows, in the order the links came in.
    pub ids: Vec<String>,
    /// Each row's host (same order as `ids`).
    pub hosts: Vec<String>,
    /// The row that downloads (None = every host is broken).
    pub chosen: Option<String>,
    /// The user picked that host for this group.
    pub picked: bool,
}

/// The link is already in Links (a paste of it is skipped). For a link another PC sends
/// (`from_device`), a private window's row doesn't count: it lives in memory only and goes at
/// quit, so that PC's link, counted as taken, would be lost with it.
fn in_links(found: &[FoundItem], link: &str, from_device: bool) -> bool {
    found.iter().any(|f| f.template.link == link && !(from_device && f.template.private))
}

/// The download a video sent by another PC joins: the same link already downloading (or
/// waiting to) here. Never a private window's: that one stays on this PC, so the other PC
/// gets a download of its own and never the private one's id (its card would follow a
/// download it can't see, and the video it sent would end with the private one, untraced).
/// Nor one waiting in the Download with Pako window: also off the other PC's list, and gone
/// if that window is cancelled here.
fn device_video_joins(downloads: &[Download], url: &str) -> Option<String> {
    downloads.iter().find(|d| d.link == url && !d.private && !d.ask && !matches!(d.status, Status::Completed | Status::Failed)).map(|d| d.id.clone())
}

/// What a video sent by another of your PCs becomes here: a download in the quality picked
/// there (the closest height at or below, as any video), in this PC's own download folder,
/// with its priority. Never held for the Download with Pako window.
pub(crate) fn device_video(url: &str, p: crate::video::Probe, quality: u32, priority: Priority, dir: PathBuf) -> Download {
    let audio = (quality == 0).then(|| "m4a".to_string());
    let name = crate::video::file_name(&p.title, audio.as_deref());
    let size = crate::video::estimate(&p.options, p.audio_size, quality);
    let src = Source::Video {
        url: url.to_string(),
        quality: Some(quality),
        audio,
        options: p.options,
        audio_size: p.audio_size,
        duration: p.duration,
        streams: Vec::new(),
        referrer: None,
        caught: false,
        recording: None,
    };
    let mut d = new_download(url, src, name, dir, None);
    d.size = size;
    d.priority = priority;
    d.ask = false;
    d
}

/// A video sent by another PC that couldn't be read after that PC was told it was taken: kept
/// as a failed download (its link, quality, priority and folder), so it isn't lost and that
/// PC's card shows it failed. Retry reads it again.
pub(crate) fn device_video_failed(url: &str, quality: u32, priority: Priority, dir: PathBuf, id: String, why: &str) -> Download {
    let p = crate::video::Probe { title: "Video".into(), ..Default::default() };
    let mut d = device_video(url, p, quality, priority, dir);
    d.id = id;
    d.status = Status::Failed;
    d.message = Some(why.to_string());
    d
}

/// Each link once (a playlist can hold the same video twice): the picker card lists rows by
/// their link, and the same link sent twice would only be one download there anyway.
pub(crate) fn once_per_link(items: Vec<PickItem>) -> Vec<PickItem> {
    let mut seen = HashSet::new();
    items.into_iter().filter(|i| seen.insert(i.link.clone())).collect()
}

/// One video or file of a playlist, a Drive folder or a link page, to pick and send.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickItem {
    pub link: String,
    pub name: String,
    pub video: bool,
    pub duration: Option<f64>,
    pub size: Option<u64>,
    /// The playlist's or folder's name: its subfolder on the other PC.
    pub folder: Option<String>,
}

/// The link another PC downloads a Links row from (as the window's `linkOf`): the link as it
/// was pasted, or for a file found in a Drive folder, its own Drive link.
pub(crate) fn link_of(f: &FoundItem) -> String {
    if let (Source::GDrive { file_id, resource_key, doc_type }, Some(_)) = (&f.template.source, &f.folder) {
        let enc = |s: &str| url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
        let key = resource_key.as_deref().map(|k| format!("?resourcekey={}", enc(k))).unwrap_or_default();
        let id = enc(file_id);
        if let Some(t) = doc_type.as_deref().filter(|t| ["document", "spreadsheets", "presentation"].contains(t)) {
            return format!("https://docs.google.com/{t}/d/{id}/edit{key}");
        }
        return format!("https://drive.google.com/file/d/{id}/view{key}");
    }
    f.template.link.clone()
}

/// What a video sent by another PC gave here.
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceVideo {
    /// The download's id.
    Downloading(String),
    /// A playlist (or no video tools here): in Links.
    ToLinks,
}

/// One of your PCs that a link can be set to download on (paired, letting this PC control it).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceChoice {
    pub id: String,
    pub name: String,
    pub online: bool,
}

/// Where picked links go (see `found_split`).
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSplit {
    /// Downloaded on this PC.
    pub here: Vec<String>,
    /// One order per PC: (its Pako ID, its name, the rows).
    pub send: Vec<(String, String, Vec<String>)>,
    /// Rows that stay in Links, with why.
    pub stay: Vec<(String, String)>,
}

pub(crate) const DEVICE_GONE: &str = "The PC this link was set to download on is no longer paired, or no longer lets this PC control it. Back to This PC.";

/// Rows set to a PC not in `allowed` go back to This PC, noted. Says whether any changed.
pub(crate) fn fallback_devices(found: &mut [FoundItem], allowed: &[String]) -> bool {
    let mut changed = false;
    for f in found.iter_mut() {
        if f.device.as_ref().is_some_and(|d| !allowed.contains(d)) {
            f.device = None;
            f.device_note = Some(DEVICE_GONE.into());
            changed = true;
        }
    }
    changed
}

/// The split of picked rows between this PC and the PCs they're set to. A row stays (noted)
/// when its PC is offline or no longer allowed, or when it must be opened here first (a
/// playlist whose videos aren't listed yet, a link page): only its videos or file are sent.
pub(crate) fn split_by_device(found: &[FoundItem], ids: &[String], devices: &[DeviceChoice]) -> DeviceSplit {
    let mut out = DeviceSplit::default();
    for f in found.iter().filter(|f| ids.contains(&f.id) && f.can_download()) {
        let Some(dev) = &f.device else {
            out.here.push(f.id.clone());
            continue;
        };
        if f.template.private {
            out.stay.push((f.id.clone(), PRIVATE_HERE.into()));
            continue;
        }
        let Some(d) = devices.iter().find(|d| &d.id == dev) else {
            out.stay.push((f.id.clone(), DEVICE_GONE.into()));
            continue;
        };
        if let Some(why) = crate::direct::open_here_first(&f.template.link) {
            out.stay.push((f.id.clone(), why.into()));
            continue;
        }
        if !d.online {
            out.stay.push((f.id.clone(), format!("{} is offline: not sent. Press Download again once it's on.", d.name)));
            continue;
        }
        match out.send.iter_mut().find(|(id, _, _)| id == dev) {
            Some((_, _, rows)) => rows.push(f.id.clone()),
            None => out.send.push((d.id.clone(), d.name.clone(), vec![f.id.clone()])),
        }
    }
    out
}

/// The host a Links row downloads from ("example.com", "drive.google.com"), for rows that can
/// be a mirror: a single file from a web link or Google Drive. Videos have none.
pub(crate) fn host_key(source: &Source) -> Option<String> {
    match source {
        Source::Video { .. } => None,
        Source::GDrive { .. } => Some("drive.google.com".into()),
        Source::Http { url } => {
            let u = url::Url::parse(url).ok()?;
            let h = u.host_str()?.to_ascii_lowercase();
            Some(h.strip_prefix("www.").unwrap_or(&h).to_string())
        }
    }
}

/// How a row takes part in mirror groups: its host and its name (cleaned, any case).
fn mirror_part(f: &FoundItem) -> Option<(String, String)> {
    // Drive folders (and their files) and videos aren't mirrored.
    if f.is_folder || f.folder.is_some() {
        return None;
    }
    let host = host_key(&f.template.source)?;
    Some((host, names::sanitize(&f.template.name).to_lowercase()))
}

/// The same file on several hosts, among the Links rows: lists of positions, each in link order.
/// Same file = same name (cleaned, any case) and the same size, known after checking. A row
/// still being checked joins when its check ends; a broken row (no size: its check failed)
/// joins a group with its name, as a host that doesn't work. Never two rows from the same host.
pub(crate) fn mirror_groups(found: &[FoundItem]) -> Vec<Vec<usize>> {
    // (size, members, hosts), and the groups of each name in the order they were made (asked for
    // on every refresh of the window, under the engine's lock: no search through every group).
    let mut groups: Vec<(u64, Vec<usize>, Vec<String>)> = Vec::new();
    let mut by_name: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, f) in found.iter().enumerate() {
        let Some(size) = f.template.size else { continue };
        if !matches!(f.state, FoundState::Ready | FoundState::Checking) {
            continue;
        }
        let Some((host, name)) = mirror_part(f) else { continue };
        let named = by_name.entry(name).or_default();
        match named.iter().copied().find(|&g| groups[g].0 == size && !groups[g].2.contains(&host)) {
            Some(g) => {
                groups[g].1.push(i);
                groups[g].2.push(host);
            }
            None => {
                named.push(groups.len());
                groups.push((size, vec![i], vec![host]));
            }
        }
    }
    // Broken rows: by name, next to rows whose size is known (never by name alone).
    for (i, f) in found.iter().enumerate() {
        if f.state != FoundState::Broken {
            continue;
        }
        let Some((host, name)) = mirror_part(f) else { continue };
        let Some(named) = by_name.get(&name) else { continue };
        if let Some(g) = named
            .iter()
            .copied()
            .find(|&g| f.template.size.is_none_or(|s| s == groups[g].0) && !groups[g].2.contains(&host))
        {
            groups[g].1.push(i);
            groups[g].2.push(host);
        }
    }
    groups
        .into_iter()
        .filter(|g| g.1.len() > 1)
        .map(|mut g| {
            g.1.sort_unstable();
            g.1
        })
        .collect()
}

/// Ready rows of a group, best first: hosts picked before (most recent first), then the order
/// the links came in. A host whose check noted a problem for today (Google's daily limit:
/// Ready with a note) comes after the hosts without one.
fn ready_in_order(found: &[FoundItem], group: &[usize], prefs: &[String]) -> Vec<usize> {
    let rank = |i: usize| {
        let host = host_key(&found[i].template.source).unwrap_or_default();
        prefs.iter().position(|p| *p == host).unwrap_or(prefs.len())
    };
    let mut ready: Vec<usize> = group.iter().copied().filter(|&i| found[i].state == FoundState::Ready).collect();
    ready.sort_by_key(|&i| (found[i].message.is_some(), rank(i), i));
    ready
}

/// The row a group downloads from, and whether the user picked it: the host picked by hand
/// (when it can download), else the first ready host by preference, else one still being
/// checked. A broken host is never chosen. None = every host is broken.
pub(crate) fn mirror_choice(found: &[FoundItem], group: &[usize], prefs: &[String]) -> (Option<usize>, bool) {
    if let Some(&i) = group.iter().find(|&&i| found[i].host_pick && found[i].can_download()) {
        return (Some(i), true);
    }
    if let Some(&i) = ready_in_order(found, group, prefs).first() {
        return (Some(i), false);
    }
    (group.iter().copied().find(|&i| found[i].state == FoundState::Checking), false)
}

/// The backups a group's download carries: its other ready hosts, in the order they'd be tried.
pub(crate) fn mirror_backups(found: &[FoundItem], group: &[usize], chosen: usize, prefs: &[String]) -> Vec<Mirror> {
    ready_in_order(found, group, prefs)
        .into_iter()
        .filter(|&i| i != chosen)
        .map(|i| {
            let t = &found[i].template;
            Mirror {
                link: t.link.clone(),
                source: t.source.clone(),
                size: t.size,
                md5: t.md5.clone(),
                validator: t.validator.clone(),
                resumable: t.resumable,
                why: None,
            }
        })
        .collect()
}

pub(crate) fn mirror_views(found: &[FoundItem], prefs: &[String]) -> Vec<MirrorView> {
    mirror_groups(found)
        .into_iter()
        .map(|g| {
            let (chosen, picked) = mirror_choice(found, &g, prefs);
            let named = chosen.unwrap_or(g[0]);
            MirrorView {
                key: found[g[0]].id.clone(),
                name: found[named].template.name.clone(),
                size: g.iter().find_map(|&i| found[i].template.size),
                ids: g.iter().map(|&i| found[i].id.clone()).collect(),
                hosts: g.iter().map(|&i| host_key(&found[i].template.source).unwrap_or_default()).collect(),
                chosen: chosen.map(|i| found[i].id.clone()),
                picked,
            }
        })
        .collect()
}

/// Puts a host at the front of the remembered ones (most recent first, a short list).
pub(crate) fn prefer_host(prefs: &mut Vec<String>, host: &str) {
    prefs.retain(|h| h != host);
    prefs.insert(0, host.to_string());
    prefs.truncate(MIRROR_HOSTS_KEPT);
}

/// What a download does after it stopped with `status` / `msg`.
#[derive(Debug, PartialEq)]
pub(crate) enum NextHost {
    /// Nothing to do with hosts: stop as usual.
    Stay,
    /// Go on with the next backup host: from its bytes so far (`keep`) or from zero.
    Switch { keep: bool },
    /// The last host failed too: stop, saying every host was tried.
    AllTried,
}

/// Did the host itself give up on the file for good (not a temporary error, not this PC's
/// disk)? The file is gone (404/410), access refused, the host's daily limit, only for some
/// people, a web page instead of the file, or another final answer.
pub(crate) fn host_gave_up(status: &Status, msg: &str) -> bool {
    match status {
        Status::QuotaExceeded | Status::NeedsAccount => true,
        Status::Failed => {
            msg == super::WEB_PAGE
                || msg == NO_ANSWER
                || [
                    "File not found",
                    "The server refused access",
                    "The server answered",
                    "Google flagged",
                    "The owner turned off",
                    "Google Drive error",
                    "Google's download limit",
                    "Google only gives",
                ]
                .iter()
                .any(|p| msg.starts_with(p))
        }
        _ => false,
    }
}

/// A strong ETag (quoted, not weak); a Last-Modified date doesn't prove two files are the same.
fn strong_etag(v: &Option<String>) -> Option<&str> {
    v.as_deref().filter(|v| v.starts_with('"') && v.len() > 2)
}

/// The next host is surely the exact same file, so the bytes so far can stay: the same size,
/// both can resume, and the same ETag or the same MD5 as the hosts report them.
pub(crate) fn same_bytes(d: &Download, next: &Mirror) -> bool {
    let same_tag = matches!((strong_etag(&d.validator), strong_etag(&next.validator)), (Some(a), Some(b)) if a == b);
    let same_md5 = matches!((&d.md5, &next.md5), (Some(a), Some(b)) if a.eq_ignore_ascii_case(b));
    d.size.is_some() && d.size == next.size && d.resumable && next.resumable && (same_tag || same_md5)
}

/// After a download stopped: go on with another host, or stop as usual.
pub(crate) fn after_failure(d: &Download, status: &Status, msg: &str) -> NextHost {
    if matches!(d.source, Source::Video { .. }) || !host_gave_up(status, msg) {
        return NextHost::Stay;
    }
    match d.mirrors.first() {
        Some(next) => NextHost::Switch { keep: same_bytes(d, next) },
        None if !d.tried.is_empty() => NextHost::AllTried,
        None => NextHost::Stay,
    }
}

/// `after_failure` for a run that just ended. `by_pako`: Pako put it back in line by itself
/// (Google's daily limit, tried again every few hours): it stays on its host and keeps waiting
/// for the limit, rather than going round the hosts that already failed and ending as Failed
/// on one of them. Your own Try again goes round them.
pub(crate) fn next_host(d: &Download, status: &Status, msg: &str, by_pako: bool) -> NextHost {
    if by_pako { NextHost::Stay } else { after_failure(d, status, msg) }
}

/// The host a download uses, in the window's words ("example.com", "Google Drive").
pub(crate) fn host_label(link: &str, source: &Source) -> String {
    match source {
        Source::GDrive { .. } => "Google Drive".into(),
        _ => host_key(source)
            .or_else(|| url::Url::parse(link).ok().and_then(|u| u.host_str().map(str::to_string)))
            .unwrap_or_else(|| "the link".into()),
    }
}

/// Why a run ended when its host gave no answer with data for `NO_ANSWER_AFTER` while the
/// download had backups (see `AnswerClock`): a reason to change host, like a final error.
pub(crate) const NO_ANSWER: &str = "The host didn't answer for 5 minutes.";

/// How long a host may go without any answer with data before a download with backups moves on.
pub(crate) const NO_ANSWER_AFTER: Duration = Duration::from_secs(5 * 60);

/// When the host of a running download last answered with data. Each run (a start, a Resume,
/// a host change) has its own, so Pause and Resume start the 5 minutes again. `limit` is set
/// only for a download that has backups and isn't Pako's own retry after Google's daily limit;
/// without it the host is tried forever, as before. The time is passed in (tests use made-up
/// times).
#[derive(Debug, Clone, Copy)]
pub(crate) struct AnswerClock {
    last: std::time::Instant,
    limit: Option<Duration>,
}

impl AnswerClock {
    pub(crate) fn new(now: std::time::Instant) -> Self {
        Self { last: now, limit: None }
    }
    /// The 5-minute rule applies to this run.
    pub(crate) fn arm(&mut self, limit: Duration) {
        self.limit = Some(limit);
    }
    /// The host answered with data (or the silence wasn't its fault: no internet here).
    pub(crate) fn answered(&mut self, now: std::time::Instant) {
        self.last = self.last.max(now);
    }
    /// The host has been silent too long: time to move to a backup.
    pub(crate) fn silent(&self, now: std::time::Instant) -> bool {
        self.limit.is_some_and(|l| now.saturating_duration_since(self.last) >= l)
    }
    /// How long until the host counts as silent (None = never): a retry wait (a server asking
    /// for an hour's pause) never runs past it.
    pub(crate) fn left(&self, now: std::time::Instant) -> Option<Duration> {
        self.limit.map(|l| l.saturating_sub(now.saturating_duration_since(self.last)))
    }
}

/// Can the bytes so far be checked against the next host with samples? The same known size,
/// both can continue part-way, and something on disk to compare.
pub(crate) fn can_spot_check(d: &Download, next: &Mirror) -> bool {
    d.size.is_some() && d.size == next.size && d.resumable && next.resumable && d.segments.iter().any(|s| s.done > 0)
}

/// Moves a download to its next backup host (after `after_failure` said Switch). The failed
/// host is noted with why. The bytes so far stay when `keep` (the same strong ETag or MD5), or
/// stay until a spot check (`spot_check`) when the new host has the same size and can
/// continue part-way; else from zero. A host that didn't answer (`NO_ANSWER`) also goes to the
/// end of the backups, to be tried again after the others. Returns what the row says.
pub(crate) fn switch_host(d: &mut Download, keep: bool, why: &str) -> String {
    // Its spot check never answered (the host failed during it): the bytes on disk came from
    // an earlier host, so this host's tag or checksum says nothing about them. Samples again.
    let keep = keep && !d.spot_check;
    let next = d.mirrors.remove(0);
    let old = Mirror {
        link: d.link.clone(),
        source: d.source.clone(),
        size: d.size,
        md5: d.md5.clone(),
        validator: d.validator.clone(),
        resumable: d.resumable,
        why: Some(why.to_string()),
    };
    let from = host_label(&old.link, &old.source);
    let silent = why == NO_ANSWER;
    // The new host is tried (again): it is no longer among the hosts that failed.
    d.tried.retain(|m| m.link != next.link);
    if silent {
        d.mirrors.push(Mirror { why: None, ..old.clone() });
    }
    d.tried.push(old);
    let had = d.segments.iter().any(|s| s.done > 0);
    let check = !keep && can_spot_check(d, &next);
    let refused = if !had || keep || check {
        None
    } else if d.size.is_some() && next.size.is_some() && d.size != next.size {
        Some("has a different version of the file")
    } else {
        Some("can't continue a download part-way")
    };
    d.link = next.link.clone();
    d.source = next.source.clone();
    d.md5 = next.md5.clone();
    d.spot_check = check;
    if keep || check {
        // The same file (checked, or checked with samples before it goes on): the parts done
        // so far stay, resumed from the new host.
        d.validator = next.validator.clone();
        d.resumable = next.resumable;
        d.spliced = had;
    } else {
        // Asked again from the new host (its size, version, resuming). The size stays until
        // then so the file keeps its name.
        d.remove_part();
        d.segments.clear();
        d.downloaded = 0;
        d.validator = None;
        d.resolved = false;
        d.spliced = false;
        if d.size.is_none() {
            d.size = next.size;
        }
    }
    d.status = Status::Queued;
    let to = host_label(&d.link, &d.source);
    let reason = if silent { format!("{from} didn't answer for 5 minutes") } else { format!("{from} didn't work") };
    let msg = match refused {
        _ if keep && had => format!("{}: continuing from {to} where {from} stopped ({reason})", d.name),
        _ if check => format!("{}: continuing from {to} ({reason}), once a few samples show {to} has the same file", d.name),
        Some(r) => format!("{}: continuing from {to} ({reason}). {to} {r}: starting over", d.name),
        None => format!("{}: continuing from {to} ({reason})", d.name),
    };
    d.message = Some(msg.clone());
    msg
}

/// Most bytes in one spot-check sample.
pub(crate) const SAMPLE: u64 = 64 * 1024;

/// Where to take the spot-check samples ([start, end) in the file): only inside parts already
/// done on disk, near the start, around the middle of what's done, and just before where it
/// stopped (the end of the furthest done stretch). No duplicates; empty when nothing is done.
pub(crate) fn sample_ranges(segments: &[Segment]) -> Vec<(u64, u64)> {
    let mut done: Vec<(u64, u64)> = segments.iter().filter(|s| s.done > 0).map(|s| (s.start, s.start + s.done)).collect();
    done.sort_unstable();
    // Touching or overlapping stretches become one.
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for (a, b) in done {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    let Some(&(first_a, first_b)) = merged.first() else { return Vec::new() };
    let total: u64 = merged.iter().map(|(a, b)| b - a).sum();
    let mut out = vec![(first_a, first_a + SAMPLE.min(first_b - first_a))];
    // The middle of what's done, counted along the done stretches.
    let mut left = total / 2;
    for &(a, b) in &merged {
        if left < b - a {
            let len = SAMPLE.min(b - a);
            let s = (a + left).saturating_sub(len / 2).clamp(a, b - len);
            out.push((s, s + len));
            break;
        }
        left -= b - a;
    }
    let &(last_a, last_b) = merged.iter().max_by_key(|(_, b)| *b).unwrap();
    out.push((last_b - SAMPLE.min(last_b - last_a), last_b));
    let mut seen = Vec::new();
    out.retain(|r| {
        let fresh = !seen.contains(r);
        seen.push(*r);
        fresh
    });
    out
}

/// Why a spot check didn't say "the same file".
#[derive(Debug)]
pub(crate) enum SpotFail {
    /// A clear answer: the bytes so far can't be continued from this host (a difference,
    /// another size, no part-way answers, the unfinished file gone): it starts over from zero.
    Refused(String),
    /// A sample couldn't be had right now (the network, a busy server, a slow answer): asked
    /// again after the usual wait (Retry-After seconds, 0 = not said); the parts stay.
    Again(String, u64),
    /// Google Drive answered a sample oddly (a hiccup it has mid-download too): asked again a
    /// few times before it counts as a clear answer.
    Odd(String),
    /// The host gave up on the file for good (gone, refused, daily limit): the run ends with
    /// it, and the parts stay for the next host (checked again there).
    Gone(FetchError),
}

/// The spot check: a few samples from the new host (with the headers it will download with),
/// compared byte for byte with the parts already on disk (`part`, `size` bytes long). Ok = all
/// identical. A sample that can't be had is never taken as "different" (`SpotFail`): a network
/// hiccup never costs the bytes so far.
pub(crate) async fn spot_check(fetcher: &Fetcher, part: &std::path::Path, segments: &[Segment], size: u64) -> Result<(), SpotFail> {
    use std::io::{Read, Seek, SeekFrom};
    const DIFFERENT: &str = "has a different version of the file";
    let refused = |why: &str| SpotFail::Refused(why.into());
    let again = |why: &str| SpotFail::Again(why.into(), 0);
    let ranges = sample_ranges(segments);
    if ranges.is_empty() {
        return Err(refused("the unfinished file has nothing to compare"));
    }
    let mut file = std::fs::File::open(part).map_err(|_| refused("the unfinished file is missing"))?;
    if file.metadata().map(|m| m.len()).ok() != Some(size) {
        return Err(refused("the unfinished file isn't the right size"));
    }
    for (a, b) in ranges {
        let want = (b - a) as usize;
        let ours = {
            let mut buf = vec![0u8; want];
            file.seek(SeekFrom::Start(a)).and_then(|_| file.read_exact(&mut buf)).map_err(|_| refused("the unfinished file couldn't be read"))?;
            buf
        };
        let resp = match tokio::time::timeout(Duration::from_secs(30), fetcher.get(a, Some(b - 1))).await {
            Err(_) => return Err(again("The server didn't answer in time")),
            Ok(Err(FetchError::Retry(m))) => return Err(SpotFail::Again(m, 0)),
            Ok(Err(FetchError::Throttled(m, wait))) => return Err(SpotFail::Again(m, wait)),
            Ok(Err(e)) => return Err(SpotFail::Gone(e)),
            Ok(Ok(r)) => r,
        };
        let range = crate::fetch::content_range(&resp);
        if resp.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            if fetcher.is_drive() {
                return Err(SpotFail::Odd("Google sent an unexpected answer".into()));
            }
            return Err(refused("can't continue a download part-way"));
        }
        match range {
            Some((start, total)) if start == a && total.is_none_or(|t| t == size) => {}
            Some((_, Some(t))) if t != size => return Err(refused(DIFFERENT)),
            _ if fetcher.is_drive() => return Err(SpotFail::Odd("Google sent an unexpected answer".into())),
            _ => return Err(refused("answered a sample oddly")),
        }
        let mut theirs: Vec<u8> = Vec::with_capacity(want);
        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        loop {
            let next = tokio::time::timeout(Duration::from_secs(30), stream.next()).await.map_err(|_| again("The server stopped sending"))?;
            match next {
                Some(Ok(chunk)) => {
                    theirs.extend_from_slice(&chunk);
                    if theirs.len() > want {
                        return Err(refused("answered a sample oddly"));
                    }
                }
                Some(Err(e)) => return Err(again(&crate::fetch::network_message(&e))),
                None => break,
            }
        }
        if theirs.len() != want {
            return Err(again("The server closed the connection early"));
        }
        if theirs != ours {
            return Err(refused(DIFFERENT));
        }
    }
    Ok(())
}

/// What the row says once the spot check answered: `to` = the new host, `from` = the host
/// before; `result` = `spot_check`'s answer.
pub(crate) fn spot_note(name: &str, to: &str, from: &str, result: &Result<(), String>) -> String {
    match result {
        Ok(()) => format!("{name}: continuing from {to} where {from} stopped"),
        Err(r) if r.starts_with("the unfinished file") => {
            format!("{name}: {r}: starting over from {to}")
        }
        Err(r) => format!("{name}: {to} {r}: starting over"),
    }
}

/// A run's end when the finished file, continued from another host's bytes, failed its
/// checksum (`job_ended` starts it over from zero on the same host).
pub(crate) const SPLICE_FAILED: &str = "The file didn't pass its final check after changing host.";

/// What the row says then.
pub(crate) fn splice_failed_note(d: &Download) -> String {
    let to = host_label(&d.link, &d.source);
    format!("{}: the finished file didn't pass its check after continuing from {to}: starting over from zero on {to}", d.name)
}

/// Every host failed: the usual message, saying so. The hosts become backups again, so Try
/// again goes round them once more: the last host first (the download is on it), then the
/// others in the order they were tried.
pub(crate) fn all_hosts_failed(d: &mut Download, msg: &str) -> String {
    let mut hosts: Vec<String> = d.tried.iter().map(|m| host_label(&m.link, &m.source)).collect();
    hosts.push(host_label(&d.link, &d.source));
    let text = format!("{msg} All {} hosts were tried: {}.", hosts.len(), hosts.join(", "));
    let mut again: Vec<Mirror> = d.tried.drain(..).collect();
    for m in again.iter_mut() {
        m.why = None;
    }
    d.mirrors = again;
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use FoundState::*;

    fn row(name: &str, link: &str, size: Option<u64>, state: FoundState) -> FoundItem {
        let src = if link.contains("drive.google.com") {
            Source::GDrive { file_id: link.rsplit('/').next().unwrap().into(), resource_key: None, doc_type: None }
        } else {
            Source::Http { url: link.into() }
        };
        let mut t = new_download(link, src, name.into(), PathBuf::new(), None);
        t.size = size;
        FoundItem::new(t, state, None, false)
    }

    fn on(mut f: FoundItem, device: &str) -> FoundItem {
        f.device = Some(device.into());
        f
    }

    /// A video another PC sends joins the same video already downloading here, but never a
    /// private window's: that PC gets a download of its own, not the private one's id.
    #[test]
    fn a_video_from_another_pc_never_joins_a_private_download() {
        let url = "https://www.youtube.com/watch?v=abc123";
        let mut private = new_download(url, Source::Http { url: url.into() }, "v.mp4".into(), PathBuf::new(), None);
        private.private = true;
        assert_eq!(device_video_joins(std::slice::from_ref(&private), url), None, "the private one stays on this PC");
        let plain = new_download(url, Source::Http { url: url.into() }, "v.mp4".into(), PathBuf::new(), None);
        assert_eq!(device_video_joins(&[private, plain.clone()], url), Some(plain.id.clone()));
        let mut asking = plain.clone();
        asking.ask = true;
        assert_eq!(device_video_joins(&[asking], url), None, "one waiting in the Download with Pako window isn't joined");
        let mut done = plain;
        done.status = Status::Completed;
        assert_eq!(device_video_joins(&[done], url), None);
    }

    /// A link another PC sends to Links never merges into a private window's row of it (memory
    /// only, gone at quit): it gets a row of its own. Pastes and the browser skip it as before.
    #[test]
    fn a_link_from_another_pc_never_merges_into_a_private_links_row() {
        let link = "https://a.example/a.zip";
        let mut private = row("a.zip", link, Some(1), Ready);
        private.template.private = true;
        let list = std::slice::from_ref(&private);
        assert!(!in_links(list, link, true), "from another PC: a row of its own");
        assert!(in_links(list, link, false), "a paste, or the browser: skipped, as before");
        let plain = row("a.zip", link, Some(1), Ready);
        assert!(in_links(&[private.clone(), plain.clone()], link, true), "a normal row of it: skipped");
        assert!(!in_links(&[plain], "https://a.example/b.zip", false));
    }

    /// A row's "Download on" is saved with it (left out for This PC), and a Links list saved
    /// before it existed still loads.
    #[test]
    fn the_per_link_device_is_saved_and_old_lists_still_load() {
        let mut f = row("a.zip", "https://a.example/a.zip", Some(1), Ready);
        let plain = serde_json::to_string(&f).unwrap();
        assert!(!plain.contains("device"), "This PC: nothing written");
        f.device = Some("home-id".into());
        f.device_note = Some("Home PC is offline".into());
        let json = serde_json::to_string(&f).unwrap();
        let back: FoundItem = serde_json::from_str(&json).unwrap();
        assert_eq!((back.device.as_deref(), back.device_note.as_deref()), (Some("home-id"), Some("Home PC is offline")));
        // An older list: no such fields.
        let mut v: serde_json::Value = serde_json::from_str(&plain).unwrap();
        v.as_object_mut().unwrap().remove("device");
        let old: FoundItem = serde_json::from_value(v).unwrap();
        assert_eq!((old.device, old.device_note), (None, None));
    }

    /// Pressing Download on a mixed list: rows for this PC download here, the others go to
    /// their PC (one order per PC); a PC offline or no longer allowed keeps its rows here with
    /// a note; a playlist not opened yet, or a link page, isn't sent as it is.
    #[test]
    fn a_mixed_list_is_split_between_this_pc_and_devices() {
        let devices = vec![
            DeviceChoice { id: "home".into(), name: "Home PC".into(), online: true },
            DeviceChoice { id: "laptop".into(), name: "Laptop".into(), online: true },
            DeviceChoice { id: "office".into(), name: "Office PC".into(), online: false },
        ];
        let mut pl = video_item("https://www.youtube.com/playlist?list=PL1", "Mix".into(), None);
        pl.state = Checking;
        let found = vec![
            row("here.zip", "https://a.example/here.zip", Some(1), Ready),
            on(row("h1.zip", "https://a.example/h1.zip", Some(1), Ready), "home"),
            on(row("l1.zip", "https://a.example/l1.zip", Some(1), Ready), "laptop"),
            on(row("h2.zip", "https://a.example/h2.zip", Some(1), Checking), "home"),
            on(row("o1.zip", "https://a.example/o1.zip", Some(1), Ready), "office"),
            on(row("gone.zip", "https://a.example/gone.zip", Some(1), Ready), "unpaired"),
            on(pl, "home"),
            on(row("broken.zip", "https://a.example/broken.zip", None, Broken), "home"),
            on(row("notpicked.zip", "https://a.example/np.zip", Some(1), Ready), "home"),
        ];
        let ids: Vec<String> = found.iter().filter(|f| f.template.name != "notpicked.zip").map(|f| f.id.clone()).collect();
        let s = split_by_device(&found, &ids, &devices);
        assert_eq!(s.here, vec![found[0].id.clone()]);
        assert_eq!(s.send, vec![
            ("home".to_string(), "Home PC".to_string(), vec![found[1].id.clone(), found[3].id.clone()]),
            ("laptop".to_string(), "Laptop".to_string(), vec![found[2].id.clone()]),
        ]);
        let stay: std::collections::HashMap<_, _> = s.stay.iter().cloned().collect();
        assert_eq!(stay.len(), 3, "{stay:?}");
        assert!(stay[&found[4].id].starts_with("Office PC is offline"));
        assert_eq!(stay[&found[5].id], DEVICE_GONE);
        assert!(stay[&found[6].id].contains("playlist"), "a playlist isn't sent before it's opened");

        // A PC unpaired (or no longer letting this PC control it): its rows go back to This PC.
        let mut list = found.clone();
        assert!(fallback_devices(&mut list, &["home".into(), "laptop".into(), "office".into()]));
        assert_eq!(list[5].device, None);
        assert_eq!(list[5].device_note.as_deref(), Some(DEVICE_GONE));
        assert_eq!(list[1].device.as_deref(), Some("home"));
        assert!(!fallback_devices(&mut list, &["home".into(), "laptop".into(), "office".into()]), "nothing more to change");
    }

    /// A video sent by another PC: a download at once, in the quality and priority picked
    /// there, never held for the Download with Pako window.
    #[test]
    fn a_video_from_a_device_downloads_in_its_quality_and_priority() {
        let p = crate::video::Probe { title: "Trip".into(), ..Default::default() };
        let d = device_video("https://www.youtube.com/watch?v=a", p, 720, Priority::First, PathBuf::from("C:/D"));
        assert!(!d.ask);
        assert_eq!(d.priority, Priority::First);
        assert_eq!(d.dest_dir, PathBuf::from("C:/D"));
        let Source::Video { quality, audio, .. } = &d.source else { panic!() };
        assert_eq!((*quality, audio.clone()), (Some(720), None));
        assert!(d.name.ends_with(".mp4"), "{}", d.name);
        let p = crate::video::Probe { title: "Song".into(), ..Default::default() };
        let d = device_video("https://www.youtube.com/watch?v=b", p, 0, Priority::Later, PathBuf::from("C:/D"));
        let Source::Video { quality, audio, .. } = &d.source else { panic!() };
        assert_eq!((*quality, audio.as_deref(), d.priority), (Some(0), Some("m4a"), Priority::Later), "audio only");
    }

    /// A video the other PC was told was taken, then couldn't be read: kept as a failed
    /// download under the id it was given (that PC's card shows it failed; Retry works).
    #[test]
    fn a_video_from_a_device_that_fails_late_stays_as_a_failed_download() {
        let d = device_video_failed("https://www.youtube.com/watch?v=a", 720, Priority::First, PathBuf::from("C:/D/Mix"), "pre-made-id".into(), "Video unavailable");
        assert_eq!((d.id.as_str(), d.status, d.message.as_deref()), ("pre-made-id", Status::Failed, Some("Video unavailable")));
        assert_eq!((d.priority, d.dest_dir.clone(), d.link.as_str()), (Priority::First, PathBuf::from("C:/D/Mix"), "https://www.youtube.com/watch?v=a"));
        let Source::Video { quality, .. } = &d.source else { panic!() };
        assert_eq!(*quality, Some(720));
    }

    /// A playlist holding the same video twice is listed once in the picker card (its rows are
    /// keyed by link).
    #[test]
    fn picked_items_are_listed_once_per_link() {
        let it = |link: &str, name: &str| PickItem { link: link.into(), name: name.into(), video: true, duration: None, size: None, folder: None };
        let got = once_per_link(vec![it("https://y/1", "a"), it("https://y/2", "b"), it("https://y/1", "a again")]);
        assert_eq!(got.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
    }

    #[test]
    fn same_name_and_size_on_other_hosts_group() {
        let found = vec![
            row("Holiday.zip", "https://a.example/Holiday.zip", Some(100), Ready),
            row("holiday.ZIP", "https://www.b.example/f/holiday.ZIP", Some(100), Ready),
            row("Holiday.zip", "https://drive.google.com/file/d/X1", Some(100), Ready),
            row("Other.zip", "https://b.example/Other.zip", Some(100), Ready),
        ];
        assert_eq!(mirror_groups(&found), vec![vec![0, 1, 2]]);
        let v = &mirror_views(&found, &[])[0];
        assert_eq!(v.hosts, vec!["a.example", "b.example", "drive.google.com"]);
        assert_eq!(v.chosen.as_deref(), Some(found[0].id.as_str()));
    }

    #[test]
    fn never_by_name_alone_or_the_same_host() {
        let found = vec![
            // Different sizes.
            row("A.zip", "https://a.example/A.zip", Some(100), Ready),
            row("A.zip", "https://b.example/A.zip", Some(101), Ready),
            // Sizes unknown on both.
            row("B.zip", "https://a.example/B.zip", None, Ready),
            row("B.zip", "https://b.example/B.zip", None, Ready),
            // Same host twice.
            row("C.zip", "https://a.example/1/C.zip", Some(5), Ready),
            row("C.zip", "https://www.a.example/2/C.zip", Some(5), Ready),
            // Still being checked (no size yet): joins when its check ends.
            row("D.zip", "https://a.example/D.zip", Some(7), Ready),
            row("D.zip", "https://b.example/D.zip", None, Checking),
            // Two broken links: never by name alone.
            row("E.zip", "https://a.example/E.zip", None, Broken),
            row("E.zip", "https://b.example/E.zip", None, Broken),
        ];
        assert!(mirror_groups(&found).is_empty(), "{:?}", mirror_groups(&found));
    }

    #[test]
    fn same_host_twice_stays_out() {
        let found = vec![
            row("C.zip", "https://a.example/1/C.zip", Some(5), Ready),
            row("C.zip", "https://a.example/2/C.zip", Some(5), Ready),
            row("C.zip", "https://b.example/C.zip", Some(5), Ready),
        ];
        assert_eq!(mirror_groups(&found), vec![vec![0, 2]]);
    }

    #[test]
    fn broken_host_joins_by_name_and_is_never_chosen() {
        let mut found = vec![
            row("Holiday.zip", "https://a.example/Holiday.zip", None, Broken),
            row("Holiday.zip", "https://b.example/Holiday.zip", Some(100), Ready),
            row("Holiday.zip", "https://c.example/Holiday.zip", Some(100), Checking),
        ];
        let g = mirror_groups(&found);
        assert_eq!(g, vec![vec![0, 1, 2]]);
        assert_eq!(mirror_choice(&found, &g[0], &[]), (Some(1), false));
        // Picked by hand: a broken host still isn't used, nor carried as a backup.
        found[0].host_pick = true;
        assert_eq!(mirror_choice(&found, &g[0], &[]), (Some(1), false));
        assert!(mirror_backups(&found, &g[0], 1, &[]).is_empty());
        // Every host broken but one still being checked: that one.
        found[1].state = Broken;
        assert_eq!(mirror_choice(&found, &g[0], &[]).0, Some(2));
    }

    #[test]
    fn folders_and_videos_are_not_mirrored() {
        let mut a = row("A.zip", "https://drive.google.com/file/d/X1", Some(5), Ready);
        a.folder = Some("Pack".into());
        let b = row("A.zip", "https://b.example/A.zip", Some(5), Ready);
        let mut v = row("A.zip", "https://c.example/A.zip", Some(5), Ready);
        v.template.source = Source::Video {
            url: "https://c.example/A.zip".into(),
            quality: None,
            audio: None,
            options: vec![],
            audio_size: None,
            duration: None,
            streams: vec![],
            referrer: None,
            caught: false,
            recording: None,
        };
        assert!(mirror_groups(&[a, b, v]).is_empty());
    }

    #[test]
    fn split_parts_on_two_hosts_group_part_by_part() {
        let found = vec![
            row("Holiday.part1.rar", "https://a.example/Holiday.part1.rar", Some(10), Ready),
            row("Holiday.part2.rar", "https://a.example/Holiday.part2.rar", Some(4), Ready),
            row("Holiday.part1.rar", "https://b.example/Holiday.part1.rar", Some(10), Ready),
            row("Holiday.part2.rar", "https://b.example/Holiday.part2.rar", Some(4), Ready),
        ];
        assert_eq!(mirror_groups(&found), vec![vec![0, 2], vec![1, 3]]);
    }

    #[test]
    fn default_host_follows_preferences_then_link_order() {
        let mut found = vec![
            row("H.zip", "https://a.example/H.zip", Some(9), Ready),
            row("H.zip", "https://b.example/H.zip", Some(9), Ready),
            row("H.zip", "https://c.example/H.zip", Some(9), Ready),
        ];
        let g = mirror_groups(&found).remove(0);
        assert_eq!(mirror_choice(&found, &g, &[]), (Some(0), false));
        let mut prefs = vec!["c.example".to_string(), "b.example".to_string()];
        assert_eq!(mirror_choice(&found, &g, &prefs), (Some(2), false));
        // Backups: the other ready hosts, by preference.
        let hosts: Vec<String> = mirror_backups(&found, &g, 2, &prefs).iter().map(|m| host_key(&m.source).unwrap()).collect();
        assert_eq!(hosts, vec!["b.example", "a.example"]);
        // A preferred host that is broken is skipped.
        found[2].state = Broken;
        assert_eq!(mirror_choice(&found, &g, &prefs), (Some(1), false));
        // A host picked by hand wins.
        found[0].host_pick = true;
        assert_eq!(mirror_choice(&found, &g, &prefs), (Some(0), true));
        // The picked host goes to the front of the remembered ones.
        prefer_host(&mut prefs, "b.example");
        assert_eq!(prefs, vec!["b.example", "c.example"]);
    }

    fn with_backup(validator: Option<&str>, md5: Option<&str>, size: Option<u64>) -> Download {
        let mut d = new_download("https://a.example/H.zip", Source::Http { url: "https://a.example/H.zip".into() }, "H.zip".into(), PathBuf::new(), None);
        d.size = Some(100);
        d.resumable = true;
        d.validator = Some("\"abc\"".into());
        d.md5 = Some("ABCDEF".into());
        d.mirrors.push(Mirror {
            link: "https://b.example/H.zip".into(),
            source: Source::Http { url: "https://b.example/H.zip".into() },
            size,
            md5: md5.map(Into::into),
            validator: validator.map(Into::into),
            resumable: true,
            why: None,
        });
        d
    }

    const GONE: &str = "File not found on the server (404).";

    #[test]
    fn fallback_only_when_the_host_gives_up() {
        let d = with_backup(None, None, Some(100));
        let switch = NextHost::Switch { keep: false };
        assert_eq!(after_failure(&d, &Status::Failed, GONE), switch);
        assert_eq!(after_failure(&d, &Status::QuotaExceeded, crate::fetch::QUOTA_MESSAGE), switch);
        assert_eq!(after_failure(&d, &Status::NeedsAccount, crate::fetch::BLOCKED_MESSAGE), switch);
        assert_eq!(after_failure(&d, &Status::Failed, super::super::WEB_PAGE), switch);
        assert_eq!(after_failure(&d, &Status::Failed, "The server refused access (403). The link may have expired."), switch);
        assert_eq!(after_failure(&d, &Status::Failed, &crate::drive::friendly_api_error(404, None, None)), switch);
        // This PC's problems, or a file that changed on the server: no other host.
        for m in [
            "Not enough space on drive D:",
            "Can't create the file: denied",
            "Download stopped before the end. Click Retry.",
            "The file came out damaged (checksum doesn't match). Click Retry to download it again.",
            super::super::FILE_CHANGED,
        ] {
            assert_eq!(after_failure(&d, &Status::Failed, m), NextHost::Stay, "{m}");
        }
        // Temporary errors never end a download (it keeps retrying on the same host).
        assert_eq!(after_failure(&d, &Status::Retrying, "Server error (503)"), NextHost::Stay);
        // No backups at all: the usual failure.
        let mut lone = d.clone();
        lone.mirrors.clear();
        assert_eq!(after_failure(&lone, &Status::Failed, GONE), NextHost::Stay);
    }

    #[test]
    fn bytes_kept_only_for_the_exact_same_file() {
        let keep = NextHost::Switch { keep: true };
        let fresh = NextHost::Switch { keep: false };
        // Same size and the same ETag, or the same MD5.
        assert_eq!(after_failure(&with_backup(Some("\"abc\""), None, Some(100)), &Status::Failed, GONE), keep);
        assert_eq!(after_failure(&with_backup(None, Some("abcdef"), Some(100)), &Status::Failed, GONE), keep);
        // Another size, another ETag, nothing to compare: from zero.
        assert_eq!(after_failure(&with_backup(Some("\"abc\""), None, Some(99)), &Status::Failed, GONE), fresh);
        assert_eq!(after_failure(&with_backup(Some("\"xyz\""), None, Some(100)), &Status::Failed, GONE), fresh);
        assert_eq!(after_failure(&with_backup(None, None, Some(100)), &Status::Failed, GONE), fresh);
        // A date or a weak ETag is no proof.
        for v in ["Tue, 03 Nov 2026 10:00:00 GMT", "W/\"abc\""] {
            let mut d = with_backup(Some(v), None, Some(100));
            d.validator = Some(v.into());
            d.md5 = None;
            assert_eq!(after_failure(&d, &Status::Failed, GONE), fresh, "{v}");
        }
        // A host that can't resume: from zero.
        let mut d = with_backup(Some("\"abc\""), None, Some(100));
        d.mirrors[0].resumable = false;
        assert_eq!(after_failure(&d, &Status::Failed, GONE), fresh);
    }

    #[test]
    fn switching_then_every_host_failed() {
        let mut d = with_backup(None, None, Some(100));
        // The backup can't continue part-way: no spot check, from zero.
        d.mirrors[0].resumable = false;
        d.downloaded = 40;
        d.segments.push(Segment { start: 0, end: 100, done: 40 });
        d.resolved = true;
        let said = switch_host(&mut d, false, GONE);
        assert_eq!(said, "H.zip: continuing from b.example (a.example didn't work). b.example can't continue a download part-way: starting over");
        assert_eq!((d.link.as_str(), d.downloaded, d.segments.len(), d.resolved), ("https://b.example/H.zip", 0, 0, false));
        assert!(!d.spot_check && !d.spliced);
        assert_eq!(d.status, Status::Queued);
        assert_eq!(d.size, Some(100), "the size stays so the name is kept");
        assert_eq!(d.tried[0].why.as_deref(), Some(GONE));
        // The second host fails too.
        assert_eq!(after_failure(&d, &Status::Failed, GONE), NextHost::AllTried);
        let text = all_hosts_failed(&mut d, GONE);
        assert_eq!(text, "File not found on the server (404). All 2 hosts were tried: a.example, b.example.");
        // Try again goes round them once more, from the first.
        assert!(d.tried.is_empty());
        assert_eq!(d.mirrors.iter().map(|m| m.link.as_str()).collect::<Vec<_>>(), vec!["https://a.example/H.zip"]);
        assert!(d.mirrors[0].why.is_none());
        // Kept bytes: the parts stay.
        let mut k = with_backup(Some("\"abc\""), None, Some(100));
        k.downloaded = 40;
        k.segments.push(Segment { start: 0, end: 100, done: 40 });
        k.resolved = true;
        let said = switch_host(&mut k, true, GONE);
        assert_eq!(said, "H.zip: continuing from b.example where a.example stopped (a.example didn't work)");
        assert_eq!((k.downloaded, k.segments.len(), k.resolved), (40, 1, true));
        assert!(k.spliced && !k.spot_check, "kept on the ETag: no samples, but the final check starts over if it fails");
    }

    #[test]
    fn the_same_size_on_a_host_that_resumes_gets_a_spot_check() {
        let mut d = with_backup(None, None, Some(100));
        d.downloaded = 40;
        d.segments.push(Segment { start: 0, end: 100, done: 40 });
        d.resolved = true;
        let said = switch_host(&mut d, false, GONE);
        assert_eq!(said, "H.zip: continuing from b.example (a.example didn't work), once a few samples show b.example has the same file");
        // The parts stay (until the samples answer), and the file isn't asked again.
        assert_eq!((d.downloaded, d.segments.len(), d.resolved), (40, 1, true));
        assert!(d.spot_check && d.spliced);
        // Another size: from zero, and why.
        let mut s = with_backup(None, None, Some(99));
        s.downloaded = 40;
        s.segments.push(Segment { start: 0, end: 100, done: 40 });
        let said = switch_host(&mut s, false, GONE);
        assert_eq!(said, "H.zip: continuing from b.example (a.example didn't work). b.example has a different version of the file: starting over");
        assert!(!s.spot_check && s.segments.is_empty());
        // Nothing on disk yet: nothing to check, plainly from the next host.
        let mut z = with_backup(None, None, Some(100));
        assert_eq!(switch_host(&mut z, false, GONE), "H.zip: continuing from b.example (a.example didn't work)");
        assert!(!z.spot_check && !z.spliced);
        // b.example failed before its samples answered, and c.example has b.example's ETag:
        // the bytes on disk are still a.example's, so c.example is checked with samples too.
        d.validator = Some("\"b1\"".into());
        d.mirrors.push(Mirror {
            link: "https://c.example/H.zip".into(),
            source: Source::Http { url: "https://c.example/H.zip".into() },
            size: Some(100),
            md5: None,
            validator: Some("\"b1\"".into()),
            resumable: true,
            why: None,
        });
        assert_eq!(after_failure(&d, &Status::Failed, GONE), NextHost::Switch { keep: true });
        let said = switch_host(&mut d, true, GONE);
        assert_eq!(said, "H.zip: continuing from c.example (b.example didn't work), once a few samples show c.example has the same file");
        assert!(d.spot_check && d.spliced);
        assert_eq!((d.downloaded, d.segments.len()), (40, 1), "the parts stay until the samples answer");
    }

    #[test]
    fn a_silent_host_gives_way_and_goes_to_the_end_of_the_backups() {
        let mut d = with_backup(None, None, Some(100));
        d.mirrors.push(Mirror { link: "https://c.example/H.zip".into(), source: Source::Http { url: "https://c.example/H.zip".into() }, size: Some(100), md5: None, validator: None, resumable: true, why: None });
        // Five minutes without an answer is a reason to change host, like a final error.
        assert_eq!(after_failure(&d, &Status::Failed, NO_ANSWER), NextHost::Switch { keep: false });
        let said = switch_host(&mut d, false, NO_ANSWER);
        assert_eq!(said, "H.zip: continuing from b.example (a.example didn't answer for 5 minutes)");
        let links = |d: &Download| d.mirrors.iter().map(|m| m.link.clone()).collect::<Vec<_>>();
        assert_eq!(links(&d), vec!["https://c.example/H.zip", "https://a.example/H.zip"], "tried again after the others");
        assert_eq!(d.tried.last().and_then(|m| m.why.as_deref()), Some(NO_ANSWER), "the details say why");
        // b.example is gone for good: c.example, then a.example again (no longer "tried").
        switch_host(&mut d, false, GONE);
        assert_eq!(d.link, "https://c.example/H.zip");
        switch_host(&mut d, false, GONE);
        assert_eq!(d.link, "https://a.example/H.zip");
        assert!(d.mirrors.is_empty());
        assert!(!d.tried.iter().any(|m| m.link == d.link), "{:?}", d.tried);
        // Every other host failed for good: a silent host without backups keeps being tried
        // (its run is never stopped: see AnswerClock); a final error ends as before.
        assert_eq!(after_failure(&d, &Status::Failed, GONE), NextHost::AllTried);
        assert_eq!(all_hosts_failed(&mut d, GONE), "File not found on the server (404). All 3 hosts were tried: b.example, c.example, a.example.");
        // Without any backups at all: nothing changes.
        let mut lone = with_backup(None, None, Some(100));
        lone.mirrors.clear();
        assert_eq!(after_failure(&lone, &Status::Failed, NO_ANSWER), NextHost::Stay);
    }

    #[test]
    fn the_five_minute_clock() {
        let t0 = std::time::Instant::now();
        let min = |m: u64, s: u64| t0 + Duration::from_secs(m * 60 + s);
        // Without backups (not armed): never silent.
        let mut c = AnswerClock::new(t0);
        assert!(!c.silent(min(600, 0)));
        // With backups: 5 minutes in a row with no answer.
        c.arm(NO_ANSWER_AFTER);
        assert!(!c.silent(min(4, 59)));
        assert_eq!(c.left(min(4, 0)), Some(Duration::from_secs(60)));
        assert!(c.silent(min(5, 0)));
        assert_eq!(c.left(min(9, 0)), Some(Duration::ZERO));
        // An answer with data starts the 5 minutes again.
        let mut c = AnswerClock::new(t0);
        c.arm(NO_ANSWER_AFTER);
        c.answered(min(3, 0));
        assert!(!c.silent(min(7, 59)));
        assert!(c.silent(min(8, 0)));
        // An older time never moves the clock back.
        c.answered(min(1, 0));
        assert!(!c.silent(min(7, 59)));
        // Pause and Resume: a new run, a new clock.
        let resumed = AnswerClock::new(min(20, 0));
        let mut r = resumed;
        r.arm(NO_ANSWER_AFTER);
        assert!(!r.silent(min(24, 0)));
    }

    #[test]
    fn samples_come_from_parts_done_on_disk() {
        const K: u64 = 1024;
        let seg = |start, end, done| Segment { start, end, done };
        assert!(sample_ranges(&[seg(0, 1000, 0)]).is_empty());
        // One part, 1 MB done of 10 MB: the start, the middle of what's done, just before the stop.
        let one = sample_ranges(&[seg(0, 10_000 * K, 1024 * K)]);
        assert_eq!(one, vec![(0, 64 * K), (480 * K, 544 * K), (960 * K, 1024 * K)]);
        // A little done: shorter samples, no duplicates, never past what's done.
        assert_eq!(sample_ranges(&[seg(0, 1000, 10)]), vec![(0, 10)]);
        // Four parts, each part-way: only inside the done stretches.
        let segs = [seg(0, 1000 * K, 300 * K), seg(1000 * K, 2000 * K, 0), seg(2000 * K, 3000 * K, 500 * K), seg(3000 * K, 4000 * K, 30 * K)];
        let r = sample_ranges(&segs);
        let inside = |(a, b): (u64, u64)| segs.iter().any(|s| a >= s.start && b <= s.start + s.done);
        assert!(r.iter().all(|&x| inside(x)), "{r:?}");
        assert_eq!(r.first(), Some(&(0, 64 * K)));
        assert_eq!(r.last(), Some(&(3000 * K, 3030 * K)), "just before where the furthest part stopped");
        assert_eq!(r.len(), 3);
    }

    /// A tiny host for the spot check: serves `body` (the whole file), answering ranges as
    /// `answer` says.
    #[derive(Clone, Copy, PartialEq)]
    enum Host {
        Ranges,
        /// Content-Range says another size.
        OtherSize,
        /// Ignores Range: the whole file with 200.
        NoRanges,
        /// The second sample answers 503.
        FailSecond,
        /// The second sample answers 404 (the file is gone from this host).
        GoneSecond,
    }

    async fn host(body: Vec<u8>, how: Host, seen: std::sync::Arc<std::sync::Mutex<Vec<String>>>) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { return };
                let (body, seen, count) = (body.clone(), seen.clone(), count.clone());
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut tmp = [0u8; 4096];
                    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        let Ok(n) = sock.read(&mut tmp).await else { return };
                        if n == 0 {
                            return;
                        }
                        buf.extend_from_slice(&tmp[..n]);
                    }
                    let head = String::from_utf8_lossy(&buf).to_string();
                    seen.lock().unwrap().push(head.clone());
                    let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let range = head
                        .lines()
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("range: bytes=").map(str::to_string))
                        .and_then(|r| {
                            let (a, b) = r.trim().split_once('-')?;
                            Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?))
                        });
                    let total = body.len();
                    let out: Vec<u8> = match (how, range) {
                        (Host::FailSecond, _) if n == 1 => b"HTTP/1.1 503 Busy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                        (Host::GoneSecond, _) if n == 1 => {
                            let mut o = b"HTTP/1.1 404 Not Found".to_vec();
                            o.extend_from_slice(b"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                            o
                        }
                        (Host::NoRanges, _) | (_, None) => {
                            let mut o = format!("HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n").into_bytes();
                            o.extend_from_slice(&body);
                            o
                        }
                        (_, Some((a, b))) => {
                            let said = if how == Host::OtherSize { total + 1 } else { total };
                            let part = &body[a..=b.min(total - 1)];
                            let mut o = format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {a}-{}/{said}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                a + part.len() - 1,
                                part.len()
                            )
                            .into_bytes();
                            o.extend_from_slice(part);
                            o
                        }
                    };
                    let _ = sock.write_all(&out).await;
                });
            }
        });
        format!("http://{addr}/H.zip")
    }

    /// The spot check deciding: identical samples continue; one different byte, another size or
    /// no ranges start over. A sample it can't get is asked again (never "different"), and a
    /// host that gives up ends the run with the parts kept. Samples use the download's headers.
    #[tokio::test]
    async fn the_spot_check_decides() {
        let size = 400 * 1024usize;
        let file: Vec<u8> = (0..size).map(|i| (i * 7 % 251) as u8).collect();
        let dir = std::env::temp_dir().join(format!("pako-spot-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let part = dir.join("H.zip.pakopart");
        // On disk: the first 300 KB done (the rest zeros), in two parts.
        let mut ours = file.clone();
        for b in &mut ours[300 * 1024..] {
            *b = 0;
        }
        std::fs::write(&part, &ours).unwrap();
        let segs = vec![
            Segment { start: 0, end: 200 * 1024, done: 200 * 1024 },
            Segment { start: 200 * 1024, end: size as u64, done: 100 * 1024 },
        ];
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        // A clear "no" as its reason; anything else as what the run does with it.
        let said = |r: std::result::Result<(), SpotFail>| -> std::result::Result<(), String> {
            r.map_err(|e| match e {
                SpotFail::Refused(why) => why,
                SpotFail::Again(why, _) => format!("again: {why}"),
                SpotFail::Odd(why) => format!("odd: {why}"),
                SpotFail::Gone(e) => format!("gone: {}", e.message()),
            })
        };
        let check = |url: String, validator: Option<&str>| {
            let mut f = Fetcher::new(crate::fetch::download_client(), &Source::Http { url });
            f.set_validator(validator.map(Into::into));
            f
        };
        // Identical: continue.
        let url = host(file.clone(), Host::Ranges, seen.clone()).await;
        assert_eq!(said(spot_check(&check(url, Some("\"b1\"")), &part, &segs, size as u64).await), Ok(()));
        {
            let seen = seen.lock().unwrap();
            assert_eq!(seen.len(), 3, "three samples");
            assert!(seen.iter().all(|h| h.to_ascii_lowercase().contains("if-range: \"b1\"")), "the headers it will download with");
            assert!(seen.iter().all(|h| !h.contains("bytes=30")), "never past what's done: {seen:?}");
        }
        // One byte differs (at the start of the last sample).
        let mut other = file.clone();
        other[300 * 1024 - 64 * 1024] ^= 1;
        let url = host(other, Host::Ranges, seen.clone()).await;
        assert_eq!(said(spot_check(&check(url, None), &part, &segs, size as u64).await), Err("has a different version of the file".into()));
        // Another size in its answer.
        let url = host(file.clone(), Host::OtherSize, seen.clone()).await;
        assert_eq!(said(spot_check(&check(url, None), &part, &segs, size as u64).await), Err("has a different version of the file".into()));
        // No ranges.
        let url = host(file.clone(), Host::NoRanges, seen.clone()).await;
        assert_eq!(said(spot_check(&check(url, None), &part, &segs, size as u64).await), Err("can't continue a download part-way".into()));
        // A sample that fails.
        let url = host(file.clone(), Host::FailSecond, seen.clone()).await;
        let r = said(spot_check(&check(url, None), &part, &segs, size as u64).await);
        assert_eq!(r, Err("again: Server error (503)".into()), "a busy host costs nothing: asked again");
        // A host with nothing listening (down, or no network here): asked again too.
        let dead = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let a = l.local_addr().unwrap();
            drop(l);
            format!("http://{a}/H.zip")
        };
        let r = said(spot_check(&check(dead, None), &part, &segs, size as u64).await);
        assert!(r.as_ref().is_err_and(|e| e.starts_with("again: ")), "{r:?}");
        // The file gone from the new host mid-check: the run ends with that, the parts stay.
        let url = host(file.clone(), Host::GoneSecond, seen.clone()).await;
        let r = said(spot_check(&check(url, None), &part, &segs, size as u64).await);
        assert!(r.as_ref().is_err_and(|e| e.starts_with("gone: File not found")), "{r:?}");
        // What the row says.
        assert_eq!(spot_note("H.zip", "b.example", "a.example", &Ok(())), "H.zip: continuing from b.example where a.example stopped");
        assert_eq!(
            spot_note("H.zip", "b.example", "a.example", &Err("has a different version of the file".into())),
            "H.zip: b.example has a different version of the file: starting over"
        );
        // The unfinished file gone: never a guess.
        std::fs::remove_file(&part).unwrap();
        let url = host(file, Host::Ranges, seen).await;
        assert_eq!(said(spot_check(&check(url, None), &part, &segs, size as u64).await), Err("the unfinished file is missing".into()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pakos_own_retry_stays_on_its_host() {
        // Google's daily limit on the last host, after the first one was gone: tried again by
        // Pako itself, still limited. It keeps waiting there (not off to the dead host, then Failed).
        let mut d = with_backup(None, None, Some(100));
        switch_host(&mut d, false, GONE);
        all_hosts_failed(&mut d, crate::fetch::QUOTA_MESSAGE);
        assert_eq!(d.mirrors.len(), 1);
        assert_eq!(next_host(&d, &Status::QuotaExceeded, crate::fetch::QUOTA_MESSAGE, true), NextHost::Stay);
        // Your own Try again goes round the hosts.
        assert_eq!(next_host(&d, &Status::QuotaExceeded, crate::fetch::QUOTA_MESSAGE, false), NextHost::Switch { keep: false });
    }

    #[test]
    fn a_host_at_its_daily_limit_is_not_the_default() {
        let mut found = vec![
            row("H.zip", "https://drive.google.com/file/d/X1", Some(9), Ready),
            row("H.zip", "https://b.example/H.zip", Some(9), Ready),
        ];
        found[0].message = Some(crate::fetch::QUOTA_MESSAGE.into());
        let g = mirror_groups(&found).remove(0);
        assert_eq!(mirror_choice(&found, &g, &[]), (Some(1), false));
        assert_eq!(mirror_backups(&found, &g, 1, &[]).len(), 1, "still a backup");
    }

    #[test]
    fn the_same_file_twice_on_each_host_makes_two_groups() {
        let found = vec![
            row("C.zip", "https://a.example/1/C.zip", Some(5), Ready),
            row("C.zip", "https://a.example/2/C.zip", Some(5), Ready),
            row("C.zip", "https://b.example/1/C.zip", Some(5), Ready),
            row("C.zip", "https://b.example/2/C.zip", Some(5), Broken),
        ];
        assert_eq!(mirror_groups(&found), vec![vec![0, 2], vec![1, 3]]);
    }

    #[test]
    fn many_links_group_quickly() {
        let mut found: Vec<FoundItem> =
            (0..5000).map(|i| row(&format!("F{i}.zip"), &format!("https://a.example/F{i}.zip"), Some(i), Ready)).collect();
        found.push(row("F7.zip", "https://b.example/F7.zip", Some(7), Ready));
        let t = std::time::Instant::now();
        assert_eq!(mirror_groups(&found), vec![vec![7, 5000]]);
        assert!(t.elapsed() < Duration::from_millis(500), "{:?}", t.elapsed());
    }
}
