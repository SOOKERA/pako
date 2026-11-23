<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // My devices › a device › Links: the links this PC sent that device to download, a card per
  // send (from here, from Links, from the Download with Pako window), with how they are doing
  // there (its Downloads list, matched by the downloads each link became). At the foot, the
  // box to send more: files and single videos go at once; a playlist, a Drive folder or a
  // link page is read on this PC first, its videos or files picked in a card here, then each
  // one is sent with its quality, priority and the playlist's or folder's name as subfolder.
  import { onMount, untrack } from "svelte";
  import { app } from "./state.svelte";
  import { inApp, type Priority } from "./api";
  import { devices, devicesApi, type Device, type DevicesState, type PickItem, type RemoteItem, type SentCard, type SentLink } from "./devices.svelte";
  import { bytes, duration } from "./format";
  import Icon from "./Icon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import RemoteRow from "./RemoteRow.svelte";
  import { fadeIn, pop, rowSlide } from "./motion";
  import { i18n, num, t, type Key } from "./i18n.svelte";

  let { device: x, state: s, ongot }: { device: Device; state: DevicesState; ongot?: () => void } = $props();

  const cards = $derived(s.sent.filter((c) => c.device === x.id));
  const remote = $derived(s.remote[x.id]?.items ?? null);
  const defaultQ = $derived(app.settings?.videoQuality ?? 1080);

  // ---------- the cards ----------
  const PRIO: Record<string, Key> = { first: "First", normal: "Normal", later: "Later" };
  const qLabel = (q: number) => (q === 0 ? t("Audio only") : t("Best up to {q}p", { q }));
  /** "3 links · Normal · Best up to 1080p" */
  function headline(c: SentCard) {
    const parts = [t("{n} links", { n: num(c.links.length) })];
    const ps = [...new Set(c.links.map((l) => l.priority ?? "normal"))];
    parts.push(ps.length === 1 ? t(PRIO[ps[0]] ?? "Normal") : t("Mixed priority"));
    const qs = [...new Set(c.links.filter((l) => l.quality != null).map((l) => l.quality as number))];
    if (qs.length) parts.push(qs.length === 1 ? qLabel(qs[0]) : t("Mixed quality"));
    return parts.join(" · ");
  }
  function when(ms: number) {
    const d = new Date(ms);
    const loc = i18n.dates;
    const time = d.toLocaleTimeString(loc, { hour: "2-digit", minute: "2-digit" });
    if (new Date().toDateString() === d.toDateString()) return t("today {time}", { time });
    return `${d.toLocaleDateString(loc, { day: "numeric", month: "short" })}, ${time}`;
  }
  /** "Sent today 14:02 · into Downloads" (or the subfolder). */
  function subline(c: SentCard) {
    const fs = [...new Set(c.links.map((l) => l.folder ?? ""))];
    const into = fs.length === 1 ? (fs[0] ? t("into {folder}", { folder: fs[0] }) : t("into Downloads")) : t("into several folders");
    return t("Sent {when} · {into}", { when: when(c.date), into });
  }
  /** Its downloads there, as that PC's list shows them now (undefined: no longer in it). */
  const itemsOf = (l: SentLink) => (l.ids ?? []).map((id) => remote?.find((r) => r.id === id));
  /** "2 done · 1 downloading 64%", from that PC's list. Null when there's nothing to follow. */
  function live(c: SentCard): { text: string; tone: "done" | "run" | "bad" | "idle" } | null {
    const ids = c.links.flatMap((l) => l.ids ?? []);
    const failed = c.links.filter((l) => l.failed).length;
    if (!ids.length) return failed ? { text: t("{n} not taken", { n: failed }), tone: "bad" } : { text: t("Sent"), tone: "idle" };
    if (!x.online) return { text: t("Shows how they're doing while {name} is on", { name: x.name }), tone: "idle" };
    if (!remote) return { text: t("Asking for their state…"), tone: "idle" };
    const got = ids.map((id) => remote.find((r) => r.id === id));
    const by = (f: (r: RemoteItem) => boolean) => got.filter((r): r is RemoteItem => !!r && f(r));
    const done = by((r) => r.status === "completed");
    const running = by((r) => ["downloading", "starting", "retrying", "verifying"].includes(r.status));
    const waiting = by((r) => r.status === "queued");
    const paused = by((r) => r.status === "paused");
    const bad = by((r) => !["completed", "downloading", "starting", "retrying", "verifying", "queued", "paused"].includes(r.status));
    const gone = got.filter((r) => !r).length;
    const parts: string[] = [];
    if (done.length) parts.push(t("{n} done", { n: done.length }));
    if (running.length) {
      const size = running.reduce((a, r) => a + r.size, 0);
      const pct = size ? Math.floor((running.reduce((a, r) => a + r.done, 0) / size) * 100) : 0;
      parts.push(`${t("{n} downloading", { n: running.length })}${size ? ` ${t("{pct}%", { pct })}` : ""}`);
    }
    if (waiting.length) parts.push(t("{n} waiting", { n: waiting.length }));
    if (paused.length) parts.push(t("{n} paused", { n: paused.length }));
    if (bad.length + failed) parts.push(t("{n} failed", { n: bad.length + failed }));
    if (gone) parts.push(t("{n} off the list", { n: gone }));
    const tone = bad.length + failed ? "bad" : running.length || waiting.length ? "run" : done.length === ids.length ? "done" : "idle";
    return { text: parts.join(" · ") || t("Sent"), tone };
  }
  let open = $state<Record<string, boolean>>({});

  // ---------- picking a playlist, a Drive folder or a link page ----------
  type Pick = PickItem & { on: boolean; q: string };
  type Picker = { key: string; link: string; state: "reading" | "ready" | "failed"; title: string; items: Pick[]; error: string; sending: boolean };
  let pickers = $state<Picker[]>([]);
  const isPlaylist = (link: string) => /youtube\.com\/playlist|\/sets\/|\/album\/|\/showcase\//i.test(link);
  const isFolder = (link: string) => /drive\.google\.com\/(drive\/)?(u\/\d+\/)?folders\//i.test(link);
  function reading(p: Picker) {
    return isPlaylist(p.link) ? t("Reading the playlist…") : isFolder(p.link) ? t("Reading the folder…") : t("Opening the link page…");
  }
  async function addPicker(link: string) {
    if (pickers.some((p) => p.link === link && p.state !== "failed")) return;
    const key = `${link}#${Date.now()}`;
    pickers = [...pickers, { key, link, state: "reading", title: "", items: [], error: "", sending: false }];
    const find = () => pickers.find((p) => p.key === key);
    try {
      const r = await devicesApi.pick(link);
      const p = find();
      if (!p) return;
      p.title = r.title;
      p.items = r.items.map((i) => ({ ...i, on: true, q: quality }));
      p.state = "ready";
    } catch (e) {
      const p = find();
      if (!p) return;
      p.error = String(e);
      p.state = "failed";
    }
  }
  const qOf = (v: string) => (v === "default" ? null : Number(v));
  async function sendPicked(p: Picker) {
    const chosen = p.items.filter((i) => i.on);
    if (!chosen.length || p.sending) return;
    p.sending = true;
    const into = folder.trim();
    const names: Record<string, string> = {};
    for (const i of chosen) names[i.link] = i.name;
    try {
      const r = await devicesApi.addLinks(
        x.id,
        chosen.map((i) => ({ link: i.link, quality: i.video ? qOf(i.q) : null, priority: priority === "normal" ? null : priority, folder: into || i.folder })),
        names,
      );
      // What that PC didn't take stays in the card, to send again; the rest leave it.
      const left = new Set(r.failed);
      if (left.size) {
        p.items = p.items.filter((i) => i.on && left.has(i.link.trim()));
        p.sending = false;
        p.error = t("{name} didn't take these {n}: {why}", { name: x.name, n: left.size, why: r.text });
      } else pickers = pickers.filter((q) => q.key !== p.key);
      result = { ok: !r.failed.length, text: t("{what} on {name}.", { what: r.text, name: x.name }) };
      devicesRefresh();
    } catch (e) {
      p.sending = false;
      p.error = String(e);
    }
  }

  // ---------- the box ----------
  let text = $state("");
  let quality = $state("default");
  let priority = $state<Priority>("normal");
  let folder = $state("");
  let busy = $state(false);
  let result = $state<{ ok: boolean; text: string } | null>(null);
  const HEIGHTS = [2160, 1440, 1080, 720, 480, 360];
  async function send() {
    if (!text.trim() || busy) return;
    busy = true;
    result = null;
    try {
      const r = await devicesApi.download(x.id, text, qOf(quality), priority === "normal" ? null : priority, folder.trim() || null);
      text = "";
      for (const l of r.pick) addPicker(l);
      if (r.text) result = { ok: true, text: t("{what} on {name}.", { what: r.text, name: x.name }) };
      else if (r.pick.length) result = { ok: true, text: t("Pick what to send in the cards above.", { n: r.pick.length }) };
      devicesRefresh();
    } catch (e) {
      result = { ok: false, text: String(e) };
    } finally {
      busy = false;
    }
  }
  const devicesRefresh = () => devices.refresh();

  // Newest at the bottom, like a chat: the list follows new cards.
  let timeline: HTMLDivElement | undefined = $state();
  let scrolledOnce = false;
  const count = $derived(cards.length + pickers.length);
  $effect(() => {
    void count;
    const glide = untrack(() => scrolledOnce) && !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    scrolledOnce = true;
    queueMicrotask(() => timeline?.scrollTo({ top: timeline.scrollHeight, behavior: glide ? "smooth" : "auto" }));
  });

  onMount(() => {
    // Design preview: an open playlist picker next to the finished card.
    if (!inApp && x.name === "Home PC" && !pickers.length) addPicker("https://www.youtube.com/playlist?list=PLpianolessons");
  });
