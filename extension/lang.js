// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The extension's words in French: it speaks Pako's language (English or French), never the
// browser's. Pako says which in its answers (`lang`, and the "language" request); the extension
// keeps the last one it heard (`pakoLang` in its storage), so it still speaks it when Pako can't
// be reached. English until Pako says French (an older Pako never says: English).
// The English is the key (kept word for word as the extension shows it); the French is written
// with plain spaces: tr() adds French typography (a non-breaking space before : and % and inside « »,
// a narrow one before ; ? !). The same words as Pako's window (its glossary:
// src/lib/i18n/fr.ts).
// Loaded before the scripts that use it: the background (importScripts in Chromium, listed
// first in Firefox's manifest), the toolbar window, the sent page, and the Download pill.
// Blocks between "full-only:start" and "full-only:end" are only in the full extension.
(() => {
  if (globalThis.__pakoWords) return;

  const FR = {
    // ---------- the right-click menu ----------
    "Download link with Pako": "Télécharger le lien avec Pako",
    "Download image with Pako": "Télécharger l’image avec Pako",
    /* full-only:start */
    "Download video with Pako": "Télécharger la vidéo avec Pako",
    "Download audio with Pako": "Télécharger l’audio avec Pako",
    /* full-only:end */
    "Send this page to Pako": "Envoyer cette page à Pako",
    "Send all links on this page to Pako (to pick from)": "Envoyer tous les liens de cette page à Pako (pour choisir)",

    // ---------- what went wrong (the toolbar's "!" and the toolbar window) ----------
    "Pako Light doesn't work on this site.": "Pako Light ne fonctionne pas sur ce site.",
    "Pako isn't connected. Click the Pako button in the toolbar, then Open Pako.":
      "Pako n’est pas connecté. Cliquez sur le bouton Pako de la barre d’outils, puis sur Ouvrir Pako.",
    "Pako didn't answer.": "Pako n’a pas répondu.",
    "No answer from {host}": "Pas de réponse de {host}",
    "Pako only takes http and https links.": "Pako ne prend que les liens http et https.",
    "No links on this page.": "Aucun lien sur cette page.",
    "Pako couldn't take this download": "Pako n’a pas pu prendre ce téléchargement",
    "{why}. Your browser couldn't start it again: open the link again to download it.":
      "{why}. Votre navigateur n’a pas pu le relancer : ouvrez à nouveau le lien pour le télécharger.",
    "{why}. Download it once more from the page.": "{why}. Téléchargez-le encore une fois depuis la page.",
    'Pako couldn\'t get "{name}", and your browser couldn\'t start it again. Download it once more from the page.':
      "Pako n’a pas pu récupérer « {name} », et votre navigateur n’a pas pu le relancer. Téléchargez-le encore une fois depuis la page.",
    "Pako couldn't get this download, and your browser couldn't start it again. Download it once more from the page.":
      "Pako n’a pas pu récupérer ce téléchargement, et votre navigateur n’a pas pu le relancer. Téléchargez-le encore une fois depuis la page.",
    "Pako's extension didn't understand this request.": "L’extension Pako n’a pas compris cette demande.",
    "Pako's extension ran into a problem. Try again.": "L’extension Pako a rencontré un problème. Réessayez.",
    // Pako's own answers (an older Pako says them in English): shown in the extension's language.
    "Pako is still starting. Try again in a moment.": "Pako démarre encore. Réessayez dans un instant.",
    "Pako didn't start in time. Open Pako, then try again.": "Pako n’a pas démarré à temps. Ouvrez Pako, puis réessayez.",
    "Pako closed before it finished checking this download.": "Pako s’est fermé avant d’avoir fini de vérifier ce téléchargement.",
    "Pako isn't running.": "Pako n’est pas lancé.",
    "Something other than your Pako answered, so nothing was sent.": "Autre chose que votre Pako a répondu, donc rien n’a été envoyé.",
    "Unknown request.": "Demande inconnue.",
    "Unreadable request.": "Demande illisible.",
    "Too many links at once. Pako takes up to about 1 MB of links per request.":
      "Trop de liens à la fois. Pako prend environ 1 Mo de liens au plus par demande.",
    "This Pako and its browser helper don't match. Restart Pako.": "Ce Pako et son assistant de navigateur ne vont pas ensemble. Redémarrez Pako.",
    "Pako lost track of this download (was it restarted?), so your browser downloads it.":
      "Pako a perdu la trace de ce téléchargement (a-t-il redémarré ?), alors votre navigateur le télécharge.",

    // ---------- the toolbar window ----------
    "Checking…": "Vérification…",
    "Connected": "Connecté",
    "Not connected": "Non connecté",
    "Not in use": "Non utilisée",
    "Details: {text}": "Détails : {text}",
    "Pako isn't connected to this browser": "Pako n’est pas connecté à ce navigateur",
    "Opens Pako and connects it in one go. If your browser asks, allow it.": "Ouvre Pako et le connecte d’un coup. Si votre navigateur le demande, autorisez-le.",
    "Open Pako": "Ouvrir Pako",
    "Opening Pako…": "Ouverture de Pako…",
    "When Pako says Connected, come back and open this popup again.": "Quand Pako affiche Connecté, revenez et rouvrez cette fenêtre.",
    "Couldn't open Pako. Open it yourself: Settings › Browser extension › Connect Pako to your browsers.":
      "Impossible d’ouvrir Pako. Ouvrez-le vous-même : Paramètres › Extension de navigateur › Connecter Pako à vos navigateurs.",
    "No Pako on this PC yet?": "Pas encore de Pako sur ce PC ?",
    "Get it at pako.sookera.com": "Obtenez-le sur pako.sookera.com",
    "This Pako is too old for this extension": "Ce Pako est trop ancien pour cette extension",
    "Update Pako (Settings › Features & updates), then open this again.": "Mettez Pako à jour (Paramètres › Fonctions et mises à jour), puis rouvrez ceci.",
    "The full Pako extension is on in this browser": "L’extension Pako complète est activée dans ce navigateur",
    "Pako Light stays out of the way: the full extension takes your downloads and right-click items.":
      "Pako Light s’efface : l’extension complète prend vos téléchargements et les éléments du clic droit.",
    "Sent to Pako ✓": "Envoyé à Pako ✓",
    "Couldn't send this page.": "Impossible d’envoyer cette page.",
    "Pako asks where to save it, unless you turned that off in Pako.": "Pako demande où l’enregistrer, sauf si vous avez désactivé cela dans Pako.",
    "Let Pako take over downloads": "Laisser Pako prendre les téléchargements",
    "Files over 2 MB you download in this browser go to Pako instead.": "Les fichiers de plus de 2 Mo que vous téléchargez dans ce navigateur vont à Pako à la place.",
    "Private windows:": "Fenêtres privées :",
    "on": "activé",
    "off": "désactivé",
    "To use Pako there, allow it on this extension's page.": "Pour y utiliser Pako, autorisez-le sur la page de cette extension.",
    "Pako keeps no trace of those downloads beyond the file.": "Pako ne garde aucune trace de ces téléchargements à part le fichier.",
    "To use Pako there: Firefox menu › Add-ons and themes › Pako › Run in Private Windows › Allow.":
      "Pour y utiliser Pako : menu de Firefox › Modules complémentaires et thèmes › Pako › Exécution dans des fenêtres privées › Autoriser.",
    'To use Pako there, turn on "{name}" on this extension\'s page.': "Pour y utiliser Pako, activez « {name} » sur la page de cette extension.",
    "Allow in Incognito": "Autoriser en mode navigation privée",
    "Allow in InPrivate": "Autoriser en mode InPrivate",
    "Open": "Ouvrir",
    /* full-only:start */
    "Updating Pako's extension… open this again in a moment.": "Mise à jour de l’extension Pako… rouvrez ceci dans un instant.",
    "Videos on this page": "Vidéos de cette page",
    "Pako asks where to save it (and the quality, for videos), unless you turned that off in Pako.":
      "Pako demande où l’enregistrer (et la qualité, pour les vidéos), sauf si vous avez désactivé cela dans Pako.",
    "Catch download links on pages": "Attraper les liens de téléchargement des pages",
    "Your browser asks before each download. With this on, download links on pages go straight to Pako, without that question.":
      "Votre navigateur demande avant chaque téléchargement. Avec ceci activé, les liens de téléchargement des pages vont directement à Pako, sans cette question.",
    "Download button on videos": "Bouton Télécharger sur les vidéos",
    "On YouTube, Vimeo, Twitch, Facebook and Instagram: shows at the top right of the video when you point at it.":
      "Sur YouTube, Vimeo, Twitch, Facebook et Instagram : s’affiche en haut à droite de la vidéo quand vous la survolez.",
    "Couldn't send this video.": "Impossible d’envoyer cette vidéo.",
    "Couldn't start recording.": "Impossible de démarrer l’enregistrement.",
    "Stop recording": "Arrêter l’enregistrement",
    "The page reloaded. Waiting for the video to play: press play if it doesn't start. It has to play to the end.":
      "La page s’est rechargée. En attente de la lecture de la vidéo : appuyez sur lecture si elle ne démarre pas. Elle doit être lue jusqu’au bout.",
    "Recording: {mb} MB. Keep this tab open: the video has to play to the end (sped up and muted when the page allows).":
      "Enregistrement : {mb} Mo. Gardez cet onglet ouvert : la vidéo doit être lue jusqu’au bout (accélérée et muette quand la page le permet).",
    "Sending the recording to Pako…": "Envoi de l’enregistrement à Pako…",
    "Sent to Pako: it makes the video in Downloads.": "Envoyé à Pako : il crée la vidéo dans Téléchargements.",
    "The last resort when Pako can't get it: Pako records the video while it plays. The page reloads, and the video has to play to the end.":
      "Le dernier recours quand Pako ne peut pas l’obtenir : Pako enregistre la vidéo pendant sa lecture. La page se recharge, et la vidéo doit être lue jusqu’au bout.",

    // ---------- videos caught on a page, the Download pill and its menu ----------
    "HLS stream": "Flux HLS",
    "DASH stream": "Flux DASH",
    "Video file": "Fichier vidéo",
    "up to {height}p": "jusqu’à {height}p",
    "Up to {height}p": "Jusqu’à {height}p",
    "Audio only": "Audio seul",
    "Download": "Télécharger",
    "Download with Pako": "Télécharger avec Pako",
    "Download this video": "Télécharger cette vidéo",
    "Sending…": "Envoi…",
    "Couldn't send this video to Pako.": "Impossible d’envoyer cette vidéo à Pako.",
    "(Browser says: {detail})": "(Le navigateur dit : {detail})",
    "Pako's extension was just updated. Reload this page and try again.": "L’extension Pako vient d’être mise à jour. Rechargez cette page et réessayez.",
    "Pako's extension isn't responding. Open the Pako button in the toolbar for details.":
      "L’extension Pako ne répond pas. Ouvrez le bouton Pako de la barre d’outils pour plus de détails.",
    "Pako didn't find this video's address.": "Pako n’a pas trouvé l’adresse de cette vidéo.",
    "This video isn't on the page any more. Play it again.": "Cette vidéo n’est plus sur la page. Relancez-la.",
    "This video is protected, so it can't be downloaded.": "Cette vidéo est protégée, elle ne peut donc pas être téléchargée.",
    "This player doesn't give its video's address. Click the Pako button: Videos on this page, or Record this video.":
      "Ce lecteur ne donne pas l’adresse de sa vidéo. Cliquez sur le bouton Pako : Vidéos de cette page, ou Enregistrer cette vidéo.",
    "Unknown video quality.": "Qualité vidéo inconnue.",
    "Pako can't download videos from this page.": "Pako ne peut pas télécharger les vidéos de cette page.",
    "Send one video at a time.": "Envoyez une vidéo à la fois.",

    // ---------- Record this video ----------
    "Record this video": "Enregistrer cette vidéo",
    "reloads the page": "recharge la page",
    "Recording…": "Enregistrement…",
    "Recording this video: the Pako button in the toolbar shows how it's going.": "Enregistrement de cette vidéo : le bouton Pako de la barre d’outils montre où il en est.",
    "Play the video first, then try again.": "Lancez d’abord la vidéo, puis réessayez.",
    "Pako is already recording a video. Stop that one first.": "Pako enregistre déjà une vidéo. Arrêtez d’abord celle-là.",
    "Pako can't record on this page.": "Pako ne peut pas enregistrer sur cette page.",
    "Pako doesn't record in private windows.": "Pako n’enregistre pas dans les fenêtres privées.",
    "This browser can't record videos. Update it, then try again.": "Ce navigateur ne peut pas enregistrer de vidéos. Mettez-le à jour, puis réessayez.",
    "The video didn't start playing, so nothing was recorded.": "La vidéo n’a pas démarré, donc rien n’a été enregistré.",
    "Pako couldn't save the recording.": "Pako n’a pas pu sauvegarder l’enregistrement.",
    "The page closed before the video ended, so nothing was saved.": "La page s’est fermée avant la fin de la vidéo, donc rien n’a été sauvegardé.",
    "Recording stopped.": "Enregistrement arrêté.",
    "This tab is closed.": "Cet onglet est fermé.",

    // ---------- a download caught at the server's answer, and its sent page ----------
    "Sent to Pako": "Envoyé à Pako",
    "This download went to Pako.": "Ce téléchargement a été envoyé à Pako.",
    "This download went to Pako. Use Back to return to your page.": "Ce téléchargement a été envoyé à Pako. Revenez à la page précédente pour retrouver votre page.",
    'This page asked for several downloads at once, so Pako took only the first ones. To download "{name}", use the page again.':
      "Cette page a demandé plusieurs téléchargements à la fois, alors Pako n’a pris que les premiers. Pour télécharger « {name} », utilisez de nouveau la page.",
    "This page asked for several downloads at once, so Pako took only the first ones. To download the others, use the page again.":
      "Cette page a demandé plusieurs téléchargements à la fois, alors Pako n’a pris que les premiers. Pour télécharger les autres, utilisez de nouveau la page.",
    "This page asked for several downloads at once, so Pako didn't take this one. Use Back, then download it again.":
      "Cette page a demandé plusieurs téléchargements à la fois, alors Pako n’a pas pris celui-ci. Revenez à la page précédente, puis téléchargez-le à nouveau.",
    "This private window's download couldn't go on in your browser. Use Back and download it once more.":
      "Le téléchargement de cette fenêtre privée n’a pas pu continuer dans votre navigateur. Revenez à la page précédente et téléchargez-le encore une fois.",
    "This private window's download goes on in your browser.": "Le téléchargement de cette fenêtre privée continue dans votre navigateur.",
    "This address can't be let through.": "Cette adresse ne peut pas passer.",
    'Pako\'s extension couldn\'t hand "{name}" back to your private window. Download it once more from the page.':
      "L’extension Pako n’a pas pu rendre « {name} » à votre fenêtre privée. Téléchargez-le encore une fois depuis la page.",
    "Pako's extension couldn't hand this download back to your private window. Download it once more from the page.":
      "L’extension Pako n’a pas pu rendre ce téléchargement à votre fenêtre privée. Téléchargez-le encore une fois depuis la page.",
    /* full-only:end */
  };

  /** One of the two languages (anything else, from an older or edited copy: English). */
  const langOf = (v) => (v === "fr" ? "fr" : "en");

  /** French typography, added to the dictionary's plain spaces (as Pako's window does). */
  const frType = (s) =>
    s
      .replace(/ :/g, " :")
      .replace(/ ([;?!])/g, " $1")
      .replace(/« /g, "« ")
      .replace(/ »/g, " »")
      .replace(/([\d}]) ?%/g,"$1 %");

  /** "{name}" placeholders filled; unknown ones stay as written. */
  const fill = (s, vars) => (vars ? s.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : s);

  /** The words for `key` (the English) in `lang`, placeholders filled. A key without French
   *  stays English. */
  function tr(lang, key, vars) {
    const fr = langOf(lang) === "fr" && Object.prototype.hasOwnProperty.call(FR, key) ? FR[key] : null;
    return fill(fr == null ? key : frType(fr), vars);
  }

  /** Text that may be one of the extension's (or Pako's) English sentences: translated when it
   *  is, else left as it is (Pako's own words already in French, a file name, the browser's). */
  function maybe(lang, text) {
    return typeof text === "string" && Object.prototype.hasOwnProperty.call(FR, text) ? tr(lang, text) : text;
  }

  /** A number as the language writes it (1.5 → "1,5" in French). */
  const num = (lang, n) => (langOf(lang) === "fr" ? String(n).replace(".", ",") : String(n));

  globalThis.__pakoWords = { FR, langOf, frType, fill, tr, maybe, num };
})();
