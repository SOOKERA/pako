// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Link gates: ad-gate link shorteners (exe.io and the like) that put countdowns, "continue"
//! pages, ads and a captcha between a pasted link and the real file.
//!
//! Pako works through them by itself, JDownloader-style, in a hidden window of its own: a small
//! script presses the gate's own buttons, waits out the countdowns (never skips them), and blocks
//! pop-ups and new tabs. Only the captcha is shown to the user, in a small Pako window with
//! everything else on the page hidden. Pako never solves a captcha itself. As soon as the page
//! heads for the real file (Google Drive, a file host, a download), Pako catches that link and the
//! Found row becomes the real file.
//!
//! The gate window gets no access to Pako's own functions (it isn't listed in the app's
//! capabilities): it only talks back through navigations to `https://pako.gate/…`, which are
//! intercepted and never leave the PC. It also keeps nothing: it runs in WebView2's InPrivate mode,
//! so the gate's and its ads' cookies and storage are gone when the window closes.
// The unit tests leave the window code out (see `open` / `show`).
#![cfg_attr(test, allow(dead_code, unused_imports))]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, OnceLock};
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::{Semaphore, mpsc};

use crate::engine::Engine;

static APP: OnceLock<AppHandle> = OnceLock::new();
/// One gate at a time: captchas come one after another, never several windows at once.
static ONE_AT_A_TIME: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(1));
static SEQ: AtomicU32 = AtomicU32::new(0);

/// Known gate sites (the widely used ad-gate shorteners and the mirror domains they hop through).
/// Other links that answer with a web page instead of a file also go through a gate window
/// (see `found.rs`).
const KNOWN_GATES: &[&str] = &[
    // exe.io family
    "exe.io", "exeygo.com", "exey.io", "exe.app",
    // ouo
    "ouo.io", "ouo.press",
    // ShrinkMe / ShrinkEarn
    "shrinkme.io", "shrinkme.us", "shrinke.me", "shrinkearn.com",
    // GPLinks, DropLink, Link1s, ShrtFly, StFly
    "gplinks.co", "gplinks.in", "gplinks.com", "droplink.co", "link1s.com", "shrtfly.com", "shrtfly.net", "stfly.me",
    // Cuty, clk.sh, fc.lc, za.gl, bc.vc, adfoc.us, adshrink
    "cuty.io", "cuty.me", "cutty.app", "clk.sh", "fc.lc", "fc-lc.xyz", "za.gl", "bc.vc", "adfoc.us", "adshrink.it",
    // Shorte.st and its mirrors
    "shorte.st", "sh.st", "gestyy.com", "ceesty.com", "corneey.com", "destyy.com", "festyy.com",
    // Linkvertise and its mirrors
    "linkvertise.com", "link-to.net", "up-to-down.net", "direct-link.net", "link-center.net", "link-target.net",
    "link-hub.net", "file-link.net",
    // Loot-Link / LootLabs
    "loot-link.com", "loot-links.com", "lootlinks.co", "lootdest.org", "lootdest.info", "links-loot.com",
    // Task / social unlockers
    "work.ink", "boost.ink", "bst.gg", "sub2unlock.com", "sub2unlock.net", "sub2get.com", "social-unlock.com",
    "rekonise.com", "mboost.me",
];

/// Gate sites that only let a person click through: they hand out "tasks" or check every click is a
/// real one. Pako doesn't press anything there; the page is shown to you straight away and Pako just
/// catches the file link at the end.
const HAND_OVER: &[&str] = &[
    "linkvertise.com", "link-to.net", "up-to-down.net", "direct-link.net", "link-center.net", "link-target.net",
    "link-hub.net", "file-link.net", "loot-link.com", "loot-links.com", "lootlinks.co", "lootdest.org",
    "lootdest.info", "links-loot.com", "work.ink", "boost.ink", "bst.gg", "sub2unlock.com", "sub2unlock.net",
    "sub2get.com", "social-unlock.com", "rekonise.com", "mboost.me",
];

