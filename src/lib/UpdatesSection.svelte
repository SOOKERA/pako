<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import Credit from "./Credit.svelte";
  // Settings › Features & updates: Pako's own updates, then what Pako can do with the free
  // tools it downloads (named by what they do; the tools themselves are in Details).
  import { onMount } from "svelte";
  import { app } from "./state.svelte";
  import { api, type ComponentKind, type ComponentStatus } from "./api";
  import { bytes } from "./format";
  import Icon from "./Icon.svelte";
  import { rowSlide } from "./motion";
  import Collapse from "./Collapse.svelte";
  import AdvancedFold from "./AdvancedFold.svelte";
  import SetGroup from "./SetGroup.svelte";
  import { t, tMaybe, type Key } from "./i18n.svelte";

  let { autoUpdate = $bindable(), onsetup }: { autoUpdate: boolean; onsetup: () => void } = $props();

  /** `name` and `what` are keys, shown with t(). */
  type Feature = { id: string; name: Key; what: Key; kinds: ComponentKind[]; soon?: boolean; removes: ComponentKind[] };
  const FEATURES: Feature[] = [
    {
      id: "archives",
      name: "Extract split archives",
      what: "When every part of a set (.part1.rar, .7z.001…) is downloaded, Pako unpacks it for you.",
      kinds: ["sevenZip"],
      removes: ["sevenZip"],
    },
    {
      id: "preview",
      name: "Preview more video types",
      what: "Watch MKV, AVI and other videos in the details panel before downloading them.",
      kinds: ["ffmpeg"],
      removes: ["ffmpeg"],
    },
    {
      id: "video",
      name: "Download videos from YouTube and other sites",
      what: "Paste a video or playlist link, pick the quality (or audio only), and Pako does the rest.",
      kinds: ["ytDlp", "deno", "ffmpeg"],
      // ffmpeg stays: video previews use it too.
      removes: ["ytDlp", "deno"],
    },
  ];

  let list = $state<ComponentStatus[]>([]);
  let checking = $state(false);
  let open = $state<Record<string, boolean>>({});
  /** "What Pako can do": folded by default, opened by hand, or by itself when a tool has an update. */
  let toolsOpen = $state(false);
  /** Ready = every tool it needs is in place (one waiting for an update still works). */
  const readyCount = $derived(FEATURES.filter((f) => !f.soon && f.kinds.every((k) => tool(k)?.installed)).length);
  /** The tools' state arrived (until then the summary says nothing, rather than "0 of 3 ready"). */
  let loaded = $state(false);
  const updateCount = $derived(FEATURES.filter((f) => featureState(f) === "update").length);
  $effect(() => {
    if (updateCount > 0) toolsOpen = true;
  });
  let removing = $state<string | null>(null);

  onMount(() => {
    reload();
    api
      .updateStatus()
      .then((u) => {
        app.update = u;
      })
      .catch(() => {});
  });

  async function reload() {
    try {
      list = await api.componentsStatus();
      loaded = true;
    } catch {
      // Kept as it was.
    }
  }

  const tool = (k: ComponentKind) => list.find((c) => c.kind === k);

  function featureState(f: Feature): "soon" | "installed" | "partly" | "update" | "none" {
    const tools = f.kinds.map(tool).filter((c): c is ComponentStatus => !!c);
    const have = tools.filter((c) => c.installed);
    if (have.some((c) => c.update)) return "update";
    if (f.soon) return have.length ? "partly" : "soon";
    if (have.length === f.kinds.length) return "installed";
    return have.length ? "partly" : "none";
  }

  function busyOf(f: Feature) {
    for (const k of f.kinds) {
      const c = tool(k);
      const b = app.componentBusy[k] ?? (c?.installing ? { stage: "Installing", done: 0, total: null } : undefined);
      if (b) return { name: c?.name ?? k, ...b };
    }
    return null;
  }

  async function installOrUpdate(f: Feature) {
    for (const k of f.kinds) {
      const c = tool(k);
      if (!c) continue;
      const needs = !c.installed || c.update;
      if (needs) await app.installComponent(k, c.name);
    }
    await reload();
  }

  async function remove(f: Feature) {
    removing = f.id;
    try {
      // All of the feature's tools together: if one is in use, none is removed.
      const kinds = f.removes.filter((k) => tool(k)?.installed);
      if (kinds.length) await api.componentsRemove(kinds);
      app.toast(t("{feature}: removed", { feature: t(f.name) }));
      await app.refresh();
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      removing = null;
      await reload();
    }
  }

  async function checkTools() {
    checking = true;
    try {
      list = await api.componentsCheck();
      loaded = true;
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      checking = false;
    }
  }

  async function checkPako(get: boolean) {
    app.update = { ...(app.update ?? { current: app.version, latest: null, notes: null, ready: false, error: null, checked: false }), busy: true };
    app.update = await api.updateCheck(get).catch((e) => ({ ...app.update!, busy: false, error: String(e) }));
  }

  const u = $derived(app.update);
