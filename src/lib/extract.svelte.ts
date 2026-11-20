// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// "Extract…" on a finished archive (.zip, .7z, .rar, or a split set's first part): 7-Zip
// extracts it next to it; one locked with a password asks for it (ExtractPassword).
import { api, type Download } from "./api";
import { app } from "./state.svelte";
import { extractable } from "./archives";

/** What a split set's first part says when it waits for its password (Rust: NEEDS_PASSWORD). */
export const NEEDS_PASSWORD = "Needs its password to extract";

/** The archive whose password is being asked, if any. */
export const extracting = $state<{ asking: Download | null }>({ asking: null });

/** A finished archive Pako can extract (not one in Private mode). */
export function canExtract(d: Download): boolean {
  return d.status === "completed" && !d.vault && extractable(d.name);
}

/** Extracts it; asks for its password when it has one. */
export async function extractNow(d: Download) {
  if (d.message === NEEDS_PASSWORD) {
    extracting.asking = d;
    return;
  }
  try {
    const out = await api.extractNow(d.id);
    app.toast(`${d.name} is extracted.`, "info", { label: "Show", run: () => api.openFolder(out) });
    await app.refresh();
  } catch (e) {
    if (String(e) === "password-needed" || String(e) === "password-wrong") extracting.asking = d;
    else app.toast(String(e), "error");
  }
}
