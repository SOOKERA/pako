<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The small list that opens from the tray icon: what's downloading, at a glance.
  import { onMount } from "svelte";
  import { app } from "$lib/state.svelte";
  import { api, inApp, isActive, listen, type DirectHistory, type DirectTransfer, type Download } from "$lib/api";
  import { devices } from "$lib/devices.svelte";
  import { initial, percent } from "$lib/direct";
  import { bytes, sentLook, speed, statusLook } from "$lib/format";
  import Icon from "$lib/Icon.svelte";
  import Logo from "$lib/Logo.svelte";
  import MiddleText from "$lib/MiddleText.svelte";
  import WinButton from "$lib/WinButton.svelte";
  import Toasts from "$lib/Toasts.svelte";
  import { fadeIn, flipIf, flipOpts, pop, rowSlide } from "$lib/motion";
  import { confirmClick, CONFIRM_FOR, type Armed } from "$lib/confirm";

  onMount(() => app.start());
  // Transfers with your own PCs come from My devices (Direct's state leaves them out). Asked
  // only while this window shows (it stays loaded when hidden), and again as soon as it shows.
  onMount(() => {
    const shows = () => document.visibilityState !== "hidden";
    devices.start(shows);
    const again = () => shows() && devices.refresh();
    document.addEventListener("visibilitychange", again);
    return () => document.removeEventListener("visibilitychange", again);
  });

  // Each time the tray icon opens the list, it slides in from the taskbar's side.
  // The window is ready in advance, so this replays the animation on every opening.
  let opening = $state(0);
  let from = $state<"bottom" | "top">("bottom");
  onMount(() => {
    if (!inApp) return;
    const un = listen<{ from: "bottom" | "top" }>("mini-show", (e) => {
      from = e.payload.from;
      devices.refresh();
      // Speeds start fresh: a reading from before the window was closed isn't blended in.
      for (const k of Object.keys(seen)) delete seen[k];
      opening++;
      // A "Remove" left waiting when the window closed asks again.
      clearTimeout(confirmTimer);
      armed = null;
    });
    return () => {
      un.then((f) => f());
    };
  });

  /** Only what matters at a glance: what's running, waiting or paused, plus what finished
   *  today (Open, Folder). Older finished ones and older failures stay in the full window. */
  const SHOWN = 8;
  /** A time from the list (seconds or milliseconds) fell on today's date. */
  function isToday(t: number | null | undefined) {
    if (!t) return false;
    return new Date(t > 1e12 ? t : t * 1000).toDateString() === new Date().toDateString();
  }
  const STOPPED = new Set<Download["status"]>(["failed", "quotaExceeded", "needsAccount"]);
  /** Re-checked every minute, so yesterday's ones leave the list after midnight. */
  let clock = $state(0);
  onMount(() => {
    const t = setInterval(() => clock++, 60_000);
    return () => clearInterval(t);
  });
  const unfinished = $derived.by(() => {
    void clock;
    const rank = (d: Download) => (isActive(d.status) ? 0 : d.status === "queued" ? 1 : d.status === "paused" ? 2 : 3);
    return app.downloads
      // Stopped ones (failed, Google's daily limit, needs the browser) only from today.
      .filter((d) => d.status !== "completed" && (!STOPPED.has(d.status) || isToday(d.date || d.addedAt)))
      .sort((a, b) => rank(a) - rank(b) || a.addedAt - b.addedAt);
  });
  const list = $derived.by(() => {
    void clock;
    const open = unfinished.slice(0, SHOWN);
    const done = app.downloads
      .filter((d) => d.status === "completed" && isToday(d.finishedAt))
      .sort((a, b) => (b.finishedAt ?? 0) - (a.finishedAt ?? 0))
      .slice(0, Math.max(3, SHOWN - open.length));
    return [...open, ...done];
  });
  const openShown = $derived(Math.min(unfinished.length, SHOWN));
  const more = $derived(unfinished.length - openShown);
  /** Nothing left to do, and something finished: "All done". */
  const allDone = $derived(!unfinished.some((d) => d.status !== "failed") && list.some((d) => d.status === "completed"));

  /** Pinned: stays open and on top, where you move it. */
  const pinned = $derived(!!app.settings?.miniPinned);
  async function togglePin() {
    const on = !pinned;
    if (app.settings) app.settings.miniPinned = on;
    await app.run(() => api.miniPin(on));
  }

  /** The switch at the top: what's downloading, what's being uploaded, or what moves in Direct
   *  (with people and with your own PCs). Direct's half shows once Direct runs on this PC. */
  let tab = $state<"down" | "up" | "direct">("down");
  const TABS = ["down", "up", "direct"] as const;
  // A send still on the list above (its files partly up already) isn't shown twice.
  const recentSent = $derived(app.sent.filter((s) => !app.sends.some((j) => j.id === s.id)).slice(0, 6 - Math.min(6, app.sends.length)));
  function showUploads() {
    tab = "up";
    // Fresh download counts (Pako asks Gofile at most every few minutes).
    app.checkSent(false);
  }
  function upPct(j: { sent: number; total: number | null; size: number }) {
    const of = j.total ?? j.size;
    return of ? Math.min(100, (j.sent / of) * 100) : 0;
  }

  /** Remove asks once: the ✕ turns into "Remove" for a few seconds, a second click removes it
   *  (what was downloaded so far is deleted, as in the full window). One button that changes,
   *  so the keyboard focus stays on it; a double-click on ✕ doesn't remove. */
  let armed = $state<Armed>(null);
  let confirmTimer: ReturnType<typeof setTimeout> | undefined;
  async function removeClick(d: Download) {
    const now = performance.now();
    const what = confirmClick(armed, d.id, now);
    if (what === "arm") {
      armed = { id: d.id, at: now };
      clearTimeout(confirmTimer);
      confirmTimer = setTimeout(() => (armed = null), CONFIRM_FOR);
    } else if (what === "confirm") {
      clearTimeout(confirmTimer);
      armed = null;
      await app.run(() => api.remove([d.id], false));
      await app.refresh();
    }
  }

  // ---------- Direct ----------
  const directOn = $derived(!!app.direct?.ready);
  // Direct stopped (or not started yet) while its half was picked: back to Downloads.
  $effect(() => {
    if (tab === "direct" && !directOn) tab = "down";
  });
  /** Who each transfer is with: your contacts, and your own PCs (My devices). */
  type Who = { name: string; device: boolean; page: "direct" | "devices" };
  const people = $derived.by(() => {
    const m = new Map<string, Who>();
    for (const c of app.direct?.contacts ?? []) m.set(c.id, { name: c.name, device: false, page: "direct" });
    for (const d of devices.data?.devices ?? []) m.set(d.id, { name: d.name, device: true, page: "devices" });
    return m;
  });
  const whoOf = (id: string): Who => people.get(id) ?? { name: "", device: false, page: "direct" };
  const FLOWING = new Set(["sending", "receiving"]);
  /** Direct's and My devices' lists are asked separately: a contact paired or unpaired in
   *  between can put the same transfer in both for a moment. Each shows once. */
  function dedupe<T extends { id: string }>(list: T[]): T[] {
    const ids = new Set<string>();
    return list.filter((x) => !ids.has(x.id) && (ids.add(x.id), true));
  }
  /** What moves now (or waits to), moving ones first. */
  const dMoving = $derived(
    dedupe([...(app.direct?.transfers ?? []), ...(devices.data?.transfers ?? [])]).sort((a, b) => Number(FLOWING.has(b.state)) - Number(FLOWING.has(a.state))),
  );
  const dFlowing = $derived(dMoving.filter((t) => FLOWING.has(t.state)).length);
  /** Today's finished ones, newest first: enough to fill the list, at least 3. */
  const dDone = $derived.by(() => {
    void clock;
    return dedupe([...(app.direct?.history ?? []), ...(devices.data?.history ?? [])])
      .filter((h) => isToday(h.date))
      .sort((a, b) => b.date - a.date)
      .slice(0, Math.max(3, SHOWN - dMoving.length));
  });
  /** Live speed per transfer, from how much moved since the last update (as in Direct). */
  const seen: Record<string, { at: number; done: number; speed: number }> = {};
  function rate(t: DirectTransfer) {
    const now = performance.now();
    const prev = seen[t.id];
    if (!prev) {
      seen[t.id] = { at: now, done: t.done, speed: 0 };
      return 0;
    }
    if (now - prev.at > 400) {
      const s = ((t.done - prev.done) * 1000) / (now - prev.at);
      prev.speed = Math.max(0, prev.speed ? prev.speed * 0.6 + s * 0.4 : s);
      prev.at = now;
      prev.done = t.done;
    }
    return prev.speed;
  }
  const dRate = () => dMoving.filter((t) => FLOWING.has(t.state)).reduce((a, t) => a + rate(t), 0);
  /** A transfer's line: what it's doing and with whom, and its colours. */
  function dLook(t: DirectTransfer) {
    const name = whoOf(t.contact).name || (t.outgoing ? "them" : "a contact");
    if (t.state === "sending" && t.relayTo) return { label: `Sending to ${t.relayTo} through ${name}`, color: "var(--st-run)", text: "var(--text)" };
    if (t.state === "sending") return { label: `Sending to ${name}`, color: "var(--st-run)", text: "var(--text)" };
    if (t.state === "receiving") return { label: `Receiving from ${name}`, color: "var(--st-run)", text: "var(--text)" };
    if (t.state === "checking") return { label: "Checking the files", color: "var(--st-check)", text: "var(--st-check-text)" };
    if (t.state === "waiting") return { label: `Waiting for ${name} to continue`, color: "var(--st-retry)", text: "var(--st-retry-text)" };
    if (t.state === "offered") return { label: `Waiting for ${name} to accept`, color: "var(--st-wait)", text: "var(--st-wait-text)" };
    return { label: t.outgoing ? `Getting ready for ${name}` : `Getting ready, from ${name}`, color: "var(--st-wait)", text: "var(--st-wait-text)" };
  }
  /** How a finished one ended, said as in Direct. */
  function dEnd(h: DirectHistory) {
    if (h.result === "done" && h.outgoing && h.relayTo) return { label: `Passed on to ${whoOf(h.contact).name || "your PC"}`, text: "var(--st-done-text)" };
    if (h.result === "done") return { label: h.outgoing ? "Delivered" : "Received", text: "var(--st-done-text)" };
    if (h.result === "declined") return { label: "Declined", text: "var(--text-3)" };
    if (h.result === "cancelled") return { label: "Cancelled", text: "var(--text-3)" };
    return { label: "Didn't go through", text: "var(--st-fail-text)" };
  }
  /** Opens a received file; a program or script is shown in its folder instead (as in Direct). */
  async function openReceived(h: DirectHistory, reveal: boolean) {
    try {
      const shownInstead = await api.directOpen(h.id, reveal);
      if (shownInstead && !reveal) app.toast("Programs and scripts aren't started from Pako. It's shown in its folder: open it there if you trust it.");
    } catch (e) {
      app.toast(String(e), "error");
    }
  }
  /** Cancel asks once, like Remove: ✕ turns into "Cancel" for a few seconds. */
  async function cancelClick(t: DirectTransfer) {
    const now = performance.now();
    const what = confirmClick(armed, t.id, now);
    if (what === "arm") {
      armed = { id: t.id, at: now };
      clearTimeout(confirmTimer);
      confirmTimer = setTimeout(() => (armed = null), CONFIRM_FOR);
    } else if (what === "confirm") {
      clearTimeout(confirmTimer);
      armed = null;
      await app.run(() => api.directCancel(t.id));
    }
  }
  /** Open Pako: on the page of the half shown. The Direct half opens the page of its top row
   *  (moving ones first, then the newest finished): My devices when that's one of your own PCs,
   *  Direct otherwise or when the half is empty. */
  function openPako() {
    if (tab === "direct") {
      const top = dMoving[0]?.contact ?? dDone[0]?.contact;
      const page = top ? whoOf(top).page : "direct";
      app.run(() => api.popupOpen({ page }, null));
    } else app.run(() => api.showMain());
  }

  /** A video's quality as its row says it ("1080p", "Audio only · MP3"); nothing for a file. */
  function videoAs(d: Download): string {
    const src = d.source;
    if (src?.kind !== "video") return "";
    if (src.quality === 0) return src.audio ? `Audio only · ${src.audio.toUpperCase()}` : "Audio only";
    const q = src.quality ?? app.settings?.videoQuality;
    return q ? `${q}p` : "";
  }

  function pct(d: Download) {
    return d.size ? Math.min(100, (d.downloaded / d.size) * 100) : 0;
  }
