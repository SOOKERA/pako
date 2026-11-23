<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The Links tab (called Found in the code): links waiting to be picked. Nothing here downloads until you press
  // Download (or Download first). Switching tabs never interrupts anything.
  import { untrack } from "svelte";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import { app, hostLook, type FoundMirror, type SortKey } from "./state.svelte";
  import { api, canDownload, type Download, type FoundItem, type Priority } from "./api";
  import Modal from "./Modal.svelte";
  import { VIRTUAL_MIN, layout, reveal, visible } from "./virtual";
  import type { MenuItem } from "./menu";
  import { ALERTS, alertLabel, alertMenu, alertOf, doneAlert, nextAlert, sharedAlert, type Alert } from "./alert";
  import SegPick from "./SegPick.svelte";
  import { bytes, dateTitle, extension, shortDate, sourceOf } from "./format";
  import Icon from "./Icon.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import { choices, targets } from "./devices.svelte";
  import { downloadFoundOn, downloadOnMenu, downloadSplit, sending } from "./downloadOn.svelte";
  import { pop } from "./motion";
  import { linkLook, linkTip, type LinkLook, type Say } from "./linkState";
  import { around, t, tMaybe, type Key } from "./i18n.svelte";

  import { priv } from "./private.svelte";
  import ColumnGrip from "./ColumnGrip.svelte";
  import ColumnsButton from "./ColumnsButton.svelte";
  import { listColumns } from "./columns.svelte";
  import { LISTS, gridTemplate, shownColumns } from "./columns";

  let { onadd, onmenu }: { onadd: () => void; onmenu: (x: number, y: number, items: MenuItem[]) => void } = $props();

  // ---------- columns: widths, shown or hidden (columns.ts) ----------
  const LIST = LISTS.found;
  /** The column titles' width: the list's room, without the scrollbar's. */
  let headW = $state(0);
  const colState = $derived(listColumns.state("found"));
  const frozen = $derived(listColumns.frozen?.list === "found" ? listColumns.frozen : null);
  const shown = $derived(frozen ? { keys: frozen.keys, tight: false } : shownColumns(LIST, colState, headW));
  const colsStyle = $derived(`--cols: ${gridTemplate(LIST, colState, shown.keys, shown.tight)}`);
  const hideCls = $derived(
    LIST.cols
      .filter((c) => !shown.keys.includes(c.key))
      .map((c) => `h-${c.key}`)
      .join(" "),
  );
  const noRoom = $derived(LIST.cols.filter((c) => !colState.hidden.includes(c.key) && !shown.keys.includes(c.key)).map((c) => c.key));
  const unsort = (key: string) => app.unsortIf("found", key);
  /** The link marks' words (linkState.ts) in the language shown. */
  const say: Say = (key, params) => t(key as Key, params);
  /** "Sort by size", and what screen readers hear ("Size, sorted lowest first. Sort by size"). */
  function sortWords(label: string, key: string, az: boolean) {
    const sort = t("Sort by {column}", { column: label.toLowerCase() });
    const s = app.sortFound;
    if (s?.key !== key) return { title: sort, aria: `${label}. ${sort}` };
    const k: Key = az ? (s.dir > 0 ? "{column}, sorted A to Z. {sort}" : "{column}, sorted Z to A. {sort}") : s.dir > 0 ? "{column}, sorted lowest first. {sort}" : "{column}, sorted highest first. {sort}";
    return { title: sort, aria: t(k, { column: label, sort }) };
  }
  const nameSort = $derived(sortWords(t("Name"), "name", true));
  const tipKeys = $derived(around(t("Tip: press {keys} anywhere to paste links."), "{keys}"));
  function columnsMenu(e: MouseEvent) {
    e.preventDefault();
    onmenu(e.clientX, e.clientY, listColumns.menu("found", unsort, () => noRoom));
  }

  const n = $derived(app.picked.length);
  /** Ticked rows as shown (a file on several hosts once; Remove takes all its hosts). */
  const tickedRows = $derived(app.oncePerFile(app.ticked).length);
  /** Spots free right now under the "At once" limit. */
  const free = $derived(Math.max(0, (app.settings?.maxActive ?? 3) - app.runningCount));
  /** "Download" unless downloads are already running and these would have to wait behind them
   *  ("Add to queue"). With nothing running it's always "Download", even for more files than fit
   *  at once: the first ones start and the rest follow by themselves. */
  const startsNow = $derived(app.runningCount === 0 || n <= free);
  const dest = $derived(app.foundDest ?? app.settings?.downloadDir ?? "");
  const destName = $derived(dest.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || dest);
  let busy = $state(false);
  // "Download on ▾": shown only while one of your devices can take links.
  targets.start();
  // The per-link "Download on" (a row's PC marker, its name in the tooltip).
  choices.start();
  const deviceName = (id: string) => choices.list.find((c) => c.id === id)?.name ?? t("another PC");

  /** "Download on ▾" in the toolbar: This PC (the usual Download) or one of your devices. */
  function onMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    // "This PC" here is for this action only: every picked link downloads here, whatever its
    // own "Download on" says (and a device picked here overrides it the same way).
    onmenu(r.left, r.bottom + 6, downloadOnMenu(targets.list, () => download(false, null, app.picked, false, true), (t) => downloadFoundOn(t, app.picked), true));
  }

  async function chooseDest() {
    const picked = await pickFolder({ directory: true, defaultPath: dest || undefined });
    if (typeof picked === "string") app.foundDest = picked;
  }

  /** Picked links that are already downloaded: in the downloads list, or the file is in the folder. */
  let dupAsk = $state<{ ids: string[]; dups: FoundItem[]; first: boolean; alert: Alert | null; privately: boolean } | null>(null);

  function sameFile(f: FoundItem, d: Download) {
    const a = f.template.source;
    const b = d.source;
    if (a.kind === "gDrive" && b.kind === "gDrive") return a.fileId === b.fileId;
    return d.link === f.template.link && d.name === f.template.name;
  }

  /** `here`: download on this PC whatever each link's own "Download on" says (the menu's
   *  "Download on this PC"). Else links set to another PC go there (`downloadSplit`). */
  /** `alert`: Off · Pop-up · Important for all of them (null = each row's own).
   *  `privately`: "Download privately", straight into Private mode (on this PC). */
  async function download(first: boolean, alert: Alert | null = null, items: FoundItem[] = app.picked, checked = false, here = false, privately = false): Promise<void> {
    if (!here && !privately && !checked && items.some((f) => f.device)) {
      if (busy) return;
      return downloadSplit(items, first, (rows): Promise<void> => download(first, alert, rows, false, true));
    }
    const ids = items.filter(canDownload).map((f) => f.id);
    if (!ids.length || busy) return;
    // Busy from the first click: a quick second one doesn't run the check (and the download) twice.
    busy = true;
    try {
      if (!checked) {
        const onDisk = new Set(await api.foundExisting(ids, app.foundDest).catch(() => [] as string[]));
        const want = new Set(ids);
        const dups = app.found.filter((f) => want.has(f.id) && (onDisk.has(f.id) || app.downloads.some((d) => sameFile(f, d))));
        if (dups.length) {
          dupAsk = { ids, dups, first, alert, privately };
          return;
        }
      }
      const count = await api.foundDownload(ids, first, app.foundDest, alert, privately);
      await app.refresh();
      if (privately) await priv.refreshList();
      if (count) app.toast(privately ? t("{n} files downloading in Private mode", { n: count }) : t("{n} files moved to Downloads", { n: count }));
      // Nothing left to pick: show the downloads running.
      if (!app.found.length) app.view = "downloads";
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      busy = false;
    }
  }

  function remove() {
    const ids = app.ticked.map((f) => f.id);
    if (ids.length) app.removeFound(ids);
  }

  /** Dead links out of the way, whatever the filter. */
  function clearBroken() {
    const ids = app.found.filter((f) => f.state === "broken").map((f) => f.id);
    if (ids.length) app.removeFound(ids, t("{n} broken links removed", { n: ids.length }));
  }

  /** The arrow next to the main button: how the picked links tell you when they finish (the
   *  same three-way as beside it, for when the window is too narrow to show it), and "Download
   *  privately" (Private mode on): straight into Private mode, encrypted, never in Downloads. */
  function moreMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const items: MenuItem[] = alertMenu(pickedAlert, setPickedAlert);
    if (priv.status.on) items.push({ separator: true }, { label: t("Download privately"), icon: "lock", hint: t("Private mode"), action: () => download(false, null, app.picked, false, true, true) });
    onmenu(r.right - 230, r.bottom + 6, items);
  }

  /** How the picked links tell you when they finish (Off · Pop-up · Important), set on them
   *  before they download; null when they differ. */
  const pickedAlert = $derived(sharedAlert(app.picked.map((f) => alertOf(f.template, doneAlert(app.settings)))));
  function setPickedAlert(a: Alert) {
    const ids = app.picked.map((f) => f.id);
    if (!ids.length) return;
    app.run(async () => {
      await api.foundSet(ids, { alert: a });
      await app.refresh();
    });
  }

  /** Right-click on a row: the ticked rows if it's one of them, else just this one. */
  function rowMenu(e: MouseEvent, row: FoundItem[]) {
    e.preventDefault();
    const inTicked = row.every((f) => app.isTicked(f)) && app.ticked.length > row.length;
    const items = inTicked ? app.ticked : row;
    const ids = items.map((f) => f.id);
    // A file on several hosts downloads once.
    const able = app.oncePerFile(items.filter(canDownload));
    const count = items.length > 1 ? items.length : 0;
    const all = (p: Priority) => items.every((f) => f.template.priority === p);
    const set = (o: Parameters<typeof api.foundSet>[1]) =>
      app.run(async () => {
        await api.foundSet(ids, o);
        await app.refresh();
      });
    const menu: MenuItem[] = [
      { label: able.length > 1 ? t("Download {n}", { n: able.length }) : t("Download"), icon: "download", disabled: !able.length, action: () => download(false, null, able) },
      { label: t("Download first"), icon: "up", disabled: !able.length, action: () => download(true, null, able) },
      { label: t("Download as Important"), icon: "bell", disabled: !able.length, action: () => download(false, "important", able) },
      ...(priv.status.on ? [{ label: t("Download privately"), icon: "lock", hint: t("Private mode"), disabled: !able.length, action: () => download(false, null, able, false, true, true) }] : []),
      ...(targets.list.length && able.length ? [{ separator: true }, ...downloadOnMenu(targets.list, null, (t) => downloadFoundOn(t, able), true)] : []),
      { separator: true },
      ...alertMenu(sharedAlert(items.map((f) => alertOf(f.template, doneAlert(app.settings)))), (a) => set({ alert: a })),
      { label: t("Priority: First"), icon: "up", hint: all("first") ? "✓" : "", action: () => set({ priority: "first" }) },
      { label: t("Priority: Normal"), icon: "minus", hint: all("normal") ? "✓" : "", action: () => set({ priority: "normal" }) },
      { label: t("Priority: Later"), icon: "down", hint: all("later") ? "✓" : "", action: () => set({ priority: "later" }) },
      {
        label: t("Save to…"),
        icon: "folder",
        action: async () => {
          const picked = await pickFolder({ directory: true, defaultPath: items[0].dest ?? dest });
          if (typeof picked === "string") set({ dest: picked });
        },
      },
      { separator: true },
      // Right-clicked inside the ticked rows: also offer to remove just this one.
      ...(inTicked
        ? [
            {
              label: row.length > 1 ? t("Remove only this folder") : t("Remove only this one"),
              icon: "trash",
              danger: true,
              action: () => app.removeFound(row.map((f) => f.id)),
            },
          ]
        : []),
      {
        label: count ? t("Remove {n}", { n: count }) : t("Remove"),
        icon: "trash",
        danger: true,
        action: () => app.removeFound(ids),
      },
    ];
    onmenu(e.clientX, e.clientY, menu);
  }

  /** The host list of a file on several hosts (its row's "on N hosts" button): pick one. */
  function hostMenu(e: MouseEvent, m: FoundMirror) {
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const items: MenuItem[] = m.items.map((f) => {
      const h = hostLook(f);
      return {
        label: sourceOf(f.template).label,
        icon: m.chosen?.id === f.id ? "check" : f.template.source.kind === "gDrive" ? "drive" : "globe",
        hint: f.template.size != null && f.state === "ready" ? `${tMaybe(h.label)} · ${bytes(f.template.size)}` : tMaybe(h.label),
        disabled: !canDownload(f),
        action: () => app.pickHost([f.id]),
      };
    });
    onmenu(r.left, r.bottom + 6, items);
  }

  /** Click = focus (details panel). Ctrl+click also ticks/unticks the row, Shift+click ticks
   *  every row from the last clicked one to this one (the usual list behaviour). */
  let anchor: string | null = null;
  function rowClick(e: MouseEvent, key: string, items: FoundItem[]) {
    if (e.shiftKey && anchor) {
      const keys = app.foundRows.map((r) => (r.kind === "folder" ? "g:" + r.key : r.f.id));
      const a = keys.indexOf(anchor);
      const b = keys.indexOf(key);
      if (a >= 0 && b >= 0) {
        const [from, to] = a < b ? [a, b] : [b, a];
        const range = app.foundRows.slice(from, to + 1).flatMap((r) => (r.kind === "folder" ? r.items : [r.f]));
        app.setTicked(range, true);
      }
    } else if (e.ctrlKey || e.metaKey) {
      const on = items.every((f) => app.isTicked(f));
      app.setTicked(items, !on);
      anchor = key;
    } else {
      anchor = key;
    }
    // Shift+↑/↓ goes on from where the mouse put the anchor.
    app.navAnchor = anchor ?? key;
    app.foundFocus = key;
  }

  /** Rows shaded every other one (a folder counts as one row; its files alternate inside it). */
  const alt = $derived.by(() => {
    const out = new Set<string>();
    let top = 0;
    let inner = 0;
    for (const r of app.foundRows) {
      if (r.kind === "child") {
        if (inner++ % 2) out.add(r.f.id);
        continue;
      }
      inner = 0;
      if (r.kind === "file" && top % 2) out.add(r.f.id);
      top++;
    }
    return out;
  });

  // ---------- long lists: draw only what's on screen ----------
  let body: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewH = $state(800);
  /** Row heights as the CSS draws them: 46 px rows, 40 px files in a folder; 8 px above a
   *  folder, below a closed folder and below a folder's last file (margins: they collapse). */
  const geo = $derived(
    layout(
      app.foundRows.map((r) => {
        if (r.kind === "folder") return { h: 46, top: 8, bottom: r.open ? 0 : 8 };
        if (r.kind === "child") return { h: 40, top: 0, bottom: r.last ? 8 : 0 };
        return { h: 46, top: 0, bottom: 0 };
      }),
    ),
  );
  const isVirtual = $derived(app.foundRows.length > VIRTUAL_MIN);
  const win = $derived(isVirtual ? visible(geo, scrollTop, viewH) : { start: 0, end: app.foundRows.length, padTop: 0, padBottom: 0 });
  const shownRows = $derived(isVirtual ? app.foundRows.slice(win.start, win.end) : app.foundRows);
  // Only a new ask scrolls (rows and places read untracked: a row being checked doesn't jump
  // the list back to the last row asked for).
  $effect(() => {
    const ask = app.revealKey;
    const el = body;
    if (!ask || !el) return;
    untrack(() => {
      const i = app.foundRows.findIndex((r) => (r.kind === "folder" ? "g:" + r.key : r.f.id) === ask.key);
      if (i >= 0) reveal(el, geo, i);
    });
  });

  /** The header's tick box: everything shown ticked, some, or none (a file on several hosts
   *  counts once). */
  const allTick = $derived.by(() => {
    const n = app.foundShown.filter((r) => app.isTicked(r.f)).length;
    return n === 0 ? "none" : n === app.foundShown.length ? "all" : "some";
  });

  /** Right-click on empty space in the list. */
  function emptyMenu(e: MouseEvent) {
    if ((e.target as HTMLElement).closest(".row")) return;
    e.preventDefault();
    const items: MenuItem[] = [
      { label: t("Paste links"), icon: "link", hint: "Ctrl+V", action: () => pasteLinks() },
      { label: t("Add links…"), icon: "plus", action: onadd },
      { separator: true },
      { label: t("Tick all"), icon: "check", hint: "Ctrl+A", disabled: !app.foundVisible.length, action: () => app.setTicked(app.foundVisible, true) },
      { label: t("Untick all"), icon: "x", hint: t("Esc"), disabled: !app.ticked.length, action: () => app.setTicked(app.foundVisible, false) },
    ];
    if (app.foundCounts.broken) {
      items.push({ separator: true });
      items.push({ label: t("Clear broken links · {n}", { n: app.foundCounts.broken }), icon: "trash", action: clearBroken });
    }
    onmenu(e.clientX, e.clientY, items);
  }

  async function pasteLinks() {
    try {
      const text = await navigator.clipboard.readText();
      if (text.trim()) await app.addToFound(text);
      else onadd();
    } catch {
      onadd(); // the clipboard couldn't be read here: the Add links box takes a paste
    }
  }


  function folderTick(items: FoundItem[]) {
    const able = items.filter(canDownload);
    // Nothing in it can download: the box still ticks and unticks all of them (for Remove).
    const pool = able.length ? able : items;
    const on = pool.filter((f) => app.isTicked(f)).length;
    return { all: pool.length > 0 && on === pool.length, some: on > 0 && on < pool.length, on, total: items.length };
  }

  /** A mark's click: does what it says, once (a double-click doesn't open the page or the
   *  browser twice). */
  const markClicked = new Map<string, number>();
  function markAct(id: string, look: LinkLook) {
    const now = performance.now();
    if (now - (markClicked.get(id) ?? -1e9) < 800) return;
    markClicked.set(id, now);
    if (look.act === "tools") app.installVideoTools();
    else if (look.act === "page") app.run(() => api.gateFocus(id));
    else if (look.act === "browser") app.run(() => api.openInDrive(id));
  }
