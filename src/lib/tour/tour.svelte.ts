// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Tutorials: a spotlight walks over the window, one bubble at a time.
import { SvelteSet } from "svelte/reactivity";
import { api } from "../api";
import { app, type View } from "../state.svelte";
import { PREPARE, TOURS, type TourId } from "./tours";

export interface Step {
  /** The page this step is on ("settings" opens the Settings window); none = whatever page is shown. */
  view?: View | "settings";
  /** `data-tour` of what to show (several: one spotlight around all of them, e.g. a whole bar);
   *  none = a bubble in the middle of the window. */
  target?: string | string[];
  title: string;
  text: string;
  /** Opens the details panel for this step (closed again when the tour ends, if it was closed). */
  panel?: boolean;
  /** Whether this step is part of the tour, read from what Pako knows (contacts, uploads, your
   *  numbers…) when the tour starts, whatever page is shown: the count is right from the first
   *  bubble. */
  when?: () => boolean;
  /** Dropped unseen if its element never shows up. Only a fallback for a `when` that was wrong:
   *  a step that may not apply says so with `when`. */
  optional?: boolean;
  /** Where the bubble goes; by default whichever side has room. "inside": in the spotlight, near its
   *  bottom (for a big area like a whole list, so the bubble doesn't sit over another part). */
  side?: "right" | "left" | "top" | "bottom" | "inside";
}

export type { TourId };

export const targetsOf = (s: Step): string[] => (s.target === undefined ? [] : Array.isArray(s.target) ? s.target : [s.target]);

/** The first element with this name that's really on screen. */
export function findTarget(name: string): HTMLElement | null {
  for (const e of document.querySelectorAll<HTMLElement>(`[data-tour="${name}"]`)) {
    if (e.closest('[inert], [aria-hidden="true"]')) continue;
    const r = e.getBoundingClientRect();
    if (r.width < 4 || r.height < 4) continue;
    if (e.checkVisibility && !e.checkVisibility({ opacityProperty: true, visibilityProperty: true })) continue;
    return e;
  }
  return null;
}

class Tour {
  active = $state(false);
  id = $state<TourId | null>(null);
  steps = $state<Step[]>([]);
  index = $state(0);
  /** Which way the last move went (so a missing optional step is skipped the same way). */
  dir = $state<1 | -1>(1);
  /** Page tutorials gone through in this session: their Tutorial button stays quiet even if an
   *  older copy of the settings arrives after (the window's record wins until the next start). */
  doneHere = new SvelteSet<TourId>();
  /** Getting ready (the Dashboard's numbers are being asked for): a second click does nothing. */
  private starting = false;

  step = $derived<Step | null>(this.active ? (this.steps[this.index] ?? null) : null);

  /** This page's tutorial has been gone through once. */
  done(id: TourId) {
    return this.doneHere.has(id) || (app.settings?.toursDone ?? []).includes(id);
  }

  /** When the last tutorial was asked for, and when it became active (to measure a slow start). */
  clickedAt = 0;
  activeAt = 0;

  async start(id: TourId) {
    if (this.active || this.starting) return;
    this.clickedAt = performance.now();
    const prepare = PREPARE[id];
    if (prepare) {
      // What some steps depend on (the Dashboard's numbers), asked first; never more than a moment.
      this.starting = true;
      try {
        await Promise.race([prepare().catch(() => {}), new Promise((r) => setTimeout(r, 1500))]);
      } finally {
        this.starting = false;
      }
      if (this.active) return;
    }
    // Direct with no contacts yet: example people, so every step has something to show. The steps
    // are decided with them in place, and if none were left they'd go away again at once, before
    // anything is drawn (nothing between here and `active` waits).
    const examples = app.directExamples(id === "direct");
    const steps = (TOURS[id] ?? []).filter((s) => !s.when || s.when());
    if (!steps.length) {
      if (examples) app.directExamples(false);
      return;
    }
    this.id = id;
    this.steps = steps;
    this.index = 0;
    this.dir = 1;
    this.activeAt = performance.now();
    this.active = true;
  }

  next() {
    if (this.index >= this.steps.length - 1) this.stop();
    else {
      this.dir = 1;
      this.index++;
    }
  }

  back() {
    if (this.index > 0) {
      this.dir = -1;
      this.index--;
    }
  }

  /** A step whose element never showed up (its `when` was wrong): it leaves this tour, so "3 of 8"
   *  stays true. */
  skip() {
    this.steps = this.steps.filter((_, i) => i !== this.index);
    if (!this.steps.length) this.stop();
    else if (this.dir === -1 && this.index > 0) this.index--;
    else if (this.index >= this.steps.length) this.stop();
  }

  stop() {
    if (!this.active) return;
    // A page's tutorial gone through once (to the end, or stopped): its button goes quiet.
    const id = this.id;
    if (id && id !== "full" && !this.done(id)) {
      this.doneHere.add(id);
      if (app.settings) app.settings = { ...app.settings, toursDone: [...(app.settings.toursDone ?? []), id] };
      // Once it's saved, the settings are read again so they agree with the window.
      api.helpSeen({ tourDone: id }).then(
        () => app.refresh(),
        () => {},
      );
    }
    this.active = false;
    this.id = null;
    // The example people go now, under the overlay fading out; the Direct page crossfades to
    // its welcome card.
    app.directExamples(false);
  }
}

export const tour = new Tour();
