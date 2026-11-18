// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The full extension and Pako Light in one browser: only one of them takes over a download and
// shows right-click items (audit 2026-11-16 evening, H2). Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { ask, browserProfile, loadEdition, plain, settle } from "./fake-browser.mjs";

const ITEM = { id: 7, url: "https://files.example.com/big.zip", referrer: "", mime: "application/zip" };

function both() {
  const profile = browserProfile();
  const onCreated = [];
  const full = loadEdition({ light: false, profile, downloadsOnCreated: onCreated });
  const light = loadEdition({ light: true, profile, downloadsOnCreated: onCreated });
  return { full, light, onCreated };
}

test("With the full extension on, one browser download reaches Pako once", async () => {
  const { full, light, onCreated } = both();
  await Promise.all(onCreated.map((f) => f(ITEM)));
  await settle(50);
  assert.equal(full.sent.length, 1);
  assert.deepEqual(full.sent[0].urls, [ITEM.url]);
  assert.equal(full.sent[0].takeover, true);
  assert.deepEqual(light.sent, []);
  // Light didn't touch the browser's download either.
  assert.deepEqual(light.log, []);
  assert.deepEqual(full.log, ["full pause", "full cancel", "full erase"]);
});

test("With the full extension on, Light shows no right-click items and says so in its popup", async () => {
  const { full, light } = both();
  await full.listeners.installed();
  await light.listeners.installed();
  await settle();
  assert.equal(full.menus.size, 6, "link, image, video, audio, page, all links");
  assert.equal(light.menus.size, 0);
  const edition = await ask(light, { type: "edition", url: "https://news.example/" });
  assert.equal(edition.fullOn, true);
  assert.equal(edition.light, true);
});

test("Without the full extension, Light takes over and shows its items", async () => {
  const profile = browserProfile();
  const light = loadEdition({ light: true, profile });
  await light.listeners.installed();
  await light.listeners.download(ITEM);
  await settle(50);
  assert.equal(light.menus.size, 4);
  assert.equal(light.sent.length, 1);
  assert.equal(light.sent[0].takeover, true);
  const edition = await ask(light, { type: "edition", url: "https://www.youtube.com/" });
  assert.equal(edition.fullOn, false);
  assert.equal(edition.skipped, true);
});

test("The full extension answers only Pako Light", async () => {
  const { full } = both();
  let answer = "none";
  full.listeners.external({ type: "pako-full-here" }, { id: "someotherextensionidaaaaaaaaaaaa" }, (r) => (answer = r));
  assert.equal(answer, "none");
  full.listeners.external({ type: "pako-full-here" }, { id: "cojnnpgiilnbkblckkjnheigiodkdlpl" }, (r) => (answer = plain(r)));
  assert.deepEqual(answer, { here: true });
});
