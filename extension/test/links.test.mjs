// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Download links on pages (browsers that ask first, Vivaldi): a page's `<a download href="…">`
// starts its download without loading anything, so the catch rule never sees it and Vivaldi asks
// first. While the catch is on and "Catch download links on pages" is on, the full Chromium
// extension puts download-links.js in web pages: a real click on such a link (the page's own
// site, http or https, the page not handling it itself) goes to Pako like a caught download; if
// Pako can't take it, the browser downloads it once. Never in a private window, Pako Light or
// Firefox; never a link built in the page (blob:, data:) or to another site.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { ask, loadEdition, settle } from "./fake-browser.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const LINKS_JS = readFileSync(join(here, "..", "download-links.js"), "utf8");

const TAB = 3;
const PRIV = 4;
const PAGE = "https://files.example.com/downloads";
const FILE = "https://files.example.com/get/big.zip";

/** A Pako that answers at once ("held"), then the check's `result`. */
function newPako({ result = { ok: true } } = {}) {
  const said = [];
  const reply = (m) => {
    said.push(m.kind + (m.hold ? "+hold" : ""));
    if (m.kind === "held") return { ...result, holds: true };
    return m.hold ? { ok: true, held: "tok-1", holds: true } : { ok: true, holds: true };
  };
  return { said, reply };
}

/** Vivaldi with Pako's helper answering (it said it holds takeovers). */
async function vivaldi(opts = {}, pako = newPako()) {
  const ext = loadEdition({
    dnr: "full",
    vivaldi: true,
    naming: true,
    tabs: { [TAB]: PAGE, [PRIV]: PAGE },
    privateTabs: [PRIV],
    privateAllowed: true,
    reply: pako.reply,
    stored: { pakoHolds: true },
    ...opts,
  });
  await settle(30);
  return ext;
}

class FakeAnchor {
  constructor(attrs = {}, page) {
    this.attrs = { ...attrs };
    this.page = page;
    this.referrerPolicy = attrs.referrerpolicy || "";
  }
  get href() {
    try {
      return new URL(this.attrs.href ?? "", this.page.location.href).href;
    } catch {
      return this.attrs.href ?? "";
    }
  }
  set href(v) {
    this.attrs.href = v;
  }
  hasAttribute(n) {
    return n in this.attrs;
  }
  getAttribute(n) {
    return n in this.attrs ? this.attrs[n] : null;
  }
  setAttribute(n, v) {
    this.attrs[n] = String(v);
  }
  // A link the script made (not in the page): the browser downloads it as usual.
  click() {
    this.page.browserClicks.push({ href: this.href, download: this.getAttribute("download"), rel: this.getAttribute("rel") });
  }
}
class FakeArea extends FakeAnchor {}

/**
 * A web page with download-links.js in it (as the browser puts it in), talking to `ext`'s
 * background. `incognito`: a private window. `frame`: inside a frame. `extensionGone`: the
 * extension was reloaded since (no one to talk to).
 */
