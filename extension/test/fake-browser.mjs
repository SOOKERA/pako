// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Loads the extension's real background.js (the full extension, or Pako Light as build.mjs makes
// it) against a fake `chrome` API, in a sandbox: no browser is started, nothing leaves the test.
// What it would send to Pako, its right-click items and its toolbar "!" are recorded.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { toLight } from "../light.mjs";

const here = dirname(fileURLToPath(import.meta.url));
export const FULL_ID = "fnaplmfbaccplngclnkchlngpkbcbhod";
export const LIGHT_ID = "cojnnpgiilnbkblckkjnheigiodkdlpl";
/** The extension's version (its manifest's), which its script says too. */
export const MANIFEST_VERSION = JSON.parse(readFileSync(join(here, "..", "manifest.chromium.json"), "utf8")).version;

/** The background script of one edition, as that edition ships it. */
export function backgroundSource(light) {
  const src = readFileSync(join(here, "..", "background.js"), "utf8");
  return light ? toLight("background.js", src) : src;
}

/** Messages between extensions in one fake browser profile (by extension ID). */
export function browserProfile() {
  const installed = new Map();
  return {
    installed,
    /** Delivers `msg` from `fromId` to `toId`'s onMessageExternal, like Chrome does. */
    send(fromId, toId, msg, callback, runtime) {
      const target = installed.get(toId);
      const listener = target?.listeners.external;
      setTimeout(() => {
        if (!listener) {
          runtime.lastError = { message: "Could not establish connection. Receiving end does not exist." };
          callback(undefined);
          runtime.lastError = null;
          return;
        }
        let answered = false;
        listener(msg, { id: fromId }, (reply) => {
          answered = true;
          callback(reply);
        });
        if (!answered) callback(undefined);
      }, 1);
    },
  };
}

/**
 * One edition's background page in a fake browser.
 * - `tabs`: { id: url } the browser's tabs (tabs.get / tabs.query read them; the first is active)
 * - `links`: what "all links on this page" finds on the page
 * - `profile`: a shared browserProfile() (several extensions in one browser)
 * - `downloadsOnCreated`: a shared list, when several extensions see the same browser downloads
 * - `videos`: what each <video> of the tab says (playing, locked) when the extension asks
 * - `closedFrames`: frame IDs the browser refuses to put a script into (gone, or closed to extensions)
 * - `naming`: the browser has Chromium's file-name step (downloads.onDeterminingFilename)
 * - `restartFails`: downloads.download (the extension starting a download itself) is refused
 * - `privateAllowed`: the browser lets the extension work in private windows
 * - `privateTabs`: the IDs of the tabs that are in a private window
 * - `stored` / `sessionStored`: what storage.local / storage.session already hold (the
 *   background page starting again: the browser restarted it, or Reload in its extensions page)
 * - `dnr`: the browser's declarativeNetRequest: "full" (rules on the server's answer, Chrome 128
 *   and later), "old" (an older one: it throws on such a rule), or false (none: Firefox's way)
 * - `vivaldi`: the browser is Vivaldi (its tabs carry `vivExtData`)
 * - `alias`: the browser also names its API `browser` (Chromium 14x and later: not Firefox)
 * - `sessionRules`: the extension's session rules already in the browser (left by its background
 *   page before it slept)
 * - `manifestVersion`: the version its manifest says (another than its script's: the browser
 *   runs an older script than the folder's)
 * - `firefox`: the browser is Firefox (its API is `browser`, with getBrowserInfo, and answers
 *   with promises)
 */
