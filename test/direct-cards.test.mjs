// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Direct's transfer cards (src/lib/DirectView.svelte, DeviceDetails.svelte) and the list of what
// was in a transfer (TransferItems.svelte), read as source: a card that holds buttons of its own
// (Cancel, Open, Folder, the arrow) is never itself a button (a button's insides are hidden from
// screen readers), and the other side's file names are only ever shown as text.
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const read = (f) => fs.readFileSync(path.resolve(here, "../src/lib", f), "utf8");
const files = ["DirectView.svelte", "DeviceDetails.svelte", "DirectDetails.svelte", "TransferItems.svelte"];

test("a transfer card holding buttons is never a button itself", () => {
  for (const f of ["DirectView.svelte", "DeviceDetails.svelte"]) {
    const bubbles = read(f).match(/<div class="bubble"[^>]*>/g) ?? [];
    assert.ok(bubbles.length > 0, f);
    for (const b of bubbles) assert.doesNotMatch(b, /role="button"/, `${f}: ${b}`);
  }
});

test("names from the other side are shown as text, never as HTML", () => {
  for (const f of files) assert.doesNotMatch(read(f), /\{@html/, f);
});
