// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! "Your Pako in numbers" (the Dashboard): one event per finished download, upload and Direct
//! transfer, and running totals per day, ISO week and month, updated as events arrive. The
//! Dashboard reads the totals, so "All time" stays instant after years. Detailed events are
//! kept about 12 months (records, today's hour-by-hour chart, the CSV export); totals forever.
//!
//! Only on this PC, in data/numbers.sqlite. Nothing is sent anywhere.

use std::path::Path;
use std::sync::Mutex;

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

/// The open database (None until the app opens it: tests and the CLI never write here).
static DB: Mutex<Option<Connection>> = Mutex::new(None);
/// Pako's data folder (Reset opens the numbers again if they couldn't be opened at start).
static DIR: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

/// Detailed events are kept this long; totals stay forever.
const KEEP_EVENTS_MS: i64 = 366 * 24 * 3600 * 1000;
/// Bumped when the backfill learns to read something new.
const BACKFILL: &str = "backfill-1";

pub const DOWNLOAD: &str = "download";
pub const UPLOAD: &str = "upload";
pub const SENT: &str = "sent";
pub const RECEIVED: &str = "received";
/// An upload Pako deleted when its "Delete after" date came (not part of the transfer totals).
pub const CLEANED: &str = "cleaned";

/// One finished transfer.
#[derive(Clone, Debug)]
pub struct Event {
    /// Unique per transfer ("dl:<id>:<finished>", "up:<id>", "direct:<id>"): the same transfer
    /// is never counted twice (backfill and live recording use the same keys).
    pub ext: String,
    /// When it ended (ms since 1970).
    pub at_ms: i64,
    pub kind: &'static str,
    pub name: String,
    pub bytes: u64,
    pub files: u32,
    /// Seconds the bytes were moving (None = not known).
    pub secs: Option<f64>,
    /// Bytes per second.
    pub avg: Option<u64>,
    pub peak: Option<u64>,
    /// The bytes that moved during `secs` (a resumed transfer moved less than its size).
    /// None = `bytes`.
    pub moved: Option<u64>,
    /// Downloads: "drive", "video" or "web"; received direct: "direct". Empty = not known.
    pub source: String,
    /// The site (downloads).
    pub site: String,
    /// "video", "audio", "image", "archive", "document", "program" or "other".
    pub ftype: String,
    /// Uploads: "gofile" or "drive"; Direct: "local" (the home network), "direct6", "direct" or "relay".
    pub route: String,
    /// Direct: the contact's Pako ID.
    pub contact: String,
    pub ok: bool,
    /// Read from older files when the numbers started (may lack speeds and details).
    pub backfilled: bool,
}

impl Event {
    pub fn new(ext: impl Into<String>, at_ms: i64, kind: &'static str) -> Self {
        Event {
            ext: ext.into(),
            at_ms,
            kind,
            name: String::new(),
            bytes: 0,
            files: 1,
            secs: None,
            avg: None,
            peak: None,
            moved: None,
            source: String::new(),
            site: String::new(),
            ftype: String::new(),
            route: String::new(),
            contact: String::new(),
            ok: true,
            backfilled: false,
        }
    }

    /// Speeds from what moved and for how long (no speed when it was too short to measure).
    pub fn timed(mut self, bytes_moved: u64, secs: f64, peak: f64) -> Self {
        if secs >= 0.5 && bytes_moved > 0 {
            self.secs = Some(secs);
            self.moved = Some(bytes_moved);
            self.avg = Some((bytes_moved as f64 / secs) as u64);
            if peak > 0.0 {
                self.peak = Some(peak as u64);
            }
        }
        self
    }
}

pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

// ---------- dates (local calendar; the keys are plain dates, so no DST maths) ----------

/// The local calendar day a moment fell on.
pub fn local_day(ms: i64) -> NaiveDate {
    match Local.timestamp_millis_opt(ms) {
        chrono::LocalResult::Single(t) => t.date_naive(),
        chrono::LocalResult::Ambiguous(t, _) => t.date_naive(),
        chrono::LocalResult::None => Utc.timestamp_millis_opt(ms).single().map(|t| t.date_naive()).unwrap_or_default(),
    }
}

fn local_hour(ms: i64) -> u32 {
    Local.timestamp_millis_opt(ms).earliest().map(|t| t.hour()).unwrap_or(0)
}

pub fn day_key(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// ISO week ("2026-W47"; the week of 31 Dec can belong to the next year). Only checked by the
/// tests now: weeks are added up from days.
#[cfg(test)]
pub fn week_key(d: NaiveDate) -> String {
    let w = d.iso_week();
    format!("{}-W{:02}", w.year(), w.week())
}

pub fn month_key(d: NaiveDate) -> String {
    format!("{}-{:02}", d.year(), d.month())
}

pub fn week_start(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

pub fn month_start(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap_or(d)
}

fn parse_day(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// The days a period covers, up to and including today: "today", "week" (since Monday),
/// "month" (since the 1st) or "all" (since the first day anything was counted).
pub fn period_range(period: &str, today: NaiveDate, first: Option<NaiveDate>) -> (NaiveDate, NaiveDate) {
    let from = match period {
        "today" => today,
        "week" => week_start(today),
        "month" => month_start(today),
        _ => first.filter(|f| *f <= today).unwrap_or(today),
    };
    (from, today)
}

// ---------- opening, recording ----------

/// Opens (or creates) data/numbers.sqlite, drops events older than a year, and reads what
/// earlier versions kept (once).
pub fn open(dir: &Path) {
    *DIR.lock().unwrap() = Some(dir.to_path_buf());
    let Some(mut conn) = open_or_start_again(&dir.join("numbers.sqlite")) else { return };
    let _ = conn.execute("DELETE FROM events WHERE at < ?1", params![now_ms() - KEEP_EVENTS_MS]);
    let _ = conn.execute("DELETE FROM totals WHERE period = 'week'", []);
    let done: Option<String> = conn.query_row("SELECT v FROM meta WHERE k = ?1", params![BACKFILL], |r| r.get(0)).optional().ok().flatten();
    if done.is_none() {
        for e in backfill_events(dir, local_day(now_ms())) {
            let _ = insert(&mut conn, &e);
        }
        let _ = conn.execute("INSERT OR REPLACE INTO meta(k, v) VALUES(?1, ?2)", params![BACKFILL, day_key(local_day(now_ms()))]);
    }
    *DB.lock().unwrap() = Some(conn);
}

/// Opens the numbers file. One that can't be read is kept aside (numbers.sqlite.bad-<time>) and
/// the numbers start again, instead of the Dashboard staying empty for good.
fn open_or_start_again(path: &Path) -> Option<Connection> {
    match open_at(path) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("numbers: {e}");
            let aside = path.with_extension(format!("sqlite.bad-{}", now_ms()));
            std::fs::rename(path, &aside).ok()?;
            for side in ["sqlite-wal", "sqlite-shm"] {
                let _ = std::fs::remove_file(path.with_extension(side));
            }
            open_at(path).map_err(|e| eprintln!("numbers: {e}")).ok()
        }
    }
}

fn open_at(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    init(&conn)?;
    Ok(conn)
}

fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA secure_delete = ON;
         CREATE TABLE IF NOT EXISTS events(
           id INTEGER PRIMARY KEY,
           ext TEXT NOT NULL UNIQUE,
           at INTEGER NOT NULL,
           day TEXT NOT NULL,
           kind TEXT NOT NULL,
           name TEXT NOT NULL DEFAULT '',
           bytes INTEGER NOT NULL DEFAULT 0,
           files INTEGER NOT NULL DEFAULT 0,
           secs REAL,
           avg INTEGER,
           peak INTEGER,
           source TEXT NOT NULL DEFAULT '',
           site TEXT NOT NULL DEFAULT '',
           ftype TEXT NOT NULL DEFAULT '',
           route TEXT NOT NULL DEFAULT '',
           contact TEXT NOT NULL DEFAULT '',
           ok INTEGER NOT NULL DEFAULT 1,
           backfilled INTEGER NOT NULL DEFAULT 0);
         CREATE INDEX IF NOT EXISTS events_day ON events(day);
         CREATE TABLE IF NOT EXISTS totals(
           period TEXT NOT NULL,
           key TEXT NOT NULL,
           kind TEXT NOT NULL,
           route TEXT NOT NULL,
           source TEXT NOT NULL,
           contact TEXT NOT NULL,
           count INTEGER NOT NULL DEFAULT 0,
           failed INTEGER NOT NULL DEFAULT 0,
           bytes INTEGER NOT NULL DEFAULT 0,
           files INTEGER NOT NULL DEFAULT 0,
           timed_bytes INTEGER NOT NULL DEFAULT 0,
           secs REAL NOT NULL DEFAULT 0,
           peak INTEGER NOT NULL DEFAULT 0,
           last INTEGER NOT NULL DEFAULT 0,
           PRIMARY KEY(period, key, kind, route, source, contact));
         CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT NOT NULL);",
    )
}

