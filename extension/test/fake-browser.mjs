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
 */
export function loadEdition({ light = false, tabs = {}, links = [], profile = browserProfile(), downloadsOnCreated, reply = () => ({ ok: true }), fetch, videos = [], closedFrames = [], naming = false, restartFails = false, privateAllowed = false } = {}) {
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
  const storage = {};
  const session = {};
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
  const tabList = () => Object.entries(tabs).map(([tid, url], i) => ({ id: Number(tid), url, title: `Page ${tid}`, active: i === 0, incognito: false }));
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
      setTimeout(() => cb(reply(m)), 2);
    },
    sendMessage: (toId, msg, cb) => profile.send(id, toId, msg, cb, runtime),
    getManifest: () => ({ version: "1.0.0" }),
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
      get: async (tid) => tabList().find((t) => t.id === tid),
      reload: async () => {},
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
    webRequest: { onHeadersReceived: ev("headers") },
    storage: {
      local: {
        get: async (d) => ({ ...d, ...storage }),
        set: async (o) => Object.assign(storage, o),
      },
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
  vm.runInContext(backgroundSource(light), vm.createContext(context), { filename: `${who}/background.js` });
  const edition = { id, who, sent, pings, reloads, menus, updates, listeners, log, storage, session, badges, fetched, fetchedWith, registered, port, injected, restarted, items, cookieStores, chrome };
  profile.installed.set(id, edition);
  return edition;
}

/** Copies a value out of the sandbox (its arrays and objects are another realm's). */
export const plain = (v) => (v === undefined ? v : JSON.parse(JSON.stringify(v)));

/** Waits for the timers the fake browser started (its answers come a few ms later). */
export const settle = (ms = 30) => new Promise((r) => setTimeout(r, ms));

/** Asks a background page like the popup does (runtime.onMessage), and waits for its answer. */
export function ask(edition, msg, sender = {}) {
  return new Promise((resolve) => edition.listeners.message(msg, sender, (r) => resolve(plain(r))));
}
