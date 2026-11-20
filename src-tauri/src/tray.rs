// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Pako in the taskbar tray: one click shows a small list of the downloads, a double
//! click opens Pako, a right click gives a short menu. With "Minimize to tray" or
//! "Close to tray" on, Pako keeps downloading while its window is hidden.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

use crate::engine::Engine;

/// Size of the compact list, in logical pixels. The window is a little bigger and
/// see-through: the margin holds the list's shadow and room for its opening slide.
const MINI_W: f64 = 360.0;
const MINI_H: f64 = 440.0;
const MARGIN: f64 = 14.0;
/// After a double click, the click that ends it is ignored for this long.
const DOUBLE_CLICK: Duration = Duration::from_millis(600);

/// When the small list last closed because it lost focus. Clicking the tray icon takes the
/// focus first, so a click right after that is the click that closed it: don't reopen.
static LAST_BLUR: Mutex<Option<Instant>> = Mutex::new(None);

pub fn show_main(app: &AppHandle) {
    hide_mini(app);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Opens Pako on its Settings (from the tray menu or the compact list).
pub fn open_settings(app: &AppHandle) {
    show_main(app);
    let _ = app.emit_to("main", "open-settings", serde_json::Value::Null);
}

/// Quit, but ask first while downloads or uploads (Send) run (unless turned off in Settings):
/// the question shows in Pako's window, which opens if it was in the tray. It carries both
/// counts, so the window can say what would stop.
pub fn request_quit(app: &AppHandle) {
    use std::sync::atomic::Ordering;
    static ASKED: Mutex<Option<Instant>> = Mutex::new(None);
    let running = app.try_state::<Arc<Engine>>().map(|e| (e.running_count(), e.settings().confirm_quit));
    let uploading = crate::engine::UPLOADS_RUNNING.load(Ordering::Relaxed);
    // Files going to or from a contact in Direct.
    let direct = app.try_state::<crate::direct::Direct>().and_then(|d| d.node().ok()).is_some_and(|n| n.busy());
    // Quit asked again within 10 s: the question may not have shown (window not answering).
    let again = ASKED.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_secs(10));
    match running {
        Some((n, true)) if (n + uploading > 0 || direct) && !again => {
            *ASKED.lock().unwrap() = Some(Instant::now());
            show_main(app);
            let _ = app.emit_to("main", "confirm-quit", serde_json::json!({ "running": n, "uploading": uploading, "direct": direct }));
        }
        _ => {
            remember_place(app);
            app.exit(0);
        }
    }
}

/// Notes where the window is (and its size) for next time. Not while minimized; when
/// maximized, the size it had before is kept.
pub fn remember_place(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else { return };
    if w.is_minimized().unwrap_or(false) {
        return;
    }
    let maximized = w.is_maximized().unwrap_or(false);
    let mut place = crate::system::load_place().unwrap_or_default();
    place.maximized = maximized;
    if !maximized {
        if let (Ok(pos), Ok(size)) = (w.outer_position(), w.inner_size()) {
            place.x = pos.x;
            place.y = pos.y;
            place.w = size.width;
            place.h = size.height;
        }
    }
    crate::system::save_place(&place);
}

/// Saves the window's place a second after it stops moving or resizing (so a crash or a
/// forced close doesn't lose it).
pub fn remember_place_soon(app: &AppHandle) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static GEN: AtomicU64 = AtomicU64::new(0);
    let mine = GEN.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if GEN.load(Ordering::Relaxed) == mine {
            let visible = app.get_webview_window("main").is_some_and(|w| w.is_visible().unwrap_or(false));
            if visible {
                remember_place(&app);
            }
        }
    });
}

