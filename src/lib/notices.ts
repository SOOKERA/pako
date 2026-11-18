// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's notifications: the kinds (the same table as src-tauri/src/notices.rs, checked by a
// test there), their icons and colours, and small helpers shared by the bell, the pop-ups and
// Settings › Notifications.
import type { Settings } from "./api";

export type NoticeKind =
  | "download-done"
  | "download-failed"
  | "browser"
  | "video-failed"
  | "upload-done"
  | "upload-failed"
  | "upload-deleted"
  | "direct-request"
  | "direct-offer"
  | "direct-incoming"
  | "direct-received"
  | "needs-you"
  | "power"
  | "update-ready"
  | "other";

/** Colour of a kind's icon: done (mint), problem (red), info (azure), wait (amber), people (violet). */
export type Tone = "done" | "fail" | "info" | "wait" | "people";

export interface KindInfo {
  id: NoticeKind;
  /** Its name in Settings › Notifications. */
  label: string;
  /** The heading it sits under in Settings. */
  group: "Downloads" | "Upload" | "Direct" | "Pako";
  icon: string;
  tone: Tone;
  /** The bell keeps it by default (every kind pops up by default). */
  bell: boolean;
}

export const KINDS: KindInfo[] = [
  { id: "download-done", label: "Download finished", group: "Downloads", icon: "check", tone: "done", bell: false },
  { id: "download-failed", label: "Download stopped", group: "Downloads", icon: "alert", tone: "fail", bell: false },
  { id: "browser", label: "Sent from your browser", group: "Downloads", icon: "globe", tone: "info", bell: false },
  { id: "video-failed", label: "Couldn't get a video", group: "Downloads", icon: "alert", tone: "fail", bell: true },
  { id: "upload-done", label: "Upload finished", group: "Upload", icon: "link", tone: "done", bell: false },
  { id: "upload-failed", label: "Couldn't upload", group: "Upload", icon: "alert", tone: "fail", bell: false },
  { id: "upload-deleted", label: "Upload deleted on its date", group: "Upload", icon: "trash", tone: "info", bell: false },
  { id: "direct-request", label: "Someone wants to connect", group: "Direct", icon: "user", tone: "people", bell: true },
  { id: "direct-offer", label: "Files waiting for your answer", group: "Direct", icon: "download", tone: "people", bell: true },
  { id: "direct-incoming", label: "Files coming in", group: "Direct", icon: "download", tone: "info", bell: false },
  { id: "direct-received", label: "Files received", group: "Direct", icon: "check", tone: "done", bell: false },
  { id: "needs-you", label: "Pako needs you", group: "Pako", icon: "bell", tone: "wait", bell: true },
  { id: "power", label: "Sleep and shut down", group: "Pako", icon: "power", tone: "wait", bell: false },
  { id: "update-ready", label: "New Pako ready", group: "Pako", icon: "download", tone: "done", bell: true },
  { id: "other", label: "Other messages", group: "Pako", icon: "help", tone: "info", bell: false },
];

export const kindInfo = (kind: string): KindInfo => KINDS.find((k) => k.id === kind) ?? KINDS[KINDS.length - 1];

/** Where a click leads: a page of the window, and the item there when there is one. Direct's
 *  questions also carry the offer (Accept / Decline) or the person asking to connect. */
export interface Go {
  /** "update": restart into the new Pako (it asks first while things are moving). */
  page: "" | "downloads" | "found" | "send" | "direct" | "devices" | "dashboard" | "update";
  id?: string;
  offer?: string;
  peer?: string;
}

/** One line in the bell. */
export interface Notice {
  id: string;
  kind: NoticeKind;
  title: string;
  body: string;
  /** When it came (ms). */
  at: number;
  read: boolean;
  go: Go;
  /** A question once settled ("Accepted", "Declined", "Done", "No longer waiting"...). */
  answered?: string | null;
}

/** One pop-up card. */
export interface PopupCard {
  id: string;
  kind: NoticeKind;
  title: string;
  body: string;
  go: Go;
  /** Waits for an answer: doesn't leave by itself. */
  stays: boolean;
}

/** A kind changed from its defaults in Settings (a missing switch keeps its default). */
export interface NoticePref {
  popup?: boolean;
  bell?: boolean;
}

/** A kind's two switches as they are now (the same rule as notices.rs `pref`). "Download
 *  finished › Pop-up" is the older notifyDone setting. */
export function prefOf(s: Settings, kind: NoticeKind): { popup: boolean; bell: boolean } {
  const k = kindInfo(kind);
  const changed = s.notices?.[k.id] ?? {};
  return { popup: k.id === "download-done" ? s.notifyDone : (changed.popup ?? true), bell: changed.bell ?? k.bell };
}

/** "now", "5 min ago", "2 h ago", "Yesterday 14:05", "12 Nov 09:30". */
export function noticeTime(at: number, now = Date.now()): string {
  const s = Math.max(0, Math.round((now - at) / 1000));
  if (s < 60) return "now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  const d = new Date(at);
  const hm = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const today = new Date(now);
  if (d.toDateString() === today.toDateString()) return s < 6 * 3600 ? `${Math.floor(s / 3600)} h ago` : hm;
  const y = new Date(now - 86_400_000);
  if (d.toDateString() === y.toDateString()) return `Yesterday ${hm}`;
  return `${d.toLocaleDateString([], { day: "numeric", month: "short" })} ${hm}`;
}

export const isToday = (at: number, now = Date.now()) => new Date(at).toDateString() === new Date(now).toDateString();