/// Records a finished transfer (does nothing before the app opened the numbers).
pub fn record(e: Event) {
    if let Some(conn) = DB.lock().unwrap().as_mut() {
        if let Err(err) = insert(conn, &e) {
            eprintln!("numbers: {err}");
        }
    }
}

/// Adds one event and its share of the day and month totals (in one transaction). Weeks are
/// added up from the days.
/// False = this transfer was already counted.
fn insert(conn: &mut Connection, e: &Event) -> rusqlite::Result<bool> {
    let day = local_day(e.at_ms);
    let tx = conn.transaction()?;
    let added = tx.execute(
        "INSERT OR IGNORE INTO events(ext, at, day, kind, name, bytes, files, secs, avg, peak, source, site, ftype, route, contact, ok, backfilled)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            e.ext,
            e.at_ms,
            day_key(day),
            e.kind,
            e.name,
            e.bytes as i64,
            e.files as i64,
            e.secs,
            e.avg.map(|v| v as i64),
            e.peak.map(|v| v as i64),
            e.source,
            e.site,
            e.ftype,
            e.route,
            e.contact,
            e.ok,
            e.backfilled
        ],
    )?;
    if added == 0 {
        return Ok(false);
    }
    let (count, failed, bytes, files) = if e.ok { (1i64, 0i64, e.bytes as i64, e.files as i64) } else { (0, e.files.max(1) as i64, 0, 0) };
    let (timed, secs) = match e.secs {
        Some(s) if e.ok && s > 0.0 => (e.moved.unwrap_or(e.bytes) as i64, s),
        _ => (0, 0.0),
    };
    let peak = if e.ok { e.peak.unwrap_or(0) as i64 } else { 0 };
    for (period, key) in [("day", day_key(day)), ("month", month_key(day))] {
        tx.execute(
            "INSERT INTO totals(period, key, kind, route, source, contact, count, failed, bytes, files, timed_bytes, secs, peak, last)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(period, key, kind, route, source, contact) DO UPDATE SET
               count = count + excluded.count, failed = failed + excluded.failed,
               bytes = bytes + excluded.bytes, files = files + excluded.files,
               timed_bytes = timed_bytes + excluded.timed_bytes, secs = secs + excluded.secs,
               peak = MAX(peak, excluded.peak), last = MAX(last, excluded.last)",
            params![period, key, e.kind, e.route, e.source, e.contact, count, failed, bytes, files, timed, secs, peak, e.at_ms],
        )?;
    }
    tx.commit()?;
    Ok(true)
}

// ---------- reading what earlier versions kept ----------

/// The first time: downloads per day (stats.json, before today), sent links (sent.json) and
/// Direct's history. Only what those files really hold: no speeds are made up, and old
/// downloads have no split by site (they were only counted per day).
fn backfill_events(dir: &Path, today: NaiveDate) -> Vec<Event> {
    let mut out = Vec::new();
    let read = |name: &str| -> Option<serde_json::Value> { serde_json::from_slice(&std::fs::read(dir.join(name)).ok()?).ok() };
    let num = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
    let text = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();

    if let Some(stats) = read("stats.json") {
        if let Some(days) = stats.get("days").and_then(|d| d.as_object()) {
            for (key, d) in days {
                let Some(date) = parse_day(key) else { continue };
                if date >= today {
                    continue; // today keeps counting live
                }
                let noon = local_noon(date);
                let (bytes, files, failed) = (num(d, "pako"), num(d, "files"), num(d, "failed"));
                if bytes > 0 || files > 0 {
                    let mut e = Event::new(format!("statsday:{key}"), noon, DOWNLOAD);
                    e.name = "Downloads that day".into();
                    e.bytes = bytes;
                    e.files = files as u32;
                    e.backfilled = true;
                    let busy = num(d, "busySecs");
                    if busy > 0 && bytes > 0 {
                        e.secs = Some(busy as f64);
                        e.avg = Some(bytes / busy);
                    }
                    let max = num(d, "max");
                    if max > 0 {
                        e.peak = Some(max);
                    }
                    out.push(e);
                }
                if failed > 0 {
                    let mut e = Event::new(format!("statsfail:{key}"), noon, DOWNLOAD);
                    e.files = failed as u32;
                    e.ok = false;
                    e.backfilled = true;
                    out.push(e);
                }
            }
        }
    }

    if let Some(serde_json::Value::Array(list)) = read("sent.json") {
        for s in &list {
            if s.get("incomplete").and_then(|x| x.as_bool()).unwrap_or(false) {
                continue;
            }
            let id = text(s, "id");
            if id.is_empty() {
                continue;
            }
            let mut e = Event::new(format!("up:{id}"), num(s, "date") as i64, UPLOAD);
            e.name = text(s, "name");
            e.bytes = num(s, "size");
            e.files = num(s, "files").max(1) as u32;
            e.route = match text(s, "host").as_str() {
                "drive" => "drive".into(),
                _ => "gofile".into(),
            };
            e.backfilled = true;
            out.push(e);
        }
    }

    if let Some(serde_json::Value::Array(list)) = read("direct_history.json") {
        for h in &list {
            if text(h, "result") != "done" {
                continue;
            }
            let outgoing = h.get("outgoing").and_then(|x| x.as_bool()).unwrap_or(false);
            let mut e = direct_event(
                &text(h, "id"),
                num(h, "date") as i64,
                outgoing,
                &text(h, "name"),
                num(h, "size"),
                num(h, "files") as u32,
                h.get("path").and_then(|x| x.as_str()),
                &text(h, "contact"),
                num(h, "durationMs"),
                num(h, "avgSpeed"),
                num(h, "peakSpeed"),
            );
            e.backfilled = true;
            out.push(e);
        }
    }
    out
}

