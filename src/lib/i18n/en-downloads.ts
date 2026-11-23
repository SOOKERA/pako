// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const downloadsEn: Record<string, Entry> = {
  "{n} connections": { one: "{n} connection", other: "{n} connections" },
  "{n} links found": { one: "{n} link found", other: "{n} links found" },
  "{n} starting": { one: "{n} starting", other: "{n} starting" },
  "{n} left: {hosts}": { one: "{n} left: {hosts}", other: "{n} left: {hosts}" },
  // Keys with a note.
  "Done (share)": "Done",
  "First (priority)": "First",
  "Normal (priority)": "Normal",
  "Later (priority)": "Later",
  "Finished (when)": "Finished",
};
