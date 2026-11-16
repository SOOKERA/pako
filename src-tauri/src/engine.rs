// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! The download engine.
//!
//! - Each file is split into parts downloaded over several connections at once.
//! - When a connection finishes its part, it takes over half of the biggest part still
//!   in progress (like IDM), so all connections stay busy until the end.
//! - Any connection problem: wait (2s, 4s, 8s ... up to 60s) and continue from the last
//!   byte written. It never gives up by itself; only "file not found"-type answers stop it.
//! - Progress (which bytes of which part are done) is saved every 2 seconds, so closing the
//!   app or a PC restart resumes where it stopped.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use futures_util::StreamExt;
use md5::{Digest, Md5};
use serde::Serialize;
/// Sends events to the window (or prints them in test mode).
pub type Emit = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;
use tokio_util::sync::CancellationToken;

use crate::drive::{self, DriveApi, LinkKind};
use crate::fetch::{FetchError, Fetcher, probe_info};
use crate::limiter::Limiter;
use crate::net::Net;
use crate::model::*;
use crate::names;
use crate::notices::{self, Go, Pop};
use crate::paths::{data_dir, write_atomic};

#[path = "found.rs"]
mod found;

/// Smallest part worth giving its own connection.
const MIN_SPLIT: u64 = 1024 * 1024;
/// Write to disk in blocks of this size.
const FLUSH_AT: usize = 512 * 1024;
const MAX_CONNECTIONS: u32 = 32;
const FILE_CHANGED: &str =
    "The file on the server changed since this download started. Use Start over to download it again.";

/// Uploads (Send) running right now. Send counts them up and down; while one runs, "when all
/// downloads finish" waits, and the PC is kept awake (when that setting is on).
pub static UPLOADS_RUNNING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Other transfers that count like an upload (Direct sends and receives): told once at start by
/// whoever runs them (see `also_busy_when`), so the engine needs to know nothing about them.
static OTHER_TRANSFERS: std::sync::OnceLock<Box<dyn Fn() -> bool + Send + Sync>> = std::sync::OnceLock::new();

/// Registers the check for other transfers running (e.g. Direct's `Node::busy`): while it says
/// yes, "when all downloads finish" waits and the PC is kept awake (when that setting is on).
/// Called without the engine's lock held, so the check may take its own locks.
// Registered by the app's start-up (lib.rs) once Direct is set up.
pub fn also_busy_when(check: impl Fn() -> bool + Send + Sync + 'static) {
    let _ = OTHER_TRANSFERS.set(Box::new(check));
}

/// An upload (Send) or another registered transfer (Direct) is running right now. Never call it
/// while holding the engine's lock (the registered check takes other locks).
pub fn transfers_running() -> bool {
    UPLOADS_RUNNING.load(Ordering::Relaxed) > 0 || OTHER_TRANSFERS.get().is_some_and(|f| f())
}

/// The download list, saved in the background (see `paths::Saver`).
static LIST_SAVER: std::sync::LazyLock<crate::paths::Saver> =
    std::sync::LazyLock::new(|| crate::paths::Saver::new(list_path()));
/// Longest wait for a save to reach the disk ("save now", closing Pako).
const SAVE_WAIT: Duration = Duration::from_secs(15);

const SETTINGS_UNREAD: &str = "Pako couldn't read your settings when it started, so it won't save over them. \
     This change lasts until Pako closes. Restart Pako to get your settings back.";

/// Disk work (writing, flushing) from async code: other tasks on this thread carry on meanwhile
/// instead of waiting for a slow disk.
fn off_async<R>(f: impl FnOnce() -> R) -> R {
    match tokio::runtime::Handle::try_current() {
        Ok(h) if h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => tokio::task::block_in_place(f),
        _ => f(),
    }
}

pub struct Engine {
    emit: Emit,
    http: reqwest::Client,
    limiter: Arc<Limiter>,
    net: Arc<Net>,
    /// Servers that said "too many requests": (connection limit, until when).
    host_limits: Mutex<HashMap<String, (u32, Instant)>>,
    inner: Mutex<Inner>,
    /// History for the Dashboard.
    stats: Mutex<crate::stats::Stats>,
    /// Last time a point was added to the history.
    last_sample: Mutex<Instant>,
    /// "Finished" / "stopped" notifications waiting a moment, so several finishing together
    /// become one.
    notes: Mutex<(Vec<Note>, Option<Instant>)>,
}

struct Inner {
    downloads: Vec<Download>,
    /// Links waiting to be picked (the Found tab).
    found: Vec<FoundItem>,
    settings: Settings,
    jobs: HashMap<String, Arc<Job>>,
    speeds: HashMap<String, Speed>,
    dirty: bool,
    last_save: Instant,
    /// Progress was being sent last tick (so one final empty update clears the speeds).
    had_jobs: bool,
    /// The saved list was read (or there was none). False = it exists but couldn't be read at
    /// start-up: nothing is saved over it this session.
    loaded_ok: bool,
    /// Same for the Links list and the settings.
    found_ok: bool,
    settings_ok: bool,
    /// Problems reading the saved files at start-up, told once the engine starts (title, text).
    start_notes: Vec<(String, String)>,
    /// Both clocks at the last look (a big jump between them = the PC was asleep).
    last_look: Option<(Instant, std::time::SystemTime)>,
    /// "When all downloads finish": what to do (this session only; back to nothing each start).
    when_done: WhenDone,
    /// The countdown before doing it (None = no countdown running).
    when_done_at: Option<Instant>,
    /// Something was downloading or waiting since the list last went quiet.
    had_work: bool,
    /// A download finished since then, and nothing was paused after it: the list went quiet
    /// because the work is done (not because of "Pause all" or only failures).
    done_any: bool,
    /// Scheduler: inside the download hours at the last look (None = not looked yet).
    sched_open: Option<bool>,
    /// "Download first" outside the download hours: these run anyway.
    sched_override: HashSet<String>,
    /// Google's daily limit: when each blocked download was first seen blocked (tried again
    /// by itself a few hours later: the limit resets daily).
    quota_seen: HashMap<String, Instant>,
    /// Downloads Pako put back in line by itself (Google's daily limit), with the status they
    /// had: stopping the same way again isn't news (see `stop_is_news`). Your own Try again,
    /// Resume or Start over clears it.
    auto_retry: HashMap<String, Status>,
}

/// A download's "finished" or "stopped" notification, waiting a moment (see `flush_notes`).
#[derive(Clone, Debug)]
struct Note {
    title: String,
    body: String,
    /// What it is about (a file's or a folder's name), for a gathered notification.
    name: String,
    /// Stopped (limit, needs an account, error), else finished.
    stopped: bool,
    /// The download, when the note is about one file (a folder or set has none).
    id: Option<String>,
    /// Its own "Notify when done" asked for it: the pop-up shows whatever Settings say.
    force: bool,
}

/// What to do when every download has finished.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WhenDone {
    Nothing,
    Sleep,
    ShutDown,
}

impl WhenDone {
    pub fn parse(s: &str) -> Self {
        match s {
            "sleep" => WhenDone::Sleep,
            "shutDown" => WhenDone::ShutDown,
            _ => WhenDone::Nothing,
        }
    }
    fn name(self) -> &'static str {
        match self {
            WhenDone::Nothing => "nothing",
            WhenDone::Sleep => "sleep",
            WhenDone::ShutDown => "shutDown",
        }
    }
}

/// Seconds between "all done" and sleeping / shutting down (time to cancel).
const WHEN_DONE_SECS: u64 = 60;

/// One look at the list for "when all downloads finish". `had_work`: something ran or waited
/// since the list was last quiet; `done_any`: a download finished since then and nothing was
/// paused after it. True = the list just went quiet because the work is done: start the
/// countdown. Both flags start again from nothing once the list is quiet.
fn all_done_now(had_work: &mut bool, done_any: &mut bool, busy: bool) -> bool {
    if busy {
        *had_work = true;
        return false;
    }
    let done = *had_work && *done_any;
    *had_work = false;
    *done_any = false;
    done
}

/// Something keeps "when all downloads finish" waiting: a download running or waiting, or an
/// upload (Send) or Direct transfer running (`transfers`, from `transfers_running`, read before
/// taking the engine's lock).
fn busy(inner: &Inner, transfers: bool) -> bool {
    !inner.jobs.is_empty() || inner.downloads.iter().any(|d| d.status == Status::Queued) || transfers
}

/// A download stopped: is that news worth a notification? Not when Pako put it back in line by
/// itself (Google's daily limit, tried every few hours) and it stopped the same way again:
/// you already know, and it keeps trying. `retried_from`: the status it had before Pako tried
/// again by itself (None = a first stop, or you asked for this run).
fn stop_is_news(retried_from: Option<&Status>, now: &Status) -> bool {
    retried_from != Some(now)
}

/// Between two looks (half a second apart), the PC's clock and the time Pako counts moved
/// apart, or a long time passed: the PC was asleep (or its clock was changed).
fn clock_jumped(before: (Instant, std::time::SystemTime), now: (Instant, std::time::SystemTime)) -> bool {
    const JUMP: f64 = 20.0;
    let counted = now.0.saturating_duration_since(before.0).as_secs_f64();
    let wall = match now.1.duration_since(before.1) {
        Ok(d) => d.as_secs_f64(),
        Err(e) => -e.duration().as_secs_f64(),
    };
    counted > JUMP || (wall - counted).abs() > JUMP
}

/// Copied links taken at once from one clipboard change (more are skipped, and said so).
const CLIPBOARD_MAX: usize = 50;

/// Inside the download hours right now (always, when the schedule is off).
fn in_hours(s: &Settings) -> bool {
    in_hours_at(s, crate::system::local_minutes())
}

fn in_hours_at(s: &Settings, now: u32) -> bool {
    !s.schedule_on || crate::system::in_window(now, s.schedule_start, s.schedule_end)
}

/// "Download first" lets an item run outside the download hours: only needed (and only kept)
/// while the hours are closed. During them it runs anyway and stops with the rest at the end.
fn first_needs_override(s: &Settings, now: u32) -> bool {
    !in_hours_at(s, now)
}

/// A finished (or failed) download for the Dashboard's numbers (written off the engine's lock).
fn numbers_event(d: &Download, speed: Option<&Speed>, ok: bool) {
    let at = crate::numbers::now_ms();
    let mut e = crate::numbers::Event::new(format!("dl:{}:{}", d.id, at), at, crate::numbers::DOWNLOAD);
    if let Some(sp) = speed {
        e = e.timed(d.downloaded.saturating_sub(sp.first), sp.busy, sp.peak);
    }
    e.name = d.name.clone();
    e.bytes = if ok { d.size.unwrap_or(d.downloaded) } else { 0 };
    e.source = match d.source {
        Source::GDrive { .. } => "drive",
        Source::Video { .. } => "video",
        Source::Http { .. } => "web",
    }
    .into();
    e.site = crate::numbers::site_of(&d.link);
    e.ftype = crate::numbers::file_type(&d.name).into();
    e.ok = ok;
    std::thread::spawn(move || crate::numbers::record(e));
}

/// Queued downloads outside the download hours say this (the time it starts again).
fn scheduled_note(start: u32) -> String {
    format!("Waiting for the scheduled hours ({})", crate::system::hhmm(start))
}

struct Speed {
    bytes: u64,
    at: Instant,
    value: f64,
    /// Bytes already there when this run started, seconds spent downloading, and the top
    /// (smoothed) speed: the Dashboard's average and peak for this download.
    first: u64,
    busy: f64,
    peak: f64,
}

struct Slot {
    seg: Segment,
    busy: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Starting,
    Downloading,
    Verifying,
    /// A video: joining picture and sound after the download.
    Joining,
    /// Audio only: converting the sound (to MP3 or M4A) after the download.
    Converting,
}

struct Job {
    id: String,
    /// Cancelled when the user pauses/removes the download or the app closes.
    cancel: CancellationToken,
    slots: Mutex<Vec<Slot>>,
    downloaded: AtomicU64,
    connections: AtomicU32,
    last_data: Mutex<Option<Instant>>,
    retry_msg: Mutex<Option<String>>,
    phase: Mutex<Phase>,
    resumable: Mutex<bool>,
    /// How many connections this file should use right now (Auto changes it while running).
    target: AtomicU32,
    /// Connection workers currently alive.
    live: AtomicU32,
    /// Last time the server said "too many requests / slow down".
    throttled_at: Mutex<Option<Instant>>,
    /// Something went wrong since Auto last looked (drop, retry, outage): don't judge that period.
    disturbed: std::sync::atomic::AtomicBool,
    /// Server this file comes from (for the per-server connection budget).
    host: Mutex<String>,
    /// "Start over" was asked while this run was going: wipe the file when it stops.
    reset: std::sync::atomic::AtomicBool,
    /// The open unfinished file (flushed to disk before each save).
    file: Mutex<Option<Arc<File>>>,
    /// Stopped on purpose to be started again (the download hours ended, or a download that
    /// comes first took its spot): back to waiting, nothing lost.
    requeue: std::sync::atomic::AtomicBool,
    /// When this run started (a download that comes first takes the spot of the newest one).
    started: Instant,
    /// The parts as they were at the last flush to disk: the only progress ever saved while
    /// running, so a power cut can't leave parts marked done that are still zeros on disk.
    flushed: Mutex<Option<Vec<Segment>>>,
}

/// Split sets being extracted right now (by their first part's path).
static EXTRACTING: std::sync::LazyLock<Mutex<HashSet<String>>> = std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

/// Never more than this many connections to one server, all files together.
const HOST_BUDGET: u32 = 24;
/// After a server says "too many requests", remember its limit this long.
const HOST_MEMORY: Duration = Duration::from_secs(30 * 60);
/// New connections start this far apart, never all at once.
const SPAWN_GAP: Duration = Duration::from_millis(250);
/// Mid-download pages from Google's public link: tries before stopping as "needs an account".
const BLOCKED_TRIES: u32 = 4;
/// Longest wait for the internet check to come back before trying the server anyway.
const OFFLINE_WAIT: Duration = Duration::from_secs(60);

/// Auto connections: start with a few, add more while it makes the file faster,
/// stop when it no longer helps, back off when the server complains, re-check every minute.
/// Periods with connection problems (internet or server) are never used to judge.
const AUTO_START: u32 = 4;
/// Video streams (YouTube and the like) start with more: one stream is usually one big file
/// that needs its full speed from the first second, and the 10 MB requests are short.
const STREAM_START: u32 = 8;
/// Google's daily download limit on a shared file: how long before Pako tries it again.
const QUOTA_RETRY: Duration = Duration::from_secs(3 * 3600);
/// The status note of a download waiting for the Download with Pako window.
pub const ASK_NOTE: &str = "Waiting for you";
const AUTO_WINDOW: Duration = Duration::from_secs(5);
const AUTO_HOLD: Duration = Duration::from_secs(60);
/// More connections must make the file at least this much faster to be kept.
const AUTO_MIN_GAIN: f64 = 1.08;
/// Speed falling below this after adding connections = the internet line changed, not the connections.
const AUTO_LINE_DROP: f64 = 0.85;

struct AutoTuner {
    /// Off = fixed number of connections; only "slow down" answers lower it.
    auto: bool,
    max: u32,
    window_at: Instant,
    window_bytes: u64,
    /// Best measured so far in this round: (connections, bytes per second).
    best: Option<(u32, f64)>,
    hold_until: Option<Instant>,
    seen_throttle: Option<Instant>,
    started: bool,
    /// Measurements in a row where more connections gave about the same speed.
    flat: u32,
    /// The last step reacted to a "too many requests" answer.
    just_throttled: bool,
}

impl AutoTuner {
    fn new(max: u32, auto: bool) -> Self {
        Self {
            auto,
            max,
            window_at: Instant::now(),
            window_bytes: 0,
            best: None,
            hold_until: None,
            seen_throttle: None,
            started: false,
            flat: 0,
            just_throttled: false,
        }
    }

    fn reset_window(&mut self, job: &Job) {
        self.window_at = Instant::now();
        self.window_bytes = job.downloaded.load(Ordering::Relaxed);
        job.disturbed.store(false, Ordering::Relaxed);
    }

    /// Called every second; returns how many connections the file should use now.
    fn step(&mut self, job: &Job) -> u32 {
        let now = Instant::now();
        let target = job.target.load(Ordering::Relaxed);
        if !self.started {
            self.started = true;
            self.reset_window(job);
            return target;
        }

        // The server said "too many": drop a quarter right away and stay there a minute.
        self.just_throttled = false;
        let throttled = *job.throttled_at.lock().unwrap();
        if throttled.is_some() && throttled != self.seen_throttle {
            self.seen_throttle = throttled;
            self.just_throttled = true;
            self.hold_until = Some(now + AUTO_HOLD);
            self.best = None;
            self.reset_window(job);
            return (target * 3 / 4).max(1);
        }
        // Fixed number of connections (Auto off): only the "slow down" rule above applies.
        if !self.auto {
            return target;
        }

        if now.duration_since(self.window_at) < AUTO_WINDOW {
            return target;
        }
        let elapsed = now.duration_since(self.window_at).as_secs_f64();
        let bytes = job.downloaded.load(Ordering::Relaxed).saturating_sub(self.window_bytes);
        let disturbed = job.disturbed.load(Ordering::Relaxed);
        self.reset_window(job);
        let speed = bytes as f64 / elapsed;

        // A drop, retry or internet outage happened: this period says nothing about connections.
        if disturbed || speed <= 0.0 {
            return target;
        }
        if let Some(h) = self.hold_until {
            if now < h {
                return target;
            }
            // A minute passed: measure again from here and try adding more.
            self.hold_until = None;
            self.best = None;
        }
        // Nearly done: more connections won't matter.
        let remaining: u64 = job.slots.lock().unwrap().iter().map(|s| s.seg.remaining()).sum();
        if remaining < target as u64 * 4 * MIN_SPLIT {
            return target;
        }

        match self.best {
            Some((best_n, best_speed)) if target > best_n => {
                if speed >= best_speed * AUTO_MIN_GAIN {
                    // Clearly faster: keep them and try a few more.
                    self.flat = 0;
                    self.best = Some((target, speed));
                    self.grow(target, now)
                } else if speed < best_speed * AUTO_LINE_DROP {
                    // Much SLOWER. Extra connections don't do that on their own: the
                    // internet line itself got slower (someone streaming, weak signal...).
                    // Not a verdict on the connections: take a fresh baseline here.
                    self.flat = 0;
                    self.best = Some((target, speed));
                    target
                } else if self.flat == 0 {
                    // About the same: measure once more before deciding.
                    self.flat = 1;
                    target
                } else {
                    // Same speed twice: more connections aren't worth it. Go back to the
                    // last good number and check again in a minute.
                    self.flat = 0;
                    self.hold_until = Some(now + AUTO_HOLD);
                    best_n
                }
            }
            _ => {
                self.flat = 0;
                self.best = Some((target, speed));
                self.grow(target, now)
            }
        }
    }

    fn grow(&mut self, target: u32, now: Instant) -> u32 {
        if target >= self.max {
            self.hold_until = Some(now + AUTO_HOLD);
            return target;
        }
        let step = if target < 8 { 2 } else { 4 };
        (target + step).min(self.max)
    }
}

enum Stop {
    Cancelled,
    Fail(Status, String),
}

impl From<FetchError> for Stop {
    fn from(e: FetchError) -> Self {
        match e {
            FetchError::Quota(m) => Stop::Fail(Status::QuotaExceeded, m),
            FetchError::Blocked(m) => Stop::Fail(Status::NeedsAccount, m),
            other => Stop::Fail(Status::Failed, other.message()),
        }
    }
}

/// What a browser request added: whether Links should open, and the downloads it made (so a
/// takeover checks its own download, never another request's for the same link).
pub struct BrowserAdded {
    pub open_links: bool,
    pub ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddResult {
    pub added: usize,
    pub errors: Vec<String>,
    /// Links that were already in the list (not added twice).
    pub skipped: usize,
}

fn list_path() -> PathBuf {
    data_dir().join("downloads.json")
}

/// What to tell at start-up about reading a saved list ("download list", "Links list"), if
/// anything went wrong.
fn list_note<T>(what: &str, path: &PathBuf, read: &Result<crate::paths::ListRead<T>, String>) -> Option<(String, String)> {
    match read {
        Err(_) => Some((
            format!("Pako couldn't read your {what}"),
            format!("So it won't save over it. Your list is kept in {}. Restart Pako to try again.", path.display()),
        )),
        Ok(r) => {
            let kept = r.kept_in.as_ref()?;
            Some(if r.unread > 0 {
                let items = if r.unread == 1 { "1 item".to_string() } else { format!("{} items", r.unread) };
                (
                    format!("Pako couldn't read part of your {what}"),
                    format!(
                        "{items} couldn't be read (maybe saved by a newer Pako) and {} left out. The list as it was is kept in {}.",
                        if r.unread == 1 { "is" } else { "are" },
                        kept.display()
                    ),
                )
            } else if r.from_backup {
                (
                    format!("Your {what} was damaged"),
                    format!("Pako used its backup copy. The damaged file is kept in {}.", kept.display()),
                )
            } else {
                (
                    format!("Your {what} was damaged"),
                    format!("Pako couldn't read it and starts with an empty list. The damaged file is kept in {}.", kept.display()),
                )
            })
        }
    }
}
fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

impl Engine {
    pub fn new(emit: Emit) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            // A connection that receives nothing for 45s is considered dead and reopened.
            .read_timeout(Duration::from_secs(45))
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .expect("http client");
        // A damaged file falls back to its .bak copy and is moved aside, never overwritten. A
        // file that can't be read right now (locked) is never saved over this session.
        let mut start_notes = Vec::new();
        let (settings, settings_ok) = match crate::paths::read_json_checked::<Settings>(&settings_path()) {
            Ok(s) => (s.unwrap_or_default(), true),
            Err(_) => {
                start_notes.push((
                    "Pako couldn't read your settings".to_string(),
                    format!(
                        "So it won't save over them, and uses the usual settings until it closes. Your settings are kept in {}. Restart Pako to use them again.",
                        settings_path().display()
                    ),
                ));
                // Not a first start: no setup questions.
                (
                    Settings {
                        setup_done: true,
                        tray_asked: true,
                        video_asked: true,
                        tour_offered: true,
                        whats_new_seen: Some(env!("CARGO_PKG_VERSION").to_string()),
                        ..Settings::default()
                    },
                    false,
                )
            }
        };
        let loaded = crate::paths::read_list_checked::<Download>(&list_path());
        start_notes.extend(list_note("download list", &list_path(), &loaded));
        let loaded_ok = loaded.is_ok();
        let mut downloads: Vec<Download> = loaded.map(|r| r.items).unwrap_or_default();
        let (found, found_ok, found_note) = found::load();
        start_notes.extend(found_note);
        // Downloads that were running when the app closed continue automatically.
        for d in &mut downloads {
            // Lists saved before the date existed: the queue time is the best guess.
            if d.date == 0 {
                d.date = d.added_at;
            }
            if d.status.is_active() {
                d.status = Status::Queued;
                d.message = None;
            }
            if d.force {
                d.priority = Priority::First;
                d.force = false;
            }
        }
        let limiter = Arc::new(Limiter::new(settings.speed_limit_kb.saturating_mul(1024)));
        crate::notices::use_settings(&settings);
        Arc::new(Self {
            emit,
            http,
            limiter,
            net: Arc::new(Net::new()),
            host_limits: Mutex::new(HashMap::new()),
            stats: Mutex::new(crate::stats::Stats::load()),
            last_sample: Mutex::new(Instant::now()),
            notes: Mutex::new((Vec::new(), None)),
            inner: Mutex::new(Inner {
                downloads,
                found,
                settings,
                jobs: HashMap::new(),
                speeds: HashMap::new(),
                dirty: false,
                last_save: Instant::now(),
                had_jobs: false,
                loaded_ok,
                found_ok,
                settings_ok,
                start_notes,
                last_look: None,
                when_done: WhenDone::Nothing,
                when_done_at: None,
                had_work: false,
                done_any: false,
                sched_open: None,
                sched_override: HashSet::new(),
                quota_seen: HashMap::new(),
                auto_retry: HashMap::new(),
            }),
        })
    }

