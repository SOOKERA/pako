// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The compact window's ✕ → "Remove": asks once, never removes on a double-click.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { confirmClick, CONFIRM_FOR, CONFIRM_SETTLE } from "../src/lib/confirm.ts";

test("first click arms, a later second click on the same one removes", () => {
  assert.equal(confirmClick(null, "a", 1000), "arm");
  assert.equal(confirmClick({ id: "a", at: 1000 }, "a", 1000 + CONFIRM_SETTLE + 1), "confirm");
});

test("a double-click on ✕ doesn't remove (the second click lands on Remove)", () => {
  assert.equal(confirmClick({ id: "a", at: 1000 }, "a", 1000 + 150), "ignore");
});

test("another row's button arms that row, never removes it", () => {
  assert.equal(confirmClick({ id: "a", at: 1000 }, "b", 2000), "arm");
});

test("after the wait, it asks again", () => {
  assert.equal(confirmClick({ id: "a", at: 1000 }, "a", 1000 + CONFIRM_FOR), "arm");
});