/// Noon on a local date (a safe moment inside that day whatever the clock change).
fn local_noon(d: NaiveDate) -> i64 {
    d.and_hms_opt(12, 0, 0)
        .and_then(|t| Local.from_local_datetime(&t).earliest())
        .map(|t| t.timestamp_millis())
        .unwrap_or(0)
}

/// A finished Direct transfer as an event (shared by the live recording and the backfill).
#[allow(clippy::too_many_arguments)]
pub fn direct_event(
    id: &str,
    at_ms: i64,
    outgoing: bool,
    name: &str,
    size: u64,
    files: u32,
    path: Option<&str>,
    contact: &str,
    duration_ms: u64,
    avg: u64,
    peak: u64,
) -> Event {
    let mut e = Event::new(format!("direct:{id}"), at_ms, if outgoing { SENT } else { RECEIVED });
    e.name = name.to_string();
    e.bytes = size;
    e.files = files.max(1);
    e.route = path.unwrap_or("").to_string();
    e.contact = contact.to_string();
    e.ftype = if files > 1 { "folder".into() } else { file_type(name).into() };
    if !outgoing {
        e.source = "direct".into();
    }
    if duration_ms > 0 {
        e.secs = Some(duration_ms as f64 / 1000.0);
        e.avg = Some(if avg > 0 { avg } else { (size as f64 / (duration_ms as f64 / 1000.0)) as u64 });
    }
    if peak > 0 {
        e.peak = Some(peak);
    }
    e
}

/// The kind of file, from its name.
pub fn file_type(name: &str) -> &'static str {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "m4v" | "wmv" | "flv" | "ts" => "video",
        "mp3" | "m4a" | "flac" | "wav" | "ogg" | "opus" | "aac" => "audio",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "heic" | "psd" => "image",
        "zip" | "rar" | "7z" | "tar" | "gz" | "xz" | "bz2" | "iso" | "001" => "archive",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "csv" | "md" | "epub" | "odt" => "document",
        "exe" | "msi" | "apk" | "dmg" | "appx" | "msix" => "program",
        _ => "other",
    }
}

/// The site of a link, without "www.".
pub fn site_of(link: &str) -> String {
    url::Url::parse(link)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string()))
        .unwrap_or_default()
}

// ---------- what the Dashboard shows ----------