    /// Not enough room on the drive for `left` more bytes of `d`, counting what the other running
    /// downloads to that drive still need: the message to show. Files are written as they come
    /// (nothing reserved), so this is the check that stops a download filling the disk halfway.
    fn space_problem(&self, d: &Download, left: u64) -> Option<String> {
        let free = crate::system::free_space(&d.dest_dir)?;
        let drive = crate::system::drive_of(&d.dest_dir);
        let others: u64 = {
            let inner = self.inner.lock().unwrap();
            inner
                .downloads
                .iter()
                .filter(|x| x.id != d.id && inner.jobs.contains_key(&x.id))
                .filter(|x| crate::system::drive_of(&x.dest_dir) == drive)
                .filter_map(|x| x.size.map(|s| s.saturating_sub(x.downloaded)))
                .sum()
        };
        // A little room to spare: a completely full disk makes Windows itself struggle.
        const SPARE: u64 = 256 * 1024 * 1024;
        // (Saturating: a server can announce any size.)
        if free >= left.saturating_add(others).saturating_add(SPARE) {
            return None;
        }
        let place = if drive.is_empty() { "the drive".to_string() } else { drive };
        Some(if others > 0 {
            format!(
                "Not enough free space on {place}: this needs {} and the other downloads there {} more, {} is free. Free some space, then click Try again.",
                human_size(left),
                human_size(others),
                human_size(free)
            )
        } else {
            format!(
                "Not enough free space on {place}: this needs {}, {} is free. Free some space, then click Try again.",
                human_size(left),
                human_size(free)
            )
        })
    }

    /// Every half second: the download hours (pause at the end, continue at the start) and
    /// "when all downloads finish" (countdown, then sleep or shut down).
    fn timekeeping(self: &Arc<Self>) {
        let mut act = None;
        let mut changed = false;
        let mut events: Vec<(&'static str, serde_json::Value)> = Vec::new();
        // Read before the lock (its check takes Direct's own locks).
        let transfers = transfers_running();
        {
            let mut inner = self.inner.lock().unwrap();
            let s = inner.settings.clone();
            let open = in_hours(&s);
            let was = inner.sched_open.replace(open);
            if was == Some(true) && !open {
                // End of the hours: running downloads go back to waiting (nothing lost), the
                // ones started with "Download first" during the hours too.
                inner.sched_override.clear();
                for job in inner.jobs.values() {
                    if !job.cancel.is_cancelled() {
                        job.requeue.store(true, Ordering::Relaxed);
                        job.cancel.cancel();
                    }
                }
            }
            if was == Some(false) && open {
                inner.sched_override.clear();
            }
            // The waiting ones say why (and stop saying it once the hours start).
            let note = scheduled_note(s.schedule_start);
            let over = inner.sched_override.clone();
            for d in inner.downloads.iter_mut() {
                if d.status != Status::Queued {
                    continue;
                }
                let scheduled = d.message.as_deref().is_some_and(|m| m.starts_with("Waiting for the scheduled hours"));
                let waits = !open && !over.contains(&d.id);
                if waits && d.message.as_deref() != Some(note.as_str()) && (d.message.is_none() || scheduled) {
                    d.message = Some(note.clone());
                    changed = true;
                } else if !waits && scheduled {
                    d.message = None;
                    changed = true;
                }
            }

            // Google's daily limit for a shared file: tried again by itself every few hours
            // (the "Try again" button does it at once).
            let blocked: HashSet<String> =
                inner.downloads.iter().filter(|d| d.status == Status::QuotaExceeded).map(|d| d.id.clone()).collect();
            inner.quota_seen.retain(|id, _| blocked.contains(id));
            let now_i = Instant::now();
            for id in &blocked {
                inner.quota_seen.entry(id.clone()).or_insert(now_i);
            }
            let due: Vec<String> =
                inner.quota_seen.iter().filter(|(_, t)| t.elapsed() >= QUOTA_RETRY).map(|(id, _)| id.clone()).collect();
            for id in due {
                inner.quota_seen.remove(&id);
                if let Some(d) = inner.downloads.iter_mut().find(|d| d.id == id) {
                    let before = std::mem::replace(&mut d.status, Status::Queued);
                    d.message = None;
                    changed = true;
                    inner.auto_retry.insert(id, before);
                }
            }

            // When all downloads finish.
            let busy = busy(&inner, transfers);
            if busy && inner.when_done_at.take().is_some() {
                events.push(("when-done", serde_json::json!({ "phase": "cancelled" })));
            }
            // The PC slept (or its clock jumped) during the countdown: the minute to cancel
            // was never seen. Stop it rather than act right after the PC wakes.
            let now = (Instant::now(), std::time::SystemTime::now());
            let woke = inner.last_look.replace(now).is_some_and(|before| clock_jumped(before, now));
            if woke && inner.when_done_at.take().is_some() {
                events.push(("when-done", serde_json::json!({ "phase": "cancelled" })));
                let body = if inner.when_done == WhenDone::Sleep {
                    "The PC was asleep during the countdown, so Pako won't put it to sleep now."
                } else {
                    "The PC was asleep during the countdown, so Pako won't shut it down now."
                };
                events.push(("notify", notices::payload("power", "Countdown stopped", body, &Go::none(), Pop::Pref)));
            }
            let Inner { had_work, done_any, .. } = &mut *inner;
            if all_done_now(had_work, done_any, busy) {
                if inner.when_done != WhenDone::Nothing && inner.when_done_at.is_none() {
                    inner.when_done_at = Some(Instant::now() + Duration::from_secs(WHEN_DONE_SECS));
                    let what = inner.when_done.name();
                    events.push(("show-main", serde_json::Value::Null));
                    events.push(("when-done", serde_json::json!({ "phase": "countdown", "action": what, "seconds": WHEN_DONE_SECS })));
                    let (title, body) = if inner.when_done == WhenDone::Sleep {
                        ("All downloads are done", "The PC goes to sleep in 1 minute. Open Pako to cancel.")
                    } else {
                        ("All downloads are done", "The PC shuts down in 1 minute. Open Pako to cancel.")
                    };
                    events.push(("notify", notices::payload("power", title, body, &Go::page("downloads"), Pop::Pref)));
                }
            }
            if inner.when_done_at.is_some_and(|at| Instant::now() >= at) {
                act = Some(inner.when_done);
                inner.when_done = WhenDone::Nothing;
                inner.when_done_at = None;
                events.push(("when-done", serde_json::json!({ "phase": "fired" })));
            }
            if changed {
                inner.dirty = true;
            }
        }
        if changed {
            self.list_changed();
        }
        for (e, p) in events {
            (self.emit)(e, p);
        }
        match act {
            Some(WhenDone::Sleep) => {
                self.save_now();
                if !crate::system::sleep_now() {
                    self.notify("power", "The PC didn't go to sleep", "Windows refused to put it to sleep. Your downloads are done.", Go::page("downloads"));
                }
            }
            Some(WhenDone::ShutDown) => {
                self.save_now();
                if !crate::system::shut_down_now() {
                    self.notify("power", "The PC didn't shut down", "Windows refused to shut it down. Your downloads are done.", Go::page("downloads"));
                }
            }
            _ => {}
        }
    }

    /// "When all downloads finish" from the window (resets to nothing after it fires or is cancelled).
    pub fn set_when_done(&self, action: &str) {
        let transfers = transfers_running();
        let mut inner = self.inner.lock().unwrap();
        inner.when_done = WhenDone::parse(action);
        // Chosen while downloads run (or wait): it fires when they're done.
        inner.had_work = busy(&inner, transfers);
        if inner.when_done == WhenDone::Nothing {
            inner.when_done_at = None;
        }
    }

    /// "Cancel" in the countdown.
    pub fn cancel_when_done(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.when_done = WhenDone::Nothing;
        inner.when_done_at = None;
    }

    /// Links copied anywhere go to Links by themselves (when the setting is on). Copies made
    /// in Pako itself, and links already in Links or the downloads, are skipped.
    fn watch_clipboard(self: &Arc<Self>) {
        let me = self.clone();
        std::thread::spawn(move || {
            let mut last = crate::system::clipboard_seq();
            loop {
                std::thread::sleep(Duration::from_millis(1000));
                let seq = crate::system::clipboard_seq();
                if seq == last {
                    continue;
                }
                last = seq;
                if !me.settings().catch_clipboard || crate::system::pako_in_front() || seq == crate::system::own_copy_seq() {
                    continue;
                }
                let Some(text) = crate::system::clipboard_text() else { continue };
                // The same text Pako copied (a link it just sent, copied back by another app).
                if crate::system::is_own_copy(&text) {
                    continue;
                }
                // Read before taking the list (a big copy takes a moment): the list only filters.
                let copied = extract_links(&text);
                if copied.is_empty() {
                    continue;
                }
                let mut links: Vec<String> = {
                    let inner = me.inner.lock().unwrap();
                    let known: HashSet<&str> =
                        inner.found.iter().map(|f| f.template.link.as_str()).chain(inner.downloads.iter().map(|d| d.link.as_str())).collect();
                    copied.into_iter().filter(|l| !known.contains(l.as_str())).collect()
                };
                if links.is_empty() {
                    continue;
                }
                // A copied page full of links doesn't fill Links with thousands of rows.
                let skipped = links.len().saturating_sub(CLIPBOARD_MAX);
                links.truncate(CLIPBOARD_MAX);
                let me2 = me.clone();
                tauri::async_runtime::spawn(async move {
                    let r = me2.found_add(&links.join("\n")).await;
                    if r.added > 0 {
                        (me2.emit)("clipboard-caught", serde_json::json!({ "added": r.added, "skipped": skipped }));
                    }
                });
            }
        });
    }

    // ---------- videos (yt-dlp) ----------

    /// Downloads a video. yt-dlp first says which streams make up the chosen quality:
    /// - plain files (YouTube, Vimeo, Dailymotion...): Pako downloads them itself, all at the
    ///   same time, each in parallel parts like any download, then ffmpeg joins them;
    /// - small pieces (HLS / DASH): yt-dlp, many pieces at a time;
    /// - anything else (live streams...): yt-dlp as it is.
    /// When Pako's own download can't work for a video (the server won't send parts, or keeps
    /// refusing the stream addresses), yt-dlp downloads it instead.
    async fn run_video(self: &Arc<Self>, job: &Arc<Job>, d: Download) -> Result<(), Stop> {
        let Source::Video { url, quality, audio, audio_size, options, streams: saved, .. } = &d.source else { return Err(Stop::Cancelled) };
        let Some(tools) = crate::video::tools() else {
            return Err(Stop::Fail(Status::Failed, crate::video::NEEDS_TOOLS.into()));
        };
        let quality = quality.unwrap_or_else(|| self.settings().video_quality);
        let audio = audio.clone().or_else(|| (quality == 0).then(|| "m4a".to_string()));
        let estimate = crate::video::estimate(options, *audio_size, if audio.is_some() { 0 } else { quality });
        let _ = std::fs::create_dir_all(&d.dest_dir);
        *job.phase.lock().unwrap() = Phase::Starting;
        // Every stream already in (stopped while joining): join them now, nothing to download.
        let all_in = !saved.is_empty()
            && saved.iter().all(|s| {
                // Pieces an older Pako named (".zdpart", or without the download's mark) continue.
                let part = crate::video::adopt_old_pieces(&d.dest_dir, &d.name, &d.id, &s.format_id, &s.ext, true);
                s.segments.iter().all(|g| g.remaining() == 0) && std::fs::metadata(&part).is_ok_and(|m| m.len() == s.size)
            });
        if all_in {
            let pieces = saved.iter().map(|s| piece_of(&d, s)).collect::<Vec<_>>();
            return self.finish_video(job, &d, &tools, &pieces, audio.as_deref()).await;
        }
        let plan = self.video_plan(job, url, quality, audio.as_deref()).await?;
        let fragments = match plan {
            crate::video::Plan::Direct(streams) => {
                match self.run_video_direct(job, &d, &tools, url, quality, audio.as_deref(), streams).await? {
                    true => return Ok(()),
                    false => crate::video::FRAGMENTS_AT_ONCE,
                }
            }
            crate::video::Plan::Fragments => crate::video::FRAGMENTS_AT_ONCE,
            crate::video::Plan::Plain => 0,
        };
        // yt-dlp downloads it: Pako's own pieces from an earlier try are of no use to it.
        self.drop_video_streams(&job.id);
        self.run_ytdlp(job, &d, &tools, url, quality, audio.as_deref(), estimate, fragments).await
    }

    /// Asks yt-dlp which streams make up the quality (waiting out a lost connection).
    async fn video_plan(&self, job: &Arc<Job>, link: &str, quality: u32, audio: Option<&str>) -> Result<crate::video::Plan, Stop> {
        let mut attempt = 0u32;
        loop {
            // The browser's login for this video (deleted again right after the answer).
            let jar = crate::video::cookie_file_for(&job.id);
            let r = tokio::select! {
                _ = job.cancel.cancelled() => return Err(Stop::Cancelled),
                r = crate::video::select(link, quality, audio, Duration::from_secs(90), jar.as_ref().map(|j| j.path())) => r,
            };
            drop(jar);
            match r {
                Ok(plan) => {
                    *job.retry_msg.lock().unwrap() = None;
                    return Ok(plan);
                }
                Err((msg, newer)) => {
                    if newer {
                        self.update_video_tool();
                    }
                    // An answer Pako can't read: yt-dlp may still download it the usual way.
                    if msg.starts_with("The video tool gave an answer") {
                        return Ok(crate::video::Plan::Fragments);
                    }
                    if crate::video::is_network(&msg) && attempt < 30 {
                        match job.wait_before_retry(&self.net, attempt, &msg, 0).await {
                            None => return Err(Stop::Cancelled),
                            Some(_) => {
                                attempt += 1;
                                continue;
                            }
                        }
                    }
                    return Err(Stop::Fail(Status::Failed, msg));
                }
            }
        }
    }

    /// The stream's size, if its server sends it in parts. Inner Err = it can't be downloaded
    /// in parts (why, in the fetcher's words).
    async fn stream_size(&self, job: &Arc<Job>, fetcher: &Fetcher) -> Result<Result<u64, String>, Stop> {
        let mut attempt = 0u32;
        loop {
            let r = tokio::select! {
                _ = job.cancel.cancelled() => return Err(Stop::Cancelled),
                r = fetcher.get(0, Some(0)) => r,
            };
            match r {
                Ok(resp) => {
                    let info = probe_info(&resp);
                    drop(resp);
                    return Ok(match info.size {
                        Some(size) if info.resumable && size > 0 => Ok(size),
                        _ => Err("The server doesn't send this video in parts".into()),
                    });
                }
                Err(FetchError::Retry(msg) | FetchError::Throttled(msg, _)) if attempt < 8 => {
                    match job.wait_before_retry(&self.net, attempt, &msg, 0).await {
                        None => return Err(Stop::Cancelled),
                        Some(_) => attempt += 1,
                    }
                }
                Err(e) => return Ok(Err(e.message())),
            }
        }
    }

    /// Pako downloads the streams itself. Ok(true) = done (joined, renamed); Ok(false) = this
    /// video can't be downloaded this way: yt-dlp takes over.
    #[allow(clippy::too_many_arguments)]
    async fn run_video_direct(
        self: &Arc<Self>,
        job: &Arc<Job>,
        d: &Download,
        tools: &crate::video::Tools,
        link: &str,
        quality: u32,
        audio: Option<&str>,
        mut streams: Vec<crate::video::Stream>,
    ) -> Result<bool, Stop> {
        // New addresses are asked for when the old ones are refused (they expire), as long as
        // the download moves between two refusals.
        let mut renewals = 0u32;
        let mut renewed_at: Option<u64> = None;
        let mut renew = |job: &Arc<Job>| -> bool {
            let now = job.downloaded.load(Ordering::Relaxed);
            if renewals >= 10 || renewed_at.is_some_and(|at| at >= now) {
                return false;
            }
            renewals += 1;
            renewed_at = Some(now);
            true
        };
        loop {
            // 1. Each stream's size, and that its server sends it in parts.
            let mut fetchers: Vec<Arc<Fetcher>> = Vec::new();
            let mut sizes: Vec<u64> = Vec::new();
            let mut refused: Option<String> = None;
            for s in &streams {
                let f = Fetcher::for_stream(self.http.clone(), &s.url, &s.headers);
                match self.stream_size(job, &f).await? {
                    Ok(size) => {
                        sizes.push(size);
                        fetchers.push(Arc::new(f));
                    }
                    Err(why) => {
                        refused = Some(why);
                        break;
                    }
                }
            }
            if let Some(why) = refused {
                if crate::video::worth_new_links(&why) && renew(job) {
                    match self.video_plan(job, link, quality, audio).await? {
                        crate::video::Plan::Direct(s) => {
                            streams = s;
                            continue;
                        }
                        _ => return Ok(false),
                    }
                }
                return Ok(false);
            }

            // 2. Lay out each stream's parts, or continue where the last run stopped.
            let state = self.lay_out_streams(job, d, &streams, &sizes)?;
            let total: u64 = state.iter().map(|s| s.size).sum();
            let have: u64 = state.iter().flat_map(|s| s.segments.iter()).map(|g| g.done).sum();
            if let Some(msg) = self.space_problem(d, total.saturating_sub(have)) {
                return Err(Stop::Fail(Status::Failed, msg));
            }
            self.record_streams(&job.id, &state, Some(total));

            // 3. All streams at the same time, each with several connections.
            match self.download_streams(job, d, &streams, state, fetchers).await {
                Ok(state) => {
                    let pieces: Vec<crate::video::Piece> = state.iter().map(|s| piece_of(d, s)).collect();
                    self.finish_video(job, d, tools, &pieces, audio).await?;
                    return Ok(true);
                }
                Err(Stop::Fail(_, msg)) if crate::video::worth_new_links(&msg) && renew(job) => {
                    // Addresses expired mid-way: new ones, and the parts already in are kept.
                    match self.video_plan(job, link, quality, audio).await? {
                        crate::video::Plan::Direct(s) => streams = s,
                        _ => return Ok(false),
                    }
                }
                Err(Stop::Fail(_, msg)) if crate::video::worth_new_links(&msg) => return Ok(false),
                Err(e) => return Err(e),
            }
        }
    }

    /// What a yt-dlp run may use: pieces at once (no more than the connections setting allows,
    /// nor the per-server budget) and its share of the speed limit (split among the downloads
    /// running when it starts).
    fn ytdlp_budget(&self, fragments: u32) -> (u32, Option<u64>) {
        let inner = self.inner.lock().unwrap();
        let s = &inner.settings;
        let mut n = fragments.min(HOST_BUDGET);
        if !s.auto_connections {
            n = n.min(s.connections.max(1));
        }
        let rate = (s.speed_limit_kb > 0).then(|| s.speed_limit_kb.saturating_mul(1024) / (inner.jobs.len().max(1) as u64));
        (if fragments == 0 { 0 } else { n.max(1) }, rate)
    }

    /// Each stream's parts: the saved ones when they belong to the same streams (same format,
    /// same size, the piece still on disk), else a fresh layout (older pieces deleted).
    fn lay_out_streams(&self, job: &Arc<Job>, d: &Download, streams: &[crate::video::Stream], sizes: &[u64]) -> Result<Vec<StreamPart>, Stop> {
        let saved: Vec<StreamPart> = {
            let inner = self.inner.lock().unwrap();
            match inner.downloads.iter().find(|x| x.id == job.id).map(|x| &x.source) {
                Some(Source::Video { streams, .. }) => streams.clone(),
                _ => Vec::new(),
            }
        };
        let (connections, auto) = {
            let s = &self.inner.lock().unwrap().settings;
            (s.connections, s.auto_connections)
        };
        let disk = |e: std::io::Error| Stop::Fail(Status::Failed, disk_message(&e));
        let mut out = Vec::new();
        for (k, (s, &size)) in streams.iter().zip(sizes).enumerate() {
            let before = saved.get(k).filter(|p| p.format_id == s.format_id && p.ext == s.ext && p.size == size);
            // A piece an older Pako named (".zdpart", or without the download's mark): continued
            // under the new name.
            let part = if before.is_some() {
                crate::video::adopt_old_pieces(&d.dest_dir, &d.name, &d.id, &s.format_id, &s.ext, true)
            } else {
                crate::video::stream_part(&d.dest_dir, &d.name, &d.id, &s.format_id, &s.ext)
            };
            let on_disk = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            if let Some(p) = before.filter(|p| !p.segments.is_empty() && on_disk == size) {
                out.push(StreamPart { has_video: s.has_video, has_audio: s.has_audio, ..p.clone() });
                continue;
            }
            // Fresh: a sparse file of the full size (parts start far into it).
            let file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&part).map_err(disk)?;
            make_sparse(&file);
            file.set_len(size).map_err(disk)?;
            out.push(StreamPart {
                format_id: s.format_id.clone(),
                ext: s.ext.clone(),
                size,
                has_video: s.has_video,
                has_audio: s.has_audio,
                segments: initial_segments(Some(size), true, if auto { STREAM_START } else { connections }),
            });
        }
        // Pieces of streams this video no longer uses.
        for p in &saved {
            if !out.iter().any(|o| o.format_id == p.format_id && o.ext == p.ext) {
                let _ = std::fs::remove_file(crate::video::stream_part(&d.dest_dir, &d.name, &d.id, &p.format_id, &p.ext));
                let _ = std::fs::remove_file(crate::video::old_stream_part(&d.dest_dir, &d.name, &d.id, &p.format_id, &p.ext));
            }
        }
        Ok(out)
    }

