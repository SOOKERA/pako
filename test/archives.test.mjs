// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// "Extract…" is offered on whole archives and a split archive's first part only.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { extractable } from "../src/lib/archives.ts";

test("whole archives and first parts extract; later parts don't", () => {
  for (const n of ["a.zip", "B.7Z", "c.rar", "d.part1.rar", "d.part01.rar", "e.7z.001", "f.zip.001"]) assert.equal(extractable(n), true, n);
  for (const n of ["d.part2.rar", "d.part02.rar", "d.part10.rar", "e.7z.002", "e.7z.010", "f.z01", "g.r00", "notes.txt", "movie.mp4"]) assert.equal(extractable(n), false, n);
});
