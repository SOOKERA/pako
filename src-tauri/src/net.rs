// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! "Is it my internet or the server?"
//!
//! When a connection fails, the engine asks here whether the PC can reach the internet at all,
//! using the same small test address Windows itself uses (and Google's as a second opinion).
//! - Internet down: downloads wait calmly and continue within ~3 s of it coming back
//!   (checking the server itself again every minute or so, in case only the test addresses
//!   are blocked).
//! - Internet fine: the problem is the server, so the normal retry delays apply.

use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const CHECKS: &[&str] = &["http://www.msftconnecttest.com/connecttest.txt", "http://www.gstatic.com/generate_204"];

pub struct Net {
    http: reqwest::Client,
    urls: Vec<String>,
    last: Mutex<Option<(Instant, bool)>>,
}

impl Net {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("http client");
        // Tests can point the check somewhere they control (PAKO_NET_CHECK_URL; the older
        // name ZD_NET_CHECK_URL still works).
        let urls = match std::env::var("PAKO_NET_CHECK_URL").or_else(|_| std::env::var("ZD_NET_CHECK_URL")) {
            Ok(u) => vec![u],
            Err(_) => CHECKS.iter().map(|s| s.to_string()).collect(),
        };
        Self { http, urls, last: Mutex::new(None) }
    }

    /// True if the internet is reachable. The answer is reused for 3 seconds.
    pub async fn online(&self) -> bool {
        let mut last = self.last.lock().await;
        if let Some((at, ok)) = *last {
            if at.elapsed() < Duration::from_secs(3) {
                return ok;
            }
        }
        let mut ok = false;
        for u in &self.urls {
            if let Ok(r) = self.http.get(u).send().await {
                if r.status().is_success() {
                    ok = true;
                    break;
                }
            }
        }
        *last = Some((Instant::now(), ok));
        ok
    }

    /// Waits until the internet is back, but no longer than `max`: the test addresses may be
    /// blocked on this network while the download server itself works fine.
    /// Some(true) = back, Some(false) = still looks offline after `max`, None = cancelled.
    pub async fn wait_online(&self, cancel: &CancellationToken, max: Duration) -> Option<bool> {
        let until = Instant::now() + max;
        loop {
            if self.online().await {
                return Some(true);
            }
            if Instant::now() >= until {
                return Some(false);
            }
            tokio::select! {
                _ = cancel.cancelled() => return None,
                _ = tokio::time::sleep(Duration::from_secs(3)) => {}
            }
        }
    }
}
