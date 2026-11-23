<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Upload…" on private files, right here: the same choices as Upload's New upload (where it
  // goes, Delete after, how, Protect with a password), then it uploads while you stay in
  // Private mode; its progress shows on the files' rows and in the details panel. Read
  // decrypted as it goes up (see private-mode.md); nothing goes to Upload's own basket.
  import { onMount, untrack } from "svelte";
  import { api, type Download, type SendMode } from "../api";
  import { app, type BasketItem } from "../state.svelte";
  import { priv } from "../private.svelte";
  import Modal from "../Modal.svelte";
  import SendBasket from "../SendBasket.svelte";
  import DriveRoom from "../DriveRoom.svelte";
  import { said, t, type Words } from "../i18n.svelte";

  let { items, title = null, onclose }: { items: Download[]; title?: string | null; onclose: () => void } = $props();

  /** What's uploaded, in words that follow a language switch (its progress row keeps them). */
  const whatW: Words = untrack(() => {
    const n = items.length;
    return title ?? (n === 1 ? items[0].name : () => t("{n} files", { n }));
  });
  const what = $derived(said(whatW));
  let basket = $state<BasketItem[]>([]);
  /** Each handed-over path's private file (the rows its progress shows on). */
  let fileOf = new Map<string, Download>();
  let error = $state("");
  /** "Make room…" in your Google Drive, over this window (back here afterwards). */
  let room = $state(false);

  onMount(async () => {
    try {
      const paths = await api.privateHandOver(items.map((d) => d.id));
      fileOf = new Map(paths.map((p, i) => [p, items[i]]));
      basket = paths.map((path, i) => ({ path, name: items[i]?.name ?? "…", private: true, dir: false, size: items[i]?.size ?? null, files: 1, error: null }));
      const sizes = await api.sendSizes(paths);
      const by = new Map(sizes.map((s) => [s.path, s]));
      basket = basket.map((b) => {
        const s = by.get(b.path);
        return s ? { ...b, name: s.name ?? b.name, size: s.size, files: s.files, error: s.error } : b;
      });
    } catch (e) {
      error = String(e);
    }
  });

  async function send(mode: SendMode | null, name: string | null, days: number, partMb: number | null, password: string | null) {
    const paths = basket.filter((b) => !b.error).map((b) => b.path);
    if (!paths.length) return;
    const files = paths.map((p) => fileOf.get(p)).filter((d): d is Download => !!d);
    try {
      const n = files.length;
      await priv.upload(files, n === items.length ? whatW : n === 1 ? files[0].name : () => t("{n} files", { n }), paths, mode, name, days, partMb, password);
      priv.say(t("Uploading {what}. Its progress shows here, in Private mode.", { what }));
      onclose();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<Modal title={t("Upload {what}", { what })} width={560} onclose={onclose}>
  <p class="msg">
    {items.length === 1
      ? t("Pako reads it decrypted as it goes up. You stay in Private mode: the upload's progress shows on its row. Leaving Private mode stops it.")
      : t("Pako reads them decrypted as they go up. You stay in Private mode: the upload's progress shows on their rows. Leaving Private mode stops it.")}
  </p>
  {#if error}<p class="err">{error}</p>{/if}
  {#if basket.length}
    <div class="wrap">
      <SendBasket {basket} onremove={(p) => (basket = basket.filter((b) => b.path !== p))} onsend={send} onroom={() => (room = true)} />
    </div>
  {/if}
</Modal>

{#if room}
  <DriveRoom onclose={() => (room = false)} />
{/if}

<style>
  .msg {
    margin: 0 0 12px;
    color: var(--text-2);
    line-height: 1.5;
  }
  .err {
    margin: 0 0 12px;
    color: var(--st-fail-text);
  }
  .wrap :global(.basket) {
    margin: 0;
  }
</style>
