// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The lists' columns: which ones show, how wide they are, and the grid they make. Every list
// with columns (Downloads, Links, the private list, Upload) works the same way: Name takes the
// room left over; each other column can be made wider or narrower (drag its left edge, or the
// keyboard) and shown or hidden (right-click the column titles); a narrow window drops the
// least important columns first, but never shows one you hid. Pure: no window, so the rules
// are tested in Node (test/columns.test.mjs).

export interface ColDef {
  key: string;
  label: string;
  /** First width, and the smallest and largest it can be made (px). */
  def: number;
  min: number;
  max: number;
  /** When the window is too narrow: lower goes first; 0 = never dropped by itself. */
  drop: number;
}

export interface ListDef {
  id: string;
  cols: ColDef[];
  /** The fixed columns before Name (tick box, type icon…), as grid tracks and their widths. */
  pre: string[];
  prePx: number[];
  /** The fixed columns after the others (the row's buttons, the columns button), likewise. */
  post: string[];
  postPx: number[];
  /** Room between columns (px). It's part of each track after the first ones (the cell keeps it
   *  as padding on its left, see GAP_CSS), so a hidden column leaves no gap of its own. */
  gap: number;
  pad: number;
  /** Name's smallest width, and how far it may go in a narrow window before columns step
   *  aside. */
  nameMin: number;
  nameTight: number;
}

/** What's remembered for a list: the widths changed from the first ones, and the columns hidden. */
export interface ListState {
  w: Record<string, number>;
  hidden: string[];
}

export const EMPTY: ListState = { w: {}, hidden: [] };

/** Keyboard steps for a column's edge (px); Shift for bigger ones. */
export const STEP = 8;
export const BIG_STEP = 40;

const col = (key: string, label: string, def: number, min: number, max: number, drop: number): ColDef => ({ key, label, def, min, max, drop });

/** The lists with columns. */
export const LISTS: Record<string, ListDef> = {
  downloads: {
    id: "downloads",
    cols: [
      // (Up to 420 px: the widest an older Pako let these be dragged.)
      col("source", "Source", 128, 70, 420, 1),
      col("progress", "Progress", 156, 110, 420, 0),
      col("speed", "Speed", 92, 70, 420, 3),
      col("status", "Status", 122, 80, 420, 4),
      col("date", "Date", 112, 80, 420, 2),
    ],
    pre: ["30px", "46px"],
    prePx: [30, 46],
    post: ["24px"],
    postPx: [24],
    gap: 0,
    pad: 32,
    nameMin: 160,
    // A small window with the details panel open: columns step aside before a name gets
    // too short to read.
    nameTight: 130,
  },
  found: {
    id: "found",
    cols: [col("source", "Source", 140, 80, 300, 1), col("size", "Size", 96, 64, 180, 3), col("link", "Link", 220, 120, 420, 4), col("date", "Added on", 112, 80, 200, 2)],
    pre: ["30px", "46px"],
    prePx: [30, 46],
    post: ["24px"],
    postPx: [24],
    gap: 0,
    pad: 32,
    nameMin: 160,
    nameTight: 130,
  },
  private: {
    id: "private",
    // "Downloaded" also says how a file is doing (downloading, failed, decrypting…): it never
    // steps aside by itself.
    cols: [col("size", "Size", 112, 70, 200, 1), col("date", "Downloaded", 116, 90, 220, 0)],
    pre: ["18px", "24px"],
    prePx: [18, 24],
    post: ["112px"],
    postPx: [112],
    gap: 12,
    pad: 20,
    nameMin: 150,
    nameTight: 80,
  },
  send: {
    id: "send",
    cols: [col("host", "Uploaded to", 216, 120, 360, 2), col("link", "Link", 230, 120, 420, 1)],
    pre: ["26px"],
    prePx: [26],
    post: ["172px"],
    postPx: [172],
    gap: 12,
    pad: 22,
    nameMin: 150,
    nameTight: 140,
  },
};

/** The list with a different column of row buttons at the end (a narrow list shows only their
 *  icons, or puts them over the last column). */
export function withPost(list: ListDef, px: number): ListDef {
  return { ...list, post: [`${px}px`], postPx: [px] };
}

/** A width kept within the column's limits (a broken saved value: its first width). */
export function clampWidth(c: ColDef, w: unknown): number {
  const n = typeof w === "number" && Number.isFinite(w) ? Math.round(w) : c.def;
  return Math.min(c.max, Math.max(c.min, n));
}

/** A column's width now: as changed, else its first width. */
export function widthOf(list: ListDef, st: ListState, key: string): number {
  const c = list.cols.find((x) => x.key === key);
  if (!c) return 0;
  return key in st.w ? clampWidth(c, st.w[key]) : c.def;
}

/** Room the fixed columns take (with their gaps, see `gap`). */
function fixedPx(list: ListDef): number {
  return list.pad + [...list.prePx, ...list.postPx].reduce((a, b) => a + b + list.gap, 0);
}

/** The columns shown in `avail` px (0 = not measured yet: all the ones not hidden), in order,
 *  and whether Name has to go down to its narrow width (`tight`). Shown columns can give way
 *  down to their smallest widths, so a column only steps aside (by its `drop` order, never the
 *  ones with 0) when even then there's no room. The user's hidden ones never show. */
