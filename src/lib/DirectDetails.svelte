<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Direct › details panel (Ctrl+I, like Downloads): the picked contact's connection, what went
  // between you (all time, from the Dashboard's numbers), speeds, and their settings.
  import { app } from "./state.svelte";
  import { api, type ContactNumbers, type DirectContact, type DirectTransfer } from "./api";
  import { bytes, shortDate, speed } from "./format";
  import Icon from "./Icon.svelte";
  import { pathLabel } from "./direct";
  import { quintOut } from "svelte/easing";
  import type { TransitionConfig } from "svelte/transition";

  let {
    drawer = false,
    contact: c,
    moving,
    speedOf,
    historyCount,
    ontags,
    onfolder,
  }: {
    /** Over the conversation (a narrow window) rather than a column of its own. */
    drawer?: boolean;
    contact: DirectContact;
    /** Their transfers moving right now. */
    moving: DirectTransfer[];
    speedOf: (t: DirectTransfer) => number;
    /** Changes when a transfer with them ends (the numbers are read again). */
    historyCount: number;
    ontags: () => void;
    onfolder: () => void;
  } = $props();

  let n = $state<ContactNumbers | null>(null);
  /** Whose numbers `n` are: another contact picked shows none until theirs arrive (never the
   *  previous person's). */
  let nFor = "";
  $effect(() => {
    const id = c.id;
    void historyCount;
    if (id !== nFor) {
      nFor = id;
      n = null;
    }
    api
      .numbersContact(id)
      .then((got) => {
        if (c.id === id) n = got;
      })
      .catch(() => {});
  });

  const up = $derived(moving.filter((t) => t.outgoing).reduce((s, t) => s + speedOf(t), 0));
  const down = $derived(moving.filter((t) => !t.outgoing).reduce((s, t) => s + speedOf(t), 0));
  const plural = (k: number, one: string, many = `${one}s`) => `${k.toLocaleString()} ${k === 1 ? one : many}`;
  const routeTip: Record<string, string> = {
    direct6: "Straight between the two PCs, over IPv6: usually the fastest.",
    direct: "Straight between the two PCs.",
    relay: "Through a relay server: still encrypted end to end, often slower.",
  };
  const directPct = $derived(n && n.directBytes + n.relayBytes ? Math.round((n.directBytes / (n.directBytes + n.relayBytes)) * 100) : null);
  const added = $derived(new Date(c.added).toLocaleDateString([], { day: "numeric", month: "long", year: "numeric" }));
  function copyId() {
    app.run(() => api.copyText(c.id)).then(() => app.toast(`${c.name}'s Pako ID is copied.`));
  }
  /** The drawer (a narrow window) slides in from the right and back out; the column version is
   *  animated by its container instead. */
  function drawerSlide(_node: Element): TransitionConfig {
    if (!drawer || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return { duration: 0 };
    return {
      duration: 260,
      easing: quintOut,
      css: (t) => `opacity:${t};transform:translateX(${(1 - t) * 24}px)`,
    };
  }
</script>

<aside class="details" data-tour="direct-details" class:drawer aria-label="About {c.name}" transition:drawerSlide|global>
  <section>
    <div class="label">Connection</div>
    <dl>
      <dt>Status</dt>
      <dd><span class="sdot" class:online={c.online} class:wait={c.waiting}></span>{c.online ? "Online" : c.waiting ? "Waiting for them" : "Offline"}</dd>
      {#if c.online}
        <dt>Route</dt>
        <dd title={routeTip[c.path ?? ""] ?? ""}>{pathLabel(c.path) || "Connected"}</dd>
      {:else}
        <dt>Last seen</dt>
        <dd>{c.lastSeen ? shortDate(c.lastSeen) : "Not yet"}</dd>
      {/if}
      <dt>Speed now</dt>
      <dd class="mono">
        {#if moving.length}
          {#if moving.some((t) => t.outgoing)}<span title="Sending to {c.name}: {speed(up)}">↑ {speed(up)}</span>{/if}
          {#if moving.some((t) => t.outgoing) && moving.some((t) => !t.outgoing)}<span class="sep">·</span>{/if}
          {#if moving.some((t) => !t.outgoing)}<span title="Receiving from {c.name}: {speed(down)}">↓ {speed(down)}</span>{/if}
        {:else}
          <span class="dim">Nothing moving</span>
        {/if}
      </dd>
    </dl>
  </section>

  <section>
    <div class="label">Between you</div>
    <div class="totals">
      <div title="Sent to {c.name}: {bytes(n?.sent ?? 0)} in {plural(n?.sentCount ?? 0, 'transfer')}">
        <span class="k"><b class="up">↑</b> Sent</span>
        <span class="v mono">{bytes(n?.sent ?? 0)}</span>
      </div>
      <div title="Received from {c.name}: {bytes(n?.received ?? 0)} in {plural(n?.receivedCount ?? 0, 'transfer')}">
        <span class="k"><b class="down">↓</b> Received</span>
        <span class="v mono">{bytes(n?.received ?? 0)}</span>
      </div>
    </div>
    <dl>
      <dt>Transfers</dt>
      <dd>{(n?.sentCount ?? 0) + (n?.receivedCount ?? 0)}</dd>
      <dt>Last exchange</dt>
      <dd>{n?.last ? shortDate(n.last) : "None yet"}</dd>
    </dl>
  </section>

  <section>
    <div class="label">Speed with {c.name}</div>
    {#if n && (n.peakSent || n.peakReceived)}
      <dl>
        <dt>Fastest</dt>
        <dd class="mono">
          <span title="Fastest send to {c.name}">↑ {n.peakSent ? speed(n.peakSent) : "—"}</span><span class="sep">·</span><span title="Fastest from {c.name}">↓ {n.peakReceived ? speed(n.peakReceived) : "—"}</span>
        </dd>
        <dt>Average</dt>
        <dd class="mono">
          <span title="Average while sending to {c.name}">↑ {n.avgSent ? speed(n.avgSent) : "—"}</span><span class="sep">·</span><span title="Average while receiving from {c.name}">↓ {n.avgReceived ? speed(n.avgReceived) : "—"}</span>
        </dd>
      </dl>
    {:else}
      <p class="none">Speeds show up after your first transfer together.</p>
    {/if}
    {#if directPct !== null}
      <div class="share" role="img" aria-label="{directPct}% went straight between your PCs, {100 - directPct}% through a relay">
        <i class="direct" style="flex: {n!.directBytes}" title="Straight between your PCs: {bytes(n!.directBytes)}"></i>
        <i class="relay" style="flex: {n!.relayBytes}" title="Through a relay: {bytes(n!.relayBytes)}"></i>
      </div>
      <div class="sharetext"><span><i class="direct"></i>Direct {directPct}%</span><span><i class="relay"></i>Via relay {100 - directPct}%</span></div>
    {/if}
  </section>

  <section>
    <div class="label">About them</div>
    <dl>
      <dt>Pako ID</dt>
      <dd class="idline">
        <span class="mono" title={c.id}>{c.id.slice(0, 6)}…{c.id.slice(-6)}</span>
        <button class="icon-btn sm" aria-label="Copy {c.name}'s Pako ID" title="Copy" onclick={copyId}><Icon name="copy" size={12} /></button>
      </dd>
      <dt>Their name</dt>
      <dd title="The name they chose for themselves">{c.theirName ?? "Not shared"}</dd>
      <dt>Tags</dt>
      <dd>
        <button class="linkish" onclick={ontags}>
          {#if c.tags?.length}
            {#each c.tags as t, i (i)}<span class="chip">{t}</span>{/each}
          {:else}
            Add tags
          {/if}
        </button>
      </dd>
      <dt>Favourite</dt>
      <dd>
        <button class="icon-btn sm star" class:on={c.favourite} role="switch" aria-checked={c.favourite} aria-label="Favourite" title={c.favourite ? "Remove from favourites" : "Add to favourites"} onclick={() => app.run(() => api.directSetFavourite(c.id, !c.favourite))}>
          <Icon name="star" size={14} />
        </button>
      </dd>
      <dt>Added</dt>
      <dd>{added}</dd>
    </dl>
    <div class="xrow" data-tour="direct-trust">
      <span class="xt">Can send to me without asking</span>
      <button class="switch" class:on={c.autoAccept} role="switch" aria-checked={c.autoAccept} aria-label="Can send to me without asking" onclick={() => app.run(() => api.directEditContact(c.id, { autoAccept: !c.autoAccept }))}></button>
    </div>
    <div class="folder">
      <span class="k">Their files go to</span>
      <div class="fline">
        <span class="path" title={c.folder}>{c.folder}</span>
        <button class="btn small" onclick={onfolder}>Change…</button>
      </div>
    </div>
  </section>
</aside>

<style>
  .details {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
  }
  /* Over the conversation, between its header and the send area (both stay in reach). */
  .details.drawer {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 288px;
    z-index: 5;
    border: none;
    border-left: 1px solid var(--line-strong);
    border-radius: 0;
    box-shadow: -12px 0 28px rgba(0, 0, 0, 0.4);
  }
  :global(:root[data-theme="light"]) .details.drawer {
    box-shadow:
      -12px 0 28px rgba(20, 26, 46, 0.1),
      -1px 0 4px rgba(20, 26, 46, 0.05);
  }
  section {
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
  }
  section:last-child {
    border-bottom: none;
  }
  .label {
    margin-bottom: 10px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  dl {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 8px 14px;
    margin: 0;
    font-size: 12px;
  }
  dt {
    color: var(--text-3);
    white-space: nowrap;
  }
  dd {
    margin: 0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    min-width: 0;
    text-align: right;
    color: var(--text-2);
  }
  .dim {
    color: var(--text-3);
  }
  .sep {
    color: var(--text-4);
  }
  .sdot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--st-pause);
  }
  .sdot.online {
    background: var(--st-done);
  }
  /* Amber like the dot on their row: waiting for them (coral would mean waiting for you). */
  .sdot.wait {
    background: var(--st-retry);
  }
  .totals {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 12px;
  }
  .totals > div {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 9px 10px;
    border-radius: var(--radius);
    background: var(--panel);
    border: 1px solid var(--line-soft);
  }
  .k {
    font-size: 11px;
    color: var(--text-3);
  }
  .k b {
    font-weight: 700;
  }
  .up {
    color: #a77be6;
  }
  .down {
    color: #2ba58c;
  }
  /* Light: the same two colours as the Dashboard's, dark enough to read. */
  :global(:root[data-theme="light"]) .up {
    color: #7a4cc2;
  }
  :global(:root[data-theme="light"]) .down {
    color: #137a66;
  }
  .v {
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
  }
  .none {
    margin: 0;
    font-size: 12px;
    color: var(--text-3);
  }
  .share {
    display: flex;
    gap: 2px;
    height: 6px;
    margin-top: 12px;
    border-radius: 3px;
    overflow: hidden;
  }
  .share i,
  .sharetext i {
    min-width: 2px;
  }
  .direct {
    background: var(--st-done);
  }
  .relay {
    background: var(--st-pause);
  }
  .sharetext {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .sharetext span {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sharetext i {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .idline .mono {
    color: var(--text-2);
  }
  .icon-btn.sm {
    width: 24px;
    height: 24px;
    margin: -4px -4px -4px 0;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-3);
    cursor: pointer;
  }
  .icon-btn.sm:hover {
    background: var(--raise-hover);
    color: var(--text);
  }
  .star.on {
    color: var(--st-retry);
  }
  .linkish {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 4px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text-2);
    font: inherit;
    cursor: pointer;
  }
  .linkish:hover {
    color: var(--text);
  }
  .chip {
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--active);
    font-size: 11px;
    color: var(--text-2);
  }
  .xrow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 14px;
  }
  .xt {
    font-size: 12px;
    color: var(--text-2);
  }
  .folder {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 14px;
  }
  .fline {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn.small {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
</style>
