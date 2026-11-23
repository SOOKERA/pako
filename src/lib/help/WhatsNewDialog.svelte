<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Shown once after Pako updates: every version since the one you had, newest first (an update
  // can skip a few), from docs/help/whats-new.md (docs/help/fr/whats-new.md in French).
  import Modal from "../Modal.svelte";
  import type { WhatsNew } from "./content";
  import { help } from "./help.svelte";
  import Markdown from "./Markdown.svelte";
  import { openWeb } from "../web";
  import { plainVersion, whatsNewTitle } from "./versions";
  import { t } from "../i18n.svelte";

  /** `seen`: the version you had (the title says "since" it when several show). */
  let { entries, seen, version, onclose }: { entries: WhatsNew[]; seen: string | null; version: string; onclose: () => void } = $props();
  const several = $derived(entries.length > 1);

  function link(href: string) {
    // A web page opens in the browser (What's new stays); a guide opens in Help.
    if (/^https?:\/\//.test(href)) return void openWeb(href);
    onclose();
    help.show(href);
  }
</script>

<Modal title={whatsNewTitle(entries.length, seen, version, t)} width={560} {onclose}>
  <div class="wn">
    {#each entries as e (e.version)}
      {#if several}
        <h3 class="ver">{t("Pako {v}", { v: plainVersion(e.version) })}</h3>
      {/if}
      <Markdown blocks={e.blocks} onlink={link} />
    {/each}
  </div>
  {#snippet actions()}
    <button
      class="btn ghost"
      onclick={() => {
        onclose();
        help.show();
      }}>{t("Help and tutorials")}</button
    >
    <button class="btn primary" onclick={onclose}>{t("Got it")}</button>
  {/snippet}
</Modal>

<style>
  .wn {
    padding: 6px 0 4px;
  }
  /* Several versions: each one's number as a heading, with a line above all but the first. */
  .ver {
    margin: 18px 0 2px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  .ver:first-child {
    margin-top: 0;
    padding-top: 0;
    border-top: none;
  }
  /* The version's sections read as small headings here. */
  .wn :global(h4) {
    margin-top: 14px;
    font-size: 12px;
    color: var(--text-3);
    letter-spacing: 0.02em;
  }
</style>
