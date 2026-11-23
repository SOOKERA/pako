// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Upload › "Delete after": the choices and how an uploaded item's date reads.
import type { SentItem } from "./api";
import { i18n, num, t } from "./i18n.svelte";

export const DAY_MS = 86_400_000;

/** The basket's "Delete after" choices (days; 0 = keep), in the language shown. */
export function deleteChoices(): { value: number; label: string }[] {
  return [
    { value: 0, label: t("Never") },
    { value: 3, label: t("{n} days", { n: 3 }) },
    { value: 7, label: t("{n} days", { n: 7 }) },
    { value: 30, label: t("{n} days", { n: 30 }) },
  ];
}

/** Who deletes it on its day, in words. */
export function whoDeletes(s: Pick<SentItem, "host" | "expiryOnHost">): string {
  if (s.host === "drive") return t("Pako deletes it from your Google Drive the first time it runs after that day.");
  return s.expiryOnHost ? t("Gofile deletes it itself on that day, even with your PC off.") : t("Pako deletes it from Gofile the first time it runs after that day.");
}

/** "Wednesday 19 November, 14:32" */
export function longDate(ms: number): string {
  return new Date(ms).toLocaleString(i18n.dates, { weekday: "long", day: "numeric", month: "long", hour: "2-digit", minute: "2-digit" });
}

/** How an uploaded item's "Delete after" reads in the list: "Deletes in 2 days", "Kept"… */
export function expiryLook(s: SentItem, now: number): { label: string; tone: "dim" | "soon" | "fail"; title: string } | null {
  if (s.gone) return null;
  if (s.deleteError) return { label: t("Couldn't delete yet"), tone: "fail", title: t("{error} Pako tries again every hour.", { error: s.deleteError }) };
  if (s.deleteAt == null) return { label: t("Kept"), tone: "dim", title: t("Kept until you delete it.") };
  const left = s.deleteAt - now;
  const title = t("Deleted on {date}. {who}", { date: longDate(s.deleteAt), who: whoDeletes(s) });
  if (left <= 0) return { label: t("Deleting soon"), tone: "soon", title };
  if (left < 3_600_000) return { label: t("Deletes soon"), tone: "soon", title };
  if (left < DAY_MS) {
    const h = Math.round(left / 3_600_000);
    return { label: t("Deletes in {n} hours", { n: h }), tone: "soon", title };
  }
  const d = Math.round(left / DAY_MS);
  return { label: d === 1 ? t("Deletes tomorrow") : t("Deletes in {n} days", { n: d }), tone: d <= 1 ? "soon" : "dim", title };
}

/** "Keep 7 more days": 7 days after its current date (or from now, when that has passed). */
export function sevenMore(s: SentItem, now: number): number {
  return Math.max(now, s.deleteAt ?? now) + 7 * DAY_MS;
}

/** "42 seconds", "4 min 31 s", "1 h 02 min" (min, s and h are the same in French) */
export function took(sec: number): string {
  const s = Math.max(1, Math.round(sec));
  if (s < 60) return t("{n} seconds", { n: s });
  if (s < 3600) return `${Math.floor(s / 60)} min ${String(s % 60).padStart(2, "0")} s`;
  return `${Math.floor(s / 3600)} h ${String(Math.floor((s % 3600) / 60)).padStart(2, "0")} min`;
}

/** "ZIP in parts": the part sizes offered (MB), and what a typed-in size may be. Shown with
 *  partLabel (the label here is the English one). */
export const PART_CHOICES: { value: number; label: string }[] = [
  { value: 500, label: "500 MB" },
  { value: 1024, label: "1 GB" },
  { value: 2048, label: "2 GB" },
];
export const PART_DEFAULT_MB = 500;
export const PART_MIN_MB = 50;
export const PART_MAX_MB = 10 * 1024;
/** Said under a typed-in part size that Pako won't take (an English key: show it with t()). */
export const PART_RANGE_NOTE = "Between 50 MB and 10 GB";

/** A part size Pako takes (whole MB, 50 MB to 10 GB). */
export function partOk(mb: number | null | undefined): mb is number {
  return typeof mb === "number" && Number.isInteger(mb) && mb >= PART_MIN_MB && mb <= PART_MAX_MB;
}

/** "500 MB", "1 GB", "1.5 GB" (French: "500 Mo", "1 Go", "1,5 Go") */
export function partLabel(mb: number): string {
  if (mb >= 1024) {
    const gb = Math.round((mb / 1024) * 10) / 10;
    return t("{n} GB", { n: num(gb, Number.isInteger(gb) ? 0 : 1) });
  }
  return t("{n} MB", { n: mb });
}

/** What people downloading it need, said next to "ZIP in parts" (an English key: show it with t()). */
export const PARTS_NOTE = "Each part uploads on its own: if the connection drops, only the part in progress starts over. To open it, people need all the parts and 7-Zip (or Pako).";
