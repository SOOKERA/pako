// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Everything sent from a private window says so (`private: true`), not only a taken-over
// download: the right-click items, all links on the page, the toolbar window's Send, the player's
// Download button and the pill. A video from a private window carries no cookies (Pako would
// have to write them to a file for its video tool), and Record this video is refused there.
// A normal window's messages are as before (no `private` field).
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { ask, loadEdition, settle } from "./fake-browser.mjs";

const PRIV = 3;
const NORMAL = 4;
const PAGE = "https://files.example.com/page";
const VIDEO = "https://www.youtube.com/watch?v=abcdefghijk";

function load(opts = {}) {
  return loadEdition({
    tabs: { [PRIV]: PAGE, [NORMAL]: PAGE },
    privateTabs: [PRIV],
    links: ["https://files.example.com/a.zip", "https://files.example.com/b.zip"],
    privateAllowed: true,
    ...opts,
  });
}
const tab = (id, url = PAGE) => ({ id, url, incognito: id === PRIV });

test("Right-click items from a private window are private; from a normal one, as before", async () => {
  const ext = load();
  const items = [
    [{ menuItemId: "pako-link", linkUrl: "https://files.example.com/a.zip", pageUrl: PAGE }, tab(PRIV)],
    [{ menuItemId: "pako-media", srcUrl: "https://files.example.com/p.jpg", mediaType: "image", pageUrl: PAGE }, tab(PRIV)],
    // "Download video with Pako" / "Download audio with Pako" on a player with its own address.
    [{ menuItemId: "pako-media-video", srcUrl: "https://files.example.com/v.mp4", mediaType: "video", pageUrl: PAGE }, tab(PRIV)],
    [{ menuItemId: "pako-media-audio", srcUrl: "https://files.example.com/s.mp3", mediaType: "audio", pageUrl: PAGE }, tab(PRIV)],
    [{ menuItemId: "pako-page", pageUrl: PAGE }, tab(PRIV)],
    // "Download video with Pako" on a video page.
    [{ menuItemId: "pako-page", pageUrl: VIDEO }, tab(PRIV, VIDEO)],
    [{ menuItemId: "pako-link", linkUrl: VIDEO, pageUrl: PAGE }, tab(PRIV)],
  ];
  for (const [info, t] of items) await ext.listeners.click(info, t);
  await settle();
  assert.equal(ext.sent.length, items.length);
  for (const m of ext.sent) {
    assert.equal(m.private, true, `${m.urls[0]} is private`);
    // No login from a private window goes with a video.
    if (m.siteCookies) assert.deepEqual(m.siteCookies, []);
  }
  const normal = load();
  await normal.listeners.click({ menuItemId: "pako-link", linkUrl: "https://files.example.com/a.zip", pageUrl: PAGE }, tab(NORMAL));
  await normal.listeners.click({ menuItemId: "pako-link", linkUrl: VIDEO, pageUrl: PAGE }, tab(NORMAL));
  await settle();
  assert.equal(normal.sent.length, 2);
  for (const m of normal.sent) assert.equal("private" in m, false, "an ordinary window's message is as before");
  assert.equal(normal.sent[1].siteCookies.length, 1, "a normal window's video keeps its site's login");
});

test("Send all links on this page from a private window: every part is private", async () => {
  const ext = load();
  await ext.listeners.click({ menuItemId: "pako-all", pageUrl: PAGE }, tab(PRIV));
  await settle();
  assert.equal(ext.sent.length, 1);
  assert.equal(ext.sent[0].kind, "links");
  assert.equal(ext.sent[0].private, true);
  const normal = load();
  await normal.listeners.click({ menuItemId: "pako-all", pageUrl: PAGE }, tab(NORMAL));
  await settle();
  assert.equal("private" in normal.sent[0], false);
});

test("The toolbar window's Send on a private window's tab is private", async () => {
  const ext = load();
  await ask(ext, { type: "send-tab", url: PAGE, incognito: true });
  await ask(ext, { type: "send-tab", url: PAGE, incognito: false });
  assert.equal(ext.sent[0].private, true);
  assert.equal("private" in ext.sent[1], false);
});

test("The player's Download button in a private window: private, with no cookies", async () => {
  const ext = load();
  const reply = await ask(ext, { type: "video", url: VIDEO, quality: 720 }, { tab: tab(PRIV, VIDEO) });
  assert.equal(reply.ok, true);
  assert.equal(ext.sent[0].kind, "video");
  assert.equal(ext.sent[0].private, true);
  assert.deepEqual(ext.sent[0].siteCookies, []);
  assert.deepEqual(ext.cookieStores, [], "no cookie store was even read");
  await ask(ext, { type: "video", url: VIDEO, quality: 720 }, { tab: tab(NORMAL, VIDEO) });
  assert.equal("private" in ext.sent[1], false);
});

test("The pill and Videos on this page in a private window: the stream is private; Record is refused", async () => {
  const ext = load();
  const m3u8 = "https://cdn.example.net/hls/index.m3u8";
  ext.listeners.headers({ tabId: PRIV, url: m3u8, type: "xmlhttprequest", statusCode: 200, responseHeaders: [], incognito: true, frameId: 0 });
  await settle(40);
  assert.equal((await ask(ext, { type: "pill-send", url: m3u8 }, { tab: { id: PRIV } })).ok, true);
  assert.equal((await ask(ext, { type: "send-stream", tabId: PRIV, url: m3u8 })).ok, true);
  assert.equal(ext.sent.length, 2);
  for (const m of ext.sent) {
    assert.equal(m.kind, "stream");
    assert.equal(m.private, true);
    assert.deepEqual(m.siteCookies, []);
  }
  const rec = await ask(ext, { type: "record-start", tabId: PRIV });
  assert.equal(rec.ok, false);
  assert.match(rec.error, /private windows/);
  assert.equal((await ask(ext, { type: "pill-record" }, { tab: { id: PRIV } })).ok, false);
  assert.equal(ext.port.length, 0, "nothing of a recording reached Pako");
});

test("A problem with something from a private window is kept in memory only, never in the extension's storage on disk", async () => {
  const said = "The video tool said: [youtube] abcdefghijk: something unexpected";
  const ext = load({ reply: () => ({ ok: false, error: said }) });
  await ask(ext, { type: "video", url: VIDEO, quality: 720 }, { tab: tab(PRIV, VIDEO) });
  await ext.listeners.click({ menuItemId: "pako-link", linkUrl: "https://files.example.com/a.zip", pageUrl: PAGE }, tab(PRIV));
  await settle();
  assert.equal(ext.storage.lastError, undefined, "nothing written to storage.local");
  assert.equal(ext.session.lastError, said, "the toolbar window still says it (from memory)");
  assert.equal(ext.storage.badge, "!");
  const normal = load({ reply: () => ({ ok: false, error: said }) });
  await ask(normal, { type: "video", url: VIDEO, quality: 720 }, { tab: tab(NORMAL, VIDEO) });
  await settle();
  assert.equal(normal.storage.lastError, said, "a normal window's problem: as before");
  assert.equal(normal.session.lastError, undefined);
});