/// Puts the window back where it was last time, if that spot is still on a screen.
pub fn restore_place(app: &AppHandle) {
    let (Some(w), Some(p)) = (app.get_webview_window("main"), crate::system::load_place()) else { return };
    if p.w >= 600 && p.h >= 400 {
        // The title bar's middle must be on a screen (a monitor may have been unplugged).
        let (cx, cy) = (p.x + p.w as i32 / 2, p.y + 16);
        let on_screen = app.available_monitors().unwrap_or_default().iter().any(|m| {
            let (mx, my) = (m.position().x, m.position().y);
            let (mw, mh) = (m.size().width as i32, m.size().height as i32);
            cx >= mx && cx < mx + mw && cy >= my && cy < my + mh
        });
        if on_screen {
            let _ = w.set_size(tauri::PhysicalSize::new(p.w, p.h));
            let _ = w.set_position(PhysicalPosition::new(p.x, p.y));
        }
    }
    if p.maximized {
        let _ = w.maximize();
    }
}

pub fn hide_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

fn hide_mini(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("mini") {
        hide_window(&w);
    }
}

/// Hides the small window, also when it was opened without taking the focus: that opening goes
/// straight to Windows, so the window library still thinks it is hidden and its own hide() would
/// do nothing. Windows is told directly as well.
pub(crate) fn hide_window(w: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(h) = w.hwnd() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
        // SAFETY: the window's own handle, valid while `w` lives.
        unsafe {
            ShowWindow(h.0 as _, SW_HIDE);
        }
    }
    let _ = w.hide();
}

/// The small window's minimise button.
pub fn hide_mini_now(app: &AppHandle) {
    hide_mini(app);
}

/// The small window is on screen right now (shown, not hidden or minimized, and not left on
/// another of Windows' desktops). Asked of Windows itself: opened without the focus, the window
/// library still thinks it is hidden. Any thread may ask (these calls only read the window).
pub(crate) fn mini_showing(app: &AppHandle) -> bool {
    let Some(w) = app.get_webview_window("mini") else { return false };
    #[cfg(windows)]
    if let Ok(h) = w.hwnd() {
        use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
        use windows_sys::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible};
        let hwnd = h.0 as _;
        // SAFETY: the window's own handle, valid while `w` lives; `cloaked` is a u32 as
        // DWMWA_CLOAKED asks, and its size is passed.
        unsafe {
            if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                return false;
            }
            // Pinned on another virtual desktop: "visible" to Windows, but hidden ("cloaked")
            // from you, so a Download finished pop-up is still wanted.
            let mut cloaked: u32 = 0;
            let ok = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as u32, (&mut cloaked as *mut u32).cast(), 4) >= 0;
            return !(ok && cloaked != 0);
        }
    }
    w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false)
}

/// Pinned: the small window stays open and on top, wherever it was moved to.
fn is_pinned(app: &AppHandle) -> bool {
    app.try_state::<Arc<Engine>>().is_some_and(|e| e.settings().mini_pinned)
}

/// The pin was switched (the setting is already saved): pinning keeps the window where it is
/// from now on.
pub fn mini_pinned(app: &AppHandle, on: bool) {
    if on {
        if let Some(w) = app.get_webview_window("mini") {
            if let Ok(p) = w.outer_position() {
                save_mini_pos(app, p.x, p.y);
            }
        }
    }
}

fn save_mini_pos(app: &AppHandle, x: i32, y: i32) {
    if let Some(e) = app.try_state::<Arc<Engine>>() {
        let _ = e.change_settings(|s| s.mini_pos = Some((x, y)));
    }
}

/// Where the pinned window was left, if that spot is still on a screen.
fn saved_mini_pos(app: &AppHandle) -> Option<PhysicalPosition<f64>> {
    let (x, y) = app.try_state::<Arc<Engine>>()?.settings().mini_pos?;
    let on_screen = app
        .available_monitors()
        .ok()?
        .iter()
        .any(|m| {
            let (mx, my) = (m.position().x, m.position().y);
            let (mw, mh) = (m.size().width as i32, m.size().height as i32);
            x + 40 >= mx && x < mx + mw - 40 && y >= my && y < my + mh - 40
        });
    on_screen.then(|| PhysicalPosition::new(x as f64, y as f64))
}

