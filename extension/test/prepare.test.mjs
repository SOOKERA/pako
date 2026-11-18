// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The player's Download button reads ahead: once you click it and its quality menu opens (never
// on hover), Pako is asked to start reading the video ("videoPrepare", no quality), so the
// quality picked next starts at once. Only a Pako that says it reads ahead is asked; a private
// window's has no cookies; Pako Light never asks.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { ask, loadEdition, plain, settle } from "./fake-browser.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const VIDEO = "https://www.youtube.com/watch?v=abcdefghijk";

// ---------- the pill on a YouTube page, in a tiny fake page ----------

class El {
  constructor(tag) {
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.listeners = {};
    this.style = {};
    this.attrs = {};
    this.textContent = "";
    this.className = "";
    this.parentNode = null;
    const on = new Set();
    this.classList = {
      add: (c) => on.add(c),
      remove: (c) => on.delete(c),
      toggle: (c, force) => ((force ?? !on.has(c)) ? on.add(c) : on.delete(c)),
      contains: (c) => on.has(c),
    };
  }
  setAttribute(k, v) {
    this.attrs[k] = String(v);
  }
  removeAttribute(k) {
    delete this.attrs[k];
  }
  getAttribute(k) {
    return this.attrs[k] ?? null;
  }
  append(...nodes) {
    for (const n of nodes) {
      if (n && typeof n === "object") {
        n.parentNode = this;
        this.children.push(n);
      }
    }
  }
  replaceChildren(...nodes) {
    this.children = [];
    this.append(...nodes);
  }
  addEventListener(type, f) {
    (this.listeners[type] ||= []).push(f);
  }
  fire(type, extra = {}) {
    // Your own clicks unless said otherwise (a page's script makes untrusted ones).
    for (const f of this.listeners[type] || []) f({ isTrusted: true, stopPropagation() {}, composedPath: () => [], ...extra });
  }
  attachShadow() {
    this.shadow = new El("#shadow");
    return this.shadow;
  }
  /** Every element under this one (through its shadow root too). */
  *all() {
    for (const c of [...(this.shadow ? [this.shadow] : []), ...this.children]) {
      yield c;
      yield* c.all();
    }
  }
}

/** pill.js and player.js on a YouTube watch page with one big video; `sent` = the messages to
 *  the extension's background page. */
function youtubePage() {
  const sent = [];
  const docListeners = {};
  const video = new El("video");
  video.getBoundingClientRect = () => ({ left: 0, top: 0, right: 640, bottom: 360, width: 640, height: 360 });
  const root = new El("html");
  root.clientWidth = 1280;
  const document = {
    documentElement: root,
    fullscreenElement: null,
    createElement: (t) => new El(t),
    createElementNS: (_ns, t) => new El(t),
    getElementsByTagName: (t) => (t === "video" ? [video] : []),
    addEventListener: (t, f) => (docListeners[t] ||= []).push(f),
  };
  const chrome = {
    runtime: {
      sendMessage: async (m) => {
        sent.push(plain(m));
        return { ok: true };
      },
    },
    storage: { local: { get: async (d) => d, set: async () => {} }, onChanged: { addListener() {} } },
  };
  const context = {
    chrome,
    document,
    location: { href: VIDEO },
    URL,
    console,
    setTimeout,
    clearTimeout,
    requestAnimationFrame: (f) => setTimeout(f, 0),
    addEventListener() {},
    innerWidth: 1280,
  };
  context.globalThis = context;
  vm.createContext(context);
  for (const f of ["pill.js", "player.js"]) vm.runInContext(readFileSync(join(here, "..", f), "utf8"), context);
  const hover = async (x, y) => {
    for (const f of docListeners.mousemove || []) f({ clientX: x, clientY: y });
    await settle(5);
  };
  const pill = () => [...root.all()].find((e) => e.tagName === "BUTTON" && e.className === "main");
  const items = () => [...root.all()].filter((e) => e.tagName === "BUTTON" && e.className === "item");
  const host = () => [...root.all()].find((e) => e.tagName === "PAKO-DOWNLOAD-BUTTON");
  /** A click somewhere else on the page. */
  const elsewhere = async () => {
    for (const f of docListeners.pointerdown || []) f({ composedPath: () => [] });
    await settle(5);
  };
  return { sent, hover, pill, items, host, elsewhere };
}

test("Hovering the video shows the pill but asks nothing of Pako", async () => {
  const page = youtubePage();
  await page.hover(100, 100);
  await page.hover(120, 140);
  assert.ok(page.pill(), "the pill is there");
  await settle(10);
  assert.deepEqual(page.sent, [], "nothing read ahead on hover");
});

