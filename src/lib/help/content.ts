// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The guides in docs/help/, bundled into Pako when it's built (one source of truth with the website).
import { parse, plain, type Block } from "./markdown";
import licenceText from "../../../LICENSE?raw";
import noticeText from "../../../NOTICE.md?raw";

const files = import.meta.glob("../../../docs/help/*.md", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const byName = new Map(Object.entries(files).map(([path, text]) => [path.split("/").pop()!, text]));

export interface Section {
  id: string;
  title: string;
  level: number;
  /** The section's words, for search. */
  text: string;
}

export interface Guide {
  /** File name, e.g. "downloading.md". */
  file: string;
  title: string;
  /** One line from the Help index. */
  about: string;
  blocks: Block[];
  sections: Section[];
}

function sectionsOf(blocks: Block[]): Section[] {
  const out: Section[] = [];
  let cur: Section | null = null;
  for (const b of blocks) {
    if (b.kind === "heading" && b.level >= 2) {
      cur = { id: b.id, title: b.text, level: b.level, text: b.text };
      out.push(cur);
    } else if (cur && b.kind === "para") cur.text += " " + b.inline.map((t) => t.text).join("");
    else if (cur && b.kind === "list") cur.text += " " + b.items.map((it) => it.map((t) => t.text).join("")).join(" ");
  }
  return out;
}

/** The guides in the order of the Help index (docs/help/README.md). */
export const guides: Guide[] = (() => {
  const index = byName.get("README.md") ?? "";
  const out: Guide[] = [];
  for (const m of index.matchAll(/^\s*\d+\.\s+\[([^\]]+)\]\(([^)]+\.md)\):?\s*(.*)$/gm)) {
    const text = byName.get(m[2]);
    if (!text) continue;
    const blocks = parse(text);
    const h1 = blocks.find((b) => b.kind === "heading" && b.level === 1);
    out.push({
      file: m[2],
      title: h1 && h1.kind === "heading" ? h1.text : m[1],
      about: plain(m[3])
        .replace(/\.$/, "")
        .replace(/^./, (c) => c.toUpperCase()),
      // The title is shown by the Help panel itself.
      blocks: blocks.filter((b) => b !== h1),
      sections: sectionsOf(blocks),
    });
  }
  return out;
})();

export const guideByFile = (file: string) => guides.find((g) => g.file === file);

export interface WhatsNew {
  /** The section's heading: a version ("0.2.0") or "Next version" before the release. */
  version: string;
  blocks: Block[];
}

/** whats-new.md, one entry per "## <version>" section, newest first. */
export const whatsNew: WhatsNew[] = (() => {
  const blocks = parse(byName.get("whats-new.md") ?? "");
  const out: WhatsNew[] = [];
  for (const b of blocks) {
    if (b.kind === "heading" && b.level === 1) continue;
    if (b.kind === "heading" && b.level === 2) out.push({ version: b.text, blocks: [] });
    else out[out.length - 1]?.blocks.push(b);
  }
  return out;
})();

/** What's new for exactly this version (shown once after an update), if the file has it. */
export const whatsNewFor = (version: string) => whatsNew.find((w) => w.version === version || w.version === `Pako ${version}`);

export interface Hit {
  guide: Guide;
  section: Section | null;
  /** A few words around the match. */
  snippet: string;
}

/** Search every guide: sections whose title or words have all the words typed. Titles first. */
export function search(query: string): Hit[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  const has = (s: string) => {
    const l = s.toLowerCase();
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
  const at = text.toLowerCase().indexOf(word);
  if (at < 0) return text.slice(0, 110) + (text.length > 110 ? "…" : "");
  const from = Math.max(0, text.lastIndexOf(" ", Math.max(0, at - 40)));
  const cut = text.slice(from, from + 120).trim();
  return (from > 0 ? "…" : "") + cut + (from + 120 < text.length ? "…" : "");
}

// ---------- About Pako: the legal notices (GPL section 0 and NOTICE.md term 1) ----------

/** The GNU GPL version 3, word for word (LICENSE), bundled like the guides. */
export const licence: string = licenceText;

/** NOTICE.md (copyright, no warranty, the additional terms), without its title: the About page
 *  shows its own. */
export const notice: Block[] = parse(noticeText).filter((b) => !(b.kind === "heading" && b.level === 1));
