// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Taking over a browser download, like IDM: with a Pako that answers at once ("held"), the
// browser's copy is cancelled straight away and only started again in the browser when Pako
// can't get the file. What can't be asked for again (a POST form, anything Pako Light can't
// tell apart) and an older Pako keep the old way: held paused until Pako confirms.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { loadEdition, settle } from "./fake-browser.mjs";

const URL_ = "https://files.example.com/big.zip";
const ITEM = { id: 7, url: URL_, referrer: "https://files.example.com/", mime: "application/zip", totalBytes: 50e6, filename: "" };

/** A Pako that answers at once: "held", then the check's `result` (after `pending` "still
 *  waiting" answers). Everything said to it, in order, is in `said`. */
function newPako({ result = { ok: true }, pending = 0 } = {}) {
  const said = [];
  let waits = pending;
  const reply = (m) => {
    said.push(m.kind + (m.hold ? "+hold" : ""));
    if (m.kind === "ping") return { ok: true, holds: true };
    if (m.kind === "held") {
      if (m.held !== "tok-1") return { ok: false, error: "Unknown.", holds: true };
      if (waits-- > 0) return { ok: false, held: "tok-1", holds: true };
      return { ...result, holds: true };
    }
    return m.hold ? { ok: true, held: "tok-1", holds: true } : { ok: true, holds: true };
  };
  return { said, reply };
}

function load(opts = {}, pako = newPako()) {
  const ext = loadEdition({ reply: pako.reply, ...opts });
  ext.storage.pakoHolds = true;
  return ext;
}

test("A Pako that answers at once: the browser's copy is cancelled straight away, Pako checks meanwhile", async () => {
  const pako = newPako();
  const ext = load({}, pako);
  await ext.listeners.download(ITEM);
  await settle(40);
  // Cancelled and erased before anything was asked of Pako; never paused, never started again.
  assert.deepEqual(ext.log, ["full cancel", "full erase"]);
  assert.deepEqual(pako.said, ["download+hold", "held"]);
  assert.equal(ext.sent[0].takeover, true);
  assert.equal(ext.sent[0].hold, true);
  assert.equal(ext.sent[0].totalBytes, 50e6);
  assert.equal(ext.sent[0].mime, "application/zip");
  assert.deepEqual(ext.sent[1], { kind: "held", held: "tok-1" });
  assert.deepEqual(ext.restarted, []);
  assert.equal(ext.storage.badge ?? "", "");
});

test("While Pako's window still waits for an answer, the extension asks again", async () => {
  const pako = newPako({ pending: 2 });
  const ext = load({}, pako);
  await ext.listeners.download(ITEM);
  await settle(60);
  assert.deepEqual(pako.said, ["download+hold", "held", "held", "held"]);
  assert.deepEqual(ext.restarted, []);
});

test("Pako can't get the file: the browser downloads it again itself, with the '!' and the reason", async () => {
  const why = "Pako couldn't get this download (the server gave Pako a web page instead of the file), so your browser downloads it.";
  const ext = load({}, newPako({ result: { ok: false, error: why } }));
  await ext.listeners.download({ ...ITEM, filename: "C:\\Downloads\\big.zip", cookieStoreId: "firefox-container-2" });
  await settle(40);
  assert.deepEqual(ext.log, ["full cancel", "full erase", "full download"]);
  assert.equal(ext.restarted.length, 1);
  assert.equal(ext.restarted[0].url, URL_);
  assert.equal(ext.restarted[0].filename, "big.zip");
  assert.equal(ext.restarted[0].conflictAction, "uniquify");
  assert.equal(ext.storage.badge, "!");
  assert.equal(ext.storage.lastError, why);
  // That new browser download is never taken over again: the browser marks it, and the
  // extension remembers the address for a while.
  const before = ext.sent.length;
  await ext.listeners.download({ ...ITEM, id: 8, byExtensionId: ext.id });
  await ext.listeners.download({ ...ITEM, id: 9 });
  await settle(30);
  assert.equal(ext.sent.length, before);
});

test("Pako not reachable after the copy was cancelled: started again in the browser", async () => {
  const ext = load({ reply: () => undefined });
  await ext.listeners.download(ITEM);
  await settle(40);
  assert.deepEqual(ext.log, ["full cancel", "full erase", "full download"]);
  assert.equal(ext.storage.badge, "!");
});

test("Declined (a video page, a Drive folder): started again in the browser, no warning", async () => {
  const ext = load({ reply: (m) => (m.kind === "download" ? { ok: false, holds: true } : { ok: true }) });
  await ext.listeners.download(ITEM);
  await settle(40);
  assert.equal(ext.restarted.length, 1);
  assert.equal(ext.storage.badge ?? "", "");
});

test("The browser can't start it again: the '!' says to open the link again", async () => {
  const ext = load({ restartFails: true }, newPako({ result: { ok: false, error: "Pako couldn't get this download (no answer)." } }));
  await ext.listeners.download(ITEM);
  await settle(40);
  assert.equal(ext.storage.badge, "!");
  assert.match(ext.storage.lastError, /^Pako couldn't get this download \(no answer\)\. Your browser couldn't start it again: open the link again/);
});

