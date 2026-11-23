<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Upload › details panel (Ctrl+I, like Downloads): the picked uploaded item's files, its link
  // as a QR code, when it gets deleted, how the upload went, downloads over time and your note.
  // Only what its row doesn't already say.
  import { untrack } from "svelte";
  import { app } from "./state.svelte";
  import { api, type SentItem } from "./api";
  import { bytes, sentLook, shortDate, speed } from "./format";
  import { rowSlide, fadeIn } from "./motion";
  import { expiryLook, longDate, sevenMore, took, whoDeletes } from "./uploads";
  import Icon from "./Icon.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import { i18n, num, t } from "./i18n.svelte";

  const s = $derived<SentItem | null>(app.sent.find((x) => x.id === app.sentPicked) ?? null);
  const drive = $derived(s?.host === "drive");
  const uploading = $derived(!!s && app.sends.some((j) => j.id === s.id && j.status === "uploading"));
  const canDelete = $derived(!!s && !s.gone && !uploading && (s.canDelete ?? (app.sendCanDelete && !!(s.folderId || s.fileId))));
  /** Its "Delete after" is being changed right now: the buttons wait. */
  const changing = $derived(!!s && app.changingSent.includes(s.id));

  // Another upload picked: its details start at the top.
  let panel = $state<HTMLDivElement | null>(null);
  let shownId: string | null = null;
  $effect(() => {
    const id = s?.id ?? null;
    if (id !== shownId) {
      shownId = id;
      if (panel) panel.scrollTop = 0;
    }
  });

  let now = $state(Date.now());
  $effect(() => {
    const tick = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(tick);
  });

  // The link as a QR code, made on this PC (nothing is sent anywhere).
  let qr = $state<{ id: string; svg: string } | null>(null);
  $effect(() => {
    const id = s?.id;
    if (!id || s?.gone || qr?.id === id) return;
    api
      .sentQr(id)
      .then((svg) => {
        if (app.sentPicked === id) qr = { id, svg };
      })
      .catch(() => {});
  });

  // Your note: saved a moment after you stop typing, and when you leave the field.
  let note = $state("");
  let noteFor = "";
  let saved = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    if (s && s.id !== noteFor) {
      // Another upload shown before the last words typed were saved (no blur: picked from a
      // notification, say): they're saved to the upload they were typed for, then it switches.
      if (noteFor) untrack(() => saveNote());
      noteFor = s.id;
      note = s.note ?? "";
      saved = false;
    }
  });
  function saveNote() {
    clearTimeout(timer);
    const id = noteFor;
    const was = app.sent.find((x) => x.id === id);
    if (!was || note.trim() === (was.note ?? "").trim()) return;
    api
      .sentSetNote(id, note)
      .then(() => {
        saved = true;
        setTimeout(() => (saved = false), 1600);
        app.refreshSends();
      })
      .catch((e) => app.toast(String(e), "error"));
  }
  function typed() {
    saved = false;
    clearTimeout(timer);
    timer = setTimeout(saveNote, 900);
  }

  const look = $derived(s ? sentLook(s) : null);
  const exp = $derived(s ? expiryLook(s, now) : null);
  const avg = $derived(s?.secs && s.secs > 0 ? s.size / s.secs : null);
  const packing = $derived(!s ? "" : s.parts ? t("ZIP in {n} parts", { n: s.parts }) : s.asFiles ? t("As they are") : s.compressed ? t("Compressed ZIP") : s.files > 1 ? t("One ZIP") : t("As it is"));
  const listed = $derived(s?.contents ?? []);
  const more = $derived(s ? Math.max(0, s.files - listed.length) : 0);

  // Downloads over time: a small step chart from the counts Pako saw.
  const W = 260;
  const H = 54;
  const chart = $derived.by(() => {
    const pts = s?.countHistory ?? [];
    if (!s || !pts.length) return null;
    const from = s.date;
    const to = Math.max(now, pts[pts.length - 1][0] + 1);
    const top = Math.max(1, ...pts.map((p) => p[1]));
    const x = (t: number) => ((t - from) / (to - from)) * W;
    const y = (n: number) => H - 3 - (n / top) * (H - 8);
    let d = `M0 ${y(0)}`;
    for (const [at, n] of pts) d += ` H${x(at).toFixed(1)} V${y(n).toFixed(1)}`;
    d += ` H${W}`;
    return { line: d, area: `${d} V${H} H0 Z`, top };
  });