    /// Notes the streams' progress (and the video's size) in the list.
    fn record_streams(&self, id: &str, state: &[StreamPart], size: Option<u64>) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == id) {
            if let Source::Video { streams, .. } = &mut dd.source {
                *streams = state.to_vec();
            }
            if let Some(s) = size {
                dd.size = Some(s);
            }
            dd.resolved = true;
        }
        inner.dirty = true;
    }

    /// yt-dlp downloads the video after all: Pako's own stream pieces go.
    fn drop_video_streams(&self, id: &str) {
        let mut inner = self.inner.lock().unwrap();
        let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == id) else { return };
        let (dir, name, did) = (dd.dest_dir.clone(), dd.name.clone(), dd.id.clone());
        if let Source::Video { streams, .. } = &mut dd.source {
            for s in streams.drain(..) {
                let _ = std::fs::remove_file(crate::video::stream_part(&dir, &name, &did, &s.format_id, &s.ext));
                let _ = std::fs::remove_file(crate::video::old_stream_part(&dir, &name, &did, &s.format_id, &s.ext));
                let _ = std::fs::remove_file(crate::video::legacy_stream_part(&dir, &name, &s.format_id, &s.ext));
            }
            inner.dirty = true;
        }
    }

    /// Downloads every stream at the same time, each like any download (parallel parts, Auto
    /// connections, pause and resume). Their progress shows as one. Returns the streams' parts
    /// once all are complete.
    async fn download_streams(
        self: &Arc<Self>,
        job: &Arc<Job>,
        d: &Download,
        streams: &[crate::video::Stream],
        mut state: Vec<StreamPart>,
        fetchers: Vec<Arc<Fetcher>>,
    ) -> Result<Vec<StreamPart>, Stop> {
        let disk = |e: std::io::Error| Stop::Fail(Status::Failed, disk_message(&e));
        let mut subs: Vec<Arc<Job>> = Vec::new();
        let mut files: Vec<Arc<File>> = Vec::new();
        for (k, st) in state.iter().enumerate() {
            let part = crate::video::stream_part(&d.dest_dir, &d.name, &d.id, &st.format_id, &st.ext);
            let file = OpenOptions::new().read(true).write(true).open(&part).map_err(disk)?;
            let done: u64 = st.segments.iter().map(|g| g.done).sum();
            let sub = Arc::new(Job::new(format!("{}#{k}", job.id), job.cancel.child_token(), done, true));
            *sub.slots.lock().unwrap() = st.segments.iter().cloned().map(|seg| Slot { seg, busy: false }).collect();
            *sub.flushed.lock().unwrap() = Some(st.segments.clone());
            *sub.phase.lock().unwrap() = Phase::Downloading;
            subs.push(sub);
            files.push(Arc::new(file));
        }
        mirror_streams(job, &subs);
        let mut runs = futures_util::stream::FuturesUnordered::new();
        for (k, sub) in subs.iter().enumerate() {
            let fetcher = fetchers[k].clone();
            let file = files[k].clone();
            let size = state[k].size;
            let chunk = streams.get(k).and_then(|s| s.chunk);
            runs.push(async move { self.run_workers(sub, fetcher, file, Some(size), true, chunk).await });
        }
        let mut tick = tokio::time::interval(Duration::from_millis(500));
        let mut last_flush = Instant::now();
        let mut failure: Option<Stop> = None;
        loop {
            tokio::select! {
                r = runs.next() => match r {
                    None => break,
                    Some(Err(f @ Stop::Fail(..))) => {
                        // One stream can't go on: stop the others too (their parts are kept).
                        if failure.is_none() {
                            failure = Some(f);
                        }
                        for s in &subs {
                            s.cancel.cancel();
                        }
                    }
                    Some(_) => {}
                },
                _ = tick.tick() => {
                    mirror_streams(job, &subs);
                    if last_flush.elapsed() >= Duration::from_secs(2) {
                        last_flush = Instant::now();
                        // Note the parts first, then flush: what is noted is on disk once the flush returns.
                        let notes: Vec<Vec<Segment>> =
                            subs.iter().map(|s| s.slots.lock().unwrap().iter().map(|x| x.seg.clone()).collect()).collect();
                        let synced: Vec<bool> = off_async(|| files.iter().map(|f| f.sync_data().is_ok()).collect());
                        for (k, segs) in notes.into_iter().enumerate() {
                            if synced[k] && !segs.is_empty() {
                                state[k].segments = segs;
                            }
                        }
                        self.record_streams(&job.id, &state, None);
                    }
                }
            }
        }
        drop(runs);
        mirror_streams(job, &subs);
        // Each run put its file on disk before it ended: keep exactly that progress.
        for (k, sub) in subs.iter().enumerate() {
            if let Some(segs) = sub.flushed.lock().unwrap().clone().filter(|s| !s.is_empty()) {
                state[k].segments = segs;
            }
        }
        self.record_streams(&job.id, &state, None);
        drop(files);
        if job.cancel.is_cancelled() {
            return Err(Stop::Cancelled);
        }
        if let Some(f) = failure {
            return Err(f);
        }
        if state.iter().any(|s| s.segments.iter().any(|g| g.remaining() > 0)) {
            return Err(Stop::Fail(Status::Failed, "Download stopped before the end. Click Retry.".into()));
        }
        Ok(state)
    }

    /// The streams are in: ffmpeg puts them in one file (or the single stream already is it),
    /// which then gets its real name. The pieces are deleted after.
    async fn finish_video(
        self: &Arc<Self>,
        job: &Arc<Job>,
        d: &Download,
        tools: &crate::video::Tools,
        pieces: &[crate::video::Piece],
        audio: Option<&str>,
    ) -> Result<(), Stop> {
        *job.phase.lock().unwrap() = if audio.is_some() { Phase::Converting } else { Phase::Joining };
        let stem = std::path::Path::new(&d.name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Video".into());
        let ext = crate::video::finish_plan(pieces, audio, std::path::Path::new("x")).ext;
        // "Title.temp.mp4": what yt-dlp also calls a file being joined (cleaned up on remove).
        let temp = d.dest_dir.join(format!("{stem}.temp.{ext}"));
        let plan = crate::video::finish_plan(pieces, audio, &temp);
        let made = match plan.ffmpeg {
            None => pieces[0].path.clone(),
            Some(args) => {
                let Some(ff) = tools.ffmpeg.clone() else {
                    return Err(Stop::Fail(Status::Failed, crate::video::NEEDS_TOOLS.into()));
                };
                let mut cmd = tokio::process::Command::new(ff);
                cmd.args(&args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped());
                cmd.kill_on_drop(true);
                let mut child = crate::video::spawn(&mut cmd).map_err(|e| Stop::Fail(Status::Failed, format!("Couldn't start ffmpeg: {e}")))?;
                let pid = child.id();
                let stderr = child.stderr.take();
                let err_task = tauri::async_runtime::spawn(async move {
                    let mut lines: Vec<String> = Vec::new();
                    if let Some(e) = stderr {
                        crate::video::lines(e, |l| {
                            if !l.trim().is_empty() {
                                lines.push(l);
                            }
                        })
                        .await;
                    }
                    lines
                });
                let status = tokio::select! {
                    s = child.wait() => s.ok(),
                    _ = job.cancel.cancelled() => {
                        if let Some(pid) = pid {
                            let _ = tauri::async_runtime::spawn_blocking(move || crate::video::kill_tree(pid)).await;
                        }
                        let _ = child.wait().await;
                        None
                    }
                };
                let errors = err_task.await.unwrap_or_default();
                if job.cancel.is_cancelled() {
                    let _ = std::fs::remove_file(&temp);
                    return Err(Stop::Cancelled);
                }
                if !status.is_some_and(|s| s.success()) {
                    let _ = std::fs::remove_file(&temp);
                    let why = errors.last().map(|l| l.chars().take(160).collect::<String>()).unwrap_or_default();
                    let what = if audio.is_some() { "convert the sound" } else { "join picture and sound" };
                    return Err(Stop::Fail(
                        Status::Failed,
                        if why.is_empty() { format!("Couldn't {what}. Click Try again.") } else { format!("Couldn't {what} ({why}). Click Try again.") },
                    ));
                }
                temp.clone()
            }
        };
        // Its real name (only recorded once the rename worked).
        let final_name = {
            let inner = self.inner.lock().unwrap();
            let taken = taken_paths(&inner.downloads, Some(&job.id));
            names::unique_final(&d.dest_dir, &format!("{stem}.{ext}"), &taken)
        };
        let target = d.dest_dir.join(&final_name);
        let mut tries = 0;
        loop {
            match std::fs::rename(&made, &target) {
                Ok(()) => break,
                // Antivirus or the indexer often holds a just-closed file for a moment.
                Err(_) if tries < 8 => {
                    tries += 1;
                    tokio::select! {
                        _ = job.cancel.cancelled() => return Err(Stop::Cancelled),
                        _ = tokio::time::sleep(Duration::from_millis(500)) => {}
                    }
                }
                Err(e) => {
                    return Err(Stop::Fail(Status::Failed, format!("Downloaded, but couldn't rename the file: {e}. Click Try again.")));
                }
            }
        }
        for p in pieces {
            if p.path != made {
                let _ = std::fs::remove_file(&p.path);
            }
        }
        let len = std::fs::metadata(&target).map(|m| m.len()).ok();
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == job.id) {
                dd.name = final_name;
                if let Some(len) = len {
                    dd.size = Some(len);
                    dd.downloaded = len;
                    job.downloaded.store(len, Ordering::Relaxed);
                }
                if let Source::Video { streams, .. } = &mut dd.source {
                    streams.clear();
                }
                dd.resolved = true;
            }
            inner.dirty = true;
        }
        mark_from_internet(&target, &d.link);
        Ok(())
    }

    /// yt-dlp downloads the video itself. Pausing stops yt-dlp; resuming starts it again and it
    /// continues its unfinished pieces. A lost connection is waited out like any download.
    /// `fragments` = pieces at once for videos that come in small pieces (0 = yt-dlp's default).
    #[allow(clippy::too_many_arguments)]
    async fn run_ytdlp(
        self: &Arc<Self>,
        job: &Arc<Job>,
        d: &Download,
        tools: &crate::video::Tools,
        url: &str,
        quality: u32,
        audio: Option<&str>,
        estimate: Option<u64>,
        fragments: u32,
    ) -> Result<(), Stop> {
        let mut attempt = 0u32;
        loop {
            if job.cancel.is_cancelled() {
                return Err(Stop::Cancelled);
            }
            // Each run starts with a clean status (no "Joining" or retry note from the last one).
            *job.phase.lock().unwrap() = Phase::Starting;
            *job.retry_msg.lock().unwrap() = None;
            *job.last_data.lock().unwrap() = None;
            // The browser's login for this video: kept for this run only.
            let jar = crate::video::cookie_file_for(&d.id);
            let (fragments, rate) = self.ytdlp_budget(fragments);
            let mut cmd = crate::video::download_command(tools, url, quality, audio, &d.dest_dir, &d.name, fragments, jar.as_ref().map(|j| j.path()), rate);
            let mut child = crate::video::spawn(&mut cmd).map_err(|e| Stop::Fail(Status::Failed, format!("Couldn't start the video tool: {e}")))?;
            let pid = child.id();
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();
            let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let err_lines = errors.clone();
            let err_task = tauri::async_runtime::spawn(async move {
                if let Some(e) = stderr {
                    crate::video::lines(e, |l| {
                        let mut v = err_lines.lock().unwrap();
                        v.push(l);
                        if v.len() > 40 {
                            v.remove(0);
                        }
                    })
                    .await;
                }
            });
            // Follow the progress: video and sound come one after the other.
            let me = self.clone();
            let job2 = job.clone();
            let final_file: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
            let final2 = final_file.clone();
            let after = if audio.is_some() { Phase::Converting } else { Phase::Joining };
            let out_task = tauri::async_runtime::spawn(async move {
                let Some(o) = stdout else { return };
                let mut parts = 1usize;
                let mut seen: Vec<String> = Vec::new();
                let mut done_before = 0u64; // bytes of the streams already finished
                let mut current = (String::new(), 0u64, None::<u64>);
                crate::video::lines(o, |l| match crate::video::parse_line(&l) {
                    crate::video::Line::Formats(f) => parts = f.split('+').count().max(1),
                    crate::video::Line::Progress { downloaded, total, format, .. } => {
                        if format != current.0 {
                            if !current.0.is_empty() && !seen.contains(&current.0) {
                                done_before += current.2.unwrap_or(current.1);
                                seen.push(current.0.clone());
                            }
                            current = (format.clone(), 0, None);
                        }
                        current.1 = downloaded;
                        current.2 = total;
                        *job2.phase.lock().unwrap() = Phase::Downloading;
                        *job2.last_data.lock().unwrap() = Some(Instant::now());
                        job2.downloaded.store(done_before + downloaded, Ordering::Relaxed);
                        // The real size, once yt-dlp knows the streams (else the estimate).
                        let streams_left = parts.saturating_sub(seen.len() + 1) as u64;
                        let known = done_before + total.unwrap_or(downloaded);
                        let size = if streams_left == 0 { Some(known) } else { estimate.map(|e| e.max(known)) };
                        if let Some(size) = size {
                            let mut inner = me.inner.lock().unwrap();
                            if let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == job2.id) {
                                dd.size = Some(size.max(done_before + downloaded));
                            }
                        }
                        // The last stream is in: what's left is joining / converting.
                        if streams_left == 0 && total.is_some_and(|t| downloaded >= t) {
                            *job2.phase.lock().unwrap() = after;
                        }
                    }
                    crate::video::Line::File(p) => *final2.lock().unwrap() = Some(p),
                    crate::video::Line::Other => {}
                })
                .await;
            });
            let status = tokio::select! {
                s = child.wait() => s.ok(),
                _ = job.cancel.cancelled() => {
                    if let Some(pid) = pid {
                        let _ = tauri::async_runtime::spawn_blocking(move || crate::video::kill_tree(pid)).await;
                    }
                    let _ = child.wait().await;
                    None
                }
            };
            let _ = out_task.await;
            let _ = err_task.await;
            if job.cancel.is_cancelled() {
                return Err(Stop::Cancelled);
            }
            if status.is_some_and(|s| s.success()) {
                let file = final_file.lock().unwrap().clone();
                let mut inner = self.inner.lock().unwrap();
                if let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == job.id) {
                    if let Some(f) = &file {
                        if let Some(n) = f.file_name() {
                            dd.name = n.to_string_lossy().to_string();
                        }
                        if let Ok(m) = std::fs::metadata(f) {
                            dd.size = Some(m.len());
                            dd.downloaded = m.len();
                            job.downloaded.store(m.len(), Ordering::Relaxed);
                        }
                    }
                    dd.resolved = true;
                }
                inner.dirty = true;
                return Ok(());
            }
            let text = errors.lock().unwrap().join("\n");
            let (msg, site_changed) = crate::video::friendly_error(&text);
            if site_changed {
                self.update_video_tool();
            }
            if crate::video::is_network(&msg) && attempt < 30 {
                match job.wait_before_retry(&self.net, attempt, &msg, 0).await {
                    None => return Err(Stop::Cancelled),
                    Some(_) => {
                        attempt += 1;
                        continue;
                    }
                }
            }
            return Err(Stop::Fail(Status::Failed, msg));
        }
    }

    /// A site changed and yt-dlp couldn't follow: get the newest yt-dlp in the background.
    fn update_video_tool(&self) {
        use crate::components::{Kind, install, installed_version, is_installing, latest_version};
        let http = self.http.clone();
        tauri::async_runtime::spawn(async move {
            if is_installing(Kind::YtDlp) {
                return;
            }
            let Some(have) = installed_version(Kind::YtDlp) else { return };
            let Ok(latest) = latest_version(&http, Kind::YtDlp).await else { return };
            if latest != have {
                let quiet = |_: &str, _: u64, _: Option<u64>| {};
                let _ = install(&http, Kind::YtDlp, &quiet).await;
            }
        });
    }

    /// Downloads running right now (for "quit anyway?").
    pub fn running_count(&self) -> usize {
        self.inner.lock().unwrap().jobs.len()
    }

    /// What finished rows drag out to Explorer or another app: the files, or for a folder its
    /// top folder. A folder goes only when every download in it is finished (all of them, not
    /// just the rows on screen) and it really is a folder named after it (older lists may have
    /// none: that would be the whole download folder). Only what is still on disk.
    pub fn drag_paths(&self, ids: &[String], folder: bool) -> Vec<String> {
        let mut picked: Vec<PathBuf> = Vec::new();
        {
            let inner = self.inner.lock().unwrap();
            let mut looked: HashSet<PathBuf> = HashSet::new();
            for id in ids {
                let Some(d) = inner.downloads.iter().find(|d| &d.id == id) else { continue };
                if !folder {
                    if d.status == Status::Completed {
                        picked.push(d.final_path());
                    }
                    continue;
                }
                let Some(g) = &d.group else { continue };
                let root = group_root(d);
                if !looked.insert(root.clone()) {
                    continue;
                }
                let named = root.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(g));
                let finished = inner
                    .downloads
                    .iter()
                    .filter(|x| x.group.as_deref() == Some(g.as_str()) && group_root(x) == root)
                    .all(|x| x.status == Status::Completed);
                if named && finished {
                    picked.push(root);
                }
            }
        }
        let mut out: Vec<String> = Vec::new();
        for p in picked {
            let s = p.display().to_string();
            if p.exists() && !out.contains(&s) {
                out.push(s);
            }
        }
        out
    }

    pub fn start(self: &Arc<Self>) {
        let notes = std::mem::take(&mut self.inner.lock().unwrap().start_notes);
        if !notes.is_empty() {
            // A list read in part (or from its backup) is saved as read soon, so the copy kept
            // aside isn't made again at every start.
            self.inner.lock().unwrap().dirty = true;
            self.save_found();
        }
        for (title, body) in notes {
            self.notify("other", &title, &body, Go::none());
        }
        // Downloads no longer sign in to Google: the old download sign-in's saved login is used
        // once more to remove the temporary copies it left in the user's Drive, then goes.
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let outcome = crate::google::clean_up_old_download_login(me.http.clone()).await;
            if let Some((title, body)) = crate::google::old_cleanup_note(&outcome) {
                me.notify("other", &title, &body, Go::none());
            }
        });
        self.recheck_found();
        self.watch_clipboard();
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                me.timekeeping();
                me.schedule();
                me.tick();
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    // ---------- reading state ----------

    pub fn downloads(&self) -> Vec<Download> {
        let mut inner = self.inner.lock().unwrap();
        Self::sync_jobs(&mut inner);
        inner.downloads.clone()
    }

    pub fn settings(&self) -> Settings {
        self.inner.lock().unwrap().settings.clone()
    }

    pub fn set_settings(self: &Arc<Self>, mut s: Settings) -> Result<()> {
        s.connections = s.connections.clamp(1, MAX_CONNECTIONS);
        s.max_active = s.max_active.clamp(1, 20);
        std::fs::create_dir_all(&s.download_dir)
            .map_err(|_| anyhow!("Can't use that folder for downloads."))?;
        self.limiter.set_rate(s.speed_limit_kb.saturating_mul(1024));
        let saved = {
            // Saved under the lock: two saves at once reach the disk in the order they were made.
            let mut inner = self.inner.lock().unwrap();
            // Kept by the small windows themselves (the pin, where it was moved, "Notify me"):
            // Settings never shows them, so an older copy of the settings can't undo them.
            s.mini_pinned = inner.settings.mini_pinned;
            s.mini_pos = inner.settings.mini_pos;
            s.ask_notify = inner.settings.ask_notify;
            // Kept by Help (What's new seen, tour offered): Settings never shows them either.
            s.whats_new_seen = inner.settings.whats_new_seen.clone();
            s.tour_offered = inner.settings.tour_offered;
            s.tours_done = inner.settings.tours_done.clone();
            // Kept by the theme switch (`theme::theme_set`), so an older copy can't undo it.
            s.theme = inner.settings.theme.clone();
            let bytes = serde_json::to_vec_pretty(&s)?;
            if inner.settings_ok {
                write_atomic(&settings_path(), &bytes)?;
                crate::notices::use_settings(&s);
                inner.settings = s;
                Ok(())
            } else {
                crate::notices::use_settings(&s);
                inner.settings = s;
                Err(anyhow!(SETTINGS_UNREAD))
            }
        };
        self.schedule();
        saved
    }

    /// Changes a few settings without checking the download folder (first-start setup must
    /// close even with that drive unplugged).
    pub fn change_settings(&self, f: impl FnOnce(&mut Settings)) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        f(&mut inner.settings);
        crate::notices::use_settings(&inner.settings);
        if !inner.settings_ok {
            bail!(SETTINGS_UNREAD);
        }
        let bytes = serde_json::to_vec_pretty(&inner.settings)?;
        write_atomic(&settings_path(), &bytes)?;
        Ok(())
    }

    fn list_changed(&self) {
        (self.emit)("list-changed", serde_json::Value::Null);
    }

    // ---------- the "Download with Pako" window (see ask.rs) ----------

    /// Waits for the Download with Pako window's answer: not started before it. A running one
    /// (a browser download Pako just checked) stops where it is.
    pub fn hold_for_ask(&self, id: &str) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(job) = inner.jobs.get(id) {
            job.cancel.cancel();
        }
        if let Some(d) = inner.downloads.iter_mut().find(|d| d.id == id) {
            d.ask = true;
            if d.status.is_active() || d.status == Status::Paused || d.status == Status::Queued {
                d.status = Status::Queued;
                d.message = Some(ASK_NOTE.into());
            }
        }
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
    }

    /// The downloads waiting for the window, oldest first.
    pub fn asking(&self) -> Vec<String> {
        let inner = self.inner.lock().unwrap();
        let mut list: Vec<&Download> = inner.downloads.iter().filter(|d| d.ask && d.status != Status::Completed).collect();
        list.sort_by_key(|d| d.date);
        list.into_iter().map(|d| d.id.clone()).collect()
    }

    /// What the window shows for one download.
    pub fn ask_view(&self, id: &str, position: usize, total: usize) -> Option<AskView> {
        let inner = self.inner.lock().unwrap();
        let d = inner.downloads.iter().find(|d| d.id == id && d.ask)?;
        let path = std::path::Path::new(&d.name);
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| d.name.clone());
        let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        let site = url::Url::parse(&d.link)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string()))
            .unwrap_or_default();
        let (video, quality, sizes) = match &d.source {
            Source::Video { quality, audio, options, audio_size, .. } => {
                let q = if audio.is_some() { 0 } else { quality.unwrap_or(inner.settings.video_quality) };
                let sizes = crate::extension::VIDEO_QUALITIES.iter().map(|&w| (w, crate::video::estimate(options, *audio_size, w))).collect();
                (true, q, sizes)
            }
            _ => (false, 0, Vec::new()),
        };
        Some(AskView {
            id: d.id.clone(),
            stem,
            ext,
            site,
            size: d.size,
            dir: d.dest_dir.to_string_lossy().to_string(),
            video,
            quality,
            sizes,
            position,
            total,
            running: inner.jobs.len(),
            notify: inner.settings.ask_notify,
        })
    }

    /// The window's answer: start it (now, or in the queue) with the chosen name, folder,
    /// quality and notification, or take it off the list.
    pub fn answer_ask(self: &Arc<Self>, a: AskAnswer) -> Result<()> {
        if !matches!(a.action.as_str(), "now" | "queue" | "cancel") {
            bail!("Pako didn't understand that answer.");
        }
        // Only a download that is waiting for this window can be answered (and only then can
        // Cancel delete its file): anything else in the list is left alone.
        if !self.inner.lock().unwrap().downloads.iter().any(|d| d.id == a.id && d.ask) {
            bail!("This download is no longer waiting.");
        }
        if a.action == "cancel" {
            crate::extension::forget_headers(&a.id);
            self.remove(std::slice::from_ref(&a.id), true);
            return Ok(());
        }
        let dir = PathBuf::from(a.dir.trim());
        if a.dir.trim().is_empty() || !dir.is_absolute() {
            bail!("Pick a folder to save to.");
        }
        std::fs::create_dir_all(&dir).map_err(|_| anyhow!("Can't use that folder. Pick another one."))?;
        let mut inner = self.inner.lock().unwrap();
        if inner.jobs.contains_key(&a.id) {
            bail!("Pako is still pausing this download. Try again in a moment.");
        }
        let running = inner.jobs.len();
        let taken = taken_paths(&inner.downloads, Some(&a.id));
        let d = inner.downloads.iter_mut().find(|d| d.id == a.id && d.ask).ok_or_else(|| anyhow!("This download is no longer in Pako."))?;
        let video = matches!(d.source, Source::Video { .. });
        if let Source::Video { quality, audio, options, audio_size, .. } = &mut d.source {
            if let Some(q) = a.quality.filter(|q| crate::extension::VIDEO_QUALITIES.contains(q)) {
                *quality = Some(q);
                *audio = if q == 0 { Some(audio.clone().filter(|x| x == "mp3").unwrap_or_else(|| "m4a".into())) } else { None };
                d.size = crate::video::estimate(options, *audio_size, q);
            }
        }
        // The kind of file stays: a video's comes from its quality, anything else keeps its own.
        let ext = match &d.source {
            Source::Video { audio: Some(a), .. } if a == "mp3" => "mp3".to_string(),
            Source::Video { audio: Some(_), .. } => "m4a".to_string(),
            _ if video => "mp4".to_string(),
            _ => std::path::Path::new(&d.name).extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default(),
        };
        let stem = names::sanitize(a.stem.trim());
        let stem = if stem.trim().is_empty() {
            std::path::Path::new(&d.name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Download".into())
        } else {
            stem
        };
        let wanted = if ext.is_empty() { stem } else { format!("{stem}.{ext}") };
        // A browser download Pako already began (a few seconds, to check it): moving or
        // renaming it starts it again from zero.
        if (dir != d.dest_dir || wanted != d.name) && (d.downloaded > 0 || !d.segments.is_empty() || d.has_part()) {
            d.remove_part();
            d.segments.clear();
            d.downloaded = 0;
            recheck_server(d);
        }
        d.name = ask_name(&d.dest_dir, &d.name, &dir, &wanted, &taken);
        d.dest_dir = dir;
        d.notify = a.notify;
        d.quiet = !a.notify;
        d.ask = false;
        d.message = None;
        if a.action == "now" && running > 0 {
            d.priority = Priority::First;
        }
        inner.dirty = true;
        drop(inner);
        let _ = self.change_settings(|s| {
            s.ask_notify = a.notify;
            if a.dont_ask {
                s.ask_browser = false;
            }
        });
        self.save_now();
        self.list_changed();
        self.schedule();
        Ok(())
    }

    // ---------- adding ----------

    pub async fn add_links(self: &Arc<Self>, text: &str, dest: Option<PathBuf>) -> AddResult {
        let dest = dest.unwrap_or_else(|| self.settings().download_dir);
        let mut added = 0;
        let mut errors = Vec::new();
        for link in extract_links(text) {
            match self.add_one(&link, &dest).await {
                Ok(n) => added += n,
                Err(e) => errors.push(format!("{}: {e}", short(&link))),
            }
        }
        if added > 0 {
            self.save_now();
            self.list_changed();
            self.schedule();
        }
        AddResult { added, errors, skipped: 0 }
    }

    /// A request from the browser extension. Downloads go straight to Downloads (videos to Links,
    /// to pick the quality); "all links on this page" go to Links. Pako Light's links never
    /// become videos (`r.light`).
    pub async fn add_from_browser(self: &Arc<Self>, r: crate::extension::Request) -> BrowserAdded {
        if r.kind == "links" {
            let res = self.found_add_from(&r.urls.join("\n"), r.light).await;
            if res.added > 0 {
                self.notify(
                    "browser",
                    "Links added",
                    &format!("{} from your browser, ready to pick in Links.", if res.added == 1 { "1 link".to_string() } else { format!("{} links", res.added) }),
                    Go::page("found"),
                );
            }
            return BrowserAdded { open_links: true, ids: Vec::new() };
        }
        let dest = self.settings().download_dir;
        let (videos, files): (Vec<String>, Vec<String>) = r.urls.iter().cloned().partition(|u| crate::video::is_video_site(u));
        // Pako Light: never a video (its video links were already left out).
        let videos = if r.light { Vec::new() } else { videos };
        let mut added = Vec::new();
        // The downloads made here (a Drive link opened through add_one has none to show).
        let mut added_ids: Vec<String> = Vec::new();
        // The Download with Pako window first (a download the browser holds is asked about
        // once Pako has checked it: see browser_request).
        let ask = !r.takeover && crate::ask::wanted();
        let mut held = Vec::new();
        for url in files {
            if drive::parse_link(&url).is_some() {
                let before: HashSet<String> = self.inner.lock().unwrap().downloads.iter().map(|d| d.id.clone()).collect();
                if let Ok(n) = self.add_one(&url, &dest).await {
                    if n > 0 {
                        added.push(names::from_url(&url));
                    }
                }
                // The Drive file (or a folder's files) this link just made.
                let made: Vec<String> =
                    self.inner.lock().unwrap().downloads.iter().filter(|d| d.link == url && !before.contains(&d.id)).map(|d| d.id.clone()).collect();
                added_ids.extend(made);
                continue;
            }
            let name = browser_name(&r.filename, &url, r.urls.len() > 1);
            let mut d = new_download(&url, Source::Http { url: url.clone() }, name.clone(), dest.clone(), None);
            crate::extension::remember_headers(&d.id, crate::extension::BrowserHeaders { referrer: r.referrer.clone(), cookies: r.cookies.clone(), ..Default::default() });
            d.ask = ask;
            if ask {
                d.message = Some(ASK_NOTE.into());
                held.push(d.id.clone());
            }
            added_ids.push(d.id.clone());
            self.push(d);
            added.push(name);
        }
        if !added.is_empty() {
            self.save_now();
            self.list_changed();
            self.schedule();
            if held.is_empty() {
                let body = if added.len() == 1 { added[0].clone() } else { format!("{} files", added.len()) };
                let one = (added.len() == 1 && added_ids.len() == 1).then(|| added_ids[0].clone());
                self.notify("browser", "Downloading in Pako", &body, Go::maybe("downloads", one));
                if !r.takeover {
                    crate::ask::started();
                }
            }
            for id in &held {
                crate::ask::enqueue(id);
            }
        }
        if !videos.is_empty() {
            self.found_add(&videos.join("\n")).await;
            self.notify("browser", "Video sent to Pako", "Pick the quality in Links, then press Download.", Go::page("found"));
            return BrowserAdded { open_links: true, ids: added_ids };
        }
        BrowserAdded { open_links: false, ids: added_ids }
    }

    async fn add_one(&self, link: &str, dest: &PathBuf) -> Result<usize> {
        if let Some(dl) = drive::parse_link(link) {
            return self.add_drive(link, dl, dest).await;
        }
        let url = url::Url::parse(link).map_err(|_| anyhow!("not a web link"))?;
        if !matches!(url.scheme(), "http" | "https") {
            bail!("only http and https links are supported");
        }
        let name = names::from_url(link);
        self.push(new_download(link, Source::Http { url: link.into() }, name, dest.clone(), None));
        Ok(1)
    }

    async fn add_drive(&self, link: &str, dl: drive::DriveLink, dest: &PathBuf) -> Result<usize> {
        // A file whose details Google won't give (no app key, not public, Google busy):
        // added as is; its first download request tells the rest (or that it needs the browser).
        let plain = |dl: drive::DriveLink| {
            let src = Source::GDrive { file_id: dl.id.clone(), resource_key: dl.resource_key, doc_type: dl.doc_type };
            new_download(link, src, format!("Drive file {}", dl.id), dest.clone(), None)
        };
        let Some(key) = drive::api_key() else {
            if dl.kind == LinkKind::Folder {
                bail!(drive::NO_KEY);
            }
            self.push(plain(dl));
            return Ok(1);
        };
        let api = DriveApi { http: &self.http, key: &key };
        let meta = match api.meta(&dl.id, dl.resource_key.as_deref()).await {
            Ok(m) => m,
            Err(e) if dl.kind == LinkKind::Folder => {
                if let Some(m) = drive::folder_browser_message(&e) {
                    bail!(m);
                }
                return Err(e);
            }
            Err(_) => {
                self.push(plain(dl));
                return Ok(1);
            }
        };
        if meta.is_folder() {
            let folder_name = names::sanitize(&meta.name);
            let files = api
                .list_recursive(&meta.id, meta.resource_key.as_deref().or(dl.resource_key.as_deref()))
                .await
                .map_err(|e| match drive::folder_browser_message(&e) {
                    Some(m) => anyhow!(m),
                    None => e,
                })?;
            if files.is_empty() {
                bail!("the folder is empty");
            }
            let root = dest.join(&folder_name);
            let mut count = 0;
            for (rel, f) in files {
                if let Some(d) = drive_download(link, &f, root.join(rel), Some(folder_name.clone())) {
                    self.push(d);
                    count += 1;
                }
            }
            return Ok(count);
        }
        let d = drive_download(link, &meta, dest.clone(), None)
            .ok_or_else(|| anyhow!("this kind of Google file (form, site...) can't be downloaded"))?;
        self.push(d);
        Ok(1)
    }

    /// Adds to the list, picking a name that doesn't overwrite anything.
    fn push(&self, d: Download) {
        self.push_many(vec![d]);
    }

    /// Adds several at once: folders are created first (outside the lock: a slow or sleeping
    /// drive doesn't freeze everything), and the taken names are worked out once, not once per
    /// file (a 5,000-file Drive folder used to take ages).
    pub(super) fn push_many(&self, mut list: Vec<Download>) {
        let dirs: HashSet<PathBuf> = list.iter().map(|d| d.dest_dir.clone()).collect();
        for dir in &dirs {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut inner = self.inner.lock().unwrap();
        let mut taken = taken_paths(&inner.downloads, None);
        for d in list.iter_mut() {
            d.name = names::unique(&d.dest_dir, &d.name, &taken);
            taken.insert(names::path_key(&d.final_path()));
        }
        if !list.is_empty() {
            // Seen as work even if it's done before the next look (a tiny file): "when done" waits for it.
            inner.had_work = true;
        }
        inner.downloads.extend(list);
        inner.dirty = true;
    }

    // ---------- user actions ----------

    pub fn pause(&self, ids: &[String]) {
        let mut inner = self.inner.lock().unwrap();
        // Paused last: the list going quiet now isn't "all downloads finished".
        if !ids.is_empty() {
            inner.done_any = false;
        }
        for id in ids {
            if let Some(job) = inner.jobs.get(id) {
                job.cancel.cancel();
            }
            if let Some(d) = inner.downloads.iter_mut().find(|d| &d.id == id) {
                if matches!(d.status, Status::Queued) || d.status.is_active() {
                    d.status = Status::Paused;
                    d.message = None;
                }
            }
        }
        inner.dirty = true;
        drop(inner);
        self.list_changed();
    }

    /// Tray menu: pause everything running or waiting.
    pub fn pause_all(&self) {
        let ids: Vec<String> = {
            let inner = self.inner.lock().unwrap();
            inner.downloads.iter().filter(|d| d.status == Status::Queued || d.status.is_active()).map(|d| d.id.clone()).collect()
        };
        self.pause(&ids);
    }

    /// Tray menu: resume everything paused (not the ones that failed).
    pub fn resume_all(self: &Arc<Self>) {
        let ids: Vec<String> = {
            let inner = self.inner.lock().unwrap();
            inner.downloads.iter().filter(|d| d.status == Status::Paused).map(|d| d.id.clone()).collect()
        };
        self.resume(&ids);
    }

    pub fn resume(self: &Arc<Self>, ids: &[String]) {
        let mut inner = self.inner.lock().unwrap();
        for id in ids {
            inner.auto_retry.remove(id);
            if let Some(d) = inner.downloads.iter_mut().find(|d| &d.id == id) {
                if matches!(d.status, Status::Paused | Status::Failed | Status::QuotaExceeded | Status::NeedsAccount) {
                    d.status = Status::Queued;
                    d.message = None;
                }
            }
        }
        inner.dirty = true;
        drop(inner);
        self.list_changed();
        self.schedule();
    }

    /// Removes from the list. Unfinished parts are always deleted; finished files only when
    /// `delete_files` is set (they go to the Recycle Bin).
    pub fn remove(self: &Arc<Self>, ids: &[String], delete_files: bool) {
        // Videos not finished: yt-dlp's pieces go too (folder, name, running).
        let mut videos: Vec<(PathBuf, String, String, bool)> = Vec::new();
        let mut inner = self.inner.lock().unwrap();
        for id in ids {
            let running = inner.jobs.get(id).cloned();
            if let Some(job) = &running {
                job.cancel.cancel();
            }
            if let Some(pos) = inner.downloads.iter().position(|d| &d.id == id) {
                let d = inner.downloads.remove(pos);
                if running.is_none() {
                    d.remove_part();
                }
                if matches!(d.source, Source::Video { .. }) && d.status != Status::Completed {
                    videos.push((d.dest_dir.clone(), d.name.clone(), d.id.clone(), running.is_some()));
                }
                // (A running job deletes its part file when it stops; see job_ended.)
                if delete_files && d.status == Status::Completed {
                    let _ = trash::delete(d.final_path());
                }
            }
            inner.speeds.remove(id);
            // Its browser cookies / referrer go with it.
            crate::extension::forget_headers(id);
        }
        inner.dirty = true;
        drop(inner);
        for (dir, name, vid, running) in videos {
            // The pieces as they are now (not whatever is there later: the same video may be
            // added again in the meantime and continue them).
            let listed = self.video_leftovers(&dir, &name, &vid);
            if !running {
                for p in listed {
                    let _ = std::fs::remove_file(p);
                }
                continue;
            }
            // After the running yt-dlp has been stopped (it holds the pieces open). Pieces it
            // made since are taken too, unless a download with that name is back in the list.
            let me = self.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let stem = std::path::Path::new(&name).file_stem().map(|s| s.to_os_string());
                let back = me.inner.lock().unwrap().downloads.iter().any(|d| {
                    d.dest_dir == dir && std::path::Path::new(&d.name).file_stem().map(|s| s.to_os_string()) == stem
                });
                if back {
                    return;
                }
                let mut all = listed;
                for p in me.video_leftovers(&dir, &name, &vid) {
                    if !all.contains(&p) {
                        all.push(p);
                    }
                }
                for p in all {
                    let _ = std::fs::remove_file(p);
                }
            });
        }
        self.save_now();
        self.list_changed();
    }

    /// yt-dlp's pieces of the video download `name` in `dir` (never another download's).
    /// Reads the folder: call it without holding the list.
    fn video_leftovers(&self, dir: &std::path::Path, name: &str, id: &str) -> Vec<PathBuf> {
        let others = other_names(&self.inner.lock().unwrap().downloads, dir, name);
        crate::video::leftovers(dir, name, id, &others)
    }

    pub fn clear_completed(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.downloads.retain(|d| {
            let keep = d.status != Status::Completed;
            if !keep {
                crate::extension::forget_headers(&d.id);
            }
            keep
        });
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
    }

    /// "Download first": priority First, and start (paused or stopped ones too). If every
    /// spot is busy, the newest lower-priority download steps aside for it (back to waiting,
    /// nothing lost). The "At once" limit always holds.
    pub fn download_first(self: &Arc<Self>, ids: &[String]) {
        let mut inner = self.inner.lock().unwrap();
        // Only outside the download hours: during them it runs anyway, and stops with the rest
        // when they end.
        let closed = first_needs_override(&inner.settings, crate::system::local_minutes());
        for id in ids {
            inner.auto_retry.remove(id);
            if closed {
                inner.sched_override.insert(id.clone());
            }
            let running = inner.jobs.get(id).map(|j| !j.cancel.is_cancelled()).unwrap_or(false);
            if let Some(d) = inner.downloads.iter_mut().find(|d| &d.id == id) {
                if d.status == Status::Completed {
                    continue;
                }
                d.priority = Priority::First;
                if !running {
                    d.status = Status::Queued;
                    d.message = None;
                }
            }
        }
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
        self.schedule();
    }

    /// "Extract when all parts are done" for split archives. Turned on for a set that is
    /// already complete: extract it now.
    pub fn set_extract(self: &Arc<Self>, ids: &[String], on: bool) {
        let mut groups = Vec::new();
        {
            let mut inner = self.inner.lock().unwrap();
            for d in inner.downloads.iter_mut().filter(|d| ids.contains(&d.id)) {
                d.extract = on;
                if let Some(g) = &d.group {
                    groups.push((g.clone(), d.dest_dir.clone()));
                }
            }
            inner.dirty = true;
        }
        self.save_now();
        self.list_changed();
        if on {
            groups.sort();
            groups.dedup();
            for (g, dir) in groups {
                self.extract_when_complete(&g, &dir);
            }
        }
    }

    /// Extracts a split archive with 7-Zip once every part of the set is downloaded.
    /// The result is noted on the set's first part ("Extracted", or why not).
    fn extract_when_complete(self: &Arc<Self>, group: &str, dir: &PathBuf) {
        // Every split set in the folder (a Drive folder can hold several), once all is done.
        let (jobs, notify) = {
            let inner = self.inner.lock().unwrap();
            let files: Vec<&Download> =
                inner.downloads.iter().filter(|d| d.group.as_deref() == Some(group) && &d.dest_dir == dir).collect();
            let names: Vec<&str> = files.iter().map(|d| d.name.as_str()).collect();
            // Each set on its own: other files in the same Drive folder (a readme that failed,
            // a Google form that can't download) don't hold it back.
            let mut running = EXTRACTING.lock().unwrap();
            let jobs: Vec<(String, PathBuf, String)> = crate::split::find_sets(&names)
                .into_iter()
                .filter(|set| {
                    set.members.iter().all(|&m| files[m].status == Status::Completed)
                        && set.members.iter().any(|&m| files[m].extract)
                })
                .map(|set| (files[set.first].id.clone(), files[set.first].final_path(), set.base))
                // Never two 7-Zip runs on the same set at once.
                .filter(|(_, path, _)| running.insert(names::path_key(path)))
                .collect();
            (jobs, files.iter().any(|d| d.notify))
        };
        if jobs.is_empty() {
            return;
        }
        let me = self.clone();
        let out_dir = dir.clone();
        tauri::async_runtime::spawn(async move {
            let mut failures = Vec::new();
            for (first_id, first_path, base) in &jobs {
                let out = out_dir.clone();
                let first = first_path.clone();
                let result = match crate::components::seven_zip() {
                    None => Err("Extracting needs 7-Zip: install it in Settings › Features & updates.".to_string()),
                    Some(exe) => tauri::async_runtime::spawn_blocking(move || {
                        let mut cmd = std::process::Command::new(exe);
                        // -snz: extracted files keep the "downloaded from the internet" mark.
                        // -aou: a file of the same name already in the folder (another file of
                        // a Drive folder, an earlier extraction) is never written over: the
                        // extracted one gets a new name ("readme_1.txt").
                        cmd.arg("x").arg("-y").arg("-aou").arg("-snz").arg(format!("-o{}", out.display())).arg(&first);
                        #[cfg(windows)]
                        {
                            use std::os::windows::process::CommandExt;
                            cmd.creation_flags(0x0800_0000); // no console window
                        }
                        match cmd.output() {
                            Ok(o) if o.status.success() => Ok(()),
                            Ok(o) => {
                                let err = String::from_utf8_lossy(&o.stderr);
                                let line = err.lines().find(|l| !l.trim().is_empty()).unwrap_or("7-Zip stopped with an error");
                                Err(format!("Couldn't extract: {}", line.trim()))
                            }
                            Err(e) => Err(format!("Couldn't start 7-Zip: {e}")),
                        }
                    })
                    .await
                    .unwrap_or_else(|_| Err("Extracting crashed".into())),
                };
                EXTRACTING.lock().unwrap().remove(&names::path_key(first_path));
                if let Err(e) = &result {
                    failures.push(format!("{base}: {e}"));
                }
                {
                    let mut inner = me.inner.lock().unwrap();
                    if let Some(d) = inner.downloads.iter_mut().find(|d| &d.id == first_id) {
                        d.message = Some(match &result {
                            Ok(()) => "Extracted".into(),
                            Err(e) => e.clone(),
                        });
                    }
                    inner.dirty = true;
                }
                me.save_now();
                me.list_changed();
            }
            if notify {
                let names: Vec<&str> = jobs.iter().map(|(_, _, b)| b.as_str()).collect();
                let (kind, title, body) = if failures.is_empty() {
                    ("download-done", "Extracted".to_string(), names.join(", "))
                } else {
                    ("download-failed", "Couldn't extract everything".to_string(), failures.join("; "))
                };
                // Asked for by the set's own "Notify when done".
                let first = jobs.first().map(|(id, _, _)| id.clone());
                (me.emit)("notify", notices::payload(kind, &title, &body, &Go::maybe("downloads", first), Pop::Force));
            }
        });
    }

    /// Split sets in the Found list (links not inside a Drive folder), for the window.
    pub fn found_sets(&self) -> Vec<crate::split::SetView> {
        let inner = self.inner.lock().unwrap();
        let loose: Vec<&FoundItem> = inner.found.iter().filter(|f| f.folder.is_none()).collect();
        let names: Vec<&str> = loose.iter().map(|f| f.template.name.as_str()).collect();
        crate::split::find_sets(&names)
            .into_iter()
            .map(|s| crate::split::SetView {
                ids: s.members.iter().map(|&i| loose[i].id.clone()).collect(),
                key: s.key,
                base: s.base,
                missing: s.missing,
            })
            .collect()
    }

    /// Split sets among the downloads, per folder (for the extract switch and missing parts).
    pub fn download_sets(&self) -> Vec<crate::split::SetView> {
        let inner = self.inner.lock().unwrap();
        let mut by_folder: HashMap<(String, PathBuf), Vec<&Download>> = HashMap::new();
        for d in inner.downloads.iter().filter(|d| d.group.is_some()) {
            by_folder.entry((d.group.clone().unwrap_or_default(), d.dest_dir.clone())).or_default().push(d);
        }
        let mut out = Vec::new();
        for files in by_folder.values() {
            let names: Vec<&str> = files.iter().map(|d| d.name.as_str()).collect();
            for s in crate::split::find_sets(&names) {
                out.push(crate::split::SetView {
                    ids: s.members.iter().map(|&i| files[i].id.clone()).collect(),
                    key: s.key,
                    base: s.base,
                    missing: s.missing,
                });
            }
        }
        out
    }

    /// The folder, group and extract choice of a split set already in the downloads, for a
    /// part of it downloaded later (so the set stays together).
    pub(super) fn existing_set_place(&self, name: &str) -> Option<(PathBuf, String, bool)> {
        let key = crate::split::set_key(name)?;
        let inner = self.inner.lock().unwrap();
        inner
            .downloads
            .iter()
            // Only a set still being downloaded: a finished one with the same name is an older
            // copy, and joining it would drop the new parts into its folder (renamed).
            .find(|d| {
                d.group.is_some() && d.status != Status::Completed && crate::split::set_key(&d.name).as_deref() == Some(key.as_str())
            })
            .map(|d| (d.dest_dir.clone(), d.group.clone().unwrap_or_default(), d.extract))
    }

    /// "Notify when done" for these downloads.
    pub fn set_notify(&self, ids: &[String], on: bool) {
        let mut inner = self.inner.lock().unwrap();
        for d in inner.downloads.iter_mut().filter(|d| ids.contains(&d.id)) {
            d.notify = on;
        }
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
    }

    /// A new save folder for downloads that haven't started writing yet (once a file is
    /// being written, it stays where it is).
    pub fn set_folder(&self, ids: &[String], dir: PathBuf) -> Result<()> {
        std::fs::create_dir_all(&dir).map_err(|_| anyhow!("Can't use that folder."))?;
        let mut inner = self.inner.lock().unwrap();
        let mut moved = 0;
        for id in ids {
            if inner.jobs.contains_key(id) {
                continue;
            }
            let taken = taken_paths(&inner.downloads, Some(id));
            let Some(d) = inner.downloads.iter_mut().find(|d| &d.id == id) else { continue };
            if d.status == Status::Completed || d.downloaded > 0 || d.has_part() {
                continue;
            }
            d.dest_dir = dir.clone();
            d.name = names::unique(&d.dest_dir, &d.name, &taken);
            moved += 1;
        }
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
        if moved == 0 && !ids.is_empty() {
            bail!("Already started: a download stays in the folder it began in.");
        }
        Ok(())
    }

    /// Changes the priority (First, Normal, Later). Doesn't start or stop anything by itself,
    /// except that a waiting download set to First may take a spot right away.
    pub fn set_priority(self: &Arc<Self>, ids: &[String], priority: Priority) {
        let mut inner = self.inner.lock().unwrap();
        for d in inner.downloads.iter_mut().filter(|d| ids.contains(&d.id)) {
            d.priority = priority;
        }
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
        self.schedule();
    }

    /// "Start over": throw away what was downloaded and download again from zero.
    pub fn start_over(self: &Arc<Self>, ids: &[String]) {
        let mut inner = self.inner.lock().unwrap();
        for id in ids {
            inner.auto_retry.remove(id);
            let running = inner.jobs.get(id).cloned();
            // A video: yt-dlp's pieces go first, or it would continue them (a running one's
            // are deleted in job_ended, once yt-dlp has stopped).
            if running.is_none() {
                let place = inner.downloads.iter().find(|d| &d.id == id && matches!(d.source, Source::Video { .. }));
                if let Some(d) = place {
                    let others = other_names(&inner.downloads, &d.dest_dir, &d.name);
                    for p in crate::video::leftovers(&d.dest_dir, &d.name, &d.id, &others) {
                        let _ = std::fs::remove_file(p);
                    }
                }
            }
            if let Some(job) = &running {
                // Finished in job_ended, once the connections are closed.
                job.reset.store(true, Ordering::Relaxed);
                job.cancel.cancel();
            }
            if let Some(d) = inner.downloads.iter_mut().find(|d| &d.id == id) {
                if running.is_none() {
                    // A finished file goes to the Recycle Bin before downloading it again.
                    if d.status == Status::Completed {
                        let _ = trash::delete(d.final_path());
                    }
                    d.remove_part();
                    d.segments.clear();
                    if let Source::Video { streams, .. } = &mut d.source {
                        streams.clear();
                    }
                    d.downloaded = 0;
                    d.status = Status::Queued;
                    d.message = None;
                    d.finished_at = None;
                    recheck_server(d);
                }
            }
        }
        inner.dirty = true;
        drop(inner);
        // On disk before the next run lays the file out again: a crash in between must not
        // leave the old parts ("half done") saved next to a new, empty file.
        self.save_now();
        self.list_changed();
        self.schedule();
    }

    /// Renames a download (the finished file on disk, or the name it will get).
    pub fn rename(&self, id: &str, new_name: &str) -> Result<String> {
        let mut inner = self.inner.lock().unwrap();
        if inner.jobs.contains_key(id) {
            bail!("Pause the download first, then rename it.");
        }
        let taken = taken_paths(&inner.downloads, Some(id));
        let others = {
            let d = inner.downloads.iter().find(|d| d.id == id).ok_or_else(|| anyhow!("Download not found"))?;
            other_names(&inner.downloads, &d.dest_dir, &d.name)
        };
        let d = inner.downloads.iter_mut().find(|d| d.id == id).ok_or_else(|| anyhow!("Download not found"))?;
        let wanted = names::sanitize(new_name.trim());
        if wanted == d.name {
            return Ok(wanted);
        }
        // Only capitals changed ("file.zip" to "File.zip"): Windows sees the same name, so its
        // own file isn't "already there".
        let name = if wanted.to_lowercase() == d.name.to_lowercase() { wanted.clone() } else { names::unique(&d.dest_dir, &wanted, &taken) };
        if d.status == Status::Completed {
            std::fs::rename(d.final_path(), d.dest_dir.join(&name))
                .map_err(|e| anyhow!("Couldn't rename the file: {e}"))?;
        } else if matches!(d.source, Source::Video { .. }) {
            // A video not finished: its pieces (Pako's own streams and yt-dlp's) are named after
            // it. They take the new name, so it carries on from them and none stays behind.
            move_video_pieces(&d.dest_dir, &d.name, &name, &d.id, &others);
        } else {
            // Left unfinished by an older Pako: renamed under the new name in one go (straight
            // from its old ending if taking the new one failed, so it never stays behind under
            // the old name).
            d.adopt_old_part();
            let from = [d.part_path(), d.old_part_path()].into_iter().find(|p| p.exists());
            if let Some(from) = from {
                std::fs::rename(from, names::part_of(&d.dest_dir, &name)).map_err(|e| anyhow!("Couldn't rename the file: {e}"))?;
            }
        }
        d.name = name.clone();
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
        Ok(name)
    }

    /// Drag and drop in the list: puts these downloads just before `target` in the queue
    /// (None = at the end), keeping their order.
    pub fn move_before(self: &Arc<Self>, ids: &[String], target: Option<&str>) {
        let mut inner = self.inner.lock().unwrap();
        inner.downloads.sort_by_key(|d| d.added_at);
        let base = inner.downloads.first().map(|d| d.added_at).unwrap_or(0);
        let (moving, mut rest): (Vec<Download>, Vec<Download>) =
            std::mem::take(&mut inner.downloads).into_iter().partition(|d| ids.contains(&d.id));
        let at = target.and_then(|t| rest.iter().position(|d| d.id == t)).unwrap_or(rest.len());
        rest.splice(at..at, moving);
        for (k, d) in rest.iter_mut().enumerate() {
            d.added_at = base + k as u64;
        }
        inner.downloads = rest;
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
        self.schedule();
    }

    /// Undo for "Remove from list" of finished downloads: puts them back as they were.
    pub fn restore_downloads(&self, list: Vec<Download>) {
        let mut inner = self.inner.lock().unwrap();
        for d in list {
            if d.status == Status::Completed && !inner.downloads.iter().any(|x| x.id == d.id) {
                inner.downloads.push(d);
            }
        }
        inner.downloads.sort_by_key(|d| d.added_at);
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
    }

    /// Moves downloads to the top (start next) or bottom (start last) of the queue.
    pub fn move_in_queue(&self, ids: &[String], to_top: bool) {
        let mut inner = self.inner.lock().unwrap();
        let min = inner.downloads.iter().map(|d| d.added_at).min().unwrap_or(0);
        let max = inner.downloads.iter().map(|d| d.added_at).max().unwrap_or(0);
        let n = ids.len() as u64;
        for (k, id) in ids.iter().enumerate() {
            if let Some(d) = inner.downloads.iter_mut().find(|d| &d.id == id) {
                d.added_at = if to_top { min.saturating_sub(n - k as u64) } else { max + 1 + k as u64 };
            }
        }
        inner.downloads.sort_by_key(|d| d.added_at);
        inner.dirty = true;
        drop(inner);
        self.save_now();
        self.list_changed();
    }

    /// Which parts of the file are done (for the map in the details panel).
    pub fn parts(&self, id: &str) -> Vec<PartView> {
        let inner = self.inner.lock().unwrap();
        if let Some(job) = inner.jobs.get(id) {
            return job
                .slots
                .lock()
                .unwrap()
                .iter()
                .map(|s| PartView { start: s.seg.start, end: s.seg.end, done: s.seg.done, active: s.busy })
                .collect();
        }
        inner
            .downloads
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.segments.iter().map(|s| PartView { start: s.start, end: s.end, done: s.done, active: false }).collect())
            .unwrap_or_default()
    }

    /// Connections a running download uses right now (for the test mode's output).
    pub fn live_connections(&self, id: &str) -> u32 {
        self.inner.lock().unwrap().jobs.get(id).map(|j| j.connections.load(Ordering::Relaxed)).unwrap_or(0)
    }

    pub fn find(&self, id: &str) -> Option<Download> {
        self.inner.lock().unwrap().downloads.iter().find(|d| d.id == id).cloned()
    }

    // ---------- scheduling ----------

    fn schedule(self: &Arc<Self>) {
        let mut to_start = Vec::new();
        {
            let mut inner = self.inner.lock().unwrap();
            let max = inner.settings.max_active as usize;
            let mut running = inner.jobs.len();
            let hours_open = in_hours(&inner.settings);
            // A download whose previous run is still shutting down (paused and resumed
            // quickly) waits until that run has fully stopped: never two runs on one file.
            let mut queued: Vec<usize> = (0..inner.downloads.len())
                .filter(|&i| {
                    let d = &inner.downloads[i];
                    // Waiting for the Download with Pako window: not started before its answer.
                    d.status == Status::Queued && !d.ask && !inner.jobs.contains_key(&d.id)
                })
                .collect();
            // First, then Normal, then Later; same level in the order they were added.
            queued.sort_by_key(|&i| (inner.downloads[i].priority, inner.downloads[i].added_at));
            // Spots already being freed for a First download (asked to step aside, still stopping).
            // Paused ones that are still closing their connections count too.
            let mut freeing = inner
                .jobs
                .values()
                .filter(|j| j.cancel.is_cancelled())
                .filter(|j| {
                    j.requeue.load(Ordering::Relaxed)
                        || inner.downloads.iter().any(|d| d.id == j.id && d.status == Status::Paused)
                })
                .count();
            for i in queued {
                // Outside the download hours only "Download first" items start.
                if !hours_open && !inner.sched_override.contains(&inner.downloads[i].id) {
                    continue;
                }
                if running >= max {
                    if inner.downloads[i].priority != Priority::First {
                        break; // the rest are lower or equal: they wait their turn
                    }
                    if freeing > 0 {
                        freeing -= 1; // a spot is on its way for this one
                        continue;
                    }
                    // The newest running download with a lower priority steps aside.
                    let victim = inner
                        .jobs
                        .values()
                        .filter(|j| !j.cancel.is_cancelled())
                        // Never one in its final checksum check: that work would be thrown away.
                        .filter(|j| *j.phase.lock().unwrap() != Phase::Verifying)
                        .filter_map(|j| {
                            let p = inner.downloads.iter().find(|d| d.id == j.id)?.priority;
                            (p > Priority::First).then_some((p, j.started, j.clone()))
                        })
                        .max_by_key(|(p, started, _)| (*p, *started));
                    match victim {
                        Some((_, _, job)) => {
                            job.requeue.store(true, Ordering::Relaxed);
                            job.cancel.cancel();
                        }
                        None => break, // every spot is taken by First downloads: wait
                    }
                    continue;
                }
                let d = &mut inner.downloads[i];
                d.status = Status::Starting;
                d.message = None;
                let job = Arc::new(Job::new(d.id.clone(), CancellationToken::new(), d.downloaded, d.resumable));
                let id = d.id.clone();
                inner.jobs.insert(id, job.clone());
                inner.had_work = true;
                to_start.push(job);
                running += 1;
            }
        }
        if !to_start.is_empty() {
            self.list_changed();
        }
        for job in to_start {
            let me = self.clone();
            tauri::async_runtime::spawn(async move {
                let outcome = me.run(&job).await;
                me.job_ended(&job, outcome).await;
            });
        }
    }

    /// Copies live progress from running jobs into the saved list.
    fn sync_jobs(inner: &mut Inner) {
        let jobs: Vec<Arc<Job>> = inner.jobs.values().cloned().collect();
        for job in jobs {
            if let Some(d) = inner.downloads.iter_mut().find(|d| d.id == job.id) {
                let slots = job.slots.lock().unwrap();
                if !slots.is_empty() {
                    d.segments = slots.iter().map(|s| s.seg.clone()).collect();
                }
                drop(slots);
                d.downloaded = job.downloaded.load(Ordering::Relaxed);
                if d.status.is_active() {
                    let (status, msg) = job.live_status();
                    d.status = status;
                    d.message = msg;
                }
            }
        }
    }

    /// The web client (shared with the video preview).
    pub fn http_client(&self) -> reqwest::Client {
        self.http.clone()
    }

    /// Where a download or a Found link comes from, and its name (for the video preview).
    pub fn source_of(&self, id: &str) -> Option<(Source, String)> {
        let inner = self.inner.lock().unwrap();
        if let Some(d) = inner.downloads.iter().find(|d| d.id == id) {
            return Some((d.source.clone(), d.name.clone()));
        }
        inner.found.iter().find(|f| f.id == id).map(|f| (f.template.source.clone(), f.template.name.clone()))
    }

    /// A download finished and a notification is wanted. A folder or split set gets one
    /// notification when its last file is done; single files finishing close together are
    /// gathered into one (sent from `tick` after a short quiet moment). `force`: the download's
    /// own "Notify when done" asked for it.
    fn note_finished(&self, id: &str, force: bool) {
        let note = {
            let inner = self.inner.lock().unwrap();
            let Some(d) = inner.downloads.iter().find(|d| d.id == id) else { return };
            match &d.group {
                Some(g) => {
                    let root = group_root(d);
                    let files: Vec<&Download> =
                        inner.downloads.iter().filter(|x| x.group.as_deref() == Some(g.as_str()) && group_root(x) == root).collect();
                    let open = files.iter().any(|x| {
                        !matches!(x.status, Status::Completed | Status::Failed | Status::NeedsAccount | Status::QuotaExceeded)
                    });
                    if open {
                        return;
                    }
                    let done = files.iter().filter(|x| x.status == Status::Completed).count();
                    let stuck = files.len() - done;
                    let body = match (done, stuck) {
                        (1, 0) => "1 file done".to_string(),
                        (n, 0) => format!("{n} files done"),
                        (n, s) => format!("{n} done, {s} need you"),
                    };
                    Note {
                        title: g.clone(),
                        body,
                        name: g.clone(),
                        stopped: done == 0,
                        id: None,
                        force: force || files.iter().any(|x| x.notify),
                    }
                }
                None => Note {
                    title: "Download finished".to_string(),
                    body: d.name.clone(),
                    name: d.name.clone(),
                    stopped: false,
                    id: Some(d.id.clone()),
                    force,
                },
            }
        };
        let mut notes = self.notes.lock().unwrap();
        notes.0.push(note);
        notes.1 = Some(Instant::now());
    }

    /// A download stopped (limit, needs an account, error) and a notification is wanted. A
    /// folder or set says it once, when its last file is settled; single files are gathered.
    fn note_stopped(&self, id: &str, msg: &str, force: bool) {
        let grouped = self.inner.lock().unwrap().downloads.iter().any(|d| d.id == id && d.group.is_some());
        if grouped {
            self.note_finished(id, force);
            return;
        }
        let name = self.inner.lock().unwrap().downloads.iter().find(|d| d.id == id).map(|d| d.name.clone());
        let Some(name) = name else { return };
        let mut notes = self.notes.lock().unwrap();
        notes.0.push(Note {
            title: format!("Download stopped: {name}"),
            body: msg.to_string(),
            name,
            stopped: true,
            id: Some(id.to_string()),
            force,
        });
        notes.1 = Some(Instant::now());
    }

    /// Sends the gathered notifications once nothing new has finished for 3 seconds: one for
    /// what finished, one for what stopped (each kind has its own switches in Settings).
    fn flush_notes(&self) {
        let ready = {
            let mut notes = self.notes.lock().unwrap();
            if notes.0.is_empty() || notes.1.is_some_and(|t| t.elapsed() < Duration::from_secs(3)) {
                return;
            }
            notes.1 = None;
            std::mem::take(&mut notes.0)
        };
        let (stopped, finished): (Vec<Note>, Vec<Note>) = ready.into_iter().partition(|n| n.stopped);
        for (kind, group) in [("download-done", finished), ("download-failed", stopped)] {
            if group.is_empty() {
                continue;
            }
            let pop = if group.iter().any(|n| n.force) { Pop::Force } else { Pop::Pref };
            let (title, body, id) = if group.len() == 1 {
                let n = group.into_iter().next().unwrap();
                (n.title, n.body, n.id)
            } else {
                let names: Vec<&str> = group.iter().take(3).map(|n| n.name.as_str()).collect();
                let more = if group.len() > 3 { format!(" and {} more", group.len() - 3) } else { String::new() };
                let title = if kind == "download-done" {
                    format!("{} downloads finished", group.len())
                } else {
                    format!("{} downloads stopped", group.len())
                };
                (title, format!("{}{more}", names.join(", ")), None)
            };
            (self.emit)("notify", notices::payload(kind, &title, &body, &Go::maybe("downloads", id), pop));
        }
    }

    /// A notification right away (e.g. "Pako needs you" for a link check): a pop-up and/or a
    /// line in the bell, as Settings › Notifications says for its kind (see notices.rs).
    pub(crate) fn notify(&self, kind: &str, title: &str, body: &str, go: Go) {
        self.notify_pop(kind, title, body, go, Pop::Pref);
    }

    /// The same, saying whether the pop-up shows (e.g. not while Pako's window is in front and
    /// already shows the news; the bell still gets it).
    pub(crate) fn notify_pop(&self, kind: &str, title: &str, body: &str, go: Go, pop: Pop) {
        (self.emit)("notify", notices::payload(kind, title, body, &go, pop));
    }

    /// Dashboard history (speeds, data per day).
    pub fn reset_stats(&self) {
        self.stats.lock().unwrap().reset();
    }

    pub fn stats(&self) -> crate::stats::StatsView {
        self.stats.lock().unwrap().view()
    }

    fn tick(&self) {
        self.flush_notes();
        {
            let mut last = self.last_sample.lock().unwrap();
            if last.elapsed() >= Duration::from_secs(5) {
                *last = Instant::now();
                self.stats.lock().unwrap().sample();
            }
        }
        // Uploads and Direct transfers, read before the lock (Direct's check takes its own).
        let uploads = transfers_running();
        let mut inner = self.inner.lock().unwrap();
        let has_jobs = !inner.jobs.is_empty();
        // Awake while downloads, uploads or Direct transfers run, while downloads wait for the
        // download hours (a PC asleep at the start time would never start them), and during
        // the countdown before sleeping / shutting down.
        let waits_for_hours = inner.settings.schedule_on
            && !in_hours(&inner.settings)
            && inner.downloads.iter().any(|d| d.status == Status::Queued);
        let countdown = inner.when_done_at.is_some();
        crate::system::keep_awake(inner.settings.keep_awake && (has_jobs || uploads || waits_for_hours || countdown));
        Self::sync_jobs(&mut inner);
        if has_jobs {
            let now = Instant::now();
            let mut progress = Vec::new();
            let ids: Vec<String> = inner.jobs.keys().cloned().collect();
            for id in ids {
                let Some(d) = inner.downloads.iter().find(|d| d.id == id).cloned() else { continue };
                let conns = inner.jobs.get(&id).map(|j| j.connections.load(Ordering::Relaxed)).unwrap_or(0);
                let sp = inner.speeds.entry(id.clone()).or_insert(Speed {
                    bytes: d.downloaded,
                    at: now,
                    value: 0.0,
                    first: d.downloaded,
                    busy: 0.0,
                    peak: 0.0,
                });
                let dt = now.duration_since(sp.at).as_secs_f64();
                if dt >= 0.4 {
                    let instant = d.downloaded.saturating_sub(sp.bytes) as f64 / dt;
                    sp.value = if sp.value == 0.0 { instant } else { sp.value * 0.7 + instant * 0.3 };
                    if d.status == Status::Downloading {
                        sp.busy += dt;
                        sp.peak = sp.peak.max(sp.value);
                    }
                    sp.bytes = d.downloaded;
                    sp.at = now;
                }
                let speed = if d.status == Status::Downloading { sp.value as u64 } else { 0 };
                progress.push(Progress {
                    id: d.id.clone(),
                    status: d.status.clone(),
                    message: d.message.clone(),
                    downloaded: d.downloaded,
                    size: d.size,
                    speed,
                    connections: conns,
                    name: d.name.clone(),
                });
            }
            (self.emit)("progress", serde_json::to_value(&progress).unwrap_or_default());
            inner.dirty = true;
        } else if inner.had_jobs {
            // Everything stopped: one empty update so the window shows 0 speed.
            (self.emit)("progress", serde_json::json!([]));
        }
        inner.had_jobs = has_jobs;
        if inner.dirty && inner.last_save.elapsed() >= Duration::from_secs(2) {
            // Note the parts FIRST, then flush: everything in that note was written before the
            // flush, so it's on disk once the flush returns. Bytes written during the flush
            // are saved next time. (A power cut can't leave "done" parts full of zeros.)
            let notes: Vec<(Arc<Job>, Vec<Segment>, Option<Arc<File>>)> = inner
                .jobs
                .values()
                .map(|j| {
                    let segs: Vec<Segment> = j.slots.lock().unwrap().iter().map(|s| s.seg.clone()).collect();
                    (j.clone(), segs, j.file.lock().unwrap().clone())
                })
                .collect();
            drop(inner);
            off_async(|| {
                for (job, segs, file) in notes {
                    let flushed = match &file {
                        Some(f) => f.sync_data().is_ok(),
                        None => false,
                    };
                    if flushed && !segs.is_empty() {
                        *job.flushed.lock().unwrap() = Some(segs);
                    }
                }
            });
            let mut inner = self.inner.lock().unwrap();
            Self::sync_jobs(&mut inner);
            Self::save(&mut inner);
        }
    }

    /// Saves the list as it is now. The bytes are made here (under the lock, so they're a true
    /// picture); the disk work happens on the saver's own thread, without the lock. Returns
    /// the save's number, to wait for it.
    fn save(inner: &mut Inner) -> Option<u64> {
        // Never save the list over a file that couldn't be read at start-up (it may only have
        // been locked for a moment by antivirus or a backup tool): the real list stays safe.
        if !inner.loaded_ok {
            return None;
        }
        let mut list = inner.downloads.clone();
        for d in list.iter_mut() {
            if let Some(job) = inner.jobs.get(&d.id) {
                if let Some(segs) = job.flushed.lock().unwrap().clone() {
                    d.downloaded = segs.iter().map(|s| s.done).sum();
                    d.segments = segs;
                }
            }
        }
        let saved = serde_json::to_vec(&list).ok().map(|bytes| LIST_SAVER.save(bytes));
        inner.dirty = false;
        inner.last_save = Instant::now();
        saved
    }

    /// Saves the list and waits until it is on disk.
    pub fn save_now(&self) {
        let saved = {
            let mut inner = self.inner.lock().unwrap();
            Self::sync_jobs(&mut inner);
            Self::save(&mut inner)
        };
        if let Some(n) = saved {
            off_async(|| LIST_SAVER.wait(n, SAVE_WAIT));
        }
    }

    /// Called when the app closes: stop connections and save exactly where each download is.
    pub fn shutdown(&self) {
        self.stats.lock().unwrap().save();
        let mut inner = self.inner.lock().unwrap();
        Self::sync_jobs(&mut inner);
        for job in inner.jobs.values() {
            job.cancel.cancel();
        }
        let saved = Self::save(&mut inner);
        drop(inner);
        // Every save asked so far reaches the disk before Pako exits.
        if let Some(n) = saved {
            LIST_SAVER.wait(n, SAVE_WAIT);
        }
        found::wait_saved(SAVE_WAIT);
        // The async stops above never get to run once Pako exits: end yt-dlp (and the ffmpeg
        // it started) now, or a second one would write to the same pieces at the next start.
        crate::video::kill_all();
        // Their cookie files never get dropped either: delete them now.
        crate::video::remove_cookie_files();
    }

    async fn job_ended(self: &Arc<Self>, job: &Arc<Job>, outcome: Result<(), Stop>) {
        // Its final progress is saved below as is: put those bytes on disk first.
        let file = job.file.lock().unwrap().clone();
        if let Some(f) = file {
            let _ = tauri::async_runtime::spawn_blocking(move || f.sync_data()).await;
        }
        // A notification for it: its own "Notify when done" (forces the pop-up), or the
        // kind's switches in Settings › Notifications. Some(force) = wanted.
        let mut stopped_note: Option<(String, bool)> = None;
        let mut finished_note: Option<bool> = None;
        let mut extract_group: Option<(String, PathBuf)> = None;
        {
            let mut inner = self.inner.lock().unwrap();
            Self::sync_jobs(&mut inner);
            // Only forget the map entry if it is still this run (defensive: never drop a newer run).
            let current = inner.jobs.get(&job.id).map(|j| Arc::ptr_eq(j, job)).unwrap_or(false);
            if !current {
                return;
            }
            // Keep only progress known to be on disk (the file was flushed before it closed,
            // see `run`): a power cut right after a pause can't leave parts marked done that
            // are still zeros. (A finished file's parts are cleared below anyway.)
            if let Some(segs) = job.flushed.lock().unwrap().clone().filter(|s| !s.is_empty()) {
                if let Some(d) = inner.downloads.iter_mut().find(|d| d.id == job.id) {
                    d.downloaded = segs.iter().map(|s| s.done).sum();
                    d.segments = segs;
                }
            }
            inner.jobs.remove(&job.id);
            let speed = inner.speeds.remove(&job.id);
            // Put back in line by Pako itself? (Only this run: any next one starts fresh.)
            let retried_from = inner.auto_retry.remove(&job.id);
            let mut done_ok = false;
            let done_pref = notices::pref(&inner.settings, "download-done");
            let stopped_pref = notices::pref(&inner.settings, "download-failed");
            let pos = inner.downloads.iter().position(|d| d.id == job.id);
            match pos {
                None => {
                    // Removed while running: delete its unfinished file now that it's closed.
                    drop(inner);
                    self.delete_orphan_part(job);
                    self.list_changed();
                    self.schedule();
                    return;
                }
                Some(i) => {
                    let others = other_names(&inner.downloads, &inner.downloads[i].dest_dir, &inner.downloads[i].name);
                    let d = &mut inner.downloads[i];
                    match outcome {
                        Ok(()) => {
                            self.stats.lock().unwrap().finished(true);
                            done_ok = true;
                            d.status = Status::Completed;
                            d.message = None;
                            d.finished_at = Some(now_secs());
                            numbers_event(d, speed.as_ref(), true);
                            if (d.notify || done_pref.popup || done_pref.bell) && !d.quiet {
                                finished_note = Some(d.notify);
                            }
                            if d.extract {
                                if let Some(g) = &d.group {
                                    extract_group = Some((g.clone(), d.dest_dir.clone()));
                                }
                            }
                            d.segments.clear();
                        }
                        Err(Stop::Cancelled) => {
                            // Paused by the user (status already Paused) or app closing,
                            // or stopped to start again (back to waiting).
                            if job.requeue.load(Ordering::Relaxed) && d.status != Status::Paused {
                                d.status = Status::Queued;
                                d.message = None;
                            }
                        }
                        Err(Stop::Fail(status, msg)) => {
                            // Counted once: Pako trying again by itself every few hours
                            // (Google's daily limit) isn't another failed download.
                            if stop_is_news(retried_from.as_ref(), &status) {
                                self.stats.lock().unwrap().finished(false);
                                numbers_event(d, speed.as_ref(), false);
                            }
                            // Say it stopped, and why (asked for by this one, or by Settings),
                            // unless it is old news (tried again by itself, stopped the same way).
                            let wanted = d.notify || ((stopped_pref.popup || stopped_pref.bell) && !d.quiet);
                            if wanted && stop_is_news(retried_from.as_ref(), &status) {
                                stopped_note = Some((msg.clone(), d.notify));
                            }
                            if stop_is_news(retried_from.as_ref(), &status) { crate::errorlog::note(crate::errorlog::Area::download(matches!(d.source, Source::Video { .. })), &msg); }
                            d.status = status;
                            d.message = Some(msg);
                        }
                    }
                    // "Start over" asked while running: now that the file is closed, wipe it,
                    // whatever way this run ended.
                    if job.reset.load(Ordering::Relaxed) {
                        if d.status == Status::Completed {
                            let _ = trash::delete(d.final_path());
                        }
                        d.remove_part();
                        // A video: yt-dlp's pieces too (it has stopped), before it can start again.
                        if matches!(d.source, Source::Video { .. }) {
                            for p in crate::video::leftovers(&d.dest_dir, &d.name, &d.id, &others) {
                                let _ = std::fs::remove_file(p);
                            }
                        }
                        done_ok = false;
                        d.segments.clear();
                        if let Source::Video { streams, .. } = &mut d.source {
                            streams.clear();
                        }
                        d.downloaded = 0;
                        d.status = Status::Queued;
                        d.message = None;
                        d.finished_at = None;
                        recheck_server(d);
                    }
                }
            }
            if done_ok {
                inner.done_any = true;
            }
            inner.dirty = true;
            Self::save(&mut inner);
        }
        self.list_changed();
        self.schedule();
        if let Some(force) = finished_note {
            self.note_finished(&job.id, force);
        }
        if let Some((msg, force)) = stopped_note {
            self.note_stopped(&job.id, &msg, force);
        }
        if let Some((group, dir)) = extract_group {
            self.extract_when_complete(&group, &dir);
        }
    }

    /// The download was removed from the list while running: delete its unfinished file.
    fn delete_orphan_part(&self, job: &Job) {
        if let Some(p) = PENDING_DELETE.lock().unwrap().remove(&job.id) {
            let _ = std::fs::remove_file(p);
        }
    }

    // ---------- running one download ----------

    async fn run(self: &Arc<Self>, job: &Arc<Job>) -> Result<(), Stop> {
        let Some(mut d) = self.find(&job.id) else { return Err(Stop::Cancelled) };
        if matches!(d.source, Source::Video { .. }) {
            return self.run_video(job, d).await;
        }
        // Left unfinished by an older Pako ("name.zdpart"): continued under the new name.
        d.adopt_old_part();
        PENDING_DELETE.lock().unwrap().insert(job.id.clone(), d.part_path());
        let mut fetcher = Fetcher::new(self.http.clone(), &d.source);
        fetcher.set_browser_headers(crate::extension::headers_for(&d.id));

        // 1. Find out the file's size, name and whether it can be split/resumed.
        if !d.resolved {
            d = self.resolve(job, &fetcher).await?;
            PENDING_DELETE.lock().unwrap().insert(job.id.clone(), d.part_path());
        }
        // Resumed parts must come from the same version of the file.
        fetcher.set_validator(d.validator.clone());
        let fetcher = Arc::new(fetcher);

        let (connections, verify, auto) = {
            let s = &self.inner.lock().unwrap().settings;
            (s.connections, s.verify_checksum, s.auto_connections)
        };

        // 2. Enough room on the drive for what's left (and for the other downloads going there)?
        //    Checked before creating anything, so a refusal leaves no empty file behind.
        if let Some(size) = d.size {
            let have: u64 = if d.resumable { d.segments.iter().map(|s| s.done).sum() } else { 0 };
            if let Some(msg) = self.space_problem(&d, size.saturating_sub(have)) {
                return Err(Stop::Fail(Status::Failed, msg));
            }
        }

        // 3. Open the unfinished file and lay out the parts.
        let _ = std::fs::create_dir_all(&d.dest_dir);
        let part = d.part_path();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&part)
            .map_err(|e| Stop::Fail(Status::Failed, format!("Can't create the file: {e}")))?;
        let resumable = d.resumable && d.size.is_some();
        let mut segments = d.segments.clone();
        let part_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        let bad_layout = segments.is_empty()
            || (resumable && Some(part_len) != d.size)
            || (!resumable && segments.iter().any(|s| s.done > 0));
        if bad_layout {
            // Paused meanwhile: stop before touching the file.
            if job.cancel.is_cancelled() {
                return Err(Stop::Cancelled);
            }
            // Parts saved from an earlier run don't fit the file any more (it was deleted, or
            // is another size): forget them on disk BEFORE the file is laid out again. Else a
            // crash in the next seconds would leave "half done" parts saved next to a new file
            // of the right size full of zeros, and the next start would continue from them.
            if !d.segments.is_empty() {
                {
                    let mut inner = self.inner.lock().unwrap();
                    if let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == job.id) {
                        dd.segments.clear();
                        dd.downloaded = 0;
                    }
                    inner.dirty = true;
                }
                self.save_now();
            }
            segments = initial_segments(d.size, resumable, if auto { AUTO_START } else { connections });
            // Sparse: parts start far into the file, and Windows would otherwise write zeros
            // over everything before each part first (minutes on a hard disk for a big file).
            make_sparse(&file);
            file.set_len(if resumable { d.size.unwrap_or(0) } else { 0 })
                .map_err(|e| Stop::Fail(Status::Failed, disk_message(&e)))?;
            job.downloaded.store(0, Ordering::Relaxed);
        } else {
            job.downloaded.store(segments.iter().map(|s| s.done).sum(), Ordering::Relaxed);
        }
        *job.resumable.lock().unwrap() = resumable;
        // What it starts from is on disk already (saved after a flush earlier, or fresh).
        *job.flushed.lock().unwrap() = Some(segments.clone());
        *job.slots.lock().unwrap() = segments.into_iter().map(|seg| Slot { seg, busy: false }).collect();
        *job.phase.lock().unwrap() = Phase::Downloading;

        // 3. Download with several connections.
        let file = Arc::new(file);
        self.run_workers(job, fetcher.clone(), file.clone(), d.size, resumable, None).await?;
        let complete = job.slots.lock().unwrap().iter().all(|s| s.seg.remaining() == 0);
        if !complete {
            return Err(Stop::Fail(Status::Failed, "Download stopped before the end. Click Retry.".into()));
        }
        // Unknown size: the final size is what we received. Cut any leftover bytes
        // (e.g. a restarted download that came out shorter).
        let total: u64 = job.slots.lock().unwrap().iter().map(|s| s.seg.done).sum();
        file.set_len(total).map_err(|e| Stop::Fail(Status::Failed, disk_message(&e)))?;
        file.sync_all().map_err(|e| Stop::Fail(Status::Failed, disk_message(&e)))?;
        // Close the handle before renaming (Windows can't rename an open file).
        drop(file);

        // 4. Check the file against Google's checksum.
        if verify {
            if let Some(expected) = d.md5.clone() {
                *job.phase.lock().unwrap() = Phase::Verifying;
                let path = part.clone();
                let cancel = job.cancel.clone();
                let actual = tokio::task::spawn_blocking(move || md5_file(&path, &cancel))
                    .await
                    .map_err(|_| Stop::Fail(Status::Failed, "Checksum check crashed".into()))?;
                match actual {
                    None => return Err(Stop::Cancelled),
                    Some(Err(e)) => return Err(Stop::Fail(Status::Failed, disk_message(&e))),
                    Some(Ok(sum)) if !sum.eq_ignore_ascii_case(&expected) => {
                        // Start over on Retry.
                        let mut inner = self.inner.lock().unwrap();
                        if let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == job.id) {
                            dd.segments.clear();
                            dd.downloaded = 0;
                        }
                        job.slots.lock().unwrap().clear();
                        *job.flushed.lock().unwrap() = None;
                        job.downloaded.store(0, Ordering::Relaxed);
                        return Err(Stop::Fail(
                            Status::Failed,
                            "The file came out damaged (checksum doesn't match). Click Retry to download it again."
                                .into(),
                        ));
                    }
                    Some(Ok(_)) => {}
                }
            }
        }

        // 5. Give it its real name. The new name is only recorded once the rename worked, so a
        // failed rename never leaves the list pointing at a part file that isn't there.
        let final_name = {
            let inner = self.inner.lock().unwrap();
            let taken = taken_paths(&inner.downloads, Some(&job.id));
            names::unique_final(&d.dest_dir, &d.name, &taken)
        };
        let target = d.dest_dir.join(&final_name);
        let mut tries = 0;
        loop {
            match std::fs::rename(&part, &target) {
                Ok(()) => break,
                // Antivirus or the indexer often holds a just-closed file for a moment.
                Err(_) if tries < 8 => {
                    tries += 1;
                    tokio::select! {
                        _ = job.cancel.cancelled() => return Err(Stop::Cancelled),
                        _ = tokio::time::sleep(Duration::from_millis(500)) => {}
                    }
                }
                Err(e) => {
                    return Err(Stop::Fail(Status::Failed, format!("Downloaded, but couldn't rename the file: {e}. Click Try again.")));
                }
            }
        }
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == job.id) {
                dd.name = final_name;
                dd.size = Some(total);
                dd.downloaded = total;
            }
        }
        mark_from_internet(&target, &d.link);
        PENDING_DELETE.lock().unwrap().remove(&job.id);
        Ok(())
    }

    /// Downloads a laid-out file with several connections (Auto adjusts how many while it
    /// runs), until every part is done, it is stopped, or a connection fails for good. The
    /// file is put on disk before this returns, and the job lets go of it. `chunk` = most bytes
    /// asked for per request (some video sites slow down bigger requests).
    async fn run_workers(
        self: &Arc<Self>,
        job: &Arc<Job>,
        fetcher: Arc<Fetcher>,
        file: Arc<File>,
        size: Option<u64>,
        resumable: bool,
        chunk: Option<u64>,
    ) -> Result<(), Stop> {
        let (connections, auto) = {
            let s = &self.inner.lock().unwrap().settings;
            (s.connections, s.auto_connections)
        };
        *job.file.lock().unwrap() = Some(file.clone());
        let abort = job.cancel.child_token();
        let failure: Arc<Mutex<Option<Stop>>> = Arc::new(Mutex::new(None));
        let host = fetcher.host();
        *job.host.lock().unwrap() = host.clone();
        // Auto's ceiling: 16 per file (8 for Drive files), lower if this server
        // recently said "too many requests".
        let base_max = if !resumable { 1 } else if auto { fetcher.auto_ceiling() } else { connections.max(1) };
        // The per-server limit and budget apply whether Auto is on or not.
        let max = base_max.min(self.host_limit(&host));
        let first = if !auto { max } else if chunk.is_some() { STREAM_START } else { AUTO_START };
        let start = first.min(max).min(self.host_budget(&host, &job.id)).max(1);
        job.target.store(start, Ordering::Relaxed);
        let mut set = tokio::task::JoinSet::new();
        let spawn = |set: &mut tokio::task::JoinSet<()>, delay: Duration| {
            let w = Worker {
                job: job.clone(),
                fetcher: fetcher.clone(),
                file: file.clone(),
                limiter: self.limiter.clone(),
                net: self.net.clone(),
                abort: abort.clone(),
                delay,
                size,
                chunk,
            };
            let failure = failure.clone();
            w.job.live.fetch_add(1, Ordering::Relaxed);
            set.spawn(async move {
                let r = w.run().await;
                // A worker that retired itself (Auto lowered the count) already left `live`.
                if !matches!(r, Ok(true)) {
                    w.job.live.fetch_sub(1, Ordering::Relaxed);
                }
                if let Err(Stop::Fail(s, m)) = r {
                    let mut f = failure.lock().unwrap();
                    if f.is_none() {
                        *f = Some(Stop::Fail(s, m));
                    }
                    w.abort.cancel();
                }
            });
        };
        for k in 0..start {
            spawn(&mut set, SPAWN_GAP * k);
        }
        // Auto: adjust the number of connections while the file downloads.
        let mut tuner = AutoTuner::new(max, auto);
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.tick().await;
        loop {
            tokio::select! {
                r = set.join_next() => {
                    if r.is_none() {
                        // Every connection left at the same moment (e.g. one retired because Auto
                        // lowered the count while another found no part free): if the file isn't
                        // finished and nothing failed or was stopped, start one again.
                        let unfinished = job.slots.lock().unwrap().iter().any(|s| s.seg.remaining() > 0);
                        let failed = failure.lock().unwrap().is_some();
                        if unfinished && !failed && !abort.is_cancelled() && !job.cancel.is_cancelled() {
                            spawn(&mut set, Duration::ZERO);
                            continue;
                        }
                        break;
                    }
                }
                // Runs in fixed mode too: "too many requests" and the per-server budget always apply.
                _ = tick.tick(), if resumable => {
                    tuner.max = base_max.min(self.host_limit(&host));
                    let mut target = tuner.step(job);
                    if tuner.just_throttled {
                        // Remember this server's limit for a while, for every file from it.
                        self.host_limits.lock().unwrap().insert(host.clone(), (target, Instant::now() + HOST_MEMORY));
                    }
                    // Share the per-server budget with other files from the same server.
                    target = target.min(tuner.max).min(self.host_budget(&host, &job.id)).max(1);
                    job.target.store(target, Ordering::Relaxed);
                    // Start extra connections only while there is work they can take.
                    let mut k = 0;
                    while job.live.load(Ordering::Relaxed) < target && !abort.is_cancelled() && job.has_work_for_more() {
                        spawn(&mut set, SPAWN_GAP * k);
                        k += 1;
                        if k >= 4 {
                            break; // at most 4 new ones per second
                        }
                    }
                }
            }
        }
        // All connections are done. Put what they wrote on disk before letting go of the file:
        // `job_ended` saves this progress (paused, stepped aside, end of the download hours,
        // failed), and only progress known to be on disk may be saved. If the flush fails, the
        // progress of the last flush is the one kept.
        let segs: Vec<Segment> = job.slots.lock().unwrap().iter().map(|s| s.seg.clone()).collect();
        let f = file.clone();
        let synced = tauri::async_runtime::spawn_blocking(move || f.sync_data().is_ok()).await.unwrap_or(false);
        if synced && !segs.is_empty() {
            *job.flushed.lock().unwrap() = Some(segs);
        }
        // Let go of the shared handle so the file closes as soon as this function returns
        // (needed to rename or delete it on Windows).
        *job.file.lock().unwrap() = None;

        if let Some(f) = failure.lock().unwrap().take() {
            return Err(f);
        }
        if job.cancel.is_cancelled() {
            return Err(Stop::Cancelled);
        }
        Ok(())
    }

    /// Connection limit remembered for a server that said "too many requests" (else no limit).
    fn host_limit(&self, host: &str) -> u32 {
        let mut limits = self.host_limits.lock().unwrap();
        match limits.get(host) {
            Some((n, until)) if *until > Instant::now() => *n,
            Some(_) => {
                limits.remove(host);
                u32::MAX
            }
            None => u32::MAX,
        }
    }

    /// How many connections this file may have, given what other files use on the same server.
    fn host_budget(&self, host: &str, job_id: &str) -> u32 {
        let inner = self.inner.lock().unwrap();
        let others: u32 = inner
            .jobs
            .values()
            .filter(|j| j.id != job_id && *j.host.lock().unwrap() == host)
            .map(|j| j.live.load(Ordering::Relaxed))
            .sum();
        HOST_BUDGET.saturating_sub(others).max(1)
    }

    /// First request: size, name, resumable. Retries forever on connection problems.
    async fn resolve(&self, job: &Arc<Job>, fetcher: &Fetcher) -> Result<Download, Stop> {
        let mut attempt = 0;
        let info = loop {
            let r = tokio::select! {
                _ = job.cancel.cancelled() => return Err(Stop::Cancelled),
                r = fetcher.get(0, None) => r,
            };
            match r {
                Ok(resp) => {
                    let info = probe_info(&resp);
                    let page = crate::fetch::is_html(&resp);
                    drop(resp); // closes the connection; parts are fetched separately
                    break (info, page);
                }
                Err(FetchError::Retry(msg)) => {
                    match job.wait_before_retry(&self.net, attempt, &msg, 0).await {
                        None => return Err(Stop::Cancelled),
                        Some(true) => attempt = 0,
                        Some(false) => attempt += 1,
                    }
                }
                // "Slow down": wait at least as long as the server asked (Retry-After).
                Err(FetchError::Throttled(msg, wait)) => {
                    match job.wait_before_retry(&self.net, attempt, &msg, wait).await {
                        None => return Err(Stop::Cancelled),
                        Some(true) => attempt = 0,
                        Some(false) => attempt += 1,
                    }
                }
                Err(e) => return Err(e.into()),
            }
        };
        let (info, page) = info;
        *job.retry_msg.lock().unwrap() = None;
        // A link to a file (".zip", ".mp4"...) that answers with a web page and no file name:
        // an expired link or a sign-in page. Saving that page under the file's name would end
        // as "finished" with a broken file.
        let expected = self.find(&job.id).map(|d| d.name).unwrap_or_default();
        if web_page_instead(&expected, page, info.name.is_some()) {
            return Err(Stop::Fail(Status::Failed, WEB_PAGE.into()));
        }

        let mut inner = self.inner.lock().unwrap();
        let taken = taken_paths(&inner.downloads, Some(&job.id));
        let Some(dd) = inner.downloads.iter_mut().find(|x| x.id == job.id) else {
            return Err(Stop::Cancelled);
        };
        // Checked again after "Start over" (the file may have changed on the server): keep the
        // name the download already has then, in case the user renamed it.
        let checked_before = dd.size.is_some() || dd.validator.is_some();
        if let Some(name) = info.name.filter(|_| !checked_before) {
            dd.name = names::unique(&dd.dest_dir, &name, &taken);
        }
        dd.size = info.size;
        dd.resumable = info.resumable;
        dd.validator = info.validator;
        dd.resolved = true;
        dd.segments.clear();
        inner.dirty = true;
        let out = inner.downloads.iter().find(|x| x.id == job.id).cloned().unwrap();
        drop(inner);
        (self.emit)("list-changed", serde_json::Value::Null);
        Ok(out)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartView {
    pub start: u64,
    pub end: u64,
    pub done: u64,
    pub active: bool,
}

/// Part files of downloads that are running, so they can be deleted if removed mid-way.
static PENDING_DELETE: std::sync::LazyLock<Mutex<HashMap<String, PathBuf>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

impl Job {
    fn new(id: String, cancel: CancellationToken, downloaded: u64, resumable: bool) -> Self {
        Job {
            id,
            cancel,
            slots: Mutex::new(Vec::new()),
            downloaded: AtomicU64::new(downloaded),
            connections: AtomicU32::new(0),
            last_data: Mutex::new(None),
            retry_msg: Mutex::new(None),
            phase: Mutex::new(Phase::Starting),
            resumable: Mutex::new(resumable),
            target: AtomicU32::new(1),
            live: AtomicU32::new(0),
            throttled_at: Mutex::new(None),
            disturbed: std::sync::atomic::AtomicBool::new(false),
            host: Mutex::new(String::new()),
            reset: std::sync::atomic::AtomicBool::new(false),
            file: Mutex::new(None),
            requeue: std::sync::atomic::AtomicBool::new(false),
            started: Instant::now(),
            flushed: Mutex::new(None),
        }
    }

    fn live_status(&self) -> (Status, Option<String>) {
        let phase = *self.phase.lock().unwrap();
        if phase == Phase::Verifying {
            return (Status::Verifying, Some("Checking the file".into()));
        }
        if phase == Phase::Joining {
            return (Status::Verifying, Some("Joining picture and sound".into()));
        }
        if phase == Phase::Converting {
            return (Status::Verifying, Some("Converting".into()));
        }
        let recent = self.last_data.lock().unwrap().map(|t| t.elapsed() < Duration::from_secs(4)).unwrap_or(false);
        if recent {
            return (Status::Downloading, None);
        }
        if let Some(m) = self.retry_msg.lock().unwrap().clone() {
            return (Status::Retrying, Some(m));
        }
        if phase == Phase::Starting {
            (Status::Starting, None)
        } else {
            (Status::Downloading, None)
        }
    }

    /// After a failed connection: if the internet itself is down, wait (up to a minute) for it
    /// to come back and continue right away; otherwise wait 2s, 4s, 8s, 16s, 32s, then every 60s.
    /// None = cancelled. Some(true) = the internet was down (don't count it as a server error).
    async fn wait_before_retry(&self, net: &Net, attempt: u32, msg: &str, min_wait: u64) -> Option<bool> {
        self.wait_before_retry_on(net, attempt, msg, min_wait, &self.cancel).await
    }

    /// Same, but stops waiting as soon as `stop` fires: a connection worker passes its run's
    /// abort token, so when another connection fails for good (limit reached, needs an
    /// account) the whole download stops at once instead of after its retry wait.
    async fn wait_before_retry_on(&self, net: &Net, attempt: u32, msg: &str, min_wait: u64, stop: &CancellationToken) -> Option<bool> {
        self.disturbed.store(true, Ordering::Relaxed);
        if !net.online().await {
            *self.retry_msg.lock().unwrap() = Some("No internet connection. Waiting for it to come back".into());
            // Wait for it a minute at most, then try the server anyway with the normal
            // delays below: the test addresses may be blocked here while the server works.
            if net.wait_online(stop, OFFLINE_WAIT).await? {
                *self.retry_msg.lock().unwrap() = Some("Internet is back. Reconnecting".into());
                let jitter = rand::random::<u64>() % 800;
                tokio::select! {
                    _ = stop.cancelled() => return None,
                    _ = tokio::time::sleep(Duration::from_millis(jitter)) => {}
                }
                return Some(true);
            }
        }
        let secs = (2u64 << attempt.min(5)).min(60).max(min_wait);
        let jitter = rand::random::<u64>() % 1000;
        *self.retry_msg.lock().unwrap() = Some(format!("{msg}. Trying again in {secs}s"));
        tokio::select! {
            _ = stop.cancelled() => None,
            _ = tokio::time::sleep(Duration::from_millis(secs * 1000 + jitter)) => Some(false),
        }
    }

    /// Auto lowered the connection count: this worker leaves (only as many as needed do).
    fn try_retire(&self) -> bool {
        loop {
            let live = self.live.load(Ordering::Relaxed);
            if live <= self.target.load(Ordering::Relaxed) {
                return false;
            }
            if self.live.compare_exchange(live, live - 1, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                return true;
            }
        }
    }

    /// True if a new connection would find something to do: a free unfinished part,
    /// or a part in progress big enough to split in two.
    fn has_work_for_more(&self) -> bool {
        let slots = self.slots.lock().unwrap();
        slots.iter().any(|s| {
            let rem = s.seg.remaining();
            (!s.busy && rem > 0) || (s.busy && rem >= 4 * MIN_SPLIT)
        })
    }

    fn over_target(&self) -> bool {
        self.live.load(Ordering::Relaxed) > self.target.load(Ordering::Relaxed)
    }

    /// Picks a part to download: a free one, or else half of the biggest part in progress.
    fn claim(&self) -> Option<usize> {
        let mut slots = self.slots.lock().unwrap();
        if let Some(i) = slots.iter().position(|s| !s.busy && s.seg.remaining() > 0) {
            slots[i].busy = true;
            return Some(i);
        }
        if !*self.resumable.lock().unwrap() {
            return None;
        }
        let (i, rem) = slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.busy)
            .map(|(i, s)| (i, s.seg.remaining()))
            .max_by_key(|(_, r)| *r)?;
        if rem < 4 * MIN_SPLIT {
            return None;
        }
        let s = &mut slots[i].seg;
        let mid = s.start + s.done + rem / 2;
        let new = Segment { start: mid, end: s.end, done: 0 };
        s.end = mid;
        slots.push(Slot { seg: new, busy: true });
        Some(slots.len() - 1)
    }

    fn release(&self, i: usize) {
        if let Some(s) = self.slots.lock().unwrap().get_mut(i) {
            s.busy = false;
        }
    }
}