function openPage(ext, { url = PAGE, tabId = TAB, incognito = false, frame = false, extensionGone = false, backgroundFails = false } = {}) {
  const listeners = { capture: {}, bubble: {} };
  const page = { location: { href: url, origin: new URL(url).origin }, browserClicks: [], messages: [] };
  const win = {
    addEventListener(type, f, capture) {
      const list = (listeners[capture ? "capture" : "bubble"][type] ||= []);
      if (!list.includes(f)) list.push(f);
    },
    removeEventListener(type, f, capture) {
      const list = listeners[capture ? "capture" : "bubble"][type] || [];
      const i = list.indexOf(f);
      if (i >= 0) list.splice(i, 1);
    },
  };
  win.top = frame ? {} : win;
  const chrome = {
    runtime: {
      id: extensionGone ? undefined : ext.id,
      sendMessage: async (msg) => {
        page.messages.push(msg);
        if (backgroundFails) throw new Error("Could not establish connection. Receiving end does not exist.");
        return ask(ext, msg, { id: ext.id, tab: { id: tabId, incognito, url }, frameId: frame ? 1 : 0, url });
      },
    },
    extension: { inIncognitoContext: incognito },
  };
  const context = {
    chrome,
    window: win,
    location: page.location,
    document: { createElement: () => new FakeAnchor({}, page) },
    HTMLAnchorElement: FakeAnchor,
    HTMLAreaElement: FakeArea,
    URL,
  };
  context.globalThis = context;
  win.window = win;
  vm.runInContext(LINKS_JS, vm.createContext(context), { filename: "download-links.js" });
  page.listeners = listeners;
  /** A click on `attrs` (an <a>, or `area`), as the browser dispatches it: the window's capture
   *  listeners, the page's own handler (`pageHandles`: it calls preventDefault; `pageDoes(a)`: it
   *  changes the link), then the window's listeners at the end. Returns whether the browser's own download was stopped. */
  page.click = (attrs, { trusted = true, button = 0, type = button === 1 ? "auxclick" : "click", ctrlKey = false, area = false, pageHandles = false, pageDoes = null } = {}) => {
    const a = new (area ? FakeArea : FakeAnchor)(attrs, page);
    const e = {
      type,
      isTrusted: trusted,
      button,
      ctrlKey,
      shiftKey: false,
      altKey: false,
      metaKey: false,
      target: a,
      defaultPrevented: false,
      preventDefault() {
        this.defaultPrevented = true;
      },
      composedPath: () => [{ tag: "span" }, a, { tag: "body" }],
    };
    for (const f of [...(listeners.capture[type] || [])]) f(e);
    if (pageHandles) e.preventDefault();
    pageDoes?.(a);
    for (const f of [...(listeners.bubble[type] || [])]) f(e);
    return e.defaultPrevented;
  };
  return page;
}

test("Vivaldi: the script is in web pages while the catch is on; a download link clicked goes to Pako, the browser downloads nothing", async () => {
  const pako = newPako();
  const ext = await vivaldi({}, pako);
  const s = ext.registered.get("pako-links");
  assert.ok(s, "the script is registered");
  assert.deepEqual(s.js, ["download-links.js"]);
  assert.deepEqual(s.matches, ["http://*/*", "https://*/*"]);
  assert.equal(s.allFrames, false, "top frame only");
  assert.equal(s.runAt, "document_start");
  assert.equal(s.persistAcrossSessions, false);
  assert.ok(s.excludeMatches.includes("*://*.youtube.com/*"), "never on the video sites");

  const page = openPage(ext);
  assert.equal(page.click({ href: "/get/big.zip#part", download: "My file.zip" }), true, "the browser's download is stopped");
  await settle(60);
  assert.deepEqual(pako.said, ["download+hold", "held"]);
  const m = ext.sent[0];
  assert.deepEqual(m.urls, [FILE], "the link, without its #");
  assert.equal(m.takeover, true);
  assert.equal(m.hold, true);
  assert.equal(m.filename, "My file.zip", "the link's own file name");
  assert.equal(m.referrer, PAGE, "the page it was clicked on");
  assert.equal(m.private, undefined);
  assert.ok(m.cookies.includes("SID=x"), "the site's cookies, like a takeover");
  assert.deepEqual(page.browserClicks, [], "nothing for the browser");
  assert.deepEqual(ext.restarted, []);
  assert.deepEqual(ext.log, []);
  // The script tells the extension the clicked link only, nothing else of the page.
  assert.deepEqual(Object.keys(page.messages[0]).sort(), ["filename", "noReferrer", "type", "url"]);

  // An <area download>, the middle button, a link without a file name, "noreferrer".
  assert.equal(page.click({ href: "/get/a.zip", download: "" }, { area: true }), true);
  assert.equal(page.click({ href: "/get/b.zip", download: "" }, { button: 1 }), true);
  assert.equal(page.click({ href: "/get/c.zip", download: "", rel: "nofollow noreferrer" }), true);
  await settle(60);
  const asked = ext.sent.filter((x) => x.kind === "download");
  assert.deepEqual(asked.map((x) => x.urls[0]), [FILE, "https://files.example.com/get/a.zip", "https://files.example.com/get/b.zip", "https://files.example.com/get/c.zip"]);
  assert.equal(asked[2].filename, "", "no name: Pako takes the server's");
  assert.equal(asked[3].referrer, "", "noreferrer: no page named");
});

