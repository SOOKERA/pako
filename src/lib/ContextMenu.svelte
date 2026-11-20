<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { pop } from "./motion";
  import Icon from "./Icon.svelte";
  import type { MenuItem } from "./menu";

  let { x, y, items, onclose }: { x: number; y: number; items: MenuItem[]; onclose: () => void } = $props();

  let el: HTMLDivElement | undefined = $state();
  let left = $state(0);
  let top = $state(0);
  let origin = $state("top left");
  let active = $state(-1);
  /** What had the keyboard before the menu (the columns button, a column's edge, a row): it gets
   *  it back when the menu is closed without choosing (Esc, a click outside). */
  const opener = document.activeElement as HTMLElement | null;
  let giveBack = false;

  onMount(async () => {
    left = x;
    top = y;
    await tick();
    if (!el) return;
    const r = el.getBoundingClientRect();
    const flipX = x + r.width > window.innerWidth - 8;
    const flipY = y + r.height > window.innerHeight - 8;
    left = flipX ? Math.max(8, x - r.width) : x;
    top = flipY ? Math.max(8, y - r.height) : y;
    origin = `${flipY ? "bottom" : "top"} ${flipX ? "right" : "left"}`;
    el.focus();
  });
  onDestroy(() => {
    if (giveBack && opener?.isConnected) opener.focus();
  });

  const usable = $derived(items.map((it, i) => (!it.separator && !it.disabled ? i : -1)).filter((i) => i >= 0));
  /** Ticks changed while the menu stays open, for items whose tick isn't read live. */
  let flipped = $state<Record<number, boolean>>({});
  const isCheck = (it: MenuItem) => it.checked !== undefined;
  const isChecked = (it: MenuItem, i: number) => (typeof it.checked === "function" ? it.checked() : (flipped[i] ?? it.checked));
  const noteOf = (it: MenuItem) => (typeof it.note === "function" ? it.note() : it.note);

  function close(back: boolean) {
    giveBack = back;
    onclose();
  }

  function run(it: MenuItem, i: number) {
    if (it.disabled || it.separator) return;
    if (it.keepOpen) {
      if (typeof it.checked === "boolean") flipped = { ...flipped, [i]: !isChecked(it, i) };
      it.action?.();
      return;
    }
    close(false);
    it.action?.();
  }

  /** The item reached with the keys gets the real focus, so a screen reader follows it. */
  function goTo(i: number) {
    active = i;
    tick().then(() => {
      const b = el?.querySelector<HTMLElement>(`[data-i="${i}"]`);
      b?.focus();
      b?.scrollIntoView({ block: "nearest" });
    });
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation(); // don't also clear the selection
      close(true);
    } else if (e.key === "Tab") {
      e.preventDefault();
      close(true);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const pos = usable.indexOf(active);
      const next = e.key === "ArrowDown" ? (pos + 1) % usable.length : (pos - 1 + usable.length) % usable.length;
      goTo(usable[next < 0 ? usable.length - 1 : next]);
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      goTo(usable[e.key === "Home" ? 0 : usable.length - 1]);
    } else if ((e.key === "Enter" || e.key === " ") && active >= 0) {
      e.preventDefault(); // (also keeps the focused button from clicking a second time)
      e.stopPropagation(); // the window would also open the focused file, or tick a row
      run(items[active], active);
    }
  }
</script>

<svelte:window onblur={onclose} onresize={onclose} />

<div
  class="catcher"
  role="presentation"
  onmousedown={() => close(true)}
  oncontextmenu={(e) => {
    e.preventDefault();
    close(true);
  }}
></div>
<div
  bind:this={el}
  class="menu"
  role="menu"
  tabindex="-1"
  style="left: {left}px; top: {top}px"
  in:pop={{ duration: 160, from: 0.95, y: 0, origin }}
  onkeydown={key}
>
  {#each items as it, i}
    {#if it.separator}
      <div class="sep" role="separator"></div>
    {:else}
      <button
        class="item"
        class:danger={it.danger}
        class:active={active === i}
        role={isCheck(it) ? "menuitemcheckbox" : "menuitem"}
        aria-checked={isCheck(it) ? !!isChecked(it, i) : undefined}
        aria-disabled={it.disabled || undefined}
        tabindex="-1"
        data-i={i}
        disabled={it.disabled}
        onmouseenter={() => (active = i)}
        onfocus={() => (active = i)}
        onclick={() => run(it, i)}
      >
        <span class="ic">
          {#if isCheck(it)}
            {#if isChecked(it, i)}<Icon name="check" size={14} stroke={2.4} />{/if}
          {:else if it.icon}<Icon name={it.icon} size={14} />{/if}
        </span>
        <span class="lbl">{it.label}</span>
        {#if noteOf(it)}<span class="note">{noteOf(it)}</span>{/if}
        {#if it.hint}<span class="hint">{it.hint}</span>{/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .catcher {
    position: fixed;
    inset: 0;
    z-index: 60;
  }
  .menu {
    position: fixed;
    z-index: 61;
    min-width: 232px;
    /* A long menu in a short window (down to 520 px high): it scrolls, so its last items
       (Remove) are never cut off below the window. */
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    padding: 5px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line-strong);
    background: var(--menu-bg);
    backdrop-filter: blur(12px);
    box-shadow: var(--shadow-menu);
    outline: none;
  }
  .item {
    width: 100%;
    height: 30px;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 10px 0 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    text-align: left;
    font-size: 12.5px;
    transition: background 80ms ease;
  }
  .item {
    outline: none;
  }
  .item.active:not(:disabled),
  .item:focus-visible:not(:disabled) {
    background: var(--active);
    color: var(--text);
  }
  .item.danger {
    color: var(--st-fail-text);
  }
  .item.danger.active:not(:disabled) {
    background: var(--danger-fill);
    color: #fff;
  }
  .item:disabled {
    opacity: 0.38;
    cursor: default;
  }
  .ic {
    width: 16px;
    display: grid;
    place-items: center;
    opacity: 0.9;
  }
  .lbl {
    flex: 1;
    white-space: nowrap;
  }
  .hint {
    font-size: 11px;
    opacity: 0.6;
    font-family: var(--mono);
  }
  .note {
    font-size: 11px;
    color: var(--text-3);
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
</style>
