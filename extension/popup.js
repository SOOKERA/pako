// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Blocks between "full-only:start" and "full-only:end" are only in the full extension.
const api = globalThis.browser ?? globalThis.chrome;
/** Pako Light (the Chrome Web Store / Edge Add-ons edition). `node extension/build.mjs` sets this
 *  to true in that edition only. */
const LIGHT = false;
const $ = (id) => document.getElementById(id);
if (LIGHT) {
  document.title = "Pako Light";
  for (const el of document.querySelectorAll(".light-only")) el.hidden = false;
}
const HOST = "app.pako.host";
const NOT_CONNECTED = "Pako isn't connected. Click the Pako button in the toolbar, then Open Pako.";
/** Opens Pako and connects it to the browsers (Pako registers this link for itself). */
const CONNECT_LINK = "pako://connect";

// ---------- Pako's language (lang.js) ----------
// This window speaks Pako's language, as the extension last heard it (`pakoLang`), and follows
// Pako's answer when it asks. The last one it showed is also kept in its own storage, read at
// once, so it opens in it straight away.

const WORDS = globalThis.__pakoWords;
let lang = "en";
try {
  lang = WORDS ? WORDS.langOf(localStorage.getItem("pakoLang")) : "en";
} catch {
  // no storage of its own here: the extension's, a moment later
}
const tr = (key, vars) => (WORDS ? WORDS.tr(lang, key, vars) : key);
/** Text that may be one of the extension's (or an older Pako's) English sentences. */
const trMaybe = (text) => (WORDS ? WORDS.maybe(lang, text) : text);
/** The words this window was written with (popup.html), by text and label. */
const written = new Map();
/** What the script put in an element since: shown again in a new language. */
const shown = new Map();

/** Puts `words` (a function, so a new language says it again) in `el`. */
function show(el, words) {
  shown.set(el, words);
  el.textContent = words();
}

/** The page in the language of the moment. */
function translatePage() {
  if (document.documentElement) document.documentElement.lang = lang;
  if (!written.size && typeof document.createTreeWalker === "function" && document.body) {
    // (4: the walker's "text nodes only".)
    const walk = document.createTreeWalker(document.body, 4);
    for (let n = walk.nextNode(); n; n = walk.nextNode()) if (n.nodeValue.trim()) written.set(n, n.nodeValue);
    for (const el of document.querySelectorAll("[aria-label]")) written.set(el, el.getAttribute("aria-label"));
  }
  for (const [node, text] of written) {
    if (node.nodeType !== 3) {
      node.setAttribute("aria-label", tr(text));
      continue;
    }
    // (The spaces around a text stay: "No Pako on this PC yet? " before its link.)
    const [, before, words, after] = /^(\s*)([\s\S]*?)(\s*)$/.exec(text);
    const said = before + tr(words) + after;
    if (node.nodeValue !== said) node.nodeValue = said;
  }
  for (const [el, words] of shown) el.textContent = words();
}

/** Pako's language, as said (`value`); a change shows every word again in it. */
function setLang(value) {
  const next = WORDS ? WORDS.langOf(value) : "en";
  try {
    localStorage.setItem("pakoLang", next);
  } catch {
    // only this window's own copy: the extension's storage still has it
  }
  if (next === lang) return;
  lang = next;
  translatePage();
}

/** "Pako isn't connected", in either language. */
const notConnected = (text) => text === NOT_CONNECTED || (!!WORDS && text === WORDS.tr("fr", NOT_CONNECTED));

translatePage();

/** The error shown now, as it came (a new language says it again). */
let errorNow = "";
function showError(text) {
  errorNow = text || "";
  show($("error"), () => trMaybe(errorNow));
  $("error").hidden = !text;
}

/** The browser's own error text, in small grey print under the status. */
function showDetail(text) {
  show($("detail"), () => (text ? tr("Details: {text}", { text }) : ""));
  $("detail").hidden = !text;
}

