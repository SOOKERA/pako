# Firefox extension: signing, shipping, updates

The full extension (`extension/`, video button included) runs in Firefox from the same files as in
Chromium browsers, with `extension/manifest.firefox.json` (ID `extension@pako.sookera.com`). Firefox only adds
extensions signed by Mozilla, so Sookera signs it as **unlisted** on addons.mozilla.org (nobody
else sees it there) and ships the signed file itself.

## How it fits together

- **Signing:** `scripts/sign-firefox.ps1` builds the extension (`node extension/build.mjs`, which
  leaves the unpacked Firefox build in `dist/extension/firefox/`), checks it with
  `web-ext lint --self-hosted`, then runs `web-ext sign --channel unlisted` and waits for Mozilla.
  The signed file lands in `dist/extension/signed/pako-firefox-<version>.xpi` (kept, one per
  version) and a copy in `dist/extension/pako-firefox.xpi`.
  - Mozilla's API login: the gitignored `signing/amo-api.txt` (`issuer=…`, `secret=…`), passed to
    web-ext only through environment variables inside the script, never printed.
  - Mozilla signs each version once: the script stops when `signed/pako-firefox-<version>.xpi`
    already exists. Raise `version` in **both** `manifest.chromium.json` and
    `manifest.firefox.json` (the build refuses different versions).
- **Shipping with Pako:** `src-tauri/build.rs` copies `dist/extension/signed/pako-firefox-<version
  in manifest.firefox.json>.xpi` into the build, and `extension.rs` holds it (`FIREFOX_XPI`,
  `include_bytes!`). An older signed version is never built in. Without the file (someone building
  the public source), Pako still builds (a cargo warning says so) and **Add to Firefox** answers
  "This copy of Pako doesn't include the Firefox version". A test checks the built-in file is the
  current version, for `extension@pako.sookera.com`, with Mozilla's signature.
  - **Add to Firefox** (`add_to_browser` in `lib.rs`) writes it to `data\extension\pako-firefox.xpi`
    and opens it with Firefox, which asks to add it. If Firefox doesn't start, the folder opens
    with the file selected. `refresh_if_outdated` rewrites it after a Pako update. The unsigned
    `pako-firefox.zip` Pako 1.0 wrote for signing by hand is removed.
- **Updates:** `manifest.firefox.json` has
  `update_url: https://github.com/sookera/pako-releases/releases/latest/download/firefox-updates.json`.
  `scripts/release.ps1` copies the signed file into `dist/<version>/` as
  `pako-firefox-<extension version>.xpi` (and `pako-firefox.xpi`, a fixed name for the website) and
  writes `firefox-updates.json` (Mozilla's update manifest: version, `update_link` to the versioned
  .xpi **in that release's tag**, `update_hash` sha256). Firefox checks it about once a day.
  - Redirects are fine: GitHub's `latest/download/…` answers 302 to the tagged file, then to
    GitHub's file host. Firefox's update checker (`AddonUpdateChecker.sys.mjs`, `CertUtils`
    `BadCertHandler`) follows redirects and only asks that the certificates are built-in ones (they
    are). `web-ext lint` reports `update_url` as an error only for listed (Mozilla-hosted)
    add-ons; with `--self-hosted` (unlisted) it passes.
  - `scripts/after-publish.ps1` checks the latest release's `firefox-updates.json` and its .xpi.

## At each release

1. The extension changed since the last release? Raise `version` in both extension manifests and
   run `powershell -ExecutionPolicy Bypass -File scripts/sign-firefox.ps1` (a few minutes; it
   uploads to Mozilla). Unchanged? Nothing to do: the same signed file goes again.
2. `scripts/release.ps1` as usual. It stops if the extension's current version isn't signed (`-WithoutFirefox`
   releases without Firefox), and if the signed file isn't the extension as it is now
   (`scripts/check-firefox-xpi.mjs` lists what changed since signing: raise the version and sign again).
   Upload every file it lists, including `pako-firefox-<version>.xpi`, `pako-firefox.xpi` and
   `firefox-updates.json`.

## Record

- 2026-11-16: extension version raised 1.2.0 → 1.2.1 (code changed since the Chrome Web Store
  draft). First signing run: Mozilla validated the upload, then refused to create the add-on with
  **"Duplicate add-on ID found"** (HTTP 409) for `pako@sookera.com`. Looking it up with the same
  API key (account "Sookera") finds no add-on with that ID, so the ID belongs to another
  addons.mozilla.org account or to a deleted add-on (Mozilla never frees a deleted add-on's ID).
  Not signed yet: either sign with the API key of the account that owns the ID, or pick a new ID
  (`manifest.firefox.json`, `FIREFOX_ID` in `src-tauri/src/extension.rs`, the guides; Pako's native
  host files list the ID, so Pako writes them again at its next start).
- 2026-11-16 (later): new ID `extension@pako.sookera.com`; 1.2.1 signed by Mozilla (unlisted).
- 2026-11-17: the signed 1.2.1 file predates the stream catcher and recorder (no `webRequest`, no
  `recorder*.js`); raised to 1.2.2 and signed again before Pako 1.1.0. `release.ps1` now refuses a signed
  file that differs from the build.
