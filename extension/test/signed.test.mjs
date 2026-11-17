// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// scripts/release.ps1 stops when the Firefox file signed by Mozilla isn't the extension as it is
// now (audit 2026-11-17 shipping, H1): scripts/check-firefox-xpi.mjs.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { makeZip } from "../zip.mjs";
import { xpiDifferences } from "../../scripts/check-firefox-xpi.mjs";

const b = (s) => Buffer.from(s);
const built = () => [
  { name: "manifest.json", data: b('{\n  "version": "1.2.2",\n  "name": "Pako"\n}\n') },
  { name: "background.js", data: b("const a = 1;\nconst b = 2;\n") },
  { name: "icons/icon16.png", data: Buffer.from([1, 2, 3]) },
];
const signed = (entries) => makeZip([...entries, { name: "META-INF/mozilla.rsa", data: b("sig") }]);

test("The same files match, whatever the line endings and manifest layout", () => {
  const xpi = signed([
    { name: "manifest.json", data: b('{"version":"1.2.2","name":"Pako"}') },
    { name: "background.js", data: b("const a = 1;\r\nconst b = 2;\r\n") },
    { name: "icons/icon16.png", data: Buffer.from([1, 2, 3]) },
  ]);
  assert.deepEqual(xpiDifferences(xpi, built()), []);
});

test("A file changed, added or removed since signing is caught", () => {
  const xpi = signed([
    { name: "manifest.json", data: b('{"version":"1.2.2","name":"Pako","permissions":[]}') },
    { name: "background.js", data: b("const a = 1;\n") },
    { name: "old.js", data: b("x") },
  ]);
  assert.deepEqual(xpiDifferences(xpi, built()), [
    "background.js: changed since it was signed",
    "icons/icon16.png: not in the signed file",
    "manifest.json: changed since it was signed",
    "old.js: only in the signed file",
  ]);
  // A picture compares byte for byte.
  const pic = signed([...built().slice(0, 2), { name: "icons/icon16.png", data: Buffer.from([1, 2, 4]) }]);
  assert.deepEqual(xpiDifferences(pic, built()), ["icons/icon16.png: changed since it was signed"]);
});
