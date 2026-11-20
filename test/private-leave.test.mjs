// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Private mode open: which moves leave its screens (and lock it), and the question asked first
// while something still runs there.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { leaveQuestion, leaves, leaveWords } from "../src/lib/privateLeave.ts";

const here = { view: "downloads", library: "all", source: null, settings: false, help: false };

test("another page, a filter, Settings or Help leaves; the header's switch doesn't", () => {
  assert.equal(leaves(here, here), false, "Normal list / Private downloads: same page, same filters");
  for (const view of ["found", "send", "direct", "devices", "dashboard"]) assert.equal(leaves(here, { ...here, view }), true, view);
  assert.equal(leaves(here, { ...here, library: "active" }), true, "Downloading");
  assert.equal(leaves(here, { ...here, source: "gdrive" }), true, "a source");
  assert.equal(leaves(here, { ...here, settings: true }), true, "Settings");
  assert.equal(leaves(here, { ...here, help: true }), true, "Help");
  assert.equal(leaves({ ...here, help: true }, here), false, "Help closing isn't leaving");
});

test("what leaving stops, in words", () => {
  assert.equal(leaveWords([], null), "", "nothing runs: locks without asking");
  assert.equal(leaveWords([{ kind: "send", what: "Holiday.mp4" }], null), "sending Holiday.mp4");
  assert.equal(leaveWords([{ kind: "upload", what: "3 files" }], null), "uploading 3 files");
  assert.equal(leaveWords([{ kind: "decrypt", what: "the folder Trip" }], null), "decrypting the folder Trip");
  assert.equal(leaveWords([], "Concert.mkv"), "the video");
  assert.equal(leaveWords([{ kind: "send", what: "a.pdf" }], "Concert.mkv"), "2 things in progress");
  assert.equal(leaveQuestion("sending Holiday.mp4"), "Leaving locks Private mode and stops sending Holiday.mp4. Leave anyway?");
});

test("after Lock stopped something: a note naming nothing private", async () => {
  const { lockedNote } = await import("../src/lib/privateLeave.ts");
  assert.equal(lockedNote([], false), "", "nothing running: no note");
  assert.equal(lockedNote(["send"], false), "Private mode locked. 1 send stopped.");
  assert.equal(lockedNote(["send", "send", "upload"], false), "Private mode locked. 2 sends and 1 upload stopped.");
  assert.equal(lockedNote(["decrypt"], true), "Private mode locked. 1 decrypt and the video stopped.");
  assert.equal(lockedNote([], true), "Private mode locked. The video stopped.");
});
