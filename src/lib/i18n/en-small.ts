// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const smallEn: Record<string, Entry> = {
  // Keys with a note (the narrow tray list and Download with Pako windows keep their own French).
  "Downloads (tray tab)": "Downloads",
  "Uploads (tray tab)": "Uploads",
  "Pause all (tray)": "Pause all",
  "Download now (ask window)": "Download now",
  "them (no name)": "them",
  "a contact (no name)": "a contact",
  "your PC (no name)": "your PC",
  "now (time)": "now",
  // Plurals.
  "and {n} more in Pako": { one: "and {n} more in Pako", other: "and {n} more in Pako" },
  "{n} more waiting": { one: "{n} more waiting", other: "{n} more waiting" },
  "{n} more after this": { one: "{n} more after this", other: "{n} more after this" },
  "{name}: {why}. {n} links stay here (see its row).": {
    one: "{name}: {why}. {n} link stays here (see its row).",
    other: "{name}: {why}. {n} links stay here (see its row).",
  },
  "{n} links stay in Links: {why}": { one: "{n} link stays in Links: {why}", other: "{n} links stay in Links: {why}" },
};