/** Asks Pako's helper directly from the popup (Chromium: `chrome.runtime.lastError` is read in
 *  the callback so the browser's own words are kept; Firefox rejects). */
function nativePing() {
  if (globalThis.browser?.runtime?.sendNativeMessage) return globalThis.browser.runtime.sendNativeMessage(HOST, { kind: "ping" });
  return new Promise((resolve, reject) => {
    try {
      chrome.runtime.sendNativeMessage(HOST, { kind: "ping" }, (reply) => {
        const err = chrome.runtime.lastError;
        if (err) reject(new Error(err.message || String(err)));
        else resolve(reply);
      });
    } catch (e) {
      reject(e);
    }
  });
}

/** Asks the background page (the message wakes it if it was asleep); if it can't answer (just
 *  reloaded, or no answer at all), the popup asks Pako's helper itself. Either way Pako's
 *  language comes with the answer (an older Pako's says none: English). */
async function ping() {
  let viaBackground;
  try {
    viaBackground = await api.runtime.sendMessage({ type: "ping" });
  } catch (e) {
    console.warn("Pako: the background page didn't answer:", e?.message || String(e));
  }
  /* full-only:start */
  backgroundSaid = viaBackground;
  /* full-only:end */
  if (viaBackground && typeof viaBackground === "object" && (viaBackground.ok || viaBackground.detail)) {
    if (typeof viaBackground.lang === "string") setLang(viaBackground.lang);
    return viaBackground;
  }
  try {
    const reply = await nativePing();
    if (reply && typeof reply === "object") {
      setLang(reply.lang);
      // (The background page follows the extension's storage.)
      await Promise.resolve(api.storage.local.set({ pakoLang: lang })).catch(() => {});
      return reply;
    }
    return { ok: false, error: "Pako didn't answer.", detail: tr("No answer from {host}", { host: HOST }) };
  } catch (e) {
    const detail = e?.message || String(e);
    console.warn(`Pako: couldn't reach ${HOST}:`, detail);
    return { ok: false, error: NOT_CONNECTED, detail };
  }
}

/** Pako Light only: steps aside when the full extension is on in this browser, and greys out
 *  "Send this page" on the sites it leaves alone. True = nothing else to show. */
async function lightEdition() {
  if (!LIGHT) return false;
  let edition = null;
  try {
    const [tab] = await api.tabs.query({ active: true, currentWindow: true });
    edition = await api.runtime.sendMessage({ type: "edition", url: tab?.url || "" });
  } catch {
    // the background page didn't answer: Light carries on as usual
  }
  if (edition?.fullOn) {
    $("yield").hidden = false;
    $("main").hidden = true;
    $("lines").hidden = true;
    show($("status"), () => tr("Not in use"));
    return true;
  }
  if (edition?.skipped) {
    $("send").disabled = true;
    show($("light-hint"), () => tr("Pako Light doesn't work on this site."));
  }
  return false;
}

/* full-only:start */
/** The background page's answer to "ping" (it says its script's version). */
let backgroundSaid;
/** The browser runs an older background script than this window's (it can keep the one it took
 *  when the extension was added, after Pako wrote a newer one in the folder: seen in Vivaldi
 *  8.3), and an older script can't tell. This window always comes from the folder: it reloads
 *  the extension, once per version. Not in Firefox (it updates its signed copy itself). */
async function olderBackground(status) {
  if (LIGHT || typeof globalThis.browser?.runtime?.getBrowserInfo === "function") return false;
  const said = backgroundSaid;
  const version = api.runtime.getManifest().version;
  if (!said || typeof said !== "object" || said.script === version) return false;
  const got = await Promise.resolve(api.storage.local.get({ reloadedFor: "" })).catch(() => null);
  if (!got || got.reloadedFor === version) return false;
  const noted = await Promise.resolve(api.storage.local.set({ reloadedFor: version, reloadedAt: Date.now() })).then(
    () => true,
    () => false,
  );
  if (!noted) return false;
  show(status, () => tr("Updating Pako's extension… open this again in a moment."));
  status.className = "status";
  setTimeout(() => api.runtime.reload(), 800);
  return true;
}
/* full-only:end */

