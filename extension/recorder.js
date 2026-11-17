// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// "Record this video" (full extension only, never in Pako Light): runs inside the page itself,
// only while you record a video (Pako's extension adds it for that, then takes it away again).
// It copies what the page gives its own video player (each SourceBuffer's data: picture and
// sound apart, with their first "init" part), and hands it to recorder-bridge.js, which sends
// it to Pako. Until the bridge says this is the tab being recorded, it only holds on to what it
// saw; anywhere else it lets go of it and does nothing more.
// A protected video (the page asks for a key system: DRM) is never recorded.
(() => {
  if (window.__pakoRecorder) return;
  window.__pakoRecorder = true;
  const TAG = "pako-recorder";
  /** Biggest part sent at once (Pako's line takes well under 1 MB per message). */
  const PART = 512 * 1024;
  /** Held at most before the bridge answers (then it lets go and stops). */
  const HOLD_MAX = 256 * 1024 * 1024;
  /** Tracks one recording takes (picture, sound and spares). */
  const TRACKS = 4;

  let armed = null; // null = not told yet; true = record; false = not this tab
  let held = [];
  let heldBytes = 0;
  let stopped = false;
  let slow = false;
  let nextTrack = 0;
  const tracks = new WeakMap();

  const toBridge = (m, transfer) => window.postMessage({ [TAG]: "page", ...m }, "*", transfer || []);

  function out(m) {
    if (stopped || armed === false) return;
    if (armed) {
      toBridge(m, m.data ? [m.data.buffer] : []);
      return;
    }
    heldBytes += m.data?.byteLength || 0;
    if (heldBytes > HOLD_MAX) {
      held = [];
      stopped = true;
      return;
    }
    held.push(m);
  }

  /** A copy of what the page appended, in parts. */
  function capture(sb, data) {
    const t = tracks.get(sb);
    if (!t || stopped || armed === false || !data) return;
    let bytes;
    if (data instanceof ArrayBuffer) bytes = new Uint8Array(data);
    else if (ArrayBuffer.isView(data)) bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    else return;
    for (let i = 0; i < bytes.byteLength; i += PART) {
      out({ t: "chunk", track: t.id, mime: t.mime, data: bytes.slice(i, i + PART) });
    }
  }

  const Source = window.MediaSource || window.ManagedMediaSource;
  const Buffer = window.SourceBuffer || window.ManagedSourceBuffer;
  if (Source && Buffer) {
    const add = Source.prototype.addSourceBuffer;
    Source.prototype.addSourceBuffer = function (mime) {
      const sb = add.call(this, mime);
      try {
        if (nextTrack < TRACKS) tracks.set(sb, { id: nextTrack++, mime: String(mime) });
      } catch {
        // never in the page's way
      }
      return sb;
    };
    const append = Buffer.prototype.appendBuffer;
    Buffer.prototype.appendBuffer = function (data) {
      // The player's own call first: one the browser refuses (its buffer is full, or it is
      // still busy) throws, and the player sends the same data again later, so only what the
      // browser took is copied (else that part would be in the recording twice).
      const r = append.call(this, data);
      try {
        capture(this, data);
      } catch {
        // never in the page's way
      }
      return r;
    };
    const change = Buffer.prototype.changeType;
    if (change) {
      Buffer.prototype.changeType = function (mime) {
        const t = tracks.get(this);
        if (t) t.mime = String(mime);
        return change.call(this, mime);
      };
    }
  }

  // A key system (DRM): this video is protected. Said once; the page carries on as it was.
  let locked = false;
  const protect = () => {
    if (locked) return;
    locked = true;
    held = [];
    toBridge({ t: "protected" });
  };
  const keys = navigator.requestMediaKeySystemAccess;
  if (keys) {
    navigator.requestMediaKeySystemAccess = function (...args) {
      protect();
      return keys.apply(this, args);
    };
  }
  document.addEventListener("encrypted", protect, true);

  // ---------- playing it through, as fast as the page lets it ----------

  /** The fastest speed this video takes (browsers allow up to 16 times). */
  function fastest(v) {
    for (const rate of [16, 8, 4, 2]) {
      try {
        v.playbackRate = rate;
        if (v.playbackRate === rate) return;
      } catch {
        // too fast for this browser: try the next
      }
    }
  }

  function drive() {
    if (stopped || !armed) return;
    for (const v of document.querySelectorAll("video")) {
      if (!v.src?.startsWith("blob:") && !v.srcObject) continue;
      v.muted = true;
      if (slow) {
        if (!v.paused) v.pause();
        continue;
      }
      if (v.playbackRate < 2) fastest(v);
      if (v.paused && !v.ended && v.readyState >= 1) v.play().catch(() => {});
    }
  }

  document.addEventListener(
    "ended",
    (e) => {
      // Only a video the page builds from parts (the one being recorded): an advert or a
      // preview playing from a plain file ending doesn't end the recording.
      const v = e.target;
      if (armed && !stopped && v instanceof HTMLVideoElement && ((v.currentSrc || v.src || "").startsWith("blob:") || v.srcObject)) toBridge({ t: "end" });
    },
    true,
  );

  window.addEventListener("message", (e) => {
    const m = e.data;
    if (e.source !== window || !m || m[TAG] !== "bridge") return;
    if (m.t === "arm") {
      armed = !!m.on;
      const list = held;
      held = [];
      heldBytes = 0;
      if (!armed) return;
      if (locked) toBridge({ t: "protected" });
      for (const x of list) toBridge(x, x.data ? [x.data.buffer] : []);
      setInterval(drive, 500);
    } else if (m.t === "slow") {
      slow = !!m.on;
      drive();
    } else if (m.t === "stop") {
      stopped = true;
      held = [];
      for (const v of document.querySelectorAll("video")) {
        try {
          v.playbackRate = 1;
        } catch {
          // left as it is
        }
      }
    }
  });
})();
