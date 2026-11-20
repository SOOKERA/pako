// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Off · Pop-up · Important: what the window shows for each download and in Settings (the same
// rules as model.rs `alert_of`, `done_alert`, `done_chime`).
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { alertMenu, alertOf, doneAlert, doneChime, nextAlert, sharedAlert } from "../src/lib/alert.ts";

test("a download's level: its choice, else an older list's flags, else the default", () => {
  assert.equal(alertOf({ alert: "important" }, "off"), "important");
  assert.equal(alertOf({ notify: true }, "off"), "pop", "old Notify when done = Pop-up");
  assert.equal(alertOf({ quiet: true }, "important"), "off", "old Notify me off = Off");
  assert.equal(alertOf({}, "important"), "important", "the default");
  assert.equal(alertOf({ alert: null }, "pop"), "pop");
});

test("New downloads and the chime start from the older switches", () => {
  assert.equal(doneAlert({}), "pop");
  assert.equal(doneAlert({ notifyDone: false }), "off", "Download finished pop-up was off: Off");
  assert.equal(doneAlert({ notifyDone: false, doneAlert: "important" }), "important", "a choice wins");
  assert.equal(doneAlert(null), "pop");
  assert.equal(doneChime({}), "all");
  assert.equal(doneChime({ playDoneSound: false }), "never");
  assert.equal(doneChime({ playDoneSound: false, doneChime: "important" }), "important");
});

test("the row's bell goes one step on each click and comes back round", () => {
  assert.equal(nextAlert("pop"), "important");
  assert.equal(nextAlert("important"), "off");
  assert.equal(nextAlert("off"), "pop");
});

test("several items: their shared level, or none ticked when they differ", () => {
  assert.equal(sharedAlert(["pop", "pop"]), "pop");
  assert.equal(sharedAlert(["pop", "off"]), null);
  assert.equal(sharedAlert([]), null);
  const picked = [];
  const menu = alertMenu("important", (a) => picked.push(a));
  assert.deepEqual(menu.map((m) => [m.label, m.hint]), [["Notify: Off", ""], ["Notify: Pop-up", ""], ["Notify: Important", "✓"]]);
  menu[0].action();
  assert.deepEqual(picked, ["off"]);
  assert.ok(alertMenu(null, () => {}).every((m) => m.hint === ""));
});
