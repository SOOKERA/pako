// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Light or dark, in every window (Pako's window, the small list, pop-ups, Download with Pako).
//
// The choice is saved in the settings ("dark", "light" or "system" = follow Windows' app mode,
// src-tauri/src/theme.rs). Each window also keeps a copy in its storage, which the page reads
// before its first paint (src/app.html), so it opens in the right colours without a flash.
// Changing it fades the old colours into the new ones (a view transition, see app.css), never
// with reduce motion.
import { api, inApp, listen, type ThemeChoice } from "./api";
import { reduced } from "./motion";

const KEY = "pako.theme";
const lightQuery = () => window.matchMedia("(prefers-color-scheme: light)");

function known(v: unknown): ThemeChoice {
  return v === "light" || v === "system" ? v : "dark";
}
function stored(): ThemeChoice {
  try {
    return known(localStorage.getItem(KEY));
  } catch {
    return "dark";
  }
}

class Theme {
  /** What was chosen. */
  choice = $state<ThemeChoice>(typeof window === "undefined" ? "dark" : stored());
  /** Windows' app mode, for "Match Windows". */
  windowsLight = $state(typeof window !== "undefined" && lightQuery().matches);
  /** What shows. */
  shown = $derived<"light" | "dark">(this.choice === "system" ? (this.windowsLight ? "light" : "dark") : this.choice);

  #started = false;
  #painted: "light" | "dark" | null = null;

  /** Once per window: shows the saved choice and follows its changes. */
  start() {
    if (this.#started) return;
    this.#started = true;
    this.#paint(false);
    const q = lightQuery();
    q.addEventListener("change", (e) => {
      this.windowsLight = e.matches;
      if (this.choice === "system") this.#paint(true);
    });
    if (!inApp) return;
    listen<ThemeChoice>("theme-changed", (e) => this.#take(known(e.payload), true));
    api.themeGet().then((t) => this.#take(known(t), false), () => {});
  }

  /** Shows a choice without saving it (Settings, before Save or Cancel). Not kept for the next
   *  start either: Pako closing meanwhile opens next time in the saved choice, without a flash. */
  preview(choice: ThemeChoice) {
    this.#take(choice, true, false);
  }

  /** Shows and saves a choice: every window follows. */
  async set(choice: ThemeChoice) {
    this.#take(choice, true);
    if (inApp) await api.themeSet(choice);
  }

  /** The title bar's sun/moon: the opposite of what shows (from "Match Windows" too). */
  toggle() {
    return this.set(this.shown === "light" ? "dark" : "light");
  }

  #take(choice: ThemeChoice, animate: boolean, keep = true) {
    this.choice = choice;
    if (keep) {
      try {
        localStorage.setItem(KEY, choice);
      } catch {
        // Private storage unavailable: the next start shows dark first, then the saved choice.
      }
    }
    this.#paint(animate);
  }

  #paint(animate: boolean) {
    const shown = this.shown;
    const root = document.documentElement;
    if (root.dataset.theme !== shown || root.style.colorScheme !== shown) {
      const apply = () => {
        root.dataset.theme = shown;
        root.style.colorScheme = shown;
      };
      // A window in the background (pop-ups, the hidden tray list) switches at once.
      if (animate && !reduced() && document.visibilityState === "visible" && document.startViewTransition) {
        document.startViewTransition(apply);
      } else {
        apply();
      }
    }
    // From here the page's own colours (app.css) paint the background.
    root.style.removeProperty("background");
    if (inApp && this.#painted !== shown) {
      this.#painted = shown;
      api.themeShown(shown === "light").catch(() => {});
    }
  }
}

export const theme = new Theme();
