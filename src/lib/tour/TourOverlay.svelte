<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The tutorial overlay: dims the window, a spotlight glides to each element, a bubble explains it.
  // A step is shown only once its element is on screen and holding still (done sliding, popping in
  // or scrolling): the bubble's text and the spotlight always change together, and a step that
  // can't be shown is dropped unseen.
  import { tick, untrack } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { inApp, listen } from "../api";
  import { app, type View } from "../state.svelte";
  import { fadeIn, pop, reduced } from "../motion";
  import { tour, findTarget, targetsOf, type Step } from "./tour.svelte";

  let {
    settingsOpen,
    onsettings,
  }: {
    /** The Settings window is open (a Settings step needs it). */
    settingsOpen: boolean;
    onsettings: (open: boolean) => void;
  } = $props();

  type Rect = { x: number; y: number; w: number; h: number };
  const PAD = 6;
  const GAP = 14;
  const EDGE = 12;
  /** Pako's own title bar: it stays usable during a tutorial (move, minimise, maximise, close). */
  const TITLE = 40;
  /** A step shows once its spotlight has held still for this many frames… */
  const STILL_FRAMES = 2;
  /** …or this long after its element was found (a panel sliding open, a card popping in)… */
  const SETTLE_MAX = 900;
  /** …and in any case this long after the step began (Next never gets stuck). */
  const SEEK_MAX = 2500;

  let vw = $state(1200);
  let vh = $state(800);
  /** The spotlight (null: no element, a bubble in the middle). */
  let spot = $state<Rect | null>(null);
  /** The step on screen: it changes together with the spotlight, never before it. */
  let shown = $state<{ step: Step; n: number; total: number } | null>(null);
  /** Looking for the next step's element: the bubble dims a little if that takes a moment. */
  let seeking = $state(false);
  let bw = $state(320);
  let bh = $state(160);
  let bubble = $state<HTMLDivElement | null>(null);
  /** Placed once: the very first position doesn't glide in from a corner. */
  let placed = $state(false);

  // What the tour changed, put back when it ends.
  let panelBefore: boolean | null = null;
  let settingsByTour = false;

  /** The step's elements, each with the scrolling boxes around it. */
  let els: { e: HTMLElement; clips: HTMLElement[] }[] = [];
  let seekStart = 0;
  let seekFor = -1;
  /** When the step's element was first measured on screen (0: not yet). */
  let foundAt = 0;
  /** A scroll was started for this step: the step waits for it (up to this time) to show. */
  let settleUntil = 0;
  /** Not before this time (a scroll that was just started hasn't moved anything yet). */
  let holdUntil = 0;
  let lastSeen: Rect | null = null;
  /** Frames in a row the spotlight hasn't moved. */
  let still = 0;
  let jumped = false;
  let frame = 0;

  /** Scrolling boxes around the element: the spotlight stays inside what they show. */
  function clippers(e: HTMLElement): HTMLElement[] {
    const out: HTMLElement[] = [];
    for (let p = e.parentElement; p && p !== document.body; p = p.parentElement) {
      const s = getComputedStyle(p);
      if (/(auto|scroll|hidden)/.test(s.overflowY + s.overflowX)) out.push(p);
    }
    return out;
  }

  /** One element, padded and cut to what its scrolling boxes show (null: scrolled out of sight). */
  function measureOne(e: HTMLElement, clips: HTMLElement[]): Rect | null {
    const r = e.getBoundingClientRect();
    let l = r.left - PAD;
    let t = r.top - PAD;
    let rr = r.right + PAD;
    let b = r.bottom + PAD;
    for (const c of clips) {
      const cr = c.getBoundingClientRect();
      l = Math.max(l, cr.left);
      t = Math.max(t, cr.top);
      rr = Math.min(rr, cr.right);
      b = Math.min(b, cr.bottom);
    }
    if (rr - l < 1 || b - t < 1) return null;
    return { x: l, y: t, w: rr - l, h: b - t };
  }

  /** One spotlight around all of the step's elements. */
  function measure(): Rect | null {
    let u: { l: number; t: number; r: number; b: number } | null = null;
    for (const { e, clips } of els) {
      const m = measureOne(e, clips);
      if (!m) continue;
      u = u
        ? { l: Math.min(u.l, m.x), t: Math.min(u.t, m.y), r: Math.max(u.r, m.x + m.w), b: Math.max(u.b, m.y + m.h) }
        : { l: m.x, t: m.y, r: m.x + m.w, b: m.y + m.h };
    }
    if (!u) return null;
    const l = Math.max(u.l, 2);
    const t = Math.max(u.t, 2);
    const r = Math.min(u.r, vw - 2);
    const b = Math.min(u.b, vh - 2);
    return { x: l, y: t, w: Math.max(0, r - l), h: Math.max(0, b - t) };
  }

  const same = (a: Rect | null, b: Rect | null) =>
    !!a && !!b && Math.abs(a.x - b.x) < 0.5 && Math.abs(a.y - b.y) < 0.5 && Math.abs(a.w - b.w) < 0.5 && Math.abs(a.h - b.h) < 0.5;

  /** The step goes on screen: text, count and spotlight at once. */
  function reveal(s: Step, i: number, at: Rect | null) {
    spot = at;
    shown = { step: s, n: i + 1, total: tour.steps.length };
    seeking = false;
    tick().then(() => bubble?.querySelector<HTMLElement>(".next")?.focus({ preventScroll: true }));
  }

  /** The step's page, the Settings window and the details panel as the step needs them. Also puts
   *  them back when something outside the tour changed them (a notification or the tray opening
   *  another page). */
  function place(s: Step) {
    if (s.view === "settings") {
      if (!settingsOpen) {
        settingsByTour = true;
        onsettings(true);
      }
    } else {
      if (settingsByTour) {
        settingsByTour = false;
        onsettings(false);
      }
      if (s.view && app.view !== s.view) app.view = s.view as View;
    }
    if (s.panel && !app.panelOpen) app.panelOpen = true;
  }

  // ---------- each step: the right page, then wait for its element ----------
  $effect(() => {
    const s = tour.step;
    const i = tour.index;
    if (s) untrack(() => go(s, i));
  });
  function go(s: Step, i: number) {
    if (panelBefore === null) panelBefore = app.panelOpen;
    place(s);
    els = [];
    lastSeen = null;
    still = 0;
    foundAt = 0;
    settleUntil = 0;
    holdUntil = 0;
    jumped = false;
    seekFor = i;
    seekStart = performance.now();
    if (!targetsOf(s).length) reveal(s, i, null);
    else seeking = true;
  }

  // One loop while the tour runs: finds the step's elements, shows the step once they hold still,
  // then follows them (panels slide, lists scroll).
  $effect(() => {
    if (!tour.active) return;
    const loop = () => {
      frame = requestAnimationFrame(loop);
      const s = tour.step;
      if (!s || seekFor !== tour.index) return;
      const names = targetsOf(s);
      if (!names.length) return;
      const now = performance.now();
      const waiting = shown?.step !== s;
      // On screen, but the page was changed under it: put the step's page back.
      if (!waiting) place(s);
      if (!els.length || els.some((x) => !x.e.isConnected)) {
        const found = names.map(findTarget).filter((e): e is HTMLElement => !!e);
        if (!found.length) {
          // Pages fade in, so give them a moment; an optional step that never shows is dropped
          // unseen, any other one gets its bubble in the middle. A step already on screen keeps
          // its bubble and looks again (its page is coming back).
          if (waiting && now - seekStart > (s.optional ? 700 : 1500)) {
            if (s.optional) tour.skip();
            else reveal(s, tour.index, null);
          }
          return;
        }
        els = found.map((e) => ({ e, clips: clippers(e) }));
        if (!waiting) return;
        // Out of sight in a scrolling list: scroll there first, then show the step.
        const box = els[0].clips[0]?.getBoundingClientRect();
        if (box) {
          const rs = found.map((e) => e.getBoundingClientRect());
          const top = Math.min(...rs.map((r) => r.top));
          const bottom = Math.max(...rs.map((r) => r.bottom));
          if (!settleUntil && (top < box.top || bottom > box.bottom)) {
            const tall = bottom - top > box.height || s.view === "settings";
            const smooth = !reduced();
            found[0].scrollIntoView({ block: tall ? "start" : "nearest", behavior: smooth ? "smooth" : "auto" });
            settleUntil = now + (smooth ? 700 : 0);
            holdUntil = now + (smooth ? 120 : 0);
            still = 0;
          }
        }
      }
      const next = measure();
      if (!waiting) {
        if (next && !same(next, spot)) spot = next;
        return;
      }
      // Never stuck: whatever happens, a step shows within a few seconds (Next always moves on).
      if (now - seekStart > SEEK_MAX) {
        reveal(s, tour.index, next);
        return;
      }
      if (!next) {
        // Nothing of it shows yet (a panel still closed, scrolled out of sight): keep waiting.
        // Once a smooth scroll had its time, jump the rest of the way.
        still = 0;
        lastSeen = null;
        if (settleUntil && now >= settleUntil && !jumped) {
          jumped = true;
          els[0].e.scrollIntoView({ block: "nearest", behavior: "auto" });
        }
        return;
      }
      if (!foundAt) foundAt = now;
      still = same(next, lastSeen) ? still + 1 : 0;
      lastSeen = next;
      // One rule for everything that moves (a panel sliding open, a card popping in, a list
      // scrolling): show it once its spotlight holds still, or after a short while at most.
      const settled = still >= STILL_FRAMES && now >= holdUntil;
      if (settled || now >= Math.max(foundAt + SETTLE_MAX, settleUntil)) reveal(s, tour.index, next);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  });

  // ---------- the end: put back what the tour opened ----------
  $effect(() => {
    if (tour.active) return;
    if (settingsByTour) {
      settingsByTour = false;
      onsettings(false);
    }
    if (panelBefore === false) app.panelOpen = false;
    panelBefore = null;
    spot = null;
    shown = null;
    seeking = false;
    placed = false;
    els = [];
  });

  // Closing the window (hidden in the tray, or quitting) or a quit question ends the tutorial,
  // so nothing is left on screen over it.
  $effect(() => {
    if (!tour.active || !inApp) return;
    const offs = [getCurrentWindow().listen("tauri://close-requested", () => tour.stop()), listen("confirm-quit", () => tour.stop())];
    return () => {
      for (const off of offs) off.then((f) => f()).catch(() => {});
    };
  });

  // ---------- the bubble's place ----------
  const pos = $derived.by(() => {
    const s = shown?.step;
    if (!spot) return { x: (vw - bw) / 2, y: (vh - bh) / 2 };
    const r = spot;
    // Never over the title bar's window buttons.
    const top = TITLE + EDGE / 2;
    const clampX = (x: number) => Math.min(Math.max(x, EDGE), vw - bw - EDGE);
    const clampY = (y: number) => Math.min(Math.max(y, top), vh - bh - EDGE);
    if (s?.side === "inside" && r.w >= bw + 2 * EDGE && r.h >= bh + 2 * EDGE) {
      return { x: clampX(r.x + r.w / 2 - bw / 2), y: clampY(r.y + r.h - bh - 3 * EDGE) };
    }
    const fits = {
      right: r.x + r.w + GAP + bw <= vw - EDGE,
      left: r.x - GAP - bw >= EDGE,
      bottom: r.y + r.h + GAP + bh <= vh - EDGE,
      top: r.y - GAP - bh >= top,
    };
    const order: ("right" | "left" | "bottom" | "top")[] = [...(s?.side && s.side !== "inside" ? [s.side] : []), "right", "left", "bottom", "top"];
    const side = order.find((o) => fits[o]);
    if (side === "right") return { x: r.x + r.w + GAP, y: clampY(r.y + r.h / 2 - bh / 2) };
    if (side === "left") return { x: r.x - GAP - bw, y: clampY(r.y + r.h / 2 - bh / 2) };
    if (side === "bottom") return { x: clampX(r.x + r.w / 2 - bw / 2), y: r.y + r.h + GAP };
    if (side === "top") return { x: clampX(r.x + r.w / 2 - bw / 2), y: r.y - GAP - bh };
    // A big element (a whole list): the side with the most room, overlapping the edge a little.
    const room = { right: vw - (r.x + r.w), left: r.x, bottom: vh - (r.y + r.h), top: r.y - TITLE };
    const best = (Object.keys(room) as (keyof typeof room)[]).sort((a, b) => room[b] - room[a])[0];
    if (best === "right") return { x: clampX(r.x + r.w + GAP), y: clampY(r.y + r.h / 2 - bh / 2) };
    if (best === "left") return { x: clampX(r.x - GAP - bw), y: clampY(r.y + r.h / 2 - bh / 2) };
    if (best === "bottom") return { x: clampX(r.x + r.w / 2 - bw / 2), y: clampY(r.y + r.h + GAP) };
    return { x: clampX(r.x + r.w / 2 - bw / 2), y: clampY(r.y - GAP - bh) };
  });
  $effect(() => {
    if (shown && !placed) requestAnimationFrame(() => (placed = true));
  });

  // ---------- keys: → next, ← back, Esc stops; nothing else reaches the window behind ----------
  function key(e: KeyboardEvent) {
    if (!tour.active) return;
    if (e.key === "Tab") {
      // Tab stays in the bubble.
      const items = [...(bubble?.querySelectorAll<HTMLElement>("button") ?? [])];
      if (!items.length) return;
      const at = items.indexOf(document.activeElement as HTMLElement);
      e.preventDefault();
      e.stopPropagation();
      items[(at + (e.shiftKey ? -1 : 1) + items.length) % items.length].focus();
      return;
    }
    e.stopPropagation();
    // A held key repeats: one step per press.
    if (e.repeat && (e.key === "ArrowRight" || e.key === "ArrowLeft" || e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      return;
    }
    if (e.key === "ArrowRight") {
      e.preventDefault();
      next();
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      back();
    } else if (e.key === "Escape") {
      e.preventDefault();
      tour.stop();
    } else if (e.key === "Enter" || e.key === " ") {
      // Enter and Space press the focused bubble button (Next by default).
      const f = document.activeElement as HTMLElement | null;
      e.preventDefault();
      if (f && bubble?.contains(f) && f.tagName === "BUTTON") f.click();
      else next();
    } else if (e.ctrlKey || e.metaKey || e.key.length === 1 || e.key === "Delete") {
      e.preventDefault();
    }
  }

  // ---------- the title bar: moving the window and its buttons work, nothing else up there ----------
  function titleBar(e: MouseEvent) {
    if (!tour.active || e.clientY >= TITLE) return;
    const t = e.target as Element | null;
    // The bubble's own buttons (Enter presses them: a click without a mouse position).
    if (t?.closest?.(".tour")) return;
    // An empty part of the bar (it moves the window) or a window button: let it through.
    if (t?.hasAttribute?.("data-tauri-drag-region") || t?.closest?.("button.win")) return;
    e.preventDefault();
    e.stopPropagation();
  }

  // ---------- drops: nothing dropped on the window while a tutorial runs ----------
  function noDrop(e: DragEvent) {
    if (!tour.active) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "none";
  }

  // Next and Back move from the step on screen, so a second press while the next one is still
  // being found doesn't jump over a step nobody saw.
  function next() {
    if (seeking) return;
    tour.next();
  }
  function back() {
    if (seeking) return;
    tour.back();
  }

  const last = $derived(!!shown && shown.n === shown.total);
</script>

<svelte:window
  bind:innerWidth={vw}
  bind:innerHeight={vh}
  onkeydowncapture={key}
  onpointerdowncapture={titleBar}
  onmousedowncapture={titleBar}
  onclickcapture={titleBar}
  ondblclickcapture={titleBar}
  onauxclickcapture={titleBar}
  oncontextmenucapture={titleBar}
  ondragentercapture={noDrop}
  ondragovercapture={noDrop}
  ondropcapture={noDrop}
/>

{#if tour.active}
  <div class="tour" transition:fadeIn={{ duration: 200 }}>
    <!-- Clicks on the dimmed window do nothing: the tour shows, it doesn't press anything. The
         title bar above it stays free, to move, minimise or close the window. -->
    <div class="catch" role="presentation"></div>
    <!-- No element: a 2 px spot under the bubble (an outline isn't drawn around nothing). -->
    <div
      class="spot"
      class:none={!spot}
      class:glide={placed}
      style="transform: translate({spot ? spot.x : vw / 2 - 1}px, {spot ? spot.y : vh / 2 - 1}px); width: {spot?.w ?? 2}px; height: {spot?.h ?? 2}px"
      aria-hidden="true"
    ></div>
    {#if shown}
      {@const s = shown.step}
      <div
        class="bubble"
        class:glide={placed}
        class:wait={seeking}
        role="dialog"
        aria-modal="true"
        aria-label="Tutorial: {s.title}"
        bind:this={bubble}
        bind:offsetWidth={bw}
        bind:offsetHeight={bh}
        style="transform: translate({Math.round(pos.x)}px, {Math.round(pos.y)}px)"
      >
        <div class="inner" in:pop={{ duration: 260, from: 0.97, y: 6 }}>
          {#key s}
            <div class="content" in:fadeIn={{ duration: 180 }}>
              <div class="count mono">{shown.n} of {shown.total}</div>
              <h3>{s.title}</h3>
              <p>{s.text}</p>
            </div>
          {/key}
          <div class="bar">
            <button class="btn ghost small skip" onclick={() => tour.stop()} title="Stop the tutorial (Esc)">{last ? "Close" : "Skip"}</button>
            <span class="grow"></span>
            {#if shown.n > 1}
              <button class="btn small" onclick={back} title="Back (←)">Back</button>
            {/if}
            <button class="btn primary small next" onclick={next} title={last ? "Finish (Enter)" : "Next (→)"}>{last ? "Done" : "Next"}</button>
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .tour {
    position: fixed;
    inset: 0;
    z-index: 90;
    pointer-events: none;
  }
  .catch {
    position: absolute;
    inset: 40px 0 0;
    pointer-events: auto;
  }
  .spot {
    position: fixed;
    left: 0;
    top: 0;
    border-radius: var(--radius-lg);
    pointer-events: none;
    /* The dim is an outline (it follows the corners): a huge box-shadow isn't painted by WebView2. */
    outline: 200vmax solid var(--tour-dim);
    box-shadow:
      0 0 0 1.5px rgba(255, 255, 255, 0.28),
      0 0 0 6px rgba(249, 98, 99, 0.12);
    transition: box-shadow var(--slow) var(--ease);
    will-change: transform;
  }
  .spot.glide {
    transition:
      transform 420ms var(--ease),
      width 420ms var(--ease),
      height 420ms var(--ease),
      box-shadow var(--slow) var(--ease);
  }
  .spot.none {
    box-shadow:
      0 0 0 0 transparent,
      0 0 0 0 transparent;
  }
  .bubble {
    position: fixed;
    left: 0;
    top: 0;
    width: 300px;
    pointer-events: auto;
    will-change: transform;
  }
  .bubble.glide {
    transition: transform 420ms var(--ease);
  }
  .inner {
    padding: 14px 16px 12px;
    background: var(--panel);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    box-shadow:
      0 18px 48px rgba(0, 0, 0, 0.5),
      0 2px 8px rgba(0, 0, 0, 0.3);
    transition: opacity var(--fast) ease;
  }
  :global(:root[data-theme="light"]) .inner {
    background: var(--bg);
    box-shadow: var(--shadow-modal);
  }
  /* Finding the next element takes a moment (a page fading in, a list scrolling): dim a little,
     only if it lasts, so quick steps don't flicker. */
  .bubble.wait .inner {
    opacity: 0.6;
    transition-delay: 150ms;
  }
  .count {
    font-size: 11px;
    color: var(--text-4);
    margin-bottom: 6px;
  }
  h3 {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  p {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 14px;
  }
  .grow {
    flex: 1;
  }
  .btn.small {
    height: 28px;
    padding: 0 12px;
    font-size: 12px;
  }
  .skip {
    margin-left: -8px;
  }
</style>
