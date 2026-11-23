// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's words in English or French, in every window (Pako's window, the tray list, pop-ups,
// Download with Pako, important notices).
//
// A string is written in English where it is used, `t("Pause all")`, and that English is its key:
// the French dictionary (./i18n/fr.ts) must have every key used, so a missing translation is a
// type error. Placeholders: t("{n} files", { n }). Plurals: an entry can be { one, other },
// picked by `n`; English plurals live in ./i18n/en.ts.
//
// The language is asked, never guessed (Pako Setup, first-start setup, Settings › Window ›
// Look › Language) and saved by Rust (src-tauri/src/i18n.rs). Each window keeps a copy in its storage
// and reads it before its first paint, so it opens in the right language; a change fades the
// words out and in, in every window at once ("language-changed"). Anything that calls t() in a
// template or a $derived updates live.
import { api, inApp, listen } from "./api";
import { reduced } from "./motion";
import { useWords } from "./format.ts";
import { fr } from "./i18n/fr.ts";
import { en } from "./i18n/en.ts";
import { frType, langOf, words as entryWords, type Entry, type Lang, type Params } from "./i18n/core.ts";

export type { Lang } from "./i18n/core.ts";
export type Key = keyof typeof fr;

/** The language choice (first-start setup, Settings): each language named in its own words, so
 *  anyone finds theirs whatever Pako shows now. */
export const LANGUAGES: { value: Lang; label: string }[] = [
  { value: "en", label: "English" },
  { value: "fr", label: "Français" },
];

const KEY = "pako.lang";

function stored(): Lang {
  try {
    return langOf(localStorage.getItem(KEY));
  } catch {
    return "en";
  }
}

class Lingo {
  /** The language shown. */
  lang = $state<Lang>(typeof window === "undefined" ? "en" : stored());
  /** For Intl numbers and lists ("1.5" / "1,5", "a, b and c"). */
  locale = $derived(this.lang === "fr" ? "fr-FR" : "en-GB");
  /** For dates and times: English keeps Windows' own way (as before French came), French its
   *  own (24-hour). Every toLocale…String / DateTimeFormat takes this, never `locale`. */
  dates = $derived<string[]>(this.lang === "fr" ? ["fr-FR"] : []);

  #started = false;

