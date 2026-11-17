// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The background page starting again (the browser restarted it, or Reload in its extensions
// page) with what an earlier version left in storage: it still answers the pill and the
// player's button, drops and clears stored data it can't use, and never reloads itself in a
// loop. And the pill says what really went wrong when the extension can't be reached: "just
// updated" only for a page whose script is from before an update.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { ask, loadEdition, settle } from "./fake-browser.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const TAB = 5;
const VIDEO = "https://www.youtube.com/watch?v=abcdefghijk";

/** What older versions (or damage) may have left in storage. */
const LEFT = {
  follows: {
    7: null,
    8: "junk",
    9: { url: 42 },
    junk: { url: "https://e.example/a.zip" },
    11: { pako: "row-11", url: "https://e.example/b.zip", name: "b.zip", private: false, sent: 0, state: "in_progress", busy: true },
  },
  pakoHolds: "yes",
  reloadedFor: 17,
  takeOver: true,
};

test("Started again with odd stored data: the player's button and the pill still get answers", async () => {
  const ext = loadEdition({ tabs: { [TAB]: VIDEO }, stored: LEFT, sessionStored: { privateFollows: [1, 2], [`caught-${TAB}`]: "junk" } });
  await settle(60);
  const video = await ask(ext, { type: "video", url: VIDEO, quality: 720 }, { tab: { id: TAB, url: VIDEO } });
  assert.equal(video.ok, true);
  assert.equal(ext.sent.filter((m) => m.kind === "video").length, 1);
  const menu = await ask(ext, { type: "pill-menu" }, { tab: { id: TAB } });
  assert.equal(menu.ok, true);
  assert.ok(Array.isArray(menu.items));
  // What it couldn't use is cleared; the good follow stays (never "busy").
  assert.deepEqual(Object.keys(ext.storage.follows), ["11"]);
  assert.equal(ext.storage.follows[11].busy, undefined);
  assert.deepEqual(ext.session.privateFollows, {});
});

test("An unknown request gets an answer that says so (never silence)", async () => {
  const ext = loadEdition({ tabs: { [TAB]: VIDEO } });
  const r = await ask(ext, { type: "nonsense" }, { tab: { id: TAB } });
  assert.equal(r.ok, false);
  assert.match(r.error, /didn't understand/);
});

test("The message listener is registered first, before any other start-up work", () => {
  const src = readFileSync(join(here, "..", "background.js"), "utf8");
  const first = src.indexOf("api.runtime.onMessage.addListener(");
  assert.ok(first > 0);
  for (const later of ["loadFollows()", "removeRecorder();", "api.contextMenus.onClicked.addListener", "api.downloads.onCreated.addListener"]) {
    assert.ok(src.indexOf(later) > first, `${later} comes after the message listener`);
  }
});

test("Never two reloads within minutes, even when the folder says another newer version", async () => {
  const reply = () => ({ ok: true, ext: "3.0.0" });
  const ext = loadEdition({ reply, stored: { reloadedFor: "2.0.0", reloadedAt: Date.now() - 60_000 } });
  await ext.listeners.installed?.({ reason: "update" });
  await settle(3200);
  assert.equal(ext.pings.length, 1);
  assert.equal(ext.reloads.length, 0);
});

/** pill.js (and caught.js on it) in a sandbox: what the page's script says when the extension
 *  can't be reached. `send`: what runtime.sendMessage does. */
function pillKit(send, extra = {}) {
  const warned = [];
  const storage = {};
  const chrome = {
    runtime: { sendMessage: send },
    storage: { local: { set: async (o) => Object.assign(storage, o), get: async (d) => d }, onChanged: { addListener() {} } },
  };
  const context = { chrome, console: { warn: (...a) => warned.push(a.join(" ")) }, ...extra };
  context.globalThis = context;
  vm.createContext(context);
  vm.runInContext(readFileSync(join(here, "..", "pill.js"), "utf8"), context);
  let made = null;
  context.__pakoPill.make = (o) => (made = o);
  vm.runInContext(readFileSync(join(here, "..", "caught.js"), "utf8"), context);
  return { kit: context.__pakoPill, made, warned, storage };
}

test("The pill says 'just updated' only for a page from before an update; otherwise the real problem", async () => {
  const old = pillKit(async () => {
    throw new Error("Extension context invalidated.");
  });
  const items = await old.made.items();
  assert.match(items[1].info, /just updated/);
  assert.equal(old.warned.length, 0);

  const gone = pillKit(async () => {
    throw new Error("Could not establish connection. Receiving end does not exist.");
  });
  const said = await gone.made.items();
  assert.match(said[1].info, /isn't responding/);
  assert.match(said[1].detail, /Receiving end does not exist/);
  assert.equal(gone.warned.length, 1, "logged in the console");
  assert.match(gone.storage.lastDetail, /Receiving end/, "the toolbar window shows it too");
});

test("A page element named 'browser' never stands in for the extension's API", async () => {
  const k = pillKit(async () => ({ ok: true, items: [{ head: "Download with Pako" }] }), { browser: { tagName: "DIV" } });
  const items = await k.made.items();
  assert.equal(items[0].head, "Download with Pako");
  assert.equal(items.length, 1);
});
