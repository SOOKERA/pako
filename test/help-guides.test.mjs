// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The guides in English and French: heading anchors, links that land, French where there is some
// and English where there isn't, search in the language shown. The real files are checked too.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { headingOf, parse, slug } from "../src/lib/help/markdown.ts";
import { buildGuides, buildWhatsNew, fold, searchGuides } from "../src/lib/help/guides.ts";

const NBSP = " ";

// ---------- Heading anchors ----------

test("a heading's own anchor {#…} is used and not shown", () => {
  const h = headingOf(`Limite de Google${NBSP}: trop de téléchargements aujourd’hui {#google-limit-too-many-downloads-today}`);
  assert.equal(h.id, "google-limit-too-many-downloads-today");
  assert.equal(h.text, `Limite de Google${NBSP}: trop de téléchargements aujourd’hui`);
  const [b] = parse(`### « Limite de Google »${NBSP}: trop {#google-limit-too-many-downloads-today}`);
  assert.deepEqual(b, { kind: "heading", level: 3, text: `« Limite de Google »${NBSP}: trop`, id: "google-limit-too-many-downloads-today" });
});

test("without {#…}, the anchor is made from the words, as before", () => {
  assert.equal(headingOf(`"Google limit": too many downloads today`).id, "google-limit-too-many-downloads-today");
  assert.equal(headingOf("Add **links**").text, "Add links");
  // Only at the end, and only plain anchor characters.
  assert.equal(headingOf("A {#b} c").id, slug("A {#b} c"));
  assert.equal(headingOf("Titre {#a b}").text, "Titre {#a b}");
  const ids = parse("## Un {#same}\n\n## Deux {#same}").map((b) => b.kind === "heading" && b.id);
  assert.deepEqual(ids, ["same", "same-1"], "repeated anchors get -1 like repeated titles");
});

// ---------- French where there is some, English elsewhere ----------

const en = new Map([
  ["README.md", "# Help\n\n1. [Downloading](downloading.md): add links.\n2. [Videos](videos.md): quality.\n"],
  ["downloading.md", "# Downloading\n\n## Add links\n\nPaste them.\n"],
  ["videos.md", "# Videos\n\n## Quality\n\nPick one.\n"],
  ["whats-new.md", "# What's new\n\n## Next version\n\n- New thing.\n\n## 1.0.1\n\n- Fix.\n\n## 1.0.0\n\n- First.\n"],
]);
const fr = new Map([
  ["README.md", `# Aide\n\n1. [Télécharger](downloading.md)${NBSP}: ajouter des liens.\n2. [Vidéos](videos.md)${NBSP}: la qualité.\n`],
  ["downloading.md", "# Télécharger\n\n## Ajouter des liens {#add-links}\n\nCollez-les.\n"],
  ["whats-new.md", "# Nouveautés\n\n## Next version\n\n## 1.0.1\n\n- Correction.\n"],
]);

test("guides: the French one where there is one, the English one otherwise, in the English order", () => {
  const g = buildGuides(en, fr);
  assert.deepEqual(g.map((x) => x.file), ["downloading.md", "videos.md"]);
  assert.equal(g[0].title, "Télécharger");
  assert.equal(g[0].about, "Ajouter des liens", "the index line after a French ' : '");
  assert.equal(g[0].sections[0].id, "add-links", "the English anchor");
  assert.equal(g[1].title, "Videos", "no French guide yet: the English one");
  assert.equal(g[1].about, "La qualité", "its line in the French index");
  assert.deepEqual(buildGuides(en).map((x) => x.title), ["Downloading", "Videos"]);
});

test("What's new: each version in French when it has a French section, in English otherwise", () => {
  const w = buildWhatsNew(en.get("whats-new.md"), fr.get("whats-new.md"));
  assert.deepEqual(w.map((x) => x.version), ["Next version", "1.0.1", "1.0.0"]);
  const words = (e) => e.blocks.map((b) => JSON.stringify(b)).join(" ");
  assert.match(words(w[0]), /New thing/, "an empty French section: the English one");
  assert.match(words(w[1]), /Correction/);
  assert.match(words(w[2]), /First/, "no French section: the English one");
});

test("search finds French words with or without accents, either apostrophe", () => {
  const g = buildGuides(new Map([...en, ["downloading.md", "# Télécharger\n\n## Limite de Google {#google-limit}\n\nRéessayez demain, l’ID reste le même.\n"]]));
  assert.equal(fold("Télécharger l’ID"), "telecharger l'id");
  assert.equal(fold("Œuvre ÉTÉ").length, "Œuvre ÉTÉ".length, "same length, so the snippet lands in the right place");
  for (const q of ["réessayez", "reessayez", "l'id", "l’id", "TELECHARGER"]) assert.ok(searchGuides(g, q).length, q);
  assert.equal(searchGuides(g, "reessayez")[0].section.id, "google-limit");
  assert.match(searchGuides(g, "reessayez")[0].snippet, /Réessayez/);
});

