// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Small helpers shared by the sidebar's contact list and the Direct tab.
import type { DirectContact } from "./api";
import { shortDate } from "./format";

export function pathLabel(p: string | null | undefined) {
  return p === "direct6" ? "Direct (IPv6)" : p === "direct" ? "Direct" : p === "relay" ? "Via relay" : "";
}

export function ago(ms: number | null) {
  if (!ms) return "not seen yet";
  const m = Math.round((Date.now() - ms) / 60000);
  if (m < 2) return "seen just now";
  if (m < 60) return `last seen ${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `last seen ${h} h ago`;
  return `last seen ${shortDate(ms)}`;
}

/** One line under a contact's name: online and how, waiting, or when last seen. */
export function statusOf(c: DirectContact) {
  if (c.online) return `Online · ${pathLabel(c.path) || "connected"}`;
  if (c.waiting) return "Waiting for them to accept";
  if (c.declined) return c.note ?? "They declined. You can ask again after 7 days.";
  if (!c.trusted) return c.note ?? "Not connected yet";
  return `Offline · ${ago(c.lastSeen)}`;
}

export const initial = (name: string) => (name.trim()[0] ?? "?").toUpperCase();

/** How far a transfer is, in whole percent, the same everywhere it shows (a contact's row, the
 *  conversation): rounded down, so 100% only once all of it is there. */
export const percent = (done: number, size: number) => (size > 0 ? Math.min(100, Math.floor((Math.min(done, size) / size) * 100)) : 0);

/** Online first, then waiting, then the rest; by name within each group. */
export function sortContacts(list: DirectContact[]) {
  const rank = (c: DirectContact) => (c.online ? 0 : c.waiting ? 1 : 2);
  return [...list].sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
}

/** The sidebar's filters for Direct: "all", "online", "waiting", "favourites", or "tag:<name>". */
export type DirectFilter = string;

/** Does this contact show under this filter? `waitingIds` = contacts with something waiting for you. */
export function matchesFilter(c: DirectContact, filter: DirectFilter, waitingIds: Set<string>) {
  if (filter === "online") return c.online;
  if (filter === "waiting") return waitingIds.has(c.id);
  if (filter === "favourites") return c.favourite;
  if (filter.startsWith("tag:")) {
    const t = filter.slice(4).toLowerCase();
    return (c.tags ?? []).some((x) => x.toLowerCase() === t);
  }
  return true;
}

/** Ready-made tags offered while you have none yet. */
export const SUGGESTED_TAGS = ["Friends", "Family", "Work"];
