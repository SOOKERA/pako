// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's browser extension: hands downloads and links to Pako on this PC.
// It talks to Pako only through the browser's "native messaging" (no network, no open port).
// Blocks between "full-only:start" and "full-only:end" are only in the full extension:
// `node extension/build.mjs` takes them out of Pako Light.

// The extension's words in French (lang.js): Firefox loads it first (its manifest), Chromium's
// service worker takes it here.
if (typeof importScripts === "function" && !globalThis.__pakoWords) {
  try {
    importScripts("lang.js");
  } catch (e) {
    console.warn("Pako: the extension's words didn't load (it speaks English):", e?.message || e);
  }
}

/** Firefox, by an API only Firefox has. Never by the name `browser`: Chromium browsers have it too
 *  now (seen in Vivaldi 8.3, Chromium 156), and taking them for Firefox would leave out what only
 *  Chromium does (the catch at the server's answer, reloading after an update, holding a private
 *  window's download paused). */
const FIREFOX = typeof globalThis.browser?.runtime?.getBrowserInfo === "function";
const api = FIREFOX ? globalThis.browser : globalThis.chrome;
/** Pako Light (the Chrome Web Store / Edge Add-ons edition). `node extension/build.mjs` sets this
 *  to true in that edition only. */
const LIGHT = false;
/** Pako Light's answer on the sites it leaves alone. */
const LIGHT_SKIPPED = "Pako Light doesn't work on this site.";
/** The full extension's ID (the copy Pako installs). When it is on in this browser, Pako Light
 *  steps aside: one extension takes over downloads, not two. */
const FULL_ID = "fnaplmfbaccplngclnkchlngpkbcbhod";
const HOST = "app.pako.host";
/** Browser downloads smaller than this stay in the browser. */
const SMALL = 2 * 1024 * 1024;
const NOT_CONNECTED = "Pako isn't connected. Click the Pako button in the toolbar, then Open Pako.";
/** This script's own version: always the manifests' (build.mjs and the tests check). A Chromium
 *  browser can keep running the background script it took when the extension was added, after
 *  Pako wrote a newer one in the folder, even across restarts (seen in Vivaldi 8.3): it then
 *  reads the new manifest but runs the old script. Told apart by this, the extension reloads. */
const SCRIPT_VERSION = "1.3.1";

// The toolbar window and the pages ask through here. Registered first, at
// once, so nothing else that runs at start-up (or fails there) can keep it from answering.
api.runtime.onMessage.addListener(onMessage);

// ---------- Pako's language ----------
// The extension speaks Pako's language (English or French), never the browser's. Pako's answers
// say it (`lang`: added by Pako's helper to every answer; an older Pako says nothing, so
// English), and the extension asks for it on its own ("language", answered by the helper alone:
// it never starts Pako) when the browser starts, when the background page starts after a while,
// and (Firefox) when a right-click menu shows. Kept in storage.local, so it holds when Pako can't
// be reached. Pako can't tell the extension itself (the browser only lets the extension ask).

const WORDS = globalThis.__pakoWords;
/** "en" or "fr": what Pako said last. */
let lang = "en";
/** When the extension last asked Pako for its language (also kept, across background starts). */
let langAskedAt = 0;
/** At most this often on its own (a background page started again, a menu shown). */
const LANG_EVERY = 10 * 60 * 1000;
const LANG_MENU_EVERY = 60 * 1000;
const langReady = Promise.resolve()
  .then(() => api.storage.local.get({ pakoLang: "en", langAskedAt: 0 }))
  .then(
    (s) => {
      lang = WORDS ? WORDS.langOf(s?.pakoLang) : "en";
      langAskedAt = Math.max(langAskedAt, Number(s?.langAskedAt) || 0);
    },
    () => {},
  );

/** The words for `key` (the English) in Pako's language, placeholders filled. */
function tr(key, vars) {
  if (WORDS) return WORDS.tr(lang, key, vars);
  return vars ? key.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : key;
}
/** Text that may be one of the extension's (or an older Pako's) English sentences. */
const trMaybe = (text) => (WORDS ? WORDS.maybe(lang, text) : text);

/** Pako said its language (`value`; none from an older Pako: English). A change remakes the
 *  right-click items in it; the toolbar window and the page scripts follow the storage. */
function learnLang(value) {
  const next = WORDS ? WORDS.langOf(value) : "en";
  if (next === lang) return;
  lang = next;
  saveState({ pakoLang: next });
  syncMenus(true);
}

/** Asks Pako's helper for Pako's language (`every`: not if asked within that long). Never
 *  through `send`: it changes nothing else. Sent as `private` so an older Pako, which doesn't
 *  know this request, notes nothing about it ("Unknown request.": English). Not reached: the
 *  language kept stays. */
let langAsking = null;
async function askLanguage(every = 0) {
  await langReady;
  if (langAsking) return langAsking;
  if (every && Date.now() - langAskedAt < every && langAskedAt <= Date.now()) return;
  langAskedAt = Date.now();
  saveState({ langAskedAt });
  langAsking = nativeMessage({ kind: "language", private: true })
    .then(
      (reply) => {
        if (reply && typeof reply === "object") learnLang(reply.lang);
      },
      () => {},
    )
    .finally(() => (langAsking = null));
  return langAsking;
}

// The toolbar window can learn it too (when it asks Pako's helper itself).
api.storage.onChanged?.addListener((changes, area) => {
  if (area !== "local" || !changes?.pakoLang) return;
  const next = WORDS ? WORDS.langOf(changes.pakoLang.newValue) : "en";
  if (next === lang) return;
  lang = next;
  syncMenus(true);
});
/* full-only:start */
/** The qualities the player's Download button offers (0 = audio only). */
const VIDEO_QUALITIES = [1080, 720, 480, 0];
/** Pako Light's store ID: the full extension tells it that it is here. */
const LIGHT_ID = "cojnnpgiilnbkblckkjnheigiodkdlpl";
/* full-only:end */
/** Pako takes up to 1 MB per message: "all links" go in parts well under that. */
const LINKS_PER_MESSAGE = 512 * 1024;
/** Pako drops longer links. */
const MAX_LINK = 8 * 1024;

/** The sites Pako Light leaves alone: it sends nothing from them and no links to them, and
 *  doesn't take over their downloads. (The full extension offers more there.) */
const LIGHT_SKIPS_SITES = [
  "youtube.com", "youtu.be", "youtube-nocookie.com", "vimeo.com", "twitch.tv", "facebook.com", "fb.watch",
  "instagram.com", "tiktok.com", "x.com", "twitter.com", "dailymotion.com", "dai.ly", "reddit.com", "rumble.com",
  "odysee.com", "bitchute.com", "bilibili.com", "streamable.com", "soundcloud.com",
];
function onSkippedSite(url) {
  try {
    const h = new URL(url).hostname.toLowerCase().replace(/\.$/, "").replace(/^www\./, "");
    return LIGHT_SKIPS_SITES.some((v) => h === v || h.endsWith("." + v));
  } catch {
    return false;
  }
}
/* full-only:start */
/** The full extension: these are the video sites Pako downloads videos from. */
const isVideoSite = onSkippedSite;
/* full-only:end */

// ---------- talking to Pako ----------

/** One message to Pako's helper. Chromium browsers report problems in `chrome.runtime.lastError`
 *  (read here, in the callback, so the browser's own words are kept); Firefox rejects. */
function nativeMessage(message) {
  if (FIREFOX) return globalThis.browser.runtime.sendNativeMessage(HOST, message);
  return new Promise((resolve, reject) => {
    try {
      chrome.runtime.sendNativeMessage(HOST, message, (reply) => {
        const err = chrome.runtime.lastError;
        if (err) reject(new Error(err.message || String(err)));
        else resolve(reply);
      });
    } catch (e) {
      reject(e);
    }
  });
}

/** Asks Pako. The answer is `{ ok, error?, detail? }`: `error` in plain words, `detail` = the
 *  browser's own error text when the line to Pako itself failed. */
async function send(message) {
  inFlight++;
  try {
    await langReady;
    const reply = await nativeMessage(message);
    if (reply && typeof reply === "object") {
      newerInFolder(reply.ext);
      learnHolds(reply);
      learnLang(reply.lang);
      // Pako's words (an older Pako's are English): in Pako's language.
      if (typeof reply.error === "string") reply.error = trMaybe(reply.error);
      /* full-only:start */
      learnAsks(reply);
      learnPrepares(reply);
      /* full-only:end */
      if (reply.ok && message.kind !== "follow") followsAgain();
      /* full-only:start */
      if (reply.ok) pakoAnswered(true);
      /* full-only:end */
      return reply;
    }
    const detail = tr("No answer from {host}", { host: HOST });
    console.warn("Pako:", detail);
    /* full-only:start */
    pakoAnswered(false);
    /* full-only:end */
    return { ok: false, error: tr("Pako didn't answer."), detail };
  } catch (e) {
    const detail = e?.message || String(e);
    console.warn(`Pako: couldn't reach ${HOST}:`, detail);
    /* full-only:start */
    pakoAnswered(false);
    /* full-only:end */
    return { ok: false, error: tr(NOT_CONNECTED), detail };
  } finally {
    inFlight--;
  }
}

// ---------- picking up a new version after a Pako update ----------
// Pako writes its new extension into the "Load unpacked" folder when it updates, but a Chromium
// browser only reads that folder when it starts. Pako's answers say which version is in the
// folder (`ext`): when it's newer than this one, the extension reloads itself once nothing is
// on its way to Pako and nothing is being recorded. Firefox updates its signed copy itself, and
// Pako Light comes from a store: neither does this. Once per version, so a copy loaded from
// another folder never reloads in a loop.

/** Messages to Pako not answered yet. */
let inFlight = 0;

/** "1.2.10" > "1.2.9". */
function newerVersion(a, b) {
  const pa = String(a).split(".").map((n) => parseInt(n, 10) || 0);
  const pb = String(b).split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    if ((pa[i] || 0) !== (pb[i] || 0)) return (pa[i] || 0) > (pb[i] || 0);
  }
  return false;
}

/** Never two reloads closer than this, whatever the folder says (no reload loop). */
const RELOAD_GAP = 10 * 60 * 1000;
let reloadPlanned = false;
async function newerInFolder(ext) {
  if (LIGHT || FIREFOX || reloadPlanned) return;
  if (typeof ext !== "string" || !newerVersion(ext, api.runtime.getManifest().version)) return;
  reloadFor(ext);
}
/** The browser runs an older script than its manifest's (see SCRIPT_VERSION): a reload makes it
 *  take the folder's. Once per version, like a newer one in the folder. */
function olderScript() {
  if (LIGHT || FIREFOX || reloadPlanned) return;
  const version = api.runtime.getManifest().version;
  if (typeof version === "string" && version !== SCRIPT_VERSION) reloadFor(version);
}
/** Reloads the extension for `ext` (the folder's version): once per version, never two reloads
 *  within RELOAD_GAP, and not while something is on its way to Pako or being recorded. */
async function reloadFor(ext) {
  if (reloadPlanned) return;
  const got = await Promise.resolve(api.storage.local.get({ reloadedFor: "", reloadedAt: 0 })).catch(() => null);
  if (!got || got.reloadedFor === ext) return;
  const at = Number(got.reloadedAt) || 0;
  if (at <= Date.now() && Date.now() - at < RELOAD_GAP) return;
  reloadPlanned = true;
  const tryReload = async () => {
    const recording = typeof rec !== "undefined" && rec && ["waiting", "recording", "saving"].includes(rec.state);
    if (inFlight > 0 || recording) return void setTimeout(tryReload, 5000);
    // Noted first: when it can't be noted, no reload (the next start couldn't tell it did).
    const noted = await Promise.resolve(api.storage.local.set({ reloadedFor: ext, reloadedAt: Date.now() })).then(() => true, () => false);
    // (Something set off for Pako while it was noted: later.)
    if (noted && inFlight > 0) return void setTimeout(tryReload, 5000);
    if (noted) api.runtime.reload();
  };
  setTimeout(tryReload, 3000);
}

/** The cookies the browser would send to this address (so downloads behind a login work), from
 *  the tab's own cookie store (a Firefox container has its own). They only travel to Pako on
 *  this PC, and Pako keeps them for that download only. Never from a private window. */
async function cookiesFor(url, incognito, storeId) {
  // A private window: only its own store, never your normal one (none known = no cookies).
  if (incognito && !(typeof storeId === "string" && storeId)) return "";
  try {
    const query = typeof storeId === "string" && storeId ? { url, storeId } : { url };
    const list = await api.cookies.getAll(query);
    const header = list.map((c) => `${c.name}=${c.value}`).join("; ");
    // Control characters could add headers: Pako refuses them, so they aren't sent.
    return header.length <= 16000 && !/[\u0000-\u001f\u007f]/.test(header) ? header : "";
  } catch {
    return "";
  }
}

/* full-only:start */
/** The site a video link belongs to, for its cookies ("youtube.com" for youtu.be too). */
function cookieSite(url) {
  try {
    const h = new URL(url).hostname.toLowerCase().replace(/\.$/, "");
    const short = { "youtu.be": "youtube.com", "fb.watch": "facebook.com", "dai.ly": "dailymotion.com" };
    if (short[h]) return short[h];
    const labels = h.split(".");
    return labels.length >= 2 ? labels.slice(-2).join(".") : "";
  } catch {
    return "";
  }
}

/** A video site's cookies (so videos that need you to be logged in work), for yt-dlp. Never
 *  from a private window; only this site's own cookies, well formed, capped. */
async function siteCookiesFor(url, incognito, storeId) {
  const site = cookieSite(url);
  // A private window: only its own store (Firefox names it), never your normal login.
  if (!site || (incognito && !(typeof storeId === "string" && storeId))) return [];
  try {
    const query = typeof storeId === "string" && storeId ? { domain: site, storeId } : { domain: site };
    const list = await api.cookies.getAll(query);
    const plain = (t) => typeof t === "string" && !/[\u0000-\u001f\u007f]/.test(t);
    const out = [];
    let bytes = 0;
    for (const c of list) {
      if (!plain(c.name) || !c.name || !plain(c.value) || !plain(c.domain) || !plain(c.path || "/")) continue;
      bytes += c.domain.length + (c.path || "/").length + c.name.length + c.value.length;
      if (bytes > 60000 || out.length >= 300) break;
      out.push({
        domain: c.hostOnly ? c.domain.replace(/^\./, "") : c.domain.startsWith(".") ? c.domain : "." + c.domain,
        path: c.path || "/",
        secure: !!c.secure,
        expires: c.session || !c.expirationDate ? 0 : Math.floor(c.expirationDate),
        name: c.name,
        value: c.value,
      });
    }
    return out;
  } catch {
    return [];
  }
}
/* full-only:end */

