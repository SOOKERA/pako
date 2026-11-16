// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Turns the full extension's files into Pako Light's (the Chrome Web Store / Edge Add-ons
// edition), for build.mjs and the tests in extension/test/:
//   - the blocks marked "full-only" are taken out
//       JS:   /* full-only:start */ … /* full-only:end */
//       HTML: <!-- full-only:start --> … <!-- full-only:end -->
//   - `const LIGHT = false;` becomes `const LIGHT = true;` (exactly once per script)
//   - and the result is checked: no word of the full extension's extras may be left in it.

/** Words that must not be anywhere in Pako Light (the stores it goes to don't allow what they
 *  stand for, and Light's wording stays neutral). The sites list Light leaves alone is allowed. */
export const LIGHT_BANNED = /video|quality|player|siteCookies|sendVideo|VIDEO_QUALITIES|yt-dlp|paste it in Pako|Ctrl\+V/i;

/** Files whose LIGHT switch is flipped. */
const SWITCHED = new Set(["background.js", "popup.js"]);

const BLOCKS = {
  js: /^[ \t]*\/\* full-only:start \*\/[\s\S]*?\/\* full-only:end \*\/[ \t]*\r?\n?/gm,
  html: /^[ \t]*<!-- full-only:start -->[\s\S]*?<!-- full-only:end -->[ \t]*\r?\n?/gm,
};
const MARKS = {
  js: [/\/\* full-only:start \*\//g, /\/\* full-only:end \*\//g],
  html: [/<!-- full-only:start -->/g, /<!-- full-only:end -->/g],
};

const count = (text, re) => (text.match(re) ?? []).length;

/** Pako Light's version of one file (`name` = its name in the extension folder). Throws when a
 *  marker is unbalanced, the LIGHT switch isn't there exactly once, or a banned word is left. */
export function toLight(name, text) {
  const kind = name.endsWith(".html") ? "html" : name.endsWith(".js") ? "js" : null;
  let out = text;
  if (kind) {
    const [start, end] = MARKS[kind];
    const starts = count(out, start);
    if (starts !== count(out, end)) throw new Error(`${name}: a full-only block isn't closed`);
    out = out.replace(BLOCKS[kind], "");
    if (count(out, start) || count(out, end)) throw new Error(`${name}: a full-only marker is left (nested or out of order)`);
  }
  if (SWITCHED.has(name)) {
    const off = /^const LIGHT = false;\r?$/gm;
    if (count(out, off) !== 1) throw new Error(`${name}: the line "const LIGHT = false;" must be there exactly once`);
    out = out.replace(off, (line) => line.replace("false", "true"));
    if (count(out, /^const LIGHT = true;\r?$/gm) !== 1 || /const LIGHT = false/.test(out)) {
      throw new Error(`${name}: Pako Light's switch didn't turn on`);
    }
  }
  const banned = out.match(LIGHT_BANNED);
  if (banned) {
    const line = out.slice(0, banned.index).split("\n").length;
    throw new Error(`${name}: Pako Light still says "${banned[0]}" (line ${line})`);
  }
  return out;
}
