// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// A button that asks once (the compact window's ✕ → "Remove"): the first click arms it, a
// second click on the same one does it.

/** How long an armed button waits for its second click (ms). */
export const CONFIRM_FOR = 4000;
/** A second click this soon after the first is the same double-click, not an answer (ms). */
export const CONFIRM_SETTLE = 400;

export type Armed = { id: string; at: number } | null;

/** What a click on `id`'s button at `now` (ms) does: arm it, do it, or nothing (a double-click). */
export function confirmClick(armed: Armed, id: string, now: number): "arm" | "confirm" | "ignore" {
  if (!armed || armed.id !== id || now - armed.at >= CONFIRM_FOR) return "arm";
  if (now - armed.at < CONFIRM_SETTLE) return "ignore";
  return "confirm";
}