function isWebLink(url) {
  return typeof url === "string" && /^https?:\/\//i.test(url);
}

/** `totalBytes` and `mime` (takeover only): the browser's file, so Pako takes it only when its
 *  own first answer from the server is the same file. */
/** `isPrivate`: from a private window: Pako keeps no trace of it beyond the file (every kind
 *  of request says so, not only a taken-over download). */
async function download(url, referrer, filename, { takeover = false, incognito = false, storeId, totalBytes = 0, mime = "", hold = false, paused = false, isPrivate = false } = {}) {
  if (!isWebLink(url)) return { ok: false, error: tr("Pako only takes http and https links.") };
  // Pako Light sends nothing from the sites it leaves alone (Pako checks again on its side).
  if (LIGHT && (onSkippedSite(url) || onSkippedSite(referrer))) return { ok: false, error: tr(LIGHT_SKIPPED) };
  const message = {
    kind: "download",
    urls: [url],
    referrer: referrer || "",
    cookies: await cookiesFor(url, incognito || isPrivate, storeId),
    filename: filename || "",
    takeover,
    totalBytes: takeover && totalBytes > 0 ? totalBytes : 0,
    mime: takeover && typeof mime === "string" ? mime : "",
  };
  // Pako may answer at once and check meanwhile (`held`): only asked of a Pako that said so.
  if (takeover && hold) message.hold = true;
  if (takeover && hold && paused) message.paused = true;
  // From a private window: Pako keeps no trace of it beyond the file.
  if (isPrivate) message.private = true;
  /* full-only:start */
  // A video page or link: the site's login goes with it (Pako downloads the video). Never a
  // private window's: Pako would have to write it to a file for its video tool.
  if (!takeover && isVideoSite(url)) message.siteCookies = isPrivate ? [] : await siteCookiesFor(url, incognito, storeId);
  /* full-only:end */
  return send(message);
}

/** "All links on this page": in parts small enough for Pako's line (each part is added to Links).
 *  Pako Light leaves out links to the sites it leaves alone, and sends nothing from them. */
async function sendLinks(urls, referrer, isPrivate = false) {
  if (LIGHT && onSkippedSite(referrer)) return { ok: false, error: tr(LIGHT_SKIPPED) };
  // The page's address travels with each part.
  const base = JSON.stringify(referrer || "").length + 64;
  const parts = [];
  let part = [];
  let size = base;
  for (const u of urls) {
    if (typeof u !== "string" || u.length > MAX_LINK) continue;
    if (LIGHT && onSkippedSite(u)) continue;
    const cost = JSON.stringify(u).length + 1;
    if (part.length && size + cost > LINKS_PER_MESSAGE) {
      parts.push(part);
      part = [];
      size = base;
    }
    part.push(u);
    size += cost;
  }
  if (part.length) parts.push(part);
  if (!parts.length) return { ok: false, error: tr("No links on this page.") };
  for (const p of parts) {
    // A private window's links: kept in Pako's memory only, marked private in Links.
    const reply = await send(isPrivate ? { kind: "links", urls: p, referrer, private: true } : { kind: "links", urls: p, referrer });
    if (!reply.ok) return reply;
  }
  return { ok: true };
}

/* full-only:start */
/** The player's Download button: one video straight to Downloads in that quality. */
async function sendVideo(url, quality, { incognito = false, storeId } = {}) {
  if (!isWebLink(url)) return { ok: false, error: tr("Pako only takes http and https links.") };
  if (!VIDEO_QUALITIES.includes(quality)) return { ok: false, error: tr("Unknown video quality.") };
  // Only a video site's link: no other site's cookies are ever read for this.
  if (!isVideoSite(url)) return { ok: false, error: tr("Pako can't download videos from this page.") };
  // A private window's: no cookies (Pako's video tool would need them in a file), no trace.
  if (incognito) return send({ kind: "video", urls: [url], quality, siteCookies: [], private: true });
  return send({ kind: "video", urls: [url], quality, siteCookies: await siteCookiesFor(url, incognito, storeId) });
}

/** Reads ahead asked of Pako (or about to be) and neither picked nor cancelled yet, by tab and
 *  link (`aheadKey`): what the menu closing, or the tab closing, stops. In memory only. */
const readsAhead = new Map();
/** Cancels on their way to Pako, by the same key: a read ahead of the same link waits for it
 *  (so the cancel never stops the newer read). */
const cancelling = new Map();
const aheadKey = (tabId, url) => `${tabId} ${url}`;

/** The player's Download button opened its quality menu (you clicked it, never on hover): Pako
 *  starts reading the video now, so the quality picked next starts at once. The same checks and
 *  cookies as the send, with no quality; only to a Pako that said it reads ahead (an older one
 *  would answer "Unknown request."). Its answer says nothing: the send that follows does. */
async function prepareVideo(url, { tabId, incognito = false, storeId } = {}) {
  if (!isWebLink(url) || !isVideoSite(url)) return;
  const key = aheadKey(tabId, url);
  // Noted before anything is awaited: a close or a pick that comes meanwhile finds it.
  const entry = { tabId, url, incognito, went: false, sent: null };
  // The cancel of an earlier read of it still on its way (never one of this read: it waits for it).
  const earlier = cancelling.get(key);
  readsAhead.set(key, entry);
  entry.sent = (async () => {
    if (!(await pakoPrepares())) return;
    const siteCookies = incognito ? [] : await siteCookiesFor(url, incognito, storeId);
    await earlier;
    // Picked, or closed again, meanwhile: nothing to read ahead.
    if (readsAhead.get(key) !== entry) return;
    entry.went = true;
    if (incognito) await send({ kind: "videoPrepare", urls: [url], siteCookies: [], private: true });
    else await send({ kind: "videoPrepare", urls: [url], siteCookies });
  })().catch(() => {});
  await entry.sent;
}

/** A quality was picked: its read ahead is the pick's now (nothing cancels it any more). */
function pickedAhead(tabId, url) {
  readsAhead.delete(aheadKey(tabId, url));
}

/** The quality menu closed without a pick, or its tab closed: Pako stops that read at once
 *  (after the read ahead itself reached Pako, so the cancel never overtakes it). */
async function cancelAhead(key) {
  const entry = readsAhead.get(key);
  if (!entry) return;
  readsAhead.delete(key);
  const done = (async () => {
    await entry.sent;
    if (!entry.went) return;
    if (entry.incognito) await send({ kind: "videoPrepareCancel", urls: [entry.url], private: true });
    else await send({ kind: "videoPrepareCancel", urls: [entry.url] });
  })().catch(() => {});
  cancelling.set(key, done);
  await done;
  if (cancelling.get(key) === done) cancelling.delete(key);
}

/** A tab closed (or was swapped): its reads ahead stop. */
function cancelAheadIn(tabId) {
  for (const [key, entry] of readsAhead) if (entry.tabId === tabId) cancelAhead(key);
}

/** Does this Pako read a video ahead (`prepares` in its answers)? Saved: the background page is
 *  started again often. */
let preparesNow = null;
async function pakoPrepares() {
  if (preparesNow === null) preparesNow = (await Promise.resolve(api.storage.local.get({ pakoPrepares: false })).catch(() => ({ pakoPrepares: false }))).pakoPrepares === true;
  return preparesNow;
}
function learnPrepares(reply) {
  const v = reply.prepares === true;
  if (v === preparesNow) return;
  preparesNow = v;
  saveState({ pakoPrepares: v });
}
/* full-only:end */

async function settings() {
  const s = await api.storage.local.get({ takeOver: true });
  return s;
}

// ---------- Pako Light and the full extension in one browser ----------

/** How long Pako Light trusts its last answer about the full extension. */
const FULL_CHECK_TTL = 30 * 1000;
let fullCheck = { at: -Infinity, here: false };

/** Pako Light only: is the full extension on in this browser (and answering)? Then Light stays
 *  out of the way: no right-click items, no taking over downloads. A message between the two
 *  extensions needs no permission. */
async function fullHere() {
  if (!LIGHT) return false;
  if (Date.now() - fullCheck.at < FULL_CHECK_TTL) return fullCheck.here;
  const here = await new Promise((resolve) => {
    try {
      api.runtime.sendMessage(FULL_ID, { type: "pako-full-here" }, (reply) => {
        void api.runtime.lastError; // not installed, or turned off: Light carries on
        resolve(reply?.here === true);
      });
    } catch {
      resolve(false);
    }
  });
  fullCheck = { at: Date.now(), here };
  return here;
}

/* full-only:start */
// The full extension answers Pako Light (and nobody else).
api.runtime.onMessageExternal?.addListener((msg, sender, respond) => {
  if (sender?.id === LIGHT_ID && msg?.type === "pako-full-here") respond({ here: true });
});
/* full-only:end */

// ---------- right-click menu ----------

/** Whether the right-click items are there now (Pako Light takes them away while the full
 *  extension is on). */
let menusShown = null;
/** The media items: "pako-media" (images) and, in the full extension, its other kinds. */
const isMediaItem = (id) => typeof id === "string" && id.startsWith("pako-media");

function makeMenus() {
  return new Promise((resolve) => {
    let made = false;
    const make = () => {
      if (made) return;
      made = true;
      // Named for what they take: on an image that is also a link, both show, and two items
      // both called "Download with Pako" couldn't be told apart.
      api.contextMenus.create({ id: "pako-link", title: tr("Download link with Pako"), contexts: ["link"] });
      api.contextMenus.create({ id: "pako-media", title: tr("Download image with Pako"), contexts: ["image"] });
      /* full-only:start */
      api.contextMenus.create({ id: "pako-media-video", title: tr("Download video with Pako"), contexts: ["video"] });
      api.contextMenus.create({ id: "pako-media-audio", title: tr("Download audio with Pako"), contexts: ["audio"] });
      /* full-only:end */
      api.contextMenus.create({ id: "pako-page", title: tr("Send this page to Pako"), contexts: ["page"] });
      api.contextMenus.create({ id: "pako-all", title: tr("Send all links on this page to Pako (to pick from)"), contexts: ["page"] });
      resolve();
    };
    // Chromium calls back; Firefox's `browser` answers with a promise.
    const r = api.contextMenus.removeAll(make);
    r?.then?.(make, make);
  });
}

/** Makes the right-click items, or (Pako Light, next to the full extension) takes them away.
 *  One at a time: two at once would make each item twice. */
let menuWork = Promise.resolve();
function syncMenus(force = false) {
  menuWork = menuWork.then(() => syncMenusNow(force)).catch(() => {});
  return menuWork;
}
async function syncMenusNow(force) {
  // (Made in Pako's language: the one kept, read at start.)
  await langReady;
  const show = !(await fullHere());
  if (!force && show === menusShown) return;
  menusShown = show;
  if (show) {
    await makeMenus();
    retitleFor({ active: true, lastFocusedWindow: true });
  } else api.contextMenus.removeAll(() => {});
}

/** The page items follow the tab: Pako Light greys them out on the sites it leaves alone. */
function retitlePageItem(url) {
  const skipped = onSkippedSite(url);
  let changes = [];
  if (LIGHT) changes = [["pako-page", { enabled: !skipped }], ["pako-all", { enabled: !skipped }]];
  /* full-only:start */
  else changes = [["pako-page", { title: skipped ? tr("Download video with Pako") : tr("Send this page to Pako") }]];
  /* full-only:end */
  for (const [id, props] of changes) {
    try {
      const r = api.contextMenus.update(id, props);
      r?.catch?.(() => {});
    } catch {
      // the menu isn't made yet: it is made with the plain title
    }
  }
}
/** The page items for the active tab that `query` finds. */
function retitleFor(query) {
  Promise.resolve(api.tabs.query(query)).then((tabs) => tabs?.[0]?.url && retitlePageItem(tabs[0].url), () => {});
}
api.tabs.onActivated.addListener(({ tabId }) => {
  if (LIGHT) syncMenus();
  api.tabs.get(tabId).then((t) => t?.url && retitlePageItem(t.url), () => {});
});
api.tabs.onUpdated.addListener((_id, change, tab) => {
  if (change.url && tab?.active) retitlePageItem(change.url);
});
// Another browser window came to the front: its tab decides.
api.windows?.onFocusChanged?.addListener((windowId) => {
  if (typeof windowId !== "number" || windowId < 0) return; // no browser window in front
  retitleFor({ active: true, windowId });
});

api.runtime.onInstalled.addListener(() => {
  syncMenus(true);
  // Say hello to Pako's helper right away (a "ping" is answered by the helper alone: it never
  // starts Pako), so Settings › Browser extension shows this browser as Installed at once.
  send({ kind: "ping" }).catch(() => {});
});
api.runtime.onStartup?.addListener(() => {
  syncMenus(true);
  // The browser just started: Pako's language may have changed meanwhile.
  askLanguage();
});
// Firefox shows its menus only when asked: they follow a language changed in Pako since.
api.contextMenus.onShown?.addListener(() => {
  askLanguage(LANG_MENU_EVERY).then(() => api.contextMenus.refresh?.());
});
// The background page started again (it sleeps when idle): asked now and then.
askLanguage(LANG_EVERY);

