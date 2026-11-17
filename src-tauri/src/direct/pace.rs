// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Pacing Direct's connections, so a long contact list stays light on the PC and the network.
//!
//! - **Calling contacts that aren't connected** goes in rounds (`presence_loop`), a few calls
//!   at a time ([`DIALS_AT_ONCE`]) and a moment apart ([`DIAL_GAP`]), never all at once. What
//!   the person does (Connect, adding someone, opening a conversation, sending) and transfers
//!   waiting for someone don't wait for the round: they call at once.
//! - **Someone who doesn't answer** is called less and less often: 3 min, then 6, 12… up to
//!   [`MAX_BACKOFF`]. That starts over when they call us, when they're seen on the same
//!   network, when the person acts on them, and when our internet comes back. A call that fails
//!   while we have no internet isn't counted against them, nor one to someone a transfer waits
//!   for (they stay at every few minutes). My devices, favourites and people used lately are
//!   called before the others, also when their turn comes during a long round.
//! - **Quiet connections close** ([`close_idle`]): those with nothing happening for
//!   [`IDLE_AFTER`] once more than [`KEEP_FREE`] are open, and anything past [`KEEP_MAX`].
//!   Never one with a transfer, the conversation open in the window, or one of My devices; the
//!   favourites and people sent to lately are kept first. Closed that way, a contact is called
//!   again after [`RECHECK_IDLE`] (or at once when the person opens them); they can still call
//!   us at any time.
//!
//! - **"Check this contact"** ([`Check`]): **Often** lasts a while (an hour, until midnight,
//!   a week: [`often_until`]), then it's Automatic again. Meanwhile they're called every round,
//!   first, never backed off and not closed for being quiet, but still within [`KEEP_MAX`].
//!   **Automatic** is the above, and a contact with nothing happening for [`QUIET_AFTER_MS`]
//!   isn't called in the background at all any more ([`gone_quiet`]; never a favourite nor
//!   someone who can send without asking: they're kept first, like favourites). My devices are
//!   Often unless set otherwise, and always stay connected. **Only when I ask** is never called
//!   in the background. Either way, they can still call us, the person's actions still call
//!   at once, and a transfer waiting for them is still called.
//!
//! Everything here is plain bookkeeping over a clock passed in, so the tests can play hours of
//! it with a thousand contacts in a moment.

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub use super::store::Check;
use super::store::Contact;

/// Automatic stops calling a contact in the background after this long with nothing happening
/// with them (ms).
pub const QUIET_AFTER_MS: u64 = 30 * 24 * 3600 * 1000;

/// One of My devices (or they say this PC is theirs): Often unless set otherwise, and their
/// connection is never closed (outside [`KEEP_MAX`]).
pub fn mine(c: &Contact) -> bool {
    c.device || c.their_device
}

/// How Pako checks this contact at `now` (ms): their choice, Often only until it ends (then the
/// choice it had before), and
/// Automatic meaning Often for My devices.
pub fn check_of(c: &Contact, now: u64) -> Check {
    let chosen = match c.check {
        Check::Often if !c.check_until.is_some_and(|t| now < t) => super::store::back_from_often(c.check_before),
        x => x,
    };
    match chosen {
        Check::Auto if mine(c) => Check::Often,
        x => x,
    }
}

/// Called every round, first, never backed off, not closed for being quiet.
pub fn often(c: &Contact, now: u64) -> bool {
    check_of(c, now) == Check::Often
}

/// Called ahead of the others, let in when the connections are full, closed only past
/// [`KEEP_MAX`]: My devices, Often, favourites and people who can send without asking. (The
/// node adds people used lately.)
pub fn kept_first(c: &Contact, now: u64) -> bool {
    mine(c) || often(c, now) || c.favourite || c.auto_accept
}

/// Automatic, not a favourite nor someone who can send without asking, and nothing happened
/// with them for [`QUIET_AFTER_MS`] (`now` in ms): not called in the background any more until
/// something happens.
pub fn gone_quiet(c: &Contact, now: u64) -> bool {
    check_of(c, now) == Check::Auto && !c.favourite && !c.auto_accept && c.last_activity.is_some_and(|t| now.saturating_sub(t) >= QUIET_AFTER_MS)
}

/// Called by the round at all (their turn still decides when): not "only when I ask", not
/// gone quiet. (A transfer waiting for them is called whatever this says: see the node.)
pub fn in_round(c: &Contact, now: u64) -> bool {
    !c.declined && check_of(c, now) != Check::Ask && !gone_quiet(c, now)
}

/// "Often for…": how long.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OftenFor {
    Hour,
    /// Until midnight (the PC's time).
    Today,
    Week,
}

/// The longest Often (ms): a week. A saved end further away than this from now is brought back
/// to it (`store::settle_checks`).
pub const OFTEN_LONGEST_MS: u64 = 7 * 24 * 3600 * 1000;

/// When Often picked at `now` (ms) ends (ms): an hour later, at the next midnight in `tz`, or
/// a week later.
pub fn often_until<Tz: chrono::TimeZone>(span: OftenFor, now: u64, tz: &Tz) -> u64 {
    const HOUR: u64 = 3600 * 1000;
    match span {
        OftenFor::Hour => now + HOUR,
        OftenFor::Week => now + OFTEN_LONGEST_MS,
        OftenFor::Today => {
            let midnight = tz.timestamp_millis_opt(now as i64).single().and_then(|t| {
                let next = t.date_naive().succ_opt()?.and_hms_opt(0, 0, 0)?;
                // (A clock change at midnight: the first moment of the next day.)
                tz.from_local_datetime(&next).earliest().or_else(|| tz.from_local_datetime(&(next + chrono::Duration::hours(1))).earliest())
            });
            midnight.map_or(now + 24 * HOUR, |t| t.timestamp_millis() as u64)
        }
    }
}

