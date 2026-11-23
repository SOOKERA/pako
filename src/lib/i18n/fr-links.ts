// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for the Links page (FoundView, FoundDetails…). See ./fr.ts for the glossary and the rules.
// Words it shares with Downloads (column titles, item settings, preview) are in ./fr-downloads.ts.
// Here: pick = choisir, tick = cocher, broken (link) = mort, host = hébergeur.
import type { Entry } from "./core";

export const links = {
  // ---- Toolbar ----
  "Adding links…": "Ajout des liens…",
  "Nothing picked": "Rien de choisi",
  "Add links (or press Ctrl+V anywhere)": "Ajouter des liens (ou appuyez sur Ctrl+V n’importe où)",
  "Remove every link that doesn't work": "Retirer tous les liens qui ne marchent pas",
  "Clear broken ·": "Retirer les liens morts ·",
  "Remove the ticked links": "Retirer les liens cochés",
  "The downloads that started last step aside (nothing lost) and these start now":
    "Les derniers téléchargements lancés laissent la place (rien n’est perdu) et ceux-ci démarrent maintenant",
  "When they finish: Off, Pop-up, or Important (a big pop-up in the middle of the screen)":
    "À la fin : Désactivé, Pop-up ou Important (un grand pop-up au milieu de l’écran)",
  "Add to queue ·": "Ajouter à la file ·",
  "More download options": "Plus d’options de téléchargement",
  "Notify when done, or Download privately": "Prévenir une fois terminé, ou Télécharger en privé",
  "{n} files downloading in Private mode": { one: "{n} fichier se télécharge en mode privé", other: "{n} fichiers se téléchargent en mode privé" },
  "{n} files moved to Downloads": { one: "{n} fichier déplacé dans Téléchargements", other: "{n} fichiers déplacés dans Téléchargements" },
  "{n} broken links removed": { one: "{n} lien mort retiré", other: "{n} liens morts retirés" },
  "another PC": "un autre PC",

  // ---- Menus ----
  "Download {n}": "Télécharger {n}",
  "Download as Important": "Télécharger en Important",
  "Save to…": "Enregistrer dans…",
  "Remove only this folder": "Retirer seulement ce dossier",
  Esc: "Échap",
  "Tick all (Ctrl+A)": "Tout cocher (Ctrl+A)",
  "Untick all (Esc)": "Tout décocher (Échap)",
  "Clear broken links · {n}": "Retirer les liens morts · {n}",

  // ---- The list ----
  "No links yet": "Aucun lien pour l’instant",
  "Paste links here first: Pako checks them, then you pick what to download.":
    "Collez d’abord les liens ici : Pako les vérifie, puis vous choisissez quoi télécharger.",
  "Pick all files in {name}": "Choisir tous les fichiers de {name}",
  "Pick {name}": "Choisir {name}",
  "Unpick {name}": "Ne plus choisir {name}",
  PARTS: "PARTIES",
  "{n} parts": { one: "{n} partie", other: "{n} parties" },
  "Incomplete set": "Ensemble incomplet",
  "Split archive": "Archive en plusieurs parties",
  Opened: "Ouvert",
  "From a private window: kept only until you quit Pako, never sent to another PC, and a private download once downloaded":
    "D’une fenêtre privée : gardé seulement jusqu’à ce que vous quittiez Pako, jamais envoyé à un autre PC, et un téléchargement privé une fois téléchargé",
  private: "privé",
  "Downloads on {device}": "Se télécharge sur {device}",
  "Back to this PC": "De retour sur ce PC",
  "The same file on {n} hosts: pick the one to download from (the others are backups)":
    "Le même fichier sur {n} hébergeurs : choisissez celui d’où télécharger (les autres servent de secours)",
  "Pick the host · on {n} hosts": "Choisir l’hébergeur · sur {n} hébergeurs",
  // The row's chip (short: it sits next to the name).
  "on {n} hosts": "{n} hébergeurs",
  "Estimated size for the chosen quality": "Taille estimée pour la qualité choisie",

  // ---- How a link is doing (linkState.ts, the details panel) ----
  "Ready · Google limit today": "Prêt · limite Google aujourd’hui",
  "Checking link": "Vérification du lien",
  "Checking video…": "Vérification de la vidéo…",
  "Installing the video tools…": "Installation des outils vidéo…",
  "Needs the video tools": "Outils vidéo nécessaires",
  "click to install them (yt-dlp, Deno and ffmpeg, about 600 MB on disk)": "cliquez pour les installer (yt-dlp, Deno et ffmpeg, environ 600 Mo sur le disque)",
  "click to show check": "cliquez pour afficher la vérification",
  "click to show page": "cliquez pour afficher la page",
  "Opening link…": "Ouverture du lien…",
  "click to see the link page and finish it yourself": "cliquez pour voir la page du lien et la terminer vous-même",
  "click to open it there": "cliquez pour l’y ouvrir",
  "click to open it there ({why})": "cliquez pour l’y ouvrir ({why})",
  "{word}: {more}": "{word} : {more}",

  // ---- Already downloaded ----
  '"{name}" is already downloaded': "« {name} » est déjà téléchargé",
  "{n} of these are already downloaded": "{n} d’entre eux sont déjà téléchargés",
  "It's already in your downloads or in the folder. Downloading again keeps the old file and saves a second copy with a new name.":
    "Il est déjà dans vos téléchargements ou dans le dossier. Le télécharger à nouveau garde l’ancien fichier et enregistre une deuxième copie sous un nouveau nom.",
  "They're already in your downloads or in the folder. Downloading again keeps the old files and saves a second copy with a new name.":
    "Ils sont déjà dans vos téléchargements ou dans le dossier. Les télécharger à nouveau garde les anciens fichiers et enregistre une deuxième copie sous un nouveau nom.",
  "and {n} more": { one: "et {n} autre", other: "et {n} autres" },
  "Download again": "Télécharger à nouveau",
  "Skip it": "Le passer",
  "Skip them": "Les passer",

  // ---- Details panel ----
  "Size: checking…": "Taille : vérification…",
  "Size unknown": "Taille inconnue",
  Hosts: "Hébergeurs",
  "Host to download from": "Hébergeur d’où télécharger",
  "Host to download the parts from": "Hébergeur d’où télécharger les parties",
  "Pako downloads from the picked host. If it fails for good, Pako goes on from the next ready one by itself.":
    "Pako télécharge depuis l’hébergeur choisi. S’il échoue pour de bon, Pako continue de lui-même avec le prochain qui est prêt.",
  "The parts on several hosts download from the picked host. If one fails for good, Pako goes on from another host by itself.":
    "Les parties présentes sur plusieurs hébergeurs se téléchargent depuis celui choisi. Si l’un échoue pour de bon, Pako continue de lui-même avec un autre.",
  "Different for each": "Différent pour chacun",
  "{name} (offline)": "{name} (hors ligne)",
  "A PC no longer offered": "Un PC qui n’est plus proposé",
  " (in a folder named {name})": " (dans un dossier nommé {name})",
  "Download sends it to that PC, with priority set here, into that PC's own download folder{folder}.":
    "Télécharger l’envoie à ce PC, avec la priorité choisie ici, dans le dossier de téléchargement de ce PC{folder}.",
  "Download sends them to that PC, with priority set here, into that PC's own download folder{folder}.":
    "Télécharger les envoie à ce PC, avec la priorité choisie ici, dans le dossier de téléchargement de ce PC{folder}.",
  "Download sends it to that PC, with the quality and priority set here, into that PC's own download folder{folder}.":
    "Télécharger l’envoie à ce PC, avec la qualité et la priorité choisies ici, dans le dossier de téléchargement de ce PC{folder}.",
  "Download sends them to that PC, with the quality and priority set here, into that PC's own download folder{folder}.":
    "Télécharger les envoie à ce PC, avec la qualité et la priorité choisies ici, dans le dossier de téléchargement de ce PC{folder}.",
  "for {n} videos": "pour {n} vidéos",
  "Default: {quality}": "Par défaut : {quality}",
  "If a video doesn't have the quality, Pako takes the closest lower one.": "Si une vidéo n’a pas cette qualité, Pako prend la plus proche en dessous.",
  "Into the set's folder, next to the parts.": "Dans le dossier de l’ensemble, à côté des parties.",
  "Incomplete: {parts}. The set can't be extracted without it.": "Incomplet : {parts}. L’ensemble ne peut pas être extrait sans.",
  Length: "Durée",
  "More ways to download": "Autres façons de télécharger",
  "Select a link to see its details and settings.": "Sélectionnez un lien pour voir ses détails et ses paramètres.",
} satisfies Record<string, Entry>;
