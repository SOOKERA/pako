// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Videos read ahead: the player's Download button in the browser asks Pako to start reading the
//! video (yt-dlp's look at it, seconds on YouTube) as soon as you open its quality menu, before you
//! pick a quality. The read doesn't depend on the quality, so the quality you pick a moment later
//! uses that read (still going, or done) instead of starting another one.
//!
//! In memory only, for a short time (`KEEP`), a few at most (`MAX_KEPT`). A read is for one link
//! with one login (the site's cookies) in one kind of window: a private window's read is never
//! used by a normal window's send, nor the other way round, nor a read made with another login.
//! A read nobody asks for stops and is dropped as soon as its menu closes without a pick
//! (`cancel`), when newer reads push it out, or when it expires: its yt-dlp is ended then (the
//! read is dropped, and the video tool's process is started to end when dropped).
//!
//! Reads ahead take their turn among their own few permits, never the sends' (see `prepare`):
//! a quality you pick never waits behind a read nobody may ask for.

use std::collections::hash_map::RandomState;
use std::future::Future;
use std::hash::BuildHasher;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{Semaphore, oneshot};

use crate::extension::SiteCookie;

/// How long a read waits for its quality to be picked.
pub(crate) const KEEP: Duration = Duration::from_secs(120);
/// Reads kept at once (the oldest gives way).
pub(crate) const MAX_KEPT: usize = 4;

/// Which read a send may use: the same link, the same login, the same kind of window.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Key {
    url: String,
    private: bool,
    /// The login the read was made with: a keyed hash of the site's cookies (not the cookies
    /// themselves, which aren't kept here).
    login: u64,
}

/// A key of its own for this run of Pako (the hash can't be worked out from outside).
static LOGIN_HASH: LazyLock<RandomState> = LazyLock::new(RandomState::new);

/// Cookies a site renews by itself every few minutes or on clicks (YouTube's SIDCC and PSIDTS
/// families, its short-lived "ST-…" ones): the same login with or without their new values.
fn renews_itself(name: &str) -> bool {
    name.ends_with("SIDCC") || name.ends_with("PSIDTS") || name.ends_with("PSIDRTS") || name.starts_with("ST-")
}

/// The login a set of cookies stands for: their domain, path, name and value, in any order;
/// not when each expires (sites push that back on every visit), nor the cookies that renew
/// themselves.
fn login(cookies: &[SiteCookie]) -> u64 {
    let mut parts: Vec<(&str, &str, &str, &str)> =
        cookies.iter().filter(|c| !renews_itself(&c.name)).map(|c| (c.domain.as_str(), c.path.as_str(), c.name.as_str(), c.value.as_str())).collect();
    parts.sort_unstable();
    parts.dedup();
    LOGIN_HASH.hash_one(&parts)
}

impl Key {
    pub(crate) fn new(url: &str, private: bool, cookies: &[SiteCookie]) -> Key {
        // (A private window never sends cookies; none count for it either way.)
        Key { url: url.to_string(), private, login: if private { login(&[]) } else { login(cookies) } }
    }
}

struct Entry<T> {
    key: Key,
    at: Instant,
    /// The read has its turn (yt-dlp runs): a send waits for it. Before that, a send reads on
    /// its own (it never waits behind reads ahead).
    started: Arc<AtomicBool>,
    result: oneshot::Receiver<T>,
}

/// The reads started ahead, each until its send takes it, its menu closes, or it expires.
/// Dropping an entry (its receiver) stops its read.
pub(crate) struct Reads<T> {
    kept: Mutex<Vec<Entry<T>>>,
    keep: Duration,
    cap: usize,
}