test("Left to the browser: an untrusted click, blob:/data:/javascript:, another site, this very page, a plain link, a modifier key, a page that handles the click itself", async () => {
  const ext = await vivaldi();
  const page = openPage(ext);
  const cases = [
    [{ href: "/get/big.zip", download: "" }, { trusted: false }],
    [{ href: "blob:https://files.example.com/1b2c", download: "x.zip" }, {}],
    [{ href: "data:application/zip;base64,UEsDBA==", download: "x.zip" }, {}],
    [{ href: "javascript:void(0)", download: "" }, {}],
    [{ href: "https://cdn.other.example/big.zip", download: "" }, {}],
    [{ href: "#top", download: "" }, {}],
    [{ href: PAGE, download: "" }, {}],
    [{ href: "/get/big.zip" }, {}],
    [{ href: "/get/big.zip", download: "" }, { ctrlKey: true }],
    [{ href: "/get/big.zip", download: "" }, { button: 2, type: "auxclick" }],
  ];
  for (const [attrs, how] of cases) assert.equal(page.click(attrs, how), false, JSON.stringify([attrs, how]));
  // The page handles the click itself (its own preventDefault): it keeps it.
  assert.equal(page.click({ href: "/get/big.zip", download: "" }, { pageHandles: true }), true);
  await settle(60);
  assert.deepEqual(page.messages, [], "nothing told");
  assert.deepEqual(ext.sent, [], "nothing to Pako");
  assert.deepEqual(page.browserClicks, []);
  // No listener is left behind by those clicks.
  assert.equal(page.listeners.bubble.click?.length ?? 0, 0);
});

test("Pako can't take it: the browser downloads it once (marked, so it isn't taken again), and the \"!\" says why", async () => {
  const why = "Pako couldn't get this download (the server gave Pako a web page instead of the file), so your browser downloads it.";
  const ext = await vivaldi({}, newPako({ result: { ok: false, error: why, followed: "row-1" } }));
  const page = openPage(ext);
  page.click({ href: "/get/big.zip", download: "big.zip" });
  await settle(80);
  assert.equal(ext.restarted.length, 1, "started again in the browser once");
  assert.equal(ext.restarted[0].url, FILE);
  assert.equal(ext.restarted[0].filename, "big.zip");
  assert.ok(ext.sent.some((m) => m.kind === "follow" && m.follow.pako === "row-1"), "Pako follows the browser's download");
  // The browser's own download of it is never taken over again (no loop).
  const before = ext.sent.length;
  await ext.listeners.naming({ id: 99, url: FILE, byExtensionId: ext.id, totalBytes: 50e6 }, () => {});
  await settle(30);
  assert.equal(ext.sent.length, before);
  assert.deepEqual(page.browserClicks, [], "and the page doesn't start it a second time");

  // Pako not there at all: the same, with the "!".
  const away = await vivaldi({ reply: (m) => (m.kind === "ping" ? { ok: true, holds: true } : undefined) });
  const p2 = openPage(away);
  p2.click({ href: "/get/big.zip", download: "" });
  await settle(80);
  assert.equal(away.restarted.length, 1);
  assert.equal(away.storage.badge, "!");
  assert.equal(away.registered.get("pako-links"), undefined, "Pako away: the script goes until it answers");
});

