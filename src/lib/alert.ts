// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// How a finished download tells you: Off · Pop-up · Important (Rust: model::Alert). Each download
// has its own choice; new ones start at Settings › Notifications › New downloads.

export type Alert = "off" | "pop" | "important";
/** Settings › Notifications › "Chime when a download finishes". */
export type DoneChime = "all" | "important" | "never";

export const ALERTS: { id: Alert; label: string; hint: string }[] = [
  { id: "off", label: "Off", hint: "No pop-up and no sound when it finishes" },
  { id: "pop", label: "Pop-up", hint: "A pop-up at the bottom right and a chime when it finishes" },
  { id: "important", label: "Important", hint: "A big pop-up in the middle of the screen that stays until you click it, and a chime" },
];

export const CHIMES: { id: DoneChime; label: string }[] = [
  { id: "all", label: "Pop-up and Important" },
  { id: "important", label: "Important only" },
  { id: "never", label: "Never" },
];

export const alertLabel = (a: Alert) => ALERTS.find((x) => x.id === a)?.label ?? "Pop-up";

/** "New downloads": as chosen, else from the older Download finished pop-up switch. */
export function doneAlert(s: { doneAlert?: Alert | null; notifyDone?: boolean } | null | undefined): Alert {
  if (s?.doneAlert) return s.doneAlert;
  return s?.notifyDone === false ? "off" : "pop";
}

/** "Chime when a download finishes": as chosen, else from the older arrived-sound switch. */
export function doneChime(s: { doneChime?: DoneChime | null; playDoneSound?: boolean } | null | undefined): DoneChime {
  if (s?.doneChime) return s.doneChime;
  return s?.playDoneSound === false ? "never" : "all";
}

/** A download's level: its own choice, else what an older list said ("Notify when done" =
 *  Pop-up, the window's "Notify me" off = Off), else the default. */
export function alertOf(d: { alert?: Alert | null; notify?: boolean; quiet?: boolean }, def: Alert): Alert {
  if (d.alert) return d.alert;
  if (d.notify) return "pop";
  if (d.quiet) return "off";
  return def;
}

/** The row's bell: each click goes one step on (Pop-up → Important → Off → Pop-up). */
export function nextAlert(a: Alert): Alert {
  return a === "pop" ? "important" : a === "important" ? "off" : "pop";
}

/** The level shared by several items, or null when they differ. */
export function sharedAlert(levels: Alert[]): Alert | null {
  return levels.length && levels.every((a) => a === levels[0]) ? levels[0] : null;
}

/** The right-click menu's three lines ("Notify: Off" · "Notify: Pop-up" · "Notify: Important"),
 *  the current one ticked (`now` null: several set differently, none ticked). */
export function alertMenu(now: Alert | null, pick: (a: Alert) => void): { label: string; icon: string; hint: string; action: () => void }[] {
  const icons: Record<Alert, string> = { off: "bell-off", pop: "bell", important: "bell" };
  return ALERTS.map((a) => ({ label: `Notify: ${a.label}`, icon: icons[a.id], hint: now === a.id ? "✓" : "", action: () => pick(a.id) }));
}