#[derive(Serialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub bytes: u64,
    pub files: u64,
    /// Transfers (downloads, links, sends).
    pub count: u64,
    pub failed: u64,
    /// Average speed while moving (bytes per second), when known.
    pub avg: Option<u64>,
    pub peak: u64,
    /// Different people (Direct).
    pub people: u64,
    /// Bytes that went straight between the two PCs (Direct: over the internet or the home
    /// network), not through a relay.
    pub direct_bytes: u64,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    /// "2026-11-16", "2026-11" or "14" (an hour of today).
    pub key: String,
    pub download: u64,
    pub upload: u64,
    pub sent: u64,
    pub received: u64,
    pub download_avg: Option<u64>,
    pub download_peak: u64,
    pub upload_avg: Option<u64>,
    pub upload_peak: u64,
    pub direct_avg: Option<u64>,
    pub direct_peak: u64,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RouteRow {
    /// Uploads: "gofile", "drive", "direct", "local". Downloads: "drive", "video", "web", "direct",
    /// "local", "earlier". "local": Direct over the home network; "direct": Direct over the internet.
    pub route: String,
    pub bytes: u64,
    pub count: u64,
    pub avg: Option<u64>,
    pub peak: u64,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub contact: String,
    pub sent: u64,
    pub received: u64,
    pub count: u64,
    pub peak_sent: u64,
    pub peak_received: u64,
    pub avg_sent: Option<u64>,
    pub avg_received: Option<u64>,
    /// ms
    pub last: i64,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub name: String,
    pub value: u64,
    pub day: String,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Records {
    pub fastest: Option<Record>,
    pub biggest: Option<Record>,
    pub busiest: Option<Record>,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NumbersView {
    pub period: String,
    pub from: String,
    pub to: String,
    /// The first day anything was counted.
    pub since: Option<String>,
    /// "hour", "day" or "month" (the charts' bars).
    pub step: String,
    pub download: Totals,
    pub upload: Totals,
    pub sent: Totals,
    pub received: Totals,
    pub buckets: Vec<Bucket>,
    pub upload_routes: Vec<RouteRow>,
    pub download_sources: Vec<RouteRow>,
    pub people: Vec<Person>,
    pub sites: Vec<Site>,
    pub records: Records,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub site: String,
    pub bytes: u64,
    pub count: u64,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContactNumbers {
    pub sent: u64,
    pub received: u64,
    pub sent_count: u64,
    pub received_count: u64,
    pub peak_sent: u64,
    pub peak_received: u64,
    pub avg_sent: Option<u64>,
    pub avg_received: Option<u64>,
    /// Not through a relay (the home network included).
    pub direct_bytes: u64,
    /// The part of `direct_bytes` that went over the home network.
    pub local_bytes: u64,
    pub relay_bytes: u64,
    /// ms
    pub last: Option<i64>,
}

fn avg_of(timed: i64, secs: f64) -> Option<u64> {
    (secs > 0.0 && timed > 0).then(|| (timed as f64 / secs) as u64)
}

/// The Dashboard's numbers for a period ("today", "week", "month", "all").
pub fn view(period: &str) -> NumbersView {
    let guard = DB.lock().unwrap();
    match guard.as_ref() {
        Some(conn) => view_at(conn, period, local_day(now_ms())).unwrap_or_default(),
        None => NumbersView { period: period.into(), ..Default::default() },
    }
}

fn first_day(conn: &Connection) -> rusqlite::Result<Option<NaiveDate>> {
    let k: Option<String> = conn.query_row(
        "SELECT MIN(key) FROM totals WHERE period = 'day' AND kind IN ('download', 'upload', 'sent', 'received')",
        [],
        |r| r.get(0),
    )?;
    Ok(k.as_deref().and_then(parse_day))
}

fn view_at(conn: &Connection, period: &str, today: NaiveDate) -> rusqlite::Result<NumbersView> {
    let first = first_day(conn)?;
    let (from, to) = period_range(period, today, first);
    let (fk, tk) = (day_key(from), day_key(to));
    let mut v = NumbersView {
        period: period.into(),
        from: fk.clone(),
        to: tk.clone(),
        since: first.map(day_key),
        ..Default::default()
    };

    // Totals per kind.
    {
        let mut q = conn.prepare(
            "SELECT kind, SUM(bytes), SUM(files), SUM(count), SUM(failed), SUM(timed_bytes), SUM(secs), MAX(peak),
                    COUNT(DISTINCT CASE WHEN contact != '' AND count > 0 THEN contact END),
                    SUM(CASE WHEN route IN ('local', 'direct6', 'direct') THEN bytes ELSE 0 END)
             FROM totals WHERE period = 'day' AND key BETWEEN ?1 AND ?2 GROUP BY kind",
        )?;
        let rows = q.query_map(params![fk, tk], |r| {
            let kind: String = r.get(0)?;
            let t = Totals {
                bytes: r.get::<_, i64>(1)? as u64,
                files: r.get::<_, i64>(2)? as u64,
                count: r.get::<_, i64>(3)? as u64,
                failed: r.get::<_, i64>(4)? as u64,
                avg: avg_of(r.get(5)?, r.get(6)?),
                peak: r.get::<_, i64>(7)? as u64,
                people: r.get::<_, i64>(8)? as u64,
                direct_bytes: r.get::<_, i64>(9)? as u64,
            };
            Ok((kind, t))
        })?;
        for row in rows {
            let (kind, t) = row?;
            match kind.as_str() {
                DOWNLOAD => v.download = t,
                UPLOAD => v.upload = t,
                SENT => v.sent = t,
                RECEIVED => v.received = t,
                _ => {}
            }
        }
    }

    // The chart's bars: hours of today, days, or months for a long "All time".
    let days = (to - from).num_days();
    if period == "today" {
        v.step = "hour".into();
        v.buckets = hour_buckets(conn, &tk)?;
    } else if days <= 62 {
        v.step = "day".into();
        v.buckets = period_buckets(conn, "day", &fk, &tk, &day_keys(from, to))?;
    } else {
        v.step = "month".into();
        v.buckets = period_buckets(conn, "month", &month_key(from), &month_key(to), &month_keys(from, to))?;
    }

    // Upload speed by route, download speed by source. Direct counts as two: over the home
    // network ("local") and over the internet ("direct", relayed or not).
    let rows = |sql: &str| -> rusqlite::Result<Vec<RouteRow>> {
        let mut q = conn.prepare(sql)?;
        let out = q
            .query_map(params![fk, tk], |r| {
                Ok(RouteRow {
                    route: r.get(0)?,
                    bytes: r.get::<_, i64>(1)? as u64,
                    count: r.get::<_, i64>(2)? as u64,
                    avg: avg_of(r.get(3)?, r.get(4)?),
                    peak: r.get::<_, i64>(5)? as u64,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>();
        out
    };
    let mut ups = rows(
        "SELECT CASE WHEN kind = 'sent' AND route = 'local' THEN 'local' WHEN kind = 'sent' THEN 'direct' ELSE route END AS r,
                SUM(bytes), SUM(count), SUM(timed_bytes), SUM(secs), MAX(peak)
         FROM totals WHERE period = 'day' AND key BETWEEN ?1 AND ?2 AND kind IN ('upload', 'sent') GROUP BY r",
    )?;
    for r in ["gofile", "drive", "direct", "local"] {
        if !ups.iter().any(|x| x.route == r) {
            ups.push(RouteRow { route: r.into(), ..Default::default() });
        }
    }
    ups.sort_by_key(|x| ["gofile", "drive", "direct", "local"].iter().position(|r| *r == x.route).unwrap_or(9));
    v.upload_routes = ups;
    let mut downs = rows(
        "SELECT CASE WHEN kind = 'received' AND route = 'local' THEN 'local' WHEN kind = 'received' THEN 'direct' WHEN source = '' THEN 'earlier' ELSE source END AS r,
                SUM(bytes), SUM(count), SUM(timed_bytes), SUM(secs), MAX(peak)
         FROM totals WHERE period = 'day' AND key BETWEEN ?1 AND ?2 AND kind IN ('download', 'received') GROUP BY r",
    )?;
    for r in ["drive", "video", "web", "direct", "local"] {
        if !downs.iter().any(|x| x.route == r) {
            downs.push(RouteRow { route: r.into(), ..Default::default() });
        }
    }
    downs.retain(|x| x.route != "earlier" || x.bytes > 0);
    downs.sort_by_key(|x| ["drive", "video", "web", "direct", "local", "earlier"].iter().position(|r| *r == x.route).unwrap_or(9));
    v.download_sources = downs;

    // People, most exchanged first.
    {
        let mut q = conn.prepare(
            "SELECT contact, kind, SUM(bytes), SUM(count), MAX(peak), SUM(timed_bytes), SUM(secs), MAX(last)
             FROM totals WHERE period = 'day' AND key BETWEEN ?1 AND ?2 AND kind IN ('sent', 'received') AND contact != ''
             GROUP BY contact, kind",
        )?;
        let rows = q.query_map(params![fk, tk], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)? as u64,
                r.get::<_, i64>(3)? as u64,
                r.get::<_, i64>(4)? as u64,
                avg_of(r.get(5)?, r.get(6)?),
                r.get::<_, i64>(7)?,
            ))
        })?;
        let mut people: Vec<Person> = Vec::new();
        for row in rows {
            let (contact, kind, bytes, count, peak, avg, last) = row?;
            let p = match people.iter_mut().find(|p| p.contact == contact) {
                Some(p) => p,
                None => {
                    people.push(Person { contact: contact.clone(), ..Default::default() });
                    people.last_mut().unwrap()
                }
            };
            p.count += count;
            p.last = p.last.max(last);
            if kind == SENT {
                p.sent += bytes;
                p.peak_sent = peak;
                p.avg_sent = avg;
            } else {
                p.received += bytes;
                p.peak_received = peak;
                p.avg_received = avg;
            }
        }
        people.retain(|p| p.count > 0);
        people.sort_by(|a, b| (b.sent + b.received).cmp(&(a.sent + a.received)));
        v.people = people;
    }

    // The sites downloaded from most (from the events: the last 12 months at most).
    {
        let mut q = conn.prepare(
            "SELECT site, SUM(bytes), COUNT(*) FROM events WHERE kind = 'download' AND ok = 1 AND site != '' AND day BETWEEN ?1 AND ?2
             GROUP BY site ORDER BY 2 DESC LIMIT 6",
        )?;
        v.sites = q
            .query_map(params![fk, tk], |r| Ok(Site { site: r.get(0)?, bytes: r.get::<_, i64>(1)? as u64, count: r.get::<_, i64>(2)? as u64 }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }

    // Records.
    v.records.fastest = conn
        .query_row(
            "SELECT name, peak, day FROM events WHERE kind = 'download' AND ok = 1 AND peak IS NOT NULL AND day BETWEEN ?1 AND ?2
             ORDER BY peak DESC LIMIT 1",
            params![fk, tk],
            |r| Ok(Record { name: r.get(0)?, value: r.get::<_, i64>(1)? as u64, day: r.get(2)? }),
        )
        .optional()?;
    v.records.biggest = conn
        .query_row(
            "SELECT name, bytes, day FROM events WHERE kind IN ('download', 'received') AND ok = 1 AND backfilled = 0 AND files <= 1
               AND day BETWEEN ?1 AND ?2 ORDER BY bytes DESC LIMIT 1",
            params![fk, tk],
            |r| Ok(Record { name: r.get(0)?, value: r.get::<_, i64>(1)? as u64, day: r.get(2)? }),
        )
        .optional()?;
    v.records.busiest = conn
        .query_row(
            "SELECT key, SUM(bytes) AS b FROM totals WHERE period = 'day' AND key BETWEEN ?1 AND ?2
               AND kind IN ('download', 'upload', 'sent', 'received') GROUP BY key HAVING b > 0
             ORDER BY b DESC LIMIT 1",
            params![fk, tk],
            |r| Ok(Record { name: String::new(), value: r.get::<_, i64>(1)? as u64, day: r.get(0)? }),
        )
        .optional()?;
    Ok(v)
}

fn day_keys(from: NaiveDate, to: NaiveDate) -> Vec<String> {
    let mut out = Vec::new();
    let mut d = from;
    while d <= to {
        out.push(day_key(d));
        d += Duration::days(1);
    }
    out
}

fn month_keys(from: NaiveDate, to: NaiveDate) -> Vec<String> {
    let mut out = Vec::new();
    let mut d = month_start(from);
    while d <= to {
        out.push(month_key(d));
        d = if d.month() == 12 {
            NaiveDate::from_ymd_opt(d.year() + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1).unwrap()
        };
    }
    out
}

/// One bar per key, every key present (empty bars for quiet days).
fn period_buckets(conn: &Connection, period: &str, from: &str, to: &str, keys: &[String]) -> rusqlite::Result<Vec<Bucket>> {
    let mut out: Vec<Bucket> = keys.iter().map(|k| Bucket { key: k.clone(), ..Default::default() }).collect();
    let mut q = conn.prepare(
        "SELECT key, kind, SUM(bytes), SUM(timed_bytes), SUM(secs), MAX(peak)
         FROM totals WHERE period = ?1 AND key BETWEEN ?2 AND ?3 GROUP BY key, kind",
    )?;
    let rows = q.query_map(params![period, from, to], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)? as u64,
            r.get::<_, i64>(3)?,
            r.get::<_, f64>(4)?,
            r.get::<_, i64>(5)? as u64,
        ))
    })?;
    // Direct combines both ways: its speed is worked out from the sums.
    let mut direct: Vec<(i64, f64)> = vec![(0, 0.0); out.len()];
    for row in rows {
        let (key, kind, bytes, timed, secs, peak) = row?;
        let Some(i) = out.iter().position(|b| b.key == key) else { continue };
        let b = &mut out[i];
        match kind.as_str() {
            DOWNLOAD => {
                b.download += bytes;
                b.download_avg = avg_of(timed, secs);
                b.download_peak = b.download_peak.max(peak);
            }
            UPLOAD => {
                b.upload += bytes;
                b.upload_avg = avg_of(timed, secs);
                b.upload_peak = b.upload_peak.max(peak);
            }
            SENT | RECEIVED => {
                if kind == SENT {
                    b.sent += bytes;
                } else {
                    b.received += bytes;
                }
                direct[i].0 += timed;
                direct[i].1 += secs;
                b.direct_peak = b.direct_peak.max(peak);
            }
            _ => {}
        }
    }
    for (b, (t, s)) in out.iter_mut().zip(direct) {
        b.direct_avg = avg_of(t, s);
    }
    Ok(out)
}

/// Today hour by hour, from the events themselves.
fn hour_buckets(conn: &Connection, day: &str) -> rusqlite::Result<Vec<Bucket>> {
    let mut out: Vec<Bucket> = (0..24).map(|h| Bucket { key: format!("{h:02}"), ..Default::default() }).collect();
    let mut sums: Vec<[(i64, f64); 3]> = vec![[(0, 0.0); 3]; 24];
    let mut q = conn.prepare("SELECT at, kind, bytes, secs, peak FROM events WHERE day = ?1 AND ok = 1")?;
    let rows = q.query_map(params![day], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, Option<f64>>(3)?, r.get::<_, Option<i64>>(4)?))
    })?;
    for row in rows {
        let (at, kind, bytes, secs, peak) = row?;
        let h = local_hour(at) as usize;
        let b = &mut out[h];
        let (slot, peak_of) = match kind.as_str() {
            DOWNLOAD => {
                b.download += bytes as u64;
                (0, &mut b.download_peak)
            }
            UPLOAD => {
                b.upload += bytes as u64;
                (1, &mut b.upload_peak)
            }
            SENT => {
                b.sent += bytes as u64;
                (2, &mut b.direct_peak)
            }
            RECEIVED => {
                b.received += bytes as u64;
                (2, &mut b.direct_peak)
            }
            _ => continue,
        };
        *peak_of = (*peak_of).max(peak.unwrap_or(0) as u64);
        if let Some(s) = secs.filter(|s| *s > 0.0) {
            sums[h][slot].0 += bytes;
            sums[h][slot].1 += s;
        }
    }
    for (b, s) in out.iter_mut().zip(sums) {
        b.download_avg = avg_of(s[0].0, s[0].1);
        b.upload_avg = avg_of(s[1].0, s[1].1);
        b.direct_avg = avg_of(s[2].0, s[2].1);
    }
    Ok(out)
}

