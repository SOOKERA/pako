// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Files and folders dropped from Explorer onto the window (the Send tab).
//!
//! Windows' own file drop is off for this window (`dragDropEnabled: false`), so the page keeps
//! normal drag and drop (dropping links, moving rows). A page never sees the paths of dropped
//! files, so the Send tab hands the dropped items to WebView2 with
//! `chrome.webview.postMessageWithAdditionalObjects("pako-files", …)`; WebView2 turns each one into
//! a file or folder object that carries its real path, read here and sent back as `files-dropped`.
//! Only objects made by a real drop (or a file picker) carry a path, so a page script can't name
//! files of its own.

use tauri::{Emitter, Manager};

/// The message name the Send tab uses.
const MESSAGE: &str = "pako-files";

#[cfg(windows)]
pub fn listen(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let emit_to = window.clone();
    let _ = window.with_webview(move |pw| {
        if let Err(e) = unsafe { add_handler(pw.controller(), emit_to) } {
            eprintln!("file drop: {e}");
        }
    });
}

#[cfg(not(windows))]
pub fn listen(_app: &tauri::AppHandle) {}

#[cfg(windows)]
unsafe fn add_handler(
    controller: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller,
    window: tauri::WebviewWindow,
) -> windows_core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2File, ICoreWebView2FileSystemHandle, ICoreWebView2WebMessageReceivedEventArgs2,
    };
    use webview2_com::{WebMessageReceivedEventHandler, take_pwstr};
    use windows_core::{Interface, PWSTR};

    let webview = unsafe { controller.CoreWebView2()? };
    let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else { return Ok(()) };
        // Wry's own handler sees every message too; only ours is looked at here.
        let text = {
            let mut p = PWSTR::null();
            if unsafe { args.TryGetWebMessageAsString(&mut p) }.is_err() {
                return Ok(());
            }
            take_pwstr(p)
        };
        if text != MESSAGE {
            return Ok(());
        }
        // Only from Pako's own page.
        let source = {
            let mut p = PWSTR::null();
            unsafe { args.Source(&mut p)? };
            take_pwstr(p)
        };
        if !is_own_page(&source) {
            return Ok(());
        }
        let Ok(args2) = args.cast::<ICoreWebView2WebMessageReceivedEventArgs2>() else {
            return Ok(());
        };
        let objects = unsafe { args2.AdditionalObjects()? };
        let mut count = 0u32;
        unsafe { objects.Count(&mut count)? };
        let mut paths: Vec<String> = Vec::new();
        // What came through, said in the message when no path could be read.
        let mut notes: Vec<String> = Vec::new();
        for i in 0..count.min(10_000) {
            let obj = match unsafe { objects.GetValueAtIndex(i) } {
                Ok(o) => o,
                Err(e) => {
                    notes.push(format!("item {i}: {}", e.code()));
                    continue;
                }
            };
            let mut p = PWSTR::null();
            let read = if let Ok(f) = obj.cast::<ICoreWebView2File>() {
                unsafe { f.Path(&mut p) }.map_err(|e| format!("file {}", e.code()))
            } else if let Ok(h) = obj.cast::<ICoreWebView2FileSystemHandle>() {
                unsafe { h.Path(&mut p) }.map_err(|e| format!("handle {}", e.code()))
            } else {
                Err("unknown kind".to_string())
            };
            match read {
                Ok(()) if !p.is_null() => {
                    let path = take_pwstr(p);
                    if !path.is_empty() && !paths.contains(&path) {
                        paths.push(path);
                    }
                }
                Ok(()) => notes.push("empty path".into()),
                Err(e) => notes.push(e),
            }
        }
        let detail = if count == 0 { "nothing came through".to_string() } else { format!("{count} items: {}", notes.join(", ")) };
        let _ = window.emit_to(window.label(), "files-dropped", serde_json::json!({ "paths": paths, "detail": detail }));
        Ok(())
    }));
    let mut token = 0i64;
    unsafe { webview.add_WebMessageReceived(&handler, &mut token)? };
    Ok(())
}

/// Pako's own page: tauri://localhost (http://tauri.localhost on Windows) or the dev server.
fn is_own_page(source: &str) -> bool {
    let Ok(u) = url::Url::parse(source) else { return false };
    match u.host_str() {
        Some("tauri.localhost") => true,
        Some("localhost") => u.scheme() == "tauri" || cfg!(debug_assertions),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_own_page;

    #[test]
    fn only_pakos_own_page() {
        assert!(is_own_page("http://tauri.localhost/"));
        assert!(is_own_page("https://tauri.localhost/index.html"));
        assert!(!is_own_page("https://example.com/"));
        assert!(!is_own_page("https://tauri.localhost.example.com/"));
        assert!(!is_own_page("not a url"));
    }
}
