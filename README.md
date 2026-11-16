# Pako

A download manager for Windows: fast downloads in several parts at once, Google Drive links and folders
that actually finish, videos from YouTube and other sites, a browser extension, links you can share
(Upload) and files sent straight to people you know (Direct).

**Pako by Sookera** · https://pako.sookera.com

This repository holds a copy of each release's source code. Contributions aren't accepted (pull requests
and issues are off). If something doesn't work, use **Help › Report a problem** in Pako, or write to
support@sookera.com.

## Download

Get **Pako Setup** (installs for you, no admin rights) or **Pako Portable** (one file, runs from any
folder) from https://pako.sookera.com or the
[latest release](https://github.com/sookera/pako-releases/releases/latest). Both are the same program
and update themselves.

## Help

The guides are in `docs/help/` and inside Pako (Help, or F1), and on https://pako.sookera.com/help.

## Building it yourself

You need Node.js, Rust and the Tauri prerequisites for Windows.

```
npm install
npx tauri dev      # run it while working on it
npx tauri build    # Pako.exe and Pako Setup in src-tauri/target/release
```

`npx tauri build` stops with a plain message when Sookera's Google files aren't there (see below). To
build without them, set `PAKO_WITHOUT_GOOGLE=1` first (PowerShell: `$env:PAKO_WITHOUT_GOOGLE = "1"`).

A build of your own has no update signing key, no Google Drive key and no Google sign-in for uploads
(Upload to Google Drive needs your own Desktop-app client in `data\google_send_client.json` or
`signing\google_send_client.json`; Gofile works without any of it). So it doesn't update itself, and
it can't list shared Drive folders (single Drive files still download). To list folders, put a Google
API key with the Drive API turned on in `data\drive_api_key.txt` (or in `signing\drive_api_key.txt`
before building).

## Licence

Pako is free software under the **GNU General Public License version 3** (`LICENSE`), with additional
terms (`NOTICE.md`):

- keep the credit "Pako by Sookera" (a changed version says "Based on Pako by Sookera");
- mark changed versions as changed, never as the original or as made by Sookera;
- the names "Pako" and "Sookera" and their logos aren't part of the licence: a version you share uses
  its own name and icon.

Copyright (C) 2026 Sookera.
