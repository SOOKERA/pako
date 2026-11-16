<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { app } from "./state.svelte";
  import { api, isActive, type Download, type Part } from "./api";
  import { bytes, bytesOf, sourceOf, speed, statusLook, timeLeft } from "./format";
  import Icon from "./Icon.svelte";
  import HelpLink from "./help/HelpLink.svelte";
  import ItemSettings from "./ItemSettings.svelte";
  import VideoPreview from "./VideoPreview.svelte";
  import { isVideo } from "./format";

  let { onremove }: { onremove: (ids: string[]) => void } = $props();

  let parts = $state<Part[]>([]);

  const d = $derived(app.focused);
  const folderFiles = $derived(
    !d && app.focusFolder ? app.rows.find((r) => r.kind === "folder" && r.key === app.focusFolder) : undefined,
  );

  // Refresh the parts map once a second while a download is shown and running.
  $effect(() => {
    const cur = d;
    if (!cur || !app.panelOpen) {
      parts = [];
      return;
    }
    let stop = false;
    const load = async () => {
      try {
        const p = await api.parts(cur.id);
        if (!stop) parts = p;
      } catch {
        /* ignore */
      }
    };
    load();
    const t = setInterval(() => {
      if (isActive(cur.status)) load();
    }, 1000);
    return () => {
      stop = true;
      clearInterval(t);
    };
  });

  function pct(x: Download) {
    if (x.status === "completed") return 100;
    return x.size ? Math.min(100, (x.downloaded / x.size) * 100) : 0;
  }
  function when(ms: number | null | undefined) {
    if (!ms) return "—";
    const date = new Date(ms > 1e12 ? ms : ms * 1000);
    const today = new Date().toDateString() === date.toDateString();
    const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    // Same wording as the list's Date column ("5 Nov"), with the year.
    return today ? `Today, ${time}` : `${date.toLocaleDateString([], { day: "numeric", month: "short", year: "numeric" })}, ${time}`;
  }
  function sourceDetail(x: Download) {
    if (x.source.kind === "gDrive") return "Google Drive (public link)";
    return sourceOf(x).label;
  }
  function primary(x: Download): { label: string; icon: string; run: () => Promise<void> } {
    if (x.status === "completed") return { label: "Open", icon: "open", run: () => api.openItem(x.id, false) };
    if (isActive(x.status) || x.status === "queued") return { label: "Pause", icon: "pause", run: () => api.pause([x.id]) };
    if (x.status === "paused") return { label: "Resume", icon: "play", run: () => api.resume([x.id]) };
    return { label: "Try again", icon: "retry", run: () => api.resume([x.id]) };
  }
</script>

