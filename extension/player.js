// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's "Download" button on video players (YouTube, Vimeo, Twitch, Facebook, Instagram), shown
// at the top right of the video while the mouse is over it. It reads the page's address and, on
// Facebook and Instagram (where a page shows many videos), the links of the post around the
// video. It lives in its own closed shadow root so the site's styles can't touch it (nor it the
// site's).
// Sites that change pages without reloading (YouTube) are handled by reading the address at
// each hover and at each click.
(() => {
  if (globalThis.__pakoPlayer) return;
  globalThis.__pakoPlayer = true;

  const api = globalThis.browser ?? globalThis.chrome;
  const QUALITIES = [
    [1080, "Up to 1080p", "MP4"],
    [720, "Up to 720p", "MP4"],
    [480, "Up to 480p", "MP4"],
    [0, "Audio only", "M4A"],
  ];
  /** Smaller videos (thumbnails, previews) get no button. */
  const MIN_W = 240;
  const MIN_H = 135;
  /** The button hides after this long without the mouse moving (like the player's own controls). */
  const IDLE_MS = 2600;
  const LABEL = "Download";

  // ---------- which video, as a link Pako knows ----------

  const YT_ID = /^[\w-]{6,20}$/;

  // Facebook and Instagram: a feed shows many videos, so the link comes from the post around the
  // video (its date or "Reels" link), else from the page's address (a post or reel opened alone).
  const IG_POST = /^\/(?:[\w.]+\/)?(p|reel|reels|tv)\/([\w-]{5,})(?:\/|$)/;
  function instagramLink(u) {
    const m = IG_POST.exec(u.pathname);
    return m ? `https://www.instagram.com/${m[1] === "reels" ? "reel" : m[1]}/${m[2]}/` : null;
  }
  function facebookLink(u) {
    const p = u.pathname;
    const v = u.searchParams.get("v") ?? "";
    let m;
    if ((p === "/watch" || p === "/watch/" || p === "/video.php") && /^\d+$/.test(v)) return `https://www.facebook.com/watch/?v=${v}`;
    if ((m = /^\/reels?\/(\d+)/.exec(p))) return `https://www.facebook.com/reel/${m[1]}`;
    if ((m = /^\/[^/]+\/videos\/(?:[^/]+\/)?(\d+)/.exec(p))) return `https://www.facebook.com/watch/?v=${m[1]}`;
    if ((m = /^\/watch\/live\/?$/.exec(p)) && /^\d+$/.test(v)) return `https://www.facebook.com/watch/?v=${v}`;
    return null;
  }
  /** The post around this video: Instagram's <article>, Facebook's post box. */
  function postOf(video, host) {
    if (host === "instagram.com") return video.closest("article");
    return video.closest('[role="article"], [aria-posinset], [data-pagelet^="FeedUnit"]');
  }
  /** The post's own link to its video: links inside a comment or a shared post within it
   *  (their own article) are skipped, and the post's time stamp / permalink comes first. */
  function linkInPost(video, host, read) {
    const post = video && postOf(video, host);
    if (!post) return null;
    const nested = (a) => {
      const inner = a.parentElement?.closest('[role="article"], article');
      return !!inner && inner !== post && post.contains(inner);
    };
    // A time stamp link: holds a <time> / <abbr>, or is labelled with the date.
    const stamp = (a) => !!a.querySelector("time, abbr") || (a.getAttribute("aria-label") || "").length > 0;
    let first = null;
    for (const a of post.querySelectorAll("a[href]")) {
      if (nested(a)) continue;
      let u;
      try {
        u = new URL(a.href, location.href);
      } catch {
        continue;
      }
      if (u.hostname.replace(/^(www|web|m)\./, "") !== host) continue;
      const link = read(u);
      if (!link) continue;
      if (stamp(a)) return link;
      first ??= link;
    }
    return first;
  }

  /** The video as a clean link (no playlist, no time), or null when it isn't one Pako knows.
   *  `video` is the <video> under the mouse (used where a page shows many videos). */
  function videoUrl(video) {
    let u;
    try {
      u = new URL(location.href);
    } catch {
      return null;
    }
    const social = u.hostname.replace(/^(www|web|m)\./, "");
    if (social === "instagram.com") return linkInPost(video, social, instagramLink) ?? instagramLink(u);
    if (social === "facebook.com") return linkInPost(video, social, facebookLink) ?? facebookLink(u);
    const host = u.hostname.replace(/^www\./, "");
    const parts = u.pathname.split("/").filter(Boolean);
    if (host === "youtube.com" || host === "youtube-nocookie.com") {
      if (u.pathname === "/watch") {
        const v = u.searchParams.get("v");
        return v && YT_ID.test(v) ? `https://www.youtube.com/watch?v=${v}` : null;
      }
      const [kind, id] = parts;
      if (!id || !YT_ID.test(id)) return null;
      if (kind === "shorts") return `https://www.youtube.com/shorts/${id}`;
      if (kind === "embed" || kind === "live") return `https://www.youtube.com/watch?v=${id}`;
      return null;
    }
    if (host === "vimeo.com") {
      const id = parts.find((p) => /^\d{3,}$/.test(p));
      return id ? `https://vimeo.com/${id}` : null;
    }
    if (host === "player.vimeo.com") {
      return parts[0] === "video" && /^\d{3,}$/.test(parts[1] ?? "") ? `https://vimeo.com/${parts[1]}` : null;
    }
    if (host === "twitch.tv") {
      if (parts[0] === "videos" && /^\d+$/.test(parts[1] ?? "")) return `https://www.twitch.tv/videos/${parts[1]}`;
      if (parts[1] === "clip" && /^[\w-]+$/.test(parts[0]) && /^[\w-]+$/.test(parts[2] ?? "")) {
        return `https://www.twitch.tv/${parts[0]}/clip/${parts[2]}`;
      }
      return null;
    }
    if (host === "clips.twitch.tv") {
      return parts.length === 1 && /^[\w-]+$/.test(parts[0]) && parts[0] !== "embed" ? `https://clips.twitch.tv/${parts[0]}` : null;
    }
    return null;
  }

  /** The biggest video on screen under the mouse. */
  function videoAt(x, y) {
    let best = null;
    let area = 0;
    for (const v of document.getElementsByTagName("video")) {
      const r = v.getBoundingClientRect();
      if (r.width < MIN_W || r.height < MIN_H) continue;
      if (x < r.left || x > r.right || y < r.top || y > r.bottom) continue;
      if (r.width * r.height > area) {
        best = v;
        area = r.width * r.height;
      }
    }
    return best;
  }

  // ---------- the button (built on first use) ----------

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
    .item .fmt { font-size: 11.5px; color: #8e95a1; }
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
  let host, wrap, main, label, menu, note;

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text) e.textContent = text;
    return e;
  }

  /** Pako's box with its arrow, drawn in the button's ink colour. */
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
    menu.append(el("div", "head", "Download with Pako"));
    for (const [q, text, fmt] of QUALITIES) {
      const item = el("button", "item");
      item.type = "button";
      item.setAttribute("role", "menuitem");
      item.append(el("span", "", text), el("span", "fmt", fmt));
      item.addEventListener("click", () => pick(q));
      menu.append(item);
    }
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
    // Keys and clicks on the button stay with it (the player's shortcuts don't fire).
    for (const type of ["keydown", "keyup", "keypress", "click", "dblclick", "mousedown", "mouseup", "pointerdown", "pointerup"]) {
      host.addEventListener(type, (e) => {
        if (type === "keydown" && e.key === "Escape" && menuOpen) setMenu(false);
        e.stopPropagation();
      });
    }
    attach();
  }

  /** In full screen only the full-screen element shows: the button goes inside it. */
  function attach() {
    if (!host) return;
    const fs = document.fullscreenElement;
    const parent = fs && fs.tagName !== "VIDEO" ? fs : document.documentElement;
    if (host.parentNode !== parent) parent.append(host);
  }

  // ---------- showing and hiding ----------

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
    menuOpen = open;
    menu.classList.toggle("open", open);
    main.setAttribute("aria-expanded", String(open));
    if (open) clearTimeout(idleTimer);
    else {
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
    const found = videoAt(lastX, lastY);
    const v = found && videoUrl(found) ? found : null;
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
  // YouTube changes pages without reloading: a new page starts without the button.
  document.addEventListener("yt-navigate-start", () => {
    if (!busy) hide();
  });

  // ---------- sending ----------

  function setLabel(text) {
    label.textContent = text;
  }

  async function pick(quality) {
    const url = videoUrl(current);
    if (!url || busy) return;
    setMenu(false);
    busy = true;
    clearTimeout(resetTimer);
    main.setAttribute("aria-busy", "true");
    setLabel("Sending…");
    let reply;
    try {
      reply = await api.runtime.sendMessage({ type: "video", url, quality });
    } catch {
      reply = { ok: false, error: "Pako's extension was just updated. Reload this page and try again." };
    }
    busy = false;
    main.removeAttribute("aria-busy");
    if (reply?.ok) {
      setLabel("Sent to Pako ✓");
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
})();
