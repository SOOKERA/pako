// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// "Videos on this page" (full extension only): what is caught from the browser's own requests,
// the list per tab, the toolbar count, what goes to Pako, and that Pako Light has none of it.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { ask, loadEdition, settle } from "./fake-browser.mjs";
import { toLight, LIGHT_BANNED } from "../light.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const TAB = 4;
const PAGE = "https://watch.example.org/v/12";

const MASTER = [
  "#EXTM3U",
  '#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="a",URI="audio/en.m3u8"',
  "#EXT-X-STREAM-INF:BANDWIDTH=900000,RESOLUTION=1280x720",
  "720/index.m3u8",
  "#EXT-X-STREAM-INF:BANDWIDTH=3000000,RESOLUTION=1920x1080",
  "1080/index.m3u8",
].join("\n");

function full(opts = {}) {
  return loadEdition({
    tabs: { [TAB]: PAGE },
    fetch: (url) => (url.includes("master.m3u8") ? MASTER : url.includes(".m3u8") ? "#EXTM3U\n#EXTINF:4,\na.ts\n" : null),
    ...opts,
  });
}

const seen = (ext, url, type, headers = {}, extra = {}) =>
  ext.listeners.headers({
    tabId: TAB,
    url,
    type,
    statusCode: 200,
    responseHeaders: Object.entries(headers).map(([name, value]) => ({ name, value })),
    ...extra,
  });

test("Catches HLS, DASH and whole video files; leaves pieces, small files and other requests", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/hls/master.m3u8?t=1", "xmlhttprequest", { "content-type": "application/vnd.apple.mpegurl" });
  seen(ext, "https://cdn.example.net/hls/720/index.m3u8", "xmlhttprequest");
  seen(ext, "https://cdn.example.net/hls/audio/en.m3u8", "xmlhttprequest");
  seen(ext, "https://cdn.example.net/hls/720/seg1.ts", "xmlhttprequest", { "content-type": "video/mp2t", "content-length": "900000" });
  seen(ext, "https://cdn.example.net/dash/stream", "xmlhttprequest", { "content-type": "application/dash+xml" });
  seen(ext, "https://files.example.net/clip.mp4", "media", { "content-type": "video/mp4", "content-range": "bytes 0-1/50000000" });
  seen(ext, "https://files.example.net/clip.mp4", "media", { "content-type": "video/mp4", "content-range": "bytes 2-9/50000000" });
  seen(ext, "https://ads.example.net/tiny.mp4", "media", { "content-type": "video/mp4", "content-length": "40000" });
  seen(ext, "https://cdn.example.net/piece.mp4", "xmlhttprequest", { "content-type": "video/mp4", "content-length": "5000000" });
  seen(ext, "https://cdn.example.net/page.html", "xmlhttprequest", { "content-type": "text/html" });
  seen(ext, "blob:https://watch.example.org/1", "media", { "content-type": "video/mp4" });
  ext.listeners.headers({ tabId: -1, url: "https://x.example/a.m3u8", type: "xmlhttprequest" });
  await settle(60);
  const r = await ask(ext, { type: "videos", tabId: TAB });
  // The master playlist stands for its qualities and its sound.
  assert.deepEqual(
    r.list.map((v) => [v.kind, v.host, v.height]),
    [
      ["hls", "cdn.example.net", 1080],
      ["dash", "cdn.example.net", 0],
      ["file", "files.example.net", 0],
    ],
  );
  assert.equal(ext.badges[TAB], "3");
  // Nothing went to Pako by itself.
  assert.deepEqual(ext.sent, []);
});

test("A new page in the tab, or the tab closing, clears its list", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/v.m3u8", "xmlhttprequest");
  await settle(40);
  assert.equal(ext.badges[TAB], "1");
  ext.listeners.headers({ tabId: TAB, url: "https://watch.example.org/other", type: "main_frame" });
  await settle(40);
  assert.equal(ext.badges[TAB], "");
  assert.deepEqual((await ask(ext, { type: "videos", tabId: TAB })).list, []);
  seen(ext, "https://cdn.example.net/v.m3u8", "xmlhttprequest");
  await settle(40);
  ext.listeners.removed(TAB);
  await settle(20);
  assert.deepEqual(ext.session, {});
});

