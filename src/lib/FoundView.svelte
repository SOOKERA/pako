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
  import { bytes, dateTitle, extension, shortDate, sourceOf } from "./format";
  import Icon from "./Icon.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import { choices, targets } from "./devices.svelte";
  import { downloadFoundOn, downloadOnMenu, downloadSplit, sending } from "./downloadOn.svelte";
  import { pop } from "./motion";

  let { onadd, onmenu }: { onadd: () => void; onmenu: (x: number, y: number, items: MenuItem[]) => void } = $props();

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
  const deviceName = (id: string) => choices.list.find((c) => c.id === id)?.name ?? "another PC";

  /** "Download on ▾" in the toolbar: This PC (the usual Download) or one of your devices. */
  function onMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    // "This PC" here is for this action only: every picked link downloads here, whatever its
    // own "Download on" says (and a device picked here overrides it the same way).
    onmenu(r.left, r.bottom + 6, downloadOnMenu(targets.list, () => download(false, false, app.picked, false, true), (t) => downloadFoundOn(t, app.picked), true));
  }

  async function chooseDest() {
    const picked = await pickFolder({ directory: true, defaultPath: dest || undefined });
    if (typeof picked === "string") app.foundDest = picked;
  }

  /** Picked links that are already downloaded: in the downloads list, or the file is in the folder. */
  let dupAsk = $state<{ ids: string[]; dups: FoundItem[]; first: boolean; notify: boolean } | null>(null);

  function sameFile(f: FoundItem, d: Download) {
    const a = f.template.source;
    const b = d.source;
    if (a.kind === "gDrive" && b.kind === "gDrive") return a.fileId === b.fileId;
    return d.link === f.template.link && d.name === f.template.name;
  }

  /** `here`: download on this PC whatever each link's own "Download on" says (the menu's
   *  "Download on this PC"). Else links set to another PC go there (`downloadSplit`). */
  async function download(first: boolean, notify = false, items: FoundItem[] = app.picked, checked = false, here = false): Promise<void> {
    if (!here && !checked && items.some((f) => f.device)) {
      if (busy) return;
      return downloadSplit(items, first, (rows): Promise<void> => download(first, notify, rows, false, true));
    }
    const ids = items.filter(canDownload).map((f) => f.id);
    if (!ids.length || busy) return;
    // Busy from the first click: a quick second one doesn't run the check (and the download) twice.
    busy = true;
    try {
      if (!checked) {
        const onDisk = new Set(await api.foundExisting(ids, app.foundDest).catch(() => [] as string[]));
        const dups = app.found.filter((f) => ids.includes(f.id) && (onDisk.has(f.id) || app.downloads.some((d) => sameFile(f, d))));
        if (dups.length) {
          dupAsk = { ids, dups, first, notify };
          return;
        }
      }
      const count = await api.foundDownload(ids, first, app.foundDest, notify);
      await app.refresh();
      if (count) app.toast(`${count === 1 ? "1 file" : `${count} files`} moved to Downloads`);
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
    if (ids.length) app.removeFound(ids, ids.length === 1 ? "1 broken link removed" : `${ids.length} broken links removed`);
  }

  /** The arrow next to the main button: the same, plus a notification when done. */
  function moreMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const items: MenuItem[] = [
      { label: startsNow ? "Download and notify me" : "Add to queue and notify me", icon: "bell", action: () => download(false, true) },
    ];
    if (!startsNow) items.push({ label: "Download first and notify me", icon: "up", action: () => download(true, true) });
    onmenu(r.right - 230, r.bottom + 6, items);
  }

  /** Right-click on a row: the ticked rows if it's one of them, else just this one. */
  function rowMenu(e: MouseEvent, row: FoundItem[]) {
    e.preventDefault();
    const inTicked = row.every((f) => app.isTicked(f)) && app.ticked.length > row.length;
    const items = inTicked ? app.ticked : row;
    const ids = items.map((f) => f.id);
    // A file on several hosts downloads once.
    const able = app.oncePerFile(items.filter(canDownload));
    const count = items.length > 1 ? ` ${items.length}` : "";
    const all = (p: Priority) => items.every((f) => f.template.priority === p);
    const set = (o: Parameters<typeof api.foundSet>[1]) =>
      app.run(async () => {
        await api.foundSet(ids, o);
        await app.refresh();
      });
    const menu: MenuItem[] = [
      { label: able.length > 1 ? `Download ${able.length}` : "Download", icon: "download", disabled: !able.length, action: () => download(false, false, able) },
      { label: "Download first", icon: "up", disabled: !able.length, action: () => download(true, false, able) },
      { label: "Download and notify me", icon: "bell", disabled: !able.length, action: () => download(false, true, able) },
      ...(targets.list.length && able.length ? [{ separator: true }, ...downloadOnMenu(targets.list, null, (t) => downloadFoundOn(t, able), true)] : []),
      { separator: true },
      { label: "Notify when done", icon: "bell", hint: items.every((f) => f.template.notify) ? "✓" : "", action: () => set({ notify: !items.every((f) => f.template.notify) }) },
      { label: "Priority: First", icon: "up", hint: all("first") ? "✓" : "", action: () => set({ priority: "first" }) },
      { label: "Priority: Normal", icon: "minus", hint: all("normal") ? "✓" : "", action: () => set({ priority: "normal" }) },
      { label: "Priority: Later", icon: "down", hint: all("later") ? "✓" : "", action: () => set({ priority: "later" }) },
      {
        label: "Save to…",
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
              label: row.length > 1 ? "Remove only this folder" : "Remove only this one",
              icon: "trash",
              danger: true,
              action: () => app.removeFound(row.map((f) => f.id)),
            },
          ]
        : []),
      {
        label: `Remove${count}`,
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
        hint: f.template.size != null && h.label === "Ready" ? `${h.label} · ${bytes(f.template.size)}` : h.label,
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
      { label: "Paste links", icon: "link", hint: "Ctrl+V", action: () => pasteLinks() },
      { label: "Add links…", icon: "plus", action: onadd },
      { separator: true },
      { label: "Tick all", icon: "check", hint: "Ctrl+A", disabled: !app.foundVisible.length, action: () => app.setTicked(app.foundVisible, true) },
      { label: "Untick all", icon: "x", hint: "Esc", disabled: !app.ticked.length, action: () => app.setTicked(app.foundVisible, false) },
    ];
    if (app.foundCounts.broken) {
      items.push({ separator: true });
      items.push({ label: `Clear broken links · ${app.foundCounts.broken}`, icon: "trash", action: clearBroken });
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
</script>

<div class="found">
  <div class="toolbar">
    <h1>Links</h1>
    <TutorialButton id="found" />
    {#if app.adding}
      <span class="adding"><span class="ring"></span>Adding links…</span>
    {/if}
    {#if n}
      <div class="selchip"><span><b>{n} picked</b>{#if app.pickedSize}<span class="dim">{` · ${bytes(app.pickedSize)}`}</span>{/if}</span></div>
    {:else if app.found.length}
      <span class="dim small none-picked">Nothing picked</span>
    {/if}
    <button class="icon-btn add" data-tour="found-add" title="Add links (or press Ctrl+V anywhere)" aria-label="Add links" onclick={onadd}><Icon name="plus" size={15} stroke={2.2} /></button>
    <div class="spacer"></div>
    <button class="btn dest" data-tour="found-dest" title={dest} onclick={chooseDest}>
      <Icon name="folder" size={14} /><span class="dim wide">Save to</span><span class="dname">{destName}</span>
      <span class="dchev"><Icon name="chevron" size={11} stroke={2.4} /></span>
    </button>
    {#if app.foundCounts.broken}
      <button class="btn ghost clearb" title="Remove every link that doesn't work" onclick={clearBroken}>
        <Icon name="trash" size={13} /><span class="wide">Clear broken ·</span>{app.foundCounts.broken}
      </button>
    {/if}
    <button class="btn ghost danger" title="Remove the ticked links" disabled={!app.ticked.length} onclick={remove}>
      <span class="narrow"><Icon name="trash" size={13} /></span><span class="mid">Remove</span>{tickedRows ? ` ${tickedRows}` : ""}
    </button>
    {#if n && !startsNow}
      <button class="btn" disabled={busy} title="The downloads that started last step aside (nothing lost) and these start now" onclick={() => download(true)}>
        <Icon name="up" size={13} stroke={2.4} /><span class="wide">Download first</span>
      </button>
    {/if}
    {#if targets.list.length}
      <button class="btn on" data-tour="found-download-on" disabled={!n || busy || sending.busy} title="Download on this PC or on one of your devices" aria-haspopup="menu" onclick={onMenu} transition:pop={{ duration: 180, from: 0.94, y: 0 }}>
        <Icon name="devices" size={14} /><span class="wide">Download on</span><span class="ochev"><Icon name="chevron" size={11} stroke={2.4} /></span>
      </button>
    {/if}
    <div class="split" data-tour="found-download">
      <button class="btn primary" disabled={!n || busy} title={startsNow ? "Download" : "Add to queue"} onclick={() => download(false)}>
        {#if startsNow}
          <Icon name="download" size={14} stroke={2.4} /><span class="mid">Download</span>{n ? ` ${n}` : ""}
        {:else}
          <Icon name="queue" size={14} stroke={2.2} /><span class="mid">Add to queue ·</span> {n}
        {/if}
      </button>
      <button class="btn primary more" disabled={!n || busy} aria-label="More download options" title="Download and notify me" onclick={moreMenu}>
        <Icon name="chevron" size={12} stroke={2.6} />
      </button>
    </div>
  </div>

  <div class="head">
    <div class="cell">
      <button
        class="check"
        class:on={allTick === "all"}
        class:some={allTick === "some"}
        role="checkbox"
        aria-checked={allTick === "some" ? "mixed" : allTick === "all"}
        disabled={!app.foundVisible.length}
        aria-label={allTick === "all" ? "Untick all" : "Tick all"}
        title={allTick === "all" ? "Untick all (Esc)" : "Tick all (Ctrl+A)"}
        onclick={() => app.setTicked(app.foundVisible, allTick !== "all")}
      >
        {#if allTick === "some"}
          <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 5h6" stroke="var(--accent-ink)" stroke-width="2" /></svg>
        {:else}
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
        {/if}
      </button>
    </div>
    <div class="cell">Type</div>
    {#each [["name", "Name", ""], ["source", "Source", "c-src"], ["size", "Size", "c-size"], ["link", "Link", ""], ["date", "Added on", "c-date"]] as [key, label, cls] (key)}
      <button class="cell sorter {cls}" class:sorted={app.sortFound?.key === key} onclick={() => app.toggleSort("found", key as SortKey)} title="Sort by {label.toLowerCase()}">
        {label}
        {#if app.sortFound?.key === key}<Icon name={app.sortFound.dir === 1 ? "up" : "down"} size={10} stroke={2.6} />{/if}
      </button>
    {/each}
  </div>

  <div
    class="body"
    data-tour="found-list"
    role="presentation"
    bind:this={body}
    bind:clientHeight={viewH}
    onscroll={(e) => (scrollTop = (e.currentTarget as HTMLElement).scrollTop)}
    oncontextmenu={emptyMenu}
  >
    {#if app.found.length === 0}
      <div class="empty">
        <div class="empty-icon"><Icon name="link" size={26} /></div>
        <h3>No links yet</h3>
        <p>Paste links here first: Pako checks them, then you pick what to download.</p>
        <button class="btn primary" onclick={onadd}><Icon name="plus" size={14} stroke={2.4} />Add links</button>
        <p class="hint">Tip: press <kbd>Ctrl</kbd> + <kbd>V</kbd> anywhere to paste links.</p>
      </div>
    {:else if app.foundRows.length === 0}
      <div class="empty small"><p>Nothing here{app.search ? ` for "${app.search}"` : ""}.</p></div>
    {:else}
      {#if isVirtual}<div style="height: {win.padTop}px"></div>{/if}
      {#each shownRows as r (r.kind === "folder" ? "g:" + r.key : r.f.id)}
        {#if r.kind === "folder"}
          {@const t = folderTick(r.items)}
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
                class:on={t.all}
                class:some={t.some}
                role="checkbox"
                aria-checked={t.some ? "mixed" : t.all}
                aria-label="Pick all files in {r.name}"
                onclick={(e) => {
                  e.stopPropagation();
                  app.setTicked(app.withMirrors(r.items), !t.all);
                }}
              >
                {#if t.some}
                  <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 5h6" stroke="var(--accent-ink)" stroke-width="2" /></svg>
                {:else}
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
                {/if}
              </button>
            </div>
            <div class="cell type">
              <span class="stb" style="background: {r.set?.missing.length ? 'var(--st-fail)' : 'transparent'}"></span>
              <FileIcon kind={r.set ? "archive" : "folder"} size={18} />
              {#if r.set}<span class="ext mono">PARTS</span>{/if}
            </div>
            <div class="cell name">
              <button
                class="chev"
                ondblclick={(e) => e.stopPropagation()}
                aria-label={r.open ? "Collapse folder" : "Expand folder"}
                onclick={(e) => {
                  e.stopPropagation();
                  app.toggleFoundFolder(r.key);
                }}
              >
                <Icon name="chevron" size={12} stroke={2.6} />
              </button>
              <span class="fname">{r.name}</span>
              {#if r.set}
                <span class="chip">{t.total} parts · {t.on} picked</span>
                {#if r.set.missing.length}<span class="chip warn" title={r.set.missing.join(", ")}>{r.set.missing[0]}{r.set.missing.length > 1 ? ` +${r.set.missing.length - 1}` : ""}</span>{/if}
              {:else}
                <span class="chip">{t.total} {t.total === 1 ? "file" : "files"} · {t.on} picked</span>
              {/if}
            </div>
            <div class="cell src c-src"><span class="dot" style="background: {sourceOf(r.items[0].template).color}"></span><span class="ell">{sourceOf(r.items[0].template).label}</span></div>
            <div class="cell size c-size mono">{size ? bytes(size) : ""}</div>
            {#if r.set}
              <div class="cell link" class:ready={!r.set.missing.length} class:broken={r.set.missing.length > 0}>
                {#if r.set.missing.length}<Icon name="alert" size={13} /><span class="ell">Incomplete set</span>{:else}<Icon name="check" size={13} stroke={2.6} /><span class="ell">Split archive</span>{/if}
              </div>
            {:else}
              <div class="cell link ready"><Icon name="check" size={13} stroke={2.6} />Opened</div>
            {/if}
            <div class="cell date mono c-date" title={dateTitle(newest)}>{shortDate(newest)}</div>
          </div>
        {:else}
          {@const f = r.f}
          {@const m = r.mirror}
          {@const all = m ? m.items : [f]}
          {@const able = canDownload(f)}
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
                aria-label={on ? `Unpick ${f.template.name}` : `Pick ${f.template.name}`}
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
              {#if f.device || f.deviceNote}
                <span
                  class="devmark"
                  class:warn={!!f.deviceNote}
                  title={f.device ? `Downloads on ${deviceName(f.device)}${f.deviceNote ? ` · ${f.deviceNote}` : ""}` : (f.deviceNote ?? "")}
                  aria-label={f.device ? `Downloads on ${deviceName(f.device)}` : "Back to this PC"}
                  transition:pop={{ duration: 160, from: 0.8, y: 0 }}
                ><Icon name="devices" size={12} stroke={2.1} />{#if f.device}<span class="devname">{deviceName(f.device)}</span>{/if}</span>
              {/if}
              {#if m}
                <button
                  class="hosts"
                  data-tour="found-hosts"
                  title="The same file on {m.items.length} hosts: pick the one to download from (the others are backups)"
                  aria-label="Pick the host · on {m.items.length} hosts"
                  aria-haspopup="menu"
                  ondblclick={(e) => e.stopPropagation()}
                  onclick={(e) => hostMenu(e, m)}
                ><span class="hlong">on {m.items.length} hosts</span><span class="hshort"><Icon name="globe" size={11} stroke={2.2} />{m.items.length}</span><Icon name="chevron" size={10} stroke={2.6} /></button>
              {/if}
            </div>
            <div class="cell src c-src"><span class="dot" style="background: {sourceOf(f.template).color}"></span><span class="ell" title={sourceOf(f.template).label}>{sourceOf(f.template).label}</span></div>
            <div class="cell size c-size mono" title={f.template.source.kind === "video" && f.template.size != null ? "Estimated size for the chosen quality" : ""}>{f.template.size != null ? (f.template.source.kind === "video" ? "≈ " : "") + bytes(f.template.size) : f.state === "checking" || f.state === "opening" ? "…" : "—"}</div>
            {#if f.deviceNote && able}
              <div class="cell link needs" title={f.deviceNote}><Icon name="alert" size={12} /><span class="ell">{f.deviceNote}</span></div>
            {:else if f.state === "ready"}
              <div class="cell link ready" title={f.message ?? ""}>
                <Icon name="check" size={13} stroke={2.6} /><span class="ell">{f.message ? "Ready · Google limit today" : "Ready"}</span>
              </div>
            {:else if f.state === "checking"}
              <div class="cell link checking"><span class="ring"></span><span class="ell">{f.template.source.kind === "video" ? "Checking video…" : "Checking link"}</span></div>
            {:else if f.state === "needsTools"}
              <div class="cell link needs" title="Downloading videos uses three free tools: yt-dlp, Deno and ffmpeg (about 200 MB).">
                <span class="ell">Needs the video tools</span>
                <button
                  class="addg"
                  disabled={app.videoToolsBusy}
                  onclick={(e) => {
                    e.stopPropagation();
                    app.installVideoTools();
                  }}>{app.videoToolsBusy ? "Installing…" : "Install"}</button
                >
              </div>
            {:else if f.state === "opening"}
              {@const waiting = (f.message ?? "").startsWith("Waiting for you")}
              {@const queued = (f.message ?? "").startsWith("Waiting for the link")}
              {@const check = (f.message ?? "").includes("check")}
              <div class="cell link" class:checking={!waiting} class:needs={waiting} title={f.message ?? ""}>
                {#if waiting}
                  <span class="ell">Waiting for you</span>
                {:else if queued}
                  <span class="ring"></span><span class="ell">Waiting its turn</span>
                {:else}
                  <span class="ring"></span><span class="ell">Opening link…</span>
                {/if}
                {#if !queued}
                  <button
                    class="addg"
                    title={waiting ? "Bring the link page to the front" : "See the link page and finish it yourself"}
                    onclick={(e) => {
                      e.stopPropagation();
                      app.run(() => api.gateFocus(f.id));
                    }}>{check ? "Show check" : "Show page"}</button
                  >
                {/if}
              </div>
            {:else if f.state === "broken"}
              <div class="cell link broken" title={f.message ?? ""}><Icon name="x" size={12} stroke={2.6} /><span class="ell">{f.message ?? "Link broken"}</span></div>
            {:else}
              <div class="cell link needs" title={f.message ?? ""}>
                <span class="ell">Only in the browser</span>
                <button
                  class="addg"
                  onclick={(e) => {
                    e.stopPropagation();
                    app.run(() => api.openInDrive(f.id));
                  }}
                >Open</button>
              </div>
            {/if}
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
    title={ask.dups.length === 1 ? `"${ask.dups[0].template.name}" is already downloaded` : `${ask.dups.length} of these are already downloaded`}
    width={480}
    onclose={() => (dupAsk = null)}
  >
    <p class="dup-msg">
      {ask.dups.length === 1 ? "It's" : "They're"} already in your downloads or in the folder.
      Downloading again keeps the old {ask.dups.length === 1 ? "file" : "files"} and saves a second copy with a new name.
    </p>
    {#if ask.dups.length > 1}
      <ul class="dup-list">
        {#each ask.dups.slice(0, 6) as f (f.id)}<li class="ell">{f.template.name}</li>{/each}
        {#if ask.dups.length > 6}<li class="more">and {ask.dups.length - 6} more</li>{/if}
      </ul>
    {/if}
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (dupAsk = null)}>Cancel</button>
      <button
        class="btn"
        onclick={() => {
          dupAsk = null;
          const items = app.found.filter((f) => ask.ids.includes(f.id));
          download(ask.first, ask.notify, items, true);
        }}>Download again</button
      >
      {#if !all}
        <button
          class="btn primary"
          onclick={() => {
            dupAsk = null;
            const skip = new Set(ask.dups.map((f) => f.id));
            const items = app.found.filter((f) => ask.ids.includes(f.id) && !skip.has(f.id));
            download(ask.first, ask.notify, items, true);
          }}>Skip {ask.dups.length === 1 ? "it" : "them"}</button
        >
      {/if}
    {/snippet}
  </Modal>
{/if}

<style>
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
  .split .btn:first-child {
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

  .head,
  .row {
    display: grid;
    grid-template-columns: 30px 46px minmax(160px, 1fr) 140px 96px 220px 112px;
    align-items: center;
    padding: 0 16px;
  }
  /* Narrow window: drop the least important columns (like the Downloads table):
     the source first, then the date, then the size. */
  @container (max-width: 900px) {
    .head,
    .row {
      grid-template-columns: 30px 46px minmax(150px, 1fr) 96px 200px 112px;
    }
    .c-src {
      display: none !important;
    }
  }
  @container (max-width: 760px) {
    .head,
    .row {
      grid-template-columns: 30px 46px minmax(140px, 1fr) 96px 180px;
    }
    .c-src,
    .c-date {
      display: none !important;
    }
  }
  @container (max-width: 600px) {
    .head,
    .row {
      grid-template-columns: 30px 46px minmax(110px, 1fr) minmax(112px, 140px);
    }
    .c-size {
      display: none !important;
    }
    /* The Link column already says "Incomplete set". */
    .chip.warn {
      display: none;
    }
  }
  .head {
    height: 32px;
    flex: none;
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
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding-bottom: 12px;
  }
  .row {
    height: 46px;
    border-bottom: 1px solid var(--line-soft);
    transition: background var(--fast) ease;
  }
  .row:hover {
    background: var(--hover);
  }
  .row {
    cursor: default;
  }
  .sorter {
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
  .addg {
    flex: none;
    height: 24px;
    padding: 0 9px;
    border: none;
    border-radius: var(--radius);
    background: rgba(209, 123, 240, 0.16);
    color: #ebc3fa;
    font-size: 12px;
    font-weight: 600;
    transition:
      background var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .addg:hover:not(:disabled) {
    background: rgba(209, 123, 240, 0.26);
  }
  :global(:root[data-theme="light"]) .addg {
    color: var(--st-limit-text);
  }
  .addg:active:not(:disabled) {
    transform: scale(0.97);
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
  /* Every other row a touch lighter, so rows are easy to follow across. Stacks on top of
     the row's own colour (folder files, red/purple tints, hover, selection). */
  .row.alt:not(.tint-fail):not(.tint-limit):not(.selected) {
    background-image: linear-gradient(var(--zebra), var(--zebra));
  }
</style>
