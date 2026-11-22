<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Pako's pop-ups: small cards at the bottom right of the screen, above the taskbar, in a
  // see-through window that never takes the focus. Newest at the bottom, at most 3 at once (the
  // others wait their turn). They leave after a few seconds (not while the mouse is on them);
  // questions (files for you, someone wants to connect, Pako needs you) stay until answered or
  // closed, or until Pako says they are settled elsewhere ("popup-resolve"). The window takes the
  // cards' height (popup_fit), and hides when none is left or while it's no time for pop-ups.
  import { onMount, tick } from "svelte";
  import { cubicIn } from "svelte/easing";
  import type { TransitionConfig } from "svelte/transition";
  import { api, inApp, listen } from "#lib/api.ts";
  import { kindInfo, type PopupCard } from "#lib/notices.ts";
  import { showcase, showcaseCards } from "#lib/showcase.ts";
  import Icon from "#lib/Icon.svelte";
  import { flipIf, flipOpts, reduced, toastIn } from "#lib/motion.ts";

  const SHOWN = 3;
  /** How long a card stays (ms), and at least this long again after the mouse leaves it. */
  const STAY = 6000;
  const AFTER_HOVER = 2500;
  /** The see-through margin around the cards (holds their shadow), as in the window's width. */
  const PAD = 14;

  let cards = $state<PopupCard[]>([]);
  const visible = $derived(cards.slice(0, SHOWN));
  const more = $derived(Math.max(0, cards.length - SHOWN));
  /** Answers being sent: their buttons wait. */
  let busy = $state<Record<string, boolean>>({});
  /** An answer that didn't go through, said on its card. */
  let errors = $state<Record<string, string>>({});

  function add(list: PopupCard[]) {
    const known = new Set(cards.map((c) => c.id));
    const fresh = list.filter((c) => !known.has(c.id));
    if (fresh.length) cards = [...cards, ...fresh];
  }
  async function take() {
    try {
      add(await api.popupTake());
    } catch {
      /* nothing waiting */
    }
  }
  function dismiss(id: string) {
    stopTimer(id);
    cards = cards.filter((c) => c.id !== id);
  }

  // ---------- leaving by themselves ----------
  const timers = new Map<string, { t: ReturnType<typeof setTimeout>; ends: number }>();
  const left = new Map<string, number>();
  let hovered = false;
  function startTimer(id: string, ms: number) {
    stopTimer(id);
    timers.set(id, { t: setTimeout(() => dismiss(id), ms), ends: Date.now() + ms });
  }
  function stopTimer(id: string) {
    const x = timers.get(id);
    if (x) clearTimeout(x.t);
    timers.delete(id);
  }
  // A card that comes into view starts its time (questions have none); not while pointed at or
  // hidden for Do not disturb (it waits, see `hushed`).
  $effect(() => {
    for (const c of visible) {
      if (c.stays || timers.has(c.id) || left.has(c.id)) continue;
      if (hovered || hushed) left.set(c.id, STAY);
      else startTimer(c.id, STAY);
    }
  });
  /** Stops every card's time, keeping what's left of it. */
  function pauseAll() {
    for (const [id, x] of timers) {
      left.set(id, Math.max(0, x.ends - Date.now()));
      stopTimer(id);
    }
  }
  /** Starts the stopped cards' time again (at least AFTER_HOVER, so they can be read). */
  function resumeAll() {
    for (const [id, ms] of left) {
      if (cards.some((c) => c.id === id)) startTimer(id, Math.max(ms, AFTER_HOVER));
    }
    left.clear();
  }
  function enter() {
    hovered = true;
    pauseAll();
  }
  function leave() {
    hovered = false;
    if (!hushed) resumeAll();
  }

  // ---------- no time for pop-ups ----------
  /** Do not disturb, a full-screen game, a presentation: the window hides and the cards wait in
   *  it, their time stopped, and show again for the rest of it afterwards. Looked at every few
   *  seconds while there are cards, and at once when settings change. */
  let hushed = false;
  async function checkQuiet() {
    if (!inApp || !cards.length) return;
    const quiet = await api.popupQuiet().catch(() => false);
    if (quiet) {
      if (!hushed) {
        // Hidden under the pointer: no mouseleave will come.
        hovered = false;
        hushed = true;
        pauseAll();
      }
    } else if (hushed) {
      hushed = false;
      if (!hovered) resumeAll();
      reported = -1;
      fit();
    }
  }
  const anyCards = $derived(cards.length > 0);
  $effect(() => {
    if (!inApp || !anyCards) return;
    const t = setInterval(checkQuiet, 5000);
    return () => clearInterval(t);
  });

  // The last card gone (closed under a still mouse, the window then hidden): no mouseleave may
  // come, so the next card would wait for one. Start afresh instead.
  $effect(() => {
    if (cards.length) return;
    hovered = false;
    left.clear();
    hushed = false;
  });

  // ---------- the window's height ----------
  let stack: HTMLDivElement | undefined = $state();
  let reported = -1;
  let shrinkLater: ReturnType<typeof setTimeout> | undefined;
  /** Grows at once (so a new card is never cut off); shrinks only once cards have finished
   *  moving, so the ones above don't get cut off on their way down. */
  function fit() {
    const need = cards.length && stack ? Math.ceil(stack.offsetHeight + PAD * 2) : 0;
    clearTimeout(shrinkLater);
    if (need > reported) send(need);
    else if (need < reported) shrinkLater = setTimeout(() => send(need), reduced() ? 0 : 420);
  }
  function send(h: number) {
    reported = h;
    if (inApp) api.popupFit(h).catch(() => {});
  }
  $effect(() => {
    void visible.length;
    void more;
    tick().then(fit);
  });
  $effect(() => {
    if (!stack) return;
    const ro = new ResizeObserver(() => fit());
    ro.observe(stack);
    return () => ro.disconnect();
  });

  // ---------- what the cards do ----------
  async function open(c: PopupCard) {
    dismiss(c.id);
    await api.popupOpen(c.go, c.id).catch(() => {});
  }
  /** A click on the card itself. A ready update restarts only from its button (as in the bell):
   *  the card just opens Pako. */
  const openCard = (c: PopupCard) => open(c.kind === "update-ready" ? { ...c, go: { page: "" } } : c);
  async function act(c: PopupCard, run: () => Promise<unknown>) {
    busy = { ...busy, [c.id]: true };
    try {
      await run();
      dismiss(c.id);
    } catch (e) {
      errors = { ...errors, [c.id]: String(e) };
    } finally {
      const { [c.id]: _, ...rest } = busy;
      busy = rest;
    }
  }
  /** Questions settled elsewhere (answered in Pako or the bell, the other person gave up, the
   *  link page finished or closed), or asked again: their cards leave. One being answered here
   *  waits for its own answer (it leaves then, or says what went wrong). */
  function settled(ids: string[]) {
    for (const id of ids) if (!busy[id]) dismiss(id);
  }

  onMount(() => {
    if (!inApp) {
      // Design preview (/popup in a browser): a few sample cards ("?showcase": the website's set).
      if (showcase) return add(showcaseCards);
      // "?direct": Direct's and My devices' cards.
      if (new URLSearchParams(location.search).has("direct"))
        return add([
          { id: "d1", kind: "direct-offer", title: "Files for you", body: "Mia wants to send you Holiday photos (1.2 GB).", go: { page: "direct", offer: "o-mia" }, stays: true },
          { id: "d2", kind: "direct-incoming", title: "Files coming in", body: "Laptop is sending you Project backup (2.4 GB).", go: { page: "devices", id: "laptop" }, stays: false },
          { id: "d3", kind: "direct-sent", title: "Files delivered", body: "Rado has received Render final v3.mp4.", go: { page: "direct", id: "rado" }, stays: false },
        ]);
      if (new URLSearchParams(location.search).get("update") === "ready")
        return add([{ id: "pu", kind: "update-ready", title: "Pako 1.1.1 is ready", body: "Restart Pako to start using it. Your downloads and lists stay.", go: { page: "update" }, stays: false }]);
      add([
        { id: "p1", kind: "browser", title: "Downloading in Pako", body: "Lofi rain cafe - 3 hours of calm music.mp4", go: { page: "downloads" }, stays: false },
        { id: "p2", kind: "download-done", title: "Download finished", body: "Project files (final).zip", go: { page: "downloads", id: "m1" }, stays: false },
        { id: "p3", kind: "direct-offer", title: "Files for you", body: "Mia wants to send you Holiday photos (1.2 GB).", go: { page: "direct", offer: "o-mia" }, stays: true },
        { id: "p4", kind: "needs-you", title: "Pako needs you", body: "Solve a quick check to open a link from ouo.io.", go: { page: "found" }, stays: true },
      ]);
      return;
    }
    const un = [
      listen("popup-new", () => take()),
      listen<string[]>("popup-resolve", (e) => settled(e.payload)),
      listen("popup-quiet-check", () => checkQuiet()),
    ];
    // Listening first: a pop-up posted meanwhile isn't left waiting for the next one.
    Promise.all(un).then(() => take());
    return () => un.forEach((u) => u.then((f) => f()));
  });

  /** Leaving: slides off to the right while it fades (the cards above then move down). */
  function cardOut(_node: Element, { duration = 220 }: { duration?: number } = {}): TransitionConfig {
    if (reduced()) return { duration: 0 };
    return { duration, easing: cubicIn, css: (t) => `opacity:${t};transform:translateX(${(1 - t) * 48}px)` };
  }
  const flipNow = $derived({ ...flipOpts, duration: reduced() ? 0 : flipOpts.duration });

  const tones: Record<string, string> = {
    done: "var(--st-done-text)",
    fail: "var(--st-fail-text)",
    info: "var(--st-run-text)",
    wait: "var(--st-retry-text)",
    people: "var(--st-wait-text)",
  };