/// Where real files live: reaching one of these means the gate is done.
const FILE_HOSTS: &[&str] = &[
    "drive.google.com", "docs.google.com", "drive.usercontent.google.com", "mega.nz", "mega.io", "mediafire.com",
    "pixeldrain.com", "gofile.io", "1fichier.com", "dropbox.com", "onedrive.live.com", "1drv.ms", "workupload.com",
    "krakenfiles.com", "buzzheavier.com", "qiwi.gg", "send.cm", "files.fm", "sendspace.com", "uptobox.com",
    "terabox.com", "1024terabox.com", "megaup.net", "uploadhaven.com", "upload.ee", "turbobit.net", "rapidgator.net",
    "nitroflare.com", "katfile.com", "ddownload.com", "userscloud.com", "hexload.com", "racaty.io", "dbree.me",
];

const FILE_EXTS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "xz", "exe", "msi", "iso", "img", "apk", "mp4", "mkv", "avi", "mov", "webm", "mp3",
    "flac", "pdf", "nsp", "xci", "nsz", "cia", "3ds", "wbfs", "rvz", "pkg",
];

/// Shown on the Found row while the captcha waits for the user.
pub const WAITING_FOR_YOU: &str = "Waiting for you: solve the check in the Pako window";

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

fn host_is(host: &str, list: &[&str]) -> bool {
    let host = host.trim_start_matches("www.");
    list.iter().any(|g| host == *g || host.ends_with(&format!(".{g}")))
}

pub fn is_known_gate(host: &str) -> bool {
    host_is(host, KNOWN_GATES)
}

/// A gate site where the page goes straight to you (see `HAND_OVER`).
fn is_hand_over(host: &str) -> bool {
    host_is(host, HAND_OVER)
}

/// A link that is clearly the file (or a file host page), not another gate step.
fn is_file_link(url: &tauri::Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    if host_is(host, FILE_HOSTS) {
        return true;
    }
    let last = url.path_segments().and_then(|mut s| s.next_back()).unwrap_or_default().to_ascii_lowercase();
    let ext = last.rsplit_once('.').map(|(_, e)| e).unwrap_or_default();
    // Also split-archive parts: .r00, .001
    FILE_EXTS.contains(&ext) || (ext.len() == 3 && (ext.starts_with('r') || ext.chars().all(|c| c.is_ascii_digit())) && ext[1..].chars().all(|c| c.is_ascii_digit()))
}

/// Same site, ignoring "www." and sub-domains (exeygo.com vs go.exeygo.com).
fn same_site(a: &str, b: &str) -> bool {
    let base = |h: &str| {
        let parts: Vec<&str> = h.trim_start_matches("www.").split('.').collect();
        parts[parts.len().saturating_sub(2)..].join(".")
    };
    base(a) == base(b)
}

enum Msg {
    Captcha,
    Solved,
    /// The page moved on: a page finished loading, a button was pressed, a countdown is running.
    Step,
    Final(String),
    /// "Show page" on the Found row.
    Show,
    /// The page went on to another gate site (exe.io → ouo.press): a fresh start there.
    Hop(String),
    Closed,
}

/// Shown on the Found row once the whole page is handed to the user.
pub const WAITING_PAGE: &str = "Waiting for you: finish opening the link in the Pako window";

/// The gate being worked on (one at a time): its Found row, for "Show page".
static CURRENT: std::sync::Mutex<Option<(String, mpsc::UnboundedSender<Msg>)>> = std::sync::Mutex::new(None);

/// Shown on a link waiting while another one is being opened.
const WAITING_TURN: &str = "Waiting for the link before it";

/// Without visible progress for this long, Pako stops pressing buttons and shows you the page.
const NO_PROGRESS: Duration = Duration::from_secs(40);
/// Pressing buttons by itself for longer than this means it isn't working: show the page.
const AUTO_MAX: Duration = Duration::from_secs(180);
/// How long the page (or a check) waits for you once shown.
const YOUR_TURN: Duration = Duration::from_secs(15 * 60);

