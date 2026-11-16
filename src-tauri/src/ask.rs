// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! The "Download with Pako" window: a download that comes from the browser (the video button,
//! the right-click items, a download the browser started) waits here for your OK, with its name,
//! folder, quality and "Notify me". Several at once are shown one after another ("1 of 3").
//! Switched off in Settings (or with "Don't show this again"), they start right away.

use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::engine::Engine;
use crate::model::{AskAnswer, AskView};

const LABEL: &str = "ask";
const WIDTH: f64 = 420.0;
const HEIGHT: f64 = 324.0;

static APP: OnceLock<AppHandle> = OnceLock::new();
/// Downloads waiting for an answer, in the order they came.
static ORDER: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn engine() -> Option<Arc<Engine>> {
    APP.get()?.try_state::<Arc<Engine>>().map(|e| e.inner().clone())
}

/// At start: downloads still waiting from last time are asked about again.
pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
    let Some(engine) = engine() else { return };
    let waiting = engine.asking();
    if waiting.is_empty() {
        return;
    }
    ORDER.lock().unwrap().extend(waiting);
    show();
}

/// Whether browser downloads wait for the window (else they start right away).
pub fn wanted() -> bool {
    engine().is_some_and(|e| e.settings().ask_browser)
}

/// A download from the browser (already marked as waiting): shows the window for it.
pub fn enqueue(id: &str) {
    {
        let mut order = ORDER.lock().unwrap();
        if !order.iter().any(|x| x == id) {
            order.push(id.to_string());
        }
    }
    show();
}

/// A download from the browser started (no window, or it was answered): the small window
/// opens by itself when that setting is on.
pub fn started() {
    if let Some(app) = APP.get() {
        if engine().is_some_and(|e| e.settings().mini_on_browser) {
            crate::tray::show_mini_quietly(app);
        }
    }
}

/// Keeps only downloads that still wait (one removed in the main window drops out here too).
fn prune(engine: &Engine) -> Vec<String> {
    let waiting = engine.asking();
    let mut order = ORDER.lock().unwrap();
    order.retain(|id| waiting.contains(id));
    // Waiting ones the list doesn't know yet (e.g. marked before this was told): at the end.
    for id in waiting {
        if !order.contains(&id) {
            order.push(id);
        }
    }
    order.clone()
}

/// The download the window shows now.
pub fn current() -> Option<AskView> {
    let engine = engine()?;
    let order = prune(&engine);
    let id = order.first()?;
    engine.ask_view(id, 1, order.len())
}

/// The window's answer; then the next one, or the window closes.
pub fn answer(a: AskAnswer) -> Result<(), String> {
    let engine = engine().ok_or("Pako is still starting.")?;
    let action = a.action.clone();
    let id = a.id.clone();
    engine.answer_ask(a).map_err(|e| e.to_string())?;
    ORDER.lock().unwrap().retain(|x| *x != id);
    if action != "cancel" {
        started();
    }
    if prune(&engine).is_empty() {
        hide();
    } else if let Some(app) = APP.get() {
        let _ = app.emit_to(LABEL, "ask-changed", ());
    }
    Ok(())
}

fn hide() {
    if let Some(w) = APP.get().and_then(|a| a.get_webview_window(LABEL)) {
        let _ = w.hide();
    }
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(w) = app.get_webview_window(LABEL) {
        return Some(w);
    }
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("ask".into()))
        .title("Download with Pako")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .always_on_top(true)
        // Light or dark, from the start (no flash while the page loads).
        .background_color(crate::theme::background(app))
        .visible(false)
        .build()
        .ok()
}

/// Opens the window in front, in the middle of the screen the mouse is on.
fn show() {
    let Some(app) = APP.get() else { return };
    let Some(w) = window(app) else { return };
    if !w.is_visible().unwrap_or(false) {
        let monitor = app
            .cursor_position()
            .ok()
            .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
            .or_else(|| app.primary_monitor().ok().flatten());
        if let Some(m) = monitor {
            let scale = m.scale_factor();
            let (ww, wh) = (WIDTH * scale, HEIGHT * scale);
            let (mx, my) = (m.position().x as f64, m.position().y as f64);
            let (sw, sh) = (m.size().width as f64, m.size().height as f64);
            let x = mx + ((sw - ww) / 2.0).max(0.0);
            let y = my + ((sh - wh) / 2.5).max(0.0);
            let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
        }
    }
    let _ = app.emit_to(LABEL, "ask-changed", ());
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
}
