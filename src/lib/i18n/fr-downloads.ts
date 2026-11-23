// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for the Downloads page: the list, its columns, details panel, add dialog, rename, extract (Table, DetailsPanel, AddDialog, RenameDialog, ExtractPassword…). See ./fr.ts for the glossary and the rules.
// Words the Links page shares with it (column titles, the per-item settings, the preview) are here too.
import type { Entry } from "./core";

export const downloads = {
  // ---- Column titles (columns.ts: every list with columns) and sorting ----
  Type: "Type",
  Progress: "Progression",
  Date: "Date",
  "Added on": "Ajouté le",
  Columns: "Colonnes",
  "Columns: show or hide them": "Colonnes : les afficher ou les masquer",
  "Always shown": "Toujours affiché",
  "No room now": "Pas de place pour l’instant",
  "Reset columns": "Rétablir les colonnes",
  "Resize {column} column": "Redimensionner la colonne {column}",
  "Sort by {column}": "Trier par {column}",
  "{column}, sorted A to Z. {sort}": "{column}, trié de A à Z. {sort}",
  "{column}, sorted Z to A. {sort}": "{column}, trié de Z à A. {sort}",
  "{column}, sorted lowest first. {sort}": "{column}, trié du plus petit au plus grand. {sort}",
  "{column}, sorted highest first. {sort}": "{column}, trié du plus grand au plus petit. {sort}",

  // ---- The list ----
  "Unselect all": "Tout désélectionner",
  "Select all files in {name}": "Sélectionner tous les fichiers de {name}",
  "Collapse folder": "Replier le dossier",
  "Expand folder": "Déplier le dossier",
  "No downloads yet": "Aucun téléchargement pour l’instant",
  "Paste a web link, or a Google Drive file or folder link.": "Collez un lien web, ou le lien d’un fichier ou d’un dossier Google Drive.",
  "Tip: press {keys} anywhere to paste links.": "Astuce : appuyez sur {keys} n’importe où pour coller des liens.",
  'Nothing here for "{search}".': "Rien ici pour « {search} ».",
  "Nothing here.": "Rien ici.",
  "This folder can't be dragged out: some of its files aren't finished, or it isn't on the disk any more.":
    "Ce dossier ne peut pas être glissé hors de Pako : certains de ses fichiers ne sont pas terminés, ou il n’est plus sur le disque.",
  "This file isn't on the disk any more (moved or deleted).": "Ce fichier n’est plus sur le disque (déplacé ou supprimé).",
  "Drag to put the file in a folder or another app": "Faites glisser pour mettre le fichier dans un dossier ou une autre appli",
  "size unknown": "taille inconnue",
  "waiting for internet": "en attente d’internet",
  "retry in {n}s": "réessai dans {n} s",
  checking: "vérification",
  checked: "vérifié",
  // A folder's status with how many of its files have it.
  "{n} starting": { one: "{n} démarre", other: "{n} démarrent" },
  "{n} checking file": "{n} en vérification",
  "{n} reconnecting": "{n} en reconnexion",
  "{n} waiting its turn": "{n} en attente",
  "{n} google limit": "{n} limite Google",
  "{n} only in the browser": "{n} dans le navigateur seulement",
  "This site only lets your browser download this file: the browser downloads it (no multi-part speed) and Pako follows it.":
    "Ce site ne laisse que votre navigateur télécharger ce fichier : le navigateur le télécharge (sans la vitesse en plusieurs parties) et Pako le suit.",
  "from {host}": "depuis {host}",
  "Priority: First (starts before the others)": "Priorité : D’abord (démarre avant les autres)",
  "Priority: Later (runs when nothing else waits)": "Priorité : Plus tard (se lance quand rien d’autre n’attend)",
  "Notify when done: {level}": "Prévenir une fois terminé : {level}",
  "Notify when done: {level} (click to change)": "Prévenir une fois terminé : {level} (cliquez pour changer)",
  "Remove from the list (the file stays)": "Retirer de la liste (le fichier reste)",
  "Want to keep these behind a PIN? Private mode, in Settings.": "Garder ces fichiers derrière un code PIN ? Mode privé, dans les Paramètres.",
  "Want to keep these behind a PIN? {mode}, in Settings.": "Garder ces fichiers derrière un code PIN ? {mode}, dans les Paramètres.",
  "Close this tip": "Fermer cette astuce",

  // ---- Details panel ----
  "Done (share)": "Fait",
  Left: "Restant",
  "Google Drive (public link)": "Google Drive (lien public)",
  "Waiting for your browser to report this download": "En attente de votre navigateur, qui doit signaler ce téléchargement",
  "Your browser can't continue this download: open the link again in the browser": "Votre navigateur ne peut pas continuer ce téléchargement : rouvrez le lien dans le navigateur",
  "This site only lets your browser download this file, so Pako can't use its multi-part speed. The browser downloads it; Pako follows it and moves it to your folder when it's done.":
    "Ce site ne laisse que votre navigateur télécharger ce fichier, donc Pako ne peut pas utiliser sa vitesse en plusieurs parties. Le navigateur le télécharge ; Pako le suit et le déplace dans votre dossier une fois terminé.",
  "From a private window: it leaves this list once you open it or click its ✕, else {when}. The file stays. Pako keeps no other trace of it.":
    "D’une fenêtre privée : il quitte cette liste dès que vous l’ouvrez ou cliquez sur son ✕, sinon {when}. Le fichier reste. Pako n’en garde aucune autre trace.",
  "an hour after it finished": "une heure après la fin",
  "when you quit Pako": "quand vous quittez Pako",
  "10 minutes after it finished": "10 minutes après la fin",
  "Remove it from the list now": "Le retirer de la liste maintenant",
  "Continuing from {host}.": "Reprise depuis {host}.",
  "{host} didn't work: {why}": "{host} n’a pas marché : {why}",
  "{host} didn't work.": "{host} n’a pas marché.",
  "The split archive was extracted into this folder.": "L’archive en plusieurs parties a été extraite dans ce dossier.",
  "It's locked with a password: type it to extract the files.": "Elle est protégée par un mot de passe : saisissez-le pour extraire les fichiers.",
  "Google only gives this file to people it's shared with. Open it in your browser (signed in to that account) and download it there: Pako catches the download.":
    "Google ne donne ce fichier qu’aux personnes avec qui il est partagé. Ouvrez-le dans votre navigateur (connecté à ce compte) et téléchargez-le là : Pako récupère le téléchargement.",
  "Open in browser": "Ouvrir dans le navigateur",
  "More about files shared with certain people": "En savoir plus sur les fichiers partagés avec certaines personnes",
  "Too many people downloaded this file today. Pako tries again by itself every few hours, or get it now from your own Drive:":
    "Trop de personnes ont téléchargé ce fichier aujourd’hui. Pako réessaie de lui-même toutes les quelques heures, ou récupérez-le maintenant depuis votre propre Drive :",
  "Open it in Drive.": "Ouvrez-le dans Drive.",
  "Click {shortcut}, then in your Drive right-click it › {copy}.": "Cliquez sur {shortcut}, puis dans votre Drive faites un clic droit dessus › {copy}.",
  "Add shortcut to Drive": "Ajouter un raccourci dans Drive",
  "Make a copy": "Créer une copie",
  "Download your copy: Pako catches it. Delete the copy afterwards to free the space.": "Téléchargez votre copie : Pako la récupère. Supprimez la copie ensuite pour libérer la place.",
  "Open in Drive": "Ouvrir dans Drive",
  "More about Google's download limit": "En savoir plus sur la limite de téléchargement de Google",
  Parts: "Parties",
  "{n} connections": { one: "{n} connexion", other: "{n} connexions" },
  "Which parts of the file are downloaded": "Les parties du fichier déjà téléchargées",
  "Your browser downloads it: Pako's queue doesn't apply.": "Votre navigateur le télécharge : la file d’attente de Pako ne s’applique pas.",
  Backups: "Secours",
  "The same file on other hosts: if this host fails for good, Pako goes on from the next one":
    "Le même fichier sur d’autres hébergeurs : si celui-ci échoue pour de bon, Pako continue avec le suivant",
  "{n} left: {hosts}": { one: "{n} restant : {hosts}", other: "{n} restants : {hosts}" },
  "None left": "Plus aucun",
  Tried: "Essayés",
  "{host}: {why}": "{host} : {why}",
  "didn't work": "n’a pas marché",
  "Saved to": "Enregistré dans",
  "File check": "Contrôle du fichier",
  "Compared with Google's own record of the file (MD5)": "Comparé à la référence du fichier chez Google (MD5)",
  "Passed: the file is complete": "Réussi : le fichier est complet",
  "Checked when it finishes": "Contrôlé à la fin",
  "Audio only ({format})": "Audio seul ({format})",
  "Up to {q}p (default)": "Jusqu’à {q}p (par défaut)",
  "After a pause": "Après une pause",
  "Continues where it stopped": "Reprend là où il s’est arrêté",
  "Downloaded by": "Téléchargé par",
  "Your browser": "Votre navigateur",
  "As your browser can": "Selon ce que permet votre navigateur",
  "Starts again from zero (this server can't continue)": "Recommence de zéro (ce serveur ne peut pas reprendre)",
  "Finished (when)": "Terminé",
  "Extract when all parts are done": "Extraire quand toutes les parties sont terminées",
  "Into this folder, next to the parts.": "Dans ce dossier, à côté des parties.",
  "Needs 7-Zip (Settings › Features & updates).": "Nécessite 7-Zip (Paramètres › Fonctions et mises à jour).",
  "Incomplete: {parts}.": "Incomplet : {parts}.",
  "Select a download to see its details.": "Sélectionnez un téléchargement pour voir ses détails.",

  // ---- Per-item settings (both details panels) ----
  default: "par défaut",
  "Back to the default folder": "Revenir au dossier par défaut",
  "Already started: it stays in this folder.": "Déjà commencé : il reste dans ce dossier.",
  "First (priority)": "D’abord",
  "Normal (priority)": "Normale",
  "Later (priority)": "Plus tard",
  "A big pop-up in the middle of the screen that stays until you click it.": "Un grand pop-up au milieu de l’écran, qui reste jusqu’à ce que vous cliquiez dessus.",

  // ---- Video preview (both details panels) ----
  Preview: "Aperçu",
  "short clip": "court extrait",
  "as it is": "tel quel",
  "Play a preview": "Lire un aperçu",
  "Asking the site…": "Demande au site…",
  "Play a few seconds to check it's the right video": "Lisez quelques secondes pour vérifier que c’est la bonne vidéo",
  "Play a short check before downloading": "Lisez un court aperçu avant de télécharger",
  "This video can't be previewed. The site didn't send it.": "Cette vidéo ne peut pas avoir d’aperçu. Le site ne l’a pas envoyée.",
  "About 20 seconds at 480p, fetched from the site when you press play.": "Environ 20 secondes en 480p, demandées au site quand vous appuyez sur lecture.",
  "Preview quality": "Qualité de l’aperçu",
  "Needs the video tools: Settings › Features & updates › Download videos from YouTube and other sites.":
    "Nécessite les outils vidéo : Paramètres › Fonctions et mises à jour › Télécharger des vidéos depuis YouTube et d’autres sites.",
  "MP4 and WebM play as they are. For other kinds of video and for {q}p, install {tool} in Settings › Features & updates.":
    "Les MP4 et WebM se lisent tels quels. Pour les autres types de vidéo et pour le {q}p, installez {tool} dans Paramètres › Fonctions et mises à jour.",

  // ---- Add links ----
  "Paste links, or any text that contains links (a message, an email, a web page). Google Drive files and folders work too.":
    "Collez des liens, ou n’importe quel texte qui en contient (un message, un e-mail, une page web). Les fichiers et dossiers Google Drive marchent aussi.",
  "{n} links found": { one: "{n} lien trouvé", other: "{n} liens trouvés" },
  "No links found in this text": "Aucun lien trouvé dans ce texte",
  "Links are picked out automatically": "Les liens sont repérés automatiquement",
  "They go to Links first: you see names and sizes, pick what you want, then download.":
    "Ils passent d’abord par Liens : vous voyez les noms et les tailles, choisissez ce que vous voulez, puis téléchargez.",
  'Drive links shared with "anyone with the link" open right here, folders too. Files only shared with some people open in your browser instead.':
    "Les liens Drive partagés avec « Tous les utilisateurs disposant du lien » s’ouvrent ici même, dossiers compris. Les fichiers partagés seulement avec certaines personnes s’ouvrent plutôt dans votre navigateur.",
  "Adding…": "Ajout…",
  "Add {n} links": "Ajouter {n} liens",
  "Add link": "Ajouter le lien",

  // ---- Rename ----
  'A file with that name exists: renamed to "{name}"': "Un fichier porte déjà ce nom : renommé en « {name} »",
  'Saved as "{name}"': "Enregistré sous « {name} »",

  // ---- Extract ----
  "{name} is extracted.": "{name} est extrait.",
  "Password for {name}": "Mot de passe pour {name}",
  "This archive is locked with a password. Type it to extract the files. Pako hands it to 7-Zip only and doesn't keep it.":
    "Cette archive est protégée par un mot de passe. Saisissez-le pour extraire les fichiers. Pako le donne seulement à 7-Zip et ne le garde pas.",
  "Hide the password": "Masquer le mot de passe",
  "Show the password": "Afficher le mot de passe",
  Hide: "Masquer",
  "That password isn't right. Check it and try again.": "Ce mot de passe n’est pas le bon. Vérifiez-le et réessayez.",
  "Extracting…": "Extraction…",
  Extract: "Extraire",
  // Rust's notes on a split set's first part (engine.rs NEEDS_PASSWORD, "Extracted"): the window
  // matches them in either language, so Rust's French must be exactly these words.
  "Needs its password to extract": "Mot de passe nécessaire pour extraire",
  Extracted: "Extrait",
} satisfies Record<string, Entry>;