async function pageLinks(tabId) {
  const [result] = await api.scripting.executeScript({
    target: { tabId },
    // (Not `document.links`: a page can put another element under that name.)
    func: () =>
      [...new Set([...document.querySelectorAll("a[href], area[href]")].map((a) => a.href).filter((h) => typeof h === "string" && /^https?:\/\//i.test(h)))].slice(0, 2000),
  });
  return result?.result ?? [];
}

api.contextMenus.onClicked.addListener(async (info, tab) => {
  await langReady;
  const referrer = info.pageUrl || tab?.url || "";
  // No tab told (Firefox, outside a tab): taken as private, so no cookies are read for it.
  const incognito = tab ? !!tab.incognito : true;
  // From a private window: Pako keeps no trace of it (whatever the item).
  const isPrivate = !!tab?.incognito;
  // Firefox containers: the tab's own cookies (Chromium browsers have no such field).
  const storeId = tab?.cookieStoreId;
  let reply;
  // Pako Light: nothing from a frame of a site it leaves alone either (an embedded one).
  if (LIGHT && onSkippedSite(info.frameUrl)) reply = { ok: false, error: tr(LIGHT_SKIPPED) };
  /* full-only:start */
  // A player built from parts has no address of its own (blob:): its page on a video site,
  // else what the Pako button offers for this page.
  else if (isMediaItem(info.menuItemId) && /^(video|audio)$/.test(info.mediaType || "") && !/^https?:/i.test(info.srcUrl || "")) {
    if (tab?.url && isVideoSite(tab.url)) reply = await download(tab.url, "", "", { incognito, storeId, isPrivate });
    else reply = { ok: false, error: tr("This player doesn't give its video's address. Click the Pako button: Videos on this page, or Record this video.") };
  }
  /* full-only:end */
  else if (info.menuItemId === "pako-link") reply = await download(info.linkUrl, referrer, "", { incognito, storeId, isPrivate });
  else if (isMediaItem(info.menuItemId)) reply = await download(info.srcUrl, referrer, "", { incognito, storeId, isPrivate });
  else if (info.menuItemId === "pako-page") reply = await download(tab?.url, "", "", { incognito, storeId, isPrivate });
  else if (info.menuItemId === "pako-all" && tab?.id != null) {
    // Pako Light doesn't even read the page's links on a site it leaves alone.
    if (LIGHT && (onSkippedSite(tab.url) || onSkippedSite(referrer))) reply = { ok: false, error: tr(LIGHT_SKIPPED) };
    else {
      const urls = await pageLinks(tab.id).catch(() => []);
      reply = urls.length ? await sendLinks(urls, referrer, isPrivate) : { ok: false, error: tr("No links on this page.") };
    }
  }
  if (reply && !reply.ok) flag(reply.error, reply.detail, isPrivate);
});

/** A short "!" on the toolbar button when something went wrong (details in the popup).
 *  `isPrivate`: about something from a private window (Pako's words may name what was sent):
 *  kept in the browser's memory only, never in the extension's storage on disk. */
async function flag(error, detail, isPrivate = false) {
  await langReady;
  // (Pako's words, from an older Pako in English: in Pako's language.)
  const said = { lastError: trMaybe(error || ""), lastDetail: detail || "" };
  if (isPrivate) await Promise.resolve(api.storage.session?.set(said)).catch(() => {});
  else await api.storage.local.set(said);
  api.action.setBadgeBackgroundColor({ color: "#F96263" });
  api.action.setBadgeText({ text: error ? "!" : "" });
}

// ---------- taking over browser downloads ----------

/** The browser flagged this file (Safe Browsing and similar): it keeps its own checks. */
function isUnsafe(item) {
  const d = item?.danger;
  return !!d && d !== "safe" && d !== "accepted";
}

/** The file's name without its folder. */
function baseName(path) {
  return String(path || "").split(/[\\/]/).pop();
}

/** Downloads the extension started again in the browser (Pako couldn't get them): never taken
 *  over again. The browser marks them too (`byExtensionId`). */
const restarted = new Set();

/** Like IDM: the browser doesn't download what Pako takes. Pako opens its window at once and
 *  its first request is the real download; only if Pako can't get the file (not the same file,
 *  no answer, Pako unreachable) does the browser download it again itself, with the "!" saying
 *  why. A download that can't be asked for again (a form sent with POST, and anything Pako
 *  Light can't tell apart) is held paused by the browser instead, until Pako confirms. */
async function takeOver(item, letGo = () => {}) {
  // When the browser told the extension (before anything below waits).
  const toldAt = Date.now();
  let released = false;
  // The browser's file-name step (Chromium): it waits for this, so no "Save as" shows meanwhile.
  const release = () => {
    if (released) return;
    released = true;
    try {
      letGo();
    } catch {
      // the download is already gone
    }
  };
  const { takeOver: on } = await settings();
  const url = item.finalUrl || item.url;
  if (!on || !isWebLink(url) || item.byExtensionId || restarted.has(url) || restarted.has(item.url)) return release();
  // Flagged files stay with the browser's warning. A private window's download reaches the
  // extension only when you allowed it there in the browser (its own switch for Pako).
  if (isUnsafe(item) || (item.incognito && !(await privateAllowed()))) return release();
  // Pako Light: the sites it leaves alone keep their downloads, and when the full extension is
  // on in this browser, that one takes over instead.
  if (LIGHT && (onSkippedSite(url) || onSkippedSite(item.url) || onSkippedSite(item.referrer))) return release();
  if (LIGHT && (await fullHere())) return release();
  let seen = null;
  /* full-only:start */
  // What the browser's own answer from the server said (size, kind, name, POST or not).
  seen = responseFor(url) || responseFor(item.url);
  // The browser told the extension long after the server answered: it asked you first ("Download
  // …? Save, Save As…"). Seen twice in a row, such downloads are caught at the server's answer.
  if (seen && namingStep) noteAsking(toldAt - seen.at > ASKED_FIRST);
  /* full-only:end */
  const holds = await pakoHolds();
  const firstSize = item.totalBytes > 0 ? item.totalBytes : seen?.size || 0;
  if (firstSize > 0 && firstSize < SMALL) return release();
  const isPrivate = !!item.incognito;
  // Its cookies come from its own window's store (a private window's is never mixed with yours).
  const storeId = item.cookieStoreId ?? (isPrivate ? PRIVATE_STORE : undefined);
  // Until Pako has it or the browser has it back, the extension doesn't reload itself for an
  // update (see newerInFolder): the browser's copy may already be gone.
  inFlight++;
  try {
    await handOff(item, url, seen, holds, firstSize, isPrivate, storeId, release);
  } finally {
    inFlight--;
  }
}

/** The rest of `takeOver`, once the download is Pako's to take. */
async function handOff(item, url, seen, holds, firstSize, isPrivate, storeId, release) {
  if (holds && canStartAgain(seen, isPrivate)) {
    const name = baseName(item.filename) || seen?.filename || "";
    // Gone from the browser at once; Pako has it from here.
    await api.downloads.cancel(item.id).catch(() => {});
    await api.downloads.erase({ id: item.id }).catch(() => {});
    release();
    const reply = await handOver(url, item.referrer || "", name, { storeId, totalBytes: firstSize, mime: item.mime || seen?.type || "", hold: true, isPrivate });
    if (!reply.ok) await startAgain(url, name, storeId, reply, isPrivate);
    return;
  }
  // Held paused while Pako is asked; it carries on in the browser if Pako can't take it.
  try {
    await api.downloads.pause(item.id);
  } catch {
    return release(); // already finished (tiny file): nothing to take over
  }
  const [now] = await api.downloads.search({ id: item.id }).catch(() => []);
  if (!now || isUnsafe(now)) {
    api.downloads.resume(item.id).catch(() => {});
    return release();
  }
  const size = now.totalBytes > 0 ? now.totalBytes : now.fileSize > 0 ? now.fileSize : 0;
  if (size > 0 && size < SMALL) {
    api.downloads.resume(item.id).catch(() => {});
    return release();
  }
  const name = baseName(now.filename) || baseName(item.filename) || seen?.filename || "";
  // Pako's answer is "ok" only once its own first answer from the server is the same file, so
  // the browser's copy is deleted only then; otherwise it carries on here.
  const reply = await handOver(url, item.referrer || "", name, {
    storeId: now.cookieStoreId ?? storeId,
    totalBytes: now.totalBytes > 0 ? now.totalBytes : 0,
    mime: now.mime || item.mime || "",
    hold: holds,
    // The browser holds its copy paused: Pako checks the file before asking you anything, so
    // the browser's paused copy (a form's answer can't be asked for again) isn't kept waiting.
    // Firefox's private window, asked with a plain GET: its copy can wait for your answer, and
    // is only dropped once Pako downloads it (nothing lost if Pako quits while it asks).
    paused: holds && !(FIREFOX && isPrivate && (!seen?.method || seen.method === "GET") && !seen?.auth),
    isPrivate,
  });
  if (reply.ok) {
    await api.downloads.cancel(item.id).catch(() => {});
    await api.downloads.erase({ id: item.id }).catch(() => {});
    release();
  } else {
    api.downloads.resume(item.id).catch(() => {});
    release();
    if (reply.error) flag(reply.error, reply.detail, isPrivate);
    // Pako couldn't get it: the browser carries on with it, and Pako follows it ("Via browser").
    if (typeof reply.followed === "string") follow(item.id, reply.followed, url, name, isPrivate);
  }
}

/** Can the browser ask for this file again if Pako can't get it? Not a form sent with POST
 *  (asking again with a plain request gives something else). Pako Light can't see how the
 *  browser asked, so it never assumes it can. */
function canStartAgain(seen, isPrivate = false) {
  if (LIGHT) return false;
  // A private window's download: kept paused in the browser until Pako really has it (Chrome,
  // Edge and similar could only start it again in your normal window; and a private download
  // isn't saved in Pako's list until it runs, so Pako quitting meanwhile mustn't lose it).
  if (isPrivate) return false;
  // Asked with a login other than cookies (a site's own sign-in box): Pako can't send it, so it
  // would never get the file, and the browser keeps its copy (paused) instead.
  if (seen?.auth) return false;
  return !seen || !seen.method || seen.method === "GET";
}

/** Hands a browser download to Pako. A Pako that answers at once ("held") is then asked for the
 *  result (again while it still waits for your answer in its window). An older Pako answers
 *  only once it has checked the file, as before. */
async function handOver(url, referrer, name, { storeId, totalBytes, mime, hold, paused = false, isPrivate = false }) {
  let reply = await download(url, referrer, name, { takeover: true, storeId, totalBytes, mime, hold, paused, isPrivate });
  while (reply.ok && typeof reply.held === "string") {
    const held = reply.held;
    // (A private window's: Pako's helper keeps no note of its problems.)
    reply = await send(isPrivate ? { kind: "held", held, private: true } : { kind: "held", held });
    // Still waiting in Pako's window: ask again.
    if (!reply.ok && reply.held === held && !reply.error) reply = { ok: true, held };
  }
  /* full-only:start */
  // Not Pako's own "no" (that comes with its "Via browser" row, or without an error): it wasn't
  // reached, so downloads aren't caught at the server's answer until it answers again.
  if (!reply.ok && reply.error && typeof reply.followed !== "string") pakoAnswered(false);
  /* full-only:end */
  return reply;
}

/** Pako couldn't get it: the browser downloads it itself (marked, so it isn't taken over
 *  again). Pako follows it as one of its own ("Via browser"); without Pako, the "!" says why. */
async function startAgain(url, name, storeId, reply, isPrivate = false) {
  restarted.add(url);
  setTimeout(() => restarted.delete(url), 60_000);
  const opts = { url, conflictAction: "uniquify" };
  if (name) opts.filename = name;
  // Firefox: the same container as before.
  if (FIREFOX && storeId) opts.cookieStoreId = storeId;
  // A private window's download is started again in a private window, with its store (Firefox;
  // Chrome and similar never start one again, see canStartAgain).
  if (isPrivate && FIREFOX) {
    opts.incognito = true;
    if (!opts.cookieStoreId) opts.cookieStoreId = "firefox-private";
  }
  let started = await api.downloads.download(opts).then((id) => id, () => null);
  if (started == null && opts.filename) {
    delete opts.filename;
    started = await api.downloads.download(opts).then((id) => id, () => null);
  }
  if (started == null) {
    const why = (reply.error || tr("Pako couldn't take this download")).replace(/[.\s]+$/, "");
    flag(tr("{why}. Your browser couldn't start it again: open the link again to download it.", { why }), reply.detail, isPrivate);
    // Pako's row for it says so too.
    if (typeof reply.followed === "string") {
      send({ kind: "follow", urls: [url], filename: name, private: isPrivate, follow: { pako: reply.followed, browserId: -1, state: "interrupted", error: "RESTART_FAILED" } });
    }
    return;
  }
  // Pako follows it ("Via browser"). Without an answer from Pako (closed): the "!" says why, and
  // Pako lists it once it answers again.
  follow(started, typeof reply.followed === "string" ? reply.followed : "", url, name, isPrivate);
  if (typeof reply.followed !== "string" && reply.error) flag(reply.error, reply.detail, isPrivate);
  watchRestart(started, name, isPrivate);
}

/** How long after starting a download again the extension checks it didn't stop at once. */
const RESTART_CHECK = 5000;

/** A download started again that the server refused straight away (a login Pako and the browser
 *  can't send again, a link that worked only once): the "!" says so, even when Pako isn't there
 *  to list it. Never a silent loss. */
function watchRestart(id, name, isPrivate) {
  const timer = setTimeout(async () => {
    const [now] = await api.downloads.search({ id }).catch(() => []);
    if (!now || now.state !== "interrupted" || now.error === "USER_CANCELED") return;
    const said = name
      ? tr('Pako couldn\'t get "{name}", and your browser couldn\'t start it again. Download it once more from the page.', { name })
      : tr("Pako couldn't get this download, and your browser couldn't start it again. Download it once more from the page.");
    flag(said, String(now.error || ""), isPrivate);
  }, RESTART_CHECK);
  timer?.unref?.();
}

// ---------- "Via browser": Pako follows a download the browser makes ----------
// When Pako can't get a file itself, the browser downloads it and Pako lists it as its own:
// the extension reports its progress (every few seconds, and at once when it changes state),
// does what you ask in Pako (pause, resume, cancel), and once it is finished Pako moves the file
// to your folder; only once it's moved does the browser's list forget it. A report Pako
// couldn't take (Pako closed) is kept and sent when Pako answers again.

/** By browser download id: { pako, url, name, private, sent, state }. Kept in storage.local;
 *  a private window's only in the browser's memory (storage.session), never on disk. */
let follows = null;
/** Each report starts Pako's helper once: every 2 s while the browser downloads, every 5 s
 *  while it is paused or stopped (only to pick up Resume from Pako). */
const FOLLOW_EVERY = 2000;
const FOLLOW_IDLE_EVERY = 5000;
/** Pako didn't answer: reports wait until then (or until another answer from Pako). */
let followQuietUntil = 0;
let followTimer = null;

/** A follow as it was kept, or null when it isn't one (left by another version, damaged). */
function keptFollow(f) {
  if (!f || typeof f !== "object" || Array.isArray(f) || typeof f.url !== "string" || !isWebLink(f.url)) return null;
  return {
    pako: typeof f.pako === "string" ? f.pako : "",
    url: f.url,
    name: typeof f.name === "string" ? f.name : "",
    private: f.private === true,
    sent: Number.isFinite(f.sent) ? f.sent : 0,
    state: typeof f.state === "string" ? f.state : "",
  };
}

/** The follows kept in `stored` (one storage area's value): the good ones, and whether any was
 *  left out. */
function keptFollows(stored) {
  const out = {};
  if (!stored || typeof stored !== "object" || Array.isArray(stored)) return { out, bad: stored != null };
  let bad = false;
  for (const [id, f] of Object.entries(stored)) {
    const k = /^\d+$/.test(id) ? keptFollow(f) : null;
    if (k) out[id] = k;
    else bad = true;
  }
  return { out, bad };
}

async function loadFollows() {
  if (follows) return follows;
  const got = await Promise.resolve(api.storage.local.get({ follows: {} })).catch(() => ({ follows: {} }));
  const priv = (await Promise.resolve(api.storage.session?.get("privateFollows")).catch(() => null))?.privateFollows;
  // What was kept is checked: anything another version (or damage) left there is dropped and
  // cleared, never trusted ("busy" never stays either: a report cut short by the browser).
  const plain = keptFollows(got?.follows);
  const hidden = keptFollows(priv);
  for (const f of Object.values(hidden.out)) f.private = true;
  follows ??= { ...plain.out, ...hidden.out };
  if (plain.bad || hidden.bad) saveFollows();
  return follows;
}
function saveFollows() {
  const plain = {};
  const priv = {};
  for (const [id, f] of Object.entries(follows || {})) {
    if (!f || typeof f !== "object") continue;
    const { busy: _busy, ...rest } = f;
    (f.private ? priv : plain)[id] = rest;
  }
  api.storage.local.set({ follows: plain }).catch(() => {});
  Promise.resolve(api.storage.session?.set({ privateFollows: priv })).catch(() => {});
}

/** Starts following a browser download (`pako`: Pako's row for it, "" = not known yet). */
async function follow(browserId, pako, url, name, isPrivate) {
  await loadFollows();
  follows[browserId] = { pako: pako || "", url, name: name || "", private: !!isPrivate, sent: 0, state: "" };
  saveFollows();
  await reportFollow(browserId, true);
  tickFollows();
}

/** One report to Pako; its answer may say what to do with the browser's download. */
async function reportFollow(browserId, now = false) {
  const f = (await loadFollows())[browserId];
  if (!f || f.busy) return;
  const [item] = await api.downloads.search({ id: Number(browserId) }).catch(() => []);
  // Gone from the browser, or the browser gave its number to another download since (its list
  // was cleared): never reported as this one.
  const other = item && typeof item.url === "string" && item.url && item.url !== f.url && item.finalUrl !== f.url;
  if (!item || other) {
    delete follows[browserId];
    saveFollows();
    return;
  }
  let state = item.state === "in_progress" && item.paused ? "paused" : item.state;
  if (!["in_progress", "paused", "complete", "interrupted"].includes(state)) state = "in_progress";
  // Changed since the last report Pako took, or (when that last one failed) since the one that
  // failed: a failed report never counts as a change again every second (Pako closed: one try
  // a minute, not a Pako.exe helper started every second).
  const changed = state !== (f.tried ?? f.state);
  const every = state === "in_progress" || state === "complete" ? FOLLOW_EVERY : FOLLOW_IDLE_EVERY;
  if (!now && !changed && Date.now() - f.sent < every) return;
  if (!changed && Date.now() < followQuietUntil) return;
  f.busy = true;
  const reply = await send({
    kind: "follow",
    urls: [f.url],
    filename: f.name,
    private: f.private,
    follow: {
      pako: f.pako,
      browserId: Number(browserId),
      path: item.filename || "",
      bytes: item.bytesReceived > 0 ? item.bytesReceived : 0,
      total: item.totalBytes > 0 ? item.totalBytes : 0,
      state,
      error: state === "interrupted" ? String(item.error || "") : "",
      canResume: !!item.canResume,
    },
  }).finally(() => (f.busy = false));
  f.sent = Date.now();
  if (!reply.ok) {
    // Pako closed or busy: kept, sent again later (the finished file is listed then).
    if (reply.error && !reply.followed) followQuietUntil = Date.now() + 60_000;
    f.tried = state;
    saveFollows();
    return;
  }
  followQuietUntil = 0;
  delete f.tried;
  f.state = state;
  if (typeof reply.followed === "string") f.pako = reply.followed;
  if (reply.command === "pause") await api.downloads.pause(item.id).catch(() => {});
  else if (reply.command === "resume") await api.downloads.resume(item.id).catch(() => {});
  else if (reply.command === "cancel") {
    await api.downloads.cancel(item.id).catch(() => {});
    await api.downloads.erase({ id: item.id }).catch(() => {});
    delete follows[browserId];
  }
  if (state === "complete") {
    // Pako moved it to your folder: the browser's list forgets it (the file stays). Pako
    // couldn't ("keep"): the browser's list keeps it, where it is. No word yet: still moving.
    if (reply.command === "forget") await api.downloads.erase({ id: item.id }).catch(() => {});
    if (reply.command === "forget" || reply.command === "keep") delete follows[browserId];
  } else if (state === "interrupted" && !item.canResume) {
    // Stopped for good (Try again in Pako can't continue it): no more reports.
    delete follows[browserId];
  }
  saveFollows();
}

/** Every few seconds while the browser downloads something for Pako. */
async function tickFollows() {
  if (followTimer) return;
  const ids = Object.keys(await loadFollows());
  if (!ids.length) return;
  followTimer = setTimeout(async () => {
    followTimer = null;
    for (const id of Object.keys(follows)) await reportFollow(id);
    tickFollows();
  }, 1000);
  // (Never keeps a test run open.)
  followTimer?.unref?.();
}

/** Pako answered again: what waited for it goes now. */
function followsAgain() {
  if (!followQuietUntil) return;
  followQuietUntil = 0;
  tickFollows();
}

api.downloads.onChanged?.addListener((delta) => {
  if (!follows || !follows[delta.id]) return;
  // A state change (finished, paused, stopped) is said at once; progress about once a second.
  reportFollow(delta.id, !!(delta.state || delta.paused || delta.error));
});

// After the browser restarts the extension: what it was following. (Never in the way of the
// listeners above: a failure here only means nothing is followed.)
loadFollows()
  .then(() => tickFollows())
  .catch((e) => console.warn("Pako: couldn't pick up the downloads it was following:", e?.message || e));

// ---------- private windows ----------
// The browser's own switch for Pako ("Allow in Incognito", "Allow in InPrivate", Firefox's "Run
// in Private Windows") decides: with it on, a private window's downloads go to Pako like any
// other, with that window's own cookies, and Pako keeps no trace of them beyond the file.

/** Chromium's private cookie store (Firefox names its own on each download). */
const PRIVATE_STORE = "1";

async function privateAllowed() {
  try {
    return !!(await api.extension?.isAllowedIncognitoAccess?.());
  } catch {
    return false;
  }
}

/** Saves what the extension learnt. What it learnt in this run is kept in memory too, and wins
 *  over what was saved: a save the browser refuses only means learning it again after the
 *  background page restarts. Said once in the service worker's console. */
let saveFailed = false;
function saveState(values) {
  return Promise.resolve()
    .then(() => api.storage.local.set(values))
    .catch((e) => {
      if (!saveFailed) console.warn("Pako: the browser didn't save the extension's settings:", e?.message || e);
      saveFailed = true;
    });
}

/** Does this Pako answer takeovers at once? Learnt from its answers (see `send`). */
let holdsNow = null;
async function pakoHolds() {
  if (holdsNow === null) holdsNow = (await api.storage.local.get({ pakoHolds: false }).catch(() => ({ pakoHolds: false }))).pakoHolds === true;
  return holdsNow;
}
function learnHolds(reply) {
  const v = reply.holds === true;
  if (v === holdsNow) return;
  holdsNow = v;
  saveState({ pakoHolds: v });
  /* full-only:start */
  syncCatch();
  /* full-only:end */
}

// Chromium: taken at the browser's file-name step, before any "Save as" window and before the
// file is written. Firefox has no such step: there it is taken as soon as the download starts.
const namingStep = api.downloads.onDeterminingFilename;
if (namingStep) {
  namingStep.addListener((item, suggest) => {
    takeOver(item, () => suggest());
    return true;
  });
}
api.downloads.onCreated.addListener((item) => {
  if (!namingStep) takeOver(item);
});

/* full-only:start */
// ---------- videos playing on the page ----------
// The browser itself loads a page's video: its HLS (.m3u8) or DASH (.mpd) playlist, or a plain
// video file. The extension only looks at those requests (it never changes or blocks them),
// keeps a short list per tab in memory (gone when the tab goes to another page or closes), and
// shows how many on the toolbar button. Nothing goes to Pako until you click "Download with Pako".

/** Videos kept per tab. */
const CAUGHT_MAX = 20;
/** Plain video files smaller than this are previews, adverts or pieces: left out. */
const MIN_VIDEO_FILE = 1024 * 1024;
/** What the page's player loads piece by piece (never a whole video on its own). */
const PIECE = /\.(ts|m4s|m4f|cmfv|cmfa|aac|vtt|webvtt|key)$/;

/** What kind of video a request is: "hls", "dash", "file", or null (anything else). `type` = the
 *  browser's request type, `contentType` and `size` from the answer's headers. */
function mediaKind(url, type, contentType, size) {
  let path;
  try {
    const u = new URL(url);
    if (u.protocol !== "http:" && u.protocol !== "https:") return null;
    path = u.pathname.toLowerCase();
  } catch {
    return null;
  }
  const ct = String(contentType || "").split(";")[0].trim().toLowerCase();
  if (path.endsWith(".m3u8") || /^(application|audio)\/(vnd\.apple\.mpegurl|x-mpegurl|mpegurl)$/.test(ct)) return "hls";
  if (path.endsWith(".mpd") || ct === "application/dash+xml") return "dash";
  if (PIECE.test(path) || ct === "video/mp2t" || ct === "video/iso.segment") return null;
  // A whole file is what the page's video element itself loads (a player that builds the video
  // from pieces fetches them another way).
  const named = /\.(mp4|m4v|webm|mov|mkv|ogv)$/.test(path);
  const isVideo = ct.startsWith("video/") || (named && (ct === "" || ct === "application/octet-stream"));
  if (type === "media" && isVideo && size >= MIN_VIDEO_FILE) return "file";
  return null;
}

/** The whole file's size from an answer's headers (a part's answer says it after the "/"). */
function sizeOf(headers) {
  const get = (n) => headers?.find((h) => h.name.toLowerCase() === n)?.value || "";
  const total = /\/(\d+)\s*$/.exec(get("content-range"));
  return total ? Number(total[1]) : Number(get("content-length")) || 0;
}

/** Reads a caught HLS playlist again from its server: a master playlist says its qualities and
 *  which playlists belong to it. `login`: with the browser's cookies for that server (a normal
 *  window's own); without them (a Firefox container tab: the background page only has the
 *  default container's), nothing is sent or kept but the request. Never for a private window. */
async function readPlaylist(url, login) {
  try {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), 5000);
    const how = login ? { cache: "force-cache", credentials: "include" } : { cache: "no-store", credentials: "omit" };
    const r = await fetch(url, { ...how, signal: ctrl.signal });
    const text = r.ok ? (await r.text()).slice(0, 2 * 1024 * 1024) : "";
    clearTimeout(timer);
    if (!text.replace(/^﻿/, "").startsWith("#EXTM3U")) return null;
    if (!text.includes("#EXT-X-STREAM-INF")) return { master: false, parts: [], height: 0 };
    const parts = [];
    let height = 0;
    const lines = text.split(/\r?\n/).map((l) => l.trim());
    lines.forEach((l, i) => {
      const uri = /URI="([^"]+)"/.exec(l);
      if (l.startsWith("#EXT-X-MEDIA:") && uri) parts.push(new URL(uri[1], url).href);
      if (l.startsWith("#EXT-X-STREAM-INF:")) {
        const res = /RESOLUTION=\d+x(\d+)/i.exec(l);
        if (res) height = Math.max(height, Number(res[1]));
        const next = lines.slice(i + 1).find((x) => x && !x.startsWith("#"));
        if (next) parts.push(new URL(next, url).href);
      }
    });
    return { master: true, parts, height };
  } catch {
    return null;
  }
}

