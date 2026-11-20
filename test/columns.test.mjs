// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The lists' columns: widths kept in range, hidden ones never shown, narrow windows dropping
// the least important first, and what's saved read back safely (older Downloads widths kept).
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { BIG_STEP, EMPTY, LISTS, STEP, clampWidth, draggedColumn, gridTemplate, keyedColumn, readSaved, roomFor, shownColumns, toggled, widthOf, withPost } from "../src/lib/columns.ts";

const dl = LISTS.downloads;
const col = (list, key) => list.cols.find((c) => c.key === key);

test("a width stays within the column's limits", () => {
  const c = col(dl, "source");
  assert.equal(clampWidth(c, 10), c.min);
  assert.equal(clampWidth(c, 9999), c.max);
  assert.equal(clampWidth(c, "200"), c.def, "not a number: the first width");
  assert.equal(clampWidth(c, NaN), c.def);
  assert.equal(clampWidth(c, 150.6), 151);
});

test("dragging the left edge left makes the column wider", () => {
  const c = col(dl, "status");
  assert.equal(draggedColumn(c, 122, -30), 152);
  assert.equal(draggedColumn(c, 122, 30), 92);
  assert.equal(draggedColumn(c, 122, 500), c.min);
});

test("keys on the edge: left wider, right narrower, Shift bigger, Home and End the ends", () => {
  const c = col(dl, "speed");
  assert.equal(keyedColumn(c, 92, "ArrowLeft", false), 92 + STEP);
  assert.equal(keyedColumn(c, 92, "ArrowRight", false), 92 - STEP);
  assert.equal(keyedColumn(c, 92, "ArrowLeft", true), 92 + BIG_STEP);
  assert.equal(keyedColumn(c, 92, "Home", false), c.max);
  assert.equal(keyedColumn(c, 92, "End", false), c.min);
  assert.equal(keyedColumn(c, 92, "Enter", false), null);
});

test("every column shows when there's room; hidden ones never do", () => {
  assert.deepEqual(shownColumns(dl, EMPTY, 2000).keys, ["source", "progress", "speed", "status", "date"]);
  const st = toggled(EMPTY, "speed");
  assert.deepEqual(shownColumns(dl, st, 2000).keys, ["source", "progress", "status", "date"]);
  assert.deepEqual(toggled(st, "speed").hidden, [], "shown again");
  assert.deepEqual(shownColumns(dl, st, 0).keys, ["source", "progress", "status", "date"], "not measured yet");
});

test("columns give way down to their smallest before one steps aside", () => {
  // Downloads: Source goes first, then the date, then the speed; Progress and Status stay down
  // to a list of about 460 px (Name keeps 130 px to stay readable), and Progress never goes.
  assert.deepEqual(shownColumns(dl, EMPTY, 880).keys, ["source", "progress", "speed", "status", "date"]);
  assert.equal(shownColumns(dl, EMPTY, 880).tight, false);
  assert.deepEqual(shownColumns(dl, EMPTY, 700).keys, ["source", "progress", "speed", "status", "date"]);
  assert.equal(shownColumns(dl, EMPTY, 700).tight, true, "Name at its narrow width before a column steps aside");
  assert.deepEqual(shownColumns(dl, EMPTY, 660).keys, ["progress", "speed", "status", "date"]);
  assert.deepEqual(shownColumns(dl, EMPTY, 560).keys, ["progress", "speed", "status"]);
  assert.deepEqual(shownColumns(dl, EMPTY, 460).keys, ["progress", "status"]);
  assert.deepEqual(shownColumns(dl, EMPTY, 450).keys, ["progress"]);
  assert.deepEqual(shownColumns(dl, EMPTY, 440).keys, ["progress"]);
  assert.equal(shownColumns(dl, EMPTY, 440).tight, false);
  assert.deepEqual(shownColumns(dl, EMPTY, 200).keys, ["progress"], "never Progress");
  // A column you hid stays hidden when there's room again.
  const st = { w: { source: 320 }, hidden: ["date"] };
  assert.deepEqual(shownColumns(dl, st, 2000).keys, ["source", "progress", "speed", "status"]);
  // The private list's Downloaded column (it also says how a file is doing) never steps aside.
  assert.ok(shownColumns(LISTS.private, EMPTY, 200).keys.includes("date"));
  assert.ok(!shownColumns(LISTS.private, EMPTY, 300).keys.includes("size"));
  // Upload: the link goes first; where it is stays down to about 440 px (a narrow page shows
  // the row's buttons as icons only: their column is 76 px then).
  assert.deepEqual(shownColumns(LISTS.send, EMPTY, 560).keys, ["host"]);
  const narrowSend = withPost(LISTS.send, 76);
  assert.deepEqual(shownColumns(narrowSend, EMPTY, 440).keys, ["host"]);
  assert.deepEqual(shownColumns(narrowSend, EMPTY, 400).keys, []);
});

