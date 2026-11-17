<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The bell in the top bar: what's important (people asking to connect, files waiting for your
  // answer, links that need you, and whatever else Settings › Notifications sends here), kept 30
  // days. A red count while something is unread; opening the panel doesn't mark anything read,
  // clicking a line or "Mark all as read" does.
  import { tick } from "svelte";
  import { app } from "./state.svelte";
  import { api } from "./api";
  import { isToday, kindInfo, noticeTime, type Notice } from "./notices";
  import Icon from "./Icon.svelte";
  import { fadeIn, flipIf, flipOpts, pop, rowSlide } from "./motion";

  let { onsettings }: { onsettings?: () => void } = $props();

  let btn: HTMLButtonElement | undefined = $state();
  let panel: HTMLDivElement | undefined = $state();
  let at = $state({ top: 46, right: 8 });

  async function toggle() {
    if (app.bellOpen) return close();
    if (btn) {
      const r = btn.getBoundingClientRect();
      at = { top: r.bottom + 6, right: Math.max(8, window.innerWidth - r.right) };
    }
    app.bellOpen = true;
    now = Date.now();
    await tick();
    panel?.focus({ preventScroll: true });
  }
  function close(refocus = true) {
    if (!app.bellOpen) return;
    app.bellOpen = false;
    if (refocus) btn?.focus({ preventScroll: true });
  }
  /** Esc closes the panel wherever the keyboard is (on the window: a button that just went grey,
   *  like Clear, drops the focus out of the panel). */
  function key(e: KeyboardEvent) {
    if (!app.bellOpen || e.key !== "Escape") return;
    e.preventDefault();
    e.stopPropagation();
    close();
  }
  /** Mark all as read and Clear turn grey once pressed: the panel takes the keyboard back, so
   *  Tab goes on inside it. */
  function then(run: () => unknown) {
    run();
    panel?.focus({ preventScroll: true });
  }
  /** Tabbing out of the panel closes it, like the other menus. */
  function focusOut(e: FocusEvent) {
    const to = e.relatedTarget as Node | null;
    if (to && !panel?.contains(to) && to !== btn) close(false);
  }

  // Relative times ("5 min ago") follow the clock while the panel is open.
  let now = $state(Date.now());
  $effect(() => {
    if (!app.bellOpen) return;
    const t = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(t);
  });

  const groups = $derived.by(() => {
    const today = app.notices.filter((n) => isToday(n.at, now));
    const earlier = app.notices.filter((n) => !isToday(n.at, now));
    return [
      { name: "Today", list: today },
      { name: "Earlier", list: earlier },
    ].filter((g) => g.list.length);
  });

  /** What a line offers: how a question ended once settled ("Accepted", "Done"...), else
   *  Direct's questions while they still wait. */
  function answerOf(n: Notice): { kind: "offer" | "request" } | { kind: "done"; text: string } | null {
    if (n.answered) return { kind: "done", text: n.answered };
    if (n.kind !== "direct-offer" && n.kind !== "direct-request") return null;
    const d = app.direct;
    if (!d?.ready) return null;
    if (n.kind === "direct-offer") return (d.offers ?? []).some((o) => o.id === n.go.offer) ? { kind: "offer" } : { kind: "done", text: "No longer waiting" };
    return (d.requests ?? []).some((r) => r.peer === n.go.peer) ? { kind: "request" } : { kind: "done", text: "No longer waiting" };
  }

  function open(n: Notice) {
    close(false);
    app.readNotices([n.id]);
    // A ready update restarts only from its button; the line itself just opens Pako.
    if (n.kind !== "update-ready") app.goTo(n.go);
  }
  /** "Restart now" on a ready update: still the newest one? (a later restart already used it) */
  const updateWaiting = (n: Notice) => !!app.update?.ready && /\d+(?:\.\d+)+/.exec(n.title)?.[0] === app.update.latest;
  function restart(n: Notice) {
    close(false);
    app.readNotices([n.id]);
    app.restartForUpdate();
  }
  function lineKey(e: KeyboardEvent, n: Notice) {
    if (e.target !== e.currentTarget) return;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      open(n);
    }
  }
  /** Lines whose answer is being sent: their buttons wait meanwhile. */
  let sending = $state<Record<string, boolean>>({});
  /** Accept / Decline files from the bell (into the contact's usual folder). Pako notes the
   *  answer on the line once it went through; if it didn't, the line shows how things stand. */
  async function answer(n: Notice, accept: boolean) {
    const offer = n.go.offer;
    if (!offer || sending[n.id]) return;
    sending = { ...sending, [n.id]: true };
    try {
      await api.directAnswerOffer(offer, accept, null);
    } catch (e) {
      app.toast(String(e), "error");
      await app.refreshNotices();
    } finally {
      const { [n.id]: _, ...rest } = sending;
      sending = rest;
    }
  }
  /** A connection request is answered in Pako (its safety code is compared there). */
  function review(n: Notice) {
    close(false);
    app.readNotices([n.id]);
    app.goTo({ page: "direct" });
  }

  const tones: Record<string, string> = {
    done: "var(--st-done-text)",
    fail: "var(--st-fail-text)",
    info: "var(--st-run-text)",
    wait: "var(--st-retry-text)",
    people: "var(--st-wait-text)",
  };
