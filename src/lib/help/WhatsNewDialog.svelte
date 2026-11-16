<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Shown once after Pako updates to a version with notes in docs/help/whats-new.md.
  import Modal from "../Modal.svelte";
  import type { WhatsNew } from "./content";
  import { help } from "./help.svelte";
  import Markdown from "./Markdown.svelte";

  let { entry, version, onclose }: { entry: WhatsNew; version: string; onclose: () => void } = $props();

  function link(href: string) {
    onclose();
    help.show(href.startsWith("http") ? undefined : href);
  }
</script>

<Modal title="What's new in Pako {version}" width={560} {onclose}>
  <div class="wn">
    <Markdown blocks={entry.blocks} onlink={link} />
  </div>
  {#snippet actions()}
    <button
      class="btn ghost"
      onclick={() => {
        onclose();
        help.show();
      }}>Help and tutorials</button
    >
    <button class="btn primary" onclick={onclose}>Got it</button>
  {/snippet}
</Modal>

<style>
  .wn {
    padding: 6px 0 4px;
  }
  /* The version's sections read as small headings here. */
  .wn :global(h4) {
    margin-top: 14px;
    font-size: 12px;
    color: var(--text-3);
    letter-spacing: 0.02em;
  }
</style>
