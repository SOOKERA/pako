<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The details panel's left edge: drag it to make the panel wider or narrower (the list next
  // to it follows at once), double-click it for the panel's first width again. From the
  // keyboard: Tab to it, ← wider, → narrower (Shift: bigger steps), Home widest, End narrowest.
  // The width is remembered per page (panelWidth.svelte.ts).
  import { panelWidths } from "./panelWidth.svelte";
  import { draggedWidth, keyedWidth, LIST_MIN, PANEL_MAX, PANEL_MIN } from "./panelSize";
  import { t } from "./i18n.svelte";

  let {
    page,
    width,
    avail,
    listMin = LIST_MIN,
  }: {
    /** Whose width this is ("downloads", "found", "send", "private", "direct"). */
    page: string;
    /** The width the panel has now. */
    width: number;
    /** The room for the list and the panel together (0: not measured yet). */
    avail: number;
    listMin?: number;
  } = $props();

  let drag: { x: number; w: number; id: number } | null = $state(null);

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { x: e.clientX, w: width, id: e.pointerId };
    panelWidths.dragging = true;
    document.documentElement.classList.add("col-resizing");
  }
  function move(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    // The button was let go somewhere the edge didn't hear it: the drag ends here.
    if (!(e.buttons & 1)) {
      end();
      return;
    }
    panelWidths.set(page, draggedWidth(drag.w, drag.x - e.clientX, avail, listMin), false);
  }
  function up(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    end();
  }
  /** The drag is over (let go, cancelled, or the pointer was taken away): saved, cursor back. */
  function end() {
    if (!drag) return;
    drag = null;
    panelWidths.dragging = false;
    panelWidths.save();
    document.documentElement.classList.remove("col-resizing");
  }
  function key(e: KeyboardEvent) {
    // Space, Enter and Delete on the edge belong to it, never to the list (tick, open, remove).
    if (e.key === " " || e.key === "Enter" || e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    const w = keyedWidth(width, e.key, e.shiftKey, avail, listMin);
    if (w === null) return;
    e.preventDefault();
    e.stopPropagation();
    panelWidths.set(page, w);
  }
</script>

<!-- A focusable separator is a control (ARIA "window splitter"): Tab reaches it, arrows move it. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="edge"
  class:on={!!drag}
  role="separator"
  aria-orientation="vertical"
  aria-label={t("Resize details panel")}
  aria-valuemin={PANEL_MIN}
  aria-valuemax={PANEL_MAX}
  aria-valuenow={width}
  title={t("Drag to resize · double-click for the first width")}
  tabindex="0"
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onlostpointercapture={end}
  ondblclick={() => panelWidths.reset(page)}
  onkeydown={key}
></div>

<style>
  .edge {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 7px;
    z-index: 6;
    cursor: col-resize;
    touch-action: none;
    outline: none;
  }
  /* A thin line on the panel's edge: shown on hover, while dragging and with the keyboard. */
  .edge::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 2px;
    background: var(--focus);
    opacity: 0;
    transition: opacity var(--fast) ease;
  }
  .edge:hover::after {
    opacity: 0.6;
    transition-delay: 120ms;
  }
  .edge.on::after,
  .edge:focus-visible::after {
    opacity: 1;
    transition-delay: 0ms;
  }
  @media (prefers-reduced-motion: reduce) {
    .edge::after {
      transition: none;
    }
  }
  /* While dragging, the pointer stays a resize arrow and nothing gets selected. */
  :global(html.col-resizing),
  :global(html.col-resizing *) {
    cursor: col-resize !important;
    user-select: none !important;
  }
</style>
