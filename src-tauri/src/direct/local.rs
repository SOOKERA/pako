// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Direct on the same network: finding contacts on the home network without the internet, and
//! telling a home-network connection ("Local network") from one over the internet.
//!
//! Finding: the library's local-network lookup (mDNS, `iroh-mdns-address-lookup`). While
//! Direct runs, this Pako asks who else is on the network and, when it may be seen there,
//! answers with its Pako ID and its own addresses on that network (the port Direct listens on
//! included). That stays on the network: routers don't pass these messages on to the
//! internet. Nothing else is announced (no name, no contacts, no relay). Who may connect
//! doesn't change: a contact connects by itself, anyone else is asked once, as for a
//! connection over the internet.
//!
//! Being seen ("Be visible to Pakos on this network", `Settings::direct_visible`): by default
//! only while every network this PC is on is one Windows marks as Private or Domain (a café or
//! hotel Wi-Fi is Public), or always, or never. Not being seen only stops the answers: this
//! Pako still listens, so it finds its contacts there and calls them. The choice is made again
//! when the setting or the network changes (`Nearby::watch`); until Windows has answered, and
//! whenever it can't, the network counts as Public.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::stream::BoxStream;
use iroh::address_lookup::{AddrFilter, AddressLookup, EndpointData, Error as LookupError, Item};
use iroh::{Endpoint, EndpointId, TransportAddr};
use iroh_mdns_address_lookup::MdnsAddressLookup;
use tokio_util::sync::CancellationToken;

use super::netkind::{self, Kind};

/// How often Windows is asked again what the networks are (a network's kind can change
/// without its addresses changing, in Windows' Settings).
const KIND_EVERY: Duration = Duration::from_secs(30);
/// How often the setting is read again.
const SETTING_EVERY: Duration = Duration::from_secs(2);

/// A home-network address: 10.x, 172.16–31.x, 192.168.x, 169.254.x (a network without a
/// router), and IPv6 fe80::/10 and fd00::/8 (fc00::/7 in full). Not this PC itself (127.x, ::1).
pub fn is_local_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_local_ip(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            (first & 0xffc0) == 0xfe80 || (first & 0xfe00) == 0xfc00
        }
    }
}

/// The way a connection travels, by the address it uses: "local" (the home network),
/// "direct6", "direct" (straight over the internet) or "relay".
pub fn route_of(addr: &TransportAddr) -> &'static str {
    match addr {
        TransportAddr::Ip(a) if is_local_ip(a.ip()) => "local",
        TransportAddr::Ip(a) if a.is_ipv6() => "direct6",
        TransportAddr::Ip(_) => "direct",
        _ => "relay",
    }
}

/// What this Pako announces on the home network: its addresses there only (no relay, no
/// address on the internet; the Pako ID always goes with them, it's the name of the record).
fn announced() -> AddrFilter {
    AddrFilter::new(|addrs| {
        Cow::Owned(addrs.iter().filter(|a| matches!(a, TransportAddr::Ip(s) if is_local_ip(s.ip()))).cloned().collect())
    })
}

/// Why this Pako isn't seen on the network now; `None`: it is. `setting` is
/// `Settings::direct_visible` ("home", the default, "always" or "never"; anything else counts
/// as "home"); `kinds` the networks this PC is on, `None` when Windows didn't answer.
pub fn why_hidden(setting: &str, kinds: Option<&[Kind]>) -> Option<&'static str> {
    match setting {
        "never" => Some("off"),
        "always" => None,
        _ => match kinds {
            Some(k) if !k.is_empty() && !k.contains(&Kind::Public) => None,
            Some(k) if k.contains(&Kind::Public) => Some("public"),
            // On no network, or Windows didn't say: the safe side.
            _ => Some("unknown"),
        },
    }
}