impl<T: Send + 'static> Reads<T> {
    pub(crate) const fn new(keep: Duration, cap: usize) -> Self {
        Reads { kept: Mutex::new(Vec::new()), keep, cap }
    }

    fn prune(kept: &mut Vec<Entry<T>>, keep: Duration, now: Instant) {
        kept.retain(|e| now.saturating_duration_since(e.at) < keep);
    }

    fn prune_now(&self) {
        Self::prune(&mut self.kept.lock().unwrap(), self.keep, Instant::now());
    }

    /// Room for a read of `key`: None when it is being read or was read already (kept). The
    /// oldest read gives way when there are `cap` already.
    fn begin(&self, key: Key, now: Instant) -> Option<(oneshot::Sender<T>, Arc<AtomicBool>)> {
        let mut kept = self.kept.lock().unwrap();
        Self::prune(&mut kept, self.keep, now);
        if kept.iter().any(|e| e.key == key) {
            return None;
        }
        while kept.len() >= self.cap.max(1) {
            kept.remove(0);
        }
        let (tx, rx) = oneshot::channel();
        let started = Arc::new(AtomicBool::new(false));
        kept.push(Entry { key, at: now, started: started.clone(), result: rx });
        Some((tx, started))
    }

    /// The read kept for `key` (taken: a read serves one send), if it hasn't expired and has its
    /// turn already. One still waiting for its turn is dropped: the send reads on its own.
    fn take(&self, key: &Key, now: Instant) -> Option<oneshot::Receiver<T>> {
        let mut kept = self.kept.lock().unwrap();
        Self::prune(&mut kept, self.keep, now);
        let i = kept.iter().position(|e| e.key == *key)?;
        let e = kept.remove(i);
        e.started.load(Ordering::SeqCst).then_some(e.result)
    }

    /// The quality menu closed without a pick: the reads of `url` from that kind of window stop
    /// and go (a read a send took already carries on). How many went.
    pub(crate) fn cancel(&self, url: &str, private: bool) -> usize {
        let mut kept = self.kept.lock().unwrap();
        let before = kept.len();
        kept.retain(|e| !(e.key.url == url && e.key.private == private));
        before - kept.len()
    }

    /// Reads kept now (expired ones aside).
    #[cfg(test)]
    fn len(&self, now: Instant) -> usize {
        let mut kept = self.kept.lock().unwrap();
        Self::prune(&mut kept, self.keep, now);
        kept.len()
    }

    /// Starts reading `key` ahead: the task to run (None: it is being read or was read already).
    /// The task waits its turn among `permits` (reads ahead only: never the sends' permits),
    /// and stops (ending yt-dlp, freeing its turn) as soon as nobody may want it any more: its
    /// menu closed (`cancel`), newer reads pushed it out, or it expired untaken. A finished
    /// read untaken is dropped when it expires.
    pub(crate) fn prepare<R, Fut>(&'static self, key: Key, permits: &'static Semaphore, read: R) -> Option<impl Future<Output = ()> + Send + 'static>
    where
        R: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let (mut tx, started) = self.begin(key, Instant::now())?;
        let deadline = tokio::time::Instant::now() + self.keep;
        Some(async move {
            enum Step<T> {
                Gone,
                Expired,
                Done(Option<T>),
            }
            let reading = async move {
                let Ok(_permit) = permits.acquire().await else { return None };
                started.store(true, Ordering::SeqCst);
                Some(read().await)
            };
            tokio::pin!(reading);
            let expiry = tokio::time::sleep_until(deadline);
            tokio::pin!(expiry);
            let mut expired = false;
            let result = loop {
                let step = tokio::select! {
                    biased;
                    // Dropped (menu closed, pushed out, expired): the read stops here.
                    _ = tx.closed() => Step::Gone,
                    _ = &mut expiry, if !expired => Step::Expired,
                    r = &mut reading => Step::Done(r),
                };
                match step {
                    Step::Gone | Step::Done(None) => return,
                    // Ours goes now unless a send took it (then it carries on for that send).
                    Step::Expired => {
                        expired = true;
                        self.prune_now();
                    }
                    Step::Done(Some(r)) => break r,
                }
            };
            let _ = tx.send(result);
            // Never picked: gone when it expires (not only at the next read).
            if !expired {
                expiry.await;
                self.prune_now();
            }
        })
    }

    /// The read for `key`: the one started ahead (waited for when it is still going) when
    /// `reuse` takes its result, else a read now (in its turn among `permits`), exactly as
    /// without a read ahead.
    pub(crate) async fn get<R, Fut>(&self, key: &Key, permits: &Semaphore, read: R, reuse: impl FnOnce(&T) -> bool) -> T
    where
        R: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        if let Some(ahead) = self.take(key, Instant::now()) {
            if let Ok(result) = ahead.await {
                if reuse(&result) {
                    return result;
                }
            }
        }
        let _permit = permits.acquire().await;
        read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn cookie(value: &str) -> SiteCookie {
        SiteCookie { domain: ".youtube.com".into(), path: "/".into(), name: "SID".into(), value: value.into(), ..Default::default() }
    }

    fn named(name: &str, value: &str, expires: u64) -> SiteCookie {
        SiteCookie { domain: ".youtube.com".into(), path: "/".into(), name: name.into(), value: value.into(), expires, ..Default::default() }
    }

    fn leak<T: Send + 'static>(r: Reads<T>) -> &'static Reads<T> {
        Box::leak(Box::new(r))
    }

    fn permits(n: usize) -> &'static Semaphore {
        Box::leak(Box::new(Semaphore::new(n)))
    }

    /// Counts when a read is dropped before it ends (as yt-dlp's process is ended then).
    struct Stopped(Arc<AtomicUsize>);
    impl Drop for Stopped {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A read that never ends by itself; `stopped` counts it being dropped.
    fn endless(stopped: &Arc<AtomicUsize>, runs: &Arc<AtomicUsize>) -> impl FnOnce() -> std::pin::Pin<Box<dyn Future<Output = usize> + Send>> + Send + 'static {
        let (stopped, runs) = (stopped.clone(), runs.clone());
        move || {
            Box::pin(async move {
                runs.fetch_add(1, Ordering::SeqCst);
                let _guard = Stopped(stopped);
                std::future::pending::<()>().await;
                0
            })
        }
    }

    const YT: &str = "https://www.youtube.com/watch?v=abcdefghijk";

    #[tokio::test]
    async fn prepare_then_send_reads_once() {
        let reads = leak(Reads::<Result<String, String>>::new(KEEP, MAX_KEPT));
        let p = permits(2);
        let runs = Arc::new(AtomicUsize::new(0));
        let key = Key::new(YT, false, &[cookie("a")]);
        let r = runs.clone();
        let task = reads
            .prepare(key.clone(), p, move || async move {
                r.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(30)).await;
                Ok("Title".to_string())
            })
            .unwrap();
        let running = tokio::spawn(task);
        // The menu opened again for the same video: no second read.
        assert!(reads.prepare(key.clone(), p, || async { Ok("again".to_string()) }).is_none());
        tokio::time::sleep(Duration::from_millis(5)).await;
        // The quality is picked while the read is still going: it waits for that read.
        let r = runs.clone();
        let got = reads
            .get(
                &key,
                permits(2),
                move || async move {
                    r.fetch_add(1, Ordering::SeqCst);
                    Ok("second read".to_string())
                },
                |r| r.is_ok(),
            )
            .await;
        assert_eq!(got, Ok("Title".to_string()));
        assert_eq!(runs.load(Ordering::SeqCst), 1, "one read for both");
        // Taken: a later send reads again (nothing kept).
        assert_eq!(reads.len(Instant::now()), 0);
        running.abort();
    }

    /// A read ahead that failed (maybe a network hiccup a minute ago) is never the final word:
    /// the pick reads again.
    #[tokio::test]
    async fn a_failed_read_ahead_is_read_again_at_the_pick() {
        let reads = leak(Reads::<Result<String, String>>::new(KEEP, MAX_KEPT));
        let p = permits(2);
        let key = Key::new(YT, false, &[]);
        let task = reads.prepare(key.clone(), p, || async { Err("No connection to the site.".to_string()) }).unwrap();
        let running = tokio::spawn(task);
        tokio::time::sleep(Duration::from_millis(20)).await;
        let got = reads.get(&key, permits(2), || async { Ok("read again".to_string()) }, |r| r.is_ok()).await;
        assert_eq!(got, Ok("read again".to_string()));
        running.abort();
    }

    #[tokio::test]
    async fn nothing_prepared_reads_as_before() {
        let reads = leak(Reads::<u32>::new(KEEP, MAX_KEPT));
        let got = reads.get(&Key::new(YT, false, &[]), permits(1), || async { 7 }, |_| true).await;
        assert_eq!(got, 7);
    }

    #[test]
    fn a_read_expires() {
        let reads = Reads::<u32>::new(KEEP, MAX_KEPT);
        let key = Key::new(YT, false, &[]);
        let t0 = Instant::now();
        let (tx, started) = reads.begin(key.clone(), t0).unwrap();
        started.store(true, Ordering::SeqCst);
        tx.send(1).unwrap();
        assert!(reads.take(&key, t0 + KEEP + Duration::from_secs(1)).is_none(), "expired: not used");
        assert_eq!(reads.len(t0 + KEEP), 0);
        // Expired, so the menu opening again starts a new read.
        assert!(reads.begin(key.clone(), t0 + KEEP + Duration::from_secs(2)).is_some());
    }

    #[test]
    fn private_and_normal_reads_and_other_logins_never_mix() {
        let reads = Reads::<&'static str>::new(KEEP, MAX_KEPT);
        let now = Instant::now();
        let put = |key: Key, v: &'static str| {
            let (tx, started) = reads.begin(key, now).unwrap();
            started.store(true, Ordering::SeqCst);
            tx.send(v).unwrap();
        };
        put(Key::new(YT, true, &[]), "private");
        // A normal window's send (even with no cookies) never gets the private read.
        assert!(reads.take(&Key::new(YT, false, &[]), now).is_none());
        put(Key::new(YT, false, &[cookie("a")]), "logged in");
        // Nor a private one the normal read, nor another login.
        assert!(reads.take(&Key::new(YT, false, &[cookie("b")]), now).is_none());
        assert!(reads.take(&Key::new(YT, false, &[]), now).is_none());
        let mut private = reads.take(&Key::new(YT, true, &[]), now).unwrap();
        assert_eq!(private.try_recv().unwrap(), "private");
        // A private window's key never holds cookies, even if some were given.
        assert_eq!(Key::new(YT, true, &[cookie("a")]), Key::new(YT, true, &[]));
        let mut normal = reads.take(&Key::new(YT, false, &[cookie("a")]), now).unwrap();
        assert_eq!(normal.try_recv().unwrap(), "logged in");
        // Another video: never.
        put(Key::new(YT, false, &[]), "x");
        assert!(reads.take(&Key::new("https://www.youtube.com/watch?v=zzzzzzzzzzz", false, &[]), now).is_none());
    }

    /// Between the menu opening and the pick the site renews some cookies (YouTube's SIDCC, a
    /// later expiry on others) and the browser may list them in another order: the same login.
    /// Another account (another SID) is not.
    #[test]
    fn the_same_login_a_moment_later_is_the_same_key() {
        let opened = [named("SID", "me", 2_000_000_000), named("SIDCC", "one", 2_000_000_000), named("__Secure-1PSIDTS", "t1", 0), named("PREF", "f6=1", 2_000_000_000)];
        let picked = [named("PREF", "f6=1", 2_000_000_600), named("__Secure-1PSIDTS", "t2", 0), named("SID", "me", 2_000_000_000), named("SIDCC", "two", 2_000_000_100), named("ST-1b2c", "csn", 0)];
        assert_eq!(Key::new(YT, false, &opened), Key::new(YT, false, &picked));
        let other = [named("SID", "someone else", 2_000_000_000), named("PREF", "f6=1", 2_000_000_000)];
        assert_ne!(Key::new(YT, false, &opened), Key::new(YT, false, &other));
        // Signed out since: not the same.
        assert_ne!(Key::new(YT, false, &opened), Key::new(YT, false, &[named("PREF", "f6=1", 0)]));
        // The key keeps no cookie value.
        assert!(!format!("{:?}", Key::new(YT, false, &opened)).contains("me"));
    }

    #[test]
    fn a_few_reads_at_most_the_oldest_gives_way() {
        let reads = Reads::<usize>::new(KEEP, 3);
        let now = Instant::now();
        let link = |i: usize| format!("https://www.youtube.com/watch?v=video{i:06}");
        let mut senders = Vec::new();
        for i in 0..5 {
            senders.push(reads.begin(Key::new(&link(i), false, &[]), now).unwrap().0);
        }
        assert_eq!(reads.len(now), 3);
        // The two oldest were pushed out: nobody waits for them any more.
        assert!(senders[0].is_closed() && senders[1].is_closed());
        assert!(!senders[4].is_closed());
        assert!(reads.take(&Key::new(&link(0), false, &[]), now).is_none());
        assert!(reads.take(&Key::new(&link(4), false, &[]), now).is_none(), "not started: the send reads on its own");
    }

    #[tokio::test]
    async fn a_read_pushed_out_while_waiting_its_turn_never_runs() {
        let reads = leak(Reads::<usize>::new(KEEP, 1));
        let p = permits(1);
        let busy = p.acquire().await.unwrap();
        let runs = Arc::new(AtomicUsize::new(0));
        let r = runs.clone();
        let first = reads.prepare(Key::new(YT, false, &[]), p, move || async move { r.fetch_add(1, Ordering::SeqCst) }).unwrap();
        let first = tokio::spawn(first);
        // Another video's menu: the first read gives way before its turn came.
        let r = runs.clone();
        let second = reads.prepare(Key::new("https://vimeo.com/123456", false, &[]), p, move || async move { r.fetch_add(1, Ordering::SeqCst) }).unwrap();
        let second = tokio::spawn(second);
        drop(busy);
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(runs.load(Ordering::SeqCst), 1, "only the read still wanted ran");
        first.abort();
        second.abort();
    }

    /// A read pushed out while it runs stops at once (its yt-dlp ends) and frees its turn.
    #[tokio::test]
    async fn a_read_pushed_out_while_running_stops_and_frees_its_turn() {
        let reads = leak(Reads::<usize>::new(KEEP, 1));
        let p = permits(1);
        let (stopped, runs) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let first = tokio::spawn(reads.prepare(Key::new(YT, false, &[]), p, endless(&stopped, &runs)).unwrap());
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!((runs.load(Ordering::SeqCst), p.available_permits()), (1, 0));
        let second = tokio::spawn(reads.prepare(Key::new("https://vimeo.com/123456", false, &[]), p, endless(&stopped, &runs)).unwrap());
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(stopped.load(Ordering::SeqCst), 1, "the first read was stopped");
        assert_eq!(runs.load(Ordering::SeqCst), 2, "the newer one has its turn");
        assert!(first.is_finished());
        second.abort();
    }

    /// The menu closed without a pick: the read stops (yt-dlp ended, its turn freed) and goes;
    /// the pick that follows later reads again. Another video's read, or the other kind of
    /// window's, carries on.
    #[tokio::test]
    async fn closing_the_menu_stops_the_read_and_a_later_pick_reads_again() {
        let reads = leak(Reads::<usize>::new(KEEP, MAX_KEPT));
        let p = permits(2);
        let (stopped, runs) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let task = tokio::spawn(reads.prepare(Key::new(YT, false, &[]), p, endless(&stopped, &runs)).unwrap());
        let other = tokio::spawn(reads.prepare(Key::new(YT, true, &[]), p, endless(&stopped, &runs)).unwrap());
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(p.available_permits(), 0);
        assert_eq!(reads.cancel(YT, false), 1);
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(stopped.load(Ordering::SeqCst), 1, "the read stopped");
        assert!(task.is_finished());
        assert_eq!(p.available_permits(), 1, "its turn is free again");
        assert_eq!(reads.len(Instant::now()), 1, "the private window's read is still there");
        assert_eq!(reads.cancel(YT, false), 0, "nothing left to cancel");
        // A later pick reads on its own.
        let got = reads.get(&Key::new(YT, false, &[]), permits(2), || async { 42 }, |_| true).await;
        assert_eq!(got, 42);
        other.abort();
    }

    /// A pick never waits behind reads ahead: when its read ahead hasn't had its turn yet, the
    /// pick reads at once among its own permits (and the read ahead never runs).
    #[tokio::test]
    async fn a_pick_never_waits_behind_reads_ahead() {
        let reads = leak(Reads::<usize>::new(KEEP, MAX_KEPT));
        let ahead = permits(1);
        let (stopped, runs) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        // Another video's read ahead holds the only turn for reads ahead.
        let other = tokio::spawn(reads.prepare(Key::new("https://vimeo.com/123456", false, &[]), ahead, endless(&stopped, &runs)).unwrap());
        let key = Key::new(YT, false, &[]);
        let r = runs.clone();
        let waiting = tokio::spawn(reads.prepare(key.clone(), ahead, move || async move { r.fetch_add(100, Ordering::SeqCst) }).unwrap());
        tokio::time::sleep(Duration::from_millis(20)).await;
        let got = tokio::time::timeout(Duration::from_secs(2), reads.get(&key, permits(2), || async { 7 }, |_| true)).await;
        assert_eq!(got, Ok(7), "the pick read at once");
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(waiting.is_finished(), "the read ahead it replaced is gone");
        assert_eq!(runs.load(Ordering::SeqCst), 1, "and never ran");
        other.abort();
    }

    /// Expired untaken: the read stops even while still running. Taken by a send before that:
    /// it carries on for the send.
    #[tokio::test]
    async fn an_expired_read_stops_but_a_taken_one_carries_on() {
        let keep = Duration::from_millis(60);
        let reads = leak(Reads::<usize>::new(keep, MAX_KEPT));
        let p = permits(2);
        let (stopped, runs) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let task = tokio::spawn(reads.prepare(Key::new(YT, false, &[]), p, endless(&stopped, &runs)).unwrap());
        tokio::time::sleep(keep + Duration::from_millis(40)).await;
        assert_eq!(stopped.load(Ordering::SeqCst), 1);
        assert!(task.is_finished());
        assert_eq!(p.available_permits(), 2);
        // Taken while running: past its expiry the send still gets it.
        let key = Key::new("https://vimeo.com/123456", false, &[]);
        let slow = tokio::spawn(
            reads
                .prepare(key.clone(), p, || async {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    5
                })
                .unwrap(),
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
        let got = reads.get(&key, permits(2), || async { 9 }, |_| true).await;
        assert_eq!(got, 5, "the read ahead, not a new read");
        slow.abort();
    }
}
