<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { untrack } from "svelte";
  import { app, hostName, hostSwitch, type Row, type SortKey } from "./state.svelte";
  import { api, inApp, isActive, type Download } from "./api";
  import { startDrag } from "@crabnebula/tauri-plugin-drag";
  import { bytes, bytesOf, downloadLook, extension, sourceOf, speed, statusLook, timeLeft, shortDate, dateTitle, NO_NET } from "./format";
  import { flipIf, flipOpts, rowSlide } from "./motion";
  import { VIRTUAL_MIN, layout, reveal, visible } from "./virtual";
  import Icon from "./Icon.svelte";
  import { alertLabel, alertOf, doneAlert, nextAlert } from "./alert";
  import { priv } from "./private.svelte";
  /** Height of the one-time Private mode tip under its row (see `.tip`). */
  const TIP_H = 40;
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import ColumnGrip from "./ColumnGrip.svelte";
  import ColumnsButton from "./ColumnsButton.svelte";
  import { listColumns } from "./columns.svelte";
  import { LISTS, gridTemplate, shownColumns } from "./columns";
  import type { MenuItem } from "./menu";
  import { around, t, tMaybe, type Key } from "./i18n.svelte";
  import type { Status } from "./api";

  let {
    oncontext,
    onadd,
    onempty,
    onmenu,
  }: {
    oncontext: (e: MouseEvent, ids: string[], clicked?: string) => void;
    onadd: () => void;
    /** Right-click on empty space in the list. */
    onempty: (e: MouseEvent) => void;
    /** Opens a menu (the columns menu). */
    onmenu: (x: number, y: number, items: MenuItem[]) => void;
  } = $props();

  // ---------- drag a row to change its place in the queue ----------
  const DRAG_TYPE = "application/x-pako-ids";
  let dropTarget = $state<string | null>(null);
  const canReorder = $derived(!app.sortDownloads);

  // ---------- drag finished files out (to Explorer, the desktop, another app) ----------
  // Finished rows don't need a place in the queue, so dragging one takes the file itself out of
  // Pako (a real Windows drag). Unfinished rows keep dragging to reorder the queue.
  let outPress: { x: number; y: number; ids: string[]; folder: boolean } | null = null;
  function outDown(e: PointerEvent, ids: string[], folder: boolean) {
    if (e.button !== 0 || e.ctrlKey || e.shiftKey || !inApp) return;
    outPress = { x: e.clientX, y: e.clientY, ids, folder };
  }
  function outMove(e: PointerEvent) {
    if (!outPress || (e.buttons & 1) === 0) {
      outPress = null;
      return;
    }
    if (Math.hypot(e.clientX - outPress.x, e.clientY - outPress.y) < 6) return;
    const press = outPress;
    outPress = null;
    dragOut(press.ids, press.folder);
  }
  async function dragOut(ids: string[], folder: boolean) {
    const paths = await api.dragPaths(ids, folder).catch(() => [] as string[]);
    if (!paths.length) {
      app.toast(
        folder
          ? t("This folder can't be dragged out: some of its files aren't finished, or it isn't on the disk any more.")
          : t("This file isn't on the disk any more (moved or deleted)."),
        "error",
      );
      return;
    }
    await startDrag({ item: paths, icon: dragIcon(paths.length, folder) }).catch((e) => app.toast(String(e), "error"));
  }
  /** Finished rows being dragged: the selection if this row is in it, else just this row. Only
   *  rows shown under the current filter or search (like every other action on the selection). */
  function outIds(d: Download) {
    if (!app.selected.has(d.id)) return [d.id];
    return app.selectedDownloads.filter((x) => x.status === "completed").map((x) => x.id);
  }
  /** The little picture under the cursor while dragging: a warm red tile with the count. */
  const icons = new Map<string, string>();
  function dragIcon(n: number, folder: boolean) {
    const key = `${n}|${folder}`;
    const cached = icons.get(key);
    if (cached) return cached;
    const c = document.createElement("canvas");
    c.width = c.height = 56;
    const g = c.getContext("2d")!;
    g.fillStyle = "#f96263";
    g.beginPath();
    g.roundRect(4, 4, 48, 48, 10);
    g.fill();
    g.fillStyle = "#17181c";
    g.font = "600 18px 'Inter Variable', 'Segoe UI', sans-serif";
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(folder ? "\u{1F4C1}" : n > 1 ? String(n) : "\u2193", 28, 29);
    const url = c.toDataURL("image/png");
    icons.set(key, url);
    return url;
  }
  function dragStart(e: DragEvent, d: Download) {
    if (!canReorder || !e.dataTransfer) return;
    const ids = app.selected.has(d.id) ? app.selectedIds : [d.id];
    e.dataTransfer.setData(DRAG_TYPE, JSON.stringify(ids));
    e.dataTransfer.effectAllowed = "move";
  }
  function dragOver(e: DragEvent, d: Download) {
    if (!e.dataTransfer?.types.includes(DRAG_TYPE)) return;
    e.preventDefault();
    e.stopPropagation();
    dropTarget = d.id;
  }
  function drop(e: DragEvent, d: Download) {
    const raw = e.dataTransfer?.getData(DRAG_TYPE);
    dropTarget = null;
    if (!raw) return;
    e.preventDefault();
    e.stopPropagation();
    const ids = (JSON.parse(raw) as string[]).filter((id) => id !== d.id);
    if (ids.length) app.run(() => api.moveBefore(ids, d.id));
  }

  // ---------- columns: widths, shown or hidden (columns.ts) ----------
  const LIST = LISTS.downloads;
  /** The column titles' width: the list's room, without the scrollbar's. */
  let headW = $state(0);
  const colState = $derived(listColumns.state("downloads"));
  const frozen = $derived(listColumns.frozen?.list === "downloads" ? listColumns.frozen : null);
  const shown = $derived(frozen ? { keys: frozen.keys, tight: false } : shownColumns(LIST, colState, headW));
  const cols = $derived(`--cols: ${gridTemplate(LIST, colState, shown.keys, shown.tight)}`);
  /** Columns not shown (hidden by you, or no room): their cells are left out of the grid. */
  const hideCls = $derived(
    LIST.cols
      .filter((c) => !shown.keys.includes(c.key))
      .map((c) => `h-${c.key}`)
      .join(" "),
  );
  const unsort = (key: string) => app.unsortIf("downloads", key);
  /** Ticked columns the list has no room for now. */
  const noRoom = $derived(LIST.cols.filter((c) => !colState.hidden.includes(c.key) && !shown.keys.includes(c.key)).map((c) => c.key));
  function columnsMenu(e: MouseEvent) {
    e.preventDefault();
    onmenu(e.clientX, e.clientY, listColumns.menu("downloads", unsort, () => noRoom));
  }

  // ---------- row motion ----------
  // Sliding and re-ordering rows is nice on a short list but measures every row, which makes
  // a long list stutter. So: no motion above this many rows, and none for a moment after the
  // filter, search or library changes (the whole list is swapped, not edited).
  const ANIMATE_MAX_ROWS = 60;

  // ---------- long lists: draw only what's on screen ----------
  let body: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewH = $state(800);
  /** Row heights as the CSS draws them: 46 px rows; a folder has 8 px above it; the gap under a
   *  closed folder or a folder's last file is 8 px (inside the row's slot). */
  const geo = $derived(
    layout(
      app.rows.map((r) => {
        if (r.kind === "folder") return { h: 46 + (r.open ? 0 : 8), top: 8, bottom: 0 };
        if (r.kind === "child") return { h: 46 + (r.last ? 8 : 0) + (r.d.id === priv.tipRow ? TIP_H : 0), top: 0, bottom: 0 };
        // The one-time Private mode tip sits right under its row.
        return { h: 46 + (r.d.id === priv.tipRow ? TIP_H : 0), top: 0, bottom: 0 };
      }),
    ),
  );
  const isVirtual = $derived(app.rows.length > VIRTUAL_MIN);
  const win = $derived(isVirtual ? visible(geo, scrollTop, viewH) : { start: 0, end: app.rows.length, padTop: 0, padBottom: 0 });
  const shownRows = $derived(isVirtual ? app.rows.slice(win.start, win.end) : app.rows);
  // A row asked for (keyboard, a fresh paste): scroll it into view, drawn or not. Only a new ask
  // scrolls: the rows and their places are read untracked, so a status change in the list
  // doesn't jump back to the last row asked for.
  $effect(() => {
    const ask = app.revealKey;
    const el = body;
    if (!ask || !el) return;
    untrack(() => {
      const i = app.rows.findIndex((r) => (r.kind === "folder" ? "g:" + r.key : r.d.id) === ask.key);
      if (i >= 0) reveal(el, geo, i);
    });
  });
  const QUIET_AFTER_FILTER_MS = 400;
  // Plain variables (not state): read by the transitions when they start.
  let quietUntil = 0;
  let filterKey = "";
  let lastLen = 0;
  let bigList = false;
  $effect.pre(() => {
    const key = `${app.library}|${app.sourceFilter}|${app.search}`;
    if (key !== filterKey) {
      filterKey = key;
      quietUntil = performance.now() + QUIET_AFTER_FILTER_MS;
    }
    // Rows leaving count too (e.g. clearing 500 finished downloads).
    const len = app.rows.length;
    bigList = Math.max(len, lastLen) > ANIMATE_MAX_ROWS;
    lastLen = len;
  });
  function motionMs(ms: number) {
    return bigList || performance.now() < quietUntil ? 0 : ms;
  }

  // ---------- per-row values ----------
  /** A folder's date: its newest file. */
  function newestOf(files: Download[]) {
    // A loop, not Math.max(...list): a folder of many thousand files would overflow the call.
    let m = 0;
    for (const x of files) m = Math.max(m, x.date || x.addedAt);
    return m;
  }

  function pct(d: Download) {
    if (d.status === "completed") return 100;
    return d.size ? Math.min(100, (d.downloaded / d.size) * 100) : 0;
  }
  function sizeText(d: Download) {
    if (d.status === "completed") return bytes(d.size ?? d.downloaded);
    if (d.size) return bytesOf(d.downloaded, d.size);
    return d.downloaded > 0 ? bytes(d.downloaded) : t("size unknown");
  }
  function speedLine(d: Download) {
    const p = app.live[d.id];
    if (d.status === "downloading" && p && p.speed > 0) return speed(p.speed);
    return "—";
  }
  function subLine(d: Download) {
    const p = app.live[d.id];
    if (d.status === "downloading" && p && d.size) return timeLeft(d.size - d.downloaded, p.speed);
    if (d.status === "retrying") {
      // Rust's words, in English or French (a message kept from before a switch stays as it was).
      if (d.message && NO_NET.test(d.message)) return t("waiting for internet");
      const m = d.message?.match(/(\d+)[\s  ]?s$/);
      return m ? t("retry in {n}s", { n: m[1] }) : "";
    }
    if (d.status === "verifying") return t("checking");
    return "";
  }

  /** A folder's status with how many of its files have it ("3 paused"). */
  const FOLDER_COUNT: Record<Exclude<Status, "completed">, Key> = {
    downloading: "{n} downloading",
    starting: "{n} starting",
    verifying: "{n} checking file",
    retrying: "{n} reconnecting",
    queued: "{n} waiting its turn",
    paused: "{n} paused",
    failed: "{n} failed",
    quotaExceeded: "{n} google limit",
    needsAccount: "{n} only in the browser",
  };
  /** "Sort by speed", and what screen readers hear ("Speed, sorted lowest first. Sort by speed"). */
  function sortWords(label: string, key: string, az: boolean) {
    const sort = t("Sort by {column}", { column: label.toLowerCase() });
    const s = app.sortDownloads;
    if (s?.key !== key) return { title: sort, aria: `${label}. ${sort}` };
    const k: Key = az ? (s.dir > 0 ? "{column}, sorted A to Z. {sort}" : "{column}, sorted Z to A. {sort}") : s.dir > 0 ? "{column}, sorted lowest first. {sort}" : "{column}, sorted highest first. {sort}";
    return { title: sort, aria: t(k, { column: label, sort }) };
  }
  const nameSort = $derived(sortWords(t("Name"), "name", true));
  const tipKeys = $derived(around(t("Tip: press {keys} anywhere to paste links."), "{keys}"));
  const pinTip = $derived(around(t("Want to keep these behind a PIN? {mode}, in Settings."), "{mode}"));

  function folderInfo(files: Download[]) {
    const size = files.reduce((s, f) => s + (f.size ?? 0), 0);
    const done = files.reduce((s, f) => s + (f.status === "completed" ? (f.size ?? f.downloaded) : f.downloaded), 0);
    const finished = files.filter((f) => f.status === "completed").length;
    const sp = files.reduce((s, f) => s + (app.live[f.id]?.speed ?? 0), 0);
    const status = app.folderStatus(files);
    const look = statusLook(status);
    const n = files.filter((f) => f.status === status).length;
    let label = look.label;
    if (status === "completed") label = t("Done");
    else if (n > 1 || files.length > 1) label = t(FOLDER_COUNT[status], { n });
    return {
      size,
      done,
      finished,
      speed: sp,
      look,
      label,
      pct: size ? Math.min(100, (done / size) * 100) : 0,
      allSelected: files.every((f) => app.selected.has(f.id)),
      someSelected: files.some((f) => app.selected.has(f.id)),
    };
  }

  function rowKey(r: Row) {
    return r.kind === "folder" ? "f:" + r.key : r.d.id;
  }

  /** Rows shaded every other one: list rows by their place in the list (a folder counts as one),
   *  a folder's files by their place inside it. */
  const alt = $derived.by(() => {
    const out = new Set<string>();
    let top = 0;
    let inner = 0;
    for (const r of app.rows) {
      if (r.kind === "child") {
        if (inner++ % 2) out.add(rowKey(r));
        continue;
      }
      inner = 0;
      if (r.kind === "file" && top % 2) out.add(rowKey(r));
      top++;
    }
    return out;
  });

  function rowClick(e: MouseEvent, d: Download) {
    app.click(d.id, e);
  }
  function rowDblClick(d: Download) {
    if (d.status === "completed") app.run(() => api.openItem(d.id, false));
    else if (!app.panelOpen) app.togglePanel();
  }
  function rowContext(e: MouseEvent, d: Download) {
    e.preventDefault();
    oncontext(e, app.contextTargets(d.id), d.id);
  }
  function folderContext(e: MouseEvent, files: Download[]) {
    e.preventDefault();
    const ids = files.map((f) => f.id);
    if (!ids.every((id) => app.selected.has(id))) app.selected = new Set(ids);
    oncontext(e, app.selectedIds);
  }

  const allState = $derived.by(() => {
    const n = app.visibleIds.filter((id) => app.selected.has(id)).length;
    return n === 0 ? "none" : n === app.visibleIds.length ? "all" : "some";
  });
