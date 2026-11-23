// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const dashboardEn: Record<string, Entry> = {
  "yet (all time)": "yet",
  "{from} to {to} (hours)": "{from} to {to}",
  "Saved {n} transfers to the file.": { one: "Saved {n} transfer to the file.", other: "Saved {n} transfers to the file." },
  "{n} stopped": { one: "{n} stopped", other: "{n} stopped" },
  "People downloaded these links {n} times (as Gofile last said)": {
    one: "People downloaded these links {n} time (as Gofile last said)",
    other: "People downloaded these links {n} times (as Gofile last said)",
  },
  "to {n} people": { one: "to {n} person", other: "to {n} people" },
  "from {n} people": { one: "from {n} person", other: "from {n} people" },
  "{n} transfers with {name}, {size} in all": { one: "{n} transfer with {name}, {size} in all", other: "{n} transfers with {name}, {size} in all" },
  "{site}: {n} downloads, {size}": { one: "{site}: {n} download, {size}", other: "{site}: {n} downloads, {size}" },
};
