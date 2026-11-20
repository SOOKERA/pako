// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Long names cut in the middle never split a character in half (an accent, an emoji, a flag).
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { graphemes } from "../src/lib/format.ts";

test("characters as people see them", () => {
  assert.deepEqual(graphemes("abc"), ["a", "b", "c"]);
  assert.deepEqual(graphemes("été"), ["é", "t", "é"], "an accent stays with its letter");
  assert.deepEqual(graphemes("👍🏽ok"), ["👍🏽", "o", "k"], "an emoji with its skin tone is one");
  assert.deepEqual(graphemes("🇲🇬.mp4").length, 5, "a flag is one");
});
