// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Is the Firefox extension signed by Mozilla the extension as it is now? scripts/release.ps1 runs
// this before building: a signed file of the same version made before the extension changed
// would otherwise ship old code in Pako.exe and in the release (Mozilla signs each version once,
// so a change needs a new version and a new signature).
//
//   node scripts/check-firefox-xpi.mjs <signed .xpi> <dist/extension/firefox folder>
//
// Exit 0 = the same files; 1 = what differs is printed. Mozilla's own files (META-INF/) are
// left out. Line endings don't count (git may check files out with CRLF on one PC and LF on
// another), and manifest.json is compared as JSON.
import { readFileSync, readdirSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { readZip } from "../extension/zip.mjs";

const TEXT = /\.(js|html|css|json|md|txt)$|^LICENSE$/;

/** The same text, whatever its line endings. */
const lf = (buf) => buf.toString("utf8").replace(/\r\n/g, "\n");

/** Every file under `dir`, as { name, data } with "/" in names. */
export function filesOf(dir) {
  const list = [];
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else list.push({ name: relative(dir, p).split(sep).join("/"), data: readFileSync(p) });
    }
  };
  walk(dir);
  return list;
}

/** What differs between the signed .xpi (a Buffer) and the built files: [] when they match. */
export function xpiDifferences(xpi, files) {
  const signed = new Map(readZip(xpi).filter((e) => !e.name.startsWith("META-INF/")).map((e) => [e.name, e.data]));
  const built = new Map(files.filter((f) => !f.name.startsWith("META-INF/")).map((f) => [f.name, f.data]));
  const out = [];
  for (const name of built.keys()) if (!signed.has(name)) out.push(`${name}: not in the signed file`);
  for (const name of signed.keys()) if (!built.has(name)) out.push(`${name}: only in the signed file`);
  for (const [name, data] of built) {
    const s = signed.get(name);
    if (!s) continue;
    let same;
    if (name === "manifest.json") {
      try {
        same = JSON.stringify(JSON.parse(lf(s))) === JSON.stringify(JSON.parse(lf(data)));
      } catch {
        same = false;
      }
    } else if (TEXT.test(name.split("/").pop())) {
      same = lf(s) === lf(data);
    } else {
      same = s.equals(data);
    }
    if (!same) out.push(`${name}: changed since it was signed`);
  }
  return out.sort();
}

const runDirectly = !!process.argv[1] && resolve(process.argv[1]).toLowerCase() === fileURLToPath(import.meta.url).toLowerCase();
if (runDirectly) {
  const [xpiPath, folder] = process.argv.slice(2);
  if (!xpiPath || !folder) {
    console.error("Use: node scripts/check-firefox-xpi.mjs <signed .xpi> <dist/extension/firefox>");
    process.exit(2);
  }
  const diff = xpiDifferences(readFileSync(xpiPath), filesOf(folder));
  if (diff.length) {
    for (const d of diff) console.log(`  ${d}`);
    process.exit(1);
  }
  console.log("The signed Firefox extension is the extension as it is now.");
}