async function init() {
  const { takeOver = true, lastError = "", lastDetail = "", pakoLang = "en" } = await api.storage.local.get({
    takeOver: true,
    lastError: "",
    lastDetail: "",
    pakoLang: "en",
  });
  // The language the extension last heard from Pako (this window may have shown another).
  setLang(pakoLang);
  $("take").setAttribute("aria-checked", String(takeOver));
  /* full-only:start */
  const { playerButton = true } = await api.storage.local.get({ playerButton: true });
  $("player").setAttribute("aria-checked", String(playerButton));
  /* full-only:end */
  // A problem with something from a private window is kept in the browser's memory only.
  const hidden = (await Promise.resolve(api.storage.session?.get({ lastError: "", lastDetail: "" })).catch(() => null)) || {};
  const fromPrivate = typeof hidden.lastError === "string" && hidden.lastError !== "";
  showError(fromPrivate ? hidden.lastError : lastError);
  showDetail(fromPrivate ? String(hidden.lastDetail || "") : lastDetail);
  // Seen: clear the "!" on the toolbar button.
  api.action.setBadgeText({ text: "" });
  await api.storage.local.set({ lastError: "", lastDetail: "" });
  if (fromPrivate) await Promise.resolve(api.storage.session.set({ lastError: "", lastDetail: "" })).catch(() => {});

  if (await lightEdition()) return;

  const reply = await ping();
  const status = $("status");
  /* full-only:start */
  if (await olderBackground(status)) return;
  /* full-only:end */
  if (reply?.ok) {
    show(status, () => tr("Connected"));
    status.className = "status ok";
    /* full-only:start */
    showVideos();
    /* full-only:end */
  } else {
    show(status, () => tr("Not connected"));
    status.className = "status bad";
    if (/forbidden/i.test(reply?.detail || "")) {
      // An older Pako doesn't know this extension yet: "Open Pako" can't fix that, an update can.
      $("connect").hidden = false;
      $("main").hidden = true;
      $("open").hidden = true;
      $("get").hidden = true; // Pako is on this PC: nothing to get
      show($("connect-title"), () => tr("This Pako is too old for this extension"));
      show($("connect-text"), () => tr("Update Pako (Settings › Features & updates), then open this again."));
      if (notConnected(errorNow)) showError("");
    } else if (!reply?.error || notConnected(reply.error)) {
      // The line to Pako itself is missing: one button opens Pako and connects it.
      $("connect").hidden = false;
      $("main").hidden = true;
      if (notConnected(errorNow)) showError("");
    } else {
      showError(reply.error);
    }
    showDetail(reply?.detail);
  }
}

/* full-only:start */
// ---------- videos on this page ----------

const KINDS = { hls: "HLS stream", dash: "DASH stream", file: "Video file" };
const PROTECTED = "This video is protected, so it can't be downloaded.";
/** The tab the list is for. */
let videoTab = null;
/** Pako couldn't take a video of this page: recording it is offered. */
let sendFailed = false;
let watching = null;

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

async function showVideos() {
  const [tab] = await api.tabs.query({ active: true, currentWindow: true });
  if (tab?.id == null || !/^https?:/i.test(tab.url || "")) return;
  videoTab = tab.id;
  let r;
  try {
    r = await api.runtime.sendMessage({ type: "videos", tabId: tab.id });
  } catch {
    return;
  }
  if (r?.ok) renderVideos(r);
}

