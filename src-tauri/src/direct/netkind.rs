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
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};

    /// More networks than this at once is not a real PC: stop counting.
    const MOST: usize = 64;

    pub struct Reader {
        nlm: Option<INetworkListManager>,
        /// COM was set up here (and is closed again when the reader goes).
        com: bool,
    }

    impl Reader {
        pub fn new() -> Self {
            // SAFETY: called once on the reader's own thread; undone in Drop when it succeeded.
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
}
