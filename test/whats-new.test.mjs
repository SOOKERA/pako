// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// What's new after an update: every version since the one you had.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { sinceSeen, versionParts } from "../src/lib/help/versions.ts";

const file = ["Next version", "1.2.5", "1.2.4", "1.2.3", "1.2.2", "1.2.1", "1.2.0", "1.1.0", "1.0.1"].map((version) => ({ version }));
const shown = (seen, now) => sinceSeen(file, seen, now).map((e) => e.version);

test("one version up: only that one", () => {
  assert.deepEqual(shown("1.2.4", "1.2.5"), ["1.2.5"]);
});

test("several versions skipped: all of them, newest first", () => {
  assert.deepEqual(shown("1.2.1", "1.2.5"), ["1.2.5", "1.2.4", "1.2.3", "1.2.2"]);
  assert.deepEqual(shown("1.1.0", "1.2.2"), ["1.2.2", "1.2.1", "1.2.0"]);
});

test("numbers compare as numbers (1.2.10 is after 1.2.9)", () => {
  const f = ["1.2.10", "1.2.9", "1.2.8"].map((version) => ({ version }));
  assert.deepEqual(sinceSeen(f, "1.2.8", "1.2.10").map((e) => e.version), ["1.2.10", "1.2.9"]);
});

test("no known previous version: only the current one; never 'Next version'", () => {
  assert.deepEqual(shown(null, "1.2.5"), ["1.2.5"]);
  assert.deepEqual(shown("not a version", "1.2.5"), ["1.2.5"]);
  assert.ok(!shown("1.0.1", "1.2.5").includes("Next version"));
});

test("nothing newer, or a version without notes: nothing to show", () => {
  assert.deepEqual(shown("1.2.5", "1.2.5"), []);
  assert.deepEqual(shown("1.2.5", "1.2.4"), [], "going back to an older Pako shows nothing");
  assert.deepEqual(shown(null, "9.9.9"), []);
});

test("versions read with or without 'Pako '", () => {
  assert.deepEqual(versionParts("Pako 1.2.0"), [1, 2, 0]);
  assert.equal(versionParts("Next version"), null);
});

// ---------- Audit 2026-11-18 (docs/audit-2026-11-18-whats-new-since.md) ----------
import { readFileSync } from "node:fs";
import { whatsNewTitle, compareVersions } from "../src/lib/help/versions.ts";

test("title: one version's own, or several since the one you had (not 'since your last update')", () => {
  assert.equal(whatsNewTitle(1, "1.2.4", "1.2.5"), "What's new in Pako 1.2.5");
  assert.equal(whatsNewTitle(4, "1.2.1", "1.2.5"), "What's new since Pako 1.2.1");
  assert.equal(whatsNewTitle(2, " Pako 1.1.0 ", "1.2.5"), "What's new since Pako 1.1.0");
  assert.equal(whatsNewTitle(1, null, "1.2.5"), "What's new in Pako 1.2.5");
});

test("a version written twice in whats-new.md shows once", () => {
  const f = [{ version: "1.2.5", n: 1 }, { version: "Pako 1.2.5", n: 2 }, { version: "1.2.4", n: 3 }];
  assert.deepEqual(sinceSeen(f, "1.2.3", "1.2.5").map((e) => e.n), [1, 3]);
});

test("whats-new.md: every section is 'Next version' or a plain version, once, newest first", () => {
  const text = readFileSync(new URL("../docs/help/whats-new.md", import.meta.url), "utf8");
  const heads = [...text.matchAll(/^##\s+(.+?)\s*$/gm)].map((m) => m[1]);
  const versions = heads.filter((h) => h !== "Next version");
  assert.ok(heads.indexOf("Next version") <= 0, "'Next version' comes first, if there");
  for (const h of versions) assert.match(h, /^\d+\.\d+\.\d+$/, `'## ${h}' would never show after an update`);
  const parts = versions.map((h) => versionParts(h));
  for (let i = 1; i < parts.length; i++) assert.ok(compareVersions(parts[i - 1], parts[i]) > 0, `'## ${versions[i]}' is out of order or twice`);
});
