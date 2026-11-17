// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! History for the Dashboard: how fast Pako downloaded, how much per day, and how the
//! whole PC's internet behaved (download and upload, from Windows' network counters).
//!
//! Kept small: 5-second points for the last hour, 1-minute points for the last 24 hours,
//! and one line per day for 90 days, saved in data/stats.json.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::paths::{data_dir, write_atomic};

/// Bytes Pako has written since it started (every download together).
pub static PAKO_BYTES: AtomicU64 = AtomicU64::new(0);

const HOUR: u64 = 3600;
const DAY: u64 = 24 * HOUR;
const KEEP_DAYS: usize = 90;
/// Samples come every 5 seconds; a longer gap means the PC slept, so at most this much
/// counts as time spent downloading.
const MAX_GAP_SECS: f64 = 15.0;

/// Bytes (received, sent) per network adapter, by its LUID.
type NetCounters = HashMap<u64, (u64, u64)>;

/// One day of history (local date).
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// Bytes Pako downloaded.
    pub pako: u64,
    /// Bytes the whole PC received / sent.
    pub down: u64,
    pub up: u64,
    pub files: u32,
    pub failed: u32,
    /// Fastest Pako speed seen (bytes per second, over 5 seconds).
    pub max: u64,
    /// Seconds Pako was downloading (for the average speed).
    pub busy_secs: u64,
}

/// A point on the speed charts: [unix seconds, Pako B/s, PC down B/s, PC up B/s].
pub type Point = [u64; 4];

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    days: BTreeMap<String, Day>,
    minutes: VecDeque<Point>,
}

pub struct Stats {
    saved: Saved,
    recent: VecDeque<Point>,
    last_at: Instant,
    last_pako: u64,
    last_net: Option<NetCounters>,
    minute: Option<(u64, [u64; 3], u64)>,
    last_save: Instant,
    /// The saved history was read (or there was none). False = it's there but couldn't be read
    /// at start-up: it is never saved over this session.
    loaded_ok: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsView {
    /// Last hour, every 5 seconds.
    pub recent: Vec<Point>,
    /// Last 24 hours, every minute.
    pub minutes: Vec<Point>,
    /// Up to 90 days, oldest first: (YYYY-MM-DD, totals).
    pub days: Vec<(String, Day)>,
    /// Whether the PC's network counters could be read.
    pub has_pc: bool,
}

fn path() -> PathBuf {
    data_dir().join("stats.json")
}

fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Stats {
    pub fn load() -> Self {
        let read = crate::paths::read_json_checked::<Saved>(&path());
        let loaded_ok = read.is_ok();
        Self {
            saved: read.ok().flatten().unwrap_or_default(),
            loaded_ok,
            recent: VecDeque::new(),
            last_at: Instant::now(),
            last_pako: PAKO_BYTES.load(Ordering::Relaxed),
            last_net: net_totals(),
            minute: None,
            last_save: Instant::now(),
        }
    }

    /// Called every few seconds: adds a point and today's totals.
    pub fn sample(&mut self) {
        let dt = self.last_at.elapsed().as_secs_f64();
        if dt < 1.0 {
            return;
        }
        self.last_at = Instant::now();
        let pako_total = PAKO_BYTES.load(Ordering::Relaxed);
        let pako = pako_total.saturating_sub(self.last_pako);
        self.last_pako = pako_total;
        let net = net_totals();
        let (down, up) = match (&self.last_net, &net) {
            (Some(before), Some(now)) => net_delta(before, now),
            _ => (0, 0),
        };
        self.last_net = net;
        let rate = |b: u64| (b as f64 / dt) as u64;
        let now = now_unix();
        let point = [now, rate(pako), rate(down), rate(up)];
        self.recent.push_back(point);
        while self.recent.front().is_some_and(|p| p[0] + HOUR < now) {
            self.recent.pop_front();
        }
        // One averaged point per minute for the 24-hour chart.
        let m = now / 60;
        match &mut self.minute {
            Some((cur, sum, n)) if *cur == m => {
                for i in 0..3 {
                    sum[i] += point[i + 1];
                }
                *n += 1;
            }
            _ => {
                if let Some((cur, sum, n)) = self.minute.take() {
                    let n = n.max(1);
                    self.saved.minutes.push_back([cur * 60, sum[0] / n, sum[1] / n, sum[2] / n]);
                }
                self.minute = Some((m, [point[1], point[2], point[3]], 1));
            }
        }
        while self.saved.minutes.front().is_some_and(|p| p[0] + DAY < now) {
            self.saved.minutes.pop_front();
        }
        let day = self.today();
        day.pako += pako;
        day.down += down;
        day.up += up;
        day.max = day.max.max(rate(pako));
        if pako > 0 {
            day.busy_secs += dt.min(MAX_GAP_SECS).round() as u64;
        }
        if self.last_save.elapsed().as_secs() >= 60 {
            self.save();
        }
    }

    /// A download finished (or stopped with an error).
    pub fn finished(&mut self, ok: bool) {
        let day = self.today();
        if ok {
            day.files += 1;
        } else {
            day.failed += 1;
        }
    }

    fn today(&mut self) -> &mut Day {
        let key = local_date();
        if !self.saved.days.contains_key(&key) {
            while self.saved.days.len() >= KEEP_DAYS {
                let oldest = self.saved.days.keys().next().cloned();
                if let Some(k) = oldest {
                    self.saved.days.remove(&k);
                }
            }
        }
        self.saved.days.entry(key).or_default()
    }

    /// Forgets the history (Dashboard › Reset statistics).
    pub fn reset(&mut self) {
        self.saved = Saved::default();
        self.recent.clear();
        self.minute = None;
        self.loaded_ok = true;
        self.save();
    }

    pub fn save(&mut self) {
        if !self.loaded_ok {
            return;
        }
        if let Ok(bytes) = serde_json::to_vec(&self.saved) {
            let _ = write_atomic(&path(), &bytes);
        }
        self.last_save = Instant::now();
    }

    pub fn view(&self) -> StatsView {
        StatsView {
            recent: self.recent.iter().copied().collect(),
            minutes: self.saved.minutes.iter().copied().collect(),
            days: self.saved.days.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            has_pc: self.last_net.is_some(),
        }
    }
}

/// Today's date on this PC's clock, as YYYY-MM-DD.
#[cfg(windows)]
fn local_date() -> String {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let mut t = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay)
}

