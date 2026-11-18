<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The small "Tutorial" button in each page's header.
  // Red with a small dot until that page's tutorial has been gone through once, then quiet.
  import Icon from "../Icon.svelte";
  import { app } from "../state.svelte";
  import { tour, type TourId } from "./tour.svelte";

  let { id }: { id: TourId } = $props();
  const fresh = $derived(!!app.settings && !tour.done(id));
</script>

<button
  class="tut"
  class:fresh
  data-tour="tutorial"
  title={fresh ? "A short tour of this page (not seen yet)" : "A short tour of this page"}
  onclick={() => tour.start(id)}
>
  <Icon name="tour" size={12} stroke={2} /><span class="lbl">Tutorial</span>
  <span class="dot" aria-hidden="true"></span>
</button>

<style>
  .tut {
    position: relative;
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--btn-line);
    border-radius: 12px;
    background: transparent;
    color: var(--text-3);
    font-size: 11.5px;
    white-space: nowrap;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      border-color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .tut:hover {
    background: var(--btn-hover);
    color: var(--text);
  }
  .tut:active {
    transform: scale(0.97);
  }
  /* Not gone through yet: Pako's red, with a dot like a notification. */
  .tut.fresh {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
    font-weight: 600;
  }
  .tut.fresh:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
    color: var(--accent-ink);
  }
  .dot {
    position: absolute;
    top: -3px;
    right: -3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 0 2px var(--accent);
    opacity: 0;
    transform: scale(0.92);
    transition:
      opacity var(--slow) var(--ease),
      transform var(--slow) var(--ease);
    pointer-events: none;
  }
  .tut.fresh .dot {
    opacity: 1;
    transform: scale(1);
  }
  /* Narrow windows: the icon alone. */
  @media (max-width: 900px) {
    .tut {
      width: 24px;
      padding: 0;
      justify-content: center;
    }
    .lbl {
      display: none;
    }
  }
</style>