test("A file a form sent with POST can't be asked for again: held paused until Pako confirms", async () => {
  for (const result of [{ ok: true }, { ok: false, error: "Pako couldn't get this download (x)." }]) {
    const pako = newPako({ result });
    const ext = load({}, pako);
    ext.listeners.headers({
      tabId: 3,
      url: URL_,
      type: "main_frame",
      method: "POST",
      statusCode: 200,
      responseHeaders: [
        { name: "Content-Disposition", value: "attachment; filename=\"report.zip\"" },
        { name: "Content-Type", value: "application/zip" },
      ],
    });
    await ext.listeners.download(ITEM);
    await settle(40);
    // Pako still opens its window at once (hold), but the browser keeps its paused copy.
    assert.deepEqual(pako.said, ["download+hold", "held"]);
    assert.deepEqual(ext.log, result.ok ? ["full pause", "full cancel", "full erase"] : ["full pause", "full resume"]);
    assert.deepEqual(ext.restarted, []);
  }
});

test("The server's own answer fills in what the browser doesn't know yet (size, kind, name)", async () => {
  const ext = load();
  ext.listeners.headers({
    tabId: 3,
    url: URL_,
    type: "sub_frame",
    method: "GET",
    statusCode: 200,
    responseHeaders: [
      { name: "Content-Disposition", value: "attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.zip" },
      { name: "Content-Type", value: "application/octet-stream" },
      { name: "Content-Length", value: "12345678" },
    ],
  });
  await ext.listeners.download({ ...ITEM, totalBytes: 0, mime: "" });
  await settle(40);
  assert.equal(ext.sent[0].filename, "résumé.zip");
  assert.equal(ext.sent[0].totalBytes, 12345678);
  assert.equal(ext.sent[0].mime, "application/octet-stream");
  // A page the browser shows itself is not noted.
  ext.listeners.headers({ tabId: 3, url: "https://news.example/", type: "main_frame", method: "POST", statusCode: 200, responseHeaders: [{ name: "Content-Type", value: "text/html" }] });
  await ext.listeners.download({ ...ITEM, id: 11, url: "https://news.example/" });
  await settle(40);
  assert.ok(!ext.log.includes("full pause"));
});

test("An older Pako (no 'holds' in its answers): paused until it confirms, as before", async () => {
  const said = [];
  const ext = loadEdition({ reply: (m) => (said.push(m), { ok: true }) });
  await ext.listeners.download(ITEM);
  await settle(40);
  assert.deepEqual(ext.log, ["full pause", "full cancel", "full erase"]);
  assert.equal(said[0].hold, undefined);
  // It said nothing about answering at once: remembered as such.
  assert.notEqual(ext.storage.pakoHolds, true);
  // An error: the paused copy carries on in the browser.
  const failing = loadEdition({ reply: () => ({ ok: false, error: "Pako couldn't start this download (x), so the browser keeps it." }) });
  await failing.listeners.download(ITEM);
  await settle(40);
  assert.deepEqual(failing.log, ["full pause", "full resume"]);
  assert.equal(failing.storage.badge, "!");
});

test("The extension learns that Pako answers at once from any of its answers", async () => {
  const ext = loadEdition({ reply: () => ({ ok: true, holds: true }) });
  await ext.listeners.installed?.();
  await settle(30);
  assert.equal(ext.storage.pakoHolds, true);
  await ext.listeners.download(ITEM);
  await settle(40);
  assert.deepEqual(ext.log, ["full cancel", "full erase"]);
});

test("Pako Light can't tell a POST form apart: it always holds the browser's copy paused", async () => {
  const pako = newPako();
  const light = load({ light: true }, pako);
  await light.listeners.download(ITEM);
  await settle(40);
  assert.deepEqual(light.log, ["light pause", "light cancel", "light erase"]);
  assert.deepEqual(pako.said, ["download+hold", "held"]);
});

test("Chromium: taken at the file-name step, before any 'Save as' (the browser is let go after)", async () => {
  const pako = newPako();
  const ext = load({ naming: true }, pako);
  assert.equal(typeof ext.listeners.naming, "function");
  // onCreated leaves it to the file-name step.
  await ext.listeners.download(ITEM);
  await settle(20);
  assert.deepEqual(ext.log, []);
  const order = [];
  const keep = ext.log.push.bind(ext.log);
  ext.log.push = (x) => (order.push(x), keep(x));
  const async_ = ext.listeners.naming(ITEM, () => order.push("suggest"));
  assert.equal(async_, true, "the browser waits for the answer");
  await settle(40);
  assert.deepEqual(order, ["full cancel", "full erase", "suggest"]);
  assert.deepEqual(pako.said, ["download+hold", "held"]);
  // Not taken (switched off): let go at once, untouched.
  const off = load({ naming: true });
  off.storage.takeOver = false;
  let let_go = false;
  off.listeners.naming(ITEM, () => (let_go = true));
  await settle(20);
  assert.equal(let_go, true);
  assert.deepEqual(off.log, []);
});
