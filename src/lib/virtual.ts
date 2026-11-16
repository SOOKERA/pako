// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Long lists: only the rows on screen (plus some above and below) are drawn, so scrolling
// stays smooth with thousands of downloads or links. Rows have fixed heights, so where each
// one sits is worked out from those heights instead of measuring the page.

/** Fewer rows than this: draw them all (and keep the row animations). */
export const VIRTUAL_MIN = 300;
/** Extra rows drawn above and below the visible area, in pixels. */
const OVERSCAN = 800;

/** One row's box: its height, and the gap (CSS margin) it wants above and below. */
export interface RowBox {
  h: number;
  top: number;
  bottom: number;
}

export interface Layout {
  /** Top of each row's box (after its top gap). */
  ys: number[];
  boxes: RowBox[];
  /** Height of everything. */
  total: number;
}

/** Stacks the rows. Gaps between two rows collapse to the larger one, like CSS margins. */
export function layout(boxes: RowBox[]): Layout {
  const ys: number[] = new Array(boxes.length);
  let y = 0;
  for (let i = 0; i < boxes.length; i++) {
    const b = boxes[i];
    const gap = i === 0 ? b.top : Math.max(boxes[i - 1].bottom, b.top);
    y += gap;
    ys[i] = y;
    y += b.h;
  }
  const total = boxes.length ? y + boxes[boxes.length - 1].bottom : 0;
  return { ys, boxes, total };
}

/** First row whose box ends below `y`. */
function firstBelow(l: Layout, y: number): number {
  let lo = 0;
  let hi = l.ys.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (l.ys[mid] + l.boxes[mid].h < y) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

/** The rows to draw for this scroll position, and the empty space above and below them. */
export function visible(l: Layout, scrollTop: number, viewH: number): { start: number; end: number; padTop: number; padBottom: number } {
  const n = l.ys.length;
  if (!n) return { start: 0, end: 0, padTop: 0, padBottom: 0 };
  const start = Math.min(n - 1, firstBelow(l, scrollTop - OVERSCAN));
  const end = Math.max(start + 1, Math.min(n, firstBelow(l, scrollTop + viewH + OVERSCAN) + 1));
  const padTop = l.ys[start] - l.boxes[start].top;
  const last = end - 1;
  const padBottom = Math.max(0, l.total - (l.ys[last] + l.boxes[last].h + l.boxes[last].bottom));
  return { start, end, padTop, padBottom };
}

/** Scrolls `body` so row `i` is in view (if it isn't already). */
export function reveal(body: HTMLElement, l: Layout, i: number) {
  const y = l.ys[i];
  if (y === undefined) return;
  const h = l.boxes[i].h;
  if (y < body.scrollTop) body.scrollTop = y - 8;
  else if (y + h > body.scrollTop + body.clientHeight) body.scrollTop = y + h - body.clientHeight + 8;
}