test("A double click hands the link once", async () => {
  const pako = newPako();
  const ext = await vivaldi({}, pako);
  const page = openPage(ext);
  page.click({ href: "/get/big.zip", download: "" });
  page.click({ href: "/get/big.zip", download: "" });
  await settle(80);
  assert.deepEqual(pako.said, ["download+hold", "held"]);
  assert.deepEqual(page.browserClicks, []);
});

test("The setting and the takeover switch: off, the script goes; a page opened before lets the browser download as usual", async () => {
  const ext = await vivaldi();
  assert.ok(ext.registered.get("pako-links"));
  const page = openPage(ext);
  await ext.chrome.storage.local.set({ catchLinks: false });
  await settle(30);
  assert.equal(ext.registered.get("pako-links"), undefined, "setting off: nothing registered");
  assert.ok(ext.rules.get(1), "the rule at the server's answer stays");
  // The page still has its script until it reloads: the extension says no, the browser downloads it.
  assert.equal(page.click({ href: "/get/big.zip", download: "big.zip" }), true);
  await settle(60);
  assert.deepEqual(ext.sent, []);
  assert.deepEqual(page.browserClicks, [{ href: FILE, download: "big.zip", rel: null }]);
  await ext.chrome.storage.local.set({ catchLinks: true });
  await settle(30);
  assert.ok(ext.registered.get("pako-links"), "on again");
  await ext.chrome.storage.local.set({ takeOver: false });
  await settle(30);
  assert.equal(ext.registered.get("pako-links"), undefined, "takeover off: nothing registered");

  // Off from the start (saved).
  const off = await vivaldi({ stored: { pakoHolds: true, catchLinks: false } });
  assert.equal(off.registered.size, 0);
  // The toolbar window shows the switch only in a browser that asks first.
  assert.deepEqual(await ask(off, { type: "asks-first" }), { ok: true, asks: true });
});

test("Not in a browser that doesn't ask first, Pako Light, Firefox", async () => {
  const chrome = loadEdition({ dnr: "full", naming: true, tabs: { [TAB]: PAGE }, stored: { pakoHolds: true } });
  await settle(30);
  assert.equal(chrome.registered.size, 0, "Chrome: nothing");
  assert.deepEqual(await ask(chrome, { type: "asks-first" }), { ok: true, asks: false });
  const light = await vivaldi({ light: true });
  assert.equal(light.registered.size, 0, "Pako Light: nothing");
  const lr = await ask(light, { type: "link-download", url: FILE }, { tab: { id: TAB, url: PAGE }, frameId: 0, url: PAGE });
  assert.equal(lr.ok, false, "Pako Light doesn't know the request");
  const firefox = await vivaldi({ firefox: true, dnr: false, naming: false });
  assert.equal(firefox.registered.size, 0, "Firefox: nothing");
  const fr = await ask(firefox, { type: "link-download", url: FILE }, { tab: { id: TAB, url: PAGE }, frameId: 0, url: PAGE });
  assert.equal(fr.taken, false);
  await settle(30);
  assert.deepEqual(firefox.sent.filter((m) => m.kind === "download"), []);
});

test("Private windows follow the catch: the script does nothing there, and the extension never takes one", async () => {
  const ext = await vivaldi();
  const page = openPage(ext, { tabId: PRIV, incognito: true });
  assert.equal(page.click({ href: "/get/big.zip", download: "" }), false, "the browser's own way (it asks, then the file-name step)");
  assert.deepEqual(page.messages, []);
  // Asked anyway (a page told otherwise): no.
  const r = await ask(ext, { type: "link-download", url: FILE, filename: "" }, { tab: { id: PRIV, incognito: true, url: PAGE }, frameId: 0, url: PAGE });
  assert.equal(r.taken, false);
  await settle(40);
  assert.deepEqual(ext.sent, []);
  assert.deepEqual(ext.restarted, []);
});

