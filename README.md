<p align="center">
  <a href="https://pako.sookera.com"><img src="https://pako.sookera.com/pako.svg" width="96" height="96" alt="Pako"></a>
</p>

<h1 align="center">Pako</h1>

<p align="center">
  Share big files with a link, send them straight to another PC,<br>
  and download fast, picking up where it stopped.
</p>

<p align="center">
  <a href="https://pako.sookera.com"><b>Download for Windows</b></a> ·
  <a href="https://pako.sookera.com/help">Help</a> ·
  <a href="https://pako.sookera.com/help/whats-new">What's new</a> ·
  <a href="https://sookera.com/terms">Terms</a> ·
  <a href="https://sookera.com/privacy">Privacy</a>
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="https://pako.sookera.com/shots/main-light@2x.webp">
    <img src="https://pako.sookera.com/shots/main@2x.webp" width="900" alt="Pako's Downloads page with several files downloading and the details panel showing one file's parts.">
  </picture>
</p>

Free for Windows 10 and 11. No account, no ads, no tracking.

## What it does

- **Share big files with a link.** Drop files or whole folders: Pako uploads them to Gofile (no account) or your own
  Google Drive, copies the link, and can delete them after 3, 7 or 30 days.
- **Send files straight to people.** From your Pako to theirs, PC to PC, encrypted, with nothing stored in between.
  On the same Wi-Fi, files go over your home network, even when the internet is down.
- **Send between your own PCs.** Pair them once, then send files with no Accept to press, send links for the
  other PC to download, and follow its downloads live.
- **Fast downloads that don't give up.** Several parts at once, picking up where it stopped after a cut or a
  restart, and going on from the next host when a file is on several.
- **Many links at once.** Paste a whole list: each link is checked first, split archives are grouped and unpacked,
  and whole Google Drive folders open as a list of files.
- **Straight from your browser.** Pako's extension for Chrome, Edge, Brave, Vivaldi, Opera and Firefox sends
  links, files and videos to Pako with a right-click.
- **Videos in the quality you want.** Paste a video or playlist link, check a short preview, pick the quality or
  Audio only. With the full browser extension, Pako also finds the video playing on almost any page.

<p align="center">
  <img src="https://pako.sookera.com/shots/devices@2x.webp" width="900" alt="Pako's My devices page sending links and a playlist's videos to another PC, with quality and priority.">
</p>

## Get it

Download **Pako Setup** (installs for you, no admin rights) or **Pako Portable** (one file, runs from any folder)
from [pako.sookera.com](https://pako.sookera.com) or the
[latest release](https://github.com/sookera/pako-releases/releases/latest). Both are the same program and update
themselves, accepting only updates signed by us. Each file's SHA-256 is on
[pako.sookera.com/checksums](https://pako.sookera.com/checksums).

The first time, Windows may say it protected your PC: choose **More info**, then **Run anyway**.

## This repository

It holds a copy of each release's source code. Contributions aren't accepted (pull requests and issues are off).
If something doesn't work, use **Help › Report a problem** in Pako, or write to support@sookera.com.

<details>
<summary><b>Building it yourself</b></summary>

You need Node.js, Rust and the Tauri prerequisites for Windows.

```
npm install
npx tauri dev      # run it while working on it
npx tauri build    # Pako.exe in src-tauri/target/release, Pako Setup in its bundle/nsis folder
```

`npx tauri build` stops with a plain message when Sookera's Google files aren't there (see below). To
build without them, set `PAKO_WITHOUT_GOOGLE=1` first (PowerShell: `$env:PAKO_WITHOUT_GOOGLE = "1"`).

A build of your own has no update signing key, no Google Drive key and no Google sign-in for uploads
(Upload to Google Drive needs your own Desktop-app client in `data\google_send_client.json` or
`signing\google_send_client.json`; Gofile works without any of it). So it doesn't update itself, and
it can't list shared Drive folders (single Drive files still download). To list folders, put a Google
API key with the Drive API turned on in `data\drive_api_key.txt` (or in `signing\drive_api_key.txt`
before building).

</details>

## Use it responsibly

Download, share and record only what you have the right to, and follow the rules of the sites you use. Pako never
downloads or records protected (DRM) videos. See the [terms of use](https://sookera.com/terms).

## Licence

Pako is free software under the **GNU General Public License version 3** (`LICENSE`), with additional
terms (`NOTICE.md`):

- keep the credit "Pako by Sookera" (a changed version says "Based on Pako by Sookera");
- mark changed versions as changed, never as the original or as made by Sookera;
- the names "Pako" and "Sookera" and their logos aren't part of the licence: a version you share uses
  its own name and icon.

Copyright (C) 2026 Sookera.

<p align="center"><sub>Pako by <a href="https://sookera.com">Sookera</a></sub></p>
