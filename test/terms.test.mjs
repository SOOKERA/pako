// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's terms of use are the website's, copied by scripts/copy-terms.mjs: only the website's own
// marks go, and the English and French carry the same date (one version of the terms).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dateOf, forPako } from "../scripts/copy-terms.mjs";

const read = (name) => readFileSync(new URL(`../${name}`, import.meta.url), "utf8");

test("the copy drops the website's comment and heading ids, nothing else", () => {
  const site = "<!-- A note for the site's editors,\n  on two lines. -->\n\n# Conditions d’utilisation\r\n\r\nDernière mise à jour : 23 novembre 2026\n\n## En bref {#the-short-version}\n\nUn texte avec {#pas-un-titre} et **gras**.\n";
  assert.equal(
    forPako(site),
    "# Conditions d’utilisation\n\nDernière mise à jour : 23 novembre 2026\n\n## En bref\n\nUn texte avec {#pas-un-titre} et **gras**.\n",
  );
  // A file without the website's marks stays as it is.
  assert.equal(forPako("# Terms of use\n\nLast updated 18 November 2026\n"), "# Terms of use\n\nLast updated 18 November 2026\n");
});

test("dates in English and in French", () => {
  assert.equal(dateOf("Last updated 23 November 2026"), "2026-11-23");
  assert.equal(dateOf("Last updated: 3 may 2027."), "2027-05-03");
  assert.equal(dateOf("Dernière mise à jour : 23 novembre 2026"), "2026-11-23");
  assert.equal(dateOf("Dernière mise à jour : 1er août 2027"), "2027-08-01");
  assert.equal(dateOf("*Dernière mise à jour : 3 Décembre 2030*"), "2030-12-03");
  assert.equal(dateOf("Last updated soon"), null);
  assert.equal(dateOf("Dernière mise à jour : 31 brumaire 2030"), null);
});

test("Pako's TERMS.md and TERMS.fr.md are in step", () => {
  const en = read("TERMS.md");
  const fr = read("TERMS.fr.md");
  assert.ok(dateOf(en), "TERMS.md has a date");
  assert.equal(dateOf(fr), dateOf(en), "the French terms carry the English terms' date");
  for (const [name, text] of [["TERMS.md", en], ["TERMS.fr.md", fr]]) {
    assert.ok(!text.includes("{#"), `${name}: no heading ids left`);
    assert.ok(!text.includes("<!--"), `${name}: no website comment left`);
    assert.equal(forPako(text), text.replace(/\r\n/g, "\n"), `${name} is as copy-terms.mjs makes it`);
  }
  const heads = (s) => s.split(/\r?\n/).filter((l) => l.startsWith("#")).length;
  assert.equal(heads(fr), heads(en), "the same sections in both");
});
