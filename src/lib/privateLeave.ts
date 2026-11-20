// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Leaving Private mode's screens locks it. Which moves leave, and what the question before
// leaving says when something is still running there. Plain functions: tested in
// test/private-leave.test.mjs.

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

const VERB: Record<WorkKind, string> = { send: "sending", upload: "uploading", decrypt: "decrypting" };

/** What leaving stops, in words: "sending Holiday.mp4", "the video", "2 things in progress";
 *  "" when nothing runs (then it locks without asking). */
export function leaveWords(work: { kind: WorkKind; what: string }[], playing: string | null): string {
  const n = work.length + (playing ? 1 : 0);
  if (n === 0) return "";
  if (n > 1) return `${n} things in progress`;
  if (playing) return "the video";
  return `${VERB[work[0].kind]} ${work[0].what}`;
}

/** After Lock stopped something: "Private mode locked. 1 send stopped." (naming nothing
 *  private; "" when nothing was running). */
export function lockedNote(kinds: WorkKind[], playing: boolean): string {
  const count = (k: WorkKind) => kinds.filter((x) => x === k).length;
  const parts: string[] = [];
  const n = (k: number, one: string, many: string) => (k === 1 ? `1 ${one}` : `${k} ${many}`);
  if (count("send")) parts.push(n(count("send"), "send", "sends"));
  if (count("upload")) parts.push(n(count("upload"), "upload", "uploads"));
  if (count("decrypt")) parts.push(n(count("decrypt"), "decrypt", "decrypts"));
  if (playing) parts.push("the video");
  if (!parts.length) return "";
  const list = parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]}`;
  return `Private mode locked. ${list[0].toUpperCase()}${list.slice(1)} stopped.`;
}

/** The whole question: "Leaving locks Private mode and stops sending Holiday.mp4. Leave anyway?" */
export function leaveQuestion(words: string): string {
  return `Leaving locks Private mode and stops ${words}. Leave anyway?`;
}