/// Everything with one contact, all time (Direct's details panel).
pub fn contact(id: &str) -> ContactNumbers {
    let guard = DB.lock().unwrap();
    guard.as_ref().and_then(|c| contact_at(c, id).ok()).unwrap_or_default()
}

fn contact_at(conn: &Connection, id: &str) -> rusqlite::Result<ContactNumbers> {
    let mut out = ContactNumbers::default();
    let mut q = conn.prepare(
        "SELECT kind, route, SUM(bytes), SUM(count), MAX(peak), SUM(timed_bytes), SUM(secs), MAX(last)
         FROM totals WHERE period = 'day' AND contact = ?1 GROUP BY kind, route",
    )?;
    let rows = q.query_map(params![id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)? as u64,
            r.get::<_, i64>(3)? as u64,
            r.get::<_, i64>(4)? as u64,
            r.get::<_, i64>(5)?,
            r.get::<_, f64>(6)?,
            r.get::<_, i64>(7)?,
        ))
    })?;
    let (mut ts, mut ss, mut tr, mut sr) = (0i64, 0f64, 0i64, 0f64);
    for row in rows {
        let (kind, route, bytes, count, peak, timed, secs, last) = row?;
        if count == 0 {
            continue;
        }
        out.last = Some(out.last.unwrap_or(0).max(last));
        if route == "relay" {
            out.relay_bytes += bytes;
        } else if route == "local" || route == "direct6" || route == "direct" {
            out.direct_bytes += bytes;
            if route == "local" {
                out.local_bytes += bytes;
            }
        }
        if kind == SENT {
            out.sent += bytes;
            out.sent_count += count;
            out.peak_sent = out.peak_sent.max(peak);
            ts += timed;
            ss += secs;
        } else if kind == RECEIVED {
            out.received += bytes;
            out.received_count += count;
            out.peak_received = out.peak_received.max(peak);
            tr += timed;
            sr += secs;
        }
    }
    out.avg_sent = avg_of(ts, ss);
    out.avg_received = avg_of(tr, sr);
    Ok(out)
}