/// Opens the small window without taking the focus (a download from the browser just started:
/// you stay in your browser). Bottom right above the taskbar, or where it was pinned.
#[cfg_attr(test, allow(dead_code))]
pub fn show_mini_quietly(app: &AppHandle) {
    if !crate::consent::ok() {
        return;
    }
    let Some(w) = mini_window(app) else { return };
    if w.is_visible().unwrap_or(false) {
        return;
    }
    let pos = saved_mini_pos(app).or_else(|| {
        let m = app.primary_monitor().ok().flatten()?;
        let scale = m.scale_factor();
        let area = m.work_area();
        let (ww, wh) = ((MINI_W + MARGIN * 2.0) * scale, (MINI_H + MARGIN * 2.0) * scale);
        let x = area.position.x as f64 + area.size.width as f64 - ww;
        let y = area.position.y as f64 + area.size.height as f64 - wh;
        Some(PhysicalPosition::new(x, y))
    });
    if let Some(p) = pos {
        let _ = w.set_position(p);
    }
    let _ = app.emit_to("mini", "mini-show", serde_json::json!({ "from": "bottom" }));
    show_without_focus(&w);
    if let Some(p) = pos {
        stay_put(app, &w, p);
    }
}

#[cfg(windows)]
pub(crate) fn show_without_focus(w: &WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SW_SHOWNOACTIVATE, ShowWindow};
    match w.hwnd() {
        // SAFETY: the window's own handle, valid while `w` lives.
        Ok(h) => unsafe {
            ShowWindow(h.0 as _, SW_SHOWNOACTIVATE);
        },
        Err(_) => {
            let _ = w.show();
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn show_without_focus(w: &WebviewWindow) {
    let _ = w.show();
}

/// The compact list's window, made once (hidden) so it opens instantly when clicked.
fn mini_window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(w) = app.get_webview_window("mini") {
        return Some(w);
    }
    let w = WebviewWindowBuilder::new(app, "mini", WebviewUrl::App("mini".into()))
        .title("Pako")
        .inner_size(MINI_W + MARGIN * 2.0, MINI_H + MARGIN * 2.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .build()
        .ok()?;
    keep_out_of_zones(&w);
    // Clicking anywhere else closes it, like any tray popup (unless it is pinned). A pinned
    // one remembers where it is moved to.
    let me = w.clone();
    let handle = app.clone();
    w.on_window_event(move |e| match e {
        WindowEvent::Focused(false) => {
            if !is_pinned(&handle) {
                *LAST_BLUR.lock().unwrap() = Some(Instant::now());
                hide_window(&me);
            }
        }
        WindowEvent::Moved(p) => {
            if is_pinned(&handle) && me.is_visible().unwrap_or(false) {
                // Saved once the dragging stops.
                use std::sync::atomic::{AtomicU64, Ordering};
                static MOVES: AtomicU64 = AtomicU64::new(0);
                let n = MOVES.fetch_add(1, Ordering::Relaxed) + 1;
                let (x, y, app) = (p.x, p.y, handle.clone());
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(600));
                    if MOVES.load(Ordering::Relaxed) == n {
                        save_mini_pos(&app, x, y);
                    }
                });
            }
        }
        _ => {}
    });
    Some(w)
}

/// A tray popup, not an app window: window managers such as PowerToys FancyZones ("move newly
/// created windows to their last known zone") leave tool windows alone. It also keeps the
/// window out of Alt+Tab, like other tray popups.
///
/// Setting the style once is not enough: the window library rewrites the whole extended style
/// each time it shows or hides the window (or changes any of its flags), which would drop it
/// again just before the window shows. A hook on the window keeps it in every rewrite.
pub(crate) fn keep_out_of_zones(w: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(h) = w.hwnd() {
        let hwnd = h.0 as isize;
        // SAFETY: the window's own handle, valid while `w` lives. The hook can only be put on
        // from the window's own thread (the main one): from elsewhere, the style is set now and
        // the hook goes on there.
        if !unsafe { tool_window::make(hwnd as _) } {
            let _ = w.run_on_main_thread(move || unsafe {
                tool_window::make(hwnd as _);
            });
        }
    }
    #[cfg(not(windows))]
    let _ = w;
}

