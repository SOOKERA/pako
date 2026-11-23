// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for the small windows: the tray list (routes/mini), Download with Pako (routes/ask), pop-ups (routes/popup) and important notices (routes/important). See ./fr.ts for the glossary and the rules.
import type { Entry } from "./core";

export const small = {
  // ---- The tray list (routes/mini) ----
  "All done": "Tout est terminé",
  "Always on top": "Toujours au premier plan",
  "Always on top: on": "Toujours au premier plan : activé",
  "Show downloads, uploads or Direct": "Voir les téléchargements, les mises en ligne ou Direct",
  "Show downloads or uploads": "Voir les téléchargements ou les mises en ligne",
  // The tabs are narrow (three share 330 px): « Téléchargements » doesn't fit beside a count.
  "Downloads (tray tab)": "Téléchargements",
  "Uploads (tray tab)": "Envoi",
  "One of your PCs": "Un de vos PC",
  "Nothing uploaded yet.": "Aucune mise en ligne pour l’instant.",
  "Not uploaded": "Non mis en ligne",
  "Copy the link of {name}": "Copier le lien : {name}",
  "Nothing sent or received today.": "Rien d’envoyé ni de reçu aujourd’hui.",
  // A Direct transfer's line. Names are never after « de » (« d’Alex »): the sentences avoid it.
  "them (no name)": "ce contact",
  "a contact (no name)": "votre contact",
  "Sending to {to} through {name}": "Envoi à {to} via {name}",
  "Sending to {name}": "Envoi à {name}",
  "Receiving from {name}": "{name} vous envoie",
  "Checking the files": "Vérification des fichiers",
  "Waiting for {name} to continue": "En attente de reprise avec {name}",
  "Waiting for {name} to accept": "{name} doit encore accepter",
  "Getting ready for {name}": "Préparation pour {name}",
  "Getting ready, from {name}": "Préparation, envoi par {name}",
  "Passed on to {name}": "Transmis à {name}",
  "your PC (no name)": "votre PC",
  "Cancel {name} now": "Annuler {name} maintenant",
  "Cancel {name}": "Annuler {name}",
  "Stops this transfer": "Arrête ce transfert",
  "{pct}% · {speed}": "{pct} % · {speed}",
  "{pct}% of {size}": "{pct} % de {size}",
  "Nothing downloading.": "Aucun téléchargement en cours.",
  "and {n} more in Pako": { one: "et {n} autre dans Pako", other: "et {n} autres dans Pako" },
  "Remove {name} now: what was downloaded so far is deleted": "Retirer {name} maintenant : ce qui a déjà été téléchargé est supprimé",
  "What was downloaded so far is deleted": "Ce qui a déjà été téléchargé est supprimé",
  "Audio only · {format}": "Audio seul · {format}",
  // The three buttons at the bottom share 340 px.
  "Pause all (tray)": "Tout en pause",
  "Open Pako": "Ouvrir Pako",

  // ---- Download with Pako (routes/ask) ----
  "Download with Pako": "Télécharger avec Pako",
  "Cancel this download": "Annuler ce téléchargement",
  "about {size}": "environ {size}",
  "You can turn it back on in {place}": "Vous pouvez le réactiver dans {place}",
  // The bar holds Cancel, Add to queue, this and the ▾ arrow on 388 px.
  "Download now (ask window)": "Lancer maintenant",
  "Not sent to {name}: {why}": "Non envoyé à {name} : {why}",
  "Download on another PC, or privately": "Télécharger sur un autre PC, ou en privé",
  "Download on another PC": "Télécharger sur un autre PC",

  // ---- Pop-ups (routes/popup) and the Important card (routes/important) ----
  "{n} more waiting": { one: "{n} autre en attente", other: "{n} autres en attente" },
  "Show in Upload": "Voir dans Envoi",
  "{n} more after this": { one: "{n} autre ensuite", other: "{n} autres ensuite" },
  "Close all": "Tout fermer",

  // ---- Kinds of notifications (notices.ts: Settings › Notifications) and their times ----
  "Download finished": "Téléchargement terminé",
  "Download stopped": "Téléchargement arrêté",
  "Sent from your browser": "Envoyé par votre navigateur",
  "Couldn't get a video": "Vidéo impossible à récupérer",
  "Upload finished": "Mise en ligne terminée",
  "Couldn't upload": "Mise en ligne impossible",
  "Upload deleted on its date": "Mise en ligne supprimée à la date prévue",
  "Someone wants to connect": "Quelqu’un veut se connecter",
  "Files waiting for your answer": "Fichiers en attente de votre réponse",
  "Files coming in": "Des fichiers arrivent",
  "Files received": "Fichiers reçus",
  "Files delivered": "Fichiers livrés",
  "Pako needs you": "Pako a besoin de vous",
  "Sleep and shut down": "Veille et arrêt",
  "New Pako ready": "Nouvelle version de Pako prête",
  "Other messages": "Autres messages",
  "now (time)": "à l’instant",
  "{n} min ago": "il y a {n} min",
  "{n} h ago": "il y a {n} h",

  // ---- Download on another PC (downloadOn.svelte.ts: Links' toasts and row notes) ----
  "{name} didn't take it: {why}": "{name} ne l’a pas accepté : {why}",
  "{name}: {why}. {n} links stay here (see its row).": {
    one: "{name} : {why}. {n} lien reste ici (voir sa ligne).",
    other: "{name} : {why}. {n} liens restent ici (voir leur ligne).",
  },
  "{n} links stay in Links: {why}": { one: "{n} lien reste dans Liens : {why}", other: "{n} liens restent dans Liens : {why}" },

  // ---- The website's sample pop-ups (showcase.ts, design preview only) ----
  "{name} wants to send you {what} ({size}).": "{name} veut vous envoyer {what} ({size}).",
  "Files for you": "Des fichiers pour vous",
  Accepted: "Accepté",
  "{name} wants to connect with you in Pako.": "{name} veut se connecter avec vous dans Pako.",
  "Downloading in Pako": "Téléchargement dans Pako",
} satisfies Record<string, Entry>;
