// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for Settings, first-start setup and the terms of use (SettingsDialog, SetupDialog, TermsDialog). See ./fr.ts for the glossary and the rules.
// Words of this area: réglage (one setting; Settings itself is Paramètres) · Avancé (Advanced) ·
// zone de notification (tray, also "taskbar tray") · pop-up · cloche (bell) · code PIN.
import type { Entry } from "./core";

export const settings = {
  // ---- The window, its tabs and the search ----
  "Settings saved": "Paramètres enregistrés",
  "Search settings": "Rechercher un réglage",
  "Settings found": "Réglages trouvés",
  "Settings sections": "Sections des Paramètres",
  Advanced: "Avancé",
  group: "groupe",
  tab: "onglet",
  "No setting matches '{words}'": "Aucun réglage ne correspond à « {words} »",
  "Changed, not saved yet": "Modifié, pas encore enregistré",
  Upload: "Envoi",
  "Save changes?": "Enregistrer les modifications ?",
  "Some settings are changed but not saved yet.": "Des réglages ont été modifiés mais pas encore enregistrés.",
  "Keep editing": "Continuer à modifier",
  Discard: "Ne pas enregistrer",

  // ---- Features & updates ----
  Installed: "Installé",
  Portable: "Portable",
  "Pako {version} is ready. Restart to start using it.": "Pako {version} est prêt. Redémarrez pour l’utiliser.",
  "Pako {version} is available.": "Pako {version} est disponible.",
  "You have the latest version.": "Vous avez la dernière version.",
  "Pako looks for a new version once a day, also while it stays open in the tray.":
    "Pako cherche une nouvelle version une fois par jour, même quand il reste ouvert dans la zone de notification.",
  Update: "Mettre à jour",
  "Check for updates": "Rechercher des mises à jour",
  "Update Pako and its tools automatically": "Mettre à jour Pako et ses outils automatiquement",
  "Once a day, also while Pako stays open in the tray. Tools update quietly; a new Pako waits for you to restart.":
    "Une fois par jour, même quand Pako reste ouvert dans la zone de notification. Les outils se mettent à jour en silence ; un nouveau Pako attend que vous redémarriez.",
  Features: "Fonctions",
  "What Pako can do": "Ce que Pako sait faire",
  "{n} updates": { one: "{n} mise à jour", other: "{n} mises à jour" },
  "Check for new versions": "Rechercher de nouvelles versions",
  "Some features use free tools that Pako downloads from their official sources and checks that each one arrived intact.":
    "Certaines fonctions utilisent des outils gratuits que Pako télécharge depuis leurs sources officielles, en vérifiant que chacun est arrivé intact.",
  "Coming soon": "Bientôt",
  "Update available": "Mise à jour disponible",
  "Partly installed": "Installée en partie",
  "Not installed": "Pas installée",
  "Working…": "En cours…",
  Install: "Installer",
  "Removing…": "Retrait…",
  "{feature}: removed": "{feature} : retirée",
  Installing: "Installation",
  "{done} of {total} (bytes)": "{done} sur {total}",
  "version {version}": "version {version}",
  "not installed": "pas installé",
  "latest {version}": "dernière {version}",
  "Extract split archives": "Extraire les archives en plusieurs parties",
  "When every part of a set (.part1.rar, .7z.001…) is downloaded, Pako unpacks it for you.":
    "Quand toutes les parties d’un ensemble (.part1.rar, .7z.001…) sont téléchargées, Pako le décompresse pour vous.",
  "Watch MKV, AVI and other videos in the details panel before downloading them.":
    "Regardez les vidéos MKV, AVI et autres dans le panneau de détails avant de les télécharger.",
  "Download videos from YouTube and other sites": "Télécharger des vidéos depuis YouTube et d’autres sites",
  "Paste a video or playlist link, pick the quality (or audio only), and Pako does the rest.":
    "Collez le lien d’une vidéo ou d’une playlist, choisissez la qualité (ou l’audio seul), et Pako fait le reste.",
  "Setup: run the first-start setup again": "Configuration : refaire la configuration du premier démarrage",
  Setup: "Configuration",
  "First-start setup": "Configuration du premier démarrage",
  "Go through the welcome questions again.": "Repasser par les questions d’accueil.",
  "Run setup again": "Refaire la configuration",

  // ---- Browser extension ----
  "Connect Pako to your browsers": "Connecter Pako à vos navigateurs",
  "Lets the Pako extension send downloads, links and videos to Pako. It talks to Pako directly on this PC: nothing goes over the internet.":
    "Permet à l’extension Pako d’envoyer téléchargements, liens et vidéos à Pako. Elle parle à Pako directement sur ce PC : rien ne passe par Internet.",
  "Add Pako to your browser": "Ajouter Pako à votre navigateur",
  "How to add Pako to your browser": "Comment ajouter Pako à votre navigateur",
  "Pick your browser: Pako opens its extensions page and shows the three steps. Then right-click a link › Download link with Pako, or use the Download button on YouTube videos.":
    "Choisissez votre navigateur : Pako ouvre sa page des extensions et montre les trois étapes. Ensuite, clic droit sur un lien › Télécharger le lien avec Pako, ou le bouton Télécharger sur les vidéos YouTube.",
  "Browser not listed?": "Votre navigateur n’est pas dans la liste ?",
  "How to add Pako to another browser": "Comment ajouter Pako à un autre navigateur",
  "Browser downloads": "Téléchargements du navigateur",
  "Ask before each browser download": "Demander avant chaque téléchargement du navigateur",
  "A small window shows the name, folder and quality first. Off: downloads from your browser start right away.":
    "Une petite fenêtre montre d’abord le nom, le dossier et la qualité. Désactivé : les téléchargements de votre navigateur démarrent tout de suite.",
  "Show the small window when a browser download starts": "Afficher la petite fenêtre quand un téléchargement du navigateur démarre",
  "The tray's list opens at the bottom right so you can see your downloads at a glance. Pin it to keep it on top. Never for a download from a private window.":
    "La liste de la zone de notification s’ouvre en bas à droite pour voir vos téléchargements d’un coup d’œil. Épinglez-la pour la garder au premier plan. Jamais pour un téléchargement depuis une fenêtre privée.",
  "Connection: the extension's files": "Connexion : les fichiers de l’extension",
  "The extension's files": "Les fichiers de l’extension",
  "For adding Pako to a browser by hand: Pako writes the extension next to its data and opens that folder.":
    "Pour ajouter Pako à un navigateur à la main : Pako écrit l’extension à côté de ses données et ouvre ce dossier.",
  "Open the extension folder": "Ouvrir le dossier de l’extension",
  // AddToBrowser
  "Looking for your browsers…": "Recherche de vos navigateurs…",
  "No browser found that Pako's extension works in (Chrome, Edge, Brave, Vivaldi, Opera, Firefox, Zen, LibreWolf, Waterfox, Floorp).":
    "Aucun navigateur trouvé où l’extension de Pako fonctionne (Chrome, Edge, Brave, Vivaldi, Opera, Firefox, Zen, LibreWolf, Waterfox, Floorp).",
  "Not used lately": "Pas utilisé récemment",
  "last used today": "utilisé aujourd’hui",
  "last used yesterday": "utilisé hier",
  "last used {n} days ago": { one: "utilisé il y a {n} jour", other: "utilisé il y a {n} jours" },
  "last used {n} weeks ago": { one: "utilisé il y a {n} semaine", other: "utilisé il y a {n} semaines" },
  "last used {n} months ago": { one: "utilisé il y a {n} mois", other: "utilisé il y a {n} mois" },
  "last used {n} years ago": { one: "utilisé il y a {n} an", other: "utilisé il y a {n} ans" },
  "Open again": "Rouvrir",
  "Add to {browser}": "Ajouter à {browser}",
  "{browser} didn't open its extensions page, so the folder opened instead. Type {page} in {browser}'s address bar.":
    "{browser} n’a pas ouvert sa page des extensions, le dossier s’est donc ouvert à la place. Tapez {page} dans la barre d’adresse de {browser}.",
  "On the extensions page, turn on {button}.": "Sur la page des extensions, activez {button}.",
  // The browsers' own words (Chrome in French).
  "Developer mode": "Mode développeur",
  "Click {button}.": "Cliquez sur {button}.",
  "Load unpacked": "Charger l’extension non empaquetée",
  "Click the address box at the top of the window that opens, press {paste} (the folder is already copied) and {enter}, then {button}.":
    "Cliquez dans la barre d’adresse en haut de la fenêtre qui s’ouvre, appuyez sur {paste} (le dossier est déjà copié) puis {enter}, et cliquez sur {button}.",
  "Click the address box at the top of the window that opens, press {paste} and {enter}, then {button}.":
    "Cliquez dans la barre d’adresse en haut de la fenêtre qui s’ouvre, appuyez sur {paste} puis {enter}, et cliquez sur {button}.",
  "Ctrl+V": "Ctrl+V",
  "Select Folder": "Sélectionner un dossier",
  "Copied": "Copié",
  "Copy again": "Copier à nouveau",
  "This copy of Pako doesn't include the Firefox version. Pako from {site} has it.":
    "Cette copie de Pako ne contient pas la version pour Firefox. Le Pako de {site} l’a.",
  "{browser} didn't open, so the folder opened instead: drag {file} into a {browser} window.":
    "{browser} ne s’est pas ouvert, le dossier s’est donc ouvert à la place : faites glisser {file} dans une fenêtre de {browser}.",
  "{browser} asks to add Pako: click {button}.": "{browser} demande s’il faut ajouter Pako : cliquez sur {button}.",
  // Firefox's own words, in French.
  "Add (Firefox button)": "Ajouter",
  "So downloads behind a login work and the Download button shows on videos, check it may use all websites: in {browser}, open {page} › Pako › {permissions} and turn on {access} if it is off.":
    "Pour que les téléchargements derrière une connexion marchent et que le bouton Télécharger s’affiche sur les vidéos, vérifiez qu’elle peut utiliser tous les sites : dans {browser}, ouvrez {page} › Pako › {permissions} et activez {access} si ce n’est pas fait.",
  Permissions: "Permissions",
  "Access your data for all websites": "Accéder à vos données sur tous les sites web",

  // ---- Accounts ----
  "Google Drive for uploads": "Google Drive pour les mises en ligne",
  "Not set up in this Pako. Uploads go to Gofile.": "Pas configuré dans ce Pako. Les mises en ligne vont sur Gofile.",
  "Google account": "Compte Google",
  "Upload big files to your own Drive. Pako only sees the files it uploads.":
    "Mettez de gros fichiers en ligne sur votre propre Drive. Pako ne voit que les fichiers qu’il met en ligne.",
  Disconnect: "Déconnecter",
  "Waiting for the browser…": "En attente du navigateur…",
  "Only used for uploads: downloads from Drive never need a sign-in. What you uploaded stays in your Drive if you disconnect.":
    "Utilisé seulement pour les mises en ligne : télécharger depuis Drive ne demande jamais de connexion. Ce que vous avez mis en ligne reste dans votre Drive si vous vous déconnectez.",
  "Google Drive connected for uploads ({email})": "Google Drive connecté pour les mises en ligne ({email})",
  "Google Drive connected for uploads": "Google Drive connecté pour les mises en ligne",

  // ---- Window ----
  Look: "Apparence",
  "Match Windows follows Windows' own light or dark mode. The sun or moon at the top of Pako switches too.":
    "Comme Windows suit le mode clair ou sombre de Windows. Le soleil ou la lune en haut de Pako le change aussi.",
  Dark: "Sombre",
  Light: "Clair",
  "Match Windows": "Comme Windows",
  Tray: "Zone de notification",
  "Minimize hides Pako next to the clock; it keeps downloading. Click the tray icon for a small list, double-click it to open Pako.":
    "Réduire cache Pako à côté de l’horloge ; il continue de télécharger. Cliquez sur l’icône de la zone de notification pour une petite liste, double-cliquez dessus pour ouvrir Pako.",
  "Close to the taskbar tray": "Fermer dans la zone de notification",
  "The close button hides Pako instead of quitting. Quit from the tray icon's menu.":
    "Le bouton Fermer cache Pako au lieu de le quitter. Quittez depuis le menu de l’icône de la zone de notification.",
  "Starting and quitting": "Démarrer et quitter",
  "Start Pako with Windows": "Démarrer Pako avec Windows",
  "Pako opens when you sign in, so waiting downloads carry on by themselves.":
    "Pako s’ouvre quand vous ouvrez votre session : les téléchargements en attente continuent d’eux-mêmes.",
  "Ask before quitting while downloading or uploading": "Demander avant de quitter pendant un téléchargement ou une mise en ligne",
  "Quitting stops downloads and uploads. Downloads continue next time you open Pako; uploads don't.":
    "Quitter arrête les téléchargements et les mises en ligne. Les téléchargements reprennent à la prochaine ouverture de Pako ; pas les mises en ligne.",
  "Tray: start in the tray": "Zone de notification : démarrer dans la zone de notification",
  "Start in the tray": "Démarrer dans la zone de notification",
  "No window at sign-in: Pako waits next to the clock.": "Aucune fenêtre à l’ouverture de session : Pako attend à côté de l’horloge.",
  "Used with Start Pako with Windows.": "Fonctionne avec Démarrer Pako avec Windows.",

  // ---- Notifications ----
  "Pop-ups show at the bottom right of the screen for a few seconds. The bell, at the top of Pako, keeps the important ones for 30 days.":
    "Les pop-ups s’affichent en bas à droite de l’écran pendant quelques secondes. La cloche, en haut de Pako, garde les importantes pendant 30 jours.",
  "When a download finishes": "Quand un téléchargement se termine",
  "New downloads": "Nouveaux téléchargements",
  "How a download tells you it's done. Each one can change it: the bell on its row, its details, or right-click.":
    "Comment un téléchargement vous prévient qu’il est terminé. Réglable aussi pour chaque téléchargement : la cloche sur sa ligne, ses détails, ou un clic droit.",
  "Important: a big pop-up in the middle of the screen that stays until you click it.":
    "Important : un grand pop-up au milieu de l’écran, qui reste jusqu’à ce que vous cliquiez dessus.",
  "Chime when a download finishes": "Son quand un téléchargement se termine",
  "A short, soft chime, with the pop-up.": "Un son court et doux, avec le pop-up.",
  "Each kind of news": "Par type de notification",
  "What shows as a pop-up and in the bell": "Ce qui s’affiche en pop-up et dans la cloche",
  What: "Quoi",
  "Pop-up": "Pop-up",
  Bell: "Cloche",
  "Set by New downloads, and for each download": "Réglé par Nouveaux téléchargements, et pour chaque téléchargement",
  "Per download": "Au cas par cas",
  "{kind}: pop-up": "{kind} : pop-up",
  "{kind}: bell": "{kind} : cloche",
  "While the small window is open, it shows downloads finishing instead of their pop-up. Important always shows, also during Do not disturb, but waits while something is full screen (a game, a video, a presentation).":
    "Quand la petite fenêtre est ouverte, elle montre les téléchargements qui se terminent à la place de leur pop-up. Important s’affiche toujours, même en Ne pas déranger, mais attend tant que quelque chose est en plein écran (un jeu, une vidéo, une présentation).",
  Sounds: "Sons",
  "Play a sound when files arrive": "Jouer un son quand des fichiers arrivent",
  "A short, soft chime when files arrive in Direct and My devices, and when someone offers you files. Silent during Do not disturb, Pako's or Windows'.":
    "Un son court et doux quand des fichiers arrivent dans Direct et Mes appareils, et quand quelqu’un vous propose des fichiers. Muet en Ne pas déranger, celui de Pako ou de Windows.",
  "Play a sound when files are sent": "Jouer un son quand des fichiers sont envoyés",
  "A brighter sound when an upload finishes, or when a Direct or My devices send is delivered. Silent at the same times.":
    "Un son plus clair quand une mise en ligne se termine, ou quand un envoi Direct ou Mes appareils est livré. Muet aux mêmes moments.",
  "Quiet times": "Moments de calme",
  "Do not disturb": "Ne pas déranger",
  "No pop-ups, except for downloads set to Important. The bell still collects.":
    "Aucun pop-up, sauf pour les téléchargements réglés sur Important. La cloche continue de tout garder.",
  "Quiet times: follow Windows' Do not disturb": "Moments de calme : suivre Ne pas déranger de Windows",
  "Follow Windows' Do not disturb": "Suivre Ne pas déranger de Windows",
  "No pop-ups during full-screen games and videos, presentations, or Windows' Do not disturb. The bell still collects.":
    "Aucun pop-up pendant les jeux et vidéos en plein écran, les présentations, ou Ne pas déranger de Windows. La cloche continue de tout garder.",

  // ---- Downloads ----
  "Downloads at the same time": "Téléchargements en même temps",
  "The rest wait in the queue and start on their own.": "Les autres attendent dans la file et démarrent d’eux-mêmes.",
  "Speed limit": "Limite de vitesse",
  "Total for all downloads, in KB/s. 0 = no limit.": "Total pour tous les téléchargements, en Ko/s. 0 = aucune limite.",
  "Speed limit in KB/s": "Limite de vitesse en Ko/s",
  "Only download at set hours": "Télécharger seulement à certaines heures",
  "Outside these hours downloads wait, and running ones pause and carry on when the hours start. \"Download first\" still starts one right away.":
    "En dehors de ces heures, les téléchargements attendent, et ceux en cours se mettent en pause puis reprennent quand les heures commencent. « Télécharger d’abord » en démarre quand même un tout de suite.",
  From: "De",
  "Start time": "Heure de début",
  to: "à",
  "End time": "Heure de fin",
  "(overnight)": "(pendant la nuit)",
  "Saving files": "Enregistrement des fichiers",
  "Download folder": "Dossier de téléchargement",
  "When every part of a split archive (.part1.rar, .7z.001…) is downloaded, extract it into its folder. Each set can change this in its details panel.":
    "Quand toutes les parties d’une archive en plusieurs parties (.part1.rar, .7z.001…) sont téléchargées, l’extraire dans son dossier. Chaque ensemble peut changer cela dans son panneau de détails.",
  "Needs 7-Zip (Features & updates).": "Nécessite 7-Zip (Fonctions et mises à jour).",
  Videos: "Vidéos",
  "Used for new video links. If a video doesn't have it, Pako takes the closest lower one. Each video can change it in its details.":
    "Utilisée pour les nouveaux liens de vidéos. Si une vidéo ne l’a pas, Pako prend la plus proche en dessous. Chaque vidéo peut la changer dans ses détails.",
  "Up to 4K (2160p)": "Jusqu’à 4K (2160p)",
  "Up to {p}p": "Jusqu’à {p}p",
  "Audio only (M4A)": "Audio seul (M4A)",
  "While downloading": "Pendant les téléchargements",
  "Keep the PC awake while downloading": "Garder le PC éveillé pendant les téléchargements",
  "Windows won't go to sleep in the middle of a download. The screen can still turn off.":
    "Windows ne se met pas en veille au milieu d’un téléchargement. L’écran peut quand même s’éteindre.",
  "When all downloads finish": "Quand tous les téléchargements sont finis",
  "For this session only: Pako starts with \"Do nothing\" each time. Also in the bottom bar.":
    "Pour cette session seulement : Pako démarre sur « Ne rien faire » à chaque fois. Aussi dans la barre du bas.",
  "Catch copied links": "Récupérer les liens copiés",
  "Links you copy in any app go to Links by themselves. Links already in Pako are skipped.":
    "Les liens que vous copiez dans n’importe quelle appli vont d’eux-mêmes dans Liens. Ceux déjà dans Pako sont ignorés.",
  "From private windows": "Depuis les fenêtres privées",
  "Remove private downloads from the list": "Retirer les téléchargements privés de la liste",
  "Private mode is on: downloads from private windows go to Private.":
    "Le mode privé est activé : les téléchargements des fenêtres privées vont dans Privé.",
  "Downloads from a private browser window (Done · private). Opening one from Pako, or its ✕, removes it at once. The file stays.":
    "Les téléchargements d’une fenêtre privée du navigateur (Terminé · privé). L’ouvrir depuis Pako, ou son ✕, le retire tout de suite. Le fichier reste.",
  "10 minutes after they finish": "10 minutes après la fin",
  "1 hour after": "1 heure après",
  "When you quit Pako": "Quand vous quittez Pako",
  "Speed, saving files, videos and while downloading: connections, checks, previews, continuing at start":
    "Vitesse, enregistrement, vidéos et pendant les téléchargements : connexions, vérifications, aperçus, reprise au démarrage",
  "Connections per file": "Connexions par fichier",
  "Auto adds connections while it makes the file faster and stops when the server or your internet can't take more.":
    "Auto ajoute des connexions tant que le fichier va plus vite, et s’arrête quand le serveur ou votre connexion Internet ne peut pas suivre.",
  "Auto connections": "Connexions auto",
  "Check Drive files when finished": "Vérifier les fichiers Drive une fois finis",
  "Compares each finished file with Google's own record of it, so a damaged file is caught.":
    "Compare chaque fichier fini avec la trace qu’en garde Google, pour repérer un fichier abîmé.",
  "Video preview quality": "Qualité de l’aperçu vidéo",
  "The small player in the details panel. Lower uses less data; each video can change it.":
    "Le petit lecteur du panneau de détails. Plus bas, il utilise moins de données ; chaque vidéo peut la changer.",
  "Continue downloads when Pako starts": "Reprendre les téléchargements au démarrage de Pako",
  "Downloads that were running when Pako closed carry on by themselves. Turned off, they're paused until you click Resume.":
    "Les téléchargements en cours à la fermeture de Pako continuent d’eux-mêmes. Désactivé, ils restent en pause jusqu’à ce que vous cliquiez sur Reprendre.",

  // ---- Private mode ----
  "On or off": "Activé ou désactivé",
  "Private mode: keep private downloads behind a PIN": "Mode privé : garder les téléchargements privés derrière un code PIN",
  "On. Downloads from private windows go to {private} in Downloads, encrypted on this PC.":
    "Activé. Les téléchargements des fenêtres privées vont dans {private} dans Téléchargements, chiffrés sur ce PC.",
  "Open it with the button, with {private} in Downloads, or with Ctrl+Shift+P.":
    "Ouvrez-le avec le bouton, avec {private} dans Téléchargements, ou avec Ctrl+Maj+P.",
  "Downloads from your browser's private windows go to a Private list in Downloads instead, their files encrypted on this PC, behind a PIN or a password.":
    "Les téléchargements des fenêtres privées de votre navigateur vont plutôt dans une liste Privé dans Téléchargements, leurs fichiers chiffrés sur ce PC, derrière un code PIN ou un mot de passe.",
  "Open (unlocked)": "Ouvert",
  "Close Settings and type your PIN or password": "Fermer les Paramètres et saisir votre code PIN ou mot de passe",
  "Open Private mode…": "Ouvrir le mode privé…",
  Locking: "Verrouillage",
  "Lock Private mode after": "Verrouiller le mode privé après",
  "With no keyboard or mouse use on the PC. It also locks at once when Windows locks, the PC sleeps, the screen saver starts, or Pako is minimised or closed.":
    "Sans utiliser le clavier ni la souris sur le PC. Il se verrouille aussi tout de suite quand Windows se verrouille, que le PC se met en veille, que l’écran de veille démarre, ou que Pako est réduit ou fermé.",
  "1 minute": "1 minute",
  "5 minutes": "5 minutes",
  "15 minutes": "15 minutes",
  "30 minutes": "30 minutes",
  "Never for being idle": "Jamais pour inactivité",
  "Clear the private list": "Vider la liste privée",
  "Private downloads, and their files, are deleted this long after they finished. Downloads still running stay.":
    "Les téléchargements privés, et leurs fichiers, sont supprimés ce temps-là après leur fin. Les téléchargements encore en cours restent.",
  Hours: "Heures",
  hours: "heures",
  "3 hours after": "3 heures après",
  "12 hours after": "12 heures après",
  "3 days after": "3 jours après",
  "After a number of hours…": "Après un nombre d’heures…",
  "Opening files": "Ouverture des fichiers",
  "Show real names when opening private files": "Afficher les vrais noms en ouvrant les fichiers privés",
  "Off: a file opened from Private mode is named like PrivateFile_2026-11-18_14h32.pdf, so its name doesn't show in the app's title or Windows' recent files.":
    "Désactivé : un fichier ouvert depuis le mode privé porte un nom comme PrivateFile_2026-11-18_14h32.pdf, pour que son nom n’apparaisse ni dans le titre de l’appli ni dans les fichiers récents de Windows.",
  "PIN and recovery": "Code PIN et récupération",
  "PIN, password and recovery code": "Code PIN, mot de passe et code de récupération",
  "A password opens Private mode.": "Un mot de passe ouvre le mode privé.",
  "A PIN opens Private mode.": "Un code PIN ouvre le mode privé.",
  "You have a recovery code.": "Vous avez un code de récupération.",
  "No recovery code yet: without one, a forgotten PIN or password can't be undone.":
    "Pas encore de code de récupération : sans lui, rien ne permet de rattraper un code PIN ou un mot de passe oublié.",
  "Change PIN or password…": "Changer le code PIN ou le mot de passe…",
  "New recovery code…": "Nouveau code de récupération…",
  "Deletes the private list and every file in it, for good, and turns Private mode off. The only way back when the PIN or password is forgotten and there's no recovery code.":
    "Supprime pour de bon la liste privée et tous ses fichiers, et désactive le mode privé. Le seul moyen de s’en sortir quand le code PIN ou le mot de passe est oublié et qu’il n’y a pas de code de récupération.",
  "Reset…": "Réinitialiser…",
  "Reset Private mode?": "Réinitialiser le mode privé ?",
  "The private list and every file in it are deleted for good (not to the Recycle Bin), and Private mode turns off. Downloads from private windows go back to showing as Done · private.":
    "La liste privée et tous ses fichiers sont supprimés pour de bon (pas dans la Corbeille), et le mode privé se désactive. Les téléchargements des fenêtres privées s’affichent de nouveau comme Terminé · privé.",
  "Delete everything": "Tout supprimer",
  "Private mode is off. Its list and files are deleted.": "Le mode privé est désactivé. Sa liste et ses fichiers sont supprimés.",

  // ---- Upload ----
  Where: "Où",
  "Upload with": "Mettre en ligne avec",
  "Your own Google Drive: shared with anyone who has the link, kept until you delete it.":
    "Votre propre Google Drive : partagé avec toute personne qui a le lien, gardé jusqu’à ce que vous le supprimiez.",
  "Gofile: no account needed. Files are kept 10 days after their last download.":
    "Gofile : aucun compte nécessaire. Les fichiers sont gardés 10 jours après leur dernier téléchargement.",
  "When you upload several files": "Quand vous mettez en ligne plusieurs fichiers",
  "They're packed into one ZIP file: whoever opens the link downloads one file and opens it.":
    "Ils sont réunis dans un seul fichier ZIP : la personne qui ouvre le lien télécharge un fichier et l’ouvre.",
  "One ZIP file, made smaller where it helps (documents, text). Takes longer to upload, and doesn't shrink videos or photos.":
    "Un seul fichier ZIP, réduit là où ça aide (documents, texte). La mise en ligne prend plus longtemps, et les vidéos ou photos ne rétrécissent pas.",
  "One ZIP file, cut into numbered parts (their size is under Advanced).":
    "Un seul fichier ZIP, coupé en parties numérotées (leur taille est sous Avancé).",
  "Each file is uploaded as it is, a few at a time, into one folder. Whoever opens the link sees the files straight away.":
    "Chaque fichier est mis en ligne tel quel, quelques-uns à la fois, dans un seul dossier. La personne qui ouvre le lien voit les fichiers tout de suite.",
  "A single file goes as it is, unless you pick Compressed ZIP or ZIP in parts for it in the upload.":
    "Un fichier seul part tel quel, sauf si vous choisissez ZIP compressé ou ZIP en parties pour lui dans la mise en ligne.",
  "Several files and unfinished uploads: part size, continuing at start":
    "Plusieurs fichiers et mises en ligne inachevées : taille des parties, reprise au démarrage",
  "For ZIP in parts. Smaller parts start over faster when the connection drops; bigger ones make fewer files.":
    "Pour ZIP en parties. Des parties plus petites repartent plus vite quand la connexion coupe ; des plus grandes font moins de fichiers.",
  "Unfinished uploads": "Mises en ligne inachevées",
  "Continue unfinished uploads when Pako starts": "Reprendre les mises en ligne inachevées au démarrage de Pako",
  "A ZIP in parts upload that stopped (Pako closed, PC restarted) carries on with the parts not up yet. Off: it waits for Try again.":
    "Une mise en ligne en ZIP en parties qui s’est arrêtée (Pako fermé, PC redémarré) continue avec les parties pas encore en ligne. Désactivé : elle attend Réessayer.",
  "Pick a part size {range} (Upload › Advanced).": "Choisissez une taille de partie {range} (Envoi › Avancé).",

  // ---- First-start setup ----
  "Set up Pako": "Configurer Pako",
  "Skip for now": "Plus tard",
  "Pick what you'd like Pako to do. You can change all of this later in Settings.":
    "Choisissez ce que Pako fera pour vous. Vous pourrez tout changer plus tard dans les Paramètres.",
  "The sun or moon at the top of Pako switches it any time.": "Le soleil ou la lune en haut de Pako le change à tout moment.",
  "Downloads go to": "Les téléchargements vont dans",
  "Where should downloads go?": "Où enregistrer les téléchargements ?",
  // The folder is named in the language of the moment (Rust, direct/node.rs: « De Rado »).
  "Files people send you in Direct go in a folder inside it, like \"From Rado\".":
    "Les fichiers qu’on vous envoie dans Direct vont dans un dossier à l’intérieur, comme « De Rado ».",
  "Preview more video types before downloading": "Aperçu de plus de types de vidéos avant le téléchargement",
  "Watch MKV, AVI and other videos in the details panel to check it's the right one.":
    "Regardez les vidéos MKV, AVI et autres dans le panneau de détails pour vérifier que c’est la bonne.",
  // A feature (« fonction », feminine) that is installed.
  "Ready (feature)": "Prête",
  "Ready · uses 7-Zip": "Prête · utilise 7-Zip",
  "Uses 7-Zip · about 2 MB": "Utilise 7-Zip · environ 2 Mo",
  "Ready · uses ffmpeg": "Prête · utilise ffmpeg",
  "Uses ffmpeg · about 150 MB": "Utilise ffmpeg · environ 150 Mo",
  "Ready · uses yt-dlp, Deno and ffmpeg": "Prête · utilise yt-dlp, Deno et ffmpeg",
  "Uses yt-dlp, Deno and ffmpeg · about 600 MB": "Utilise yt-dlp, Deno et ffmpeg · environ 600 Mo",
  "Keep Pako and its tools up to date automatically": "Garder Pako et ses outils à jour automatiquement",
  Browser: "Navigateur",
  "Add Pako to my browsers (right-click › Download link with Pako)": "Ajouter Pako à mes navigateurs (clic droit › Télécharger le lien avec Pako)",
  "Links, files and videos go from your browser to Pako, plus a Download button on video players (YouTube, Vimeo, Twitch, Facebook, Instagram). Everything stays on this PC.":
    "Liens, fichiers et vidéos passent de votre navigateur à Pako, plus un bouton Télécharger sur les lecteurs vidéo (YouTube, Vimeo, Twitch, Facebook, Instagram). Tout reste sur ce PC.",
  "Add Pako's extension to your browser:": "Ajoutez l’extension de Pako à votre navigateur :",
  "Upload big files to your Google Drive": "Mettre de gros fichiers en ligne sur votre Google Drive",
  "Connected: {email}": "Connecté : {email}",
  "Optional. Pako only sees the files it uploads. Without it, Upload uses Gofile (no account needed).":
    "Facultatif. Pako ne voit que les fichiers qu’il met en ligne. Sans lui, Envoi utilise Gofile (aucun compte nécessaire).",
  "Minimize to the tray": "Réduire dans la zone de notification",
  "Minimize hides Pako next to the clock; it keeps downloading.": "Réduire cache Pako à côté de l’horloge ; il continue de télécharger.",
  "Close to the tray": "Fermer dans la zone de notification",
  "The close button hides Pako; downloads keep going. Quit from the tray icon.":
    "Le bouton Fermer cache Pako ; les téléchargements continuent. Quittez depuis l’icône de la zone de notification.",
  "Waiting downloads carry on by themselves after you sign in.": "Les téléchargements en attente continuent d’eux-mêmes après l’ouverture de session.",
  "Downloads about {n} MB": "Télécharge environ {n} Mo",
  "Set up": "Configurer",
  "Getting it ready in the background…": "Préparation en arrière-plan…",
  "Getting them ready in the background…": "Préparation en arrière-plan…",

  // ---- Terms of use (the window around TERMS.md) ----
  "Pako's terms of use": "Conditions d’utilisation de Pako",
  "Pako now has terms of use. Please read them: Pako carries on as soon as you agree.":
    "Pako a maintenant des conditions d’utilisation. Merci de les lire : Pako continue dès que vous les acceptez.",
  "Your downloads and lists stay as they are. Paused and waiting downloads simply wait until you agree, and nothing is lost.":
    "Vos téléchargements et vos listes restent tels quels. Les téléchargements en pause et en attente attendent simplement votre accord, et rien n’est perdu.",
  "Before you start, please read Pako's terms of use. Pako starts as soon as you agree.":
    "Avant de commencer, prenez un moment pour lire les conditions d’utilisation de Pako. Pako démarre dès que vous les acceptez.",
  "Also on {link}, always in their newest version.": "Aussi sur {link}, toujours dans leur dernière version.",
  "Quit Pako": "Quitter Pako",
  "Agree and continue": "Accepter et continuer",
  "Pako couldn't save that you agreed, so it asks again next time: {error}":
    "Pako n’a pas pu enregistrer votre accord, il vous le redemandera la prochaine fois : {error}",
} satisfies Record<string, Entry>;