/// A dialog-like window (Download with Pako) that stays an app window (on the taskbar, in
/// Alt+Tab) but which window managers leave alone: PowerToys FancyZones doesn't snap a popup
/// window that has no resizable frame with minimize and maximize buttons. Pako's dialogs have
/// none of those, so marking them as popups is enough. Kept through the window library's style
/// rewrites by the same hook as `keep_out_of_zones`; on Windows 11 its corners stay rounded.
pub(crate) fn keep_dialog_out_of_zones(w: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(h) = w.hwnd() {
        let hwnd = h.0 as isize;
        // SAFETY: as in `keep_out_of_zones`.
        if !unsafe { tool_window::make_popup(hwnd as _) } {
            let _ = w.run_on_main_thread(move || unsafe {
                tool_window::make_popup(hwnd as _);
            });
        }
    }
    #[cfg(not(windows))]
    let _ = w;
}

/// Per window (by its handle): bumped to stop its running `hold_place` (the window opening
/// again, or you dragging it). One window's turn never ends another's.
static STAY_PUT: std::sync::LazyLock<Mutex<std::collections::HashMap<isize, u64>>> = std::sync::LazyLock::new(Default::default);

/// A new turn for the window `key`; returns it.
fn stay_put_bump(key: isize) -> u64 {
    let mut all = STAY_PUT.lock().unwrap();
    let n = all.entry(key).or_insert(0);
    *n += 1;
    *n
}
fn stay_put_turn(key: isize) -> u64 {
    STAY_PUT.lock().unwrap().get(&key).copied().unwrap_or(0)
}
/// The window's key for `STAY_PUT` (its handle on Windows).
fn stay_key(w: &WebviewWindow) -> isize {
    #[cfg(windows)]
    {
        w.hwnd().map(|h| h.0 as isize).unwrap_or(0)
    }
    #[cfg(not(windows))]
    {
        let _ = w;
        0
    }
}

#[cfg(windows)]
mod tool_window {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GWL_STYLE, GetWindowLongW, STYLESTRUCT, SetWindowLongW, WM_ENTERSIZEMOVE, WM_NCDESTROY,
        WM_STYLECHANGING, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_POPUP,
    };

    const ID: usize = 0x5041_4b4f;
    /// What the hook keeps (its reference data): a tool window, or a popup (dialog) window.
    const TOOL: usize = 0;
    const POPUP: usize = 1;

    /// A tool window's extended style: not an app window (taskbar, Alt+Tab, window managers).
    pub(super) fn style(ex: u32) -> u32 {
        (ex | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW
    }

    /// Makes `hwnd` a tool window for good. The style is set from any thread (32-bit calls: the
    /// extended style fits); the hook only from the window's own thread: false otherwise.
    ///
    /// # Safety
    /// `hwnd` is a live window.
    pub(super) unsafe fn make(hwnd: HWND) -> bool {
        unsafe {
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, style(ex) as i32);
            SetWindowSubclass(hwnd, Some(hook), ID, TOOL) != 0
        }
    }

    /// Makes `hwnd` a popup window for good (style from any thread, hook only from its own:
    /// false otherwise), with rounded corners on Windows 11 as an app window has.
    ///
    /// # Safety
    /// `hwnd` is a live window.
    pub(super) unsafe fn make_popup(hwnd: HWND) -> bool {
        use windows_sys::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};
        unsafe {
            let st = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            SetWindowLongW(hwnd, GWL_STYLE, (st | WS_POPUP) as i32);
            // Windows 10 doesn't know the attribute and says so: nothing to do there.
            let round = DWMWCP_ROUND;
            DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &round as *const _ as _, 4);
            SetWindowSubclass(hwnd, Some(hook), ID, POPUP) != 0
        }
    }

    unsafe extern "system" fn hook(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, id: usize, kind: usize) -> LRESULT {
        match msg {
            // SAFETY: for GWL_EXSTYLE and GWL_STYLE, lParam is the STYLESTRUCT Windows is about
            // to apply.
            WM_STYLECHANGING if kind == TOOL && wp as i32 == GWL_EXSTYLE => {
                if let Some(s) = unsafe { (lp as *mut STYLESTRUCT).as_mut() } {
                    s.styleNew = style(s.styleNew);
                }
            }
            WM_STYLECHANGING if kind == POPUP && wp as i32 == GWL_STYLE => {
                if let Some(s) = unsafe { (lp as *mut STYLESTRUCT).as_mut() } {
                    s.styleNew |= WS_POPUP;
                }
            }
            // You started dragging it: wherever you put it is where it goes.
            WM_ENTERSIZEMOVE => {
                super::stay_put_bump(hwnd as isize);
            }
            WM_NCDESTROY => {
                // SAFETY: our own hook, taken off the window it is on.
                unsafe { RemoveWindowSubclass(hwnd, Some(hook), id) };
            }
            _ => {}
        }
        // SAFETY: the message as Windows passed it, on to the window's own procedure.
        unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
    }
}

