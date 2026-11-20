// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// How wide a details panel may be: the width you dragged it to (remembered per page), kept
// between a smallest and a largest width, and never so wide that the list next to it gets
// cramped (the panel gives way first, down to its smallest width). Plain functions: tested in
// test/panel-size.test.mjs.

/** Smallest width: every panel's buttons and values still fit (checked at this width). */
export const PANEL_MIN = 280;
/** Largest width: past this the panel only adds empty space. */
export const PANEL_MAX = 560;
/** The width a panel starts at (and goes back to on a double-click on its edge). */
export const PANEL_DEFAULT = 320;
/** What the list next to the panel keeps at least before the panel gives way. */
export const LIST_MIN = 420;
/** One arrow key press on the panel's edge (Shift: four times as much). */
export const PANEL_STEP = 16;

/** A remembered width, made safe: a number in range, else the default. */
export function savedWidth(w: unknown, def = PANEL_DEFAULT): number {
  if (typeof w !== "number" || !Number.isFinite(w)) return def;
  return Math.round(Math.min(PANEL_MAX, Math.max(PANEL_MIN, w)));
}

/** The width the panel really gets: the remembered one, unless the list next to it would be
 *  narrower than `listMin` in `avail` (the room for list and panel together); then smaller,
 *  but never under the smallest width. `avail` 0 (not measured yet): the remembered one. */
export function shownWidth(saved: number, avail: number, listMin = LIST_MIN): number {
  const w = savedWidth(saved);
  if (!(avail > 0)) return w;
  return Math.round(Math.max(PANEL_MIN, Math.min(w, avail - listMin)));
}

/** While dragging: the width for the pointer `dx` px left of where the drag started (the
 *  edge is on the panel's left, so left = wider), within range and leaving the list its room. */
export function draggedWidth(start: number, dx: number, avail: number, listMin = LIST_MIN): number {
  const most = avail > 0 ? Math.max(PANEL_MIN, Math.min(PANEL_MAX, avail - listMin)) : PANEL_MAX;
  return Math.round(Math.max(PANEL_MIN, Math.min(most, start + dx)));
}

/** A key on the panel's edge: the new width, or null for a key it doesn't use. ← widens
 *  (the edge moves left), → narrows, Home: the largest that fits, End: the smallest. */
export function keyedWidth(w: number, key: string, shift: boolean, avail: number, listMin = LIST_MIN): number | null {
  const step = PANEL_STEP * (shift ? 4 : 1);
  const dx = key === "ArrowLeft" ? step : key === "ArrowRight" ? -step : key === "Home" ? PANEL_MAX : key === "End" ? -PANEL_MAX : null;
  if (dx === null) return null;
  return draggedWidth(w, dx, avail, listMin);
}
