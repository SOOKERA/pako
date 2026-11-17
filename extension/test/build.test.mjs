// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The build: Pako Light has none of the full extension's extras, zips use "/" in their entry
// names, and the licence travels with every copy (audit 2026-11-16 evening, H3, L1, M4).
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { toLight, LIGHT_BANNED } from "../light.mjs";
import { makeZip, readZip } from "../zip.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const dist = join(here, "..", "..", "dist", "extension");

test("toLight takes out full-only blocks and turns the switch on once", () => {
  const js = "const LIGHT = false;\nconst a = 1;\n/* full-only:start */\nconst q = 'quality';\n/* full-only:end */\nconst b = 2;\n";
  assert.equal(toLight("popup.js", js), "const LIGHT = true;\nconst a = 1;\nconst b = 2;\n");
  const html = "<p>a</p>\n  <!-- full-only:start -->\n  <p>player</p>\n  <!-- full-only:end -->\n<p>b</p>\n";
  assert.equal(toLight("popup.html", html), "<p>a</p>\n<p>b</p>\n");
  // CRLF files work the same.
  assert.equal(toLight("popup.js", js.replace(/\n/g, "\r\n")), "const LIGHT = true;\r\nconst a = 1;\r\nconst b = 2;\r\n");
});

test("toLight refuses what would ship the wrong edition", () => {
  // The switch is missing, or there twice (a comment quoting it on its own line counts).
  assert.throws(() => toLight("popup.js", "const x = 1;\n"), /exactly once/);
  assert.throws(() => toLight("popup.js", "const LIGHT = false;\nconst LIGHT = false;\n"), /exactly once/);
  // A block left open, or a word of the full extension outside a block.
  assert.throws(() => toLight("popup.js", "const LIGHT = false;\n/* full-only:start */\n"), /isn't closed/);
  assert.throws(() => toLight("popup.js", "const LIGHT = false;\n// paste the video link\n"), /video/);
  assert.throws(() => toLight("popup.html", "<p>Pick the quality</p>"), /quality/);
});

test("Each source file turns into a clean Pako Light file", () => {
  for (const f of ["background.js", "popup.js", "popup.html", "popup.css"]) {
    const text = toLight(f, readFileSync(join(here, "..", f), "utf8"));
    assert.ok(!LIGHT_BANNED.test(text), f);
    assert.ok(!/full-only:(start|end) (\*\/|-->)/.test(text), f);
  }
});

test("The zip writer reads back what it wrote, names with /", () => {
  const entries = [
    { name: "a.txt", data: Buffer.from("hello") },
    { name: "icons/b.bin", data: Buffer.alloc(5000, 7) },
  ];
  const back = readZip(makeZip(entries));
  assert.deepEqual(back.map((e) => e.name), ["a.txt", "icons/b.bin"]);
  assert.ok(back[1].data.equals(entries[1].data));
  assert.throws(() => makeZip([{ name: "icons\\b.bin", data: Buffer.alloc(1) }]), /bad entry name/);
});

test("node extension/build.mjs: both zips are right", () => {
  execFileSync(process.execPath, [join(here, "..", "build.mjs")], { stdio: "pipe" });
  const light = readZip(readFileSync(join(dist, "pako-light-store.zip")));
  const firefox = readZip(readFileSync(join(dist, "pako-firefox.zip")));
  for (const zip of [light, firefox]) {
    assert.ok(zip.every((e) => !e.name.includes("\\")));
    for (const f of ["manifest.json", "background.js", "popup.html", "LICENSE", "NOTICE.md", "icons/icon128.png"]) {
      assert.ok(zip.some((e) => e.name === f), f);
    }
  }
  assert.ok(firefox.some((e) => e.name === "player.js"));
  // Mozilla refuses a new extension that doesn't say what it passes on.
  const gecko = JSON.parse(firefox.find((e) => e.name === "manifest.json").data.toString("utf8")).browser_specific_settings.gecko;
  assert.ok(gecko.data_collection_permissions.required.length > 0);
  // Firefox takes the extension's updates from Pako's latest release (scripts/release.ps1).
  assert.equal(gecko.update_url, "https://github.com/sookera/pako-releases/releases/latest/download/firefox-updates.json");
  // The unpacked folder scripts/sign-firefox.ps1 sends to Mozilla is the zip's twin.
  const folder = JSON.parse(readFileSync(join(dist, "firefox", "manifest.json"), "utf8"));
  assert.equal(folder.browser_specific_settings.gecko.id, "extension@pako.sookera.com");
  // One version for both browser families.
  const chromium = JSON.parse(readFileSync(join(dist, "chromium", "manifest.json"), "utf8"));
  assert.equal(folder.version, chromium.version);
  assert.ok(!light.some((e) => e.name === "player.js"));
  // The Download pill on videos (pill.js, shared by player.js and caught.js): the full extension
  // only, and pill.js runs before player.js on the video sites.
  for (const f of ["pill.js", "caught.js"]) {
    assert.ok(firefox.some((e) => e.name === f), f);
    assert.ok(!light.some((e) => e.name === f), f);
  }
  for (const m of [folder, chromium]) assert.deepEqual(m.content_scripts[0].js, ["pill.js", "player.js"]);
  // "Videos on this page" and "Record this video": the full extension only.
  for (const f of ["recorder.js", "recorder-bridge.js"]) {
    assert.ok(firefox.some((e) => e.name === f), f);
    assert.ok(!light.some((e) => e.name === f), f);
  }
  assert.ok(JSON.parse(firefox.find((e) => e.name === "manifest.json").data.toString("utf8")).permissions.includes("webRequest"));
  for (const e of light) {
    if (e.name === "LICENSE" || !/\.(js|html|css|json|md)$/.test(e.name)) continue;
    assert.ok(!LIGHT_BANNED.test(e.data.toString("utf8")), e.name);
  }
  const manifest = JSON.parse(light.find((e) => e.name === "manifest.json").data.toString("utf8"));
  assert.equal(manifest.name, "Pako Light");
  assert.equal(manifest.key, undefined);
  assert.equal(manifest.content_scripts, undefined);
  assert.ok(!manifest.permissions.includes("webRequest"));
  // Catching a download at the server's answer (browsers that ask first): the full Chromium
  // extension only. Pako Light and Firefox keep their way: no rule permission, no sent page.
  assert.ok(chromium.permissions.includes("declarativeNetRequestWithHostAccess"));
  assert.ok(!chromium.permissions.includes("declarativeNetRequest"), "the one without an install warning");
  assert.deepEqual(chromium.web_accessible_resources, [{ resources: ["pako-sent.html"], matches: ["<all_urls>"], use_dynamic_url: true }]);
  for (const f of ["pako-sent.html", "pako-sent.js"]) assert.ok(readFileSync(join(dist, "chromium", f)), f);
  for (const m of [manifest, folder]) {
    assert.ok(!m.permissions.some((p) => p.startsWith("declarativeNetRequest")));
    assert.equal(m.web_accessible_resources, undefined);
  }
  for (const zip of [light, firefox]) assert.ok(!zip.some((e) => e.name.startsWith("pako-sent")));
  assert.match(light.find((e) => e.name === "background.js").data.toString("utf8"), /^const LIGHT = true;$/m);
});
