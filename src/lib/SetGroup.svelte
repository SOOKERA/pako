<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Settings: one titled group of a tab ("Speed", "Saving files"…), its rows in one card with a
  // hairline between them. The rows are the tab's own (their padding is set there); a row marked
  // `attached` (the hours under "Only download at set hours", a note under its row) belongs to the
  // row above and gets no line. Settings' search finds groups by their title (`data-group`).
  import type { Snippet } from "svelte";

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<div class="sgroup" data-group={title}>
  <h4 class="gt">{title}</h4>
  <div class="card">{@render children()}</div>
</div>

<style>
  .sgroup {
    margin-top: 18px;
  }
  /* Quiet, the same as the small headings elsewhere in Settings (POP-UP, BELL). */
  .gt {
    margin: 0 0 7px 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .card {
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
  }
  /* A hairline between rows, also between rows that slide in together. */
  .card > :global(* + *:not(.attached)),
  .card > :global(.reveal) > :global(* + *:not(.attached)) {
    border-top: 1px solid var(--line);
  }
  /* Picked from Settings' search: the card itself lights up (a group's tint behind it would be
     hidden by the card), fading like a row's (SettingsDialog.svelte). */
  :global(.sgroup.set-flash)::after {
    display: none;
  }
  .sgroup:global(.set-flash) > .card {
    animation: cardFlash 1.8s var(--ease-out);
  }
  @keyframes cardFlash {
    0%,
    35% {
      border-color: var(--focus);
      box-shadow: 0 0 0 3px var(--focus-soft);
    }
    100% {
      box-shadow: 0 0 0 3px transparent;
    }
  }
  /* Windows' reduce motion: lit up without fading, then off. */
  @media (prefers-reduced-motion: reduce) {
    .sgroup:global(.set-flash) > .card {
      border-color: var(--focus);
      box-shadow: 0 0 0 3px var(--focus-soft);
    }
  }
</style>
