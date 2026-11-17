// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// recorder.js (inside the page) with a stand-in for the browser's video player parts.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const source = readFileSync(new URL("../recorder.js", import.meta.url), "utf8");

function page() {
  const posted = [];
  const on = { window: {}, document: {} };
  class SourceBuffer {
    appendBuffer() {
      // The browser refuses while its buffer is full (the player sends the same data again).
      if (this.full) throw new Error("QuotaExceededError");
    }
  }
  class MediaSource {
    addSourceBuffer() {
      return new SourceBuffer();
    }
  }
  class HTMLVideoElement {}
  const window = {
    MediaSource,
    SourceBuffer,
    postMessage: (m) => posted.push(m),
    addEventListener: (t, f) => (on.window[t] ||= []).push(f),
  };
  const document = { addEventListener: (t, f) => (on.document[t] ||= []).push(f), querySelectorAll: () => [] };
  vm.runInNewContext(source, { window, document, navigator: {}, HTMLVideoElement, setInterval: () => 0, ArrayBuffer, Uint8Array });
  // The bridge says this is the tab being recorded.
  for (const f of on.window.message) f({ source: window, data: { "pako-recorder": "bridge", t: "arm", on: true } });
  const ended = (src) => {
    const v = Object.assign(new HTMLVideoElement(), { src, currentSrc: src });
    for (const f of on.document.ended) f({ target: v });
  };
  return { posted, sb: new MediaSource().addSourceBuffer("video/mp4"), ended };
}

test("A part the browser refuses isn't recorded (the player sends it again)", () => {
  const { posted, sb } = page();
  sb.appendBuffer(new Uint8Array([1, 2, 3]));
  sb.full = true;
  assert.throws(() => sb.appendBuffer(new Uint8Array([4, 5])));
  sb.full = false;
  sb.appendBuffer(new Uint8Array([4, 5]));
  const chunks = posted.filter((m) => m.t === "chunk").map((m) => [...m.data]);
  assert.deepEqual(chunks, [[1, 2, 3], [4, 5]]);
});

test("Only the end of a video built from parts ends the recording, not an advert's", () => {
  const { posted, ended } = page();
  ended("https://ads.example/spot.mp4");
  assert.equal(posted.filter((m) => m.t === "end").length, 0);
  ended("blob:https://site.example/1234");
  assert.equal(posted.filter((m) => m.t === "end").length, 1);
});
