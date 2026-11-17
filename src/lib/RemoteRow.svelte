<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // One download of another device's list (My devices › Downloads, and inside a card of
  // My devices › Links): its state, and what this PC may do with it there (pause, resume,
  // priority, remove, get the finished file).
  import { app } from "./state.svelte";
  import { devicesApi, type Device, type RemoteItem } from "./devices.svelte";
  import type { Priority } from "./api";
  import { bytes, speed, statusLook } from "./format";
  import Icon from "./Icon.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import type { MenuItem } from "./menu";
  import { rowSlide } from "./motion";

  let { device: x, item: it, ongot, compact = false }: { device: Device; item: RemoteItem; ongot?: () => void; compact?: boolean } = $props();

  const ACTIVE = new Set(["queued", "starting", "downloading", "retrying", "verifying"]);
  const RESUMABLE = new Set(["paused", "failed", "quotaExceeded", "needsAccount"]);
  let acting = $state(false);
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  const look = $derived(statusLook(it.status, it.message));
  const line = $derived.by(() => {
    if (it.status === "completed") return `${look.label} · ${bytes(it.size)}`;
    if (it.status === "failed") return it.message ?? look.label;
    const of = it.size ? `${bytes(it.done)} of ${bytes(it.size)}` : bytes(it.done);
    return `${look.label} · ${of}${it.speed > 0 && it.status === "downloading" ? " · " + speed(it.speed) : ""}`;
  });
  const prio = $derived<Priority>(it.priority ?? "normal");

  async function act(action: "pause" | "resume" | "remove" | "get" | Priority) {
    acting = true;
    try {
      const text = await devicesApi.item(x.id, it.id, action);
      if (action === "get") {
        app.toast(`${x.name} is sending ${it.name}. It lands in ${x.folder}.`);
        ongot?.();
      } else if (action === "remove") app.toast(text);
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      acting = false;
    }
  }

  function priorityMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const one = (p: Priority, label: string, icon: string): MenuItem => ({ label, icon, hint: prio === p ? "✓" : "", action: () => prio !== p && act(p) });
    menu = { x: r.right - 200, y: r.bottom + 4, items: [one("first", "Priority: First", "up"), one("normal", "Priority: Normal", "minus"), one("later", "Priority: Later", "down")] };
  }
</script>

<div class="ritem" class:compact transition:rowSlide={{ duration: 200 }}>
  <FileIcon name={it.name} size={compact ? 16 : 18} />
  <div class="rtext">
    <MiddleText text={it.name} class="rname" />
    <span class="rline">
      <span class="sdot" style:background={look.color}></span>{line}
      {#if it.priority && it.status !== "completed"}<span class="pchip" class:first={it.priority === "first"}>{it.priority === "first" ? "First" : "Later"}</span>{/if}
    </span>
    {#if ACTIVE.has(it.status) || it.status === "paused"}
      <span class="rbar"><i style:transform="scaleX({it.size ? Math.min(1, it.done / it.size) : 0})"></i></span>
    {/if}
  </div>
  <div class="racts">
    {#if it.status === "completed"}
      <button class="btn small" disabled={acting} onclick={() => act("get")} title="Send it to this PC, into {x.folder}"><Icon name="down" size={12} />Get this file</button>
    {:else}
      {#if ACTIVE.has(it.status)}
        <button class="icon-btn" disabled={acting} aria-label="Pause {it.name}" title="Pause" onclick={() => act("pause")}><Icon name="pause" size={13} /></button>
      {:else if RESUMABLE.has(it.status)}
        <button class="icon-btn" disabled={acting} aria-label="Resume {it.name}" title="Resume" onclick={() => act("resume")}><Icon name="play" size={13} /></button>
      {/if}
      <button class="icon-btn" disabled={acting} aria-label="Priority of {it.name}" aria-haspopup="menu" title="Priority: {prio === 'first' ? 'First' : prio === 'later' ? 'Later' : 'Normal'}" onclick={priorityMenu}>
        <Icon name={prio === "first" ? "up" : prio === "later" ? "down" : "minus"} size={13} />
      </button>
    {/if}
    <button class="icon-btn" disabled={acting} aria-label="Remove {it.name} from the list" title="Remove from the list (a finished file stays on {x.name})" onclick={() => act("remove")}><Icon name="x" size={13} /></button>
  </div>
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}

<style>
  .ritem {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-radius: var(--radius);
    transition: background var(--fast) ease;
  }
  .ritem:hover {
    background: var(--raise-hover);
  }
  .ritem.compact {
    padding: 6px 4px;
    gap: 8px;
  }
  .rtext {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .rtext :global(.rname) {
    font-weight: 500;
    min-width: 0;
  }
  .compact .rtext :global(.rname) {
    font-size: 12.5px;
  }
  .rline {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sdot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
  .pchip {
    flex: none;
    height: 16px;
    display: inline-flex;
    align-items: center;
    padding: 0 6px;
    border-radius: var(--radius-sm);
    background: var(--active);
    color: var(--text-2);
    font-size: 10.5px;
    font-weight: 600;
  }
  .pchip.first {
    background: color-mix(in srgb, var(--st-run) 15%, transparent);
    color: var(--st-run-text);
  }
  .rbar {
    height: 3px;
    margin: 4px 0 0;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }
  .rbar i {
    display: block;
    height: 100%;
    background: var(--st-run);
    transform-origin: left center;
    transition: transform var(--mid) var(--ease-out);
  }
  .racts {
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .icon-btn {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--raise-hover);
    color: var(--text);
  }
  .icon-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .btn.small {
    height: 28px;
    font-size: 12px;
  }
</style>
