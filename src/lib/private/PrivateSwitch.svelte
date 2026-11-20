<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // While Private mode is open, the list header's two buttons, in the same place on both lists:
  // on the private list "Normal list" (Private mode stays open) and Lock; on the normal list
  // "Private downloads" (back, no PIN; a shield, so two padlocks never sit side by side when
  // the labels fold away) and Lock. Their violet says Private mode is open; any
  // other way off these two lists locks it (see privateLeave.ts).
  import { priv } from "../private.svelte";
  import Icon from "../Icon.svelte";

  let { on }: { /** Which list shows: "private" or "normal". */ on: "private" | "normal" } = $props();
</script>

<div class="pswitch" role="group" aria-label="Private mode is open">
  {#if on === "private"}
    <button class="btn pv" data-tour="private-normal" aria-label="Normal list" title="Normal list: your other downloads. Private mode stays open." onclick={() => priv.leave()}><Icon name="list" size={13} /><span class="lbl">Normal list</span></button>
  {:else}
    <button class="btn pv" data-tour="private-back" aria-label="Private downloads" title="Private downloads: back to your private list" onclick={() => priv.enter()}><Icon name="shield" size={13} /><span class="lbl">Private downloads</span></button>
  {/if}
  <button class="btn pv" data-tour="private-lock" aria-label="Lock" title="Lock Private mode (Ctrl+Shift+P)" onclick={() => priv.lockByHand()}><Icon name="lock" size={13} /><span class="lbl">Lock</span></button>
</div>

<style>
  .pswitch {
    display: flex;
    gap: 6px;
    flex: none;
  }
  /* Violet only here (and on the sidebar's Private entry): Private mode is open. */
  .pv {
    border-color: var(--priv-line);
    color: var(--priv-text);
    background: var(--priv-tint);
  }
  .pv:hover {
    border-color: var(--priv);
    color: var(--priv-text);
    background: var(--priv-tint);
  }
  /* A narrow list (small window, details panel open): icons only, their names on hover and
     for screen readers; never cut off. */
  @container (max-width: 1100px) {
    .pv {
      width: 32px;
      padding: 0;
    }
    .lbl {
      display: none;
    }
  }
  .pv:focus-visible {
    outline: 2px solid var(--priv);
    outline-offset: 1px;
  }
</style>
