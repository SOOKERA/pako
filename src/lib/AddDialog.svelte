<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { app } from "./state.svelte";
  import { api } from "./api";
  import { findLinks } from "./format";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { t } from "./i18n.svelte";

  let { text = "", onclose }: { text?: string; onclose: () => void } = $props();

  let value = $state(untrack(() => text));
  let busy = $state(false);
  let area: HTMLTextAreaElement | undefined = $state();

  const links = $derived(findLinks(value));
  const drive = $derived(links.filter((l) => /(drive|docs)\.google\.com|drive\.usercontent/i.test(l)).length);

  onMount(() => setTimeout(() => area?.focus(), 60));

  async function submit() {
    if (!links.length || busy) return;
    busy = true;
    const added = await app.addToFound(value);
    busy = false;
    if (added > 0) onclose();
  }
</script>

<Modal title={t("Add links")} width={540} {onclose}>
  <textarea
    bind:this={area}
    bind:value
    class="input"
    rows="6"
    spellcheck="false"
    placeholder={t("Paste links, or any text that contains links (a message, an email, a web page). Google Drive files and folders work too.")}
    onkeydown={(e) => e.key === "Enter" && e.ctrlKey && submit()}
  ></textarea>
  <div class="found" class:none={!links.length}>
    {#if links.length}
      <Icon name="check" size={13} stroke={2.4} />
      {t("{n} links found", { n: links.length })}{drive ? ` (${drive} Google Drive)` : ""}
    {:else}
      {value.trim() ? t("No links found in this text") : t("Links are picked out automatically")}
    {/if}
  </div>

  <p class="hint">{t("They go to Links first: you see names and sizes, pick what you want, then download.")}</p>
  {#if drive}
    <p class="note">{t('Drive links shared with "anyone with the link" open right here, folders too. Files only shared with some people open in your browser instead.')}</p>
  {/if}

  {#snippet actions()}
    <button class="btn ghost" onclick={onclose}>{t("Cancel")}</button>
    <button class="btn primary" disabled={busy || !links.length} onclick={submit}>
      {busy ? t("Adding…") : links.length > 1 ? t("Add {n} links", { n: links.length }) : t("Add link")}
    </button>
  {/snippet}
</Modal>

<style>
  textarea {
    min-height: 130px;
  }
  .found {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    font-size: 12px;
    color: var(--st-done-text);
    transition: color var(--mid) ease;
  }
  .found.none {
    color: var(--text-3);
  }
  .hint {
    margin: 14px 0 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .note {
    margin: 14px 0 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--raise);
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-2);
  }
</style>