function renderVideos(r) {
  const busy = r.recording && ["waiting", "recording", "saving"].includes(r.recording.state);
  $("videos").hidden = !(r.list.length || r.playing || r.protected || r.recording);
  $("videos-locked").hidden = !r.protected;
  const ul = $("videos-list");
  ul.replaceChildren();
  if (!r.protected) {
    for (const v of r.list) {
      const li = el("li", "caught-item");
      const what = el("div", "caught-what");
      const kind = tr(KINDS[v.kind]);
      what.append(el("div", "t", v.height ? `${kind} · ${tr("up to {height}p", { height: v.height })}` : kind), el("div", "muted", v.host));
      const go = el("button", "secondary", tr("Download with Pako"));
      go.addEventListener("click", () => sendVideoStream(v.url, go));
      li.append(what, go);
      ul.append(li);
    }
  }
  if (!r.list.length && r.playing && !r.protected) ul.append(el("li", "muted", tr("Pako didn't find this video's address.")));
  renderRecording(r, busy);
  // While recording, the window follows it.
  if (busy && !watching) watching = setInterval(showVideos, 1000);
  if (!busy && watching) {
    clearInterval(watching);
    watching = null;
  }
}

/** "Record this video": offered whenever a video plays or was found on the page (Pako may have
 *  failed to get it after taking it: the small window can't know), or Pako refused it. Never
 *  for a protected one. */
function renderRecording(r, busy) {
  const s = r.recording;
  const offer = !r.protected && (sendFailed || r.playing || r.list.length > 0);
  $("rec").hidden = !(busy || s || offer) || r.protected;
  $("rec-start").hidden = busy || !offer;
  $("rec-cancel").hidden = !busy || s?.state === "saving";
  const mb = s ? Math.round((s.bytes / (1024 * 1024)) * 10) / 10 : 0;
  const text = {
    waiting: () => tr("The page reloaded. Waiting for the video to play: press play if it doesn't start. It has to play to the end."),
    recording: () => tr("Recording: {mb} MB. Keep this tab open: the video has to play to the end (sped up and muted when the page allows).", { mb: WORDS ? WORDS.num(lang, mb) : mb }),
    saving: () => tr("Sending the recording to Pako…"),
    done: () => tr("Sent to Pako: it makes the video in Downloads."),
  };
  const lastResort = () => tr("The last resort when Pako can't get it: Pako records the video while it plays. The page reloads, and the video has to play to the end.");
  if (s && text[s.state]) show($("rec-text"), text[s.state]);
  else if (s?.state === "failed" && !offer) show($("rec-text"), () => trMaybe(s.error));
  else show($("rec-text"), () => `${s?.state === "failed" ? trMaybe(s.error) + " " : ""}${lastResort()}`);
}

async function sendVideoStream(url, button) {
  button.disabled = true;
  let reply;
  try {
    reply = await api.runtime.sendMessage({ type: "send-stream", tabId: videoTab, url });
  } catch (e) {
    reply = { ok: false, error: "Couldn't send this video.", detail: e?.message || String(e) };
  }
  if (reply?.ok) {
    button.textContent = tr("Sent to Pako ✓");
    return;
  }
  button.disabled = false;
  showError(reply?.error || "Couldn't send this video.");
  showDetail(reply?.detail);
  if (reply?.error !== PROTECTED && reply?.error !== tr(PROTECTED)) sendFailed = true;
  showVideos();
}

$("rec-start").addEventListener("click", async () => {
  $("rec-start").disabled = true;
  showError("");
  showDetail("");
  let reply;
  try {
    reply = await api.runtime.sendMessage({ type: "record-start", tabId: videoTab });
  } catch (e) {
    reply = { ok: false, error: "Couldn't start recording.", detail: e?.message || String(e) };
  }
  $("rec-start").disabled = false;
  if (!reply?.ok) {
    showError(reply?.error || "Couldn't start recording.");
    showDetail(reply?.detail);
  }
  showVideos();
});

$("rec-cancel").addEventListener("click", async () => {
  await api.runtime.sendMessage({ type: "record-cancel", tabId: videoTab }).catch(() => {});
  showVideos();
});
/* full-only:end */

/** Opens a pako:// link: in the current tab when it's a web page (it stays as it is, the
 *  browser only asks to open Pako), else in a new tab. */
