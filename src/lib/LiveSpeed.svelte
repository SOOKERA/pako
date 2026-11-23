<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Dashboard › Right now: Pako's download speed over the last hour or day, next to everything
  // the PC received and sent (data/stats.json, sampled every few seconds).
  import { onMount } from "svelte";
  import { api, type StatsView } from "./api";
  import { speed } from "./format";
  import { niceMax } from "./chart";
  import SegPick from "./SegPick.svelte";
  import { fadeIn } from "./motion";
  import { i18n, t } from "./i18n.svelte";

  let stats = $state<StatsView | null>(null);
  let window_ = $state<"hour" | "day">("hour");

  onMount(() => {
    const load = async () => {
      try {
        stats = await api.stats();
      } catch {
        /* keep the last numbers */
      }
    };
    load();
    const timer = setInterval(load, 5000);
    return () => clearInterval(timer);
  });

  const SERIES = $derived([
    { key: 1, label: "Pako", color: "var(--c-down)" },
    { key: 2, label: t("Whole PC · download"), color: "var(--c-pc-down)" },
    { key: 3, label: t("Whole PC · upload"), color: "var(--c-pc-up)" },
  ] as const);
  const points = $derived(window_ === "hour" ? (stats?.recent ?? []) : (stats?.minutes ?? []));
  const step = $derived(window_ === "hour" ? 5 : 60);
  const spanSecs = $derived(window_ === "hour" ? 3600 : 86400);

  let lw = $state(800);
  const LH = 170;
  const pad = { l: 78, r: 14, t: 12, b: 26 };
  const t1 = $derived(points.length ? points[points.length - 1][0] : Math.floor(Date.now() / 1000));
  const t0 = $derived(t1 - spanSecs);
  const maxY = $derived(niceMax(Math.max(1024, ...points.map((p) => Math.max(p[1], p[2], p[3])))));
  const x = (at: number) => pad.l + ((at - t0) / (t1 - t0)) * (lw - pad.l - pad.r);
  const y = (v: number) => pad.t + (1 - v / maxY) * (LH - pad.t - pad.b);

  /** One path per series; a gap where Pako wasn't running breaks the line. */
  function linePath(k: 1 | 2 | 3) {
    let d = "";
    let prev = -Infinity;
    for (const p of points) {
      if (p[0] < t0) continue;
      d += (p[0] - prev > step * 2.5 ? "M" : "L") + x(p[0]).toFixed(1) + " " + y(p[k]).toFixed(1);
      prev = p[0];
    }
    return d;
  }
  function clock(at: number) {
    return new Date(at * 1000).toLocaleTimeString(i18n.dates, { hour: "2-digit", minute: "2-digit" });
  }
  const xTicks = $derived.by(() => {
    const every = window_ === "hour" ? 15 * 60 : 6 * 3600;
    const out: number[] = [];
    for (let at = Math.ceil(t0 / every) * every; at <= t1; at += every) out.push(at);
    return out;
  });

  /** The hovered point, kept by its time: the list is reloaded every 5 seconds. */
  let hover = $state<number | null>(null);
  const hoverPoint = $derived(hover === null ? null : (points.find((p) => p[0] === hover && p[0] >= t0) ?? null));
  function onMove(e: PointerEvent) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect();
    const at = t0 + ((e.clientX - r.left - pad.l) / (lw - pad.l - pad.r)) * (t1 - t0);
    let best = -1;
    let dist = Infinity;
    points.forEach((p, i) => {
      const dd = Math.abs(p[0] - at);
      if (dd < dist) {
        dist = dd;
        best = i;
      }
    });
    hover = best >= 0 && dist <= step * 3 ? points[best][0] : null;
  }
</script>

<section class="card">
  <div class="head">
    <div>
      <h2>{t("Right now")}</h2>
      <div class="sub">{t("How fast Pako downloads, next to everything your PC receives and sends.")}</div>
    </div>
    <SegPick
      small
      label={t("Time window")}
      options={[
        { value: "hour", label: t("Last hour") },
        { value: "day", label: t("Last 24 hours") },
      ]}
      value={window_}
      onpick={(w) => (window_ = w as "hour" | "day")}
    />
  </div>
  <div class="legend">
    {#each SERIES as s (s.key)}
      {#if s.key === 1 || stats?.hasPc}<span><i style="background: {s.color}"></i>{s.label}</span>{/if}
    {/each}
  </div>
  <div class="chart" bind:clientWidth={lw}>
    {#if points.length < 2}
      <div class="nodata" in:fadeIn={{ duration: 160 }}>{t("This fills in while Pako is open. Come back in a minute.")}</div>
    {:else}
      {#key window_}
      <svg width={lw} height={LH} role="img" aria-label={window_ === "hour" ? t("Speed over the last hour") : t("Speed over the last 24 hours")} onpointermove={onMove} onpointerleave={() => (hover = null)} in:fadeIn={{ duration: 160 }}>
        {#each [0, 0.5, 1] as f (f)}
          <line x1={pad.l} x2={lw - pad.r} y1={y(maxY * f)} y2={y(maxY * f)} class="grid" />
          <text x={pad.l - 10} y={y(maxY * f) + 4} class="ax" text-anchor="end">{speed(maxY * f)}</text>
        {/each}
        {#each xTicks as at (at)}
          <text x={x(at)} y={LH - 6} class="ax" text-anchor="middle">{clock(at)}</text>
        {/each}
        {#if stats?.hasPc}
          <path d={linePath(3)} class="ln draw" pathLength="1" style="stroke: var(--c-pc-up)" />
          <path d={linePath(2)} class="ln draw" pathLength="1" style="stroke: var(--c-pc-down)" />
        {/if}
        <path d={linePath(1)} class="ln draw" pathLength="1" style="stroke: var(--c-down)" />
        {#if hoverPoint}
          {@const p = hoverPoint}
          {@const px = x(p[0])}
          <line x1={px} x2={px} y1={pad.t} y2={LH - pad.b} class="cross" />
          {#each SERIES as s (s.key)}
            {#if s.key === 1 || stats?.hasPc}<circle cx={px} cy={y(p[s.key])} r="4" style="fill: {s.color}" class="dot" />{/if}
          {/each}
        {/if}
      </svg>
      {/key}
      {#if hoverPoint}
        {@const p = hoverPoint}
        <div class="tip glide" style="transform: translateX({Math.min(x(p[0]) + 14, lw - 210)}px)" in:fadeIn={{ duration: 120 }}>
          <div class="tt">{clock(p[0])}</div>
          {#each SERIES as s (s.key)}
            {#if s.key === 1 || stats?.hasPc}<div class="tr"><i style="background: {s.color}"></i><span>{s.label}</span><b class="mono">{speed(p[s.key])}</b></div>{/if}
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</section>

<style>
  /* The lines draw themselves when the window changes; the tooltip glides along. */
  .draw {
    stroke-dasharray: 1;
    animation: draw var(--slow) var(--ease-out) backwards;
  }
  @keyframes draw {
    from {
      stroke-dashoffset: 1;
    }
  }
  .tip.glide {
    left: 0;
    transition: transform var(--fast) var(--ease);
  }
  .nodata {
    height: 120px;
    display: grid;
    place-items: center;
    font-size: 12.5px;
    color: var(--text-3);
  }
</style>
