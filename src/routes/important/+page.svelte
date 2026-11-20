<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The Important card: a download set to Important finished. A big card in the middle of the
  // screen the mouse is on, in a see-through window that never takes the focus (what you were
  // typing goes on where it was). It stays until clicked: Open, Show in folder, ✕, or the card
  // itself (Pako opens on that download). It gets through Do not disturb, but never shows over
  // something full screen (a game, a video, a presentation): it waits, then shows. Several at
  // once: one at a time, the next after the first is closed.
  import { onMount, tick } from "svelte";
  import { cubicIn } from "svelte/easing";
  import type { TransitionConfig } from "svelte/transition";
  import { api, inApp, listen } from "$lib/api";
  import type { PopupCard } from "$lib/notices";
  import Icon from "$lib/Icon.svelte";
  import { pop, reduced, rowSlide } from "$lib/motion";

  /** The see-through margin around the card (holds its shadow), as in the window's width. */
  const PAD = 16;
  /** While something is full screen, asked again this often (ms). */
  const WAIT_CHECK = 2000;

  let cards = $state<PopupCard[]>([]);
  const card = $derived(cards[0] ?? null);
  /** Answers being sent: the buttons wait. */
  let busy = $state(false);
  let error = $state("");

  function add(list: PopupCard[]) {
    const known = new Set(cards.map((c) => c.id));
    const fresh = list.filter((c) => !known.has(c.id));
    if (fresh.length) cards = [...cards, ...fresh];
  }
  async function take() {
    try {
      add(await api.importantTake());
    } catch {
      /* nothing waiting */
    }
  }
  function dismiss() {
    error = "";
    cards = cards.slice(1);
  }
  function closeAll() {
    error = "";
    cards = [];
  }

  // ---------- showing it ----------
  let box: HTMLDivElement | undefined = $state();
  /** The card that has chimed (once each, when it really shows). */
  let chimed = "";
  let waitTimer: ReturnType<typeof setTimeout> | undefined;
  /** The window takes the card's height and shows in the middle of the screen, unless something
   *  is full screen: then it asks again in a moment. */
  async function fit() {
    clearTimeout(waitTimer);
    if (!inApp) return;
    const h = card && box ? Math.ceil(box.offsetHeight + PAD * 2) : 0;
    // The last one closed: the window hides once it has faded out.
    if (!h) {
      waitTimer = setTimeout(() => !card && api.importantFit(0).catch(() => {}), reduced() ? 0 : 200);
      return;
    }
    const waits = await api.importantFit(h).catch(() => false);
    if (!card) return;
    if (waits) {
      waitTimer = setTimeout(fit, WAIT_CHECK);
      return;
    }
    if (chimed !== card.id) {
      chimed = card.id;
      api.importantShown(card.id).catch(() => {});
    }
  }
  // While it shows: something going full screen afterwards (a game, a video, a presentation)
  // hides it until that ends; then it comes back (without chiming again).
  $effect(() => {
    if (!inApp || !card) return;
    const t = setInterval(fit, WAIT_CHECK);
    return () => clearInterval(t);
  });
  $effect(() => {
    void card?.id;
    void cards.length;
    tick().then(fit);
  });
  $effect(() => {
    if (!box) return;
    const ro = new ResizeObserver(() => fit());
    ro.observe(box);
    return () => ro.disconnect();
  });

  // ---------- what it does ----------
  async function openPako() {
    if (!card) return;
    const c = card;
    dismiss();
    await api.popupOpen(c.go, c.id).catch(() => {});
  }
  async function act(run: () => Promise<unknown>) {
    busy = true;
    try {
      await run();
      dismiss();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    if (!inApp) {
      // Design preview (/important in a browser): a sample card; "?private" Private mode's,
      // "?many" with others waiting behind it.
      const q = new URLSearchParams(location.search);
      if (q.has("private"))
        return add([{ id: "i0", kind: "download-done", title: "A private download finished", body: "Open Private mode to see it.", go: { page: "downloads" }, stays: true }]);
      if (q.has("group"))
        return add([{ id: "g1", kind: "download-done", title: "3 downloads finished", body: "Contract.pdf, Invoice.pdf, Receipt.pdf", go: { page: "downloads", id: "m1" }, stays: true, many: 3 }]);
      add([{ id: "i1", kind: "download-done", title: "Download finished", body: "Contract signed - final version.pdf", go: { page: "downloads", id: "m1" }, stays: true }]);
      if (q.has("many"))
        add([
          { id: "i2", kind: "download-done", title: "Download finished", body: "Tax return 2026.pdf", go: { page: "downloads", id: "m2" }, stays: true },
          { id: "i3", kind: "download-done", title: "Download finished", body: "Flight tickets.pdf", go: { page: "downloads", id: "m3" }, stays: true },
        ]);
      return;
    }
    const un = [listen("important-new", () => take())];
    // Listening first: a card posted meanwhile isn't left waiting for the next one.
    Promise.all(un).then(() => take());
    return () => {
      clearTimeout(waitTimer);
      un.forEach((u) => u.then((f) => f()));
    };
  });

  /** Leaving: a quick fade as it settles back a little. */
  function cardOut(_node: Element, { duration = 180 }: { duration?: number } = {}): TransitionConfig {
    if (reduced()) return { duration: 0 };
    return { duration, easing: cubicIn, css: (t) => `opacity:${t};transform:scale(${0.97 + 0.03 * t})` };
  }
