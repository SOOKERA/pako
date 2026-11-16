// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Apple-style motion: quick, soft ease-out with a tiny scale, never bouncy or slow.
import { cubicOut, quintOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";
import { flip, type AnimationConfig } from "svelte/animate";

/** Windows' "Animation effects" are off. */
export const reduced = () => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Dialogs and menus: fade in while growing from 96% (and a few px up). */
export function pop(
  _node: Element,
  {
    duration = 220,
    from = 0.96,
    y = 6,
    origin = "center",
    delay = 0,
  }: { duration?: number; from?: number; y?: number; origin?: string; delay?: number } = {},
): TransitionConfig {
  if (reduced()) return { duration: 0 };
  return {
    duration,
    delay,
    easing: quintOut,
    css: (t) =>
      `opacity:${t};transform-origin:${origin};transform:translateY(${(1 - t) * y}px) scale(${from + (1 - from) * t})`,
  };
}

/** Backdrop. */
export function fadeIn(_node: Element, { duration = 180 }: { duration?: number } = {}): TransitionConfig {
  if (reduced()) return { duration: 0 };
  return { duration, easing: cubicOut, css: (t) => `opacity:${t}` };
}

/** Rows: open/close in height (folders expanding, rows added or removed). */
export function rowSlide(node: Element, { duration = 220 }: { duration?: number } = {}): TransitionConfig {
  // No motion (long lists): don't even measure the row, reading its height forces a layout.
  if (reduced() || duration === 0) return { duration: 0 };
  const h = (node as HTMLElement).offsetHeight;
  return {
    duration,
    easing: quintOut,
    css: (t) => `height:${t * h}px;opacity:${Math.min(1, t * 1.6)};overflow:hidden`,
  };
}

/** Toasts: slide up from the bottom. */
export function toastIn(_node: Element, { duration = 260 }: { duration?: number } = {}): TransitionConfig {
  if (reduced()) return { duration: 0 };
  return {
    duration,
    easing: quintOut,
    css: (t) => `opacity:${t};transform:translateY(${(1 - t) * 14}px) scale(${0.98 + 0.02 * t})`,
  };
}

export const flipOpts = { duration: 260, easing: quintOut };

/** Rows moving to their new place, skipped entirely (no style reads) when duration is 0. */
export function flipIf(
  node: Element,
  rects: { from: DOMRect; to: DOMRect },
  params: { duration: number; easing?: (t: number) => number },
): AnimationConfig {
  if (params.duration === 0) return { duration: 0 };
  return flip(node, rects, params);
}