// ---------- The real guides ----------

const dir = new URL("../docs/help/", import.meta.url);
const read = (sub) => {
  const at = new URL(sub, dir);
  if (!existsSync(at)) return new Map();
  return new Map(
    readdirSync(at)
      .filter((f) => f.endsWith(".md") && f !== "AUDIT.md" && f !== "TRADUCTION.md")
      .map((f) => [f, readFileSync(new URL(f, at), "utf8")]),
  );
};
const enFiles = read("./");
const frFiles = read("fr/");
const anchors = (md) => new Set(parse(md).flatMap((b) => (b.kind === "heading" && b.level >= 2 ? [b.id] : [])));
const guideFiles = [...enFiles.keys()].filter((f) => f !== "README.md" && f !== "whats-new.md");

test("every guide link in the window lands on a section, in English and in French", () => {
  const src = new URL("../src/", import.meta.url);
  const found = new Set();
  for (const f of readdirSync(src, { recursive: true })) {
    if (!/\.(svelte|ts)$/.test(f) || f.includes("i18n")) continue;
    for (const m of readFileSync(new URL(f.replaceAll("\\", "/"), src), "utf8").matchAll(/["'`]([a-z-]+\.md)#([a-z0-9_-]+)["'`]/g)) found.add(`${m[1]}#${m[2]}`);
  }
  assert.ok(found.size >= 3, "the window's links were found");
  for (const link of found) {
    const [file, anchor] = link.split("#");
    assert.ok(enFiles.has(file), `${link}: no such guide`);
    assert.ok(anchors(enFiles.get(file)).has(anchor), `${link}: no such section in docs/help/${file}`);
    if (frFiles.has(file)) assert.ok(anchors(frFiles.get(file)).has(anchor), `${link}: docs/help/fr/${file} has no {#${anchor}}`);
  }
});

test("links between guides land, in English and in French", () => {
  for (const [lang, files] of [["en", enFiles], ["fr", frFiles]]) {
    for (const [name, md] of files) {
      const links = [...md.matchAll(/\]\(([^)\s]+)\)/g)].map((m) => m[1]).filter((h) => !/^(https?:|mailto:)/.test(h));
      for (const href of links) {
        const [file, anchor] = href.split("#");
        const target = file || name;
        const where = `docs/help/${lang === "fr" ? "fr/" : ""}${name}: (${href})`;
        assert.ok(enFiles.has(target), `${where}: no such guide`);
        if (!anchor) continue;
        const text = (lang === "fr" && frFiles.get(target)) || enFiles.get(target);
        assert.ok(anchors(text).has(anchor), `${where}: no such section`);
      }
    }
  }
});

test("French guides: the same guides, each section keeping its English anchor", () => {
  for (const f of frFiles.keys()) assert.ok(enFiles.has(f), `docs/help/fr/${f} has no English guide of the same name`);
  for (const f of guideFiles.filter((x) => frFiles.has(x))) {
    const want = [...anchors(enFiles.get(f))].sort();
    const got = [...anchors(frFiles.get(f))].sort();
    const missing = want.filter((a) => !got.includes(a));
    const extra = got.filter((a) => !want.includes(a));
    assert.deepEqual({ missing, extra }, { missing: [], extra: [] }, `docs/help/fr/${f}: every heading ends with {#<the English heading's anchor>}`);
  }
  if (frFiles.has("README.md")) {
    const listed = (md) => [...md.matchAll(/^\s*\d+\.\s+\[[^\]]+\]\(([^)]+\.md)\)/gm)].map((m) => m[1]);
    assert.deepEqual(listed(frFiles.get("README.md")), listed(enFiles.get("README.md")), "docs/help/fr/README.md lists the same guides in the same order");
  }
});

test("French guides: French typography written in (outside code and addresses)", () => {
  for (const [f, md] of frFiles) {
    let fence = false;
    md.split(/\r?\n/).forEach((line, i) => {
      if (line.startsWith("```")) fence = !fence;
      if (fence || line.startsWith("```")) return;
      const text = line.replace(/`[^`]*`/g, "").replace(/\]\([^)]*\)/g, "]").replace(/\{#[^}]*\}$/, "");
      const at = `docs/help/fr/${f}:${i + 1}`;
      assert.ok(!text.includes("'"), `${at}: the French apostrophe is ’`);
      assert.ok(!text.includes("..."), `${at}: … not ...`);
      assert.ok(!/[  ]:/.test(text), `${at}: a non-breaking space (U+00A0) before :`);
      assert.ok(!/[  ][;!?]/.test(text), `${at}: a narrow non-breaking space (U+202F) before ; ! ?`);
      assert.ok(!/«[  ]|[  ]»/.test(text), `${at}: non-breaking spaces (U+00A0) inside « »`);
      assert.ok(!/«[^ ]|[^ ]»/.test(text), `${at}: a non-breaking space (U+00A0) inside « »`);
    });
  }
});
