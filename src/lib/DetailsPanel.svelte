<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import MiddleText from "./MiddleText.svelte";
  import { app, hostName, hostSwitch } from "./state.svelte";
  import { api, isActive, type Download, type Part } from "./api";
  import { bytes, bytesOf, downloadLook, sourceOf, speed, statusLook, timeLeft, wordColor } from "./format";
  import Icon from "./Icon.svelte";
  import HelpLink from "./help/HelpLink.svelte";
  import ItemSettings from "./ItemSettings.svelte";
  import { alertOf, doneAlert } from "./alert";
  import VideoPreview from "./VideoPreview.svelte";
  import { isVideo } from "./format";
  import { extracting, isExtracted, needsPassword } from "./extract.svelte";
  import { i18n, num, t } from "./i18n.svelte";

  let { onremove }: { onremove: (ids: string[]) => void } = $props();

  let parts = $state<Part[]>([]);
  /** The download the parts map is for (another one picked: its old map isn't shown meanwhile). */
  let partsFor = "";

  const d = $derived(app.focused);
  const folderFiles = $derived(
    !d && app.focusFolder ? app.rows.find((r) => r.kind === "folder" && r.key === app.focusFolder) : undefined,
  );

  // Refresh the parts map once a second while a download is shown and running.
  $effect(() => {
    const cur = d;
    // Not on screen (panel closed, or Direct / My devices / the Dashboard shown): no asking every second.
    if (!cur || !app.panelOpen || app.view === "direct" || app.view === "dashboard" || app.view === "devices") {
      parts = [];
      partsFor = "";
      return;
    }
    if (cur.id !== partsFor) {
      partsFor = cur.id;
      parts = [];
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
    const tick = setInterval(() => {
      if (isActive(cur.status)) load();
    }, 1000);
    return () => {
      stop = true;
      clearInterval(tick);
    };
  });

  /** The spot check's answer in Rust's note ("H.zip: continuing from…" / "H.zip : reprise…"),
   *  without the name; not while it still waits for the samples. */
  function spotAnswer(x: Download): string {
    const m = x.message ?? "";
    const head = [`${x.name}: `, `${x.name} : `].find((h) => m.startsWith(h));
    if (!head || x.status === "retrying" || m.includes("once a few samples") || m.includes("une fois que quelques échantillons")) return "";
    return m.slice(head.length).replace(/^./, (c) => c.toUpperCase());
  }
  function pct(x: Download) {
    if (x.status === "completed") return 100;
    return x.size ? Math.min(100, (x.downloaded / x.size) * 100) : 0;
  }
  function when(ms: number | null | undefined) {
    if (!ms) return "—";
    const date = new Date(ms > 1e12 ? ms : ms * 1000);
    const today = new Date().toDateString() === date.toDateString();
    const time = date.toLocaleTimeString(i18n.dates, { hour: "2-digit", minute: "2-digit" });
    // Same wording as the list's Date column ("5 Nov"), with the year.
    return today ? t("Today, {time}", { time }) : `${date.toLocaleDateString(i18n.dates, { day: "numeric", month: "short", year: "numeric" })}, ${time}`;
  }
  /** "1.5 MB/s" as its number and its unit. */
  const speedParts = (n: number) => speed(n).split(/[\s  ]/);
  function sourceDetail(x: Download) {
    if (x.source.kind === "gDrive") return t("Google Drive (public link)");
    return sourceOf(x).label;
  }
  /** "Click Add shortcut to Drive, then in your Drive right-click it › Make a copy.", around its two buttons. */
  const copySteps = $derived.by(() => {
    const s = t("Click {shortcut}, then in your Drive right-click it › {copy}.");
    const [a, rest] = [s.slice(0, s.indexOf("{shortcut}")), s.slice(s.indexOf("{shortcut}") + 10)];
    const i = rest.indexOf("{copy}");
    return [a, rest.slice(0, i), rest.slice(i + 6)];
  });
  function primary(x: Download): { label: string; icon: string; run: () => Promise<void>; off?: string } {
    if (x.status === "completed") return { label: t("Open"), icon: "open", run: () => api.openItem(x.id, false) };
    // Done by the browser (through the extension), where the browser can.
    if (x.viaBrowser) {
      if (x.status === "downloading") return { label: t("Pause"), icon: "pause", run: () => api.pause([x.id]) };
      const can = x.viaBrowser.browserId >= 0 && (x.status === "paused" || x.viaBrowser.canResume);
      return {
        label: x.status === "paused" ? t("Resume") : t("Try again"),
        icon: x.status === "paused" ? "play" : "retry",
        run: () => api.resume([x.id]),
        off: can ? undefined : x.viaBrowser.browserId < 0 ? t("Waiting for your browser to report this download") : t("Your browser can't continue this download: open the link again in the browser"),
      };
    }
    if (isActive(x.status) || x.status === "queued") return { label: t("Pause"), icon: "pause", run: () => api.pause([x.id]) };
    if (x.status === "paused") return { label: t("Resume"), icon: "play", run: () => api.resume([x.id]) };
    return { label: t("Try again"), icon: "retry", run: () => api.resume([x.id]) };
  }
</script>

<div class="panel details-panel">
  {#if d}
    {@const look = downloadLook(d)}
    {@const p = app.live[d.id]}
    {@const pr = primary(d)}
    <section class="top">
      <div class="headline">
        <span class="st" style="color: {wordColor(look)}">{look.label}</span>
        <span class="src">{sourceOf(d).label}</span>
        <button class="icon-btn" aria-label={t("Close details panel (Ctrl+I)")} title={t("Close (Ctrl+I)")} onclick={() => app.togglePanel()}>
          <Icon name="chevronsRight" size={14} stroke={2} />
        </button>
      </div>
      <div class="name">{d.name}</div>

      <div class="stats">
        <div><span class="k">{t("Done (share)")}</span><span class="v mono">{num(Math.floor(pct(d) * 10) / 10, 1)}{i18n.lang === "fr" ? " " : ""}<small>%</small></span></div>
        <div>
          <span class="k">{t("Speed")}</span>
          {#if p && p.speed > 0}
            <span class="v mono">{speedParts(p.speed)[0]}<small> {speedParts(p.speed)[1]}</small></span>
          {:else}<span class="v mono dim">—</span>{/if}
        </div>
        <div>
          <span class="k">{t("Left")}</span>
          {#if p && p.speed > 0 && d.size}<span class="v mono sm">{timeLeft(d.size - d.downloaded, p.speed, { bare: true })}</span>
          {:else}<span class="v mono dim">—</span>{/if}
        </div>
      </div>

      <div class="bar"><div class="fill" style="width: {pct(d)}%; background: {look.color}"></div></div>
      <div class="sizes mono">
        <span>{d.size ? bytesOf(d.downloaded, d.size) : bytes(d.downloaded)}</span>
        {#if d.status === "completed"}<span class="ok">{t("Complete")}</span>{/if}
      </div>

      {#if d.message && (d.status === "retrying" || d.status === "failed" || d.status === "quotaExceeded")}
        <div class="callout" class:warn={d.status !== "failed"} class:err={d.status === "failed"}>
          <Icon name="alert" size={14} />
          <span>{d.message}</span>
        </div>
      {/if}
      {#if d.viaBrowser && d.status !== "completed"}
        <div class="callout">
          <Icon name="globe" size={14} />
          <span>{t("This site only lets your browser download this file, so Pako can't use its multi-part speed. The browser downloads it; Pako follows it and moves it to your folder when it's done.")}</span>
        </div>
      {/if}
      {#if d.private && d.status === "completed"}
        <div class="callout">
          <Icon name="check" size={14} />
          <span>{t("From a private window: it leaves this list once you open it or click its ✕, else {when}. The file stays. Pako keeps no other trace of it.", { when: app.settings?.privateKeep === "1h" ? t("an hour after it finished") : app.settings?.privateKeep === "close" ? t("when you quit Pako") : t("10 minutes after it finished") })} <button class="linkish" onclick={() => app.run(async () => { await api.forgetPrivate(d.id); await app.refresh(); })}>{t("Remove it from the list now")}</button></span>
        </div>
      {/if}
      {#if hostSwitch(d)}
        {@const last = d.tried?.[d.tried.length - 1]}
        <div class="callout">
          <Icon name="retry" size={14} />
          <span>
            {t("Continuing from {host}.", { host: hostName(d) })} {last ? (last.why ? t("{host} didn't work: {why}", { host: hostName(last), why: last.why }) : t("{host} didn't work.", { host: hostName(last) })) : ""}
            <!-- The spot check's answer (kept where it stopped, or starting over), set while it runs. -->
            {#if spotAnswer(d)}<br />{spotAnswer(d)}.{/if}
          </span>
        </div>
      {/if}
      {#if d.message && d.status === "completed"}
        <div class="callout" class:ok-note={isExtracted(d.message)}>
          <Icon name={isExtracted(d.message) ? "check" : "alert"} size={14} />
          <span
            >{isExtracted(d.message) ? t("The split archive was extracted into this folder.") : needsPassword(d.message) ? t("It's locked with a password: type it to extract the files.") : d.message}
            {#if needsPassword(d.message)}<button class="linkish" onclick={() => (extracting.asking = d)}>{t("Type the password…")}</button>{/if}</span
          >
        </div>
      {/if}
      {#if d.status === "needsAccount"}
        <div class="quota">
          <p>{t("Google only gives this file to people it's shared with. Open it in your browser (signed in to that account) and download it there: Pako catches the download.")}</p>
          <div class="acts">
            <button class="btn primary small" onclick={() => app.run(() => api.openInDrive(d.id))}><Icon name="globe" size={13} />{t("Open in browser")}</button>
            <span class="grow"></span>
            <HelpLink to="downloading.md#only-in-the-browser-files-shared-with-certain-people" label={t("More about files shared with certain people")} />
          </div>
        </div>
      {/if}
      {#if d.status === "quotaExceeded"}
        <div class="quota">
          <p class="lead">{t("Too many people downloaded this file today. Pako tries again by itself every few hours, or get it now from your own Drive:")}</p>
          <ol>
            <li>{t("Open it in Drive.")}</li>
            <li>{copySteps[0]}<b>{t("Add shortcut to Drive")}</b>{copySteps[1]}<b>{t("Make a copy")}</b>{copySteps[2]}</li>
            <li>{t("Download your copy: Pako catches it. Delete the copy afterwards to free the space.")}</li>
          </ol>
          <div class="acts">
            <button class="btn primary small" onclick={() => app.run(() => api.openInDrive(d.id))}><Icon name="globe" size={13} />{t("Open in Drive")}</button>
            <button class="btn small" onclick={() => app.run(() => api.resume([d.id]))}><Icon name="retry" size={13} />{t("Try again")}</button>
            <span class="grow"></span>
            <HelpLink to="downloading.md#google-limit-too-many-downloads-today" label={t("More about Google's download limit")} />
          </div>
        </div>
      {/if}
    </section>

    {#if d.resumable && parts.length > 1 && d.size}
      <section>
        <div class="label">
          {t("Parts")}
          <span class="muted">{t("{n} connections", { n: p?.connections ?? 0 })}{app.settings?.autoConnections ? ` · ${t("Auto")}` : ""}</span>
        </div>
        <div class="map" aria-label={t("Which parts of the file are downloaded")}>
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

    {#if isVideo(d.name) && d.status !== "completed"}
      <VideoPreview id={d.id} site={d.source.kind === "video"} />
    {/if}

    {#if d.status !== "completed"}
      <ItemSettings
        dest={d.destDir}
        destLocked={!d.viaBrowser && (isActive(d.status) || d.downloaded > 0)}
        priority={d.priority}
        priorityOff={d.viaBrowser ? t("Your browser downloads it: Pako's queue doesn't apply.") : undefined}
        alert={alertOf(d, doneAlert(app.settings))}
        onDest={(dir) => app.run(() => api.setFolder([d.id], dir))}
        onPriority={(p) => app.run(() => api.setPriority([d.id], p))}
        onAlert={(a) => app.run(() => api.setAlert([d.id], a))}
      />
    {/if}

    <section class="details">
      <div class="label">{t("Details")}</div>
      <dl>
        <dt>{t("Source")}</dt><dd><MiddleText text={sourceDetail(d)} /></dd>
        {#if d.mirrors?.length || d.tried?.length}
          {@const left = d.mirrors ?? []}
          <dt>{t("Backups")}</dt>
          <dd title={t("The same file on other hosts: if this host fails for good, Pako goes on from the next one")}>
            {left.length ? t("{n} left: {hosts}", { n: left.length, hosts: left.map(hostName).join(", ") }) : t("None left")}
          </dd>
          {#if d.tried?.length}<dt>{t("Tried")}</dt><dd title={d.tried.map((m) => t("{host}: {why}", { host: hostName(m), why: m.why ?? t("didn't work") })).join("\n")}>{d.tried.map(hostName).join(", ")}</dd>{/if}
        {/if}
        <dt>{t("Saved to")}</dt>
        <dd><button class="linkish" onclick={() => app.run(() => api.openItem(d.id, true))}><MiddleText text={d.destDir} /></button></dd>
        {#if d.md5}<dt>{t("File check")}</dt><dd title={t("Compared with Google's own record of the file (MD5)")}>{d.status === "completed" ? t("Passed: the file is complete") : t("Checked when it finishes")}</dd>{/if}
        {#if d.source.kind === "video"}
          <dt>{t("Quality")}</dt><dd>{d.source.audio ? t("Audio only ({format})", { format: d.source.audio.toUpperCase() }) : d.source.quality ? t("Up to {q}p", { q: d.source.quality }) : t("Up to {q}p (default)", { q: app.settings?.videoQuality || 1080 })}</dd>
          <dt>{t("After a pause")}</dt><dd>{t("Continues where it stopped")}</dd>
        {:else if d.viaBrowser}
          <dt>{t("Downloaded by")}</dt><dd>{t("Your browser")}</dd>
          <dt>{t("After a pause")}</dt><dd>{d.viaBrowser.canResume ? t("Continues where it stopped") : t("As your browser can")}</dd>
        {:else}
          <dt>{t("After a pause")}</dt><dd>{d.resolved ? (d.resumable ? t("Continues where it stopped") : t("Starts again from zero (this server can't continue)")) : "—"}</dd>
        {/if}
        <dt>{t("Added")}</dt><dd>{when(d.date || d.addedAt)}</dd>
        {#if d.finishedAt}<dt>{t("Finished (when)")}</dt><dd>{when(d.finishedAt)}</dd>{/if}
      </dl>
    </section>

    <div class="panel-actions">
      <button class="btn" disabled={!!pr.off} title={pr.off} onclick={() => app.run(pr.run)}><Icon name={pr.icon} size={13} /><span class="l">{pr.label}</span></button>
      <button class="btn" onclick={() => app.run(() => api.openItem(d.id, true))}><Icon name="folder" size={13} /><span class="l">{t("Folder")}</span></button>
      <button class="btn danger" onclick={() => onremove([d.id])}><Icon name="trash" size={13} /><span class="l">{t("Remove")}</span></button>
    </div>
  {:else if folderFiles && folderFiles.kind === "folder"}
    {@const files = folderFiles.files}
    {@const size = files.reduce((s, f) => s + (f.size ?? 0), 0)}
    {@const done = files.reduce((s, f) => s + (f.status === "completed" ? (f.size ?? f.downloaded) : f.downloaded), 0)}
    {@const look = statusLook(app.folderStatus(files))}
    <section class="top">
      <div class="headline">
        <span class="st" style="color: {wordColor(look)}">{t("Folder")}</span>
        <span class="src">{t("{n} files", { n: files.length })}</span>
        <button class="icon-btn" aria-label={t("Close details panel (Ctrl+I)")} title={t("Close (Ctrl+I)")} onclick={() => app.togglePanel()}><Icon name="chevronsRight" size={14} stroke={2} /></button>
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
            <div class="xt">{t("Extract when all parts are done")}</div>
            <div class="xn">{app.sevenZip ? t("Into this folder, next to the parts.") : t("Needs 7-Zip (Settings › Features & updates).")}</div>
          </div>
          <button class="switch" class:on role="switch" aria-checked={on} aria-label={t("Extract when all parts are done")} onclick={() => app.run(() => api.setExtract(setIds, !on))}></button>
        </div>
        {#if split.missing.length}
          <div class="callout err"><Icon name="alert" size={14} /><span>{t("Incomplete: {parts}.", { parts: split.missing.join(", ") })}</span></div>
        {/if}
      </section>
    {/if}
    <section class="details">
      <div class="label">{t("Files")}</div>
      <dl>
        {#each ["downloading", "retrying", "queued", "paused", "failed", "quotaExceeded", "needsAccount", "completed"] as s}
          {@const n = files.filter((f) => f.status === s).length}
          {#if n}<dt style="color: {statusLook(s as Download['status']).text}">{statusLook(s as Download["status"]).label}</dt><dd>{n}</dd>{/if}
        {/each}
      </dl>
    </section>
    <div class="panel-actions">
      <button class="btn" onclick={() => app.run(() => api.resume(files.map((f) => f.id)))}><Icon name="play" size={13} /><span class="l">{t("Resume")}</span></button>
      <button class="btn" onclick={() => app.run(() => api.pause(files.map((f) => f.id)))}><Icon name="pause" size={13} /><span class="l">{t("Pause")}</span></button>
      <button class="btn danger" onclick={() => onremove(files.map((f) => f.id))}><Icon name="trash" size={13} /><span class="l">{t("Remove")}</span></button>
    </div>
  {:else}
    <div class="none">
      <div class="headline end">
        <button class="icon-btn" aria-label={t("Close details panel")} onclick={() => app.togglePanel()}><Icon name="chevronsRight" size={14} stroke={2} /></button>
      </div>
      <div class="none-body">
        <Icon name="list" size={22} />
        <p>{t("Select a download to see its details.")}</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .panel {
    width: var(--pw, 320px);
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
    grid-template-columns: auto minmax(0, 1fr);
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
    max-width: 100%;
    border: none;
    background: none;
    padding: 0;
    color: var(--text-2);
    text-align: right;
    text-decoration: underline;
    text-decoration-color: var(--underline);
    text-underline-offset: 3px;
  }
  .linkish:hover {
    color: var(--text);
    text-decoration-color: var(--text-3);
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
