// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The Download pill on a page's video (full extension only): it is put into a page only after a
// video was caught there, once per frame, into that frame (else the top frame); its menu offers
// what the popup offers; Pako Light has none of it.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { ask, loadEdition, settle } from "./fake-browser.mjs";
import { toLight } from "../light.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const TAB = 6;
const PAGE = "https://watch.example.org/v/7";
const FILES = ["pill.js", "caught.js"];

const MASTER = [
  "#EXTM3U",
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

/** A request the tab's frame `frameId` made (`documentId`: that frame's page, as Chromium names it). */
const seen = (ext, url, type, { headers = {}, ...extra } = {}) =>
  ext.listeners.headers({
    tabId: TAB,
    url,
    type,
    statusCode: 200,
    responseHeaders: Object.entries(headers).map(([name, value]) => ({ name, value })),
    ...extra,
  });

const fromTab = { tab: { id: TAB } };
const texts = (items) => items.filter((i) => i.text).map((i) => i.text);

test("No video caught, no pill: the page is never touched", async () => {
  const ext = full();
  seen(ext, PAGE, "main_frame", { frameId: 0 });
  seen(ext, "https://cdn.example.net/page.html", "xmlhttprequest", { frameId: 0, headers: { "content-type": "text/html" } });
  seen(ext, "https://ads.example.net/tiny.mp4", "media", { frameId: 0, headers: { "content-type": "video/mp4", "content-length": "40000" } });
  await settle(60);
  assert.deepEqual(ext.injected, []);
});

test("A caught video puts the pill into the frame that loaded it, once per frame", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/hls/master.m3u8", "xmlhttprequest", { frameId: 3, documentId: "doc-a" });
  await settle(60);
  assert.deepEqual(ext.injected, [{ tabId: TAB, frameIds: [3], files: FILES }]);
  // Its own playlists, another video from the same frame, two requests at once: not again.
  seen(ext, "https://cdn.example.net/hls/720/index.m3u8", "xmlhttprequest", { frameId: 3, documentId: "doc-a" });
  seen(ext, "https://cdn.example.net/other.mpd", "xmlhttprequest", { frameId: 3, documentId: "doc-a" });
  seen(ext, "https://cdn.example.net/more.mpd", "xmlhttprequest", { frameId: 3, documentId: "doc-a" });
  await settle(60);
  assert.equal(ext.injected.length, 1);
  // Another frame of the tab that loads a video gets its own.
  seen(ext, "https://files.example.net/clip.mp4", "media", { frameId: 0, documentId: "doc-top", headers: { "content-type": "video/mp4", "content-length": "5000000" } });
  await settle(60);
  assert.deepEqual(ext.injected.map((i) => i.frameIds), [[3], [0]]);
});

test("The background page waking up again doesn't inject twice", async () => {
  const first = full();
  seen(first, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: 2, documentId: "doc-a" });
  await settle(60);
  assert.equal(first.injected.length, 1);
  // A fresh background page with the same memory-only storage.
  const again = full();
  Object.assign(again.session, JSON.parse(JSON.stringify(first.session)));
  seen(again, "https://cdn.example.net/b.m3u8", "xmlhttprequest", { frameId: 2, documentId: "doc-a" });
  await settle(60);
  assert.deepEqual(again.injected, []);
});

test("No frame known, or a frame closed to it: the top frame", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: -1 });
  await settle(60);
  assert.deepEqual(ext.injected.map((i) => i.frameIds), [[0]]);
  const closed = full({ closedFrames: [5] });
  seen(closed, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: 5, documentUrl: "https://player.example.com/e/1" });
  await settle(60);
  assert.deepEqual(closed.injected.map((i) => i.frameIds), [[0]]);
});