/** One tab's list, kept in the browser's memory-only session storage (it outlives the
 *  background page going to sleep, never the browser). */
const caughtKey = (tabId) => `caught-${tabId}`;
async function caughtList(tabId) {
  let kept;
  try {
    kept = (await api.storage.session.get(caughtKey(tabId)))[caughtKey(tabId)];
  } catch {
    return [];
  }
  if (kept == null) return [];
  // Only what this version keeps there (anything else is dropped, and cleared).
  const good = (v) => v && typeof v === "object" && typeof v.url === "string" && isWebLink(v.url) && typeof v.kind === "string";
  const list = Array.isArray(kept) ? kept.filter(good).map((v) => ({ ...v, parts: Array.isArray(v.parts) ? v.parts.filter((p) => typeof p === "string") : [] })) : [];
  if (!Array.isArray(kept) || list.length !== kept.length) Promise.resolve(api.storage.session.remove(caughtKey(tabId))).catch(() => {});
  return list;
}

/** What the popup lists: a master playlist stands for the playlists it names. */
function visible(list) {
  const covered = new Set(list.flatMap((v) => v.parts || []));
  return list.filter((v) => !covered.has(v.url));
}

async function saveCaught(tabId, list) {
  try {
    if (list.length) await api.storage.session.set({ [caughtKey(tabId)]: list });
    else await api.storage.session.remove(caughtKey(tabId));
  } catch {
    // the tab's list can't be kept: the toolbar button still says it
  }
  const n = visible(list).length;
  try {
    await api.action.setBadgeBackgroundColor({ tabId, color: "#F96263" });
    await api.action.setBadgeText({ tabId, text: n ? String(n) : "" });
  } catch {
    // the tab closed meanwhile
  }
}

/** One tab's list changes one step at a time (requests come in together). */
const caughtWork = new Map();
function changeCaught(tabId, change) {
  const next = (caughtWork.get(tabId) ?? Promise.resolve()).then(async () => {
    const changed = await change(await caughtList(tabId));
    if (changed) await saveCaught(tabId, changed);
  });
  caughtWork.set(tabId, next.catch(() => {}));
  return next;
}

// ---------- what the browser's server said about a download ----------
// A page or frame answer that is a file (Content-Disposition: attachment, or a kind of file the
// browser doesn't show): kept a short while, so a takeover knows how the browser asked (POST
// can't be asked again) and what the server said (size, kind, name). Only read, never changed.

/** By address: { at, method, size, type, filename }. */
const responses = new Map();
const RESPONSE_KEEP = 2 * 60 * 1000;
/** What the browser shows itself (a page, a picture...), not a download. */
const SHOWN = /^(text\/|image\/|video\/|audio\/|multipart\/|application\/(xhtml\+xml|xml|json|javascript|pdf|x-javascript)$)/;

