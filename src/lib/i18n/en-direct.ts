// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const directEn: Record<string, Entry> = {
  "{n} contacts": { one: "{n} contact", other: "{n} contacts" },
  "{n} folders": { one: "{n} folder", other: "{n} folders" },
  "and {n} more files": { one: "and {n} more file", other: "and {n} more files" },
  "and {n} more not listed": { one: "and {n} more not listed", other: "and {n} more not listed" },
};
