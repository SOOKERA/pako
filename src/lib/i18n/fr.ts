// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako in French. Keys are the English words as the window writes them (`t("Pause all")`);
// values are the French, in one file per area (fr-*.ts) so each area is translated on its own.
// Typography (non-breaking spaces before : and inside « », narrow ones before ; ? !) is added by
// t(): write plain spaces here. Plurals: { one, other } ("{n} fichier" / "{n} fichiers"; French
// counts 0 and 1 as one). English plurals and keys that need a note live in ./en.ts.
//
// Tone: calm, short, friendly, « vous ». The same thing has the same word everywhere. Glossary
// (shared with Sookera's other apps where they overlap):
//   appli (app; never « app ») · Paramètres (Settings) · vous · Annuler · Enregistrer (Save) ·
//   Fermer · Ouvrir · Supprimer (Delete) · Retirer (Remove, from a list) · Réessayer (Try again)
//   téléchargement / télécharger (download) · Téléchargements (Downloads, the page) ·
//   Liens (Links, the page) · lien · Envoi (Upload, the page) · mettre en ligne / une mise en
//   ligne (upload; « envoyer » / un envoi: Direct and My devices) · pop-up (masculine: un pop-up)
//   · saisir (type a password, PIN or code: « Saisissez ») · les Paramètres (Pako's, capitalised)
//   · Direct (Direct, unchanged) · Mes appareils (My devices) · Tableau de bord (Dashboard) ·
//   Mode privé (Private mode) · dossier (folder) · fichier (file) · pause / Mettre en pause ·
//   Reprendre (Resume) · Priorité : D’abord / Normale / Plus tard (First / Normal / Later) ·
//   contact · appareil (device, a paired PC) · PC · barre des tâches (taskbar) · zone de
//   notification (tray) · Démarrer avec Windows · navigateur (browser) · extension ·
//   Ne pas déranger (Do not disturb) · Aide (Help) · Nouveautés (What's new) ·
//   Signaler un problème (Report a problem) · Tutoriel · Visite rapide (Quick tour) ·
//   clair / sombre (light / dark) · Comme Windows (Match Windows).
// Units: Ko, Mo, Go, To (KB, MB…); "/s" stays; decimal comma (format.ts).
import type { Entry } from "./core";
import { common } from "./fr-common.ts";
import { shell } from "./fr-shell.ts";
import { downloads } from "./fr-downloads.ts";
import { links } from "./fr-links.ts";
import { upload } from "./fr-upload.ts";
import { direct } from "./fr-direct.ts";
import { devices } from "./fr-devices.ts";
import { dashboard } from "./fr-dashboard.ts";
import { settings } from "./fr-settings.ts";
import { privacy } from "./fr-private.ts";
import { help } from "./fr-help.ts";
import { tours } from "./fr-tours.ts";
import { small } from "./fr-small.ts";

export const parts = { common, shell, downloads, links, upload, direct, devices, dashboard, settings, privacy, help, tours, small };
export const fr = {
  ...common,
  ...shell,
  ...downloads,
  ...links,
  ...upload,
  ...direct,
  ...devices,
  ...dashboard,
  ...settings,
  ...privacy,
  ...help,
  ...tours,
  ...small,
} satisfies Record<string, Entry>;
