<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Details panel of the Found tab: what the link is, and its own settings
  // (save folder, priority, notify) before it downloads.
  import { app } from "./state.svelte";
  import { api, canDownload, type FoundItem, type Priority } from "./api";
  import { bytes, duration, extension, pickQuality, sourceOf } from "./format";
  import Icon from "./Icon.svelte";
  import ItemSettings from "./ItemSettings.svelte";
  import VideoPreview from "./VideoPreview.svelte";
  import { isVideo } from "./format";

  const items = $derived(app.foundFocusItems);
  const one = $derived(items.length === 1 && !app.foundFocus?.startsWith("g:") ? items[0] : null);
  /** The focused group is a split archive (not a Drive folder). */
  const set = $derived(app.foundFocus?.startsWith("g:") ? app.foundSets.find((s) => "g:" + s.key === app.foundFocus) : undefined);
  const title = $derived(one ? one.template.name : set ? set.base : (items[0]?.folder ?? ""));
  const size = $derived(items.reduce((s, f) => s + (f.template.size ?? 0), 0));
  const defaultDest = $derived(app.foundDest ?? app.settings?.downloadDir ?? "");
  const able = $derived(items.filter(canDownload));
  const extractOn = $derived(set ? set.members.every((f) => f.extract ?? app.settings?.extractDefault ?? false) : false);

  function look(f: FoundItem) {
    switch (f.state) {
      case "ready":
        return { label: "Ready", color: "var(--st-done-text)" };
      case "checking":
        return { label: "Checking link", color: "var(--st-check-text)" };
      case "opening":
        return (f.message ?? "").startsWith("Waiting for you")
          ? { label: "Waiting for you", color: "var(--st-limit-text)" }
          : { label: "Opening link…", color: "var(--st-check-text)" };
      case "broken":
        return { label: "Link broken", color: "var(--st-fail-text)" };
      case "needsTools":
        return { label: "Needs the video tools", color: "var(--st-limit-text)" };
      default:
        return { label: "Only in the browser", color: "var(--st-limit-text)" };
    }
  }

  async function change(o: { dest?: string | null; priority?: Priority; notify?: boolean; extract?: boolean }) {
    // Extracting is about the whole set, even parts hidden by the filter.
    const target = o.extract !== undefined && set ? set.members : items;
    await app.run(() => api.foundSet(target.map((f) => f.id), o));
    await app.refresh();
  }

  // ---------- video quality ----------
  /** The video rows among the focused ones (quality applies to them). */
  const videos = $derived(items.filter((f) => f.template.source.kind === "video" && f.state !== "needsTools"));
  const defaultQ = $derived(app.settings?.videoQuality ?? 1080);
  /** Qualities to offer: every height any of them has (one video: its own list with sizes). */
  const heights = $derived.by(() => {
    const all = new Set<number>();
    for (const f of videos) if (f.template.source.kind === "video") for (const o of f.template.source.options ?? []) all.add(o.height);
    return [...all].sort((a, b) => b - a);
  });
  /** What the select shows: "default", a height, "m4a" or "mp3" (or "mixed" across several). */
  const current = $derived.by(() => {
    const vals = videos.map((f) => {
      const s = f.template.source;
      if (s.kind !== "video") return "default";
      if (s.audio) return s.audio;
      return s.quality == null ? "default" : String(s.quality);
    });
    return vals.length && vals.every((v) => v === vals[0]) ? vals[0] : "mixed";
  });
  function sizeFor(want: number, audio = false) {
    if (videos.length !== 1) return "";
    const s = videos[0].template.source;
    if (s.kind !== "video") return "";
    if (audio) return s.audioSize ? ` · ≈ ${bytes(s.audioSize)}` : "";
    const o = pickQuality(s.options ?? [], want);
    return o?.size ? ` · ≈ ${bytes(o.size)}` : "";
  }
  const defaultLabel = $derived(defaultQ === 0 ? "Audio only (M4A)" : `Best up to ${defaultQ}p`);
  async function setQuality(v: string) {
    const ids = videos.map((f) => f.id);
    if (!ids.length || v === "mixed") return;
    const quality = v === "default" ? null : v === "m4a" || v === "mp3" ? 0 : Number(v);
    const audio = v === "m4a" || v === "mp3" ? v : null;
    await app.run(() => api.foundSetVideo(ids, quality, audio));
    await app.refresh();
  }

  async function downloadThese() {
    const ids = able.map((f) => f.id);
    if (!ids.length) return;
    await app.run(async () => {
      const n = await api.foundDownload(ids, false, app.foundDest);
      await app.refresh();
      if (n) app.toast(`${n === 1 ? "1 file" : `${n} files`} moved to Downloads`);
    });
  }

  async function remove() {
    await app.removeFound(items.map((f) => f.id));
  }

  function when(ms: number) {
    const date = new Date(ms);
    const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    return new Date().toDateString() === date.toDateString() ? `Today, ${time}` : `${date.toLocaleDateString([], { day: "numeric", month: "short", year: "numeric" })}, ${time}`;
  }
