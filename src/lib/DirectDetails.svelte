<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts" module>
  import type { OftenFor } from "./api";
  /** "Often for…" picked in this window, per contact, with the end it should have (else the
   *  span is read from when it ends). */
  const picked = new Map<string, { span: OftenFor; until: number }>();
  /** When "Often for…" picked now would end, as Pako works it out (`pace::often_until`). */
  function endOf(span: OftenFor, now: number): number {
    if (span === "hour") return now + 3_600_000;
    if (span === "week") return now + 7 * 86_400_000;
    const d = new Date(now);
    d.setHours(24, 0, 0, 0);
    return d.getTime();
  }
</script>

<script lang="ts">
  // Direct › details panel (Ctrl+I, like Downloads): the picked contact's connection, what went
  // between you (all time, from the Dashboard's numbers), speeds, and their settings.
  import { app } from "./state.svelte";
  import { api, type ContactNumbers, type DirectCheck, type DirectContact, type DirectItem, type DirectTransfer } from "./api";
  import { bytes, shortDate, speed } from "./format";
  import Icon from "./Icon.svelte";
  import SegPick from "./SegPick.svelte";
  import TransferItems from "./TransferItems.svelte";
  import PanelResizer from "./PanelResizer.svelte";
  import { fadeIn, rowSlide } from "./motion";
  import { isLocal, itemsCount, pathLabel } from "./direct";
  import { quintOut } from "svelte/easing";
  import type { TransitionConfig } from "svelte/transition";

  let {
    drawer = false,
    contact: c,
    moving,
    speedOf,
    historyCount,
    transfer = null,
    onopen,
    onletgo,
    ontags,
    onfolder,
    resize = null,
  }: {
    /** Over the conversation (a narrow window) rather than a column of its own. */
    drawer?: boolean;
    contact: DirectContact;
    /** Their transfers moving right now. */
    moving: DirectTransfer[];
    speedOf: (t: DirectTransfer) => number;
    /** Changes when a transfer with them ends (the numbers are read again). */
    historyCount: number;
    /** The card picked in the conversation: what was in it (`openId`: received, opens). */
    transfer?: { name: string; outgoing: boolean; files: number; size: number; items: DirectItem[]; more: number; openId: string | null } | null;
    onopen?: (id: string, part: number, reveal: boolean) => void;
    /** Back to the contact's details. */
    onletgo?: () => void;
    ontags: () => void;
    onfolder: () => void;
    /** Over the conversation: its own edge to resize it (the column has its edge outside). */
    resize?: { width: number; avail: number; listMin: number } | null;
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
    local: "Over your home network, PC to PC: nothing goes through the internet. Usually the fastest.",
    direct6: "Straight between the two PCs, over IPv6: usually the fastest.",
    direct: "Straight between the two PCs.",
    relay: "Through a relay server: still encrypted end to end, often slower.",
  };
  /** How what went between you travelled: over the home network, straight between the PCs
   *  over the internet, through a relay. Rounded so the three add up to 100. */
  const share = $derived.by(() => {
    if (!n) return null;
    const local = Math.min(n.localBytes ?? 0, n.directBytes);
    const parts = [
      { key: "local", label: "Local network", bytes: local, tip: "Over your home network" },
      { key: "direct", label: "Direct", bytes: n.directBytes - local, tip: "Straight between your PCs" },
      { key: "relay", label: "Via relay", bytes: n.relayBytes, tip: "Through a relay" },
    ];
    const total = parts.reduce((a, p) => a + p.bytes, 0);
    if (!total) return null;
    const pct = parts.map((p) => Math.round((p.bytes / total) * 100));
    // The biggest part takes what rounding left over.
    const big = parts.reduce((b, p, i) => (p.bytes > parts[b].bytes ? i : b), 0);
    pct[big] += 100 - pct.reduce((a, x) => a + x, 0);
    // A part that never happened isn't listed (Direct and Via relay always are).
    return parts.map((p, i) => ({ ...p, pct: pct[i] })).filter((p) => p.bytes > 0 || p.key !== "local");
  });
  // "Check this contact": how Pako calls them in the background. Often lasts a while, then
  // it's back to the choice it had before by itself (When I ask, else Automatic).
  const CHECKS: { value: DirectCheck; label: string }[] = [
    { value: "auto", label: "Automatic" },
    { value: "often", label: "Often for…" },
    { value: "ask", label: "When I ask" },
  ];
  const SPANS: { value: OftenFor; label: string }[] = [
    { value: "hour", label: "1 hour" },
    { value: "today", label: "Today" },
    { value: "week", label: "1 week" },
  ];
  const check = $derived<DirectCheck>(c.check === "often" && !c.checkUntil ? "auto" : (c.check ?? "auto"));
  const isMidnight = (d: Date) => d.getHours() + d.getMinutes() + d.getSeconds() + d.getMilliseconds() === 0;
  /** Which "Often for…" runs: as picked here (if that's still the one running: not one picked
   *  since from the right-click menu), else from its end (midnight = Today, more than an hour
   *  away = 1 week). */
  const span = $derived.by<OftenFor>(() => {
    const until = c.checkUntil ?? 0;
    const known = picked.get(c.id);
    if (known && Math.abs(known.until - until) < 120_000) return known.span;
    if (isMidnight(new Date(until))) return "today";
    return until - Date.now() > 3_660_000 ? "week" : "hour";
  });
  /** "18:40", "midnight", or the day for a week. */
  function untilText(until: number): string {
    const d = new Date(until);
    const left = until - Date.now();
    if (isMidnight(d) && left <= 86_400_000) return "midnight";
    if (left < 86_400_000) return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    return d.toLocaleDateString([], { day: "numeric", month: "long" });
  }
  /** One line under the picker: what the choice does for this contact. */
  const checkNote = $derived(
    check === "often"
      ? `Checked often until ${untilText(c.checkUntil ?? 0)}${c.checkBefore === "ask" ? ", then back to When I ask" : ""}.`
      : check === "ask"
        ? "Only when you open, send or check."
        : c.checkAs === "often"
          ? "Always connected: they say this PC is theirs."
          : c.favourite
            ? "Less when away. Favourites never stop."
            : c.autoAccept
              ? "Less when away. Never stops: they send without asking."
              : "Pauses after 30 days with nothing new.",
  );
  function pickCheck(v: DirectCheck) {
    // "Often for…" starts with an hour; the row under it changes how long.
    if (v === "often") {
      if (check !== "often") setOften("hour");
      return;
    }
    picked.delete(c.id);
    app.run(() => api.directSetCheck(c.id, v));
  }
  function setOften(s: OftenFor) {
    picked.set(c.id, { span: s, until: endOf(s, Date.now()) });
    app.run(() => api.directSetCheck(c.id, "often", s));
  }
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
  {#if drawer && resize}
    <PanelResizer page="direct" width={resize.width} avail={resize.avail} listMin={resize.listMin} />
  {/if}
  {#if transfer}
    {@const x = transfer}
    <section class="picked" aria-label="What was in {x.name}" in:fadeIn={{ duration: 160 }}>
      <div class="phead">
        <div class="label">{x.outgoing ? `Sent to ${c.name}` : `Received from ${c.name}`}</div>
        <button class="icon-btn sm" aria-label="Back to {c.name}'s details (Esc)" title="Back to {c.name}'s details (Esc)" onclick={() => onletgo?.()}><Icon name="x" size={12} /></button>
      </div>
      <div class="pname" title={x.name}>{x.name}</div>
      <div class="psub">{itemsCount(x.files, x.items, x.more)} · {bytes(x.size)}</div>
      <div class="plist">
        <TransferItems compact items={x.items} more={x.more} open={x.openId && onopen ? (i, reveal) => onopen(x.openId!, i, reveal) : null} />
      </div>
    </section>
  {:else}
    <div class="about" in:fadeIn={{ duration: 160 }}>
      <section>
        <div class="label">Connection</div>
        <dl>
          <dt>Status</dt>
          <dd>
            <span class="sdot" class:online={c.online} class:local={isLocal(c)} class:wait={c.waiting}></span>{c.online ? "Online" : c.waiting ? "Waiting for them" : c.unchecked ? "Not checked lately" : c.idle ? "Seen lately" : "Offline"}
          </dd>
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
        {#if c.unchecked}
          <div class="xrow" transition:rowSlide={{ duration: 200 }}>
            <span class="xt dim">Not called by itself.</span>
            <button class="btn small" onclick={() => app.run(() => api.directConnect(c.id))}><Icon name="retry" size={12} />Check now</button>
          </div>
        {/if}
        <div class="checkpick">
          <span class="k">Check this contact</span>
          <div class="picks">
            <SegPick small options={CHECKS} value={check} label="Check this contact" onpick={pickCheck} />
            {#if check === "often"}
              <div transition:rowSlide={{ duration: 200 }}>
                <div class="spans"><SegPick small options={SPANS} value={span} label="Check often for" onpick={setOften} /></div>
              </div>
            {/if}
          </div>
          <span class="note">{checkNote}</span>
        </div>
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
        {#if share}
          <div class="share" role="img" aria-label={share.map((p) => `${p.label} ${p.pct}%`).join(", ")}>
            {#each share as p (p.key)}
              {#if p.bytes > 0}<i class={p.key} style="flex: {p.bytes}" title="{p.tip}: {bytes(p.bytes)}"></i>{/if}
            {/each}
          </div>
          <div class="sharetext" class:three={share.length > 2}>{#each share as p (p.key)}<span><i class={p.key}></i>{p.label} {p.pct}%</span>{/each}</div>
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
        <div class="xrow">
          <span class="xt">Mute<span class="xsub">No pop-ups or sounds from them</span></span>
          <button class="switch" class:on={c.muted} role="switch" aria-checked={!!c.muted} aria-label="Mute" onclick={() => app.run(() => api.directSetMuted(c.id, !c.muted))}></button>
        </div>
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
    </div>
  {/if}
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
    width: var(--pw, 288px);
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
  /* The card picked in the conversation: its files and folders, in place of the contact's
     details until you let go (Esc, the ×, a click elsewhere). */
  .picked {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-bottom: none;
    --ti-hover: var(--hover);
  }
  .about {
    display: flex;
    flex-direction: column;
  }
  .phead {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: -4px -6px 0 0;
  }
  .phead .label {
    flex: 1;
    min-width: 0;
    margin: 0;
  }
  .pname {
    margin-top: 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .psub {
    margin-top: 2px;
    font-size: 12px;
    color: var(--text-3);
  }
  /* A long list scrolls inside the panel. */
  .plist {
    flex: 1;
    min-height: 0;
    margin: 10px -8px 0;
    overflow-y: auto;
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
    transition: background var(--mid) ease;
  }
  .sdot.online {
    background: var(--st-done);
  }
  /* Blue like the dot on their row: on the same network. */
  .sdot.local {
    background: var(--st-run);
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
  .share .local,
  .sharetext .local {
    background: var(--st-run);
  }
  .direct {
    background: var(--st-done);
  }
  .relay {
    background: var(--st-pause);
  }
  .sharetext {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  /* Three parts (the home network too) wrap from the left, like a legend. */
  .sharetext.three {
    justify-content: flex-start;
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
  .xsub {
    display: block;
    margin-top: 2px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .checkpick {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
    margin-top: 14px;
  }
  .checkpick :global(.seg) {
    display: flex;
  }
  /* Equal thirds, so the "Often for…" row's buttons line up under the ones above. */
  .checkpick :global(.seg button) {
    flex: 1 1 0;
    padding: 0 6px;
    justify-content: center;
  }
  .picks {
    display: flex;
    flex-direction: column;
  }
  /* (Flex, so its picker stretches like the one above.) */
  .spans {
    display: flex;
    flex-direction: column;
    padding-top: 6px;
  }
  .note {
    font-size: 11.5px;
    color: var(--text-3);
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