/// Works through a gate link in the background; the result lands on the Found row.
/// The unit tests have no windows (and the window code needs what only Pako.exe has): as when
/// Pako has no window yet.
#[cfg(test)]
pub fn open(engine: Arc<Engine>, id: String, _link: String) {
    engine.gate_failed(&id, "Link pages can't be opened here.");
}

#[cfg(not(test))]
pub fn open(engine: Arc<Engine>, id: String, link: String) {
    let Some(app) = APP.get().cloned() else {
        engine.gate_failed(&id, "Link pages can't be opened here.");
        return;
    };
    tauri::async_runtime::spawn(async move {
        let _turn = match ONE_AT_A_TIME.try_acquire() {
            Ok(t) => t,
            Err(_) => {
                engine.gate_status(&id, Some(WAITING_TURN));
                let Ok(t) = ONE_AT_A_TIME.acquire().await else { return };
                engine.gate_status(&id, None);
                t
            }
        };
        // Removed from Found meanwhile: nothing to do.
        if !engine.found_has(&id) {
            return;
        }
        let result = run(&app, &engine, &id, &link).await;
        *CURRENT.lock().unwrap() = None;
        // Its "Pako needs you" (if it asked) is settled: the pop-up leaves, the bell says how it ended.
        crate::notices::resolve(&app, "needs-you", &id, if result.is_ok() { "Done" } else { "Not finished" });
        match result {
            Ok(file) => engine.gate_done(&id, file),
            Err(msg) => engine.gate_failed(&id, &msg),
        }
    });
}

/// Works through a gate link for My devices › Links (no Links row): the real file's link, or
/// why not. Waits its turn like the others; a page that needs you shows the same way.
pub async fn resolve(engine: Arc<Engine>, link: String) -> Result<String, String> {
    let app = APP.get().cloned().ok_or("Link pages can't be opened here.")?;
    let _turn = ONE_AT_A_TIME.acquire().await.map_err(|_| "Pako is closing.".to_string())?;
    let id = format!("pick-{}", uuid::Uuid::new_v4().simple());
    let result = run(&app, &engine, &id, &link).await;
    *CURRENT.lock().unwrap() = None;
    crate::notices::resolve(&app, "needs-you", &id, if result.is_ok() { "Done" } else { "Not finished" });
    result
}

/// auto: Pako presses the buttons. captcha: a check is shown. manual: the whole page is yours.
#[derive(PartialEq)]
enum Mode {
    Auto,
    Captcha,
    Manual,
}

fn show_page(win: &tauri::WebviewWindow, title: &str) {
    let _ = win.eval(MANUAL_JS);
    let _ = win.set_title(title);
    to_middle(win);
    let _ = win.show();
    let _ = win.unminimize();
    let _ = win.set_focus();
}

/// A hidden link page about to show: in the middle of the screen the mouse is on (the one you
/// are using), at the size it has. One already on screen stays where you put it.
fn to_middle(win: &tauri::WebviewWindow) {
    if win.is_visible().unwrap_or(false) {
        return;
    }
    let scale = win.scale_factor().unwrap_or(1.0);
    let place = win
        .outer_size()
        .ok()
        .map(|s| s.to_logical::<f64>(scale))
        .and_then(|s| crate::tray::middle_of_pointer_screen(win.app_handle(), s.width, s.height));
    match place {
        Some(p) => {
            let _ = win.set_position(p);
            // Snapped elsewhere as it shows (a window manager): back in the middle; dragging it
            // yourself ends that at once.
            let me = win.clone();
            crate::tray::hold_place(win, p, move || me.is_visible().unwrap_or(false));
        }
        None => {
            let _ = win.center();
        }
    }
}

