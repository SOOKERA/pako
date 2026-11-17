<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { onMount, tick } from "svelte";
  import { app } from "$lib/state.svelte";
  import { api, inApp, isActive, listen, type Download, type PakoLinkResult, type Priority } from "$lib/api";
  import { bytes, findLinks, scheduleOpen } from "$lib/format";
  import type { MenuItem } from "$lib/menu";
  import Icon from "$lib/Icon.svelte";
  import TitleBar from "$lib/TitleBar.svelte";
  import Sidebar from "$lib/Sidebar.svelte";
  import Table from "$lib/Table.svelte";
  import DetailsPanel from "$lib/DetailsPanel.svelte";
  import UploadDetails from "$lib/UploadDetails.svelte";
  import BottomBar from "$lib/BottomBar.svelte";
  import ContextMenu from "$lib/ContextMenu.svelte";
  import AddDialog from "$lib/AddDialog.svelte";
  import SettingsDialog from "$lib/SettingsDialog.svelte";
  import RenameDialog from "$lib/RenameDialog.svelte";
  import Modal from "$lib/Modal.svelte";
  import Toasts from "$lib/Toasts.svelte";
  import FoundView from "$lib/FoundView.svelte";
  import FoundDetails from "$lib/FoundDetails.svelte";
  import SetupDialog from "$lib/SetupDialog.svelte";
  import TermsDialog from "$lib/TermsDialog.svelte";
  import DashboardView from "$lib/DashboardView.svelte";
  import SendView from "$lib/SendView.svelte";
  import DirectView from "$lib/DirectView.svelte";
  import DevicesView from "$lib/DevicesView.svelte";
  import DirectPrompts from "$lib/DirectPrompts.svelte";
  import HelpDialog from "$lib/help/HelpDialog.svelte";
  import WhatsNewDialog from "$lib/help/WhatsNewDialog.svelte";
  import ReportDialog from "$lib/help/ReportDialog.svelte";
  import { help } from "$lib/help/help.svelte";
  import { whatsNewFor } from "$lib/help/content";
  import TourOverlay from "$lib/tour/TourOverlay.svelte";
  import TutorialButton from "$lib/tour/TutorialButton.svelte";
  import { tour } from "$lib/tour/tour.svelte";

  let titleBar: ReturnType<typeof TitleBar> | undefined = $state();
  let addText = $state<string | null>(null);
  let settingsOpen = $state<false | "general" | "accounts" | "browser" | "notices">(false);
  /** The extension's "Open Pako" button (a pako:// link): how connecting went. */
  let linkResult = $state<PakoLinkResult | null>(null);

  /** Shows what a pako:// link did: Settings › Browser extension with a short confirmation
   *  (during the first-start setup, just a note). */
  async function takePakoLink() {
    const r = await api.takePakoLink().catch(() => null);
    if (!r) return;
    await app.refresh();
    if (trayAsk) {
      if (r.ok) app.toast("Connected. Go back to your browser and open Pako's popup again.");
      else app.toast(`Couldn't connect: ${r.error ?? "unknown problem"}`, "error");
      return;
    }
    // Opened fresh, so it shows the switch now on (not an older copy of the settings).
    settingsOpen = false;
    await tick();
    linkResult = r;
    settingsOpen = "browser";
  }
  let renaming = $state<Download | null>(null);
  let removing = $state<Download[] | null>(null);
  /** Ticks while the sleep / shut-down countdown is on screen. */
  let clock = $state(Date.now());
  $effect(() => {
    if (!app.whenDoneCountdown) return;
    const t = setInterval(() => (clock = Date.now()), 250);
    return () => clearInterval(t);
  });

  /** "Quit while downloading / uploading?": how many of each are running. */
  let quitAsk = $state<{ running: number; uploading: number; direct: boolean } | null>(null);
  let quitDontAsk = $state(false);
  async function quitAnyway() {
    if (quitDontAsk && app.settings) await app.run(() => api.setSettings({ ...app.settings!, confirmQuit: false }));
    await api.quitNow();
  }
  let restarting = $state<Download[] | null>(null);
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  onMount(() => {
    const stop = app.start();
    // "Settings…" from the tray icon's menu or the compact list.
    // Not over the first-start setup: Settings would open under it.
    const un = listen("open-settings", () => {
      if (!trayAsk) settingsOpen = "general";
    });
    const unQuit = listen<{ running?: number; uploading?: number; direct?: boolean }>("confirm-quit", (e) => {
      quitAsk = { running: e.payload.running ?? 0, uploading: e.payload.uploading ?? 0, direct: !!e.payload.direct };
    });
    // A video or "all links on this page" from the browser extension: show Links.
    const unView = listen<"found" | "downloads">("open-view", (e) => (app.view = e.payload));
    // Pako opened (or started) by the extension's "Open Pako" button.
    const unLink = listen("pako-link", () => takePakoLink());
    takePakoLink();
    return () => {
      unLink.then((f) => f());
      stop?.();
      un.then((f) => f());
      unQuit.then((f) => f());
      unView.then((f) => f());
    };
  });

  // ---------- dropping links, text or .txt files onto the window ----------
  let dropping = $state(false);
  const OWN_DRAG = "application/x-pako-ids";
  function isOutsideDrop(e: DragEvent) {
    const t = e.dataTransfer?.types ?? [];
    return !t.includes(OWN_DRAG) && (t.includes("Files") || t.includes("text/uri-list") || t.includes("text/plain"));
  }
  /** Upload and Direct take dropped files in their own page area (they stop the event there).
   *  A file or link dragged anywhere else on those pages (the sidebar, the top bar, the details
   *  panel, a dialog) reaches this window handler: left alone, the browser part of the window
   *  would open what was dropped in place of Pako. So it's refused, except text dragged into a
   *  field. */
  function refusedHere(e: DragEvent) {
    // The terms of use wait: nothing dropped is taken (nor opened in place of Pako).
    if (termsAsk) {
      e.preventDefault();
      if (e.dataTransfer) e.dataTransfer.dropEffect = "none";
      return true;
    }
    if (app.view !== "send" && app.view !== "direct" && app.view !== "devices") return false;
    const files = e.dataTransfer?.types.includes("Files");
    const field = e.target instanceof Element && !!e.target.closest("input, textarea");
    if (!files && field) return true; // text into a field: the field's own business
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "none";
    return true;
  }
  function onDragOver(e: DragEvent) {
    if (!isOutsideDrop(e) || refusedHere(e)) return;
    e.preventDefault();
    dropping = true;
  }
  function onDragLeave(e: DragEvent) {
    if (!e.relatedTarget) dropping = false;
  }
  async function onDrop(e: DragEvent) {
    if (!isOutsideDrop(e)) return;
    dropping = false;
    if (refusedHere(e)) return;
    e.preventDefault();
    const dt = e.dataTransfer!;
    let text = [dt.getData("text/uri-list"), dt.getData("text/plain")].filter(Boolean).join("\n");
    // Text files (link lists, .url shortcuts, saved pages): read them for links. Big files skipped.
    for (const f of Array.from(dt.files)) {
      if (f.size < 5_000_000 && /\.(txt|url|htm|html|csv|md|json|log)$/i.test(f.name)) text += "\n" + (await f.text());
    }
    if (findLinks(text).length) app.addToFound(text);
    else app.toast("No links found in what was dropped", "error");
  }

  // ---------- toolbar ----------
  const title = $derived.by(() => {
    if (app.sourceFilter) return app.sources.find((s) => s.key === app.sourceFilter)?.label ?? "Downloads";
    return { all: "All downloads", active: "Downloading", waiting: "Waiting", done: "Finished", problems: "Problems" }[app.library];
  });
  const sel = $derived(app.selectedDownloads);
  const selSize = $derived(sel.reduce((s, d) => s + (d.size ?? 0), 0));
  const canResume = (d: Download) => d.status === "paused" || d.status === "failed" || d.status === "quotaExceeded" || d.status === "needsAccount";
  const canPause = (d: Download) => isActive(d.status) || d.status === "queued";
  // Nothing selected: the downloads shown under the current filter/search, never hidden ones.
  const resumeTargets = $derived((sel.length ? sel : app.visible).filter(canResume));
  const pauseTargets = $derived((sel.length ? sel : app.visible).filter(canPause));
  const filtered = $derived(app.visible.length !== app.downloads.length);
  const n = (list: unknown[]) => (!list.length ? "" : sel.length || filtered ? ` ${list.length}` : " all");

  // ---------- actions ----------
  function askRemove(ids: string[]) {
    const ds = app.downloads.filter((d) => ids.includes(d.id));
    if (ds.length) removing = ds;
  }
  async function doRemove(deleteFiles: boolean) {
    const list = removing!;
    const ids = list.map((d) => d.id);
    removing = null;
    try {
      await api.remove(ids, deleteFiles);
    } catch (e) {
      // Not removed: say why, and don't offer to undo something that didn't happen.
      app.toast(String(e), "error");
      await app.refresh();
      return;
    }
    app.clearSelection();
    await app.refresh();
    // Finished files only taken off the list (nothing deleted): that can be undone.
    if (!deleteFiles && list.every((d) => d.status === "completed")) {
      app.offerUndo(list.length === 1 ? "Removed from the list" : `${list.length} removed from the list`, () => api.restoreDownloads(list));
    }
  }
  function askStartOver(ds: Download[]) {
    if (ds.some((d) => d.status === "completed" || d.downloaded > 0)) restarting = ds;
    else app.run(() => api.startOver(ds.map((d) => d.id)));
  }
  async function doStartOver() {
    const ids = restarting!.map((d) => d.id);
    restarting = null;
    await app.run(() => api.startOver(ids));
  }
  async function copyLinks(ds: Download[]) {
    try {
      await navigator.clipboard.writeText(ds.map((d) => d.link).join("\n"));
      app.toast(ds.length === 1 ? "Link copied" : `${ds.length} links copied`);
    } catch {
      app.toast("Couldn't copy to the clipboard", "error");
    }
  }

  // ---------- right-click menu (lean, JDownloader-style) ----------
  function openMenu(e: MouseEvent, ids: string[], clicked?: string) {
    const ds = app.downloads.filter((d) => ids.includes(d.id));
    if (!ds.length) return;
    const one = ds.length === 1 ? ds[0] : null;
    const count = ds.length > 1 ? ` ${ds.length}` : "";
    const idsOf = (pred: (d: Download) => boolean) => ds.filter(pred).map((d) => d.id);
    const startable = idsOf((d) => d.status !== "completed" && !isActive(d.status));
    const pausable = idsOf(canPause);
    const resumable = idsOf(canResume);
    const anyFailed = ds.some((d) => d.status === "failed" || d.status === "quotaExceeded" || d.status === "needsAccount");
    const anyDone = ds.some((d) => d.status === "completed");

    const items: MenuItem[] = [];
    if (startable.length) items.push({ label: "Download first", icon: "up", action: () =>
          app.run(() => api.downloadFirst(startable)).then(() => {
            if (!scheduleOpen(app.settings)) app.toast("Starting now, outside the scheduled download hours");
          }) });
    if (pausable.length) items.push({ label: `Pause${count}`, icon: "pause", action: () => app.run(() => api.pause(pausable)) });
    if (resumable.length)
      items.push({ label: anyFailed ? `Try again${count}` : `Resume${count}`, icon: anyFailed ? "retry" : "play", action: () => app.run(() => api.resume(resumable)) });
    items.push({ label: "Start over", icon: "restart", action: () => askStartOver(ds) });
    items.push({ separator: true });
    if (one?.status === "completed") items.push({ label: "Open file", icon: "open", hint: "Enter", action: () => app.run(() => api.openItem(one.id, false)) });
    items.push({ label: "Show in folder", icon: "folder", disabled: !one, action: () => one && app.run(() => api.openItem(one.id, true)) });
    if (one) items.push({ label: "Open link in browser", icon: "globe", action: () => app.run(() => api.openLink(one.id)) });
    items.push({ label: ds.length > 1 ? `Copy ${ds.length} links` : "Copy link", icon: "link", action: () => copyLinks(ds) });
    if (one) items.push({ label: "Rename…", icon: "pencil", hint: "F2", disabled: isActive(one.status), action: () => (renaming = one) });
    // Google only gives it in the browser, or today's limit is reached: its page in Drive
    // (download there, or make a copy in your Drive: Pako catches that download).
    if (one?.status === "needsAccount" || one?.status === "quotaExceeded") {
      items.push({ separator: true });
      items.push({ label: one.status === "quotaExceeded" ? "Open in Drive (make a copy)" : "Open in browser to download", icon: "globe", action: () => app.run(() => api.openInDrive(one.id)) });
    }
    items.push({ separator: true });
    const unfinished = ds.filter((d) => d.status !== "completed");
    if (unfinished.length) {
      const ids = unfinished.map((d) => d.id);
      const notifying = unfinished.every((d) => d.notify);
      items.push({ label: "Notify when done", icon: "bell", hint: notifying ? "✓" : "", action: () => app.run(() => api.setNotify(ids, !notifying)) });
      const all = (p: Priority) => unfinished.every((d) => d.priority === p);
      items.push({ label: "Priority: First", icon: "up", hint: all("first") ? "✓" : "", action: () => app.run(() => api.setPriority(ids, "first")) });
      items.push({ label: "Priority: Normal", icon: "minus", hint: all("normal") ? "✓" : "", action: () => app.run(() => api.setPriority(ids, "normal")) });
      items.push({ label: "Priority: Later", icon: "down", hint: all("later") ? "✓" : "", action: () => app.run(() => api.setPriority(ids, "later")) });
      items.push({ separator: true });
    }
    items.push({ label: "Move to top of queue", icon: "top", action: () => app.run(() => api.moveInQueue(ds.map((d) => d.id), true)).then(() => app.queueHint()) });
    items.push({ label: "Move to bottom of queue", icon: "bottom", action: () => app.run(() => api.moveInQueue(ds.map((d) => d.id), false)).then(() => app.queueHint()) });
    items.push({ separator: true });
    // Right-clicked inside a multi-selection: also offer to remove just the row under the mouse.
    const under = clicked && ds.length > 1 ? ds.find((d) => d.id === clicked) : undefined;
    if (under) items.push({ label: "Remove only this one", icon: "trash", danger: true, action: () => askRemove([under.id]) });
    items.push({ label: anyDone ? `Remove${count}…` : `Remove${count}`, icon: "trash", hint: ds.length > 1 ? "" : "Del", danger: true, action: () => askRemove(ds.map((d) => d.id)) });
    menu = { x: e.clientX, y: e.clientY, items };
  }

  // ---------- keyboard ----------
  function typing() {
    const el = document.activeElement;
    return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA");
  }
  /** A button, link or field has the keyboard focus: Enter belongs to it, not to the list. */
  function onControl() {
    const el = document.activeElement;
    return !!el && !!el.closest("button, a[href], input, textarea, select, [contenteditable], [role='switch']");
  }
  // The terms of use, before anything else (a new Pako, or once for one used before it had
  // terms): nothing else shows or runs until they're accepted.
  const termsAsk = $derived(app.loaded && inApp && app.termsNeeded);
  // First start (or "Run setup again"): the setup window.
  const trayAsk = $derived(!termsAsk && app.loaded && !!app.settings && ((inApp && !app.settings.setupDone) || app.setupOpen));
  /** Video downloading arrived after the first-start setup: offered once. */
  const videoAsk = $derived(!termsAsk && app.loaded && !!app.settings && inApp && app.settings.setupDone && !app.settings.videoAsked);
  async function answerVideo(install: boolean) {
    if (!app.settings) return;
    await app.run(() => api.setSettings({ ...app.settings!, videoAsked: true }));
    await app.refresh();
    if (install) {
      app.toast("Getting the video tools ready in the background…");
      app.installVideoTools();
    }
  }

  // ---------- Help: What's new once after an update, then the tour offered once ----------
  let whatsNewDone = $state(false);
  const whatsNewEntry = $derived(app.version ? whatsNewFor(app.version) : undefined);
  const whatsNewShow = $derived(
    !whatsNewDone && !termsAsk && !!whatsNewEntry && app.loaded && !!app.settings && inApp && app.settings.setupDone && !trayAsk && !videoAsk && app.settings.whatsNewSeen !== app.version,
  );
  function closeWhatsNew() {
    whatsNewDone = true;
    if (app.settings) app.settings = { ...app.settings, whatsNewSeen: app.version };
    api.helpSeen({ whatsNew: app.version }).catch(() => {});
  }
  let tourAskDone = $state(false);
  const tourAsk = $derived(
    !tourAskDone && !termsAsk && app.loaded && !!app.settings && inApp && app.settings.setupDone && !app.settings.tourOffered && !trayAsk && !videoAsk && !whatsNewShow && !help.open && !tour.active,
  );
  function answerTour(take: boolean) {
    tourAskDone = true;
    if (app.settings) app.settings = { ...app.settings, tourOffered: true };
    api.helpSeen({ tourOffered: true }).catch(() => {});
    if (take) tour.start("full");
  }

  /** A dialog or menu is open: the list behind it takes no shortcuts or pastes. */
  function covered() {
    return termsAsk || app.modals > 0 || addText !== null || !!settingsOpen || !!renaming || !!removing || !!restarting || trayAsk || !!menu || help.open || tour.active || app.bellOpen;
  }
  function onKey(e: KeyboardEvent) {
    // Already handled (Enter in a menu or the Rename box): don't also open the file.
    if (e.defaultPrevented) return;
    if (e.key === "F1") {
      e.preventDefault();
      if (!covered()) help.show();
      return;
    }
    if (covered()) return;
    const ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && (e.key.toLowerCase() === "f" || e.key.toLowerCase() === "k")) {
      e.preventDefault();
      titleBar?.focusSearch();
    } else if (ctrl && e.key.toLowerCase() === "z" && !typing()) {
      e.preventDefault();
      app.undo();
    } else if (ctrl && e.key.toLowerCase() === "i") {
      e.preventDefault();
      app.togglePanel();
    } else if (ctrl && e.key === "Tab") {
      // Switch between Downloads and Found.
      e.preventDefault();
      app.view = app.view === "found" ? "downloads" : "found";
    } else if (typing()) {
      return;
    } else if (app.view !== "downloads" && app.view !== "found") {
      // Upload, Direct and the Dashboard have no list these keys move in: they must not act on
      // the Downloads list hidden behind them (Space would tick a download you can't see).
      return;
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      app.moveFocus(e.key === "ArrowDown" ? 1 : -1, e.shiftKey);
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      app.moveFocus(e.key === "End" ? Infinity : -Infinity, e.shiftKey);
    } else if (e.key === " " && !onControl()) {
      e.preventDefault();
      app.toggleFocused();
    } else if (e.key === "Delete" && app.view === "found") {
      removeFocusedFound();
    } else if (app.view === "found" && ctrl && e.key.toLowerCase() === "a") {
      e.preventDefault();
      app.setTicked(app.foundVisible, true);
    } else if (app.view === "found" && e.key === "Escape") {
      app.setTicked(app.foundVisible, false);
    } else if (app.view !== "downloads") {
      return;
    } else if (ctrl && e.key.toLowerCase() === "a") {
      e.preventDefault();
      app.selectAll();
    } else if (e.key === "Escape") {
      app.clearSelection();
    } else if (e.key === "Delete") {
      // The focused row (the one last clicked), not the ticked ones.
      const folder = !app.focused && app.focusFolder ? app.rows.find((r) => r.kind === "folder" && r.key === app.focusFolder) : undefined;
      const ids = app.focused ? [app.focused.id] : folder && folder.kind === "folder" ? folder.files.map((d) => d.id) : [];
      if (ids.length) askRemove(ids);
    } else if (e.key === "F2" && app.focused && !isActive(app.focused.status)) {
      renaming = app.focused;
    } else if (e.key === "Enter" && !onControl() && app.focused?.status === "completed") {
      app.run(() => api.openItem(app.focused!.id, false));
    }
  }
  /** Delete in Found: removes the focused link (or the focused folder's files) at once,
   *  and moves the focus to the next row so Delete can be pressed again. */
  async function removeFocusedFound() {
    const items = app.withMirrors(app.foundFocusItems);
    if (!items.length) return;
    const gone = new Set(items.map((f) => f.id));
    const keys = app.foundRows
      .filter((r) => r.kind !== "child" || !app.foundCollapsed.has(r.folderKey))
      .map((r) => (r.kind === "folder" ? "g:" + r.key : r.f.id));
    const at = keys.indexOf(app.foundFocus ?? "");
    const next = [...keys.slice(at + 1), ...keys.slice(0, Math.max(at, 0)).reverse()].find(
      (k) => !gone.has(k) && !(k.startsWith("g:") && app.foundFocus === k),
    );
    await app.removeFound([...gone]);
    app.foundFocus = next ?? null;
  }

  /** Right-click on empty space in the Downloads list. */
  function emptyMenu(e: MouseEvent) {
    e.preventDefault();
    const items: MenuItem[] = [
      {
        label: "Paste links",
        icon: "link",
        hint: "Ctrl+V",
        action: async () => {
          try {
            const text = await navigator.clipboard.readText();
            if (text.trim()) await app.addToFound(text);
            else addText = "";
          } catch {
            addText = "";
          }
        },
      },
      { label: "Add links…", icon: "plus", action: () => (addText = "") },
      { separator: true },
      { label: "Select all", icon: "check", hint: "Ctrl+A", disabled: !app.visible.length, action: () => app.selectAll() },
      { label: "Clear finished", icon: "trash", disabled: !app.counts.done, action: () => app.run(() => api.clearCompleted()) },
    ];
    menu = { x: e.clientX, y: e.clientY, items };
  }

  function onPaste(e: ClipboardEvent) {
    if (typing() || covered()) return;
    const text = e.clipboardData?.getData("text") ?? "";
    if (findLinks(text).length) {
      e.preventDefault();
      app.addToFound(text);
    }
  }
