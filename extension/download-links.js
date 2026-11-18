// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Download links on pages, in browsers that ask "Download …?" before an extension sees a download
// (Vivaldi). A link made to download (`<a download href="…">`) starts its download without
// loading anything, so the extension's rule at the server's answer never sees it. The background
// puts this script in web pages (top frame only) only while that catch is on and "Catch download
// links on pages" is on. It reads nothing of the page: only when you click such a link (a real
// click, Enter on it, or the middle button) and the page didn't handle that click itself, it stops
// the browser's download and tells the extension the link, its file name and whether the page
// may be named to its site. If the extension doesn't take it, the browser downloads it as usual.
// Never in a private window, never a link to another site (the browser loads that as a page),
// nor one built inside the page (blob:, data:).
(() => {
  const api = globalThis.chrome;
  // Once per page; never in a private window (the browser's own way there, like the rule).
  if (globalThis.__pakoLinks || !api?.runtime?.id || api.extension?.inIncognitoContext) return;
  globalThis.__pakoLinks = true;
  if (window.top !== window) return;

  /** The address without its "#…". */
  const bare = (href) => {
    const u = new URL(href);
    u.hash = "";
    return u.href;
  };

  /** The link (<a> or <area>) this event is a real click on, or null. */
  function anchorOf(e) {
    if (!e.isTrusted) return null;
    // The left button (Enter on a link is a click too) or the middle one; no Ctrl, Shift, Alt.
    if (e.type === "click" ? e.button !== 0 : e.button !== 1) return null;
    if (e.ctrlKey || e.shiftKey || e.altKey || e.metaKey) return null;
    const path = typeof e.composedPath === "function" ? e.composedPath() : [e.target];
    return path.find((n) => n instanceof HTMLAnchorElement || n instanceof HTMLAreaElement) || null;
  }

  /** The download link `a` is, as it is now, or null. Read at the end of the click: the page's
   *  own handler may have changed it meanwhile (its address, a blob: it just made, its
   *  `download`), and the browser downloads what the link says then. */
  function linkOf(a) {
    if (!a || !a.hasAttribute("download")) return null;
    let url;
    try {
      url = new URL(a.href);
    } catch {
      return null;
    }
    if (url.protocol !== "http:" && url.protocol !== "https:") return null;
    // The browser downloads only a link to the page's own site (it loads others as pages).
    if (url.origin !== location.origin) return null;
    // A link to this very page.
    if (bare(url.href) === bare(location.href)) return null;
    const rel = String(a.getAttribute("rel") || "");
    return {
      a,
      url: bare(url.href),
      filename: String(a.getAttribute("download") || ""),
      noReferrer: /(^|\s)noreferrer(\s|$)/i.test(rel) || a.referrerPolicy === "no-referrer",
    };
  }

  /** The browser downloads it as it would have: a copy of the link, not in the page (the page
   *  sees nothing again), clicked by the script (so it isn't caught again). */
  function letBrowser(link) {
    const b = document.createElement("a");
    b.href = link.url;
    b.setAttribute("download", link.filename);
    if (link.a.getAttribute("rel")) b.setAttribute("rel", link.a.getAttribute("rel"));
    if (link.a.referrerPolicy) b.referrerPolicy = link.a.referrerPolicy;
    b.click();
  }

  async function hand(link) {
    let reply = null;
    try {
      reply = await api.runtime.sendMessage({ type: "link-download", url: link.url, filename: link.filename, noReferrer: link.noReferrer });
    } catch {
      reply = null;
    }
    if (reply?.taken !== true) letBrowser(link);
  }

  /** The decision waits for the end of the click (the last listener on the window), so a page
   *  that handles the click itself (preventDefault) keeps it, and a link the page changed in
   *  its own handler is taken as the browser would take it. */
  let decide = null;
  function onCapture(e) {
    if (decide) window.removeEventListener(decide.type, decide.f);
    decide = null;
    const a = anchorOf(e);
    // (The extension was reloaded or removed: this copy of the script has no one to tell.)
    if (!a || !api.runtime?.id) return;
    const f = (ev) => {
      if (ev !== e) return;
      window.removeEventListener(e.type, f);
      decide = null;
      if (e.defaultPrevented || !api.runtime?.id) return;
      const link = linkOf(a);
      if (!link) return;
      e.preventDefault();
      hand(link);
    };
    decide = { type: e.type, f };
    window.addEventListener(e.type, f);
  }
  window.addEventListener("click", onCapture, true);
  window.addEventListener("auxclick", onCapture, true);
})();
