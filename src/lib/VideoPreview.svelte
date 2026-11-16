<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A small player in the details panel: check a video is the real thing before (or
  // while) downloading it. Plays only when asked, at a low quality (480p by default).
  import { app } from "./state.svelte";
  import { api } from "./api";
  import Icon from "./Icon.svelte";

  let { id }: { id: string } = $props();

  let quality = $state(app.settings?.previewQuality ?? 480);
  let src = $state<string | null>(null);
  let converted = $state(false);
  let error = $state<string | null>(null);
  let loading = $state(false);
  /** Bumped by each request and by a change of item: an older reply is dropped. */
  let ticket = 0;

  // Another item selected: stop and reset.
  $effect(() => {
    id;
    ticket++;
    src = null;
    error = null;
    loading = false;
  });

  async function play(q = quality) {
    const mine = ++ticket;
    loading = true;
    error = null;
    try {
      const info = await api.previewUrl(id, q);
      if (mine !== ticket) return;
      src = info.url;
      converted = info.converted;
    } catch (e) {
      if (mine === ticket) error = String(e);
    } finally {
      if (mine === ticket) loading = false;
    }
  }

  // The details panel closed (Ctrl+I), or Direct / the Dashboard shown: the panel stays in the
  // page, hidden, so the video would go on playing out of sight. It pauses instead.
  let video: HTMLVideoElement | undefined = $state();
  $effect(() => {
    if (video && (!app.panelOpen || app.view === "direct" || app.view === "dashboard")) video.pause();
  });

  function setQuality(q: number) {
    quality = q;
    if (src && converted) play(q);
  }
</script>

<section class="preview">
  <div class="label">
    Preview
    {#if src}<span class="muted">{converted ? `${quality}p` : "as it is"}</span>{/if}
  </div>
  <div class="screen">
    {#if src}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video bind:this={video} {src} controls autoplay preload="metadata"></video>
    {:else}
      <button class="play" disabled={loading} aria-label="Play a preview" onclick={() => play()}>
        <Icon name="play" size={18} />
      </button>
      <span class="hint">{loading ? "Starting…" : "Play a short check before downloading"}</span>
    {/if}
  </div>
  {#if error}
    <div class="note">{error}</div>
  {/if}
  <div class="seg" role="radiogroup" aria-label="Preview quality">
    {#each [360, 480, 720] as q (q)}
      <button role="radio" aria-checked={quality === q} class:on={quality === q} onclick={() => setQuality(q)}>{q}p</button>
    {/each}
  </div>
  {#if !app.ffmpeg}
    <div class="note">MP4 and WebM play as they are. Install ffmpeg (Settings › Features &amp; updates) for other formats and for {quality}p.</div>
  {/if}
</section>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }
  .label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .muted {
    margin-left: 6px;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 500;
  }
  .screen {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: var(--radius);
    background: #0b0d10;
    border: 1px solid var(--line);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }
  video {
    width: 100%;
    height: 100%;
    background: #000;
  }
  .play {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    border: none;
    display: grid;
    place-items: center;
    padding-left: 3px;
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
    transition:
      background var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .play:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
  }
  .play:active:not(:disabled) {
    transform: scale(0.97);
  }
  .hint {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .note {
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .seg {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius);
    background: var(--bar);
    border: 1px solid var(--line);
  }
  .seg button {
    height: 26px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 12px;
  }
  .seg button:hover:not(.on) {
    background: var(--raise);
    color: var(--text);
  }
  .seg button.on {
    background: var(--active);
    color: var(--text);
    font-weight: 600;
  }
</style>