async fn run(app: &AppHandle, engine: &Arc<Engine>, id: &str, link: &str) -> Result<String, String> {
    let url: tauri::Url = link.parse().map_err(|_| "Not a web link.".to_string())?;
    let gate_host = url.host_str().unwrap_or_default().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();
    *CURRENT.lock().unwrap() = Some((id.to_string(), tx.clone()));
    // Set once the page is handed to the user: on every page after that, Pako only watches.
    let manual = Arc::new(AtomicBool::new(false));
    let label = format!("gate-{}", SEQ.fetch_add(1, Ordering::Relaxed));
    // Set once the final "Get link" was pressed: from then on, leaving the gate's site means
    // reaching the destination, whatever it is.
    let went = Arc::new(AtomicBool::new(false));
    // Where the page last was, for the message if it gets stuck.
    let last_host = Arc::new(std::sync::Mutex::new(gate_host.clone()));

    // A step to another site that isn't a gate nor a file host: before "Get link", that's an ad
    // trying to take over the page (they do that when their pop-up is blocked). Stay put.
    let takeover_host = gate_host.clone();
    let is_takeover = Arc::new(move |u: &tauri::Url| {
        let host = u.host_str().unwrap_or_default();
        !same_site(host, &takeover_host) && !is_known_gate(host) && !is_file_link(u)
    });

    // The gate site the page is on now (changes when one gate hands over to another).
    let cur_site = std::sync::Mutex::new(gate_host.clone());
    let nav_tx = tx.clone();
    let nav_went = went.clone();
    let nav_takeover = is_takeover.clone();
    let win_tx = tx.clone();
    let win_went = went.clone();
    let win_takeover = is_takeover.clone();
    let title_tx = tx.clone();
    let title_went = went.clone();
    let load_tx = tx.clone();
    let load_host = last_host.clone();
    let load_manual = manual.clone();
    let dl_tx = tx.clone();
    let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title("Pako · Opening link")
        .inner_size(520.0, 640.0)
        .min_inner_size(360.0, 420.0)
        // No maximize button: with it, a window manager (PowerToys FancyZones) snaps even a popup
        // window into a zone. It stays resizable, can be minimized and is on the taskbar.
        .maximizable(false)
        .center()
        .visible(false)
        // Keeps nothing: the page runs in WebView2's InPrivate mode (same environment as Pako's
        // own windows, a throw-away profile for this window only), so the cookies and storage of
        // the gate and its ads are gone when the window closes, and Pako's own storage (theme,
        // window preferences) is never touched. Each link page starts with no cookies.
        .incognito(true)
        .initialization_script(GATE_SCRIPT)
        .on_navigation(move |u| {
            // A page's own blank and in-memory pages load; any other kind of address (file:,
            // ms-settings:, an app's link) would hand the page to something outside this window.
            if !matches!(u.scheme(), "http" | "https") {
                return matches!(u.scheme(), "about" | "blob" | "data");
            }
            if is_file_link(u) {
                let _ = nav_tx.send(Msg::Final(u.to_string()));
                return false;
            }
            let host = u.host_str().unwrap_or_default();
            // Handed over to another gate (exe.io → ouo.press): its own "Get link" comes later,
            // so jumping elsewhere before that is an ad again, not the file.
            if is_known_gate(host) {
                let mut cur = cur_site.lock().unwrap();
                if !same_site(host, &cur) {
                    *cur = host.to_string();
                    nav_went.store(false, Ordering::Relaxed);
                    let _ = nav_tx.send(Msg::Hop(host.to_string()));
                }
                return true;
            }
            if nav_takeover(u) {
                if nav_went.load(Ordering::Relaxed) {
                    let _ = nav_tx.send(Msg::Final(u.to_string()));
                }
                return false;
            }
            true
        })
        // New windows never open. One heading for the file is the destination.
        .on_new_window(move |u, _| {
            if is_file_link(&u) || (win_went.load(Ordering::Relaxed) && win_takeover(&u)) {
                let _ = win_tx.send(Msg::Final(u.to_string()));
            }
            tauri::webview::NewWindowResponse::Deny
        })
        // The page's script talks back through its title (it has no access to Pako itself).
        .on_document_title_changed(move |_, title| {
            let Some(sig) = title.strip_prefix(SIGNAL) else { return };
            let _ = match sig {
                "captcha" => title_tx.send(Msg::Captcha),
                "solved" => title_tx.send(Msg::Solved),
                "go" => {
                    title_went.store(true, Ordering::Relaxed);
                    title_tx.send(Msg::Step)
                }
                _ => title_tx.send(Msg::Step),
            };
        })
        .on_page_load(move |w, p| {
            if matches!(p.event(), tauri::webview::PageLoadEvent::Finished) {
                if let Some(h) = p.url().host_str() {
                    *load_host.lock().unwrap() = h.to_string();
                }
                // Once the page is yours, it stays yours on the next pages too. (Not while only a
                // check is shown: after it, Pako carries on pressing the buttons.)
                if load_manual.load(Ordering::Relaxed) {
                    let _ = w.eval(MANUAL_JS);
                }
                let _ = load_tx.send(Msg::Step);
            }
        })
        .on_download(move |_, event| {
            if let tauri::webview::DownloadEvent::Requested { url, .. } = event {
                let _ = dl_tx.send(Msg::Final(url.to_string()));
            }
            false
        })
        .build()
        .map_err(|e| format!("Couldn't open the link page: {e}"))?;
    // Like Download with Pako: FancyZones never snaps it into a zone (see to_middle too).
    crate::tray::keep_dialog_out_of_zones(&win);
    let close_tx = tx.clone();
    win.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed) {
            let _ = close_tx.send(Msg::Closed);
        }
    });

    let mut mode = Mode::Auto;
    // Time Pako has spent pressing buttons by itself (restarts after each check and on each new gate).
    let mut started = tokio::time::Instant::now();
    let page_title = format!("Pako · Finish opening this link · {gate_host}");
    // A site only a person can click through: yours from the start.
    if is_hand_over(&gate_host) {
        mode = Mode::Manual;
        manual.store(true, Ordering::Relaxed);
        show_page(&win, &page_title);
        engine.gate_status(id, Some(WAITING_PAGE));
        engine.notify_row(id, "needs-you", "Pako needs you", &format!("{gate_host} only lets a person click through: finish it in the Pako window."), crate::notices::Go::item("found", id));
    }

    let result = loop {
        let limit = match mode {
            Mode::Auto => NO_PROGRESS.min(AUTO_MAX.saturating_sub(started.elapsed())).max(Duration::from_secs(1)),
            _ => YOUR_TURN,
        };
        match tokio::time::timeout(limit, rx.recv()).await {
            // Pressing buttons isn't getting anywhere: hand the page to the user.
            Err(_) if mode == Mode::Auto => {
                mode = Mode::Manual;
                    manual.store(true, Ordering::Relaxed);
                show_page(&win, &page_title);
                engine.gate_status(id, Some(WAITING_PAGE));
                engine.notify_row(id, "needs-you", "Pako needs you", &format!("Finish opening a link from {gate_host} in the Pako window."), crate::notices::Go::item("found", id));
            }
            Err(_) => {
                let at = last_host.lock().unwrap().clone();
                break Err(format!("The link page wasn't finished (stopped at {at}). Paste the link again to retry."));
            }
            Ok(None) | Ok(Some(Msg::Closed)) => {
                break Err("The link page was closed before the file link was found. Paste it again to retry.".into());
            }
            Ok(Some(Msg::Final(file))) => break Ok(file),
            Ok(Some(Msg::Step)) => {
                if mode == Mode::Auto && started.elapsed() >= AUTO_MAX {
                    mode = Mode::Manual;
                    manual.store(true, Ordering::Relaxed);
                    show_page(&win, &page_title);
                    engine.gate_status(id, Some(WAITING_PAGE));
                    engine.notify_row(id, "needs-you", "Pako needs you", &format!("Finish opening a link from {gate_host} in the Pako window."), crate::notices::Go::item("found", id));
                }
            }
            Ok(Some(Msg::Show)) => match mode {
                // A check on screen: just bring it to the front (Pako carries on after it).
                Mode::Captcha => {
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
                _ => {
                    if mode == Mode::Auto {
                        mode = Mode::Manual;
                        manual.store(true, Ordering::Relaxed);
                        engine.gate_status(id, Some(WAITING_PAGE));
                    }
                    show_page(&win, &page_title);
                }
            },
            Ok(Some(Msg::Hop(host))) => {
                started = tokio::time::Instant::now();
                // Handed on to a site only a person can click through: the page is yours from here.
                if is_hand_over(&host) && mode != Mode::Manual {
                    mode = Mode::Manual;
                    manual.store(true, Ordering::Relaxed);
                    show_page(&win, &page_title);
                    engine.gate_status(id, Some(WAITING_PAGE));
                    engine.notify_row(id, "needs-you", "Pako needs you", &format!("{host} only lets a person click through: finish it in the Pako window."), crate::notices::Go::item("found", id));
                }
            }
            Ok(Some(Msg::Solved)) => {
                started = tokio::time::Instant::now();
                if mode == Mode::Captcha {
                    mode = Mode::Auto;
                    let _ = win.hide();
                    engine.gate_status(id, None);
                    crate::notices::resolve(app, "needs-you", id, "Solved");
                }
            }
            Ok(Some(Msg::Captcha)) => {
                if mode != Mode::Auto {
                    continue; // already on screen
                }
                // Some checks pass by themselves in a moment: don't flash a window for those.
                tokio::time::sleep(Duration::from_millis(2500)).await;
                let mut passed = false;
                while let Ok(m) = rx.try_recv() {
                    match m {
                        Msg::Solved => passed = true,
                        Msg::Final(file) => {
                            let _ = win.destroy();
                            return Ok(file);
                        }
                        Msg::Closed => return Err("The link page was closed.".into()),
                        _ => {}
                    }
                }
                if passed {
                    continue;
                }
                mode = Mode::Captcha;
                let _ = win.set_title(&format!("Pako · Solve this to continue · {gate_host}"));
                to_middle(&win);
                let _ = win.show();
                let _ = win.set_focus();
                engine.gate_status(id, Some(WAITING_FOR_YOU));
                engine.notify_row(id, "needs-you", "Pako needs you", &format!("Solve a quick check to open a link from {gate_host}."), crate::notices::Go::item("found", id));
            }
        }
    };
    let _ = win.destroy();
    result
}

