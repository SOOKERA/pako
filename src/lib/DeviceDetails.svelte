<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // My devices › the picked device: Files (sent between your devices, no Accept, live progress),
  // Links (the links this PC sent it to download, a card per send, and the box to send more:
  // see DeviceLinks), Downloads (its list live: pause / resume / priority / remove / get),
  // Settings (its receiving folder, the control permission, the safety code, its history).
  import { untrack } from "svelte";
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api, inApp, type DirectHistory, type DirectItem, type DirectTransfer } from "./api";
  import { devices, devicesApi, dropOnDevice, onLan, sendToDevice, type Device, type DevicesState } from "./devices.svelte";
  import { ago, hasList, itemsCount, pathLabel, percent } from "./direct";
  import { bytes, shortDate, speed } from "./format";
  import Icon from "./Icon.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import TransferItems from "./TransferItems.svelte";
  import Modal from "./Modal.svelte";
  import SegPick from "./SegPick.svelte";
  import DeviceLinks from "./DeviceLinks.svelte";
  import RemoteRow from "./RemoteRow.svelte";
  import { fadeIn, pop, rowSlide } from "./motion";
  import { around, t } from "./i18n.svelte";

  let { device: x, state: s }: { device: Device; state: DevicesState } = $props();

  type Tab = "files" | "links" | "downloads" | "settings";
  let tab = $state<Tab>("files");

  const status = $derived.by(() => {
    if (x.asksToPair) return t("Wants to pair: confirm above");
    if (!x.paired) return x.online ? t("Connected · not confirmed on it yet") : t("Confirm on the other PC (it asks there)");
    if (x.online) return t("Online · {how}", { how: pathLabel(x.path) || t("connected") });
    if (!s.internet) return t("Not reachable without internet");
    return t("Offline · {seen}", { seen: ago(x.lastSeen) });
  });

  // ---------- Files ----------
  type Row = { kind: "history"; h: DirectHistory } | { kind: "moving"; m: DirectTransfer };
  const rows = $derived<Row[]>([
    ...s.history.filter((h) => h.contact === x.id).map((h) => ({ kind: "history" as const, h })),
    ...s.transfers.filter((m) => m.contact === x.id).map((m) => ({ kind: "moving" as const, m })),
  ]);
  let timeline: HTMLDivElement | undefined = $state();
  let scrolledOnce = false;
  const rowCount = $derived(rows.length);
  $effect(() => {
    void rowCount;
    void tab;
    const glide = untrack(() => scrolledOnce) && !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    scrolledOnce = true;
    queueMicrotask(() => timeline?.scrollTo({ top: timeline.scrollHeight, behavior: glide ? "smooth" : "auto" }));
  });
  const last: Record<string, { at: number; done: number; speed: number }> = {};
  function speedOf(m: DirectTransfer) {
    const now = performance.now();
    const p = last[m.id];
    if (!p) {
      last[m.id] = { at: now, done: m.done, speed: 0 };
      return 0;
    }
    if (now - p.at > 400) {
      const v = ((m.done - p.done) * 1000) / (now - p.at);
      p.speed = p.speed ? p.speed * 0.6 + v * 0.4 : v;
      p.at = now;
      p.done = m.done;
    }
    return p.speed;
  }
  function resultText(h: DirectHistory) {
    if (h.result === "done") return h.outgoing ? t("Delivered") : t("Received");
    if (h.result === "cancelled") return t("Cancelled");
    if (h.result === "declined") return t("Declined");
    return h.message ?? t("Didn't go through");
  }
  function movingText(m: DirectTransfer) {
    if (m.state === "offered") return t("Starting…");
    if (m.state === "preparing") return t("Getting ready…");
    if (m.state === "checking") return t("Checking the files…");
    if (m.state === "waiting") return t("Waiting for {name} to continue · {pct}% done", { name: x.name, pct: percent(m.done, m.size) });
    return t("{pct}% · {done} of {total}", { pct: percent(m.done, m.size), done: bytes(m.done), total: bytes(m.size) });
  }
  /** Cards unfolded (what was in them shown). */
  let unfolded = $state<Record<string, boolean>>({});
  /** One file or folder of a received transfer (a program or script is shown in its folder). */
  async function openPart(id: string, part: number, reveal: boolean) {
    if (await api.directOpen(id, reveal, part)) app.toast(t("Programs and scripts aren't started from Pako. It's shown in its folder: open it there if you trust it."));
  }
  async function openReceived(id: string) {
    if (await api.directOpen(id, false)) app.toast(t("Programs and scripts aren't started from Pako. It's shown in its folder: open it there if you trust it."));
  }
  let sending = $state(false);
  function sent(ok: boolean, message: string) {
    if (ok) tab = "files";
    else app.toast(message, "error");
  }
  async function sendPaths(paths: string[]) {
    if (!paths.length || sending) return;
    sending = true;
    try {
      await sendToDevice(x, paths);
      sent(true, "");
    } catch (e) {
      sent(false, String(e));
    } finally {
      sending = false;
    }
  }
  async function chooseFiles(dir: boolean) {
    if (!inApp) return app.toast(t("Preview mode: nothing is sent."));
    const r = await pick({ multiple: true, directory: dir, title: dir ? t("Folders to send to {name}", { name: x.name }) : t("Files to send to {name}", { name: x.name }) });
    await sendPaths(Array.isArray(r) ? r : r ? [r] : []);
  }
  // Dropped from Explorer: read by the window, then sent (see devices.svelte.ts).
  let dropping = $state(false);
  let depth = 0;
  const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes("Files");
  function dragEnter(e: DragEvent) {
    if (!hasFiles(e)) return;
    depth++;
    dropping = true;
  }
  function dragOver(e: DragEvent) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer!.dropEffect = "copy";
  }
  function dragLeave(e: DragEvent) {
    if (hasFiles(e) && --depth <= 0) {
      depth = 0;
      dropping = false;
    }
  }
  function drop(e: DragEvent) {
    if (!hasFiles(e)) return;
    depth = 0;
    dropping = false;
    dropOnDevice(e, x, sent);
  }

  // ---------- Downloads ----------
  const canControl = $derived(x.paired && x.theirControl);
  const remote = $derived(s.remote[x.id]?.items ?? null);
  // Its list comes only while a tab shows it (Links follows its downloads too).
  $effect(() => {
    const id = x.id;
    const on = (tab === "downloads" || tab === "links") && canControl && x.online;
    if (!on) return;
    devicesApi.watch(id, true).catch(() => {});
    return () => void devicesApi.watch(id, false).catch(() => {});
  });

  // ---------- Settings ----------
  async function changeFolder() {
    if (!inApp) return app.toast(t("Preview mode: no folder picker."));
    const r = await pick({ directory: true, multiple: false, title: t("Where files from {name} are saved", { name: x.name }) });
    if (typeof r === "string") await app.run(() => devicesApi.setFolder(x.id, r)).then(() => devices.refresh());
  }
  async function toggleControl() {
    await app.run(() => devicesApi.setControl(x.id, !x.control));
    devices.refresh();
  }
  const log = $derived(s.log.filter((l) => l.device === x.id).slice(-60).reverse());
  let renaming = $state(false);
  let renameTo = $state("");
  let removing = $state(false);
  let clearing = $state(false);
  function copyId() {
    app.run(() => api.copyText(x.id)).then(() => app.toast(t("{name}'s Pako ID is copied.", { name: x.name })));
  }
  /** "To finish pairing, confirm on <b>{name}</b>…": the code in bold in the middle. */
  const finishLine = $derived(around(t("To finish pairing, confirm on {name}: it asks there, in My devices, with this safety code: {code}", { name: x.name }), "{code}"));
  const tabs = $derived([
    { value: "files", label: t("Files") },
    { value: "links", label: t("Links") },
    { value: "downloads", label: t("Downloads") },
    { value: "settings", label: t("Settings") },
  ]);
