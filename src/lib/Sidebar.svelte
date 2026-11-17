<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { app, type FoundFilter, type Library } from "./state.svelte";
  import { api } from "./api";
  import type { MenuItem } from "./menu";
  import { SUGGESTED_TAGS } from "./direct";
  import { devices } from "./devices.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import { help } from "./help/help.svelte";
  import { fadeIn, flipIf, flipOpts, pop, rowSlide } from "./motion";

  let {
    onsettings,
    onmenu,
  }: { onsettings: (section?: "accounts") => void; onmenu?: (x: number, y: number, items: MenuItem[]) => void } = $props();

  // My devices: its badge (pairing questions, files moving) shows from any page.
  devices.start();

  // ---------- Direct: filters and your tags ----------
  const directShows: { key: string; label: string; icon?: string; dot?: string }[] = [
    { key: "all", label: "All contacts", icon: "user" },
    { key: "online", label: "Online", dot: "var(--st-done)" },
    // Coral like the "!" on a contact that waits for your answer (amber on a contact = waiting for them).
    { key: "waiting", label: "Waiting for you", dot: "var(--accent)" },
    { key: "favourites", label: "Favourites", icon: "star" },
  ];
  function directCount(key: string) {
    if (key === "waiting") return app.directWaiting;
    return app.directCounts[key as "all" | "online" | "favourites"] ?? 0;
  }
  let tagDialog = $state<{ mode: "new" | "rename" | "delete"; tag: string } | null>(null);
  let tagName = $state("");
  let tagError = $state("");
  function openTag(mode: "new" | "rename" | "delete", tag = "") {
    tagDialog = { mode, tag };
    tagName = mode === "rename" ? tag : "";
    tagError = "";
  }
  function tagMenu(e: MouseEvent, tag: string) {
    e.preventDefault();
    e.stopPropagation();
    onmenu?.(e.clientX, e.clientY, [
      { label: "Rename tag…", action: () => openTag("rename", tag) },
      { label: "Delete tag…", danger: true, action: () => openTag("delete", tag) },
    ]);
  }
  async function saveTag(name = tagName) {
    if (!tagDialog) return;
    tagError = "";
    try {
      if (tagDialog.mode === "new") {
        const made = await api.directAddTag(name);
        app.directFilter = `tag:${made}`;
      } else if (tagDialog.mode === "rename") {
        await api.directRenameTag(tagDialog.tag, name);
        if (app.directFilter === `tag:${tagDialog.tag}`) app.directFilter = `tag:${name.trim()}`;
      } else {
        await api.directDeleteTag(tagDialog.tag);
        if (app.directFilter === `tag:${tagDialog.tag}`) app.directFilter = "all";
      }
      tagDialog = null;
      await app.refreshDirect();
    } catch (err) {
      tagError = String(err);
    }
  }

  const libs: { key: Library; label: string; icon: string }[] = [
    { key: "all", label: "All downloads", icon: "list" },
    { key: "active", label: "Downloading", icon: "download" },
    { key: "waiting", label: "Waiting", icon: "clock" },
    { key: "done", label: "Finished", icon: "check" },
    { key: "problems", label: "Problems", icon: "alert" },
  ];

  const foundFilters: { key: FoundFilter; label: string; dot: string; hollow?: boolean }[] = [
    { key: "all", label: "Everything", dot: "" },
    { key: "ready", label: "Ready", dot: "var(--st-done)" },
    { key: "needs", label: "Only in the browser", dot: "var(--st-limit)" },
    { key: "broken", label: "Link broken", dot: "var(--st-fail)" },
    { key: "checking", label: "Checking", dot: "var(--st-check)", hollow: true },
  ];

  function pickLibrary(k: Library) {
    app.library = k;
    app.sourceFilter = null;
  }
  function pickSource(k: string) {
    app.sourceFilter = app.sourceFilter === k ? null : k;
  }
</script>

