// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
fn main() {
    println!("cargo:rerun-if-env-changed=PAKO_WITHOUT_GOOGLE");
    // Pako's Drive-only app key (lists public Drive folders without signing in), built in from
    // the gitignored signing/drive_api_key.txt when it's there. Never printed or put in git.
    let key_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("signing").join("drive_api_key.txt");
    println!("cargo:rerun-if-changed={}", key_file.display());
    println!("cargo:rerun-if-env-changed=PAKO_DRIVE_API_KEY");
    let raw = std::env::var("PAKO_DRIVE_API_KEY").ok().or_else(|| std::fs::read_to_string(&key_file).ok());
    // Notepad may save the file with a BOM in front: not part of the key.
    let key = raw.as_deref().map(|k| k.trim_start_matches('\u{feff}').trim().to_string());
    match key {
        Some(k) if !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') => {
            println!("cargo:rustc-env=PAKO_DRIVE_API_KEY={k}");
        }
        // (Never the key itself in the message.)
        Some(_) => missing("signing/drive_api_key.txt doesn't hold a valid key: this build can't list Drive folders."),
        None => missing("No signing/drive_api_key.txt: this build can't list Drive folders."),
    }
    send_client();
    tauri_build::build()
}

/// A Google key or sign-in that can't be built in. A debug build only warns. A release build
/// (`tauri build`, so every published Pako) stops: without it, a release would lose Drive
/// folders or uploads to Drive, with only a warning lost in a long build log. Someone building
/// Pako without Sookera's Google registrations sets PAKO_WITHOUT_GOOGLE=1 (scripts/release.ps1
/// refuses it).
fn missing(why: &str) {
    let release = std::env::var("PROFILE").is_ok_and(|p| p == "release");
    let allowed = std::env::var("PAKO_WITHOUT_GOOGLE").is_ok_and(|v| v == "1");
    if release && !allowed {
        panic!("{why} A release build must have it (or set PAKO_WITHOUT_GOOGLE=1 to build without Google's features).");
    }
    println!("cargo:warning={why}");
}

/// The value of `"name": "…"` in a JSON text, when it is a string (a plain look-up: the build
/// has no JSON library). `"client_id": null, "project_id": "x"` gives None, not "project_id".
fn json_string(text: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\"");
    let at = text.find(&key)? + key.len();
    let rest = text[at..].trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
}

/// A Google OAuth client ID: "<digits>-<lower-case letters and digits>.apps.googleusercontent.com".
fn is_client_id(id: &str) -> bool {
    let Some(head) = id.strip_suffix(".apps.googleusercontent.com") else { return false };
    let Some((num, rest)) = head.split_once('-') else { return false };
    !num.is_empty()
        && num.chars().all(|c| c.is_ascii_digit())
        && !rest.is_empty()
        && rest.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// The Google sign-in for "Upload to Google Drive" (a Desktop-app client of Sookera's Google project):
/// built in from the gitignored signing/google_send_client.json, so every Pako can connect a Drive.
/// Google treats a desktop app's client secret as not confidential (it ships inside the app); it
/// only identifies the app on Google's sign-in screen. Never printed.
fn send_client() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("signing").join("google_send_client.json");
    println!("cargo:rerun-if-changed={}", file.display());
    let Ok(text) = std::fs::read_to_string(&file) else {
        missing("No signing/google_send_client.json: this build can't connect a Google Drive for uploads.");
        return;
    };
    let secret_ok = |v: &str| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    match (json_string(&text, "client_id"), json_string(&text, "client_secret")) {
        (Some(id), Some(secret)) if is_client_id(&id) && secret_ok(&secret) => {
            println!("cargo:rustc-env=PAKO_SEND_CLIENT_ID={id}");
            println!("cargo:rustc-env=PAKO_SEND_CLIENT_SECRET={secret}");
        }
        _ => missing("signing/google_send_client.json isn't valid: this build can't connect a Google Drive for uploads."),
    }
}