</script>

<!-- A card's chevron, and what was in it unfolded (as in Direct). -->
{#snippet chev(key: string, name: string)}
  <button
    class="chev"
    class:open={unfolded[key]}
    aria-expanded={!!unfolded[key]}
    aria-controls="items-{key}"
    aria-label={unfolded[key] ? t("Hide what was in {name}", { name }) : t("Show what was in {name}", { name })}
    title={unfolded[key] ? t("Hide the files") : t("Show the files")}
    onclick={() => (unfolded[key] = !unfolded[key])}><Icon name="chevron" size={12} stroke={2.4} /></button
  >
{/snippet}

{#snippet fold(key: string, items: DirectItem[] | undefined, more: number | undefined, openId: string | null)}
  {#if unfolded[key] && items?.length}
    <div class="mlist" transition:rowSlide={{ duration: 220 }}>
      <TransferItems {items} more={more ?? 0} id="items-{key}" open={openId ? (i, reveal) => app.run(() => openPart(openId, i, reveal)) : null} />
    </div>
  {/if}
{/snippet}

<section class="pane card" role="presentation" data-tour="devices-pane" ondragenter={dragEnter} ondragover={dragOver} ondragleave={dragLeave} ondrop={drop}>
  {#if dropping}
    <div class="dropzone" aria-hidden="true" transition:fadeIn={{ duration: 120 }}>
      <div class="dropcard"><Icon name="send" size={20} />{t("Drop to send to {name}", { name: x.name })}</div>
    </div>
  {/if}
  <header class="head">
    <span class="avatar"><Icon name="devices" size={17} /><span class="dot" class:online={x.online} class:local={onLan(x)} class:wait={!x.paired}></span></span>
    <div class="htext">
      <div class="hname">{x.name}</div>
      <div class="hstat">{status}</div>
    </div>
    <div class="spacer"></div>
    {#if !x.online && x.device}
      <button class="btn small" onclick={() => app.run(() => api.directConnect(x.id))}><Icon name="retry" size={12} />{t("Connect")}</button>
    {/if}
    <div class="tabwrap" data-tour="devices-tabs">
    <!-- Narrow (a small window, the details beside it): a menu instead, so the name keeps room. -->
    <select class="input small tabmenu" aria-label={t("Show")} value={tab} onchange={(e) => (tab = e.currentTarget.value as Tab)}>
      {#each tabs as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
    </select>
    <div class="tabs">
      <SegPick
        small
        label={t("Show")}
        options={tabs}
        value={tab}
        onpick={(v) => (tab = v as Tab)}
      />
    </div>
    </div>
  </header>

  {#if x.device && !x.paired}
    <div class="note" transition:rowSlide={{ duration: 220 }}>
      <Icon name="alert" size={14} />
      <span>{finishLine[0]}<b class="mono">{x.code}</b>{finishLine[1]}{#if x.online}. {t("If it doesn't ask, update Pako on {name}: older Pakos don't know My devices.", { name: x.name })}{/if}</span>
    </div>
  {/if}

  {#key tab}
    <div class="tabbody" in:fadeIn={{ duration: 160 }}>
      {#if tab === "files"}
        <div class="timeline" bind:this={timeline}>
          {#each rows as r (r.kind === "history" ? `h:${r.h.outgoing ? "o" : "i"}:${r.h.id}` : `t:${r.m.outgoing ? "o" : "i"}:${r.m.id}`)}
            {#if r.kind === "history"}
              {@const h = r.h}
              <div class="msg" class:out={h.outgoing} class:bad={h.result !== "done"} in:pop={{ duration: 240, from: 0.98, y: 8, origin: h.outgoing ? "right bottom" : "left bottom" }}>
                <div class="bubble">
                  <div class="mtop"><FileIcon name={h.name} kind={h.files > 1 ? "folder" : undefined} size={18} /><MiddleText text={h.name} class="mname" />{#if hasList(h.items, h.more)}{@render chev(`i:${h.outgoing ? "o" : "i"}:${h.id}`, h.name)}{/if}</div>
                  <div class="mmeta">{itemsCount(h.files, h.items, h.more)} · {bytes(h.size)}{#if h.relayTo}{` · ${t("for {name}", { name: h.relayTo })}`}{/if}</div>
                  <div class="mfoot">
                    <span class="res" class:ok={h.result === "done"}>{resultText(h)}</span>
                    <span class="dim">{shortDate(h.date)}{#if h.path}{" · " + pathLabel(h.path)}{/if}</span>
                    {#if !h.outgoing && h.result === "done" && h.savedTo}
                      <span class="acts">
                        <button class="link" onclick={() => app.run(() => openReceived(h.id))}>{t("Open")}</button>
                        <button class="link" onclick={() => app.run(() => api.directOpen(h.id, true))}>{t("Folder")}</button>
                      </span>
                    {/if}
                  </div>
                  {@render fold(`i:${h.outgoing ? "o" : "i"}:${h.id}`, h.items, h.more, !h.outgoing && h.result === "done" && h.savedTo ? h.id : null)}
                </div>
              </div>
            {:else}
              {@const m = r.m}
              <div class="msg live" class:out={m.outgoing} in:pop={{ duration: 240, from: 0.98, y: 8, origin: m.outgoing ? "right bottom" : "left bottom" }}>
                <div class="bubble">
                  <div class="mtop"><FileIcon name={m.name} kind={m.files > 1 ? "folder" : undefined} size={18} /><MiddleText text={m.name} class="mname" />{#if hasList(m.items, m.more)}{@render chev(`i:${m.outgoing ? "o" : "i"}:${m.id}`, m.name)}{/if}</div>
                  <div class="mmeta">{itemsCount(m.files, m.items, m.more)} · {bytes(m.size)}{#if m.relayTo}{` · ${t("for {name}", { name: m.relayTo })}`}{/if}</div>
                  <div class="bar" class:waiting={m.state === "waiting"}><i style:transform="scaleX({Math.min(1, m.done / Math.max(1, m.size))})"></i></div>
                  <div class="mfoot">
                    <span class="dim">{movingText(m)}{#if m.state === "sending" || m.state === "receiving"}{" · " + speed(speedOf(m))}{/if}{#if m.path}{" · " + pathLabel(m.path)}{/if}</span>
                    <button class="link end" onclick={() => app.run(() => api.directCancel(m.id))}>{t("Cancel")}</button>
                  </div>
                  {@render fold(`i:${m.outgoing ? "o" : "i"}:${m.id}`, m.items, m.more, null)}
                </div>
              </div>
            {/if}
          {:else}
            <div class="nothing">
              <Icon name="send" size={20} />
              <p>{t("Nothing sent between this PC and {name} yet.", { name: x.name })}</p>
              <p class="dim">{t("Drop files or folders here: they go straight to {name}, no Accept needed.", { name: x.name })}</p>
            </div>
          {/each}
        </div>
        <footer class="compose" data-tour="devices-send">
          <span class="dim grow">
            {#if !x.paired}{t("Files go without asking once both PCs confirmed.")}{:else if x.online}{t("Drop files or folders anywhere here to send them to {name}.", { name: x.name })}{:else}{t("{name} is offline: files go only while both PCs are on.", { name: x.name })}{/if}
          </span>
          <button class="btn small" disabled={!x.online || sending} onclick={() => chooseFiles(false)}><Icon name="file" size={12} />{t("Choose files")}</button>
          <button class="btn small" disabled={!x.online || sending} onclick={() => chooseFiles(true)}><Icon name="folder" size={12} />{t("Choose folder")}</button>
        </footer>
      {:else if tab === "links"}
        {#if !canControl}
          <div class="blank">
            <div class="blank-icon"><Icon name="link" size={22} /></div>
            <p><b>{t("{name} doesn't let this PC start its downloads.", { name: x.name })}</b></p>
            <p class="dim">{t('On {name}: My devices › this PC › Settings, turn on "… can control my Pako".', { name: x.name })}</p>
          </div>
        {:else}
          <DeviceLinks device={x} state={s} ongot={() => (tab = "files")} />
        {/if}
      {:else if tab === "downloads"}
        {#if !canControl}
          <div class="blank">
            <div class="blank-icon"><Icon name="download" size={22} /></div>
            <p><b>{t("{name} doesn't let this PC see or start its downloads.", { name: x.name })}</b></p>
            <p class="dim">{t('On {name}: My devices › this PC › Settings, turn on "… can control my Pako".', { name: x.name })}</p>
          </div>
        {:else}
          <div class="rlist">
            {#if !x.online}
              <p class="dim pad">{t("{name} is offline. Its downloads show here while it's on.", { name: x.name })}</p>
            {:else if !remote}
              <p class="dim pad">{t("Asking {name} for its downloads…", { name: x.name })}</p>
            {:else}
              {#each remote as it (it.id)}
                <RemoteRow device={x} item={it} ongot={() => (tab = "files")} />
              {:else}
                <p class="dim pad">{t("No downloads on {name}.", { name: x.name })}</p>
              {/each}
            {/if}
          </div>
        {/if}
      {:else}
        <div class="settings">
          <div class="srow">
            <div class="stext">
              <div class="slabel">{t("Files from {name} are saved in", { name: x.name })}</div>
              <div class="path mono" title={x.folder}>{x.folder}</div>
              <div class="shint">{t("Files from this device are saved here, and only here. {name} can't pick another place on this PC.", { name: x.name })}</div>
            </div>
            <button class="btn small" onclick={changeFolder}><Icon name="folder" size={12} />{t("Change…")}</button>
          </div>
          {#if x.device}
            <div class="srow">
              <div class="stext">
                <div class="slabel">{t("{name} can control my Pako", { name: x.name })}</div>
                <div class="shint">{t("It can see this PC's downloads, add links to them (into this PC's download folder, or a folder it names inside it), pause or resume them, change their priority, remove them, and get finished ones. It can also see your Direct contacts and your history with them, and send files to them through this PC. Nothing else: it can't open or run anything here, or look in other folders. Turned off, its next order is refused.")}</div>
              </div>
              <button class="switch" class:on={x.control} role="switch" aria-checked={x.control} aria-label={t("{name} can control my Pako", { name: x.name })} onclick={toggleControl}></button>
            </div>
          {/if}
          <div class="srow">
            <div class="stext">
              <div class="slabel">{t("Safety code")}</div>
              <div class="mono code">{x.code}</div>
              <div class="shint">{t("The same on both PCs.")} {t("Pako ID")} <span class="mono">{x.id.slice(0, 8)}…</span> <button class="link" onclick={copyId}>{t("Copy")}</button></div>
            </div>
          </div>
          <div class="loghead">
            <span class="slabel">{t("History")}</span>
            <div class="spacer"></div>
            {#if log.length}<button class="link" onclick={() => (clearing = true)}>{t("Clear (history)")}</button>{/if}
          </div>
          <div class="log">
            {#each log as l, i (l.date + ":" + i)}
              <div class="lrow" class:bad={!l.ok}>
                <span class="who">{l.byMe ? t("From this PC") : t("From {name}", { name: x.name })}</span>
                <span class="what">{l.text}</span>
                <span class="when dim">{shortDate(l.date)}</span>
              </div>
            {:else}
              <p class="dim">{t("Orders between this PC and {name} show here.", { name: x.name })}</p>
            {/each}
          </div>
          <div class="danger">
            <button class="btn small" onclick={() => ((renaming = true), (renameTo = x.name))}><Icon name="pencil" size={12} />{t("Rename")}</button>
            <button class="btn small bad" onclick={() => (removing = true)}><Icon name="trash" size={12} />{t("Remove this device")}</button>
          </div>
        </div>
      {/if}
    </div>
  {/key}
</section>

{#if renaming}
  <Modal title={t("Rename {name}", { name: x.name })} width={420} onclose={() => (renaming = false)}>
    <input class="input" bind:value={renameTo} maxlength="60" aria-label={t("New name")} onkeydown={(e) => e.key === "Enter" && (renaming = false, app.run(() => devicesApi.rename(x.id, renameTo)))} />
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (renaming = false)}>{t("Cancel")}</button>
      <button class="btn primary" disabled={!renameTo.trim()} onclick={() => ((renaming = false), app.run(() => devicesApi.rename(x.id, renameTo)))}>{t("Rename")}</button>
    {/snippet}
  </Modal>
{/if}
{#if removing}
  <Modal title={t("Remove {name}?", { name: x.name })} width={440} onclose={() => (removing = false)}>
    <p class="msgtext">{t("{name} stops being one of your devices: it can't send files without asking or control this Pako any more, and the connection closes. Files already received stay. To pair again later, add it again on both PCs.", { name: x.name })}</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (removing = false)}>{t("Cancel")}</button>
      <button class="btn danger" onclick={() => ((removing = false), app.run(() => api.directRemoveContact(x.id)).then(() => devices.refresh()))}>{t("Remove")}</button>
    {/snippet}
  </Modal>
{/if}
{#if clearing}
  <Modal title={t("Clear the history?")} width={420} onclose={() => (clearing = false)}>
    <p class="msgtext">{t("The list of orders between this PC and {name} is deleted from this PC. Files and downloads don't change.", { name: x.name })}</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (clearing = false)}>{t("Cancel")}</button>
      <button class="btn danger" onclick={() => ((clearing = false), app.run(() => devicesApi.clearLog(x.id)))}>{t("Clear (history)")}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .card {
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
  }
  .pane {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    container: devpane / inline-size;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--line);
  }
  .avatar {
    position: relative;
    flex: none;
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: var(--avatar-bg);
    box-shadow: inset 0 0 0 1px var(--avatar-ring);
    color: var(--text-2);
  }
  .dot {
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: var(--st-pause);
    border: 2px solid var(--raise);
    transition: background var(--mid) ease;
  }
  .dot.online {
    background: var(--st-done);
  }
  .dot.local {
    background: var(--st-run);
  }
  .dot.wait {
    background: var(--st-retry);
  }
  .htext {
    flex: 1 1 auto;
    min-width: 72px;
  }
  .hname {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hstat {
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spacer,
  .grow {
    flex: 1;
  }
  .tabwrap,
  .tabs {
    flex: none;
  }
  .tabmenu {
    display: none;
    flex: none;
    width: auto;
  }
  /* Narrow: the four tabs fold into a menu (the name stays readable), and the send bar's
     words take their own line above the buttons. */
  @container devpane (max-width: 560px) {
    .tabs {
      display: none;
    }
    .tabmenu {
      display: block;
    }
    .compose {
      flex-wrap: wrap;
    }
    .compose > .grow {
      flex-basis: 100%;
    }
  }
  .dim {
    color: var(--text-3);
  }
  .note {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 12px 0;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--panel);
    border: 1px solid var(--line-soft);
    font-size: 12.5px;
    color: var(--text-2);
  }
  .note b {
    color: var(--text);
    letter-spacing: 0.04em;
  }
  .tabbody {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  /* ---------- files ---------- */
  .timeline {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
  }
  .nothing {
    margin: auto;
    display: grid;
    justify-items: center;
    gap: 6px;
    text-align: center;
    color: var(--text-3);
    font-size: 12.5px;
  }
  .nothing p {
    margin: 0;
    max-width: 360px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .nothing p.dim {
    color: var(--text-3);
  }
  .msg {
    display: flex;
  }
  .msg.out {
    justify-content: flex-end;
  }
  .bubble {
    width: min(420px, 82%);
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    background: var(--bubble);
    border: 1px solid var(--bubble-line);
    box-shadow: var(--bubble-shadow);
  }
  .msg.out .bubble {
    background: var(--bubble-out);
  }
  .msg.bad .bubble {
    opacity: 0.8;
  }
  .mtop {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .bubble {
    --ti-hover: var(--active);
    --ti-meta: var(--bubble-meta);
  }
  .chev {
    flex: none;
    width: 22px;
    height: 22px;
    margin: -2px -4px -2px 0;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--bubble-meta);
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .chev:hover {
    background: var(--active);
    color: var(--text);
  }
  .chev :global(svg) {
    transition: transform var(--mid) var(--ease);
  }
  .chev.open :global(svg) {
    transform: rotate(180deg);
  }
  /* What was in it, unfolded under the card's foot. */
  .mlist {
    margin: 8px -4px 0 18px;
    padding-top: 6px;
    border-top: 1px solid var(--bubble-line);
  }
  .mlist :global(.tis) {
    max-height: 240px;
    overflow-y: auto;
  }
  .mtop :global(.mname) {
    font-weight: 500;
    min-width: 0;
  }
  .mmeta {
    margin: 4px 0 0 26px;
    font-size: 12px;
    color: var(--bubble-meta);
  }
  .mfoot .dim {
    color: var(--bubble-meta);
  }
  .mfoot {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 10px;
    margin: 6px 0 0 26px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .res {
    color: var(--st-fail-text);
    font-weight: 500;
  }
  .res.ok {
    color: var(--delivered-text);
  }
  .acts {
    display: flex;
    gap: 10px;
    margin-left: auto;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--focus-text);
    font: inherit;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .link.end {
    margin-left: auto;
  }
  .bar {
    height: 4px;
    margin: 8px 0 0 26px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--st-run);
    transform-origin: left;
    transition:
      transform var(--mid) var(--ease-out),
      background-color var(--mid) var(--ease-out);
  }
  /* Cut short, waiting for the other PC: the bar rests in the paused colour. */
  .bar.waiting i {
    background: var(--st-pause);
  }
  .compose {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--line);
    background: var(--panel);
    font-size: 12.5px;
  }
  .btn.small {
    height: 28px;
    font-size: 12px;
  }
  .btn.bad {
    color: var(--st-fail-text);
  }
  /* ---------- downloads ---------- */
  .blank {
    margin: auto;
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 24px;
    text-align: center;
  }
  .blank p {
    margin: 0;
    max-width: 420px;
    line-height: 1.5;
  }
  .blank-icon {
    width: 46px;
    height: 46px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--panel);
    color: var(--text-3);
    margin-bottom: 4px;
  }
  .rlist {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px;
  }
  .pad {
    padding: 10px 8px;
    margin: 0;
    font-size: 12.5px;
  }
  /* ---------- settings ---------- */
  .settings {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 14px 14px;
  }
  .srow {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 0;
    border-bottom: 1px solid var(--line-soft);
  }
  .stext {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .slabel {
    font-weight: 500;
  }
  .shint {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-3);
  }
  .path {
    font-size: 12px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .code {
    font-size: 12.5px;
    letter-spacing: 0.04em;
  }
  .loghead {
    display: flex;
    align-items: center;
    margin-top: 10px;
    font-size: 12.5px;
  }
  .log {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12.5px;
  }
  .log p {
    margin: 4px 0;
  }
  .lrow {
    display: grid;
    grid-template-columns: 112px minmax(0, 1fr) auto;
    gap: 10px;
    padding: 5px 0;
    align-items: baseline;
  }
  .who {
    color: var(--text-3);
    font-size: 12px;
  }
  .what {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lrow.bad .what {
    color: var(--st-fail-text);
  }
  .when {
    font-size: 12px;
  }
  .danger {
    display: flex;
    gap: 8px;
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--line-soft);
  }
  .msgtext {
    margin: 0;
    line-height: 1.55;
    color: var(--text-2);
  }
  .dropzone {
    position: absolute;
    inset: 0;
    z-index: 10;
    display: grid;
    place-items: center;
    background: var(--drop-scrim);
    border: 2px dashed var(--accent);
    border-radius: var(--radius-lg);
    pointer-events: none;
  }
  .dropcard {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border-radius: var(--radius-lg);
    background: var(--raise);
    border: 1px solid var(--line-strong);
    font-weight: 500;
  }
  .dropcard :global(svg) {
    color: var(--accent);
  }
</style>
