<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { app } from "./state.svelte";
  import { fadeIn, toastIn, flipIf, flipOpts } from "./motion";
  import Icon from "./Icon.svelte";

  /** `plain`: only messages without a button (the small tray window: "Link copied", what went
   *  wrong; "Show" and "Restart now" belong to Pako's own window). */
  let { plain = false }: { plain?: boolean } = $props();
  const list = $derived(plain ? app.toasts.filter((t) => !t.action) : app.toasts);
  function hold(on: boolean) {
    app.toastHold = on;
    if (!on) app.toastFree = performance.now();
  }
</script>

<div
  class="toasts"
  aria-live="polite"
  role="region"
  aria-label="Messages"
  onpointerenter={() => hold(true)}
  onpointerleave={() => hold(false)}
  onfocusin={() => hold(true)}
  onfocusout={() => hold(false)}
>
  {#each list as t (t.id)}
    <div class="toast" class:error={t.kind === "error"} role={t.kind === "error" ? "alert" : undefined} in:toastIn out:fadeIn={{ duration: 140 }} animate:flipIf={flipOpts}>
      <Icon name={t.kind === "error" ? "alert" : "check"} size={14} stroke={2} />
      <span>{t.text}</span>
      {#if t.action}
        <button
          class="act"
          onclick={() => {
            app.toasts = app.toasts.filter((x) => x.id !== t.id);
            t.action?.run();
          }}>{t.action.label}</button
        >
      {/if}
      <button aria-label="Dismiss" onclick={() => (app.toasts = app.toasts.filter((x) => x.id !== t.id))}><Icon name="x" size={11} stroke={2.2} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 44px;
    z-index: 70;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: min(420px, calc(100vw - 32px));
  }
  .toast {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 10px 10px 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line-strong);
    background: rgba(30, 34, 41, 0.97);
    backdrop-filter: blur(12px);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--text);
  }
  .toast :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--st-done);
  }
  .toast.error {
    border-color: rgba(242, 75, 79, 0.4);
  }
  .toast.error :global(svg) {
    color: var(--st-fail-text);
  }
  .toast span {
    flex: 1;
    overflow-wrap: anywhere;
  }
  .toast button.act {
    width: auto;
    height: 22px;
    margin-top: -1px;
    padding: 0 9px;
    border-radius: var(--radius);
    background: var(--line-strong);
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
  }
  .toast button.act:hover {
    background: #414854;
    color: #fff;
  }
  :global(:root[data-theme="light"]) .toast {
    background: var(--pop-bg);
    box-shadow: var(--shadow-menu);
  }
  :global(:root[data-theme="light"]) .toast button.act:hover {
    background: var(--active-hover);
    color: var(--text);
  }
  .toast button {
    width: 20px;
    height: 20px;
    flex: none;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-3);
  }
  .toast button:hover {
    background: var(--raise);
    color: var(--text);
  }
  .toast button :global(svg) {
    color: inherit;
    margin: 0;
  }
</style>
