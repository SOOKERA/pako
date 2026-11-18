// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// A browser that keeps running an older background script than the folder's (seen in Vivaldi 8.3,
// Chromium 156): after Pako wrote a newer extension in the "Load unpacked" folder, the browser
// reads the new manifest but goes on running the script it took when the extension was added,
// even after a restart, until the extension is reloaded. The script says its own version
// (SCRIPT_VERSION): a newer script that finds itself older than its manifest reloads, and the
// toolbar window (always from the folder) reloads an older script that can't tell.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { ask, loadEdition, settle, MANIFEST_VERSION } from "./fake-browser.mjs";
import { toLight } from "../light.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const source = (f) => readFileSync(join(here, "..", f), "utf8");

test("The background script says the manifests' version", () => {
  const said = [...source("background.js").matchAll(/^const SCRIPT_VERSION = "([^"]*)";\r?$/gm)].map((m) => m[1]);
  assert.deepEqual(said, [MANIFEST_VERSION]);
  assert.equal(JSON.parse(source("manifest.firefox.json")).version, MANIFEST_VERSION);
});

test("A script older than its manifest reloads the extension once, never in a loop; the same version doesn't", async () => {
  const older = loadEdition({ manifestVersion: "9.9.9" });
  await settle(3200);
  assert.equal(older.reloads.length, 1, "reloaded for the folder's script");
  assert.equal(older.storage.reloadedFor, "9.9.9");
  // The browser still runs the old one after the reload: no second reload for that version.
  const still = loadEdition({ manifestVersion: "9.9.9", stored: older.storage });
  await settle(3200);
  assert.equal(still.reloads.length, 0);
  const same = loadEdition({});
  await settle(3200);
  assert.equal(same.reloads.length, 0, "this script is the folder's");
  // Pako Light comes from a store, which updates it properly.
  const light = loadEdition({ light: true, manifestVersion: "9.9.9" });
  await settle(3200);
  assert.equal(light.reloads.length, 0);
});

test("The background page's answer to the toolbar window says its script's version", async () => {
  const ext = loadEdition({ reply: () => ({ ok: true, holds: true }) });
  const r = await ask(ext, { type: "ping" });
  assert.equal(r.ok, true);
  assert.equal(r.script, MANIFEST_VERSION);
});

/** The toolbar window in a sandbox. `background`: what the background page answers to "ping"
 *  (undefined: no answer). */
function popup({ background, light = false, stored = {}, firefox = false } = {}) {
  const storage = { ...stored };
  const reloads = [];
  const elements = new Map();
  const element = () => {
    const t = { textContent: "", hidden: false, className: "", disabled: false, attrs: {} };
    return new Proxy(t, {
      get(o, k) {
        if (k in o) return o[k];
        if (k === "getAttribute") return (n) => o.attrs[n] ?? null;
        if (k === "setAttribute") return (n, v) => (o.attrs[n] = String(v));
        if (k === "classList") return { add() {}, remove() {}, toggle() {}, contains: () => false };
        if (k === "style") return {};
        return () => {};
      },
      set(o, k, v) {
        o[k] = v;
        return true;
      },
    });
  };
  const document = {
    title: "",
    getElementById: (id) => elements.get(id) ?? elements.set(id, element()).get(id),
    querySelectorAll: () => [],
    createElement: () => element(),
  };
  const chrome = {
    runtime: {
      id: "fnaplmfbaccplngclnkchlngpkbcbhod",
      lastError: null,
      getManifest: () => ({ version: MANIFEST_VERSION }),
      reload: () => reloads.push(Date.now()),
      sendMessage: async (m) => (m?.type === "ping" ? background : m?.type === "private" ? { allowed: false } : undefined),
      sendNativeMessage: (_h, _m, cb) => setTimeout(() => cb({ ok: true, holds: true }), 1),
      ...(firefox ? { getBrowserInfo: async () => ({ name: "Firefox" }) } : {}),
    },
    storage: {
      local: { get: async (d) => ({ ...d, ...storage }), set: async (o) => void Object.assign(storage, o) },
      session: { get: async (d) => ({ ...d }), set: async () => {} },
    },
    action: { setBadgeText() {} },
    tabs: { query: async () => [{ id: 3, url: "about:blank" }], create: async () => {}, update: async () => {} },
  };
  const context = { chrome, document, navigator: { userAgent: "Chrome" }, window: { close() {} }, console, setTimeout, clearTimeout, URL };
  if (firefox) context.browser = chrome;
  vm.runInContext(light ? toLight("popup.js", source("popup.js")) : source("popup.js"), vm.createContext(context));
  return { storage, reloads, status: () => document.getElementById("status").textContent };
}

test("The toolbar window reloads an older background script the browser kept (it can't tell itself), once per version", async () => {
  // An older script answers "ping" with Pako's own answer only (no `script`).
  const old = popup({ background: { ok: true, holds: true } });
  await settle(100);
  assert.match(old.status(), /Updating Pako's extension/);
  await settle(900);
  assert.equal(old.reloads.length, 1);
  assert.equal(old.storage.reloadedFor, MANIFEST_VERSION);
  // Opened again and still old (the reload didn't help): no loop.
  const again = popup({ background: { ok: true }, stored: old.storage });
  await settle(1000);
  assert.equal(again.reloads.length, 0);
  assert.equal(again.status(), "Connected");
  // The folder's own script, connected or not: nothing.
  for (const background of [{ ok: true, script: MANIFEST_VERSION }, { ok: false, error: "Pako isn't connected.", detail: "x", script: MANIFEST_VERSION }]) {
    const now = popup({ background });
    await settle(1000);
    assert.equal(now.reloads.length, 0, JSON.stringify(background));
  }
  // No answer at all (the background page just restarting): never a reload on that.
  const silent = popup({ background: undefined });
  await settle(1000);
  assert.equal(silent.reloads.length, 0);
  // Firefox updates its signed copy itself; Pako Light comes from a store.
  for (const opts of [{ firefox: true }, { light: true }]) {
    const other = popup({ background: { ok: true }, ...opts });
    await settle(1000);
    assert.equal(other.reloads.length, 0, JSON.stringify(opts));
  }
});