</script>

<div class="panel details-panel" bind:this={panel}>
  {#if s}
    {#key s.id}
      <div class="inner" in:fadeIn={{ duration: 160 }}>
        <section class="top">
          <div class="headline">
            <span class="st" class:gone={s.gone} class:partial={s.incomplete}>{s.gone ? t("Gone") : s.incomplete ? (uploading ? t("Uploading") : t("Not all uploaded")) : t("Uploaded")}</span>
            <span class="src"><Icon name={drive ? "drive" : "globe"} size={12} />{drive ? "Google Drive" : "Gofile"}</span>
            <button class="icon-btn" aria-label={t("Close details panel (Ctrl+I)")} title={t("Close (Ctrl+I)")} onclick={() => app.togglePanel()}>
              <Icon name="chevronsRight" size={14} stroke={2} />
            </button>
          </div>
          <div class="name">{s.name}</div>
          <div class="stats">
            <div><span class="k">{t("Size")}</span><span class="v mono">{bytes(s.size)}</span></div>
            <div><span class="k">{t("Files")}</span><span class="v mono">{num(s.files)}</span></div>
            <div>
              <span class="k">{t("Downloads")}</span>
              <span class="v mono" class:dim={s.downloads == null}>{s.downloads == null ? "—" : num(s.downloads)}</span>
            </div>
          </div>
        </section>

        {#if !s.gone}
          <section class="linksec">
            <div class="label">{t("Link")}</div>
            <div class="qrrow">
              <div class="qr" aria-label={t("QR code of the link")} role="img">
                {#if qr?.id === s.id}
                  <div class="qrimg" in:fadeIn={{ duration: 200 }}>{@html qr.svg}</div>
                {/if}
              </div>
              <div class="qrside">
                <button class="linkish mono" title={t("Open the download page")} onclick={() => app.run(() => api.sentOpen(s.id))}>{s.link.replace(/^https?:\/\//, "")}</button>
                <p class="hint">{t("Point a phone's camera at the code to open the link.")}</p>
              </div>
            </div>
          </section>

          <section>
            <div class="label">{t("Delete after")}</div>
            {#if exp}
              {#key exp.label}
                <div class="when {exp.tone}" in:fadeIn={{ duration: 180 }}>
                  <Icon name={s.deleteAt == null ? "check" : s.deleteError ? "alert" : "clock"} size={14} />
                  <div>
                    <div class="wt">{s.deleteAt == null ? t("Kept until you delete it") : exp.label}</div>
                    {#if s.deleteAt != null}<div class="wn">{longDate(s.deleteAt)}. {whoDeletes(s)}</div>{/if}
                  </div>
                </div>
              {/key}
            {/if}
            {#if s.deleteError}
              <div class="callout err" transition:rowSlide><Icon name="alert" size={14} /><span>{t("{error} Pako tries again every hour.", { error: s.deleteError })}</span></div>
            {/if}
            {#if canDelete}
              <div class="acts">
                {#if s.deleteAt != null}
                  <button class="btn small" disabled={changing} onclick={() => app.setSentExpiry(s, sevenMore(s, Date.now()))}><Icon name="clock" size={12} />{t("Keep 7 more days")}</button>
                  <button class="btn small" disabled={changing} onclick={() => app.setSentExpiry(s, null)}><Icon name="check" size={12} />{t("Keep")}</button>
                {:else}
                  <button class="btn small" disabled={changing} onclick={() => app.setSentExpiry(s, sevenMore(s, Date.now()))}><Icon name="clock" size={12} />{t("Delete in 7 days")}</button>
                {/if}
              </div>
            {/if}
          </section>
        {/if}

        <section>
          <div class="label">{t("Files inside")}<span class="muted">{num(s.files)} · {bytes(s.size)}</span></div>
          {#if listed.length}
            <ul class="files">
              {#each listed as f, i (i)}
                <li>
                  <FileIcon name={f.name} size={14} />
                  <MiddleText text={f.name} class="fn" />
                  <span class="fs mono">{bytes(f.size)}</span>
                </li>
              {/each}
            </ul>
            {#if more}<p class="none">{t("And {n} more.", { n: num(more) })}</p>{/if}
          {:else}
            <p class="none">{t("Pako didn't keep the list of files for this older upload.")}</p>
          {/if}
        </section>

        <section>
          <div class="label">{t("How it went")}</div>
          <dl>
            <dt>{t("Uploaded")}</dt>
            <dd title={new Date(s.date).toLocaleString(i18n.dates, { dateStyle: "full", timeStyle: "short" })}>{shortDate(s.date)}</dd>
            {#if s.secs}<dt>{t("Took")}</dt><dd>{took(s.secs)}</dd>{/if}
            {#if avg}<dt>{t("Average speed")}</dt><dd class="mono">{speed(avg)}</dd>{/if}
            {#if s.peak}<dt>{t("Top speed")}</dt><dd class="mono">{speed(s.peak)}</dd>{/if}
            <dt>{t("Uploaded as")}</dt>
            <dd>{packing}</dd>
            <dt>{t("Uploaded to")}</dt>
            <dd>{drive ? t("Your Google Drive") : "Gofile"}</dd>
          </dl>
        </section>

        {#if chart}
          <section>
            <div class="label">{t("Downloads by others")}<span class="muted">{look?.label ?? ""}</span></div>
            <svg class="chart" viewBox="0 0 {W} {H}" preserveAspectRatio="none" role="img" aria-label={t("Downloads over time: {n} so far", { n: s.downloads ?? 0 })}>
              <path class="area" d={chart.area} />
              <path class="line" d={chart.line} />
            </svg>
            <div class="axis"><span>{shortDate(s.date)}</span><span>{t("Now")}</span></div>
            {#if s.downloadedAt}<p class="none">{t("Last downloaded around {date}. Gofile counts downloads, not who downloaded.", { date: shortDate(s.downloadedAt) })}</p>{/if}
          </section>
        {/if}

        <section class="notesec">
          <div class="label">
            {t("Note")}
            {#if saved}<span class="muted ok" transition:fadeIn={{ duration: 160 }}>{t("Saved")}</span>{/if}
          </div>
          <textarea class="input" rows="3" maxlength="500" placeholder={t("Who it's for, what's inside… (only on this PC)")} bind:value={note} oninput={typed} onblur={saveNote}></textarea>
        </section>

        <!-- The buttons, at the bottom as in every details panel: the main one first. -->
        {#if !s.gone || canDelete}
          <div class="panel-actions">
            {#if !s.gone}
              <button class="btn primary" title={t("Copy the link")} onclick={() => app.copyLink(s.link)}><Icon name="copy" size={13} /><span class="l">{t("Copy link")}</span></button>
              <button class="btn" title={t("Open the download page in your browser")} onclick={() => app.run(() => api.sentOpen(s.id))}><Icon name="globe" size={13} /><span class="l">{t("Open page")}</span></button>
            {/if}
            {#if canDelete}
              <button class="btn danger" title={t("Delete it now from where it was uploaded")} onclick={() => (app.sentDeleting = s)}><Icon name="trash" size={13} /><span class="l">{t("Delete now…")}</span></button>
            {/if}
          </div>
        {/if}
      </div>
    {/key}
  {:else}
    <div class="nothing">
      <div class="headline end">
        <button class="icon-btn" aria-label={t("Close details panel")} onclick={() => app.togglePanel()}><Icon name="chevronsRight" size={14} stroke={2} /></button>
      </div>
      <div class="nothing-body">
        <Icon name="upload" size={22} />
        <p>{t("Select an upload to see its details.")}</p>
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
  .inner {
    display: flex;
    flex-direction: column;
    min-height: 100%;
  }
  section {
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }
  section:last-child {
    border-bottom: none;
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
    color: var(--st-done-text);
  }
  .st.partial {
    color: var(--st-wait-text);
  }
  .st.gone {
    color: var(--st-fail-text);
  }
  .src {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 5px;
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
    min-width: 0;
  }
  .k {
    font-size: 11px;
    color: var(--text-3);
  }
  .v {
    font-size: 16px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .v.dim {
    color: var(--text-4);
  }
  .label {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
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
    white-space: nowrap;
  }
  .muted.ok {
    color: var(--st-done-text);
  }
  /* ---------- link ---------- */
  .qrrow {
    display: grid;
    grid-template-columns: 104px minmax(0, 1fr);
    gap: 14px;
    align-items: start;
  }
  .qr {
    width: 104px;
    height: 104px;
    padding: 8px;
    border-radius: var(--radius);
    background: #fff;
  }
  .qrimg,
  .qrimg :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .qrside {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
  .linkish {
    max-width: 100%;
    padding: 0;
    border: none;
    background: none;
    color: var(--st-run-text);
    font-size: 12px;
    text-align: left;
    overflow-wrap: anywhere;
    cursor: pointer;
  }
  .linkish:hover {
    text-decoration: underline;
  }
  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--text-3);
  }
  /* ---------- delete after ---------- */
  .when {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    color: var(--text-2);
  }
  .when :global(svg) {
    margin-top: 1px;
    color: var(--text-3);
  }
  .when.soon :global(svg),
  .when.soon .wt {
    color: var(--st-retry-text);
  }
  .when.fail :global(svg),
  .when.fail .wt {
    color: var(--st-fail-text);
  }
  .wt {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }
  .wn {
    margin-top: 3px;
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .callout {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin-top: 10px;
    padding: 9px 10px;
    border-radius: var(--radius);
    font-size: 12px;
    line-height: 1.4;
  }
  .callout.err {
    background: var(--tint-fail);
    color: var(--st-fail-text);
  }
  .callout :global(svg) {
    flex: none;
    margin-top: 1px;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 12px;
  }
  .btn.small {
    height: 28px;
    padding: 0 10px;
    font-size: 12px;
  }
  /* ---------- files ---------- */
  .files {
    max-height: 188px;
    overflow-y: auto;
    margin: 0;
    padding: 2px 0;
    list-style: none;
    border: 1px solid var(--line-soft);
    border-radius: var(--radius);
    background: var(--raise);
  }
  .files li {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr) auto;
    align-items: center;
    gap: 9px;
    height: 30px;
    padding: 0 10px;
  }
  .files li + li {
    border-top: 1px solid var(--line-soft);
  }
  .files :global(.fn) {
    min-width: 0;
    font-size: 12px;
    color: var(--text-2);
  }
  .fs {
    font-size: 11px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .none {
    margin: 8px 0 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .label + .none {
    margin-top: 0;
  }
  /* ---------- how it went ---------- */
  dl {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 8px 14px;
    margin: 0;
    font-size: 12px;
  }
  dt {
    color: var(--text-3);
    white-space: nowrap;
  }
  dd {
    margin: 0;
    text-align: right;
    overflow-wrap: anywhere;
    color: var(--text-2);
  }
  /* ---------- downloads over time ---------- */
  .chart {
    display: block;
    width: 100%;
    height: 54px;
  }
  .chart .area {
    fill: rgba(70, 201, 126, 0.12);
  }
  .chart .line {
    fill: none;
    stroke: var(--st-done);
    stroke-width: 1.6;
    vector-effect: non-scaling-stroke;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    margin-top: 4px;
    font-size: 11px;
    color: var(--text-4);
  }
  /* ---------- note ---------- */
  textarea {
    width: 100%;
    min-height: 66px;
    padding: 8px 10px;
    resize: vertical;
    font: inherit;
    font-size: 12.5px;
    line-height: 1.45;
  }
  .nothing {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .nothing-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--text-4);
    padding-bottom: 60px;
  }
  .nothing-body p {
    margin: 0;
    font-size: 12px;
    color: var(--text-3);
  }
</style>
