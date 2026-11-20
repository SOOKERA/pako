// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Each list's columns as you set them (widths, hidden ones), remembered with the window's other
// preferences ("listColumns"). The Downloads widths an older Pako kept ("columns") carry over.
// The rules are in columns.ts.
import { LS } from "./state.svelte";
import { EMPTY, LISTS, clampWidth, readSaved, toggled, type ColDef, type ListState } from "./columns";
import type { MenuItem } from "./menu";

class ListColumns {
  private all = $state<Record<string, ListState>>(readSaved(LS.get("listColumns", null), LS.get("columns", null)));
  /** A column's edge is being dragged in `list`: the columns shown stay as they were at its
   *  start (none steps aside or comes back mid-drag). */
  frozen = $state<{ list: string; keys: string[] } | null>(null);
  /** A column was just shown or hidden from the menu: the lists ease to their new columns
   *  (only then: dragging an edge or resizing the window follows at once). */
  easing = $state(false);
  private easeTimer: ReturnType<typeof setTimeout> | undefined;
  private ease() {
    this.easing = true;
    clearTimeout(this.easeTimer);
    this.easeTimer = setTimeout(() => (this.easing = false), 400);
  }

  state(list: string): ListState {
    return this.all[list] ?? EMPTY;
  }

  setWidth(list: string, c: ColDef, w: number, save = true) {
    const st = this.state(list);
    const v = clampWidth(c, w);
    const { [c.key]: _old, ...rest } = st.w;
    this.all = { ...this.all, [list]: { ...st, w: v === c.def ? rest : { ...rest, [c.key]: v } } };
    if (save) this.save();
  }

  /** One column back to its first width (double-click on its edge). */
  resetWidth(list: string, c: ColDef) {
    this.setWidth(list, c, c.def);
  }

  toggle(list: string, key: string) {
    this.ease();
    this.all = { ...this.all, [list]: toggled(this.state(list), key) };
    this.save();
  }

  /** Every column shown again, at its first width. */
  reset(list: string) {
    this.ease();
    this.all = { ...this.all, [list]: { w: {}, hidden: [] } };
    this.save();
  }

  save() {
    LS.set("listColumns", this.all);
  }

  /** The columns menu (right-click on the column titles, or the button at their end): each
   *  column with a tick when it shows (Name always does), then Reset columns. `onhide`: a
   *  column was hidden (e.g. the list was sorted by it). `noRoom`: the ticked columns the list
   *  has no room for right now (they come back with more room); read live, so ticking one the
   *  list can't fit says so at once. */
  menu(list: string, onhide?: (key: string) => void, noRoom: () => string[] = () => []): MenuItem[] {
    const def = LISTS[list];
    return [
      { label: "Name", checked: true, disabled: true, note: "Always shown" },
      ...def.cols.map(
        (c): MenuItem => ({
          label: c.label,
          checked: () => !this.state(list).hidden.includes(c.key),
          note: () => (noRoom().includes(c.key) && !this.state(list).hidden.includes(c.key) ? "No room now" : undefined),
          keepOpen: true,
          action: () => {
            const hiding = !this.state(list).hidden.includes(c.key);
            this.toggle(list, c.key);
            if (hiding) onhide?.(c.key);
          },
        }),
      ),
      { separator: true },
      { label: "Reset columns", icon: "restart", action: () => this.reset(list) },
    ];
  }
}

export const listColumns = new ListColumns();
