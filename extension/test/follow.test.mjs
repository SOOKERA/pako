// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// "Via browser": when Pako can't get a file itself, the browser downloads it and Pako follows it
// as one of its own (progress, Pause / Resume / Cancel from Pako, the finished file moved and
// the browser's list entry erased; reports kept while Pako is closed). And private windows:
// taken over only where the browser lets Pako work there, with that window's own cookies.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { ask, loadEdition, settle } from "./fake-browser.mjs";

const URL_ = "https://files.example.com/big.zip";
const ITEM = { id: 7, url: URL_, referrer: "https://files.example.com/", mime: "application/zip", totalBytes: 50e6, filename: "" };
const WHY = "Pako couldn't get this download itself (the server refused it), so your browser downloads it and Pako follows it.";

/** A Pako that answers at once and can't get the file: the row "row-1" follows the browser.
 *  `follow` answers each report (default: ok). `up` = false: Pako unreachable for reports. */
function pako({ follow = (m) => ({ ok: true, followed: "row-1", ...(m.follow.state === "complete" ? { command: "forget" } : {}) }) } = {}) {
  const state = { up: true, reports: [] };
  state.reply = (m) => {
    if (m.kind === "ping") return { ok: true, holds: true };
    if (m.kind === "held") return { ok: false, error: WHY, followed: "row-1", holds: true };
    if (m.kind === "follow") {
      if (!state.up) return undefined;
      state.reports.push(m);
      return { ...follow(m), holds: true };
    }
    return m.hold ? { ok: true, held: "row-1", holds: true } : { ok: true, holds: true };
  };
  return state;
}

function load(p, opts = {}) {
  const ext = loadEdition({ reply: (m) => p.reply(m), ...opts });
  ext.storage.pakoHolds = true;
  return ext;
}

test("Pako can't get it: the browser downloads it and Pako follows it as its own", async () => {
  const p = pako();
  const ext = load(p);
  await ext.listeners.download(ITEM);
  await settle(40);
  // Started again in the browser (id 99), reported to Pako's row at once.
  assert.equal(ext.restarted.length, 1);
  assert.equal(p.reports.length, 1);
  const first = p.reports[0];
  assert.deepEqual([...first.urls], [URL_]);
  assert.equal(first.follow.pako, "row-1");
  assert.equal(first.follow.browserId, 99);
  assert.equal(first.follow.state, "in_progress");
  // Pako's row says it (no "!" on the toolbar button: Pako has it).
  assert.equal(ext.storage.badge ?? "", "");
  assert.ok(ext.storage.follows[99]);
});

test("Progress every 2 seconds; a change of state at once", async () => {
  const p = pako();
  const ext = load(p);
  await ext.listeners.download(ITEM);
  await settle(40);
  ext.items[99] = { bytesReceived: 1000 };
  ext.listeners.changed({ id: 99 });
  ext.listeners.changed({ id: 99 });
  await settle(30);
  assert.equal(p.reports.length, 1, "not more than every 2 seconds");
  ext.items[99] = { bytesReceived: 2000, paused: true };
  ext.listeners.changed({ id: 99, paused: { current: true } });
  await settle(30);
  assert.equal(p.reports.length, 2);
  assert.equal(p.reports[1].follow.state, "paused");
  assert.equal(p.reports[1].follow.bytes, 2000);
  // About 2 seconds later, the next progress goes by itself.
  ext.items[99] = { bytesReceived: 3000, paused: false };
  await settle(3300);
  assert.ok(p.reports.length >= 3);
  assert.equal(p.reports.at(-1).follow.bytes, 3000);
});

test("Pause, Resume and Cancel in Pako are done by the browser", async () => {
  let command = "pause";
  const p = pako({ follow: () => ({ ok: true, followed: "row-1", command }) });
  const ext = load(p);
  await ext.listeners.download(ITEM);
  await settle(40);
  assert.ok(ext.log.includes("full pause 99"));
  command = "resume";
  ext.items[99] = { paused: true };
  ext.listeners.changed({ id: 99, paused: { current: true } });
  await settle(30);
  assert.ok(ext.log.includes("full resume 99"));
  command = "cancel";
  ext.items[99] = { paused: false };
  ext.listeners.changed({ id: 99, paused: { current: false } });
  await settle(30);
  assert.ok(ext.log.includes("full cancel 99") && ext.log.includes("full erase 99"));
  assert.equal(ext.storage.follows[99], undefined, "not followed any more");
});

test("Finished: reported with the browser's file, then the browser's list forgets it", async () => {
  const p = pako();
  const ext = load(p);
  await ext.listeners.download(ITEM);
  await settle(40);
  ext.items[99] = { state: "complete", bytesReceived: 50e6, filename: "C:\\Downloads\\big (1).zip" };
  ext.listeners.changed({ id: 99, state: { current: "complete" } });
  await settle(30);
  const last = p.reports.at(-1).follow;
  assert.equal(last.state, "complete");
  assert.equal(last.path, "C:\\Downloads\\big (1).zip");
  assert.equal(last.total, 50e6);
  assert.ok(ext.log.includes("full erase 99"));
  assert.equal(ext.storage.follows[99], undefined);
});

test("Pako closed when the browser finished: kept, and listed once Pako answers again", async () => {
  const p = pako();
  const ext = load(p);
  await ext.listeners.download(ITEM);
  await settle(40);
  p.up = false;
  ext.items[99] = { state: "complete", bytesReceived: 50e6, filename: "C:\\Downloads\\big (1).zip" };
  ext.listeners.changed({ id: 99, state: { current: "complete" } });
  await settle(30);
  assert.ok(ext.storage.follows[99], "kept for later");
  assert.ok(!ext.log.includes("full erase 99"), "still in the browser's list");
  // Pako answers again (here: the toolbar window checks the line).
  p.up = true;
  await ask(ext, { type: "ping" });
  await settle(3300);
  assert.equal(p.reports.at(-1).follow.state, "complete");
  assert.ok(ext.log.includes("full erase 99"));
  assert.equal(ext.storage.follows[99], undefined);
});

