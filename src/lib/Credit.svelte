<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Pako by Sookera" with the licence (Help › About Pako: copyright, no warranty, the licence and
  // its additional terms), the privacy page and the help on the web (Settings and Help).
  import { openWeb, SITE } from "./web";
  import { help } from "./help/help.svelte";
</script>

<div class="credit">
  <span>Pako by <button class="lk" onclick={() => openWeb(SITE.studio)} title="sookera.com">Sookera</button></span>
  <span class="sep" aria-hidden="true">·</span>
  <button class="lk" onclick={() => help.show("about")} title="About Pako: copyright, licence, no warranty">Free software (GPL-3.0)</button>
  <span class="sep" aria-hidden="true">·</span>
  <button class="lk" onclick={() => openWeb(SITE.privacy)} title="sookera.com/privacy">Privacy</button>
  <span class="sep" aria-hidden="true">·</span>
  <button class="lk" onclick={() => openWeb(SITE.help)} title="pako.sookera.com/help">Help online</button>
</div>

<style>
  .credit {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 12px;
    color: var(--text-3);
  }
  .sep {
    color: var(--text-4);
  }
  .lk {
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    color: var(--text-2);
    cursor: pointer;
    text-decoration: underline;
    text-decoration-color: var(--line-strong);
    text-underline-offset: 3px;
    transition:
      color var(--fast) ease,
      text-decoration-color var(--fast) ease;
  }
  .lk:hover {
    color: var(--text);
    text-decoration-color: var(--accent);
  }
</style>
