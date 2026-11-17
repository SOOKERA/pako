// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's "Download" pill on a page's video (full extension only): shown at the top right of the
// video while the mouse is over it, hidden again when the mouse rests or leaves. It lives in its
// own closed shadow root so the page's styles can't touch it (nor it the page's). It reads nothing
// from the page but its <video> elements and their size and place on screen.
// Used by player.js (the video sites Pako knows) and caught.js (a page where the extension caught
// a video): each says which video gets the pill, what its menu offers and what a click does.
(() => {
  if (globalThis.__pakoPill) return;

  const api = globalThis.browser ?? globalThis.chrome;
  /** Smaller videos (thumbnails, previews) get no pill. */
  const MIN_W = 240;
  const MIN_H = 135;
  /** The pill hides after this long without the mouse moving (like the player's own controls). */
  const IDLE_MS = 2600;
  const LABEL = "Download";
  const SENT = "Sent to Pako ✓";

  const bigEnough = (r) => r.width >= MIN_W && r.height >= MIN_H;
  const inside = (r, x, y) => x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;

  /** The biggest video on screen under the mouse. */
  function videoAt(x, y) {
    let best = null;
    let area = 0;
    for (const v of document.getElementsByTagName("video")) {
      const r = v.getBoundingClientRect();
      if (!bigEnough(r) || !inside(r, x, y)) continue;
      if (r.width * r.height > area) {
        best = v;
        area = r.width * r.height;
      }
    }
    return best;
  }

  const CSS = `
    :host { all: initial; }
    .wrap {
      position: fixed;
      display: flex;
      flex-direction: column;
      align-items: flex-end;
      gap: 6px;
      font: 13px/1.3 "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
      opacity: 0;
      transform: translateY(-4px);
      pointer-events: none;
      transition: opacity 140ms ease, transform 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }
    .wrap.on { opacity: 1; transform: none; pointer-events: auto; }
    button { font: inherit; margin: 0; cursor: pointer; }
    .main {
      display: inline-flex;
      align-items: center;
      gap: 7px;
      height: 32px;
      padding: 0 13px 0 10px;
      border: none;
      border-radius: 8px;
      background: #f96263;
      color: #17181c;
      font-weight: 600;
      white-space: nowrap;
      box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35), 0 1px 3px rgba(0, 0, 0, 0.25);
      transition: background 120ms ease, transform 120ms ease;
    }
    .main:hover { background: #ff7676; }
    .main:active { transform: scale(0.96); }
    .main:focus-visible { outline: 2px solid #7db4ff; outline-offset: 2px; }
    .item:focus-visible { outline: 2px solid #7db4ff; outline-offset: -2px; }
    .main[aria-busy="true"] { cursor: default; }
    .main svg { flex: none; display: block; }
    .menu {
      display: none;
      min-width: 188px;
      max-width: 320px;
      padding: 4px;
      border: 1px solid #343a44;
      border-radius: 10px;
      background: #1a1d22;
      color: #eceef1;
      box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3);
    }
    .menu.open { display: block; }
    .head {
      padding: 6px 8px 4px;
      font-size: 11px;
      font-weight: 600;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      color: #8e95a1;
    }
    .item {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      width: 100%;
      height: 30px;
      padding: 0 8px;
      border: none;
      border-radius: 6px;
      background: transparent;
      color: #eceef1;
      text-align: left;
    }
    .item:hover { background: #262a31; }
    .item > span { white-space: nowrap; }
    .item .fmt {
      flex: 0 1 auto;
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      font-size: 11.5px;
      color: #8e95a1;
    }
    .sep { height: 1px; margin: 4px 4px; background: #262a31; }
    .info {
      max-width: 240px;
      padding: 4px 8px 6px;
      font-size: 12px;
      line-height: 1.4;
      color: #8e95a1;
    }
    .note {
      display: none;
      max-width: 220px;
      margin: 4px 4px 2px;
      padding: 6px 4px 2px;
      border-top: 1px solid #262a31;
      font-size: 12px;
      line-height: 1.4;
      color: #ff7a7d;
    }
    .note.on { display: block; }
  `;

  const SVG = "http://www.w3.org/2000/svg";

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text) e.textContent = text;
    return e;
  }

  /** Pako's box with its arrow, drawn in the pill's ink colour. */
  function icon() {
    const svg = document.createElementNS(SVG, "svg");
    svg.setAttribute("width", "16");
    svg.setAttribute("height", "16");
    svg.setAttribute("viewBox", "4 4 56 56");
    svg.setAttribute("aria-hidden", "true");
    const box = document.createElementNS(SVG, "path");
    box.setAttribute(
      "d",
      "M30.257 6.98 Q32 6 33.743 6.98 L54.257 18.52 Q56 19.5 56 21.5 L56 42.5 Q56 44.5 54.257 45.48 L33.743 57.02 Q32 58 30.257 57.02 L9.743 45.48 Q8 44.5 8 42.5 L8 21.5 Q8 19.5 9.743 18.52 Z",
    );
    box.setAttribute("fill", "#17181c");
    const arrow = document.createElementNS(SVG, "path");
    arrow.setAttribute("d", "M28.5 18 H35.5 V31 H42 L32 44 L22 31 H28.5 Z");
    arrow.setAttribute("fill", "#f96263");
    svg.append(box, arrow);
    return svg;
  }

  /**
   * One pill for this frame.
   * - `find(x, y)`: the <video> the pill goes on with the mouse at x, y, or null.
   * - `items(video)`: the menu, an array (or a promise of one) of
   *   `{ head }` (a small title), `{ text, side, ... }` (an item; `side` = grey text on its right),
   *   `{ sep: true }` (a line) or `{ info }` (grey text).
   * - `pick(item, video)`: what a click on an item does; answers `{ ok, error?, detail? }`
   *   (null: nothing was done). On success the pill says the item's `done`, else "Sent to Pako ✓".
   * Returns `{ hide }`: hides the pill unless it is sending.
   */
  function make({ find, items, pick }) {
    let host, wrap, main, label, menu, note;
    let enabled = true;
    let current = null;
    let shown = false;
    let menuOpen = false;
    let overButton = false;
    let busy = false;
    let lastX = -1;
    let lastY = -1;
    let raf = 0;
    let idleTimer = 0;
    let resetTimer = 0;
    /** Each menu opening; an answer for an older one is dropped. */
    let opening = 0;

    function build() {
      if (host) return;
      host = document.createElement("pako-download-button");
      host.style.cssText = "all: initial; position: fixed; top: 0; left: 0; width: 0; height: 0; z-index: 2147483647;";
      const root = host.attachShadow({ mode: "closed" });
      let styled = false;
      try {
        const sheet = new CSSStyleSheet();
        sheet.replaceSync(CSS);
        root.adoptedStyleSheets = [sheet];
        styled = true;
      } catch {
        // Not supported here: a plain style element instead.
      }
      if (!styled) root.append(el("style", "", CSS));

      wrap = el("div", "wrap");
      main = el("button", "main");
      main.type = "button";
      main.setAttribute("aria-haspopup", "menu");
      main.setAttribute("aria-expanded", "false");
      main.title = "Download with Pako";
      label = el("span", "", LABEL);
      main.append(icon(), label);
      menu = el("div", "menu");
      menu.setAttribute("role", "menu");
      note = el("div", "note");
      note.setAttribute("role", "status");
      menu.append(note);
      wrap.append(main, menu);
      root.append(wrap);

      main.addEventListener("click", () => {
        if (!busy) setMenu(!menuOpen);
      });
      wrap.addEventListener("mouseenter", () => (overButton = true));
      wrap.addEventListener("mouseleave", () => {
        overButton = false;
        restartIdle();
      });
      // Keys and clicks on the pill stay with it (the player's shortcuts don't fire).
      for (const type of ["keydown", "keyup", "keypress", "click", "dblclick", "mousedown", "mouseup", "pointerdown", "pointerup"]) {
        host.addEventListener(type, (e) => {
          if (type === "keydown" && e.key === "Escape" && menuOpen) setMenu(false);
          e.stopPropagation();
        });
      }
      attach();
    }

    /** In full screen only the full-screen element shows: the pill goes inside it. */
    function attach() {
      if (!host) return;
      const fs = document.fullscreenElement;
      const parent = fs && fs.tagName !== "VIDEO" ? fs : document.documentElement;
      if (host.parentNode !== parent) parent.append(host);
    }

    /** The menu's entries (the note for errors stays last). */
    function fill(list) {
      const nodes = [];
      for (const it of list || []) {
        if (it.head) nodes.push(el("div", "head", it.head));
        else if (it.sep) nodes.push(el("div", "sep"));
        else if (it.info) nodes.push(el("div", "info", it.info));
        else if (it.text) {
          const item = el("button", "item");
          item.type = "button";
          item.setAttribute("role", "menuitem");
          item.append(el("span", "", it.text));
          if (it.side) {
            const side = el("span", "fmt", it.side);
            side.title = it.side;
            item.append(side);
          }
          item.addEventListener("click", () => choose(it));
          nodes.push(item);
        }
      }
      menu.replaceChildren(...nodes, note);
    }

    function openMenu() {
      const mine = ++opening;
      const video = current;
      let list;
      try {
        list = items(video);
      } catch {
        list = null;
      }
      if (list && typeof list.then === "function") {
        fill([{ head: "Download with Pako" }, { info: "…" }]);
        list.then(
          (l) => mine === opening && menuOpen && fill(l),
          () => mine === opening && menuOpen && fill([{ head: "Download with Pako" }, { info: "Pako's extension was just updated. Reload this page and try again." }]),
        );
      } else fill(list);
    }

    function place() {
      if (!current || !current.isConnected) return hide();
      const r = current.getBoundingClientRect();
      const vw = document.documentElement.clientWidth || innerWidth;
      wrap.style.top = `${Math.max(r.top, 0) + 12}px`;
      wrap.style.right = `${Math.max(vw - r.right, 0) + 12}px`;
    }

    function show() {
      build();
      attach();
      place();
      if (!shown) {
        shown = true;
        wrap.classList.add("on");
      }
      restartIdle();
    }

    function hide() {
      clearTimeout(idleTimer);
      if (!shown) return;
      shown = false;
      setMenu(false);
      wrap.classList.remove("on");
    }

    function restartIdle() {
      clearTimeout(idleTimer);
      idleTimer = setTimeout(() => {
        if (!menuOpen && !overButton && !busy) hide();
      }, IDLE_MS);
    }

    function setMenu(open) {
      if (!menu) return;
      const opened = open && !menuOpen;
      menuOpen = open;
      if (opened) openMenu();
      menu.classList.toggle("open", open);
      main.setAttribute("aria-expanded", String(open));
      if (open) clearTimeout(idleTimer);
      else {
        opening++;
        setNote("");
        if (shown) restartIdle();
      }
    }

    function setNote(text) {
      note.textContent = text;
      note.classList.toggle("on", !!text);
    }

    function check() {
      raf = 0;
      if (!enabled) return hide();
      if (menuOpen || busy) return;
      const v = find(lastX, lastY);
      if (v) {
        current = v;
        show();
      } else if (!overButton) {
        hide();
      }
    }

    document.addEventListener(
      "mousemove",
      (e) => {
        lastX = e.clientX;
        lastY = e.clientY;
        if (!raf) raf = requestAnimationFrame(check);
      },
      { capture: true, passive: true },
    );
    // The mouse left the window.
    document.addEventListener("mouseout", (e) => {
      if (!e.relatedTarget && !menuOpen && !busy) hide();
    });
    // A click elsewhere closes the menu.
    document.addEventListener(
      "pointerdown",
      (e) => {
        if (menuOpen && host && !e.composedPath().includes(host)) setMenu(false);
      },
      { capture: true },
    );
    addEventListener("scroll", () => shown && place(), { capture: true, passive: true });
    addEventListener("resize", () => shown && place(), { passive: true });
    document.addEventListener("fullscreenchange", () => {
      attach();
      if (shown) place();
    });

    // ---------- sending ----------

    function setLabel(text) {
      label.textContent = text;
    }

    async function choose(item) {
      if (busy) return;
      setMenu(false);
      busy = true;
      clearTimeout(resetTimer);
      main.setAttribute("aria-busy", "true");
      setLabel("Sending…");
      let reply;
      try {
        reply = await pick(item, current);
      } catch {
        reply = { ok: false, error: "Pako's extension was just updated. Reload this page and try again." };
      }
      busy = false;
      main.removeAttribute("aria-busy");
      if (reply === null) {
        setLabel(LABEL);
        restartIdle();
      } else if (reply?.ok) {
        setLabel(item.done || SENT);
        restartIdle();
        resetTimer = setTimeout(() => setLabel(LABEL), 2500);
      } else {
        setLabel(LABEL);
        if (!shown) show();
        setMenu(true);
        // The browser's own words too, when the line to Pako itself failed.
        const detail = reply?.detail ? ` (Browser says: ${reply.detail})` : "";
        setNote((reply?.error || "Couldn't send this video to Pako.") + detail);
      }
    }

    // ---------- the popup's "Download button on videos" switch ----------

    api.storage.local
      .get({ playerButton: true })
      .then((s) => {
        enabled = s.playerButton !== false;
      })
      .catch(() => {});
    api.storage.onChanged.addListener((changes, area) => {
      if (area !== "local" || !changes.playerButton) return;
      enabled = changes.playerButton.newValue !== false;
      if (!enabled) hide();
    });

    return {
      hide: () => {
        if (!busy) hide();
      },
    };
  }

  globalThis.__pakoPill = { MIN_W, MIN_H, bigEnough, inside, videoAt, make };
})();
