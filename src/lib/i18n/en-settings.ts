// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const settingsEn: Record<string, Entry> = {
  "Ready (feature)": "Ready",
  "{n} updates": { one: "{n} update", other: "{n} updates" },
  "{done} of {total} (bytes)": "{done} of {total}",
  "last used {n} days ago": { one: "last used {n} day ago", other: "last used {n} days ago" },
  "last used {n} weeks ago": { one: "last used {n} week ago", other: "last used {n} weeks ago" },
  "last used {n} months ago": { one: "last used {n} month ago", other: "last used {n} months ago" },
  "last used {n} years ago": { one: "last used {n} year ago", other: "last used {n} years ago" },
  "Add (Firefox button)": "Add",
  "Open (unlocked)": "Open",
};
