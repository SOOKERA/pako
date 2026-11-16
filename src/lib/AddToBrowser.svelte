<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Add Pako to your browser": one button per installed browser. Chromium browsers get their
  // extensions page opened with the extension's folder already copied, and three short steps;
  // Firefox gets the signed file opened (it then offers to add Pako), or a word about signing.
  import { app } from "./state.svelte";
  import { api, type AddedToBrowser, type Browser } from "./api";
  import Icon from "./Icon.svelte";

  /** `flat`: no box around it (inside another box, as in the first-start setup). */
  let { flat = false }: { flat?: boolean } = $props();

  let browsers = $state<Browser[] | null>(null);
  let busy = $state<string | null>(null);
  let added = $state<Record<string, AddedToBrowser>>({});
  let copied = $state(false);

  api
    .browsersFound()
    .then((l) => (browsers = l ?? []))
    .catch(() => (browsers = []));

  const SHORT: Record<Browser["id"], string> = { chrome: "Chrome", edge: "Edge", brave: "Brave", vivaldi: "Vivaldi", firefox: "Firefox" };
  const PAGE: Record<Browser["id"], string> = {
    chrome: "chrome://extensions",
    edge: "edge://extensions",
    brave: "brave://extensions",
    vivaldi: "vivaldi://extensions",
    firefox: "about:addons",
  };

  async function add(b: Browser) {
    busy = b.id;
    try {
      added = { ...added, [b.id]: await api.addToBrowser(b.id) };
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
      No browser found that Pako's extension works in (Chrome, Edge, Brave, Vivaldi, Firefox).
      <button class="link" onclick={openFolder}>Open the extension folder</button>
    </div>
  {:else}
    {#each browsers as b (b.id)}
      {@const r = added[b.id]}
      {@const again = !!r && r.opened !== "needsSigning"}
      <div class="row">
        <div class="head">
          <span class="name"><Icon name="globe" size={14} />{b.name}</span>
          <button class="btn" disabled={busy === b.id} onclick={() => add(b)}>
            {busy === b.id ? "Opening…" : again ? "Open again" : `Add to ${SHORT[b.id]}`}
          </button>
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
        {:else if r && r.opened === "firefox"}
          <p class="note">Firefox is asking to add Pako: click <b>Add</b>. Then, so downloads behind a login work, allow it on all websites (<span class="mono">about:addons</span> › Pako › Permissions).</p>
        {:else if r}
          <p class="note">
            The Firefox version needs a free signature from Mozilla first (once). Put the signed <span class="mono">pako-firefox.xpi</span> in the extension folder, then press Add to Firefox again.
            <button class="link" onclick={() => app.run(() => api.extensionSteps())}>Show the steps</button>
          </p>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
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
