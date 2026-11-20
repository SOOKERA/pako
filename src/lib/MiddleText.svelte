<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts" module>
  import { clearWidths } from "./format";
  /** Bumped when fonts finish loading: every cut text is measured again with the real font
   *  (widths measured earlier used the stand-in font and could cut without "…"). */
  let fontEpoch = $state(0);
  if (typeof document !== "undefined" && document.fonts) {
    const bump = () => {
      clearWidths();
      fontEpoch++;
    };
    document.fonts.addEventListener("loadingdone", bump);
    document.fonts.ready.then(bump);
  }
</script>

<script lang="ts">
  // Text cut in the middle when too long ("Project render fin…v3.mp4"), full text on hover.
  import { middleEllipsis } from "./format";

  let { text, class: cls = "" }: { text: string; class?: string } = $props();
  let el: HTMLSpanElement | undefined = $state();
  let width = $state(0);
  let font = $state("");

  $effect(() => {
    if (!el) return;
    const cs = getComputedStyle(el);
    font = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
    const ro = new ResizeObserver(() => (width = el!.clientWidth));
    ro.observe(el);
    width = el.clientWidth;
    return () => ro.disconnect();
  });

  const shown = $derived.by(() => {
    void fontEpoch;
    return width && font ? middleEllipsis(text, width - 1, font) : text;
  });
</script>

<span bind:this={el} class="mt {cls}" title={shown !== text ? text : undefined}>{shown}</span>

<style>
  .mt {
    display: block;
    min-width: 0;
    flex: 1 1 auto;
    white-space: nowrap;
    overflow: hidden;
  }
</style>