async function openPako() {
  const [tab] = await api.tabs.query({ active: true, currentWindow: true });
  try {
    if (tab?.id != null && /^https?:/i.test(tab.url || "")) await api.tabs.update(tab.id, { url: CONNECT_LINK });
    else await api.tabs.create({ url: CONNECT_LINK });
  } catch {
    await api.tabs.create({ url: CONNECT_LINK });
  }
}

$("open").addEventListener("click", async () => {
  $("open").disabled = true;
  try {
    await openPako();
    show($("open"), () => tr("Opening Pako…"));
    show($("connect-text"), () => tr("When Pako says Connected, come back and open this popup again."));
  } catch (e) {
    $("open").disabled = false;
    showError("Couldn't open Pako. Open it yourself: Settings › Browser extension › Connect Pako to your browsers.");
    showDetail(e?.message || String(e));
  }
});

/* full-only:start */
$("player").addEventListener("click", async () => {
  const on = $("player").getAttribute("aria-checked") !== "true";
  $("player").setAttribute("aria-checked", String(on));
  await api.storage.local.set({ playerButton: on });
});

// "Catch download links on pages": only in a browser that asks before each download (Vivaldi).
async function showLinks() {
  const r = await api.runtime.sendMessage({ type: "asks-first" }).catch(() => null);
  if (!r?.asks) return;
  const { catchLinks = true } = await api.storage.local.get({ catchLinks: true });
  $("links").setAttribute("aria-checked", String(catchLinks !== false));
  $("links-line").hidden = false;
}
$("links").addEventListener("click", async () => {
  const on = $("links").getAttribute("aria-checked") !== "true";
  $("links").setAttribute("aria-checked", String(on));
  await api.storage.local.set({ catchLinks: on });
});
if (!LIGHT) showLinks();
/* full-only:end */

// Private windows: the browser's own switch for Pako decides ("Allow in Incognito", "Allow in
// InPrivate", Firefox's "Run in Private Windows"). Only said here, with a way to that switch.
async function showPrivate() {
  const r = await api.runtime.sendMessage({ type: "private" }).catch(() => null);
  if (!r) return;
  const firefox = !!globalThis.browser?.runtime?.getBrowserInfo;
  const edge = /Edg\//.test(navigator.userAgent);
  show($("private-state"), () => tr(r.allowed ? "on" : "off"));
  show($("private-text"), () =>
    r.allowed
      ? tr("Pako keeps no trace of those downloads beyond the file.")
      : firefox
        ? tr("To use Pako there: Firefox menu › Add-ons and themes › Pako › Run in Private Windows › Allow.")
        : tr('To use Pako there, turn on "{name}" on this extension\'s page.', { name: tr(edge ? "Allow in InPrivate" : "Allow in Incognito") }),
  );
  // Firefox doesn't let an extension open its add-ons page.
  $("private-open").hidden = r.allowed || firefox;
  $("private-line").hidden = false;
}
$("private-open").addEventListener("click", () => {
  const edge = /Edg\//.test(navigator.userAgent);
  api.tabs.create({ url: `${edge ? "edge" : "chrome"}://extensions/?id=${api.runtime.id}` });
  window.close();
});
showPrivate();

$("take").addEventListener("click", async () => {
  const on = $("take").getAttribute("aria-checked") !== "true";
  $("take").setAttribute("aria-checked", String(on));
  await api.storage.local.set({ takeOver: on });
});

$("send").addEventListener("click", async () => {
  const [tab] = await api.tabs.query({ active: true, currentWindow: true });
  $("send").disabled = true;
  let reply;
  try {
    reply = await api.runtime.sendMessage({ type: "send-tab", url: tab?.url, incognito: !!tab?.incognito, storeId: tab?.cookieStoreId });
  } catch (e) {
    reply = { ok: false, error: "Couldn't send this page.", detail: e?.message || String(e) };
  }
  $("send").disabled = false;
  if (reply?.ok) {
    show($("send"), () => tr("Sent to Pako ✓"));
    setTimeout(() => window.close(), 700);
  } else {
    showError(reply?.error || "Couldn't send this page.");
    showDetail(reply?.detail);
  }
});

init();
