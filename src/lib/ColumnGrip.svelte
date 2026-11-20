<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A column's left edge in a list's titles: drag it to make the column wider or narrower (the
  // list follows at once), double-click it for the first width again. From the keyboard: Tab to
  // it, ← wider, → narrower (Shift: bigger steps), Home widest, End narrowest. A column can't
  // grow past the room left (Name keeps its smallest width), and while it's dragged no other
  // column steps aside or comes back. Remembered per list (columns.svelte.ts). Same as the
  // details panel's edge (PanelResizer).
  import { onMount } from "svelte";
  import { listColumns } from "./columns.svelte";
  import { LISTS, draggedColumn, keyedColumn, roomFor, type ColDef } from "./columns";

  let {
    list,
    col,
    keys,
    tight,
    avail,
  }: {
    list: string;
    col: ColDef;
    /** The columns shown now, and whether they're already down to their smallest widths. */
    keys: string[];
    tight: boolean;
    /** The list's room (its column titles' width; 0 = not measured yet). */
    avail: number;
  } = $props();

  const def = $derived(LISTS[list]);
  let me: HTMLElement | undefined = $state();
  /** The column's width as drawn (it may be squeezed below the one chosen). */
  let drawn = $state(0);
  let drag: { x: number; w: number; id: number; max: number } | null = $state(null);
  /** No room to make anything wider (the columns are at their smallest): nothing to drag. */
  const inert = $derived(tight && !drag);

  const cellOf = (key: string) => me?.parentElement?.parentElement?.querySelector<HTMLElement>(`.c-${key}`) ?? null;
  const widthNow = (key: string) => Math.max(0, Math.round((cellOf(key)?.getBoundingClientRect().width ?? 0) - def.gap));
  onMount(() => {
    const cell = me?.parentElement;
    if (!cell) return;
    const ro = new ResizeObserver(() => (drawn = widthNow(col.key)));
    ro.observe(cell);
    return () => ro.disconnect();
  });
  /** The widest it can be now: the room left with the other columns as drawn. */
  function maxNow() {
    const widths: Record<string, number> = {};
    for (const k of keys) widths[k] = widthNow(k);
    return roomFor(def, keys, widths, avail, col.key);
  }

  function down(e: PointerEvent) {
    if (e.button !== 0 || inert) return;
    e.preventDefault();
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { x: e.clientX, w: widthNow(col.key), id: e.pointerId, max: maxNow() };
    listColumns.frozen = { list, keys: [...keys] };
    document.documentElement.classList.add("col-resizing");
  }
  function move(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    // The button was let go somewhere the edge didn't hear it: the drag ends here.
    if (!(e.buttons & 1)) {
      end();
      return;
    }
    listColumns.setWidth(list, col, draggedColumn(col, drag.w, e.clientX - drag.x, drag.max), false);
  }
  function up(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    end();
  }
  /** The drag is over (let go, cancelled, or the pointer was taken away): saved, cursor back. */
  function end() {
    if (!drag) return;
    drag = null;
    listColumns.frozen = null;
    listColumns.save();
    document.documentElement.classList.remove("col-resizing");
  }
  function key(e: KeyboardEvent) {
    // Space, Enter and Delete on the edge belong to it, never to the list (tick, open, remove).
    if (e.key === " " || e.key === "Enter" || e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    if (inert) return;
    const w = keyedColumn(col, widthNow(col.key), e.key, e.shiftKey, maxNow());
    if (w === null) return;
    e.preventDefault();
    e.stopPropagation(); // the list's own arrow keys move through the rows
    listColumns.setWidth(list, col, w);
  }
</script>

<!-- A focusable separator is a control (ARIA "window splitter"): Tab reaches it, arrows move it. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<span
  bind:this={me}
  class="grip"
  class:on={!!drag}
  class:inert
  role="separator"
  aria-orientation="vertical"
  aria-label="Resize {col.label} column"
  aria-valuemin={col.min}
  aria-valuemax={col.max}
  aria-valuenow={drawn}
  aria-disabled={inert || undefined}
  tabindex={inert ? -1 : 0}
  title={inert ? undefined : "Drag to resize · double-click for the first width"}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={end}
  onlostpointercapture={end}
  ondblclick={(e) => {
    e.stopPropagation();
    if (!inert) listColumns.resetWidth(list, col);
  }}
  onclick={(e) => e.stopPropagation()}
  onkeydown={key}
></span>

<style>
  .grip {
    position: absolute;
    left: -5px;
    top: 6px;
    bottom: 6px;
    width: 9px;
    z-index: 1;
    cursor: col-resize;
    touch-action: none;
    outline: none;
  }
  /* The resting line between titles; on hover, while dragging and with the keyboard it turns
     into the same blue line as the details panel's edge. */
  .grip::after {
    content: "";
    position: absolute;
    left: 4px;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--line);
    transition:
      background var(--fast) var(--ease),
      box-shadow var(--fast) var(--ease);
  }
  .grip:hover::after {
    background: var(--focus);
    box-shadow: 0 0 0 0.5px var(--focus);
    transition-delay: 80ms;
  }
  .grip.on::after,
  .grip:focus-visible::after {
    background: var(--focus);
    box-shadow: 0 0 0 1px var(--focus);
    transition-delay: 0ms;
  }
  .grip.inert {
    cursor: default;
  }
  .grip.inert:hover::after {
    background: var(--line);
    box-shadow: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .grip::after {
      transition: none;
    }
  }
  :global(html.col-resizing),
  :global(html.col-resizing *) {
    cursor: col-resize !important;
    user-select: none !important;
  }
</style>