struct Worker {
    job: Arc<Job>,
    fetcher: Arc<Fetcher>,
    file: Arc<File>,
    limiter: Arc<Limiter>,
    net: Arc<Net>,
    abort: CancellationToken,
    /// Wait this long before the first request (connections start staggered).
    delay: Duration,
    /// The file's size as first checked (None = unknown).
    size: Option<u64>,
    /// Most bytes asked for in one request (None = the whole part at once).
    chunk: Option<u64>,
}

enum Flush {
    More,
    PartDone,
}

impl Worker {
    /// Ok(true) = this worker retired itself because Auto lowered the connection count.
    async fn run(&self) -> Result<bool, Stop> {
        if !self.delay.is_zero() {
            tokio::select! {
                _ = self.abort.cancelled() => return Ok(false),
                _ = tokio::time::sleep(self.delay) => {}
            }
        }
        while !self.abort.is_cancelled() {
            if self.job.try_retire() {
                return Ok(true);
            }
            let Some(i) = self.job.claim() else { return Ok(false) };
            self.job.connections.fetch_add(1, Ordering::Relaxed);
            let r = self.part(i).await;
            self.job.connections.fetch_sub(1, Ordering::Relaxed);
            self.job.release(i);
            r?;
        }
        Ok(false)
    }

    /// Downloads one part, reconnecting as often as needed.
    async fn part(&self, i: usize) -> Result<(), Stop> {
        let mut attempt = 0u32;
        let mut blocked_tries = 0u32;
        // Odd answers from Google to "continue from here" (see below), tried again a few times.
        let mut odd_tries = 0u32;
        loop {
            let (pos, end) = {
                let slots = self.job.slots.lock().unwrap();
                let s = &slots[i].seg;
                (s.start + s.done, s.end)
            };
            if pos >= end {
                return Ok(());
            }
            let mut to = if end == u64::MAX { None } else { Some(end - 1) };
            if let Some(c) = self.chunk {
                to = Some(to.map_or(pos + c - 1, |t| t.min(pos + c - 1)));
            }
            let r = tokio::select! {
                _ = self.abort.cancelled() => return Ok(()),
                r = self.fetcher.get(pos, to) => r,
            };
            let resp = match r {
                Ok(r) => r,
                Err(FetchError::Retry(msg)) => {
                    match self.job.wait_before_retry_on(&self.net, attempt, &msg, 0, &self.abort).await {
                        None => return Ok(()),
                        Some(true) => attempt = 0,
                        Some(false) => attempt += 1,
                    }
                    continue;
                }
                Err(FetchError::Throttled(msg, wait)) => {
                    // The server says "too many connections": tell Auto, then wait
                    // (at least as long as the server asked).
                    *self.job.throttled_at.lock().unwrap() = Some(Instant::now());
                    match self.job.wait_before_retry_on(&self.net, attempt, &msg, wait, &self.abort).await {
                        None => return Ok(()),
                        Some(true) => attempt = 0,
                        Some(false) => attempt += 1,
                    }
                    // Auto may have lowered the count meanwhile: give the part back.
                    if self.job.over_target() {
                        return Ok(());
                    }
                    continue;
                }
                Err(FetchError::Blocked(msg)) => {
                    // Mid-download, Google's public link sometimes sends a page for a while:
                    // wait and try a few times before deciding it's only shared with some people.
                    blocked_tries += 1;
                    if blocked_tries > BLOCKED_TRIES {
                        return Err(FetchError::Blocked(msg).into());
                    }
                    let note =
                        format!("Google sent a page instead of the file. Trying again ({blocked_tries} of {BLOCKED_TRIES})");
                    match self.job.wait_before_retry_on(&self.net, attempt, &note, 0, &self.abort).await {
                        None => return Ok(()),
                        Some(true) => attempt = 0,
                        Some(false) => attempt += 1,
                    }
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            blocked_tries = 0;
            // The server answers again: an old "Trying again in 2s" must not come back
            // during a short pause in the data.
            *self.job.retry_msg.lock().unwrap() = None;

            let mut cur = pos;
            let partial = resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;
            // Google Drive: a Drive file id is one file, so a piece that starts at the wrong
            // place, or the whole file sent instead of a piece, is a hiccup on Google's side,
            // not a changed file. Try again (nothing is lost) instead of telling the user to
            // start over. Only a different total size means the file really changed.
            if self.fetcher.is_drive() {
                let wrong_start = partial
                    && crate::fetch::content_range(&resp).is_some_and(|(start, total)| {
                        start != pos && !matches!((total, self.size), (Some(t), Some(s)) if t != s)
                    });
                let whole_instead = pos > 0 && !partial && *self.job.resumable.lock().unwrap();
                if wrong_start || whole_instead {
                    drop(resp);
                    odd_tries += 1;
                    if odd_tries > 6 {
                        return Err(Stop::Fail(
                            Status::Failed,
                            "Google keeps answering oddly for this file. Click Try again later: nothing downloaded so far is lost.".into(),
                        ));
                    }
                    let note = "Google sent an unexpected answer";
                    match self.job.wait_before_retry_on(&self.net, attempt, note, 0, &self.abort).await {
                        None => return Ok(()),
                        Some(true) => attempt = 0,
                        Some(false) => attempt += 1,
                    }
                    continue;
                }
            }
            if partial {
                // The piece must start exactly where we asked and belong to a file of the
                // same size; anything else means the file was replaced on the server, and
                // mixing its bytes with the old ones would give a broken file.
                if let Some((start, total)) = crate::fetch::content_range(&resp) {
                    let size_changed = matches!((total, self.size), (Some(t), Some(s)) if t != s);
                    if start != pos || size_changed {
                        return Err(Stop::Fail(Status::Failed, FILE_CHANGED.into()));
                    }
                }
            }
            if pos == 0 && !partial && *self.job.resumable.lock().unwrap() {
                // The whole file instead of the piece asked for: fine if it is the same file.
                // But when the version was sent along (If-Range), a whole-file answer means
                // it didn't match: the file was replaced.
                let other_size = matches!((resp.content_length(), self.size), (Some(l), Some(s)) if l != s);
                if self.fetcher.sends_version() || other_size {
                    return Err(Stop::Fail(Status::Failed, FILE_CHANGED.into()));
                }
            }
            if pos > 0 && !partial {
                if *self.job.resumable.lock().unwrap() {
                    // A server that allowed resuming before only stops when the file changed
                    // (If-Range didn't match) or its setup changed: either way start over.
                    return Err(Stop::Fail(
                        Status::Failed,
                        "The server stopped allowing resumed downloads: the file may have changed on the server. Use Start over to download it again.".into(),
                    ));
                }
                // Single connection without resume support: start again from zero.
                let mut slots = self.job.slots.lock().unwrap();
                slots[i].seg.done = 0;
                self.job.downloaded.store(0, Ordering::Relaxed);
                cur = 0;
                let _ = self.file.set_len(0);
            }

            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::with_capacity(FLUSH_AT + 64 * 1024);
            let mut progressed = false;
            let outcome: Option<String> = loop {
                let next = tokio::select! {
                    _ = self.abort.cancelled() => {
                        self.flush(i, &mut cur, &mut buf)?;
                        return Ok(());
                    }
                    n = stream.next() => n,
                };
                match next {
                    Some(Ok(chunk)) => {
                        buf.extend_from_slice(&chunk);
                        *self.job.last_data.lock().unwrap() = Some(Instant::now());
                        // The speed limit's wait (long when the limit is low and many
                        // connections share it) ends at once on Pause: what came in is kept.
                        tokio::select! {
                            _ = self.abort.cancelled() => {
                                self.flush(i, &mut cur, &mut buf)?;
                                return Ok(());
                            }
                            _ = self.limiter.acquire(chunk.len()) => {}
                        }
                        if buf.len() >= FLUSH_AT {
                            progressed = true;
                            if let Flush::PartDone = self.flush(i, &mut cur, &mut buf)? {
                                return Ok(());
                            }
                            // Auto lowered the count: hand this part back to the others.
                            if self.job.over_target() {
                                return Ok(());
                            }
                        }
                    }
                    Some(Err(e)) => {
                        self.flush(i, &mut cur, &mut buf)?;
                        break Some(crate::fetch::network_message(&e));
                    }
                    None => {
                        if let Flush::PartDone = self.flush(i, &mut cur, &mut buf)? {
                            return Ok(());
                        }
                        let mut slots = self.job.slots.lock().unwrap();
                        let s = &mut slots[i].seg;
                        if s.end == u64::MAX && self.chunk.is_none() {
                            // Unknown size: the end of the stream is the end of the file.
                            s.end = s.start + s.done;
                            return Ok(());
                        }
                        if s.remaining() == 0 {
                            return Ok(());
                        }
                        // One piece of a part asked for in pieces: ask for the next one now.
                        if to.is_some_and(|t| cur > t) && self.chunk.is_some() {
                            break None;
                        }
                        break Some("The server closed the connection early".into());
                    }
                }
            };
            if progressed || cur > pos {
                attempt = 0;
            }
            if let Some(msg) = outcome {
                match self.job.wait_before_retry_on(&self.net, attempt, &msg, 0, &self.abort).await {
                    None => return Ok(()),
                    Some(true) => attempt = 0,
                    Some(false) => attempt += 1,
                }
                if self.job.over_target() {
                    return Ok(());
                }
            }
        }
    }

    /// Writes buffered bytes at their position, never past the (possibly shrunk) end of the part.
    fn flush(&self, i: usize, cur: &mut u64, buf: &mut Vec<u8>) -> Result<Flush, Stop> {
        if buf.is_empty() {
            let slots = self.job.slots.lock().unwrap();
            return Ok(if *cur >= slots[i].seg.end { Flush::PartDone } else { Flush::More });
        }
        let end = self.job.slots.lock().unwrap()[i].seg.end;
        let writable = if *cur >= end { 0 } else { (end - *cur).min(buf.len() as u64) as usize };
        if writable > 0 {
            // A slow disk (USB drive, busy hard disk) holds up this connection only.
            off_async(|| write_all_at(&self.file, &buf[..writable], *cur)).map_err(|e| Stop::Fail(Status::Failed, disk_message(&e)))?;
        }
        let mut slots = self.job.slots.lock().unwrap();
        let s = &mut slots[i].seg;
        let new_done = (*cur + writable as u64 - s.start).min(s.end - s.start);
        let delta = new_done.saturating_sub(s.done);
        s.done = s.done.max(new_done);
        let finished = *cur + writable as u64 >= s.end;
        drop(slots);
        self.job.downloaded.fetch_add(delta, Ordering::Relaxed);
        crate::stats::PAKO_BYTES.fetch_add(delta, Ordering::Relaxed);
        *cur += writable as u64;
        buf.clear();
        Ok(if finished { Flush::PartDone } else { Flush::More })
    }
}

/// A video stream's piece on disk, to join.
fn piece_of(d: &Download, s: &StreamPart) -> crate::video::Piece {
    crate::video::Piece {
        path: crate::video::stream_part(&d.dest_dir, &d.name, &d.id, &s.format_id, &s.ext),
        ext: s.ext.clone(),
        has_video: s.has_video,
        has_audio: s.has_audio,
    }
}

/// A video's streams download as jobs of their own: their progress shows as the video's.
fn mirror_streams(job: &Job, subs: &[Arc<Job>]) {
    let downloaded: u64 = subs.iter().map(|s| s.downloaded.load(Ordering::Relaxed)).sum();
    job.downloaded.store(downloaded, Ordering::Relaxed);
    let connections: u32 = subs.iter().map(|s| s.connections.load(Ordering::Relaxed)).sum();
    job.connections.store(connections, Ordering::Relaxed);
    let last = subs.iter().filter_map(|s| *s.last_data.lock().unwrap()).max();
    if last.is_some() {
        *job.last_data.lock().unwrap() = last;
        *job.phase.lock().unwrap() = Phase::Downloading;
    }
    let retry = subs.iter().find_map(|s| s.retry_msg.lock().unwrap().clone());
    *job.retry_msg.lock().unwrap() = retry;
}

fn write_all_at(file: &File, mut data: &[u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !data.is_empty() {
        let n = file.seek_write(data, offset)?;
        if n == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "disk write failed"));
        }
        data = &data[n..];
        offset += n as u64;
    }
    Ok(())
}

