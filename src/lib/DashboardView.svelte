<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Dashboard: your Pako in numbers. Read from data/numbers.sqlite (running totals per day, week
  // and month, so every period opens at once). Everything stays on this PC.
  import { onMount } from "svelte";
  import { save as saveFile } from "@tauri-apps/plugin-dialog";
  import { api, inApp, type NumbersBucket, type NumbersPeriod, type NumbersRoute, type NumbersView } from "./api";
  import { app } from "./state.svelte";
  import type { MenuItem } from "./menu";
  import { bytes, speed } from "./format";
  import { barPath, bucketLabel, bucketTitle, dayLabel, labelEvery, niceMax } from "./chart";
  import Icon from "./Icon.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import Modal from "./Modal.svelte";
  import LiveSpeed from "./LiveSpeed.svelte";
  import SegPick from "./SegPick.svelte";
  import EaseNum from "./EaseNum.svelte";
  import { fadeIn, flipIf, flipOpts, pop, rowSlide } from "./motion";
  import "./dash.css";

  let { onmenu }: { onmenu: (x: number, y: number, items: MenuItem[]) => void } = $props();

  const PERIODS: { id: NumbersPeriod; label: string }[] = [
    { id: "today", label: "Today" },
    { id: "week", label: "This week" },
    { id: "month", label: "This month" },
    { id: "all", label: "All time" },
  ];
  let period = $state<NumbersPeriod>("month");
  let v = $state<NumbersView | null>(null);

  async function load() {
    const p = period;
    try {
      const got = await api.numbersView(p);
      app.numbersEmpty = got.since === null;
      if (p === period) v = got;
    } catch {
      /* keep the last numbers */
    }
  }
  $effect(() => {
    void period;
    load();
  });
  onMount(() => {
    // Numbers change when a transfer finishes: a quiet refresh now and then is enough.
    const t = setInterval(load, 15000);
    return () => clearInterval(t);
  });

  /** Nothing was ever counted. */
  const empty = $derived(v !== null && v.since === null);
  const when = $derived({ today: "today", week: "this week", month: "this month", all: "yet" }[period]);
  const plural = (n: number, one: string, many = one + "s") => `${n.toLocaleString()} ${n === 1 ? one : many}`;

  // ---------- the four totals ----------
  /** How often people downloaded the links you sent in this period (Gofile's last count). */
  const fetchedByOthers = $derived.by(() => {
    if (!v) return 0;
    const from = new Date(v.from + "T00:00:00").getTime();
    return app.sent.filter((s) => s.date >= from && !s.incomplete).reduce((n, s) => n + (s.downloads ?? 0), 0);
  });
  const directShare = $derived(v && v.sent.bytes ? Math.round((v.sent.directBytes / v.sent.bytes) * 100) : null);

  // ---------- activity: what moved, per hour, day or month ----------
  const KINDS = [
    { k: "download", label: "Downloaded", color: "var(--c-down)" },
    { k: "upload", label: "Uploaded", color: "var(--c-up)" },
    { k: "sent", label: "Sent direct", color: "var(--c-sent)" },
    { k: "received", label: "Received direct", color: "var(--c-recv)" },
  ] as const;
  const buckets = $derived(v?.buckets ?? []);
  const step = $derived(v?.step || "day");
  const withYear = $derived(step !== "hour" && buckets.length > 1 && buckets[0].key.slice(0, 4) !== buckets[buckets.length - 1].key.slice(0, 4));
  const sum = (b: NumbersBucket) => b.download + b.upload + b.sent + b.received;
  const hasActivity = $derived(buckets.some((b) => sum(b) > 0));

  let aw = $state(800);
  const AH = 196;
  const apad = { l: 66, r: 6, t: 10, b: 24 };
  const aMax = $derived(niceMax(Math.max(1024 * 1024, ...buckets.map(sum))));
  const aBand = $derived((aw - apad.l - apad.r) / Math.max(1, buckets.length));
  const ay = (val: number) => apad.t + (1 - val / aMax) * (AH - apad.t - apad.b);
  const aEvery = $derived(labelEvery(buckets.length, aw - apad.l - apad.r, step === "hour" ? 44 : 64));
  /** The bar's parts, bottom up; only the top one has round corners. */
  function stack(b: NumbersBucket, x: number, w: number) {
    const parts = KINDS.map((k) => ({ color: k.color, val: b[k.k] })).filter((p) => p.val > 0);
    let acc = 0;
    return parts.map((p, i) => {
      const base = ay(acc);
      acc += p.val;
      return { color: p.color, d: barPath(x, w, ay(acc), base, i === parts.length - 1) };
    });
  }
  let aHover = $state<string | null>(null);
  const aIndex = $derived(aHover === null ? -1 : buckets.findIndex((b) => b.key === aHover));

  // ---------- speed over time ----------
  type SpeedKind = "download" | "upload" | "direct";
  const SPEEDS: Record<SpeedKind, { label: string; color: string; none: string }> = {
    download: { label: "Downloads", color: "var(--c-down)", none: "No downloads" },
    upload: { label: "Uploads", color: "var(--c-up)", none: "No uploads" },
    direct: { label: "Direct", color: "var(--c-sent)", none: "No Direct transfers" },
  };
  let speedKind = $state<SpeedKind>("download");
  const speedOf = (b: NumbersBucket, k: SpeedKind) =>
    k === "download"
      ? { avg: b.downloadAvg, peak: b.downloadPeak, amount: b.download }
      : k === "upload"
        ? { avg: b.uploadAvg, peak: b.uploadPeak, amount: b.upload }
        : { avg: b.directAvg, peak: b.directPeak, amount: b.sent + b.received };
  let sw = $state(800);
  const SH = 180;
  const spad = { l: 78, r: 10, t: 26, b: 24 };
  const sPoints = $derived(buckets.map((b, i) => ({ i, key: b.key, ...speedOf(b, speedKind) })).filter((p) => p.avg !== null || p.peak > 0));
  const sMax = $derived(niceMax(Math.max(1024 * 1024, ...sPoints.map((p) => Math.max(p.peak, p.avg ?? 0)))));
  const sBand = $derived((sw - spad.l - spad.r) / Math.max(1, buckets.length));
  const sx = (i: number) => spad.l + (i + 0.5) * sBand;
  const sy = (val: number) => spad.t + (1 - val / sMax) * (SH - spad.t - spad.b);
  const sEvery = $derived(labelEvery(buckets.length, sw - spad.l - spad.r, step === "hour" ? 44 : 64));
  const avgPath = $derived(
    sPoints
      .filter((p) => p.avg !== null)
      .map((p, n) => (n ? "L" : "M") + sx(p.i).toFixed(1) + " " + sy(p.avg!).toFixed(1))
      .join(""),
  );
  const topPoint = $derived(sPoints.reduce<(typeof sPoints)[number] | null>((m, p) => (p.peak > (m?.peak ?? 0) ? p : m), null));
  let sHover = $state<string | null>(null);
  const sHoverPoint = $derived(sHover === null ? null : (sPoints.find((p) => p.key === sHover) ?? null));
  function onSpeedMove(e: PointerEvent) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect();
    const i = Math.floor((e.clientX - r.left - spad.l) / sBand);
    let best: (typeof sPoints)[number] | null = null;
    for (const p of sPoints) if (!best || Math.abs(p.i - i) < Math.abs(best.i - i)) best = p;
    sHover = best && Math.abs(best.i - i) <= Math.max(2, buckets.length / 15) ? best.key : null;
  }

  // ---------- which way is fastest ----------
  const UPLOAD_NAMES: Record<string, [string, string]> = {
    gofile: ["Gofile link", "link"],
    drive: ["Google Drive link", "link"],
    direct: ["Direct", "send"],
  };
  const SOURCE_NAMES: Record<string, [string, string]> = {
    drive: ["Google Drive", "file"],
    video: ["Videos", "video"],
    web: ["Other sites", "file"],
    direct: ["Received in Direct", "transfer"],
    earlier: ["Earlier downloads", "day"],
  };
  const SOURCE_COLORS: Record<string, string> = {
    drive: "var(--src-drive)",
    video: "var(--src-video)",
    web: "var(--src-web)",
    direct: "var(--c-recv)",
    earlier: "var(--track)",
  };
  /** The row with the best average, when at least two can be compared. */
  function fastest(rows: NumbersRoute[]) {
    const timed = rows.filter((r) => r.avg !== null);
    if (timed.length < 2) return null;
    return timed.reduce((m, r) => ((r.avg ?? 0) > (m.avg ?? 0) ? r : m)).route;
  }
  const scaleOf = (rows: NumbersRoute[]) => Math.max(1, ...rows.map((r) => Math.max(r.peak, r.avg ?? 0)));

  // ---------- people ----------
  const nameOf = (id: string) => app.directContacts.find((c) => c.id === id)?.name ?? "Removed contact";
  const peopleMax = $derived(Math.max(1, ...(v?.people ?? []).map((p) => p.sent + p.received)));
  function openPerson(id: string) {
    if (!app.directContacts.some((c) => c.id === id)) return;
    app.directPicked = id;
    app.view = "direct";
  }

  // ---------- sources ----------
  const downloadedTotal = $derived((v?.downloadSources ?? []).reduce((s, r) => s + r.bytes, 0));
  const sitesMax = $derived(Math.max(1, ...(v?.sites ?? []).map((s) => s.bytes)));
  const pct = (part: number, whole: number) => (whole ? Math.round((part / whole) * 100) : 0);

  // ---------- save, reset ----------
  let resetting = $state(false);
  function more(e: MouseEvent) {
    const b = (e.currentTarget as HTMLElement).getBoundingClientRect();
    onmenu(b.right - 250, b.bottom + 4, [
      { label: "Save the details as a CSV file…", icon: "download", disabled: empty, action: exportCsv },
      { separator: true },
      { label: "Reset statistics…", icon: "trash", danger: true, disabled: empty, action: () => (resetting = true) },
    ]);
  }
  async function exportCsv() {
    if (!inApp) {
      app.toast("Preview mode: nothing is saved");
      return;
    }
    const today = new Date();
    const stamp = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
    const path = await saveFile({ title: "Save Pako's numbers", defaultPath: `Pako numbers ${stamp}.csv`, filters: [{ name: "CSV (opens in Excel)", extensions: ["csv"] }] });
    if (!path) return;
    try {
      const n = await api.numbersExport(path);
      app.toast(`Saved ${plural(n, "transfer")} to the file.`);
    } catch (err) {
      app.toast(String(err), "error");
    }
  }
  async function doReset() {
    resetting = false;
    try {
      await api.numbersReset();
      await load();
      app.toast("Statistics reset.");
    } catch (err) {
      app.toast(String(err), "error");
    }
  }

  const rangeText = $derived.by(() => {
    if (!v) return "";
    if (period === "today") return "Today, hour by hour";
    const by = step === "month" ? "month by month" : "day by day";
    if (period === "all") return `Since ${dayLabel(v.from)}, ${by}`;
    return `${dayLabel(v.from)} to ${dayLabel(v.to)}, ${by}`;
  });
