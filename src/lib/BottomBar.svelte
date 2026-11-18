<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { tick } from "svelte";
  import { app } from "./state.svelte";
  import { scheduleOpen, speed, hhmm } from "./format";
  import type { WhenDone } from "./api";

  const WHEN: { v: WhenDone; label: string }[] = [
    { v: "nothing", label: "Do nothing" },
    { v: "sleep", label: "Sleep" },
    { v: "shutDown", label: "Shut down" },
  ];
  let whenOpen = $state(false);
  // The menu floats above the bar (the bar itself clips what sticks out of it).
  let whenBtn: HTMLButtonElement | undefined = $state();
  let menuAt = $state({ left: 16, bottom: 40 });
  function toggleWhen() {
    if (!whenOpen && whenBtn) {
      const r = whenBtn.getBoundingClientRect();
      menuAt = { left: Math.max(8, Math.min(r.left, window.innerWidth - 228)), bottom: window.innerHeight - r.top + 6 };
    }
    whenOpen = !whenOpen;
    // Keyboard: the menu takes the focus on the current choice.
    if (whenOpen) tick().then(() => (menuEl?.querySelector<HTMLElement>('[aria-checked="true"]') ?? menuEl?.querySelector<HTMLElement>("button"))?.focus());
  }
  let menuEl: HTMLDivElement | undefined = $state();
  function closeWhen(refocus = true) {
    whenOpen = false;
    if (refocus) whenBtn?.focus();
  }
  /** Esc closes, the arrows move between the choices, Tab leaves (and closes). */
  function menuKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      closeWhen();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const items = [...(menuEl?.querySelectorAll<HTMLElement>("button") ?? [])];
      const at = items.indexOf(document.activeElement as HTMLElement);
      items[(at + (e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length]?.focus();
    } else if (e.key === "Tab") {
      closeWhen(false);
    }
  }
  const whenLabel = $derived(WHEN.find((w) => w.v === app.whenDone)?.label ?? "Do nothing");
  // Re-checked every minute so the "scheduled" note follows the clock.
  let now = $state(new Date());
  $effect(() => {
    const t = setInterval(() => (now = new Date()), 30_000);
    return () => clearInterval(t);
  });
  const scheduledOff = $derived(!scheduleOpen(app.settings, now));

  const max = $derived(app.settings?.maxActive ?? 3);
  const slots = $derived(Array.from({ length: Math.min(max, 10) }, (_, i) => i < app.runningCount));
</script>

