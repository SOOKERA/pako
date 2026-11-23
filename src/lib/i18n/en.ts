// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English where the key alone is not the words: plurals ({ one, other }: "{n} files" shows "1 file")
// and keys that carry a note to tell two uses apart ("Next (tour)" shows "Next"). Every other
// English string is its own key (see ../i18n.svelte.ts). One file per area (en-*.ts), like the French.
import type { Entry } from "./core";
import type { fr } from "./fr.ts";
import { commonEn } from "./en-common.ts";
import { shellEn } from "./en-shell.ts";
import { downloadsEn } from "./en-downloads.ts";
import { linksEn } from "./en-links.ts";
import { uploadEn } from "./en-upload.ts";
import { directEn } from "./en-direct.ts";
import { devicesEn } from "./en-devices.ts";
import { dashboardEn } from "./en-dashboard.ts";
import { settingsEn } from "./en-settings.ts";
import { privacyEn } from "./en-private.ts";
import { helpEn } from "./en-help.ts";
import { toursEn } from "./en-tours.ts";
import { smallEn } from "./en-small.ts";

export const en: Partial<Record<keyof typeof fr, Entry>> = { ...commonEn, ...shellEn, ...downloadsEn, ...linksEn, ...uploadEn, ...directEn, ...devicesEn, ...dashboardEn, ...settingsEn, ...privacyEn, ...helpEn, ...toursEn, ...smallEn } as Partial<Record<keyof typeof fr, Entry>>;
