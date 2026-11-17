// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// "Record this video" (full extension only, never in Pako Light): the line between recorder.js
// (inside the page) and the extension. It asks the extension whether this tab is the one being
// recorded; if not, it tells the recorder to let go and does nothing else. If so, it passes the
// recorded parts on (a few at a time, holding the video back while Pako catches up).
(() => {
  // (Not just `globalThis.browser`: a page element with id="browser" answers to that name too.)
  const api = globalThis.browser?.runtime ? globalThis.browser : globalThis.chrome;
  const TAG = "pako-recorder";
  /** Parts on their way to Pako at once. */
  const IN_FLIGHT = 4;
  /** Waiting parts that pause the video, and that let it play again. */
  const HOLD_ON = 48;
  const HOLD_OFF = 8;

  const toPage = (m) => window.postMessage({ [TAG]: "bridge", ...m }, "*");
  let port = null;
  let early = [];
  const queue = [];
  let inFlight = 0;
  let slow = false;
  let ended = false;

  function base64(bytes) {
    let s = "";
    for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
    return btoa(s);
  }

  function pump() {
    if (!port) return;
    while (inFlight < IN_FLIGHT && queue.length) {
      port.postMessage(queue.shift());
      inFlight++;
    }
    if (!slow && queue.length > HOLD_ON) toPage({ t: "slow", on: (slow = true) });
    else if (slow && queue.length < HOLD_OFF) toPage({ t: "slow", on: (slow = false) });
    if (ended && !queue.length && !inFlight) {
      port.postMessage({ t: "end" });
      ended = false;
    }
  }

  function fromPage(m) {
    if (m.t === "chunk" && m.data) {
      const bytes = new Uint8Array(m.data);
      queue.push({ t: "chunk", track: Number(m.track), mime: String(m.mime || ""), size: bytes.length, data: base64(bytes) });
    } else if (m.t === "protected") {
      port.postMessage({ t: "protected" });
    } else if (m.t === "end") {
      ended = true;
    }
    pump();
  }

  window.addEventListener("message", (e) => {
    const m = e.data;
    if (e.source !== window || !m || m[TAG] !== "page") return;
    if (port) fromPage(m);
    else if (early) early.push(m);
  });

  Promise.resolve(api.runtime.sendMessage({ type: "rec-hello" }))
    .then((reply) => {
      const on = reply?.record === true;
      toPage({ t: "arm", on });
      if (!on) {
        early = null;
        return;
      }
      port = api.runtime.connect({ name: "pako-rec" });
      port.onMessage.addListener((m) => {
        if (m?.t === "ack") {
          inFlight = Math.max(0, inFlight - 1);
          pump();
        } else if (m?.t === "stop") {
          toPage({ t: "stop" });
          queue.length = 0;
          port?.disconnect();
          port = null;
        }
      });
      port.onDisconnect.addListener(() => {
        port = null;
        toPage({ t: "stop" });
      });
      const first = early || [];
      early = null;
      for (const m of first) fromPage(m);
    })
    .catch(() => toPage({ t: "arm", on: false }));
})();
