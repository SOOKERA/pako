<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts" generics="T extends string | number">
  // A small segmented choice whose highlight slides to the picked option (Upload basket, details).
  import { untrack } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    options,
    value,
    label,
    onpick,
    small = false,
  }: {
    options: { value: T; label: string; icon?: string; hint?: string }[];
    /** null: none of them (several items set differently). */
    value: T | null;
    /** For screen readers. */
    label: string;
    onpick: (v: T) => void;
    /** Shorter buttons, for a switch inside a card. */
    small?: boolean;
  } = $props();

  let box = $state<HTMLDivElement | null>(null);
  let thumb = $state<{ x: number; y: number; w: number; h: number } | null>(null);
  /** The first placement doesn't slide in from the left edge. */
  let placed = $state(false);

  /** Arrow keys move the choice (and the keyboard) like any radio group; Home/End the ends. */
  function keys(e: KeyboardEvent) {
    const step = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (!step && e.key !== "Home" && e.key !== "End") return;
    e.preventDefault();
    const i = Math.max(0, options.findIndex((o) => o.value === value));
    const to = e.key === "Home" ? 0 : e.key === "End" ? options.length - 1 : (i + step + options.length) % options.length;
    onpick(options[to].value);
    requestAnimationFrame(() => box?.querySelectorAll<HTMLButtonElement>("button")[to]?.focus());
  }

  function measure() {
    const i = options.findIndex((o) => o.value === value);
    // None picked (several items that differ): no highlight.
    if (i < 0) {
      thumb = null;
      return;
    }
    const b = box?.querySelectorAll<HTMLButtonElement>("button")[i];
    if (!b) return;
    const x = b.offsetLeft;
    const y = b.offsetTop;
    const w = b.offsetWidth;
    const h = b.offsetHeight;
    if (thumb && thumb.x === x && thumb.y === y && thumb.w === w && thumb.h === h) return;
    thumb = { x, y, w, h };
  }
  $effect(() => {
    void value;
    void options.length;
    // Only the picked value and the options re-measure: reading `thumb` here as well would loop.
    untrack(() => {
      measure();
      if (!placed && thumb) requestAnimationFrame(() => (placed = true));
    });
  });
  // Fonts loading or the window changing size move the buttons: the highlight follows.
  $effect(() => {
    if (!box) return;
    const ro = new ResizeObserver(() => measure());
    ro.observe(box);
    return () => ro.disconnect();
  });
</script>

<div class="seg" class:small role="radiogroup" tabindex="-1" aria-label={label} bind:this={box} onkeydown={keys}>
  {#if thumb}<span class="thumb" class:placed style="transform: translate({thumb.x - 2}px, {thumb.y - 2}px); width: {thumb.w}px; height: {thumb.h}px" aria-hidden="true"></span>{/if}
  {#each options as o, i (o.value)}
    <button role="radio" aria-checked={value === o.value} tabindex={value === o.value || (i === 0 && !options.some((x) => x.value === value)) ? 0 : -1} class:on={value === o.value} title={o.hint} onclick={() => onpick(o.value)}>
      {#if o.icon}<Icon name={o.icon} size={13} />{/if}{o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    position: relative;
    display: inline-flex;
    /* A narrow place (Upload's basket in a small window): the choices wrap onto a second line
       instead of pushing a sideways scrollbar; the highlight follows them. */
    flex-wrap: wrap;
    max-width: 100%;
    gap: 2px;
    padding: 2px;
    /* the same edge as a button, so the switcher reads as one control on any card */
    border: 1px solid var(--btn-line);
    border-radius: var(--radius);
    background: var(--panel);
    justify-self: start;
  }
  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    border-radius: var(--radius-sm);
    background: var(--active);
    pointer-events: none;
  }
  /* Light: a white marker with a soft shadow on a grey track. */
  :global(:root[data-theme="light"]) .seg {
    background: var(--raise);
  }
  :global(:root[data-theme="light"]) .thumb {
    background: var(--bg);
    box-shadow:
      0 1px 2px rgba(20, 26, 46, 0.12),
      0 0 0 1px rgba(20, 26, 46, 0.04);
  }
  .thumb.placed {
    transition:
      transform var(--mid) var(--ease),
      width var(--mid) var(--ease),
      height var(--mid) var(--ease);
  }
  button {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 12px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font: inherit;
    font-size: 12.5px;
    white-space: nowrap;
    cursor: pointer;
    transition:
      color var(--fast) ease,
      background var(--fast) ease;
  }
  .small button {
    height: 22px;
    padding: 0 9px;
    font-size: 12px;
  }
  button:hover:not(.on) {
    color: var(--text);
    background: var(--raise-hover);
  }
  button.on {
    color: var(--text);
    /* Same width either way, so the highlight doesn't jump when the weight changes. */
    text-shadow: 0 0 0.01px currentColor, 0.3px 0 0 currentColor;
  }
  button:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
</style>
