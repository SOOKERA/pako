<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The private list (Private mode, open): what's in the vault, newest first. A folder download
  // (a Drive folder, a split set, a playlist) is one folder row with its files inside. Click to
  // pick, Ctrl/Shift-click or the boxes to pick several (Ctrl+A all, Esc none). Videos play in
  // Pako's own player; other files open from a temporary copy. Decrypt… and Delete work on one
  // file, a folder or everything picked; the details panel shows the picked one. The header's
  // Normal list keeps Private mode open; Lock and any other way off it lock it (privateLeave.ts).
  // Sends, uploads and Decrypt… run here and show their progress on the rows.
  import { SvelteSet } from "svelte/reactivity";
  import { api, isActive, type Download } from "../api";
  import { app } from "../state.svelte";
  import { priv } from "../private.svelte";
  import { bytes, bytesOf, isVideo, speed, statusLook } from "../format";
  import type { MenuItem } from "../menu";
  import { fadeIn, flipIf, flipOpts, rowSlide, sideSlide } from "../motion";
  import FileIcon from "../FileIcon.svelte";
  import Icon from "../Icon.svelte";
  import Modal from "../Modal.svelte";
  import PrivatePlayer from "./PrivatePlayer.svelte";
  import PrivateDecrypt from "./PrivateDecrypt.svelte";
  import PrivateDetails from "./PrivateDetails.svelte";
  import PanelResizer from "../PanelResizer.svelte";
  import { panelWidths } from "../panelWidth.svelte";
  import { shownWidth } from "../panelSize";
  import PrivateSendDirect from "./PrivateSendDirect.svelte";
  import PrivateUpload from "./PrivateUpload.svelte";
  import PrivateSwitch from "./PrivateSwitch.svelte";
  import TutorialButton from "../tour/TutorialButton.svelte";
  import { help } from "../help/help.svelte";
  import ColumnGrip from "../ColumnGrip.svelte";
  import ColumnsButton from "../ColumnsButton.svelte";
  import { listColumns } from "../columns.svelte";
  import { LISTS, gridTemplate, shownColumns, withPost } from "../columns";

  let { onmenu }: { onmenu: (x: number, y: number, items: MenuItem[]) => void } = $props();

  // ---------- columns: widths, shown or hidden (columns.ts; only widths, nothing private) ----------
  const LIST = LISTS.private;
  /** The column titles' width: the list's room (the same width its container query sees). */
  let headW = $state(0);
  /** 560 px or less: the row's buttons go over the date, their column keeps the columns button. */
  const listNow = $derived(headW > 0 && headW <= 560 ? withPost(LIST, 24) : LIST);
  const colState = $derived(listColumns.state("private"));
  const frozen = $derived(listColumns.frozen?.list === "private" ? listColumns.frozen : null);
  const shown = $derived(frozen ? { keys: frozen.keys, tight: false } : shownColumns(listNow, colState, headW));
  /** A folder's files sit 14 px in: their Name gives those 14 px, so every column stays under
   *  its title. */
  const CHILD_IN = 14;
  const colsStyle = $derived(
    `--cols: ${gridTemplate(listNow, colState, shown.keys, shown.tight)}; --cols-child: ${gridTemplate({ ...listNow, nameMin: listNow.nameMin - CHILD_IN, nameTight: listNow.nameTight - CHILD_IN }, colState, shown.keys, shown.tight)}`,
  );
  /** "Downloaded" hidden by you: how each file is doing shows under its name instead. */
  const stateUnder = $derived(colState.hidden.includes("date"));
  const hideCls = $derived(
    LIST.cols
      .filter((c) => !shown.keys.includes(c.key))
      .map((c) => `h-${c.key}`)
      .join(" "),
  );
  const noRoom = $derived(LIST.cols.filter((c) => !colState.hidden.includes(c.key) && !shown.keys.includes(c.key)).map((c) => c.key));
  function columnsMenu(e: MouseEvent) {
    e.preventDefault();
    onmenu(e.clientX, e.clientY, listColumns.menu("private", undefined, () => noRoom));
  }
  // The details panel's width (dragged on its edge, remembered); it gives way to the list
  // first when the window is narrow.
  let bodyW = $state(0);
  const sideW = $derived(shownWidth(panelWidths.of("private"), bodyW));

  type Folder = { name: string; files: Download[]; date: number };
  type Row = { kind: "file"; d: Download; rel: string | null; last?: boolean } | { kind: "folder"; f: Folder; open: boolean };

  const SEP = /[\\/]/;
  const done = (d: Download) => d.status === "completed";
  const running = (d: Download) => ["starting", "downloading", "retrying", "verifying"].includes(d.status);
  const canPause = (d: Download) => isActive(d.status) || d.status === "queued";
  /** When it was downloaded (finished ones), else when it was added. */
  const when = (d: Download) => (done(d) && d.finishedAt ? d.finishedAt * 1000 : d.date || d.addedAt);
  const topOf = (d: Download) => (d.vaultFolder ? d.vaultFolder.split(SEP)[0] : null);

  /** Folders closed by a click on their arrow (open by default). */
  const closed = new SvelteSet<string>();
  const rows = $derived.by(() => {
    const folders = new Map<string, Folder>();
    const loose: Download[] = [];
    // The title bar's search, here over private names (in memory only, never saved).
    const q = app.search.trim().toLowerCase();
    const hit = (d: Download) => !q || d.name.toLowerCase().includes(q) || (topOf(d) ?? "").toLowerCase().includes(q);
    for (const d of priv.list.filter(hit)) {
      const top = topOf(d);
      if (!top) {
        loose.push(d);
        continue;
      }
      const f = folders.get(top) ?? { name: top, files: [], date: 0 };
      f.files.push(d);
      f.date = Math.max(f.date, when(d));
      folders.set(top, f);
    }
    const tops: ({ at: number; d?: Download; f?: Folder })[] = [
      ...loose.map((d) => ({ at: when(d), d })),
      ...[...folders.values()].map((f) => ({ at: f.date, f })),
    ].sort((a, b) => b.at - a.at);
    const out: Row[] = [];
    for (const t of tops) {
      if (t.d) {
        out.push({ kind: "file", d: t.d, rel: null });
        continue;
      }
      const f = t.f!;
      const open = !closed.has(f.name);
      out.push({ kind: "folder", f, open });
      if (!open) continue;
      const files = [...f.files].sort((a, b) => ((a.vaultFolder ?? "") + "\u0000" + a.name).localeCompare((b.vaultFolder ?? "") + "\u0000" + b.name));
      files.forEach((d, i) => {
        const inner = (d.vaultFolder ?? "").split(SEP).slice(1).join("\\");
        out.push({ kind: "file", d, rel: inner || null, last: i === files.length - 1 });
      });
    }
    return out;
  });
  const fileRows = $derived(rows.flatMap((r) => (r.kind === "file" ? [r.d] : [])));
  const keyOf = (r: Row) => (r.kind === "file" ? r.d.id : `folder:${r.f.name}`);
  /** Rows shaded every other one, as in the other lists: a folder counts as one row, its files
   *  alternate inside it. By place in the list, so it holds while scrolling and filtering. */
  const alt = $derived.by(() => {
    const out = new Set<string>();
    let top = 0;
    let inner = 0;
    for (const r of rows) {
      if (r.kind === "file" && topOf(r.d)) {
        if (inner++ % 2) out.add(r.d.id);
        continue;
      }
      inner = 0;
      if (r.kind === "file" && top % 2) out.add(r.d.id);
      top++;
    }
    return out;
  });

  /** Picked files (by id); `focus`: the row the details panel shows ("folder:<name>" for a folder). */
  const picked = new SvelteSet<string>();
  let focus = $state<string | null>(null);
  let anchor: string | null = null;
  const pickedList = $derived(priv.list.filter((d) => picked.has(d.id)));
  const focusFile = $derived(focus && !focus.startsWith("folder:") ? (priv.list.find((d) => d.id === focus) ?? null) : null);
  const focusFolder = $derived.by(() => {
    if (!focus?.startsWith("folder:")) return null;
    const name = focus.slice(7);
    const files = priv.list.filter((d) => topOf(d) === name);
    return files.length ? { name, files } : null;
  });
  // Rows that left the list (deleted, decrypted) leave what's picked too.
  $effect(() => {
    const ids = new Set(priv.list.map((d) => d.id));
    for (const id of [...picked]) if (!ids.has(id)) picked.delete(id);
    if (focus && !focus.startsWith("folder:") && !ids.has(focus)) focus = null;
  });
  /** What the details panel shows: several picked (not one folder), else the focused row. */
  const several = $derived(picked.size > 1 && !(focusFolder && focusFolder.files.length === picked.size));

  let playing = $state<Download | null>(null);
  $effect(() => {
    priv.playing = playing ? playing.name : null;
  });
  let deleting = $state<Download[] | null>(null);
  let decrypting = $state<{ items: Download[]; folder: string | null } | null>(null);
  /** "Send with Direct…": the files, and a folder's name as the send's title. */
  let sendingDirect = $state<{ items: Download[]; title: string | null } | null>(null);
  /** "Upload…": its choices, here (no trip to Upload). */
  let uploading = $state<{ items: Download[]; title: string | null } | null>(null);

  /** "Upload…": the files go to Upload's basket, read decrypted as they go up. */
  function upload(list: Download[], title: string | null = null) {
    uploading = { items: list, title };
  }

  function use(d: Download) {
    if (!done(d)) return;
    if (isVideo(d.name)) playing = d;
    else app.run(() => api.privateOpen(d.id));
  }

  function pickOnly(ids: string[], at: string | null) {
    picked.clear();
    for (const id of ids) picked.add(id);
    focus = at;
  }

  function clickFile(e: MouseEvent, d: Download) {
    if (e.shiftKey && anchor) {
      const a = fileRows.findIndex((x) => x.id === anchor);
      const b = fileRows.findIndex((x) => x.id === d.id);
      if (a >= 0 && b >= 0) {
        const [lo, hi] = a < b ? [a, b] : [b, a];
        if (!e.ctrlKey) picked.clear();
        for (const x of fileRows.slice(lo, hi + 1)) picked.add(x.id);
        focus = d.id;
        return;
      }
    }
    if (e.ctrlKey || e.metaKey) {
      if (picked.has(d.id)) picked.delete(d.id);
      else picked.add(d.id);
      focus = d.id;
    } else pickOnly([d.id], d.id);
    anchor = d.id;
  }

  function clickFolder(e: MouseEvent, f: Folder) {
    const ids = f.files.map((d) => d.id);
    if (e.ctrlKey || e.metaKey) {
      toggle(ids);
      focus = `folder:${f.name}`;
    } else pickOnly(ids, `folder:${f.name}`);
  }

  /** A box (or Ctrl-click on a folder): adds these, or takes them off when all were picked. */
  function toggle(ids: string[]) {
    const all = ids.every((id) => picked.has(id));
    for (const id of ids) {
      if (all) picked.delete(id);
      else picked.add(id);
    }
    if (!focus && ids.length === 1) focus = ids[0];
  }

  const allPicked = $derived(priv.list.length > 0 && priv.list.every((d) => picked.has(d.id)));
  const somePicked = $derived(picked.size > 0 && !allPicked);

  /** What a right-click or the "N selected" bar acts on. */
  function actionsFor(list: Download[], folder: string | null): MenuItem[] {
    const items: MenuItem[] = [];
    const one = list.length === 1 && !folder ? list[0] : null;
    if (one && done(one)) items.push(isVideo(one.name) ? { label: "Play", icon: "play", hint: "Enter", action: () => use(one) } : { label: "Open", icon: "open", hint: "Enter", action: () => use(one) });
    if (list.every(done)) {
      items.push({ label: folder ? "Decrypt folder…" : one ? "Decrypt…" : `Decrypt ${list.length} files…`, icon: "unlock", action: () => (decrypting = { items: list, folder }) });
      // Sent as normal files, read decrypted as they go: no readable copy on this PC.
      items.push({ label: "Send with Direct…", icon: "send", action: () => (sendingDirect = { items: list, title: folder }) });
      items.push({ label: "Upload…", icon: "upload", action: () => upload(list) });
    }
    const pausing = list.filter(canPause).map((d) => d.id);
    const resuming = list.filter((d) => !done(d) && !canPause(d)).map((d) => d.id);
    if (pausing.length) items.push({ label: pausing.length > 1 ? `Pause ${pausing.length}` : "Pause", icon: "pause", action: () => app.run(() => api.pause(pausing)).then(() => priv.refreshList()) });
    if (resuming.length)
      items.push({
        label: list.some((d) => d.status === "failed") ? "Try again" : resuming.length > 1 ? `Resume ${resuming.length}` : "Resume",
        icon: "play",
        action: () => app.run(() => api.resume(resuming)).then(() => priv.refreshList()),
      });
    if (items.length) items.push({ separator: true });
    items.push({ label: folder ? "Delete folder…" : one ? "Delete…" : `Delete ${list.length} files…`, icon: "trash", danger: true, hint: "Del", action: () => (deleting = list) });
    return items;
  }
  /** The "N selected" bar's buttons (the menu's, without Open). */
  const batch = $derived(several ? actionsFor(pickedList, null).filter((m) => "action" in m && m.label !== "Play" && m.label !== "Open" && m.label !== "Upload…" && m.label !== "Send with Direct…") : []);

  /** A row's id in the page, for the list's active row (screen readers) and scrolling to it. */
  const domId = (key: string) => "prow-" + key.replace(/[^A-Za-z0-9_-]/g, (c) => `_${c.charCodeAt(0)}`);
  // The focused row stays in view while the keys move it.
  $effect(() => {
    const k = focus;
    if (!k) return;
    queueMicrotask(() => document.getElementById(domId(k))?.scrollIntoView({ block: "nearest" }));
  });
  /** Shift+F10 / the Menu key: the focused row's menu, where the row is. */
  function menuAtFocus() {
    const k = focus;
    if (!k) return;
    const el = document.getElementById(domId(k));
    const r = el?.getBoundingClientRect();
    if (!r) return;
    const at = { preventDefault() {}, clientX: r.left + 40, clientY: r.bottom - 4 } as unknown as MouseEvent;
    if (k.startsWith("folder:")) {
      const f = rows.find((x) => x.kind === "folder" && `folder:${x.f.name}` === k);
      if (f && f.kind === "folder") menuFolder(at, f.f);
    } else {
      const d = priv.list.find((x) => x.id === k);
      if (d) menuFile(at, d);
    }
  }

  function menuFile(e: MouseEvent, d: Download) {
    e.preventDefault();
    if (!picked.has(d.id)) pickOnly([d.id], d.id);
    onmenu(e.clientX, e.clientY, actionsFor(picked.size > 1 ? pickedList : [d], null));
  }
  function menuFolder(e: MouseEvent, f: Folder) {
    e.preventDefault();
    pickOnly(
      f.files.map((d) => d.id),
      `folder:${f.name}`,
    );
    onmenu(e.clientX, e.clientY, actionsFor(f.files, f.name));
  }

  function onkey(e: KeyboardEvent) {
    if (playing || deleting || decrypting || sendingDirect || priv.pad) return;
    // Something over the list (a dialog, Settings, Help, a menu, the bell): its keys, not ours.
    if (app.modals > 0 || app.bellOpen || help.open || document.querySelector('[role="menu"]')) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest("input:not([type=checkbox]),textarea,select,[contenteditable]")) return;
    // A column's or the details panel's edge: its keys resize, nothing else.
    if (target?.closest("[role=separator]")) return;
    // Enter on a focused button presses that button, never "open the picked file".
    if (e.key === "Enter" && target?.closest("button, a, [role=button]")) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      pickOnly(
        priv.list.map((d) => d.id),
        focus,
      );
      return;
    }
    if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === "f" || e.key.toLowerCase() === "k")) {
      // Ctrl+F / Ctrl+K: the title bar's search, which searches this list too.
      e.preventDefault();
      const box = document.querySelector<HTMLInputElement>('[data-tour="search"] input');
      box?.focus();
      box?.select();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "i") {
      e.preventDefault();
      priv.details = !priv.details;
      return;
    }
    if (e.key === "Escape" && picked.size) {
      picked.clear();
      return;
    }
    if ((e.shiftKey && e.key === "F10") || e.key === "ContextMenu") {
      e.preventDefault();
      menuAtFocus();
      return;
    }
    if (e.key === " " && focus) {
      // Space ticks (or unticks) the focused file or folder.
      e.preventDefault();
      const f = focus.startsWith("folder:") ? rows.find((x) => x.kind === "folder" && `folder:${x.f.name}` === focus) : null;
      toggle(f && f.kind === "folder" ? f.f.files.map((d) => d.id) : [focus]);
      return;
    }
    if (e.key === "Enter" && focusFile) use(focusFile);
    else if (e.key === "Delete") {
      const list = picked.size ? pickedList : focusFolder ? focusFolder.files : focusFile ? [focusFile] : [];
      if (list.length) deleting = list;
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Home" || e.key === "End") {
      e.preventDefault();
      const keys = rows.map(keyOf);
      const i = focus ? keys.indexOf(focus) : -1;
      const to = e.key === "Home" ? 0 : e.key === "End" ? keys.length - 1 : i + (e.key === "ArrowDown" ? 1 : -1);
      const row = rows[Math.max(0, Math.min(keys.length - 1, to))];
      if (!row) return;
      if (row.kind === "file") {
        pickOnly([row.d.id], row.d.id);
        anchor = row.d.id;
      } else
        pickOnly(
          row.f.files.map((d) => d.id),
          `folder:${row.f.name}`,
        );
    }
  }

  function line(d: Download) {
    if (done(d)) return bytes(d.size ?? d.downloaded);
    if (d.size) return bytesOf(d.downloaded, d.size);
    return bytes(d.downloaded);
  }
  function pct(d: Download) {
    return done(d) ? 100 : d.size ? Math.min(100, (d.downloaded / d.size) * 100) : 0;
  }
  /** When it was downloaded: "14:32" today, else "18 Nov, 14:32" (its temporary copies are
   *  named after the same time: PrivateFile_2026-11-18_14h32). */
  function stamp(ms: number) {
    const date = new Date(ms);
    // The 24-hour clock in every language, like the temporary copies' names (14h32).
    const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hourCycle: "h23" });
    if (new Date().toDateString() === date.toDateString()) return time;
    const year = date.getFullYear() === new Date().getFullYear() ? undefined : ("numeric" as const);
    return `${date.toLocaleDateString([], { day: "numeric", month: "short", year })}, ${time}`;
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="pv">
  <div class="toolbar">
    <span class="mark"><Icon name="unlock" size={15} stroke={1.9} /></span>
    <h1>Private downloads</h1>
    <TutorialButton id="private" />
    {#if several}
      <span class="sel" in:fadeIn={{ duration: 160 }} title="{picked.size} selected">
        {picked.size}<span class="n">&nbsp;selected</span>
        <button class="icon-btn tiny" aria-label="Select none (Esc)" title="Select none (Esc)" onclick={() => picked.clear()}><Icon name="x" size={11} /></button>
      </span>
    {:else}
      <span class="dim small">{priv.list.length} {priv.list.length === 1 ? "item" : "items"}</span>
    {/if}
    <div class="spacer"></div>
    {#if batch.length}
      <div class="batch" data-tour="private-batch" in:fadeIn={{ duration: 160 }}>
        {#each batch as m (m.label)}
          {#if "action" in m}
            <button class="btn small" class:danger={m.danger} aria-label={m.label} title={m.label} onclick={() => m.action?.()}><Icon name={m.icon ?? "check"} size={13} /><span class="lbl">{m.label}</span></button>
          {/if}
        {/each}
      </div>
    {/if}
    <PrivateSwitch on="private" />
  </div>

  <div class="body" bind:clientWidth={bodyW}>
    {#if rows.length}
      <div class="list {hideCls}" class:easing={listColumns.easing && rows.length <= 60} style={colsStyle} data-tour="private-list">
        <div class="head" role="presentation" oncontextmenu={columnsMenu} bind:clientWidth={headW}>
          <input
            type="checkbox"
            class="box"
            aria-label="Select all (Ctrl+A)"
            checked={allPicked}
            indeterminate={somePicked}
            onchange={() =>
              allPicked
                ? picked.clear()
                : pickOnly(
                    priv.list.map((d) => d.id),
                    focus,
                  )}
          />
          <span></span><span>Name</span>
          {#each LIST.cols as c (c.key)}
            <span class="hc c-{c.key}" class:r={c.key === "size"}><ColumnGrip list="private" col={c} keys={shown.keys} tight={shown.tight} avail={headW} />{c.label}</span>
          {/each}
          <span class="colcell"><ColumnsButton list="private" {onmenu} noRoom={() => noRoom} /></span>
        </div>
        <!-- The rows alone are the list (the titles' edges and buttons aren't its options). -->
        <div class="opts" role="listbox" tabindex="0" aria-multiselectable="true" aria-label="Private downloads" aria-activedescendant={focus ? domId(focus) : undefined}>
        {#each rows as r (keyOf(r))}
          <div class="rw" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
          {#if r.kind === "folder"}
            {@const ids = r.f.files.map((d) => d.id)}
            {@const on = ids.every((id) => picked.has(id))}
            {@const size = r.f.files.reduce((n, d) => n + (d.size ?? d.downloaded), 0)}
            {@const left = r.f.files.filter((d) => !done(d)).length}
            {@const fw = r.f.files.map((d) => priv.workOn(d.id)[0]).find((x) => !!x)}
            {@const fp = fw ? priv.progress(fw) : null}
            {@const fState = fp ? `${fp.label}${fp.frac !== null ? ` · ${Math.floor(fp.frac * 100)}%` : ""}` : left ? `${r.f.files.length - left} of ${r.f.files.length} done` : stamp(r.f.date)}
            {@const fColor = fp ? (fp.failed ? "var(--st-fail-text)" : "var(--priv-text)") : left ? "var(--text-2)" : "var(--text-3)"}
            <div
              class="row folder"
              class:on
              class:focus={focus === `folder:${r.f.name}`}
              id={domId(`folder:${r.f.name}`)}
              aria-label="{r.f.name}, folder, {fState}"
              role="option"
              tabindex="-1"
              aria-selected={on}
              onclick={(e) => clickFolder(e, r.f)}
              oncontextmenu={(e) => menuFolder(e, r.f)}
              onkeydown={() => {}}
            >
              <input
                type="checkbox"
                class="box"
                aria-label="Select {r.f.name}"
                checked={on}
                indeterminate={!on && ids.some((id) => picked.has(id))}
                onclick={(e) => e.stopPropagation()}
                onchange={() => toggle(ids)}
              />
              <button
                class="fold"
                class:open={r.open}
                aria-expanded={r.open}
                aria-label={r.open ? `Close ${r.f.name}` : `Open ${r.f.name}`}
                onclick={(e) => {
                  e.stopPropagation();
                  if (r.open) closed.add(r.f.name);
                  else closed.delete(r.f.name);
                }}><Icon name="chevron" size={13} stroke={2} /></button
              >
              <div class="main">
                <span class="name"><Icon name="folder" size={14} />{r.f.name}<span class="chip">{r.f.files.length} files</span></span>
                {#if stateUnder}<span class="under" style="color: {fColor}">{fState}</span>{/if}
              </div>
              <span class="size mono c-size">{bytes(size)}</span>
              <span class="state c-date" style="color: {fColor}" title={fp?.label ?? ""}>{fState}</span>
              <div class="acts">
                {#if !left}
                  <button
                    class="icon-btn"
                    title="Decrypt folder…"
                    aria-label="Decrypt {r.f.name}"
                    onclick={(e) => {
                      e.stopPropagation();
                      decrypting = { items: r.f.files, folder: r.f.name };
                    }}><Icon name="unlock" size={13} /></button
                  >
                {/if}
                <button
                  class="icon-btn danger"
                  title="Delete folder…"
                  aria-label="Delete {r.f.name}"
                  onclick={(e) => {
                    e.stopPropagation();
                    deleting = r.f.files;
                  }}><Icon name="trash" size={13} /></button
                >
              </div>
            </div>
          {:else}
            {@const d = r.d}
            {@const look = statusLook(d.status, d.message)}
            {@const w = priv.workOn(d.id)[0]}
            {@const wp = w ? priv.progress(w) : null}
            {@const dState = wp ? `${wp.label}${wp.frac !== null ? ` · ${Math.floor(wp.frac * 100)}%` : ""}` : done(d) ? stamp(when(d)) : running(d) && priv.speeds[d.id] ? speed(priv.speeds[d.id]) : look.label}
            {@const dColor = wp ? (wp.failed ? "var(--st-fail-text)" : "var(--priv-text)") : done(d) ? "var(--text-3)" : look.text}
            <div
              class="row"
              class:alt={alt.has(d.id)}
              class:child={!!topOf(d)}
              class:last={r.last}
              class:on={picked.has(d.id)}
              class:focus={focus === d.id}
              id={domId(d.id)}
              aria-label="{d.name}, {done(d) ? `downloaded ${stamp(when(d))}` : line(d)}"
              role="option"
              tabindex="-1"
              aria-selected={picked.has(d.id)}
              onclick={(e) => clickFile(e, d)}
              ondblclick={() => use(d)}
              oncontextmenu={(e) => menuFile(e, d)}
              onkeydown={() => {}}
            >
              <input type="checkbox" class="box" aria-label="Select {d.name}" checked={picked.has(d.id)} onclick={(e) => e.stopPropagation()} onchange={() => toggle([d.id])} />
              <FileIcon name={d.name} size={20} faded={d.status === "paused"} />
              <div class="main">
                <span class="name" title={d.name}>{#if r.rel}<span class="rel">{r.rel}\</span>{/if}{d.name}</span>
                {#if !done(d)}
                  <div class="bar"><div class="fill" style="width: {pct(d)}%; background: {look.color}"></div></div>
                {:else if wp}
                  <div class="bar" transition:rowSlide={{ duration: 160 }}><div class="fill" class:wait={wp.frac === null} style="width: {wp.frac === null ? 100 : wp.frac * 100}%; background: {wp.failed ? 'var(--st-fail)' : 'var(--priv)'}"></div></div>
                {/if}
                {#if stateUnder}<span class="under" style="color: {dColor}" title={wp ? wp.label : (d.message ?? "")}>{dState}</span>{/if}
              </div>
              <span class="size mono c-size">{line(d)}</span>
              <span class="state c-date" style="color: {dColor}" title={wp ? wp.label : (d.message ?? "")}>
                {dState}
              </span>
              <div class="acts" data-tour="private-actions">
                {#if done(d)}
                  <button
                    class="icon-btn"
                    title={isVideo(d.name) ? "Play in Pako" : "Open"}
                    aria-label={isVideo(d.name) ? `Play ${d.name}` : `Open ${d.name}`}
                    onclick={(e) => {
                      e.stopPropagation();
                      use(d);
                    }}><Icon name={isVideo(d.name) ? "play" : "open"} size={13} /></button
                  >
                  <button
                    class="icon-btn"
                    title="Send with Direct or Upload"
                    aria-label="Send {d.name}"
                    aria-haspopup="menu"
                    onclick={(e) => {
                      e.stopPropagation();
                      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
                      onmenu(r.right - 200, r.bottom + 4, [
                        { label: "Send with Direct…", icon: "send", action: () => (sendingDirect = { items: [d], title: null }) },
                        { label: "Upload…", icon: "upload", action: () => upload([d]) },
                      ]);
                    }}><Icon name="send" size={13} /></button
                  >
                  <button
                    class="icon-btn"
                    title="Decrypt…"
                    aria-label="Decrypt {d.name}"
                    onclick={(e) => {
                      e.stopPropagation();
                      decrypting = { items: [d], folder: null };
                    }}><Icon name="unlock" size={13} /></button
                  >
                {/if}
                <button
                  class="icon-btn danger"
                  title="Delete…"
                  aria-label="Delete {d.name}"
                  onclick={(e) => {
                    e.stopPropagation();
                    deleting = [d];
                  }}><Icon name="trash" size={13} /></button
                >
              </div>
            </div>
          {/if}
          </div>
        {/each}
        </div>
      </div>
    {:else}
      <!-- The tutorial's first step points here too when nothing is in Private mode yet. -->
      <div class="empty" data-tour="private-list">
        <span class="emark"><Icon name="lock" size={20} stroke={1.8} /></span>
        <p class="et">Nothing in Private mode yet</p>
        <p class="es">
          Downloads from your browser's private windows come here, encrypted. So do links you send with <b>Download privately</b> (in Links and the Download with Pako window). You can
          also move a finished download here: right-click it, then <b>Move to Private</b>.
        </p>
      </div>
    {/if}
    {#if priv.details}
      <div class="side" class:resizing={panelWidths.dragging} style="--pw: {sideW}px" transition:sideSlide={{ duration: 240 }}>
        <PanelResizer page="private" width={sideW} avail={bodyW} />
        <PrivateDetails
          d={several ? null : focusFile}
          folder={several ? { name: `${picked.size} selected`, files: pickedList, several: true } : focusFolder}
          onuse={use}
          ondecrypt={(items, folder) => (decrypting = { items, folder })}
          onsend={(items, folder) => (sendingDirect = { items, title: folder })}
          onupload={upload}
          ondelete={(items) => (deleting = items)}
          onclose={() => (priv.details = false)}
        />
      </div>
    {/if}
  </div>
</div>

{#if playing}
  <PrivatePlayer d={playing} onclose={() => (playing = null)} />
{/if}

{#if sendingDirect}
  <PrivateSendDirect items={sendingDirect.items} title={sendingDirect.title} onclose={() => (sendingDirect = null)} />
{/if}

{#if uploading}
  <PrivateUpload items={uploading.items} title={uploading.title} onclose={() => (uploading = null)} />
{/if}

{#if decrypting}
  <PrivateDecrypt items={decrypting.items} folder={decrypting.folder} onclose={() => (decrypting = null)} />
{/if}

{#if deleting}
  {@const list = deleting}
  <Modal title={list.length === 1 ? `Delete ${list[0].name}?` : `Delete ${list.length} private files?`} width={440} onclose={() => (deleting = null)}>
    <p class="msg">{list.length === 1 ? "It's" : "They're"} deleted from Private mode for good: there is no copy in the Recycle Bin.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (deleting = null)}>Cancel</button>
      <button
        class="btn danger-fill"
        onclick={() => {
          const ids = list.map((d) => d.id);
          deleting = null;
          app.run(async () => {
            await api.privateRemove(ids);
            await priv.refreshList();
          });
        }}><Icon name="trash" size={13} />Delete</button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .pv {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container-type: inline-size;
    background: linear-gradient(var(--priv-tint), var(--priv-tint)), var(--bg);
    animation: pvIn var(--slow) var(--ease);
  }
  @keyframes pvIn {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 52px;
    padding: 0 16px;
    border-bottom: 1px solid var(--priv-line);
  }
  .mark {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    background: var(--priv);
    color: #fff;
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .side {
    position: relative;
    display: flex;
    min-height: 0;
  }
  .list {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding: 0 8px 6px;
  }
  .head {
    position: sticky;
    top: 0;
    z-index: 1;
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    /* The room between columns is inside their tracks (columns.ts, gap): none here. */
    column-gap: 0;
    height: 32px;
    padding: 0 10px;
    margin-bottom: 4px;
    font-size: 11.5px;
    color: var(--text-3);
    background: linear-gradient(var(--priv-tint), var(--priv-tint)), var(--bg);
    border-bottom: 1px solid var(--priv-line);
  }
  .head .r {
    text-align: right;
  }
  .box {
    width: 14px;
    height: 14px;
    margin: 0;
    accent-color: var(--priv);
  }
  .row {
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    column-gap: 0;
    min-height: 46px;
    padding: 0 10px;
    border-radius: var(--radius);
    transition:
      background var(--fast) ease,
      --zb var(--mid) ease;
    cursor: default;
  }
  .row:hover {
    background: color-mix(in srgb, var(--priv) 7%, transparent);
  }
  .row.on {
    background: color-mix(in srgb, var(--priv) 14%, transparent);
  }
  /* Every other row a touch lighter (see `alt`), in Private mode's own shade; over the row's own
     colour (hover), never over a picked row. Every file row carries the layer, so a row whose
     place changes eases. */
  .opts .row:not(.on):not(.folder) {
    background-image: linear-gradient(var(--zb), var(--zb));
  }
  .row.alt {
    --zb: var(--zebra-priv);
  }
  .row.focus {
    box-shadow: inset 2px 0 0 var(--priv);
  }
  .row.child {
    grid-template-columns: var(--cols-child);
    margin-left: 14px;
    border-left: 1px solid var(--priv-line);
    border-radius: 0 var(--radius) var(--radius) 0;
  }
  .row.child.last {
    margin-bottom: 4px;
  }
  .row.folder .name {
    display: flex;
    align-items: center;
    gap: 7px;
    font-weight: 500;
  }
  .chip {
    font-size: 11px;
    font-weight: 400;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
    color: var(--priv-text);
  }
  .rel {
    color: var(--text-3);
  }
  .fold {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-2);
    transform: rotate(-90deg);
    transition:
      transform var(--fast) var(--ease),
      background var(--fast) ease;
  }
  .fold.open {
    transform: rotate(0deg);
  }
  .fold:hover {
    background: var(--raise);
  }
  .sel {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 2px 9px;
    border-radius: 999px;
    font-size: 12px;
    white-space: nowrap;
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
    color: var(--priv-text);
  }
  .icon-btn.tiny {
    width: 18px;
    height: 18px;
  }
  .batch {
    display: flex;
    gap: 6px;
    min-width: 0;
  }
  h1 {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Narrow (small window, details panel open, several picked): the batch buttons keep their
     icons (names on hover); Normal list and Lock always stay in view (PrivateSwitch). */
  @container (max-width: 860px) {
    .batch .lbl {
      display: none;
    }
    .batch .btn {
      width: 28px;
      padding: 0;
    }
  }
  @container (max-width: 560px) {
    .toolbar {
      gap: 8px;
      padding: 0 12px;
    }
    .toolbar :global(.tut .lbl),
    .sel .n {
      display: none;
    }
  }
  .btn.danger:hover {
    color: var(--st-fail-text);
  }
  .icon-btn.on {
    color: var(--priv-text);
    background: var(--priv-tint);
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .bar {
    height: 3px;
    border-radius: 2px;
    background: var(--line);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 2px;
    transition: width var(--mid) var(--ease);
  }
  /* Started, how far not known yet (a send waiting to be accepted): a soft breathing bar. */
  .fill.wait {
    opacity: 0.45;
    animation: breathe 1.6s var(--ease) infinite alternate;
  }
  @keyframes breathe {
    to {
      opacity: 0.2;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .fill.wait {
      animation: none;
    }
  }
  .size {
    font-size: 12px;
    color: var(--text-2);
    text-align: right;
  }
  .state {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acts {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--fast) ease;
  }
  .row:hover .acts,
  .row.on .acts,
  .acts:focus-within {
    opacity: 1;
  }
  .icon-btn.danger:hover {
    color: var(--st-fail-text);
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 40px;
    text-align: center;
  }
  .emark {
    width: 46px;
    height: 46px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
    color: var(--priv-text);
    margin-bottom: 6px;
  }
  .et {
    margin: 0;
    font-weight: 600;
  }
  .es {
    margin: 0;
    max-width: 420px;
    color: var(--text-2);
    line-height: 1.55;
  }
  .msg {
    margin: 0;
    color: var(--text-2);
    line-height: 1.55;
  }
  .dim {
    color: var(--text-3);
  }
  .small {
    font-size: 12px;
  }
  /* A narrow window (or the details panel open beside it): the name keeps its room; the size
     column goes, and the row's buttons show over the date on hover. */
  .list {
    container-type: inline-size;
  }
  /* The columns as you set them (columns.ts): Size goes first when the room is short, and a
     column you hid never shows. */
  .list.h-size .c-size,
  .list.h-date .c-date {
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
  .list.easing .head {
    transition: grid-template-columns var(--mid) var(--ease);
  }
  .list.easing .row {
    transition:
      grid-template-columns var(--mid) var(--ease),
      background var(--fast) ease,
      --zb var(--mid) ease;
  }
  /* Meanwhile a column opening up keeps its content inside it (the titles keep their edges). */
  .list.easing .row > * {
    overflow: hidden;
  }
  .head .hc {
    position: relative;
    align-self: stretch;
    display: flex;
    align-items: center;
  }
  /* Each column after Name keeps the room before it on its left (see columns.ts, gap). */
  .c-size,
  .c-date,
  .acts,
  .colcell {
    margin-left: 12px;
    min-width: 0;
  }
  .row > :global(.box),
  .head > .box {
    justify-self: start;
  }
  .under {
    display: block;
    margin-top: 2px;
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .head .hc.r {
    justify-content: flex-end;
  }
  .colcell {
    display: flex;
    justify-content: flex-end;
  }
  @container (max-width: 560px) {
    /* The row's buttons move over the date (their column keeps only the columns button). */
    .acts {
      position: absolute;
      right: 10px;
      background: var(--panel);
      border-radius: 6px;
    }
    .row {
      position: relative;
    }
    /* Over the date: only for the row under the mouse (not every picked one). */
    .row.on .acts {
      opacity: 0;
    }
    .row:hover .acts,
    .acts:focus-within {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pv {
      animation: none;
    }
    .fold {
      transition: none;
    }
  }
</style>
