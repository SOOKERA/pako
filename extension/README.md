# Pako browser extension

Sends downloads, links and videos from your browser to Pako. It talks to Pako directly on this
PC through the browser's "native messaging": nothing listens on the network.

- **Download button on videos** (YouTube watch pages, Shorts and embedded players, Vimeo,
  Twitch): point at the video and a red **Download** button shows at its top right. Pick
  **Up to 1080p / 720p / 480p** or **Audio only** and the video starts in Downloads straight
  away (Pako's video tools must be installed). Turn the button off in the toolbar popup.
- **Right-click a link, image or video › Download with Pako**: it starts in Downloads.
- **Right-click the page › Send all links on this page to Pako**: they go to Links, to pick from.
- **Toolbar button › Send this page to Pako**: handy for YouTube (videos go to Links so you can
  pick the quality).
- **Let Pako take over downloads** (in the toolbar popup, on by default): files over 2 MB you
  download in the browser go to Pako instead. If Pako can't take one, the browser keeps it.

The browser's cookies for a download go to Pako with it, so files behind a login work. Pako
keeps them for that download only, encrypted by Windows for your Windows account (so the
download still works after a restart), and deletes them once it is finished or removed.

## 1. Connect Pako (once)

Pako's first-start setup has **Add Pako to my browsers** ticked: that connects it. Later, or
if it was skipped: **Settings › Browser extension › Connect Pako to your browsers**, then
**Save**.

Easiest of all: if the extension's toolbar popup says **Not connected**, press **Open Pako**.
The browser asks to open Pako (allow it); Pako opens, connects itself and says **Connected**.
Open the popup again: it says Connected too.

(This uses a `pako://connect` link that Pako registers for this Windows user each time it
starts. It only ever opens Pako and switches the connection on.)

## 2. Vivaldi, Chrome, Edge, Brave

In Pako: **Settings › Browser extension › Add Pako to your browser** (also in the first-start
setup) lists the browsers on this PC. Press **Add to Chrome** (or Edge, Brave, Vivaldi): Pako
opens the browser's extensions page and copies the extension's folder for you. Then:

1. Turn on **Developer mode**.
2. Click **Load unpacked**.
3. In the folder window, click the address box at the top, press **Ctrl+V** and **Enter**,
   then **Select Folder**.
4. Pin Pako to the toolbar; its popup should say **Connected**.

Keep the folder where it is (Pako's `data\extension\chromium`): the browser loads the
extension from it. (Doing it by hand: open `chrome://extensions`, `edge://extensions`,
`brave://extensions` or `vivaldi://extensions` and pick that folder. **Open the extension
folder** in Settings shows it.)

(Building from the source instead: `node extension/build.mjs` writes the same files to
`dist/extension/`, plus Pako Light's store zip.)

## 3. Firefox (needs Mozilla's free signature once)

Firefox only runs extensions signed by Mozilla. "Unlisted" means nobody else sees it.

1. Go to <https://addons.mozilla.org/developers/> and sign in or create a free Firefox account.
2. Click **Submit a New Add-on**.
3. Choose **On your own** (unlisted), then **Continue**.
4. Upload `pako-firefox.zip`. Accept the developer agreement if it asks. If it asks about
   source code, answer **No** (the files aren't minified or generated).
5. Wait for the automatic check (usually a few minutes), then download the signed `.xpi` file
   from the add-on's page.
6. Rename it `pako-firefox.xpi` and put it in Pako's extension folder, next to
   `pako-firefox.zip`. From then on **Add to Firefox** in Pako opens it in Firefox, which
   offers to add Pako: click **Add**. (Or drag the `.xpi` into a Firefox window.)
7. In Firefox: open the Pako extension's settings (about:addons › Pako › Permissions) and allow
   **Access your data for all websites**, so downloads behind a login work and the Download
   button shows on videos.

A new version of the extension needs the same steps with the new zip (raise `version` in both
`manifest.*.json` files first).

## For developers

- Two editions, two IDs (Pako answers both):
  - **The full extension** (this folder; video button included): `manifest.chromium.json` carries a
    fixed `key`, so its ID is always `fnaplmfbaccplngclnkchlngpkbcbhod`. The private key is in the
    git-ignored `signing/extension-key.pem`. Installed by Pako ("Load unpacked"), and offered on the
    website.
  - **Pako Light** (Chrome Web Store and Edge Add-ons): those stores don't allow video
    downloading, so Light has none of it, neither in what it does nor in its files.
    `node extension/build.mjs` makes `pako-light-store.zip` from the same files (`light.mjs`):
    the blocks marked `/* full-only:start */ … /* full-only:end */` (JS) and
    `<!-- full-only:start --> … <!-- full-only:end -->` (popup.html) are taken out,
    `LIGHT = true`, no `player.js`, its own name and icons (`icons-light/`). The build stops if a
    word of the full extension's extras (video, quality, player…) is left. Store ID
    `cojnnpgiilnbkblckkjnheigiodkdlpl`.
  - Light sends nothing from the sites in `LIGHT_SKIPS_SITES` (its page items are greyed out
    there), leaves links to them out of "all links", doesn't take over their downloads, and offers
    "Download with Pako" on pictures only. Pako checks again: the browser starts the native host
    with `chrome-extension://<id>/`, so Pako knows a request is Light's (`LIGHT_IDS` in
    `src-tauri/src/extension.rs`), refuses videos, drops video-site links, and never asks yt-dlp
    about Light's links in Links.
  - Both editions in one browser: Light asks the full extension
    (`runtime.sendMessage(fnap…, {type: "pako-full-here"})`; no permission needed) and, when it
    answers, makes no right-click items and doesn't take over downloads; its popup says so. Pako
    also takes the same takeover (same link and size) only once within 30 s, and gives both
    requests that answer.
  - **Edge Add-ons** gives the item its own ID. When the Edge draft exists, add its ID to
    `CHROMIUM_IDS` and `LIGHT_IDS` in `src-tauri/src/extension.rs` and release that Pako before
    submitting (see `docs/store/chrome-web-store.md`).
- Firefox ID: `pako@sookera.com`.
- Messages to the native host: `ping`; `download` / `links` with `urls` (plus `referrer`,
  `cookies`, `filename`, `takeover`); `video` with one `urls` entry (a video site Pako knows) and
  `quality` 1080, 720, 480 or 0 (audio only). Pako checks each one again on its side.
- Tests (Node only, no browser, nothing to install): `node --test extension/test/*.test.mjs`.
  They load the real `background.js` of each edition against a fake browser API (Light sends
  nothing from video sites, the two editions don't both take over a download), and build and
  read back both zips.
- Every copy carries `LICENSE` and `NOTICE.md` (the folder Pako writes, both zips).
- `player.js` is the Download button on video players: a content script that reads only the
  page's address, draws its button in a closed shadow root, and asks the background page.
- `pako://connect` is the only `pako://` link Pako acts on (any other is ignored).
- Native host name: `app.pako.host`. Pako writes the host files to `data/browser/` and the
  per-user registry entries (Chrome, Chromium, Edge, Brave, Vivaldi, Firefox) when the switch is
  on, and removes them when it's off.
