// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// "Extract…" on a finished archive (.zip, .7z, .rar, or a split set's first part): 7-Zip
// extracts it next to it; one locked with a password asks for it (ExtractPassword).
import { api, type Download } from "./api";
import { app } from "./state.svelte";
import { extractable } from "./archives";
import { t, tIn, type Key } from "./i18n.svelte";

/** What a split set's first part says when it waits for its password (Rust: NEEDS_PASSWORD). */
export const NEEDS_PASSWORD = "Needs its password to extract";

/** Rust's note in either language (a note written before a switch keeps its words; Rust's French
 *  is the dictionary's, typography included). */
const said = (key: Key, m: string | null | undefined) => !!m && (m === key || m === tIn("fr", key));
/** The split set waits for its password. */
export const needsPassword = (m: string | null | undefined) => said(NEEDS_PASSWORD, m);
/** The split set was extracted (Rust: "Extracted"). */
export const isExtracted = (m: string | null | undefined) => said("Extracted", m);

/** The archive whose password is being asked, if any. */
export const extracting = $state<{ asking: Download | null }>({ asking: null });

/** A finished archive Pako can extract (not one in Private mode). */
export function canExtract(d: Download): boolean {
  return d.status === "completed" && !d.vault && extractable(d.name);
}

/** Extracts it; asks for its password when it has one. */
export async function extractNow(d: Download) {
  if (needsPassword(d.message)) {
    extracting.asking = d;
    return;
  }
  try {
    const out = await api.extractNow(d.id);
    app.toast(t("{name} is extracted.", { name: d.name }), "info", { label: t("Show"), run: () => api.openFolder(out) });
    await app.refresh();
  } catch (e) {
    if (String(e) === "password-needed" || String(e) === "password-wrong") extracting.asking = d;
    else app.toast(String(e), "error");
  }
}