/** The name in a Content-Disposition header ("filename*=UTF-8''…" first, then "filename=…"). */
function dispositionName(cd) {
  const star = /filename\*\s*=\s*(?:[\w-]+)?'[^']*'([^;]+)/i.exec(cd);
  if (star) {
    try {
      return baseName(decodeURIComponent(star[1].trim().replace(/^"|"$/g, "")));
    } catch {
      // badly written: the plain name below
    }
  }
  const plainName = /filename\s*=\s*("([^"]*)"|[^;]+)/i.exec(cd);
  return plainName ? baseName((plainName[2] ?? plainName[1]).trim()) : "";
}

function noteDownloadResponse(details) {
  const headers = details.responseHeaders || [];
  const get = (n) => headers.find((h) => h.name.toLowerCase() === n)?.value || "";
  const cd = get("content-disposition");
  const type = get("content-type").split(";")[0].trim().toLowerCase();
  const attachment = /^\s*attachment/i.test(cd);
  if (!attachment && (!type || SHOWN.test(type))) return false;
  const now = Date.now();
  for (const [u, r] of responses) if (now - r.at > RESPONSE_KEEP || responses.size > 50) responses.delete(u);
  const size = parseInt(get("content-length"), 10);
  const asked = requests.get(details.url);
  responses.set(details.url, {
    at: now,
    method: String(details.method || "GET").toUpperCase(),
    size: size > 0 ? size : 0,
    type,
    filename: dispositionName(cd),
    // Asked with the site's own sign-in (see notedRequest).
    auth: !!asked && now - asked.at <= RESPONSE_KEEP && asked.auth,
    // Its tab as it is now (a tab opened only for a download is closed soon after), in case
    // it is caught at this answer (see handCaught).
    tab: typeof details.tabId === "number" && details.tabId >= 0 ? Promise.resolve(api.tabs.get(details.tabId)).catch(() => null) : null,
    initiator: typeof details.initiator === "string" ? details.initiator : "",
  });
  return true;
}

/** Requests that may become downloads, by address: whether the browser sent a login other than
 *  cookies with it (Authorization: a site's own sign-in box, the only one a page's link or form
 *  carries). Pako can't send that, so such a download stays with the browser (paused) until
 *  Pako has it. Only that yes/no is kept, a short while, never the header itself. */
const requests = new Map();
function notedRequest(details) {
  const auth = (details.requestHeaders || []).some((h) => String(h?.name || "").toLowerCase() === "authorization");
  const now = Date.now();
  for (const [u, r] of requests) if (now - r.at > RESPONSE_KEEP || requests.size > 100) requests.delete(u);
  if (auth || requests.has(details.url)) requests.set(details.url, { at: now, auth });
}

function responseFor(url) {
  const r = responses.get(url);
  return r && Date.now() - r.at <= RESPONSE_KEEP ? r : null;
}

/** A request the browser made in a tab: a video is noted, a new page clears the list. */
function onMediaHeaders(details) {
  const { tabId, url, type } = details;
  // ("other": a link with the download attribute, and what the browser fetches by itself.)
  const isFile = (type === "main_frame" || type === "sub_frame" || type === "other") && noteDownloadResponse(details);
  if (type === "sub_frame" || type === "other") return;
  if (typeof tabId !== "number" || tabId < 0) return;
  if (type === "main_frame") {
    // A one-time exception for a private tab's file (see againInTab) has done its work once its
    // address answers for good. A redirect isn't that: the browser asks again (often the same
    // address, after a cookie) and checks the rules anew, likely before the exception would be
    // back (seen in Vivaldi 8.3: the file was then caught again and lost).
    if (finalAnswer(details.statusCode)) dropAllowFor(tabId, url);
    // How the tab's page was asked for (a file's answer, a redirect or a "204 No Content"
    // doesn't replace the page): a form's answer isn't gone back to by the sent page (see
    // sentPageAnswer).
    if (!isFile && loadsPage(details.statusCode)) notePageMethod(tabId, String(details.method || "GET").toUpperCase(), details.requestId);
    changeCaught(tabId, async (list) => (list.length ? [] : null));
    // The new page gets the pill again once a video is caught on it.
    forgetPill(tabId);
    return;
  }
  if (details.statusCode >= 400) return;
  const headers = details.responseHeaders || [];
  const contentType = headers.find((h) => h.name.toLowerCase() === "content-type")?.value || "";
  const kind = mediaKind(url, type, contentType, sizeOf(headers));
  if (!kind || url.length > MAX_LINK) return;
  // The document that loaded it (an embedded player's frame): the referrer the browser sent.
  const doc = details.documentUrl || details.initiator || "";
  let added = false;
  changeCaught(tabId, async (list) => {
    if (list.some((v) => v.url === url) || list.length >= CAUGHT_MAX) return null;
    const item = { url, kind, doc: isWebLink(doc) ? doc : "", height: 0, parts: [] };
    if (kind === "hls") {
      // A private window's playlist is never asked for again: the background page would ask with
      // the normal windows' cookies and keep the answer in their cache. (Firefox says it in the
      // request; Chromium only in the tab. Not known = private.)
      const incognito = typeof details.incognito === "boolean" ? details.incognito : ((await api.tabs.get(tabId).catch(() => null))?.incognito ?? true);
      if (!incognito) {
        const store = details.cookieStoreId;
        const p = await readPlaylist(url, !store || store === "firefox-default");
        if (p?.master) Object.assign(item, { height: p.height, parts: p.parts });
      }
    }
    added = true;
    return [...list, item];
  }).then(() => {
    // The pill goes on the video in the frame that loaded it (that frame's page: Chromium names
    // its document, Firefox gives its address).
    if (added) offerPill(tabId, details.frameId, details.documentId || details.documentUrl || "");
  }, () => {});
}

api.webRequest?.onHeadersReceived?.addListener(onMediaHeaders, { urls: ["<all_urls>"], types: ["main_frame", "sub_frame", "media", "xmlhttprequest", "other"] }, ["responseHeaders"]);
// A page's load stopped before it showed (see pageLoadFailed).
api.webRequest?.onErrorOccurred?.addListener(pageLoadFailed, { urls: ["<all_urls>"], types: ["main_frame"] });
// Only whether a login header went with it (Authorization is given without "extraHeaders").
api.webRequest?.onSendHeaders?.addListener(notedRequest, { urls: ["<all_urls>"], types: ["main_frame", "sub_frame", "other"] }, ["requestHeaders"]);

// ---------- catching a download at the server's answer (browsers that ask first) ----------
// Some Chromium browsers ask "Download …? Save / Save As / Open / Cancel" before any extension
// hears of the download (Vivaldi does, unless "Save Files to Default Location Without Asking" is
// on), so the file-name step above comes after that question: you'd answer twice. In those
// browsers, a rule of the extension's own in the browser (declarativeNetRequest, on the server's
// answer: Chrome 128 and later) turns a page's file answer away before it becomes a download,
// so the browser asks nothing and starts nothing. The tab shows Pako's blank "sent" page for a
// moment, which goes straight back to your page (or closes the tab opened only for the file), and
// the extension hands the address to Pako like a takeover, with what the server said (noted
// above). Other browsers keep the file-name step: it already comes before any question there, and
// it leaves the page alone.
// The rule is there only while the takeover switch is on, Pako's helper answers and answers
// takeovers at once ("holds"), this browser asks first, and the browser takes the rule. It never
// catches a form's answer (POST), a private window's tab, the video sites, a file under 2 MB (by
// its Content-Length), nor what the extension starts itself (a download request, not a page's).
// Only the extension sees what it caught: it follows the browser's own turn-away (onBeforeRedirect
// to the sent page), never a guess.
// What it can't hand to Pako goes back to the browser: a site's own sign-in (Authorization: the
// browser's rules decide before the extension hears of that header, so it is caught, then started
// again in the browser), and a private window's tab opened in the moment before the rule could
// leave it out (sent back to that tab, where the browser downloads it).
// The sent page asks the extension before it goes back: it stays (and says so) when going back
// would reload a form's answer, or when the same file comes again in that tab at once (a page that
// starts its download by itself, reloaded by going back: never handed twice, never a loop).

/** Session rule IDs: the catch rule, and a ring of one-time exceptions (see againInTab). */
const CATCH_RULE = 1;
const ALLOW_RULES = 1000;
const ALLOW_RING = 20;
/** A one-time exception the tab didn't use within this goes (the tab asks again at once). */
const ALLOW_FOR = 60 * 1000;
/** The page a caught answer is turned into (web-accessible, at an address that changes). */
const SENT_PAGE = "/pako-sent.html";
/** Longer than this between the server's answer and the file-name step: the browser asked first. */
const ASKED_FIRST = 1000;
/** Seen asking first this many times in a row: this browser asks first (once may be Chrome's own
 *  "download multiple files?" question, or a slow start of the file). */
const ASKED_TWICE = 2;
/** Pako's helper didn't answer: asked again after this, until it does. */
const AWAY_RETRY = 60 * 1000;
/** How long a tab's sent page waits to be told what to say (see sentPageAnswer). */
const CAUGHT_AGAIN = 30 * 1000;
/** The same file caught again in the same tab within this, while Pako has (or is getting) the
 *  first one: the page asked for it again by itself (back on a "your download starts..." page).
 *  Later, or when Pako didn't take the first one, it is handed again (Pako itself still answers
 *  a repeat of one it is checking with the same answer). */
const CAUGHT_REPEAT = 8 * 1000;
/** Catches from one tab within this many ms are counted (see CATCH_MAX). */
const CATCH_BURST = 10 * 1000;
/** At most this many catches per tab in CATCH_BURST, and only one of them from a frame (a page
 *  pushing files by itself through hidden frames): it never gets around the browser's own
 *  "download multiple files?" question or its question before downloads. Past it, nothing more
 *  is handed to Pako from that tab, and the tab isn't caught for CATCH_QUIET, so the browser
 *  asks as usual. (Clicking a few files one after another on a page stays well under it.) */
const CATCH_MAX = 4;
const CATCH_QUIET = 60 * 1000;
/** Kinds of file no browser shows itself: a page's answer of one of them is a download. */
const DOWNLOAD_TYPES = [
  "application/octet-stream", "application/zip", "application/x-zip-compressed", "application/x-7z-compressed",
  "application/x-rar-compressed", "application/vnd.rar", "application/x-tar", "application/gzip", "application/x-gzip",
  "application/x-bzip2", "application/x-xz", "application/zstd", "application/x-msdownload", "application/x-msi",
  "application/vnd.microsoft.portable-executable", "application/x-iso9660-image", "application/x-apple-diskimage",
  "application/vnd.android.package-archive", "application/vnd.ms-cab-compressed", "application/x-debian-package",
  "application/x-rpm",
];

/** The browser refused the rule (older than Chrome 128): the file-name step, for this run. */
let catchRefused = false;
/** The rule last given to the browser (null: not set since this background page started). */
let catchApplied = null;
let catchWork = Promise.resolve();
let pakoAwayNow = null;
let awayTimer = null;
let allowNext = 0;
/** The one-time exceptions in the browser now: rule ID → { tabId, url, timer }. */
const allowRules = new Map();
/** "tab address" → { at, took } for the last catch of that file there (see CAUGHT_REPEAT);
 *  `took`: Pako took it (true), didn't (false), not known yet (undefined). */
const caughtLately = new Map();
/** By tab: when it was caught lately ({ at, frame }), for CATCH_MAX. */
const catchCounts = new Map();
/** By tab: not caught until then (CATCH_QUIET), so the browser asks as usual. */
const quietTabs = new Map();
/** By tab: what its sent page is told (see sentPageAnswer). */
const sentPages = new Map();
/** By tab: how its page was asked for ("POST": a form's answer, which going back would send again). */
const pageMethods = new Map();
/** By tab: the page last noted as loading in it, and whether the one before it was a form's
 *  answer (see pageLoadFailed). */
const pageLoads = new Map();
const REDIRECTS = [301, 302, 303, 307, 308];
/** An answer that replaces the tab's page: not a redirect, not "204 No Content" / "205 Reset
 *  Content" (the browser keeps the page), not an early "1xx". (An error status shows the
 *  server's error page; a "304" the browser's copy.) */
function loadsPage(status) {
  if (typeof status !== "number") return true;
  return status >= 200 && status !== 204 && status !== 205 && !REDIRECTS.includes(status);
}
/** The address's last answer for this request: not a redirect, nor a sign-in question (401, 407:
 *  the browser may ask again with the sign-in). */
function finalAnswer(status) {
  return typeof status !== "number" || (status >= 200 && !REDIRECTS.includes(status) && status !== 401 && status !== 407);
}
/** A form's answer is also kept in the browser's memory (storage.session): the background
 *  sleeps after 30 s without events, and you often read a page longer than that before you
 *  click its download. Only a form's answer is written, so a plain page costs nothing. */
const postKey = (tabId) => `post-${tabId}`;
function notePageMethod(tabId, method, requestId) {
  // (Read before it changes; from the browser's memory after a sleep.)
  const before = method === "POST" ? null : pageIsPost(tabId);
  if (before && requestId != null) pageLoads.set(tabId, { requestId, before });
  else pageLoads.delete(tabId);
  const was = pageMethods.get(tabId);
  pageMethods.set(tabId, method);
  if (method === "POST") Promise.resolve(api.storage.session?.set({ [postKey(tabId)]: true })).catch(() => {});
  else if (was !== "GET") Promise.resolve(api.storage.session?.remove(postKey(tabId))).catch(() => {});
}
/** Whether the tab's page is a form's answer (asked of the browser's memory after a sleep). */
async function pageIsPost(tabId) {
  if (pageMethods.has(tabId)) return pageMethods.get(tabId) === "POST";
  const got = await Promise.resolve(api.storage.session?.get(postKey(tabId))).catch(() => null);
  return got?.[postKey(tabId)] === true;
}
/** A page's load failed or was stopped after its answer came (Esc, another click): it may never
 *  have replaced the page. If the page before was a form's answer, it still counts as one (going
 *  back would send it again); staying on the sent page is the safe side if it did show. */
function pageLoadFailed(details) {
  const l = pageLoads.get(details?.tabId);
  if (!l || l.requestId !== details.requestId) return;
  pageLoads.delete(details.tabId);
  l.before.then(
    (post) => {
      if (post && !pageLoads.has(details.tabId) && pageMethods.get(details.tabId) !== "POST") notePageMethod(details.tabId, "POST");
    },
    () => {},
  );
}
/** What says it's the same file again: its name and size (a link signed anew each time, or
 *  from another mirror, is another address for the same file), else its address. */