/// Calls of a round in flight at once.
pub const DIALS_AT_ONCE: usize = 8;
/// The least time between two calls of a round being started.
pub const DIAL_GAP: Duration = Duration::from_millis(250);
/// The longest wait between two calls to someone who never answers.
pub const MAX_BACKOFF: Duration = Duration::from_secs(60 * 60);
/// A connection with nothing happening this long may close.
pub const IDLE_AFTER: Duration = Duration::from_secs(10 * 60);
/// Up to this many connections stay open however quiet they are (a small list never closes
/// any: it feels as before).
pub const KEEP_FREE: usize = 16;
/// Never more open than this (transfers, the open conversation and My devices aside): past it
/// the least recently used close after [`IDLE_OVER_CAP`].
pub const KEEP_MAX: usize = 64;
pub const IDLE_OVER_CAP: Duration = Duration::from_secs(60);
/// Someone whose connection closed for being quiet is called again after this.
pub const RECHECK_IDLE: Duration = Duration::from_secs(60 * 60);
/// "Used lately": a transfer with them ended within this (ms).
pub const RECENT_USE_MS: u64 = 7 * 24 * 3600 * 1000;
/// Close code of a connection closed for being quiet (the other Pako then doesn't call back at
/// once either; older Pakos just see it closed).
pub const IDLE_CODE: u32 = 4;

/// Whether the round may call someone now, with `open` connections open or being made: always
/// for My devices, favourites and people used lately (`wanted`); the others wait while the
/// connections are at [`KEEP_MAX`] (they keep their turn, and get it once some close).
pub fn room_for(open: usize, wanted: bool) -> bool {
    wanted || open < KEEP_MAX
}

/// Each contact's next call.
#[derive(Default)]
pub struct Tries {
    map: HashMap<String, Try>,
}

struct Try {
    /// Calls in a row that weren't answered.
    fails: u32,
    next: Instant,
    /// Closed for being quiet (they were there), not unreachable.
    idle: bool,
}

/// How long to wait after the `fails`-th call in a row went unanswered: `round`, doubled for
/// each more, up to [`MAX_BACKOFF`].
pub fn backoff(fails: u32, round: Duration) -> Duration {
    let doubled = round.saturating_mul(1u32 << fails.saturating_sub(1).min(16));
    doubled.min(MAX_BACKOFF.max(round))
}

/// Up to a tenth more, always the same for the same contact and count: calls that began
/// together drift apart instead of coming back together every time.
fn jitter(id: &str, n: u32, d: Duration) -> Duration {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in id.bytes().chain(n.to_le_bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    d + d.mul_f64((h % 1000) as f64 / 10_000.0)
}

impl Tries {
    /// Their turn in the round (never tried, or their wait is over).
    pub fn due(&self, id: &str, now: Instant) -> bool {
        self.map.get(id).is_none_or(|t| t.next <= now)
    }

    /// A call didn't get through. `uncounted`: not counted against them (we have no internet
    /// ourselves, or a transfer waits for them): tried again next round, the count unchanged.
    pub fn failed(&mut self, id: &str, now: Instant, uncounted: bool, round: Duration) {
        let t = self.map.entry(id.to_string()).or_insert(Try { fails: 0, next: now, idle: false });
        t.idle = false;
        if uncounted {
            t.next = now + round;
        } else {
            t.fails = t.fails.saturating_add(1);
            t.next = now + jitter(id, t.fails, backoff(t.fails, round));
        }
    }

    /// Their turn now, the count started over (they called us, were seen on the network, the
    /// person acted on them).
    pub fn reset(&mut self, id: &str) {
        self.map.remove(id);
    }

    /// Everyone's turn now (our internet came back).
    pub fn reset_all(&mut self) {
        self.map.clear();
    }

    /// The connection ended (not for being quiet): called again next round, as before.
    pub fn lost(&mut self, id: &str, now: Instant, round: Duration) {
        match self.map.get(id) {
            // Closed for being quiet just now: that wait stands.
            Some(t) if t.idle && t.next > now => {}
            _ => {
                self.map.insert(id.to_string(), Try { fails: 0, next: now + round, idle: false });
            }
        }
    }

    /// Closed for being quiet (by either side): called again after [`RECHECK_IDLE`].
    pub fn rested(&mut self, id: &str, now: Instant) {
        self.map.insert(id.to_string(), Try { fails: 0, next: now + jitter(id, 0, RECHECK_IDLE), idle: true });
    }

    /// Closed for being quiet and not found unreachable since.
    pub fn is_idle(&self, id: &str) -> bool {
        self.map.get(id).is_some_and(|t| t.idle)
    }

    #[cfg(test)]
    pub fn fails(&self, id: &str) -> u32 {
        self.map.get(id).map_or(0, |t| t.fails)
    }

    #[cfg(test)]
    pub fn next(&self, id: &str) -> Option<Instant> {
        self.map.get(id).map(|t| t.next)
    }
}

/// One open connection, for [`close_idle`].
pub struct Open<'a> {
    pub id: &'a str,
    /// Last time something happened on it.
    pub used: Instant,
    /// Never closed: a transfer with them, their conversation open, one of My devices.
    pub held: bool,
    /// Kept first: a favourite, or used lately.
    pub wanted: bool,
}

