<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { theme } from "#lib/theme.svelte.ts";
  import { i18n } from "#lib/i18n.svelte.ts";
  import { prewarm } from "#lib/prewarm.ts";
  let { children } = $props();
  // Light or dark, English or French, in every window.
  onMount(() => theme.start());
  onMount(() => i18n.start());
  // First opens as smooth as later ones (fonts' other letter sets, in idle time).
  onMount(prewarm);

  /** The browser's own right-click menu (Back, Refresh, Print…) never shows in Pako's windows:
   *  Pako's own menus open where they're made for. Text boxes and selected text keep it (copy,
   *  paste). */
  function noBrowserMenu(e: MouseEvent) {
    const t = e.target as HTMLElement | null;
    // (A switch, slider or colour box isn't a text box: the browser would show its page menu.)
    const text = "input:not([type=checkbox], [type=radio], [type=range], [type=color], [type=button], [type=submit])";
    if (t?.closest?.(`${text}, textarea, [contenteditable]:not([contenteditable='false'])`)) return;
    if (window.getSelection()?.toString().trim()) return;
    // Development: Shift+right-click keeps it (Inspect).
    if (import.meta.env.DEV && e.shiftKey) return;
    e.preventDefault();
  }
</script>

<svelte:window oncontextmenu={noBrowserMenu} />

{@render children()}