function sameFileKeys(tabId, url, seen) {
  const keys = [`${tabId} ${url}`];
  let name = seen?.filename || "";
  if (!name) {
    try {
      name = decodeURIComponent(new URL(url).pathname.split("/").pop() || "");
    } catch {
      name = "";
    }
  }
  if (name && seen?.size > 0) keys.push(`${tabId} file ${name} ${seen.size}`);
  return keys;
}

/** The catch rule. `privateTabs`: a private window's tabs (never caught: its download can't be
 *  started again there if Pako can't get it).
 *  Frames are caught too (the usual hidden download frame). Known limit: a page script that
 *  watches its hidden download frame and reads it (jQuery fileDownload's way) can't read the
 *  sent page there (another origin), and may then say the download failed though Pako has it,
 *  unless the server's "done" cookie came with the answer. The rule can't tell a hidden frame
 *  from a shown one, a turned-away answer can only become another page (never "nothing"), and
 *  leaving frames out would bring the browser's question back for every frame download. */
function catchRule(privateTabs) {
  return {
    id: CATCH_RULE,
    priority: 1,
    action: { type: "redirect", redirect: { extensionPath: SENT_PAGE } },
    condition: {
      resourceTypes: ["main_frame", "sub_frame"],
      requestMethods: ["get"],
      excludedRequestDomains: LIGHT_SKIPS_SITES,
      excludedInitiatorDomains: LIGHT_SKIPS_SITES,
      ...(privateTabs.length ? { excludedTabIds: privateTabs } : {}),
      // A file: "Content-Disposition: attachment", or a kind of file the browser never shows.
      responseHeaders: [
        { header: "content-disposition", values: ["attachment*"] },
        { header: "content-type", values: DOWNLOAD_TYPES.flatMap((t) => [t, `${t};*`]) },
      ],
      // Not a small file (under about 2 MB: "?" is zero or one character, so up to 6 digits,
      // 1 and up to 6 more, 20 and up to 5 more): it stays in the browser.
      excludedResponseHeaders: [{ header: "content-length", values: ["??????", "1??????", "20?????"] }],
    },
  };
}

/** Puts the catch rule in the browser, or takes it away, as things are now. One at a time. */
function syncCatch() {
  catchWork = catchWork.then(syncCatchNow, syncCatchNow);
  return catchWork;
}
/** Whether downloads are caught now, as things are: `asks` (this browser asks first), `on` (the
 *  catch is on: it asks first, the takeover switch is on, Pako's helper answers and answers
 *  takeovers at once), `links` (download links on pages are caught too, see catchLinks), and the
 *  browser's tabs. Null when the extension's storage can't be read. */
async function catchState() {
  const s = await Promise.resolve(api.storage.local.get({ takeOver: true, pakoHolds: false, pakoAway: false, asksFirst: false, toldAsks: false, catchLinks: true })).catch(() => null);
  if (!s) return null;
  const tabs = (await Promise.resolve(api.tabs.query({})).catch(() => [])) || [];
  // Pako said so (it knows its browser by its program: `asksFirst` in its answers; kept, so a
  // background page started again keeps the rule before it next talks to Pako), seen asking
  // first (noteAsking), or Vivaldi's own tab data (older Vivaldi versions).
  const asks = toldAsks || s.toldAsks === true || (asksLearnt ?? s.asksFirst === true) || tabs.some((t) => t && typeof t === "object" && "vivExtData" in t);
  // What this run learnt wins over what was saved (a save the browser refused never lands).
  const away = pakoAwayNow ?? s.pakoAway === true;
  const holds = holdsNow ?? s.pakoHolds === true;
  const on = asks && s.takeOver !== false && holds && !away;
  return { asks, away, on, links: on && s.catchLinks !== false, tabs };
}

async function syncCatchNow() {
  const dnr = api.declarativeNetRequest;
  // Firefox keeps its own way (no question comes before its extensions' downloads.onCreated).
  if (!dnr?.updateSessionRules || FIREFOX) return;
  const state = await catchState();
  if (!state) return;
  const { tabs, away } = state;
  if (away) pingLater();
  // Download links on pages (no rule needed for them: also in a browser that refused the rule).
  await syncLinks(state.links);
  const want = !catchRefused && state.on;
  const now = Date.now();
  for (const [id, until] of quietTabs) if (until <= now) quietTabs.delete(id);
  const left = [...tabs.filter((t) => t?.incognito && Number.isInteger(t.id)).map((t) => t.id), ...quietTabs.keys()];
  const rule = want ? catchRule([...new Set(left)]) : null;
  const key = rule ? JSON.stringify(rule) : "";
  if (key === catchApplied) return;
  try {
    await dnr.updateSessionRules({ removeRuleIds: [CATCH_RULE], addRules: rule ? [rule] : [] });
    catchApplied = key;
  } catch (e) {
    // A browser that doesn't know rules on the server's answer refuses it (some throw at once):
    // downloads keep being taken over at the file-name step.
    if (rule) {
      catchRefused = true;
      console.warn("Pako: this browser can't catch downloads at the server's answer:", e?.message || e);
    }
  }
}

/** At the file-name step: whether the browser told the extension long after the server answered
 *  (`slow`: it asked you first). Learnt after ASKED_TWICE in a row, unlearnt by one told at once,
 *  so a single slow one (Chrome's own "download multiple files?" question, a file slow to start)
 *  never turns the catch on in a browser that doesn't ask. */
let askingWork = Promise.resolve();
/** What noteAsking learnt in this run (used when the browser refuses saves); null: nothing yet. */
let askingNow = { asksFirst: false, askedSlow: 0 };
let asksLearnt = null;
/** Pako said this browser asks before its extensions see a download (`asksFirst`). Saved too: a
 *  browser is the same browser for good, and the background page is started again often. */
let toldAsks = false;
function learnAsks(reply) {
  if (reply.asksFirst !== true || toldAsks) return;
  toldAsks = true;
  saveState({ toldAsks: true });
  syncCatch();
}
function noteAsking(slow) {
  // One at a time: two downloads told at once would each read the count before the other wrote it.
  askingWork = askingWork.then(
    () => noteAskingNow(slow),
    () => noteAskingNow(slow),
  );
  return askingWork;
}
async function noteAskingNow(slow) {
  const stored = await Promise.resolve(api.storage.local.get({ asksFirst: false, askedSlow: 0 })).catch(() => null);
  // A browser that refuses saves: counted in memory for this run.
  const got = saveFailed || !stored ? askingNow : stored;
  const was = got.asksFirst === true;
  const n = slow ? Math.min((Number(got.askedSlow) || 0) + 1, ASKED_TWICE) : 0;
  const asks = slow && (was || n >= ASKED_TWICE);
  if (n === got.askedSlow && asks === was) return;
  askingNow = { asksFirst: asks, askedSlow: n };
  asksLearnt = asks;
  await saveState({ asksFirst: asks, askedSlow: n });
  if (asks !== was) syncCatch();
}

/** Pako's helper answered (`true`), or a handoff didn't reach Pako (`false`): the catch rule goes
 *  at once, so nothing is caught that Pako can't take, and comes back when it answers again. */
function pakoAnswered(yes) {
  if (yes) {
    clearTimeout(awayTimer);
    awayTimer = null;
  } else pingLater();
  if (pakoAwayNow === !yes) return;
  pakoAwayNow = !yes;
  saveState({ pakoAway: !yes }).then(() => syncCatch());
}
function pingLater() {
  if (awayTimer) return;
  awayTimer = setTimeout(() => {
    awayTimer = null;
    send({ kind: "ping" });
  }, AWAY_RETRY);
  awayTimer?.unref?.();
}

/** The page the extension turns a caught answer into. */
function isSentPage(url) {
  try {
    const u = new URL(url);
    return u.protocol === "chrome-extension:" && u.pathname === SENT_PAGE;
  } catch {
    return false;
  }
}

/** The page the download was clicked on: the tab's page (still the old one while the browser
 *  waits for the answer), else the site that asked. */
function catchReferrer(tab, seen, url) {
  if (tab && isWebLink(tab.url) && tab.url !== url) return tab.url;
  return isWebLink(seen?.initiator) ? `${seen.initiator}/` : "";
}

/** The browser turned a file answer away (the catch rule). Noted at once (before anything else
 *  can run): the tab as it is, what its sent page will be told, and whether this tab just had
 *  the same file (then it isn't handed again). */
function onCaught(details) {
  if (!isSentPage(details.redirectUrl)) return;
  const now = Date.now();
  for (const [k, c] of caughtLately) if (now - c.at > CAUGHT_REPEAT || caughtLately.size > 50) caughtLately.delete(k);
  const seen = responseFor(details.url);
  const keys = sameFileKeys(details.tabId, details.url, seen);
  // The page asked for the same file again by itself, a moment after Pako took it (or while it
  // still gets it): not handed twice. Pako didn't take it, or it's later: handed again (a new
  // click, or the first one was cancelled in Pako).
  const again = keys.some((k) => caughtLately.has(k) && caughtLately.get(k).took !== false);
  // (Asked now, while the tab is surely there: its sent page waits for this before it closes it.)
  const tab = Promise.resolve(seen?.tab || (details.tabId >= 0 ? api.tabs.get(details.tabId) : null)).catch(() => null);
  const frame = details.type !== "main_frame";
  let over = false;
  if (!again) {
    const counts = (catchCounts.get(details.tabId) || []).filter((c) => now - c.at < CATCH_BURST);
    over = counts.length >= CATCH_MAX || (frame && counts.some((c) => c.frame));
    counts.push({ at: now, frame });
    catchCounts.set(details.tabId, counts);
    if (catchCounts.size > 50) catchCounts.delete(catchCounts.keys().next().value);
  }
  let handed = null;
  if (over) {
    // A page pushing files by itself: this one isn't handed to Pako, and the tab isn't caught
    // for a while, so the browser asks as usual. The "!" says so (never a silent loss).
    if (details.tabId >= 0) {
      quietTabs.set(details.tabId, now + CATCH_QUIET);
      syncCatch();
      const t = setTimeout(syncCatch, CATCH_QUIET + 50);
      t?.unref?.();
    }
    const name = seen?.filename || "";
    const said = name
      ? tr('This page asked for several downloads at once, so Pako took only the first ones. To download "{name}", use the page again.', { name })
      : tr("This page asked for several downloads at once, so Pako took only the first ones. To download the others, use the page again.");
    flag(said, "", false);
  } else if (!again) {
    const entry = { at: now, took: undefined };
    for (const k of keys) caughtLately.set(k, entry);
    // Counted from now (the browser's copy is already gone): no reload for an update meanwhile.
    inFlight++;
    handed = handCaught(details, seen, tab)
      .then((v) => {
        entry.took = v === "pako";
        return v;
      })
      .catch((e) => {
        entry.took = false;
        console.warn("Pako:", e?.message || e);
      })
      .finally(() => inFlight--);
  }
  if (details.type === "main_frame" && details.tabId >= 0) {
    const said = over ? tr("This page asked for several downloads at once, so Pako didn't take this one. Use Back, then download it again.") : "";
    sentPages.set(details.tabId, { tab, stay: over || again || pageIsPost(details.tabId), handed, at: now, said });
  }
}

/** What the sent page in this tab does: go back (or close a tab opened only for the file), or
 *  stay and say why. Answered once the tab is known. */
async function sentPageAnswer(tabId) {
  const s = sentPages.get(tabId);
  sentPages.delete(tabId);
  if (!s || Date.now() - s.at > CAUGHT_AGAIN) return { ok: true, stay: false };
  const tab = await s.tab;
  if (tab?.incognito) {
    // Handed back to the tab, or not: said as it is.
    if ((await s.handed) === false) return { ok: true, stay: true, said: tr("This private window's download couldn't go on in your browser. Use Back and download it once more.") };
    return { ok: true, stay: true, said: tr("This private window's download goes on in your browser.") };
  }
  if (s.said) return { ok: true, stay: true, said: s.said };
  if (await s.stay) return { ok: true, stay: true, said: tr("This download went to Pako. Use Back to return to your page.") };
  return { ok: true, stay: false };
}

/** Pako gets a caught file, like a takeover. If Pako can't, the browser downloads it after all
 *  (the extension's own request is never caught), with the "!" saying why. */
async function handCaught(details, seen, tabNow) {
  const url = details.url;
  if (!isWebLink(url)) return;
  const tab = await tabNow;
  const name = seen?.filename || "";
  // A private window's tab, opened just before the rule could leave it out: back to that tab,
  // where the browser downloads it (and the file-name step takes it, held paused, as private).
  if (tab?.incognito) return againInTab(tab, url, name);
  // Not known: taken as private (no normal login, never started again in a normal window).
  const isPrivate = !tab;
  const storeId = isPrivate ? PRIVATE_STORE : undefined;
  // (Counted in inFlight by onCaught, from the moment it was caught.)
  const { takeOver: on } = await settings();
  let reply = { ok: false };
  // (Switched off meanwhile, or asked with a site's own sign-in: the browser downloads it.)
  if (on !== false && !seen?.auth) {
    reply = await handOver(url, catchReferrer(tab, seen, url), name, { storeId, totalBytes: seen?.size || 0, mime: seen?.type || "", hold: true, isPrivate });
  }
  if (reply.ok) return "pako";
  if (isPrivate) {
    const why = (reply.error || tr("Pako couldn't take this download")).replace(/[.\s]+$/, "");
    flag(tr("{why}. Download it once more from the page.", { why }), reply.detail, true);
    if (typeof reply.followed === "string") {
      send({ kind: "follow", urls: [url], filename: name, private: true, follow: { pako: reply.followed, browserId: -1, state: "interrupted", error: "RESTART_FAILED" } });
    }
    return;
  }
  await startAgain(url, name, storeId, reply, false);
}

/** A private window's file that was caught: asked for again in its own tab, past the catch rule
 *  (a one-time exception for that tab and address, set before the tab asks). The browser then
 *  downloads it there, as if it had never been caught. Whether it was handed back. The exception
 *  goes once the tab's answer for it comes (see onMediaHeaders), when the tab closes, or after
 *  ALLOW_FOR. */
