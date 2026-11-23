// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const shellEn: Record<string, Entry> = {
  // ---- Keys with a note (the same English, said differently in French) ----
  "Off (notify)": "Off",
  "Pop-up (notify)": "Pop-up",
  "Important (notify)": "Important",
  "Downloading (filter)": "Downloading",
  "Ready (filter)": "Ready",
  "Checking (filter)": "Checking",
  "Waiting for you (contacts)": "Waiting for you",
  "All downloads (title)": "All downloads",
  "Show (toast)": "Show",
  "Del (key)": "Del",
  "name (sorted by)": "name",
  "source (sorted by)": "source",
  "size (sorted by)": "size",
  "progress (sorted by)": "progress",
  "speed (sorted by)": "speed",
  "status (sorted by)": "status",
  "link (sorted by)": "link",
  "date (sorted by)": "date",

  // ---- Plurals ----
  "{n} days left": { one: "{n} day left", other: "{n} days left" },
  "Notifications, {n} unread": { one: "Notifications, {n} unread", other: "Notifications, {n} unread" },
  "{n} waiting to be picked": { one: "{n} waiting to be picked", other: "{n} waiting to be picked" },
  "The tag is taken off {n} contacts. The contacts themselves stay.": {
    one: "The tag is taken off {n} contact. The contacts themselves stay.",
    other: "The tag is taken off {n} contacts. The contacts themselves stay.",
  },
  "{n} problems": { one: "{n} problem", other: "{n} problems" },
  "{n} more links couldn't be added": { one: "{n} more link couldn't be added", other: "{n} more links couldn't be added" },
  "{n} links were already in Links": { one: "{n} link was already in Links", other: "{n} links were already in Links" },
  "{n} links caught from the clipboard": { one: "{n} link caught from the clipboard", other: "{n} links caught from the clipboard" },
  "{n} more links were skipped (too many at once)": {
    one: "{n} more link was skipped (too many at once)",
    other: "{n} more links were skipped (too many at once)",
  },
  "{n} links removed": { one: "{n} link removed", other: "{n} links removed" },
  "{n} uploads are running. They stop and don't continue after Pako closes: upload them again from Upload. One file or a ZIP starts over from the beginning; of several files, the ones already up stay on Gofile.": {
    one: "An upload is running. It stops and doesn't continue after Pako closes: upload it again from Upload. One file or a ZIP starts over from the beginning; of several files, the ones already up stay on Gofile.",
    other: "{n} uploads are running. They stop and don't continue after Pako closes: upload them again from Upload. One file or a ZIP starts over from the beginning; of several files, the ones already up stay on Gofile.",
  },
  "{n} downloads are running. They continue where they left off after the restart.": {
    one: "A download is running. It continues where it left off after the restart.",
    other: "{n} downloads are running. They continue where they left off after the restart.",
  },
  "{n} uploads are running": { one: "An upload is running", other: "{n} uploads are running" },
  "{n} downloads are running": { one: "A download is running", other: "{n} downloads are running" },
  "If you quit, the downloads stop and continue where they left off next time you open Pako.": {
    one: "If you quit, the download stops and continues where it left off next time you open Pako.",
    other: "If you quit, the downloads stop and continue where they left off next time you open Pako.",
  },
  "The uploads stop and don't continue next time: upload them again from Upload.": {
    one: "The upload stops and doesn't continue next time: upload it again from Upload.",
    other: "The uploads stop and don't continue next time: upload them again from Upload.",
  },
  "If you quit, the uploads stop and don't continue next time: upload them again from Upload.": {
    one: "If you quit, the upload stops and doesn't continue next time: upload it again from Upload.",
    other: "If you quit, the uploads stop and don't continue next time: upload them again from Upload.",
  },
  "The files stay in their folders unless you delete them too (to the Recycle Bin).": {
    one: "The file stays in its folder unless you delete it too (to the Recycle Bin).",
    other: "The files stay in their folders unless you delete them too (to the Recycle Bin).",
  },
  "Unfinished parts are deleted. The {n} finished files stay unless you delete them too (to the Recycle Bin).": {
    one: "Unfinished parts are deleted. The {n} finished file stays unless you delete them too (to the Recycle Bin).",
    other: "Unfinished parts are deleted. The {n} finished files stay unless you delete them too (to the Recycle Bin).",
  },
  "Delete files too": { one: "Delete file too", other: "Delete files too" },
  "These {n} downloads start again from zero.": {
    one: "This download starts again from zero.",
    other: "These {n} downloads start again from zero.",
  },
};