  /** Once per window: shows the saved language and follows its changes. */
  start() {
    if (this.#started) return;
    this.#started = true;
    document.documentElement.lang = this.lang;
    if (!inApp) return;
    listen<string>("language-changed", (e) => void switchTo(langOf(e.payload)));
    // The saved one (a window opened before a change, or storage cleared): at once, no fade.
    api.languageGet().then((l) => keep(langOf(l)), () => {});
  }

  /** Picks a language: every window fades to it, Rust's words and the tray follow, it's saved. */
  async set(lang: Lang) {
    if (!inApp) return switchTo(lang);
    await api.languageSet(lang);
  }

  /** Shows a language in this window only, without saving it (Settings, before Save or Cancel).
   *  Not kept for the next start either: Pako closing meanwhile opens in the saved language. */
  preview(lang: Lang) {
    return switchTo(lang, false);
  }
}
export const i18n = new Lingo();

/** Takes a language without fading (the saved one, at start); `store`: this window opens in it
 *  next time too. */
function keep(lang: Lang, store = true) {
  i18n.lang = lang;
  document.documentElement.lang = lang;
  if (!store) return;
  try {
    localStorage.setItem(KEY, lang);
  } catch {
    // Private storage unavailable: the next start shows English first, then the saved language.
  }
}

// ---- Words ----

const frCache = new Map<string, Entry>();
function french(key: Key): Entry {
  let e = frCache.get(key);
  if (e === undefined) {
    const raw = fr[key] as Entry;
    e = typeof raw === "string" ? frType(raw) : { one: frType(raw.one), other: frType(raw.other) };
    frCache.set(key, e);
  }
  return e;
}

/** The words for `key` in the language shown, placeholders filled. */
export function t(key: Key, params?: Params): string {
  return tIn(i18n.lang, key, params);
}

/** The words for `key` in a given language (the language choice names each in its own words). */
export function tIn(lang: Lang, key: Key, params?: Params): string {
  const e: Entry = lang === "fr" ? french(key) : (en[key] ?? key);
  return entryWords(e, lang, params);
}

/** A string that may or may not be one of ours (a name Pako gave vs. one the user typed): ours
 *  are translated, anything else stays as written. */
export function tMaybe(s: string): string {
  if (i18n.lang === "en" || !(s in fr)) return s;
  return t(s as Key);
}
export function isKey(s: string): s is Key {
  return s in fr;
}

// format.ts and alert.ts (sizes, times, statuses, how a download tells you) speak through this
// window's t(): handed in here, once, so every window that shows words (the main one, the tray
// list, pop-ups, Download with Pako) has them in its language.
useWords(t, () => i18n.lang);

/** Words on screen that may outlive a language switch (a toast, a status line kept in state):
 *  a function gives them again in the language of the moment. */
export type Words = string | (() => string);
export const said = (w: Words): string => (typeof w === "function" ? w() : w);

/** A sentence around a mark ("Click {button} to…") as [before, after], to put something styled
 *  (a button's name, a link) in the middle. */
export function around(s: string, mark: string): [string, string] {
  const i = s.indexOf(mark);
  return i < 0 ? [s, ""] : [s.slice(0, i), s.slice(i + mark.length)];
}

/** A list joined the way each language does ("a, b and c" / "a, b et c"). */
export function list(items: string[]): string {
  return new Intl.ListFormat(i18n.locale, { type: "conjunction" }).format(items);
}

/** A number the way the language writes it ("1.5" / "1,5"). */
export function num(n: number, digits = 0): string {
  return new Intl.NumberFormat(i18n.locale, { minimumFractionDigits: digits, maximumFractionDigits: digits }).format(n);
}

// ---- Switching, live and smooth ----

let switching: Promise<void> = Promise.resolve();

/** Every window: the words on screen fade out, change, and fade back in (only the text fades;
 *  backgrounds, sizes and places stay as they are; never both languages at once). One switch
 *  after the other, so a quick second pick never cuts the first one's fade short. */
function switchTo(lang: Lang, store = true): Promise<void> {
  const run = switching.then(() => switchNow(lang, store));
  switching = run.catch(() => {});
  return run;
}

async function switchNow(lang: Lang, store: boolean) {
  if (i18n.lang === lang || typeof document === "undefined") {
    keep(lang, store);
    return;
  }
  // A window in the background (a pop-up, the hidden tray list) or reduce motion: at once.
  if (reduced() || document.visibilityState !== "visible") {
    keep(lang, store);
    return;
  }
  const root = document.documentElement;
  root.classList.add("lang-swap");
  // A window hidden mid-fade gets no frames (and its animations may stall): never wait on them
  // longer than a moment, so the switch, and the one queued after it, always finish.
  const atMost = (p: Promise<unknown>, ms: number) => Promise.race([p, new Promise((r) => setTimeout(r, ms))]);
  const fade = (from: number, to: number, ms: number) =>
    atMost(
      root
        .animate([{ "--lang-a": from } as Keyframe, { "--lang-a": to } as Keyframe], { duration: ms, easing: "ease-in-out", fill: "forwards" })
        .finished.catch(() => {}),
      ms + 50,
    );
  await fade(1, 0, 120);
  keep(lang, store);
  // Two frames: Svelte has written the new words (tooltips too: they're title attributes) and
  // the browser has laid them out.
  await atMost(new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))), 50);
  await fade(0, 1, 200);
  for (const a of root.getAnimations()) {
    const kf = (a.effect as KeyframeEffect | null)?.getKeyframes?.()[0] as Record<string, unknown> | undefined;
    if (kf && "--lang-a" in kf) a.cancel();
  }
  root.classList.remove("lang-swap");
}