<aside>
  <div class="views" role="tablist" aria-label="Pako lists">
    <button class="view" data-tour="nav-downloads" class:on={app.view === "downloads"} role="tab" aria-selected={app.view === "downloads"} onclick={() => (app.view = "downloads")}>
      <span class="lead"><Icon name="download" size={16} stroke={1.9} />Downloads</span>
      {#if app.runningCount}
        <span class="run" title="{app.runningCount} downloading"><span class="live"></span>{app.runningCount}</span>
      {/if}
    </button>
    <button class="view" data-tour="nav-found" class:on={app.view === "found"} role="tab" aria-selected={app.view === "found"} onclick={() => (app.view = "found")}>
      <span class="lead"><Icon name="link" size={16} stroke={1.9} />Links</span>
      {#if app.foundCounts.all}<span class="badge" title="{app.foundCounts.all} waiting to be picked" transition:pop={{ duration: 180, from: 0.92, y: 0 }}>{app.foundCounts.all}</span>{/if}
    </button>
    <button class="view" data-tour="nav-send" class:on={app.view === "send"} role="tab" aria-selected={app.view === "send"} onclick={() => (app.view = "send")}>
      <span class="lead"><Icon name="upload" size={16} stroke={1.9} />Upload</span>
      {#if app.uploadingCount}
        <span class="run" title="{app.uploadingCount} uploading"><span class="live"></span>{app.uploadingCount}</span>
      {/if}
    </button>
    <button class="view" data-tour="nav-direct" class:on={app.view === "direct"} role="tab" aria-selected={app.view === "direct"} onclick={() => (app.view = "direct")}>
      <span class="lead"><Icon name="send" size={16} stroke={1.9} />Direct</span>
      {#if app.directWaiting}
        <span class="badge attn" title="Waiting for your answer" transition:pop={{ duration: 180, from: 0.92, y: 0 }}>{app.directWaiting}</span>
      {:else if app.directMoving}
        <span class="run" title="{app.directMoving} moving"><span class="live"></span>{app.directMoving}</span>
      {/if}
    </button>
    <button class="view" data-tour="nav-devices" class:on={app.view === "devices"} role="tab" aria-selected={app.view === "devices"} onclick={() => (app.view = "devices")}>
      <span class="lead"><Icon name="devices" size={16} stroke={1.9} />My devices</span>
      {#if devices.waiting}
        <span class="badge attn" title="Waiting for your answer" transition:pop={{ duration: 180, from: 0.92, y: 0 }}>{devices.waiting}</span>
      {:else if devices.moving}
        <span class="run" title="{devices.moving} moving"><span class="live"></span>{devices.moving}</span>
      {/if}
    </button>
    <button class="view" data-tour="nav-dashboard" class:on={app.view === "dashboard"} role="tab" aria-selected={app.view === "dashboard"} onclick={() => (app.view = "dashboard")}>
      <span class="lead"><Icon name="chart" size={16} stroke={1.9} />Dashboard</span>
    </button>
  </div>

  {#key app.view}
  <div class="filters" data-tour="filters" in:fadeIn={{ duration: 160 }}>
  {#if app.view !== "dashboard" && app.view !== "send" && app.view !== "devices"}<div class="label">Show</div>{/if}
  {#if app.view === "dashboard" || app.view === "send" || app.view === "devices"}
    <!-- the Dashboard has its own period picker -->
  {:else if app.view === "direct"}
    <nav>
      {#each directShows as f (f.key)}
        <button class="item" class:on={app.directFilter === f.key} onclick={() => (app.directFilter = f.key)}>
          <span class="lead">
            {#if f.dot}<span class="dot" style="--c: {f.dot}"></span>{:else}<Icon name={f.icon ?? "list"} size={15} />{/if}
            {f.label}
          </span>
          <span class="count mono" class:attn={f.key === "waiting" && app.directWaiting > 0}>{directCount(f.key)}</span>
        </button>
      {/each}
    </nav>
    <div class="label">Tags</div>
    <nav data-tour="tags">
      {#each app.directTags as t (t)}
        <button
          class="item tagrow"
          class:on={app.directFilter === `tag:${t}`}
          onclick={() => (app.directFilter = `tag:${t}`)}
          oncontextmenu={(e) => tagMenu(e, t)}
          title="{t} · right-click to rename or delete"
          transition:rowSlide={{ duration: 200 }}
          animate:flipIf={flipOpts}
        >
          <span class="lead"><Icon name="tag" size={15} /><span class="tname">{t}</span></span>
          <span class="count mono">{app.directCounts.tags[t] ?? 0}</span>
        </button>
      {/each}
      <button class="item newtag" onclick={() => openTag("new")}>
        <span class="lead"><Icon name="plus" size={15} />New tag</span>
      </button>
    </nav>
  {:else if app.view === "found"}
    <nav>
      {#each foundFilters as f (f.key)}
        <button class="item" class:on={app.foundFilter === f.key} onclick={() => (app.foundFilter = f.key)}>
          <span class="lead">
            {#if f.dot}<span class="dot" class:hollow={f.hollow} style="--c: {f.dot}"></span>{:else}<Icon name="list" size={15} />{/if}
            {f.label}
          </span>
          <span class="count mono">{app.foundCounts[f.key]}</span>
        </button>
      {/each}
    </nav>
  {:else}
  <nav>
    {#each libs as l (l.key)}
      <button class="item" class:on={app.library === l.key && !app.sourceFilter} onclick={() => pickLibrary(l.key)}>
        <span class="lead"><Icon name={l.icon} size={15} />{l.label}</span>
        <span class="count mono" class:alert={l.key === "problems" && app.counts.problems > 0}>{app.counts[l.key]}</span>
      </button>
    {/each}
  </nav>
  {/if}

  {#if app.view === "downloads" && app.sources.length}
    <div class="label">Sources</div>
    <nav>
      {#each app.sources as s (s.key)}
        <button class="item" class:on={app.sourceFilter === s.key} onclick={() => pickSource(s.key)}>
          <span class="lead" title={s.label}><span class="dot" style="background: {s.color}"></span><span class="tname">{s.label}</span></span>
          <span class="count mono">{s.count}</span>
        </button>
      {/each}
    </nav>
  {/if}
  </div>
  {/key}

  {#if tagDialog}
    {@const dlg = tagDialog}
    <Modal
      title={dlg.mode === "new" ? "New tag" : dlg.mode === "rename" ? `Rename "${dlg.tag}"` : `Delete "${dlg.tag}"?`}
      width={420}
      onclose={() => (tagDialog = null)}
    >
      {#if dlg.mode === "delete"}
        <p class="msg">The tag is taken off {app.directCounts.tags[dlg.tag] === 1 ? "1 contact" : `${app.directCounts.tags[dlg.tag] ?? 0} contacts`}. The contacts themselves stay.</p>
      {:else}
        <input
          class="input"
          bind:value={tagName}
          maxlength="30"
          placeholder={dlg.mode === "new" ? "Friends, Work…" : ""}
          aria-label="Tag name"
          onkeydown={(e) => e.key === "Enter" && tagName.trim() && saveTag()}
        />
        {#if dlg.mode === "new" && !app.directTags.length}
          <div class="suggest">
            <span class="hint">Or pick one:</span>
            {#each SUGGESTED_TAGS as sug (sug)}
              <button class="chip" onclick={() => saveTag(sug)}>{sug}</button>
            {/each}
          </div>
        {/if}
        {#if tagError}<p class="err">{tagError}</p>{/if}
        <p class="hint">Tags are only on this PC: your contacts never see them.</p>
      {/if}
      {#snippet actions()}
        <button class="btn ghost" onclick={() => (tagDialog = null)}>Cancel</button>
        {#if dlg.mode === "delete"}
          <button class="btn danger-fill" onclick={() => saveTag()}><Icon name="trash" size={13} />Delete tag</button>
        {:else}
          <button class="btn primary" disabled={!tagName.trim()} onclick={() => saveTag()}>{dlg.mode === "new" ? "Add tag" : "Rename"}</button>
        {/if}
      {/snippet}
    </Modal>
  {/if}

  <div class="bottom">
    <button class="item" data-tour="help" title="Help (F1)" onclick={() => help.show()}>
      <span class="lead"><Icon name="help" size={15} />Help</span>
    </button>
    <button class="item" onclick={() => onsettings("accounts")}>
      <span class="lead"><Icon name="user" size={15} />Accounts</span>
    </button>
    <button class="item" data-tour="settings" onclick={() => onsettings()}>
      <span class="lead"><Icon name="settings" size={15} />Settings</span>
    </button>
  </div>
</aside>

<style>
  /* The filters scroll on their own: the pages above and Help/Settings below always show. */
  .filters {
    flex: 1 1 auto;
    min-height: 64px;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    margin: 0 -10px;
    padding: 0 10px;
  }
  aside {
    width: 220px;
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 14px 10px 10px;
    background: var(--panel);
    border-right: 1px solid var(--line);
    overflow-y: auto;
  }
  .label {
    padding: 0 10px 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin-bottom: 18px;
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    text-align: left;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .item:hover {
    background: var(--raise);
    color: var(--text);
  }
  .item:active {
    transform: scale(0.98);
  }
  .item.on {
    background: var(--select);
    color: var(--text);
  }
  .views {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-bottom: 14px;
    margin-bottom: 14px;
    border-bottom: 1px solid var(--line);
  }
  .view {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding: 0 10px 0 12px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    font-size: 14px;
    font-weight: 500;
    text-align: left;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .view:hover {
    background: var(--raise);
    color: var(--text);
  }
  .view:active {
    transform: scale(0.98);
  }
  .view.on {
    background: var(--active);
    color: var(--text);
    font-weight: 600;
  }
  .view.on::before {
    content: "";
    position: absolute;
    left: 0;
    top: 9px;
    bottom: 9px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
    animation: grow var(--mid) var(--ease);
  }
  @keyframes grow {
    from {
      transform: scaleY(0);
    }
  }
  .badge {
    min-width: 20px;
    height: 18px;
    padding: 0 6px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    /* a plain count (links waiting to be picked), not an alarm: grey, not red */
    background: var(--line-strong);
    color: var(--text);
    font-size: 11px;
    font-weight: 600;
  }
  .run {
    height: 18px;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 7px;
    border-radius: 9px;
    background: rgba(77, 163, 255, 0.14);
    color: var(--st-run-chip);
    font-size: 11px;
    font-weight: 600;
  }
  .live {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--st-run);
    animation: breathe 1.6s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      opacity: 0.3;
    }
  }
  .lead {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .count {
    font-size: 11px;
    color: var(--text-3);
  }
  .count.alert {
    color: var(--st-fail-text);
  }
  /* Waiting for your answer: coral, Pako's "needs you" colour (red is for problems). */
  .count.attn {
    color: var(--accent-text);
    font-weight: 600;
  }
  .badge.attn {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
    background: var(--text-4);
  }
  .lead .dot {
    background: var(--c, var(--text-4));
  }
  .lead .dot.hollow {
    background: transparent;
    border: 1.5px solid var(--c);
  }
  .tname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .newtag {
    color: var(--text-3);
  }
  .suggest {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
  }
  .chip {
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--line-strong);
    border-radius: 13px;
    background: var(--raise);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }
  .chip:hover {
    background: var(--raise-hover);
  }
  .hint {
    margin: 8px 0 0;
    font-size: 12px;
    color: var(--text-3);
  }
  .suggest .hint {
    margin: 0 2px 0 0;
  }
  .msg {
    margin: 0;
    line-height: 1.55;
    color: var(--text-2);
  }
  .err {
    margin: 8px 0 0;
    font-size: 12.5px;
    color: var(--st-fail-text);
  }
  .bottom {
    margin-top: auto;
    padding-top: 8px;
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
</style>
