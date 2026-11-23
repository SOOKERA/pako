<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A small "?" next to something tricky: opens Help on the guide section that explains it.
  import Icon from "../Icon.svelte";
  import { help } from "./help.svelte";
  import { t, tMaybe } from "../i18n.svelte";

  /** `label`: its words (already translated, or English words of ours: they are translated here). */
  let { to, label }: { to: string; label?: string } = $props();
  const words = $derived(label ? tMaybe(label) : t("How this works"));
</script>

<button class="hl" title={words} aria-label={words} onclick={() => help.show(to)}><Icon name="help" size={15} stroke={1.8} /></button>

<style>
  .hl {
    flex: none;
    width: 24px;
    height: 24px;
    display: inline-grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: var(--text-3);
    vertical-align: middle;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .hl:hover {
    background: var(--raise);
    color: var(--text);
  }
  .hl:active {
    transform: scale(0.97);
  }
</style>
