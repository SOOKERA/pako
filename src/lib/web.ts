// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Web pages opened in the user's browser (never inside Pako).
import { inApp } from "./api";
import { app } from "./state.svelte";
import { t } from "./i18n.svelte";

/** Sookera's pages about Pako. */
export const SITE = {
  studio: "https://sookera.com",
  privacy: "https://sookera.com/privacy",
  /** The terms of use (TERMS.md, word for word). */
  terms: "https://sookera.com/terms",
  help: "https://pako.sookera.com/help",
  /** The licence Pako is shared under (its additional terms are in NOTICE.md with the code). */
  licence: "https://www.gnu.org/licenses/gpl-3.0.html",
  /** Pako's source code (a copy of each release; contributions aren't taken). */
  source: "https://github.com/sookera/pako",
};

/** Sookera's support address, as the terms of use write it (a mailto: link). */
const SUPPORT_MAIL = "mailto:support@sookera.com";

/** A link in a text Pako shows (the terms, NOTICE.md): web pages in the browser, Sookera's
 *  support address in the email app. Anything else does nothing. */
export async function openLink(href: string) {
  if (href === SUPPORT_MAIL && inApp) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    openUrl(href).catch(() => app.toast(() => t("Couldn't open your email app. Write to support@sookera.com."), "error"));
  } else openWeb(href);
}

export async function openWeb(url: string) {
  if (!/^https:\/\//.test(url)) return;
  if (inApp) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    openUrl(url).catch((e) => app.toast(() => t("Couldn't open the page: {error}", { error: String(e) }), "error"));
  } else window.open(url, "_blank", "noopener");
}
