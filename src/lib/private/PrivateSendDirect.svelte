<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Send with Direct…" on private files: pick a contact or one of your PCs. Pako reads the
  // files decrypted as they go (no readable copy on this PC); the other side gets normal files.
  // The send shows in Direct's history (or My devices') like any other.
  import { onMount } from "svelte";
  import { api, type Download } from "../api";
  import { app } from "../state.svelte";
  import { devices } from "../devices.svelte";
  import { priv } from "../private.svelte";
  import Modal from "../Modal.svelte";
  import Icon from "../Icon.svelte";

  let { items, title = null, onclose }: { items: Download[]; title?: string | null; onclose: () => void } = $props();

  // Direct may not be running yet (it starts the first time its page opens): start it, so your
  // contacts are listed and the send can go.
  onMount(() => {
    devices.start();
    api.directWake().catch(() => {});
    app.refreshDirect?.();
  });

  type Who = { id: string; name: string; online: boolean; device: boolean };
  let find = $state("");
  let busy = $state(false);
  let error = $state("");

  const people = $derived.by(() => {
    const all: Who[] = [
      ...(devices.data?.devices ?? []).filter((d) => d.paired).map((d) => ({ id: d.id, name: d.name, online: d.online, device: true })),
      ...(app.direct?.contacts ?? []).filter((c) => c.trusted).map((c) => ({ id: c.id, name: c.name, online: c.online, device: false })),
    ];
    const q = find.trim().toLowerCase();
    return all.filter((w) => !q || w.name.toLowerCase().includes(q)).sort((a, b) => Number(b.online) - Number(a.online) || a.name.localeCompare(b.name));
  });
  const what = $derived(title ?? (items.length === 1 ? items[0].name : `${items.length} files`));

  async function send(w: Who) {
    if (busy) return;
    busy = true;
    error = "";
    try {
      const paths = await api.privateHandOver(items.map((d) => d.id));
      const job = await api.directSend(w.id, paths, false, title);
      // You stay in Private mode: its progress shows on the files' rows and in the details panel.
      priv.sending(items, what, w.name, job, w.device);
      priv.say(`Sending ${what} to ${w.name}. Its progress shows here, in Private mode.`);
      onclose();
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }
</script>

<Modal title="Send {what} with Direct" width={440} onclose={() => !busy && onclose()} outsideCloses={!busy} closable={!busy}>
  <div class="pick">
    <p class="msg">Pako reads {items.length === 1 ? "it" : "them"} decrypted as {items.length === 1 ? "it goes" : "they go"}: no readable copy is made on this PC. The other side gets normal files, and the send shows in Direct's history. You stay here: its progress shows on {items.length === 1 ? "its row" : "their rows"}, and leaving Private mode stops it.</p>
    <input class="input" placeholder="Find a contact or one of your PCs" aria-label="Find a contact" bind:value={find} />
    <div class="list" role="listbox" aria-label="Send to">
      {#each people as w (w.id)}
        <button class="who" role="option" aria-selected="false" disabled={busy} onclick={() => send(w)}>
          <span class="badge">{#if w.device}<Icon name="devices" size={13} />{:else}{w.name.slice(0, 1).toUpperCase()}{/if}</span>
          <span class="nm">{w.name}</span>
          <span class="st" class:on={w.online}>{w.online ? "Online" : "Offline"}</span>
        </button>
      {:else}
        <p class="none">{find ? "Nobody by that name." : "No contacts yet: add someone in Direct first."}</p>
      {/each}
    </div>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </div>
  {#snippet actions()}
    <button class="btn ghost" disabled={busy} onclick={onclose}>Cancel</button>
  {/snippet}
</Modal>

<style>
  .pick {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .msg {
    margin: 0;
    color: var(--text-2);
    line-height: 1.5;
    font-size: 12.5px;
  }
  .list {
    max-height: 260px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    transition: background var(--fast) ease;
  }
  .who:hover:not(:disabled) {
    background: var(--raise);
  }
  .badge {
    width: 26px;
    height: 26px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--raise);
    border: 1px solid var(--line);
    font-size: 12px;
    font-weight: 600;
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .st {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .st.on {
    color: var(--st-done-text);
  }
  .none {
    margin: 8px 0;
    color: var(--text-3);
    font-size: 12.5px;
  }
  .err {
    margin: 0;
    font-size: 12.5px;
    color: var(--st-fail-text);
  }
  @media (prefers-reduced-motion: reduce) {
    .who {
      transition: none;
    }
  }
</style>