test("Never from a frame, another site's page, or when the extension can't be reached", async () => {
  const ext = await vivaldi();
  assert.equal(openPage(ext, { frame: true }).click({ href: "/get/big.zip", download: "" }), false, "a frame: left alone");
  assert.equal(openPage(ext, { extensionGone: true }).click({ href: "/get/big.zip", download: "" }), false, "an extension reloaded since: left alone");
  // The background asked about a link to another site, or from a frame: no.
  for (const sender of [
    { tab: { id: TAB, url: PAGE }, frameId: 0, url: PAGE, msg: "https://cdn.other.example/big.zip" },
    { tab: { id: TAB, url: PAGE }, frameId: 2, url: PAGE, msg: FILE },
    { tab: { id: TAB, url: "https://www.youtube.com/" }, frameId: 0, url: "https://www.youtube.com/", msg: "https://www.youtube.com/a.zip" },
  ]) {
    const r = await ask(ext, { type: "link-download", url: sender.msg, filename: "" }, sender);
    assert.equal(r.taken, false, JSON.stringify(sender));
  }
  // The message doesn't get through: the browser downloads it as usual.
  const page = openPage(ext, { backgroundFails: true });
  assert.equal(page.click({ href: "/get/big.zip", download: "x.zip" }), true);
  await settle(30);
  assert.deepEqual(page.browserClicks, [{ href: FILE, download: "x.zip", rel: null }]);
  assert.deepEqual(ext.sent, []);
});

test("A link the page changes in its own click handler is taken as the browser would take it (read at the end of the click)", async () => {
  const pako = newPako();
  const ext = await vivaldi({}, pako);
  const page = openPage(ext);
  // The page puts the real address in at the click (a signed link, say): that one goes to Pako.
  assert.equal(page.click({ href: "/get/placeholder", download: "" }, { pageDoes: (a) => (a.href = "/get/real.zip") }), true);
  // The page makes the file itself and points the link at it (blob:): the browser's, untouched.
  assert.equal(page.click({ href: "/export", download: "report.csv" }, { pageDoes: (a) => (a.href = "blob:https://files.example.com/9f8e") }), false);
  // The page takes `download` away: a plain link now, left alone.
  assert.equal(page.click({ href: "/get/other.zip", download: "" }, { pageDoes: (a) => delete a.attrs.download }), false);
  // The page makes a plain link a download link at the click (the browser downloads it): caught.
  assert.equal(page.click({ href: "/get/late.zip" }, { pageDoes: (a) => a.setAttribute("download", "late.zip") }), true);
  await settle(80);
  const asked = ext.sent.filter((m) => m.kind === "download");
  assert.deepEqual(asked.map((m) => m.urls[0]), ["https://files.example.com/get/real.zip", "https://files.example.com/get/late.zip"]);
  assert.equal(asked[1].filename, "late.zip");
  assert.deepEqual(page.browserClicks, []);
});

test("Pako couldn't get it: clicked once more, the page's own download goes to the browser (never Pako's again)", async () => {
  const why = "Pako couldn't get this download (the server said no).";
  const ext = await vivaldi({}, newPako({ result: { ok: false, error: why, followed: "row-1" } }));
  const page = openPage(ext);
  page.click({ href: "/get/big.zip", download: "big.zip" });
  await settle(80);
  assert.equal(ext.restarted.length, 1, "started again in the browser once");
  const before = ext.sent.filter((m) => m.kind === "download").length;
  // Clicked once more (in another tab, past the double-click wait): the page's own click, as the
  // browser makes it.
  const again = openPage(ext, { tabId: 5 });
  assert.equal(again.click({ href: "/get/big.zip", download: "big.zip" }), true);
  await settle(60);
  assert.equal(ext.sent.filter((m) => m.kind === "download").length, before, "not handed to Pako again");
  assert.equal(ext.restarted.length, 1, "not started again by the extension");
  assert.deepEqual(page.browserClicks, []);
  assert.deepEqual(again.browserClicks, [{ href: FILE, download: "big.zip", rel: null }], "the browser downloads it from the page");
});
