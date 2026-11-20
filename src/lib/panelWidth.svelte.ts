// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The details panels' widths, one per page ("downloads", "found", "send", "private",
// "direct"), remembered with the window's other preferences (it's only a width: the private
// list's is no private data). See panelSize.ts for the rules.
import { LS } from "./state.svelte";
import { PANEL_DEFAULT, savedWidth } from "./panelSize";

class PanelWidths {
  private widths = $state<Record<string, number>>(LS.get("panelWidths", {}));
  /** A panel's edge is being dragged: the panels follow the pointer at once (no easing). */
  dragging = $state(false);

  /** The width remembered for `page` (its default until it's dragged). */
  of(page: string, def = PANEL_DEFAULT): number {
    return savedWidth(this.widths[page], def);
  }

  /** A new width for `page`; `save` false while dragging (saved once it ends). */
  set(page: string, w: number, save = true) {
    this.widths = { ...this.widths, [page]: savedWidth(w) };
    if (save) LS.set("panelWidths", this.widths);
  }

  /** Back to `page`'s default width (double-click on the edge). */
  reset(page: string) {
    const { [page]: _gone, ...rest } = this.widths;
    this.widths = rest;
    LS.set("panelWidths", this.widths);
  }

  save() {
    LS.set("panelWidths", this.widths);
  }
}

export const panelWidths = new PanelWidths();
