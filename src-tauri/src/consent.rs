// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Nothing runs before the terms of use are accepted (TERMS.md, see terms.rs).
//!
//! Until then Pako only shows its window with the terms (Agree and continue, or Quit): no
//! downloads start, Direct and My devices don't start (no network node, nothing said on the
//! home network), the browser extension is told "Pako needs you to accept its terms first",
//! the tray only opens or quits Pako, no update checks or tool installs, no copied links caught,
//! no uploads deleted on their date. Downloads and lists stay as they are: waiting or paused
//! downloads simply wait. Everything starts the moment the terms are accepted, no restart.
//!
//! Asked once per copy of the settings: `terms_accepted` (the terms' date, "2026-11-18") is
//! saved at once and never asked again, also after updates or a newer TERMS.md (the terms say
//! continued use means acceptance, and the newest text is always at sookera.com/terms).

use std::sync::Mutex;

use crate::model::Settings;

/// People who used Pako before it had terms (their settings say the first-start setup was done,
/// without `terms_accepted`) are asked once too, with the same screen. `false` = they aren't.
pub const ASK_EXISTING_USERS: bool = true;

/// Ask again those who accepted terms dated before this ("2027-01-31"), for a big change to the
/// terms. Never on its own: only by setting this on purpose. None = never ask again.
pub const REASK_IF_ACCEPTED_BEFORE: Option<&str> = None;

/// What a request gets while the terms wait (the browser extension, and the window's commands).
pub const NOT_YET: &str = "Pako needs you to accept its terms first.";

/// These settings still have to accept the terms. Only a missing (or blank) date asks: never
/// Pako's version, never the date of the TERMS.md this Pako carries.
pub fn needed(s: &Settings) -> bool {
    match s.terms_accepted.as_deref().map(str::trim).filter(|d| !d.is_empty()) {
        None => ASK_EXISTING_USERS || !s.setup_done,
        Some(date) => REASK_IF_ACCEPTED_BEFORE.is_some_and(|cut| date < cut),
    }
}

type Waiting = Vec<Box<dyn FnOnce() + Send>>;

/// Closed until the terms are accepted: what waits behind it starts when it opens (once).
pub struct Gate {
    state: Mutex<(bool, Waiting)>,
}

impl Gate {
    pub const fn new() -> Self {
        Self { state: Mutex::new((false, Vec::new())) }
    }

    pub fn is_open(&self) -> bool {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).0
    }

    /// Runs `f` now if the gate is open, else the moment it opens.
    pub fn after(&self, f: impl FnOnce() + Send + 'static) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.0 {
            drop(st);
            f();
        } else {
            st.1.push(Box::new(f));
        }
    }

    /// Opens it and starts what waited, in the order it was added (outside the lock: what
    /// starts may ask whether the gate is open).
    pub fn open(&self) {
        let waiting = {
            let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
            st.0 = true;
            std::mem::take(&mut st.1)
        };
        for f in waiting {
            f();
        }
    }
}

static GATE: Gate = Gate::new();

/// The terms are accepted (or didn't need to be): Pako runs normally.
pub fn ok() -> bool {
    GATE.is_open()
}

/// For a command the window can call: refused while the terms wait.
pub fn check() -> Result<(), String> {
    if ok() { Ok(()) } else { Err(NOT_YET.into()) }
}

/// Starts `f` once the terms are accepted (now if they already are).
pub fn after(f: impl FnOnce() + Send + 'static) {
    GATE.after(f)
}

/// The terms were accepted (or these settings don't need to): everything waiting starts.
pub fn open() {
    GATE.open()
}

/// At start, from the settings just read.
pub fn start_with(s: &Settings) {
    if !needed(s) {
        open();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    fn settings(json: &str) -> Settings {
        serde_json::from_str(json).expect("settings load")
    }

    /// Settings without the field (a new copy, or Pako 1.0.x's) are asked; a recorded date is
    /// never asked again, whatever Pako's version or the date of its TERMS.md.
    #[test]
    fn asked_only_when_the_date_is_missing() {
        assert!(needed(&Settings::default()), "a new copy is asked");
        let old = settings(r#"{"setupDone":true,"whatsNewSeen":"1.0.1"}"#);
        assert_eq!(needed(&old), ASK_EXISTING_USERS, "settings from 1.0.x");
        assert!(needed(&settings(r#"{"setupDone":true,"termsAccepted":"  "}"#)) == ASK_EXISTING_USERS);
        // Accepted once, then a newer Pako (its What's new seen, later versions): not asked.
        let accepted = settings(r#"{"setupDone":true,"termsAccepted":"2026-11-18","whatsNewSeen":"1.1.0"}"#);
        assert!(!needed(&accepted));
        let later = Settings { whats_new_seen: Some("9.4.2".into()), ..accepted.clone() };
        assert!(!needed(&later));
        // Accepted, then TERMS.md gets a newer date: still not asked (only the deliberate
        // REASK_IF_ACCEPTED_BEFORE would).
        assert!(REASK_IF_ACCEPTED_BEFORE.is_none());
        let newer_terms = crate::terms::version_of("Last updated 2 March 2031").unwrap();
        assert!(newer_terms.as_str() > "2026-11-18");
        assert!(!needed(&accepted));
        // Saved and read back: the date stays.
        let back: Settings = serde_json::from_str(&serde_json::to_string(&accepted).unwrap()).unwrap();
        assert_eq!(back.terms_accepted.as_deref(), Some("2026-11-18"));
    }

    /// What waits behind the gate (the engine's queue and clipboard, Direct, update checks,
    /// tool installs, upload clean-up) doesn't start before the terms are accepted, starts once
    /// when they are, and later services start straight away.
    #[test]
    fn services_start_only_after_acceptance() {
        let gate = Gate::new();
        let started = Arc::new(AtomicU32::new(0));
        for _ in 0..3 {
            let s = started.clone();
            gate.after(move || {
                s.fetch_add(1, Ordering::SeqCst);
            });
        }
        assert!(!gate.is_open());
        assert_eq!(started.load(Ordering::SeqCst), 0, "nothing starts before acceptance");
        gate.open();
        assert!(gate.is_open());
        assert_eq!(started.load(Ordering::SeqCst), 3, "everything starts on acceptance, no restart");
        gate.open();
        assert_eq!(started.load(Ordering::SeqCst), 3, "and only once");
        let s = started.clone();
        gate.after(move || {
            s.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(started.load(Ordering::SeqCst), 4);
    }

    /// A service that asks whether the gate is open while it starts doesn't wait on itself.
    #[test]
    fn a_starting_service_can_ask() {
        static G: Gate = Gate::new();
        let seen = Arc::new(Mutex::new(None));
        let s = seen.clone();
        G.after(move || *s.lock().unwrap() = Some(G.is_open()));
        G.open();
        assert_eq!(*seen.lock().unwrap(), Some(true));
    }
}
