// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The details panels' widths: remembered, kept in range, giving way to the list first.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { draggedWidth, keyedWidth, LIST_MIN, PANEL_DEFAULT, PANEL_MAX, PANEL_MIN, PANEL_STEP, savedWidth, shownWidth } from "../src/lib/panelSize.ts";

test("a remembered width is made safe", () => {
  assert.equal(savedWidth(undefined), PANEL_DEFAULT);
  assert.equal(savedWidth("400"), PANEL_DEFAULT, "not a number: the default");
  assert.equal(savedWidth(NaN), PANEL_DEFAULT);
  assert.equal(savedWidth(undefined, 288), 288, "a page's own default");
  assert.equal(savedWidth(100), PANEL_MIN);
  assert.equal(savedWidth(5000), PANEL_MAX);
  assert.equal(savedWidth(401.6), 402);
});

test("the panel gives way to the list first, never under its smallest width", () => {
  assert.equal(shownWidth(400, 0), 400, "not measured yet: as remembered");
  assert.equal(shownWidth(400, 2000), 400, "room enough");
  assert.equal(shownWidth(400, 700), 700 - LIST_MIN, "the list keeps its room");
  assert.equal(shownWidth(400, 500), PANEL_MIN, "smallest window: the panel's smallest width");
  assert.equal(shownWidth(400, 700, 120), 400, "over the conversation: a smaller strip is enough");
});

test("dragging the edge: left widens, within range and leaving the list its room", () => {
  assert.equal(draggedWidth(320, 50, 2000), 370);
  assert.equal(draggedWidth(320, -100, 2000), PANEL_MIN);
  assert.equal(draggedWidth(320, 900, 2000), PANEL_MAX);
  assert.equal(draggedWidth(320, 900, 900), 900 - LIST_MIN, "no wider than the list allows");
  assert.equal(draggedWidth(320, 900, 600), PANEL_MIN, "never under the smallest");
  assert.equal(draggedWidth(320, 900, 0), PANEL_MAX, "not measured: the largest");
});

test("keys on the edge", () => {
  assert.equal(keyedWidth(320, "ArrowLeft", false, 2000), 320 + PANEL_STEP);
  assert.equal(keyedWidth(320, "ArrowRight", false, 2000), 320 - PANEL_STEP);
  assert.equal(keyedWidth(320, "ArrowLeft", true, 2000), 320 + PANEL_STEP * 4);
  assert.equal(keyedWidth(320, "Home", false, 2000), PANEL_MAX);
  assert.equal(keyedWidth(320, "End", false, 2000), PANEL_MIN);
  assert.equal(keyedWidth(320, "Enter", false, 2000), null);
});