/// The addresses that tell which network this PC is on (a change: maybe another network).
fn home_addrs(data: &EndpointData) -> BTreeSet<SocketAddr> {
    data.ip_addrs().filter(|a| is_local_ip(a.ip())).copied().collect()
}

/// Stands between the endpoint and the local lookup: passes this Pako's addresses on only
/// while it may be seen; otherwise the lookup has nothing to announce and answers no one.
/// Finding others goes through untouched.
#[derive(Debug)]
struct Gate {
    mdns: MdnsAddressLookup,
    st: Mutex<GateState>,
    /// Asks Windows again at once (the addresses changed).
    kick: mpsc::Sender<()>,
    /// Counts address changes: an answer from Windows read before the last one is too old.
    moves: Arc<AtomicU64>,
}

#[derive(Debug, Default)]
struct GateState {
    /// The endpoint's last addresses.
    last: Option<EndpointData>,
    open: bool,
}

impl Gate {
    fn forward(&self, st: &GateState) {
        let data = if st.open { st.last.clone().unwrap_or_default() } else { EndpointData::default() };
        self.mdns.publish(&data);
    }

    fn set_open(&self, open: bool) {
        let mut st = self.st.lock().unwrap();
        if st.open != open {
            st.open = open;
            self.forward(&st);
        }
    }
}

impl AddressLookup for Gate {
    fn publish(&self, data: &EndpointData) {
        let mut st = self.st.lock().unwrap();
        let moved = st.last.as_ref().is_some_and(|l| home_addrs(l) != home_addrs(data));
        st.last = Some(data.clone());
        if moved {
            // Maybe another network: quiet until Windows says what it is.
            self.moves.fetch_add(1, Ordering::SeqCst);
            st.open = false;
            let _ = self.kick.send(());
        }
        self.forward(&st);
    }

    fn resolve(&self, id: EndpointId) -> Option<BoxStream<'static, Result<Item, LookupError>>> {
        self.mdns.resolve(id)
    }
}

/// The local lookup of a running Direct, and whether this Pako is seen on the network.
pub struct Nearby {
    mdns: MdnsAddressLookup,
    gate: Arc<Gate>,
    /// Taken by `watch`.
    kick: Mutex<Option<mpsc::Receiver<()>>>,
    /// For the window: "visible", or why not ("off", "public", "unknown").
    status: Mutex<&'static str>,
}

impl Nearby {
    /// The lookup itself, to follow the Pakos found on the network.
    pub fn mdns(&self) -> MdnsAddressLookup {
        self.mdns.clone()
    }