</script>

<section data-tour="set-updates">
  <h3>{t("Features & updates")}</h3>

  <SetGroup title="Pako">
  <div class="line">
    <div>
      <div class="t">Pako <span class="ver mono">{app.version}</span><span class="kind">· {app.installed ? t("Installed") : t("Portable")}</span></div>
      <div class="muted">
        {#if u?.busy}
          {t("Checking…")}
        {:else if u?.ready}
          {t("Pako {version} is ready. Restart to start using it.", { version: u.latest ?? "" })}
        {:else if u?.latest}
          {t("Pako {version} is available.", { version: u.latest })}{u.notes ? ` ${u.notes}` : ""}
          {#if u.error}<span class="err">{u.error}</span>{/if}
        {:else if u?.error}
          {u.error}
        {:else if u?.checked}
          {t("You have the latest version.")}
        {:else}
          {t("Pako looks for a new version once a day, also while it stays open in the tray.")}
        {/if}
      </div>
    </div>
    <div class="ctrl">
      {#if u?.ready}
        <button class="btn primary" onclick={() => app.restartForUpdate()}>{t("Restart now")}</button>
      {:else if u?.latest}
        <button class="btn primary" disabled={u.busy} onclick={() => checkPako(true)}>{t("Update")}</button>
      {:else}
        <button class="btn" disabled={u?.busy} onclick={() => checkPako(autoUpdate)}>{t("Check for updates")}</button>
      {/if}
    </div>
  </div>
  <!-- Under the whole row, so it stays on one line. -->
  <div class="by attached"><Credit /></div>
  <div class="line">
    <div>
      <div class="t">{t("Update Pako and its tools automatically")}</div>
      <div class="muted">{t("Once a day, also while Pako stays open in the tray. Tools update quietly; a new Pako waits for you to restart.")}</div>
    </div>
    <div class="ctrl">
      <button class="switch" class:on={autoUpdate} role="switch" aria-checked={autoUpdate} aria-label={t("Update Pako and its tools automatically")} onclick={() => (autoUpdate = !autoUpdate)}></button>
    </div>
  </div>
  </SetGroup>

  <SetGroup title={t("Features")}>
  <div data-fold>
  <div class="fhead">
    <button class="fold" data-fold-toggle aria-expanded={toolsOpen} aria-controls="set-tools" onclick={() => (toolsOpen = !toolsOpen)}>
      <span class="chev" class:on={toolsOpen}><Icon name="chevron" size={12} stroke={2.6} /></span>
      <span>
        <span class="t">{t("What Pako can do")}</span>
        <span class="muted sum">
          {#if loaded}{t("{n} of {total} ready", { n: readyCount, total: FEATURES.filter((f) => !f.soon).length })}{:else}&nbsp;{/if}{#if updateCount}<span class="upd">{" · "}{t("{n} updates", { n: updateCount })}</span>{/if}
        </span>
      </span>
    </button>
    <button class="btn" disabled={checking} onclick={checkTools}>{checking ? t("Checking…") : t("Check for new versions")}</button>
  </div>

  <Collapse open={toolsOpen} id="set-tools">
  <div class="muted fnote">{t("Some features use free tools that Pako downloads from their official sources and checks that each one arrived intact.")}</div>
  <div class="feats">
    {#each FEATURES as f (f.id)}
      {@const st = featureState(f)}
      {@const b = busyOf(f)}
      <div class="feat">
        <div class="frow">
          <div class="finfo">
            <div class="fname">
              {t(f.name)}
              {#if f.soon}<span class="pill">{t("Coming soon")}</span>{/if}
            </div>
            <div class="muted">{t(f.what)}</div>
          </div>
          <div class="fstate" class:ok={st === "installed"} class:upd={st === "update"}>
            {#if st === "installed"}<Icon name="check" size={12} stroke={2.6} />{t("Ready (feature)")}
            {:else if st === "update"}{t("Update available")}
            {:else if st === "partly" && !f.soon}{t("Partly installed")}
            {:else if st === "none"}{t("Not installed")}{/if}
          </div>
          <div class="factions">
            {#if !f.soon && (st === "none" || st === "partly")}
              <button class="btn" disabled={!!b} onclick={() => installOrUpdate(f)}>{b ? t("Working…") : t("Install")}</button>
            {:else if st === "update"}
              <button class="btn" disabled={!!b} onclick={() => installOrUpdate(f)}>{b ? t("Working…") : t("Update")}</button>
            {/if}
            {#if f.removes.some((k) => tool(k)?.installed)}
              <button class="btn ghost danger" disabled={!!b || removing === f.id} onclick={() => remove(f)}>{removing === f.id ? t("Removing…") : t("Remove")}</button>
            {/if}
          </div>
        </div>
        {#if b}
          <div class="reveal" transition:rowSlide={{ duration: 200 }}>
          <div class="prog">
            <div class="bar"><div class="fill" style="width: {b.total ? (b.done / b.total) * 100 : 8}%"></div></div>
            <span class="mono">{b.name} · {tMaybe(b.stage)}{b.total ? ` · ${t("{done} of {total} (bytes)", { done: bytes(b.done), total: bytes(b.total) })}` : ""}</span>
          </div>
          </div>
        {/if}
        <button class="more" aria-expanded={!!open[f.id]} onclick={() => (open = { ...open, [f.id]: !open[f.id] })}>
          <Icon name="chevron" size={11} stroke={2.6} />{t("Details")}
        </button>
        {#if open[f.id]}
          <div class="reveal" transition:rowSlide={{ duration: 220 }}>
          <div class="details">
            {#each f.kinds as k (k)}
              {@const c = tool(k)}
              {#if c}
                <div class="drow">
                  <span class="dname">{c.name}</span>
                  <span class="mono">{c.installed ? t("version {version}", { version: c.installed }) : t("not installed")}{c.latest && (c.update || !c.installed) ? ` · ${t("latest {version}", { version: c.latest })}` : ""}</span>
                  <span class="mono">{c.size ? bytes(c.size) : ""}</span>
                  <span class="src">{c.source}</span>
                </div>
              {/if}
            {/each}
          </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>
  </Collapse>
  </div>
  </SetGroup>

  <AdvancedFold tab="updates" hint={t("Setup: run the first-start setup again")}>
    <SetGroup title={t("Setup")}>
      <div class="line">
        <div>
          <div class="t">{t("First-start setup")}</div>
          <div class="muted">{t("Go through the welcome questions again.")}</div>
        </div>
        <div class="ctrl">
          <button class="btn" onclick={onsetup}>{t("Run setup again")}</button>
        </div>
      </div>
    </SetGroup>
  </AdvancedFold>
</section>

<style>
  /* The same as Settings' other tabs (SettingsDialog.svelte): a title, then titled cards. */
  section {
    padding: 0 0 4px;
  }
  h3 {
    margin: 2px 0 -4px 2px;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0;
    color: var(--text);
  }
  .line,
  .fhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 12px 14px;
  }
  .ctrl {
    flex: none;
  }
  .fhead .btn {
    flex: none;
  }
  .fold {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .fold > span:last-child {
    display: flex;
    flex-direction: column;
  }
  .fold .t {
    margin-bottom: 2px;
  }
  .chev {
    display: inline-flex;
    color: var(--text-3);
    transform: rotate(-90deg);
    transition: transform 0.16s ease;
  }
  .chev.on {
    transform: none;
  }
  .sum .upd {
    color: var(--st-done-text);
  }
  .fnote {
    margin: 0;
    padding: 0 14px 0 34px;
  }
  .t {
    font-weight: 500;
    margin-bottom: 3px;
  }
  .ver {
    margin-left: 6px;
    font-size: 12px;
    color: var(--text-3);
  }
  .by {
    margin-top: -4px;
    padding: 0 14px 12px;
  }
  .kind {
    margin-left: 6px;
    font-size: 12px;
    font-weight: 400;
    color: var(--text-3);
  }
  .muted {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .err {
    display: block;
    margin-top: 3px;
    color: var(--st-fail-text);
  }
  /* Inside its group's card: one feature per row, a hairline between, as the other rows. */
  .feats {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--line);
  }
  .feat {
    padding: 12px 14px 10px;
  }
  .feat + .feat {
    border-top: 1px solid var(--line);
  }
  .frow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 14px;
  }
  .fname {
    font-weight: 600;
  }
  .pill {
    display: inline-block;
    margin-left: 6px;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--line-strong);
    color: var(--text-2);
    font-size: 10.5px;
    font-weight: 600;
    vertical-align: 1px;
  }
  .fstate {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .fstate.ok {
    color: var(--st-done-text);
  }
  .fstate.upd {
    color: var(--st-run-text);
  }
  .factions {
    display: flex;
    gap: 6px;
  }
  .factions .btn {
    min-width: 84px;
    justify-content: center;
  }
  .more {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 8px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text-3);
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
  }
  .more:hover {
    color: var(--text-2);
  }
  .more :global(svg) {
    transition: transform var(--mid) var(--ease);
  }
  .more[aria-expanded="true"] :global(svg) {
    transform: rotate(180deg);
  }
  /* What opens under a feature slides open; flow-root keeps its top margin inside the slide. */
  .reveal {
    display: flow-root;
  }
  .details {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--bg);
    border: 1px solid var(--line);
  }
  .drow {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) 64px 190px;
    gap: 10px;
    align-items: baseline;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .dname {
    color: var(--text-2);
    font-weight: 500;
  }
  .drow > :nth-child(3) {
    text-align: right;
  }
  .src {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-4);
  }
  .prog {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-top: 10px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 2px;
    background: var(--st-run);
    transition: width 250ms linear;
  }
</style>