fn initial_segments(size: Option<u64>, resumable: bool, connections: u32) -> Vec<Segment> {
    match size {
        Some(0) => vec![Segment { start: 0, end: 0, done: 0 }],
        Some(size) if resumable => {
            let n = (size / MIN_SPLIT).clamp(1, connections.max(1) as u64);
            let chunk = size / n;
            (0..n)
                .map(|k| Segment {
                    start: k * chunk,
                    end: if k == n - 1 { size } else { (k + 1) * chunk },
                    done: 0,
                })
                .collect()
        }
        Some(size) => vec![Segment { start: 0, end: size, done: 0 }],
        None => vec![Segment { start: 0, end: u64::MAX, done: 0 }],
    }
}

fn md5_file(path: &PathBuf, cancel: &CancellationToken) -> Option<std::io::Result<String>> {
    use std::io::Read;
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Some(Err(e)),
    };
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 4 * 1024 * 1024];
    loop {
        if cancel.is_cancelled() {
            return None;
        }
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(e) => return Some(Err(e)),
        }
    }
    let digest = hasher.finalize();
    Some(Ok(digest.iter().map(|b| format!("{b:02x}")).collect()))
}

/// Tags a finished file as "downloaded from the internet" (the Zone.Identifier stream every
/// browser writes), so Windows SmartScreen and Office's Protected View still check it before
/// it runs or opens. Silently skipped where the disk can't hold it (FAT32, exFAT).
fn mark_from_internet(path: &std::path::Path, link: &str) {
    let host = if link.starts_with("http://") || link.starts_with("https://") { link } else { "" };
    let text = format!("[ZoneTransfer]\r\nZoneId=3\r\nHostUrl={host}\r\n");
    let mut stream = path.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    let _ = std::fs::write(std::path::PathBuf::from(stream), text);
}