    pub fn status(&self) -> &'static str {
        *self.status.lock().unwrap()
    }

    /// Decides, and decides again, whether this Pako is seen: when the setting changes (read
    /// every 2 s), when the addresses change (at once), and every 30 s (Windows is asked on a
    /// thread of its own, see `netkind`). Ends with Direct (`stop`), not seen.
    pub async fn watch(self: Arc<Self>, setting: impl Fn() -> String + Send + 'static, stop: CancellationToken, changed: impl Fn() + Send + 'static) {
        let Some(kick) = self.kick.lock().unwrap().take() else { return };
        // (address changes counted when Windows was asked, its answer)
        let (tx, mut kinds) = tokio::sync::watch::channel::<(u64, Option<Vec<Kind>>)>((0, None));
        let moves = self.gate.moves.clone();
        let asker = std::thread::Builder::new().name("pako-network-kind".into()).spawn(move || {
            let mut reader = netkind::Reader::new();
            // Asked again soon after a change (and at start): Windows may still be naming a
            // new network.
            let mut follow = 2u8;
            loop {
                let at = moves.load(Ordering::SeqCst);
                if tx.send((at, reader.read())).is_err() {
                    return;
                }
                let wait = match follow {
                    2 => Duration::from_secs(3),
                    1 => Duration::from_secs(10),
                    _ => KIND_EVERY,
                };
                follow = follow.saturating_sub(1);
                match kick.recv_timeout(wait) {
                    Ok(()) => {
                        while kick.try_recv().is_ok() {}
                        follow = 2;
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        });
        let mut asking = asker.is_ok();
        loop {
            let why = {
                let got = kinds.borrow();
                // No answer yet, one from before the addresses changed, or no one asking any
                // more: the safe side.
                let fresh = asking && got.0 == self.gate.moves.load(Ordering::SeqCst);
                why_hidden(&setting(), if fresh { got.1.as_deref() } else { None })
            };
            self.gate.set_open(why.is_none());
            let status = why.unwrap_or("visible");
            let was = std::mem::replace(&mut *self.status.lock().unwrap(), status);
            if was != status {
                changed();
            }
            tokio::select! {
                _ = stop.cancelled() => {
                    self.gate.set_open(false);
                    return;
                }
                r = kinds.changed(), if asking => if r.is_err() { asking = false },
                _ = tokio::time::sleep(SETTING_EVERY) => {}
            }
        }
    }
}

/// Starts looking for Pakos on the home network next to the public lookup, not seen there
/// until `Nearby::watch` decides it may be. `None` when the network doesn't allow it (Direct
/// then works as before, over the internet only).
pub fn start(ep: &Endpoint) -> Option<Arc<Nearby>> {
    let mdns = MdnsAddressLookup::builder().service_name("pako").addr_filter(announced()).build(ep.id()).ok()?;
    let (kick, asked) = mpsc::channel();
    let gate = Arc::new(Gate { mdns: mdns.clone(), st: Mutex::default(), kick, moves: Arc::default() });
    ep.address_lookup().ok()?.add(gate.clone());
    Some(Arc::new(Nearby { mdns, gate, kick: Mutex::new(Some(asked)), status: Mutex::new("unknown") }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn home_network_addresses() {
        for s in ["192.168.1.20", "10.0.0.5", "172.16.0.1", "172.31.255.254", "169.254.3.4", "fe80::1", "febf::1", "fd12:3456::1", "fc00::1", "::ffff:192.168.0.2"] {
            assert!(is_local_ip(ip(s)), "{s} is on the home network");
        }
        for s in ["8.8.8.8", "172.15.0.1", "172.32.0.1", "100.64.0.1", "127.0.0.1", "2a01:e0a::1", "::1", "fec0::1", "::ffff:8.8.8.8"] {
            assert!(!is_local_ip(ip(s)), "{s} isn't on the home network");
        }
    }

    #[test]
    fn route_labels() {
        let at = |s: &str| TransportAddr::Ip(s.parse::<SocketAddr>().unwrap());
        assert_eq!(route_of(&at("192.168.1.20:4000")), "local");
        assert_eq!(route_of(&at("[fe80::1]:4000")), "local");
        assert_eq!(route_of(&at("[fd00::7]:4000")), "local");
        assert_eq!(route_of(&at("[2a01:e0a::1]:4000")), "direct6");
        assert_eq!(route_of(&at("81.2.3.4:4000")), "direct");
        assert_eq!(route_of(&TransportAddr::Relay("https://relay.example.org".parse().unwrap())), "relay");
    }

    /// The setting and the kinds of the networks this PC is on: seen or not, and why.
    #[test]
    fn who_is_seen() {
        use Kind::*;
        let home: &[&[Kind]] = &[&[Private], &[Domain], &[Private, Domain], &[Private, Private]];
        let not_home: &[&[Kind]] = &[&[Public], &[Private, Public], &[Public, Domain]];
        for k in home {
            assert_eq!(why_hidden("home", Some(k)), None, "{k:?}");
            assert_eq!(why_hidden("", Some(k)), None, "nothing saved: home");
            assert_eq!(why_hidden("something new", Some(k)), None, "an unknown value: home");
            assert_eq!(why_hidden("always", Some(k)), None);
            assert_eq!(why_hidden("never", Some(k)), Some("off"));
        }
        for k in not_home {
            assert_eq!(why_hidden("home", Some(k)), Some("public"), "{k:?}");
            assert_eq!(why_hidden("always", Some(k)), None, "{k:?}");
            assert_eq!(why_hidden("never", Some(k)), Some("off"));
        }
        // Windows didn't answer, or no network: the safe side.
        assert_eq!(why_hidden("home", None), Some("unknown"));
        assert_eq!(why_hidden("home", Some(&[])), Some("unknown"));
        assert_eq!(why_hidden("always", None), None);
        assert_eq!(why_hidden("never", None), Some("off"));
    }

    #[test]
    fn a_new_network_is_noticed() {
        let at = |addrs: &[&str]| EndpointData::new(addrs.iter().map(|s| TransportAddr::Ip(s.parse().unwrap())).collect());
        let a = at(&["192.168.1.20:4000", "81.2.3.4:4000"]);
        assert_eq!(home_addrs(&a), home_addrs(&at(&["192.168.1.20:4000", "81.2.3.9:4000"])), "only the internet side changed");
        assert_ne!(home_addrs(&a), home_addrs(&at(&["10.0.0.7:4000", "81.2.3.4:4000"])), "another network");
    }

    #[test]
    fn announces_home_addresses_only() {
        let all = vec![
            TransportAddr::Ip("192.168.1.20:4000".parse().unwrap()),
            TransportAddr::Ip("81.2.3.4:4000".parse().unwrap()),
            TransportAddr::Ip("[2a01:e0a::1]:4000".parse().unwrap()),
            TransportAddr::Ip("[fe80::1]:4000".parse().unwrap()),
            TransportAddr::Relay("https://relay.example.org".parse().unwrap()),
        ];
        let data = iroh::address_lookup::EndpointData::new(all);
        let kept: Vec<String> = data.filtered_addrs(&announced()).iter().map(|a| format!("{a:?}")).collect();
        assert_eq!(kept.len(), 2, "{kept:?}");
        assert!(kept.iter().all(|a| a.contains("192.168.1.20") || a.contains("fe80::1")), "{kept:?}");
    }

    /// Two Pakos with no relay and no public lookup (as without internet) find each other on
    /// the network by the local lookup alone, from the Pako ID only, and the connection shows
    /// as "local". Needs this PC on a network (Wi-Fi or cable) that lets multicast through, so
    /// it's run by hand: `cargo test --lib local_lookup -- --ignored`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs a real network: run by hand"]
    async fn local_lookup_without_internet() {
        use iroh::endpoint::presets;
        const ALPN: &[u8] = b"pako/test-local/1";
        let bind = || async {
            Endpoint::builder(presets::Minimal).relay_mode(iroh::RelayMode::Disabled).alpns(vec![ALPN.to_vec()]).bind().await.unwrap()
        };
        let (a, b) = (bind().await, bind().await);
        // b may be seen whatever the network (the decision is tested in `who_is_seen`); a
        // only listens, as on a public network.
        let _la = start(&a).expect("local lookup on a");
        let lb = start(&b).expect("local lookup on b");
        lb.gate.set_open(true);
        let b_id = b.id();
        let accept = tokio::spawn(async move {
            let conn = b.accept().await.unwrap().await.unwrap();
            let _ = conn.closed().await;
            b
        });
        // Only the ID: the address comes from the network.
        let conn = tokio::time::timeout(std::time::Duration::from_secs(30), a.connect(iroh::EndpointAddr::new(b_id), ALPN))
            .await
            .expect("found within 30 s")
            .expect("connected");
        let paths = conn.paths();
        let sel = paths.iter().find(|p| p.is_selected()).expect("a path");
        assert_eq!(route_of(sel.remote_addr()), "local", "{:?}", sel.remote_addr());
        conn.close(0u32.into(), b"bye");
        accept.abort();
    }
}
