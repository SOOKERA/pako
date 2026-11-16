<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Send: big files to anyone with a link, through Gofile (no account) or your own Google Drive.
  // The Upload tab. Picked and dropped files gather in the "New upload" basket first; one Upload makes one link.
  // Files go as they are (several at a time into one folder, one link); packing them into one ZIP
  // is an option.
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api, listen, type SendJob, type SendMode, type SentItem } from "./api";
  import type { MenuItem } from "./menu";
  import { bytes, sentLook, shortDate, speed, timeLeft } from "./format";
  import Icon from "./Icon.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import Modal from "./Modal.svelte";
  import SendDriveBar from "./SendDriveBar.svelte";
  import SendBasket from "./SendBasket.svelte";
  import { fade } from "svelte/transition";
  import { flipIf, flipOpts, rowSlide } from "./motion";
  import { expiryLook, sevenMore } from "./uploads";

  let { onmenu }: { onmenu: (x: number, y: number, items: MenuItem[]) => void } = $props();

  /** "Deletes in 2 days" stays right while the window is open. */
  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(t);
  });
  /** Lots of rows: no row motion (each one would be measured). */
  const rowMotion = $derived(app.sent.length > 150 ? 0 : 220);
  /** "Delete downloaded ones": the items in the confirm dialog. */
  let bulk = $state<SentItem[] | null>(null);
  let bulkBusy = $state(false);

  // Download counts: asked for (in the background, at most every few minutes) when Send opens.
  let asked = false;
  $effect(() => {
    if (!asked && app.sent.length) {
      asked = true;
      app.checkSent(false);
    }
  });
  async function refreshCounts() {
    if (!(await app.checkSent(true))) app.toast("Downloads were checked less than a minute ago.");
  }
  function checkedTitle() {
    const at = app.sentCheckedAt;
    if (!at) return "Check downloads";
    const min = Math.round((Date.now() - at) / 60_000);
    return `Check downloads again (checked ${min < 1 ? "just now" : min === 1 ? "1 minute ago" : `${min} minutes ago`})`;
  }
  /** Can be deleted from Gofile right now. */
  function deletable(s: SentItem) {
    return !s.gone && (s.canDelete ?? (app.sendCanDelete && !!(s.folderId || s.fileId))) && !busy(s);
  }
  /** Downloaded at least once: safe to delete. */
  /** Uploaded items, without the ones still uploading (those show under "Uploading" only). */
  const sentList = $derived(app.sent.filter((s) => !app.sends.some((j) => j.id === s.id)));
  /** Gofile's keep rule only matters when something in the list is on Gofile. */
  const anyGofile = $derived(sentList.some((s) => (s.host ?? "gofile") === "gofile"));
  const downloaded = $derived(sentList.filter((s) => (s.host ?? "gofile") === "gofile" && (s.downloads ?? 0) > 0 && deletable(s)));

  /** Starts one send of these paths (the basket's). The basket empties once it has started. */
  async function startSend(paths: string[], m: SendMode | null, name: string | null, days: number) {
    try {
      await api.sendStart(paths, m, name, days);
      // Only what was sent leaves the basket: files added while it started stay for the next one.
      const sent = new Set(paths);
      app.basket = app.basket.filter((b) => !sent.has(b.path));
      await app.refreshSends();
    } catch (e) {
      app.toast(String(e), "error");
    }
  }
  /** The basket's Send: everything in it (but what can't be read), one send, one link. */
  async function sendBasket(m: SendMode | null, name: string | null, days: number) {
    const paths = app.basket.filter((b) => !b.error).map((b) => b.path);
    if (!paths.length) return;
    if (app.sendHost === "drive" && !(await fitsDrive(paths, m, name, days))) return;
    await startSend(paths, m, name, days);
  }

  /** The send doesn't fit in your Google Drive: what it needs and what's free. */
  let noRoom = $state<{ need: number; free: number; paths: string[]; mode: SendMode | null; name: string | null; days: number } | null>(null);
  let driveBar = $state<{ openRoom: () => void } | null>(null);
  /** Before a Google Drive send: connected, and it fits (said either way). */
  async function fitsDrive(paths: string[], m: SendMode | null, name: string | null, days: number): Promise<boolean> {
    try {
      const st = await api.sendDriveStatus();
      if (!st.configured) {
        app.toast(`${st.missingNote ?? "Uploading to Google Drive isn't set up in this Pako."} Upload to Gofile instead.`, "error");
        return false;
      }
      if (!st.signedIn) {
        app.toast("Connect Google Drive first (the button above the list), or upload to Gofile.", "error");
        return false;
      }
      if (st.limit == null || st.used == null) return true;
      const need = await api.sendMeasure(paths, m);
      const free = Math.max(0, st.limit - st.used);
      if (need > free) {
        noRoom = { need, free, paths, mode: m, name, days };
        return false;
      }
      app.toast(`This upload needs ${bytes(need)}, ${bytes(free)} free in your Google Drive`);
      return true;
    } catch (e) {
      app.toast(String(e), "error");
      return false;
    }
  }
  /** "Upload to Gofile instead": switches (remembered) and uploads these files. */
  async function sendWithGofile(paths: string[], m: SendMode | null, name: string | null, days: number) {
    noRoom = null;
    await app.setSendHost("gofile");
    await startSend(paths, m, name, days);
  }
  /** Where a sent item is, in words. */
  const where = (s: SentItem) => (s.host === "drive" ? "your Google Drive" : "Gofile");
  /** The pickers add to the basket; nothing is sent until its Send. */
  async function chooseFiles() {
    const r = await pick({ multiple: true, directory: false, title: "Add files to your upload" });
    await app.addToBasket(Array.isArray(r) ? r : r ? [r] : []);
  }
  async function chooseFolder() {
    const r = await pick({ multiple: true, directory: true, title: "Add a folder to your upload" });
    await app.addToBasket(Array.isArray(r) ? r : r ? [r] : []);
  }
  /** Sends being tried again right now: a second click does nothing. */
  let retrying = $state<string[]>([]);
  async function tryAgain(j: SendJob) {
    if (retrying.includes(j.id)) return;
    retrying = [...retrying, j.id];
    try {
      await app.run(async () => {
        // Several files that partly failed: only the missing ones, into the same folder.
        if (j.canResume && (await api.sendResume(j.id))) return;
        await api.sendDismiss(j.id);
        // The same way and with the same name as the first time (a one-off ZIP stays a ZIP).
        await app.send(j.paths, j.mode ?? (j.asFiles ? "files" : null), j.name, j.deleteDays ?? null, j.host ?? null);
      });
    } finally {
      retrying = retrying.filter((id) => id !== j.id);
    }
  }
  /** Still uploading: deleting it from Gofile now would only make the rest fail. */
  function busy(s: SentItem) {
    return app.sends.some((j) => j.id === s.id && j.status === "uploading");
  }

  function pct(j: SendJob) {
    const of = j.total ?? j.size;
    return of ? Math.min(100, (j.sent / of) * 100) : 0;
  }
  function what(n: number, size: number) {
    return `${n === 1 ? "1 file" : `${n.toLocaleString()} files`} · ${bytes(size)}`;
  }

  function sentMenu(e: MouseEvent, s: SentItem) {
    e.preventDefault();
    e.stopPropagation();
    app.sentPicked = s.id;
    const items: MenuItem[] = [
      { label: "Copy link", icon: "copy", action: () => app.copyLink(s.link) },
      { label: "Open the download page", icon: "open", action: () => app.run(() => api.sentOpen(s.id)) },
      { label: app.panelOpen ? "Hide details" : "Show details", icon: "list", action: () => app.togglePanel() },
      { separator: true },
      {
        label: "Remove from the list",
        icon: "x",
        action: () =>
          app.run(async () => {
            await api.sentRemove(s.id, false);
            app.toast(`Removed from the list. The file stays in ${where(s)}.`);
          }),
      },
    ];
    if (deletable(s)) {
      // "Delete after": move the date, keep it, or delete it now.
      const busy = app.changingSent.includes(s.id);
      const later: MenuItem[] =
        s.deleteAt != null
          ? [
              { label: "Keep 7 more days", icon: "clock", disabled: busy, action: () => app.setSentExpiry(s, sevenMore(s, Date.now())) },
              { label: "Keep (don't delete)", icon: "check", disabled: busy, action: () => app.setSentExpiry(s, null) },
            ]
          : [{ label: "Delete in 7 days", icon: "clock", disabled: busy, action: () => app.setSentExpiry(s, sevenMore(s, Date.now())) }];
      items.splice(items.length - 1, 0, ...later);
      items.push({ label: s.host === "drive" ? "Delete now from Google Drive…" : "Delete now from Gofile…", icon: "trash", action: () => (app.sentDeleting = s) });
    }
    onmenu(e.clientX, e.clientY, items);
  }

  async function deleteOnline() {
    const s = app.sentDeleting;
    app.sentDeleting = null;
    if (!s) return;
    await app.run(async () => {
      await api.sentRemove(s.id, true);
      app.toast(`Deleted "${s.name}" from ${where(s)}`);
    });
  }

  async function deleteDownloaded() {
    const list = bulk;
    if (!list || bulkBusy) return;
    bulkBusy = true;
    let done = 0;
    const failed: string[] = [];
    for (const s of list) {
      try {
        await api.sentRemove(s.id, true);
        done++;
      } catch (e) {
        failed.push(String(e));
      }
    }
    bulkBusy = false;
    bulk = null;
    const what = (n: number) => (n === 1 ? "1 item" : `${n} items`);
    if (!failed.length) app.toast(`Deleted ${what(done)} from Gofile`);
    else app.toast(`Deleted ${what(done)}. ${what(failed.length)} couldn't be deleted: ${failed[0]}`, "error");
  }

  async function removeFromList(s: SentItem) {
    await app.run(() => api.sentRemove(s.id, false));
  }

  // ---------- files and folders dragged in from Explorer ----------
  // A page never sees the paths of dropped files: the dropped items go to WebView2, which hands
  // their real paths to Pako (filedrop.rs); they come back as "files-dropped".
  let dropping = $state(false);
  let dragDepth = 0;
  const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes("Files");
  function dragEnter(e: DragEvent) {
    if (!hasFiles(e)) return;
    dragDepth++;
    dropping = true;
  }
  function dragOver(e: DragEvent) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer!.dropEffect = "copy";
    dropping = true;
  }
  function dragLeave(e: DragEvent) {
    if (!hasFiles(e)) return;
    if (--dragDepth <= 0) {
      dragDepth = 0;
      dropping = false;
    }
  }
  async function drop(e: DragEvent) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    e.stopPropagation();
    dragDepth = 0;
    dropping = false;
    const dt = e.dataTransfer!;
    const webview = (window as { chrome?: { webview?: { postMessageWithAdditionalObjects?: (m: string, o: unknown[]) => void } } }).chrome?.webview;
    if (!webview?.postMessageWithAdditionalObjects) {
      app.toast("Dropping files doesn't work here. Use Choose files or Choose folder.", "error");
      return;
    }
    // Read everything now: the drop's contents are gone once this handler waits.
    const files = Array.from(dt.files);
    type WithHandle = DataTransferItem & { getAsFileSystemHandle?: () => Promise<unknown> };
    const asked = Array.from(dt.items)
      .filter((i) => i.kind === "file")
      .map((i) => (i as WithHandle).getAsFileSystemHandle?.() ?? Promise.reject());
    if (!files.length && !asked.length) return;
    // Both kinds go: plain dropped files (Microsoft's documented way) and file-system handles
    // (they also carry a folder's path). Pako keeps each path once.
    const got = await Promise.allSettled(asked);
    const handles = got.flatMap((r) => (r.status === "fulfilled" && r.value ? [r.value] : []));
    try {
      webview.postMessageWithAdditionalObjects("pako-files", [...files, ...handles]);
    } catch {
      try {
        webview.postMessageWithAdditionalObjects("pako-files", files);
      } catch (e) {
        app.toast(`Pako couldn't read what was dropped (${String(e)}). Use Choose files or Choose folder.`, "error");
      }
    }
  }
  $effect(() => {
    const un = listen<{ paths: string[]; detail: string }>("files-dropped", (e) => {
      if (e.payload.paths.length) app.addToBasket(e.payload.paths);
      else app.toast(`Pako couldn't read what was dropped (${e.payload.detail}). Use Choose files or Choose folder.`, "error");
    });
    return () => {
      un.then((f) => f());
    };
  });