/// Marks the unfinished file "sparse" on NTFS: regions not downloaded yet take no space and
/// aren't filled with zeros first. Without it, a part starting at 15 GB makes Windows write
/// 15 GB of zeros before the first byte lands. Harmless where unsupported (FAT32, exFAT).
fn make_sparse(file: &File) {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::IO::DeviceIoControl;
        use windows_sys::Win32::System::Ioctl::FSCTL_SET_SPARSE;
        let mut returned = 0u32;
        // SAFETY: the handle is valid for the life of `file`; no buffers are passed.
        unsafe {
            DeviceIoControl(
                file.as_raw_handle() as _,
                FSCTL_SET_SPARSE,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            );
        }
    }
    #[cfg(not(windows))]
    let _ = file;
}

fn disk_message(e: &std::io::Error) -> String {
    if e.raw_os_error() == Some(112) {
        "The disk is full. Free some space, then click Retry.".into()
    } else {
        format!("Couldn't write to the disk: {e}")
    }
}

/// Paths other downloads will use (case-insensitive, like Windows).
/// The top folder of a group (a Drive folder, or a split set's folder): the nearest folder
/// above the file named after the group. Files of one group share it, even in subfolders.
pub(crate) fn group_root(d: &Download) -> PathBuf {
    if let Some(g) = &d.group {
        for a in d.dest_dir.ancestors() {
            if a.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(g)) {
                return a.to_path_buf();
            }
        }
    }
    d.dest_dir.clone()
}

