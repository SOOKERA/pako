// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Links' Link column: every state has plain words (never an internal name), its own mark, and
// a click only where it can do something.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { linkLook, linkTip } from "../src/lib/linkState.ts";

const look = (state, more = {}) => linkLook({ state, video: false, ...more });

test("each state says what it is, in words", () => {
  assert.equal(look("ready").word, "Ready");
  assert.equal(look("checking").word, "Checking link");
  assert.equal(look("checking", { video: true }).word, "Checking video…");
  assert.equal(look("needsTools").word, "Needs the video tools");
  assert.equal(look("broken").word, "Link broken");
  assert.equal(look("broken", { message: "Not found (404)" }).word, "Not found (404)");
  assert.equal(look("needsAccount").word, "Only in the browser");
  for (const s of ["ready", "checking", "needsTools", "opening", "broken", "needsAccount"]) assert.doesNotMatch(look(s).word, /needsTools|needsAccount/);
});

test("Google's daily limit: its own mark, not plain Ready", () => {
  const l = look("ready", { message: "Google's limit for this file is reached for today." });
  assert.equal(l.word, "Ready · Google limit today");
  assert.equal(l.mark, "limit");
  assert.notEqual(l.mark, look("ready").mark);
});

test("a link page: waiting for you, opening, or waiting its turn", () => {
  const you = look("opening", { message: "Waiting for you: finish the check" });
  assert.deepEqual([you.word, you.act, you.mark], ["Waiting for you", "page", "alert"]);
  assert.match(linkTip(you), /show check/);
  const opening = look("opening", { message: "Opening the page" });
  assert.deepEqual([opening.word, opening.act, opening.quiet], ["Opening link…", "page", true]);
  const turn = look("opening", { message: "Waiting for the link page before it" });
  assert.equal(turn.word, "Waiting its turn");
  assert.equal(turn.act, undefined, "nothing to show yet: no click");
});

test("clicks only where they do something; the tools being installed isn't clickable", () => {
  assert.equal(look("needsTools").act, "tools");
  assert.equal(look("needsTools", { toolsBusy: true }).act, undefined);
  assert.equal(look("needsAccount").act, "browser");
  assert.equal(look("ready").act, undefined);
  assert.equal(look("broken").act, undefined);
  // "Download on" can't send it: its note wins, as a warning.
  const d = look("ready", { deviceNote: "Private downloads stay on this PC." });
  assert.deepEqual([d.word, d.kind, d.mark], ["Private downloads stay on this PC.", "needs", "alert"]);
});