</script>

<div class="send" role="presentation" ondragenter={dragEnter} ondragover={dragOver} ondragleave={dragLeave} ondrop={drop}>
  {#if dropping}
    <div class="dropzone" aria-hidden="true">
      <div class="dropcard"><Icon name="upload" size={22} />Drop files or folders to add them to your upload</div>
    </div>
  {/if}
  <div class="toolbar">
    <h1>Upload</h1>
    <TutorialButton id="send" />
    {#if app.uploadingCount}
      <span class="dim small live-text"><span class="live"></span><span class="lt">{app.uploadingCount} uploading · {speed(app.uploadSpeed)}</span></span>
    {/if}
    <div class="spacer"></div>
    <button class="btn" title="Add a folder to your upload" onclick={chooseFolder}><Icon name="folder" size={14} /><span class="lbl">Choose folder</span></button>
    <button class="btn primary" data-tour="send-choose" title="Add files to your upload" onclick={chooseFiles}><Icon name="upload" size={14} stroke={2.2} /><span class="lbl">Choose files</span></button>
  </div>

  {#if app.sendHost === "drive"}
    <SendDriveBar bind:this={driveBar} />
  {/if}
  <div class="body">
    {#if app.sentListNote && !app.sentListNoteClosed}
      <div class="listnote" role="status" transition:rowSlide={{ duration: 240 }}>
        <Icon name="alert" size={14} />
        <span>{app.sentListNote}</span>
        <button class="icon-btn" aria-label="Close" title="Close" onclick={() => (app.sentListNoteClosed = true)}><Icon name="x" size={12} /></button>
      </div>
    {/if}
    {#if app.basket.length}
      <div transition:rowSlide={{ duration: 260 }}>
        <SendBasket onaddfiles={chooseFiles} onaddfolder={chooseFolder} onsend={sendBasket} onroom={() => driveBar?.openRoom()} />
      </div>
    {/if}
    {#if !app.sends.length && !app.sent.length && !app.basket.length}
      <div class="empty">
        <div class="empty-icon"><Icon name="upload" size={26} /></div>
        <h3>Share big files with a link</h3>
        {#if app.sendHost === "drive"}
          <p>Add files or folders (or drop them here), then press Upload. Pako uploads them to your Google Drive, shares them with anyone who has the link, and copies the link for you. Anyone can download it in their browser.</p>
        {:else}
          <p>Add files or folders (or drop them here), then press Upload. Pako uploads them to Gofile (free, no account needed) and copies the link for you. Anyone can download it in their browser.</p>
        {/if}
        <div class="empty-actions">
          <button class="btn" onclick={chooseFolder}><Icon name="folder" size={14} />Choose folder</button>
          <button class="btn primary" onclick={chooseFiles}><Icon name="plus" size={14} stroke={2.2} />Choose files</button>
        </div>
        <p class="hint">Several files or a folder go as they are, into one folder with one link.{#if app.sendHost === "gofile"} {app.sendKeepNote}{:else} They stay in your Drive until you delete them.{/if}</p>
      </div>
    {:else}
      {#if app.sends.length}
        <div class="section">Uploading</div>
        <div class="cards">
          {#each app.sends as j (j.id)}
            {@const p = pct(j)}
            <div class="card" class:failed={j.status === "failed"}>
              <div class="ficon"><FileIcon name={j.name} kind={j.asFiles ? "folder" : undefined} size={22} /></div>
              <div class="main">
                <div class="top">
                  <MiddleText text={j.name} class="nm" />
                  <span class="meta">{what(j.files, j.size)}</span>
                </div>
                {#if j.status === "uploading"}
                  <div class="bar"><div class="fill" style="width: {p}%"></div></div>
                  <div class="sub mono">
                    {#if j.total}
                      <span>{Math.floor(p)}% · {bytes(j.sent)} of {bytes(j.total)}</span>
                    {:else}
                      <span>{bytes(j.sent)} packed and uploaded</span>
                    {/if}
                    <span>{j.speed ? speed(j.speed) : "Starting…"}{j.total && j.speed ? ` · ${timeLeft(j.total - j.sent, j.speed)}` : ""}</span>
                  </div>
                  {#if j.asFiles}
                    <div class="sub mono"><span>{j.filesDone.toLocaleString()} of {j.files.toLocaleString()} files</span></div>
                  {/if}
                {:else}
                  <div class="err"><Icon name="alert" size={13} /><span>{j.message}</span></div>
                {/if}
              </div>
              <div class="actions">
                {#if j.status === "uploading"}
                  <button class="btn ghost small" onclick={() => app.run(() => api.sendCancel(j.id))}>Cancel</button>
                {:else}
                  {#if j.link}
                    <button class="btn ghost small" title="The link already works for the files that are up" onclick={() => app.copyLink(j.link!)}><Icon name="copy" size={12} />Copy link</button>
                  {/if}
                  {#if j.canResume || j.paths.length}
                    <button class="btn small" disabled={retrying.includes(j.id)} onclick={() => tryAgain(j)}><Icon name="retry" size={12} />Try again</button>
                  {/if}
                  <button class="icon-btn" aria-label="Dismiss" title="Dismiss" onclick={() => app.run(() => api.sendDismiss(j.id))}><Icon name="x" size={13} /></button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}

      {#if sentList.length}
        <div class="sent-head">
          <div class="section">Uploaded</div>
          <div class="spacer"></div>
          {#if app.sentChecking}<span class="checking">Checking downloads…</span>{/if}
          <button class="icon-btn refresh" class:spin={app.sentChecking} disabled={app.sentChecking} aria-label="Check downloads" title={checkedTitle()} onclick={refreshCounts}><Icon name="retry" size={13} /></button>
          {#if downloaded.length}
            <button class="btn ghost hbtn" title="Delete from Gofile everything that was downloaded at least once" onclick={() => (bulk = downloaded)}><Icon name="trash" size={12} />Delete downloaded ones</button>
          {/if}
        </div>
        {#if anyGofile}<p class="keep">{app.sendKeepNote}{#if app.sentCountsNote}<span class="note"> {app.sentCountsNote}</span>{/if}</p>{/if}
        <div class="sent" data-tour="send-uploaded">
          <div class="shead" role="row">
            <span></span>
            <span>Name</span>
            <span>Uploaded to</span>
            <span>Downloads</span>
            <span class="h-link">Link</span>
            <span></span>
          </div>
          {#each sentList as s, i (s.id)}
            {@const look = sentLook(s)}
            {@const exp = expiryLook(s, now)}
            <div
              class="srow"
              class:alt={i % 2 === 1}
              class:gone={s.gone}
              class:sel={app.sentPicked === s.id}
              role="row"
              tabindex="-1"
              aria-selected={app.sentPicked === s.id}
              onclick={() => (app.sentPicked = s.id)}
              ondblclick={(e) => !(e.target as HTMLElement).closest("button") && !app.panelOpen && app.togglePanel()}
              onkeydown={(e) => e.key === "Enter" && !app.panelOpen && app.togglePanel()}
              oncontextmenu={(e) => sentMenu(e, s)}
              transition:rowSlide={{ duration: rowMotion }}
              animate:flipIf={{ ...flipOpts, duration: rowMotion ? flipOpts.duration : 0 }}
            >
              <div class="ficon"><FileIcon name={s.name} kind={s.asFiles ? "folder" : undefined} size={20} /></div>
              <div class="sname">
                <MiddleText text={s.name} class="nm" />
                <span class="meta">{#if s.soon && s.deleteAt == null}<span class="soon" title="10 days without a download and Gofile may remove it">May be removed by Gofile soon</span>{" · "}{/if}{#if s.incomplete}<span class="partial" title="Only these files are in {where(s)}. The link works for them.">{busy(s) ? "Uploading" : "Not all uploaded"}</span>{" · "}{/if}{what(s.files, s.size)} · {shortDate(s.date)}</span>
              </div>
              <div class="hostcol">
                <div class="host" class:drive={s.host === "drive"} title={s.host === "drive" ? "In your Google Drive" : "On Gofile"}>
                  <Icon name={s.host === "drive" ? "drive" : "globe"} size={13} />{s.host === "drive" ? "Google Drive" : "Gofile"}
                </div>
                {#if exp}
                  {#key exp.label}<span class="exp {exp.tone}" title={exp.title} in:fade={{ duration: 180 }}>{exp.label}</span>{/key}
                {/if}
              </div>
              <div class="dl" style:color={look?.text} title={s.gone ? "Deleted or expired on Gofile: the link doesn't work any more" : look ? "Gofile counts downloads, not who downloaded" : undefined}>
                {#if look}<span class="dot" style:background={look.dot}></span>{look.label}{/if}
              </div>
              <button class="slink mono" title="Open the download page" onclick={() => app.run(() => api.sentOpen(s.id))}>{s.link.replace(/^https?:\/\//, "")}</button>
              <div class="sact">
                {#if s.gone}
                  <button class="btn small" title="Remove from list" onclick={() => removeFromList(s)}><Icon name="x" size={12} /><span class="lbl">Remove from list</span></button>
                {:else}
                  <button class="btn small" title="Copy link" onclick={() => app.copyLink(s.link)}><Icon name="copy" size={12} /><span class="lbl">Copy link</span></button>
                {/if}
                <button class="icon-btn" aria-label="More for {s.name}" title="More" onclick={(e) => sentMenu(e, s)}><Icon name="chevron" size={13} stroke={2.4} /></button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>

{#if app.sentDeleting}
  {@const s = app.sentDeleting}
  {@const drive = s.host === "drive"}
  <Modal title={`Delete "${s.name}" from ${drive ? "Google Drive" : "Gofile"}?`} width={460} onclose={() => (app.sentDeleting = null)}>
    <p class="msg">The link stops working for everyone, and the file can't be brought back{drive ? " (it doesn't go to your Drive's bin)" : ""}. Your own copy on this PC isn't touched.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (app.sentDeleting = null)}>Cancel</button>
      <button class="btn danger-fill" onclick={deleteOnline}><Icon name="trash" size={13} />Delete from {drive ? "Google Drive" : "Gofile"}</button>
    {/snippet}
  </Modal>
{/if}

{#if noRoom}
  {@const r = noRoom}
  <Modal title="This doesn't fit in your Google Drive" width={460} onclose={() => (noRoom = null)}>
    <p class="msg">This upload needs {bytes(r.need)}, and your Google Drive has {bytes(r.free)} free. Make room by deleting older uploads, or upload it to Gofile.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (noRoom = null)}>Cancel</button>
      <button class="btn" onclick={() => sendWithGofile(r.paths, r.mode, r.name, r.days)}>Upload to Gofile instead</button>
      <button class="btn primary" onclick={() => { noRoom = null; driveBar?.openRoom(); }}><Icon name="trash" size={13} />Make room…</button>
    {/snippet}
  </Modal>
{/if}

{#if bulk}
  {@const list = bulk}
  <Modal title={list.length === 1 ? "Delete 1 downloaded item from Gofile?" : `Delete ${list.length} downloaded items from Gofile?`} width={500} onclose={() => !bulkBusy && (bulk = null)}>
    <p class="msg">These were downloaded at least once. Their links stop working for everyone, and they can't be brought back. Your own copies on this PC aren't touched.</p>
    <ul class="blist">
      {#each list as s (s.id)}
        <li>
          <FileIcon name={s.name} kind={s.asFiles ? "folder" : undefined} size={16} />
          <MiddleText text={s.name} class="nm" />
          <span class="bmeta">{sentLook(s)?.label} · {bytes(s.size)}</span>
        </li>
      {/each}
    </ul>
    {#snippet actions()}
      <button class="btn ghost" disabled={bulkBusy} onclick={() => (bulk = null)}>Cancel</button>
      <button class="btn danger-fill" disabled={bulkBusy} onclick={deleteDownloaded}><Icon name="trash" size={13} />{bulkBusy ? "Deleting…" : "Delete from Gofile"}</button>
    {/snippet}
  </Modal>
{/if}

<style>
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
  .send {
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
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .dim {
    color: var(--text-3);
  }
  .small {
    font-size: 12px;
  }
  .live-text {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    white-space: nowrap;
  }
  .live {
    /* Keeps its size (and its halo) when the text next to it has to give way. */
    flex: none;
    width: 6px;
    height: 6px;
    margin-left: 3px;
    border-radius: 50%;
    background: var(--st-run);
    box-shadow: 0 0 0 3px rgba(77, 163, 255, 0.18);
  }
  .toolbar {
    white-space: nowrap;
  }
  .toolbar .btn {
    flex: none;
  }
  .live-text {
    min-width: 0;
    overflow: hidden;
  }
  .live-text .lt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Narrow window: words give way before any button leaves the window. */
  @container (max-width: 620px) {
    .toolbar .lbl {
      display: none;
    }
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px 16px 20px;
  }
  .section {
    margin: 14px 2px 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  /* ---------- sending ---------- */
  .cards {
    display: grid;
    gap: 8px;
  }
  .card {
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--panel);
  }
  .card.failed {
    background: var(--tint-fail);
    border-color: rgba(242, 75, 79, 0.28);
  }
  .ficon {
    display: grid;
    place-items: center;
  }
  .main {
    min-width: 0;
    display: grid;
    gap: 7px;
  }
  .top {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }
  .top :global(.nm) {
    min-width: 0;
    font-weight: 600;
    color: var(--text);
  }
  .meta {
    flex: none;
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .bar {
    height: 5px;
    border-radius: var(--radius-sm);
    background: var(--track);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: var(--radius-sm);
    background: var(--st-run);
    transition: width var(--mid) var(--ease-out);
  }
  .sub {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 2px 12px;
    font-size: 11px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .err {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: var(--st-fail-text);
  }
  .err :global(svg) {
    flex: none;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  /* A narrow page (a small window with the details panel open): the buttons go under the
     upload, so its name, progress and message keep the width. */
  @container (max-width: 620px) {
    .card {
      grid-template-columns: 28px minmax(0, 1fr);
      row-gap: 8px;
    }
    .card .actions {
      grid-column: 1 / -1;
      justify-content: flex-end;
    }
  }

  /* ---------- sent ---------- */
  .sent {
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  .sent-head {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    margin: 12px 0 2px 2px;
  }
  .sent-head .section {
    margin: 0;
  }
  .checking {
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .refresh:disabled {
    cursor: default;
  }
  .refresh.spin :global(svg) {
    animation: spin 900ms linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .refresh.spin :global(svg) {
      animation: none;
    }
  }
  .hbtn {
    height: 28px;
    padding: 0 10px;
    font-size: 12px;
  }
  .shead,
  .srow {
    display: grid;
    grid-template-columns: 26px minmax(0, 1fr) 136px 136px minmax(120px, 210px) 172px;
    align-items: center;
    gap: 12px;
    height: 52px;
    padding: 0 10px 0 12px;
    border-bottom: 1px solid var(--line-soft);
    transition: background var(--fast) ease;
  }
  .shead {
    align-items: center;
    gap: 12px;
    height: 30px;
    padding: 0 10px 0 12px;
    border-bottom: 1px solid var(--line);
    background: var(--head);
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-3);
  }
  .h-link {
    padding-left: 8px;
  }
  .host {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .host :global(svg) {
    flex: none;
    color: var(--text-3);
  }
  .host.drive :global(svg) {
    color: var(--src-drive);
  }
  .srow:last-child {
    border-bottom: none;
  }
  .srow.alt {
    background-image: linear-gradient(var(--zebra), var(--zebra));
  }
  .srow:hover {
    background-color: var(--hover);
  }
  .srow {
    cursor: default;
  }
  .srow.sel,
  .srow.sel:hover {
    background-color: var(--active);
  }
  .hostcol {
    min-width: 0;
    display: grid;
    gap: 3px;
  }
  .exp {
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .exp.soon {
    color: var(--st-retry-text);
    font-weight: 500;
  }
  .exp.fail {
    color: var(--st-fail-text);
    font-weight: 500;
  }
  .sname {
    min-width: 0;
    display: grid;
    gap: 3px;
  }
  .sname :global(.nm) {
    min-width: 0;
    color: var(--text);
    font-weight: 500;
  }
  .sname .meta {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dl {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    white-space: nowrap;
  }
  .dot {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
  }
  .srow.gone .sname :global(.nm),
  .srow.gone .slink {
    color: var(--text-3);
  }
  .srow.gone .slink {
    text-decoration: line-through;
  }
  .slink {
    min-width: 0;
    height: 26px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--st-run-text);
    font-size: 12px;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
  }
  .slink:hover {
    background: rgba(77, 163, 255, 0.1);
    text-decoration: underline;
  }
  .partial {
    color: var(--st-wait-text);
    font-weight: 500;
  }
  .soon {
    color: var(--st-retry-text);
    font-weight: 500;
  }
  .sact {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }
  /* The link column gives way first (Copy link and the details panel have it too). */
  @container (max-width: 940px) {
    .shead,
    .srow {
      grid-template-columns: 26px minmax(0, 1fr) 136px 136px 172px;
    }
    .slink,
    .h-link {
      display: none;
    }
  }
  /* With the details panel open: the row buttons keep their icons only. */
  @container (max-width: 700px) {
    .shead,
    .srow {
      grid-template-columns: 26px minmax(0, 1fr) 124px 132px 76px;
    }
    .sact .lbl {
      display: none;
    }
  }
  @container (max-width: 540px) {
    .shead,
    .srow {
      grid-template-columns: 26px minmax(0, 1fr) 124px 76px;
    }
    .dl,
    .shead > span:nth-child(4) {
      display: none;
    }
  }
  /* The smallest window with the details panel open: the name keeps the room, where it is and
     when it's deleted are in the panel next to it. */
  @container (max-width: 440px) {
    .shead,
    .srow {
      grid-template-columns: 26px minmax(0, 1fr) 76px;
    }
    .hostcol,
    .shead > span:nth-child(3) {
      display: none;
    }
  }
  .listnote {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 12px;
    padding: 9px 8px 9px 12px;
    border-radius: var(--radius);
    background: var(--tint-fail);
    color: var(--st-fail-text);
    font-size: 12px;
    line-height: 1.45;
  }
  .listnote > :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .listnote span {
    flex: 1;
    min-width: 0;
  }
  .listnote .icon-btn {
    flex: none;
    margin: -3px 0;
  }
  .keep {
    margin: 0 2px 10px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .keep .note {
    color: var(--st-wait-text);
  }
  .blist {
    max-height: 240px;
    overflow-y: auto;
    margin: 14px 0 0;
    padding: 4px 0;
    list-style: none;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
  }
  .blist li {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 12px;
  }
  .blist li :global(.nm) {
    min-width: 0;
    color: var(--text);
  }
  .bmeta {
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
  }

  /* ---------- empty ---------- */
  .empty {
    min-height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 24px 24px 48px;
    color: var(--text-3);
    text-align: center;
  }
  .empty h3 {
    margin: 10px 0 0;
    color: var(--text);
    font-size: 16px;
    font-weight: 600;
  }
  .empty p {
    max-width: 440px;
    margin: 0 0 10px;
    line-height: 1.5;
  }
  .empty .hint {
    margin-top: 14px;
    font-size: 12px;
  }
  .empty-actions {
    display: flex;
    gap: 8px;
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
  .msg {
    margin: 0;
    color: var(--text-2);
    line-height: 1.5;
  }
</style>