test("Held paused (a POST form) and Pako can't get it: the browser carries on, and Pako follows that one", async () => {
  const p = pako();
  const ext = load(p);
  ext.listeners.headers({
    tabId: 3,
    url: URL_,
    type: "main_frame",
    method: "POST",
    statusCode: 200,
    responseHeaders: [{ name: "Content-Disposition", value: "attachment" }],
  });
  await ext.listeners.download(ITEM);
  await settle(40);
  assert.deepEqual(ext.log.slice(0, 2), ["full pause", "full resume"]);
  assert.equal(ext.restarted.length, 0);
  assert.equal(p.reports[0].follow.browserId, 7);
});

test("Private windows: ignored unless the browser lets Pako work there", async () => {
  const p = pako();
  const off = load(p);
  await off.listeners.download({ ...ITEM, incognito: true });
  await settle(30);
  assert.deepEqual(off.log, []);
  assert.deepEqual(off.sent, []);
  // Allowed: taken over like any other, with the private window's own cookies only.
  const okPako = { reply: (m) => (m.kind === "held" ? { ok: true, holds: true } : m.hold ? { ok: true, held: "r", holds: true } : { ok: true, holds: true }) };
  const on = load(okPako, { privateAllowed: true });
  await on.listeners.download({ ...ITEM, incognito: true });
  await settle(40);
  // (Chrome and similar: held paused, never started again outside the private window.)
  assert.deepEqual(on.log, ["full pause", "full cancel", "full erase"]);
  assert.equal(on.sent[0].private, true);
  assert.equal(on.sent[0].paused, true);
  assert.equal(on.sent[0].cookies, "SID=private");
  assert.deepEqual(on.cookieStores, ["1"], "never the normal store");
  // An ordinary window's download says nothing about private.
  const normal = load(okPako, { privateAllowed: true });
  await normal.listeners.download(ITEM);
  await settle(40);
  assert.equal(normal.sent[0].private, undefined);
});

test("The toolbar window says whether private windows are on", async () => {
  const off = load(pako());
  const on = load(pako(), { privateAllowed: true });
  assert.equal((await ask(off, { type: "private" })).allowed, false);
  assert.equal((await ask(on, { type: "private" })).allowed, true);
});

test("Finished but not moved yet: followed until Pako says; Pako couldn't move it: the browser's list keeps it", async () => {
  let command;
  const p = pako({ follow: () => ({ ok: true, followed: "row-1", command }) });
  const ext = load(p);
  await ext.listeners.download(ITEM);
  await settle(40);
  ext.items[99] = { state: "complete", bytesReceived: 50e6, filename: "D:\Downloads\big.zip" };
  ext.listeners.changed({ id: 99, state: { current: "complete" } });
  await settle(30);
  assert.ok(!ext.log.includes("full erase 99"), "Pako is still moving it");
  assert.ok(ext.storage.follows[99]);
  command = "keep";
  await settle(3300);
  assert.ok(!ext.log.includes("full erase 99"), "not moved: the browser's list keeps it");
  assert.equal(ext.storage.follows[99], undefined);
});

test("A browser number now used by another download is never reported as this one", async () => {
  const p = pako();
  const ext = load(p);
  await ext.listeners.download(ITEM);
  await settle(40);
  const before = p.reports.length;
  ext.items[99] = { url: "https://elsewhere.example/mine.pdf", state: "complete", filename: "C:\Users\me\Downloads\mine.pdf" };
  ext.listeners.changed({ id: 99, state: { current: "complete" } });
  await settle(30);
  assert.equal(p.reports.length, before);
  assert.equal(ext.storage.follows[99], undefined);
  assert.ok(!ext.log.includes("full erase 99"));
});

test("Stopped by the browser: followed on while it can continue (Try again in Pako), dropped when it can't", async () => {
  let command;
  const p = pako({ follow: () => ({ ok: true, followed: "row-1", command }) });
  const ext = load(p);
  await ext.listeners.download(ITEM);
  await settle(40);
  ext.items[99] = { state: "interrupted", error: "NETWORK_FAILED", canResume: true };
  ext.listeners.changed({ id: 99, state: { current: "interrupted" } });
  await settle(30);
  assert.ok(ext.storage.follows[99], "Try again can still reach it");
  command = "resume";
  await settle(5300);
  assert.ok(ext.log.includes("full resume 99"));
  command = undefined;
  ext.items[99] = { state: "interrupted", error: "SERVER_FORBIDDEN", canResume: false };
  ext.listeners.changed({ id: 99, error: { current: "SERVER_FORBIDDEN" } });
  await settle(30);
  ext.listeners.changed({ id: 99, state: { current: "interrupted" } });
  await settle(30);
  assert.equal(ext.storage.follows[99], undefined);
});

test("A private window's download: followed in memory only, and its checks say they're private", async () => {
  const p = pako();
  const ext = load(p, { privateAllowed: true });
  await ext.listeners.download({ ...ITEM, incognito: true });
  await settle(60);
  const held = ext.sent.find((m) => m.kind === "held");
  assert.equal(held.private, true);
  assert.equal(p.reports[0].private, true);
  assert.equal(p.reports[0].follow.browserId, 7, "the paused private download, never one started again");
  assert.equal(ext.restarted.length, 0);
  assert.deepEqual(Object.keys(ext.storage.follows || {}), [], "nothing on disk");
  assert.ok(ext.session.privateFollows[7]);
});
