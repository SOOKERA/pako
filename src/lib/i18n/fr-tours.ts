// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for the tutorials and the quick tour (src/lib/tour/*). See ./fr.ts for the glossary and the rules.
// Buttons and pages are named with the French labels the window shows. When a step changes in
// tours.ts, change its French here in the same commit (a step without French is a type error).
import type { Entry } from "./core";

export const tours = {
  // ---- The tutorial bubble and the Tutorial button ----
  "Tutorial: {title}": "Tutoriel : {title}",
  "Stop the tutorial (Esc)": "Arrêter le tutoriel (Échap)",
  "Back (←)": "Retour (←)",
  "Finish (Enter)": "Terminer (Entrée)",
  "Next (→)": "Suivant (→)",
  "Tutorial": "Tutoriel",
  "A short tour of this page": "Une courte visite de cette page",
  "A short tour of this page (not seen yet)": "Une courte visite de cette page (pas encore vue)",

  // ---- Pages with a tutorial (Help) and step titles ----
  "Your downloads": "Vos téléchargements",
  "Show and Sources": "Afficher et Sources",
  "Search, pause, add": "Rechercher, mettre en pause, ajouter",
  "When done and At once": "À la fin et À la fois",
  "Your links": "Vos liens",
  "Add, Save to, Download": "Ajouter, Enregistrer dans, Télécharger",
  "Your contacts": "Vos contacts",
  "Your history together": "Vos échanges",
  "Pair a PC": "Associer un PC",
  "Files, Links, Downloads, Settings": "Fichiers, Liens, Téléchargements, Paramètres",
  "Nothing yet": "Rien pour l’instant",
  "Totals and Activity": "Totaux et Activité",
  "Speed and routes": "Vitesse et chemins",
  "Your people and Records": "Vos contacts et Records",
  "Open, Send, Decrypt, Delete": "Ouvrir, Envoyer, Déchiffrer, Supprimer",
  "Normal list and Lock": "Liste normale et Verrouiller",
  "Welcome to Pako": "Bienvenue dans Pako",
  "Downloads and Links": "Téléchargements et Liens",
  "Upload and Direct": "Envoi et Direct",
  "Help and Settings": "Aide et Paramètres",
  "Next to the clock": "À côté de l’horloge",

  // ---- Steps, in the order of tours.ts ----
  "Everything Pako is downloading or has downloaded, with its progress, speed and status. Click a column title to sort, drag its edge to resize it, and right-click the titles to show or hide columns. Right-click a download for more. The bell at the end of a row says how it tells you when it's done: Pop-up, Important (a big pop-up in the middle of the screen) or Off; click it to change. Drag a finished download out of the list to drop it in a folder, an email or another app.":
    "Tout ce que Pako télécharge ou a téléchargé, avec sa progression, sa vitesse et son état. Cliquez sur le titre d’une colonne pour trier, faites glisser son bord pour la redimensionner, et faites un clic droit sur les titres pour afficher ou masquer des colonnes. Faites un clic droit sur un téléchargement pour en voir plus. La cloche au bout d’une ligne indique comment il vous prévient quand il est terminé : Pop-up, Important (un grand pop-up au milieu de l’écran) ou Désactivé ; cliquez dessus pour changer. Faites glisser un téléchargement terminé hors de la liste pour le déposer dans un dossier, un e-mail ou une autre appli.",
  "Show only what's downloading, waiting, finished or has a problem. Sources shows one kind of link, like Google Drive.":
    "N’affichez que ce qui se télécharge, attend, est terminé ou a un problème. Sources n’affiche qu’un type de lien, comme Google Drive.",
  "Downloads from your browser's private windows, encrypted and behind your PIN or password. Click Private (or press Ctrl+Shift+P) to open it.":
    "Les téléchargements des fenêtres privées de votre navigateur, chiffrés et protégés par votre code PIN ou votre mot de passe. Cliquez sur Privé (ou appuyez sur Ctrl+Maj+P) pour l’ouvrir.",
  "Private downloads goes back to your private list, Lock locks it. Going to another page, a filter on the left, Settings or Help locks it too.":
    "Téléchargements privés revient à votre liste privée, Verrouiller la verrouille. Aller sur une autre page, un filtre à gauche, les Paramètres ou l’Aide la verrouille aussi.",
  "Ctrl+F finds a download by name. Resume and Pause work on the selected downloads, or on all of them when none is selected; Remove needs a selection. + adds links, and so does Ctrl+V anywhere in Pako.":
    "Ctrl+F trouve un téléchargement par son nom. Reprendre et Pause agissent sur les téléchargements sélectionnés, ou sur tous quand aucun n’est sélectionné ; Retirer demande une sélection. + ajoute des liens, tout comme Ctrl+V partout dans Pako.",
  "Select a download to see its parts filling in, where it's saved, and what to do if something went wrong. Ctrl+I shows or hides this panel; drag its left edge to make it wider or narrower (each page remembers its width).":
    "Sélectionnez un téléchargement pour voir ses parties se remplir, où il est enregistré, et que faire en cas de problème. Ctrl+I affiche ou masque ce panneau ; faites glisser son bord gauche pour l’élargir ou le rétrécir (chaque page retient sa largeur).",
  "When done lets the PC sleep or shut down once everything has finished, after a short countdown you can cancel. At once sets how many downloads run together.":
    "À la fin laisse le PC se mettre en veille ou s’éteindre quand tout est fini, après un court compte à rebours que vous pouvez annuler. À la fois règle combien de téléchargements se font ensemble.",
  "Links you paste land here first, and Pako checks each one: its name, its size and whether it works (the small mark after the size: point at it to read it, click it when it asks for something). Tick the ones you want. Ctrl+A ticks everything shown, Esc unticks. The same file on several hosts is one row: \"on 3 hosts\" picks the host, the others are backups. Drag a column title's edge to resize it, and right-click the titles to show or hide columns.":
    "Les liens que vous collez arrivent d’abord ici, et Pako vérifie chacun : son nom, sa taille et s’il fonctionne (la petite marque après la taille : pointez dessus pour la lire, cliquez dessus quand elle demande quelque chose). Cochez ceux que vous voulez. Ctrl+A coche tout ce qui est affiché, Échap décoche. Le même fichier sur plusieurs hébergeurs ne fait qu’une ligne : « 3 hébergeurs » choisit l’hébergeur, les autres servent de secours. Faites glisser le bord du titre d’une colonne pour la redimensionner, et faites un clic droit sur les titres pour afficher ou masquer des colonnes.",
  "Only the links that are ready, broken, still being checked, or that only work in the browser.":
    "Seulement les liens prêts, qui ne marchent pas, encore en cours de vérification, ou qui ne marchent que dans le navigateur.",
  "+ pastes more links. The folder picks where they're saved. Download starts the ticked links; when the running ones leave no room it says Add to queue, with Download first next to it. Off, Pop-up and Important (beside it, or in its arrow) say how they tell you when they're done; the bell at the end of a row does the same for one link. With Private mode on, the arrow also offers Download privately: straight into Private mode, encrypted.":
    "+ colle d’autres liens. Le dossier choisit où ils sont enregistrés. Télécharger lance les liens cochés ; quand ceux en cours ne laissent plus de place, il devient Ajouter à la file, avec Télécharger d’abord à côté. Désactivé, Pop-up et Important (à côté, ou dans sa flèche) indiquent comment ils vous préviennent quand ils sont terminés ; la cloche au bout d’une ligne fait de même pour un seul lien. Avec le mode privé activé, la flèche propose aussi Télécharger en privé : directement dans le mode privé, chiffré.",
  "Download on lets one of your other PCs download the ticked links instead, this time only (it's shown while a device that lets this PC control it is online). Their quality and priority go with them.":
    "Télécharger sur laisse un de vos autres PC télécharger les liens cochés à votre place, cette fois seulement (affiché quand un appareil qui laisse ce PC le contrôler est en ligne). Leur qualité et leur priorité les accompagnent.",
  "Select a link to see what Pako found out about it, and give it its own folder if you like. Download on sends it to another of your PCs each time you press Download (a PC mark shows on its row).":
    "Sélectionnez un lien pour voir ce que Pako a appris sur lui, et donnez-lui son propre dossier si vous voulez. Télécharger sur l’envoie à un autre de vos PC chaque fois que vous appuyez sur Télécharger (une marque PC s’affiche sur sa ligne).",
  "Share big files with a link anyone can open. Drag and drop files or folders anywhere on this page, from Explorer or your desktop, or pick them with this button.":
    "Partagez de gros fichiers avec un lien que tout le monde peut ouvrir. Glissez-déposez des fichiers ou des dossiers n’importe où sur cette page, depuis l’Explorateur de fichiers ou votre Bureau, ou choisissez-les avec ce bouton.",
  "Files gather here first: nothing is uploaded until you click Upload. Choose where they go (Gofile needs no account, Google Drive uses your own), when they delete themselves, and how several files are packed. Protect with a password locks them before they leave your PC: people open them with 7-Zip or Pako and that password.":
    "Les fichiers se rassemblent d’abord ici : rien n’est mis en ligne avant que vous cliquiez sur Mettre en ligne. Choisissez où ils vont (Gofile ne demande pas de compte, Google Drive utilise le vôtre), quand ils se suppriment d’eux-mêmes, et comment plusieurs fichiers sont regroupés. Protéger par un mot de passe les verrouille avant qu’ils quittent votre PC : on les ouvre avec 7-Zip ou Pako et ce mot de passe.",
  "Each upload with where it went, when it deletes itself and how often it was downloaded. Copy its link from here. Drag a column title's edge to resize it, and right-click the titles to show or hide columns.":
    "Chaque mise en ligne avec sa destination, quand elle se supprime d’elle-même et combien de fois elle a été téléchargée. Copiez son lien d’ici. Faites glisser le bord du titre d’une colonne pour la redimensionner, et faites un clic droit sur les titres pour afficher ou masquer des colonnes.",
  "Select an upload to see its files, its link as a QR code and how fast it went.":
    "Sélectionnez une mise en ligne pour voir ses fichiers, son lien en QR code et à quelle vitesse elle est partie.",
  "Send files straight to another Pako, PC to PC, nothing uploaded in between. On the left: show who's online, waiting for you or a favourite, and group people with tags like Work. Right-click a tag to rename or delete it.":
    "Envoyez des fichiers directement à un autre Pako, de PC à PC, sans rien mettre en ligne entre les deux. À gauche : affichez qui est en ligne, qui vous attend ou vos favoris, et regroupez les gens avec des étiquettes comme Travail. Faites un clic droit sur une étiquette pour la renommer ou la supprimer.",
  "Everyone you've connected with. A green dot means online, a blue one online on your home network, an amber one that they haven't accepted yet, and a red ! that something waits for your answer. + Add adds someone with their Pako ID. At the top, under You, sending from: the name your contacts see (click it to change it) and your Pako ID to share. Its PC part sends from another of your PCs: pick one to see its contacts and send to them through it. At the bottom: whether Pakos on the same network can see you.":
    "Toutes les personnes avec qui vous vous êtes connecté. Un point vert signifie en ligne, un bleu en ligne sur votre réseau domestique, un orange que la personne n’a pas encore accepté, et un ! rouge que quelque chose attend votre réponse. + Ajouter ajoute quelqu’un avec son ID Pako. En haut, sous Vous, qui envoyez depuis : le nom que voient vos contacts (cliquez dessus pour le changer) et votre ID Pako à partager. Sa partie PC envoie depuis un autre de vos PC : choisissez-en un pour voir ses contacts et leur envoyer des fichiers par lui. En bas : si les Pako du même réseau peuvent vous voir.",
  "Everything you sent each other, with progress and speed. To send, drag and drop files or folders anywhere on this page (from Explorer or your desktop), or use Choose files. They go straight to this person, encrypted.":
    "Tout ce que vous vous êtes envoyé, avec la progression et la vitesse. Pour envoyer, glissez-déposez des fichiers ou des dossiers n’importe où sur cette page (depuis l’Explorateur de fichiers ou votre Bureau), ou utilisez Choisir des fichiers. Ils vont directement à cette personne, chiffrés.",
  "Your speeds with this person, what you exchanged, their tags and where their files go. Check this contact says how often Pako calls them by itself, or often for an hour, today or a week; Mute stops their pop-ups and sounds. Turn on Can send to me without asking for people you trust.":
    "Vos vitesses avec cette personne, ce que vous avez échangé, ses étiquettes et où vont ses fichiers. Vérifier ce contact indique à quelle fréquence Pako l’appelle de lui-même, ou souvent pendant une heure, aujourd’hui ou une semaine ; Mettre en sourdine coupe ses pop-ups et ses sons. Activez Peut m’envoyer sans demander pour les personnes en qui vous avez confiance.",
  "Your own PCs, paired once. Files between them go without anyone accepting, and you can see and start downloads on the other PC. Direct stays for other people.":
    "Vos propres PC, associés une fois. Les fichiers passent de l’un à l’autre sans que personne n’ait à accepter, et vous pouvez voir et lancer des téléchargements sur l’autre PC. Direct reste pour les autres personnes.",
  "Add a device shows this PC's Pako ID. Enter it on your other PC (or that PC's ID here), pick where its files are saved, and confirm on both PCs with the same safety code.":
    "Ajouter un appareil affiche l’ID Pako de ce PC. Saisissez-le sur votre autre PC (ou l’ID de cet autre PC ici), choisissez où ses fichiers sont enregistrés, et confirmez sur les deux PC avec le même code de sécurité.",
  "A green dot means online, a blue one online on your home network (the fastest, no internet used), grey offline. Drop files on a device to send them to it. + Add pairs another PC. Under This PC, click its name to rename it: your other PCs see the new name.":
    "Un point vert signifie en ligne, un bleu en ligne sur votre réseau domestique (le plus rapide, sans passer par internet), gris hors ligne. Déposez des fichiers sur un appareil pour les lui envoyer. + Ajouter associe un autre PC. Sous Ce PC, cliquez sur son nom pour le renommer : vos autres PC voient le nouveau nom.",
  "Files: everything sent between the two PCs, with progress and speed. Links: paste links for that PC to download, with their quality and priority (a playlist opens here first, to pick its videos), and follow each send. Downloads: the other PC's downloads live, pause, resume, change their priority or remove them, and Get this file. Settings: where its files are saved (only there), whether it can control this Pako, and its history.":
    "Fichiers : tout ce qui a été envoyé entre les deux PC, avec la progression et la vitesse. Liens : collez des liens que ce PC doit télécharger, avec leur qualité et leur priorité (une playlist s’ouvre d’abord ici, pour choisir ses vidéos), et suivez chaque envoi. Téléchargements : les téléchargements de l’autre PC en direct ; mettez-les en pause, reprenez-les, changez leur priorité ou retirez-les, et Récupérer ce fichier. Paramètres : où ses fichiers sont enregistrés (là seulement), s’il peut contrôler ce Pako, et son historique.",
  "Your Pako in numbers. Pick Today, This week, This month or All time: the whole page follows it. The gear at the top right saves your numbers as a file for Excel, or starts them over.":
    "Votre Pako en chiffres. Choisissez Aujourd’hui, Cette semaine, Ce mois-ci ou Depuis toujours : toute la page suit. La roue dentée en haut à droite enregistre vos chiffres dans un fichier pour Excel, ou les remet à zéro.",
  "Your numbers show up here as you download, upload and send.":
    "Vos chiffres apparaissent ici à mesure que vous téléchargez, mettez en ligne et envoyez.",
  "What you downloaded, uploaded, sent and received in that period, and how it spread over the days. Point at a bar for its numbers.":
    "Ce que vous avez téléchargé, mis en ligne, envoyé et reçu sur cette période, et comment cela s’est réparti sur les jours. Pointez une barre pour voir ses chiffres.",
  "Your average and top speeds, and which way is fastest for you: Gofile, Google Drive or Direct.":
    "Vos vitesses moyenne et de pointe, et le chemin le plus rapide pour vous : Gofile, Google Drive ou Direct.",
  "Who you exchanged the most with in Direct and your best speeds with them, then your records: fastest download, biggest file and more.":
    "Les personnes avec qui vous avez le plus échangé dans Direct et vos meilleures vitesses avec elles, puis vos records : téléchargement le plus rapide, plus gros fichier et plus encore.",
  "Downloads from your browser's private windows, links you sent with Download privately, and files you moved here. Their files are encrypted on this PC: they only open from here. A folder download is one folder row. Double-click a file: a video plays in Pako, anything else opens from a temporary copy that Pako deletes afterwards. Drag a column's edge to make it wider or narrower, and right-click the column titles to show or hide columns.":
    "Les téléchargements des fenêtres privées de votre navigateur, les liens envoyés avec Télécharger en privé, et les fichiers que vous avez déplacés ici. Leurs fichiers sont chiffrés sur ce PC : ils ne s’ouvrent que d’ici. Un téléchargement de dossier tient en une ligne de dossier. Double-cliquez sur un fichier : une vidéo se lit dans Pako, tout le reste s’ouvre depuis une copie temporaire que Pako supprime ensuite. Faites glisser le bord d’une colonne pour l’élargir ou la rétrécir, et faites un clic droit sur les titres des colonnes pour afficher ou masquer des colonnes.",
  "On each row: play or open it, send it with Direct or Upload (read decrypted as it goes, no readable copy), Decrypt it into a readable file where you pick (then take it off the private list, move it to Downloads, or leave it here too), or delete it for good. Tick the boxes (or Ctrl-click, Shift-click, Ctrl+A) to Decrypt or Delete several at once, or a whole folder.":
    "Sur chaque ligne : lisez-le ou ouvrez-le, envoyez-le avec Direct ou mettez-le en ligne (lu déchiffré au fil de l’envoi, sans copie lisible), Déchiffrez-le en un fichier lisible là où vous voulez (puis retirez-le de la liste privée, déplacez-le dans Téléchargements, ou laissez-le aussi ici), ou supprimez-le pour de bon. Cochez les cases (ou Ctrl+clic, Maj+clic, Ctrl+A) pour Déchiffrer ou Supprimer plusieurs éléments à la fois, ou tout un dossier.",
  "The picked download: its progress, where it came from and when it was downloaded; while it downloads, its priority and how it tells you it's done (Off, Pop-up or Important). Its buttons are at the bottom: Open or Play, Decrypt…, Send…, Upload…, Delete…. Ctrl+I hides or shows it.":
    "Le téléchargement choisi : sa progression, sa provenance et sa date de téléchargement ; pendant qu’il se télécharge, sa priorité et comment il vous prévient quand il est terminé (Désactivé, Pop-up ou Important). Ses boutons sont en bas : Ouvrir ou Lire, Déchiffrer…, Envoyer…, Mettre en ligne…, Supprimer…. Ctrl+I le masque ou l’affiche.",
  "Normal list shows your other downloads and keeps Private mode open (Private downloads, in the same place, brings you back). Lock locks it now (Ctrl+Shift+P too). Going anywhere else locks it too: another page, a filter on the left, Settings or Help. It also locks by itself when you step away, lock Windows, minimise or close Pako. Sending, uploading and Decrypt… show their progress right here.":
    "Liste normale affiche vos autres téléchargements et garde le mode privé ouvert (Téléchargements privés, au même endroit, vous y ramène). Verrouiller le verrouille tout de suite (Ctrl+Maj+P aussi). Aller ailleurs le verrouille aussi : une autre page, un filtre à gauche, les Paramètres ou l’Aide. Il se verrouille aussi tout seul quand vous vous éloignez, verrouillez Windows, réduisez ou fermez Pako. Les envois, les mises en ligne et Déchiffrer… affichent leur progression juste ici.",
  "One tab per part of Settings, each in small titled groups: click a tab, or use ↑ ↓ and Enter (Ctrl+Tab from anywhere in Settings). Search finds any setting or group by name, even under Advanced. Save keeps the changes in every tab at once; a dot marks a tab with changes not saved yet.":
    "Un onglet par partie des Paramètres, chacun en petits groupes avec un titre : cliquez sur un onglet, ou utilisez ↑ ↓ et Entrée (Ctrl+Tab depuis n’importe où dans les Paramètres). La recherche trouve n’importe quel réglage ou groupe par son nom, même sous Avancé. Enregistrer garde les changements de tous les onglets à la fois ; un point marque un onglet avec des changements pas encore enregistrés.",
  "Pako's version and its updates. The tools for features like videos fold under What Pako can do. At the bottom of each tab, Advanced holds what most people never change: here, running the first-start setup again.":
    "La version de Pako et ses mises à jour. Les outils des fonctions comme les vidéos se replient sous Ce que Pako sait faire. En bas de chaque onglet, Avancé contient ce que la plupart des gens ne changent jamais : ici, relancer la configuration du premier démarrage.",
  "Connect Pako to your browsers, add the extension (each browser says when it's Installed), and choose whether Pako asks before each browser download.":
    "Connectez Pako à vos navigateurs, ajoutez l’extension (chaque navigateur affiche Installé une fois qu’elle y est), et choisissez si Pako demande avant chaque téléchargement du navigateur.",
  "Connect your Google Drive for uploads. Downloads never need a sign-in.":
    "Connectez votre Google Drive pour les mises en ligne. Les téléchargements ne demandent jamais de connexion à un compte.",
  "The language (English or French), light or dark, what minimising and closing do, starting with Windows, and asking before quitting.":
    "La langue (anglais ou français), clair ou sombre, ce que font la réduction et la fermeture de la fenêtre, le démarrage avec Windows, et la confirmation avant de quitter.",
  "How new downloads tell you they're done (Off, Pop-up or Important) and when they chime, what shows as a pop-up and what the bell keeps, the sounds, and Do not disturb.":
    "Comment les nouveaux téléchargements vous préviennent quand ils sont terminés (Désactivé, Pop-up ou Important) et quand ils sonnent, ce qui s’affiche en pop-up et ce que garde la cloche, les sons, et Ne pas déranger.",
  "Speed (how many at once, a speed limit, set hours), saving files (your download folder, split archives), videos, what happens while downloading, and downloads from private windows. Connections per file and other rarely changed ones are under Advanced.":
    "La vitesse (combien à la fois, une limite de vitesse, des horaires), l’enregistrement des fichiers (votre dossier de téléchargement, les archives en plusieurs parties), les vidéos, ce qui se passe pendant les téléchargements, et les téléchargements des fenêtres privées. Les connexions par fichier et d’autres réglages rarement changés sont sous Avancé.",
  "Turn on Private mode to keep downloads from private windows behind a PIN or a password, then choose when it locks, when its list clears, and manage the PIN and recovery code.":
    "Activez le mode privé pour garder les téléchargements des fenêtres privées derrière un code PIN ou un mot de passe, puis choisissez quand il se verrouille et quand sa liste se vide, et gérez le code PIN et le code de récupération.",
  "Where uploads go and how several files are packed.":
    "Où vont les mises en ligne et comment plusieurs fichiers sont regroupés.",
  "A one-minute look around. Every page has a Tutorial button like this one for its details. Use → and ←, and Esc stops.":
    "Un petit tour d’une minute. Chaque page a un bouton Tutoriel comme celui-ci pour ses détails. Utilisez → et ←, et Échap arrête.",
  "Paste links anywhere with Ctrl+V. Pako checks them in Links, you pick what you want, and they download in Downloads.":
    "Collez des liens n’importe où avec Ctrl+V. Pako les vérifie dans Liens, vous choisissez ce que vous voulez, et ils se téléchargent dans Téléchargements.",
  "Upload shares big files with a link anyone can open. Direct sends files straight to another Pako, PC to PC.":
    "Envoi partage de gros fichiers avec un lien que tout le monde peut ouvrir. Direct envoie des fichiers directement à un autre Pako, de PC à PC.",
  "Your own PCs, paired once: send files between them without asking, and see or start downloads on the other one.":
    "Vos propres PC, associés une fois : envoyez des fichiers de l’un à l’autre sans demander, et voyez ou lancez des téléchargements sur l’autre.",
  "Your Pako in numbers: what you moved, how fast, and with whom.":
    "Votre Pako en chiffres : ce que vous avez transféré, à quelle vitesse, et avec qui.",
  "Downloads kept behind your PIN or password, encrypted on this PC. Click Private or press Ctrl+Shift+P.":
    "Des téléchargements gardés derrière votre code PIN ou votre mot de passe, chiffrés sur ce PC. Cliquez sur Privé ou appuyez sur Ctrl+Maj+P.",
  "Guides, What's new, every tutorial and Report a problem are in Help (F1). Settings has your download folder, the browser extension and more.":
    "Les guides, les Nouveautés, tous les tutoriels et Signaler un problème sont dans l’Aide (F1). Les Paramètres contiennent votre dossier de téléchargement, l’extension de navigateur et plus encore.",
  "Pako also lives in the tray: one click on its icon shows today's downloads, uploads and Direct transfers in a small window. Pin it to keep it on top.":
    "Pako vit aussi dans la zone de notification : un clic sur son icône affiche les téléchargements, mises en ligne et transferts Direct du jour dans une petite fenêtre. Épinglez-la pour la garder au premier plan.",
} satisfies Record<string, Entry>;
