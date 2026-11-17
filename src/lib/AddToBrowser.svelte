<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Add Pako to your browser": one button per installed browser. Chromium browsers get their
  // extensions page opened with the extension's folder already copied, and three short steps;
  // Firefox and the browsers built on it (Zen, LibreWolf, Waterfox, Floorp) get the signed file
  // built into Pako opened (they then offer to add Pako).
  // A browser whose extension talked to Pako lately shows "Connected" (noted by the native host).
  import { app } from "./state.svelte";
  import { api, type AddedToBrowser, type Browser, type BrowserSeen } from "./api";
  import Icon from "./Icon.svelte";
  import { rowSlide } from "./motion";

  /** `flat`: no box around it (inside another box, as in the first-start setup). */
  let { flat = false }: { flat?: boolean } = $props();

  let browsers = $state<Browser[] | null>(null);
  let busy = $state<string | null>(null);
  let added = $state<Record<string, AddedToBrowser>>({});
  /** When "Add to …" was clicked (unix seconds). Seen since (the extension says hello when it's
   *  installed): the browser shows Installed again, not "Open again". */
  let addedAt = $state<Record<string, number>>({});
  let copied = $state(false);

  api
    .browsersFound()
    .then((l) => (browsers = l ?? []))
    .catch(() => (browsers = []));

  // The browsers seen, read now and each time the window comes back (after adding Pako in the
  // browser, it turns Connected as soon as you're back). Nothing runs in between.
  let seen = $state<BrowserSeen[]>([]);
  function loadSeen() {
    api
      .browsersConnected()
      .then((l) => (seen = l ?? []))
      .catch(() => {});
  }
  $effect(() => {
    loadSeen();
    const back = () => document.visibilityState === "visible" && loadSeen();
    window.addEventListener("focus", loadSeen);
    document.addEventListener("visibilitychange", back);
    return () => {
      window.removeEventListener("focus", loadSeen);
      document.removeEventListener("visibilitychange", back);
    };
  });

  const DAY = 86400;
  /** Connected = talked to Pako in the last 30 days. */
  const RECENT = 30 * DAY;

  /** The latest note for this browser (either edition), if any. */
  function lastSeen(id: Browser["id"]): BrowserSeen | null {
    let best: BrowserSeen | null = null;
    for (const s of seen) if (s.browser === id && (!best || s.lastSeen > best.lastSeen)) best = s;
    return best;
  }

  /** "today", "yesterday", "3 days ago", "2 weeks ago", "4 months ago" (by calendar day). */
  function ago(unix: number): string {
    const day = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
    const days = Math.max(0, Math.round((day(new Date()) - day(new Date(unix * 1000))) / (DAY * 1000)));
    if (days === 0) return "today";
    if (days === 1) return "yesterday";
    if (days < 14) return `${days} days ago`;
    if (days < 60) return `${Math.floor(days / 7)} weeks ago`;
    const months = Math.floor(days / 30);
    return months < 24 ? `${months} months ago` : `${Math.floor(days / 365)} years ago`;
  }

  const SHORT: Record<Browser["id"], string> = {
    chrome: "Chrome",
    edge: "Edge",
    brave: "Brave",
    vivaldi: "Vivaldi",
    opera: "Opera",
    operagx: "Opera GX",
    firefox: "Firefox",
    zen: "Zen",
    librewolf: "LibreWolf",
    waterfox: "Waterfox",
    floorp: "Floorp",
  };
  const PAGE: Record<Browser["id"], string> = {
    chrome: "chrome://extensions",
    edge: "edge://extensions",
    brave: "brave://extensions",
    vivaldi: "vivaldi://extensions",
    opera: "opera://extensions",
    operagx: "opera://extensions",
    firefox: "about:addons",
    zen: "about:addons",
    librewolf: "about:addons",
    waterfox: "about:addons",
    floorp: "about:addons",
  };

  async function add(b: Browser) {
    busy = b.id;
    try {
      const at = Math.floor(Date.now() / 1000);
      added = { ...added, [b.id]: await api.addToBrowser(b.id) };
      addedAt = { ...addedAt, [b.id]: at };
    } catch (e) {
      app.toast(String(e), "error");
    }
    busy = null;
  }

  async function copyAgain(path: string) {
    try {
      await api.copyText(path);
      copied = true;
      setTimeout(() => (copied = false), 1600);
    } catch (e) {
      app.toast(String(e), "error");
    }
  }

  async function openFolder() {
    await app.run(async () => api.openFolder(await api.extensionFiles()));
  }
</script>