export function loadEdition({ manifestVersion = MANIFEST_VERSION, light = false, tabs = {}, links = [], profile = browserProfile(), downloadsOnCreated, reply = () => ({ ok: true }), fetch, videos = [], closedFrames = [], naming = false, restartFails = false, privateAllowed = false, privateTabs = [], stored = {}, sessionStored = {}, dnr = false, vivaldi = false, alias = false, sessionRules = [], firefox = false } = {}) {
  const id = light ? LIGHT_ID : FULL_ID;
  const who = light ? "light" : "full";
  const sent = [];
  const pings = [];
  /** How many times the extension reloaded itself (runtime.reload). */
  const reloads = [];
  const menus = new Map();
  const updates = [];
  const listeners = {};
  const log = [];
  const storage = JSON.parse(JSON.stringify(stored));
  const session = JSON.parse(JSON.stringify(sessionStored));
  const badges = {};
  const fetched = [];
  /** Downloads the extension started itself (downloads.download options). */
  const restarted = [];
  /** What downloads.search says about each browser download (by id), over the defaults. */
  const items = {};
  /** The cookie stores asked for (cookies.getAll's storeId, or "default"). */
  const cookieStores = [];
  /** The fetch options of each playlist read (credentials, cache). */
  const fetchedWith = [];
  /** The recorder's content scripts registered now (by id), and the messages on the native port. */
  const registered = new Map();
  const port = [];
  /** The files put into a page (scripting.executeScript with files): { tabId, frameIds, files }. */
  const injected = [];
  const ev = (name) => ({ addListener: (f) => (listeners[name] = f) });
  const tabList = () =>
    Object.entries(tabs).map(([tid, url], i) => ({ id: Number(tid), url, title: `Page ${tid}`, active: i === 0, incognito: privateTabs.includes(Number(tid)), ...(vivaldi ? { vivExtData: "{}" } : {}) }));
  /** The extension's session rules in the browser (declarativeNetRequest), by ID. */
  const rules = new Map(sessionRules.map((r) => [r.id, plain(r)]));
  /** Tabs the extension sent to an address (tabs.update): { tabId, url }. */
  const navigated = [];
  const storageChanged = (o) => listeners.storageChanged?.(Object.fromEntries(Object.keys(o).map((k) => [k, { newValue: o[k] }])), "local");
  const runtime = {
    id,
    lastError: null,
    onInstalled: ev("installed"),
    onStartup: ev("startup"),
    onMessage: ev("message"),
    onMessageExternal: ev("external"),
    sendNativeMessage: (_host, m, cb) => {
      // A "ping" only checks the line (the extension says hello when installed): kept apart, so
      // `sent` holds what was really asked of Pako.
      if (m?.kind === "ping") pings.push(plain(m));
      else sent.push(plain(m));
      // (Firefox answers with a promise.)
      if (typeof cb !== "function") return new Promise((resolve) => setTimeout(() => resolve(reply(m)), 2));
      setTimeout(() => cb(reply(m)), 2);
    },
    ...(firefox ? { getBrowserInfo: async () => ({ name: "Firefox" }) } : {}),
    sendMessage: (toId, msg, cb) => profile.send(id, toId, msg, cb, runtime),
    getManifest: () => ({ version: manifestVersion }),
    reload: () => reloads.push(Date.now()),
    onConnect: ev("connect"),
    connectNative: () => {
      const onMessage = [];
      return {
        onMessage: { addListener: (f) => onMessage.push(f) },
        onDisconnect: { addListener() {} },
        postMessage: (m) => {
          port.push(plain(m));
          setTimeout(() => onMessage.forEach((f) => f(reply(m))), 2);
        },
        disconnect() {},
      };
    },
  };
  const chrome = {
    runtime,
    contextMenus: {
      removeAll: (cb) => {
        menus.clear();
        cb?.();
      },
      create: (o) => menus.set(o.id, plain(o)),
      update: (mid, props) => {
        updates.push([mid, plain(props)]);
        if (menus.has(mid)) Object.assign(menus.get(mid), plain(props));
        return Promise.resolve();
      },
      onClicked: ev("click"),
    },
    tabs: {
      onActivated: ev("activated"),
      onUpdated: ev("updated"),
      onRemoved: ev("removed"),
      onCreated: ev("tabCreated"),
      get: async (tid) => tabList().find((t) => t.id === tid),
      reload: async () => {},
      update: async (tid, props) => {
        if (!tabList().some((t) => t.id === tid)) throw new Error(`No tab with id: ${tid}.`);
        navigated.push(plain({ tabId: tid, ...props }));
        return tabList().find((t) => t.id === tid);
      },
      query: async (q) => {
        const list = tabList();
        if (q?.windowId != null) return list.filter((t) => t.active);
        return list.filter((t) => !q?.active || t.active);
      },
    },
    windows: { WINDOW_ID_NONE: -1, onFocusChanged: ev("focus") },
    cookies: {
      getAll: async (q) => {
        cookieStores.push(q?.storeId ?? "default");
        return [{ name: "SID", value: q?.storeId === "1" ? "private" : "x", domain: q?.url ? new URL(q.url).hostname : ".example", hostOnly: !!q?.url, path: "/" }];
      },
    },
    extension: { isAllowedIncognitoAccess: async () => privateAllowed },
    webRequest: { onHeadersReceived: ev("headers"), onSendHeaders: ev("sendHeaders"), onBeforeRedirect: ev("redirect"), onErrorOccurred: ev("requestError") },
    storage: {
      local: {
        get: async (d) => ({ ...d, ...storage }),
        set: async (o) => {
          Object.assign(storage, o);
          storageChanged(o);
        },
      },
      onChanged: ev("storageChanged"),
      session: {
        get: async (k) => (k in session ? { [k]: JSON.parse(JSON.stringify(session[k])) } : {}),
        set: async (o) => Object.assign(session, JSON.parse(JSON.stringify(o))),
        remove: async (k) => delete session[k],
      },
    },
    action: {
      setBadgeBackgroundColor() {},
      setBadgeText: ({ tabId, text }) => (tabId == null ? (storage.badge = text) : (badges[tabId] = text)),
    },
    downloads: {
      onCreated: { addListener: (f) => (downloadsOnCreated ? downloadsOnCreated.push(f) : (listeners.download = f)) },
      // (The download the extension started itself, id 99, is named in the log.)
      pause: async (did) => log.push(did === 99 ? `${who} pause 99` : `${who} pause`),
      search: async ({ id: did }) => [
        { id: did, totalBytes: 50e6, mime: "application/zip", filename: "C:\\Downloads\\big.zip", state: "in_progress", bytesReceived: 0, ...items[did] },
      ],
      cancel: async (did) => log.push(did === 99 ? `${who} cancel 99` : `${who} cancel`),
      erase: async ({ id: did } = {}) => log.push(did === 99 ? `${who} erase 99` : `${who} erase`),
      resume: async (did) => log.push(did === 99 ? `${who} resume 99` : `${who} resume`),
      onChanged: ev("changed"),
      download: async (o) => {
        log.push(`${who} download`);
        if (restartFails) throw new Error("Invalid URL");
        restarted.push(plain(o));
        return 99;
      },
      ...(naming ? { onDeterminingFilename: { addListener: (f) => (listeners.naming = f) } } : {}),
    },
    ...(dnr
      ? {
          declarativeNetRequest: {
            updateSessionRules: (o) => {
              // An older browser doesn't know conditions on the server's answer: Chrome throws at once.
              if (dnr === "old" && (o?.addRules || []).some((r) => r.condition?.responseHeaders || r.condition?.excludedResponseHeaders)) {
                throw new TypeError("Error at property 'condition': Unexpected property: 'responseHeaders'.");
              }
              for (const rid of o?.removeRuleIds || []) rules.delete(rid);
              for (const r of o?.addRules || []) rules.set(r.id, plain(r));
              return Promise.resolve();
            },
            getSessionRules: async () => [...rules.values()].map(plain),
          },
        }
      : {}),
    scripting: {
      executeScript: async ({ target, files }) => {
        if (files) {
          if (target.frameIds?.some((f) => closedFrames.includes(f))) throw new Error("No frame with id");
          injected.push(plain({ tabId: target.tabId, frameIds: target.frameIds, files }));
          return [{ result: undefined }];
        }
        return [{ result: target?.allFrames ? videos : links }];
      },
      registerContentScripts: async (list) => list.forEach((c) => registered.set(c.id, plain(c))),
      getRegisteredContentScripts: async ({ ids } = {}) => [...registered.values()].filter((c) => !ids || ids.includes(c.id)),
      unregisterContentScripts: async ({ ids }) => ids.forEach((i) => registered.delete(i)),
    },
  };
  const fakeFetch = async (url, opts) => {
    fetched.push(url);
    fetchedWith.push({ credentials: opts?.credentials, cache: opts?.cache });
    const text = fetch ? fetch(url, opts) : null;
    return { ok: text != null, text: async () => text ?? "" };
  };
  const context = { chrome, console, URL, setTimeout, clearTimeout, AbortController, fetch: fakeFetch, crypto: globalThis.crypto };
  if (alias || firefox) context.browser = chrome;
  vm.runInContext(backgroundSource(light), vm.createContext(context), { filename: `${who}/background.js` });
  const edition = { id, who, rules, navigated, sent, pings, reloads, menus, updates, listeners, log, storage, session, badges, fetched, fetchedWith, registered, port, injected, restarted, items, cookieStores, chrome };
  profile.installed.set(id, edition);
  return edition;
}

