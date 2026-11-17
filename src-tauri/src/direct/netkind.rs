// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! What Windows calls the networks this PC is on: Public, Private (a home network) or Domain
//! (a company network). Asked from Windows' Network List Manager (the same answer as Windows'
//! Settings › Network › "Network profile type"), on a thread of its own: no PowerShell, no
//! window.

/// A network's kind, as Windows marks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Public,
    Private,
    Domain,
}

/// Asks Windows. Kept for the life of its thread (COM is set up on it once).
pub struct Reader {
    #[cfg(windows)]
    inner: win::Reader,
}

impl Reader {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            inner: win::Reader::new(),
        }
    }

    /// The kinds of the networks this PC is connected to now (one per network; empty when it
    /// is on none). `None`: Windows didn't answer.
    pub fn read(&mut self) -> Option<Vec<Kind>> {
        #[cfg(windows)]
        return self.inner.read();
        #[cfg(not(windows))]
        None
    }
}

#[cfg(windows)]
mod win {
    use super::Kind;
    use windows::Win32::Networking::NetworkListManager::{
        INetworkListManager, NetworkListManager, NLM_ENUM_NETWORK_CONNECTED, NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED, NLM_NETWORK_CATEGORY_PRIVATE,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoIncrementMTAUsage, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    /// More networks than this at once is not a real PC: stop counting.
    const MOST: usize = 64;

    /// Keeps Windows' shared COM space for background threads (the "MTA") alive until Pako
    /// closes. Without it, the reader's thread could be the last one that set COM up, and its
    /// `CoUninitialize` then tears that space down under other threads still using COM inside
    /// it without having set it up themselves (Direct's network library asks Windows for the
    /// default route through WMI that way, on its own background threads): their next COM call
    /// crashes Pako (STATUS_ACCESS_VIOLATION). Done once, before the first `CoInitializeEx`.
    fn keep_mta_alive() {
        static KEPT: std::sync::Once = std::sync::Once::new();
        KEPT.call_once(|| {
            // SAFETY: a plain call; the cookie is deliberately never given back.
            let _ = unsafe { CoIncrementMTAUsage() };
        });
    }

    pub struct Reader {
        nlm: Option<INetworkListManager>,
        /// COM was set up here (and is closed again when the reader goes).
        com: bool,
    }

    impl Reader {
        pub fn new() -> Self {
            keep_mta_alive();
            // SAFETY: called once on the reader's own thread; undone in Drop when it succeeded.
            // (The shared COM space outlives that: `keep_mta_alive`.)
            let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
            Self { nlm: None, com }
        }

        pub fn read(&mut self) -> Option<Vec<Kind>> {
            let got = self.ask();
            if got.is_none() {
                // Made again next time (Windows' network service may have restarted).
                self.nlm = None;
            }
            got
        }

        fn ask(&mut self) -> Option<Vec<Kind>> {
            if self.nlm.is_none() {
                // SAFETY: a standard COM object, created on a thread where COM is set up.
                self.nlm = unsafe { CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL) }.ok();
            }
            let nlm = self.nlm.as_ref()?;
            // SAFETY: plain COM calls on a live object; every result is checked.
            unsafe {
                let list = nlm.GetNetworks(NLM_ENUM_NETWORK_CONNECTED).ok()?;
                let mut kinds = Vec::new();
                while kinds.len() < MOST {
                    let mut one = [None];
                    let mut fetched = 0u32;
                    list.Next(&mut one, Some(&mut fetched)).ok()?;
                    let Some(net) = one[0].take() else { break };
                    if fetched == 0 {
                        break;
                    }
                    kinds.push(match net.GetCategory().ok()? {
                        NLM_NETWORK_CATEGORY_PRIVATE => Kind::Private,
                        NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED => Kind::Domain,
                        // Public, and anything Windows may add later: the safe side.
                        _ => Kind::Public,
                    });
                }
                Some(kinds)
            }
        }
    }

    impl Drop for Reader {
        fn drop(&mut self) {
            self.nlm = None;
            if self.com {
                // SAFETY: pairs the successful CoInitializeEx of this thread.
                unsafe { CoUninitialize() };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows answers on this PC (whatever its networks are). Read-only.
    #[cfg(windows)]
    #[test]
    fn windows_answers() {
        let got = std::thread::spawn(|| Reader::new().read()).join().unwrap();
        eprintln!("networks on this PC: {got:?}");
        assert!(got.is_some(), "the Network List Manager answered");
    }

    /// Readers coming and going (each sets COM up on its thread, then closes it) while other
    /// threads use COM without setting it up, as Direct's network library does for WMI. The
    /// reader closing COM used to tear the shared COM space down under them: the whole program
    /// crashed (STATUS_ACCESS_VIOLATION) within about a second of this.
    #[cfg(windows)]
    #[test]
    fn readers_closing_never_pull_com_from_under_other_threads() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
        use windows::Win32::Networking::NetworkListManager::{INetworkListManager, NetworkListManager, NLM_ENUM_NETWORK_CONNECTED};
        use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
        // A first reader makes sure the shared COM space exists before the others use it.
        std::thread::spawn(|| Reader::new().read()).join().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let used = Arc::new(AtomicU32::new(0));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let stop = stop.clone();
            threads.push(std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let _ = Reader::new().read();
                }
            }));
        }
        for _ in 0..4 {
            let (stop, used) = (stop.clone(), used.clone());
            threads.push(std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    // SAFETY: plain COM calls on objects this thread made; no CoInitializeEx here
                    // on purpose (the shared COM space is used as it is).
                    let ok = unsafe {
                        CoCreateInstance::<_, INetworkListManager>(&NetworkListManager, None, CLSCTX_ALL)
                            .and_then(|nlm| nlm.GetNetworks(NLM_ENUM_NETWORK_CONNECTED))
                            .is_ok()
                    };
                    if ok {
                        used.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
        stop.store(true, Ordering::Relaxed);
        for t in threads {
            t.join().unwrap();
        }
        assert!(used.load(Ordering::Relaxed) > 0, "the threads without their own COM set-up could use it");
    }
}
