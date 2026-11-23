// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The guides in docs/help/, bundled into Pako when it's built (one source of truth with the website),
// in the language Pako shows: docs/help/fr/ in French, the English guide where there's no French one.
import { i18n, type Lang } from "../i18n.svelte";
import { parse, type Block } from "./markdown";
import { buildGuides, buildWhatsNew, searchGuides, type Files, type Guide, type Hit, type WhatsNew } from "./guides";
import { sinceSeen } from "./versions";
import licenceText from "../../../LICENSE?raw";
import noticeText from "../../../NOTICE.md?raw";
import termsText from "../../../TERMS.md?raw";

export type { Guide, Hit, Section, WhatsNew } from "./guides";

// docs/help/fr/TRADUCTION.md is the translators' note, not a guide.
const files = import.meta.glob(["../../../docs/help/*.md", "../../../docs/help/fr/*.md", "!../../../docs/help/fr/TRADUCTION.md"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;
const byLang: Record<Lang, Files> = { en: new Map(), fr: new Map() };
for (const [path, text] of Object.entries(files)) {
  const parts = path.split("/");
  const name = parts.pop()!;
  byLang[parts.pop() === "fr" ? "fr" : "en"].set(name, text);
}

// Each language is read once, the first time it shows.
const made = new Map<Lang, { guides: Guide[]; whatsNew: WhatsNew[] }>();
function shown() {
  const lang = i18n.lang;
  let m = made.get(lang);
  if (!m) {
    const local = lang === "en" ? undefined : byLang[lang];
    m = {
      guides: buildGuides(byLang.en, local),
      whatsNew: buildWhatsNew(byLang.en.get("whats-new.md") ?? "", local?.get("whats-new.md")),
    };
    made.set(lang, m);
  }
  return m;
}

/** The guides in the order of the Help index (docs/help/README.md), in the language shown. */
export const guides = (): Guide[] => shown().guides;

export const guideByFile = (file: string) => guides().find((g) => g.file === file);

/** whats-new.md, one entry per "## <version>" section, newest first, in the language shown. */
export const whatsNew = (): WhatsNew[] => shown().whatsNew;

/** What's new to show once after an update: every version since `seen` (the one you had),
 *  newest first; just `version`'s own without a known `seen`. Empty when there's nothing. */
export const whatsNewSince = (seen: string | null | undefined, version: string) => sinceSeen(whatsNew(), seen, version);

/** Search the guides shown: sections whose title or words have all the words typed. Titles first. */
export const search = (query: string): Hit[] => searchGuides(guides(), query);

// ---------- About Pako: the legal notices (GPL section 0 and NOTICE.md term 1) ----------

/** The GNU GPL version 3, word for word (LICENSE), bundled like the guides. */
export const licence: string = licenceText;

/** TERMS.md, the terms of use (word for word sookera.com/terms), without its title: the terms
 *  step and the About page show their own. */
export const terms: Block[] = parse(termsText).filter((b) => !(b.kind === "heading" && b.level === 1));

/** NOTICE.md (copyright, no warranty, the additional terms), without its title: the About page
 *  shows its own. */
export const notice: Block[] = parse(noticeText).filter((b) => !(b.kind === "heading" && b.level === 1));