test("Clicking Download opens the menu and starts Pako's read at once; the pick follows", async () => {
  const page = youtubePage();
  await page.hover(100, 100);
  page.pill().fire("click");
  await settle(5);
  assert.deepEqual(page.sent, [{ type: "video-prepare", url: VIDEO }], "no quality: the read is the same for all");
  assert.equal(page.items().length, 4);
  // Closing the menu again without a pick: Pako stops that read.
  page.pill().fire("click");
  await settle(5);
  assert.deepEqual(page.sent.at(-1), { type: "video-prepare-cancel", url: VIDEO });
  // Opened again and a quality picked: the send as before, and no cancel (the pick uses it).
  page.pill().fire("click");
  page.items()[1].fire("click");
  await settle(5);
  assert.deepEqual(page.sent.at(-1), { type: "video", url: VIDEO, quality: 720 });
  assert.deepEqual(page.sent.map((m) => m.type), ["video-prepare", "video-prepare-cancel", "video-prepare", "video"]);
  // The pill hides later: nothing more.
  await page.elsewhere();
  assert.equal(page.sent.length, 4);
});

test("The menu closing without a pick cancels the read: Escape, a click elsewhere, the pill hidden", async () => {
  const page = youtubePage();
  await page.hover(100, 100);
  page.pill().fire("click");
  page.host().fire("keydown", { key: "Escape" });
  await settle(5);
  page.pill().fire("click");
  await page.elsewhere();
  // Once per menu: closed already, a second click elsewhere says nothing.
  await page.elsewhere();
  assert.deepEqual(page.sent.map((m) => m.type), ["video-prepare", "video-prepare-cancel", "video-prepare", "video-prepare-cancel"]);
  assert.ok(page.sent.every((m) => m.url === VIDEO));
});

test("A page's script can't open the menu or pick (only your own clicks count)", async () => {
  const page = youtubePage();
  await page.hover(100, 100);
  page.pill().fire("click", { isTrusted: false });
  await settle(5);
  assert.deepEqual(page.sent, [], "no read ahead");
  page.pill().fire("click");
  page.items()[1].fire("click", { isTrusted: false });
  await settle(5);
  assert.deepEqual(page.sent.map((m) => m.type), ["video-prepare"], "no pick");
});

// ---------- the background page ----------

const tab = (id, incognito = false) => ({ id, url: VIDEO, incognito });

test("The read ahead goes to Pako only once Pako said it reads ahead; with the site's login only", async () => {
  const ext = loadEdition({ tabs: { 1: VIDEO }, reply: () => ({ ok: true, prepares: true }) });
  // Not known yet (an older Pako never says it): nothing sent.
  assert.equal((await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) })).ok, true);
  await settle();
  assert.deepEqual(ext.sent, []);
  // Any answer from this Pako says it.
  await ask(ext, { type: "ping" });
  await settle();
  assert.equal(ext.storage.pakoPrepares, true);
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  await settle();
  assert.equal(ext.sent.length, 1);
  assert.equal(ext.sent[0].kind, "videoPrepare");
  assert.deepEqual(ext.sent[0].urls, [VIDEO]);
  assert.equal("quality" in ext.sent[0], false);
  assert.equal(ext.sent[0].siteCookies.length, 1, "the video site's own login, as for the send");
  assert.equal("private" in ext.sent[0], false);
});

test("A read ahead from a private window: private, no cookies, no cookie store read", async () => {
  const ext = loadEdition({ tabs: { 3: VIDEO }, privateTabs: [3], privateAllowed: true, stored: { pakoPrepares: true } });
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(3, true) });
  await settle();
  assert.equal(ext.sent.length, 1);
  assert.equal(ext.sent[0].private, true);
  assert.deepEqual(ext.sent[0].siteCookies, []);
  assert.deepEqual(ext.cookieStores, []);
});

test("Never another site, never from outside a page, and a failed read ahead says nothing", async () => {
  const ext = loadEdition({ tabs: { 1: VIDEO }, stored: { pakoPrepares: true }, reply: (m) => (m.kind === "videoPrepare" ? { ok: false, error: "Unknown request.", prepares: true } : { ok: true, prepares: true }) });
  await ask(ext, { type: "video-prepare", url: "https://shop.example.co.uk/x" }, { tab: tab(1) });
  await ask(ext, { type: "video-prepare", url: VIDEO });
  await settle();
  assert.deepEqual(ext.sent, []);
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  await settle();
  assert.equal(ext.sent.length, 1);
  assert.equal(ext.storage.lastError, undefined, "no problem flagged");
  assert.equal(ext.storage.badge, undefined);
});

