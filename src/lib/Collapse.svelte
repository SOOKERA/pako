<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // What a fold holds (Settings' Advanced, "What Pako can do"): it slides open and shut in
  // height, and stays in the page while shut (inert, so neither the keyboard nor a tutorial
  // reaches it), so Settings' search still finds the settings inside.
  import type { Snippet } from "svelte";

  let { open, id, children }: { open: boolean; id?: string; children: Snippet } = $props();
</script>

<div class="clp" class:open {id} inert={!open}>
  <div class="in">{@render children()}</div>
</div>

<style>
  /* 0fr → 1fr: the height eases to the content's own, whatever it is (Windows' reduce motion:
     at once, app.css). */
  .clp {
    display: grid;
    grid-template-rows: 0fr;
    opacity: 0;
    transition:
      grid-template-rows var(--slow) var(--ease),
      opacity var(--mid) ease;
  }
  .clp.open {
    grid-template-rows: 1fr;
    opacity: 1;
  }
  /* 4px of room on each side so focus rings and switches aren't cut off at the edges; flow-root
     keeps the first row's top margin inside the slide. */
  .in {
    display: flow-root;
    min-height: 0;
    overflow: hidden;
    margin: 0 -4px -4px;
    padding: 0 4px 4px;
  }
</style>
