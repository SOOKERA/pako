// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for the Dashboard (DashboardView…). See ./fr.ts for the glossary and the rules.
// Words of this area: chiffres (numbers) · statistiques · mis en ligne (uploaded) · envoyé / reçu en
// Direct · vitesse de pointe (top speed) · transfert.
import type { Entry } from "./core";

export const dashboard = {
  // Rust names a day kept from before the Dashboard (numbers.rs), shown as a record's name.
  "Downloads that day": "Téléchargements de ce jour-là",
  Period: "Période",
  "This week": "Cette semaine",
  "This month": "Ce mois-ci",
  "All time": "Depuis toujours",
  // The end of "Nothing moved today", "… this week", "… yet".
  today: "aujourd’hui",
  "this week": "cette semaine",
  "this month": "ce mois-ci",
  "yet (all time)": "pour l’instant",
  "Save or reset the numbers": "Enregistrer ou réinitialiser les chiffres",
  "Save the details as a CSV file…": "Enregistrer le détail en CSV…",
  "Reset statistics…": "Réinitialiser les statistiques…",
  "Save Pako's numbers": "Enregistrer les chiffres de Pako",
  "Pako numbers {date}.csv": "Chiffres Pako {date}.csv",
  "CSV (opens in Excel)": "CSV (s’ouvre dans Excel)",
  "Saved {n} transfers to the file.": { one: "{n} transfert enregistré dans le fichier.", other: "{n} transferts enregistrés dans le fichier." },
  "Statistics reset.": "Statistiques réinitialisées.",
  "Reset statistics?": "Réinitialiser les statistiques ?",
  "Every number on the Dashboard starts again from zero, including the speed history. Your files, downloads, uploads and contacts stay as they are. This can't be undone.":
    "Tous les chiffres du tableau de bord repartent de zéro, historique des vitesses compris. Vos fichiers, téléchargements, mises en ligne et contacts restent tels quels. Impossible de revenir en arrière.",
  Reset: "Réinitialiser",

  // ---- Empty ----
  "Your numbers show up as you use Pako": "Vos chiffres apparaissent à mesure que vous utilisez Pako",
  "Downloads, uploads and Direct transfers are counted here as they finish: how much, how fast, and with whom. Only on this PC.":
    "Les téléchargements, les mises en ligne et les transferts Direct sont comptés ici quand ils se terminent : combien, à quelle vitesse et avec qui. Seulement sur ce PC.",

  // ---- The four totals ----
  "Sent direct": "Envoyé en Direct",
  "Received direct": "Reçu en Direct",
  "{n} stopped": { one: "{n} arrêté", other: "{n} arrêtés" },
  "Stopped with an error": "Arrêté par une erreur",
  "People downloaded these links {n} times (as Gofile last said)": {
    one: "Ces liens ont été téléchargés {n} fois (d’après le dernier relevé de Gofile)",
    other: "Ces liens ont été téléchargés {n} fois (d’après le dernier relevé de Gofile)",
  },
  "to {n} people": { one: "à {n} personne", other: "à {n} personnes" },
  "from {n} people": { one: "de {n} personne", other: "de {n} personnes" },
  "{n}% went straight from your PC to theirs, the rest through a relay": "{n} % est allé directement de votre PC au sien, le reste par un relais",
  "{n}% direct": "{n} % en direct",

  // ---- Activity ----
  Activity: "Activité",
  "Today, hour by hour": "Aujourd’hui, heure par heure",
  "Since {date}, month by month": "Depuis le {date}, mois par mois",
  "Since {date}, day by day": "Depuis le {date}, jour par jour",
  "{from} to {to}, month by month": "Du {from} au {to}, mois par mois",
  "{from} to {to}, day by day": "Du {from} au {to}, jour par jour",
  "{from} to {to} (hours)": "De {from} à {to}",
  "Nothing moved {when}.": "Rien n’a été transféré {when}.",
  "Nothing moved {when}": "Rien n’a été transféré {when}",
  "What Pako moved, {range}": "Ce que Pako a transféré, {range}",
  "In all": "Au total",

  // ---- Speed ----
  "Average speed while moving, and the top speed reached.": "Vitesse moyenne pendant le transfert, et vitesse de pointe atteinte.",
  "Which speed": "Quelle vitesse",
  Uploads: "Mises en ligne",
  "Up to the top speed": "Jusqu’à la vitesse de pointe",
  "No downloads with a measured speed {when}.": "Aucun téléchargement à vitesse mesurée {when}.",
  "No uploads with a measured speed {when}.": "Aucune mise en ligne à vitesse mesurée {when}.",
  "No Direct transfers with a measured speed {when}.": "Aucun transfert Direct à vitesse mesurée {when}.",
  "Downloads speed, {range}": "Vitesse des téléchargements, {range}",
  "Uploads speed, {range}": "Vitesse des mises en ligne, {range}",
  "Direct speed, {range}": "Vitesse de Direct, {range}",
  "Top {speed}": "Pointe à {speed}",
  "Not measured": "Non mesurée",
  Moved: "Transféré",

  // ---- Which way is fastest ----
  "Sending files": "Envoi de fichiers",
  "Average upload speed, by the way you sent them.": "Vitesse moyenne d’envoi, selon la façon dont vous les avez envoyés.",
  "Getting files": "Réception de fichiers",
  "Average download speed, by where they came from.": "Vitesse moyenne de téléchargement, selon leur provenance.",
  "Gofile link": "Lien Gofile",
  "Google Drive link": "Lien Google Drive",
  "Other sites": "Autres sites",
  "Received in Direct": "Reçu en Direct",
  "Received on local network": "Reçu sur le réseau local",
  "Earlier downloads": "Téléchargements plus anciens",
  "Average {speed} while moving": "Moyenne de {speed} pendant le transfert",
  "Top speed {speed}": "Vitesse de pointe {speed}",
  "The top speed reached": "La vitesse de pointe atteinte",
  "top {speed}": "pointe à {speed}",
  "Not used {when}": "Aucune utilisation {when}",

  // ---- People and sources ----
  "Your people": "Vos contacts",
  "Who you exchanged the most with in Direct.": "Ceux avec qui vous avez le plus échangé dans Direct.",
  "No Direct transfers {when}.": "Aucun transfert Direct {when}.",
  "Removed contact": "Contact supprimé",
  "Open {name} in Direct": "Ouvrir {name} dans Direct",
  "Sent to {name}: {size}": "Envoyé à {name} : {size}",
  "Received from {name}: {size}": "Reçu de {name} : {size}",
  "Fastest send to {name}: {speed}": "Envoi le plus rapide à {name} : {speed}",
  "Fastest from {name}: {speed}": "Réception la plus rapide de {name} : {speed}",
  "{n} transfers with {name}, {size} in all": {
    one: "{n} transfert avec {name}, {size} au total",
    other: "{n} transferts avec {name}, {size} au total",
  },
  "Where files came from": "D’où viennent les fichiers",
  "Everything you got, by source.": "Tout ce que vous avez reçu, par source.",
  "Nothing downloaded or received {when}.": "Rien de téléchargé ni de reçu {when}.",
  "Share of what you got, by source": "Part de ce que vous avez reçu, par source",
  "{name}: {size} ({n}%)": "{name} : {size} ({n} %)",
  "{n}%": "{n} %",
  "Top sites": "Sites les plus utilisés",
  "{site}: {n} downloads, {size}": { one: "{site} : {n} téléchargement, {size}", other: "{site} : {n} téléchargements, {size}" },

  // ---- Records ----
  "Fastest download": "Téléchargement le plus rapide",
  "No measured download {when}": "Aucun téléchargement mesuré {when}",
  "Biggest file": "Plus gros fichier",
  "No file {when}": "Aucun fichier {when}",
  "Busiest day": "Jour le plus chargé",
} satisfies Record<string, Entry>;
