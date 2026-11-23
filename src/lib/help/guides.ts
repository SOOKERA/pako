// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The guides and What's new made from their Markdown, in English or in a translation that may be
// partial (docs/help/fr/): whatever isn't translated yet shows in English. No window needed, so
// it's tested with Node (test/help-guides.test.mjs); ./content.ts gives it the bundled files.
import { parse, plain, type Block } from "./markdown.ts";

export interface Section {
  id: string;
  title: string;
  level: number;
  /** The section's words, for search. */
  text: string;
}

export interface Guide {
  /** File name, e.g. "downloading.md" (the same in every language). */
  file: string;
  title: string;
  /** One line from the Help index. */
  about: string;
  blocks: Block[];
  sections: Section[];
}

export interface WhatsNew {
  /** The section's heading: a version ("0.2.0") or "Next version" before the release (the same
   *  headings in every language). */
  version: string;
  blocks: Block[];
}

/** A guide folder's files by name ("README.md", "downloading.md"…). */
export type Files = Map<string, string>;

function sectionsOf(blocks: Block[]): Section[] {
  const out: Section[] = [];
  let cur: Section | null = null;
  for (const b of blocks) {
    if (b.kind === "heading" && b.level >= 2) {
      cur = { id: b.id, title: b.text, level: b.level, text: b.text };
      out.push(cur);
    } else if (cur && b.kind === "para") cur.text += " " + b.inline.map((x) => x.text).join("");
    else if (cur && b.kind === "list") cur.text += " " + b.items.map((it) => it.map((x) => x.text).join("")).join(" ");
  }
  return out;
}

/** The Help index's lines (README.md): `1. [Title](file.md): what it's about.` (French puts a
 *  space before the colon). */
function indexOf(readme: string): { title: string; file: string; about: string }[] {
  return [...readme.matchAll(/^\s*\d+\.\s+\[([^\]]+)\]\(([^)]+\.md)\)\s*:?\s*(.*)$/gm)].map((m) => ({ title: m[1], file: m[2], about: m[3] }));
}

/** The guides in the order of the English index (docs/help/README.md). With a translation
 *  (`local`), each guide and its line in the index are the translated ones where they exist. */
export function buildGuides(en: Files, local?: Files): Guide[] {
  const localAbout = new Map(indexOf(local?.get("README.md") ?? "").map((e) => [e.file, e]));
  const out: Guide[] = [];
  for (const e of indexOf(en.get("README.md") ?? "")) {
    const text = local?.get(e.file) ?? en.get(e.file);
    if (!text) continue;
    const line = localAbout.get(e.file) ?? e;
    const blocks = parse(text);
    const h1 = blocks.find((b) => b.kind === "heading" && b.level === 1);
    out.push({
      file: e.file,
      title: h1 && h1.kind === "heading" ? h1.text : line.title,
      about: plain(line.about)
        .replace(/\.$/, "")
        .replace(/^./, (c) => c.toUpperCase()),
      // The title is shown by the Help panel itself.
      blocks: blocks.filter((b) => b !== h1),
      sections: sectionsOf(blocks),
    });
  }
  return out;
}

/** whats-new.md, one entry per "## <version>" section, newest first. */
export function whatsNewOf(md: string): WhatsNew[] {
  const out: WhatsNew[] = [];
  for (const b of parse(md)) {
    if (b.kind === "heading" && b.level === 1) continue;
    if (b.kind === "heading" && b.level === 2) out.push({ version: b.text, blocks: [] });
    else out[out.length - 1]?.blocks.push(b);
  }
  return out;
}

/** What's new in the English order; each version in the translation's words when it has that
 *  section (with something in it, or the English has nothing either), in English otherwise. */
export function buildWhatsNew(en: string, local?: string): WhatsNew[] {
  const english = whatsNewOf(en);
  if (!local) return english;
  const translated = new Map(whatsNewOf(local).map((w) => [w.version, w]));
  return english.map((w) => {
    const l = translated.get(w.version);
    return l && (l.blocks.length || !w.blocks.length) ? l : w;
  });
}

export interface Hit {
  guide: Guide;
  section: Section | null;
  /** A few words around the match. */
  snippet: string;
}

/** Lower case without accents and with one kind of apostrophe, the same length as `s` (so a
 *  place found in it is the same place in `s`): "Télécharger l’ID" → "telecharger l'id". */
export function fold(s: string): string {
  return s
    .toLowerCase()
    .replace(/[À-ɏ]/g, (c) => c.normalize("NFD").replace(/\p{M}/gu, "") || c)
    .replace(/[‘’ʼ]/g, "'");
}

/** Search the guides: sections whose title or words have all the words typed (accents and
 *  apostrophes don't matter). Titles first. */
export function searchGuides(guides: Guide[], query: string): Hit[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  const has = (s: string) => {
    const l = fold(s);
    return words.every((w) => l.includes(w));
  };
  const hits: (Hit & { score: number })[] = [];
  for (const g of guides) {
    if (has(g.title + " " + g.about)) hits.push({ guide: g, section: null, snippet: g.about, score: 3 });
    for (const s of g.sections) {
      if (!has(s.text)) continue;
      const inTitle = has(s.title);
      hits.push({ guide: g, section: s, snippet: snippet(s.text.slice(s.title.length).trim(), words[0]), score: inTitle ? 2 : 1 });
    }
  }
  return hits.sort((a, b) => b.score - a.score).slice(0, 30);
}

function snippet(text: string, word: string): string {
  const at = fold(text).indexOf(word);
  if (at < 0) return text.slice(0, 110) + (text.length > 110 ? "…" : "");
  const from = Math.max(0, text.lastIndexOf(" ", Math.max(0, at - 40)));
  const cut = text.slice(from, from + 120).trim();
  return (from > 0 ? "…" : "") + cut + (from + 120 < text.length ? "…" : "");
}
