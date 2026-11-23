// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako in French: the translation's own rules, and the French dictionary checked as a whole.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { frType, fill, langOf, pluralOf, words, placeholders } from "../src/lib/i18n/core.ts";
import { fr, parts } from "../src/lib/i18n/fr.ts";
import { en } from "../src/lib/i18n/en.ts";

test("only French is French", () => {
  assert.equal(langOf("fr"), "fr");
  for (const v of ["en", "", null, undefined, "FR ", "de", "system"]) assert.equal(langOf(v), "en", String(v));
});

test("French typography: non-breaking spaces where French puts them", () => {
  assert.equal(frType("Dossier : Téléchargements"), "Dossier : Téléchargements");
  assert.equal(frType("Vraiment ? Oui !"), "Vraiment ? Oui !");
  assert.equal(frType("le dossier « Pako »"), "le dossier « Pako »");
  assert.equal(frType("https://pako.sookera.com"), "https://pako.sookera.com", "no space before a URL's colon");
  assert.equal(frType("Envoi {pct}%"), "Envoi {pct} %", "a non-breaking space before %");
  assert.equal(frType("déjà 45 %"), "déjà 45 %");
  assert.equal(frType("100%"), "100 %");
});

test("placeholders filled, unknown ones kept", () => {
  assert.equal(fill("{n} sur {total}", { n: 2, total: 5 }), "2 sur 5");
  assert.equal(fill("{n} {other}", { n: 1 }), "1 {other}");
});

test("plurals: French counts 0 and 1 as one, English only 1", () => {
  assert.equal(pluralOf("fr", 0), "one");
  assert.equal(pluralOf("fr", 1), "one");
  assert.equal(pluralOf("fr", 2), "other");
  assert.equal(pluralOf("en", 0), "other");
  assert.equal(pluralOf("en", 1), "one");
  const e = { one: "{n} fichier", other: "{n} fichiers" };
  assert.equal(words(e, "fr", { n: 0 }), "0 fichier");
  assert.equal(words(e, "fr", { n: 3 }), "3 fichiers");
  // A count already written by num(): the plural follows the number it stands for.
  assert.equal(words({ one: "{n} Go", other: "{n} Go (pl)" }, "fr", { n: "1,5" }), "1,5 Go");
  assert.equal(words(e, "fr", { n: "1 000" }), "1 000 fichiers");
  const en1 = { one: "{n} file", other: "{n} files" };
  assert.equal(words(en1, "en", { n: "1,000" }), "1,000 files");
  assert.equal(words(en1, "en", { n: "1" }), "1 file");
});

test("no key is in two areas (each area is translated on its own)", () => {
  const seen = new Map();
  for (const [area, dict] of Object.entries(parts)) {
    for (const k of Object.keys(dict)) {
      assert.ok(!seen.has(k), `"${k}" is in both ${seen.get(k)} and ${area}: keep it in one (shared words: fr-common.ts)`);
      seen.set(k, area);
    }
  }
});

test("every French entry follows the rules", () => {
  for (const [key, value] of Object.entries(fr)) {
    const forms = typeof value === "string" ? [value] : [value.one, value.other];
    for (const f of forms) {
      assert.ok(f.trim().length > 0, `"${key}": empty French`);
      assert.ok(!/\bapps?\b/i.test(f), `"${key}": « appli », never « app » (${f})`);
      assert.ok(!f.includes("'"), `"${key}": the French apostrophe is ’ (${f})`);
      assert.ok(!f.includes("..."), `"${key}": … not ... (${f})`);
      assert.ok(!/[  ]/.test(f), `"${key}": plain spaces in the dictionary (t() adds the French ones) (${f})`);
      assert.ok(!/ {2}/.test(f), `"${key}": a double space (${f})`);
    }
    // The same placeholders as the English (the key, or its English entry).
    const english = en[key] ?? key;
    assert.deepEqual(placeholders(value), placeholders(english), `"${key}": placeholders differ`);
    if (typeof value !== "string") assert.ok(typeof english !== "string", `"${key}": a French plural needs an English one in en.ts`);
  }
});
