// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// A small, safe Markdown reader for Pako's guides (docs/help/*.md): headings, paragraphs,
// lists, **bold**, *italic*, `code` and [links](…), and a heading's own anchor written after it
// (`## Titre {#english-title}`, see docs/help/fr/TRADUCTION.md). No raw HTML: everything becomes text.

export type Inline =
  | { kind: "text"; text: string }
  | { kind: "bold"; text: string }
  | { kind: "italic"; text: string }
  | { kind: "code"; text: string }
  | { kind: "link"; text: string; href: string };

export type Block =
  | { kind: "heading"; level: number; text: string; id: string }
  | { kind: "para"; inline: Inline[] }
  | { kind: "list"; ordered: boolean; start: number; items: Inline[][] }
  | { kind: "code"; text: string };

/** The anchor of a heading, as GitHub and the website make it: `"Google limit": too many` → `google-limit-too-many`. */
export function slug(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s-]/gu, "")
    .trim()
    .replace(/\s/g, "-");
}

/** A heading's own anchor, after its words: `## Limite de Google {#google-limit}`. A translated
 *  guide keeps the English heading's anchor this way, so links from the window still land. */
const HEADING_ID = /\s*\{#([A-Za-z0-9_-]+)\}$/;

/** A heading line's words and anchor: its own one (`{#…}`, not shown) or one made from its words. */
export function headingOf(src: string): { text: string; id: string } {
  const own = HEADING_ID.exec(src);
  const text = plain(own ? src.slice(0, own.index) : src);
  return { text, id: own ? own[1] : slug(text) };
}

/** The text of a line without its Markdown marks (for search and titles). */
export function plain(text: string): string {
  return inline(text)
    .map((t) => t.text)
    .join("");
}

export function inline(src: string): Inline[] {
  const out: Inline[] = [];
  let text = "";
  const flush = () => {
    if (text) out.push({ kind: "text", text });
    text = "";
  };
  let i = 0;
  while (i < src.length) {
    const rest = src.slice(i);
    let m: RegExpMatchArray | null;
    if ((m = rest.match(/^\*\*(.+?)\*\*/))) {
      flush();
      out.push({ kind: "bold", text: m[1].replace(/\\(.)/g, "$1") });
    } else if ((m = rest.match(/^`([^`]+)`/))) {
      flush();
      out.push({ kind: "code", text: m[1] });
    } else if ((m = rest.match(/^\[([^\]]+)\]\(([^)\s]+)\)/))) {
      flush();
      out.push({ kind: "link", text: plain(m[1]), href: m[2] });
    } else if ((m = rest.match(/^\*([^*\s][^*]*?)\*/))) {
      flush();
      out.push({ kind: "italic", text: m[1] });
    } else if ((m = rest.match(/^\\(.)/))) {
      text += m[1];
    } else {
      text += src[i++];
      continue;
    }
    i += m[0].length;
  }
  flush();
  return out;
}

export function parse(md: string): Block[] {
  const lines = md.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let para: string[] = [];
  let list: { ordered: boolean; start: number; items: string[] } | null = null;
  const used = new Map<string, number>();

  const endPara = () => {
    if (para.length) blocks.push({ kind: "para", inline: inline(para.join(" ")) });
    para = [];
  };
  const endList = () => {
    if (list) blocks.push({ kind: "list", ordered: list.ordered, start: list.start, items: list.items.map(inline) });
    list = null;
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    let m: RegExpMatchArray | null;
    if (line.startsWith("```")) {
      endPara();
      endList();
      const code: string[] = [];
      while (++i < lines.length && !lines[i].startsWith("```")) code.push(lines[i]);
      blocks.push({ kind: "code", text: code.join("\n") });
    } else if ((m = line.match(/^(#{1,6})\s+(.*?)\s*#*\s*$/))) {
      endPara();
      endList();
      const { text, id: base } = headingOf(m[2]);
      // Repeated headings get -1, -2… like GitHub.
      let id = base;
      const n = used.get(id) ?? 0;
      used.set(id, n + 1);
      if (n) id = `${id}-${n}`;
      blocks.push({ kind: "heading", level: m[1].length, text, id });
    } else if ((m = line.match(/^\s{0,3}([-*+]|(\d+)[.)])\s+(.*)$/))) {
      endPara();
      const ordered = !!m[2];
      if (!list || list.ordered !== ordered) {
        endList();
        list = { ordered, start: ordered ? Number(m[2]) : 1, items: [] };
      }
      list.items.push(m[3]);
    } else if (!line.trim()) {
      endPara();
      // A blank line between items keeps the list going; anything else ends it.
      if (list) {
        const next = lines.slice(i + 1).find((l) => l.trim());
        if (!next || !/^\s{0,3}([-*+]|\d+[.)])\s+/.test(next)) endList();
      }
    } else if (list && /^\s{2,}\S/.test(line)) {
      list.items[list.items.length - 1] += " " + line.trim();
    } else {
      endList();
      para.push(line.trim());
    }
  }
  endPara();
  endList();
  return blocks;
}
