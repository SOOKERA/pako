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
    // Quit asked again within 10 s: the question may not have shown (window not answering).
    let again = ASKED.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_secs(10));
    match running {
        Some((n, true)) if n + uploading > 0 && !again => {
            *ASKED.lock().unwrap() = Some(Instant::now());
            show_main(app);
            let _ = app.emit_to("main", "confirm-quit", serde_json::json!({ "running": n, "uploading": uploading }));
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
pub fn show_mini_quietly(app: &AppHandle) {
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
        let _ = w.set_position(PhysicalPosition::new(x, y.max(my)));
    }
    // The list plays its opening slide, from the side the taskbar is on.
    let _ = app.emit_to("mini", "mini-show", serde_json::json!({ "from": from }));
    let _ = w.show();
    let _ = w.set_focus();
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

    let mut tray = TrayIconBuilder::with_id("pako")
        .tooltip("Pako")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| {
            let engine = app.try_state::<Arc<Engine>>();
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