test("dragging a column wider stops where the room ends", () => {
  const keys = ["source", "progress", "speed", "status", "date"];
  const widths = { source: 128, progress: 156, speed: 92, status: 122, date: 112 };
  // 1060 px: 132 fixed, Name 160, the others 128 + 156 + 92 + 112 = 488: Status can have 280.
  assert.equal(roomFor(dl, keys, widths, 1060, "status"), 280);
  assert.equal(roomFor(dl, keys, widths, 600, "status"), col(dl, "status").min, "never under its smallest");
  assert.equal(roomFor(dl, keys, widths, 0, "status"), col(dl, "status").max, "not measured yet");
  assert.equal(draggedColumn(col(dl, "status"), 122, -500, 280), 280);
  assert.equal(keyedColumn(col(dl, "status"), 276, "ArrowLeft", true, 280), 280);
  assert.equal(keyedColumn(col(dl, "status"), 122, "Home", false, 280), 280);
});

test("the grid: fixed columns, Name taking what's left, then each column in its place", () => {
  const st = { w: { status: 140 }, hidden: [] };
  const g = gridTemplate(dl, st, ["progress", "status"], false);
  assert.equal(g, "30px 46px minmax(160px, 1fr) minmax(0px, 0px) minmax(110px, 156px) minmax(0px, 0px) minmax(80px, 140px) minmax(0px, 0px) 24px", "a column not shown keeps its place at no width");
  const tightGrid = gridTemplate(dl, st, ["progress"], true);
  assert.ok(tightGrid.includes(`minmax(${dl.nameTight}px, 1fr)`));
  assert.ok(tightGrid.includes("minmax(110px, 110px)"), "the columns left at their smallest: Name gets the rest");
  // Lists with room between columns: it's inside each track after Name, so a hidden column
  // leaves no gap.
  assert.equal(gridTemplate(LISTS.send, EMPTY, ["host"], false), "38px minmax(150px, 1fr) minmax(132px, 228px) minmax(0px, 0px) 184px");
});

test("saved settings are read back safely; an older Pako's Downloads widths are kept", () => {
  const legacy = { source: 200, progress: 156, speed: 9999, status: "x" };
  const fromOld = readSaved(null, legacy);
  assert.deepEqual(fromOld.downloads.w, { source: 200, speed: 420 }, "same as first width or broken: not stored; never narrower than an older Pako allowed");
  assert.deepEqual(fromOld.found, { w: {}, hidden: [] });
  // Newer settings win over the old ones; junk is left out.
  const saved = readSaved({ downloads: { w: { date: 150 }, hidden: ["speed", "speed", "nope", 3] }, found: "junk", other: {} }, legacy);
  assert.deepEqual(saved.downloads, { w: { date: 150 }, hidden: ["speed"] });
  assert.deepEqual(saved.found, { w: {}, hidden: [] });
  assert.equal(Object.keys(saved).sort().join(), "downloads,found,private,send");
  assert.equal(widthOf(dl, saved.downloads, "date"), 150);
  assert.equal(widthOf(dl, saved.downloads, "source"), col(dl, "source").def);
});

test("Links' Link column is a small mark: a width saved for its old words isn't kept", () => {
  const saved = readSaved({ found: { w: { link: 260, source: 150 }, hidden: [] } });
  assert.deepEqual(saved.found.w, { source: 150 }, "the old text width goes; others stay");
  const small = readSaved({ found: { w: { link: 44 }, hidden: [] } });
  assert.deepEqual(small.found.w, { link: 44 });
});
