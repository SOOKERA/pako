// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const devicesEn: Record<string, Entry> = {
  // A key with a note: "Clear" the history (French « Effacer »; the basket's Clear is « Vider »).
  "Clear (history)": "Clear",
  "Relaying {n} files to {name}": { one: "Relaying {n} file to {name}", other: "Relaying {n} files to {name}" },
  "Added {n} links to the downloads": { one: "Added {n} link to the downloads", other: "Added {n} links to the downloads" },
  "{name} didn't take these {n}: {why}": { one: "{name} didn't take this one: {why}", other: "{name} didn't take these {n}: {why}" },
  "Pick what to send in the cards above.": { one: "Pick what to send in the card above.", other: "Pick what to send in the cards above." },
  "Show the {n} links": { one: "Show the link", other: "Show the {n} links" },
  // The same English either way; the French agrees ("terminé" / "terminés").
  "{n} not taken": { one: "{n} not taken", other: "{n} not taken" },
};