/// The connections to close now for being quiet (see the module's notes).
pub fn close_idle<'a>(open: &[Open<'a>], now: Instant) -> Vec<&'a str> {
    let held = open.iter().filter(|o| o.held).count();
    let mut rest: Vec<&Open> = open.iter().filter(|o| !o.held).collect();
    // The ones to keep first: wanted, then the most recently used.
    rest.sort_by(|a, b| b.wanted.cmp(&a.wanted).then(b.used.cmp(&a.used)));
    let room = KEEP_MAX.saturating_sub(held);
    let mut out = Vec::new();
    // Quiet ones beyond the free room close, last in the order first; wanted ones only past
    // the cap.
    let mut kept = held + rest.len();
    for (i, o) in rest.iter().enumerate().rev() {
        let quiet = now.saturating_duration_since(o.used);
        let over_cap = i >= room && quiet >= IDLE_OVER_CAP;
        let idle = !o.wanted && kept > held.max(KEEP_FREE) && quiet >= IDLE_AFTER;
        if over_cap || idle {
            out.push(o.id);
            kept -= 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    //! A thousand contacts played over hours of a made-up clock: how many calls go out, how
    //! many connections stay open.
    use super::*;
    use std::collections::{HashMap, HashSet, VecDeque};

    const ROUND: Duration = Duration::from_secs(180);
    const TICK: Duration = Duration::from_secs(30);
    const SEC: Duration = Duration::from_secs(1);

    #[test]
    fn backoff_doubles_up_to_the_cap() {
        let mins: Vec<u64> = (1..=8).map(|n| backoff(n, ROUND).as_secs() / 60).collect();
        assert_eq!(mins, [3, 6, 12, 24, 48, 60, 60, 60]);
        assert_eq!(backoff(1000, ROUND), MAX_BACKOFF);
    }

    #[test]
    fn backoff_grows_and_starts_over() {
        let t0 = Instant::now();
        let mut tries = Tries::default();
        let id = "far";
        assert!(tries.due(id, t0));
        let mut now = t0;
        let mut waits = Vec::new();
        for _ in 0..7 {
            tries.failed(id, now, false, ROUND);
            let next = tries.next(id).unwrap();
            waits.push((next - now).as_secs());
            assert!(!tries.due(id, now + SEC));
            now = next;
        }
        // 3, 6, 12, 24, 48, 60, 60 minutes, each up to a tenth more.
        for (w, base) in waits.iter().zip([3u64, 6, 12, 24, 48, 60, 60]) {
            assert!(*w >= base * 60 && *w <= base * 66, "{w} s for {base} min");
        }
        assert_eq!(tries.fails(id), 7);
        // No internet on our side: not counted, tried next round.
        tries.failed(id, now, true, ROUND);
        assert_eq!(tries.fails(id), 7);
        assert_eq!(tries.next(id), Some(now + ROUND));
        // They call us / are seen on the network / the person presses Connect: their turn now.
        tries.reset(id);
        assert!(tries.due(id, now));
        assert_eq!(tries.fails(id), 0);
        tries.failed(id, now, false, ROUND);
        assert!(tries.next(id).unwrap() - now < ROUND + ROUND / 9, "starts over at 3 minutes");
        // Our internet comes back: everyone's turn.
        tries.reset_all();
        assert!(tries.due(id, now));
    }

    #[test]
    fn closed_for_quiet_waits_an_hour_and_isnt_unreachable() {
        let now = Instant::now();
        let mut tries = Tries::default();
        tries.rested("q", now);
        assert!(tries.is_idle("q"));
        // The connection's end comes in after: the hour stands.
        tries.lost("q", now + SEC, ROUND);
        assert!(tries.next("q").unwrap() >= now + RECHECK_IDLE);
        assert!(!tries.due("q", now + ROUND * 2));
        assert!(tries.due("q", now + RECHECK_IDLE + RECHECK_IDLE / 9));
        // Not there when called again: an ordinary offline contact from then on.
        tries.failed("q", now + RECHECK_IDLE * 2, false, ROUND);
        assert!(!tries.is_idle("q"));
        // A connection lost otherwise: next round, as before.
        tries.lost("l", now, ROUND);
        assert_eq!(tries.next("l"), Some(now + ROUND));
    }

    fn open(id: &str, used: Instant, held: bool, wanted: bool) -> Open<'_> {
        Open { id, used, held, wanted }
    }

    #[test]
    fn a_short_list_never_closes() {
        let now = Instant::now();
        let ids: Vec<String> = (0..KEEP_FREE).map(|i| format!("c{i}")).collect();
        let list: Vec<Open> = ids.iter().map(|id| open(id, now - Duration::from_secs(86400), false, false)).collect();
        assert!(close_idle(&list, now).is_empty(), "a day quiet, still open");
    }

    #[test]
    fn quiet_ones_close_past_the_free_room_and_nothing_held_ever() {
        let now = Instant::now();
        let ids: Vec<String> = (0..100).map(|i| format!("c{i:03}")).collect();
        // c000-c009 held (transfers, devices), c010-c029 wanted, the rest neither; all quiet
        // for an hour except c090-c099 (busy a minute ago... not quite: 30 s ago).
        let list: Vec<Open> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| open(id, if i >= 90 { now - 30 * SEC } else { now - RECHECK_IDLE }, i < 10, (10..30).contains(&i)))
            .collect();
        let closed: HashSet<&str> = close_idle(&list, now).into_iter().collect();
        assert!(ids[..30].iter().all(|id| !closed.contains(id.as_str())), "held and wanted stay");
        assert!(ids[30..90].iter().all(|id| closed.contains(id.as_str())), "the quiet rest close");
        assert!(ids[90..].iter().all(|id| !closed.contains(id.as_str())), "busy ones stay");
        assert!(list.len() - closed.len() <= KEEP_MAX);

        // 100 wanted, quiet past a minute: only the most recent fill the room up to the cap.
        let list: Vec<Open> = ids.iter().enumerate().map(|(i, id)| open(id, now - Duration::from_secs(120 + i as u64), i < 5, true)).collect();
        let closed = close_idle(&list, now);
        assert_eq!(list.len() - closed.len(), KEEP_MAX);
        assert!(ids[..KEEP_MAX].iter().all(|id| !closed.contains(&id.as_str())), "the most recently used stay");
    }

    // ---------- a thousand contacts over hours ----------

    /// A contact in the play: online or not, wanted or not, and the contact itself (how it's
    /// checked, its last activity).
    struct Who {
        id: String,
        online: bool,
        wanted: bool,
        c: Contact,
    }

    /// The play's wall clock (ms) at its start: 1 January 2026.
    const BASE_MS: u64 = 1_767_225_600_000;
    const DAY_MS: u64 = 24 * 3600 * 1000;

    /// What the node does each tick, played over a made-up clock. Calls last 15 s when nobody
    /// answers and 1 s when they do; nothing happens on a connection after it opens.
    struct Play {
        people: Vec<Who>,
        tries: Tries,
        live: HashMap<String, Instant>,
        queue: VecDeque<String>,
        /// The first line (devices, Often, favourites, used lately), as in the node.
        lead: VecDeque<String>,
        /// (ends, who, a round call: its failure counts)
        flying: Vec<(Instant, String, bool)>,
        last_start: Option<Instant>,
        /// Calls started, by the minute.
        started: Vec<Instant>,
        most_flying: usize,
        most_live: usize,
        /// Most open right after a sweep (the cap is kept by closing).
        most_swept: usize,
        /// Contacts a transfer waits for since this time (called every 30 s for an hour).
        waiting: HashMap<String, Instant>,
        waiting_calls: HashMap<String, Vec<Instant>>,
        /// The start, for the wall clock.
        t0: Instant,
        /// Round calls started, by contact.
        calls_to: HashMap<String, Vec<Instant>>,
    }

    impl Play {
        fn new(n: usize, online: usize, wanted: usize) -> Self {
            let people = (0..n)
                .map(|i| {
                    let id = format!("p{i:04}");
                    // Something happened with everyone a day before the start.
                    let c = Contact { id: id.clone(), name: id.clone(), trusted: true, favourite: i < wanted, last_activity: Some(BASE_MS - DAY_MS), ..Default::default() };
                    Who { id, online: i < online, wanted: i < wanted, c }
                })
                .collect();
            Play {
                people,
                tries: Tries::default(),
                live: HashMap::new(),
                queue: VecDeque::new(),
                lead: VecDeque::new(),
                flying: Vec::new(),
                last_start: None,
                started: Vec::new(),
                most_flying: 0,
                most_live: 0,
                most_swept: 0,
                waiting: HashMap::new(),
                waiting_calls: HashMap::new(),
                t0: Instant::now(),
                calls_to: HashMap::new(),
            }
        }

        /// The wall clock (ms) at `now`.
        fn ms(&self, now: Instant) -> u64 {
            BASE_MS + now.saturating_duration_since(self.t0).as_millis() as u64
        }

        fn set(&mut self, range: std::ops::Range<usize>, f: impl Fn(&mut Contact)) {
            for w in &mut self.people[range] {
                f(&mut w.c);
            }
        }

        /// Kept first and let past the cap: wanted (used lately), or `kept_first` (My devices,
        /// Often, favourites, people who can send without asking).
        fn first(&self, id: &str, now: Instant) -> bool {
            let w = self.who(id);
            w.wanted || kept_first(&w.c, self.ms(now))
        }

        fn who(&self, id: &str) -> &Who {
            self.people.iter().find(|p| p.id == id).unwrap()
        }

        /// The round: who's due joins the queue (as `presence_loop` does each tick).
        fn round(&mut self, now: Instant) {
            let ms = self.ms(now);
            for p in &self.people {
                let busy = self.live.contains_key(&p.id) || self.queue.contains(&p.id) || self.lead.contains(&p.id) || self.flying.iter().any(|(_, id, _)| *id == p.id);
                if !busy && in_round(&p.c, ms) && self.tries.due(&p.id, now) {
                    if p.wanted || kept_first(&p.c, ms) { self.lead.push_back(p.id.clone()) } else { self.queue.push_back(p.id.clone()) }
                }
            }
            // Waiting transfers: called at once, every tick, past the round (in their first hour).
            let wait: Vec<String> = self.waiting.iter().filter(|(_, since)| now - **since < Duration::from_secs(3600)).map(|(id, _)| id.clone()).collect();
            for id in wait {
                if !self.live.contains_key(&id) {
                    self.waiting_calls.entry(id.clone()).or_default().push(now);
                    self.call(id, now, false);
                }
            }
        }

        fn call(&mut self, id: String, now: Instant, round: bool) {
            let ends = now + if self.who(&id).online { SEC } else { 15 * SEC };
            if round {
                self.started.push(now);
                self.calls_to.entry(id.clone()).or_default().push(now);
            }
            self.flying.push((ends, id, round));
        }

        /// One second: calls finish, the round's worker starts what it may.
        fn second(&mut self, now: Instant) {
            let (done, still): (Vec<_>, Vec<_>) = std::mem::take(&mut self.flying).into_iter().partition(|(end, _, _)| *end <= now);
            self.flying = still;
            for (_, id, round) in done {
                if self.who(&id).online {
                    self.tries.reset(&id);
                    self.live.insert(id, now);
                } else if round {
                    let often = often(&self.who(&id).c, self.ms(now));
                    self.tries.failed(&id, now, often, ROUND);
                }
            }
            let round_flying = self.flying.iter().filter(|f| f.2).count();
            if round_flying < DIALS_AT_ONCE && self.last_start.is_none_or(|t| now - t >= DIAL_GAP) {
                // (One per second here: the gap is shorter, the play's step isn't.)
                // Two lines, as the node: the ones kept first go ahead of the others.
                while let Some(id) = self.lead.pop_front().or_else(|| self.queue.pop_front()) {
                    if !room_for(self.live.len() + round_flying, self.first(&id, now)) {
                        continue;
                    }
                    self.last_start = Some(now);
                    self.call(id, now, true);
                    break;
                }
            }
            self.most_flying = self.most_flying.max(self.flying.iter().filter(|f| f.2).count());
            self.most_live = self.most_live.max(self.live.len());
        }

        fn sweep(&mut self, now: Instant) {
            // Held as in the node: My devices only (no transfers or open conversation here).
            let list: Vec<Open> = self.live.iter().map(|(id, used)| Open { id, used: *used, held: mine(&self.who(id).c), wanted: self.first(id, now) }).collect();
            let shut: Vec<String> = close_idle(&list, now).into_iter().map(String::from).collect();
            for id in shut {
                self.live.remove(&id);
                self.tries.rested(&id, now);
            }
            self.most_swept = self.most_swept.max(self.live.len());
        }

        fn run(&mut self, from: Instant, secs: u64) {
            for s in 0..secs {
                let now = from + Duration::from_secs(s);
                if s % TICK.as_secs() == 0 {
                    self.round(now);
                    self.sweep(now);
                }
                self.second(now);
            }
        }

        fn calls_between(&self, a: Instant, b: Instant) -> usize {
            self.started.iter().filter(|t| **t >= a && **t < b).count()
        }

        fn calls_of(&self, id: &str) -> &[Instant] {
            self.calls_to.get(id).map_or(&[], |v| v.as_slice())
        }
    }

    /// 1000 contacts, 900 never online, 100 online (30 of them favourites or used lately).
    /// Before: every 3 minutes 900 calls at once (18,000 an hour) and 100 connections open for
    /// good. Now: at most 8 at once, the first round spread over minutes, then about one call an
    /// hour per contact that doesn't answer; at most 64 connections open (here 16 + the 30
    /// wanted ones, and briefly the ones just called).
    #[test]
    fn a_thousand_contacts_stay_light() {
        let t0 = Instant::now();
        let mut p = Play::new(1000, 100, 30);
        let hours = 6;
        // The first quarter hour: everyone online answers the first round at once, and the
        // ones past the cap close a minute later.
        p.run(t0, 900);
        let first_most_live = p.most_live;
        p.most_live = 0;
        p.run(t0 + Duration::from_secs(900), hours * 3600 - 900);

        assert!(p.most_flying <= DIALS_AT_ONCE, "never more than {DIALS_AT_ONCE} calls at once ({})", p.most_flying);
        // Spread: never more than one round call started per second.
        let mut per_sec: HashMap<Instant, usize> = HashMap::new();
        for t in &p.started {
            *per_sec.entry(*t).or_default() += 1;
        }
        assert!(per_sec.values().all(|n| *n == 1));
        let first_hour = p.calls_between(t0, t0 + Duration::from_secs(3600));
        let last_hour = p.calls_between(t0 + Duration::from_secs((hours - 1) * 3600), t0 + Duration::from_secs(hours * 3600));
        eprintln!("1000 contacts (900 offline, 100 online): {first_hour} calls in the first hour, {last_hour} in the 6th; at most {} at once; at most {first_most_live} open in the first quarter hour, then at most {}, {} at the end", p.most_flying, p.most_live, p.live.len());
        // Steady: about one call an hour for each one who never answers, plus the hourly look
        // at the quiet ones that closed.
        assert!(last_hour <= 1100, "{last_hour} calls in an hour");
        assert!(first_hour < 18_000 / 4);
        // Open connections: settled at or under the cap.
        assert!(p.most_live <= KEEP_MAX, "{} open at most once settled", p.most_live);
        assert!(p.live.len() >= 30, "the wanted ones stay connected");
        for w in p.people.iter().filter(|w| w.wanted) {
            assert!(p.live.contains_key(&w.id), "{} stays connected", w.id);
        }
    }

    /// A transfer waits for someone who's offline: called every 30 s for its first hour,
    /// whatever their place in the round or their backoff.
    #[test]
    fn a_waiting_transfers_contact_is_called_every_30_seconds_for_an_hour() {
        let t0 = Instant::now();
        let mut p = Play::new(1000, 100, 30);
        let gone = "p0999".to_string();
        // Long unanswered already: far in their backoff.
        for _ in 0..10 {
            p.tries.failed(&gone, t0, false, ROUND);
        }
        p.waiting.insert(gone.clone(), t0);
        p.run(t0, 2 * 3600);
        let calls = &p.waiting_calls[&gone];
        assert_eq!(calls.len(), 120, "every 30 s for an hour");
        assert!(calls.windows(2).all(|w| w[1] - w[0] == TICK));
        assert!(calls.iter().all(|t| *t - t0 < Duration::from_secs(3600)));
    }

    /// Five contacts: the first calls go within a couple of seconds, nobody quiet is closed.
    #[test]
    fn five_contacts_feel_as_before() {
        let t0 = Instant::now();
        let mut p = Play::new(5, 3, 0);
        p.run(t0, 3);
        assert_eq!(p.started.len(), 3, "the first calls start at once");
        p.run(t0 + Duration::from_secs(3), 3 * 3600);
        assert_eq!(p.live.len(), 3, "the online ones stay connected however quiet");
    }

    // ---------- "Check this contact" ----------

    fn contact(f: impl Fn(&mut Contact)) -> Contact {
        let mut c = Contact { id: "x".into(), name: "X".into(), trusted: true, last_activity: Some(BASE_MS), ..Default::default() };
        f(&mut c);
        c
    }

    /// Automatic is Often for My devices (unless set otherwise); people who can send without
    /// asking are like favourites: kept first and never quiet, but not Often; only Automatic
    /// goes quiet; Ask is never in the round, nor someone who declined.
    #[test]
    fn the_check_modes_resolve() {
        let day = |d: u64| BASE_MS + d * DAY_MS;
        assert_eq!(check_of(&contact(|_| {}), BASE_MS), Check::Auto);
        assert_eq!(check_of(&contact(|c| c.device = true), BASE_MS), Check::Often);
        assert_eq!(check_of(&contact(|c| c.their_device = true), BASE_MS), Check::Often);
        assert_eq!(check_of(&contact(|c| c.auto_accept = true), BASE_MS), Check::Auto, "can send without asking: not Often");
        assert!(kept_first(&contact(|c| c.auto_accept = true), BASE_MS) && kept_first(&contact(|c| c.favourite = true), BASE_MS));
        assert!(!kept_first(&contact(|_| {}), BASE_MS));
        assert_eq!(check_of(&contact(|c| (c.device, c.check) = (true, Check::Ask)), BASE_MS), Check::Ask, "a choice made stands");
        let plain = contact(|_| {});
        assert!(in_round(&plain, day(29)) && !gone_quiet(&plain, day(29)));
        assert!(gone_quiet(&plain, day(30)) && !in_round(&plain, day(30)), "30 days with nothing: not called in the background");
        assert!(!gone_quiet(&contact(|c| c.favourite = true), day(400)), "favourites never go quiet");
        assert!(!gone_quiet(&contact(|c| c.auto_accept = true), day(400)), "nor people who can send without asking");
        assert!(!gone_quiet(&contact(|c| c.device = true), day(400)));
        let often = contact(|c| (c.check, c.check_until) = (Check::Often, Some(day(401))));
        assert!(!gone_quiet(&often, day(400)) && kept_first(&often, day(400)), "Often, while it lasts");
        assert!(gone_quiet(&often, day(401)), "then Automatic: 30 days with nothing, quiet");
        assert_eq!(check_of(&contact(|c| c.check = Check::Often), BASE_MS), Check::Auto, "Often with no end (an older file): Automatic");
        assert!(!in_round(&contact(|c| c.check = Check::Ask), BASE_MS), "only when you ask");
        assert!(!in_round(&contact(|c| c.declined = true), BASE_MS));
        // Something happens (a transfer, a call from them, Check now): 30 more days.
        let mut c = contact(|_| {});
        c.last_activity = Some(day(40));
        assert!(in_round(&c, day(41)));
    }

    /// Often, offline: called every round (3 minutes, never backed off); Often, online and
    /// quiet: never closed, past the free room too. Ask: never called by the round.
    #[test]
    fn often_is_called_every_round_and_never_closed_and_ask_never() {
        let t0 = Instant::now();
        let mut p = Play::new(1000, 100, 0);
        // p0000-p0039 online and Often (more than the 16 kept free), p0999 offline and Often,
        // p0998 offline and Ask, p0050 online and Ask.
        let often = |c: &mut Contact| (c.check, c.check_until) = (Check::Often, Some(BASE_MS + 7 * DAY_MS));
        p.set(0..40, often);
        p.set(999..1000, often);
        p.set(998..999, |c| c.check = Check::Ask);
        p.set(50..51, |c| c.check = Check::Ask);
        p.run(t0, 6 * 3600);
        let far = p.calls_of("p0999");
        assert!(far.len() >= 100, "{} calls in 6 h: every round", far.len());
        assert!(far.windows(2).all(|w| w[1] - w[0] <= ROUND + TICK + 15 * SEC), "never backed off");
        for i in 0..40 {
            let id = format!("p{i:04}");
            assert!(p.live.contains_key(&id), "{id} stays connected, quiet for hours");
            assert_eq!(p.calls_of(&id).len(), 1, "{id}: called once, never closed");
        }
        assert!(p.calls_of("p0998").is_empty() && p.calls_of("p0050").is_empty(), "Ask: never called in the background");
        assert!(p.live.len() <= KEEP_MAX);
        // An ordinary one who never answers: backed off to once an hour.
        assert!(p.calls_of("p0997").len() <= 12, "{}", p.calls_of("p0997").len());
    }

    /// Automatic: called until 30 days pass with nothing happening, then not at all; something
    /// happening starts the 30 days over.
    #[test]
    fn automatic_goes_quiet_after_30_days_and_wakes_on_activity() {
        let t0 = Instant::now();
        let mut p = Play::new(3, 0, 0);
        // p0000: its 30 days end 20 minutes in; p0001: a favourite, as old; p0002: fresh.
        p.set(0..2, |c| c.last_activity = Some(BASE_MS + 20 * 60 * 1000 - QUIET_AFTER_MS));
        p.set(1..2, |c| c.favourite = true);
        p.run(t0, 3 * 3600);
        let quiet_at = p.t0 + Duration::from_secs(20 * 60);
        assert!(!p.calls_of("p0000").is_empty() && p.calls_of("p0000").iter().all(|t| *t < quiet_at), "called, then never after its 30 days");
        assert!(p.calls_of("p0001").iter().any(|t| *t > quiet_at + Duration::from_secs(3600)), "favourites never go quiet");
        assert!(p.calls_of("p0002").iter().any(|t| *t > quiet_at + Duration::from_secs(3600)));
        // Check now (or a transfer, or them calling): called again.
        let now = t0 + Duration::from_secs(3 * 3600);
        let ms = p.ms(now);
        p.set(0..1, |c| c.last_activity = Some(ms));
        p.tries.reset("p0000");
        let before = p.calls_of("p0000").len();
        p.run(now, 60);
        assert_eq!(p.calls_of("p0000").len(), before + 1);
    }

    /// 1000 contacts, most of them gone quiet (nothing for 40 days): 870 never online, 70
    /// online; plus 30 online favourites and 30 offline ones still active. With everyone
    /// active, the 6th hour makes ~1000 calls; with most quiet, close to none: only the 30
    /// active ones who don't answer (about one an hour each).
    #[test]
    fn a_thousand_mostly_quiet_contacts_make_almost_no_calls() {
        let t0 = Instant::now();
        let hours = 6;
        let hour = |p: &Play, h: u64| p.calls_between(p.t0 + Duration::from_secs(h * 3600), p.t0 + Duration::from_secs((h + 1) * 3600));
        // Before: everyone active.
        let mut all = Play::new(1000, 100, 30);
        all.run(t0, hours * 3600);
        let before = (hour(&all, 0), hour(&all, hours - 1));
        // Now: most gone quiet.
        let mut p = Play::new(1000, 100, 30);
        p.set(30..100, |c| c.last_activity = Some(BASE_MS - 40 * DAY_MS));
        p.set(100..970, |c| c.last_activity = Some(BASE_MS - 40 * DAY_MS));
        p.run(t0, hours * 3600);
        let now = (hour(&p, 0), hour(&p, hours - 1));
        eprintln!(
            "1000 contacts, 940 gone quiet (870 offline, 70 online): {} calls in the first hour, {} in the 6th (all active: {} and {}); {} open at the end",
            now.0,
            now.1,
            before.0,
            before.1,
            p.live.len()
        );
        assert!(p.people[30..970].iter().all(|w| p.calls_of(&w.id).is_empty()), "the quiet ones are never called");
        // The 30 active offline ones, about once an hour each.
        assert!(now.1 <= 40, "{} calls in the 6th hour", now.1);
        assert!(now.1 * 20 < before.1, "{} vs {}", now.1, before.1);
        for w in p.people.iter().filter(|w| w.wanted) {
            assert!(p.live.contains_key(&w.id), "{} stays connected", w.id);
        }
    }

    // ---------- "Often for…" and people who can send without asking ----------

    /// "Often for…" ends after an hour, at midnight (the PC's time), or after a week.
    #[test]
    fn often_ends_at_its_time() {
        use chrono::{TimeZone, Timelike};
        // Madagascar (UTC+3), no clock changes.
        let tz = chrono::FixedOffset::east_opt(3 * 3600).unwrap();
        let at = |y, mo, d, h, mi| tz.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap().timestamp_millis() as u64;
        let now = at(2026, 11, 17, 18, 40);
        assert_eq!(often_until(OftenFor::Hour, now, &tz), at(2026, 11, 17, 19, 40));
        assert_eq!(often_until(OftenFor::Week, now, &tz), at(2026, 11, 24, 18, 40));
        assert_eq!(often_until(OftenFor::Today, now, &tz), at(2026, 11, 18, 0, 0), "Today: until midnight there, not in UTC");
        assert_eq!(often_until(OftenFor::Today, at(2026, 11, 17, 23, 59) + 59_999, &tz), at(2026, 11, 18, 0, 0));
        assert_eq!(often_until(OftenFor::Today, at(2026, 11, 18, 0, 0), &tz), at(2026, 11, 19, 0, 0), "picked at midnight: the whole day");
        assert_eq!(often_until(OftenFor::Today, at(2026, 12, 31, 21, 0), &tz), at(2027, 1, 1, 0, 0));
        // The PC's own clock: its next midnight, within a day (an hour more for a clock change).
        let real = super::super::node::now_ms();
        let end = often_until(OftenFor::Today, real, &chrono::Local);
        assert!(end > real && end <= real + 25 * 3600 * 1000);
        let t = chrono::Local.timestamp_millis_opt(end as i64).unwrap();
        assert_eq!((t.hour(), t.minute(), t.second()), (0, 0, 0));
    }

    /// Often for an hour, someone offline: called every round during that hour, then Automatic
    /// by itself (backed off like anyone); the saved choice goes back to Automatic too.
    #[test]
    fn often_for_an_hour_runs_out_back_to_automatic() {
        let t0 = Instant::now();
        let hour_ms = 3600 * 1000;
        let mut p = Play::new(1000, 100, 0);
        p.set(999..1000, |c| (c.check, c.check_until, c.last_activity) = (Check::Often, Some(BASE_MS + hour_ms), Some(BASE_MS)));
        let c = p.who("p0999").c.clone();
        assert_eq!(check_of(&c, BASE_MS + hour_ms - 1), Check::Often);
        assert_eq!(check_of(&c, BASE_MS + hour_ms), Check::Auto, "its hour is up");
        p.run(t0, 4 * 3600);
        let end = t0 + Duration::from_secs(3600);
        let calls = p.calls_of("p0999");
        let during = calls.iter().filter(|t| **t < end).count();
        let after = calls.iter().filter(|t| **t >= end).count();
        assert!(during >= 15, "{during} calls in its hour: every round");
        assert!(after <= 8, "{after} calls in the 3 hours after: backed off as Automatic");
        assert!(p.calls_of("p0998").len() <= 12, "an ordinary one, for comparison");
        // The saved choice: Automatic again once the time is up, the end forgotten.
        let mut list = vec![c];
        assert!(!super::super::store::settle_checks(&mut list, BASE_MS + hour_ms - 1));
        assert_eq!(list[0].check, Check::Often);
        assert!(super::super::store::settle_checks(&mut list, BASE_MS + hour_ms));
        assert_eq!((list[0].check, list[0].check_until), (Check::Auto, None));
    }

    /// Often, and people who can send without asking, count toward the cap: 50 of each online
    /// and quiet, at most 64 open after each sweep (the ones past it close a minute after).
    #[test]
    fn often_and_auto_accept_stay_within_the_cap() {
        let t0 = Instant::now();
        let mut p = Play::new(1000, 100, 0);
        p.set(0..50, |c| (c.check, c.check_until) = (Check::Often, Some(BASE_MS + 7 * DAY_MS)));
        p.set(50..100, |c| c.auto_accept = true);
        p.run(t0, 3 * 3600);
        assert!(p.most_swept <= KEEP_MAX, "{} open after a sweep", p.most_swept);
        assert!(p.most_swept >= KEEP_MAX - 4, "{}: the room is used", p.most_swept);
        // Not closed for being quiet below the cap: with 40 of them, all stay connected.
        let mut p = Play::new(1000, 100, 0);
        p.set(0..20, |c| (c.check, c.check_until) = (Check::Often, Some(BASE_MS + 7 * DAY_MS)));
        p.set(20..40, |c| c.auto_accept = true);
        p.run(t0, 3 * 3600);
        for i in 0..40 {
            let id = format!("p{i:04}");
            assert!(p.live.contains_key(&id) && p.calls_of(&id).len() == 1, "{id} stays connected, quiet for hours");
        }
    }

    /// The audit's case (2026-11-18): 1000 contacts, 100 of whom can send without asking and
    /// are offline (nothing with them for 40 days). They're kept first and never go quiet, but
    /// backed off like anyone: they no longer fill every round, and the others still get called.
    #[test]
    fn a_thousand_contacts_with_100_offline_auto_accept_stay_light() {
        let t0 = Instant::now();
        let hours = 6;
        let mut p = Play::new(1000, 100, 30);
        p.set(900..1000, |c| (c.auto_accept, c.last_activity) = (true, Some(BASE_MS - 40 * DAY_MS)));
        p.run(t0, 900);
        let first_most_live = p.most_live;
        p.most_live = 0;
        p.run(t0 + Duration::from_secs(900), hours * 3600 - 900);
        let first_hour = p.calls_between(t0, t0 + Duration::from_secs(3600));
        let last_hour = p.calls_between(t0 + Duration::from_secs((hours - 1) * 3600), t0 + Duration::from_secs(hours * 3600));
        let auto: Vec<usize> = p.people[900..].iter().map(|w| p.calls_of(&w.id).len()).collect();
        let others: Vec<usize> = p.people[100..900].iter().map(|w| p.calls_of(&w.id).len()).collect();
        eprintln!(
            "1000 contacts (100 online, 800 offline, 100 offline that can send without asking): {first_hour} calls in the first hour, {last_hour} in the 6th; at most {} at once; at most {first_most_live} open in the first quarter hour, then at most {}, {} after a sweep, {} at the end; each auto-accept one called {}-{} times in 6 h, each other offline one {}-{}",
            p.most_flying,
            p.most_live,
            p.most_swept,
            p.live.len(),
            auto.iter().min().unwrap(),
            auto.iter().max().unwrap(),
            others.iter().min().unwrap(),
            others.iter().max().unwrap(),
        );
        assert!(p.most_flying <= DIALS_AT_ONCE);
        assert!(last_hour <= 1100, "{last_hour} calls in the 6th hour");
        assert!(first_hour < 18_000 / 4);
        assert!(p.most_swept <= KEEP_MAX);
        // Never quiet (still called after hours), but backed off: about once an hour, not every
        // round (that was 120 calls each in 6 h).
        assert!(auto.iter().all(|n| (5..=12).contains(n)), "{auto:?}");
        // The others aren't crowded out.
        assert!(others.iter().all(|n| *n >= 5), "{:?}", others.iter().min());
        for w in p.people.iter().filter(|w| w.wanted) {
            assert!(p.live.contains_key(&w.id), "{} stays connected", w.id);
        }
    }
}
