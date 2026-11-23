// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const privacyEn: Record<string, Entry> = {
  // Plurals.
  "{n} decrypts": { one: "{n} decrypt", other: "{n} decrypts" },
  // Keys with a note (two uses of the same English).
  "Done (how far)": "Done",
  "Left (time)": "Left",
  "selected (after a count)": "selected",
  "{name} are in your Downloads now. (a folder)": "{name} are in your Downloads now.",
  "{name} are decrypted and off the private list. (a folder)": "{name} are decrypted and off the private list.",
};