/// Puts the small window back where Pako placed it if something moved it just after it showed
/// (a window manager snapping it into a zone). Pinned windows can be moved by you, so only an
/// unpinned one is checked, a few times in its first half second; it stops as soon as the
/// window opens again or you start dragging it.
fn stay_put(app: &AppHandle, w: &WebviewWindow, p: PhysicalPosition<f64>) {
    let app = app.clone();
    hold_place(w, p, move || !is_pinned(&app) && mini_showing(&app));
}

/// Puts a window back at `p` if something moves it in its first half second on screen (a window
/// manager snapping it into a zone), while `still` holds. Stops as soon as a window held this
/// way opens again or you start dragging it (the hook of `keep_out_of_zones` /
/// `keep_dialog_out_of_zones` sees the drag start).
pub(crate) fn hold_place(w: &WebviewWindow, p: PhysicalPosition<f64>, still: impl Fn() -> bool + Send + 'static) {
    let key = stay_key(w);
    let turn = stay_put_bump(key);
    let w = w.clone();
    std::thread::spawn(move || {
        for wait in [60u64, 140, 300] {
            std::thread::sleep(Duration::from_millis(wait));
            if stay_put_turn(key) != turn || !still() {
                return;
            }
            let Ok(now) = w.outer_position() else { return };
            if moved_away(now.x, now.y, p) {
                let _ = w.set_position(p);
            }
        }
    });
}

/// Where a window `width` × `height` (logical pixels) sits in the middle of the screen the mouse
/// is on (the main screen if that can't be told), a little above the centre like a dialog.
pub(crate) fn middle_of_pointer_screen(app: &AppHandle, width: f64, height: f64) -> Option<PhysicalPosition<f64>> {
    let m = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let area = m.work_area();
    Some(middle_of(
        (area.position.x as f64, area.position.y as f64),
        (area.size.width as f64, area.size.height as f64),
        (width * m.scale_factor(), height * m.scale_factor()),
    ))
}

/// The top left corner that puts a window of `size` in the middle of the screen area at `at`
/// with `area` (all physical pixels), a little above the centre; never off its top left.
fn middle_of(at: (f64, f64), area: (f64, f64), size: (f64, f64)) -> PhysicalPosition<f64> {
    let x = at.0 + ((area.0 - size.0) / 2.0).max(0.0);
    let y = at.1 + ((area.1 - size.1) / 2.5).max(0.0);
    PhysicalPosition::new(x.round(), y.round())
}

/// More than a couple of pixels from where it was put (rounding of scaled positions aside).
fn moved_away(x: i32, y: i32, p: PhysicalPosition<f64>) -> bool {
    (x as f64 - p.x).abs() > 2.0 || (y as f64 - p.y).abs() > 2.0
}

