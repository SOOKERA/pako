// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Light or dark (Settings › Window › Theme, and the sun/moon button in the title bar).
//!
//! The choice is saved in the settings ("dark", "light" or "system" = follow Windows' app
//! mode). Each window shows it itself (see `src/lib/theme.svelte.ts`); here the windows that
//! have their own background (Pako's window and Download with Pako) get the matching colour,
//! so nothing flashes dark or white while a page loads or the window is resized, and every
//! window is told when the choice changes.
use std::sync::Arc;

use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::engine::Engine;

/// The pages' own background (`--bg` in `src/app.css`), dark and light.
const DARK: Color = Color(0x0f, 0x11, 0x14, 0xff);
const LIGHT: Color = Color(0xff, 0xff, 0xff, 0xff);

/// One of the three known choices (anything else, from an older or edited file: dark).
pub fn choice(saved: &str) -> &'static str {
    match saved {
        "light" => "light",
        "system" => "system",
        _ => "dark",
    }
}

/// The colour behind the pages for the saved choice. "Match Windows" asks Windows (through
/// Pako's window, which follows its app mode).
pub fn background(app: &AppHandle) -> Color {
    let saved = app.try_state::<Arc<Engine>>().map(|e| e.settings().theme).unwrap_or_default();
    let light = match choice(&saved) {
        "light" => true,
        "system" => app.get_webview_window("main").and_then(|w| w.theme().ok()) == Some(tauri::Theme::Light),
        _ => false,
    };
    if light { LIGHT } else { DARK }
}

fn paint(app: &AppHandle, color: Color) {
    for label in ["main", "ask"] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.set_background_color(Some(color));
        }
    }
}

/// At start, before Pako's window shows: its background matches the saved choice.
pub fn paint_start(app: &AppHandle) {
    paint(app, background(app));
}

/// The saved choice, for the windows that don't read the settings (pop-ups, Download with Pako).
#[tauri::command]
pub fn theme_get(engine: State<'_, Arc<Engine>>) -> String {
    choice(&engine.settings().theme).into()
}

/// Saves the choice and tells every window. Settings' Save comes here too: the settings
/// themselves never change it (an older copy can't put the previous one back).
#[tauri::command]
pub fn theme_set(app: AppHandle, engine: State<'_, Arc<Engine>>, theme: String) -> Result<(), String> {
    let theme = choice(&theme);
    let saved = engine.change_settings(|s| s.theme = theme.into());
    // Shown even if saving failed (it then lasts until Pako closes).
    let _ = app.emit("theme-changed", theme);
    saved.map_err(|e| e.to_string())
}

/// What a window shows now (light or dark, "Match Windows" decided by the page): the windows'
/// own background follows.
#[tauri::command]
pub fn theme_shown(app: AppHandle, light: bool) {
    paint(&app, if light { LIGHT } else { DARK });
}