export function shownColumns(list: ListDef, st: ListState, avail: number): { keys: string[]; tight: boolean } {
  const hidden = new Set(st.hidden);
  let keys = list.cols.filter((c) => !hidden.has(c.key)).map((c) => c.key);
  if (avail <= 0) return { keys, tight: false };
  const need = (ks: string[], name: number) => fixedPx(list) + name + ks.reduce((a, k) => a + list.cols.find((c) => c.key === k)!.min + list.gap, 0);
  while (need(keys, list.nameTight) > avail) {
    const next = keys
      .map((k) => list.cols.find((c) => c.key === k)!)
      .filter((c) => c.drop > 0)
      .sort((a, b) => a.drop - b.drop)[0];
    if (!next) break;
    keys = keys.filter((k) => k !== next.key);
  }
  return { keys, tight: need(keys, list.nameMin) > avail };
}

/** The grid: the fixed columns, Name taking what's left, then every other column in its place:
 *  a shown one between its smallest width and its own (Name keeps its smallest first), one not
 *  shown at no width. The same tracks either way, so showing or hiding one can ease. */
export function gridTemplate(list: ListDef, st: ListState, keys: string[], tight: boolean): string {
  const g = list.gap;
  const px = (n: number) => `${n + g}px`;
  const name = `minmax(${tight ? list.nameTight : list.nameMin}px, 1fr)`;
  // So little room that Name needs its narrow width: the columns left keep only their smallest
  // width, and Name gets the rest.
  const data = list.cols.map((c) =>
    !keys.includes(c.key) ? "minmax(0px, 0px)" : tight ? `minmax(${px(c.min)}, ${px(c.min)})` : `minmax(${px(c.min)}, ${px(widthOf(list, st, c.key))})`,
  );
  const fixed = (tracks: string[], widths: number[]) => (g ? widths.map(px) : tracks);
  return [...fixed(list.pre, list.prePx), name, ...data, ...fixed(list.post, list.postPx)].join(" ");
}

/** The widest a column can be dragged now: what's left once Name keeps its smallest width and
 *  the other columns shown keep theirs (`widths`: as drawn now). */
export function roomFor(list: ListDef, keys: string[], widths: Record<string, number>, avail: number, key: string): number {
  const c = list.cols.find((x) => x.key === key)!;
  if (avail <= 0) return c.max;
  const others = keys.filter((k) => k !== key).reduce((a, k) => a + (widths[k] ?? 0) + list.gap, 0);
  const room = avail - fixedPx(list) - list.nameMin - others - list.gap;
  return Math.max(c.min, Math.min(c.max, Math.floor(room)));
}

/** A column's width after its edge moved `dx` px to the right (the edge is its left side:
 *  moving it left makes the column wider). */
export function draggedColumn(c: ColDef, startW: number, dx: number, max = c.max): number {
  return Math.min(max, clampWidth(c, startW - dx));
}

/** A column's width after a key on its edge (← wider, → narrower, Shift: bigger steps, Home
 *  widest, End narrowest); null for other keys. */
export function keyedColumn(c: ColDef, w: number, key: string, shift: boolean, max = c.max): number | null {
  const step = shift ? BIG_STEP : STEP;
  const top = Math.max(c.min, Math.min(c.max, max));
  if (key === "ArrowLeft") return Math.min(top, clampWidth(c, w + step));
  if (key === "ArrowRight") return Math.min(top, clampWidth(c, w - step));
  if (key === "Home") return top;
  if (key === "End") return c.min;
  return null;
}

/** A column shown or hidden (Name isn't one of them: it always shows). */
export function toggled(st: ListState, key: string): ListState {
  const hidden = st.hidden.includes(key) ? st.hidden.filter((k) => k !== key) : [...st.hidden, key];
  return { ...st, hidden };
}

/** Saved settings read back: unknown lists and columns, broken values and hidden columns that
 *  don't exist are left out. `legacy`: the Downloads widths an older Pako saved ("columns"),
 *  kept when nothing newer is there. */
export function readSaved(raw: unknown, legacy?: unknown): Record<string, ListState> {
  const out: Record<string, ListState> = {};
  const src = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  for (const id of Object.keys(LISTS)) {
    const list = LISTS[id];
    const one = src[id] && typeof src[id] === "object" ? (src[id] as Partial<ListState>) : null;
    const w: Record<string, number> = {};
    const fromW = one?.w && typeof one.w === "object" ? one.w : id === "downloads" && legacy && typeof legacy === "object" && !one ? (legacy as Record<string, unknown>) : {};
    for (const c of list.cols) {
      const v = (fromW as Record<string, unknown>)[c.key];
      if (typeof v === "number" && Number.isFinite(v) && Math.round(v) !== c.def) w[c.key] = clampWidth(c, v);
    }
    const hidden = Array.isArray(one?.hidden) ? one!.hidden.filter((k): k is string => typeof k === "string" && list.cols.some((c) => c.key === k)) : [];
    out[id] = { w, hidden: [...new Set(hidden)] };
  }
  return out;
}