</script>

<div class="frame" class:preview={!inApp}>
  {#if card}
    {#key card.id}
      <div class="wrap" bind:this={box} in:pop={{ duration: 260, from: 0.94, y: 10 }} out:cardOut>
        <div
          class="card"
          role="button"
          tabindex="-1"
          onclick={openPako}
          onkeydown={(e) => e.key === "Enter" && e.target === e.currentTarget && openPako()}
        >
          <span class="chip"><Icon name="check" size={22} stroke={2.4} /></span>
          <div class="txt">
            <div class="title">{card.title}</div>
            <div class="name">{error || card.body}</div>
            <div class="acts">
              {#if card.go.id && !card.many}
                <button class="btn primary" disabled={busy} onclick={(e) => (e.stopPropagation(), act(() => api.openItem(card.go.id!, false)))}>Open</button>
                <button class="btn" disabled={busy} onclick={(e) => (e.stopPropagation(), act(() => api.openItem(card.go.id!, true)))}>Show in folder</button>
              {:else if card.go.id}
                <!-- Several, all in one folder: that folder (one of them shown in it). -->
                <button class="btn primary" disabled={busy} onclick={(e) => (e.stopPropagation(), act(() => api.openItem(card.go.id!, true)))}>Show in folder</button>
                <button class="btn" disabled={busy} onclick={(e) => (e.stopPropagation(), openPako())}>Open Pako</button>
              {:else}
                <button class="btn primary" disabled={busy} onclick={(e) => (e.stopPropagation(), openPako())}>Open Pako</button>
              {/if}
            </div>
            {#if cards.length > 1}
              <div class="queue" transition:rowSlide>
                <span class="more">{cards.length - 1} more after this</span>
                <button class="link" onclick={(e) => (e.stopPropagation(), closeAll())}>Close all</button>
              </div>
            {/if}
          </div>
          <button class="close" aria-label="Close" title="Close" onclick={(e) => (e.stopPropagation(), dismiss())}><Icon name="x" size={13} stroke={2.2} /></button>
        </div>
      </div>
    {/key}
  {/if}
</div>

<style>
  /* See-through window: only the card (with its shadow) shows. */
  :global(html),
  :global(body) {
    background: transparent !important;
  }
  /* One cell: the card leaving and the next one coming sit on top of each other, never stacked. */
  .frame {
    min-height: 100vh;
    box-sizing: border-box;
    padding: 16px;
    display: grid;
    align-items: start;
    justify-items: center;
    overflow: hidden;
  }
  /* In a normal browser (design preview), on Pako's background, in the middle. */
  .frame.preview {
    background: var(--bg);
    align-items: center;
  }
  .wrap {
    grid-area: 1 / 1;
    width: 420px;
  }
  .card {
    position: relative;
    display: flex;
    gap: 16px;
    padding: 22px 44px 20px 22px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line-strong);
    /* Pako's colour along the top: you see at a glance it's the important kind. */
    box-shadow:
      inset 0 3px 0 var(--accent),
      0 18px 48px rgba(0, 0, 0, 0.5),
      0 2px 8px rgba(0, 0, 0, 0.3);
    background: rgba(24, 27, 32, 0.98);
    cursor: pointer;
    outline: none;
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease;
  }
  .card:hover {
    background: rgba(30, 34, 40, 0.99);
    border-color: #3d4450;
  }
  :global(:root[data-theme="light"]) .card {
    background: rgba(255, 255, 255, 0.99);
    box-shadow:
      inset 0 3px 0 var(--accent),
      0 18px 48px rgba(20, 26, 46, 0.2),
      0 2px 8px rgba(20, 26, 46, 0.12);
  }
  :global(:root[data-theme="light"]) .card:hover {
    background: var(--panel);
    border-color: var(--control);
  }
  .chip {
    width: 44px;
    height: 44px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 12px;
    color: var(--st-done-text);
    background: color-mix(in srgb, var(--st-done-text) 15%, transparent);
  }
  .txt {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-3);
  }
  .name {
    margin-top: 4px;
    font-size: 19px;
    font-weight: 600;
    line-height: 1.3;
    color: var(--text);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
  }
  /* Later cards waiting: under the buttons, so they never crowd them. */
  .queue {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 14px;
    padding-top: 10px;
    border-top: 1px solid var(--line-soft);
  }
  .more {
    font-size: 11.5px;
    color: var(--text-4);
    white-space: nowrap;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    font-size: 11.5px;
    color: var(--text-3);
    text-decoration: underline;
    text-underline-offset: 2px;
    white-space: nowrap;
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
  }
  .close {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-4);
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .close:hover {
    background: var(--raise-hover);
    color: var(--text);
  }
</style>