<div class="browsers" class:flat>
  {#if browsers === null}
    <div class="empty">Looking for your browsers…</div>
  {:else if browsers.length === 0}
    <div class="empty">
      No browser found that Pako's extension works in (Chrome, Edge, Brave, Vivaldi, Opera, Firefox, Zen, LibreWolf, Waterfox, Floorp).
      <button class="link" onclick={openFolder}>Open the extension folder</button>
    </div>
  {:else}
    {#each browsers as b (b.id)}
      {@const r = added[b.id]}
      {@const s = lastSeen(b.id)}
      {@const again = !!r && r.opened !== "unavailable" && !(s && s.lastSeen >= (addedAt[b.id] ?? Infinity))}
      {@const connected = !!s && Date.now() / 1000 - s.lastSeen <= RECENT}
      <div class="row">
        <div class="head">
          <div class="who">
            <span class="name"><Icon name="globe" size={14} />{b.name}</span>
            {#if s}
              <div class="status" class:connected in:rowSlide>
                <span class="dot"></span>
                <span class="state">{connected ? "Connected" : "Not used lately"}</span>
                {#if s.edition === "light"}<span class="sep">·</span><span>Pako Light</span>{/if}
                <span class="sep">·</span><span>last used {ago(s.lastSeen)}</span>
              </div>
            {/if}
          </div>
          {#if connected && !again && busy !== b.id}
            <!-- Installed and talking to Pako: nothing to do here (Open the extension folder above
                 is there for a reinstall). -->
            <span class="installed"><Icon name="check" size={14} />Installed</span>
          {:else}
            <button class="btn" disabled={busy === b.id} onclick={() => add(b)}>
              {busy === b.id ? "Opening…" : again ? "Open again" : `Add to ${SHORT[b.id]}`}
            </button>
          {/if}
        </div>

        {#if r && b.family === "chromium"}
          {#if r.opened === "folder"}
            <p class="note">{SHORT[b.id]} didn't open its extensions page, so the folder opened instead. Type <span class="mono">{PAGE[b.id]}</span> in {SHORT[b.id]}'s address bar.</p>
          {/if}
          <ol class="steps">
            <li>On the extensions page, turn on <b>Developer mode</b>.</li>
            <li>Click <b>Load unpacked</b>.</li>
            <li>Click the address box at the top of the window that opens, press <b>Ctrl+V</b>{r.copied ? " (the folder is already copied)" : ""} and <b>Enter</b>, then <b>Select Folder</b>.</li>
          </ol>
          <div class="path">
            <span class="mono">{r.folder}</span>
            <button class="link" onclick={() => copyAgain(r.folder)}>{copied ? "Copied" : r.copied ? "Copy again" : "Copy"}</button>
          </div>
        {:else if r && r.opened === "unavailable"}
          <p class="note">This copy of Pako doesn't include the Firefox version. Pako from <span class="mono">pako.sookera.com</span> has it.</p>
        {:else if r}
          {#if r.opened === "firefoxFolder"}
            <p class="note">{SHORT[b.id]} didn't open, so the folder opened instead: drag <span class="mono">pako-firefox.xpi</span> into a {SHORT[b.id]} window.</p>
          {/if}
          <ol class="steps">
            <li>{SHORT[b.id]} asks to add Pako: click <b>Add</b>.</li>
            <li>So downloads behind a login work and the Download button shows on videos, check it may use all websites: in {SHORT[b.id]}, open <span class="mono">{PAGE[b.id]}</span> › Pako › <b>Permissions</b> and turn on <b>Access your data for all websites</b> if it is off.</li>
          </ol>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .installed {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 12px;
    color: var(--st-done-text);
    font-weight: 500;
    white-space: nowrap;
  }
  .browsers {
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--raise);
  }
  .browsers.flat {
    border: none;
    border-radius: 0;
    background: transparent;
  }
  .row,
  .empty {
    padding: 10px 12px;
  }
  .flat .row,
  .flat .empty {
    padding: 10px 0;
  }
  .row + .row {
    border-top: 1px solid var(--line);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .who {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .status {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 6px;
    /* Under the name's text, past its icon (14px + 8px gap). */
    padding-left: 22px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-3);
  }
  .status .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-4);
    flex: none;
  }
  .status.connected .dot {
    background: var(--st-done);
  }
  .status.connected .state {
    color: var(--st-done-text);
    font-weight: 500;
  }
  .sep {
    color: var(--text-4);
  }
  .name {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
    color: var(--text);
  }
  .name :global(svg) {
    color: var(--text-3);
  }
  .empty,
  .note,
  .steps {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-3);
  }
  .note {
    margin: 8px 0 0;
  }
  .steps {
    margin: 8px 0 0;
    padding-left: 18px;
    display: grid;
    gap: 4px;
  }
  b {
    font-weight: 600;
    color: var(--text-2);
  }
  .mono {
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--text-2);
  }
  .path {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 8px;
    padding: 6px 6px 6px 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--bar);
  }
  .note .mono {
    white-space: nowrap;
  }
  .path .mono {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .link {
    flex: none;
    padding: 2px 4px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--accent-text);
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }
  .link:hover {
    color: var(--accent-hover);
    background: var(--accent-soft);
  }
  /* Light: the lighter coral on its pale tint is too faint to read; the darker one stays. */
  :global(:root[data-theme="light"]) .link:hover {
    color: var(--accent-text);
  }
</style>