<div class="panel">
  {#if d}
    {@const look = statusLook(d.status, d.message)}
    {@const p = app.live[d.id]}
    {@const pr = primary(d)}
    <section class="top">
      <div class="headline">
        <span class="st" style="color: {look.color}">{look.label}</span>
        <span class="src">{sourceOf(d).label}</span>
        <button class="icon-btn" aria-label="Close details panel (Ctrl+I)" title="Close (Ctrl+I)" onclick={() => app.togglePanel()}>
          <Icon name="chevronsRight" size={14} stroke={2} />
        </button>
      </div>
      <div class="name">{d.name}</div>

      <div class="stats">
        <div><span class="k">Done</span><span class="v mono">{pct(d).toFixed(1)}<small>%</small></span></div>
        <div>
          <span class="k">Speed</span>
          {#if p && p.speed > 0}
            <span class="v mono">{speed(p.speed).split(" ")[0]}<small> {speed(p.speed).split(" ")[1]}</small></span>
          {:else}<span class="v mono dim">—</span>{/if}
        </div>
        <div>
          <span class="k">Left</span>
          {#if p && p.speed > 0 && d.size}<span class="v mono sm">{timeLeft(d.size - d.downloaded, p.speed).replace(" left", "")}</span>
          {:else}<span class="v mono dim">—</span>{/if}
        </div>
      </div>

      <div class="bar"><div class="fill" style="width: {pct(d)}%; background: {look.color}"></div></div>
      <div class="sizes mono">
        <span>{d.size ? bytesOf(d.downloaded, d.size) : bytes(d.downloaded)}</span>
        {#if d.status === "completed"}<span class="ok">Complete</span>{/if}
      </div>

      {#if d.message && (d.status === "retrying" || d.status === "failed" || d.status === "quotaExceeded")}
        <div class="callout" class:warn={d.status !== "failed"} class:err={d.status === "failed"}>
          <Icon name="alert" size={14} />
          <span>{d.message}</span>
        </div>
      {/if}
      {#if d.message && d.status === "completed"}
        <div class="callout" class:ok-note={d.message === "Extracted"}>
          <Icon name={d.message === "Extracted" ? "check" : "alert"} size={14} />
          <span>{d.message === "Extracted" ? "The split archive was extracted into this folder." : d.message}</span>
        </div>
      {/if}
      {#if d.status === "needsAccount"}
        <div class="quota">
          <p>Google only gives this file to people it's shared with. Open it in your browser (signed in to that account) and download it there: Pako catches the download.</p>
          <div class="acts">
            <button class="btn primary small" onclick={() => app.run(() => api.openInDrive(d.id))}><Icon name="globe" size={13} />Open in browser</button>
            <span class="grow"></span>
            <HelpLink to="downloading.md#only-in-the-browser-files-shared-with-certain-people" label="More about files shared with certain people" />
          </div>
        </div>
      {/if}
      {#if d.status === "quotaExceeded"}
        <div class="quota">
          <p class="lead">Too many people downloaded this file today. Pako tries again by itself every few hours, or get it now from your own Drive:</p>
          <ol>
            <li>Open it in Drive.</li>
            <li>Click <b>Add shortcut to Drive</b>, then in your Drive right-click it › <b>Make a copy</b>.</li>
            <li>Download your copy: Pako catches it. Delete the copy afterwards to free the space.</li>
          </ol>
          <div class="acts">
            <button class="btn primary small" onclick={() => app.run(() => api.openInDrive(d.id))}><Icon name="globe" size={13} />Open in Drive</button>
            <button class="btn small" onclick={() => app.run(() => api.resume([d.id]))}><Icon name="retry" size={13} />Try again</button>
            <span class="grow"></span>
            <HelpLink to="downloading.md#google-limit-too-many-downloads-today" label="More about Google's download limit" />
          </div>
        </div>
      {/if}
    </section>

    {#if d.resumable && parts.length > 1 && d.size}
      <section>
        <div class="label">
          Parts
          <span class="muted">{p?.connections ?? 0} {p?.connections === 1 ? "connection" : "connections"}{app.settings?.autoConnections ? " · Auto" : ""}</span>
        </div>
        <div class="map" aria-label="Which parts of the file are downloaded">
          {#each parts as part}
            {@const total = d.size}
            {#if part.done > 0}
              <span class="seg done" style="left: {(part.start / total) * 100}%; width: {(part.done / total) * 100}%"></span>
            {/if}
            {#if part.active}
              <span class="seg head" style="left: {((part.start + part.done) / total) * 100}%"></span>
            {/if}
          {/each}
        </div>
      </section>
    {/if}

    {#if isVideo(d.name) && d.status !== "completed" && d.source.kind !== "video"}
      <VideoPreview id={d.id} />
    {/if}

    {#if d.status !== "completed"}
      <ItemSettings
        dest={d.destDir}
        destLocked={isActive(d.status) || d.downloaded > 0}
        priority={d.priority}
        notify={d.notify}
        onDest={(dir) => app.run(() => api.setFolder([d.id], dir))}
        onPriority={(p) => app.run(() => api.setPriority([d.id], p))}
        onNotify={(on) => app.run(() => api.setNotify([d.id], on))}
      />
    {/if}

    <section class="details">
      <div class="label">Details</div>
      <dl>
        <dt>Source</dt><dd>{sourceDetail(d)}</dd>
        <dt>Saved to</dt>
        <dd><button class="linkish" onclick={() => app.run(() => api.openItem(d.id, true))} title={d.destDir}>{d.destDir}</button></dd>
        {#if d.md5}<dt>File check</dt><dd title="Compared with Google's own record of the file (MD5)">{d.status === "completed" ? "Passed: the file is complete" : "Checked when it finishes"}</dd>{/if}
        {#if d.source.kind === "video"}
          <dt>Quality</dt><dd>{d.source.audio ? `Audio only (${d.source.audio.toUpperCase()})` : d.source.quality ? `Up to ${d.source.quality}p` : `Up to ${app.settings?.videoQuality || 1080}p (default)`}</dd>
          <dt>After a pause</dt><dd>Continues where it stopped</dd>
        {:else}
          <dt>After a pause</dt><dd>{d.resolved ? (d.resumable ? "Continues where it stopped" : "Starts again from zero (this server can't continue)") : "—"}</dd>
        {/if}
        <dt>Added</dt><dd>{when(d.date || d.addedAt)}</dd>
        {#if d.finishedAt}<dt>Finished</dt><dd>{when(d.finishedAt)}</dd>{/if}
      </dl>
    </section>

    <div class="actions">
      <button class="btn" onclick={() => app.run(pr.run)}><Icon name={pr.icon} size={13} />{pr.label}</button>
      <button class="btn" onclick={() => app.run(() => api.openItem(d.id, true))}><Icon name="folder" size={13} />Folder</button>
      <button class="btn danger" onclick={() => onremove([d.id])}><Icon name="trash" size={13} />Remove</button>
    </div>
  {:else if folderFiles && folderFiles.kind === "folder"}
    {@const files = folderFiles.files}
    {@const size = files.reduce((s, f) => s + (f.size ?? 0), 0)}
    {@const done = files.reduce((s, f) => s + (f.status === "completed" ? (f.size ?? f.downloaded) : f.downloaded), 0)}
    {@const look = statusLook(app.folderStatus(files))}
    <section class="top">
      <div class="headline">
        <span class="st" style="color: {look.color}">Folder</span>
        <span class="src">{files.length} files</span>
        <button class="icon-btn" aria-label="Close details panel" onclick={() => app.togglePanel()}><Icon name="chevronsRight" size={14} stroke={2} /></button>
      </div>
      <div class="name">{folderFiles.name}</div>
      <div class="bar"><div class="fill" style="width: {size ? (done / size) * 100 : 0}%; background: {look.color}"></div></div>
      <div class="sizes mono"><span>{size ? bytesOf(done, size) : bytes(done)}</span></div>
    </section>
    {@const split = app.downloadSets.find((s) => s.ids.some((id) => files.some((f) => f.id === id)))}
    {#if split}
      {@const setIds = split.ids}
      {@const on = app.downloads.filter((x) => setIds.includes(x.id)).every((x) => x.extract)}
      <!-- The switch acts on the whole set, also parts the filter or search hides (like Links). -->
      <section class="xsec">
        <div class="xrow">
          <div>
            <div class="xt">Extract when all parts are done</div>
            <div class="xn">{app.sevenZip ? "Into this folder, next to the parts." : "Needs 7-Zip (Settings › Features & updates)."}</div>
          </div>
          <button class="switch" class:on role="switch" aria-checked={on} aria-label="Extract when all parts are done" onclick={() => app.run(() => api.setExtract(setIds, !on))}></button>
        </div>
        {#if split.missing.length}
          <div class="callout err"><Icon name="alert" size={14} /><span>Incomplete: {split.missing.join(", ")}.</span></div>
        {/if}
      </section>
    {/if}
    <section class="details">
      <div class="label">Files</div>
      <dl>
        {#each ["downloading", "retrying", "queued", "paused", "failed", "quotaExceeded", "needsAccount", "completed"] as s}
          {@const n = files.filter((f) => f.status === s).length}
          {#if n}<dt style="color: {statusLook(s as Download['status']).text}">{statusLook(s as Download["status"]).label}</dt><dd>{n}</dd>{/if}
        {/each}
      </dl>
    </section>
    <div class="actions">
      <button class="btn" onclick={() => app.run(() => api.resume(files.map((f) => f.id)))}><Icon name="play" size={13} />Resume</button>
      <button class="btn" onclick={() => app.run(() => api.pause(files.map((f) => f.id)))}><Icon name="pause" size={13} />Pause</button>
      <button class="btn danger" onclick={() => onremove(files.map((f) => f.id))}><Icon name="trash" size={13} />Remove</button>
    </div>
  {:else}
    <div class="none">
      <div class="headline end">
        <button class="icon-btn" aria-label="Close details panel" onclick={() => app.togglePanel()}><Icon name="chevronsRight" size={14} stroke={2} /></button>
      </div>
      <div class="none-body">
        <Icon name="list" size={22} />
        <p>Select a download to see its details.</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .panel {
    width: 320px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    overflow-y: auto;
    overflow-x: hidden;
  }
  section {
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }
  .xsec {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .xrow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .xt {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .xn {
    margin-top: 2px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .top {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .headline {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-right: -8px;
  }
  .headline.end {
    justify-content: flex-end;
    padding: 12px 18px 0;
  }
  .st {
    font-size: 12px;
    font-weight: 600;
    transition: color var(--mid) ease;
  }
  .src {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-3);
  }
  .name {
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    margin-top: 2px;
  }
  .stats div {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .k {
    font-size: 11px;
    color: var(--text-3);
  }
  .v {
    font-size: 18px;
    white-space: nowrap;
  }
  .v.sm {
    font-size: 14px;
    padding-top: 3px;
  }
  .v small {
    font-size: 11px;
    color: var(--text-3);
  }
  .v.dim {
    color: var(--text-4);
  }
  .bar {
    height: 6px;
    border-radius: var(--radius-sm);
    background: var(--track);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: var(--radius-sm);
    transition:
      width 450ms var(--ease-out),
      background var(--mid) ease;
  }
  .sizes {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-3);
    margin-top: -4px;
  }
  .ok {
    color: var(--st-done-text);
  }
  .callout {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 9px 10px;
    border-radius: var(--radius);
    font-size: 12px;
    line-height: 1.4;
  }
  .callout.warn {
    background: rgba(242, 182, 72, 0.09);
    color: var(--st-retry-text);
  }
  .callout.err {
    background: var(--tint-fail);
    color: var(--st-fail-text);
  }
  .callout:not(.warn):not(.err) {
    background: var(--raise);
    color: var(--text-2);
  }
  .callout.ok-note {
    background: rgba(70, 201, 126, 0.1);
    color: var(--st-done-text);
  }
  .callout :global(svg) {
    flex: none;
    margin-top: 1px;
  }
  .quota {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
    color: var(--text-2);
  }
  .quota p {
    margin: 0;
    line-height: 1.45;
  }
  .quota .lead {
    color: var(--text);
  }
  .quota ol {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    line-height: 1.45;
  }
  .quota b {
    font-weight: 600;
    color: var(--text);
  }
  .quota .acts {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
  }
  .quota .grow {
    flex: 1;
  }
  .btn.small {
    height: 30px;
    font-size: 12px;
  }
  .label {
    display: flex;
    justify-content: space-between;
    margin-bottom: 10px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .muted {
    font-size: 11px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-3);
  }
  .map {
    position: relative;
    height: 14px;
    border-radius: var(--radius-sm);
    background: var(--parts-map);
    overflow: hidden;
  }
  .seg {
    position: absolute;
    top: 0;
    bottom: 0;
  }
  .seg.done {
    background: var(--st-run);
    opacity: 0.85;
    transition:
      width 900ms linear,
      left 300ms ease;
  }
  .seg.head {
    width: 2px;
    margin-left: -1px;
    background: #fff;
    box-shadow: 0 0 6px var(--st-run);
    transition: left 900ms linear;
  }
  /* On the light map a white edge would vanish: the darker azure instead. */
  :global(:root[data-theme="light"]) .seg.head {
    background: var(--st-run-text);
    box-shadow: none;
  }
  .details dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 8px 14px;
    margin: 0;
    font-size: 12px;
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
    text-align: right;
    overflow-wrap: anywhere;
    color: var(--text-2);
  }
  .linkish {
    border: none;
    background: none;
    padding: 0;
    color: var(--text-2);
    text-align: right;
    overflow-wrap: anywhere;
    text-decoration: underline;
    text-decoration-color: var(--underline);
    text-underline-offset: 3px;
  }
  .linkish:hover {
    color: var(--text);
    text-decoration-color: var(--text-3);
  }
  .actions {
    margin-top: auto;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
    padding: 14px 18px;
    border-top: 1px solid var(--line);
  }
  .none {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .none-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--text-4);
    padding-bottom: 60px;
  }
  .none-body p {
    margin: 0;
    font-size: 12px;
    color: var(--text-3);
  }
</style>
