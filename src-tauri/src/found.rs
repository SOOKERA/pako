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

/// A video row in Links, still to be checked.
fn video_item(link: &str, title: String, duration: Option<f64>) -> FoundItem {
    let src = Source::Video { url: link.into(), quality: None, audio: None, options: Vec::new(), audio_size: None, duration, streams: Vec::new() };
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
        if let Ok(bytes) = serde_json::to_vec(&inner.found) {
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
        let mut added = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();
        let mut to_check = Vec::new();
        let mut to_open = Vec::new();
        for link in extract_links(text) {
            if self.inner.lock().unwrap().found.iter().any(|f| f.template.link == link) {
                skipped += 1;
                continue;
            }
            match self.found_items(&link, no_video).await {
                Ok(items) => {
                    let mut inner = self.inner.lock().unwrap();
                    // Checked again now: another paste of the same link may have landed while
                    // this one was being opened. (A Drive folder's files all carry its link.)
                    if inner.found.iter().any(|f| f.template.link == link) {
                        skipped += 1;
                        continue;
                    }
                    for mut it in items {
                        it.no_video = no_video;
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
                        crate::errorlog::note(crate::errorlog::Area::LinkCheck, &e.message());
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
                    crate::errorlog::note(crate::errorlog::Area::Video, &msg);
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
    pub async fn video_from_browser(self: &Arc<Self>, url: String, quality: u32, cookies: Vec<crate::extension::SiteCookie>) -> Result<(), String> {
        /// Videos being read right now (a double click doesn't add one twice).
        static READING: LazyLock<std::sync::Mutex<HashSet<String>>> = LazyLock::new(Default::default);
        if !crate::video::ready() {
            return Err("Pako needs its video tools first. In Pako: Settings › Features & updates › Download videos from YouTube and other sites.".into());
        }
        let already = self
            .inner
            .lock()
            .unwrap()
            .downloads
            .iter()
            .any(|d| d.link == url && !matches!(d.status, Status::Completed | Status::Failed));
        if already || !READING.lock().unwrap().insert(url.clone()) {
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
                    me.found_add(&url).await;
                    me.notify("browser", "Playlist sent to Pako", "Pick the videos in Links, then press Download.", crate::notices::Go::page("found"));
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
                    };
                    let mut d = new_download(&url, src, name.clone(), me.settings().download_dir, None);
                    d.size = size;
                    // The site's login stays with this download (encrypted) for yt-dlp.
                    if !cookies.is_empty() {
                        crate::extension::remember_headers(&d.id, crate::extension::BrowserHeaders { site_cookies: cookies.clone(), ..Default::default() });
                    }
                    // The Download with Pako window first (quality, folder, name), when it's on.
                    d.ask = crate::ask::wanted();
                    if d.ask {
                        d.message = Some(crate::engine::ASK_NOTE.into());
                    }
                    let (id, held) = (d.id.clone(), d.ask);
                    me.push(d);
                    me.save_now();
                    me.list_changed();
                    me.schedule();
                    if held {
                        crate::ask::enqueue(&id);
                    } else {
                        me.notify("browser", "Downloading in Pako", &name, crate::notices::Go::item("downloads", id.clone()));
                        crate::ask::started();
                    }
                    let _ = answer.send(Ok(()));
                }
                Err((msg, newer)) => {
                    if newer {
                        me.update_video_tool_soon();
                    }
                    // The button says it when it is still waiting; else a notification does.
                    if answer.send(Err(msg.clone())).is_err() {
                        me.notify("video-failed", "Pako couldn't get this video", &msg, crate::notices::Go::none());
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
        crate::errorlog::note(crate::errorlog::Area::LinkCheck, msg);
        if let Some(f) = self.inner.lock().unwrap().found.iter_mut().find(|f| f.id == id) {
            f.state = FoundState::Broken;
            f.message = Some(msg.to_string());
        }
        self.save_found();
        self.list_changed();
    }

    /// The gate led to the real file: the row becomes that file (or a Drive folder's files),
    /// keeping its own settings (folder, priority, notify).
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
                        crate::errorlog::note(crate::errorlog::Area::LinkCheck, &format!("Reached the file link, but: {e}"));
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
            let picked: Vec<FoundItem> =
                inner.found.iter().filter(|f| ids.contains(&f.id) && f.can_download()).cloned().collect();
            let gone: HashSet<&str> = picked.iter().map(|f| f.id.as_str()).collect();
            inner.found.retain(|f| !gone.contains(f.id.as_str()));
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

    /// Picked links whose file already exists where it would be saved (a duplicate on disk).
    pub fn found_existing(&self, ids: &[String], dest: Option<PathBuf>) -> Vec<String> {
        // Where each would land, read under the lock; the disk is checked after letting go of it.
        let places: Vec<(String, Vec<PathBuf>)> = {
            let inner = self.inner.lock().unwrap();
            let fallback = dest.unwrap_or_else(|| inner.settings.download_dir.clone());
            inner
                .found
                .iter()
                .filter(|f| ids.contains(&f.id))
                .map(|f| {
                    let base = f.dest.clone().unwrap_or_else(|| fallback.clone());
                    let name = &f.template.name;
                    // Plain files land in the folder; Drive folder files inside a folder named after it.
                    let mut paths = vec![base.join(name)];
                    if let Some(g) = &f.folder {
                        paths.push(base.join(g).join(name));
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

    pub fn found_remove(&self, ids: &[String]) {
        self.inner.lock().unwrap().found.retain(|f| !ids.contains(&f.id));
        self.save_found();
        self.list_changed();
    }
}