/// Shows the compact list next to the tray icon (or hides it if it is open).
fn toggle_mini(app: &AppHandle, near: PhysicalPosition<f64>) {
    let Some(w) = mini_window(app) else { return };
    if w.is_visible().unwrap_or(false) {
        hide_window(&w);
        return;
    }
    if LAST_BLUR.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(800)) {
        return;
    }
    // Pinned and moved somewhere: it opens there.
    if is_pinned(app) {
        if let Some(p) = saved_mini_pos(app) {
            let _ = w.set_position(p);
            let _ = app.emit_to("mini", "mini-show", serde_json::json!({ "from": "bottom" }));
            let _ = w.show();
            let _ = w.set_focus();
            return;
        }
    }
    // Above the tray icon (taskbar at the bottom), or below it (taskbar at the top).
    let mut from = "bottom";
    let mut placed = None;
    if let Ok(Some(m)) = app.monitor_from_point(near.x, near.y) {
        let scale = m.scale_factor();
        let (ww, wh) = ((MINI_W + MARGIN * 2.0) * scale, (MINI_H + MARGIN * 2.0) * scale);
        let (mx, my) = (m.position().x as f64, m.position().y as f64);
        let (sw, sh) = (m.size().width as f64, m.size().height as f64);
        // A screen narrower than the list: its left edge (clamp needs min <= max).
        let x = (near.x - ww / 2.0).clamp(mx, (mx + sw - ww).max(mx));
        let below = near.y < my + sh / 2.0;
        if below {
            from = "top";
        }
        let y = if below { near.y + 4.0 } else { near.y - wh - 4.0 };
        placed = Some(PhysicalPosition::new(x, y.max(my)));
    }
    if let Some(p) = placed {
        let _ = w.set_position(p);
    }
    // The list plays its opening slide, from the side the taskbar is on.
    let _ = app.emit_to("mini", "mini-show", serde_json::json!({ "from": from }));
    let _ = w.show();
    let _ = w.set_focus();
    if let Some(p) = placed {
        stay_put(app, &w, p);
    }
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Pako", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause all", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Resume all", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Pako", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &settings, &sep1, &pause, &resume, &sep2, &quit])?;

    let last_double: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));

    // Terms of use still to accept (a Pako just updated in the tray): said quietly here; the
    // window opens on them when the user opens Pako or the browser sends it something.
    let mut tray = TrayIconBuilder::with_id("pako")
        .tooltip(if crate::consent::ok() { "Pako" } else { "Pako: open it to continue" })
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| {
            let engine = app.try_state::<Arc<Engine>>();
            // The terms of use wait: the tray only opens Pako (on them) or quits it.
            if !crate::consent::ok() && e.id().as_ref() != "quit" {
                show_main(app);
                return;
            }
            match e.id().as_ref() {
                "open" => show_main(app),
                "settings" => open_settings(app),
                "pause" => {
                    if let Some(en) = engine {
                        en.pause_all();
                    }
                }
                "resume" => {
                    if let Some(en) = engine {
                        en.resume_all();
                    }
                }
                "quit" => request_quit(app),
                _ => {}
            }
        })
        .on_tray_icon_event(move |tray, event| {
            let app = tray.app_handle().clone();
            match event {
                // One click: the list opens straight away (no waiting to see if a second
                // click comes). A double click then opens Pako and the list closes.
                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, position, .. } => {
                    if last_double.lock().unwrap().is_some_and(|t| t.elapsed() < DOUBLE_CLICK) {
                        return;
                    }
                    // The terms of use wait: Pako opens on them instead of the list.
                    if !crate::consent::ok() {
                        show_main(&app);
                        return;
                    }
                    toggle_mini(&app, position);
                }
                TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => {
                    *last_double.lock().unwrap() = Some(Instant::now());
                    show_main(&app);
                }
                _ => {}
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
        let plain = icon.clone().to_owned();
        let dotted = with_dot(&plain);
        let _ = ICONS.set((plain, dotted));
    }
    tray.build(app)?;
    // Ready in advance, hidden: the first click doesn't wait for it to load.
    mini_window(app.handle());
    crate::notices::make_window(app.handle());
    crate::notices::make_important_window(app.handle());
    // The dot, if the bell has something unread from last time.
    crate::notices::start(app.handle());
    Ok(())
}

