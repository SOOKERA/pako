// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Which What's new sections an update shows: every version since the one you had, not only the
// newest (an update from 1.2.1 to 1.2.5 shows 1.2.5, 1.2.4, 1.2.3 and 1.2.2).

/** "1.2.10" → [1, 2, 10]; "Pako 1.2.0" works too. Null for a heading that isn't a version. */
export function versionParts(text: string): number[] | null {
  const m = /^(?:Pako\s+)?(\d+)\.(\d+)\.(\d+)$/.exec(text.trim());
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

/** Negative when a is older than b, 0 when the same, positive when newer. */
export function compareVersions(a: number[], b: number[]): number {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return 0;
}

/** "Pako 1.2.0 " → "1.2.0"; what isn't a version stays as it is. */
export function plainVersion(text: string): string {
  const v = versionParts(text);
  return v ? v.join(".") : text;
}

/** The sections to show after updating from `seen` to `now`, newest first: every version newer
 *  than `seen` and not newer than `now`. Without a known `seen` (a Pako from before it noted it,
 *  or an unreadable one), only `now`'s own section. Headings that aren't versions ("Next
 *  version") never show, and a version written twice shows once (its first section). */
export function sinceSeen<T extends { version: string }>(entries: T[], seen: string | null | undefined, now: string): T[] {
  const nowParts = versionParts(now);
  if (!nowParts) return [];
  const seenParts = seen ? versionParts(seen) : null;
  const taken = new Set<string>();
  const picked = entries
    .map((e) => ({ e, v: versionParts(e.version) }))
    .filter((x): x is { e: T; v: number[] } => x.v !== null)
    .filter(({ v }) => compareVersions(v, nowParts) <= 0 && (seenParts ? compareVersions(v, seenParts) > 0 : compareVersions(v, nowParts) === 0))
    .filter(({ v }) => !taken.has(v.join(".")) && !!taken.add(v.join(".")));
  picked.sort((a, b) => compareVersions(b.v, a.v));
  return picked.map((x) => x.e);
}

/** The dialog's title: one version's own ("What's new in Pako 1.2.5"), or, for several, the one
 *  you had ("What's new since Pako 1.2.1"): "since your last update" read as "since this one". */
export function whatsNewTitle(shown: number, seen: string | null | undefined, now: string): string {
  if (shown > 1 && seen && versionParts(seen)) return `What's new since Pako ${plainVersion(seen)}`;
  return `What's new in Pako ${plainVersion(now)}`;
}