test("A new page in the tab gets the pill again once a video is caught on it", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: 0 });
  await settle(60);
  seen(ext, "https://watch.example.org/v/8", "main_frame", { frameId: 0 });
  await settle(60);
  assert.equal(ext.injected.length, 1);
  seen(ext, "https://cdn.example.net/b.m3u8", "xmlhttprequest", { frameId: 0 });
  await settle(60);
  assert.equal(ext.injected.length, 2);
  // The tab closing forgets it too.
  ext.listeners.removed(TAB);
  await settle(30);
  assert.deepEqual(ext.session, {});
});

test("With Download button on videos turned off, nothing is put into the page", async () => {
  const ext = full();
  ext.storage.playerButton = false;
  seen(ext, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: 0 });
  await settle(60);
  assert.deepEqual(ext.injected, []);
});

test("Pako Light has no pill: nothing caught, nothing put into a page, no menu", async () => {
  const light = loadEdition({ light: true, tabs: { [TAB]: PAGE } });
  assert.equal(light.listeners.headers, undefined);
  for (const type of ["pill-menu", "pill-send", "pill-record"]) {
    const r = await ask(light, { type, url: "https://cdn.example.net/a.m3u8" }, fromTab);
    assert.equal(r.ok, false, type);
    assert.equal(r.items, undefined, type);
  }
  await settle();
  assert.deepEqual(light.injected, []);
  assert.deepEqual(light.sent, []);
  const text = toLight("background.js", readFileSync(join(here, "..", "background.js"), "utf8"));
  assert.ok(!/pill|caught\.js|frameIds/.test(text));
});

test("Menu: one caught video downloads at once, with Record this video under it", async () => {
  const ext = full({ videos: [{ playing: true, locked: false }] });
  seen(ext, "https://cdn.example.net/hls/master.m3u8", "xmlhttprequest", { frameId: 0 });
  await settle(60);
  const { ok, items } = await ask(ext, { type: "pill-menu" }, fromTab);
  assert.equal(ok, true);
  assert.deepEqual(texts(items), ["Download this video", "Record this video"]);
  assert.equal(items[1].side, "up to 1080p");
  // Download goes the popup's way: the stream, its page and title, that server's cookies.
  const reply = await ask(ext, items[1].send, fromTab);
  assert.equal(reply.ok, true);
  assert.equal(ext.sent.length, 1);
  assert.equal(ext.sent[0].kind, "stream");
  assert.deepEqual(ext.sent[0].urls, ["https://cdn.example.net/hls/master.m3u8"]);
});

test("Menu: several caught videos are listed like in the popup", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/hls/master.m3u8", "xmlhttprequest", { frameId: 0 });
  seen(ext, "https://cdn.example.net/dash/stream.mpd", "xmlhttprequest", { frameId: 0 });
  seen(ext, "https://files.example.net/clip.mp4", "media", { frameId: 0, headers: { "content-type": "video/mp4", "content-length": "5000000" } });
  await settle(80);
  const { items } = await ask(ext, { type: "pill-menu" }, fromTab);
  assert.deepEqual(texts(items), ["HLS stream · up to 1080p", "DASH stream", "Video file", "Record this video"]);
  assert.deepEqual(
    items.filter((i) => i.send?.type === "pill-send").map((i) => i.side),
    ["cdn.example.net", "cdn.example.net", "files.example.net"],
  );
});

test("Menu: a protected video offers nothing, not even Record this video", async () => {
  const ext = full({ videos: [{ playing: true, locked: true }] });
  seen(ext, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: 0 });
  await settle(60);
  const { items } = await ask(ext, { type: "pill-menu" }, fromTab);
  assert.deepEqual(texts(items), []);
  assert.ok(items.some((i) => i.info === "This video is protected, so it can't be downloaded."));
  // Asked anyway: refused, nothing recorded.
  const r = await ask(ext, { type: "pill-record" }, fromTab);
  assert.equal(r.ok, false);
  assert.equal(ext.registered.size, 0);
});