/// The tray icon as it is, and with the bell's dot.
static ICONS: std::sync::OnceLock<(Image<'static>, Image<'static>)> = std::sync::OnceLock::new();
/// The dot the bell wants (set by notices.rs under its history's lock, so the last change wins).
static DOT_WANTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The dot shown right now (only the main thread changes it).
static DOT_SHOWN: Mutex<bool> = Mutex::new(false);

/// Notes whether the bell has something unread. Cheap and never touches the tray: safe under
/// any lock. `show_dot` then puts it on the icon.
pub fn want_dot(on: bool) {
    DOT_WANTED.store(on, std::sync::atomic::Ordering::SeqCst);
}

/// A dot on the tray icon while the bell has something unread: the latest wanted state is
/// applied on the main thread (never waits for it), so changes from other threads can't land
/// in the wrong order.
pub fn show_dot(app: &AppHandle) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some((plain, dotted)) = ICONS.get() else { return };
        let on = DOT_WANTED.load(std::sync::atomic::Ordering::SeqCst);
        {
            let mut shown = DOT_SHOWN.lock().unwrap();
            if *shown == on {
                return;
            }
            *shown = on;
        }
        if let Some(tray) = a.tray_by_id("pako") {
            let _ = tray.set_icon(Some(if on { dotted.clone() } else { plain.clone() }));
            let _ = tray.set_tooltip(Some(if on { "Pako: new notifications" } else { "Pako" }));
        }
    });
}

