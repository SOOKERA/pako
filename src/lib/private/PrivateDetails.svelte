<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The private list's details panel: the picked file (or folder) with its progress, where it
  // came from, how it tells you it's done, its priority and its buttons. Only shown while
  // Private mode is open; it empties the moment it locks (the list it reads is cleared).
  import { api, isActive, type Download, type Priority } from "../api";
  import { app } from "../state.svelte";
  import { priv } from "../private.svelte";
  import { bytes, bytesOf, downloadLook, isVideo, sourceOf, speed, timeLeft, wordColor } from "../format";
  import { ALERTS, alertOf, doneAlert, type Alert } from "../alert";
  import { fadeIn, rowSlide } from "../motion";
  import Icon from "../Icon.svelte";
  import SegPick from "../SegPick.svelte";
  import MiddleText from "../MiddleText.svelte";
  import { i18n, num, said, t, tMaybe } from "../i18n.svelte";

  let {
    d,
    folder,
    onuse,
    ondecrypt,
    onsend,
    onupload,
    ondelete,
    onclose,
  }: {
    d: Download | null;
    folder: { name: string; files: Download[]; several?: boolean } | null;
    onuse: (d: Download) => void;
    ondecrypt: (items: Download[], folder: string | null) => void;
    /** "Send with Direct…" (a folder's name as the send's title) and "Upload…". */
    onsend: (items: Download[], folder: string | null) => void;
    onupload: (items: Download[]) => void;
    ondelete: (items: Download[]) => void;
    onclose: () => void;
  } = $props();

  const done = (x: Download) => x.status === "completed";
  const canPause = (x: Download) => isActive(x.status) || x.status === "queued";
  const levels: { value: Priority; label: string; icon?: string }[] = $derived([
    { value: "first", label: t("First"), icon: "up" },
    { value: "normal", label: t("Normal") },
    { value: "later", label: t("Later"), icon: "down" },
  ]);
  const alerts = $derived(ALERTS.map((a) => ({ value: a.id, label: tMaybe(a.label), hint: tMaybe(a.hint) })));

  function pct(x: Download) {
    return done(x) ? 100 : x.size ? Math.min(100, (x.downloaded / x.size) * 100) : 0;
  }
  /** "Today, 14:32" / "18 Nov 2026, 14:32". */
  function when(ms: number | null | undefined) {
    if (!ms) return "—";
    const date = new Date(ms > 1e12 ? ms : ms * 1000);
    const time = date.toLocaleTimeString(i18n.dates, { hour: "2-digit", minute: "2-digit", hourCycle: "h23" });
    return new Date().toDateString() === date.toDateString()
      ? t("Today, {time}", { time })
      : `${date.toLocaleDateString(i18n.dates, { day: "numeric", month: "short", year: "numeric" })}, ${time}`;
  }
  function host(x: Download) {
    try {
      return new URL(x.link).host;
    } catch {
      return "";
    }
  }
  /** One level for several files, or null when they differ. */
  function shared<T>(list: T[]): T | null {
    return list.length && list.every((v) => v === list[0]) ? list[0] : null;
  }

  const files = $derived(d ? [d] : (folder?.files ?? []));
  const unfinished = $derived(files.filter((x) => !done(x)));
  /** Sends, uploads and Decrypt… running on these files (each once). */
  const busy = $derived([...new Map(files.flatMap((x) => priv.workOn(x.id)).map((w) => [w.key, w])).values()]);
  const level = $derived(shared(unfinished.map((x) => alertOf(x, doneAlert(app.settings)))));
  const prio = $derived(shared(unfinished.map((x) => x.priority)));
  const folderSize = $derived(folder ? folder.files.reduce((n, x) => n + (x.size ?? x.downloaded), 0) : 0);
  const folderDone = $derived(folder ? folder.files.filter(done).length : 0);

  async function setAlert(a: Alert) {
    const ids = unfinished.map((x) => x.id);
    await app.run(() => api.setAlert(ids, a));
    await priv.refreshList();
  }
  async function setPriority(p: Priority) {
    const ids = unfinished.map((x) => x.id);
    await app.run(() => api.setPriority(ids, p));
    await priv.refreshList();
  }
  async function pauseOrResume(list: Download[]) {
    const pausing = list.filter(canPause).map((x) => x.id);
    if (pausing.length) await app.run(() => api.pause(pausing));
    else await app.run(() => api.resume(list.filter((x) => !done(x)).map((x) => x.id)));
    await priv.refreshList();
  }
</script>