</script>

<div class="frame">
{#key opening}
<div class="mini" class:from-top={from === "top"}>
  <header data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region><Logo size={14} /></span>
    {#if tab === "down" && allDone && app.totalSpeed === 0}
      <span class="spd done" data-tauri-drag-region in:fadeIn={{ duration: 160 }}><Icon name="check" size={12} stroke={2.4} />All done</span>
    {:else if tab === "down"}
      <span class="spd mono" class:on={app.totalSpeed > 0} data-tauri-drag-region in:fadeIn={{ duration: 160 }}><Icon name="download" size={12} stroke={2.2} />{speed(app.totalSpeed)}</span>
    {:else if tab === "up"}
      <span class="spd mono" class:on={app.uploadSpeed > 0} data-tauri-drag-region in:fadeIn={{ duration: 160 }}><Icon name="upload" size={12} stroke={2.2} />{speed(app.uploadSpeed)}</span>
    {:else}
      {@const r = dRate()}
      <span class="spd mono" class:on={r > 0} data-tauri-drag-region in:fadeIn={{ duration: 160 }}><Icon name="send" size={12} stroke={2.2} />{speed(r)}</span>
    {/if}
    <span class="grow" data-tauri-drag-region></span>
    <button class="icon-btn gear" title="Settings" aria-label="Settings" onclick={() => app.run(() => api.openSettings())}><Icon name="settings" size={13} /></button>
    <span class="sep"></span>
    <WinButton kind="pin" small on={pinned} label="Always on top" title={pinned ? "Always on top: on" : "Always on top"} onclick={togglePin} />
    <WinButton kind="minimize" small label="Minimize" onclick={() => app.run(() => api.miniHide())} />
    <WinButton kind="close" small label="Close" onclick={() => app.run(() => api.miniHide())} />
  </header>

  <div class="switch2" class:three={directOn} role="tablist" aria-label={directOn ? "Show downloads, uploads or Direct" : "Show downloads or uploads"}>
    <span class="thumb" style:transform="translateX(calc({TABS.indexOf(tab)} * (100% + 2px)))" aria-hidden="true"></span>
    <button role="tab" aria-selected={tab === "down"} class:on={tab === "down"} onclick={() => (tab = "down")}>
      Downloads{#if app.runningCount}<span class="cnt" transition:pop={{ duration: 180, from: 0.92, y: 0 }}>{app.runningCount}</span>{/if}
    </button>
    <button role="tab" aria-selected={tab === "up"} class:on={tab === "up"} onclick={showUploads}>
      Uploads{#if app.uploadingCount}<span class="cnt" transition:pop={{ duration: 180, from: 0.92, y: 0 }}>{app.uploadingCount}</span>{/if}
    </button>
    <!-- Always there, folded to nothing until Direct runs: it opens out (and the highlight
         narrows) smoothly instead of jumping. -->
    <button
      role="tab"
      class="third"
      aria-selected={tab === "direct"}
      aria-hidden={!directOn}
      tabindex={directOn ? 0 : -1}
      inert={!directOn}
      class:on={tab === "direct"}
      onclick={() => (tab = "direct")}
    >
      Direct{#if dFlowing}<span class="cnt" transition:pop={{ duration: 180, from: 0.92, y: 0 }}>{dFlowing}</span>{/if}
    </button>
  </div>

  {#snippet who(w: Who)}
    {#if w.device}
      <span class="who dev" title={w.name || "One of your PCs"}><Icon name="devices" size={11} stroke={2.2} /></span>
    {:else}
      <span class="who" title={w.name}>{initial(w.name || "?")}</span>
    {/if}
  {/snippet}

  {#if tab === "up"}
  <div class="list" in:fadeIn={{ duration: 160 }}>
    {#if !app.sends.length && !recentSent.length}
      <div class="empty" in:fadeIn={{ duration: 160 }}>
        <Icon name="upload" size={20} stroke={2} />
        <p>Nothing uploaded yet.</p>
      </div>
    {:else}
      {#each app.sends as j (j.id)}
        <div class="row" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
        <div class="item">
          <div class="top"><MiddleText text={j.name} class="nm" /></div>
          {#if j.status === "uploading"}
            <div class="bar"><div class="fill" style="width: {upPct(j)}%; background: var(--st-run)"></div></div>
            <div class="sub">
              <span style="color: var(--st-run-text)">Uploading</span>
              <span class="mono">{j.speed ? speed(j.speed) : `${Math.floor(upPct(j))}%`}</span>
            </div>
          {:else}
            <div class="sub"><span style="color: var(--st-fail-text)">Not uploaded</span><span class="mono">{bytes(j.size)}</span></div>
          {/if}
        </div>
        </div>
      {/each}
      {#each recentSent as s (s.id)}
        {@const look = sentLook(s)}
        <div class="row" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
        <div class="item">
          <div class="top">
            <MiddleText text={s.name} class="nm" />
            <button class="icon-btn" aria-label="Copy the link of {s.name}" title="Copy link" onclick={() => app.copyLink(s.link)}><Icon name="copy" size={12} /></button>
          </div>
          <div class="sub">
            {#if look}
              <span class="badge" style:color={look.text}><span class="dot" style:background={look.dot}></span>{look.label}</span>
            {:else if s.incomplete}
              <span style="color: var(--st-wait-text)">Not all uploaded</span>
            {:else}
              <span style="color: var(--st-done-text)">Uploaded</span>
            {/if}
            <span class="mono">{bytes(s.size)}</span>
          </div>
        </div>
        </div>
      {/each}
    {/if}
  </div>
  {:else if tab === "direct"}
  <div class="list" in:fadeIn={{ duration: 160 }}>
    {#if !dMoving.length && !dDone.length}
      <div class="empty" in:fadeIn={{ duration: 160 }}>
        <Icon name="send" size={20} stroke={2} />
        <p>Nothing sent or received today.</p>
      </div>
    {:else}
      {#each dMoving as t (t.id)}
        {@const w = whoOf(t.contact)}
        {@const look = dLook(t)}
        {@const r = FLOWING.has(t.state) ? rate(t) : 0}
        <div class="row" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
        <div class="item">
          <div class="top">
            {@render who(w)}
            <MiddleText text={t.name} class="nm" />
            <button
              class={armed?.id === t.id ? "act danger" : "icon-btn"}
              aria-label={armed?.id === t.id ? `Cancel ${t.name} now` : `Cancel ${t.name}`}
              title={armed?.id === t.id ? "Stops this transfer" : "Cancel"}
              onclick={() => cancelClick(t)}
            >
              {#if armed?.id === t.id}<span in:fadeIn={{ duration: 120 }}>Cancel</span>{:else}<Icon name="x" size={12} />{/if}
            </button>
          </div>
          <div class="bar"><div class="fill" style="width: {percent(t.done, t.size)}%; background: {look.color}"></div></div>
          <div class="sub">
            <span class="cut" style="color: {look.text}">{look.label}</span>
            <span class="mono">{r > 0 ? `${percent(t.done, t.size)}% · ${speed(r)}` : `${percent(t.done, t.size)}% of ${bytes(t.size)}`}</span>
          </div>
        </div>
        </div>
      {/each}
      {#each dDone as h (h.id)}
        {@const w = whoOf(h.contact)}
        {@const end = dEnd(h)}
        <div class="row" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
        <div class="item done" in:fadeIn={{ duration: 160 }}>
          <div class="top">
            {@render who(w)}
            <MiddleText text={h.name} class="nm" />
            {#if h.result === "done" && !h.outgoing && h.savedTo}
              <button class="act" onclick={() => openReceived(h, false)}>Open</button>
              <button class="act" onclick={() => openReceived(h, true)}>Folder</button>
            {:else}
              <span class="end" style:color={end.text}>{end.label}</span>
            {/if}
          </div>
        </div>
        </div>
      {/each}
    {/if}
  </div>
  {:else}
  <div class="list" in:fadeIn={{ duration: 160 }}>
    {#if !list.length}
      <div class="empty" in:fadeIn={{ duration: 160 }}>
        <Icon name="check" size={20} stroke={2} />
        <p>Nothing downloading.</p>
      </div>
    {:else}
      {#each list as d, i (d.id)}
        {@const look = statusLook(d.status, d.message)}
        {@const p = app.live[d.id]}
        <div class="row" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
        {#if i === openShown && more > 0}<div class="more">and {more} more in Pako</div>{/if}
        {#if d.status === "completed"}
        <div class="item done" in:fadeIn={{ duration: 160 }}>
          <div class="top">
            <span class="ok"><Icon name="check" size={11} stroke={2.6} /></span>
            <MiddleText text={d.name} class="nm" />
            <button class="act" onclick={() => app.run(() => api.openItem(d.id, false))}>Open</button>
            <button class="act" onclick={() => app.run(() => api.openItem(d.id, true))}>Folder</button>
          </div>
        </div>
        {:else}
        <div class="item">
          <div class="top">
            <MiddleText text={d.name} class="nm" />
            {#if isActive(d.status) || d.status === "queued"}
              <button class="icon-btn" aria-label="Pause {d.name}" onclick={() => app.run(() => api.pause([d.id]))}><Icon name="pause" size={12} /></button>
            {:else}
              <button class="icon-btn" aria-label="Resume {d.name}" onclick={() => app.run(() => api.resume([d.id]))}><Icon name="play" size={12} /></button>
            {/if}
            <button
              class={armed?.id === d.id ? "act danger" : "icon-btn"}
              aria-label={armed?.id === d.id ? `Remove ${d.name} now: what was downloaded so far is deleted` : `Remove ${d.name}`}
              title={armed?.id === d.id ? "What was downloaded so far is deleted" : "Remove"}
              onclick={() => removeClick(d)}
            >
              {#if armed?.id === d.id}<span in:fadeIn={{ duration: 120 }}>Remove</span>{:else}<Icon name="x" size={12} />{/if}
            </button>
          </div>
          <div class="bar"><div class="fill" style="width: {pct(d)}%; background: {look.color}"></div></div>
          <div class="sub">
            <span style="color: {look.text}">{videoAs(d) ? `${look.label} · ${videoAs(d)}` : look.label}</span>
            <span class="mono">{p?.speed ? speed(p.speed) : d.size ? `${Math.floor(pct(d))}% of ${bytes(d.size)}` : bytes(d.downloaded)}</span>
          </div>
        </div>
        {/if}
        </div>
      {/each}
      {#if openShown === list.length && more > 0}<div class="more">and {more} more in Pako</div>{/if}
    {/if}
  </div>
  {/if}

  <footer>
    {#if tab === "down"}
      <button class="btn ghost small" onclick={() => app.run(() => api.pauseAll())} in:fadeIn={{ duration: 160 }}><Icon name="pause" size={12} />Pause all</button>
      <button class="btn ghost small" onclick={() => app.run(() => api.resumeAll())} in:fadeIn={{ duration: 160 }}><Icon name="play" size={12} />Resume all</button>
    {/if}
    <button class="btn small open" onclick={openPako}>Open Pako</button>
  </footer>
</div>
{/key}
<!-- "Link copied", or why Open or Pause didn't work: said here too, over the list. -->
<Toasts plain />
</div>

<style>
  /* Downloads / Uploads (/ Direct). --k goes 0 → 1 as the Direct half opens out: the halves
     share the width and the highlight narrows and moves with it, all in one smooth step. */
  @property --k {
    syntax: "<number>";
    inherits: true;
    initial-value: 0;
  }
  .switch2 {
    --k: 0;
    position: relative;
    display: flex;
    transition: --k var(--mid) var(--ease);
    margin: 10px 12px 4px;
    padding: 2px;
    border-radius: calc(var(--radius) + 2px);
    background: var(--bg);
    border: 1px solid var(--line);
  }
  /* One highlight under the picked half that slides across. */
  .thumb {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 2px;
    width: calc((100% - 6px - 2px * var(--k)) / (2 + var(--k)));
    border-radius: var(--radius);
    background: var(--raise);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    pointer-events: none;
    transition: transform var(--mid) var(--ease);
  }
  :global(:root[data-theme="light"]) .thumb {
    background: var(--bg);
    box-shadow:
      0 1px 2px rgba(20, 26, 46, 0.12),
      0 0 0 1px rgba(20, 26, 46, 0.04);
  }
  .switch2.three {
    --k: 1;
  }
  .switch2 button {
    flex: 1 1 0;
    min-width: 0;
    padding: 0;
    position: relative;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-3);
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--fast) ease, color var(--fast) ease;
  }
  .switch2 button + button {
    margin-left: 2px;
  }
  .switch2 button.third {
    flex-grow: var(--k);
    margin-left: calc(2px * var(--k));
    overflow: hidden;
    white-space: nowrap;
    opacity: var(--k);
  }
  .switch2 button:hover {
    color: var(--text-2);
  }
  .switch2 button.on {
    color: var(--text);
  }
  .cnt {
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: var(--radius-lg);
    /* things in progress: azure, like the sidebar's counts (red would read as problems) */
    background: rgba(77, 163, 255, 0.14);
    color: var(--st-run-chip);
    font-size: 10.5px;
    font-weight: 600;
    display: inline-grid;
    place-items: center;
  }

  /* See-through window: only the list (with its shadow) shows. */
  :global(html),
  :global(body) {
    background: transparent !important;
  }
  .frame {
    height: 100vh;
    box-sizing: border-box;
    padding: 14px;
  }
  /* Messages sit inside the list's box, just over its bottom bar. */
  .frame :global(div.toasts) {
    right: 26px;
    bottom: 70px;
    max-width: calc(100vw - 52px);
  }
  .mini {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45), 0 2px 6px rgba(0, 0, 0, 0.3);
    overflow: hidden;
    box-sizing: border-box;
    animation: rise 220ms var(--ease-out) both;
  }
  /* Light: softer, in two layers (the window around it is see-through). */
  :global(:root[data-theme="light"]) .mini {
    box-shadow:
      0 8px 24px rgba(20, 26, 46, 0.14),
      0 1px 4px rgba(20, 26, 46, 0.08);
  }
  .mini.from-top {
    animation-name: drop;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.985);
    }
  }
  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-10px) scale(0.985);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .mini {
      animation: none;
    }
  }
  /* The main window's title bar, scaled down (32 px instead of 40), like the Download
     with Pako window's. */
  header {
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
  .spd {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-3);
  }
  .spd.on {
    color: var(--st-run-text);
  }
  .spd.done {
    color: var(--st-done-text);
    font-weight: 500;
  }
  .grow {
    flex: 1;
    align-self: stretch;
  }
  .gear {
    width: 26px;
    height: 24px;
    margin-right: 6px;
  }
  .sep {
    width: 1px;
    height: 14px;
    background: var(--line);
  }
  .small {
    height: 28px;
    padding: 0 10px;
    font-size: 12px;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px 0;
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 9px 12px 10px 14px;
    border-bottom: 1px solid var(--line-soft);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .top :global(.nm) {
    flex: 1;
    min-width: 0;
    font-weight: 500;
  }
  .top .icon-btn {
    width: 24px;
    height: 24px;
    flex: none;
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
    transition: width 900ms linear;
  }
  .sub {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
  }
  .item.done {
    padding-top: 6px;
    padding-bottom: 6px;
  }
  /* Who a Direct transfer is with: their initial, or a PC for one of your own. */
  .who {
    width: 18px;
    height: 18px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--avatar-bg);
    box-shadow: inset 0 0 0 1px var(--avatar-ring);
    color: var(--text);
    font-size: 10px;
    font-weight: 600;
    line-height: 1;
  }
  .who.dev {
    color: var(--text-2);
  }
  .sub .cut {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub .mono {
    flex: none;
  }
  .end {
    flex: none;
    padding: 0 6px;
    font-size: 12px;
  }
  .ok {
    width: 16px;
    height: 16px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: rgba(70, 201, 126, 0.14);
    color: var(--st-done-text);
  }
  .act {
    height: 24px;
    padding: 0 6px;
    flex: none;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--focus-text);
    font-size: 12px;
    font-weight: 500;
  }
  .act:hover {
    background: var(--raise);
  }
  .act.danger {
    color: var(--st-fail-text);
  }
  .more {
    padding: 10px 14px;
    font-size: 12px;
    color: var(--text-3);
  }
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-3);
  }
  .empty p {
    margin: 0;
    font-size: 12.5px;
  }
  footer {
    height: 44px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    border-top: 1px solid var(--line);
  }
  footer .open {
    margin-left: auto;
  }
</style>
