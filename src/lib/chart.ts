// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Small helpers for the Dashboard's hand-made SVG charts.

/** A round top for an axis, in the same 1024-based units the app shows (MB, GB). */
export function niceMax(v: number) {
  let unit = 1;
  while (v / unit >= 1024 && unit < 1024 ** 4) unit *= 1024;
  const n = v / unit;
  const p = Math.pow(10, Math.floor(Math.log10(Math.max(n, 1e-9))));
  for (const m of [1, 2, 2.5, 4, 5, 10]) if (m * p >= n) return m * p * unit;
  return 10 * p * unit;
}

/** A bar from `base` up to `top` (pixels) with rounded top corners. */
export function barPath(x: number, w: number, top: number, base: number, round = true) {
  const h = base - top;
  if (h <= 0.4 || w <= 0) return "";
  const r = round ? Math.min(3, w / 2, h) : 0;
  return `M${x} ${base}V${top + r}Q${x} ${top} ${x + r} ${top}H${x + w - r}Q${x + w} ${top} ${x + w} ${top + r}V${base}Z`;
}

const dateOf = (key: string) => new Date(key.length === 7 ? key + "-01T12:00:00" : key + "T12:00:00");

/** "16 Nov", "Nov 2026" or "14:00", for a bar's key. */
export function bucketLabel(key: string, step: string, withYear = false) {
  if (step === "hour") return `${key}:00`;
  const d = dateOf(key);
  if (step === "month") return d.toLocaleDateString([], { month: "short", year: withYear ? "numeric" : undefined });
  return d.toLocaleDateString([], { day: "numeric", month: "short", year: withYear ? "numeric" : undefined });
}

/** The long form for tooltips: "Monday 16 November", "November 2026", "14:00 to 15:00". */
export function bucketTitle(key: string, step: string) {
  if (step === "hour") return `${key}:00 to ${String((Number(key) + 1) % 24).padStart(2, "0")}:00`;
  const d = dateOf(key);
  if (step === "month") return d.toLocaleDateString([], { month: "long", year: "numeric" });
  return d.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long", year: d.getFullYear() === new Date().getFullYear() ? undefined : "numeric" });
}

/** "16 Nov" (with the year when it isn't this year), for a "YYYY-MM-DD" day. */
export function dayLabel(key: string) {
  const d = dateOf(key);
  return d.toLocaleDateString([], { day: "numeric", month: "short", year: d.getFullYear() === new Date().getFullYear() ? undefined : "numeric" });
}

/** Which bars get a label under the chart (about `want` of them, always the last). */
export function labelEvery(n: number, width: number, want = 72) {
  const fit = Math.max(1, Math.floor(width / want));
  return Math.max(1, Math.ceil(n / fit));
}
