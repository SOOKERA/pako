// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Work a first open would otherwise do on the spot, done once in idle time after start, in
// small slices, so every page and tutorial is smooth the very first time too: the fonts' other
// letter sets (a name with "ă" or "đ" otherwise loads them mid-animation and the text jumps).
// Nothing here touches private data or the network (the fonts are Pako's own files).

const FAMILIES = ["Inter Variable", "JetBrains Mono Variable"];
/** One letter from each of the fonts' other sets: Latin Extended, Greek (and its accents), Cyrillic (and its extended letters), Vietnamese. */
const SAMPLES = ["ĂĐŁŐ", "ΑΩ", "ἀἐ", "ДЖ", "ҐҒ", "Ơư"];

type Idle = (cb: () => void) => void;
const idle: Idle =
  typeof requestIdleCallback === "function" ? (cb) => requestIdleCallback(() => cb(), { timeout: 4000 }) : (cb) => setTimeout(cb, 1500);

let done = false;

/** Once per window, after it has settled. */
export function prewarm() {
  if (done || typeof document === "undefined" || !document.fonts) return;
  done = true;
  const jobs: (() => Promise<unknown>)[] = [];
  for (const f of FAMILIES) for (const s of SAMPLES) jobs.push(() => document.fonts.load(`400 14px "${f}"`, s).catch(() => []));
  const next = () => {
    const job = jobs.shift();
    if (job) idle(() => void job().finally(next));
  };
  // Not during the first second: the window's own start comes first.
  setTimeout(next, 1200);
}