</script>

<svelte:window onpointermove={outMove} onpointerup={() => (outPress = null)} />

<div class="table {hideCls}" class:easing={listColumns.easing && app.rows.length <= ANIMATE_MAX_ROWS} data-tour="dl-list" style={cols}>
  <div class="head" role="presentation" oncontextmenu={columnsMenu} bind:clientWidth={headW}>
    <div class="cell">
      <button
        class="check"
        class:on={allState === "all"}
        class:some={allState === "some"}
        role="checkbox"
        aria-checked={allState === "some" ? "mixed" : allState === "all"}
        aria-label={allState === "all" ? t("Unselect all") : t("Select all")}
        onclick={() => (allState === "all" ? app.clearSelection() : app.selectAll())}
      >
        {#if allState === "some"}
          <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 5h6" stroke="var(--accent-ink)" stroke-width="2" /></svg>
        {:else}
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
        {/if}
      </button>
    </div>
    <div class="cell">{t("Type")}</div>
    <button class="cell sorter" class:sorted={app.sortDownloads?.key === "name"} onclick={() => app.toggleSort("downloads", "name")} title={nameSort.title} aria-label={nameSort.aria}>
      {t("Name")}{#if app.sortDownloads?.key === "name"}<Icon name={app.sortDownloads.dir === 1 ? "up" : "down"} size={10} stroke={2.6} />{/if}
    </button>
    {#each LIST.cols as c (c.key)}
      {@const sw = sortWords(c.label, c.key, false)}
      <div class="cell resizable c-{c.key}">
        <ColumnGrip list="downloads" col={c} keys={shown.keys} tight={shown.tight} avail={headW} />
        <button class="sorter" class:sorted={app.sortDownloads?.key === c.key} onclick={() => app.toggleSort("downloads", c.key as SortKey)} title={sw.title} aria-label={sw.aria}>
          {c.label}{#if app.sortDownloads?.key === c.key}<Icon name={app.sortDownloads.dir === 1 ? "up" : "down"} size={10} stroke={2.6} />{/if}
        </button>
      </div>
    {/each}
    <div class="cell colcell"><ColumnsButton list="downloads" {onmenu} onhide={unsort} noRoom={() => noRoom} /></div>
  </div>

  <div
    class="body"
    role="grid"
    tabindex="-1"
    aria-label={t("Downloads")}
    aria-multiselectable="true"
    aria-rowcount={app.rows.length}
    aria-activedescendant={app.focus ? "drow-" + app.focus : undefined}
    onpointerenter={() => (app.listHover = true)}
    onpointerleave={() => (app.listHover = false)}
    bind:this={body}
    bind:clientHeight={viewH}
    onscroll={(e) => (scrollTop = (e.currentTarget as HTMLElement).scrollTop)}
    oncontextmenu={(e) => !(e.target as HTMLElement).closest(".row") && onempty(e)}
  >
    {#if !app.loaded}
      <!-- nothing yet -->
    {:else if app.downloads.length === 0}
      <div class="empty">
        <div class="empty-icon"><Icon name="download" size={26} /></div>
        <h3>{t("No downloads yet")}</h3>
        <p>{t("Paste a web link, or a Google Drive file or folder link.")}</p>
        <button class="btn primary" onclick={onadd}><Icon name="plus" size={14} stroke={2.4} />{t("Add links")}</button>
        <p class="hint">{tipKeys[0]}<kbd>Ctrl</kbd> + <kbd>V</kbd>{tipKeys[1]}</p>
      </div>
    {:else if app.rows.length === 0}
      <div class="empty small"><p>{app.search ? t('Nothing here for "{search}".', { search: app.search }) : t("Nothing here.")}</p></div>
    {:else}
      {#if isVirtual}<div class="vpad" style="height: {win.padTop}px"></div>{/if}
      {#each shownRows as r (rowKey(r))}
        <div class="slot" class:gap={(r.kind === "folder" && !r.open) || (r.kind === "child" && r.last)} animate:flipIf={{ ...flipOpts, duration: Date.now() - app.folderOpenedAt < 400 ? 0 : motionMs(flipOpts.duration) }} transition:rowSlide={{ duration: motionMs(220) }}>
          {#if r.kind === "folder"}
            {@const f = folderInfo(r.files)}
            <div
              class="row folder"
              class:open={r.open}
              class:selected={f.allSelected}
              onpointerdown={(e) => f.finished === r.files.length && outDown(e, r.files.map((x) => x.id), true)}
              role="row"
              tabindex="-1"
              data-key={"g:" + r.key}
              class:focused={app.focusFolder === r.key && !app.focus}
              oncontextmenu={(e) => folderContext(e, r.files)}
              onclick={(e) => {
                app.focusFolder = r.key;
                app.focus = null;
                app.navAnchor = "g:" + r.key;
                // One click opens / closes it (Ctrl / Shift clicks only select).
                // The second click of a double-click doesn't close it again.
                if (e.detail <= 1 && !(e.ctrlKey || e.shiftKey || e.metaKey)) app.toggleFolderOpen(r.key);
              }}
              onkeydown={(e) => e.key === "Enter" && app.toggleFolderOpen(r.key)}
            >
              <div class="cell">
                <button
                  class="check"
                  class:on={f.allSelected}
                  class:some={!f.allSelected && f.someSelected}
                  role="checkbox"
                  aria-checked={!f.allSelected && f.someSelected ? "mixed" : f.allSelected}
                  aria-label={t("Select all files in {name}", { name: r.name })}
                  onclick={(e) => {
                    e.stopPropagation();
                    app.toggleFolder(r.files);
                  }}
                >
                  {#if !f.allSelected && f.someSelected}
                    <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 5h6" stroke="var(--accent-ink)" stroke-width="2" /></svg>
                  {:else}
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
                  {/if}
                </button>
              </div>
              <div class="cell type"><span class="stb" style="background: {f.look.color}"></span><FileIcon kind="folder" size={18} /></div>
              <div class="cell name">
                <button
                  class="chev"
                  ondblclick={(e) => e.stopPropagation()}
                  aria-label={r.open ? t("Collapse folder") : t("Expand folder")}
                  aria-expanded={r.open}
                  onclick={(e) => {
                    e.stopPropagation();
                    app.toggleFolderOpen(r.key);
                  }}
                >
                  <Icon name="chevron" size={14} stroke={2.6} />
                </button>
                <span class="fname">{r.name}</span>
                <span class="chip">{t("{n} files", { n: r.files.length })}{f.finished ? ` · ${t("{n} done", { n: f.finished })}` : ""}</span>
              </div>
              <div class="cell src c-source">
                <span class="dot" style="background: {sourceOf(r.files[0]).color}"></span>
                <span class="ell">{sourceOf(r.files[0]).label}</span>
              </div>
              <div class="cell prog c-progress">
                <div class="barline">
                  <div class="bar"><div class="fill" style="width: {f.pct}%; background: {f.look.color}"></div></div>
                  <span class="pct mono">{t("{pct}%", { pct: Math.floor(f.pct) })}</span>
                </div>
                <span class="sub mono">{f.size ? bytesOf(f.done, f.size) : bytes(f.done)}</span>
              </div>
              <div class="cell spd mono c-speed">
                <span>{f.speed > 0 ? speed(f.speed) : "—"}</span>
              </div>
              <div class="cell status c-status" style="color: {f.look.text}" title={f.label}><span class="ell">{f.label}</span></div>
              <div class="cell date mono c-date" title={dateTitle(newestOf(r.files), null)}>{shortDate(newestOf(r.files))}</div>
            </div>
          {:else}
            {@const d = r.d}
            {@const look = downloadLook(d)}
            {@const sub = subLine(d)}
            <div
              class="row"
              class:alt={alt.has(d.id)}
              class:child={r.kind === "child"}
              class:last={r.kind === "child" && r.last}
              class:selected={app.selected.has(d.id)}
              class:focused={app.focus === d.id}
              class:tint-fail={d.status === "failed"}
              class:tint-limit={d.status === "quotaExceeded" || d.status === "needsAccount"}
              role="row"
              id={"drow-" + d.id}
              aria-selected={app.selected.has(d.id)}
              aria-label="{d.name}, {look.label}"
              tabindex="-1"
              class:drop-before={dropTarget === d.id}
              data-key={d.id}
              draggable={canReorder && d.status !== "completed"}
              onpointerdown={(e) => d.status === "completed" && outDown(e, outIds(d), false)}
              title={d.status === "completed" ? t("Drag to put the file in a folder or another app") : undefined}
              ondragstart={(e) => dragStart(e, d)}
              ondragover={(e) => dragOver(e, d)}
              ondragleave={() => dropTarget === d.id && (dropTarget = null)}
              ondrop={(e) => drop(e, d)}
              ondragend={() => (dropTarget = null)}
              onclick={(e) => rowClick(e, d)}
              ondblclick={() => rowDblClick(d)}
              oncontextmenu={(e) => rowContext(e, d)}
              onkeydown={() => {}}
            >
              <div class="cell">
                <button
                  class="check"
                  class:on={app.selected.has(d.id)}
                  role="checkbox"
                  aria-checked={app.selected.has(d.id)}
                  aria-label={t("Select {name}", { name: d.name })}
                  onclick={(e) => {
                    e.stopPropagation();
                    app.toggle(d.id);
                    app.anchor = d.id;
                    app.navAnchor = d.id;
                  }}
                >
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
                </button>
              </div>
              <div class="cell type">
                <span class="stb" style="background: {look.color}"></span>
                <FileIcon name={d.name} size={18} faded={d.status === "paused"} />
                {#if extension(d.name)}<span class="ext mono">{extension(d.name)}</span>{/if}
              </div>
              <div class="cell name">
                {#if r.kind === "child"}<span class="rail" class:end={r.last}></span>{/if}
                <MiddleText text={d.name} class="nm" />
                {#if d.viaBrowser}
                  <span class="vb" title={t("This site only lets your browser download this file: the browser downloads it (no multi-part speed) and Pako follows it.")}>{t("Via browser")}</span>
                {/if}
                {#if hostSwitch(d)}
                  <span class="via" title={hostSwitch(d)}>{t("from {host}", { host: hostName(d) })}</span>
                {/if}
                {#if d.priority === "first"}
                  <span class="prio first" title={t("Priority: First (starts before the others)")}><Icon name="up" size={11} stroke={2.6} /></span>
                {:else if d.priority === "later"}
                  <span class="prio later" title={t("Priority: Later (runs when nothing else waits)")}><Icon name="down" size={11} stroke={2.6} /></span>
                {/if}
                {#if d.status !== "completed"}
                  {@const level = alertOf(d, doneAlert(app.settings))}
                  <!-- How it tells you when it finishes: each click goes one step on. Pop-up (the
                       usual) shows only while pointed at; Off and Important always show. -->
                  <button
                    class="bell {level}"
                    title={t("Notify when done: {level} (click to change)", { level: alertLabel(level) })}
                    aria-label={t("Notify when done: {level}", { level: alertLabel(level) })}
                    tabindex="-1"
                    draggable="false"
                    onpointerdown={(e) => e.stopPropagation()}
                    ondragstart={(e) => (e.preventDefault(), e.stopPropagation())}
                    ondblclick={(e) => e.stopPropagation()}
                    onclick={(e) => {
                      e.stopPropagation();
                      app.run(() => api.setAlert([d.id], nextAlert(level)));
                    }}
                  >
                    <Icon name={level === "off" ? "bell-off" : "bell"} size={level === "important" ? 13 : 12} stroke={2.2} />
                  </button>
                {/if}
              </div>
              <div class="cell src c-source">
                <span class="dot" style="background: {sourceOf(d).color}"></span>
                <span class="ell" title={sourceOf(d).label}>{sourceOf(d).label}</span>
              </div>
              <div class="cell prog c-progress">
                <div class="barline">
                  <div class="bar"><div class="fill" class:pulse={d.status === "verifying"} style="width: {pct(d)}%; background: {look.color}"></div></div>
                  <span class="pct mono">{d.size || d.status === "completed" ? t("{pct}%", { pct: Math.floor(pct(d)) }) : ""}</span>
                </div>
                <span class="sub mono">{sizeText(d)}</span>
              </div>
              <div class="cell spd mono c-speed">
                <span>{speedLine(d)}</span>
                {#if sub}<span class="sub">{sub}</span>{/if}
              </div>
              <div class="cell status c-status" style="color: {look.text}" title={tMaybe(d.message ?? "")}>
                <span class="ell">{look.label}{d.status === "completed" && d.md5 && app.settings?.verifyChecksum ? ` · ${t("checked")}` : ""}</span>
                {#if isActive(d.status) && d.status === "downloading"}<span class="live"></span>{/if}
                {#if d.status === "completed" && d.private}
                  <!-- A private window's finished download: off the list now (the file stays). -->
                  <button
                    class="icon-btn forget"
                    aria-label={t("Remove {name} from the list", { name: d.name })}
                    title={t("Remove from the list (the file stays)")}
                    onpointerdown={(e) => e.stopPropagation()}
                    ondblclick={(e) => e.stopPropagation()}
                    onclick={(e) => {
                      e.stopPropagation();
                      app.run(async () => {
                        await api.forgetPrivate(d.id);
                        await app.refresh();
                      });
                    }}><Icon name="x" size={12} /></button
                  >
                {/if}
              </div>
              <div class="cell date mono c-date" title={dateTitle(d.date || d.addedAt, d.finishedAt)}>{shortDate(d.date || d.addedAt)}</div>
            </div>
            {#if d.id === priv.tipRow}
              <!-- Once, under the first "Done · private" row: Private mode keeps these behind a PIN. -->
              <div class="tip" role="note" transition:rowSlide={{ duration: 240 }} {@attach () => priv.tipShown()}>
                <span class="tip-in">
                  <Icon name="lock" size={13} />
                  <span class="tip-t" title={t("Want to keep these behind a PIN? Private mode, in Settings.")}>{pinTip[0]}<b>{t("Private mode")}</b>{pinTip[1]}</span>
                  <button class="btn small tip-go" onclick={() => { priv.closeTip(); priv.setupOpen = true; }}>{t("Turn on…")}</button>
                  <button class="icon-btn" aria-label={t("Close this tip")} title={t("Close")} onclick={() => priv.closeTip()}><Icon name="x" size={12} /></button>
                </span>
              </div>
            {/if}
          {/if}
        </div>
      {/each}
      {#if isVirtual}<div class="vpad" style="height: {win.padBottom}px"></div>{/if}
    {/if}
  </div>
</div>

<style>
  .table {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container-type: inline-size;
  }
  /* The columns as you set them (columns.ts): a narrow window drops the least important first
     (the source, then the date, then the speed, then the status) instead of squeezing the
     name, and never shows one you hid. */
  .head,
  .row {
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    padding: 0 16px;
  }
  .table.h-source .c-source,
  .table.h-progress .c-progress,
  .table.h-speed .c-speed,
  .table.h-status .c-status,
  .table.h-date .c-date {
    /* Not shown: its place in the grid is at no width (see gridTemplate), nothing in it shows. */
    visibility: hidden;
    /* No sliver: its own padding or margin would still take room at no width. */
    padding: 0 !important;
    margin: 0 !important;
    border: 0 !important;
    overflow: hidden;
    pointer-events: none;
  }
  /* Shown or hidden from the columns menu: the columns ease to their new widths. */
  .table.easing .head {
    transition: grid-template-columns var(--mid) var(--ease);
  }
  .table.easing .row {
    transition:
      grid-template-columns var(--mid) var(--ease),
      background var(--fast) ease,
      --zb var(--mid) ease;
  }
  /* Meanwhile a column opening up keeps its content inside it (the titles keep their edges). */
  .table.easing .row > * {
    overflow: hidden;
  }
  .colcell {
    justify-content: flex-end;
  }
  .head {
    height: 32px;
    flex: none;
    /* The list's scrollbar takes room from the rows: the titles keep the same room free, so
       each title stays right above its column whether the list scrolls or not. */
    overflow: hidden;
    scrollbar-gutter: stable;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-3);
    background: var(--head);
    border-bottom: 1px solid var(--line);
  }
  .head .cell {
    position: relative;
    height: 100%;
    display: flex;
    align-items: center;
  }
  .body {
    /* Long lists draw only the rows on screen: the browser must not shift the scroll
       position when the space above them changes. */
    overflow-anchor: none;
    scrollbar-gutter: stable;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding-bottom: 12px;
  }
  .slot {
    will-change: transform;
  }
  .vpad {
    flex: none;
  }
  .row {
    height: 46px;
    border-bottom: 1px solid var(--line-soft);
    transition:
      background var(--fast) ease,
      --zb var(--mid) ease;
  }
  .row:hover {
    background: var(--row-hover);
  }
  .row.selected {
    background: var(--select);
  }
  .row.focused {
    box-shadow: inset 2px 0 0 var(--accent);
  }
  /* Dragging a row: a line shows where it will land. */
  .row.drop-before {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .sorter {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 100%;
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    text-align: left;
    cursor: pointer;
    transition: color var(--fast) ease;
  }
  .sorter:hover,
  .sorter.sorted {
    color: var(--text);
  }
  .row.tint-fail:not(.selected) {
    background: var(--tint-fail);
  }
  .row.tint-limit:not(.selected) {
    background: var(--tint-limit);
  }

  /* Folder group: header + files in one bordered, inset block */
  .row.folder,
  .row.child {
    margin: 0 8px;
    padding: 0 7px;
    border-left: 1px solid var(--line-strong);
    border-right: 1px solid var(--line-strong);
  }
  .row.folder {
    margin-top: 8px;
    border-top: 1px solid var(--line-strong);
    border-radius: var(--radius) var(--radius) 0 0;
    background: var(--group-head);
    cursor: pointer;
  }
  .row.folder:not(.open) {
    border-bottom: 1px solid var(--line-strong);
    border-radius: var(--radius);
  }
  /* Open: a clear line between the folder and its files. */
  .row.folder.open {
    border-bottom: 1px solid var(--line-strong);
  }
  .row.folder:hover {
    background: var(--group-head-hover);
  }
  .row.folder.selected {
    background: var(--group-head-hover);
  }
  /* The gap under a folder block lives in its slot (padding, not margin): it is part of the
     slide when a folder opens or closes, so the rows below don't jump. */
  .slot {
    transition: padding-bottom 220ms var(--ease-out);
  }
  .slot.gap {
    padding-bottom: 8px;
  }
  .row.child {
    background: var(--group-body);
  }
  .row.child:hover {
    background: var(--child-hover);
  }
  .row.child.selected {
    background: var(--child-select);
  }
  .row.child.last {
    border-bottom: 1px solid var(--line-strong);
    border-radius: 0 0 var(--radius) var(--radius);
  }

  .cell {
    min-width: 0;
    display: flex;
    align-items: center;
  }
  .type {
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 2px;
    transition: color var(--mid) ease;
  }
  /* Status at a glance: a thin bar in the status colour before the icon. */
  .type {
    position: relative;
    padding-left: 9px;
  }
  .stb {
    position: absolute;
    left: 0;
    top: 50%;
    width: 3px;
    height: 26px;
    margin-top: -13px;
    border-radius: 2px;
    transition: background var(--mid) ease;
  }
  .ext {
    color: var(--text-3);
    font-size: 8.5px;
    line-height: 1;
    letter-spacing: 0.02em;
    max-width: 40px;
    overflow: hidden;
  }
  .name {
    gap: 8px;
    padding-right: 16px;
    align-self: stretch;
  }
  .name :global(.nm) {
    font-weight: 450;
  }
  .prio {
    flex: none;
    width: 18px;
    height: 18px;
    border-radius: var(--radius-sm);
    display: grid;
    place-items: center;
  }
  /* Continuing from a backup host (the first one failed): a quiet note, never red. */
  /* "Via browser": small, plain, never squeezes the name (its whole word or nothing). */
  .vb {
    flex: none;
    height: 18px;
    line-height: 18px;
    padding: 0 7px;
    border-radius: var(--radius-sm);
    box-shadow: inset 0 0 0 1px var(--line-strong);
    color: var(--text-2);
    font-size: 11px;
    font-weight: 550;
    white-space: nowrap;
  }
  .via {
    flex: 0 1000 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    height: 18px;
    line-height: 18px;
    padding: 0 7px;
    border-radius: var(--radius-sm);
    background: var(--active);
    color: var(--text-2);
    font-size: 11px;
    font-weight: 500;
  }
  /* "First" is a choice, not a problem: bright grey, never red. */
  .prio.first {
    color: var(--text);
    background: var(--line-strong);
  }
  .prio.later {
    color: var(--text-3);
    background: var(--raise);
  }
  /* The row's bell (Notify when done). Always the same size, so the name never moves. */
  .bell {
    flex: none;
    width: 20px;
    height: 20px;
    margin-left: auto;
    padding: 0;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-4);
    cursor: pointer;
    transition:
      opacity var(--fast) ease,
      color var(--fast) ease,
      background var(--fast) ease;
  }
  .bell:hover {
    color: var(--text);
    background: var(--raise-hover);
  }
  .bell:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  /* Pop-up is the usual: only while the row is pointed at. (No Tab stop on every row: the
     keyboard reaches the same choice in the details and the right-click menu.) */
  .bell.pop {
    opacity: 0;
  }
  .row:hover .bell.pop {
    opacity: 1;
  }
  /* Important: filled, in Pako's colour, always there. */
  .bell.important {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .bell.important :global(path) {
    fill: currentColor;
  }
  .bell.important:hover {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .rail {
    position: relative;
    width: 14px;
    flex: none;
    align-self: stretch;
    margin-left: 4px;
    border-left: 1px solid var(--rail);
  }
  .rail::after {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    width: 11px;
    border-top: 1px solid var(--rail);
  }
  .rail.end {
    align-self: flex-start;
    height: 50%;
  }
  .rail.end::after {
    top: 100%;
  }
  .chev {
    width: 24px;
    height: 24px;
    flex: none;
    display: grid;
    place-items: center;
    padding: 0;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius);
    background: var(--active);
    color: var(--text);
    transition:
      background var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .chev:hover {
    background: var(--active-hover);
  }
  .chev:active {
    transform: scale(0.97);
  }
  .chev :global(svg) {
    transform: rotate(-90deg);
    transition: transform var(--mid) var(--ease);
  }
  .folder.open .chev :global(svg) {
    transform: rotate(0deg);
  }
  .fname {
    flex: 0 1 auto;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 56px;
  }
  .chip {
    /* shrinks before the folder name does */
    flex: 0 1000 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-2);
    background: var(--active);
    border-radius: var(--radius-sm);
    padding: 1px 7px;
    white-space: nowrap;
  }
  .src {
    gap: 8px;
    padding-right: 12px;
    color: var(--text-2);
    font-size: 12px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }
  .ell {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .prog {
    flex-direction: column;
    align-items: stretch;
    gap: 5px;
    padding-right: 20px;
  }
  .barline {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .bar {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 2px;
    transition:
      width 450ms var(--ease-out),
      background var(--mid) ease;
  }
  .fill.pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }
  .pct {
    width: 34px;
    text-align: right;
    font-size: 11px;
    color: var(--text-2);
  }
  .sub {
    font-size: 10.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spd {
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    font-size: 12px;
  }
  .date {
    font-size: 11px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status {
    gap: 8px;
    font-size: 12px;
    transition: color var(--mid) ease;
  }
  /* The one-time Private mode tip under the first "Done · private" row. */
  .tip {
    height: 40px;
    overflow: hidden;
  }
  .tip-in {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    margin: 0 8px 6px 44px;
    padding: 0 6px 0 12px;
    border-radius: var(--radius);
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
    color: var(--priv-text);
    font-size: 12.5px;
  }
  .tip-t {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  /* The ✕ of a "Done · private" row, right after its status. */
  .icon-btn.forget {
    width: 24px;
    height: 24px;
    flex: none;
    margin: -4px 0 -4px -4px;
    color: var(--text-3);
  }
  .icon-btn.forget:hover {
    color: var(--text);
  }
  .live {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--st-run);
    flex: none;
    animation: breathe 1.6s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      opacity: 0.25;
      transform: scale(0.7);
    }
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--text-3);
    padding-bottom: 48px;
    animation: rise var(--slow) var(--ease);
  }
  .empty.small {
    height: auto;
    padding: 48px 0;
  }
  .empty h3 {
    margin: 10px 0 0;
    color: var(--text);
    font-size: 16px;
    font-weight: 600;
  }
  .empty p {
    margin: 0 0 10px;
  }
  .empty .hint {
    margin-top: 12px;
    font-size: 12px;
  }
  .empty-icon {
    width: 52px;
    height: 52px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    background: var(--accent-soft);
    color: var(--accent);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  /* Every other row a touch lighter, so rows are easy to follow across a long list: list rows
     by their place in the list (a folder counts as one), a folder's files by their place inside
     it (see `alt`). The stripe lies over the row's own colour (folder files, red/purple tints,
     hover), so those still read; a selected row shows its selection alone. Every file row
     carries the layer (transparent when not striped), so a row whose place changes eases. */
  .table .row:not(.selected):not(.folder) {
    background-image: linear-gradient(var(--zb), var(--zb));
  }
  .row.alt {
    --zb: var(--zebra);
  }
</style>
