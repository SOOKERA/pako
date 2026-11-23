<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Settings: the "Advanced" fold at the bottom of a tab, for what most people never change,
  // under the same group titles as the tab's other settings (SetGroup.svelte).
  // Shut by default; open or shut is remembered per tab on this PC. Settings' search opens it
  // (`data-fold` / `data-fold-toggle`) to show a setting inside.
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import Collapse from "./Collapse.svelte";
  import { t } from "./i18n.svelte";

  let { tab, hint, children }: { tab: string; /** What's inside, in a few words. */ hint: string; children: Snippet } = $props();

  const KEY = "pako.settingsAdvanced";
  function remembered(): Record<string, boolean> {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? "{}");
      return v && typeof v === "object" ? v : {};
    } catch {
      return {};
    }
  }
  // svelte-ignore state_referenced_locally
  let open = $state(!!remembered()[tab]);
  function toggle() {
    open = !open;
    try {
      localStorage.setItem(KEY, JSON.stringify({ ...remembered(), [tab]: open }));
    } catch {
      // Not remembered (storage blocked): it still opens.
    }
  }
</script>

<div class="adv" data-fold>
  <button class="fold" data-fold-toggle aria-expanded={open} aria-controls="adv-{tab}" onclick={toggle}>
    <span class="chev" class:on={open}><Icon name="chevron" size={12} stroke={2.6} /></span>
    <span class="words">
      <span class="t">{t("Advanced")}</span>
      <span class="muted">{hint}</span>
    </span>
  </button>
  <Collapse {open} id="adv-{tab}">{@render children()}</Collapse>
</div>

<style>
  /* Below the tab's groups; inside, the advanced settings sit under the same group titles. */
  .adv {
    margin-top: 22px;
  }
  /* The same look as "What Pako can do" (UpdatesSection.svelte). */
  .fold {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    max-width: 100%;
    padding: 2px 0 2px 2px;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .words {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .t {
    font-weight: 500;
    margin-bottom: 2px;
  }
  .muted {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .fold:hover .t {
    color: var(--text);
  }
  .chev {
    display: inline-flex;
    flex: none;
    color: var(--text-3);
    transform: rotate(-90deg);
    transition: transform var(--mid) var(--ease);
  }
  .chev.on {
    transform: none;
  }
</style>
