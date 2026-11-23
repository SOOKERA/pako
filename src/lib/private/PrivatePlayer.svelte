<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A private video in Pako's own player: decrypted as it plays, no readable copy anywhere,
  // nothing noted about it. While it plays with Pako's window in front, Private mode doesn't
  // lock for lack of keyboard or mouse (Windows locking still locks it).
  import { onMount } from "svelte";
  import { api, type Download } from "../api";
  import Modal from "../Modal.svelte";
  import { t } from "../i18n.svelte";

  let { d, onclose }: { d: Download; onclose: () => void } = $props();

  let src = $state<string | null>(null);
  let error = $state("");
  let video: HTMLVideoElement | undefined = $state();

  function report() {
    const on = !!video && !video.paused && !video.ended && document.visibilityState === "visible" && document.hasFocus();
    api.privatePlaying(on).catch(() => {});
  }

  onMount(() => {
    api
      .privatePlay(d.id)
      .then((info) => (src = info.url))
      .catch((e) => (error = String(e)));
    const tick = setInterval(report, 5000);
    window.addEventListener("blur", report);
    window.addEventListener("focus", report);
    document.addEventListener("visibilitychange", report);
    return () => {
      clearInterval(tick);
      window.removeEventListener("blur", report);
      window.removeEventListener("focus", report);
      document.removeEventListener("visibilitychange", report);
      api.privatePlaying(false).catch(() => {});
    };
  });
</script>

<Modal title={d.name} width={760} onclose={onclose}>
  <div class="stage">
    {#if src}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video bind:this={video} {src} controls autoplay onplay={report} onpause={report} onended={report} onerror={() => (error = t("This video can't be played in Pako."))}></video>
    {:else if error}
      <p class="err">{error}</p>
    {:else}
      <p class="wait">{t("Opening…")}</p>
    {/if}
  </div>
</Modal>

<style>
  .stage {
    display: grid;
    place-items: center;
    min-height: 240px;
    border-radius: var(--radius);
    background: #000;
    overflow: hidden;
  }
  video {
    width: 100%;
    max-height: 60vh;
    display: block;
  }
  .err,
  .wait {
    margin: 0;
    padding: 24px;
    color: #c9ccd3;
    text-align: center;
  }
</style>
