// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The "sent" page (pako-sent.html): the browser turned a download's answer away and the extension
// handed it to Pako. It asks the extension first (which also lets it note the tab before the tab
// can close), then goes back to the page you clicked it on, or, in a tab opened only for the file,
// closes that tab (as a browser closes it after a download). It stays and says so when the
// extension says to: going back would reload a form's answer, the page asked for the same file
// again by itself (no loop), or a private window's file goes on in the browser in this tab. Inside
// a frame of a page, or when you come back to it with Back, Forward or Reload, it only says what
// happened.
(() => {
  const api = globalThis.chrome;
  const how = globalThis.performance?.getEntriesByType?.("navigation")?.[0]?.type;
  const say = (text) => {
    const show = () => {
      const p = document.getElementById("said");
      if (!p) return;
      if (typeof text === "string" && text) p.textContent = text;
      p.hidden = false;
    };
    if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", show);
    else show();
  };
  // Back here from the browser's back-forward cache: the page as it was left (blank) says it.
  globalThis.addEventListener?.("pageshow", (e) => {
    if (e.persisted) say();
  });
  if (window.top !== window || how === "back_forward" || how === "reload") {
    say();
    return;
  }
  const leave = () => {
    if (history.length > 1) {
      history.back();
      return;
    }
    try {
      api.tabs.getCurrent((tab) => {
        void api.runtime.lastError;
        if (tab?.id != null) api.tabs.remove(tab.id, () => void api.runtime.lastError);
        else say();
      });
    } catch {
      say();
    }
  };
  let done = false;
  const answer = (a) => {
    if (done) return;
    done = true;
    if (a && a.stay === true) say(typeof a.said === "string" ? a.said : "");
    else leave();
  };
  try {
    api.runtime.sendMessage({ type: "sent-page" }, (a) => {
      void api.runtime.lastError;
      answer(a);
    });
  } catch {
    answer(null);
  }
  // No answer (the extension restarting): back, as without it.
  setTimeout(() => answer(null), 5000);
})();
