# Pako browser extension

Sends downloads, links and videos from your browser to Pako. It talks to Pako directly on this
PC through the browser's "native messaging": nothing listens on the network.

- **Download button on videos** (YouTube watch pages, Shorts and embedded players, Vimeo,
  Twitch, Facebook, Instagram): point at the video and a red **Download** button shows at its top right. Pick
  **Up to 1080p / 720p / 480p** or **Audio only** and the video starts in Downloads straight
  away (Pako's video tools must be installed). Turn the button off in the toolbar popup.
- **Videos on this page** (toolbar popup): the video a page plays (HLS, DASH or a plain video
  file, also in embedded players), noticed from the browser's own requests; the toolbar badge shows
  how many. **Download with Pako** sends one; **Record this video** is the last resort.
- **Right-click a link, image or video › Download with Pako**: it starts in Downloads.
- **Right-click the page › Send all links on this page to Pako**: they go to Links, to pick from.
- **Toolbar button › Send this page to Pako**: handy for YouTube (videos go to Links so you can
  pick the quality).
- **Let Pako take over downloads** (in the toolbar popup, on by default): files over 2 MB you
  download in the browser go to Pako instead. If Pako can't take one, the browser keeps it.

The extension speaks Pako's language (English or French, as chosen in Pako), not the
browser's: its right-click items, toolbar popup and Download button follow Pako. It learns it
from Pako each time it talks to it (and when the browser starts), so after switching the
language in Pako, open the toolbar popup once to see the extension follow at once. With an
older Pako it speaks English.

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

## 2. Vivaldi, Chrome, Edge, Brave, Opera

In Pako: **Settings › Browser extension › Add Pako to your browser** (also in the first-start
setup) lists the browsers on this PC. Press **Add to Chrome** (or Edge, Brave, Vivaldi, Opera, Opera GX): Pako
opens the browser's extensions page and copies the extension's folder for you. Then:

1. Turn on **Developer mode**.
2. Click **Load unpacked**.
3. In the folder window, click the address box at the top, press **Ctrl+V** and **Enter**,
   then **Select Folder**.
4. Pin Pako to the toolbar; its popup should say **Connected**.

Keep the folder where it is (Pako's `data\extension\chromium`): the browser loads the
extension from it. (Doing it by hand: open `chrome://extensions`, `edge://extensions`,
`brave://extensions`, `vivaldi://extensions` or `opera://extensions` and pick that folder. **Open the extension
folder** in Settings shows it.)

(Building from the source instead: `node extension/build.mjs` writes the same files to
`dist/extension/`, plus Pako Light's store zip.)

## 3. Firefox (and Zen, LibreWolf, Waterfox, Floorp)

The Firefox version comes with Pako, signed by Mozilla (Firefox only adds signed extensions).
Zen, LibreWolf, Waterfox and Floorp take the same file: **Add to Zen** (…) works the same way,
and they keep it up to date by themselves too.

1. In Pako: **Settings › Browser extension › Add to Firefox**. Firefox opens
   `pako-firefox.xpi` (in this folder) and asks to add Pako: click **Add**. (If Firefox doesn't
   open, drag `pako-firefox.xpi` into a Firefox window.)
2. In Firefox: about:addons › Pako › **Permissions**: turn on **Access your data for all
   websites** if it is off, so downloads behind a login work and the Download button shows on
   videos.
3. Pin Pako to the toolbar; its popup should say **Connected**.

Firefox keeps the extension up to date by itself. Firefox 140 and later list what the extension
passes to Pako on this PC when adding it (`data_collection_permissions`: the download's cookies,
addresses, and file names and links).

A copy of Pako built from the source without Sookera's signed file has no `pako-firefox.xpi`:
Pako then says the Firefox version isn't included.

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
    `LIGHT = true`, no `player.js` / `recorder.js` / `recorder-bridge.js` and no `webRequest` permission, its own name and icons (`icons-light/`). The build stops if a
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
    `CHROMIUM_IDS` and `LIGHT_IDS` in `src-tauri/src/extension.rs`, let the full extension answer
    it too (`LIGHT_ID` in `background.js`, else the Edge copy never steps aside), and release that
    Pako before submitting (see `docs/store/chrome-web-store.md`).
- Firefox ID: `extension@pako.sookera.com` (`pako@sookera.com` was refused by Mozilla as taken). Signing (unlisted, on addons.mozilla.org), how Pako ships the
  signed file and how Firefox gets updates: `docs/firefox-extension.md`.
- Pako's language: `lang.js` holds the extension's French (the English is the key; French
  typography is added by its `tr`), loaded before the scripts that speak (Chromium's service
  worker imports it, Firefox's manifest lists it first, the popup, the sent page and the pill
  load it). Pako's helper adds `lang` ("en" / "fr", from Pako's saved settings) to every answer,
  and answers `{kind: "language"}` by itself (never starting Pako). The extension asks it when
  the browser starts and now and then when its background page starts (at most every 10
  minutes; Firefox also when a right-click menu shows), keeps it in `storage.local`
  (`pakoLang`) for when Pako can't be reached, and remakes its right-click items when it
  changes. It asks with `private: true`, so an older Pako (which answers "Unknown request.")
  notes nothing; no `lang` in an answer = an older Pako = English. Pako can't tell the
  extension by itself (native messaging only goes from the browser to Pako).
- Messages to the native host: `ping`; `language`; `download` / `links` with `urls` (plus `referrer`,
  `cookies`, `filename`, `takeover`); `video` with one `urls` entry (a video site Pako knows) and
  `quality` 1080, 720, 480 or 0 (audio only); `stream` with one `urls` entry (the caught
  stream), `referrer` (the document that loaded it), `filename` (the page's title) and
  `siteCookies` (the stream's own server's); `record` with `record: {action: start | chunk | end |
  cancel, session, page, title, track, mime, seq, data (base64, at most 512 KB)}`, sent over one
  `connectNative` line. Pako checks each one again on its side (`src-tauri/src/stream.rs`), and
  refuses `stream` and `record` from Pako Light.
- Tests (Node only, no browser, nothing to install): `node --test extension/test/*.test.mjs`.
  They load the real `background.js` of each edition against a fake browser API (Light sends
  nothing from video sites, the two editions don't both take over a download), and build and
  read back both zips.
- Every copy carries `LICENSE` and `NOTICE.md` (the folder Pako writes, both zips).
- `player.js` is the Download button on video players: a content script that reads only the
  page's address (on Facebook and Instagram also the links of the post around the video), draws its button in a closed shadow root, and asks the background page.
- **Videos on this page** (full only, `background.js`): `webRequest.onHeadersReceived`
  (observation only) notes HLS / DASH playlists and whole video files (the media element's own
  requests, 1 MB or more; never single pieces) per tab in `storage.session` (memory only), cleared
  when the tab loads another page or closes. A caught HLS playlist is read again (browser cache
  first) to tell a master playlist (its qualities; its own playlists then leave the list).
- **Record this video** (full only): the popup asks the background page, which tells Pako
  (`record` start), registers `recorder.js` (MAIN world: wraps `MediaSource.addSourceBuffer`,
  `SourceBuffer.appendBuffer` / `changeType`, `requestMediaKeySystemAccess`) and
  `recorder-bridge.js` (isolated: asks whether its tab is the recorded one, then passes the parts
  on a `pako-rec` port, a few at a time, pausing the video while Pako catches up) for all frames at
  `document_start`, and reloads the tab. Only the recorded tab arms them; they are unregistered
  when the recording ends. A key system (DRM) stops it at once. Needs Firefox 128+ / a recent
  Chromium (`world: "MAIN"`).
- `pako://connect` is the only `pako://` link Pako acts on (any other is ignored).
- Native host name: `app.pako.host`. Pako writes the host files to `data/browser/` and the
  per-user registry entries (Chrome, Chromium, Edge, Brave, Vivaldi, Mozilla, which Firefox, Zen,
  Waterfox, Floorp and LibreWolf read, and LibreWolf's own for its 2022 builds) when the switch is
  on, and removes them when it's off.