</script>

<div class="dash">
  <div class="toolbar">
    <h1>Dashboard</h1>
    <TutorialButton id="dashboard" />
    <div class="spacer"></div>
    <span class="period" data-tour="dash-period"><SegPick label="Period" options={PERIODS.map((p) => ({ value: p.id, label: p.label }))} value={period} onpick={(p) => (period = p)} /></span>
    <button class="icon-btn" data-tour="dash-more" aria-label="More" title="Save or reset the numbers" onclick={more}><Icon name="settings" size={14} /></button>
  </div>

  <div class="body">
    {#if empty}
      <section class="card emptycard" data-tour="dash-empty" in:pop={{ duration: 260, from: 0.98, y: 8 }}>
        <div class="emptyicon"><Icon name="chart" size={22} /></div>
        <h2>Your numbers show up as you use Pako</h2>
        <p>Downloads, uploads and Direct transfers are counted here as they finish: how much, how fast, and with whom. Only on this PC.</p>
      </section>
    {:else if v}
      <!-- The four totals -->
      <div class="tiles" data-tour="dash-totals" in:pop={{ duration: 260, from: 0.99, y: 8 }}>
        <div class="tile">
          <span class="k"><i style="background: var(--c-down)"></i>Downloaded</span>
          <span class="v mono"><EaseNum value={v.download.bytes} format={bytes} /></span>
          <span class="d">
            {plural(v.download.files, "file")}{#if v.download.failed}<span class="bad" title="Stopped with an error">&ensp;·&ensp;{v.download.failed} stopped</span>{/if}
          </span>
        </div>
        <div class="tile">
          <span class="k"><i style="background: var(--c-up)"></i>Uploaded</span>
          <span class="v mono"><EaseNum value={v.upload.bytes} format={bytes} /></span>
          <span class="d">
            {plural(v.upload.count, "link")}{#if v.upload.count}<span title="People downloaded these links {plural(fetchedByOthers, 'time')} (as Gofile last said)">&ensp;·&ensp;{plural(fetchedByOthers, "download")}</span>{/if}
          </span>
        </div>
        <div class="tile">
          <span class="k"><i style="background: var(--c-sent)"></i>Sent direct</span>
          <span class="v mono"><EaseNum value={v.sent.bytes} format={bytes} /></span>
          <span class="d">
            to {plural(v.sent.people, "person", "people")}{#if directShare !== null}<span title="{directShare}% went straight from your PC to theirs, the rest through a relay">&ensp;·&ensp;{directShare}% direct</span>{/if}
          </span>
        </div>
        <div class="tile">
          <span class="k"><i style="background: var(--c-recv)"></i>Received direct</span>
          <span class="v mono"><EaseNum value={v.received.bytes} format={bytes} /></span>
          <span class="d">from {plural(v.received.people, "person", "people")}&ensp;·&ensp;{plural(v.received.count, "transfer")}</span>
        </div>
      </div>

      <!-- Activity -->
      <section class="card" data-tour="dash-activity" in:pop={{ duration: 260, from: 0.99, y: 8, delay: 40 }}>
        <div class="head">
          <div>
            <h2>Activity</h2>
            <div class="sub">{rangeText}</div>
          </div>
        </div>
        <div class="legend">
          {#each KINDS as k (k.k)}<span><i style="background: {k.color}"></i>{k.label}</span>{/each}
        </div>
        <div class="chart" bind:clientWidth={aw}>
          {#if !hasActivity}
            <div class="nodata" in:fadeIn={{ duration: 160 }}>Nothing moved {when}.</div>
          {:else}
            {#key v.period}
            <svg width={aw} height={AH} role="img" aria-label="What Pako moved, {rangeText.toLowerCase()}" onpointerleave={() => (aHover = null)} in:fadeIn={{ duration: 160 }}>
              {#each [0, 0.5, 1] as f (f)}
                <line x1={apad.l} x2={aw - apad.r} y1={ay(aMax * f)} y2={ay(aMax * f)} class="grid" />
                <text x={apad.l - 10} y={ay(aMax * f) + 4} class="ax" text-anchor="end">{bytes(aMax * f)}</text>
              {/each}
              {#each buckets as b, i (b.key)}
                {@const x0 = apad.l + i * aBand}
                {@const w = Math.max(2, Math.min(aBand * 0.7, 28))}
                <rect x={x0} y={apad.t} width={aBand} height={AH - apad.t - apad.b} class="hit" class:on={aIndex === i} role="presentation" onpointerenter={() => (aHover = b.key)} />
                <g class="grow" style="transform-origin: 0 {AH - apad.b}px; animation-delay: {Math.min(i * 8, 160)}ms">
                  {#each stack(b, x0 + (aBand - w) / 2, w) as part, n (n)}
                    <path d={part.d} style="fill: {part.color}" class="barp" />
                  {/each}
                </g>
                {#if i % aEvery === 0}
                  <text x={x0 + aBand / 2} y={AH - 6} class="ax" text-anchor="middle">{bucketLabel(b.key, step, withYear)}</text>
                {/if}
              {/each}
            </svg>
            {/key}
            {#if aIndex >= 0}
              {@const b = buckets[aIndex]}
              {@const bx = apad.l + aIndex * aBand}
              <div class="tip glide" style="transform: translateX({bx + aBand + 228 > aw ? Math.max(0, bx - 222) : bx + aBand + 8}px)" in:fadeIn={{ duration: 120 }}>
                <div class="tt">{bucketTitle(b.key, step)}</div>
                {#each KINDS as k (k.k)}
                  <div class="tr"><i style="background: {k.color}"></i><span>{k.label}</span><b class="mono">{bytes(b[k.k])}</b></div>
                {/each}
                <div class="tr total"><i class="hollow"></i><span>In all</span><b class="mono">{bytes(sum(b))}</b></div>
              </div>
            {/if}
          {/if}
        </div>
      </section>

      <!-- Speed over time -->
      <section class="card" data-tour="dash-speed" in:pop={{ duration: 260, from: 0.99, y: 8, delay: 80 }}>
        <div class="head">
          <div>
            <h2>Speed</h2>
            <div class="sub">Average speed while moving, and the top speed reached.</div>
          </div>
          <SegPick
            small
            label="Which speed"
            options={(Object.keys(SPEEDS) as SpeedKind[]).map((k) => ({ value: k, label: SPEEDS[k].label }))}
            value={speedKind}
            onpick={(k) => (speedKind = k)}
          />
        </div>
        <div class="legend">
          <span><i style="background: {SPEEDS[speedKind].color}"></i>Average</span>
          <span><i class="range" style="--c: {SPEEDS[speedKind].color}"></i>Up to the top speed</span>
        </div>
        <div class="chart" bind:clientWidth={sw}>
          {#if !sPoints.length}
            <div class="nodata" in:fadeIn={{ duration: 160 }}>{SPEEDS[speedKind].none} with a measured speed {when}.</div>
          {:else}
            {#key `${v.period}|${speedKind}`}
            <svg width={sw} height={SH} role="img" aria-label="{SPEEDS[speedKind].label} speed, {rangeText.toLowerCase()}" onpointermove={onSpeedMove} onpointerleave={() => (sHover = null)} in:fadeIn={{ duration: 160 }}>
              {#each [0, 0.5, 1] as f (f)}
                <line x1={spad.l} x2={sw - spad.r} y1={sy(sMax * f)} y2={sy(sMax * f)} class="grid" />
                <text x={spad.l - 10} y={sy(sMax * f) + 4} class="ax" text-anchor="end">{speed(sMax * f)}</text>
              {/each}
              {#each buckets as b, i (b.key)}
                {#if i % sEvery === 0}
                  <text x={sx(i)} y={SH - 6} class="ax" text-anchor="middle">{bucketLabel(b.key, step, withYear)}</text>
                {/if}
              {/each}
              {#each sPoints as p (p.key)}
                {#if p.peak > (p.avg ?? 0)}
                  <line x1={sx(p.i)} x2={sx(p.i)} y1={sy(p.avg ?? 0)} y2={sy(p.peak)} class="whisker appear" style="stroke: {SPEEDS[speedKind].color}" />
                  <line x1={sx(p.i) - 3} x2={sx(p.i) + 3} y1={sy(p.peak)} y2={sy(p.peak)} class="whisker appear" style="stroke: {SPEEDS[speedKind].color}" />
                {/if}
              {/each}
              <path d={avgPath} class="ln draw" pathLength="1" style="stroke: {SPEEDS[speedKind].color}" />
              {#each sPoints as p (p.key)}
                {#if p.avg !== null}<circle cx={sx(p.i)} cy={sy(p.avg)} r={sHover === p.key ? 4.5 : 2.5} style="fill: {SPEEDS[speedKind].color}" class="dot appear" />{/if}
              {/each}
              {#if topPoint && topPoint.peak > 0}
                {@const tx = sx(topPoint.i)}
                {@const anchor = tx < spad.l + 70 ? "start" : tx > sw - 80 ? "end" : "middle"}
                <circle cx={tx} cy={sy(topPoint.peak)} r="4" class="top appear" />
                <text x={anchor === "start" ? tx - 4 : anchor === "end" ? tx + 4 : tx} y={sy(topPoint.peak) - 9} class="toplabel appear" text-anchor={anchor}>Top {speed(topPoint.peak)}</text>
              {/if}
            </svg>
            {/key}
            {#if sHoverPoint}
              {@const p = sHoverPoint}
              {@const px = sx(p.i)}
              <div class="tip glide" style="transform: translateX({px + 228 > sw ? Math.max(0, px - 222) : px + 12}px)" in:fadeIn={{ duration: 120 }}>
                <div class="tt">{bucketTitle(p.key, step)}</div>
                <div class="tr"><i style="background: {SPEEDS[speedKind].color}"></i><span>Average</span><b class="mono">{p.avg === null ? "Not measured" : speed(p.avg)}</b></div>
                <div class="tr"><i class="hollow"></i><span>Top speed</span><b class="mono">{p.peak ? speed(p.peak) : "Not measured"}</b></div>
                <div class="tr total"><i class="hollow"></i><span>Moved</span><b class="mono">{bytes(p.amount)}</b></div>
              </div>
            {/if}
          {/if}
        </div>
      </section>

      <!-- Which way is fastest -->
      <div class="pair" data-tour="dash-routes" in:pop={{ duration: 260, from: 0.99, y: 8, delay: 120 }}>
        {#each [{ title: "Sending files", sub: "Average upload speed, by the way you sent them.", rows: v.uploadRoutes, names: UPLOAD_NAMES, color: "var(--c-up)" }, { title: "Getting files", sub: "Average download speed, by where they came from.", rows: v.downloadSources, names: SOURCE_NAMES, color: "var(--c-down)" }] as card (card.title)}
          {@const scale = scaleOf(card.rows)}
          {@const best = fastest(card.rows)}
          <section class="card routes">
            <h2>{card.title}</h2>
            <div class="sub">{card.sub}</div>
            <div class="rlist">
              {#each card.rows as r (r.route)}
                {@const [name, unit] = card.names[r.route] ?? [r.route, "transfer"]}
                <div class="route" class:unused={!r.count}>
                  <div class="rname">
                    <span>{name}</span>
                    {#if best === r.route}<span class="badge">Fastest</span>{/if}
                  </div>
                  <div class="rvalue mono" title={r.avg === null ? "" : `Average ${speed(r.avg)} while moving`}>{#if r.avg === null}—{:else}<EaseNum value={r.avg} format={speed} />{/if}</div>
                  <div class="track" aria-hidden="true">
                    {#if r.avg !== null}<i class="fill" style="width: {(r.avg / scale) * 100}%; background: {card.color}"></i>{/if}
                    {#if r.peak}<i class="peak" style="left: {(r.peak / scale) * 100}%" title="Top speed {speed(r.peak)}"></i>{/if}
                  </div>
                  <div class="rsub">
                    {#if r.count}
                      {plural(r.count, unit)} · {bytes(r.bytes)}{#if r.peak}<span title="The top speed reached">&ensp;·&ensp;top {speed(r.peak)}</span>{/if}
                    {:else}
                      Not used {when}
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </section>
        {/each}
      </div>

      <!-- People and sources -->
      <div class="pair wide" in:pop={{ duration: 260, from: 0.99, y: 8, delay: 160 }}>
        <section class="card people" data-tour="dash-people">
          <h2>Your people</h2>
          <div class="sub">Who you exchanged the most with in Direct.</div>
          {#if !v.people.length}
            <div class="nodata small" in:fadeIn={{ duration: 160 }}>No Direct transfers {when}.</div>
          {:else}
            <div class="plist">
              {#each v.people as p, i (p.contact)}
                {@const name = nameOf(p.contact)}
                {@const total = p.sent + p.received}
                <button class="person" onclick={() => openPerson(p.contact)} title="Open {name} in Direct" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
                  <span class="rank mono">{i + 1}</span>
                  <span class="avatar">{name.slice(0, 1).toUpperCase()}</span>
                  <span class="who">
                    <span class="pname">{name}</span>
                    <span class="pbar" aria-hidden="true">
                      <i style="width: {(p.sent / peopleMax) * 100}%; background: var(--c-sent)"></i><i style="width: {(p.received / peopleMax) * 100}%; background: var(--c-recv)"></i>
                    </span>
                  </span>
                  <span class="amounts">
                    <span class="mono">
                      <span title="Sent to {name}: {bytes(p.sent)}" class:zero={!p.sent}><b class="up">↑</b> {bytes(p.sent)}</span>
                      <span class="sep">·</span>
                      <span title="Received from {name}: {bytes(p.received)}" class:zero={!p.received}><b class="down">↓</b> {bytes(p.received)}</span>
                    </span>
                    <span class="fast mono">
                      {#if p.peakSent}<span title="Fastest send to {name}: {speed(p.peakSent)}">↑ {speed(p.peakSent)}</span>{/if}
                      {#if p.peakSent && p.peakReceived}<span class="sep">·</span>{/if}
                      {#if p.peakReceived}<span title="Fastest from {name}: {speed(p.peakReceived)}">↓ {speed(p.peakReceived)}</span>{/if}
                      {#if !p.peakSent && !p.peakReceived}<span title="{plural(p.count, 'transfer')} with {name}, {bytes(total)} in all">{plural(p.count, "transfer")}</span>{/if}
                    </span>
                  </span>
                </button>
              {/each}
            </div>
          {/if}
        </section>

        <section class="card sources">
          <h2>Where files came from</h2>
          <div class="sub">Everything you got, by source.</div>
          {#if !downloadedTotal}
            <div class="nodata small" in:fadeIn={{ duration: 160 }}>Nothing downloaded or received {when}.</div>
          {:else}
            <div class="share" role="img" aria-label="Share of what you got, by source">
              {#each v.downloadSources.filter((r) => r.bytes) as r (r.route)}
                <i style="flex: {r.bytes}; background: {SOURCE_COLORS[r.route]}" title="{SOURCE_NAMES[r.route]?.[0] ?? r.route}: {bytes(r.bytes)} ({pct(r.bytes, downloadedTotal)}%)"></i>
              {/each}
            </div>
            <div class="slist">
              {#each v.downloadSources.filter((r) => r.bytes) as r (r.route)}
                <div class="srow" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
                  <i style="background: {SOURCE_COLORS[r.route]}"></i>
                  <span class="sname">{SOURCE_NAMES[r.route]?.[0] ?? r.route}</span>
                  <span class="mono dim">{pct(r.bytes, downloadedTotal)}%</span>
                  <span class="mono">{bytes(r.bytes)}</span>
                </div>
              {/each}
            </div>
            {#if v.sites.length}
              <div class="sitehead">Top sites</div>
              <div class="sites">
                {#each v.sites as s (s.site)}
                  <div class="site" title="{s.site}: {plural(s.count, 'download')}, {bytes(s.bytes)}" transition:rowSlide={{ duration: 200 }} animate:flipIf={flipOpts}>
                    <span class="sname">{s.site}</span>
                    <span class="sbar"><i style="width: {(s.bytes / sitesMax) * 100}%"></i></span>
                    <span class="mono">{bytes(s.bytes)}</span>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}
        </section>
      </div>

      <!-- Records -->
      <div class="records" data-tour="dash-records" in:pop={{ duration: 260, from: 0.99, y: 8, delay: 200 }}>
        <div class="record">
          <span class="ricon" style="color: var(--c-down)"><Icon name="bolt" size={16} /></span>
          <span class="k">Fastest download</span>
          {#if v.records.fastest}
            <span class="rv mono"><EaseNum value={v.records.fastest.value} format={speed} /></span>
            <span class="d" title={v.records.fastest.name}>{v.records.fastest.name} · {dayLabel(v.records.fastest.day)}</span>
          {:else}
            <span class="rv dim">—</span><span class="d">No measured download {when}</span>
          {/if}
        </div>
        <div class="record">
          <span class="ricon" style="color: var(--c-recv)"><Icon name="file" size={16} /></span>
          <span class="k">Biggest file</span>
          {#if v.records.biggest}
            <span class="rv mono"><EaseNum value={v.records.biggest.value} format={bytes} /></span>
            <span class="d" title={v.records.biggest.name}>{v.records.biggest.name} · {dayLabel(v.records.biggest.day)}</span>
          {:else}
            <span class="rv dim">—</span><span class="d">No file {when}</span>
          {/if}
        </div>
        <div class="record">
          <span class="ricon" style="color: var(--c-up)"><Icon name="clock" size={16} /></span>
          <span class="k">Busiest day</span>
          {#if v.records.busiest}
            <span class="rv mono"><EaseNum value={v.records.busiest.value} format={bytes} /></span>
            <span class="d">{bucketTitle(v.records.busiest.day, "day")}</span>
          {:else}
            <span class="rv dim">—</span><span class="d">Nothing moved {when}</span>
          {/if}
        </div>
      </div>
    {/if}

    <LiveSpeed />
  </div>
</div>

{#if resetting}
  <Modal title="Reset statistics?" width={440} onclose={() => (resetting = false)}>
    <p class="msg">Every number on the Dashboard starts again from zero, including the speed history. Your files, downloads, uploads and contacts stay as they are. This can't be undone.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (resetting = false)}>Cancel</button>
      <button class="btn danger-fill" onclick={doReset}>Reset</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .dash {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .period {
    display: flex;
  }
  .toolbar {
    height: 52px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 16px;
    border-bottom: 1px solid var(--line);
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .dim {
    color: var(--text-3);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  /* the four totals */
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
  }
  .tile,
  .record {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
    padding: 14px 16px;
    border-radius: var(--radius-lg);
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .k {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--text-3);
  }
  .k i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .v {
    font-size: 22px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.01em;
  }
  .d {
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .d .bad {
    color: var(--st-fail-text);
  }

  .nodata {
    height: 140px;
    display: grid;
    place-items: center;
    font-size: 12.5px;
    color: var(--text-3);
  }
  .nodata.small {
    height: 96px;
  }
  .barp {
    pointer-events: none;
  }
  /* Bars grow up from the axis, one after another; the line draws itself; dots and marks fade in. */
  .grow {
    transform-box: view-box;
    animation: grow-y var(--slow) var(--ease-out) backwards;
  }
  @keyframes grow-y {
    from {
      transform: scaleY(0);
    }
  }
  .draw {
    stroke-dasharray: 1;
    animation: draw var(--slow) var(--ease-out) backwards;
  }
  @keyframes draw {
    from {
      stroke-dashoffset: 1;
    }
  }
  .appear {
    animation: appear var(--slow) var(--ease-out) 120ms backwards;
  }
  @keyframes appear {
    from {
      opacity: 0;
    }
  }
  .tip.glide {
    left: 0;
    transition: transform var(--fast) var(--ease);
  }
  /* Bars that change with the period ease to their new length, and grow in the first time. */
  @keyframes grow-x {
    from {
      transform: scaleX(0);
    }
  }
  .whisker {
    stroke-width: 1.5;
    opacity: 0.4;
    pointer-events: none;
  }
  .top {
    fill: var(--panel);
    stroke: var(--text);
    stroke-width: 2;
  }
  .toplabel {
    fill: var(--text);
    font-size: 11px;
    font-weight: 600;
    font-family: var(--mono);
  }
  .legend .range {
    width: 3px;
    height: 12px;
    border-radius: 2px;
    background: var(--c);
    opacity: 0.45;
  }

  /* side-by-side cards */
  .pair {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
  }
  .pair > .card {
    flex: 1 1 340px;
  }
  .pair.wide > .people {
    flex: 3 1 440px;
  }
  .pair.wide > .sources {
    flex: 2 1 300px;
  }

  /* which way is fastest */
  .rlist {
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin-top: 14px;
  }
  .route {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px 12px;
    align-items: center;
  }
  .rname {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text);
    min-width: 0;
  }
  .badge {
    padding: 1px 7px;
    border-radius: 999px;
    background: rgba(70, 201, 126, 0.14);
    color: var(--st-done-text);
    font-size: 11px;
    font-weight: 600;
  }
  .rvalue {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    text-align: right;
  }
  .track {
    grid-column: 1 / -1;
    position: relative;
    height: 6px;
    border-radius: 3px;
    background: var(--raise);
  }
  .track .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: 3px;
    transform-origin: left;
    animation: grow-x var(--slow) var(--ease-out) backwards;
    transition: width var(--slow) var(--ease);
  }
  .track .peak {
    position: absolute;
    top: -3px;
    width: 2px;
    height: 12px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--text-2);
    transition: left var(--slow) var(--ease);
  }
  .rsub {
    grid-column: 1 / -1;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .route.unused .rname {
    color: var(--text-3);
  }

  /* people */
  .plist {
    display: flex;
    flex-direction: column;
    margin-top: 10px;
  }
  .person {
    display: grid;
    grid-template-columns: 16px 30px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    padding: 8px 8px;
    margin: 0 -8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: inherit;
    text-align: left;
    font: inherit;
    cursor: pointer;
    transition: background var(--fast) ease;
  }
  .person:hover {
    background: var(--raise);
  }
  .person + .person {
    border-top: 1px solid var(--line-soft);
  }
  .rank {
    font-size: 11.5px;
    color: var(--text-3);
    text-align: right;
  }
  .avatar {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--avatar-bg);
    box-shadow: inset 0 0 0 1px var(--avatar-ring);
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }
  .who {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .pname {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pbar {
    display: flex;
    gap: 2px;
    height: 4px;
    transform-origin: left;
    animation: grow-x var(--slow) var(--ease-out) backwards;
  }
  .pbar i {
    border-radius: 2px;
    min-width: 0;
    transition: width var(--slow) var(--ease);
  }
  .amounts {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 3px;
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
  }
  .amounts .fast {
    font-size: 11px;
    color: var(--text-3);
  }
  .amounts b {
    font-weight: 600;
  }
  .amounts b.up {
    color: var(--c-sent-text);
  }
  .amounts b.down {
    color: var(--c-recv-text);
  }
  .sep {
    margin: 0 4px;
    color: var(--text-4);
  }
  .zero {
    color: var(--text-3);
  }
  .zero b {
    opacity: 0.6;
  }

  /* sources */
  .share {
    display: flex;
    gap: 2px;
    height: 10px;
    margin-top: 14px;
    border-radius: 5px;
    overflow: hidden;
    transform-origin: left;
    animation: grow-x var(--slow) var(--ease-out) backwards;
  }
  .share i {
    min-width: 3px;
    transition: flex-grow var(--slow) var(--ease);
  }
  .slist {
    display: flex;
    flex-direction: column;
    gap: 7px;
    margin-top: 12px;
  }
  .srow {
    display: grid;
    grid-template-columns: 10px minmax(0, 1fr) auto 64px;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .srow i {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }
  .srow .mono {
    text-align: right;
  }
  .sname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sitehead {
    margin: 16px 0 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .sites {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .site {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 70px 64px;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .site .mono {
    text-align: right;
  }
  .sbar {
    height: 4px;
    border-radius: 2px;
    background: var(--raise);
  }
  .sbar i {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--c-down);
    transform-origin: left;
    animation: grow-x var(--slow) var(--ease-out) backwards;
    transition: width var(--slow) var(--ease);
  }

  /* records */
  .records {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 10px;
  }
  .record {
    position: relative;
    padding-left: 54px;
  }
  .ricon {
    position: absolute;
    left: 16px;
    top: 16px;
    width: 26px;
    height: 26px;
    border-radius: var(--radius);
    display: grid;
    place-items: center;
    background: var(--raise);
  }
  .rv {
    font-size: 17px;
    font-weight: 600;
    color: var(--text);
  }

  /* nothing counted yet */
  .emptycard {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 40px 24px;
  }
  .emptyicon {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    margin-bottom: 14px;
    background: var(--raise);
    color: var(--text-2);
  }
  .emptycard p {
    max-width: 420px;
    margin: 8px 0 0;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--text-3);
  }
  .msg {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-2);
  }
</style>