/** A declarativeNetRequest value pattern ("*" any characters, "?" zero or one), case-insensitive. */
function glob(pattern, value) {
  const re = String(pattern)
    .split("")
    .map((ch) => (ch === "*" ? ".*" : ch === "?" ? ".?" : ch.replace(/[.+^${}()|[\]\\]/g, (m) => "\\" + m)))
    .join("");
  return new RegExp(`^${re}$`, "i").test(String(value));
}
function onDomain(url, domains) {
  try {
    const h = new URL(url).hostname;
    return (domains || []).some((d) => h === d || h.endsWith("." + d));
  } catch {
    return false;
  }
}
function headersMatch(list, headers) {
  return list.some((c) => headers.some((h) => h.name.toLowerCase() === c.header.toLowerCase() && (!c.values || c.values.some((v) => glob(v, h.value)))));
}

/** The rule that decides a request on the server's answer, as Chrome's declarativeNetRequest
 *  does (the highest priority; "allow" first at the same priority), or null. */
export function decidingRule(rules, r) {
  const hits = [...rules.values()].filter((rule) => {
    const c = rule.condition || {};
    if (c.resourceTypes && !c.resourceTypes.includes(r.type)) return false;
    if (c.requestMethods && !c.requestMethods.includes(r.method.toLowerCase())) return false;
    if (c.excludedTabIds?.includes(r.tabId)) return false;
    if (c.tabIds && !c.tabIds.includes(r.tabId)) return false;
    if (onDomain(r.url, c.excludedRequestDomains)) return false;
    if (r.initiator && onDomain(r.initiator, c.excludedInitiatorDomains)) return false;
    if (c.urlFilter) {
      const exact = /^\|(.*)\|$/.exec(c.urlFilter);
      if (exact ? r.url.toLowerCase() !== exact[1].toLowerCase() : !r.url.includes(c.urlFilter)) return false;
    }
    if (c.excludedResponseHeaders && headersMatch(c.excludedResponseHeaders, r.responseHeaders)) return false;
    if (c.responseHeaders && !headersMatch(c.responseHeaders, r.responseHeaders)) return false;
    return true;
  });
  const rank = (rule) => (rule.priority || 1) * 2 + (rule.action.type === "allow" ? 1 : 0);
  hits.sort((a, b) => rank(b) - rank(a));
  return hits[0] || null;
}