/// An unfinished video download renamed from `old` to `new`: its pieces in `dir` (see
/// `video::leftovers`, never another download's) are renamed from "Old.…" to "New.…". A piece
/// whose new name is already taken stays as it is.
fn move_video_pieces(dir: &std::path::Path, old: &str, new: &str, id: &str, others: &[String]) {
    let stem = |n: &str| std::path::Path::new(n).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let (from, to) = (stem(old), stem(new));
    if from.is_empty() || to.is_empty() || from == to {
        return;
    }
    for piece in crate::video::leftovers(dir, old, id, others) {
        let Some(file) = piece.file_name().map(|n| n.to_string_lossy().to_string()) else { continue };
        let Some(rest) = file.strip_prefix(&from) else { continue };
        let target = dir.join(format!("{to}{rest}"));
        if !target.exists() {
            let _ = std::fs::rename(&piece, &target);
        }
    }
}

/// Names of the other downloads in `dir` (whose video pieces are never touched for `name`).
fn other_names(list: &[Download], dir: &std::path::Path, name: &str) -> Vec<String> {
    list.iter().filter(|d| d.dest_dir == dir && d.name != name).map(|d| d.name.clone()).collect()
}

fn taken_paths(list: &[Download], except: Option<&str>) -> HashSet<String> {
    list.iter()
        .filter(|d| Some(d.id.as_str()) != except)
        .map(|d| names::path_key(&d.final_path()))
        .collect()
}

