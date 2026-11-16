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

function showError(text) {
  $("error").textContent = text || "";
  $("error").hidden = !text;
}

/** The browser's own error text, in small grey print under the status. */
function showDetail(text) {
  $("detail").textContent = text ? `Details: ${text}` : "";
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
 *  reloaded, or no answer at all), the popup asks Pako's helper itself. */
async function ping() {
  let viaBackground;
  try {
    viaBackground = await api.runtime.sendMessage({ type: "ping" });
  } catch (e) {
    console.warn("Pako: the background page didn't answer:", e?.message || String(e));
  }
  if (viaBackground && typeof viaBackground === "object" && (viaBackground.ok || viaBackground.detail)) return viaBackground;
  try {
    const reply = await nativePing();
    return reply && typeof reply === "object" ? reply : { ok: false, error: "Pako didn't answer.", detail: `No answer from ${HOST}` };
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
    $("status").textContent = "Not in use";
    return true;
  }
  if (edition?.skipped) {
    $("send").disabled = true;
    $("light-hint").textContent = "Pako Light doesn't work on this site.";
  }
  return false;
}

async function init() {
  const { takeOver = true, lastError = "", lastDetail = "" } = await api.storage.local.get({
    takeOver: true,
    lastError: "",
    lastDetail: "",
  });
  $("take").setAttribute("aria-checked", String(takeOver));
  /* full-only:start */
  const { playerButton = true } = await api.storage.local.get({ playerButton: true });
  $("player").setAttribute("aria-checked", String(playerButton));
  /* full-only:end */
  showError(lastError);
  showDetail(lastDetail);
  // Seen: clear the "!" on the toolbar button.
  api.action.setBadgeText({ text: "" });
  await api.storage.local.set({ lastError: "", lastDetail: "" });

  if (await lightEdition()) return;

  const reply = await ping();
  const status = $("status");
  if (reply?.ok) {
    status.textContent = "Connected";
    status.className = "status ok";
  } else {
    status.textContent = "Not connected";
    status.className = "status bad";
    if (/forbidden/i.test(reply?.detail || "")) {
      // An older Pako doesn't know this extension yet: "Open Pako" can't fix that, an update can.
      $("connect").hidden = false;
      $("main").hidden = true;
      $("open").hidden = true;
      $("get").hidden = true; // Pako is on this PC: nothing to get
      $("connect-title").textContent = "This Pako is too old for this extension";
      $("connect-text").textContent = "Update Pako (Settings › Features & updates), then open this again.";
      if ($("error").textContent === NOT_CONNECTED) showError("");
    } else if (!reply?.error || reply.error === NOT_CONNECTED) {
      // The line to Pako itself is missing: one button opens Pako and connects it.
      $("connect").hidden = false;
      $("main").hidden = true;
      if ($("error").textContent === NOT_CONNECTED) showError("");
    } else {
      showError(reply.error);
    }
    showDetail(reply?.detail);
  }
}

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
    $("open").textContent = "Opening Pako…";
    $("connect-text").textContent = "When Pako says Connected, come back and open this popup again.";
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
/* full-only:end */

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
    $("send").textContent = "Sent to Pako ✓";
    setTimeout(() => window.close(), 700);
  } else {
    showError(reply?.error || "Couldn't send this page.");
    showDetail(reply?.detail);
  }
});

init();
