// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The Download pill on a page where the full extension caught a video (Videos on this page).
// Not a content script: background.js puts it (after pill.js) into the frame that loaded the
// video only once it caught one there, once per page. It reads nothing from the page but its
// <video> elements, their size and place, and whether one plays (to pick which gets the pill).
// Its menu comes from background.js: the videos caught in this tab and Record this video.
// On the video sites Pako knows, player.js has the pill and this one steps aside.
(() => {
  if (globalThis.__pakoCaught) return;
  const kit = globalThis.__pakoPill;
  if (!kit) return;
  globalThis.__pakoCaught = true;

  // (pill.js's choice: never a page element named "browser".)
  const api = kit.api;

  /** The video that gets the pill: the largest playing one in this frame, else the largest one
   *  on screen (thumbnails and previews are too small). */
  function target() {
    const vw = document.documentElement.clientWidth || innerWidth;
    const vh = document.documentElement.clientHeight || innerHeight;
    let best = null;
    let bestScore = 0;
    for (const v of document.getElementsByTagName("video")) {
      const r = v.getBoundingClientRect();
      if (!kit.bigEnough(r) || r.bottom <= 0 || r.right <= 0 || r.top >= vh || r.left >= vw) continue;
      const playing = !v.paused && !v.ended;
      // Playing first, then the bigger.
      const score = (playing ? 1e12 : 0) + r.width * r.height;
      if (score > bestScore) {
        best = v;
        bestScore = score;
      }
    }
    return best;
  }

  kit.make({
    find: (x, y) => {
      if (globalThis.__pakoPlayer) return null;
      const v = target();
      return v && kit.inside(v.getBoundingClientRect(), x, y) ? v : null;
    },
    items: async () => {
      let r;
      try {
        r = await api.runtime.sendMessage({ type: "pill-menu" });
      } catch (e) {
        const t = kit.trouble(e);
        return [{ head: kit.tr("Download with Pako") }, { info: t.text, detail: t.detail }];
      }
      if (r?.ok && Array.isArray(r.items)) return r.items;
      // The extension answered with a problem: what it said (it did answer, so not "isn't
      // responding"). No answer at all: the plain words, kept for the toolbar window.
      if (typeof r?.error === "string" && r.error) return [{ head: kit.tr("Download with Pako") }, { info: r.error, detail: String(r.detail || "") }];
      const t = kit.trouble(new Error("No menu in the answer"));
      return [{ head: kit.tr("Download with Pako") }, { info: t.text, detail: t.detail }];
    },
    pick: (item) => (item.send ? api.runtime.sendMessage(item.send) : null),
  });
})();
