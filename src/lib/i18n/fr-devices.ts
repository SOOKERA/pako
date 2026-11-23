// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for My devices (DevicesView…). See ./fr.ts for the glossary and the rules.
// Words: Mes appareils, appareil (a paired PC), associer / association (pair / pairing), code de
// sécurité (safety code), ID Pako, ordre (an order from another PC), « Télécharger sur <appareil> ».
import type { Entry } from "./core";

export const devices = {
  // ---- the page (DevicesView) ----
  "Your own PCs: send files between them, see and start their downloads": "Vos PC : échangez des fichiers, voyez et lancez leurs téléchargements",
  "Relaying {n} files to {name}": { one: "Relais de {n} fichier vers {name}", other: "Relais de {n} fichiers vers {name}" },
  "Wants to pair with this PC": "Veut s’associer à ce PC",
  "Waiting for you to confirm on it": "En attente de votre confirmation sur celui-ci",
  "Confirm on the other PC": "Confirmez sur l’autre PC",
  "This PC's Pako ID is copied. Enter it on your other PC, under My devices › Add a device.":
    "L’ID Pako de ce PC est copié. Saisissez-le sur votre autre PC, dans Mes appareils › Ajouter un appareil.",
  "This PC is now called {name} on your devices.": "Ce PC s’appelle maintenant {name} sur vos appareils.",
  "Preview mode: no folder picker.": "Mode aperçu : pas de choix de dossier.",
  "Where files from this device are saved": "Où sont enregistrés les fichiers de cet appareil",
  "Name this PC first: your other PC sees this name.": "Nommez d’abord ce PC : votre autre PC voit ce nom.",
  "Paired on this PC. Now confirm on the other one: it asks you there.": "Associé sur ce PC. Confirmez maintenant sur l’autre : il vous le demande là-bas.",
  "{name} is paired. Files between your two PCs now go without asking.": "{name} est associé. Les fichiers entre vos deux PC passent maintenant sans demander.",
  "Where files from {name} are saved": "Où sont enregistrés les fichiers de {name}",
  "{name} wants to pair with this PC as one of your devices.": "{name} veut s’associer à ce PC comme l’un de vos appareils.",
  "{name}, one of your Direct contacts, says this PC is one of its devices.": "{name}, l’un de vos contacts Direct, dit que ce PC est l’un de ses appareils.",
  "You already have a device or contact called {name}. This is a different PC: if you expected {name}, decline.":
    "Vous avez déjà un appareil ou un contact nommé {name}. C’est un autre PC : si vous attendiez {name}, refusez.",
  "Check that {name} shows the same safety code:": "Vérifiez que {name} affiche le même code de sécurité :",
  "If it doesn't, decline.": "Sinon, refusez.",
  "If it doesn't, don't confirm.": "Sinon, ne confirmez pas.",
  "Only for your own PCs: it can then send files here without asking, see and add to this PC's downloads, get finished ones, see your Direct contacts and your history with them, and send files to them through this PC.":
    "Seulement pour vos propres PC : il pourra alors envoyer des fichiers ici sans demander, voir les téléchargements de ce PC et en ajouter, récupérer ceux qui sont terminés, voir vos contacts Direct et votre historique avec eux, et leur envoyer des fichiers via ce PC.",
  "Its files are saved in": "Ses fichiers sont enregistrés dans",
  "This PC's name (like Desk PC)": "Le nom de ce PC (comme PC bureau)",
  "This PC's name": "Nom de ce PC",
  "This is my device": "C’est mon appareil",
  "Your PCs, together": "Tous vos PC, réunis",
  "Pair your own PCs once, like this one and your laptop. Then send files between them without anyone accepting, see the other PC's downloads live, and start downloads on it from here. Everything goes PC to PC, encrypted: on your home network it doesn't even use the internet.":
    "Associez une fois vos propres PC, comme celui-ci et votre portable. Ensuite, envoyez des fichiers entre eux sans que personne n’ait à accepter, voyez en direct les téléchargements de l’autre PC, et lancez-y des téléchargements d’ici. Tout passe de PC à PC, chiffré : sur votre réseau domestique, internet n’est même pas utilisé.",
  "This PC's Pako ID": "ID Pako de ce PC",
  "On your other PC, open My devices › Add a device and enter this ID, or enter that PC's ID here.":
    "Sur votre autre PC, ouvrez Mes appareils › Ajouter un appareil et saisissez cet ID, ou saisissez ici l’ID de cet autre PC.",
  "Add a device": "Ajouter un appareil",
  "Pair another of your PCs": "Associer un autre de vos PC",
  "Drop files here to send them to {name}": "Déposez des fichiers ici pour les envoyer à {name}",
  "Like Desk PC": "Comme PC bureau",
  "Rename this PC (your other PCs see this name)": "Renommer ce PC (vos autres PC voient ce nom)",
  "Name this PC": "Nommer ce PC",
  "Pair another of your own PCs. Do this on both PCs, or enter one PC's ID on the other: the other PC then asks you to confirm.":
    "Associez un autre de vos propres PC. Faites-le sur les deux PC, ou saisissez l’ID d’un PC sur l’autre : l’autre PC vous demande alors de confirmer.",
  "Like Desk PC (your other PC sees it)": "Comme PC bureau (votre autre PC le voit)",
  "The other PC's Pako ID": "ID Pako de l’autre PC",
  "On that PC: My devices › Add a device › Copy": "Sur cet autre PC : Mes appareils › Ajouter un appareil › Copier",
  "Its name": "Son nom",
  "Like Laptop or Home PC": "Comme Portable ou PC maison",
  "Files from it are saved in": "Ses fichiers sont enregistrés dans",
  "Files from this device are saved here, and only here. It can't pick another place on this PC.":
    "Les fichiers de cet appareil sont enregistrés ici, et seulement ici. Il ne peut pas choisir un autre emplacement sur ce PC.",
  "The other PC asks you to confirm and shows a safety code: check that it's the same on both PCs.":
    "L’autre PC vous demande de confirmer et affiche un code de sécurité : vérifiez qu’il est le même sur les deux PC.",
  "Pair this device": "Associer cet appareil",

  // ---- a device (DeviceDetails) ----
  "Wants to pair: confirm above": "Veut s’associer : confirmez ci-dessus",
  "Connected · not confirmed on it yet": "Connecté · pas encore confirmé sur celui-ci",
  "Confirm on the other PC (it asks there)": "Confirmez sur l’autre PC (il le demande là-bas)",
  "Preview mode: nothing is sent.": "Mode aperçu : rien n’est envoyé.",
  "Folders to send to {name}": "Dossiers à envoyer à {name}",
  "Files to send to {name}": "Fichiers à envoyer à {name}",
  "To finish pairing, confirm on {name}: it asks there, in My devices, with this safety code: {code}":
    "Pour terminer l’association, confirmez sur {name} : il le demande là-bas, dans Mes appareils, avec ce code de sécurité : {code}",
  "If it doesn't ask, update Pako on {name}: older Pakos don't know My devices.": "S’il ne demande rien, mettez à jour Pako sur {name} : les anciens Pako ne connaissent pas Mes appareils.",
  "Drop to send to {name}": "Déposez pour envoyer à {name}",
  "for {name}": "pour {name}",
  "Nothing sent between this PC and {name} yet.": "Rien n’a encore été envoyé entre ce PC et {name}.",
  "Drop files or folders here: they go straight to {name}, no Accept needed.": "Déposez des fichiers ou des dossiers ici : ils vont directement à {name}, sans rien à accepter.",
  "Files go without asking once both PCs confirmed.": "Les fichiers passent sans demander une fois que les deux PC ont confirmé.",
  "Drop files or folders anywhere here to send them to {name}.": "Déposez des fichiers ou des dossiers n’importe où ici pour les envoyer à {name}.",
  "{name} is offline: files go only while both PCs are on.": "{name} est hors ligne : les fichiers ne passent que quand les deux PC sont allumés.",
  "{name} doesn't let this PC start its downloads.": "{name} ne permet pas à ce PC de lancer ses téléchargements.",
  "{name} doesn't let this PC see or start its downloads.": "{name} ne permet pas à ce PC de voir ou de lancer ses téléchargements.",
  'On {name}: My devices › this PC › Settings, turn on "… can control my Pako".': "Sur {name} : Mes appareils › ce PC › Paramètres, activez « … peut contrôler mon Pako ».",
  "{name} is offline. Its downloads show here while it's on.": "{name} est hors ligne. Ses téléchargements s’affichent ici quand il est allumé.",
  "Asking {name} for its downloads…": "Demande de ses téléchargements à {name}…",
  "No downloads on {name}.": "Aucun téléchargement sur {name}.",
  "Files from {name} are saved in": "Les fichiers de {name} sont enregistrés dans",
  "Files from this device are saved here, and only here. {name} can't pick another place on this PC.":
    "Les fichiers de cet appareil sont enregistrés ici, et seulement ici. {name} ne peut pas choisir un autre emplacement sur ce PC.",
  "{name} can control my Pako": "{name} peut contrôler mon Pako",
  "It can see this PC's downloads, add links to them (into this PC's download folder, or a folder it names inside it), pause or resume them, change their priority, remove them, and get finished ones. It can also see your Direct contacts and your history with them, and send files to them through this PC. Nothing else: it can't open or run anything here, or look in other folders. Turned off, its next order is refused.":
    "Il peut voir les téléchargements de ce PC, y ajouter des liens (dans le dossier de téléchargement de ce PC, ou un dossier qu’il nomme à l’intérieur), les mettre en pause ou les reprendre, changer leur priorité, les retirer, et récupérer ceux qui sont terminés. Il peut aussi voir vos contacts Direct et votre historique avec eux, et leur envoyer des fichiers via ce PC. Rien d’autre : il ne peut rien ouvrir ni lancer ici, ni regarder dans d’autres dossiers. Désactivé, son prochain ordre est refusé.",
  "The same on both PCs.": "Le même sur les deux PC.",
  History: "Historique",
  "Clear (history)": "Effacer",
  "From this PC": "Depuis ce PC",
  "From {name}": "Depuis {name}",
  "Orders between this PC and {name} show here.": "Les ordres entre ce PC et {name} s’affichent ici.",
  "Remove this device": "Retirer cet appareil",
  "{name} stops being one of your devices: it can't send files without asking or control this Pako any more, and the connection closes. Files already received stay. To pair again later, add it again on both PCs.":
    "{name} n’est plus l’un de vos appareils : il ne peut plus envoyer de fichiers sans demander ni contrôler ce Pako, et la connexion se ferme. Les fichiers déjà reçus restent. Pour l’associer à nouveau plus tard, ajoutez-le à nouveau sur les deux PC.",
  "Clear the history?": "Effacer l’historique ?",
  "The list of orders between this PC and {name} is deleted from this PC. Files and downloads don't change.":
    "La liste des ordres entre ce PC et {name} est supprimée de ce PC. Les fichiers et les téléchargements ne changent pas.",

  // ---- a device › Links (DeviceLinks) ----
  "Mixed priority": "Priorités mélangées",
  "Mixed quality": "Qualités mélangées",
  "today {time}": "aujourd’hui {time}",
  "into {folder}": "dans {folder}",
  "into Downloads": "dans Téléchargements",
  "into several folders": "dans plusieurs dossiers",
  "Sent {when} · {into}": "Envoyé {when} · {into}",
  "{n} not taken": { one: "{n} non accepté", other: "{n} non acceptés" },
  "Shows how they're doing while {name} is on": "Leur avancement s’affiche quand {name} est allumé",
  "Asking for their state…": "Demande de leur état…",
  "{n} off the list": "{n} hors de la liste",
  "Reading the playlist…": "Lecture de la playlist…",
  "Reading the folder…": "Lecture du dossier…",
  "Opening the link page…": "Ouverture de la page du lien…",
  "{name} didn't take these {n}: {why}": { one: "{name} n’a pas pris celui-ci : {why}", other: "{name} n’a pas pris ces {n} : {why}" },
  "Pick what to send in the cards above.": { one: "Choisissez quoi envoyer dans la carte ci-dessus.", other: "Choisissez quoi envoyer dans les cartes ci-dessus." },
  "Hide the links": "Masquer les liens",
  "Show the {n} links": { one: "Afficher le lien", other: "Afficher les {n} liens" },
  "Not taken": "Non accepté",
  "Off its list": "Plus dans sa liste",
  "Sent · no live state from that Pako": "Envoyé · pas d’état en direct de ce Pako",
  "Tick {name}": "Cocher {name}",
  "Untick {name}": "Décocher {name}",
  "Quality of {name}": "Qualité de {name}",
  "Into {folder} on {name}": "Dans {folder} sur {name}",
  "Send {n} to {name}": "Envoyer {n} à {name}",
  "No links sent to {name} yet.": "Aucun lien envoyé à {name} pour l’instant.",
  "Paste links below: {name} downloads them into its own download folder. A playlist or a folder opens here first, to pick what to send.":
    "Collez des liens ci-dessous : {name} les télécharge dans son propre dossier de téléchargement. Une playlist ou un dossier s’ouvre d’abord ici, pour choisir quoi envoyer.",
  "Paste links for {name} to download": "Collez des liens à télécharger par {name}",
  "Links for {name} to download": "Liens à télécharger par {name}",
  Send: "Envoyer",
  "Quality for videos": "Qualité des vidéos",
  "Into folder": "Dans le dossier",
  "Optional, inside its download folder": "Facultatif, dans son dossier de téléchargement",
  "Into folder (optional)": "Dans le dossier (facultatif)",

  // ---- one download of a device (RemoteRow) ----
  "{done} of {total}": "{done} sur {total}",
  "{device} is sending {name}. It lands in {folder}.": "{device} envoie {name}. Il arrive dans {folder}.",
  "Send it to this PC, into {folder}": "L’envoyer à ce PC, dans {folder}",
  "Get this file": "Récupérer ce fichier",
  "Priority of {name}": "Priorité de {name}",
  "Remove from the list (a finished file stays on {device})": "Retirer de la liste (un fichier terminé reste sur {device})",

  // ---- "Download on ▾", sending files, the design preview (devices.svelte.ts) ----
  "Download on {name}": "Télécharger sur {name}",
  "Paired as your device": "Associé comme votre appareil",
  "Added {n} links to the downloads": { one: "{n} lien ajouté aux téléchargements", other: "{n} liens ajoutés aux téléchargements" },
  "Paused {name}": "Mis en pause : {name}",
  "Allowed to control this Pako": "Autorisé à contrôler ce Pako",
  "No longer allowed to control this Pako": "N’est plus autorisé à contrôler ce Pako",
  "Give this PC a name (like Desk PC).": "Donnez un nom à ce PC (comme PC bureau).",
  "Preview mode: nothing is sent to the other PC.": "Mode aperçu : rien n’est envoyé à l’autre PC.",
  "Preview mode: nothing is paired.": "Mode aperçu : rien n’est associé.",
  "Pako couldn't read what was dropped ({error}). Use Choose files.": "Pako n’a pas pu lire ce qui a été déposé ({error}). Utilisez Choisir des fichiers.",
  "Pako couldn't read what was dropped ({error}).": "Pako n’a pas pu lire ce qui a été déposé ({error}).",
} satisfies Record<string, Entry>;
