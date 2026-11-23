// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's terms of use are sookera.com's, word for word: this copies the website's terms.md over
// TERMS.md and terms.fr.md over TERMS.fr.md (sookera.com/terms and sookera.com/fr/terms).
// Only what is there for the website goes: its first HTML comment (a note for its editors) and the
// " {#id}" at the end of headings (the page's anchors). Both must carry the same date, the
// version of the terms Pako saves when they're accepted.
//
//   node scripts/copy-terms.mjs [<the website's content folder>]
//
// The folder defaults to the website's repository as the studio keeps it, two folders up from
// Pako in "Sookera Studio": sookera-web/sites/sookera/src/content.
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));
const SITE_CONTENT = resolve(ROOT, "..", "..", "Sookera Studio", "sookera-web", "sites", "sookera", "src", "content");

/** The pairs copied: the website's file, Pako's. */
export const FILES = [
  ["terms.md", "TERMS.md"],
  ["terms.fr.md", "TERMS.fr.md"],
];

const MONTHS_EN = ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];
const MONTHS_FR = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];

/** The website's Markdown as Pako carries it: no leading HTML comment, no " {#id}" on headings,
 *  LF line ends, one line end at the close. Every other character stays. */
export function forPako(md) {
  let text = md.replace(/^\uFEFF/, "").replace(/\r\n/g, "\n");
  const lead = text.match(/^\s*<!--[\s\S]*?-->[ \t]*\n/);
  if (lead) text = text.slice(lead[0].length);
  text = text.replace(/^\s*\n/, "");
  text = text.replace(/^(#{1,6} .*?)[ \t]+\{#[A-Za-z0-9_-]+\}[ \t]*$/gm, "$1");
  return text.trimEnd() + "\n";
}

/** The date of the terms ("2026-11-23"): "Last updated 23 November 2026" in English,
 *  "Dernière mise à jour : 23 novembre 2026" in French (as src-tauri/src/terms.rs reads it). */
export function dateOf(md) {
  for (const raw of md.split(/\r?\n/)) {
    const line = raw.trim().replace(/^[*_]+|[*_]+$/g, "").trim();
    const m = line.match(/^(?:Last updated|Dernière mise à jour)\s*:?\s*(\d{1,2})(?:er)?\s+(\p{L}+)\s+(\d{4})\.?$/iu);
    if (!m) continue;
    const name = m[2].toLowerCase();
    const month = (MONTHS_EN.includes(name) ? MONTHS_EN.indexOf(name) : MONTHS_FR.indexOf(name)) + 1;
    const day = Number(m[1]);
    if (month < 1 || day < 1 || day > 31) continue;
    return `${m[3]}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  }
  return null;
}

function main(from) {
  const made = FILES.map(([site, pako]) => {
    const text = forPako(readFileSync(join(from, site), "utf8"));
    if (text.includes("{#")) throw new Error(`${site}: a "{#" is left after the copy.`);
    // Pako shows the Markdown as it is: a website comment anywhere else would show as text.
    if (text.includes("<!--")) throw new Error(`${site}: an HTML comment is left after the copy (only the first one is the website's note).`);
    const date = dateOf(text);
    if (!date) throw new Error(`${site} has no "Last updated" / "Dernière mise à jour" date.`);
    return { site, pako, text, date };
  });
  const dates = new Set(made.map((m) => m.date));
  if (dates.size !== 1) {
    throw new Error(`The terms don't carry the same date (${made.map((m) => `${m.site}: ${m.date}`).join(", ")}): the French must be in step with the English.`);
  }
  for (const m of made) {
    const out = join(ROOT, m.pako);
    let old = "";
    try {
      old = readFileSync(out, "utf8");
    } catch {}
    writeFileSync(out, m.text);
    console.log(`${m.pako}: ${old.replace(/\r\n/g, "\n") === m.text ? "already the same" : "copied"} from ${m.site} (${m.date}).`);
  }
}

if (import.meta.main ?? process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    main(resolve(process.argv[2] ?? SITE_CONTENT));
  } catch (e) {
    console.error(String(e?.message ?? e));
    process.exit(1);
  }
}
