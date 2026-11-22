// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// scripts/check-npm-audit.mjs: what fails a release.
import { test } from "node:test";
import assert from "node:assert/strict";
import { judge } from "../scripts/check-npm-audit.mjs";

const adv = (id, severity, title = "t") => ({ source: 1, title, severity, url: `https://github.com/advisories/${id}` });

test("moderate or worse fails, low is only shown, allowed ones pass", () => {
  const report = {
    vulnerabilities: {
      a: { severity: "high", via: [adv("GHSA-aaaa-aaaa-aaaa", "high")] },
      b: { severity: "low", via: [adv("GHSA-bbbb-bbbb-bbbb", "low")] },
      c: { severity: "critical", via: [adv("GHSA-cccc-cccc-cccc", "critical")] },
      d: { severity: "moderate", via: [adv("GHSA-dddd-dddd-dddd", "moderate")] },
    },
  };
  const r = judge(report, { "GHSA-cccc-cccc-cccc": "checked" });
  assert.deepEqual(r.fail.map((a) => a.id).sort(), ["GHSA-aaaa-aaaa-aaaa", "GHSA-dddd-dddd-dddd"]);
  assert.deepEqual(r.low.map((a) => a.id), ["GHSA-bbbb-bbbb-bbbb"]);
  assert.deepEqual(r.allowed.map((a) => a.id), ["GHSA-cccc-cccc-cccc"]);
  assert.deepEqual(r.stale, []);
});

test("one advisory in several packages counts once; 'via another package' is skipped", () => {
  const report = {
    vulnerabilities: {
      lib: { severity: "high", via: [adv("GHSA-eeee-eeee-eeee", "high")] },
      tool: { severity: "high", via: ["lib", adv("GHSA-eeee-eeee-eeee", "high")] },
      wrapper: { severity: "high", via: ["tool"] },
    },
  };
  const r = judge(report, {});
  assert.equal(r.fail.length, 1);
  assert.deepEqual(r.fail[0].packages, ["lib", "tool"]);
});

test("an allowed advisory that is gone is reported as stale; an empty report passes", () => {
  const r = judge({ vulnerabilities: {} }, { "GHSA-ffff-ffff-ffff": "old" });
  assert.deepEqual(r, { fail: [], allowed: [], low: [], stale: ["GHSA-ffff-ffff-ffff"] });
  assert.deepEqual(judge({}, {}).fail, []);
});

test("an unknown severity is treated as high", () => {
  const r = judge({ vulnerabilities: { x: { via: [{ title: "?", url: "https://github.com/advisories/GHSA-gggg-gggg-gggg" }] } } }, {});
  assert.equal(r.fail.length, 1);
});
