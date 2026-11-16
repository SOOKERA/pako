<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // One integrated bar instead of the Windows frame. Empty areas drag the window.
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, inApp } from "./api";
  import { app } from "./state.svelte";
  import { findLinks } from "./format";
  import Icon from "./Icon.svelte";
  import Logo from "./Logo.svelte";
  import WinButton from "./WinButton.svelte";
  import Bell from "./Bell.svelte";
  import { theme } from "./theme.svelte";

  let {
    onpastelinks,
    onnoticesettings,
  }: {
    onpastelinks: (text: string) => void;
    /** The bell's "Choose what shows here": Settings › Notifications. */
    onnoticesettings?: () => void;
  } = $props();
  let maximized = $state(false);
  let searchEl: HTMLInputElement | undefined = $state();

  const win = inApp ? getCurrentWindow() : null;

  onMount(() => {
    if (!win) return;
    const check = async () => (maximized = await win.isMaximized());
    check();
    const un = win.onResized(check);
    return () => {
      un.then((f) => f());
    };
  });

  export function focusSearch() {
    searchEl?.focus();
    searchEl?.select();
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === "Enter" && findLinks(app.search).length) {
      onpastelinks(app.search);
      app.search = "";
    }
    if (e.key === "Escape") {
      // Only the search box: the window's Esc (clear the selection / untick all) doesn't run.
      e.preventDefault();
      e.stopPropagation();
      app.search = "";
      searchEl?.blur();
    }
  }

  function onSearchPaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData("text") ?? "";
    if (findLinks(text).length) {
      e.preventDefault();
      onpastelinks(text);
    }
  }
</script>

<div class="bar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <Logo size={20} />
  </div>

  <div class="center" data-tauri-drag-region>
    <label class="search" data-tour="search" class:filled={app.search}>
      <Icon name="search" size={13} stroke={2} />
      <input
        bind:this={searchEl}
        bind:value={app.search}
        placeholder="Search, or paste links to download"
        aria-label="Search downloads, or paste links"
        spellcheck="false"
        onkeydown={onSearchKey}
        onpaste={onSearchPaste}
      />
      {#if app.search}
        <button class="clear" aria-label="Clear search" onclick={() => (app.search = "")}><Icon name="x" size={11} stroke={2.2} /></button>
      {:else}
        <kbd>Ctrl F</kbd>
      {/if}
    </label>
  </div>

  <div class="right">
    <Bell onsettings={onnoticesettings} />
    <!-- Light or dark: shows what a click gives (a sun in dark, a moon in light). -->
    <button
      class="tool mode"
      class:light={theme.shown === "light"}
      aria-label={theme.shown === "light" ? "Dark mode" : "Light mode"}
      title={theme.shown === "light" ? "Dark mode" : "Light mode"}
      onclick={() => app.run(() => theme.toggle())}
    >
      <svg class="sun" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4" />
      </svg>
      <svg class="moon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true">
        <path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z" />
      </svg>
    </button>
    <button
      class="tool layout"
      class:on={app.panelOpen}
      aria-label={app.panelOpen ? "Hide details panel (Ctrl+I)" : "Show details panel (Ctrl+I)"}
      title={app.panelOpen ? "Hide details panel (Ctrl+I)" : "Show details panel (Ctrl+I)"}
      onclick={() => app.togglePanel()}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
        <rect x="3" y="5" width="18" height="14" rx="1.5" />
        <path d="M15 5v14" />
        <path class="fill" d="M15 5h6v14h-6z" fill="currentColor" stroke="none" />
      </svg>
    </button>
    <span class="sep"></span>
    {#if !app.settings?.minimizeToTray}
      <WinButton kind="tray" label="Hide in the taskbar tray" title="Hide in the taskbar tray (keeps downloading)" onclick={() => app.run(() => api.hideToTray())} />
    {/if}
    <WinButton
      kind="minimize"
      label={app.settings?.minimizeToTray ? "Minimize to the taskbar tray" : "Minimize"}
      onclick={() => (app.settings?.minimizeToTray ? app.run(() => api.hideToTray()) : win?.minimize())}
    />
    <WinButton kind={maximized ? "restore" : "maximize"} label={maximized ? "Restore" : "Maximize"} onclick={() => win?.toggleMaximize()} />
    <WinButton kind="close" label="Close" onclick={() => win?.close()} />
  </div>
</div>

<style>
  .bar {
    height: 40px;
    flex: none;
    display: grid;
    grid-template-columns: 220px 1fr auto;
    align-items: center;
    background: var(--bar);
    border-bottom: 1px solid var(--line);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-left: 16px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .center {
    display: flex;
    justify-content: center;
    padding: 0 16px;
    min-width: 0;
  }
  .search {
    width: min(460px, 100%);
    height: 28px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px 0 10px;
    border-radius: var(--radius);
    border: 1px solid var(--control);
    background: var(--panel);
    color: var(--text-3);
    font-size: 12px;
    transition:
      border-color var(--fast) ease,
      box-shadow var(--fast) ease,
      background var(--fast) ease;
  }
  .search:focus-within {
    border-color: var(--focus);
    box-shadow: 0 0 0 3px var(--focus-soft);
    background: var(--bar);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
  }
  .search input::placeholder {
    color: var(--text-3);
  }
  .clear {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: var(--raise);
    color: var(--text-3);
  }
  .right {
    display: flex;
    align-items: center;
    align-self: stretch;
  }
  .tool {
    width: 32px;
    height: 28px;
    margin-right: 8px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-3);
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .tool:hover {
    background: var(--raise);
    color: var(--text);
  }
  .tool:active {
    transform: scale(0.97);
  }
  .mode {
    margin-right: 2px;
  }
  /* The sun and the moon sit on top of each other: one fades and turns a little into the other. */
  .mode svg {
    grid-area: 1 / 1;
    transition:
      opacity var(--mid) var(--ease),
      transform var(--mid) var(--ease);
  }
  .mode .moon,
  .mode.light .sun {
    opacity: 0;
    transform: scale(0.6) rotate(-30deg);
  }
  .mode.light .moon {
    opacity: 1;
    transform: none;
  }
  .layout .fill {
    opacity: 0;
    transition: opacity var(--mid) ease;
  }
  .layout.on {
    color: var(--accent-strong);
  }
  .layout.on .fill {
    opacity: 0.28;
  }
  .sep {
    width: 1px;
    height: 18px;
    background: var(--line);
  }
</style>