<footer>
  <span class="speed mono" class:idle={app.totalSpeed === 0}>
    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" aria-hidden="true"><path d="M12 4v14M6 12l6 6 6-6" /></svg>
    {app.totalSpeed > 0 ? speed(app.totalSpeed) : "0 B/s"}
  </span>

  <span class="when" data-tour="when-done">
    <button bind:this={whenBtn} class="whenbtn" class:set={app.whenDone !== "nothing"} aria-haspopup="menu" aria-expanded={whenOpen} title="What Pako does when every download has finished" onclick={toggleWhen}>
      <svg class="pw" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" aria-hidden="true"><path d="M12 3v9M6.3 7.2a8 8 0 1 0 11.4 0" /></svg>
      {#if app.whenDone === "nothing"}<span class="wtext">When done</span>{:else}<b>{whenLabel} when done</b>{/if}
      <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 6.5L5 3.5l3 3" fill="none" stroke="currentColor" stroke-width="1.6" /></svg>
    </button>
    {#if whenOpen}
      <button class="scrim" aria-label="Close" tabindex="-1" onclick={() => closeWhen(false)}></button>
      <div class="whenmenu" role="menu" tabindex="-1" bind:this={menuEl} onkeydown={menuKey} style="left: {menuAt.left}px; bottom: {menuAt.bottom}px">
        {#each WHEN as w (w.v)}
          <button
            role="menuitemradio"
            aria-checked={app.whenDone === w.v}
            class:on={app.whenDone === w.v}
            onclick={() => {
              closeWhen();
              app.setWhenDone(w.v);
            }}
          >
            <span class="tick">{#if app.whenDone === w.v}<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>{/if}</span>{w.label}
          </button>
        {/each}
        <div class="whennote">Back to "Do nothing" after it happens, and each time Pako starts.</div>
      </div>
    {/if}
  </span>

  <span class="group" data-tour="at-once">
    <span class="atonce">At once</span>
    <span class="stepper">
      <button aria-label="Run fewer downloads at once" disabled={max <= 1} onclick={() => app.setMaxActive(max - 1)}>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 5h7" stroke="currentColor" stroke-width="1.6" /></svg>
      </button>
      <span class="val">
        <span class="slots">
          {#each slots as busy, i (i)}<span class="slot" class:busy></span>{/each}
        </span>
        <span class="mono n">{max}</span>
      </span>
      <button aria-label="Run more downloads at once" disabled={max >= 20} onclick={() => app.setMaxActive(max + 1)}>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 5h7M5 1.5v7" stroke="currentColor" stroke-width="1.6" /></svg>
      </button>
    </span>
    <span class="running">{app.runningCount} running</span>
  </span>

  {#if app.waitingCount}<span class="waiting">{app.waitingCount} waiting</span>{/if}
  {#if scheduledOff && app.settings}<span class="sched" title="Settings › Downloads › Only download at set hours">Downloads start at {hhmm(app.settings.scheduleStart)}</span>{/if}
  {#if app.counts.problems}<span class="bad">{app.counts.problems} {app.counts.problems === 1 ? "problem" : "problems"}</span>{/if}

  <span class="hint">Ctrl+V anywhere to paste links</span>

</footer>

<style>
  footer {
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 0 16px;
    border-top: 1px solid var(--line);
    background: var(--head);
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    container-type: inline-size;
  }
  /* Narrow window: the paste tip goes first, so "When done" always stays in reach. */
  @container (max-width: 1000px) {
    .hint {
      display: none;
    }
  }
  .sched {
    color: var(--st-wait-text);
  }
  .when {
    position: relative;
    flex: none;
  }
  @container (max-width: 560px) {
    .wtext {
      display: none;
    }
  }
  /* Smallest window with the details panel open: the numbers stay, their words go. */
  @container (max-width: 480px) {
    .atonce,
    .running,
    .waiting {
      display: none;
    }
  }
  @container (max-width: 400px) {
    footer {
      gap: 10px;
      padding: 0 10px;
    }
    .speed {
      min-width: 0;
    }
  }
  .whenbtn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-3);
    font: inherit;
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .whenbtn b {
    font-weight: 600;
    color: var(--text-2);
  }
  .whenbtn:hover {
    background: var(--hover);
    color: var(--text-2);
  }
  /* Sleep / shut down is armed: amber, the "heads up" colour (red would read as a problem). */
  .whenbtn.set b,
  .whenbtn.set .pw {
    color: var(--st-retry-text);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    border: none;
    background: transparent;
    cursor: default;
  }
  .whenmenu {
    position: fixed;
    z-index: 61;
    width: 220px;
    padding: 5px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line-strong);
    background: var(--raise);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    white-space: normal;
  }
  :global(:root[data-theme="light"]) .whenmenu {
    box-shadow: var(--shadow-menu);
  }
  .whenmenu button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 9px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    font: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .whenmenu button:focus-visible {
    outline-offset: -2px;
  }
  .whenmenu button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .whenmenu button.on {
    color: var(--text);
  }
  .tick {
    width: 12px;
    display: grid;
    place-items: center;
    color: var(--text);
  }
  .whennote {
    padding: 6px 9px 4px;
    margin-top: 4px;
    border-top: 1px solid var(--line-soft);
    font-size: 11px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .speed {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    min-width: 92px;
  }
  .speed.idle {
    color: var(--text-3);
  }
  .group {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .stepper {
    display: inline-flex;
    align-items: center;
    height: 22px;
    border: 1px solid var(--btn-line);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .stepper button {
    width: 22px;
    height: 100%;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    background: var(--btn);
    color: var(--text);
    transition:
      background var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .stepper button:hover:not(:disabled) {
    background: var(--btn-hover);
  }
  .stepper button:active:not(:disabled) {
    transform: scale(0.85);
  }
  .stepper button:disabled {
    color: var(--text-4);
    cursor: default;
  }
  .val {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border-left: 1px solid var(--btn-line);
    border-right: 1px solid var(--btn-line);
    height: 100%;
  }
  .slots {
    display: inline-flex;
    gap: 2px;
  }
  .slot {
    width: 7px;
    height: 7px;
    border-radius: 1.5px;
    background: var(--track);
    animation: slot-in var(--mid) var(--ease);
    transition: background var(--mid) ease;
  }
  .slot.busy {
    background: var(--st-run);
  }
  @keyframes slot-in {
    from {
      transform: scale(0);
    }
  }
  .n {
    color: var(--text);
    min-width: 12px;
    text-align: center;
  }
  .bad {
    color: var(--st-fail-text);
  }
  .hint {
    margin-left: auto;
  }
</style>