/// "Start over" on a web link: ask the server again for the file's size and version, since
/// the usual reason to start over is that the file changed there (resuming would refuse it).
fn recheck_server(d: &mut Download) {
    match d.source {
        Source::Http { .. } => d.resolved = false,
        // A Drive file replaced by a newer version: its size is asked again (else every run
        // stops with "the file changed"). Google's checksum noted when it was added may be the
        // old version's, so it isn't used for this file any more.
        Source::GDrive { .. } => {
            d.resolved = false;
            d.md5 = None;
        }
        Source::Video { .. } => {}
    }
}

/// The name a download gets from the Download with Pako window's answer. Left where it is and
/// as it is named, it keeps its name: its own unfinished file (a browser download Pako already
/// began) is not "a name already taken", and it carries on from it. Moved or renamed, it gets a
/// free name in the new place.
fn ask_name(old_dir: &std::path::Path, old_name: &str, dir: &std::path::Path, wanted: &str, taken: &HashSet<String>) -> String {
    if dir == old_dir && wanted == old_name {
        return old_name.to_string();
    }
    names::unique(dir, wanted, taken)
}

/// The name of a download sent by the browser: the browser's own name for the file when it
/// gave one, else the link's. (A request without a name arrives as "download", what an empty
/// name is cleaned to: the link's name is better, and is "download" too when it has none.)
fn browser_name(given: &str, url: &str, several: bool) -> String {
    if several || given.is_empty() || given == "download" {
        names::from_url(url)
    } else {
        given.to_string()
    }
}

/// Said when a link to a file answers with a web page instead.
const WEB_PAGE: &str = "The server sent a web page instead of the file: the link may have expired, or it needs you to sign in. Open it in your browser and download it there: Pako catches the download.";

/// The first answer for `name` is a web page (`html`) without a file name of its own
/// (`named`), while the name says it is a file a web page never is (an archive, a video, a
/// program, a PDF...). Pages, and links without such an ending, are downloaded as they are.
fn web_page_instead(name: &str, html: bool, named: bool) -> bool {
    if !html || named {
        return false;
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    matches!(crate::numbers::file_type(name), "video" | "audio" | "image" | "archive" | "program")
        || matches!(ext.as_str(), "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "epub" | "odt")
}

fn new_download(link: &str, source: Source, name: String, dest_dir: PathBuf, group: Option<String>) -> Download {
    Download {
        id: uuid::Uuid::new_v4().to_string(),
        link: link.to_string(),
        source,
        name,
        dest_dir,
        size: None,
        md5: None,
        status: Status::Queued,
        message: None,
        resumable: false,
        segments: Vec::new(),
        downloaded: 0,
        added_at: now_millis(),
        date: now_millis(),
        finished_at: None,
        group,
        resolved: false,
        force: false,
        priority: Priority::Normal,
        notify: false,
        extract: false,
        validator: None,
        ask: false,
        quiet: false,
    }
}

/// A list entry for a Drive file whose details we already have from the API. Google Docs,
/// Sheets, Slides and Drawings are exported (Word, Excel, PowerPoint, PNG) through Google's
/// public export link; other native kinds (forms, sites…) can't be downloaded.
fn drive_download(link: &str, f: &drive::FileMeta, dest_dir: PathBuf, group: Option<String>) -> Option<Download> {
    let mut name = names::sanitize(&f.name);
    let mut doc_type = None;
    if drive::is_google_native(&f.mime_type) {
        let (kind, ext) = drive::export_for(&f.mime_type)?;
        doc_type = Some(kind.to_string());
        if !name.to_ascii_lowercase().ends_with(&format!(".{ext}")) {
            name = format!("{name}.{ext}");
        }
    }
    let exported = doc_type.is_some();
    let src = Source::GDrive { file_id: f.id.clone(), resource_key: f.resource_key.clone(), doc_type };
    let mut d = new_download(link, src, name, dest_dir, group);
    d.size = if exported { None } else { f.size };
    d.md5 = if exported { None } else { f.md5_checksum.clone() };
    d.resumable = !exported;
    d.resolved = true;
    Some(d)
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Finds every http(s) link in any text (chat messages, emails, web page text...),
/// in order, without duplicates. Like JDownloader's link grabber.
pub fn extract_links(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let bytes = text.as_bytes();
    let starts_link = |b: &[u8]| {
        (b.len() >= 8 && b[..8].eq_ignore_ascii_case(b"https://")) || (b.len() >= 7 && b[..7].eq_ignore_ascii_case(b"http://"))
    };
    // One pass over the text: each place is looked at once (a big copy stays quick).
    let mut i = 0;
    while i < bytes.len() {
        if !matches!(bytes[i], b'h' | b'H') || !starts_link(&bytes[i..]) {
            i += 1;
            continue;
        }
        let start = i;
        let end = text[start..]
            .char_indices()
            .find(|(_, c)| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '`' | '[' | ']' | '{' | '}' | '|' | '\\' | '^'))
            .map(|(k, _)| start + k)
            .unwrap_or(text.len());
        let mut link = &text[start..end];
        // Drop punctuation that ends a sentence: "see https://x.com/file.zip)."
        loop {
            let trimmed = link.trim_end_matches(['.', ',', ';', ':', '!', '?', '*']);
            let trimmed = if trimmed.ends_with(')') && !trimmed.contains('(') { &trimmed[..trimmed.len() - 1] } else { trimmed };
            if trimmed.len() == link.len() {
                break;
            }
            link = trimmed;
        }
        if link.len() > "https://".len() && url::Url::parse(link).is_ok() && seen.insert(link.to_string()) {
            out.push(link.to_string());
        }
        i = end.max(start + 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        all_done_now, ask_name, browser_name, busy, clock_jumped, extract_links, first_needs_override, move_video_pieces, new_download, recheck_server,
        stop_is_news, transfers_running, web_page_instead, Inner, WhenDone, UPLOADS_RUNNING,
    };
    use crate::model::{Settings, Source, Status};
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    #[test]
    fn a_stop_is_said_only_when_it_is_news() {
        // The first stop, or a run you asked for (Try again clears Pako's own flag): said.
        assert!(stop_is_news(None, &Status::QuotaExceeded));
        assert!(stop_is_news(None, &Status::Failed));
        // Pako tried again by itself and it stopped the same way: not said again.
        assert!(!stop_is_news(Some(&Status::QuotaExceeded), &Status::QuotaExceeded));
        // Tried again by itself, but it stopped another way: news.
        assert!(stop_is_news(Some(&Status::QuotaExceeded), &Status::Failed));
    }

    #[test]
    fn a_sleeping_pc_is_spotted() {
        let i = Instant::now() + Duration::from_secs(1000);
        let w = std::time::SystemTime::now();
        let s = Duration::from_secs;
        // Half a second on both clocks: nothing happened.
        assert!(!clock_jumped((i, w), (i + Duration::from_millis(500), w + Duration::from_millis(500))));
        // Asleep for an hour: the PC's clock moved and the counted time didn't, or both did.
        assert!(clock_jumped((i, w), (i + Duration::from_millis(500), w + s(3600))));
        assert!(clock_jumped((i, w), (i + s(3600), w + s(3600))));
        // The clock put back.
        assert!(clock_jumped((i, w), (i + s(1), w - s(60))));
    }

    #[test]
    fn uploads_keep_when_done_waiting() {
        let inner = Inner {
            downloads: Vec::new(),
            found: Vec::new(),
            settings: Settings::default(),
            jobs: HashMap::new(),
            speeds: HashMap::new(),
            dirty: false,
            last_save: Instant::now(),
            had_jobs: false,
            loaded_ok: true,
            found_ok: true,
            settings_ok: true,
            start_notes: Vec::new(),
            last_look: None,
            when_done: WhenDone::Nothing,
            when_done_at: None,
            had_work: false,
            done_any: false,
            sched_open: None,
            sched_override: HashSet::new(),
            quota_seen: HashMap::new(),
            auto_retry: HashMap::new(),
        };
        assert!(!busy(&inner, false));
        UPLOADS_RUNNING.fetch_add(1, Ordering::Relaxed);
        let with_upload = busy(&inner, transfers_running());
        UPLOADS_RUNNING.fetch_sub(1, Ordering::Relaxed);
        assert!(with_upload);
        // A Direct transfer (or any registered one) counts too.
        assert!(busy(&inner, true));
    }

    #[test]
    fn a_web_page_is_not_saved_as_the_file() {
        // A ".zip" link answering with a page and no file name: an expired link or a sign-in.
        assert!(web_page_instead("setup.zip", true, false));
        assert!(web_page_instead("Film.MP4", true, false));
        assert!(web_page_instead("report.pdf", true, false));
        // The server names a file (Content-Disposition): it is the file.
        assert!(!web_page_instead("setup.zip", true, true));
        // Not a page, or a link that may well be a page: downloaded as it is.
        assert!(!web_page_instead("setup.zip", false, false));
        assert!(!web_page_instead("index.html", true, false));
        assert!(!web_page_instead("download", true, false));
        assert!(!web_page_instead("notes.txt", true, false));
    }

    /// The Download with Pako window answered without changing the name or the folder: the
    /// download keeps its name even though its own unfinished file is already there (a browser
    /// download Pako began to check it). It used to become "setup (1).exe", start again from
    /// zero and leave "setup.exe.pakopart" behind.
    #[test]
    fn an_unchanged_answer_keeps_the_name_and_its_unfinished_file() {
        let dir = std::env::temp_dir().join(format!("pako-ask-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(crate::names::part_of(&dir, "setup.exe"), b"begun").unwrap();
        let none = HashSet::new();
        assert_eq!(ask_name(&dir, "setup.exe", &dir, "setup.exe", &none), "setup.exe");
        // Renamed to a name whose file (or unfinished file) is there: a free name.
        std::fs::write(dir.join("other.exe"), b"x").unwrap();
        assert_eq!(ask_name(&dir, "setup.exe", &dir, "other.exe", &none), "other (1).exe");
        assert_eq!(ask_name(&dir, "setup.exe", &dir, "new.exe", &none), "new.exe");
        // Moved to another folder under the same name: checked there.
        let other = dir.join("sub");
        std::fs::create_dir_all(&other).unwrap();
        assert_eq!(ask_name(&dir, "setup.exe", &other, "setup.exe", &none), "setup.exe");
        std::fs::write(other.join("setup.exe"), b"x").unwrap();
        assert_eq!(ask_name(&dir, "setup.exe", &other, "setup.exe", &none), "setup (1).exe");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A right-click "Download link with Pako" sends no file name (it arrives as "download"):
    /// the link's own name is used, not a file called "download".
    #[test]
    fn a_browser_link_without_a_name_takes_the_links_name() {
        let url = "https://example.com/files/setup%20v2.zip?x=1";
        assert_eq!(browser_name(&crate::names::sanitize(""), url, false), "setup v2.zip");
        assert_eq!(browser_name("", url, false), "setup v2.zip");
        // The browser's own name for its download wins.
        assert_eq!(browser_name("Report 2026.pdf", url, false), "Report 2026.pdf");
        // Several links at once: each its own name.
        assert_eq!(browser_name("Report 2026.pdf", url, true), "setup v2.zip");
        // A link without a name of its own.
        assert_eq!(browser_name("download", "https://example.com/", false), "download");
    }

    #[test]
    fn start_over_asks_again_for_a_replaced_drive_file() {
        let src = Source::GDrive { file_id: "ID1234567890".into(), resource_key: None, doc_type: None };
        let mut d = new_download("https://drive.google.com/file/d/ID1234567890/view", src, "a.zip".into(), PathBuf::from("C:\\D"), None);
        d.resolved = true;
        d.size = Some(10);
        d.md5 = Some("abc".into());
        recheck_server(&mut d);
        assert!(!d.resolved && d.md5.is_none());
        // The size stays until the server says the new one (the name is kept too).
        assert_eq!(d.size, Some(10));
    }

    #[test]
    fn a_renamed_video_keeps_its_pieces() {
        let dir = std::env::temp_dir().join(format!("pako-rename-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let id = "1a2b3c4d-0000-0000-0000-000000000000";
        let own = crate::video::stream_part(&dir, "Old.mp4", id, "137", "mp4");
        std::fs::write(&own, b"v").unwrap();
        std::fs::write(dir.join("Old.f140.m4a.part"), b"a").unwrap();
        // Another download's file with a similar name stays.
        std::fs::write(dir.join("Old remix.mp4"), b"x").unwrap();
        move_video_pieces(&dir, "Old.mp4", "New.mp4", id, &["Old remix.mp4".to_string()]);
        assert!(crate::video::stream_part(&dir, "New.mp4", id, "137", "mp4").exists());
        assert!(dir.join("New.f140.m4a.part").exists());
        assert!(!own.exists() && dir.join("Old remix.mp4").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn download_first_overrides_only_outside_the_hours() {
        let s = Settings { schedule_on: true, schedule_start: 22 * 60, schedule_end: 7 * 60, ..Default::default() };
        // 23:00, inside the hours: no override (it would keep running past 07:00).
        assert!(!first_needs_override(&s, 23 * 60));
        // 12:00, outside: it runs anyway.
        assert!(first_needs_override(&s, 12 * 60));
        // Schedule off: never.
        let off = Settings { schedule_on: false, ..s };
        assert!(!first_needs_override(&off, 12 * 60));
    }

    #[test]
    fn when_done_needs_a_finished_download() {
        // Downloads run, one finishes, the list goes quiet: countdown.
        let (mut work, mut done) = (false, false);
        assert!(!all_done_now(&mut work, &mut done, true));
        done = true; // a download finished (job_ended)
        assert!(all_done_now(&mut work, &mut done, false));
        // Once only.
        assert!(!all_done_now(&mut work, &mut done, false));

        // One finished, then "Pause all" (pause clears done_any), quiet: no countdown.
        assert!(!all_done_now(&mut work, &mut done, true));
        done = true;
        assert!(!all_done_now(&mut work, &mut done, true));
        done = false;
        assert!(!all_done_now(&mut work, &mut done, false));

        // Only failures: nothing finished, no countdown.
        assert!(!all_done_now(&mut work, &mut done, true));
        assert!(!all_done_now(&mut work, &mut done, false));

        // A tiny download added and finished between two looks (push sets had_work).
        work = true;
        done = true;
        assert!(all_done_now(&mut work, &mut done, false));
    }

    #[test]
    fn many_links_in_one_scan() {
        let text: String = (0..2000).map(|i| format!("https://example.com/f{i}.zip ")).collect();
        let links = extract_links(&text);
        assert_eq!(links.len(), 2000);
        assert_eq!(links[1999], "https://example.com/f1999.zip");
        assert_eq!(
            extract_links("HTTPS://Example.com/A and http://x.org/b https://example.com/f"),
            vec!["HTTPS://Example.com/A", "http://x.org/b", "https://example.com/f"]
        );
    }

    #[test]
    fn finds_links_in_any_text() {
        let text = "Hi! Here are the files: https://drive.google.com/file/d/abcDEF1234567/view?usp=sharing, \
                    and (https://example.com/a%20b.zip). Also http://x.org/f.iso\nhttps://example.com/a%20b.zip again.\
                    <a href=\"https://site.com/page\">link</a>";
        assert_eq!(
            extract_links(text),
            vec![
                "https://drive.google.com/file/d/abcDEF1234567/view?usp=sharing",
                "https://example.com/a%20b.zip",
                "http://x.org/f.iso",
                "https://site.com/page",
            ]
        );
    }
}

fn human_size(n: u64) -> String {
    let gb = n as f64 / 1_073_741_824.0;
    if gb >= 1.0 { format!("{gb:.1} GB") } else { format!("{:.0} MB", n as f64 / 1_048_576.0) }
}

fn short(link: &str) -> String {
    if link.chars().count() > 60 { format!("{}…", link.chars().take(60).collect::<String>()) } else { link.to_string() }
}

/// Real check of fast video downloads (internet and the video tools needed). Run by hand:
/// PAKO_DATA_DIR = a folder whose "components" holds yt-dlp, Deno and ffmpeg (nothing of Pako's
/// own lists is used), PAKO_REAL_VIDEO = a video link, PAKO_REAL_QUALITY = height (720).
/// The downloaded file is checked with ffprobe, then deleted.
#[cfg(test)]
mod real_video {
    use super::*;

    #[test]
    #[ignore]
    fn real_video_download() {
        let Ok(url) = std::env::var("PAKO_REAL_VIDEO") else { return };
        let quality: u32 = std::env::var("PAKO_REAL_QUALITY").ok().and_then(|q| q.parse().ok()).unwrap_or(720);
        let dest = std::env::temp_dir().join(format!("pako-real-video-{}", std::process::id()));
        let engine = Engine::new(Arc::new(|_, _| {}));
        let src = Source::Video {
            url: url.clone(),
            quality: Some(quality),
            audio: None,
            options: Vec::new(),
            audio_size: None,
            duration: None,
            streams: Vec::new(),
        };
        let d = new_download(&url, src, "Real test.mp4".into(), dest.clone(), None);
        let id = d.id.clone();
        engine.push(d);
        engine.schedule();
        let t0 = Instant::now();
        let mut last = (0u64, Instant::now());
        let (mut peak_conns, mut peak_speed, mut streams_seen, mut first_data) = (0u32, 0f64, 0usize, None::<f64>);
        let d = loop {
            std::thread::sleep(Duration::from_secs(1));
            let (d, job) = {
                let inner = engine.inner.lock().unwrap();
                (inner.downloads.iter().find(|x| x.id == id).unwrap().clone(), inner.jobs.get(&id).cloned())
            };
            let (conns, got, phase) = job
                .map(|j| (j.connections.load(Ordering::Relaxed), j.downloaded.load(Ordering::Relaxed), j.live_status().1))
                .unwrap_or((0, d.downloaded, None));
            if let Source::Video { streams, .. } = &d.source {
                streams_seen = streams_seen.max(streams.len());
            }
            let speed = got.saturating_sub(last.0) as f64 / last.1.elapsed().as_secs_f64();
            last = (got, Instant::now());
            if got > 0 && first_data.is_none() {
                first_data = Some(t0.elapsed().as_secs_f64());
            }
            peak_conns = peak_conns.max(conns);
            peak_speed = peak_speed.max(speed);
            println!(
                "{:>4}s {:?} {:?} {} of {:?} bytes, {} connections, {:.1} MB/s",
                t0.elapsed().as_secs(),
                d.status,
                phase.or(d.message.clone()),
                got,
                d.size,
                conns,
                speed / 1e6
            );
            if matches!(d.status, Status::Completed | Status::Failed) || t0.elapsed() > Duration::from_secs(900) {
                break d;
            }
        };
        let secs = t0.elapsed().as_secs_f64();
        let path = d.final_path();
        println!("RESULT {:?} {:?} in {secs:.1}s (first data after {:?}s)", d.status, d.message, first_data);
        println!("RESULT streams downloaded by Pako: {streams_seen}, most connections at once: {peak_conns}, best speed {:.1} MB/s", peak_speed / 1e6);
        println!("RESULT file {} ({:?} bytes), average {:.1} MB/s", path.display(), d.size, d.size.unwrap_or(0) as f64 / secs / 1e6);
        let probe = crate::components::ffmpeg().map(|f| f.with_file_name("ffprobe.exe")).and_then(|ffprobe| {
            std::process::Command::new(ffprobe)
                .args(["-v", "error", "-show_entries", "stream=codec_type,codec_name:format=duration", "-of", "compact"])
                .arg(&path)
                .output()
                .ok()
        });
        let text = probe.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
        println!("RESULT ffprobe:\n{text}");
        let leftovers: Vec<_> = std::fs::read_dir(&dest).map(|r| r.flatten().map(|e| e.file_name()).collect()).unwrap_or_default();
        println!("RESULT files in the folder: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&dest);
        assert_eq!(d.status, Status::Completed);
        assert!(text.contains("codec_type=video") && text.contains("codec_type=audio"));
    }
}