test("Download with Pako sends the stream, its page, the title and that server's cookies", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/hls/master.m3u8", "xmlhttprequest", {}, { initiator: "https://player.example.com" });
  await settle(40);
  const reply = await ask(ext, { type: "send-stream", tabId: TAB, url: "https://cdn.example.net/hls/master.m3u8" });
  assert.equal(reply.ok, true);
  assert.equal(ext.sent.length, 1);
  const m = ext.sent[0];
  assert.equal(m.kind, "stream");
  assert.deepEqual(m.urls, ["https://cdn.example.net/hls/master.m3u8"]);
  // The embedded player's frame loaded it: that is the referrer the browser sent.
  assert.equal(m.referrer, "https://player.example.com");
  assert.equal(m.filename, `Page ${TAB}`);
  assert.deepEqual(
    m.siteCookies.map((c) => c.domain),
    ["cdn.example.net"],
  );
  // Only what was caught on that tab, and only from the toolbar window.
  assert.equal((await ask(ext, { type: "send-stream", tabId: TAB, url: "https://evil.example/x.m3u8" })).ok, false);
  assert.equal((await ask(ext, { type: "send-stream", tabId: TAB, url: "https://cdn.example.net/hls/master.m3u8" }, { tab: { id: TAB } })).ok, false);
  assert.equal(ext.sent.length, 1);
});

test("The popup hears whether a video plays and whether it is protected", async () => {
  const ext = full({ videos: [{ playing: true, locked: false }] });
  let r = await ask(ext, { type: "videos", tabId: TAB });
  assert.equal(r.playing, true);
  assert.equal(r.protected, false);
  const locked = full({ videos: [{ playing: true, locked: true }] });
  r = await ask(locked, { type: "videos", tabId: TAB });
  assert.equal(r.protected, true);
});

test("Pako Light has none of it: no catching, no list, no recording, no permission", async () => {
  const light = loadEdition({ light: true, tabs: { [TAB]: PAGE } });
  assert.equal(light.listeners.headers, undefined);
  assert.equal(light.listeners.removed, undefined);
  for (const type of ["videos", "send-stream", "record-start", "rec-hello"]) {
    const r = await ask(light, { type, tabId: TAB, url: "https://cdn.example.net/v.m3u8" }, type === "rec-hello" ? { tab: { id: TAB } } : {});
    assert.equal(r.ok, false, type);
    assert.equal(r.record, undefined, type);
  }
  await settle();
  assert.deepEqual(light.sent, []);
  for (const f of ["background.js", "popup.js", "popup.html", "popup.css"]) {
    const text = toLight(f, readFileSync(join(here, "..", f), "utf8"));
    assert.ok(!/webRequest|recorder|caught|send-stream|record-start/.test(text.replace(/\.caught[\w-]*/g, "")), f);
    assert.ok(!LIGHT_BANNED.test(text), f);
  }
});

test("A private window's playlist is never read again; a container's is read without any login", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/hls/master.m3u8", "xmlhttprequest", {}, { incognito: true });
  await settle(40);
  // Still listed (it goes to Pako only on a click, without cookies), but never asked for again.
  assert.equal((await ask(ext, { type: "videos", tabId: TAB })).list.length, 1);
  assert.deepEqual(ext.fetched, []);
  const box = full();
  seen(box, "https://cdn.example.net/hls/master.m3u8", "xmlhttprequest", {}, { cookieStoreId: "firefox-container-1" });
  await settle(40);
  assert.deepEqual(box.fetchedWith, [{ credentials: "omit", cache: "no-store" }]);
  const normal = full();
  seen(normal, "https://cdn.example.net/hls/master.m3u8", "xmlhttprequest", {}, { cookieStoreId: "firefox-default" });
  await settle(40);
  assert.deepEqual(normal.fetchedWith, [{ credentials: "include", cache: "force-cache" }]);
});

test("Record this video: the recorder goes from every site when the recorded tab closes or never plays", async () => {
  const ext = full();
  assert.equal((await ask(ext, { type: "record-start", tabId: TAB })).ok, true);
  assert.deepEqual([...ext.registered.keys()].sort(), ["pako-rec-bridge", "pako-rec-main"]);
  // Only the recorded tab is told to record.
  assert.deepEqual(await ask(ext, { type: "rec-hello" }, { tab: { id: TAB } }), { record: true });
  assert.deepEqual(await ask(ext, { type: "rec-hello" }, { tab: { id: TAB + 1 } }), { record: false });
  ext.listeners.removed(TAB);
  await settle(40);
  assert.equal(ext.registered.size, 0);
  assert.equal(ext.port.at(-1).record.action, "cancel");
});

test("Record this video: once the video plays, no other page gets the recorder", async () => {
  const ext = full();
  assert.equal((await ask(ext, { type: "record-start", tabId: TAB })).ok, true);
  const listeners = [];
  const page = {
    name: "pako-rec",
    sender: { tab: { id: TAB } },
    onMessage: { addListener: (f) => listeners.push(f) },
    onDisconnect: { addListener() {} },
    postMessage() {},
    disconnect() {},
  };
  ext.listeners.connect(page);
  listeners[0]({ t: "chunk", track: 0, mime: "video/mp4", data: "AQID", size: 3 });
  await settle(40);
  assert.equal(ext.registered.size, 0);
  assert.equal(ext.port.at(-1).record.action, "chunk");
});