</script>

{#snippet markIcon(look: LinkLook)}
  {#if look.mark === "check"}<Icon name="check" size={14} stroke={2.6} />{:else if look.mark === "limit"}<span class="lmark"><Icon name="check" size={14} stroke={2.6} /><span class="ldot"></span></span>{:else if look.mark === "ring"}<span class="ring"></span>{:else if look.mark === "alert"}<Icon name="alert" size={14} />{:else if look.mark === "x"}<Icon name="x" size={14} stroke={2.6} />{:else if look.mark === "tools"}<Icon name="download" size={13} stroke={2.2} />{:else}<Icon name="open" size={13} stroke={2.2} />{/if}
{/snippet}

<div class="found {hideCls}" class:easing={listColumns.easing && app.foundRows.length <= 60} style={colsStyle}>
  <div class="toolbar">
    <h1>{t("Links")}</h1>
    <TutorialButton id="found" />
    {#if app.adding}
      <span class="adding"><span class="ring"></span>{t("Adding links…")}</span>
    {/if}
    {#if n}
      <div class="selchip"><span><b>{t("{n} picked", { n })}</b>{#if app.pickedSize}<span class="dim">{` · ${bytes(app.pickedSize)}`}</span>{/if}</span></div>
    {:else if app.found.length}
      <span class="dim small none-picked">{t("Nothing picked")}</span>
    {/if}
    <button class="icon-btn add" data-tour="found-add" title={t("Add links (or press Ctrl+V anywhere)")} aria-label={t("Add links")} onclick={onadd}><Icon name="plus" size={15} stroke={2.2} /></button>
    <div class="spacer"></div>
    <button class="btn dest" data-tour="found-dest" title={dest} onclick={chooseDest}>
      <Icon name="folder" size={14} /><span class="dim wide">{t("Save to")}</span><span class="dname">{destName}</span>
      <span class="dchev"><Icon name="chevron" size={11} stroke={2.4} /></span>
    </button>
    {#if app.foundCounts.broken}
      <button class="btn ghost clearb" title={t("Remove every link that doesn't work")} onclick={clearBroken}>
        <Icon name="trash" size={13} /><span class="wide">{t("Clear broken ·")}</span>{app.foundCounts.broken}
      </button>
    {/if}
    <button class="btn ghost danger" title={t("Remove the ticked links")} disabled={!app.ticked.length} onclick={remove}>
      <span class="narrow"><Icon name="trash" size={13} /></span><span class="mid">{t("Remove")}</span>{tickedRows ? ` ${tickedRows}` : ""}
    </button>
    {#if n && !startsNow}
      <button class="btn" disabled={busy} title={t("The downloads that started last step aside (nothing lost) and these start now")} onclick={() => download(true)}>
        <Icon name="up" size={13} stroke={2.4} /><span class="wide">{t("Download first")}</span>
      </button>
    {/if}
    {#if targets.list.length}
      <button class="btn on" data-tour="found-download-on" disabled={!n || busy || sending.busy} title={t("Download on this PC or on one of your devices")} aria-haspopup="menu" onclick={onMenu} transition:pop={{ duration: 180, from: 0.94, y: 0 }}>
        <Icon name="devices" size={14} /><span class="wide">{t("Download on")}</span><span class="ochev"><Icon name="chevron" size={11} stroke={2.4} /></span>
      </button>
    {/if}
    <div class="notify" data-tour="found-notify" title={t("When they finish: Off, Pop-up, or Important (a big pop-up in the middle of the screen)")}>
      <Icon name={pickedAlert === "off" ? "bell-off" : "bell"} size={14} />
      <SegPick small label={t("Notify when done")} options={ALERTS.map((a) => ({ value: a.id, label: a.label, hint: a.hint }))} value={n ? pickedAlert : null} onpick={setPickedAlert} />
    </div>
    <div class="split" data-tour="found-download">
      <button class="btn primary" disabled={!n || busy} title={startsNow ? t("Download") : t("Add to queue")} onclick={() => download(false)}>
        {#if startsNow}
          <Icon name="download" size={14} stroke={2.4} /><span class="mid">{t("Download")}</span>{n ? ` ${n}` : ""}
        {:else}
          <Icon name="queue" size={14} stroke={2.2} /><span class="mid">{t("Add to queue ·")}</span> {n}
        {/if}
      </button>
      <button class="btn primary more" disabled={!n || busy} aria-label={t("More download options")} title={priv.status.on ? t("Notify when done, or Download privately") : t("Notify when done")} onclick={moreMenu}>
        <Icon name="chevron" size={12} stroke={2.6} />
      </button>
    </div>
  </div>

  <div class="head" role="presentation" oncontextmenu={columnsMenu} bind:clientWidth={headW}>
    <div class="cell">
      <button
        class="check"
        class:on={allTick === "all"}
        class:some={allTick === "some"}
        role="checkbox"
        aria-checked={allTick === "some" ? "mixed" : allTick === "all"}
        disabled={!app.foundVisible.length}
        aria-label={allTick === "all" ? t("Untick all") : t("Tick all")}
        title={allTick === "all" ? t("Untick all (Esc)") : t("Tick all (Ctrl+A)")}
        onclick={() => app.setTicked(app.foundVisible, allTick !== "all")}
      >
        {#if allTick === "some"}
          <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 5h6" stroke="var(--accent-ink)" stroke-width="2" /></svg>
        {:else}
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
        {/if}
      </button>
    </div>
    <div class="cell">{t("Type")}</div>
    <button class="cell sorter" class:sorted={app.sortFound?.key === "name"} onclick={() => app.toggleSort("found", "name")} title={nameSort.title} aria-label={nameSort.aria}>
      {t("Name")}
      {#if app.sortFound?.key === "name"}<Icon name={app.sortFound.dir === 1 ? "up" : "down"} size={10} stroke={2.6} />{/if}
    </button>
    {#each LIST.cols as c (c.key)}
      {@const sw = sortWords(c.label, c.key, false)}
      <div class="cell hc c-{c.key}">
        <ColumnGrip list="found" col={c} keys={shown.keys} tight={shown.tight} avail={headW} />
        <button class="sorter" class:sorted={app.sortFound?.key === c.key} onclick={() => app.toggleSort("found", c.key as SortKey)} title={sw.title} aria-label={sw.aria}>
          {#if c.key === "link"}<Icon name="link" size={12} stroke={2.2} />{:else}{c.label}{/if}
          {#if app.sortFound?.key === c.key}<Icon name={app.sortFound.dir === 1 ? "up" : "down"} size={10} stroke={2.6} />{/if}
        </button>
      </div>
    {/each}
    <div class="cell colcell"><ColumnsButton list="found" {onmenu} onhide={unsort} noRoom={() => noRoom} /></div>
  </div>

  <div
    class="body"
    data-tour="found-list"
    role="grid"
    tabindex="-1"
    aria-label={t("Links")}
    aria-multiselectable="true"
    aria-activedescendant={app.foundFocus && !app.foundFocus.startsWith("g:") ? "frow-" + app.foundFocus : undefined}
    bind:this={body}
    bind:clientHeight={viewH}
    onscroll={(e) => (scrollTop = (e.currentTarget as HTMLElement).scrollTop)}
    oncontextmenu={emptyMenu}
  >
    {#if app.found.length === 0}
      <div class="empty">
        <div class="empty-icon"><Icon name="link" size={26} /></div>
        <h3>{t("No links yet")}</h3>
        <p>{t("Paste links here first: Pako checks them, then you pick what to download.")}</p>
        <button class="btn primary" onclick={onadd}><Icon name="plus" size={14} stroke={2.4} />{t("Add links")}</button>
        <p class="hint">{tipKeys[0]}<kbd>Ctrl</kbd> + <kbd>V</kbd>{tipKeys[1]}</p>
      </div>
    {:else if app.foundRows.length === 0}
      <div class="empty small"><p>{app.search ? t('Nothing here for "{search}".', { search: app.search }) : t("Nothing here.")}</p></div>
    {:else}
      {#if isVirtual}<div style="height: {win.padTop}px"></div>{/if}
      {#each shownRows as r (r.kind === "folder" ? "g:" + r.key : r.f.id)}
        {#if r.kind === "folder"}
          {@const tk = folderTick(r.items)}
          {@const size = r.items.reduce((s, f) => s + (f.template.size ?? 0), 0)}
          {@const newest = r.items.reduce((m, x) => Math.max(m, x.template.date || x.template.addedAt), 0)}
          <div
            class="row folder"
            class:open={r.open}
            class:focused={app.foundFocus === "g:" + r.key}
            role="row"
            tabindex="-1"
            data-key={"g:" + r.key}
            onclick={(e) => {
              rowClick(e, "g:" + r.key, r.items);
              // One click opens / closes it (Ctrl / Shift clicks only tick).
              // The second click of a double-click doesn't close it again.
                if (e.detail <= 1 && !(e.ctrlKey || e.shiftKey || e.metaKey)) app.toggleFoundFolder(r.key);
            }}
            oncontextmenu={(e) => rowMenu(e, app.withMirrors(r.items))}
            onkeydown={(e) => e.key === "Enter" && (app.foundFocus = "g:" + r.key)}
          >
            <div class="cell">
              <button
                class="check"
                class:on={tk.all}
                class:some={tk.some}
                role="checkbox"
                aria-checked={tk.some ? "mixed" : tk.all}
                aria-label={t("Pick all files in {name}", { name: r.name })}
                onclick={(e) => {
                  e.stopPropagation();
                  app.setTicked(app.withMirrors(r.items), !tk.all);
                }}
              >
                {#if tk.some}
                  <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 5h6" stroke="var(--accent-ink)" stroke-width="2" /></svg>
                {:else}
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
                {/if}
              </button>
            </div>
            <div class="cell type">
              <span class="stb" style="background: {r.set?.missing.length ? 'var(--st-fail)' : 'transparent'}"></span>
              <FileIcon kind={r.set ? "archive" : "folder"} size={18} />
              {#if r.set}<span class="ext mono">{t("PARTS")}</span>{/if}
            </div>
            <div class="cell name">
              <button
                class="chev"
                ondblclick={(e) => e.stopPropagation()}
                aria-label={r.open ? t("Collapse folder") : t("Expand folder")}
                onclick={(e) => {
                  e.stopPropagation();
                  app.toggleFoundFolder(r.key);
                }}
              >
                <Icon name="chevron" size={12} stroke={2.6} />
              </button>
              <span class="fname">{r.name}</span>
              {#if r.set}
                <span class="chip">{t("{n} parts", { n: tk.total })} · {t("{n} picked", { n: tk.on })}</span>
                {#if r.set.missing.length}<span class="chip warn" title={r.set.missing.join(", ")}>{r.set.missing[0]}{r.set.missing.length > 1 ? ` +${r.set.missing.length - 1}` : ""}</span>{/if}
              {:else}
                <span class="chip">{t("{n} files", { n: tk.total })} · {t("{n} picked", { n: tk.on })}</span>
              {/if}
            </div>
            <div class="cell src c-source"><span class="dot" style="background: {sourceOf(r.items[0].template).color}"></span><span class="ell">{sourceOf(r.items[0].template).label}</span></div>
            <div class="cell size c-size mono">{size ? bytes(size) : ""}</div>
            {#if r.set}
              {@const said = r.set.missing.length ? t("Incomplete set") : t("Split archive")}
              <div class="cell link c-link" class:ready={!r.set.missing.length} class:broken={r.set.missing.length > 0} title={said} role="img" aria-label={said}>
                {#if r.set.missing.length}<Icon name="alert" size={14} />{:else}<Icon name="check" size={14} stroke={2.6} />{/if}
              </div>
            {:else}
              <div class="cell link c-link ready" title={t("Opened")} role="img" aria-label={t("Opened")}><Icon name="check" size={14} stroke={2.6} /></div>
            {/if}
            <div class="cell date mono c-date" title={dateTitle(newest)}>{shortDate(newest)}</div>
          </div>
        {:else}
          {@const f = r.f}
          {@const m = r.mirror}
          {@const all = m ? m.items : [f]}
          {@const able = canDownload(f)}
          {@const look = linkLook({ state: f.state, message: f.message, video: f.template.source.kind === "video", deviceNote: f.deviceNote && able ? f.deviceNote : null, toolsBusy: app.videoToolsBusy }, say)}
          {@const on = app.isTicked(f)}
          <div
            class="row"
            class:alt={alt.has(f.id)}
            class:child={r.kind === "child"}
            class:last={r.kind === "child" && r.last}
            class:off={!on && able}
            class:tint-fail={f.state === "broken"}
            class:tint-limit={f.state === "needsAccount"}
            class:focused={app.foundFocus === f.id}
            data-key={f.id}
            id={"frow-" + f.id}
            aria-selected={on}
            aria-label="{f.template.name}, {look.word}"
            role="row"
            tabindex="-1"
            onclick={(e) => rowClick(e, f.id, all)}
            oncontextmenu={(e) => rowMenu(e, all)}
            onkeydown={(e) => e.key === "Enter" && (app.foundFocus = f.id)}
          >
            <div class="cell">
              <button
                class="check"
                class:on
                role="checkbox"
                aria-checked={on}
                aria-label={on ? t("Unpick {name}", { name: f.template.name }) : t("Pick {name}", { name: f.template.name })}
                onclick={(e) => {
                  e.stopPropagation();
                  app.setTicked(all, !on);
                }}
              >
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
              </button>
            </div>
            <div class="cell type">
              <span
                class="stb"
                style="background: {f.state === 'broken' ? 'var(--st-fail)' : f.state === 'needsAccount' ? 'var(--st-limit)' : f.state === 'checking' ? 'var(--st-check)' : 'transparent'}"
              ></span>
              <FileIcon name={f.template.name} kind={f.isFolder ? "folder" : undefined} size={18} />
              {#if !f.isFolder && extension(f.template.name)}<span class="ext mono">{extension(f.template.name)}</span>{/if}
            </div>
            <div class="cell name">
              {#if r.kind === "child"}<span class="rail" class:end={r.last}></span>{/if}
              <MiddleText text={f.template.name} class="nm" />
              {#if able || f.state === "checking"}
                {@const level = alertOf(f.template, doneAlert(app.settings))}
                <!-- How it tells you when it finishes (as on Downloads' rows): each click goes one
                     step on. Pop-up (the usual) shows only while pointed at. -->
                <button
                  class="bell {level}"
                  title={t("Notify when done: {level} (click to change)", { level: alertLabel(level) })}
                  aria-label={t("Notify when done: {level}", { level: alertLabel(level) })}
                  tabindex="-1"
                  onpointerdown={(e) => e.stopPropagation()}
                  ondblclick={(e) => e.stopPropagation()}
                  onclick={(e) => {
                    e.stopPropagation();
                    app.run(async () => {
                      await api.foundSet([f.id], { alert: nextAlert(level) });
                      await app.refresh();
                    });
                  }}
                >
                  <Icon name={level === "off" ? "bell-off" : "bell"} size={level === "important" ? 13 : 12} stroke={2.2} />
                </button>
              {/if}
              {#if f.template.private}
                <span
                  class="devmark"
                  title={t("From a private window: kept only until you quit Pako, never sent to another PC, and a private download once downloaded")}
                  aria-label={t("Private")}
                ><Icon name="lock" size={11} stroke={2.2} /><span class="devname">{t("private")}</span></span>
              {/if}
              {#if f.device || f.deviceNote}
                <span
                  class="devmark"
                  class:warn={!!f.deviceNote}
                  title={f.device ? `${t("Downloads on {device}", { device: deviceName(f.device) })}${f.deviceNote ? ` · ${f.deviceNote}` : ""}` : (f.deviceNote ?? "")}
                  aria-label={f.device ? t("Downloads on {device}", { device: deviceName(f.device) }) : t("Back to this PC")}
                  transition:pop={{ duration: 160, from: 0.8, y: 0 }}
                ><Icon name="devices" size={12} stroke={2.1} />{#if f.device}<span class="devname">{deviceName(f.device)}</span>{/if}</span>
              {/if}
              {#if m}
                <button
                  class="hosts"
                  data-tour="found-hosts"
                  title={t("The same file on {n} hosts: pick the one to download from (the others are backups)", { n: m.items.length })}
                  aria-label={t("Pick the host · on {n} hosts", { n: m.items.length })}
                  aria-haspopup="menu"
                  ondblclick={(e) => e.stopPropagation()}
                  onclick={(e) => hostMenu(e, m)}
                ><span class="hlong">{t("on {n} hosts", { n: m.items.length })}</span><span class="hshort"><Icon name="globe" size={11} stroke={2.2} />{m.items.length}</span><Icon name="chevron" size={10} stroke={2.6} /></button>
              {/if}
            </div>
            <div class="cell src c-source"><span class="dot" style="background: {sourceOf(f.template).color}"></span><span class="ell" title={sourceOf(f.template).label}>{sourceOf(f.template).label}</span></div>
            <div class="cell size c-size mono" title={f.template.source.kind === "video" && f.template.size != null ? t("Estimated size for the chosen quality") : ""}>{f.template.size != null ? (f.template.source.kind === "video" ? "≈ " : "") + bytes(f.template.size) : f.state === "checking" || f.state === "opening" ? "…" : "—"}</div>
            <!-- The link's state as one small mark (its words in the tooltip and for screen readers,
                 from linkState.ts); a state you can act on is a button that does it (install the
                 tools, show the link page, open it in the browser). -->
            <div class="cell link c-link {look.kind}">
              {#key look.mark + look.kind}
                {#if look.act}
                  <button
                    class="sbtn"
                    class:quiet={look.quiet}
                    title={linkTip(look, say)}
                    aria-label={look.word}
                    in:pop={{ duration: 160, from: 0.6, y: 0 }}
                    onclick={(e) => {
                      e.stopPropagation();
                      markAct(f.id, look);
                    }}>{@render markIcon(look)}</button
                  >
                {:else}
                  <span class="smark" title={linkTip(look, say)} role="img" aria-label={look.word} in:pop={{ duration: 160, from: 0.6, y: 0 }}>{@render markIcon(look)}</span>
                {/if}
              {/key}
            </div>
            <div class="cell date mono c-date" title={dateTitle(f.template.date || f.template.addedAt)}>{shortDate(f.template.date || f.template.addedAt)}</div>
          </div>
        {/if}
      {/each}
      {#if isVirtual}<div style="height: {win.padBottom}px"></div>{/if}
    {/if}
  </div>
</div>

{#if dupAsk}
  {@const ask = dupAsk}
  {@const all = ask.dups.length === ask.ids.length}
  <Modal
    title={ask.dups.length === 1 ? t('"{name}" is already downloaded', { name: ask.dups[0].template.name }) : t("{n} of these are already downloaded", { n: ask.dups.length })}
    width={480}
    onclose={() => (dupAsk = null)}
  >
    <p class="dup-msg">
      {ask.dups.length === 1
        ? t("It's already in your downloads or in the folder. Downloading again keeps the old file and saves a second copy with a new name.")
        : t("They're already in your downloads or in the folder. Downloading again keeps the old files and saves a second copy with a new name.")}
    </p>
    {#if ask.dups.length > 1}
      <ul class="dup-list">
        {#each ask.dups.slice(0, 6) as f (f.id)}<li class="ell">{f.template.name}</li>{/each}
        {#if ask.dups.length > 6}<li class="more">{t("and {n} more", { n: ask.dups.length - 6 })}</li>{/if}
      </ul>
    {/if}
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (dupAsk = null)}>{t("Cancel")}</button>
      <button
        class="btn"
        onclick={() => {
          dupAsk = null;
          const askIds = new Set(ask.ids);
          const items = app.found.filter((f) => askIds.has(f.id));
          download(ask.first, ask.alert, items, true, false, ask.privately);
        }}>{t("Download again")}</button
      >
      {#if !all}
        <button
          class="btn primary"
          onclick={() => {
            dupAsk = null;
            const skip = new Set(ask.dups.map((f) => f.id));
            const askIds = new Set(ask.ids);
            const items = app.found.filter((f) => askIds.has(f.id) && !skip.has(f.id));
            download(ask.first, ask.alert, items, true, false, ask.privately);
          }}>{ask.dups.length === 1 ? t("Skip it") : t("Skip them")}</button
        >
      {/if}
    {/snippet}
  </Modal>
{/if}

<style>
  /* Notify when done for the picked links (as in the Download with Pako window). */
  .notify {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-3);
  }
  /* Less room: the three-way goes (the arrow next to Download offers it too). */
  @container (max-width: 1100px) {
    .notify {
      display: none;
    }
  }
  /* A row's bell (Notify when done), as on Downloads' rows. */
  .bell {
    flex: none;
    width: 20px;
    height: 20px;
    margin-left: 6px;
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
  .bell.pop {
    opacity: 0;
  }
  .row:hover .bell.pop {
    opacity: 1;
  }
  .bell.important {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .bell.important :global(path) {
    fill: currentColor;
  }
  @media (prefers-reduced-motion: reduce) {
    .bell {
      transition: none;
    }
  }
  .dup-msg {
    margin: 0;
    color: var(--text-2);
    line-height: 1.5;
  }
  .dup-list {
    margin: 12px 0 0;
    padding: 10px 12px;
    list-style: none;
    border-radius: var(--radius-lg);
    background: var(--bg);
    border: 1px solid var(--line);
    font-size: 12.5px;
    color: var(--text-2);
    display: grid;
    gap: 4px;
  }
  .dup-list .more {
    color: var(--text-3);
  }
  .found {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container-type: inline-size;
  }
  .toolbar {
    height: 52px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 16px;
    border-bottom: 1px solid var(--line);
  }
  /* The title and the folder name give way first; the split Download button never shrinks. */
  .toolbar > h1,
  .toolbar > .dest {
    min-width: 0;
  }
  .toolbar > .split {
    flex: none;
  }
  .narrow {
    display: none;
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .selchip {
    height: 24px;
    display: flex;
    align-items: center;
    padding: 0 9px;
    border-radius: var(--radius);
    background: var(--select);
    font-size: 12px;
    white-space: nowrap;
  }
  .dim {
    color: var(--text-3);
  }
  .small {
    font-size: 12px;
  }
  .spacer {
    flex: 1;
  }
  .dest {
    max-width: 260px;
    overflow: hidden;
    justify-content: flex-start;
  }
  .dchev {
    display: inline-flex;
  }
  /* "Download on ▾": its arrow stays even when the toolbar keeps only icons. */
  .ochev {
    display: inline-flex;
    color: var(--text-3);
  }
  .dname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
  }
  .btn.danger {
    color: var(--st-fail-text);
  }
  .split {
    display: flex;
  }
  .split .btn:first-child:not(:last-child) {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .split .more {
    width: 30px;
    padding: 0;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
    border-left: 1px solid rgba(0, 0, 0, 0.22);
  }
  /* Less room (details panel open, small window): secondary buttons keep their icon only.
     These come after the base rules above so they win. */
  @container (max-width: 980px) {
    .wide {
      display: none;
    }
    /* "Download on" keeps only its PC icon (its title and menu say the rest). */
    .toolbar .btn.on {
      width: 34px;
      padding: 0;
    }
    .ochev {
      display: none;
    }
    .dest {
      max-width: 160px;
    }
  }
  /* The main button already shows the count. */
  @container (max-width: 760px) {
    .selchip,
    .none-picked {
      display: none;
    }
  }
  /* French words are longer: the folder keeps only its icon a little sooner (its title says
     where), so the page title isn't cut. */
  @container (max-width: 800px) {
    :global(html[lang="fr"]) .toolbar .dname,
    :global(html[lang="fr"]) .toolbar .dchev {
      display: none;
    }
  }
  /* Smallest window with the details panel open: the Download button always stays in
     view, the count moves into it and the other buttons keep only their icon. */
  @container (max-width: 600px) {
    .toolbar {
      gap: 6px;
    }
    .dname,
    .dchev,
    .mid {
      display: none;
    }
    /* Two bins side by side can't be told apart: "Clear broken" stays in the right-click menu. */
    .toolbar .clearb {
      display: none;
    }
    .narrow {
      display: inline-flex;
    }
    .toolbar .btn:not(.more) {
      padding: 0 9px;
    }
  }
  @container (max-width: 480px) {
    h1 {
      display: none;
    }
  }

  /* The columns as you set them (columns.ts): a narrow window drops the least important first
     (the source, then the date, then the size) and never shows one you hid. */
  .head,
  .row {
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    padding: 0 16px;
  }
  .found.h-source .c-source,
  .found.h-size .c-size,
  .found.h-link .c-link,
  .found.h-date .c-date {
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
  .found.easing .head {
    transition: grid-template-columns var(--mid) var(--ease);
  }
  .found.easing .row {
    transition:
      grid-template-columns var(--mid) var(--ease),
      background var(--fast) ease,
      --zb var(--mid) ease;
  }
  /* Meanwhile a column opening up keeps its content inside it (the titles keep their edges). */
  .found.easing .row > * {
    overflow: hidden;
  }
  @container (max-width: 600px) {
    /* The Link column's mark already says "Incomplete set" (on hover and for screen readers). */
    .chip.warn {
      display: none;
    }
  }
  .head .hc {
    position: relative;
    height: 100%;
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
  .row {
    cursor: default;
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
  .row.focused {
    background: var(--select);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .row.off .name,
  .row.off .size {
    color: var(--text-3);
  }
  .row.tint-fail {
    background: var(--tint-fail);
  }
  .row.tint-limit {
    background: var(--tint-limit);
  }

  /* Drive folder: header + its files in one bordered block, files joined by a tree line */
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
  }
  .row.folder:hover {
    background: var(--group-head-hover);
  }
  .row.folder.open {
    border-bottom: 1px solid var(--line-strong);
  }
  .row.folder:not(.open) {
    border-bottom: 1px solid var(--line-strong);
    border-radius: var(--radius);
    margin-bottom: 8px;
  }
  .row.child {
    height: 40px;
    background: var(--group-body);
  }
  .row.child.last {
    margin-bottom: 8px;
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
    color: var(--text-2);
  }
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
  .child .name {
    color: var(--text-2);
  }
  .rail {
    position: relative;
    width: 14px;
    flex: none;
    align-self: stretch;
    margin-left: 11px;
    border-left: 1.5px solid var(--rail);
  }
  .rail::after {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    width: 12px;
    border-top: 1.5px solid var(--rail);
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
  }
  .chip {
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
  .chip.warn {
    flex: 0 1 auto;
    color: var(--st-fail-text);
    background: var(--tint-fail);
  }
  /* "on 3 hosts": opens the host list. Sized like the chips, a button. */
  .hosts {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    margin-left: -2px;
    padding: 0 6px 0 7px;
    border: none;
    border-radius: var(--radius-sm);
    background: var(--active);
    color: var(--text-2);
    font: inherit;
    font-size: 11px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .hosts:hover {
    background: var(--active-hover);
    color: var(--text);
  }
  .hosts:active {
    transform: scale(0.97);
  }
  .hosts :global(svg) {
    opacity: 0.7;
  }
  .hshort {
    display: none;
    align-items: center;
    gap: 3px;
  }
  /* A link set to download on another PC: a small PC mark with its name. */
  .devmark {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    max-width: 140px;
    padding: 0 7px 0 6px;
    border-radius: var(--radius-sm);
    background: var(--active);
    color: var(--text-2);
    font-size: 11px;
    font-weight: 500;
    white-space: nowrap;
  }
  .devmark.warn {
    background: var(--tint-limit);
    color: var(--st-limit-text);
  }
  .devname {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Small window with the details panel open: the name keeps its room, the button shows the
     number of hosts only (its tooltip says the rest). */
  @container (max-width: 600px) {
    .hlong,
    .hosts > :global(svg) {
      display: none;
    }
    .hosts {
      padding: 0 6px;
    }
    .devname {
      display: none;
    }
    .devmark {
      padding: 0 5px;
    }
    .hshort {
      display: inline-flex;
    }
    /* A part of a split set: its hosts are picked in the set's details (Hosts). */
    .child .hosts {
      display: none;
    }
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
  .size {
    font-size: 12px;
    color: var(--text-2);
  }
  .date {
    font-size: 11px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .adding {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: var(--st-check-text);
  }
  .link {
    gap: 7px;
    font-size: 12.5px;
    justify-content: center;
  }
  /* The Link column's title is its icon, centred over the marks. */
  .head .c-link :global(.sorter) {
    width: 100%;
    justify-content: center;
  }
  /* "Ready · Google limit today": the ✓ with a small dot in the limit colour. */
  .lmark {
    position: relative;
    display: inline-flex;
  }
  .ldot {
    position: absolute;
    right: -3px;
    top: -2px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--st-limit);
    box-shadow: 0 0 0 1.5px var(--panel);
  }
  /* A state you can act on: a small button in the state's colour. */
  .sbtn {
    width: 26px;
    height: 22px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    border-radius: var(--radius);
    background: rgba(209, 123, 240, 0.16);
    color: inherit;
    transition:
      background var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .sbtn.quiet {
    background: transparent;
  }
  .sbtn:hover:not(:disabled) {
    background: rgba(209, 123, 240, 0.26);
  }
  .sbtn.quiet:hover:not(:disabled) {
    background: var(--btn-hover);
  }
  .sbtn:active:not(:disabled) {
    transform: scale(0.94);
  }
  .sbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  .smark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  /* The mark keeps its size when a long message has to be cut. */
  .link :global(svg) {
    flex: none;
  }
  .link.ready {
    color: var(--st-done-text);
  }
  .link.checking {
    color: var(--st-check-text);
  }
  .link.broken {
    color: var(--st-fail-text);
  }
  .link.needs {
    color: var(--st-limit-text);
  }
  .ring {
    width: 9px;
    height: 9px;
    flex: none;
    box-sizing: border-box;
    border-radius: 50%;
    border: 1.5px solid var(--st-check);
    animation: breathe 1.6s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      opacity: 0.35;
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
  /* Every other row a touch lighter (see `alt`: a folder counts as one row, its files alternate
     inside it). Lies over the row's own colour (folder files, tints, hover); the focused row
     shows its own colour alone. Every link row carries the layer, so a row whose place changes
     eases. */
  .found .row:not(.focused):not(.folder) {
    background-image: linear-gradient(var(--zb), var(--zb));
  }
  .row.alt {
    --zb: var(--zebra);
  }
</style>
