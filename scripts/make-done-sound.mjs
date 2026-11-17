// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//
// Makes Pako's "download finished" chime, src-tauri/sounds/done.wav: two soft rising notes
// (E5 then A5, an octave lower than the first try: softer, less sharp), sine waves with a gentle
// start and fade, about 0.42 s, quiet (peak -16 dBFS), as a small
// 16-bit mono WAV at 22.05 kHz. Pako plays it when a download marked "Notify when done"
// finishes (src-tauri/src/notices.rs). Run again after changing it:
//
//   node scripts/make-done-sound.mjs
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const RATE = 22050;
const LENGTH = 0.42; // seconds
const PEAK_DB = -16;

// [frequency (Hz), start (s), how long it rings (s)]
const NOTES = [
  [659.25, 0.0, 0.2], // E5
  [880.0, 0.12, 0.3], // A5
];
const ATTACK = 0.010; // a soft start, no click, with a little edge
/** A touch of the octave above, for a bit of brightness (0 = pure sine). */
const BRIGHT = 0.15;

const n = Math.round(RATE * LENGTH);
const mix = new Float64Array(n);
for (const [freq, start, ring] of NOTES) {
  const from = Math.round(start * RATE);
  const count = Math.min(Math.round(ring * RATE), n - from);
  for (let i = 0; i < count; i++) {
    const t = i / RATE;
    const rise = Math.min(1, t / ATTACK);
    // A quick fade that reaches silence exactly at the note's end.
    const fall = Math.exp((-5 * t) / ring) * (1 - t / ring);
    const tone = Math.sin(2 * Math.PI * freq * t) + BRIGHT * Math.sin(4 * Math.PI * freq * t);
    mix[from + i] += tone * rise * fall;
  }
}

const peak = mix.reduce((m, v) => Math.max(m, Math.abs(v)), 0);
const gain = Math.pow(10, PEAK_DB / 20) / peak;

const data = Buffer.alloc(n * 2);
for (let i = 0; i < n; i++) {
  data.writeInt16LE(Math.round(mix[i] * gain * 32767), i * 2);
}

const header = Buffer.alloc(44);
header.write("RIFF", 0);
header.writeUInt32LE(36 + data.length, 4);
header.write("WAVEfmt ", 8);
header.writeUInt32LE(16, 16); // format block size
header.writeUInt16LE(1, 20); // PCM
header.writeUInt16LE(1, 22); // mono
header.writeUInt32LE(RATE, 24);
header.writeUInt32LE(RATE * 2, 28); // bytes per second
header.writeUInt16LE(2, 32); // bytes per sample
header.writeUInt16LE(16, 34); // bits per sample
header.write("data", 36);
header.writeUInt32LE(data.length, 40);

const out = join(dirname(fileURLToPath(import.meta.url)), "..", "src-tauri", "sounds", "done.wav");
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, Buffer.concat([header, data]));
console.log(`${out}: ${44 + data.length} bytes, ${LENGTH} s, peak ${PEAK_DB} dBFS`);
