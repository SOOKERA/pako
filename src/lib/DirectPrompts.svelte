<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Direct's questions, shown on any tab: "… wants to connect" (the first time only) and
  // "… wants to send you …" (unless that contact may send without asking). One at a time,
  // oldest first. Accept wakes up a moment after a question appears, so a click meant for
  // something else can't accept it.
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api } from "./api";
  import { bytes } from "./format";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  const request = $derived(app.direct?.requests?.[0] ?? null);
  const offer = $derived(request ? null : (app.direct?.offers?.[0] ?? null));
  /** A folder picked for this offer (else the contact's folder). */
  let folder = $state<string | null>(null);
  let lastOffer = "";
  $effect(() => {
    if (offer && offer.id !== lastOffer) {
      lastOffer = offer.id;
      folder = null;
    }
  });
  const plural = (n: number, one: string, many = `${one}s`) => `${n.toLocaleString()} ${n === 1 ? one : many}`;

  /** Accept answers only once the question has been on screen for a moment. */
  let armed = $state(false);
  const shown = $derived(request ? `r:${request.peer}` : offer ? `o:${offer.id}` : "");
  $effect(() => {
    if (!shown) return;
    armed = false;
    const t = setTimeout(() => (armed = true), 600);
    return () => clearTimeout(t);
  });

  async function change() {
    const r = await pick({ directory: true, multiple: false, title: "Where to save these files" });
    if (typeof r === "string") folder = r;
  }
</script>

{#if request}
  {@const r = request}
  <Modal title="{r.name} wants to connect" width={480} closeLabel="Decline" onclose={() => app.run(() => api.directAnswerRequest(r.peer, false))}>
    <div class="body">
      <p class="who dim small"><Icon name="alert" size={13} />Not in your contacts. The name is the one they chose; Pako can't check it.</p>
      {#if r.knownAs}
        <p class="warn small">You already have a contact called {r.knownAs}. This is a different Pako: if you expected {r.knownAs}, decline.</p>
      {/if}
      <p>Once you accept, {r.name} becomes one of your contacts in <b>Direct</b> and you connect by yourselves whenever both Pakos are open. You're only asked this once.</p>
      <div class="code">
        <span class="dim">Safety code</span>
        <b class="mono">{r.code}</b>
      </div>
      <p class="dim small">If you know them, check they see the same code (by voice or chat). A different code means it isn't them.</p>
      <p class="dim small">If you decline (or close this), Pako turns {r.name} away without asking you for 7 days.</p>
    </div>
    {#snippet actions()}
      <button class="btn ghost" title="Decline (Esc)" onclick={() => app.run(() => api.directAnswerRequest(r.peer, false))}>Decline</button>
      <button class="btn primary" class:arming={!armed} aria-disabled={!armed} onclick={() => armed && app.run(() => api.directAnswerRequest(r.peer, true))}>Accept</button>
    {/snippet}
  </Modal>
{:else if offer}
  {@const o = offer}
  <Modal title="{o.contactName} wants to send you files" width={500} closeLabel="Decline" onclose={() => app.run(() => api.directAnswerOffer(o.id, false))}>
    <div class="body">
      <div class="what">
        <Icon name={o.files > 1 ? "folder" : "file"} size={18} />
        <div>
          <div class="wname">{o.name}</div>
          <div class="dim small">{plural(o.files, "file")} · {bytes(o.size)}</div>
        </div>
      </div>
      {#if o.files > 1}
        <ul class="sample dim small">
          {#each o.sample as s (s)}<li>{s}</li>{/each}
          {#if o.files > o.sample.length}<li>and {plural(o.files - o.sample.length, "more file")}</li>{/if}
        </ul>
      {/if}
      <span class="lbl dim small">Save to</span>
      <div class="field">
        <span class="path" title={folder ?? o.folder}>{folder ?? o.folder}</span>
        <button class="link" onclick={change}>Change</button>
      </div>
      {#if o.askedBecause}
        <p class="dim small">{o.askedBecause}</p>
      {:else}
        <p class="dim small">To skip this question for {o.contactName}, turn on "Can send to me without asking" in Direct.</p>
      {/if}
    </div>
    {#snippet actions()}
      <button class="btn ghost" title="Decline (Esc)" onclick={() => app.run(() => api.directAnswerOffer(o.id, false))}>Decline</button>
      <button class="btn primary" class:arming={!armed} aria-disabled={!armed} onclick={() => armed && app.run(() => api.directAnswerOffer(o.id, true, folder))}><Icon name="download" size={13} stroke={2.2} />Accept</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .body p {
    margin: 0;
    line-height: 1.55;
    color: var(--text-2);
  }
  .dim {
    color: var(--text-3);
  }
  .body p.dim {
    color: var(--text-3);
  }
  .small {
    font-size: 12px;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .who :global(svg) {
    flex: none;
  }
  .warn {
    padding: 8px 10px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--st-retry) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--st-retry) 30%, transparent);
    line-height: 1.5;
  }
  .body p.warn {
    color: var(--text);
  }
  /* Just shown: Accept fades in and doesn't take clicks yet. */
  .btn.primary {
    transition:
      opacity var(--slow) var(--ease),
      background-color var(--fast) var(--ease);
  }
  .btn.primary.arming {
    opacity: 0.55;
    pointer-events: none;
  }
  .code {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--panel);
    border: 1px solid var(--line-soft);
  }
  .code b {
    font-size: 16px;
    letter-spacing: 0.06em;
    color: var(--text);
  }
  .what {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .what :global(svg) {
    color: var(--accent-strong);
  }
  .wname {
    font-weight: 600;
  }
  .sample {
    margin: 0;
    padding-left: 18px;
    line-height: 1.6;
    max-height: 120px;
    overflow: hidden;
  }
  .lbl {
    margin-top: 2px;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--radius);
    border: 1px solid var(--control);
    background: var(--bg);
  }
  .path {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--focus-text);
    font: inherit;
    cursor: pointer;
  }
</style>
