// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// How a link in Links is doing, in words and as one small mark (the Link column, and the row's
// name for screen readers). One place, so the mark, its tooltip and the row always agree.
// No window here (tested in Node): the words come through `say` (the window passes t(); the
// tests see the English).
import { fill, type Params } from "./i18n/core.ts";

/** Words in the language shown: the English is the key (t() in the window). */
export type Say = (key: string, params?: Params) => string;
const english: Say = (key, params) => fill(key, params);

/** Rust's notes on a link page, in English or French (a note written before a switch keeps its
 *  words). Rust's gates.rs: WAITING_TURN, WAITING_FOR_YOU / WAITING_PAGE. */
const TURN = ["Waiting for the link", "En attente du lien"];
const YOU = ["Waiting for you", "En attente de vous", "À vous"];
const CHECK = ["check", "vérification"];
const startsAny = (s: string, list: string[]) => list.some((x) => s.startsWith(x));
/** A link page waits for you (Rust's note, either language). */
export const waitingForYou = (msg: string | null | undefined) => startsAny(msg ?? "", YOU);

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

export function linkLook(f: LinkFacts, say: Say = english): LinkLook {
  const msg = f.message ?? "";
  if (f.deviceNote) return { word: f.deviceNote, kind: "needs", mark: "alert" };
  switch (f.state) {
    case "ready":
      return msg ? { word: say("Ready · Google limit today"), kind: "needs", mark: "limit", more: msg } : { word: say("Ready"), kind: "ready", mark: "check" };
    case "checking":
      return { word: f.video ? say("Checking video…") : say("Checking link"), kind: "checking", mark: "ring" };
    case "needsTools":
      return f.toolsBusy
        ? { word: say("Installing the video tools…"), kind: "checking", mark: "ring" }
        : { word: say("Needs the video tools"), kind: "needs", mark: "tools", act: "tools", more: say("click to install them (yt-dlp, Deno and ffmpeg, about 600 MB on disk)") };
    case "opening": {
      if (startsAny(msg, TURN)) return { word: say("Waiting its turn"), kind: "checking", mark: "ring", more: msg };
      const waiting = startsAny(msg, YOU);
      const check = CHECK.some((x) => msg.includes(x));
      return waiting
        ? { word: say("Waiting for you"), kind: "needs", mark: "alert", act: "page", more: check ? say("click to show check") : say("click to show page") }
        : { word: say("Opening link…"), kind: "checking", mark: "ring", act: "page", quiet: true, more: say("click to see the link page and finish it yourself") };
    }
    case "broken":
      return { word: msg || say("Link broken"), kind: "broken", mark: "x" };
    default:
      // Google gives it only to certain accounts.
      return { word: say("Only in the browser"), kind: "needs", mark: "browser", act: "browser", more: msg ? say("click to open it there ({why})", { why: msg }) : say("click to open it there") };
  }
}

/** The tooltip: the words, then what a click does. */
export const linkTip = (l: LinkLook, say: Say = english): string => (l.more ? say("{word}: {more}", { word: l.word, more: l.more }) : l.word);
