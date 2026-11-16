// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Web pages opened in the user's browser (never inside Pako).
import { inApp } from "./api";
import { app } from "./state.svelte";

/** Sookera's pages about Pako. */
export const SITE = {
  studio: "https://sookera.com",
  privacy: "https://sookera.com/privacy",
  help: "https://pako.sookera.com/help",
  /** The licence Pako is shared under (its additional terms are in NOTICE.md with the code). */
  licence: "https://www.gnu.org/licenses/gpl-3.0.html",
  /** Pako's source code (a copy of each release; contributions aren't taken). */
  source: "https://github.com/sookera/pako",
};

export async function openWeb(url: string) {
  if (!/^https:\/\//.test(url)) return;
  if (inApp) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    openUrl(url).catch((e) => app.toast(`Couldn't open the page: ${e}`, "error"));
  } else window.open(url, "_blank", "noopener");
}
