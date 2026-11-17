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
    firefox_xpi();
    terms_page();
    system_dlls_only();
    tauri_build::build()
}

// The Markdown-to-text conversion, shared with Pako itself (its tests are there).
#[path = "src/terms.rs"]
#[allow(dead_code)]
mod terms;

/// Pako Setup's "Terms of use" page (bundle.licenseFile in tauri.conf.json): TERMS.md as plain
/// text in gen/terms.txt (gitignored), made fresh for every build so it never drifts from
/// TERMS.md. `npx tauri build` builds Pako (and so runs this) before it makes the installer.
fn terms_page() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = manifest.join("..").join("TERMS.md");
    println!("cargo:rerun-if-changed={}", source.display());
    let md = std::fs::read_to_string(&source).unwrap_or_else(|_| panic!("{} is missing: Pako needs its terms of use.", source.display()));
    if md.trim().is_empty() {
        panic!("TERMS.md is empty: Pako needs its terms of use.");
    }
    if terms::version_of(&md).is_none() {
        panic!("TERMS.md has no \"Last updated <day> <month> <year>\" line: Pako saves that date when the terms are accepted.");
    }
    let text = terms::plain_text(&md);
    let out = manifest.join("gen").join("terms.txt");
    // Written only when it changes (tauri dev would see a new file each time otherwise). A text
    // that isn't plain ASCII gets a UTF-8 mark at the start, so the installer reads it right.
    let bytes = if text.is_ascii() { text.into_bytes() } else { [b"\xEF\xBB\xBF".as_slice(), text.as_bytes()].concat() };
    if std::fs::read(&out).ok().as_deref() != Some(bytes.as_slice()) {
        std::fs::create_dir_all(out.parent().unwrap()).expect("make src-tauri/gen");
        std::fs::write(&out, bytes).expect("write src-tauri/gen/terms.txt for Pako Setup");
    }
}

/// Windows looks for the DLLs a program names (dwmapi.dll, powrprof.dll, windowscodecs.dll…) in
/// the program's own folder first. The portable Pako.exe often sits in Downloads, where a file
/// with one of those names (saved by a browser, or by Pako itself) would be loaded and run at
/// Pako's next start. This linker setting (LOAD_LIBRARY_SEARCH_SYSTEM32) makes Windows take them,
/// and what they need in turn, from its own System32 folder only. `main` does the same for DLLs
/// asked for while Pako runs. Checked by `scripts/release.ps1` on the built exe.
fn system_dlls_only() {
    if std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|e| e == "msvc") {
        println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
    }
}

/// The Firefox extension signed by Mozilla (scripts/sign-firefox.ps1), built into Pako so "Add to
/// Firefox" works on any PC: dist/extension/signed/pako-firefox-<version>.xpi, for the version in
/// extension/manifest.firefox.json (an older signed file is never built in). Without it (someone
/// building the public source), Pako still builds and says Firefox isn't available: the file
/// copied for extension.rs is then empty. scripts/release.ps1 checks a release has it.
fn firefox_xpi() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let manifest = root.join("extension").join("manifest.firefox.json");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let version = std::fs::read_to_string(&manifest).ok().and_then(|t| json_string(&t, "version"));
    let signed_dir = root.join("dist").join("extension").join("signed");
    // (Cargo watches a folder's files, so a newly signed version triggers a new build.)
    println!("cargo:rerun-if-changed={}", signed_dir.display());
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("pako-firefox.xpi");
    let signed = version
        .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .map(|v| signed_dir.join(format!("pako-firefox-{v}.xpi")));
    let bytes = signed.as_ref().and_then(|p| std::fs::read(p).ok()).unwrap_or_default();
    // An .xpi is a zip: "PK" first.
    if !bytes.is_empty() && !bytes.starts_with(b"PK") {
        panic!("{} isn't a signed extension (not a zip): sign it again with scripts/sign-firefox.ps1.", signed.unwrap().display());
    }
    if bytes.is_empty() {
        println!("cargo:warning=No signed Firefox extension for this version (dist/extension/signed/, made by scripts/sign-firefox.ps1): this build says Firefox isn't available.");
    }
    std::fs::write(&out, &bytes).expect("write the Firefox extension for the build");
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