</script>

<div class="panel">
  {#if items.length}
    {@const lk = one ? look(one) : { label: set ? "Split archive" : "Folder", color: "var(--text-2)" }}
    <section class="top">
      <div class="headline">
        <span class="st" style="color: {lk.color}">{lk.label}</span>
        <span class="src">{sourceOf(items[0].template).label}</span>
        <button class="icon-btn" aria-label="Close details panel" onclick={() => app.togglePanel()}><Icon name="chevronsRight" size={14} stroke={2} /></button>
      </div>
      <div class="name">{title}</div>
      <div class="facts mono">
        <span>{size ? bytes(size) : one?.state === "checking" ? "Size: checking…" : "Size unknown"}</span>
        {#if !one}<span>{items.length} {set ? (items.length === 1 ? "part" : "parts") : items.length === 1 ? "file" : "files"}</span>{/if}
      </div>
      {#if one && (one.state === "broken" || one.state === "needsAccount" || one.message)}
        <div class="callout" class:err={one.state === "broken"} class:limit={one.state === "needsAccount"}>
          <Icon name="alert" size={14} />
          <span>{one.message ?? look(one).label}</span>
        </div>
        {#if one.state === "needsAccount"}
          <button class="btn primary small" onclick={() => app.run(() => api.openInDrive(one.id))}>
            <Icon name="globe" size={13} />Open in browser
          </button>
        {/if}
      {/if}
    </section>

    {#if one && isVideo(one.template.name) && one.state !== "broken" && one.template.source.kind !== "video"}
      <VideoPreview id={one.id} />
    {/if}

    <ItemSettings
      dest={items[0].dest ?? defaultDest}
      isDefault={items.every((f) => !f.dest)}
      priority={items[0].template.priority}
      notify={items.every((f) => f.template.notify)}
      count={items.length}
      onDest={(dir) => change({ dest: dir })}
      onResetDest={() => change({ dest: null })}
      onPriority={(p) => change({ priority: p })}
      onNotify={(on) => change({ notify: on })}
    >
      {#snippet extra()}
        {#if videos.length}
          <div class="qfield">
            <span class="qk">Quality{#if videos.length > 1}<span class="qn">{` · for ${videos.length} videos`}</span>{/if}</span>
            <select class="input" aria-label="Video quality" value={current} onchange={(e) => setQuality(e.currentTarget.value)}>
              {#if current === "mixed"}<option value="mixed" disabled>Different for each</option>{/if}
              <option value="default">Default: {defaultLabel}{sizeFor(defaultQ, defaultQ === 0)}</option>
              {#each heights as h (h)}
                <option value={String(h)}>{h}p{h >= 2160 ? " (4K)" : h >= 1440 ? " (2K)" : ""}{sizeFor(h)}</option>
              {/each}
              <option value="m4a">Audio only (M4A){sizeFor(0, true)}</option>
              <option value="mp3">Audio only (MP3){sizeFor(0, true)}</option>
            </select>
            <span class="qnote">If a video doesn't have the quality, Pako takes the closest lower one.</span>
          </div>
        {/if}
        {#if set}
          <div class="xrow">
            <div>
              <div class="xt">Extract when all parts are done</div>
              <div class="xn">
                {#if !app.sevenZip}Needs 7-Zip (Settings › Features &amp; updates).{:else}Into the set's folder, next to the parts.{/if}
              </div>
            </div>
            <button class="switch" class:on={extractOn} role="switch" aria-checked={extractOn} aria-label="Extract when all parts are done" onclick={() => change({ extract: !extractOn })}></button>
          </div>
          {#if set.missing.length}
            <div class="callout err"><Icon name="alert" size={14} /><span>Incomplete: {set.missing.join(", ")}. The set can't be extracted without it.</span></div>
          {/if}
        {/if}
      {/snippet}
    </ItemSettings>

    {#if one}
      <section class="details">
        <div class="label">Details</div>
        <dl>
          <dt>Link</dt><dd class="link" title={one.template.link}>{one.template.link}</dd>
          {#if extension(one.template.name)}<dt>Type</dt><dd>{extension(one.template.name)}</dd>{/if}
          {#if one.folder}<dt>In folder</dt><dd>{one.folder}{one.relDir ? `\\${one.relDir}` : ""}</dd>{/if}
          {#if one.template.source.kind === "video"}<dt>Length</dt><dd>{duration(one.template.source.duration)}</dd>{/if}
          <dt>Added</dt><dd>{when(one.template.date || one.template.addedAt)}</dd>
        </dl>
      </section>
    {/if}

    <div class="actions">
      <button class="btn primary" disabled={!able.length} onclick={downloadThese}>
        <Icon name="download" size={13} stroke={2.4} />{able.length > 1 ? `Download ${able.length}` : "Download"}
      </button>
      <button class="btn danger" onclick={remove}><Icon name="trash" size={13} />Remove</button>
    </div>
  {:else}
    <div class="none">
      <div class="headline end">
        <button class="icon-btn" aria-label="Close details panel" onclick={() => app.togglePanel()}><Icon name="chevronsRight" size={14} stroke={2} /></button>
      </div>
      <div class="none-body">
        <Icon name="link" size={22} />
        <p>Select a link to see its details and settings.</p>
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
  .facts {
    display: flex;
    gap: 14px;
    font-size: 12px;
    color: var(--text-2);
  }
  .callout {
    display: flex;
    gap: 8px;
    padding: 9px 11px;
    border-radius: var(--radius);
    background: var(--raise);
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-2);
  }
  .callout :global(svg) {
    flex: none;
    margin-top: 1px;
  }
  .callout.err {
    background: var(--tint-fail);
    color: var(--st-fail-text);
  }
  .callout.limit {
    background: var(--tint-limit);
    color: var(--st-limit-text);
  }
  .qfield {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .qk {
    font-size: 12px;
    color: var(--text-3);
  }
  .qn {
    color: var(--text-4);
  }
  .qfield select {
    width: 100%;
  }
  .qnote {
    font-size: 11.5px;
    color: var(--text-3);
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
  .small {
    height: 30px;
    align-self: flex-start;
  }
  .label {
    margin-bottom: 10px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  dl {
    display: grid;
    grid-template-columns: 78px minmax(0, 1fr);
    row-gap: 8px;
    margin: 0;
    font-size: 12.5px;
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
    color: var(--text-2);
    text-align: right;
    overflow-wrap: anywhere;
  }
  dd.link {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .actions {
    margin-top: auto;
    display: flex;
    gap: 8px;
    padding: 14px 18px;
    border-top: 1px solid var(--line);
  }
  .actions .btn.primary {
    flex: 1;
  }
  .btn.danger {
    color: var(--st-fail-text);
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
    color: var(--text-3);
    padding: 0 24px 60px;
    text-align: center;
  }
  .none-body p {
    margin: 0;
    font-size: 12.5px;
  }
</style>
