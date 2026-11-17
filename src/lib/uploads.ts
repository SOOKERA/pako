// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Upload › "Delete after": the choices and how an uploaded item's date reads.
import type { SentItem } from "./api";

export const DAY_MS = 86_400_000;

/** The basket's "Delete after" choices (days; 0 = keep). */
export const DELETE_CHOICES: { value: number; label: string }[] = [
  { value: 0, label: "Never" },
  { value: 3, label: "3 days" },
  { value: 7, label: "7 days" },
  { value: 30, label: "30 days" },
];

/** Who deletes it on its day, in words. */
export function whoDeletes(s: Pick<SentItem, "host" | "expiryOnHost">): string {
  if (s.host === "drive") return "Pako deletes it from your Google Drive the first time it runs after that day.";
  return s.expiryOnHost ? "Gofile deletes it itself on that day, even with your PC off." : "Pako deletes it from Gofile the first time it runs after that day.";
}

/** "Wednesday 19 November, 14:32" */
export function longDate(ms: number): string {
  return new Date(ms).toLocaleString([], { weekday: "long", day: "numeric", month: "long", hour: "2-digit", minute: "2-digit" });
}

/** How an uploaded item's "Delete after" reads in the list: "Deletes in 2 days", "Kept"… */
export function expiryLook(s: SentItem, now: number): { label: string; tone: "dim" | "soon" | "fail"; title: string } | null {
  if (s.gone) return null;
  if (s.deleteError) return { label: "Couldn't delete yet", tone: "fail", title: `${s.deleteError} Pako tries again every hour.` };
  if (s.deleteAt == null) return { label: "Kept", tone: "dim", title: "Kept until you delete it." };
  const left = s.deleteAt - now;
  const title = `Deleted on ${longDate(s.deleteAt)}. ${whoDeletes(s)}`;
  if (left <= 0) return { label: "Deleting soon", tone: "soon", title };
  if (left < 3_600_000) return { label: "Deletes soon", tone: "soon", title };
  if (left < DAY_MS) {
    const h = Math.round(left / 3_600_000);
    return { label: `Deletes in ${h} ${h === 1 ? "hour" : "hours"}`, tone: "soon", title };
  }
  const d = Math.round(left / DAY_MS);
  return { label: d === 1 ? "Deletes tomorrow" : `Deletes in ${d} days`, tone: d <= 1 ? "soon" : "dim", title };
}

/** "Keep 7 more days": 7 days after its current date (or from now, when that has passed). */
export function sevenMore(s: SentItem, now: number): number {
  return Math.max(now, s.deleteAt ?? now) + 7 * DAY_MS;
}

/** "42 seconds", "4 min 31 s", "1 h 02 min" */
export function took(sec: number): string {
  const s = Math.max(1, Math.round(sec));
  if (s < 60) return `${s} ${s === 1 ? "second" : "seconds"}`;
  if (s < 3600) return `${Math.floor(s / 60)} min ${String(s % 60).padStart(2, "0")} s`;
  return `${Math.floor(s / 3600)} h ${String(Math.floor((s % 3600) / 60)).padStart(2, "0")} min`;
}

/** "ZIP in parts": the part sizes offered (MB), and what a typed-in size may be. */
export const PART_CHOICES: { value: number; label: string }[] = [
  { value: 500, label: "500 MB" },
  { value: 1024, label: "1 GB" },
  { value: 2048, label: "2 GB" },
];
export const PART_DEFAULT_MB = 500;
export const PART_MIN_MB = 50;
export const PART_MAX_MB = 10 * 1024;
/** Said under a typed-in part size that Pako won't take. */
export const PART_RANGE_NOTE = "Between 50 MB and 10 GB";

/** A part size Pako takes (whole MB, 50 MB to 10 GB). */
export function partOk(mb: number | null | undefined): mb is number {
  return typeof mb === "number" && Number.isInteger(mb) && mb >= PART_MIN_MB && mb <= PART_MAX_MB;
}

/** "500 MB", "1 GB", "1.5 GB" */
export function partLabel(mb: number): string {
  if (mb >= 1024) {
    const gb = mb / 1024;
    return `${Number.isInteger(gb) ? gb : gb.toFixed(1).replace(/\.0$/, "")} GB`;
  }
  return `${mb} MB`;
}

/** What people downloading it need, said next to "ZIP in parts". */
export const PARTS_NOTE = "Each part uploads on its own: if the connection drops, only the part in progress starts over. To open it, people need all the parts and 7-Zip (or Pako).";