/// A copy of the icon with Pako's red dot in its top right corner, in a thin see-through ring
/// so it stands out on the icon (drawn smooth: edge pixels partly covered).
fn with_dot(icon: &Image<'_>) -> Image<'static> {
    let (w, h) = (icon.width() as usize, icon.height() as usize);
    let mut px = icon.rgba().to_vec();
    let size = w.min(h) as f64;
    let r = size * 0.22;
    let ring = size * 0.07;
    let (cx, cy) = (w as f64 - r - size * 0.02, r + size * 0.02);
    // Pako's red (--accent).
    let red = [0xf9u8, 0x62, 0x63];
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
            let i = (y * w + x) * 4;
            if i + 3 >= px.len() {
                continue;
            }
            // Cut the ring out of the icon, then lay the dot on top.
            let cut = (r + ring + 0.5 - d).clamp(0.0, 1.0);
            let a = px[i + 3] as f64 * (1.0 - cut);
            let dot = (r + 0.5 - d).clamp(0.0, 1.0);
            let out_a = dot * 255.0 + a * (1.0 - dot);
            for c in 0..3 {
                let under = px[i + c] as f64 * a;
                let v = if out_a > 0.0 { (red[c] as f64 * dot * 255.0 + under * (1.0 - dot)) / out_a } else { 0.0 };
                px[i + c] = v.round().clamp(0.0, 255.0) as u8;
            }
            px[i + 3] = out_a.round().clamp(0.0, 255.0) as u8;
        }
    }
    Image::new_owned(px, w as u32, h as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_snapped_elsewhere_is_put_back_but_rounding_is_not_a_move() {
        let p = PhysicalPosition::new(1500.0, 800.0);
        assert!(!moved_away(1500, 800, p));
        assert!(!moved_away(1501, 799, p), "scaled positions round by a pixel");
        assert!(moved_away(0, 0, p), "snapped into a zone at the top left");
        assert!(moved_away(1500, 400, p));
    }

    #[test]
    fn a_dialog_opens_in_the_middle_of_the_screen_it_is_on() {
        // A second screen to the right of the main one, its taskbar taking the bottom.
        let p = middle_of((1920.0, 0.0), (2560.0, 1400.0), (630.0, 486.0));
        assert_eq!((p.x, p.y), (1920.0 + 965.0, 366.0), "centred, a little above the middle");
        // A window bigger than the screen: its top left corner stays on it.
        let p = middle_of((-1280.0, 0.0), (800.0, 600.0), (1000.0, 700.0));
        assert_eq!((p.x, p.y), (-1280.0, 0.0));
    }

    /// Download with Pako stays an app window (taskbar, Alt+Tab) but a popup one, which
    /// FancyZones leaves alone; the window library's style rewrites can't drop that.
    #[cfg(windows)]
    #[test]
    fn a_dialog_stays_a_popup_window_on_the_taskbar_through_a_rewrite() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GWL_EXSTYLE, GWL_STYLE, GetWindowLongW, SetWindowLongW, WS_CLIPSIBLINGS,
            WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_POPUP, WS_SYSMENU,
        };
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        // SAFETY: a hidden window of this thread, destroyed at the end; only its style is used.
        unsafe {
            let hwnd = CreateWindowExW(WS_EX_APPWINDOW, class.as_ptr(), std::ptr::null(), WS_SYSMENU, 0, 0, 10, 10, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null());
            assert!(!hwnd.is_null());
            let st = || GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let ex = || GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            assert!(tool_window::make_popup(hwnd));
            assert_ne!(st() & WS_POPUP, 0);
            // What the window library does on show(): the whole style, computed from its flags.
            SetWindowLongW(hwnd, GWL_STYLE, (WS_SYSMENU | WS_CLIPSIBLINGS) as i32);
            assert_ne!(st() & WS_POPUP, 0, "kept a popup window");
            assert_ne!(st() & WS_CLIPSIBLINGS, 0, "the rest of the rewrite applies");
            SetWindowLongW(hwnd, GWL_EXSTYLE, WS_EX_APPWINDOW as i32);
            assert_eq!(ex() & (WS_EX_TOOLWINDOW | WS_EX_APPWINDOW), WS_EX_APPWINDOW, "still on the taskbar");
            DestroyWindow(hwnd);
        }
    }

    /// The window library rewrites the whole extended style on each show and hide: the hook
    /// keeps the tool window style in every rewrite, and a drag stops `stay_put`.
    #[cfg(windows)]
    #[test]
    fn the_tool_window_style_survives_a_rewrite_and_a_drag_stops_stay_put() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GWL_EXSTYLE, GetWindowLongW, PM_NOREMOVE, PeekMessageW, SendMessageW, SetWindowLongW,
            WM_ENTERSIZEMOVE, WS_EX_ACCEPTFILES, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        };
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        // SAFETY: a hidden window of this thread, destroyed at the end; only its style is used.
        unsafe {
            let hwnd = CreateWindowExW(WS_EX_APPWINDOW, class.as_ptr(), std::ptr::null(), 0, 0, 0, 10, 10, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null());
            assert!(!hwnd.is_null());
            let ex = || GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            // From another thread: the style is set, the hook can't go on (the main thread does it).
            // Windows hands that thread's style change to this one, so this one keeps taking
            // messages meanwhile (as the main thread does), within a time limit.
            let h = hwnd as isize;
            let t = std::thread::spawn(move || tool_window::make(h as _));
            let start = Instant::now();
            let mut msg = std::mem::zeroed();
            while !t.is_finished() && start.elapsed() < Duration::from_secs(5) {
                PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(t.is_finished(), "the style change from another thread went through");
            assert!(!t.join().unwrap());
            assert_eq!(ex() & (WS_EX_TOOLWINDOW | WS_EX_APPWINDOW), WS_EX_TOOLWINDOW);
            assert!(tool_window::make(hwnd));
            // What the window library does on show(): the whole style, computed from its flags.
            SetWindowLongW(hwnd, GWL_EXSTYLE, (WS_EX_APPWINDOW | WS_EX_ACCEPTFILES) as i32);
            assert_eq!(ex() & (WS_EX_TOOLWINDOW | WS_EX_APPWINDOW), WS_EX_TOOLWINDOW, "kept a tool window");
            assert_ne!(ex() & WS_EX_ACCEPTFILES, 0, "the rest of the rewrite applies");
            let before = stay_put_turn(hwnd as isize);
            let other = stay_put_bump(hwnd as isize + 1);
            SendMessageW(hwnd, WM_ENTERSIZEMOVE, 0, 0);
            assert_ne!(stay_put_turn(hwnd as isize), before, "dragging stops stay_put");
            assert_eq!(stay_put_turn(hwnd as isize + 1), other, "another window's turn goes on");
            DestroyWindow(hwnd);
        }
    }
}
