// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's browser extension: hands downloads and links to Pako on this PC.
// It talks to Pako only through the browser's "native messaging" (no network, no open port).
// Blocks between "full-only:start" and "full-only:end" are only in the full extension:
// `node extension/build.mjs` takes them out of Pako Light.

const api = globalThis.browser ?? globalThis.chrome;
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
  if (globalThis.browser?.runtime?.sendNativeMessage) return globalThis.browser.runtime.sendNativeMessage(HOST, message);
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
  try {
    const reply = await nativeMessage(message);
    if (reply && typeof reply === "object") return reply;
    const detail = `No answer from ${HOST}`;
    console.warn("Pako:", detail);
    return { ok: false, error: "Pako didn't answer.", detail };
  } catch (e) {
    const detail = e?.message || String(e);
    console.warn(`Pako: couldn't reach ${HOST}:`, detail);
    return { ok: false, error: NOT_CONNECTED, detail };
  }
}

/** The cookies the browser would send to this address (so downloads behind a login work), from
 *  the tab's own cookie store (a Firefox container has its own). They only travel to Pako on
 *  this PC, and Pako keeps them for that download only. Never from a private window. */
async function cookiesFor(url, incognito, storeId) {
  if (incognito) return "";
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
  if (incognito || !site) return [];
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
async function download(url, referrer, filename, { takeover = false, incognito = false, storeId, totalBytes = 0, mime = "" } = {}) {
  if (!isWebLink(url)) return { ok: false, error: "Pako only takes http and https links." };
  // Pako Light sends nothing from the sites it leaves alone (Pako checks again on its side).
  if (LIGHT && (onSkippedSite(url) || onSkippedSite(referrer))) return { ok: false, error: LIGHT_SKIPPED };
  const message = {
    kind: "download",
    urls: [url],
    referrer: referrer || "",
    cookies: await cookiesFor(url, incognito, storeId),
    filename: filename || "",
    takeover,
    totalBytes: takeover && totalBytes > 0 ? totalBytes : 0,
    mime: takeover && typeof mime === "string" ? mime : "",
  };
  /* full-only:start */
  // A video page or link: the site's login goes with it (Pako downloads the video).
  if (!takeover && isVideoSite(url)) message.siteCookies = await siteCookiesFor(url, incognito, storeId);
  /* full-only:end */
  return send(message);
}

/** "All links on this page": in parts small enough for Pako's line (each part is added to Links).
 *  Pako Light leaves out links to the sites it leaves alone, and sends nothing from them. */
async function sendLinks(urls, referrer) {
  if (LIGHT && onSkippedSite(referrer)) return { ok: false, error: LIGHT_SKIPPED };
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
  if (!parts.length) return { ok: false, error: "No links on this page." };
  for (const p of parts) {
    const reply = await send({ kind: "links", urls: p, referrer });
    if (!reply.ok) return reply;
  }
  return { ok: true };
}

/* full-only:start */
/** The player's Download button: one video straight to Downloads in that quality. */
async function sendVideo(url, quality, { incognito = false, storeId } = {}) {
  if (!isWebLink(url)) return { ok: false, error: "Pako only takes http and https links." };
  if (!VIDEO_QUALITIES.includes(quality)) return { ok: false, error: "Unknown video quality." };
  return send({ kind: "video", urls: [url], quality, siteCookies: await siteCookiesFor(url, incognito, storeId) });
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

function makeMenus() {
  return new Promise((resolve) => {
    let made = false;
    const make = () => {
      if (made) return;
      made = true;
      api.contextMenus.create({ id: "pako-link", title: "Download with Pako", contexts: ["link"] });
      let media = ["image"];
      /* full-only:start */
      media = ["image", "video", "audio"];
      /* full-only:end */
      api.contextMenus.create({ id: "pako-media", title: "Download with Pako", contexts: media });
      api.contextMenus.create({ id: "pako-page", title: "Send this page to Pako", contexts: ["page"] });
      api.contextMenus.create({ id: "pako-all", title: "Send all links on this page to Pako (to pick from)", contexts: ["page"] });
      resolve();
    };
    // Chromium calls back; Firefox's `browser` answers with a promise.
    const r = api.contextMenus.removeAll(make);
    r?.then?.(make, make);
  });
}

/** Makes the right-click items, or (Pako Light, next to the full extension) takes them away. */
async function syncMenus(force = false) {
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
  else changes = [["pako-page", { title: skipped ? "Download video with Pako" : "Send this page to Pako" }]];
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

api.runtime.onInstalled.addListener(() => syncMenus(true));
api.runtime.onStartup?.addListener(() => syncMenus(true));

async function pageLinks(tabId) {
  const [result] = await api.scripting.executeScript({
    target: { tabId },
    func: () => [...new Set([...document.links].map((a) => a.href).filter((h) => /^https?:\/\//i.test(h)))].slice(0, 2000),
  });
  return result?.result ?? [];
}

api.contextMenus.onClicked.addListener(async (info, tab) => {
  const referrer = info.pageUrl || tab?.url || "";
  const incognito = !!tab?.incognito;
  // Firefox containers: the tab's own cookies (Chromium browsers have no such field).
  const storeId = tab?.cookieStoreId;
  let reply;
  if (info.menuItemId === "pako-link") reply = await download(info.linkUrl, referrer, "", { incognito, storeId });
  else if (info.menuItemId === "pako-media") reply = await download(info.srcUrl, referrer, "", { incognito, storeId });
  else if (info.menuItemId === "pako-page") reply = await download(tab?.url, "", "", { incognito, storeId });
  else if (info.menuItemId === "pako-all" && tab?.id != null) {
    // Pako Light doesn't even read the page's links on a site it leaves alone.
    if (LIGHT && (onSkippedSite(tab.url) || onSkippedSite(referrer))) reply = { ok: false, error: LIGHT_SKIPPED };
    else {
      const urls = await pageLinks(tab.id).catch(() => []);
      reply = urls.length ? await sendLinks(urls, referrer) : { ok: false, error: "No links on this page." };
    }
  }
  if (reply && !reply.ok) flag(reply.error, reply.detail);
});

/** A short "!" on the toolbar button when something went wrong (details in the popup). */
async function flag(error, detail) {
  await api.storage.local.set({ lastError: error || "", lastDetail: detail || "" });
  api.action.setBadgeBackgroundColor({ color: "#F96263" });
  api.action.setBadgeText({ text: error ? "!" : "" });
}

// ---------- taking over browser downloads ----------

/** The browser flagged this file (Safe Browsing and similar): it keeps its own checks. */
function isUnsafe(item) {
  const d = item?.danger;
  return !!d && d !== "safe" && d !== "accepted";
}

api.downloads.onCreated.addListener(async (item) => {
  const { takeOver } = await settings();
  const url = item.finalUrl || item.url;
  if (!takeOver || !isWebLink(url) || item.byExtensionId) return;
  // Private windows stay private; flagged files stay with the browser's warning.
  if (item.incognito || isUnsafe(item)) return;
  // Pako Light: the sites it leaves alone keep their downloads, and when the full extension is
  // on in this browser, that one takes over instead.
  if (LIGHT && (onSkippedSite(url) || onSkippedSite(item.referrer))) return;
  if (LIGHT && (await fullHere())) return;
  // Hold it while Pako is asked; it carries on in the browser if Pako can't take it.
  try {
    await api.downloads.pause(item.id);
  } catch {
    return; // already finished (tiny file): nothing to take over
  }
  const [now] = await api.downloads.search({ id: item.id }).catch(() => []);
  if (!now || isUnsafe(now)) {
    api.downloads.resume(item.id).catch(() => {});
    return;
  }
  const size = now.totalBytes > 0 ? now.totalBytes : now.fileSize > 0 ? now.fileSize : 0;
  if (size > 0 && size < SMALL) {
    api.downloads.resume(item.id).catch(() => {});
    return;
  }
  const name = (now.filename || "").split(/[\\/]/).pop();
  // Pako answers "ok" only once its own first request for the file got a good answer, so the
  // browser's copy is deleted only then; otherwise it carries on here.
  const totalBytes = now.totalBytes > 0 ? now.totalBytes : 0;
  const reply = await download(url, item.referrer || "", name, {
    takeover: true,
    storeId: now.cookieStoreId ?? item.cookieStoreId,
    totalBytes,
    mime: now.mime || item.mime || "",
  });
  if (reply.ok) {
    await api.downloads.cancel(item.id).catch(() => {});
    await api.downloads.erase({ id: item.id }).catch(() => {});
  } else {
    api.downloads.resume(item.id).catch(() => {});
    if (reply.error) flag(reply.error, reply.detail);
  }
});

// ---------- the popup asks through here ----------

api.runtime.onMessage.addListener((msg, sender, respond) => {
  (async () => {
    if (msg?.type === "send-tab") respond(await download(msg.url, "", "", { incognito: !!msg.incognito, storeId: msg.storeId }));
    else if (msg?.type === "ping") respond(await send({ kind: "ping" }));
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
      const reply = await sendVideo(msg.url, msg.quality, { incognito: !!sender.tab.incognito, storeId: sender.tab.cookieStoreId });
      if (!reply.ok && reply.error) flag(reply.error, reply.detail);
      respond(reply);
    }
    /* full-only:end */
    else respond({ ok: false });
  })().catch((e) => respond({ ok: false, error: NOT_CONNECTED, detail: e?.message || String(e) }));
  return true; // answered later
});