// ---------- export, reset ----------

/// Writes every kept event to a CSV file (opens in Excel). Returns how many rows.
pub fn export_csv(path: &Path) -> Result<usize, String> {
    let guard = DB.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| tr!("The numbers aren't open.", "Les chiffres ne sont pas ouverts."))?;
    let (csv, n) = csv_of(conn)?;
    // A BOM so Excel reads names with accents correctly.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(csv.as_bytes());
    std::fs::write(path, bytes).map_err(|e| tr!("Couldn't save the file: {e}", "Impossible d’enregistrer le fichier : {e}"))?;
    Ok(n)
}

fn csv_of(conn: &Connection) -> Result<(String, usize), String> {
    let mut q = conn
        .prepare("SELECT at, day, kind, name, bytes, files, secs, avg, peak, source, site, ftype, route, contact, ok, backfilled FROM events ORDER BY at")
        .map_err(|e| e.to_string())?;
    let mut csv = String::from("time,day,kind,name,bytes,files,seconds,average_bytes_per_second,peak_bytes_per_second,source,site,file_type,route,contact,ok,from_older_files\r\n");
    let mut n = 0usize;
    let rows = q
        .query_map([], |r| {
            let at: i64 = r.get(0)?;
            let when = Local.timestamp_millis_opt(at).earliest().map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
            let opt_f = |v: Option<f64>| v.map(|x| format!("{x:.1}")).unwrap_or_default();
            let opt_i = |v: Option<i64>| v.map(|x| x.to_string()).unwrap_or_default();
            Ok(vec![
                when,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?.to_string(),
                r.get::<_, i64>(5)?.to_string(),
                opt_f(r.get(6)?),
                opt_i(r.get(7)?),
                opt_i(r.get(8)?),
                r.get::<_, String>(9)?,
                r.get::<_, String>(10)?,
                r.get::<_, String>(11)?,
                r.get::<_, String>(12)?,
                r.get::<_, String>(13)?,
                if r.get::<_, bool>(14)? { "yes".into() } else { "no".into() },
                if r.get::<_, bool>(15)? { "yes".into() } else { "no".into() },
            ])
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let row = row.map_err(|e| e.to_string())?;
        csv.push_str(&row.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(","));
        csv.push_str("\r\n");
        n += 1;
    }
    Ok((csv, n))
}

fn csv_field(s: &str) -> String {
    // A leading = + - @ would run as a formula in Excel: quote it as text.
    let risky = s.starts_with(['=', '+', '-', '@', '\t', '\r']) && s.parse::<f64>().is_err();
    if risky || s.contains([',', '"', '\n', '\r']) {
        let inner = if risky { format!("'{s}") } else { s.to_string() };
        format!("\"{}\"", inner.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Forgets every number (the files themselves are untouched). Older files aren't read again.
pub fn reset() -> Result<(), String> {
    if DB.lock().unwrap().is_none() {
        let dir = DIR.lock().unwrap().clone();
        if let Some(dir) = dir {
            open(&dir);
        }
    }
    let guard = DB.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| tr!("Pako couldn't open its numbers. Restart Pako, then try again.", "Pako n’a pas pu ouvrir ses chiffres. Redémarrez Pako, puis réessayez."))?;
    reset_at(conn).map_err(|e| e.to_string())
}

/// All or nothing: a reset cut off part-way leaves the numbers as they were.
fn reset_at(conn: &Connection) -> rusqlite::Result<()> {
    // One transaction: a statement failing part-way rolls it all back (dropped without commit)
    // instead of leaving it open on the connection.
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(&format!(
        "DELETE FROM events; DELETE FROM totals; INSERT OR REPLACE INTO meta(k, v) VALUES('{BACKFILL}', 'reset');"
    ))?;
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        init(&c).unwrap();
        c
    }

    fn at(day: NaiveDate) -> i64 {
        local_noon(day)
    }

    #[test]
    fn week_and_month_keys_across_boundaries() {
        // Monday start; Sunday belongs to the week before.
        assert_eq!(week_start(d(2026, 11, 16)), d(2026, 11, 16));
        assert_eq!(week_start(d(2026, 11, 22)), d(2026, 11, 16));
        assert_eq!(week_start(d(2026, 11, 15)), d(2026, 11, 9));
        // ISO weeks at the turn of the year.
        assert_eq!(week_key(d(2026, 12, 31)), "2026-W53");
        assert_eq!(week_key(d(2027, 1, 3)), "2026-W53");
        assert_eq!(week_key(d(2027, 1, 4)), "2027-W01");
        assert_eq!(week_key(d(2025, 12, 29)), "2026-W01");
        assert_eq!(month_key(d(2028, 2, 29)), "2028-02");
        assert_eq!(month_start(d(2028, 2, 29)), d(2028, 2, 1));
        // A week reaching back into last month (and last year).
        assert_eq!(week_start(d(2027, 1, 1)), d(2026, 12, 28));
    }

    #[test]
    fn period_ranges() {
        let today = d(2026, 11, 18);
        assert_eq!(period_range("today", today, None), (today, today));
        assert_eq!(period_range("week", today, None), (d(2026, 11, 16), today));
        assert_eq!(period_range("month", today, None), (d(2026, 11, 1), today));
        assert_eq!(period_range("all", today, Some(d(2025, 3, 2))), (d(2025, 3, 2), today));
        assert_eq!(period_range("all", today, None), (today, today));
        assert_eq!(month_keys(d(2025, 11, 20), d(2026, 2, 3)), vec!["2025-11", "2025-12", "2026-01", "2026-02"]);
        assert_eq!(day_keys(d(2028, 2, 27), d(2028, 3, 1)).len(), 4); // leap year
    }

    /// Direct over the home network is its own route ("Local network") in "Sending files" and
    /// "Getting files", and counts as straight between the PCs (not through a relay).
    #[test]
    fn local_network_is_its_own_route() {
        let mut c = mem();
        let today = d(2026, 11, 18);
        let s = direct_event("l1", at(today), true, "Film", 4_000_000_000, 1, Some("local"), "mum", 40_000, 0, 120_000_000);
        insert(&mut c, &s).unwrap();
        let s = direct_event("w1", at(today), true, "Photos", 30_000_000, 3, Some("direct"), "mum", 3_000, 0, 12_000_000);
        insert(&mut c, &s).unwrap();
        let s = direct_event("l2", at(today), false, "Scans", 500_000_000, 2, Some("local"), "mum", 5_000, 0, 110_000_000);
        insert(&mut c, &s).unwrap();
        let v = view_at(&c, "today", today).unwrap();
        let routes: Vec<&str> = v.upload_routes.iter().map(|r| r.route.as_str()).collect();
        assert_eq!(routes, ["gofile", "drive", "direct", "local"]);
        let local = v.upload_routes.iter().find(|r| r.route == "local").unwrap();
        assert_eq!((local.bytes, local.count, local.avg), (4_000_000_000, 1, Some(100_000_000)));
        assert_eq!(v.upload_routes.iter().find(|r| r.route == "direct").unwrap().bytes, 30_000_000);
        let got = v.download_sources.iter().find(|r| r.route == "local").unwrap();
        assert_eq!(got.bytes, 500_000_000);
        assert_eq!(v.download_sources.iter().find(|r| r.route == "direct").unwrap().bytes, 0);
        assert_eq!(v.sent.direct_bytes, 4_030_000_000, "the home network counts as straight between the PCs");
        assert_eq!(v.received.direct_bytes, 500_000_000);
        let p = contact_at(&c, "mum").unwrap();
        assert_eq!((p.direct_bytes, p.relay_bytes), (4_530_000_000, 0));
        assert_eq!(p.local_bytes, 4_500_000_000, "the home network's share is its own part");
    }

    #[test]
    fn rollups_add_up_and_count_once() {
        let mut c = mem();
        let today = d(2026, 11, 18);
        let mut a = Event::new("dl:a", at(today), DOWNLOAD).timed(100_000_000, 10.0, 15_000_000.0);
        a.bytes = 100_000_000;
        a.source = "drive".into();
        a.site = "drive.google.com".into();
        assert!(insert(&mut c, &a).unwrap());
        assert!(!insert(&mut c, &a).unwrap(), "the same download is never counted twice");
        let mut b = Event::new("dl:b", at(d(2026, 11, 2)), DOWNLOAD).timed(50_000_000, 25.0, 4_000_000.0);
        b.bytes = 50_000_000;
        b.source = "web".into();
        insert(&mut c, &b).unwrap();
        let mut f = Event::new("dl:c", at(today), DOWNLOAD);
        f.ok = false;
        insert(&mut c, &f).unwrap();
        let mut up = Event::new("up:1", at(today), UPLOAD).timed(20_000_000, 8.0, 3_000_000.0);
        up.bytes = 20_000_000;
        up.route = "gofile".into();
        insert(&mut c, &up).unwrap();
        let mut s = direct_event("x", at(today), true, "Photos", 30_000_000, 3, Some("direct6"), "rado", 2_000, 0, 20_000_000);
        insert(&mut c, &s).unwrap();
        s = direct_event("y", at(today), false, "Song", 10_000_000, 1, Some("relay"), "rado", 4_000, 0, 3_000_000);
        insert(&mut c, &s).unwrap();

        let v = view_at(&c, "month", today).unwrap();
        assert_eq!(v.download.bytes, 150_000_000);
        assert_eq!(v.download.count, 2);
        assert_eq!(v.download.failed, 1);
        assert_eq!(v.download.avg, Some(150_000_000 / 35));
        assert_eq!(v.download.peak, 15_000_000);
        assert_eq!(v.upload.count, 1);
        assert_eq!(v.sent.bytes, 30_000_000);
        assert_eq!(v.sent.people, 1);
        assert_eq!(v.sent.direct_bytes, 30_000_000);
        assert_eq!(v.received.direct_bytes, 0, "the received one came through a relay");
        assert_eq!(v.step, "day");
        assert_eq!(v.buckets.len(), 18);
        let sum: u64 = v.buckets.iter().map(|b| b.download).sum();
        assert_eq!(sum, v.download.bytes, "the bars add up to the total");
        // Routes and sources.
        let go = v.upload_routes.iter().find(|r| r.route == "gofile").unwrap();
        assert_eq!(go.avg, Some(2_500_000));
        let direct = v.upload_routes.iter().find(|r| r.route == "direct").unwrap();
        assert_eq!(direct.bytes, 30_000_000);
        assert_eq!(v.upload_routes.iter().find(|r| r.route == "drive").unwrap().count, 0);
        assert_eq!(v.download_sources.iter().find(|r| r.route == "direct").unwrap().bytes, 10_000_000);
        // People add up with the totals.
        assert_eq!(v.people.len(), 1);
        assert_eq!(v.people[0].sent, v.sent.bytes);
        assert_eq!(v.people[0].received, v.received.bytes);
        assert_eq!(v.people[0].peak_sent, 20_000_000);
        assert_eq!(v.sites.len(), 1);
        assert_eq!((v.sites[0].site.as_str(), v.sites[0].bytes), ("drive.google.com", 100_000_000));
        // Records.
        assert_eq!(v.records.fastest.as_ref().unwrap().value, 15_000_000);
        assert_eq!(v.records.busiest.as_ref().unwrap().day, "2026-11-18");
        // Week: only from Monday 16th.
        let w = view_at(&c, "week", today).unwrap();
        assert_eq!(w.download.bytes, 100_000_000);
        // Today, hour by hour.
        let t = view_at(&c, "today", today).unwrap();
        assert_eq!(t.step, "hour");
        assert_eq!(t.buckets.len(), 24);
        assert_eq!(t.buckets.iter().map(|b| b.download).sum::<u64>(), 100_000_000);
        // All time over a long span uses months.
        let mut old = Event::new("dl:old", at(d(2025, 6, 1)), DOWNLOAD);
        old.bytes = 1;
        insert(&mut c, &old).unwrap();
        let all = view_at(&c, "all", today).unwrap();
        assert_eq!(all.step, "month");
        assert_eq!(all.since.as_deref(), Some("2025-06-01"));
        assert_eq!(all.buckets.first().unwrap().key, "2025-06");
        assert_eq!(all.buckets.iter().map(|b| b.download).sum::<u64>(), all.download.bytes);
        // One contact, all time.
        let p = contact_at(&c, "rado").unwrap();
        assert_eq!((p.sent, p.received, p.sent_count, p.received_count), (30_000_000, 10_000_000, 1, 1));
        assert_eq!((p.direct_bytes, p.relay_bytes), (30_000_000, 10_000_000));
        assert_eq!(p.avg_sent, Some(15_000_000));
    }

    #[test]
    fn backfill_reads_older_files_without_inventing_speeds() {
        let dir = std::env::temp_dir().join(format!("pako-numbers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("stats.json"),
            r#"{"days":{"2026-11-10":{"pako":5000,"down":1,"up":1,"files":2,"failed":1,"max":900,"busySecs":10},
                         "2026-11-18":{"pako":7,"down":0,"up":0,"files":1,"failed":0,"max":0,"busySecs":0}},"minutes":[]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("sent.json"),
            r#"[{"id":"s1","name":"a.zip","size":800,"files":1,"date":1794700000000,"link":"x","compressed":false,"host":"drive"},
                {"id":"s2","name":"b","size":5,"files":2,"date":1794700000000,"link":"y","compressed":false,"incomplete":true}]"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("direct_history.json"),
            r#"[{"id":"t1","contact":"rado","outgoing":true,"name":"p","files":1,"size":300,"date":1794700000000,"result":"done","path":"relay"},
                {"id":"t2","contact":"rado","outgoing":false,"name":"q","files":1,"size":9,"date":1794700000000,"result":"declined"}]"#,
        )
        .unwrap();
        let events = backfill_events(&dir, d(2026, 11, 18));
        let _ = std::fs::remove_dir_all(&dir);
        let ext: Vec<&str> = events.iter().map(|e| e.ext.as_str()).collect();
        assert!(ext.contains(&"statsday:2026-11-10") && ext.contains(&"statsfail:2026-11-10"));
        assert!(!ext.contains(&"statsday:2026-11-18"), "today keeps counting live");
        assert!(ext.contains(&"up:s1") && !ext.contains(&"up:s2"), "unfinished sends aren't counted");
        assert!(ext.contains(&"direct:t1") && !ext.contains(&"direct:t2"));
        let day = events.iter().find(|e| e.ext == "statsday:2026-11-10").unwrap();
        assert_eq!((day.bytes, day.avg, day.peak), (5000, Some(500), Some(900)));
        assert!(day.source.is_empty(), "no split by site for older days");
        let up = events.iter().find(|e| e.ext == "up:s1").unwrap();
        assert_eq!((up.route.as_str(), up.avg), ("drive", None), "no speed was kept for older uploads");
        let t1 = events.iter().find(|e| e.ext == "direct:t1").unwrap();
        assert_eq!((t1.kind, t1.route.as_str(), t1.avg), (SENT, "relay", None));
        assert!(events.iter().all(|e| e.backfilled));
        // And they land in the totals like live events (the failed day counts as 1 failure).
        let mut c = mem();
        for e in &events {
            insert(&mut c, e).unwrap();
        }
        let v = view_at(&c, "all", d(2026, 11, 18)).unwrap();
        assert_eq!(v.download.bytes, 5000);
        assert_eq!(v.download.failed, 1);
        assert!(v.download_sources.iter().any(|r| r.route == "earlier" && r.bytes == 5000));
    }

    #[test]
    fn csv_export_has_one_row_per_event() {
        let mut c = mem();
        let mut a = Event::new("dl:a", at(d(2026, 11, 2)), DOWNLOAD).timed(1000, 2.0, 700.0);
        a.name = "Report, final.pdf".into();
        a.bytes = 1000;
        insert(&mut c, &a).unwrap();
        let mut b = Event::new("up:b", at(d(2026, 11, 3)), UPLOAD);
        b.name = "=SUM(A1)".into();
        b.route = "gofile".into();
        b.backfilled = true;
        insert(&mut c, &b).unwrap();
        let (csv, n) = csv_of(&c).unwrap();
        assert_eq!(n, 2);
        let lines: Vec<&str> = csv.split("\r\n").filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("time,day,kind,name,bytes"));
        assert!(lines[1].contains(",2026-11-02,download,\"Report, final.pdf\",1000,1,2.0,500,700,"));
        assert!(lines[2].contains(",upload,\"'=SUM(A1)\",0,1,,,,"), "{}", lines[2]);
        assert!(lines[2].ends_with(",gofile,,yes,yes"), "{}", lines[2]);
    }

    #[test]
    fn csv_fields_are_safe() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("=cmd()"), "\"'=cmd()\"");
        assert_eq!(csv_field("-12.5"), "-12.5");
        assert_eq!(csv_field("\tcmd"), "\"'\tcmd\"");
        assert_eq!(csv_field("@SUM(A1)"), "\"'@SUM(A1)\"");
    }

    #[test]
    fn file_types_and_sites() {
        assert_eq!(file_type("Movie.MKV"), "video");
        assert_eq!(file_type("part.7z"), "archive");
        assert_eq!(file_type("noext"), "other");
        assert_eq!(site_of("https://www.youtube.com/watch?v=x"), "youtube.com");
        assert_eq!(site_of("not a link"), "");
    }

    /// A resumed download's speed comes from what moved while it was timed, not its whole size.
    #[test]
    fn speed_counts_only_what_moved() {
        let mut c = mem();
        let today = d(2026, 11, 18);
        // 1 GB file, 100 MB of it fetched in 10 s after a resume.
        let mut e = Event::new("dl:r", at(today), DOWNLOAD).timed(100_000_000, 10.0, 12_000_000.0);
        e.bytes = 1_000_000_000;
        insert(&mut c, &e).unwrap();
        let v = view_at(&c, "today", today).unwrap();
        assert_eq!(v.download.bytes, 1_000_000_000);
        assert_eq!(v.download.avg, Some(10_000_000));
    }

    /// Uploads Pako deleted on their date aren't transfers: not the first day, not the busiest.
    #[test]
    fn cleaned_uploads_arent_transfers() {
        let mut c = mem();
        let today = d(2026, 11, 18);
        let mut gone = Event::new("cleaned:1", at(d(2026, 11, 3)), CLEANED);
        gone.bytes = 9_000_000_000;
        insert(&mut c, &gone).unwrap();
        let mut dl = Event::new("dl:1", at(d(2026, 11, 10)), DOWNLOAD);
        dl.bytes = 5_000_000;
        insert(&mut c, &dl).unwrap();
        assert_eq!(first_day(&c).unwrap(), Some(d(2026, 11, 10)));
        let v = view_at(&c, "month", today).unwrap();
        assert_eq!(v.records.busiest.as_ref().map(|r| (r.day.as_str(), r.value)), Some(("2026-11-10", 5_000_000)));
        // Weeks come from the days: no week rows are kept.
        let weeks: i64 = c.query_row("SELECT COUNT(*) FROM totals WHERE period = 'week'", [], |r| r.get(0)).unwrap();
        assert_eq!(weeks, 0);
    }

    /// A reset that fails part-way changes nothing and leaves no transaction open: the next
    /// event is still counted.
    #[test]
    fn a_failed_reset_rolls_back() {
        let mut c = mem();
        insert(&mut c, &Event::new("dl:1", at(d(2026, 11, 10)), DOWNLOAD)).unwrap();
        // The reset's last statement (writing to meta) fails, after the two DELETEs ran.
        c.execute_batch("CREATE TRIGGER no_meta BEFORE INSERT ON meta BEGIN SELECT RAISE(ABORT, 'no'); END;").unwrap();
        assert!(reset_at(&c).is_err());
        let n: i64 = c.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "the DELETEs were rolled back");
        assert!(c.is_autocommit(), "no transaction left open");
        assert!(insert(&mut c, &Event::new("dl:2", at(d(2026, 11, 11)), DOWNLOAD)).unwrap());
    }

    /// Reset empties everything at once; a damaged file is set aside and the numbers start again.
    #[test]
    fn reset_and_a_damaged_file() {
        let mut c = mem();
        insert(&mut c, &Event::new("dl:1", at(d(2026, 11, 10)), DOWNLOAD)).unwrap();
        reset_at(&c).unwrap();
        let n: i64 = c.query_row("SELECT (SELECT COUNT(*) FROM events) + (SELECT COUNT(*) FROM totals)", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
        let v: String = c.query_row("SELECT v FROM meta WHERE k = ?1", params![BACKFILL], |r| r.get(0)).unwrap();
        assert_eq!(v, "reset");

        let dir = std::env::temp_dir().join(format!("pako-numbers-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("numbers.sqlite");
        std::fs::write(&p, vec![7u8; 4096]).unwrap();
        let mut conn = open_or_start_again(&p).expect("starts again");
        insert(&mut conn, &Event::new("dl:2", at(d(2026, 11, 10)), DOWNLOAD)).unwrap();
        drop(conn);
        let aside = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().starts_with("numbers.sqlite.bad-")).count();
        assert_eq!(aside, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
