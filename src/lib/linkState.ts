// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// How a link in Links is doing, in words and as one small mark (the Link column, and the row's
// name for screen readers). One place, so the mark, its tooltip and the row always agree.

export type LinkMark = "check" | "limit" | "ring" | "alert" | "x" | "tools" | "browser";
/** The colour family (the Link column's classes). */
export type LinkKind = "ready" | "checking" | "needs" | "broken";
/** What a click on the mark does, when it can do something. */
export type LinkAct = "tools" | "page" | "browser";

export interface LinkLook {
  /** The words: the tooltip's start and what screen readers say. */
  word: string;
  kind: LinkKind;
  mark: LinkMark;
  act?: LinkAct;
  /** The mark is a button, but quiet (no fill): a page still opening, which can be shown. */
  quiet?: boolean;
  /** More for the tooltip (what a click does, the server's message). */
  more?: string;
}

export interface LinkFacts {
  state: string;
  message?: string | null;
  video: boolean;
  /** "Download on" can't send this one: its note (only when it could be downloaded). */
  deviceNote?: string | null;
  toolsBusy?: boolean;
}

export function linkLook(f: LinkFacts): LinkLook {
  const msg = f.message ?? "";
  if (f.deviceNote) return { word: f.deviceNote, kind: "needs", mark: "alert" };
  switch (f.state) {
    case "ready":
      return msg ? { word: "Ready · Google limit today", kind: "needs", mark: "limit", more: msg } : { word: "Ready", kind: "ready", mark: "check" };
    case "checking":
      return { word: f.video ? "Checking video…" : "Checking link", kind: "checking", mark: "ring" };
    case "needsTools":
      return f.toolsBusy
        ? { word: "Installing the video tools…", kind: "checking", mark: "ring" }
        : { word: "Needs the video tools", kind: "needs", mark: "tools", act: "tools", more: "click to install them (yt-dlp, Deno and ffmpeg, about 600 MB on disk)" };
    case "opening": {
      if (msg.startsWith("Waiting for the link")) return { word: "Waiting its turn", kind: "checking", mark: "ring", more: msg };
      const waiting = msg.startsWith("Waiting for you");
      const show = msg.includes("check") ? "Show check" : "Show page";
      return waiting
        ? { word: "Waiting for you", kind: "needs", mark: "alert", act: "page", more: `click to ${show.toLowerCase()}` }
        : { word: "Opening link…", kind: "checking", mark: "ring", act: "page", quiet: true, more: "click to see the link page and finish it yourself" };
    }
    case "broken":
      return { word: msg || "Link broken", kind: "broken", mark: "x" };
    default:
      // Google gives it only to certain accounts.
      return { word: "Only in the browser", kind: "needs", mark: "browser", act: "browser", more: msg ? `click to open it there (${msg})` : "click to open it there" };
  }
}

/** The tooltip: the words, then what a click does. */
export const linkTip = (l: LinkLook): string => (l.more ? `${l.word}: ${l.more}` : l.word);
