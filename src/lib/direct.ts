// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Small helpers shared by the sidebar's contact list and the Direct tab.
import type { DirectContact, DirectState } from "./api";
import { shortDate } from "./format";

/** How a connection travels: "local" (the home network), "direct6", "direct" or "relay". */
export function pathLabel(p: string | null | undefined) {
  return p === "local" ? "Local network" : p === "direct6" ? "Direct (IPv6)" : p === "direct" ? "Direct" : p === "relay" ? "Via relay" : "";
}

/** What Direct's state says beyond the contacts: the first start on this PC (Windows may ask
 *  about the firewall), whether the internet answers (without it, only contacts on the
 *  same network can be reached), and whether Pakos on the same network see you: "visible", or
 *  why not ("off": the setting, "public": Windows marks the network as public, "unknown":
 *  Windows didn't say). */
export type DirectExtra = { firstStart?: boolean; internet?: boolean; nearby?: "visible" | "off" | "public" | "unknown" };

/** The line under "Be visible to Pakos on this network" when you aren't (nothing when you are,
 *  or when you chose Never: the choice already says it). */
export function nearbyNote(n: DirectExtra["nearby"]) {
  if (n === "public") return "Not visible on this network (Windows marks it, or another network this PC is on, as public).";
  if (n === "unknown") return "Not visible on this network (Windows doesn't say it's a home network).";
  return "";
}
export const directExtra = (d: DirectState | null | undefined): DirectExtra => (d ?? {}) as DirectExtra;

/** The dot on a contact's initial: grey offline, green online, blue online on the same
 *  network, amber waiting for them to accept. */
export const isLocal = (c: DirectContact) => c.online && c.path === "local";

export function ago(ms: number | null) {
  if (!ms) return "not seen yet";
  const m = Math.round((Date.now() - ms) / 60000);
  if (m < 2) return "seen just now";
  if (m < 60) return `last seen ${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `last seen ${h} h ago`;
  return `last seen ${shortDate(ms)}`;
}

/** One line under a contact's name: online and how, waiting, or when last seen. `internet`:
 *  false while the internet is down (only contacts on the same network can be reached). */
export function statusOf(c: DirectContact, internet = true) {
  if (c.online) return `Online · ${pathLabel(c.path) || "connected"}`;
  if (c.waiting) return "Waiting for them to accept";
  if (c.declined) return c.note ?? "They declined. You can ask again after 7 days.";
  if (!internet) return "Not reachable without internet";
  if (!c.trusted) return c.note ?? "Not connected yet";
  // Not called in the background (30 days with nothing, or only when you ask).
  if (c.unchecked) return "Not checked lately";
  // Closed for being quiet: not offline, just not connected right now.
  if (c.idle) return ago(c.lastSeen).replace(/^./, (x) => x.toUpperCase());
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
