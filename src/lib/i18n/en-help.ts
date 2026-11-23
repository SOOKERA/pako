// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const helpEn: Record<string, Entry> = {
  // Report a problem: what each noted problem was about (a noun, not the buttons' verbs).
  "Download (problem area)": "Download",
  "Video (problem area)": "Video",
  "Link check (problem area)": "Link check",
  "Upload (problem area)": "Upload",
  "Direct (problem area)": "Direct",
  "Update (problem area)": "Update",
  "Browser extension (problem area)": "Browser extension",
  "Other (problem area)": "Other",
  // The theme, as the report names it.
  "Light (theme)": "Light",
  "Dark (theme)": "Dark",
  "The last {n} problems Pako noted. Links show only their site; file and folder names, IDs and your user name are left out.": {
    one: "The last problem Pako noted. Links show only their site; file and folder names, IDs and your user name are left out.",
    other: "The last {n} problems Pako noted. Links show only their site; file and folder names, IDs and your user name are left out.",
  },
  "Too many reports from this connection. Try again in {n} minutes, or use Send by email.": {
    one: "Too many reports from this connection. Try again in {n} minute, or use Send by email.",
    other: "Too many reports from this connection. Try again in {n} minutes, or use Send by email.",
  },
};
