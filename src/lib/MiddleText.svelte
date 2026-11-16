<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
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

  const shown = $derived(width && font ? middleEllipsis(text, width - 1, font) : text);
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
