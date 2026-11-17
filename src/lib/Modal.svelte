<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts" module>
  /** Open dialogs, newest last: keys go to the top one only (Help opened from Settings, say). */
  const stack: object[] = [];
</script>

<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import { fadeIn, pop } from "./motion";
  import { app } from "./state.svelte";

  let {
    title,
    width = 480,
    onclose,
    autofocus = true,
    closeLabel = "Close",
    outsideCloses = true,
    closable = true,
    children,
    actions,
  }: {
    title: string;
    width?: number;
    onclose: () => void;
    /** Put the cursor in the dialog's first field when it opens (off for long forms like Settings). */
    autofocus?: boolean;
    /** What the × (and Esc) does, when it's more than closing: "Decline". Shown as its tooltip. */
    closeLabel?: string;
    /** A click beside the dialog closes it (off where that would answer for you: first-start setup). */
    outsideCloses?: boolean;
    /** Off: no ×, and Esc does nothing (the terms of use: only their own buttons answer). */
    closable?: boolean;
    children: Snippet;
    actions?: Snippet;
  } = $props();

  let dialog: HTMLDivElement | undefined = $state();
  let body: HTMLDivElement | undefined = $state();
  /** Long content: a line under the title / over the buttons shows there is more that way. */
  let above = $state(false);
  let below = $state(false);
  function edges() {
    if (!body) return;
    above = body.scrollTop > 1;
    below = body.scrollTop + body.clientHeight < body.scrollHeight - 1;
  }
  $effect(() => {
    if (!body) return;
    edges();
    const ro = new ResizeObserver(edges);
    ro.observe(body);
    for (const c of body.children) ro.observe(c);
    return () => ro.disconnect();
  });

  const FOCUSABLE =
    'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

  // Keyboard users land inside the dialog (its first field, else the dialog itself),
  // and get back to where they were when it closes.
  const me = {};
  onMount(() => {
    app.modals++;
    stack.push(me);
    const before = document.activeElement as HTMLElement | null;
    const field = autofocus ? dialog?.querySelector<HTMLElement>("input:not([disabled]), textarea:not([disabled])") : null;
    (field ?? dialog)?.focus({ preventScroll: true });
    return () => {
      app.modals--;
      stack.splice(stack.indexOf(me), 1);
      if (before?.isConnected) before.focus({ preventScroll: true });
    };
  });

  function key(e: KeyboardEvent) {
    if (stack[stack.length - 1] !== me) return;
    if (e.key === "Escape") {
      e.stopPropagation();
      if (closable) onclose();
    } else if (e.key === "Tab" && dialog) {
      // Tab cycles inside the dialog instead of wandering into the window behind it.
      const items = [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.getClientRects().length > 0);
      if (!items.length) {
        e.preventDefault();
        dialog.focus();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      const inside = !!active && dialog.contains(active) && active !== dialog;
      if (e.shiftKey && (!inside || active === first)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (!inside || active === last)) {
        e.preventDefault();
        first.focus();
      }
    }
  }
</script>

<svelte:window onkeydown={key} />

<div class="backdrop" role="presentation" transition:fadeIn onmousedown={(e) => closable && outsideCloses && e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" aria-label={title} tabindex="-1" bind:this={dialog} style="width: min({width}px, calc(100vw - 48px))" transition:pop>
    <header class:line={above} class:bare={!closable}>
      <h2>{title}</h2>
      {#if closable}
        <button class="icon-btn" aria-label={closeLabel} title={closeLabel === "Close" ? undefined : closeLabel} onclick={onclose}>
          <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M1.5 1.5l9 9M10.5 1.5l-9 9" stroke="currentColor" stroke-width="1.5" /></svg>
        </button>
      {/if}
    </header>
    <div class="body" bind:this={body} onscroll={edges}>{@render children()}</div>
    {#if actions}<footer class:line={below}>{@render actions()}</footer>{/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: var(--scrim);
    /* The blur stays (user, 2026-11-18). Light mode's dim used to be too faint and bluish, so the
       blurred page behind read as a see-through, washed-out window: --scrim is darker there now. */
    backdrop-filter: blur(3px);
  }
  .modal {
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-modal);
  }
  .modal:focus {
    outline: none;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 14px 12px 20px;
    border-bottom: 1px solid transparent;
    transition: border-color var(--fast) ease;
  }
  /* No ×: the same height and the title lined up with the body's text. */
  header.bare {
    min-height: 55px;
    padding-right: 20px;
  }
  header.line,
  footer.line {
    border-color: var(--line);
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .body {
    padding: 2px 20px 4px;
    overflow-y: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 20px 16px;
    border-top: 1px solid transparent;
    transition: border-color var(--fast) ease;
  }
</style>