</script>

<div class="frame" class:preview={!inApp}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="stack" bind:this={stack} onmouseenter={enter} onmouseleave={leave}>
    {#if more}
      <div class="more">{more} more waiting</div>
    {/if}
    {#each visible as c (c.id)}
      {@const k = kindInfo(c.kind)}
      <div class="slot" in:toastIn={{ duration: 300 }} out:cardOut animate:flipIf={flipNow}>
        <div
          class="card"
          role="button"
          tabindex="-1"
          onclick={() => openCard(c)}
          onkeydown={(e) => e.key === "Enter" && e.target === e.currentTarget && openCard(c)}
        >
          <span class="chip" style:color={tones[k.tone]}><Icon name={k.icon} size={15} stroke={2} /></span>
          <div class="txt">
            <div class="title">{c.title}</div>
            <div class="body">{errors[c.id] ?? c.body}</div>
            {#if c.kind === "download-done" && c.go.id}
              <div class="acts">
                <button class="btn small" disabled={busy[c.id]} onclick={(e) => (e.stopPropagation(), act(c, () => api.openItem(c.go.id!, false)))}>Open</button>
                <button class="btn small" disabled={busy[c.id]} onclick={(e) => (e.stopPropagation(), act(c, () => api.openItem(c.go.id!, true)))}>Show in folder</button>
              </div>
            {:else if c.kind === "direct-offer" && c.go.offer}
              <div class="acts">
                <button class="btn small" disabled={busy[c.id]} onclick={(e) => (e.stopPropagation(), act(c, () => api.directAnswerOffer(c.go.offer!, false)))}>Decline</button>
                <button class="btn primary small" disabled={busy[c.id]} onclick={(e) => (e.stopPropagation(), act(c, () => api.directAnswerOffer(c.go.offer!, true)))}
                  ><Icon name="download" size={12} stroke={2.2} />Accept</button
                >
              </div>
            {:else if c.kind === "direct-request"}
              <div class="acts">
                <button class="btn small" onclick={(e) => (e.stopPropagation(), open(c.go.page === "devices" ? c : { ...c, go: { page: "direct" } }))}>Review</button>
              </div>
            {:else if c.kind === "update-ready"}
              <div class="acts">
                <button class="btn primary small" onclick={(e) => (e.stopPropagation(), open(c))}>Restart now</button>
              </div>
            {:else if c.kind === "upload-done" && c.go.id}
              <div class="acts">
                <button class="btn small" onclick={(e) => (e.stopPropagation(), open(c))}>Show in Upload</button>
              </div>
            {/if}
          </div>
          <button class="close" aria-label="Close" title="Close" onclick={(e) => (e.stopPropagation(), dismiss(c.id))}><Icon name="x" size={11} stroke={2.2} /></button>
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  /* See-through window: only the cards (with their shadow) show. */
  :global(html),
  :global(body) {
    background: transparent !important;
  }
  .frame {
    height: 100vh;
    box-sizing: border-box;
    padding: 14px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    overflow: hidden;
  }
  /* In a normal browser (design preview), on Pako's background. */
  .frame.preview {
    background: var(--bg);
    align-items: flex-end;
  }
  .stack {
    position: relative;
    width: 340px;
    display: flex;
    flex-direction: column;
  }
  .more {
    align-self: center;
    margin-bottom: 2px;
    padding: 3px 10px;
    border-radius: 10px;
    background: rgba(28, 31, 37, 0.92);
    border: 1px solid var(--line-strong);
    font-size: 11px;
    color: var(--text-3);
  }
  .slot {
    padding-top: 8px;
  }
  .card {
    position: relative;
    display: flex;
    gap: 11px;
    padding: 12px 34px 12px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line-strong);
    background: rgba(24, 27, 32, 0.97);
    box-shadow:
      0 8px 22px rgba(0, 0, 0, 0.42),
      0 1px 4px rgba(0, 0, 0, 0.3);
    cursor: pointer;
    outline: none;
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease;
  }
  .card:hover {
    background: rgba(30, 34, 40, 0.98);
    border-color: #3d4450;
  }
  /* Light: white cards, a softer shadow in two layers (still clear on any desktop). */
  :global(:root[data-theme="light"]) .more {
    background: rgba(255, 255, 255, 0.94);
  }
  :global(:root[data-theme="light"]) .card {
    background: rgba(255, 255, 255, 0.98);
    box-shadow:
      0 8px 24px rgba(20, 26, 46, 0.16),
      0 1px 4px rgba(20, 26, 46, 0.1);
  }
  :global(:root[data-theme="light"]) .card:hover {
    background: var(--panel);
    border-color: var(--control);
  }
  .chip {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: color-mix(in srgb, currentColor 14%, transparent);
  }
  .txt {
    flex: 1;
    min-width: 0;
    padding-top: 1px;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .body {
    margin-top: 2px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    gap: 6px;
    margin-top: 9px;
  }
  .btn.small {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
  .close {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-4);
    opacity: 0.7;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      opacity var(--fast) ease;
  }
  .card:hover .close {
    opacity: 1;
  }
  .close:hover {
    background: var(--raise-hover);
    color: var(--text);
  }
</style>
