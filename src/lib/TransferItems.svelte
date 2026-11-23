<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // What was in a Direct transfer: its files and folders (a folder with its file count), each
  // with its size; received ones open from here (Open / Folder). Unfolded on a card, and in
  // Direct's details panel for the card picked.
  import type { DirectItem } from "./api";
  import { bytes } from "./format";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import Icon from "./Icon.svelte";
  import { num, t } from "./i18n.svelte";

  let {
    items,
    more = 0,
    id,
    label,
    open = null,
    compact = false,
  }: {
    items: DirectItem[];
    /** Items past the 200 kept (only counted). */
    more?: number;
    id?: string;
    label?: string;
    /** Received and still where it was saved: opens one of them (its place in `items`). */
    open?: ((part: number, reveal: boolean) => void) | null;
    /** A narrow column (the details panel): Open and Folder as small icons. */
    compact?: boolean;
  } = $props();

  const count = (n: number) => t("{n} files", { n: num(n) });
  /** Pako knows where this one went (a single file is the transfer itself). */
  const canOpen = (it: DirectItem) => !!open && (!!it.at || (items.length === 1 && !it.dir));
</script>

<ul class="tis" {id} aria-label={label ?? t("Files and folders in it")}>
  {#each items as it, i (i)}
    <li class="ti">
      <FileIcon name={it.name} kind={it.dir ? "folder" : undefined} size={14} />
      <MiddleText text={it.name} class="tin" />
      <span class="tim">{it.dir ? `${count(it.files)} · ` : ""}{bytes(it.size)}</span>
      {#if canOpen(it)}
        <span class="tia" class:compact>
          {#if compact}
            <button class="ib" aria-label={t("Open {name}", { name: it.name })} title={t("Open")} onclick={() => open?.(i, false)}><Icon name="open" size={12} /></button>
            <button class="ib" aria-label={t("Show {name} in its folder", { name: it.name })} title={t("Show in its folder")} onclick={() => open?.(i, true)}><Icon name="folder" size={12} /></button>
          {:else}
            <button class="link" aria-label={t("Open {name}", { name: it.name })} onclick={() => open?.(i, false)}>{t("Open")}</button>
            <button class="link" aria-label={t("Show {name} in its folder", { name: it.name })} onclick={() => open?.(i, true)}>{t("Folder")}</button>
          {/if}
        </span>
      {/if}
    </li>
  {/each}
  {#if more}
    <li class="timore">{t("and {n} more not listed", { n: num(more) })}</li>
  {/if}
</ul>

<style>
  .tis {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    font-size: 12px;
  }
  .ti {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    min-height: 26px;
    padding: 3px 8px;
    border-radius: var(--radius);
    transition: background var(--fast) ease;
  }
  .ti:hover {
    background: var(--ti-hover, var(--hover));
  }
  .ti :global(.tin) {
    color: var(--ti-text, var(--text));
  }
  .tim {
    flex: none;
    color: var(--ti-meta, var(--text-3));
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .tia {
    flex: none;
    display: flex;
    gap: 10px;
    margin-left: 4px;
  }
  .tia.compact {
    gap: 2px;
    margin: -2px -4px -2px 0;
  }
  .ib {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--text-3);
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .ib:hover {
    background: var(--active);
    color: var(--text);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--focus-text);
    font: inherit;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .timore {
    padding: 4px 8px 2px;
    color: var(--ti-meta, var(--text-3));
  }
</style>