test("Pako Light never reads ahead, even when asked", async () => {
  const ext = loadEdition({ light: true, tabs: { 1: VIDEO }, stored: { pakoPrepares: true } });
  const reply = await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  assert.equal(reply.ok, false);
  await settle();
  assert.deepEqual(ext.sent, []);
});

// ---------- stopping a read ahead ----------

/** What was asked of Pako to stop. (Each Pako below says it reads ahead in its answers.) */
const cancels = (ext) => ext.sent.filter((m) => m.kind === "videoPrepareCancel");

test("The menu closed without a pick: Pako is told to stop that read, after the read itself", async () => {
  const ext = loadEdition({ tabs: { 1: VIDEO, 3: VIDEO }, privateTabs: [3], privateAllowed: true, stored: { pakoPrepares: true }, reply: () => ({ ok: true, prepares: true }) });
  // Closed at once, before the read ahead was even sent: neither goes to Pako.
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  assert.equal((await ask(ext, { type: "video-prepare-cancel", url: VIDEO }, { tab: tab(1) })).ok, true);
  await settle();
  assert.deepEqual(ext.sent, []);
  // Closed a moment later: the cancel follows the read ahead.
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  await settle();
  await ask(ext, { type: "video-prepare-cancel", url: VIDEO }, { tab: tab(1) });
  await settle();
  assert.deepEqual(ext.sent.map((m) => m.kind), ["videoPrepare", "videoPrepareCancel"]);
  assert.deepEqual(cancels(ext)[0].urls, [VIDEO]);
  assert.equal("siteCookies" in cancels(ext)[0], false, "no cookies with a cancel");
  assert.equal("private" in cancels(ext)[0], false);
  // Closed twice: told once.
  await ask(ext, { type: "video-prepare-cancel", url: VIDEO }, { tab: tab(1) });
  await settle();
  assert.equal(cancels(ext).length, 1);
  // A private window's: private.
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(3, true) });
  await settle();
  await ask(ext, { type: "video-prepare-cancel", url: VIDEO }, { tab: tab(3, true) });
  await settle();
  assert.equal(cancels(ext).length, 2);
  assert.equal(cancels(ext)[1].private, true);
});

test("A pick is never cancelled: not by the menu closing for it, nor by its tab closing later", async () => {
  // Picked before the read ahead was even sent: only the pick goes (it reads on its own).
  const quick = loadEdition({ tabs: { 1: VIDEO }, stored: { pakoPrepares: true }, reply: () => ({ ok: true, prepares: true }) });
  await ask(quick, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  await ask(quick, { type: "video", url: VIDEO, quality: 720 }, { tab: tab(1) });
  await settle();
  assert.deepEqual(quick.sent.map((m) => m.kind), ["video"]);
  const ext = loadEdition({ tabs: { 1: VIDEO }, stored: { pakoPrepares: true }, reply: () => ({ ok: true, prepares: true }) });
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  await settle();
  await ask(ext, { type: "video", url: VIDEO, quality: 720 }, { tab: tab(1) });
  await ask(ext, { type: "video-prepare-cancel", url: VIDEO }, { tab: tab(1) });
  ext.listeners.removed(1);
  await settle();
  assert.deepEqual(ext.sent.map((m) => m.kind), ["videoPrepare", "video"]);
});

test("A tab closed with its menu open: the read stops; other tabs' reads carry on", async () => {
  const ext = loadEdition({ tabs: { 1: VIDEO, 2: VIDEO }, stored: { pakoPrepares: true }, reply: () => ({ ok: true, prepares: true }) });
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(2) });
  await settle();
  ext.listeners.removed(1);
  await settle();
  assert.equal(cancels(ext).length, 1);
  ext.listeners.removed(1);
  await settle();
  assert.equal(cancels(ext).length, 1, "once");
});

test("No cancel to a Pako that never read ahead, nor from Pako Light", async () => {
  const ext = loadEdition({ tabs: { 1: VIDEO } });
  await ask(ext, { type: "video-prepare", url: VIDEO }, { tab: tab(1) });
  await ask(ext, { type: "video-prepare-cancel", url: VIDEO }, { tab: tab(1) });
  ext.listeners.removed(1);
  await settle();
  assert.deepEqual(ext.sent, []);
  const light = loadEdition({ light: true, tabs: { 1: VIDEO }, stored: { pakoPrepares: true }, reply: () => ({ ok: true, prepares: true }) });
  assert.equal((await ask(light, { type: "video-prepare-cancel", url: VIDEO }, { tab: tab(1) })).ok, false);
  await settle();
  assert.deepEqual(light.sent, []);
});