async function againInTab(tab, url, name) {
  const id = ALLOW_RULES + (allowNext++ % ALLOW_RING);
  try {
    // (A filter can't hold * ^ | as plain characters.)
    if (/[*^|]/.test(url) || url.length > 4000) throw new Error(tr("This address can't be let through."));
    const rule = { id, priority: 2, action: { type: "allow" }, condition: { urlFilter: `|${url}|`, tabIds: [tab.id], resourceTypes: ["main_frame"] } };
    clearTimeout(allowRules.get(id)?.timer);
    const timer = setTimeout(() => dropAllow(id), ALLOW_FOR);
    timer?.unref?.();
    allowRules.set(id, { tabId: tab.id, url, timer });
    await api.declarativeNetRequest.updateSessionRules({ removeRuleIds: [id], addRules: [rule] });
    await api.tabs.update(tab.id, { url });
    return true;
  } catch (e) {
    dropAllow(id);
    const said = name
      ? tr('Pako\'s extension couldn\'t hand "{name}" back to your private window. Download it once more from the page.', { name })
      : tr("Pako's extension couldn't hand this download back to your private window. Download it once more from the page.");
    flag(said, e?.message || String(e), true);
    return false;
  }
}

/** Takes a one-time exception out of the browser. */
function dropAllow(id) {
  clearTimeout(allowRules.get(id)?.timer);
  allowRules.delete(id);
  Promise.resolve(api.declarativeNetRequest?.updateSessionRules({ removeRuleIds: [id] })).catch(() => {});
}
/** A tab's one-time exceptions: the one for that address (its answer came: used), or all of them
 *  (`url` null: the tab closed). */
function dropAllowFor(tabId, url) {
  for (const [id, a] of allowRules) {
    if (a.tabId === tabId && (url == null || a.url.toLowerCase() === String(url).toLowerCase())) dropAllow(id);
  }
}
/** At start: exceptions left by the background before it slept go (each was for one request,
 *  made at once). */
function dropOldAllows() {
  const dnr = api.declarativeNetRequest;
  if (!dnr?.getSessionRules || FIREFOX) return;
  Promise.resolve(dnr.getSessionRules())
    .then((rules) => {
      const old = (rules || []).map((r) => r?.id).filter((id) => id >= ALLOW_RULES && id < ALLOW_RULES + ALLOW_RING && !allowRules.has(id));
      if (old.length) return dnr.updateSessionRules({ removeRuleIds: old });
    })
    .catch(() => {});
}

// ---------- download links on pages (browsers that ask first) ----------
// A page's link made to download (`<a download href="…">`, on the page's own site) starts its
// download without loading anything in the tab, so the catch rule never sees it, and Vivaldi
// asks first. While the catch is on and "Catch download links on pages" is on (catchLinks, on
// by default), a small script of the extension's own is in every web page's top frame
// (download-links.js): it reads nothing of the page, and only when you click such a link (a
// real click, Enter on it, or the middle button) and the page didn't handle that click itself,
// it stops the browser's download and tells the extension the link. The extension hands it to
// Pako like a caught download; if Pako can't take it, the browser downloads it after all
// (startAgain). Never in a private window (like the rule: the browser asks, then the file-name
// step takes it), never on a video site, never a link to another site (the browser loads that as
// a page, where the rule sees it), nor one built inside the page (blob:, data:). The script is in
// the browser only while all that is on; a page opened before keeps it until it reloads, and its
// clicks then go to the browser as usual (the extension says no, the page starts it again).

/** The script's ID in the browser (scripting.registerContentScripts). */
const LINKS_SCRIPT = "pako-links";
/** The same link from the same tab within this: a double click, handed once. */
const LINK_AGAIN = 3000;
/** Whether the script is registered now (null: not checked since this background page started). */
let linksApplied = null;
/** "tab address" → when it was handed last (see LINK_AGAIN). */
const linksLately = new Map();

/** Puts the link script in the browser, or takes it away. Called by syncCatchNow (one at a time). */
async function syncLinks(want) {
  const sc = api.scripting;
  if (FIREFOX || !sc?.registerContentScripts) return;
  if (want === linksApplied) return;
  try {
    // (Registered scripts outlive the background page, not the browser.)
    const there = (await sc.getRegisteredContentScripts({ ids: [LINKS_SCRIPT] })).length > 0;
    if (want && !there) {
      await sc.registerContentScripts([
        {
          id: LINKS_SCRIPT,
          js: ["download-links.js"],
          matches: ["http://*/*", "https://*/*"],
          // ("*.site" is the site and every part of it.)
          excludeMatches: LIGHT_SKIPS_SITES.map((d) => `*://*.${d}/*`),
          allFrames: false,
          runAt: "document_start",
          persistAcrossSessions: false,
        },
      ]);
    } else if (!want && there) await sc.unregisterContentScripts({ ids: [LINKS_SCRIPT] });
    linksApplied = want;
  } catch (e) {
    console.warn("Pako: the browser didn't take the script for download links:", e?.message || e);
  }
}

/** The address without its "#…" (never sent to a server). */
function withoutHash(url) {
  try {
    const u = new URL(url);
    u.hash = "";
    return u.href;
  } catch {
    return "";
  }
}

/** A download link clicked on a page (download-links.js). `{ taken: true }`: the extension has it
 *  (Pako, or the browser if Pako can't); `{ taken: false }`: the page lets the browser download
 *  it as usual. Answered at once: Pako may wait for you in its window. */
async function linkClicked(msg, sender) {
  const no = { ok: true, taken: false };
  const tab = sender?.tab;
  // Only a normal window's tab (a private window's: the browser's way, like the rule), its top frame.
  if (FIREFOX || !tab || tab.incognito || sender.frameId !== 0 || !Number.isInteger(tab.id)) return no;
  const url = withoutHash(msg?.url);
  const page = withoutHash(sender.url);
  if (!isWebLink(url) || !isWebLink(page) || url.length > MAX_LINK) return no;
  // The page's own site only (the browser loads a link to another site as a page: the rule's).
  if (new URL(url).origin !== new URL(page).origin) return no;
  if (onSkippedSite(url) || onSkippedSite(page)) return no;
  // Switched off, or Pako away, since the page got its script.
  if (!(await catchState())?.links) return no;
  const now = Date.now();
  for (const [k, at] of linksLately) if (now - at > LINK_AGAIN || linksLately.size > 50) linksLately.delete(k);
  const key = `${tab.id} ${url}`;
  if (linksLately.has(key)) return { ok: true, taken: true };
  // Pako just couldn't get it (and the browser got it again, or couldn't): clicked once more, the
  // page's own download, as the browser makes it (with the page's own way to the file: its
  // service worker, its login), never Pako's again, or the link could never be downloaded.
  if (restarted.has(url)) return no;
  linksLately.set(key, now);
  const name = baseName(typeof msg.filename === "string" ? msg.filename : "")
    .replace(/[\u0000-\u001f\u007f]/g, "")
    .trim()
    .slice(0, 255);
  const referrer = msg.noReferrer === true ? "" : page;
  // No reload for an update until Pako (or the browser) has it.
  inFlight++;
  handLink(url, referrer, name)
    .catch((e) => console.warn("Pako:", e?.message || e))
    .finally(() => inFlight--);
  return { ok: true, taken: true };
}

/** Pako gets a clicked download link, like a caught download (its size isn't known yet). If Pako
 *  can't, the browser downloads it after all (marked, so it isn't taken again), with the "!". */
async function handLink(url, referrer, name) {
  const { takeOver: on } = await settings();
  let reply = { ok: false };
  if (on !== false) reply = await handOver(url, referrer, name, { totalBytes: 0, mime: "", hold: true, isPrivate: false });
  if (!reply.ok) await startAgain(url, name, undefined, reply, false);
}

api.webRequest?.onBeforeRedirect?.addListener(onCaught, { urls: ["<all_urls>"], types: ["main_frame", "sub_frame"] });
// The takeover switch and "Catch download links on pages" (the toolbar window sets them), and a
// new private tab (never caught).
api.storage.onChanged?.addListener((changes, area) => {
  if (area === "local" && changes && ("takeOver" in changes || "catchLinks" in changes)) syncCatch();
});
api.tabs.onCreated?.addListener((t) => {
  if (t?.incognito) syncCatch();
});
// A tab swapped for another (a page loaded ahead of time): its new ID may be a private one.
api.tabs.onReplaced?.addListener(() => syncCatch());
// The browser keeps the rule while it runs, not after it restarts: set again at every start.
syncCatch();
dropOldAllows();
// An older script than the folder's, kept by the browser: reloaded once.
olderScript();

function forgetTab(tabId) {
  caughtWork.delete(tabId);
  pageMethods.delete(tabId);
  pageLoads.delete(tabId);
  sentPages.delete(tabId);
  dropAllowFor(tabId, null);
  Promise.resolve(api.storage.session?.remove(caughtKey(tabId))).catch(() => {});
  Promise.resolve(api.storage.session?.remove(postKey(tabId))).catch(() => {});
  forgetPill(tabId);
  cancelAheadIn(tabId);
}
api.tabs.onRemoved?.addListener((tabId) => {
  forgetTab(tabId);
  // The recorded tab closed: the recording ends, and its recorder goes from every site.
  if (rec?.tabId === tabId) stopRecording("This tab is closed.");
});
// The browser swapped a tab for another (a discarded tab reloading): the old one's list goes.
api.tabs.onReplaced?.addListener((_added, removed) => forgetTab(removed));

/** The browser's cookies for the stream's own address only (what the browser itself sends it),
 *  never from a private window. */
async function streamCookies(url, incognito, storeId) {
  if (incognito) return [];
  try {
    const query = typeof storeId === "string" && storeId ? { url, storeId } : { url };
    const list = await api.cookies.getAll(query);
    const plain = (t) => typeof t === "string" && !/[\u0000-\u001f\u007f]/.test(t);
    return list
      .filter((c) => c.name && plain(c.name) && plain(c.value) && plain(c.domain) && plain(c.path || "/"))
      .slice(0, 300)
      .map((c) => ({
        domain: c.hostOnly ? c.domain.replace(/^\./, "") : c.domain.startsWith(".") ? c.domain : "." + c.domain,
        path: c.path || "/",
        secure: !!c.secure,
        expires: c.session || !c.expirationDate ? 0 : Math.floor(c.expirationDate),
        name: c.name,
        value: c.value,
      }));
  } catch {
    return [];
  }
}

/** "Download with Pako" on one of a tab's videos: the stream, the page it played on (as the
 *  referrer), the page's title (the file's name) and the cookies for the stream's server. */
async function sendStream(tabId, url) {
  const item = (await caughtList(tabId)).find((v) => v.url === url);
  if (!item) return { ok: false, error: tr("This video isn't on the page any more. Play it again.") };
  const tab = await api.tabs.get(tabId).catch(() => null);
  if (!tab) return { ok: false, error: tr("This tab is closed.") };
  // Asked again at the click (the pill's menu or the popup may have been opened before the
  // page asked for a key system): never a protected video.
  if ((await videosOf(tabId)).protected) return { ok: false, error: tr(PROTECTED) };
  const message = {
    kind: "stream",
    urls: [item.url],
    referrer: item.doc || (isWebLink(tab.url) ? tab.url : ""),
    filename: String(tab.title || "").slice(0, 300),
    siteCookies: await streamCookies(item.url, !!tab.incognito, tab.cookieStoreId),
  };
  // A private window's video: Pako keeps no trace of it beyond the file.
  if (tab.incognito) message.private = true;
  return send(message);
}

/** What the popup shows for a tab: its videos, whether a video plays there, and whether it is
 *  protected (DRM: the page's video uses a key system). */
async function videosOf(tabId) {
  const list = visible(await caughtList(tabId)).map((v) => {
    let host = "";
    try {
      host = new URL(v.url).hostname;
    } catch {
      // (only web links are kept)
    }
    return { url: v.url, kind: v.kind, host, height: v.height || 0 };
  });
  let playing = false;
  let locked = false;
  try {
    const results = await api.scripting.executeScript({
      target: { tabId, allFrames: true },
      func: () =>
        [...document.querySelectorAll("video")].map((v) => ({
          playing: v.readyState >= 2 && (!v.paused || v.currentTime > 0),
          locked: !!v.mediaKeys,
        })),
    });
    for (const r of results || []) {
      for (const v of r?.result || []) {
        playing ||= v.playing;
        locked ||= v.locked;
      }
    }
  } catch {
    // a page the browser keeps extensions out of (its own pages, the stores)
  }
  return { ok: true, list, playing, protected: locked, recording: recordState(tabId) };
}

// ---------- the Download pill on a page's video ----------
// Once a video is caught in a tab, the pill (pill.js + caught.js) is put into the frame that
// loaded it (else the top frame), once per frame and page. No page where nothing was caught is
// ever touched, and on the video sites Pako knows, player.js keeps its own pill.

const PILL_FILES = ["lang.js", "pill.js", "caught.js"];
/** The popup's names for what was caught. */
const CAUGHT_KINDS = { hls: "HLS stream", dash: "DASH stream", file: "Video file" };
/** The frames of a tab that have the pill (`frameId|page`), in the same memory-only storage as
 *  the tab's list (the background page may sleep in between). */
const pillKey = (tabId) => `pill-${tabId}`;
const pillWork = new Map();

function forgetPill(tabId) {
  pillWork.delete(tabId);
  Promise.resolve(api.storage.session?.remove(pillKey(tabId))).catch(() => {});
}

/** Whether the pill goes into this frame now: not when the "Download button on videos" switch
 *  is off, nor a second time into the same frame's page. Notes it as done. */
async function pillWanted(tabId, frame, page) {
  const { playerButton } = await api.storage.local.get({ playerButton: true });
  if (playerButton === false) return false;
  const key = `${frame}|${page}`;
  const kept = (await api.storage.session.get(pillKey(tabId)))[pillKey(tabId)];
  // (Only a list of frames counts: anything else kept there is started again.)
  const done = Array.isArray(kept) ? kept.filter((k) => typeof k === "string") : [];
  if (done.includes(key)) return false;
  await api.storage.session.set({ [pillKey(tabId)]: [...done, key] });
  return true;
}

/** Puts the pill into the frame where a video was caught (`frameId` from the request), else
 *  into the top frame. One tab at a time, so two requests together inject it once. */
function offerPill(tabId, frameId, page) {
  const frame = Number.isInteger(frameId) && frameId >= 0 ? frameId : 0;
  const next = (pillWork.get(tabId) ?? Promise.resolve()).then(async () => {
    if (!(await pillWanted(tabId, frame, String(page || "")))) return;
    try {
      await api.scripting.executeScript({ target: { tabId, frameIds: [frame] }, files: PILL_FILES });
    } catch {
      // That frame is gone or closed to extensions: the top frame instead (once too).
      if (frame !== 0 && (await pillWanted(tabId, 0, ""))) {
        await api.scripting.executeScript({ target: { tabId, frameIds: [0] }, files: PILL_FILES }).catch(() => {});
      }
    }
  });
  pillWork.set(tabId, next.catch(() => {}));
  return next;
}

