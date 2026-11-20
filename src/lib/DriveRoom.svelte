<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Make room…" in your Google Drive: tick older uploads, delete them from your Drive. Opened
  // from Upload (its Google Drive bar, or when an upload doesn't fit) and from Private mode's
  // Upload… window, over it (you're back in that window afterwards).
  import { onMount, untrack } from "svelte";
  import { api, type SentItem } from "./api";
  import { app } from "./state.svelte";
  import { bytes, shortDate } from "./format";
  import Icon from "./Icon.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import Modal from "./Modal.svelte";

  let { onclose, onfreed = () => {} }: { onclose: () => void; /** Something was deleted: the room changed. */ onfreed?: () => void } = $props();

  const WEEK = 7 * 86_400_000;
  /** Pako's uploads in your Drive, oldest first (not ones still uploading). */
  const sends = $derived(
    app.sent
      .filter((s) => s.host === "drive" && !s.gone && !app.sends.some((j) => j.id === s.id && j.status === "uploading"))
      .sort((a, b) => a.date - b.date),
  );
  /** The Google account connected now: uploads in another account can't be deleted from here
   *  (they're listed apart, not offered). */
  let email = $state<string | null>(null);
  onMount(async () => {
    try {
      email = (await api.sendDriveStatus()).email ?? null;
    } catch {
      // Not known: every upload is offered, as before.
    }
  });
  const same = (s: SentItem) => !email || !s.account || s.account.toLowerCase() === email.toLowerCase();
  const mine = $derived(sends.filter(same));
  const elsewhere = $derived(sends.filter((s) => !same(s)));
  // Older than a week ticked to start with.
  let picked = $state<Set<string>>(untrack(() => new Set(sends.filter((s) => Date.now() - s.date > WEEK).map((s) => s.id))));
  let deleting = $state(false);
  /** What's ticked among what's listed now (an upload that left the list no longer counts). */
  const ticked = $derived(mine.filter((s) => picked.has(s.id)));
  const pickedSize = $derived(ticked.reduce((t, s) => t + s.size, 0));

  function toggle(s: SentItem) {
    const next = new Set(picked);
    if (next.has(s.id)) next.delete(s.id);
    else next.add(s.id);
    picked = next;
  }
  async function deletePicked() {
    const list = ticked;
    if (!list.length || deleting) return;
    deleting = true;
    let done = 0;
    let freed = 0;
    const failed: string[] = [];
    for (const s of list) {
      try {
        await api.sentRemove(s.id, true);
        done++;
        freed += s.size;
      } catch (e) {
        failed.push(String(e));
      }
    }
    deleting = false;
    const n = (k: number) => (k === 1 ? "1 upload" : `${k} uploads`);
    if (!failed.length) app.toast(`Deleted ${n(done)} from your Google Drive, ${bytes(freed)} freed`);
    else app.toast(`Deleted ${n(done)}. ${n(failed.length)} couldn't be deleted: ${failed[0]}`, "error");
    if (done) {
      app.driveTick++;
      onfreed();
    }
    onclose();
  }
</script>

<Modal title="Make room in your Google Drive" width={540} onclose={() => !deleting && onclose()}>
  <p class="msg">Ticked uploads are deleted from your Google Drive for good, and their links stop working for everyone. Your own copies on this PC aren't touched. Uploads older than a week are ticked to start with.</p>
  {#if elsewhere.length}
    <p class="msg dim">{elsewhere.length === 1 ? "1 upload is" : `${elsewhere.length} uploads are`} in another Google account{elsewhere[0].account ? ` (${elsewhere[0].account})` : ""}: connect that account to delete {elsewhere.length === 1 ? "it" : "them"}.</p>
  {/if}
  {#if mine.length}
    <ul class="rlist">
      {#each mine as s (s.id)}
        <li>
          <button class="row" role="checkbox" aria-checked={picked.has(s.id)} onclick={() => toggle(s)}>
            <span class="check" class:on={picked.has(s.id)} aria-hidden="true">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
            </span>
            <FileIcon name={s.name} kind={s.asFiles ? "folder" : undefined} size={16} />
            <MiddleText text={s.name} class="nm" />
            <span class="rmeta">{bytes(s.size)} · {shortDate(s.date)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="msg dim">Nothing Pako uploaded is in your Google Drive yet, so there's nothing to delete here. Free some room in Google Drive itself.</p>
  {/if}
  {#snippet actions()}
    <span class="dim picked">{ticked.length ? `${ticked.length} ticked · ${bytes(pickedSize)} freed` : "Nothing ticked"}</span>
    <button class="btn ghost" disabled={deleting} onclick={onclose}>Cancel</button>
    <button class="btn danger-fill" disabled={deleting || !ticked.length} onclick={deletePicked}><Icon name="trash" size={13} />{deleting ? "Deleting…" : "Delete from Google Drive"}</button>
  {/snippet}
</Modal>

<style>
  .msg {
    margin: 0 0 12px;
    line-height: 1.55;
    color: var(--text-2);
  }
  .dim {
    color: var(--text-3);
  }
  .rlist {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 300px;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: var(--radius);
  }
  .rlist li + li {
    border-top: 1px solid var(--line-soft);
  }
  .rlist .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .rlist .row:hover {
    background: var(--hover);
  }
  .rlist :global(.nm) {
    flex: 1;
    min-width: 0;
  }
  .rmeta {
    flex: none;
    color: var(--text-3);
    font-size: 12px;
  }
  .picked {
    margin-right: auto;
    font-size: 12px;
  }
</style>
