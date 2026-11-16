// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Test mode without the window:
//!   <app>.exe --cli [--dest DIR] [--connections N] [--speed KB] LINK...
//! Downloads the links, prints progress every second, exits when everything has stopped.
//! Exit code 0 = all completed.
//!
//! It uses its own data folder (`<normal data folder>\cli-test`, or PAKO_DATA_DIR when set),
//! so it never touches the window's list or settings, and can run while the window is open.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::engine::Engine;
use crate::model::Status;

pub fn main(args: Vec<String>) -> i32 {
    // Before anything reads paths::data_dir(), which keeps the first answer for good.
    if crate::paths::data_dir_override().is_none() {
        // Nothing else runs yet (no threads), so changing the environment is safe.
        unsafe { std::env::set_var("PAKO_DATA_DIR", crate::paths::default_data_dir().join("cli-test")) };
    }
    tauri::async_runtime::block_on(run(args))
}

/// The window's data folder, worked out like paths::data_dir() (which can't be asked without
/// fixing it for this process): the project's data folder in debug builds, else `data` next
/// to the .exe when it exists, else %LOCALAPPDATA%\Pako.

async fn run(args: Vec<String>) -> i32 {
    let mut links = Vec::new();
    let mut dest: Option<PathBuf> = None;
    let mut connections: Option<u32> = None;
    let mut speed: Option<u64> = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--cli" => {}
            "--dest" => dest = it.next().map(PathBuf::from),
            // A number = fixed connections (Auto off). Without it, Auto is used.
            "--connections" => connections = it.next().and_then(|v| v.parse().ok()),
            "--speed" => speed = it.next().and_then(|v| v.parse().ok()),
            _ => links.push(a),
        }
    }

    let emit: crate::engine::Emit = Arc::new(|_, _| {});
    let engine = Engine::new(emit);
    println!("data folder (the CLI's own): {}", crate::paths::data_dir().display());

    let mut s = engine.settings();
    s.auto_connections = connections.is_none();
    if let Some(c) = connections {
        s.connections = c;
    }
    if let Some(k) = speed {
        s.speed_limit_kb = k;
    }
    if let Some(d) = &dest {
        s.download_dir = d.clone();
    }
    if let Err(e) = engine.set_settings(s) {
        println!("settings: {e}");
    }

    if !links.is_empty() {
        let r = engine.add_links(&links.join("\n"), dest).await;
        println!("added {} download(s)", r.added);
        for e in &r.errors {
            println!("  problem: {e}");
        }
    }
    engine.start();

    let mut last_line = String::new();
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let list = engine.downloads();
        let busy = list.iter().any(|d| d.status == Status::Queued || d.status.is_active());
        for d in list.iter().filter(|d| d.status.is_active()) {
            let pct = d.size.filter(|s| *s > 0).map(|s| d.downloaded as f64 * 100.0 / s as f64).unwrap_or(0.0);
            let line = format!(
                "{:?} {:5.1}% {} / {} conn={} {} {}",
                d.status,
                pct,
                d.downloaded,
                d.size.map(|s| s.to_string()).unwrap_or("?".into()),
                engine.live_connections(&d.id),
                d.name,
                d.message.clone().unwrap_or_default()
            );
            if line != last_line {
                println!("{line}");
                last_line = line;
            }
        }
        if !busy {
            break;
        }
    }
    engine.save_now();
    let list = engine.downloads();
    let mut all_ok = true;
    for d in &list {
        println!("RESULT {:?} {} {}", d.status, d.final_path().display(), d.message.clone().unwrap_or_default());
        if d.status != Status::Completed {
            all_ok = false;
        }
    }
    if all_ok { 0 } else { 1 }
}