<aside class="panel details-panel" aria-label={t("Details")} data-tour="private-details">
  {#if d}
    {@const look = downloadLook({ ...d, private: false })}
    {@const sp = priv.speeds[d.id] ?? 0}
    {#key d.id}
      <section class="top" in:fadeIn={{ duration: 160 }}>
        <div class="headline">
          <span class="st" style="color: {wordColor(look)}">{look.label}</span>
          <span class="src">{sourceOf(d).label}</span>
          <button class="icon-btn" aria-label={t("Close details panel (Ctrl+I)")} title={t("Close (Ctrl+I)")} onclick={onclose}><Icon name="chevronsRight" size={14} stroke={2} /></button>
        </div>
        <div class="name">{d.name}</div>
        <div class="stats">
          <div><span class="k">{t("Done (how far)")}</span><span class="v mono">{num(Math.floor(pct(d) * 10) / 10, 1)}{i18n.lang === "fr" ? " " : ""}<small>%</small></span></div>
          <div>
            <span class="k">{t("Speed")}</span>
            {#if sp > 0}<span class="v mono">{speed(sp).split(" ")[0]}<small> {speed(sp).split(" ")[1]}</small></span>{:else}<span class="v mono dim">—</span>{/if}
          </div>
          <div>
            <span class="k">{t("Left (time)")}</span>
            {#if sp > 0 && d.size}<span class="v mono sm">{timeLeft(d.size - d.downloaded, sp, { bare: true })}</span>{:else}<span class="v mono dim">—</span>{/if}
          </div>
        </div>
        <div class="bar"><div class="fill" style="width: {pct(d)}%; background: {done(d) ? 'var(--priv)' : look.color}"></div></div>
        <div class="sizes mono">
          <span>{d.size ? bytesOf(d.downloaded, d.size) : bytes(d.downloaded)}</span>
          {#if done(d)}<span class="ok">{t("Complete")}</span>{/if}
        </div>
        {#if d.message && !done(d)}
          <div class="callout" class:err={d.status === "failed"}><Icon name="alert" size={14} /><span>{d.message}</span></div>
        {/if}
      </section>
    {/key}
  {:else if folder}
    {#key folder.name}
      <section class="top" in:fadeIn={{ duration: 160 }}>
        <div class="headline">
          <span class="st">{folder.several ? t("Selected") : t("Folder")}</span>
          <span class="src">{t("{n} files", { n: folder.files.length })}</span>
          <button class="icon-btn" aria-label={t("Close details panel (Ctrl+I)")} title={t("Close (Ctrl+I)")} onclick={onclose}><Icon name="chevronsRight" size={14} stroke={2} /></button>
        </div>
        <div class="name">{folder.name}</div>
        <div class="stats two">
          <div><span class="k">{t("Finished")}</span><span class="v mono">{folderDone}<small> {t("of {total}", { total: folder.files.length })}</small></span></div>
          <div><span class="k">{t("Size")}</span><span class="v mono sm">{bytes(folderSize)}</span></div>
        </div>
      </section>
    {/key}
  {:else}
    <div class="none">
      <Icon name="list" size={18} />
      <span>{t("Select a private download to see its details.")}</span>
    </div>
  {/if}

  {#if busy.length}
    <section class="work" transition:rowSlide={{ duration: 200 }}>
      <div class="label">{t("In progress")}</div>
      {#each busy as w (w.key)}
        {@const p = priv.progress(w)}
        <div class="wk">
          <div class="wl"><span class="wt" class:bad={p.failed}>{p.label}</span>{#if p.frac !== null}<span class="mono wp">{t("{pct}%", { pct: Math.floor(p.frac * 100) })}</span>{/if}</div>
          {#if !p.failed}<div class="bar"><div class="fill" class:wait={p.frac === null} style="width: {p.frac === null ? 100 : p.frac * 100}%; background: var(--priv)"></div></div>{/if}
          <div class="wn">{w.kind !== "decrypt" ? t("{what}: leaving Private mode stops it", { what: said(w.what) }) : said(w.what)}</div>
        </div>
      {/each}
    </section>
  {/if}

  {#if files.length && unfinished.length}
    <section class="settings">
      <div class="label">{t("Settings")}{#if unfinished.length > 1}<span class="muted">{` · ${t("for {n} files", { n: unfinished.length })}`}</span>{/if}</div>
      <div class="row-l">{t("Priority")}</div>
      <SegPick options={levels} value={prio} label={t("Priority")} onpick={setPriority} />
      <div class="row-l">{t("Notify when done")}</div>
      <SegPick options={alerts} value={level} label={t("Notify when done")} onpick={setAlert} />
    </section>
  {/if}

  {#if d}
    <section class="facts">
      <div class="label">{t("Details")}</div>
      <dl>
        <dt>{t("Source")}</dt><dd><MiddleText text={`${sourceOf(d).label}${host(d) && host(d) !== sourceOf(d).label ? ` · ${host(d)}` : ""}`} /></dd>
        <dt>{t("Saved as")}</dt><dd><MiddleText text={d.name} /></dd>
        {#if d.vaultFolder}<dt>{t("In folder")}</dt><dd><MiddleText text={d.vaultFolder} /></dd>{/if}
        <dt>{t("Kept")}</dt><dd>{t("Encrypted in Private mode")}</dd>
        <dt>{t("Added")}</dt><dd>{when(d.addedAt)}</dd>
        {#if done(d)}<dt>{t("Downloaded")}</dt><dd>{when(d.finishedAt)}</dd>{/if}
      </dl>
    </section>
  {/if}

  <!-- The buttons, at the bottom as in every details panel: the main one first. -->
  {#if d}
    <div class="panel-actions">
      {#if done(d)}
        <button class="btn primary" onclick={() => onuse(d)}><Icon name={isVideo(d.name) ? "play" : "open"} size={13} /><span class="l">{isVideo(d.name) ? t("Play") : t("Open")}</span></button>
        <button class="btn" title={t("Decrypt…: a readable copy where you pick")} onclick={() => ondecrypt([d], null)}><Icon name="unlock" size={13} /><span class="l">{t("Decrypt…")}</span></button>
        <button class="btn" title={t("Send with Direct…")} onclick={() => onsend([d], null)}><Icon name="send" size={13} /><span class="l">{t("Send…")}</span></button>
        <button class="btn" title={t("Upload…: share it with a link")} onclick={() => onupload([d])}><Icon name="upload" size={13} /><span class="l">{t("Upload…")}</span></button>
      {:else}
        <button class="btn primary" onclick={() => pauseOrResume([d])}>
          <Icon name={canPause(d) ? "pause" : d.status === "paused" ? "play" : "retry"} size={13} /><span class="l">{canPause(d) ? t("Pause") : d.status === "paused" ? t("Resume") : t("Try again")}</span>
        </button>
      {/if}
      <button class="btn danger" title={t("Delete…: the file and its place in Private mode")} onclick={() => ondelete([d])}><Icon name="trash" size={13} /><span class="l">{t("Delete…")}</span></button>
    </div>
  {:else if folder}
    <div class="panel-actions">
      {#if folderDone === folder.files.length}
        <button class="btn primary" title={folder.several ? t("Decrypt…: readable copies where you pick") : t("Decrypt the whole folder…: a readable copy where you pick")} onclick={() => ondecrypt(folder.files, folder.several ? null : folder.name)}><Icon name="unlock" size={13} /><span class="l">{t("Decrypt…")}</span></button>
        <button class="btn" title={t("Send with Direct…")} onclick={() => onsend(folder.files, folder.several ? null : folder.name)}><Icon name="send" size={13} /><span class="l">{t("Send…")}</span></button>
        <button class="btn" title={t("Upload…: share them with a link")} onclick={() => onupload(folder.files)}><Icon name="upload" size={13} /><span class="l">{t("Upload…")}</span></button>
      {:else}
        <button class="btn primary" onclick={() => pauseOrResume(folder.files)}>
          <Icon name={folder.files.some(canPause) ? "pause" : "play"} size={13} /><span class="l">{folder.files.some(canPause) ? t("Pause") : t("Resume")}</span>
        </button>
      {/if}
      <button class="btn danger" title={t("Delete…: the files and their place in Private mode")} onclick={() => ondelete(folder.files)}><Icon name="trash" size={13} /><span class="l">{t("Delete…")}</span></button>
    </div>
  {/if}
</aside>

<style>
  .panel {
    width: var(--pw, 320px);
    flex: none;
    min-height: 0;
    overflow-y: auto;
    border-left: 1px solid var(--priv-line);
    background: var(--panel);
    display: flex;
    flex-direction: column;
  }
  section {
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }
  .headline {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }
  .st {
    font-weight: 600;
  }
  .src {
    color: var(--text-3);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name {
    margin-top: 8px;
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
    word-break: break-word;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin-top: 14px;
  }
  .stats.two {
    grid-template-columns: repeat(2, 1fr);
  }
  .stats div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .k {
    font-size: 11px;
    color: var(--text-3);
  }
  .v {
    font-size: 18px;
    font-weight: 600;
  }
  .v.sm {
    font-size: 14px;
    padding-top: 3px;
  }
  .v small {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-3);
  }
  .dim {
    color: var(--text-3);
  }
  .bar {
    height: 4px;
    margin-top: 12px;
    border-radius: 2px;
    background: var(--line);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 2px;
    transition: width var(--mid) var(--ease);
  }
  .sizes {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 12px;
    color: var(--text-2);
  }
  .ok {
    color: var(--priv-text);
  }
  .callout {
    display: flex;
    gap: 8px;
    margin-top: 12px;
    padding: 9px 10px;
    border-radius: var(--radius);
    background: var(--raise);
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-2);
  }
  .callout.err {
    color: var(--st-fail-text);
  }
  .btn.danger:hover {
    color: var(--st-fail-text);
  }
  .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    margin-bottom: 10px;
  }
  .muted {
    font-weight: 400;
    color: var(--text-3);
  }
  .row-l {
    font-size: 12px;
    color: var(--text-3);
    margin: 10px 0 6px;
  }
  .row-l:first-of-type {
    margin-top: 0;
  }
  dl {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 7px 12px;
    margin: 0;
    font-size: 12.5px;
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .none {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
    color: var(--text-3);
    font-size: 12.5px;
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
  .work {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .wk {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .wl {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 12.5px;
  }
  .wt {
    flex: 1;
    min-width: 0;
    color: var(--priv-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wt.bad {
    color: var(--st-fail-text);
    white-space: normal;
  }
  .wp {
    font-size: 12px;
    color: var(--text-2);
  }
  .wn {
    font-size: 11.5px;
    color: var(--text-3);
    overflow-wrap: anywhere;
  }
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
</style>
