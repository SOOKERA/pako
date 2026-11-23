<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The small button at the end of a list's column titles: the same columns menu as a
  // right-click on the titles (show or hide each column, Reset columns).
  import Icon from "./Icon.svelte";
  import { listColumns } from "./columns.svelte";
  import type { MenuItem } from "./menu";
  import { t } from "./i18n.svelte";

  let {
    list,
    onmenu,
    onhide,
    noRoom = () => [],
  }: {
    list: string;
    onmenu: (x: number, y: number, items: MenuItem[]) => void;
    onhide?: (key: string) => void;
    /** The ticked columns the list has no room for now (read live while the menu is open). */
    noRoom?: () => string[];
  } = $props();

  function open(e: MouseEvent) {
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    onmenu(r.right - 232, r.bottom + 4, listColumns.menu(list, onhide, noRoom));
  }
</script>

<button class="colbtn" aria-label={t("Columns")} aria-haspopup="menu" title={t("Columns: show or hide them")} onclick={open}><Icon name="columns" size={13} /></button>

<style>
  .colbtn {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-3);
    transition:
      background var(--fast) var(--ease),
      color var(--fast) var(--ease);
  }
  .colbtn:hover {
    background: var(--hover);
    color: var(--text);
  }
  .colbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
</style>
