// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Builds the extension for both browser families:
//   dist/extension/chromium/             folder for "Load unpacked" (Vivaldi, Chrome, Edge, Brave)
//   dist/extension/firefox/              the Firefox version, unpacked: scripts/sign-firefox.ps1
//                                        sends this folder to Mozilla for signing
//   dist/extension/pako-firefox.zip      the same, zipped
//   dist/extension/pako-light-store.zip  Pako Light, to upload to the Chrome Web Store and Edge Add-ons
// Run: node extension/build.mjs
// (Pako itself writes only the full extension, from the same source files: the chromium folder,
// and the Firefox version signed by Mozilla when one was built into it, in Settings › Browser
// extension. It never writes Pako Light.)
import { cpSync, mkdirSync, rmSync, copyFileSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { toLight, LIGHT_BANNED } from "./light.mjs";
import { makeZip, readZip } from "./zip.mjs";

/** The version background.js says it is (`const SCRIPT_VERSION = "…";`), or null. */
function scriptVersion(source) {
  const all = [...source.matchAll(/^const SCRIPT_VERSION = "([^"]*)";\r?$/gm)];
  return all.length === 1 ? all[0][1] : null;
}

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const out = join(root, "dist", "extension");
const shared = ["lang.js", "background.js", "pill.js", "player.js", "caught.js", "recorder.js", "recorder-bridge.js", "popup.html", "popup.css", "popup.js", "pako-sent.html", "pako-sent.js", "download-links.js", "icons"];
/** Only the full extension has these: the Download pill on videos, "Record this video", and the
 *  catch in the browsers that ask before downloading. */
const FULL_ONLY_FILES = ["pill.js", "player.js", "caught.js", "recorder.js", "recorder-bridge.js", "pako-sent.html", "pako-sent.js", "download-links.js"];
/** Only the full extension asks for these: noticing the videos a page plays, and catching a
 *  download at the server's answer in the browsers that ask before downloading. */
const FULL_ONLY_PERMISSIONS = ["webRequest", "declarativeNetRequestWithHostAccess"];
/** Only the Chromium folder has these: the page a download caught at the server's answer turns
 *  into, and the script for download links on pages (Firefox keeps its own way). */
const CHROMIUM_ONLY_FILES = ["pako-sent.html", "pako-sent.js", "download-links.js"];
/** The licence and its additional terms travel with every copy. */
const legal = ["LICENSE", "NOTICE.md"];

function stage(dir, manifest) {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  for (const f of shared) cpSync(join(here, f), join(dir, f), { recursive: true });
  for (const f of legal) copyFileSync(join(root, f), join(dir, f));
  copyFileSync(join(here, manifest), join(dir, "manifest.json"));
}

/** Every file under `dir`, as zip entries named with "/". */
function entriesOf(dir) {
  const list = [];
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else list.push({ name: relative(dir, p).split(sep).join("/"), data: readFileSync(p) });
    }
  };
  walk(dir);
  return list;
}

/** Zips a staged folder with its files at the top of the zip (as the stores expect), checks the
 *  zip reads back the same, then removes the folder (unless `keep`). */
function zipUp(dir, zip, keep = false) {
  const entries = entriesOf(dir);
  writeFileSync(zip, makeZip(entries));
  const back = readZip(readFileSync(zip));
  if (back.length !== entries.length || back.some((e, i) => e.name !== entries[i].name || !e.data.equals(entries[i].data))) {
    throw new Error(`${zip}: the zip doesn't read back the same`);
  }
  if (back.some((e) => e.name.includes("\\"))) throw new Error(`${zip}: an entry name has a "\\"`);
  if (!keep) rmSync(dir, { recursive: true, force: true });
}

mkdirSync(out, { recursive: true });
stage(join(out, "chromium"), "manifest.chromium.json");

const ff = join(out, "firefox");
stage(ff, "manifest.firefox.json");
for (const f of CHROMIUM_ONLY_FILES) rmSync(join(ff, f));
zipUp(ff, join(out, "pako-firefox.zip"), true);
// Chromium and Firefox are the same extension: one version number for both.
const versionOf = (f) => JSON.parse(readFileSync(join(here, f), "utf8")).version;
if (versionOf("manifest.chromium.json") !== versionOf("manifest.firefox.json")) {
  throw new Error("manifest.chromium.json and manifest.firefox.json have different versions: make them the same");
}
// The background script says its version too (a browser can keep running an older script after
// the folder was updated: the extension tells them apart by it).
const scriptSays = scriptVersion(readFileSync(join(here, "background.js"), "utf8"));
if (scriptSays !== versionOf("manifest.chromium.json")) {
  throw new Error(`background.js says SCRIPT_VERSION = "${scriptSays}" but the manifests say ${versionOf("manifest.chromium.json")}: raise it with them`);
}

// Pako Light: the Chrome Web Store / Edge Add-ons edition. Those stores don't allow downloading
// videos (YouTube above all), so it has none of that: no player.js, the full-only blocks taken
// out of its scripts and window (see light.mjs), and the build stops if a word of it is left.
// Its own name and icons, and no "key": the store gives it its own ID
// (cojnnpgiilnbkblckkjnheigiodkdlpl), separate from the full extension's.
const store = join(out, "chrome-store");
stage(store, "manifest.chromium.json");
for (const f of FULL_ONLY_FILES) rmSync(join(store, f));
rmSync(join(store, "icons"), { recursive: true, force: true });
cpSync(join(here, "icons-light"), join(store, "icons"), { recursive: true });
for (const f of ["lang.js", "background.js", "popup.js", "popup.html", "popup.css"]) {
  const file = join(store, f);
  writeFileSync(file, toLight(f, readFileSync(file, "utf8")));
}
// NOTICE.md's last part is about the Pako app's own parts and tools, not this extension.
const notice = join(store, "NOTICE.md");
writeFileSync(notice, toLight("NOTICE.md", readFileSync(notice, "utf8").replace(/\r?\n## Third-party parts[\s\S]*$/, "\n")));
const m = JSON.parse(readFileSync(join(store, "manifest.json"), "utf8"));
delete m.key;
delete m.content_scripts;
delete m.web_accessible_resources;
m.permissions = m.permissions.filter((p) => !FULL_ONLY_PERMISSIONS.includes(p));
m.name = "Pako Light";
m.short_name = "Pako Light";
m.description = "Send downloads and links from your browser to Pako, the download manager for Windows.";
m.action.default_title = "Pako Light";
writeFileSync(join(store, "manifest.json"), toLight("manifest.json", JSON.stringify(m, null, 2) + "\n"));
// Nothing else may slip in (LICENSE is the GPL's own text, word for word).
for (const e of entriesOf(store)) {
  if (e.name === "LICENSE" || !/\.(js|html|css|json|md)$/.test(e.name)) continue;
  if (LIGHT_BANNED.test(e.data.toString("utf8"))) throw new Error(`Pako Light's ${e.name} still has a word of the full extension`);
}
zipUp(store, join(out, "pako-light-store.zip"));
rmSync(join(out, "pako-chrome-store.zip"), { force: true });

console.log("Ready:", join(out, "chromium"), "·", ff, "·", join(out, "pako-firefox.zip"), "·", join(out, "pako-light-store.zip"));
