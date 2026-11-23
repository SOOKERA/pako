// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note used by more than one area (see ./en.ts; the French is in
// ./fr-common.ts).
import type { Entry } from "./core";

export const commonEn: Record<string, Entry> = {
  "{n} days": { one: "{n} day", other: "{n} days" },
  "Upload (page)": "Upload",
  "{n} items": { one: "{n} item", other: "{n} items" },
  "{n} selected": { one: "{n} selected", other: "{n} selected" },
  "Enter (key)": "Enter",
  "{n} downloads": { one: "{n} download", other: "{n} downloads" },
  "{n} uploads": { one: "{n} upload", other: "{n} uploads" },
  "{n} files": { one: "{n} file", other: "{n} files" },
  "{n} done": { one: "{n} done", other: "{n} done" },
  "{n} picked": { one: "{n} picked", other: "{n} picked" },
  "{n} of {total} ready": { one: "{n} of {total} ready", other: "{n} of {total} ready" },
  "Uploaded (list)": "Uploaded",
  "{n} seconds": { one: "{n} second", other: "{n} seconds" },
  "{n} transfers": { one: "{n} transfer", other: "{n} transfers" },
  "{n} links": { one: "{n} link", other: "{n} links" },
  "{n} videos": { one: "{n} video", other: "{n} videos" },
  "{n} sends": { one: "{n} send", other: "{n} sends" },
};
