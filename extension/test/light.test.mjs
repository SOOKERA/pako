// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako Light sends nothing from video sites and no links to them, and offers nothing for
// <video>/<audio> (audit 2026-11-16 evening, H1 and L2). Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { loadEdition, plain, settle } from "./fake-browser.mjs";

const VIDEO_SITE = /(^|\.)(youtube\.com|youtu\.be|vimeo\.com|twitch\.tv)$/;
const isVideoLink = (u) => VIDEO_SITE.test(new URL(u).hostname.replace(/^www\./, ""));
const allUrls = (sent) => sent.flatMap((m) => m.urls ?? []);

const PAGE_LINKS = [
  "https://www.youtube.com/watch?v=aaaaaaaaaaa",
  "https://youtu.be/bbbbbbbbbbb",
  "https://m.youtube.com/shorts/ccccccccccc",
  "https://vimeo.com/123456",
  "https://files.example.com/setup.zip",
  "https://news.example/report.pdf",
];

async function light(opts = {}) {
  const ext = loadEdition({ light: true, ...opts });
  await ext.listeners.installed();
  await settle();
  return ext;
}

test("Light's media item is for images only; the full extension's covers video and audio", async () => {
  const ext = await light();
  assert.deepEqual(ext.menus.get("pako-media").contexts, ["image"]);
  assert.deepEqual([...ext.menus.keys()].sort(), ["pako-all", "pako-link", "pako-media", "pako-page"]);
  const full = loadEdition({ light: false });
  await full.listeners.installed();
  await settle();
  assert.deepEqual(full.menus.get("pako-media").contexts, ["image", "video", "audio"]);
});

test("Send all links on a video site: nothing is read or sent", async () => {
  const ext = await light({ tabs: { 1: "https://www.youtube.com/" }, links: PAGE_LINKS });
  await ext.listeners.click({ menuItemId: "pako-all", pageUrl: "https://www.youtube.com/" }, { id: 1, url: "https://www.youtube.com/" });
  await settle();
  assert.deepEqual(ext.sent, []);
  assert.equal(ext.storage.lastError, "Pako Light doesn't work on this site.");
});

test("Send all links on another page: video-site links are left out", async () => {
  const ext = await light({ tabs: { 1: "https://news.example/" }, links: PAGE_LINKS });
  await ext.listeners.click({ menuItemId: "pako-all", pageUrl: "https://news.example/" }, { id: 1, url: "https://news.example/" });
  await settle();
  const links = ext.sent.filter((m) => m.kind === "links");
  assert.equal(links.length, 1);
  assert.deepEqual(links[0].urls, ["https://files.example.com/setup.zip", "https://news.example/report.pdf"]);
  assert.ok(!allUrls(ext.sent).some(isVideoLink));
});

test("A page of video links only: nothing is sent", async () => {
  const ext = await light({ tabs: { 1: "https://blog.example/" }, links: PAGE_LINKS.slice(0, 4) });
  await ext.listeners.click({ menuItemId: "pako-all", pageUrl: "https://blog.example/" }, { id: 1, url: "https://blog.example/" });
  await settle();
  assert.deepEqual(ext.sent, []);
});

test("This page, a video link, or a picture on a video site: nothing is sent", async () => {
  const ext = await light({ tabs: { 1: "https://www.youtube.com/watch?v=aaaaaaaaaaa" } });
  const tab = { id: 1, url: "https://www.youtube.com/watch?v=aaaaaaaaaaa" };
  await ext.listeners.click({ menuItemId: "pako-page" }, tab);
  await ext.listeners.click({ menuItemId: "pako-link", linkUrl: "https://www.youtube.com/watch?v=bbbbbbbbbbb", pageUrl: "https://news.example/" }, { id: 2, url: "https://news.example/" });
  await ext.listeners.click({ menuItemId: "pako-media", srcUrl: "https://i.ytimg.com/vi/x/hq.jpg", pageUrl: tab.url }, tab);
  await settle();
  assert.deepEqual(ext.sent, []);
  // The popup's Send button: refused too.
  const reply = await new Promise((r) => ext.listeners.message({ type: "send-tab", url: tab.url }, {}, (x) => r(plain(x))));
  assert.equal(reply.ok, false);
  assert.deepEqual(ext.sent, []);
});

test("A picture on another site still goes to Pako", async () => {
  const ext = await light();
  await ext.listeners.click({ menuItemId: "pako-media", srcUrl: "https://cdn.example.net/photo.jpg", pageUrl: "https://news.example/" }, { id: 1, url: "https://news.example/" });
  await settle();
  assert.equal(ext.sent.length, 1);
  assert.deepEqual(ext.sent[0].urls, ["https://cdn.example.net/photo.jpg"]);
  assert.equal(ext.sent[0].siteCookies, undefined);
});

test("On a video site both page items are greyed out, and back on elsewhere", async () => {
  const ext = await light({ tabs: { 1: "https://www.youtube.com/", 2: "https://news.example/" } });
  await ext.listeners.activated({ tabId: 1 });
  await settle();
  assert.equal(ext.menus.get("pako-page").enabled, false);
  assert.equal(ext.menus.get("pako-all").enabled, false);
  await ext.listeners.activated({ tabId: 2 });
  await settle();
  assert.equal(ext.menus.get("pako-page").enabled, true);
  assert.equal(ext.menus.get("pako-all").enabled, true);
});

test("Switching windows updates the page items (L2)", async () => {
  const ext = await light({ tabs: { 1: "https://www.youtube.com/" } });
  ext.listeners.focus(7);
  await settle();
  assert.equal(ext.menus.get("pako-all").enabled, false);
  ext.updates.length = 0;
  ext.listeners.focus(-1); // no browser window in front: nothing to do
  await settle();
  assert.deepEqual(ext.updates, []);
  // The full extension retitles its page item the same way.
  const full = loadEdition({ tabs: { 1: "https://www.youtube.com/" } });
  await full.listeners.installed();
  full.listeners.focus(3);
  await settle();
  assert.equal(full.menus.get("pako-page").title, "Download video with Pako");
});

test("Browser downloads from a video site stay in the browser", async () => {
  const ext = await light();
  await ext.listeners.download({ id: 5, url: "https://rr1.googlevideo.example/file.zip", referrer: "https://www.youtube.com/watch?v=x" });
  await ext.listeners.download({ id: 6, url: "https://www.youtube.com/some.zip", referrer: "" });
  await settle();
  assert.deepEqual(ext.sent, []);
  assert.deepEqual(ext.log, []); // not even paused
  await ext.listeners.download({ id: 7, url: "https://files.example.com/big.zip", referrer: "https://files.example.com/" });
  await settle();
  assert.equal(ext.sent.length, 1);
  assert.equal(ext.sent[0].takeover, true);
});

test("Light never sends a video request, even when asked", async () => {
  const ext = await light();
  const reply = await new Promise((r) => ext.listeners.message({ type: "video", url: "https://www.youtube.com/watch?v=x", quality: 720 }, { tab: { id: 1 } }, r));
  assert.equal(reply.ok, false);
  assert.deepEqual(ext.sent, []);
});
