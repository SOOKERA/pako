// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for the Direct page and its prompts (DirectView, DirectPrompts…). See ./fr.ts for the glossary and the rules.
// Words: Direct (the page, unchanged) · contact · ID Pako (Pako ID) · Accepter / Refuser ·
// en ligne / hors ligne · Réseau local / Direct / Via relais (how a connection travels) ·
// Étiquettes (Tags) · En sourdine / Mettre en sourdine (Muted / Mute) · Vérifier (Check, how
// Pako calls a contact: Automatique / Souvent… / Manuel) · réseau domestique (home network).
// "Pakos" (several copies of Pako) stays « Pako » in French: des Pako, vos deux Pako.
import type { Entry } from "./core";

export const direct = {
  // ---- The page ----
  "Files straight to people you know, PC to PC": "Des fichiers directement aux personnes que vous connaissez, de PC à PC",
  "Direct is starting…": "Direct démarre…",
  "Windows may ask whether Pako can use the network. Allow it on private networks, so Pakos on your home network can reach you.":
    "Windows peut demander si Pako peut utiliser le réseau. Autorisez-le sur les réseaux privés, pour que les Pako de votre réseau domestique puissent vous joindre.",
  "Send files straight to people you know": "Envoyez des fichiers directement aux personnes que vous connaissez",
  "Files go PC to PC, encrypted, with nothing stored anywhere. Share your Pako ID with a friend, or add theirs: the first time they're asked to accept, then you connect by yourselves.":
    "Les fichiers vont de PC à PC, chiffrés, sans rien stocker nulle part. Partagez votre ID Pako avec un ami, ou ajoutez le sien : la première fois, on lui demande d’accepter, puis vous vous connectez tout seuls.",
  "Your Pako ID": "Votre ID Pako",
  "Add contact": "Ajouter un contact",
  "Add a contact": "Ajouter un contact",
  "Add a contact by their Pako ID": "Ajouter un contact par son ID Pako",
  "Your Pako ID is copied. Send it to the people you want to connect with.": "Votre ID Pako est copié. Envoyez-le aux personnes avec qui vous voulez vous connecter.",
  "Made-up people, only shown during the tutorial": "Des personnes inventées, montrées seulement pendant le tutoriel",
  Examples: "Exemples",

  // ---- Seen on this network ----
  "Be visible to Pakos on this network": "Être visible des Pako sur ce réseau",
  "Pakos on the same Wi-Fi or cable can find you there, even without the internet. Either way, you still find your contacts there.":
    "Les Pako sur le même Wi-Fi ou le même câble peuvent vous y trouver, même sans internet. Dans tous les cas, vous y trouvez toujours vos contacts.",
  "On home networks": "Sur les réseaux domestiques",
  Always: "Toujours",
  "Not visible on this network (Windows marks it, or another network this PC is on, as public).":
    "Pas visible sur ce réseau (Windows le marque comme public, ou un autre réseau de ce PC).",
  "Not visible on this network (Windows doesn't say it's a home network).": "Pas visible sur ce réseau (Windows n’indique pas que c’est un réseau domestique).",

  // ---- You, sending from ----
  "You, sending from": "Vous, qui envoyez depuis",
  "What your contacts see": "Ce que vos contacts voient",
  "Your name": "Votre nom",
  "The name {pc}'s contacts see. Change it on {pc}.": "Le nom que voient les contacts de {pc}. Changez-le sur {pc}.",
  "Your name: what your contacts see. Click to change it.": "Votre nom : ce que vos contacts voient. Cliquez pour le changer.",
  "Send from this PC or another of your PCs": "Envoyer depuis ce PC ou un autre de vos PC",
  "That PC": "Ce PC",
  "that PC": "ce PC",
  "your other PC": "votre autre PC",
  'Contacts see "{name}" · {count}': "Les contacts voient « {name} » · {count}",
  "no name yet": "pas encore de nom",
  "{n} contacts": { one: "{n} contact", other: "{n} contacts" },
  "calls itself {name}": "se nomme {name}",
  "Renaming stays in My devices.": "Pour renommer, c’est dans Mes appareils.",

  // ---- The list of people ----
  Favourite: "Favori",
  "Search contacts": "Chercher dans les contacts",
  Muted: "En sourdine",
  "Muted: no pop-ups or sounds from {name}": "En sourdine : aucun pop-up ni aucun son de {name}",
  "{n} transfers with {name}": "{n} transferts avec {name}",
  'Nobody matches "{q}".': "Personne ne correspond à « {q} ».",
  "Reading {pc}'s contacts…": "Lecture des contacts de {pc}…",
  "{pc} has no contacts here.": "{pc} n’a pas de contacts ici.",
  "Nobody here right now.": "Personne ici pour l’instant.",
  "Nobody matches your search.": "Personne ne correspond à votre recherche.",
  "Nobody under this filter. Pick another one on the left.": "Personne dans ce filtre. Choisissez-en un autre à gauche.",
  "Sending and receiving {pct}%": "Envoi et réception {pct}%",

  // ---- A contact's status line ----
  "Waiting for them to accept": "En attente de son accord",
  "They declined. You can ask again after 7 days.": "La personne a refusé. Vous pourrez redemander dans 7 jours.",
  "Not connected yet": "Pas encore connecté",
  "Not checked lately": "Pas vérifié récemment",
  "not seen yet": "pas encore vu",
  "seen just now": "vu à l’instant",
  "last seen {m} min ago": "vu il y a {m} min",
  "last seen {h} h ago": "vu il y a {h} h",
  "last seen {date}": "vu : {date}",
  "Direct (IPv6)": "Direct (IPv6)",
  "Via relay": "Via relais",

  // ---- The conversation ----
  'calls themselves "{name}"': "se fait appeler « {name} »",
  "can send without asking": "peut envoyer sans demander",
  "a contact of {pc}: files go through it": "un contact de {pc} : les fichiers passent par lui",
  "Check now": "Vérifier maintenant",
  "Hide details (Ctrl+I)": "Masquer les détails (Ctrl+I)",
  "Show details (Ctrl+I)": "Afficher les détails (Ctrl+I)",
  "{name} declined. For 7 days their Pako turns you away without asking them; after that, press Connect to ask again.":
    "{name} a refusé. Pendant 7 jours, son Pako vous refuse sans lui demander ; ensuite, appuyez sur Connecter pour redemander.",
  "The first time, {name} is asked to accept. Compare this code with theirs once (by voice or chat):":
    "La première fois, on demande à {name} d’accepter. Comparez une fois ce code avec le sien (de vive voix ou par message) :",
  "Drop files or folders to send them to {name}": "Déposez des fichiers ou des dossiers pour les envoyer à {name}",
  "Sent: {name}": "Envoyé : {name}",
  "Received: {name}": "Reçu : {name}",
  "Sending: {name}": "En cours d’envoi : {name}",
  "Receiving: {name}": "En cours de réception : {name}",
  "Sent through {pc}: {name}": "Envoyé via {pc} : {name}",
  "Sent on for {name}": "Transmis pour {name}",
  "from {name}": "depuis {name}",
  "Through {pc}": "Via {pc}",
  "through {pc}": "via {pc}",
  "Waiting for them to accept…": "En attente de son accord…",
  "{pct}% · {done} of {size}": "{pct}% · {done} sur {size}",
  "Arriving from {pc} {pct}% · then to {name}": "Arrive de {pc} {pct}% · puis vers {name}",
  "From {pc} · Waiting for {name} to be online": "De {pc} · En attente que {name} soit en ligne",
  "To {pc} {pct}%": "Vers {pc} {pct}%",
  "{leg1} · Waiting for {name}": "{leg1} · En attente de {name}",
  "{leg1} · Waiting for {name} to accept…": "{leg1} · En attente que {name} accepte…",
  "{leg1} · To {name} {pct}%": "{leg1} · Vers {name} {pct}%",
  "{leg1} · Waiting for {name} to continue · {pct}% done": "{leg1} · En attente que {name} reprenne · déjà {pct}%",
  "Expired: not accepted within 7 days": "Expiré : pas accepté sous 7 jours",
  "Reading {pc}'s history with {name}…": "Lecture de l’historique de {pc} avec {name}…",
  "Nothing sent or received between {pc} and {name} yet.": "Rien d’envoyé ni de reçu entre {pc} et {name} pour l’instant.",
  "Nothing sent or received with {name} yet.": "Rien d’envoyé ni de reçu avec {name} pour l’instant.",

  // ---- The send area ----
  "Name (optional)": "Nom (facultatif)",
  "Name of this send": "Nom de cet envoi",
  "Once {name} has them, a copy stays on {pc} (in its folder for files from this PC). Off: the copy there is deleted.":
    "Une fois que {name} les a, une copie reste sur {pc} (dans son dossier pour les fichiers de ce PC). Désactivé : la copie là-bas est supprimée.",
  "Keep a copy on {pc}": "Garder une copie sur {pc}",
  "Send to {name} through {pc}": "Envoyer à {name} via {pc}",
  "Send to {name}": "Envoyer à {name}",
  "{pc} is offline. Drop files or folders here, or": "{pc} est hors ligne. Déposez fichiers ou dossiers ici, ou",
  "{name} hasn't accepted {pc} yet. Drop files or folders here, or": "{name} n’a pas encore accepté {pc}. Déposez fichiers ou dossiers ici, ou",
  "{pc} passes them on once {name} is back. Drop files or folders here, or": "{pc} les transmet dès que {name} revient. Déposez fichiers ou dossiers ici, ou",
  "Drop files or folders here, or": "Déposez fichiers ou dossiers ici, ou",
  "{name} is offline: files go only while you're both online. Drop files or folders here, or":
    "{name} est hors ligne : les fichiers ne partent que quand vous êtes tous deux en ligne. Déposez fichiers ou dossiers ici, ou",
  "Choose files to send": "Choisir les fichiers à envoyer",
  "Choose folders to send": "Choisir les dossiers à envoyer",
  "Pako couldn't read what was dropped ({detail}).": "Pako n’a pas pu lire ce qui a été déposé ({detail}).",

  // ---- A contact's menu and windows ----
  "Tags…": "Étiquettes…",
  "Remove from favourites": "Retirer des favoris",
  "Add to favourites": "Ajouter aux favoris",
  Mute: "Mettre en sourdine",
  Unmute: "Retirer la sourdine",
  "Check often for 1 hour": "Vérifier souvent pendant 1 heure",
  "Can send to me without asking": "Peut m’envoyer sans demander",
  "Save their files in…": "Enregistrer ses fichiers dans…",
  "Clear history…": "Effacer l’historique…",
  "Remove contact": "Retirer le contact",
  "Where {name}'s files go": "Où vont les fichiers de {name}",
  "Fill in your name first: they see it when you add them.": "Indiquez d’abord votre nom : la personne le voit quand vous l’ajoutez.",
  "Asked once: your contacts see this name, so they know who's asking. You can change it later: click it under {place}, at the top of the contact list.":
    "Demandé une seule fois : vos contacts voient ce nom, pour savoir qui demande. Vous pourrez le changer plus tard : cliquez dessus sous {place}, en haut de la liste des contacts.",
  "Their Pako ID": "Son ID Pako",
  "Paste the ID they sent you": "Collez l’ID qu’on vous a envoyé",
  "Their name": "Son nom",
  "What you call them": "Comment vous l’appelez",
  "The first time, they're asked to accept. After that you connect by yourselves whenever both Pakos are open.":
    "La première fois, on lui demande d’accepter. Ensuite, vous vous connectez tout seuls dès que vos deux Pako sont ouverts.",
  "Added. Pako is reaching them now; the first time, they're asked to accept.": "Ajouté. Pako le contacte maintenant ; la première fois, on lui demande d’accepter.",
  "Tags for {name}": "Étiquettes de {name}",
  "Ideas:": "Idées :",
  Friends: "Amis",
  Family: "Famille",
  Work: "Travail",
  "Tags are only on this PC: {name} never sees them.": "Les étiquettes restent sur ce PC : {name} ne les voit jamais.",
  "Clear your history with {name}?": "Effacer votre historique avec {name} ?",
  "The list of what you sent and received with {name} is emptied on this PC and can't be brought back. The files themselves stay where they are, and {name} stays in your contacts.":
    "La liste de ce que vous avez envoyé et reçu avec {name} est vidée sur ce PC et ne peut pas être récupérée. Les fichiers eux-mêmes restent où ils sont, et {name} reste dans vos contacts.",
  "Clear history": "Effacer l’historique",
  "You won't be connected any more. For 7 days Pako turns them away without asking you; after that, if they reach you again, you're asked to accept like the first time. Files already received stay on your PC.":
    "Vous ne serez plus connectés. Pendant 7 jours, Pako refuse cette personne sans vous demander ; ensuite, si elle vous joint à nouveau, on vous demande d’accepter comme la première fois. Les fichiers déjà reçus restent sur votre PC.",

  // ---- The details panel ----
  "About {name}": "À propos de {name}",
  "What was in {name}": "Contenu de {name}",
  "Sent to {name}": "Envoyé à {name}",
  "Received from {name}": "Reçu de {name}",
  "Back to {name}'s details (Esc)": "Retour aux détails de {name} (Échap)",
  "Waiting for them": "En attente",
  "Seen lately": "Vu récemment",
  Route: "Trajet",
  "Last seen": "Vu en dernier",
  "Not yet": "Pas encore",
  "Speed now": "Vitesse actuelle",
  "Sending to {name}: {speed}": "Envoi à {name} : {speed}",
  "Receiving from {name}: {speed}": "Réception de {name} : {speed}",
  "Nothing moving": "Rien en cours",
  "Not called by itself.": "Pako ne l’appelle pas de lui-même.",
  "Check this contact": "Vérifier ce contact",
  "Check often for": "Vérifier souvent pendant",
  Automatic: "Automatique",
  "Often for…": "Souvent…",
  "When I ask": "Manuel",
  "1 hour": "1 heure",
  "1 week": "1 semaine",
  midnight: "minuit",
  "Checked often until {when}.": "Vérifié souvent jusqu’à {when}.",
  "Checked often until {when}, then back to When I ask.": "Vérifié souvent jusqu’à {when}, puis retour à Manuel.",
  "Checked often until {day}.": "Vérifié souvent jusqu’au {day}.",
  "Checked often until {day}, then back to When I ask.": "Vérifié souvent jusqu’au {day}, puis retour à Manuel.",
  "Only when you open, send or check.": "Seulement quand vous ouvrez, envoyez ou vérifiez.",
  "Always connected: they say this PC is theirs.": "Toujours connecté : la personne indique que ce PC est le sien.",
  "Less when away. Favourites never stop.": "Moins en votre absence. Les favoris ne s’arrêtent jamais.",
  "Less when away. Never stops: they send without asking.": "Moins en votre absence. Jamais arrêté : la personne envoie sans demander.",
  "Pauses after 30 days with nothing new.": "Se met en pause après 30 jours sans rien de nouveau.",
  "Between you": "Entre vous",
  "Sent to {name}: {size} in {count}": "Envoyé à {name} : {size} en {count}",
  "Received from {name}: {size} in {count}": "Reçu de {name} : {size} en {count}",
  Transfers: "Transferts",
  "Last exchange": "Dernier échange",
  "None yet": "Aucun pour l’instant",
  "Speed with {name}": "Vitesse avec {name}",
  "Fastest send to {name}": "Envoi le plus rapide à {name}",
  "Fastest from {name}": "Réception la plus rapide de {name}",
  "Average while sending to {name}": "Moyenne en envoyant à {name}",
  "Average while receiving from {name}": "Moyenne en recevant de {name}",
  "Speeds show up after your first transfer together.": "Les vitesses s’affichent après votre premier transfert ensemble.",
  "Over your home network, PC to PC: nothing goes through the internet. Usually the fastest.":
    "Par votre réseau domestique, de PC à PC : rien ne passe par internet. En général le plus rapide.",
  "Straight between the two PCs, over IPv6: usually the fastest.": "Directement entre les deux PC, en IPv6 : en général le plus rapide.",
  "Straight between the two PCs.": "Directement entre les deux PC.",
  "Through a relay server: still encrypted end to end, often slower.": "Par un serveur relais : toujours chiffré de bout en bout, souvent plus lent.",
  "Over your home network": "Par votre réseau domestique",
  "Straight between your PCs": "Directement entre vos PC",
  "Through a relay": "Par un relais",
  "{label} {pct}%": "{label} {pct}%",
  "{tip}: {size}": "{tip} : {size}",
  "About them": "À son sujet",
  "Copy {name}'s Pako ID": "Copier l’ID Pako de {name}",
  "The name they chose for themselves": "Le nom que la personne s’est choisi",
  "Not shared": "Non partagé",
  "Add tags": "Ajouter des étiquettes",
  "No pop-ups or sounds from them": "Aucun pop-up ni aucun son de sa part",
  "Their files go to": "Ses fichiers vont dans",

  // ---- What was in a transfer ----
  "{n} folders": { one: "{n} dossier", other: "{n} dossiers" },
  "{folders}, {files} · {all} in all": "{folders}, {files} · {all} en tout",
  "Files and folders in it": "Fichiers et dossiers qu’il contient",
  "Show {name} in its folder": "Afficher {name} dans son dossier",
  "Show in its folder": "Afficher dans son dossier",
  "and {n} more not listed": { one: "et {n} autre non listé", other: "et {n} autres non listés" },

  // ---- Direct's questions (on any page) ----
  "{name} wants to connect": "{name} veut se connecter",
  "Decline (Esc)": "Refuser (Échap)",
  "Not in your contacts. The name is the one they chose; Pako can't check it.":
    "Pas dans vos contacts. Le nom est celui que la personne a choisi ; Pako ne peut pas le vérifier.",
  "You already have a contact called {name}. This is a different Pako: if you expected {name}, decline.":
    "Vous avez déjà un contact appelé {name}. C’est un autre Pako : si vous attendiez {name}, refusez.",
  "Once you accept, {name} becomes one of your contacts in {page} and you connect by yourselves whenever both Pakos are open. You're only asked this once.":
    "Une fois accepté, {name} devient l’un de vos contacts dans {page} et vous vous connectez tout seuls dès que vos deux Pako sont ouverts. On ne vous le demande qu’une fois.",
  "If you know them, check they see the same code (by voice or chat). A different code means it isn't them.":
    "Si vous connaissez cette personne, vérifiez qu’elle voit le même code (de vive voix ou par message). Un code différent veut dire que ce n’est pas elle.",
  "If you decline (or close this), Pako turns {name} away without asking you for 7 days.":
    "Si vous refusez (ou fermez ceci), Pako refuse {name} sans vous demander pendant 7 jours.",
  "Your name (what {name} sees)": "Votre nom (ce que voit {name})",
  "Fill in your name first: {name} sees it when you accept.": "Indiquez d’abord votre nom : {name} le voit quand vous acceptez.",
  "{name} wants to send you files": "{name} veut vous envoyer des fichiers",
  "and {n} more files": { one: "et {n} autre fichier", other: "et {n} autres fichiers" },
  "Where to save these files": "Où enregistrer ces fichiers",
  "Files from {name} come without asking once it's paired both ways: confirm it in My devices on both PCs.":
    "Les fichiers de {name} arrivent sans demander une fois l’association faite dans les deux sens : confirmez-la dans Mes appareils sur les deux PC.",
  'To skip this question for {name}, turn on "Can send to me without asking" in Direct.':
    "Pour ne plus avoir cette question pour {name}, activez « Peut m’envoyer sans demander » dans Direct.",
} satisfies Record<string, Entry>;