/// Brings the link page to the front ("Show page" / "Show check" on the Found row). If Pako was
/// still pressing buttons by itself, the page is handed to you from here on.
pub fn focus_waiting(id: &str) {
    if let Some((current, tx)) = CURRENT.lock().unwrap().as_ref() {
        if current == id {
            let _ = tx.send(Msg::Show);
        }
    }
}

/// Start of a title the page's script sets to talk to Pako (an invisible character, then "pako:").
const SIGNAL: &str = "\u{2063}pako:";

/// The page is yours: the script stops pressing buttons and the "only the check" view is undone.
const MANUAL_JS: &str = "try{window.__pakoManual=true;sessionStorage.setItem('__pakoManual','1');var s=document.getElementById('pako-focus');if(s)s.remove();}catch(e){}";

/// Runs in the gate's page. It only presses the gate's own buttons (never images or frames,
/// where ads live), waits for countdowns to finish by itself (buttons stay disabled until then),
/// and reports to Pako through the page title.
const GATE_SCRIPT: &str = r#"
(() => {
  if (window.top !== window || window.__pakoGate) return;
  window.__pakoGate = true;
  const signal = (s) => {
    try {
      const before = document.title.startsWith('⁣pako:') ? '' : document.title;
      document.title = '⁣pako:' + s;
      setTimeout(() => { if (document.title.startsWith('⁣pako:')) document.title = before; }, 80);
    } catch (e) {}
  };
  const manual = () => { try { return !!window.__pakoManual || sessionStorage.getItem('__pakoManual') === '1'; } catch (e) { return !!window.__pakoManual; } };

  const CAPTCHA = '.cf-turnstile, .g-recaptcha:not([data-size="invisible"]), .h-captcha, [data-sitekey]:not([data-size="invisible"]):not(input):not(button), iframe[src*="challenges.cloudflare.com"], iframe[src*="google.com/recaptcha"]:not([src*="size=invisible"]), iframe[src*="recaptcha.net"]:not([src*="size=invisible"]), iframe[src*="hcaptcha.com"]';
  const TOKENS = 'input[name="cf-turnstile-response"], textarea[name="g-recaptcha-response"], textarea[name="h-captcha-response"]';
  const STEP = /^(continue|continue with ads|next|next step|proceed|click here to continue|i am not a robot|i'm not a robot|im not a robot|i'm a human|im a human|i am a human|i am human|i'm human|verify|skip|skip ad|skip ads|unlock|unlock link|unlock now|click to continue|click here to proceed|proceed to link|continue to next step|next page|free access with ads|continue for free|i understand|accept and continue|close ad|skip this ad)$/i;
  // The last button, the one that leaves for the file ("Go to your link", "Get link"...).
  const FINAL = /^((get|go to|open|visit|access|continue to) (the |your )?(download )?(link|file|download)|download link|go to download|your link|get your file|get download link|get link now|go to destination|continue to destination|open destination)$/i;

  const visible = (el) => {
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    return r.width > 4 && r.height > 4 && cs.visibility !== 'hidden' && cs.display !== 'none' && cs.opacity !== '0';
  };
  const enabled = (el) => !el.disabled && !el.classList.contains('disabled') && el.getAttribute('aria-disabled') !== 'true';
  const sameSite = (href) => { try { return new URL(href, location.href).host === location.host; } catch (e) { return false; } };
  const label = (el) => (el.innerText || el.value || '').replace(/[^\p{L}\p{N}' ]+/gu, ' ').replace(/\s+/g, ' ').trim();
  // The label a button will have once its countdown ends (some gates keep it in data-ready-label).
  const readyLabel = (el) => (el.dataset && el.dataset.readyLabel ? el.dataset.readyLabel.replace(/[^\p{L}\p{N}' ]+/gu, ' ').replace(/\s+/g, ' ').trim() : label(el));

  // Links stay in this window (where Pako watches); "Get link" pressed by you also tells Pako.
  document.addEventListener('click', (e) => {
    const el = e.target && e.target.closest ? e.target.closest('a, button, input[type=submit], input[type=button]') : null;
    if (!el) return;
    if (el.tagName === 'A' && el.target) el.target = '_self';
    if (!e.isTrusted) return;
    const href = el.tagName === 'A' ? el.getAttribute('href') || '' : '';
    const offSite = href && !href.startsWith('#') && !href.startsWith('javascript') && !sameSite(href);
    if (FINAL.test(label(el)) || (manual() && offSite)) signal('go');
  }, true);

  // A visible timer or "please wait" text: the gate is counting down by itself.
  const WAIT_TEXT = /(please wait|wait \d+|\d+\s*(s|sec|secs|seconds)\b|generating|redirecting|loading your link)/i;
  const counting = () => [...document.querySelectorAll('[id*=timer], [class*=timer], [id*=count], [class*=count], [id*=wait], [class*=wait]')]
    .some((el) => visible(el) && WAIT_TEXT.test((el.innerText || '').slice(0, 80)));

  let shown = false, solved = false, lastClick = 0, lastWait = 0;
  const loaded = Date.now();

  const focusCaptcha = (el) => {
    if (manual() || document.getElementById('pako-focus')) return;
    const box = el.closest('form') || el.parentElement || el;
    box.classList.add('pako-keep');
    const st = document.createElement('style');
    st.id = 'pako-focus';
    st.textContent = 'html,body{background:#0f1114!important;overflow:hidden!important}' +
      'body *{visibility:hidden!important}' +
      '.pako-keep,.pako-keep *{visibility:visible!important}' +
      '.pako-keep{position:fixed!important;left:50%!important;top:50%!important;transform:translate(-50%,-50%)!important;' +
      'z-index:2147483647!important;margin:0!important;max-width:92vw!important;background:#fff!important;' +
      'border-radius:12px!important;padding:18px!important;box-shadow:0 10px 40px rgba(0,0,0,.5)!important}';
    (document.head || document.documentElement).appendChild(st);
  };
  const unfocus = () => { const st = document.getElementById('pako-focus'); if (st) st.remove(); };

  const tick = () => {
    const cap = [...document.querySelectorAll(CAPTCHA)].find(visible);
    const token = [...document.querySelectorAll(TOKENS)].some((t) => (t.value || '').length > 10);
    if (cap && !token) {
      if (!shown) { shown = true; focusCaptcha(cap); signal('captcha'); }
      return; // the user's turn: nothing is pressed meanwhile
    }
    if (shown && !solved && (token || !cap)) { solved = true; unfocus(); signal('solved'); }
    if (manual()) return; // the page is yours: Pako only watches for the file link
    if (Date.now() - lastClick < 2500 || Date.now() - loaded < 1500) return;

    const all = [...document.querySelectorAll('button, input[type=submit], input[type=button], a')].filter((el) => {
      const text = readyLabel(el);
      if (!(STEP.test(text) || FINAL.test(text)) || !visible(el)) return false;
      // Links to other sites are ads, except the final "Get link".
      if (el.tagName === 'A') {
        const href = el.getAttribute('href') || '';
        if (href && !href.startsWith('#') && !href.startsWith('javascript') && !sameSite(href) && !FINAL.test(text)) return false;
      }
      return true;
    });
    const buttons = all.filter((el) => enabled(el) && (STEP.test(label(el)) || FINAL.test(label(el))));
    const btn = buttons.find((b) => FINAL.test(label(b))) || buttons[0];
    if (!btn) {
      // A countdown is running (its button still waiting, or a timer on the page): the page is alive.
      if ((all.length || counting()) && Date.now() - lastWait > 10000) { lastWait = Date.now(); signal('wait'); }
      return;
    }
    lastClick = Date.now();
    signal(FINAL.test(label(btn)) ? 'go' : 'step');
    setTimeout(() => btn.click(), 300);
  };

  const start = () => { setTimeout(tick, 1500); setInterval(tick, 900); };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start); else start();
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_gates_and_files() {
        assert!(is_known_gate("exeygo.com"));
        assert!(is_known_gate("www.exe.io"));
        assert!(!is_known_gate("google.com"));
        assert!(is_file_link(&"https://drive.google.com/file/d/abc/view".parse().unwrap()));
        assert!(is_file_link(&"https://example.org/files/game.part1.rar".parse().unwrap()));
        assert!(is_file_link(&"https://example.org/x/archive.7z.001".parse().unwrap()));
        assert!(!is_file_link(&"https://exeygo.com/fefwp2".parse().unwrap()));
        assert!(same_site("go.exeygo.com", "exeygo.com"));
        assert!(!same_site("drive.google.com", "exeygo.com"));
    }

    #[test]
    fn popular_gates_and_mirrors() {
        for h in ["ouo.press", "gplinks.co", "droplink.co", "shrinke.me", "sh.st", "gestyy.com", "cuty.me", "link-target.net", "lootdest.org", "work.ink", "www.linkvertise.com"] {
            assert!(is_known_gate(h), "{h} should be a known gate");
        }
        // Only a person can click these through: shown straight away.
        assert!(is_hand_over("linkvertise.com"));
        assert!(is_hand_over("direct-link.net"));
        assert!(is_hand_over("work.ink"));
        assert!(!is_hand_over("ouo.io"));
        assert!(!is_hand_over("exeygo.com"));
        // Every hand-over site is also a known gate (so pasted links open through the gate window).
        for h in HAND_OVER {
            assert!(is_known_gate(h), "{h} must also be in KNOWN_GATES");
        }
        assert!(is_file_link(&"https://terabox.com/s/abc".parse().unwrap()));
        assert!(!is_file_link(&"https://gplinks.co/xyz".parse().unwrap()));
    }
}