</script>

<div class="links">
  <div class="timeline" bind:this={timeline}>
    {#each cards as c (c.id)}
      {@const st = live(c)}
      <div class="msg out" in:pop={{ duration: 240, from: 0.98, y: 8, origin: "right bottom" }}>
        <div class="bubble">
          <div class="mtop">
            <span class="licon"><Icon name="link" size={14} /></span>
            <span class="mhead">{headline(c)}</span>
          </div>
          <div class="mmeta">{subline(c)}</div>
          {#if st}
            <div class="mstate" class:done={st.tone === "done"} class:run={st.tone === "run"} class:bad={st.tone === "bad"}>{st.text}</div>
          {/if}
          <div class="mfoot">
            <button class="link" aria-expanded={!!open[c.id]} onclick={() => (open = { ...open, [c.id]: !open[c.id] })}>
              {open[c.id] ? t("Hide the links") : t("Show the {n} links", { n: c.links.length })}
            </button>
          </div>
          {#if open[c.id]}
            <div class="items" transition:rowSlide={{ duration: 200 }}>
              {#each c.links as l (l.link)}
                {@const got = itemsOf(l)}
                {#if got.length && got.every(Boolean) && remote}
                  {#each got as it (it!.id)}<RemoteRow device={x} item={it!} {ongot} compact />{/each}
                {:else}
                  <div class="iline">
                    <MiddleText text={l.name || l.link} class="iname" />
                    <span class="istate" class:bad={l.failed}>
                      {#if l.failed}{t("Not taken")}{:else if (l.ids ?? []).length && remote}{t("Off its list")}{:else if (l.ids ?? []).length}{t("Sent")}{:else}{t("Sent · no live state from that Pako")}{/if}
                    </span>
                  </div>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/each}

    {#each pickers as p (p.key)}
      {@const picked = p.items.filter((i) => i.on).length}
      <div class="msg out" in:pop={{ duration: 240, from: 0.98, y: 8, origin: "right bottom" }}>
        <div class="bubble picker">
          <div class="mtop">
            <span class="licon"><Icon name={isFolder(p.link) ? "folder" : p.items.some((i) => i.video) || isPlaylist(p.link) ? "play" : "link"} size={14} /></span>
            <span class="mhead"><MiddleText text={p.title || p.link} /></span>
            <button class="icon-btn" aria-label={t("Cancel")} title={t("Cancel")} onclick={() => (pickers = pickers.filter((q) => q.key !== p.key))}><Icon name="x" size={13} /></button>
          </div>
          {#if p.state === "reading"}
            <div class="mmeta reading" in:fadeIn={{ duration: 160 }}><span class="ring"></span>{reading(p)}</div>
          {:else if p.state === "failed"}
            <div class="mstate bad" in:fadeIn={{ duration: 160 }}>{p.error}</div>
          {:else}
            <div class="mmeta pickhead">
              <span>{p.items.some((i) => i.video) ? t("{n} videos", { n: num(p.items.length) }) : t("{n} files", { n: num(p.items.length) })} · {t("{n} picked", { n: picked })}</span>
              <button class="link" onclick={() => { const all = picked < p.items.length; for (const i of p.items) i.on = all; }}>{picked < p.items.length ? t("Tick all") : t("Untick all")}</button>
            </div>
            <div class="plist" transition:rowSlide={{ duration: 200 }}>
              {#each p.items as i (i.link)}
                <div class="prow" class:off={!i.on}>
                  <button class="check" class:on={i.on} role="checkbox" aria-checked={i.on} aria-label={i.on ? t("Untick {name}", { name: i.name }) : t("Tick {name}", { name: i.name })} onclick={() => (i.on = !i.on)}>
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
                  </button>
                  <MiddleText text={i.name} class="pname" />
                  <span class="pmeta mono">{i.video ? (i.duration ? duration(i.duration) : "") : i.size != null ? bytes(i.size) : ""}</span>
                  {#if i.video}
                    <select class="input tiny" aria-label={t("Quality of {name}", { name: i.name })} bind:value={i.q} disabled={!i.on}>
                      <option value="default">{qLabel(defaultQ)}</option>
                      {#each HEIGHTS as h (h)}<option value={String(h)}>{h}p</option>{/each}
                      <option value="0">{t("Audio only")}</option>
                    </select>
                  {/if}
                </div>
              {/each}
            </div>
            {#if p.error}<div class="mstate bad">{p.error}</div>{/if}
            <div class="pfoot">
              <span class="dim">{t("Into {folder} on {name}", { folder: folder.trim() || p.items[0]?.folder || t("Downloads"), name: x.name })}</span>
              <button class="btn ghost small" onclick={() => (pickers = pickers.filter((q) => q.key !== p.key))}>{t("Cancel")}</button>
              <button class="btn primary small" disabled={!picked || p.sending || !x.online} onclick={() => sendPicked(p)}>
                <Icon name="send" size={12} />{t("Send {n} to {name}", { n: picked, name: x.name })}
              </button>
            </div>
          {/if}
        </div>
      </div>
    {/each}

    {#if !cards.length && !pickers.length}
      <div class="nothing">
        <Icon name="link" size={20} />
        <p>{t("No links sent to {name} yet.", { name: x.name })}</p>
        <p class="dim">{t("Paste links below: {name} downloads them into its own download folder. A playlist or a folder opens here first, to pick what to send.", { name: x.name })}</p>
      </div>
    {/if}
  </div>

  <footer class="compose" data-tour="devices-order">
    <div class="orow">
      <input class="input grow" bind:value={text} placeholder={t("Paste links for {name} to download", { name: x.name })} spellcheck="false" aria-label={t("Links for {name} to download", { name: x.name })} onkeydown={(e) => e.key === "Enter" && send()} />
      <button class="btn primary small send" disabled={!text.trim() || busy || !x.online} title={x.online ? "" : t("{name} is offline", { name: x.name })} onclick={send}><Icon name="send" size={12} />{t("Send")}</button>
    </div>
    <div class="opts">
      <label class="opt">
        <span>{t("Quality")}</span>
        <select class="input tiny" bind:value={quality} aria-label={t("Quality for videos")}>
          <option value="default">{qLabel(defaultQ)}</option>
          {#each HEIGHTS as h (h)}<option value={String(h)}>{h}p</option>{/each}
          <option value="0">{t("Audio only")}</option>
        </select>
      </label>
      <label class="opt">
        <span>{t("Priority")}</span>
        <select class="input tiny" bind:value={priority} aria-label={t("Priority")}>
          <option value="first">{t("First")}</option>
          <option value="normal">{t("Normal")}</option>
          <option value="later">{t("Later")}</option>
        </select>
      </label>
      <label class="opt grow">
        <span>{t("Into folder")}</span>
        <input class="input tiny grow" bind:value={folder} maxlength="120" placeholder={t("Optional, inside its download folder")} spellcheck="false" aria-label={t("Into folder (optional)")} />
      </label>
    </div>
    {#if result}
      <p class="result" class:bad={!result.ok} transition:rowSlide={{ duration: 200 }}>{result.text}</p>
    {/if}
  </footer>
</div>

<style>
  .links {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
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
    max-width: 380px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .nothing p.dim {
    color: var(--text-3);
  }
  .msg {
    display: flex;
    justify-content: flex-end;
  }
  .bubble {
    width: min(460px, 86%);
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    background: var(--bubble-out);
    border: 1px solid var(--bubble-line);
    box-shadow: var(--bubble-shadow);
  }
  .bubble.picker {
    width: min(520px, 92%);
  }
  .mtop {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .licon {
    flex: none;
    width: 18px;
    display: grid;
    place-items: center;
    color: var(--text-2);
  }
  .mhead {
    flex: 1;
    min-width: 0;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .mmeta,
  .mstate,
  .mfoot {
    margin: 4px 0 0 26px;
    font-size: 12px;
    color: var(--bubble-meta);
    font-variant-numeric: tabular-nums;
  }
  .mstate {
    font-weight: 500;
    color: var(--text-2);
  }
  .mstate.done {
    color: var(--delivered-text);
  }
  .mstate.run {
    color: var(--st-run-text);
  }
  .mstate.bad {
    color: var(--st-fail-text);
  }
  .mfoot {
    margin-top: 6px;
  }
  .reading {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ring {
    width: 11px;
    height: 11px;
    border-radius: 50%;
    border: 2px solid var(--line-strong);
    border-top-color: var(--st-check);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ring {
      animation: none;
    }
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
  .items {
    margin: 6px 0 0 18px;
    display: flex;
    flex-direction: column;
  }
  .iline {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 4px;
    font-size: 12.5px;
    min-width: 0;
  }
  .iline :global(.iname) {
    flex: 1;
    min-width: 0;
  }
  .istate {
    flex: none;
    font-size: 12px;
    color: var(--text-3);
  }
  .istate.bad {
    color: var(--st-fail-text);
  }
  .pickhead {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .pickhead .link {
    margin-left: auto;
  }
  .plist {
    margin: 8px 0 0 18px;
    max-height: 260px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .prow {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 4px;
    border-radius: var(--radius);
    font-size: 12.5px;
    transition:
      background var(--fast) ease,
      opacity var(--fast) ease;
  }
  .prow:hover {
    background: var(--raise-hover);
  }
  .prow.off :global(.pname),
  .prow.off .pmeta {
    opacity: 0.55;
  }
  .prow :global(.pname) {
    flex: 1;
    min-width: 0;
  }
  .pmeta {
    flex: none;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .check {
    flex: none;
    width: 16px;
    height: 16px;
    display: grid;
    place-items: center;
    padding: 0;
    border-radius: 4px;
    border: 1.5px solid var(--line-strong);
    background: transparent;
    cursor: pointer;
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease;
  }
  .check svg {
    opacity: 0;
    transition: opacity var(--fast) ease;
  }
  .check.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .check.on svg {
    opacity: 1;
  }
  .pfoot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 0 0 26px;
    font-size: 12px;
  }
  .pfoot .dim {
    flex: 1;
    min-width: 0;
    color: var(--bubble-meta);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .icon-btn {
    flex: none;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-3);
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .icon-btn:hover {
    background: var(--raise-hover);
    color: var(--text);
  }
  /* ---------- the box ---------- */
  .compose {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--line);
    background: var(--panel);
    font-size: 12.5px;
  }
  .orow,
  .opts {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .opts {
    gap: 14px;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--text-3);
    font-size: 12px;
    white-space: nowrap;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .input.tiny {
    height: 26px;
    padding-top: 0;
    padding-bottom: 0;
    font-size: 12px;
  }
  select.input.tiny {
    width: auto;
    padding-right: 26px;
  }
  .btn.small {
    height: 28px;
    font-size: 12px;
  }
  .orow .input,
  .orow .send {
    height: 30px;
  }
  .send {
    min-width: 76px;
  }
  .result {
    margin: 0;
    font-size: 12.5px;
    color: var(--st-done-text);
  }
  .result.bad {
    color: var(--st-fail-text);
  }
</style>
