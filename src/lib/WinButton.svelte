<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A window button of Pako's own title bar (minimise, maximise, close...), the same in every
  // window. `small`: the small windows' lower title bar (same look, scaled down).
  type Kind = "minimize" | "maximize" | "restore" | "close" | "tray" | "pin";
  let {
    kind,
    label,
    title,
    small = false,
    on = false,
    onclick,
  }: { kind: Kind; label: string; title?: string; small?: boolean; on?: boolean; onclick: () => void } = $props();

  /** The glyph's size: 10 px in the main window, 9 px in the small ones (the tray arrow a bit more). */
  const size = $derived((kind === "tray" ? 12 : 10) - (small ? 1 : 0));
</script>

<button class="win" class:close={kind === "close"} class:small class:on aria-label={label} aria-pressed={kind === "pin" ? on : undefined} {title} {onclick}>
  {#if kind === "minimize"}
    <svg width={size} height={size} viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" stroke="currentColor" /></svg>
  {:else if kind === "maximize"}
    <svg width={size} height={size} viewBox="0 0 10 10" aria-hidden="true"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" /></svg>
  {:else if kind === "restore"}
    <svg width={size} height={size} viewBox="0 0 10 10" aria-hidden="true"><path d="M2.5 2.5V.5h7v7h-2M.5 2.5h7v7h-7z" fill="none" stroke="currentColor" /></svg>
  {:else if kind === "close"}
    <svg width={size} height={size} viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" /></svg>
  {:else if kind === "tray"}
    <svg width={size} height={size} viewBox="0 0 12 12" fill="none" stroke="currentColor" aria-hidden="true">
      <path d="M6 .5v6.5M3.5 4.5L6 7l2.5-2.5" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M.5 8v3.5h11V8" stroke-linejoin="round" />
    </svg>
  {:else}
    <!-- Pin: drawn like the other window glyphs (10 px grid, 1 px line). Filled when on. -->
    <svg width={size} height={size} viewBox="0 0 10 10" aria-hidden="true">
      <path d="M3 .5h4M3.5 .5v3.2L1.5 6h7L6.5 3.7V.5z" fill={on ? "currentColor" : "none"} stroke="currentColor" stroke-linejoin="round" />
      <path d="M5 6v3.5" stroke="currentColor" />
    </svg>
  {/if}
</button>

<style>
  .win {
    width: 46px;
    height: 100%;
    flex: none;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: var(--text-2);
    transition: background var(--fast) ease, color var(--fast) ease;
  }
  /* Same proportions as the main window's (46 × 40), on a 32 px bar. */
  .win.small {
    width: 38px;
  }
  .win:hover {
    background: var(--raise-hover);
    color: var(--text);
  }
  .win.on {
    color: var(--accent-strong);
  }
  .win.on:hover {
    color: var(--accent-hover);
  }
  /* Light: the lighter coral fades into the grey hover; the darker edge colour stays clear. */
  :global(:root[data-theme="light"]) .win.on:hover {
    color: var(--accent-strong);
  }
  .win.close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .win:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
</style>
