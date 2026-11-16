// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Total speed limit shared by all downloads (token bucket). Rate 0 = no limit.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub struct Limiter {
    rate: AtomicU64, // bytes per second
    state: Mutex<(f64, Instant)>,
}

impl Limiter {
    pub fn new(rate: u64) -> Self {
        Self { rate: AtomicU64::new(rate), state: Mutex::new((0.0, Instant::now())) }
    }

    pub fn set_rate(&self, rate: u64) {
        self.rate.store(rate, Ordering::Relaxed);
    }

    pub async fn acquire(&self, bytes: usize) {
        let rate = self.rate.load(Ordering::Relaxed);
        if rate == 0 {
            return;
        }
        let wait = {
            let mut st = self.state.lock().unwrap();
            let now = Instant::now();
            let elapsed = now.duration_since(st.1).as_secs_f64();
            // Allow at most half a second of burst.
            st.0 = (st.0 + elapsed * rate as f64).min(rate as f64 / 2.0);
            st.1 = now;
            st.0 -= bytes as f64;
            if st.0 < 0.0 { Some(Duration::from_secs_f64(-st.0 / rate as f64)) } else { None }
        };
        if let Some(w) = wait {
            tokio::time::sleep(w).await;
        }
    }
}