#[cfg(not(windows))]
fn local_date() -> String {
    let days = now_unix() / DAY;
    format!("day-{days}")
}

/// Bytes received and sent since the last sample, adapter by adapter. An adapter that just
/// came up (Wi-Fi reconnecting) or whose counter went back (reset) counts nothing this time:
/// its counter holds everything since the PC started, not the last few seconds.
fn net_delta(before: &NetCounters, now: &NetCounters) -> (u64, u64) {
    let mut down = 0;
    let mut up = 0;
    for (luid, &(d1, u1)) in now {
        if let Some(&(d0, u0)) = before.get(luid) {
            if d1 >= d0 && u1 >= u0 {
                down += d1 - d0;
                up += u1 - u0;
            }
        }
    }
    (down, up)
}

/// Bytes received and sent by each of the PC's real network adapters since they started
/// (Wi-Fi, Ethernet; not loopback or virtual adapters, which would count twice).
#[cfg(windows)]
fn net_totals() -> Option<NetCounters> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_ROW2, MIB_IF_TABLE2};
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: a plain out-pointer; Windows allocates the table (freed below with FreeMibTable).
    if unsafe { GetIfTable2(&mut table) } != 0 || table.is_null() {
        return None;
    }
    let mut counters = NetCounters::new();
    // SAFETY: Windows wrote NumEntries rows from `Table` on. The rows are reached through a raw
    // pointer to the field, not through the one-row array Rust declares (a reference to that
    // array would only cover its first row), and are read before the table is freed.
    unsafe {
        let n = (*table).NumEntries as usize;
        let first = std::ptr::addr_of!((*table).Table).cast::<MIB_IF_ROW2>();
        let rows = std::slice::from_raw_parts(first, n);
        for r in rows {
            let hardware = r.InterfaceAndOperStatusFlags._bitfield & 1 == 1;
            let up_now = r.OperStatus == 1; // IfOperStatusUp
            let loopback = r.Type == 24; // IF_TYPE_SOFTWARE_LOOPBACK
            if hardware && up_now && !loopback {
                counters.insert(r.InterfaceLuid.Value, (r.InOctets, r.OutOctets));
            }
        }
        FreeMibTable(table as *const _);
    }
    Some(counters)
}

#[cfg(not(windows))]
fn net_totals() -> Option<NetCounters> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn reads_the_pc_counters_and_date() {
        let counters = net_totals().expect("network counters");
        let down: u64 = counters.values().map(|c| c.0).sum();
        assert!(down > 0, "a connected PC has received some bytes");
        let d = local_date();
        assert_eq!(d.len(), 10);
        assert_eq!(&d[4..5], "-");
    }

    #[test]
    fn counts_only_what_adapters_moved_since_the_last_sample() {
        let before: NetCounters = [(1, (1_000, 500)), (2, (9_000, 9_000))].into();
        // Adapter 1 moved 200/50; adapter 2 was reset; adapter 3 (Wi-Fi back) just appeared
        // with everything since boot.
        let now: NetCounters = [(1, (1_200, 550)), (2, (10, 10)), (3, (5_000_000_000, 1_000_000))].into();
        assert_eq!(net_delta(&before, &now), (200, 50));
        // Next sample: adapter 3 now counts normally.
        let later: NetCounters = [(1, (1_200, 550)), (2, (20, 15)), (3, (5_000_000_100, 1_000_001))].into();
        assert_eq!(net_delta(&now, &later), (110, 6));
    }
}
