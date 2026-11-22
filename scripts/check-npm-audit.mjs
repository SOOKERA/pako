// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// `npm audit` for the release (scripts/release.ps1): fails on any known vulnerability of
// moderate severity or worse in Pako's npm packages (the window's code and the build tools),
// except the ones listed in ALLOWED below, each checked by hand and with why it can't reach Pako.
// Low ones are shown, never a failure. Exit code: 0 = fine, 1 = a vulnerability to deal with,
// 2 = npm audit couldn't run (offline?).
import { execSync } from "node:child_process";

/** Advisories checked by hand (GHSA id → why Pako isn't affected). Remove an entry as soon as
 * the package is updated past it; add one only after reading the advisory against Pako's code. */
/** @type {Record<string, string>} */
export const ALLOWED = {
  // Empty since SvelteKit 3 (2026-11-22). Shape: "GHSA-xxxx-xxxx-xxxx": "package: what it is. Why it can't reach Pako.",
};

const RANK = { info: 0, low: 1, moderate: 2, high: 3, critical: 4 };

/** Sorts `npm audit --json`'s advisories into what fails the release, what is allowed, and the
 * low ones. Each advisory once, with the packages it is in. */
export function judge(report, allowed = ALLOWED) {
  const byId = new Map();
  for (const [pkg, v] of Object.entries(report?.vulnerabilities ?? {})) {
    for (const via of v.via ?? []) {
      if (typeof via === "string") continue; // through another package: listed under that one
      const id = String(via.url ?? "").match(/GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}/i)?.[0] ?? String(via.source ?? via.title);
      const seen = byId.get(id);
      if (seen) {
        if (!seen.packages.includes(pkg)) seen.packages.push(pkg);
      } else {
        byId.set(id, { id, title: via.title ?? "", severity: via.severity ?? v.severity ?? "high", url: via.url ?? "", packages: [pkg] });
      }
    }
  }
  const fail = [];
  const allowedHits = [];
  const low = [];
  for (const a of byId.values()) {
    if (Object.hasOwn(allowed, a.id)) allowedHits.push({ ...a, why: allowed[a.id] });
    else if ((RANK[a.severity] ?? RANK.high) >= RANK.moderate) fail.push(a);
    else low.push(a);
  }
  // Entries no longer found: the package moved past them, so they should go.
  const stale = Object.keys(allowed).filter((id) => !byId.has(id));
  return { fail, allowed: allowedHits, low, stale };
}

function main() {
  let out;
  try {
    // One fixed command (no input in it), through the shell so npm.cmd is found on Windows.
    out = execSync("npm audit --json", {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      maxBuffer: 64 * 1024 * 1024,
    });
  } catch (e) {
    // npm audit exits 1 when it finds anything: its report is still on stdout.
    out = e.stdout;
  }
  let report;
  try {
    report = JSON.parse(out);
  } catch {
    console.error("npm audit gave no report (offline?): the npm packages weren't checked.");
    process.exit(2);
  }
  if (report.error) {
    console.error(`npm audit couldn't run: ${report.error.summary ?? report.error.code ?? "unknown error"}`);
    process.exit(2);
  }
  // Only the report format this script reads (npm 7 and newer): another one would pass unread.
  if (report.auditReportVersion !== 2 || typeof report.vulnerabilities !== "object" || report.vulnerabilities === null) {
    console.error(`npm audit's report isn't in the format this check reads (auditReportVersion ${report.auditReportVersion ?? "missing"}): the npm packages weren't checked.`);
    process.exit(2);
  }
  const r = judge(report);
  const line = (a) => `  ${a.severity.padEnd(8)} ${a.id}  ${a.packages.join(", ")}: ${a.title}`;
  if (r.low.length) console.log(`Low (shown, not a failure):\n${r.low.map(line).join("\n")}`);
  if (r.allowed.length) console.log(`Checked by hand, can't reach Pako:\n${r.allowed.map((a) => `${line(a)}\n           ${a.why}`).join("\n")}`);
  if (r.stale.length) console.log(`No longer found (remove from scripts/check-npm-audit.mjs ALLOWED): ${r.stale.join(", ")}`);
  if (r.fail.length) {
    console.error(`Known vulnerabilities in Pako's npm packages:\n${r.fail.map((a) => `${line(a)}\n           ${a.url}`).join("\n")}`);
    process.exit(1);
  }
  console.log("npm packages: no known vulnerability to deal with.");
}

// import.meta.main, not a path comparison: through a junction or link the two paths differ, and
// the check would then silently not run.
if (import.meta.main === undefined) {
  // Node older than 24.2 (or 22.18) has no import.meta.main: refuse rather than skip silently.
  console.error(`Node ${process.version} is too old for this check (it needs 24.2 or newer): the npm packages weren't checked.`);
  process.exit(2);
}
if (import.meta.main) main();