test("A video that becomes protected after the menu opened isn't sent (pill and popup)", async () => {
  const videos = [{ playing: true, locked: false }];
  const ext = full({ videos });
  seen(ext, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: 0 });
  await settle(60);
  const { items } = await ask(ext, { type: "pill-menu" }, fromTab);
  const download = items.find((i) => i.send?.type === "pill-send");
  assert.ok(download);
  // The page asks for a key system (its real programme starts) before the click.
  videos.push({ playing: true, locked: true });
  const r = await ask(ext, download.send, fromTab);
  assert.equal(r.ok, false);
  assert.equal(r.error, "This video is protected, so it can't be downloaded.");
  const popup = await ask(ext, { type: "send-stream", tabId: TAB, url: download.send.url });
  assert.equal(popup.ok, false);
  assert.deepEqual(ext.sent, []);
});

test("Menu: Record this video follows the popup's rule", async () => {
  // Nothing listed and nothing playing: no Record this video.
  const idle = full();
  let { items } = await ask(idle, { type: "pill-menu" }, fromTab);
  assert.deepEqual(texts(items), []);
  assert.equal((await ask(idle, { type: "pill-record" }, fromTab)).ok, false);
  // Nothing listed but a video plays: offered.
  const playing = full({ videos: [{ playing: true, locked: false }] });
  ({ items } = await ask(playing, { type: "pill-menu" }, fromTab));
  assert.deepEqual(texts(items), ["Record this video"]);
  // While recording this tab: not offered again.
  seen(playing, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: 0 });
  await settle(60);
  assert.equal((await ask(playing, { type: "pill-record" }, fromTab)).ok, true);
  ({ items } = await ask(playing, { type: "pill-menu" }, fromTab));
  assert.deepEqual(texts(items), ["Download this video"]);
  // (The tab closes: the recording and its 5-minute wait end.)
  playing.listeners.removed(TAB);
  await settle(30);
});

test("The pill only ever asks about its own tab, and only from a page", async () => {
  const ext = full();
  seen(ext, "https://cdn.example.net/a.m3u8", "xmlhttprequest", { frameId: 0 });
  await settle(60);
  // Another tab can't download this tab's video, nor anything that wasn't caught.
  assert.equal((await ask(ext, { type: "pill-send", url: "https://cdn.example.net/a.m3u8" }, { tab: { id: TAB + 1 } })).ok, false);
  assert.equal((await ask(ext, { type: "pill-send", url: "https://evil.example/x.m3u8" }, fromTab)).ok, false);
  // Not a page's message: the toolbar window has its own.
  assert.equal((await ask(ext, { type: "pill-menu" })).ok, false);
  assert.deepEqual(ext.sent, []);
});

test("Installed or updated: the extension says hello to Pako at once (Settings shows it Installed)", async () => {
  for (const make of [full, (o) => loadEdition({ light: true, ...o })]) {
    const ext = await make();
    await ext.listeners.installed?.({ reason: "install" });
    await settle();
    assert.equal(ext.pings.length, 1, `${ext.who}: one hello`);
    assert.deepEqual(ext.sent, [], `${ext.who}: nothing else asked of Pako`);
  }
});

test("A hello that finds a newer extension in Pako's folder reloads once, never in a loop", async () => {
  // Pako's folder says 2.0.0; this copy is 1.0.0 (loaded from another folder, so the reload
  // brings 1.0.0 back): install → hello → reload → installed again → hello → no second reload.
  const reply = () => ({ ok: true, ext: "2.0.0" });
  const first = loadEdition({ reply });
  await first.listeners.installed?.({ reason: "install" });
  await settle(3200);
  assert.equal(first.reloads.length, 1, "reloaded once for 2.0.0");
  assert.equal(first.storage.reloadedFor, "2.0.0");
  // The reloaded copy: same storage, fresh page.
  const again = loadEdition({ reply });
  Object.assign(again.storage, first.storage);
  await again.listeners.installed?.({ reason: "update" });
  await settle(3200);
  assert.equal(again.pings.length, 1, "it still says hello");
  assert.equal(again.reloads.length, 0, "but doesn't reload again");
});
