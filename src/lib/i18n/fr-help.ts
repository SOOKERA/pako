// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for Help, What's new and Report a problem's windows (src/lib/help/*; the guides' own text is in docs/help/fr/). See ./fr.ts for the glossary and the rules.
import type { Entry } from "./core";

export const help = {
  // ---- Help ----
  "Search the guides": "Rechercher dans l’aide",
  "Start here": "Pour commencer",
  "What's new": "Nouveautés",
  "About Pako": "À propos de Pako",
  "Report a problem": "Signaler un problème",
  "{key} opens Help anywhere in Pako.": "{key} ouvre l’Aide partout dans Pako.",
  Search: "Recherche",
  'Nothing in the guides matches "{q}". Try fewer or other words.': "Rien dans les guides ne correspond à « {q} ». Essayez avec moins de mots, ou d’autres.",
  Tutorials: "Tutoriels",
  "A spotlight walks you through the window. The quick tour shows what each page is for; each page's own tutorial shows its details.":
    "Un projecteur vous fait le tour de la fenêtre. La visite rapide montre à quoi sert chaque page ; le tutoriel de chaque page en montre les détails.",
  "Quick tour": "Visite rapide",
  Guides: "Guides",
  "The latest changes in Pako.": "Les derniers changements de Pako.",
  "This is Pako {v}.": "Vous utilisez Pako {v}.",
  "Something went wrong? Send Sookera a report in one click, or by email.": "Un problème ? Envoyez un rapport à Sookera en un clic, ou par e-mail.",
  "version {v}": "version {v}",
  "Pako comes with no warranty. You may share and change it under the GNU GPL version 3 with Sookera's additional terms. Its {source} is on GitHub.":
    "Pako est fourni sans aucune garantie. Vous pouvez le partager et le modifier selon la GNU GPL version 3, avec les conditions supplémentaires de Sookera. Son {source} est sur GitHub.",
  "source code": "code source",
  "Legal text": "Texte juridique",
  "Additional terms": "Conditions supplémentaires",
  Licence: "Licence",
  "Pako's terms of use, as you accepted them. The newest version is always on {link}.":
    "Les conditions d’utilisation de Pako, telles que vous les avez acceptées. La version la plus récente est toujours sur {link}.",
  "The GNU General Public License version 3, as it comes with Pako. Also on {link}.":
    "La GNU General Public License version 3, telle qu’elle est fournie avec Pako. Aussi sur {link}.",
  "This guide isn't here any more.": "Ce guide n’existe plus.",
  "Close Settings first: this tutorial shows another page.": "Fermez d’abord les Paramètres : ce tutoriel montre une autre page.",
  "How this works": "Comment ça marche",

  // ---- What's new ----
  "Pako {v}": "Pako {v}",
  // The heading of what comes in the next version (docs/help/whats-new.md keeps it in English).
  "Next version": "Prochaine version",
  "What's new since Pako {v}": "Nouveautés depuis Pako {v}",
  "What's new in Pako {v}": "Nouveautés de Pako {v}",
  "Help and tutorials": "Aide et tutoriels",
  "Got it": "J’ai compris",

  // ---- Report a problem ----
  "Download (problem area)": "Téléchargement",
  "Video (problem area)": "Vidéo",
  "Link check (problem area)": "Vérification de lien",
  "Upload (problem area)": "Envoi",
  "Direct (problem area)": "Direct",
  "Update (problem area)": "Mise à jour",
  "Browser extension (problem area)": "Extension de navigateur",
  "Other (problem area)": "Autre",
  "Light (theme)": "Clair",
  "Dark (theme)": "Sombre",
  "{theme} (Match Windows)": "{theme} (Comme Windows)",
  // Pako Setup and Pako Portable are names.
  "{v} (Setup)": "{v} (Setup)",
  "{v} (Portable)": "{v} (Portable)",
  "Theme: {look}": "Thème : {look}",
  "Time zone: {zone}": "Fuseau horaire : {zone}",
  "Recent problems: not included.": "Problèmes récents : non inclus.",
  "Recent problems: none noted.": "Problèmes récents : aucun noté.",
  "Recent problems (newest first, {n} of {total}):": "Problèmes récents (les plus récents d’abord, {n} sur {total}) :",
  "{area}: {message}": "{area} : {message}",
  "What happened:": "Ce qui s’est passé :",
  "Details are on your clipboard: paste them here.": "Les détails sont dans votre presse-papiers : collez-les ici.",
  "Couldn't reach Sookera. Check your connection and try again, or use Send by email.":
    "Impossible de joindre Sookera. Vérifiez votre connexion et réessayez, ou envoyez-le par e-mail.",
  "Too many reports from this connection. Try again in {n} minutes, or use Send by email.": {
    one: "Trop de rapports depuis cette connexion. Réessayez dans {n} minute, ou envoyez-le par e-mail.",
    other: "Trop de rapports depuis cette connexion. Réessayez dans {n} minutes, ou envoyez-le par e-mail.",
  },
  "Sookera's report service isn't answering right now. Try again later, or use Send by email.":
    "Le service de rapports de Sookera ne répond pas pour le moment. Réessayez plus tard, ou envoyez-le par e-mail.",
  "Couldn't send the report ({why}). Use Send by email instead.": "Impossible d’envoyer le rapport ({why}). Envoyez-le plutôt par e-mail.",
  "Pako {v}: problem report": "Pako {v} : rapport de problème",
  "Say what happened first": "Dites d’abord ce qui s’est passé",
  "Thank you. Your report's reference is {ref}. If you write to {email} about it, mention it.":
    "Merci. La référence de votre rapport est {ref}. Si vous écrivez à {email} à son sujet, indiquez-la.",
  "Couldn't read Pako's details: {why}": "Impossible de lire les détails de Pako : {why}",
  "Report sent to Sookera. Its reference is #{ref}.": "Rapport envoyé à Sookera. Sa référence est #{ref}.",
  "The report wasn't sent. {why}": "Le rapport n’a pas été envoyé. {why}",
  "Your email app has the report. Paste the details where it says, check it, then send it.":
    "Le rapport est dans votre messagerie. Collez les détails à l’endroit indiqué, vérifiez-le, puis envoyez-le.",
  "Your email app has the report. Check it, then send it.": "Le rapport est dans votre messagerie. Vérifiez-le, puis envoyez-le.",
  "Details copied. Paste them into an email to {email}.": "Détails copiés. Collez-les dans un e-mail à {email}.",
  "Save the problem report": "Enregistrer le rapport de problème",
  "Pako problem report {date}.txt": "Rapport de problème Pako {date}.txt",
  "Text file": "Fichier texte",
  "Saved. Attach the file to an email to {email}.": "Enregistré. Joignez le fichier à un e-mail à {email}.",
  "Sent to Sookera": "Envoyé à Sookera",
  "What happened?": "Que s’est-il passé ?",
  "What you did and what you saw. For example: a Google Drive folder stopped at 80% with “Server error (502)”, and Retry didn't help.":
    "Ce que vous avez fait et ce que vous avez vu. Par exemple : un dossier Google Drive s’est arrêté à 80 % avec « Erreur du serveur (502) », et Réessayer n’a rien changé.",
  "Sent with your report": "Envoyé avec votre rapport",
  "Time zone": "Fuseau horaire",
  "Include recent problems": "Joindre les problèmes récents",
  "The last problems Pako noted on this PC.": "Les derniers problèmes notés par Pako sur ce PC.",
  "The last {n} problems Pako noted. Links show only their site; file and folder names, IDs and your user name are left out.": {
    one: "Le dernier problème noté par Pako. Les liens n’affichent que leur site ; les noms de fichiers et de dossiers, les identifiants et votre nom d’utilisateur sont retirés.",
    other: "Les {n} derniers problèmes notés par Pako. Les liens n’affichent que leur site ; les noms de fichiers et de dossiers, les identifiants et votre nom d’utilisateur sont retirés.",
  },
  "Pako hasn't noted any problems on this PC.": "Pako n’a noté aucun problème sur ce PC.",
  "Show fewer": "Afficher moins",
  "Show all {n}": "Tout afficher ({n})",
  "Send report sends what's shown here to Sookera, kept private and used only to fix Pako. Nothing goes until you click it.":
    "Envoyer le rapport transmet à Sookera ce qui est affiché ici, gardé privé et utilisé seulement pour corriger Pako. Rien ne part avant que vous cliquiez dessus.",
  // The dialog's four buttons share one row: short French so they fit.
  "Save as a file…": "Enregistrer…",
  "Copy details": "Copier",
  "Opens your email app, to {email}": "Ouvre votre messagerie, pour écrire à {email}",
  "Send by email": "Par e-mail",
  "Ctrl+Enter": "Ctrl+Entrée",
  "Sending…": "Envoi en cours…",
  "Send report": "Envoyer le rapport",
} satisfies Record<string, Entry>;