</script>

<svelte:window onkeydowncapture={key} />

<button
  bind:this={btn}
  class="bell"
  class:on={app.bellOpen}
  aria-haspopup="dialog"
  aria-expanded={app.bellOpen}
  aria-label={app.noticesUnread ? `Notifications, ${app.noticesUnread} unread` : "Notifications"}
  title="Notifications"
  onclick={toggle}
>
  <span class="glyph"><Icon name="bell" size={15} stroke={1.8} /></span>
  {#if app.noticesUnread}
    <span class="count mono" transition:pop={{ duration: 200, from: 0.92, y: 0 }}>
      {#key app.noticesUnread}<span class="num" in:fadeIn={{ duration: 160 }}>{app.noticesUnread > 99 ? "99+" : app.noticesUnread}</span>{/key}
    </span>
  {/if}
</button>

{#if app.bellOpen}
  <button class="scrim" aria-label="Close notifications" tabindex="-1" onclick={() => close(false)}></button>
  <div
    bind:this={panel}
    class="panel"
    role="dialog"
    aria-label="Notifications"
    tabindex="-1"
    style="top: {at.top}px; right: {at.right}px"
    transition:pop={{ duration: 200, from: 0.97, y: -6, origin: "top right" }}
    onfocusout={focusOut}
  >
    <header>
      <h2>Notifications</h2>
      <span class="grow"></span>
      <button class="link" disabled={!app.noticesUnread} onclick={() => then(() => app.readNotices(null))}>Mark all as read</button>
      <button class="link" disabled={!app.notices.length} onclick={() => then(() => app.clearNotices())}>Clear</button>
      {#if onsettings}
        <button
          class="icon-btn gear"
          title="Choose what shows here"
          aria-label="Notification settings"
          onclick={() => {
            close(false);
            onsettings?.();
          }}><Icon name="settings" size={13} /></button
        >
      {/if}
    </header>

    <div class="list">
      {#if !app.notices.length}
        <div class="empty" in:fadeIn={{ duration: 180 }}>
          <span class="ring"><Icon name="bell" size={18} stroke={1.8} /></span>
          <p class="et">No notifications</p>
          <p class="es">People asking to connect, files waiting for your answer and links that need you show up here.</p>
        </div>
      {:else}
        {#each groups as g (g.name)}
          <div class="group" transition:rowSlide={{ duration: 200 }}>
            <div class="gname">{g.name}</div>
            {#each g.list as n (n.id)}
              {@const k = kindInfo(n.kind)}
              {@const ans = answerOf(n)}
              <div class="row" transition:rowSlide={{ duration: 220 }} animate:flipIf={flipOpts}>
                <div class="line" class:unread={!n.read} role="button" tabindex="0" onclick={() => open(n)} onkeydown={(e) => lineKey(e, n)}>
                  {#if !n.read}<span class="dot" transition:pop={{ duration: 180, from: 0.92, y: 0 }} aria-label="Unread"></span>{/if}
                  <span class="chip" style:color={tones[k.tone]}><Icon name={k.icon} size={14} stroke={2} /></span>
                  <div class="txt">
                    <div class="top">
                      <span class="title">{n.title}</span>
                      <span class="time">{noticeTime(n.at, now)}</span>
                    </div>
                    <div class="body">{n.body}</div>
                    {#if ans?.kind === "offer"}
                      <div class="acts" in:fadeIn={{ duration: 160 }}>
                        <button class="btn small" disabled={sending[n.id]} onclick={(e) => (e.stopPropagation(), answer(n, false))}>Decline</button>
                        <button class="btn primary small" disabled={sending[n.id]} onclick={(e) => (e.stopPropagation(), answer(n, true))}><Icon name="download" size={12} stroke={2.2} />Accept</button>
                      </div>
                    {:else if ans?.kind === "request"}
                      <div class="acts" in:fadeIn={{ duration: 160 }}>
                        <button class="btn small" onclick={(e) => (e.stopPropagation(), review(n))}>Review</button>
                      </div>
                    {:else if n.kind === "update-ready" && updateWaiting(n)}
                      <div class="acts" in:fadeIn={{ duration: 160 }}>
                        <button class="btn primary small" onclick={(e) => (e.stopPropagation(), restart(n))}>Restart now</button>
                      </div>
                    {:else if ans?.kind === "done"}
                      <div class="answered" in:fadeIn={{ duration: 200 }}><Icon name="check" size={11} stroke={2.4} />{ans.text}</div>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
  </div>
{/if}

<style>
  /* The same size and look as the details-panel button next to it. */
  .bell {
    position: relative;
    width: 32px;
    height: 28px;
    margin-right: 2px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-3);
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .bell:hover,
  .bell.on {
    background: var(--raise);
    color: var(--text);
  }
  .bell:active {
    transform: scale(0.97);
  }
  .glyph {
    display: grid;
    place-items: center;
  }
  /* Red count, ringed in the bar's colour so it stands off the bell. */
  .count {
    position: absolute;
    top: 1px;
    right: 2px;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-ink);
    box-shadow: 0 0 0 2px var(--bar);
    font-size: 9.5px;
    font-weight: 700;
    line-height: 1;
    pointer-events: none;
  }
  .num {
    display: block;
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    border: none;
    background: transparent;
    cursor: default;
  }
  .panel {
    position: fixed;
    z-index: 61;
    width: 380px;
    max-height: min(560px, calc(100vh - 64px));
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line-strong);
    background: var(--pop-bg);
    backdrop-filter: blur(12px);
    box-shadow: var(--shadow-menu);
    outline: none;
    overflow: hidden;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 10px 8px 10px 16px;
    border-bottom: 1px solid var(--line-soft);
  }
  h2 {
    margin: 0;
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text);
  }
  .grow {
    flex: 1;
  }
  .link {
    height: 26px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--focus-text);
    font-size: 12px;
    font-weight: 500;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      opacity var(--fast) ease;
  }
  .link:hover:not(:disabled) {
    background: var(--raise-hover);
  }
  .link:disabled {
    color: var(--text-4);
    opacity: 0.6;
    cursor: default;
  }
  .gear {
    width: 26px;
    height: 26px;
    margin-left: 2px;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 0 6px;
  }
  /* A line between the groups, so each reads apart (like Settings › Notifications). */
  .group + .group {
    margin-top: 4px;
    border-top: 1px solid var(--line-soft);
  }
  .gname {
    padding: 10px 16px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .row {
    padding: 0 6px;
  }
  .line {
    position: relative;
    display: flex;
    gap: 10px;
    padding: 9px 10px 10px 12px;
    border-radius: 8px;
    outline-offset: -2px;
    transition: background var(--fast) ease;
  }
  .line:hover {
    background: var(--hover);
  }
  .dot {
    position: absolute;
    left: 3px;
    top: 19px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .chip {
    width: 28px;
    height: 28px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: color-mix(in srgb, currentColor 13%, transparent);
  }
  .txt {
    flex: 1;
    min-width: 0;
  }
  .top {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-2);
  }
  .unread .title {
    font-weight: 600;
    color: var(--text);
  }
  .time {
    flex: none;
    font-size: 11px;
    color: var(--text-4);
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
  }
  .acts {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .btn.small {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
  .answered {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: 6px;
    font-size: 11.5px;
    color: var(--text-4);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 34px 32px 38px;
    text-align: center;
  }
  .ring {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--raise);
    color: var(--text-3);
    margin-bottom: 12px;
  }
  .et {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
  }
  .es {
    margin: 4px 0 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-3);
  }
</style>