/** The pill's menu for a tab, from what the popup shows there (videosOf): one caught video
 *  downloads at once, several are listed like in the popup, and Record this video follows the
 *  popup's rule (a video plays or is listed, never a protected one, not while recording). */
function pillMenu({ list, playing, protected: locked, recording }) {
  const items = [{ head: tr("Download with Pako") }];
  if (locked) return [...items, { info: tr(PROTECTED) }];
  const upTo = (v) => (v.height ? tr("up to {height}p", { height: v.height }) : "");
  const kind = (v) => tr(CAUGHT_KINDS[v.kind]);
  if (list.length === 1) {
    const v = list[0];
    items.push({ text: tr("Download this video"), side: upTo(v) || kind(v), send: { type: "pill-send", url: v.url } });
  } else {
    for (const v of list) {
      const text = upTo(v) ? `${kind(v)} · ${upTo(v)}` : kind(v);
      items.push({ text, side: v.host, send: { type: "pill-send", url: v.url } });
    }
  }
  if (!list.length) items.push({ info: tr("Pako didn't find this video's address.") });
  const busy = recording && ["waiting", "recording", "saving"].includes(recording.state);
  if (busy) items.push({ sep: true }, { info: tr("Recording this video: the Pako button in the toolbar shows how it's going.") });
  else if (playing || list.length) {
    items.push({ sep: true }, { text: tr("Record this video"), side: tr("reloads the page"), send: { type: "pill-record" }, done: tr("Recording…") });
  }
  return items;
}

/** Record this video from the pill: only where the pill would offer it. */
async function recordFromPill(tabId) {
  const v = await videosOf(tabId);
  if (v.protected) return { ok: false, error: tr(PROTECTED) };
  if (!v.playing && !v.list.length) return { ok: false, error: tr("Play the video first, then try again.") };
  return startRecording(tabId);
}

// ---------- "Record this video" (the last resort) ----------
// When Pako can't download a video, the extension can copy what the page gives its own player
// while the video plays (fast and muted when the page allows), and send it to Pako in small
// parts; Pako makes the video at the end. The page reloads once so nothing is missed. Never a
// protected (DRM) video: recording stops as soon as the page asks for a key system.

const RECORDER_SCRIPTS = ["pako-rec-main", "pako-rec-bridge"];
/** How long the reloaded page has to start playing the video. The recorder is added to every
 *  page opened meanwhile (the browser can't add it to one tab only; it does nothing outside the
 *  recorded one), so this is kept short, and it is taken away as soon as the video plays. */
const REC_WAIT = 5 * 60 * 1000;
const PROTECTED = "This video is protected, so it can't be downloaded.";
/** The recording going on (one at a time), or the last one's outcome. */
let rec = null;

function recordState(tabId) {
  if (!rec || rec.tabId !== tabId) return null;
  return { state: rec.state, bytes: rec.bytes, error: rec.error || "" };
}

/** One line to Pako's helper for the whole recording: the parts go one after the other, and the
 *  answers come back in the same order. */
function openNative() {
  const port = api.runtime.connectNative(HOST);
  const waiting = [];
  let gone = false;
  port.onMessage.addListener((reply) => {
    if (reply && typeof reply === "object") {
      learnLang(reply.lang);
      if (typeof reply.error === "string") reply.error = trMaybe(reply.error);
    }
    waiting.shift()?.(reply && typeof reply === "object" ? reply : { ok: false, error: tr("Pako didn't answer.") });
  });
  port.onDisconnect.addListener(() => {
    void api.runtime.lastError;
    gone = true;
    for (const resolve of waiting.splice(0)) resolve({ ok: false, error: tr(NOT_CONNECTED) });
  });
  return {
    ask: (message) =>
      new Promise((resolve) => {
        if (gone) return resolve({ ok: false, error: tr(NOT_CONNECTED) });
        waiting.push(resolve);
        try {
          port.postMessage(message);
        } catch {
          waiting.pop();
          resolve({ ok: false, error: tr(NOT_CONNECTED) });
        }
      }),
    close: () => {
      gone = true;
      try {
        port.disconnect();
      } catch {
        // already closed
      }
    },
  };
}

async function removeRecorder() {
  try {
    const ids = (await api.scripting.getRegisteredContentScripts({ ids: RECORDER_SCRIPTS })).map((s) => s.id);
    if (ids.length) await api.scripting.unregisterContentScripts({ ids });
  } catch {
    // none there, or a browser without them
  }
}

const recordMessage = (r, action, extra = {}) => ({ kind: "record", record: { action, session: r.session, ...extra } });

/** Ends the recording: `error` = why it stopped (Pako drops what it had then, unless `told`). */
async function stopRecording(error, told = false) {
  const r = rec;
  if (!r || r.state === "done" || r.state === "failed") return;
  r.state = error ? "failed" : "done";
  // (Pako's words too, an older Pako's in English: in Pako's language.)
  r.error = error ? trMaybe(error) : "";
  clearTimeout(r.timer);
  try {
    r.page?.postMessage({ t: "stop" });
  } catch {
    // the page is gone
  }
  if (error && !told) await r.native.ask(recordMessage(r, "cancel"));
  r.native.close();
  await removeRecorder();
}

/** Starts recording a tab's video: Pako is asked first (it says no when its video tools are
 *  missing), then the page reloads with the recorder in it. */
async function startRecording(tabId) {
  if (rec && ["waiting", "recording", "saving"].includes(rec.state)) {
    return { ok: false, error: tr("Pako is already recording a video. Stop that one first.") };
  }
  const tab = await api.tabs.get(tabId).catch(() => null);
  if (!tab || !isWebLink(tab.url)) return { ok: false, error: tr("Pako can't record on this page.") };
  if (tab.incognito) return { ok: false, error: tr("Pako doesn't record in private windows.") };
  const r = { tabId, session: crypto.randomUUID(), state: "waiting", bytes: 0, error: "", seq: {}, page: null, queue: Promise.resolve() };
  r.native = openNative();
  const reply = await r.native.ask(recordMessage(r, "start", { page: tab.url, title: String(tab.title || "").slice(0, 300) }));
  if (!reply.ok) {
    r.native.close();
    return reply;
  }
  await removeRecorder();
  try {
    const common = { matches: ["<all_urls>"], allFrames: true, runAt: "document_start", persistAcrossSessions: false };
    await api.scripting.registerContentScripts([
      { id: "pako-rec-main", js: ["recorder.js"], world: "MAIN", ...common },
      { id: "pako-rec-bridge", js: ["recorder-bridge.js"], ...common },
    ]);
  } catch (e) {
    await r.native.ask(recordMessage(r, "cancel"));
    r.native.close();
    return { ok: false, error: tr("This browser can't record videos. Update it, then try again."), detail: e?.message || String(e) };
  }
  rec = r;
  r.timer = setTimeout(() => {
    if (rec === r && r.state === "waiting") stopRecording("The video didn't start playing, so nothing was recorded.");
  }, REC_WAIT);
  try {
    await api.tabs.reload(tabId);
  } catch {
    await stopRecording("This tab is closed.");
  }
  return { ok: true };
}

/** The recorder in a page asks whether to record: only in the recorded tab, while it waits. */
function recorderHello(sender) {
  return { record: !!rec && rec.state === "waiting" && sender?.tab?.id === rec.tabId };
}

/** The recorder's line from a page: only the first frame that sends something is recorded. */
api.runtime.onConnect?.addListener((port) => {
  if (port.name !== "pako-rec") return;
  const r = rec;
  if (!r || !["waiting", "recording"].includes(r.state) || port.sender?.tab?.id !== r.tabId) {
    port.postMessage({ t: "stop" });
    port.disconnect();
    return;
  }
  port.onMessage.addListener((m) => {
    if (rec !== r || (r.page && r.page !== port)) {
      port.postMessage({ t: "stop" });
      return;
    }
    if (m?.t === "protected") {
      stopRecording(PROTECTED);
    } else if (m?.t === "chunk" && Number.isInteger(m.track)) {
      r.page = port;
      if (r.state === "waiting") {
        r.state = "recording";
        clearTimeout(r.timer);
        // The recorded page has its recorder: no other page opened from now on gets it.
        removeRecorder();
      }
      const seq = (r.seq[m.track] = (r.seq[m.track] ?? -1) + 1);
      // In order: each part waits for Pako's answer to the one before.
      r.queue = r.queue.then(async () => {
        if (r.state !== "recording") return;
        const reply = await r.native.ask(recordMessage(r, "chunk", { track: m.track, mime: String(m.mime || ""), seq, data: String(m.data || "") }));
        if (!reply.ok) return stopRecording(reply.error || "Pako couldn't save the recording.");
        r.bytes += Number(m.size) || 0;
        port.postMessage({ t: "ack" });
      });
    } else if (m?.t === "end" && r.page === port) {
      r.queue = r.queue.then(async () => {
        if (r.state !== "recording") return;
        r.state = "saving";
        const reply = await r.native.ask(recordMessage(r, "end"));
        r.state = "recording";
        await stopRecording(reply.ok ? "" : reply.error || "Pako couldn't save the recording.", true);
      });
    }
  });
  port.onDisconnect.addListener(() => {
    // The page closed or went elsewhere before the video ended.
    if (rec === r && r.page === port && r.state === "recording") stopRecording("The page closed before the video ended, so nothing was saved.");
  });
});

// A recording the background page lost (the browser restarted it): its recorder goes.
removeRecorder();
/* full-only:end */

// ---------- the popup asks through here ----------

/** The messages from the toolbar window and the pages (registered at the
 *  top, see `api.runtime.onMessage`). Every one is answered, whatever goes wrong. */
function onMessage(msg, sender, respond) {
  (async () => {
    // (Its answers in Pako's language: the one kept, read at start.)
    await langReady;
    // "Send this page" comes from the toolbar window only, never from a page in a tab. A
    // private window's tab (the window says so): Pako keeps no trace of it.
    if (msg?.type === "send-tab" && !sender?.tab) respond(await download(msg.url, "", "", { incognito: !!msg.incognito, storeId: msg.storeId, isPrivate: msg.incognito === true }));
    // (With this script's version: the toolbar window tells an older script the browser kept.)
    else if (msg?.type === "ping") respond({ ...(await send({ kind: "ping" })), script: SCRIPT_VERSION, lang });
    // The toolbar window: are private windows on (the browser's own switch for Pako)?
    else if (msg?.type === "private") respond({ allowed: await privateAllowed() });
    // Pako Light's popup: is the full extension on here (then Light stays out of the way), and
    // is this tab on a site Light leaves alone (its Send button is greyed out there)?
    else if (msg?.type === "edition") {
      const fullOn = await fullHere();
      if (LIGHT) syncMenus();
      respond({ ok: true, light: LIGHT, fullOn, skipped: LIGHT && onSkippedSite(msg.url) });
    }
    /* full-only:start */
    // From the Download button on a video player (only from a page in a tab).
    else if (msg?.type === "video" && sender?.tab && !LIGHT) {
      // The pick wins over the menu closing: its read ahead is never cancelled now.
      pickedAhead(sender.tab.id, msg.url);
      const reply = await sendVideo(msg.url, msg.quality, { incognito: !!sender.tab.incognito, storeId: sender.tab.cookieStoreId });
      if (!reply.ok && reply.error) flag(reply.error, reply.detail, !!sender.tab.incognito);
      respond(reply);
    }
    // The same button's quality menu opened: answered at once (the pill never waits for it).
    else if (msg?.type === "video-prepare" && sender?.tab && !LIGHT) {
      respond({ ok: true });
      await prepareVideo(msg.url, { tabId: sender.tab.id, incognito: !!sender.tab.incognito, storeId: sender.tab.cookieStoreId }).catch(() => {});
    }
    // That menu closed without a pick: Pako stops the read at once (answered at once too).
    else if (msg?.type === "video-prepare-cancel" && sender?.tab && !LIGHT) {
      respond({ ok: true });
      await cancelAhead(aheadKey(sender.tab.id, msg.url));
    }
    // The toolbar window's "Videos on this page" (never from a page in a tab).
    else if (msg?.type === "videos" && !sender?.tab && !LIGHT) respond(await videosOf(Number(msg.tabId)));
    else if (msg?.type === "send-stream" && !sender?.tab && !LIGHT) respond(await sendStream(Number(msg.tabId), msg.url));
    else if (msg?.type === "record-start" && !sender?.tab && !LIGHT) respond(await startRecording(Number(msg.tabId)));
    else if (msg?.type === "record-cancel" && !sender?.tab && !LIGHT) {
      if (rec?.tabId === Number(msg.tabId)) await stopRecording("Recording stopped.");
      respond({ ok: true });
    } else if (msg?.type === "rec-hello" && sender?.tab && !LIGHT) respond(recorderHello(sender));
    // The Download pill on a page's video (caught.js): only about its own tab, never another.
    else if (msg?.type === "pill-menu" && sender?.tab && !LIGHT) {
      const v = await videosOf(sender.tab.id);
      respond({ ok: true, items: pillMenu(v) });
    } else if (msg?.type === "pill-send" && sender?.tab && !LIGHT) respond(await sendStream(sender.tab.id, msg.url));
    else if (msg?.type === "pill-record" && sender?.tab && !LIGHT) respond(await recordFromPill(sender.tab.id));
    // The extension's own sent page, in a tab: go back, or stay (see sentPageAnswer).
    else if (msg?.type === "sent-page" && sender?.tab && sender.id === api.runtime.id && isSentPage(sender.url) && !LIGHT) respond(await sentPageAnswer(sender.tab.id));
    // A download link clicked on a page (download-links.js, only from a page in a tab).
    else if (msg?.type === "link-download" && sender?.tab && !LIGHT) respond(await linkClicked(msg, sender));
    // The toolbar window: does this browser ask before downloads (then it shows "Catch download
    // links on pages")?
    else if (msg?.type === "asks-first" && !sender?.tab && !LIGHT) respond({ ok: true, asks: !FIREFOX && !!api.declarativeNetRequest?.updateSessionRules && !!(await catchState())?.asks });
    /* full-only:end */
    else respond({ ok: false, error: tr("Pako's extension didn't understand this request."), detail: String(msg?.type || "") });
  })().catch((e) => {
    console.warn("Pako:", e);
    respond({ ok: false, error: tr("Pako's extension ran into a problem. Try again."), detail: e?.message || String(e) });
  });
  return true; // answered later
}

