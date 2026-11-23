// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Leaving Private mode's screens locks it. Which moves leave, and what the question before
// leaving says when something is still running there. Plain functions: tested in
// test/private-leave.test.mjs.
import type { Key } from "./i18n.svelte.ts";
import { words, type Params } from "./i18n/core.ts";
import { en } from "./i18n/en.ts";

export type WorkKind = "send" | "upload" | "decrypt";

/** Where the window is: its page, the Downloads filters, and Settings / Help being open. */
export interface Where {
  view: string;
  library: string;
  source: string | null;
  settings: boolean;
  help: boolean;
}

/** From `from` to `to` with Private mode open: does that leave its screens (and lock it)?
 *  Another page, a sidebar filter (Downloading, Finished, a source…), Settings or Help:
 *  yes. Showing the private list or the normal one with the header's buttons changes none of
 *  these: no. */
export function leaves(from: Where, to: Where): boolean {
  if (to.view !== "downloads") return true;
  if (to.settings && !from.settings) return true;
  if (to.help && !from.help) return true;
  return to.library !== from.library || to.source !== from.source;
}

// ---- Words: English here (and in the tests); the window gives its own t() (private.svelte.ts),
// so they follow the language shown. ----

type Say = (key: Key, params?: Params) => string;
let say: Say = (key, params) => words(en[key] ?? key, "en", params);
let locale = () => "en-GB";
/** The window's words and its locale (for "a, b and c"). */
export function useWords(fn: Say, loc: () => string) {
  say = fn;
  locale = loc;
}

const DOING: Record<WorkKind, Key> = { send: "sending {what}", upload: "uploading {what}", decrypt: "decrypting {what}" };

/** What leaving stops, in words: "sending Holiday.mp4", "the video", "2 things in progress";
 *  "" when nothing runs (then it locks without asking). */
export function leaveWords(work: { kind: WorkKind; what: string }[], playing: string | null): string {
  const n = work.length + (playing ? 1 : 0);
  if (n === 0) return "";
  if (n > 1) return say("{n} things in progress", { n });
  if (playing) return say("the video");
  return say(DOING[work[0].kind], { what: work[0].what });
}

/** After Lock stopped something: "Private mode locked. 1 send stopped." (naming nothing
 *  private; "" when nothing was running). */
export function lockedNote(kinds: WorkKind[], playing: boolean): string {
  const count = (k: WorkKind) => kinds.filter((x) => x === k).length;
  const parts: string[] = [];
  if (count("send")) parts.push(say("{n} sends", { n: count("send") }));
  if (count("upload")) parts.push(say("{n} uploads", { n: count("upload") }));
  if (count("decrypt")) parts.push(say("{n} decrypts", { n: count("decrypt") }));
  if (!parts.length) return playing ? say("Private mode locked. The video stopped.") : "";
  if (playing) parts.push(say("the video"));
  const list = new Intl.ListFormat(locale(), { type: "conjunction" }).format(parts);
  return say("Private mode locked. {list} stopped.", { list });
}

/** The whole question: "Leaving locks Private mode and stops sending Holiday.mp4. Leave anyway?" */
export function leaveQuestion(stops: string): string {
  return say("Leaving locks Private mode and stops {words}. Leave anyway?", { words: stops });
}
