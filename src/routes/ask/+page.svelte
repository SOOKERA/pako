<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Download with Pako": a download from the browser waits here for your OK, with its name,
  // folder, quality (videos) and "Notify me". Several at once come one after another.
  import { onMount, tick } from "svelte";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import { api, inApp, listen, type AskView, type PrivateStatus } from "#lib/api.ts";
  import { bytes } from "#lib/format.ts";
  import FileIcon from "#lib/FileIcon.svelte";
  import Logo from "#lib/Logo.svelte";
  import MiddleText from "#lib/MiddleText.svelte";
  import WinButton from "#lib/WinButton.svelte";
  import { fadeIn, pop } from "#lib/motion.ts";
  import ContextMenu from "#lib/ContextMenu.svelte";
  import Icon from "#lib/Icon.svelte";
  import SegPick from "#lib/SegPick.svelte";
  import { ALERTS, type Alert } from "#lib/alert.ts";
  import type { MenuItem } from "#lib/menu.ts";
  import { devicesApi, downloadOnMenu, targets, type DownloadTarget } from "#lib/devices.svelte.ts";
  import { t, tMaybe } from "#lib/i18n.svelte.ts";

  let view = $state<AskView | null>(null);
  /** "Download on" another PC: offered while one is online, never for a video caught on a page
   *  or a recording (only this PC can make those). */
  const sendable = $derived(targets.list.length > 0 && !view?.hereOnly);
  /** Private mode is on: "Download privately" (straight into it, encrypted) is offered too. */
  let privOn = $state(false);
  let stem = $state("");
  let dir = $state("");
  let quality = $state(1080);
  /** Off · Pop-up · Important for this download (starts at Settings' New downloads). */
  let alert = $state<Alert>("pop");
  let dontAsk = $state(false);
  let busy = $state(false);
  let error = $state("");
  let nameEl: HTMLInputElement | undefined = $state();

  // Words given when shown, so they follow a language switch.
  const QUALITIES: [number, () => string][] = [
    [1080, () => t("Up to {q}p", { q: 1080 })],
    [720, () => t("Up to {q}p", { q: 720 })],
    [480, () => t("Up to {q}p", { q: 480 })],
    [0, () => t("Audio only")],
  ];

  /** A video's kind of file follows its quality; anything else keeps its own. */
  const ext = $derived(view?.video ? (quality === 0 ? "m4a" : "mp4") : (view?.ext ?? ""));
  const size = $derived(view?.video ? (view.sizes.find(([q]) => q === quality)?.[1] ?? view.size) : (view?.size ?? null));
  /** Other downloads running: "Download now" goes before them, "Add to queue" after. */
  const busyElse = $derived((view?.running ?? 0) > 0);

  async function load() {
    // Couldn't ask: what's shown stays (the next change asks again).
    const v = await api.askCurrent().catch(() => undefined);
    if (v === undefined) return;
    // Another download joined the line: the one shown stays as you're editing it (name,
    // folder, quality, a click in progress); only its "1 of 3" changes.
    if (v && view && v.id === view.id) {
      view = { ...view, position: v.position, total: v.total, running: v.running };
      return;
    }
    view = v;
    error = "";
    busy = false;
    if (!v) return;
    stem = v.stem;
    dir = v.dir;
    quality = v.quality;
    alert = v.alert;
    await tick();
    nameEl?.focus();
    nameEl?.select();
  }

  // The arrow next to the main button (while one of your devices can take links, or Private
  // mode is on): "Download on" another PC, and "Download privately". The main button always
  // downloads on this PC, the usual way.
  let onMenu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
  const more = $derived(sendable || privOn);
  function openOn(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const items: MenuItem[] = sendable ? downloadOnMenu(targets.list, () => answer("now"), sendTo, true) : [{ label: t("Download on this PC"), icon: "download", action: () => answer("now") }];
    if (privOn) {
      // Right under "Download on this PC": into Private mode, encrypted from its first byte.
      items.splice(1, 0, { label: t("Download privately"), icon: "lock", hint: t("Private mode"), action: () => answer("now", true) });
    }
    onMenu = { x: r.right, y: r.top - 6, items };
  }
  /** That device downloads it instead, and it leaves this PC (the window moves on). */
  async function sendTo(target: DownloadTarget) {
    if (!view || busy) return;
    busy = true;
    error = "";
    try {
      await devicesApi.sendAsk(view.id, target.id, view.video ? quality : null);
      if (!inApp) busy = false;
    } catch (e) {
      error = t("Not sent to {name}: {why}", { name: target.name, why: String(e) });
      busy = false;
    }
  }

  onMount(() => {
    targets.start();
    load();
    api
      .privateStatus()
      .then((st) => (privOn = st.on))
      .catch(() => {});
    if (!inApp) return;
    const un = listen("ask-changed", () => load());
    const unPriv = listen<PrivateStatus>("private-changed", (e) => (privOn = e.payload.on));
    return () => {
      un.then((f) => f());
      unPriv.then((f) => f());
    };
  });

  async function answer(action: "now" | "queue" | "cancel", privately = false) {
    if (!view || busy) return;
    busy = true;
    error = "";
    try {
      await api.askAnswer({ id: view.id, action, stem, dir, quality: view.video ? quality : null, alert, dontAsk, private: privately });
      if (!inApp) busy = false;
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }

  async function changeFolder() {
    const picked = await pickFolder({ directory: true, defaultPath: dir || undefined, title: t("Save to") });
    if (typeof picked === "string") dir = picked;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      answer("cancel");
    } else if (e.key === "Enter" && !(e.target instanceof HTMLButtonElement) && !(e.target instanceof HTMLSelectElement)) {
      e.preventDefault();
      answer("now");
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="ask">
  <div class="titlebar" data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region><Logo size={14} word={false} /></span>
    <span class="title" data-tauri-drag-region>{t("Download with Pako")}</span>
    {#if view && view.total > 1}{#key view.position}<span class="count" data-tauri-drag-region in:fadeIn={{ duration: 160 }}>{t("{n} of {total}", { n: view.position, total: view.total })}</span>{/key}{/if}
    <span class="grow" data-tauri-drag-region></span>
    <WinButton kind="close" small label={t("Cancel this download")} title={t("Cancel (Esc)")} onclick={() => answer("cancel")} />
  </div>

  {#if view}
    <!-- Each download (the first, then the next one waiting) comes in softly. -->
    {#key view.id}
    <div class="body" in:pop|global={{ duration: 240, from: 0.99, y: 6 }}>
      <div class="file">
        <FileIcon name={`${stem}.${ext}`} size={32} />
        <div class="what">
          <div class="nm" title={`${stem}.${ext}`}>{stem}{ext ? `.${ext}` : ""}</div>
          <div class="sub">{view.site}{size != null ? ` · ${view.video ? t("about {size}", { size: bytes(size) }) : bytes(size)}` : ""}</div>
        </div>
      </div>

      <label class="lbl" for="ask-name">{t("Name")}</label>
      <div class="name">
        <input id="ask-name" class="input" bind:this={nameEl} bind:value={stem} spellcheck="false" autocomplete="off" />
        {#if ext}<span class="ext mono">.{ext}</span>{/if}
      </div>

      <div class="row" class:two={view.video}>
        <div>
          <span class="lbl">{t("Save to")}</span>
          <div class="folder">
            <MiddleText text={dir} class="path" />
            <button class="link" onclick={changeFolder}>{t("Change")}</button>
          </div>
        </div>
        {#if view.video}
          <div>
            <label class="lbl" for="ask-quality">{t("Quality")}</label>
            <select id="ask-quality" class="input" bind:value={quality}>
              {#each QUALITIES as [q, label] (q)}
                <option value={q}>{label()}</option>
              {/each}
            </select>
          </div>
        {/if}
      </div>

      <div class="opts">
        <div class="notify" title={t("Notify when done")}>
          <Icon name={alert === "off" ? "bell-off" : "bell"} size={14} />
          <SegPick small label={t("Notify when done")} options={ALERTS.map((a) => ({ value: a.id, label: tMaybe(a.label), hint: tMaybe(a.hint) }))} value={alert} onpick={(a) => (alert = a)} />
        </div>
        <label class="dont" title={t("You can turn it back on in {place}", { place: `${t("Settings")} › ${t("Browser extension")}` })}>
          <button class="check" class:on={dontAsk} role="checkbox" aria-checked={dontAsk} aria-label={t("Don't ask again")} onclick={() => (dontAsk = !dontAsk)}>
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
          </button>
          <span>{t("Don't ask again")}</span>
        </label>
      </div>
    </div>
    {/key}

    <div class="foot">
      {#if error}<span class="err" role="alert" in:fadeIn={{ duration: 160 }}>{error}</span>{/if}
      <button class="btn ghost" disabled={busy} onclick={() => answer("cancel")}>{t("Cancel")}</button>
      {#if busyElse}
        <button class="btn" disabled={busy} onclick={() => answer("queue")}>{t("Add to queue")}</button>
        <button class="btn primary" class:joined={more} disabled={busy} onclick={() => answer("now")}>{t("Download now (ask window)")}</button>
      {:else}
        <button class="btn primary" class:joined={more} disabled={busy} onclick={() => answer("now")}>{t("Download")}</button>
      {/if}
      {#if more}
        <button
          class="btn primary more"
          disabled={busy}
          aria-label={sendable && privOn ? t("Download on another PC, or privately") : sendable ? t("Download on another PC") : t("Download privately")}
          aria-haspopup="menu"
          title={sendable && privOn ? t("Download on this PC, privately, or on one of your devices") : sendable ? t("Download on this PC or on one of your devices") : t("Download, or Download privately")}
          onclick={openOn}
        >
          <Icon name="chevron" size={12} stroke={2.6} />
        </button>
      {/if}
    </div>
  {/if}
</div>

{#if onMenu}
  <ContextMenu x={onMenu.x} y={onMenu.y} items={onMenu.items} onclose={() => (onMenu = null)} />
{/if}

<style>
  .ask {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    overflow: hidden;
  }
  /* The main window's title bar, scaled down (32 px instead of 40). */
  .titlebar {
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    background: var(--bar);
    border-bottom: 1px solid var(--line);
  }
  .brand {
    display: grid;
    place-items: center;
    padding: 0 8px 0 12px;
  }
  .title {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-2);
  }
  .count {
    margin-left: 8px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .grow {
    flex: 1;
    align-self: stretch;
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 16px 16px 0;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }
  .what {
    min-width: 0;
  }
  .nm {
    font-size: 13.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    margin-top: 2px;
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lbl {
    display: block;
    margin-bottom: 6px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .input {
    height: 32px;
  }
  .name {
    position: relative;
    margin-bottom: 12px;
  }
  .name .input {
    padding-right: 52px;
  }
  .ext {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 12px;
    color: var(--text-3);
    pointer-events: none;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 12px;
    margin-bottom: 16px;
  }
  .row.two {
    grid-template-columns: minmax(0, 1fr) 128px;
  }
  .folder {
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 4px 0 10px;
    border: 1px solid var(--control);
    border-radius: var(--radius);
    background: var(--bar);
  }
  .folder :global(.path) {
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .link {
    height: 24px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--focus-text);
    font-size: 12px;
    font-weight: 500;
  }
  .link:hover {
    background: var(--raise);
  }
  .opts {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .notify {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-3);
  }
  .foot {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--line);
    background: var(--head);
  }
  .dont {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-3);
    cursor: pointer;
  }
  /* On its own line over the buttons (beside three of them it had room for a word a line); the
     bar gives up a little padding so the choices above keep their place. */
  .foot:has(.err) {
    padding-top: 9px;
    padding-bottom: 9px;
  }
  /* The main button and its "Download on ▾" arrow, as one. */
  .btn.primary.joined {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .btn.primary.more {
    width: 30px;
    margin-left: -8px;
    padding: 0;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
    border-left: 1px solid rgba(0, 0, 0, 0.22);
  }
  .err {
    flex: 1 0 100%;
    min-width: 0;
    margin-bottom: -3px;
    font-size: 11.5px;
    line-height: 1.35;
    color: var(--st-fail-text);
  }
</style>
