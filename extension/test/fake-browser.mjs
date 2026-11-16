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
 */
export function loadEdition({ light = false, tabs = {}, links = [], profile = browserProfile(), downloadsOnCreated, reply = () => ({ ok: true }) } = {}) {
  const id = light ? LIGHT_ID : FULL_ID;
  const who = light ? "light" : "full";
  const sent = [];
  const menus = new Map();
  const updates = [];
  const listeners = {};
  const log = [];
  const storage = {};
  const ev = (name) => ({ addListener: (f) => (listeners[name] = f) });
  const tabList = () => Object.entries(tabs).map(([tid, url], i) => ({ id: Number(tid), url, active: i === 0, incognito: false }));
  const runtime = {
    id,
    lastError: null,
    onInstalled: ev("installed"),
    onStartup: ev("startup"),
    onMessage: ev("message"),
    onMessageExternal: ev("external"),
    sendNativeMessage: (_host, m, cb) => {
      sent.push(plain(m));
      setTimeout(() => cb(reply(m)), 2);
    },
    sendMessage: (toId, msg, cb) => profile.send(id, toId, msg, cb, runtime),
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
      get: async (tid) => tabList().find((t) => t.id === tid),
      query: async (q) => {
        const list = tabList();
        if (q?.windowId != null) return list.filter((t) => t.active);
        return list.filter((t) => !q?.active || t.active);
      },
    },
    windows: { WINDOW_ID_NONE: -1, onFocusChanged: ev("focus") },
    cookies: { getAll: async () => [{ name: "SID", value: "x" }] },
    storage: {
      local: {
        get: async (d) => ({ ...d, ...storage }),
        set: async (o) => Object.assign(storage, o),
      },
    },
    action: { setBadgeBackgroundColor() {}, setBadgeText: ({ text }) => (storage.badge = text) },
    downloads: {
      onCreated: { addListener: (f) => (downloadsOnCreated ? downloadsOnCreated.push(f) : (listeners.download = f)) },
      pause: async () => log.push(`${who} pause`),
      search: async ({ id: did }) => [{ id: did, totalBytes: 50e6, mime: "application/zip", filename: "C:\\Downloads\\big.zip" }],
      cancel: async () => log.push(`${who} cancel`),
      erase: async () => log.push(`${who} erase`),
      resume: async () => log.push(`${who} resume`),
    },
    scripting: { executeScript: async () => [{ result: links }] },
  };
  vm.runInContext(backgroundSource(light), vm.createContext({ chrome, console, URL, setTimeout, clearTimeout }), { filename: `${who}/background.js` });
  const edition = { id, who, sent, menus, updates, listeners, log, storage, chrome };
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
