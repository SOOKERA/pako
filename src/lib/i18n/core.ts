// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The parts of Pako's translation that need no window (tested with Node: test/i18n.test.mjs):
// French typography, placeholders and plurals. The reactive side is ../i18n.svelte.ts.

export type Lang = "en" | "fr";
/** Words that change with a number: `{ one: "{n} file", other: "{n} files" }`. */
export interface Plural {
  one: string;
  other: string;
}
export type Entry = string | Plural;
export type Params = Record<string, string | number>;

/** One of the two languages (anything else, from an older or edited copy: English). */
export function langOf(v: unknown): Lang {
  return v === "fr" ? "fr" : "en";
}

/** French typography, added to the dictionary's plain spaces: a non-breaking space before : and
 *  », after « and before % ("45 %"); a narrow one before ; ? !. (Never inside a placeholder or a URL: "{n}:" or
 *  "https://" have no space before the colon.) */
export function frType(s: string): string {
  return s
    .replace(/ :/g, " :")
    .replace(/ ([;?!])/g, " $1")
    .replace(/« /g, "« ")
    .replace(/ »/g, " »")
    .replace(/([\d}]) ?%/g,"$1 %");
}

/** "{name}" placeholders filled; unknown ones stay as written. */
export function fill(s: string, p?: Params): string {
  if (!p) return s;
  return s.replace(/\{(\w+)\}/g, (m, k: string) => (k in p ? String(p[k]) : m));
}

const rules = new Map<Lang, Intl.PluralRules>();
/** "one" or "other" for `n` in a language (French: 0 and 1 are singular, English: only 1). */
export function pluralOf(lang: Lang, n: number): "one" | "other" {
  let r = rules.get(lang);
  if (!r) rules.set(lang, (r = new Intl.PluralRules(lang === "fr" ? "fr-FR" : "en-GB")));
  return r.select(n) === "one" ? "one" : "other";
}

/** The number a count stands for, whether given raw (3) or already written for the language by
 *  num() ("1,000" in English, "1,5" or "1 000" in French): the plural follows the number, not the
 *  way it's written. */
export function countOf(v: string | number | undefined, lang: Lang): number {
  if (v === undefined) return 0;
  if (typeof v === "number") return v;
  // \s covers the no-break spaces French groups thousands with.
  const s = lang === "fr" ? v.replace(/\s/g, "").replace(",", ".") : v.replace(/[\s,]/g, "");
  return Number(s);
}

/** An entry's words for `params.n` (a plain string has one form), placeholders filled. */
export function words(e: Entry, lang: Lang, params?: Params): string {
  if (typeof e === "string") return fill(e, params);
  const n = countOf(params?.n, lang);
  return fill(pluralOf(lang, n) === "one" ? e.one : e.other, params);
}

/** The placeholders an entry uses ("{n}", "{name}"…), sorted: French and English must match. */
export function placeholders(e: Entry): string[] {
  const all = typeof e === "string" ? e : `${e.one} ${e.other}`;
  return [...new Set(all.match(/\{\w+\}/g) ?? [])].sort();
}
