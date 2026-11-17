// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Before anything else: DLLs asked for by name while Pako runs (uxtheme.dll for the dark title
/// bar, and whatever Windows' own parts ask for) come from Windows' System32 folder, never from
/// Pako's own folder or the folder Pako was started in. A file with such a name next to a
/// portable Pako.exe (in Downloads, say) would otherwise be loaded and run. The DLLs Pako.exe
/// names itself are covered by the linker setting in build.rs. Pako ships no DLL of its own.
#[cfg(windows)]
fn system_dlls_only() {
    use windows_sys::Win32::System::LibraryLoader::{LOAD_LIBRARY_SEARCH_SYSTEM32, LOAD_LIBRARY_SEARCH_USER_DIRS, SetDefaultDllDirectories};
    // SAFETY: a plain call with constant flags. If Windows refuses, the search stays as it was.
    unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_USER_DIRS) };
}

fn main() {
    #[cfg(windows)]
    system_dlls_only();
    // (args_os: an argument that isn't valid Unicode must not stop Pako; `args()` would panic.)
    let args: Vec<String> = std::env::args_os().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
    // Started by the uninstaller: remove what Pako registered with Windows (and its data when
    // asked), open no window.
    if args.first().map(|a| a == pako_lib::install::UNINSTALL_CLEANUP).unwrap_or(false) {
        std::process::exit(pako_lib::install::uninstall_cleanup(&args));
    }
    // Started by a browser for the Pako extension: answer it, never open a window.
    if pako_lib::extension::is_host_launch(&args) {
        std::process::exit(pako_lib::extension::run_host(&args));
    }
    if args.first().map(|a| a == "--cli").unwrap_or(false) {
        std::process::exit(pako_lib::cli::main(args));
    }
    // Release tool (signing new versions): never used by the app itself.
    if args.first().map(|a| a == "--release").unwrap_or(false) {
        std::process::exit(pako_lib::release::main(args));
    }
    pako_lib::run()
}