/** The browser loading a page or frame whose answer may be a file: the extension sees the request
 *  go out (onSendHeaders), then the server's answer (onHeadersReceived, before the browser's
 *  rules, as in Chrome), then the browser's rules decide. Returns "caught" (turned into the
 *  extension's sent page: no download) or "download" (the browser goes on with it). */
export async function browse(edition, { url, type = "main_frame", method = "GET", tabId = 3, initiator = "", requestHeaders = [], responseHeaders = [], statusCode = 200 }) {
  const details = { url, type, method, tabId, frameId: type === "main_frame" ? 0 : 1, initiator, statusCode };
  // Chrome decides with the rules without conditions on the answer when the request starts
  // (one added later, even before the answer, doesn't count for this request: checked in Vivaldi
  // 8.3), and with those on the answer when it comes.
  const onAnswer = (r) => !!(r.condition?.responseHeaders || r.condition?.excludedResponseHeaders);
  const atStart = [...edition.rules].filter(([, r]) => !onAnswer(r));
  edition.listeners.sendHeaders?.({ ...details, requestHeaders });
  // The server takes a moment to answer.
  await settle(5);
  edition.listeners.headers?.({ ...details, responseHeaders });
  const deciding = new Map([...atStart, ...[...edition.rules].filter(([, r]) => onAnswer(r))]);
  const rule = decidingRule(deciding, { ...details, responseHeaders });
  if (rule?.action.type !== "redirect") return "download";
  edition.listeners.redirect?.({ ...details, redirectUrl: `chrome-extension://1c4ea6cf-7588-4816-8277-3a2758e249c6${rule.action.redirect.extensionPath}` });
  return "caught";
}

/** Copies a value out of the sandbox (its arrays and objects are another realm's). */
export const plain = (v) => (v === undefined ? v : JSON.parse(JSON.stringify(v)));

/** Waits for the timers the fake browser started (its answers come a few ms later). */
export const settle = (ms = 30) => new Promise((r) => setTimeout(r, ms));

/** Asks a background page like the popup does (runtime.onMessage), and waits for its answer. */
export function ask(edition, msg, sender = {}) {
  return new Promise((resolve) => edition.listeners.message(msg, sender, (r) => resolve(plain(r))));
}