</script>

<svelte:window onkeydown={onKey} onpaste={onPaste} ondragover={onDragOver} ondragleave={onDragLeave} ondrop={onDrop} />

{#if dropping}
  <div class="dropzone" aria-hidden="true">
    <div class="dropcard"><Icon name="link" size={22} />Drop links or a text file to add them to Links</div>
  </div>
{/if}

<div class="app">
  <TitleBar bind:this={titleBar} onpastelinks={(t) => app.addToFound(t)} onnoticesettings={() => (settingsOpen = "notices")} />
  <div class="main">
    <Sidebar onsettings={(s) => (settingsOpen = s ?? "general")} onmenu={(x, y, items) => (menu = { x, y, items })} />

    <section class="content">
      {#if app.view === "dashboard"}
        <DashboardView onmenu={(x, y, items) => (menu = { x, y, items })} />
      {:else if app.view === "found"}
        <FoundView onadd={() => (addText = "")} onmenu={(x, y, items) => (menu = { x, y, items })} />
      {:else if app.view === "send"}
        <SendView onmenu={(x, y, items) => (menu = { x, y, items })} />
      {:else if app.view === "direct"}
        <DirectView onmenu={(x, y, items) => (menu = { x, y, items })} />
      {:else if app.view === "devices"}
        <DevicesView />
      {:else}
      <div class="toolbar">
        <h1>{title}</h1>
        <TutorialButton id="downloads" />
        {#if sel.length}
          <div class="selchip">
            <span><b>{sel.length} selected</b>{#if selSize}<span class="dim">{` · ${bytes(selSize)}`}</span>{/if}</span>
            <button aria-label="Clear selection" onclick={() => app.clearSelection()}><Icon name="x" size={10} stroke={2.4} /></button>
          </div>
        {:else}
          <span class="dim small count-text">{app.downloads.length} {app.downloads.length === 1 ? "item" : "items"}</span>
        {/if}
        <div class="spacer"></div>
        <div class="seg" data-tour="dl-actions">
          <button title="Resume{n(resumeTargets)}" disabled={!resumeTargets.length} onclick={() => app.run(() => api.resume(resumeTargets.map((d) => d.id)))}>
            <Icon name="play" size={12} /><span class="lbl">Resume{n(resumeTargets)}</span>
          </button>
          <button title="Pause{n(pauseTargets)}" disabled={!pauseTargets.length} onclick={() => app.run(() => api.pause(pauseTargets.map((d) => d.id)))}>
            <Icon name="pause" size={12} /><span class="lbl">Pause{n(pauseTargets)}</span>
          </button>
          <button title="Remove selected" class="danger" disabled={!sel.length} onclick={() => askRemove(sel.map((d) => d.id))}>
            <Icon name="trash" size={12} /><span class="lbl">Remove{sel.length ? ` ${sel.length}` : ""}</span>
          </button>
        </div>
        <button class="btn primary add" data-tour="dl-add" title="Add links" onclick={() => (addText = "")}><Icon name="plus" size={14} stroke={2.4} /><span class="lbl">Add links</span></button>
      </div>

      <Table oncontext={openMenu} onadd={() => (addText = "")} onempty={emptyMenu} />
      {/if}
      <BottomBar />
    </section>

    <!-- inert when closed: Tab can't reach the hidden panel's buttons. -->
    <aside
      class="side"
      data-tour="details"
      class:open={app.panelOpen && app.view !== "dashboard" && app.view !== "direct" && app.view !== "devices"}
      aria-hidden={!app.panelOpen || app.view === "dashboard" || app.view === "direct" || app.view === "devices"}
      inert={!app.panelOpen || app.view === "dashboard" || app.view === "direct" || app.view === "devices"}
    >
      {#if app.view === "found"}
        <FoundDetails />
      {:else if app.view === "send"}
        <UploadDetails />
      {:else if app.view !== "devices"}
        <!-- Never on My devices, even while the panel slides away: it has no details panel. -->
        <DetailsPanel onremove={askRemove} />
      {/if}
    </aside>
  </div>
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}

{#if addText !== null}
  <AddDialog text={addText} onclose={() => (addText = null)} />
{/if}

{#if settingsOpen && app.settings}
  <SettingsDialog
    section={settingsOpen === "accounts" || settingsOpen === "browser" || settingsOpen === "notices" ? settingsOpen : undefined}
    {linkResult}
    onclose={() => {
      settingsOpen = false;
      linkResult = null;
    }}
  />
{/if}

{#if renaming}
  <RenameDialog d={renaming} onclose={() => (renaming = null)} />
{/if}

{#if app.whenDoneCountdown}
  {@const cd = app.whenDoneCountdown}
  {@const left = Math.max(0, Math.ceil((cd.endsAt - clock) / 1000))}
  <Modal title={cd.action === "sleep" ? `Going to sleep in ${left} s` : `Shutting down in ${left} s`} width={440} onclose={() => app.cancelWhenDone()}>
    <p class="msg">All downloads are done. {cd.action === "sleep" ? "The PC goes to sleep" : "The PC shuts down"} unless you cancel.</p>
    {#snippet actions()}
      <button class="btn primary" onclick={() => app.cancelWhenDone()}>Cancel</button>
    {/snippet}
  </Modal>
{/if}

{#if quitAsk !== null}
  {@const dn = quitAsk.running}
  {@const up = quitAsk.uploading}
  {@const downs = dn === 1 ? "1 download" : `${dn} downloads`}
  {@const ups = up === 1 ? "1 upload" : `${up} uploads`}
  {@const dt = quitAsk.direct}
  <Modal
    title={dt && (dn || up) ? "Pako is still busy" : dt ? "Files are on their way in Direct" : dn && up ? `${downs} and ${ups} are running` : up ? (up === 1 ? "An upload is running" : `${ups} are running`) : dn === 1 ? "A download is running" : `${downs} are running`}
    width={460}
    onclose={() => (quitAsk = null)}
  >
    {#if dn}
      <p class="msg">If you quit, {dn === 1 ? "the download stops" : "the downloads stop"} and {dn === 1 ? "continues" : "continue"} where {dn === 1 ? "it" : "they"} left off next time you open Pako.</p>
    {/if}
    {#if up}
      <p class="msg">
        {dn ? "The" : "If you quit, the"} {up === 1 ? "upload stops" : "uploads stop"} and {up === 1 ? "doesn't" : "don't"} continue next time: upload {up === 1 ? "it" : "them"} again from Upload.
        One file or a ZIP starts over from the beginning; of several files, the ones already up stay on Gofile.
      </p>
    {/if}
    {#if dt}
      <p class="msg">{dn || up ? "Files going to or from your contacts in Direct stop too" : "If you quit, files going to or from your contacts in Direct stop"}: they have to be sent again.</p>
    {/if}
    <button class="dont-ask" role="checkbox" aria-checked={quitDontAsk} onclick={() => (quitDontAsk = !quitDontAsk)}>
      <span class="check" class:on={quitDontAsk} aria-hidden="true">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
      </span>
      Don't ask again
    </button>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (quitAsk = null)}>{dt || (dn && up) ? "Don't quit" : up ? "Keep uploading" : "Keep downloading"}</button>
      <button class="btn danger-fill" onclick={quitAnyway}>Quit</button>
    {/snippet}
  </Modal>
{/if}

{#if removing}
  {@const done = removing.filter((d) => d.status === "completed").length}
  <Modal title={removing.length === 1 ? `Remove "${removing[0].name}"?` : `Remove ${removing.length} downloads?`} width={460} onclose={() => (removing = null)}>
    <p class="msg">
      {#if done && done === removing.length}
        The {done === 1 ? "file stays" : "files stay"} in {done === 1 ? "its folder" : "their folders"} unless you delete {done === 1 ? "it" : "them"} too (to the Recycle Bin).
      {:else if done}
        Unfinished parts are deleted. The {done} finished {done === 1 ? "file stays" : "files stay"} unless you delete them too (to the Recycle Bin).
      {:else}
        What was downloaded so far will be deleted.
      {/if}
    </p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (removing = null)}>Cancel</button>
      {#if done}
        <button class="btn danger" onclick={() => doRemove(true)}><Icon name="trash" size={13} />Delete {done === 1 ? "file" : "files"} too</button>
        <button class="btn primary" onclick={() => doRemove(false)}>Remove from list</button>
      {:else}
        <button class="btn danger-fill" onclick={() => doRemove(false)}>Remove</button>
      {/if}
    {/snippet}
  </Modal>
{/if}

{#if restarting}
  <Modal title="Start over?" width={440} onclose={() => (restarting = null)}>
    <p class="msg">
      {restarting.length === 1 ? "This download starts" : `These ${restarting.length} downloads start`} again from zero.
      {#if restarting.some((d) => d.status === "completed")}Finished files go to the Recycle Bin first.{/if}
    </p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (restarting = null)}>Cancel</button>
      <button class="btn primary" onclick={doStartOver}>Start over</button>
    {/snippet}
  </Modal>
{/if}

{#if termsAsk}
  <TermsDialog />
{:else if trayAsk}
  <SetupDialog />
{:else if videoAsk}
  <Modal title="Pako can download videos now" width={460} onclose={() => answerVideo(false)}>
    <p class="msg">Paste a YouTube (or other video site) link or playlist, pick the quality or audio only, and Pako does the rest.</p>
    <p class="msg small-note">It uses three free tools, yt-dlp, Deno and ffmpeg (about 200 MB), downloaded from their official sources. You can also turn this on later in Settings › Features &amp; updates.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => answerVideo(false)}>Not now</button>
      <button class="btn primary" onclick={() => answerVideo(true)}>Install video tools</button>
    {/snippet}
  </Modal>
{/if}

{#if help.open}
  <HelpDialog />
{:else if whatsNewShow && whatsNewEntry}
  <WhatsNewDialog entry={whatsNewEntry} version={app.version} onclose={closeWhatsNew} />
{:else if tourAsk}
  <Modal title="Take a quick tour?" width={440} onclose={() => answerTour(false)}>
    <p class="msg">A spotlight shows what each page of Pako is for. It takes about a minute.</p>
    <p class="msg small-note">Each page also has a Tutorial button that shows its details, and Help has them all.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => answerTour(false)}>Not now</button>
      <button class="btn primary" onclick={() => answerTour(true)}>Take the tour</button>
    {/snippet}
  </Modal>
{/if}
<!-- Help › Report a problem, over Help. -->
{#if help.report}
  <ReportDialog />
{/if}

<TourOverlay settingsOpen={!!settingsOpen} onsettings={(open) => (settingsOpen = open ? (settingsOpen || "general") : false)} />

<Toasts />

<DirectPrompts />

<style>
  .small-note {
    margin-top: 10px;
    font-size: 12.5px;
    color: var(--text-3);
  }
  .dont-ask {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
    padding: 0;
    border: none;
    background: none;
    font-size: 13px;
    color: var(--text-2);
    cursor: pointer;
  }
  .dropzone {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    background: var(--drop-scrim);
    border: 2px dashed var(--accent);
    pointer-events: none;
    animation: dz-in 120ms ease;
  }
  .dropcard {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 18px;
    border-radius: var(--radius-lg);
    background: var(--raise);
    border: 1px solid var(--line-strong);
    color: var(--text);
    font-size: 14px;
    font-weight: 500;
  }
  .dropcard :global(svg) {
    color: var(--accent);
  }
  @keyframes dz-in {
    from {
      opacity: 0;
    }
  }
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .main {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    container-type: inline-size;
  }
  .toolbar {
    height: 52px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 16px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  @container (max-width: 760px) {
    .toolbar .count-text {
      display: none;
    }
    .seg .lbl {
      display: none;
    }
    .seg button {
      padding: 0 10px;
    }
  }
  /* Smallest window with the details panel open: Add links keeps its icon and stays in view. */
  @container (max-width: 520px) {
    .toolbar {
      gap: 8px;
    }
    .selchip .dim,
    .add .lbl {
      display: none;
    }
    .toolbar .btn.add {
      width: 32px;
      padding: 0;
    }
  }
  h1 {
    min-width: 0;
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dim {
    color: var(--text-3);
  }
  .small {
    font-size: 12px;
  }
  .selchip {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 4px 0 10px;
    border-radius: var(--radius);
    background: var(--select);
    font-size: 12px;
    white-space: nowrap;
    animation: chip-in var(--mid) var(--ease);
  }
  .selchip b {
    font-weight: 600;
  }
  .selchip button {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-3);
    transition: background var(--fast) ease, color var(--fast) ease;
  }
  .selchip button:hover {
    background: var(--active);
    color: var(--text);
  }
  @keyframes chip-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
  .spacer {
    flex: 1;
  }
  .seg {
    flex: none;
    display: flex;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .seg button {
    height: 30px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    border: none;
    border-right: 1px solid var(--line);
    background: var(--raise);
    color: var(--text-2);
    white-space: nowrap;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .seg button:last-child {
    border-right: none;
  }
  .seg button:hover:not(:disabled) {
    background: var(--raise-hover);
    color: var(--text);
  }
  .seg button:active:not(:disabled) :global(svg) {
    transform: scale(0.85);
  }
  .seg button :global(svg) {
    transition: transform var(--fast) var(--ease);
  }
  .seg button:disabled {
    color: var(--text-4);
    cursor: default;
  }
  .seg button.danger:not(:disabled) {
    color: var(--st-fail-text);
  }
  .toolbar .btn.primary {
    flex: none;
    height: 32px;
  }

  /* Details panel slides in and out */
  .side {
    width: 0;
    flex: none;
    overflow: hidden;
    border-left: 1px solid transparent;
    transition:
      width var(--slow) var(--ease),
      border-color var(--slow) ease;
  }
  .side.open {
    width: 320px;
    border-left-color: var(--line);
  }
  .side :global(.panel) {
    opacity: 0;
    transform: translateX(12px);
    transition:
      opacity var(--mid) ease,
      transform var(--slow) var(--ease);
  }
  .side.open :global(.panel) {
    opacity: 1;
    transform: none;
  }
  .msg {
    margin: 0;
    line-height: 1.5;
    color: var(--text-2);
  }
  .msg + .msg {
    margin-top: 10px;
  }
</style>
