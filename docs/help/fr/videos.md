# Vidéos

Téléchargez des vidéos depuis YouTube et beaucoup d’autres sites, dans la qualité que vous choisissez, ou seulement le son.

## Avant la première vidéo {#before-the-first-video}

Les vidéos ont besoin de trois outils gratuits (yt-dlp, Deno et ffmpeg, environ 600 Mo sur le disque). S’ils manquent, un lien de vidéo dans **Liens** affiche un petit bouton de téléchargement dans sa colonne **Lien** (**Outils vidéo nécessaires**) : cliquez dessus. Vous pouvez aussi les installer dans **Paramètres › Fonctions et mises à jour › Télécharger des vidéos depuis YouTube et d’autres sites**. Avec **Mettre à jour Pako et ses outils automatiquement** activé (au même endroit), Pako les tient à jour.

Pako installe l’édition de yt-dlp qui démarre le plus vite (un dossier avec yt-dlp.exe dedans, pas le fichier unique). Un yt-dlp installé par un ancien Pako continue de fonctionner et change à sa prochaine mise à jour : tout seul avec les mises à jour automatiques activées, sinon avec **Mettre à jour** sous **Ce que Pako sait faire**.

## Télécharger une vidéo {#download-a-video}

1. Copiez l’adresse de la vidéo et appuyez sur **Ctrl+V** dans Pako (ou utilisez le bouton **Télécharger** du navigateur, voir plus bas).
2. La vidéo apparaît dans **Liens** avec sa taille pour la qualité choisie.
3. Sélectionnez-la pour changer la **Qualité** à droite si vous voulez, puis cliquez sur **Télécharger**.

Un lien de playlist liste toutes ses vidéos, pour que vous cochiez celles que vous voulez.

## Vérifier que c’est la bonne vidéo {#check-its-the-right-video}

Avant le téléchargement (ou pendant), vous pouvez lire quelques secondes de la vidéo pour vous assurer que c’est celle que vous voulez :

1. Sélectionnez la vidéo dans **Liens** (ou le téléchargement non terminé dans **Téléchargements**).
2. Sous **Aperçu**, dans les détails à droite, cliquez sur le bouton de lecture.
3. Après un instant, environ 20 secondes se lisent en 480p ou moins, quelques secondes après le début de la vidéo.

Rien n’est récupéré sur le site tant que vous ne cliquez pas sur lecture. L’extrait s’arrête quand vous choisissez un autre élément ou masquez les détails. Il a besoin des outils vidéo. Les directs ne peuvent pas avoir d’aperçu, et une vidéo privée, soumise à une limite d’âge ou protégée (DRM) indique **Pas d’aperçu possible pour cette vidéo** et pourquoi.

## Qualité et audio {#quality-and-audio}

- La liste **Qualité** propose les tailles que la vidéo a vraiment (1080p, 720p…), plus **Audio seul (M4A)** et **Audio seul (MP3)**.
- **Par défaut** utilise **Paramètres › Téléchargements › Qualité vidéo**.
- Si une vidéo n’a pas la qualité choisie, Pako prend la plus proche en dessous.
- Cliquez sur l’en-tête d’une playlist dans **Liens** pour régler la même qualité pour toutes ses vidéos.

## Le bouton Télécharger sur les lecteurs vidéo {#the-download-button-on-video-players}

Ne téléchargez et n’enregistrez que ce que vous avez le droit de télécharger, et respectez les règles des sites que vous utilisez.

Avec l’[extension de navigateur](browser-extension.md) complète (pas Pako Light), un bouton **Télécharger** apparaît en haut à droite d’une vidéo quand vous pointez dessus, sur YouTube, Vimeo, Twitch, Facebook et Instagram :

1. Cliquez sur **Télécharger**. Pako commence tout de suite à lire la vidéo, pendant que vous choisissez.
2. Choisissez **Jusqu’à 1080p**, **Jusqu’à 720p**, **Jusqu’à 480p** ou **Audio seul**.
3. Le bouton indique **Envoyé à Pako ✓**, et la vidéo commence à se télécharger dans Pako. Si **Demander avant chaque téléchargement du navigateur** est activé, Pako affiche d’abord la fenêtre **Télécharger avec Pako**.

Pako commence à lire le titre et les qualités de la vidéo dès que vous cliquez sur **Télécharger** (jamais quand vous pointez seulement sur la vidéo), si bien qu’une fois la qualité choisie, la vidéo démarre souvent aussitôt. Si vous fermez le menu sans choisir, Pako arrête et oublie cette lecture dès que vous le fermez (ou au bout de deux minutes au plus), et rien n’apparaît dans Pako. Le bouton répond en une seconde environ. Si Pako voit tout de suite qu’il ne peut pas récupérer la vidéo (les outils vidéo ne sont pas installés, ou elle est déjà dans Pako), le bouton dit pourquoi. Sinon, un pop-up le dit dès que Pako le sait, et la cloche le garde : **Pako n’a pas pu récupérer cette vidéo** (voir [Notifications](notifications.md)). Cliquer à nouveau sur le bouton pendant que Pako lit encore la vidéo indique **Pako récupère déjà cette vidéo.**

Sur Facebook et Instagram, le bouton prend la vidéo sur laquelle vous pointez, même dans un fil avec beaucoup de vidéos.

Vous pouvez aussi faire un clic droit sur une page de vidéo › **Télécharger la vidéo avec Pako**, un clic droit sur un lien vers une vidéo › **Télécharger le lien avec Pako**, ou cliquer sur **Envoyer cette page à Pako** dans la petite fenêtre de l’extension. La vidéo se télécharge alors dans votre qualité habituelle (**Paramètres › Téléchargements › Qualité vidéo**), en passant par la fenêtre **Télécharger avec Pako** si elle est activée. Une playlist va plutôt dans **Liens**, pour que vous cochiez les vidéos que vous voulez (le pop-up indique **Playlist envoyée à Pako**).

## Vidéos privées et avec connexion {#private-and-logged-in-videos}

Les vidéos envoyées depuis votre navigateur (le bouton ou le menu du clic droit) utilisent la connexion de votre navigateur pour ce site : les vidéos que vous pouvez regarder une fois connecté peuvent donc être téléchargées aussi. La connexion est gardée chiffrée, utilisée seulement pour ce téléchargement, et supprimée ensuite. Pendant le téléchargement de la vidéo, la connexion est dans un petit fichier que vous seul pouvez lire, dans le dossier temporaire de Windows (`%TEMP%\Pako`) ; ce fichier est supprimé dès que le téléchargement s’arrête, et quand Pako se ferme. Les vidéos des fenêtres privées ne la reçoivent jamais.

## La vidéo en lecture sur n’importe quelle page {#the-video-playing-on-any-page}

Avec l’[extension de navigateur](browser-extension.md#videos-on-this-page) complète, vous pouvez télécharger la vidéo en lecture sur presque n’importe quelle page, pas seulement sur les sites ci-dessus :

1. Lisez la vidéo un instant.
2. Cliquez sur le bouton Pako dans la barre d’outils du navigateur.
3. Sous **Vidéos de cette page**, cliquez sur **Télécharger avec Pako** à côté de la vidéo.

Ou pointez sur la vidéo elle-même : une fois que Pako l’a trouvée, le même bouton **Télécharger** apparaît en haut à droite, comme sur les sites ci-dessus. Cliquez dessus, puis sur **Télécharger cette vidéo** (quand Pako a trouvé plusieurs vidéos sur la page, il les liste comme le fait la petite fenêtre : choisissez-en une). Son menu propose aussi **Enregistrer cette vidéo** (plus bas), et il ne propose rien pour une vidéo protégée. Le bouton n’apparaît que sur les pages où Pako a trouvé une vidéo ; **Bouton Télécharger sur les vidéos**, dans la petite fenêtre de l’extension, le désactive aussi.

La vidéo va dans **Téléchargements**, nommée d’après le titre de la page, dans votre qualité habituelle (**Paramètres › Téléchargements › Qualité vidéo**), en passant par la fenêtre **Télécharger avec Pako** si elle est activée. Pako la récupère de la même façon que la page : depuis la même page, avec la connexion de votre navigateur pour le serveur de la vidéo. Quand son outil vidéo ne peut pas récupérer une vidéo faite de petits morceaux (un flux HLS), Pako réessaie tout seul, en récupérant les morceaux lui-même ; il reprend à partir des morceaux qu’il a déjà après une pause ou un redémarrage.

Si ça ne marche pas, le téléchargement indique « Pako n’a pas pu récupérer cette vidéo. » Essayez alors **Enregistrer cette vidéo**.

## Enregistrer cette vidéo {#record-this-video}

Le dernier recours, proposé dans la petite fenêtre de l’extension (extension complète), et dans le menu du bouton **Télécharger** sur la vidéo, pendant qu’une vidéo se lit ou est listée sous **Vidéos de cette page**, pour quand Pako n’a pas pu la récupérer :

1. Cliquez sur **Enregistrer cette vidéo**. La page se recharge.
2. Lancez la lecture si la vidéo ne démarre pas toute seule. Pako l’enregistre pendant qu’elle se lit, en accéléré et sans le son quand la page le permet. Si la vidéo ne démarre pas dans les 5 minutes, l’enregistrement s’arrête : « La vidéo n’a pas démarré, donc rien n’a été enregistré. »
3. Gardez l’onglet ouvert : **la vidéo doit se lire jusqu’au bout**. La petite fenêtre montre combien est enregistré ; **Arrêter l’enregistrement** l’arrête et rien n’est gardé.
4. À la fin, la petite fenêtre indique « Envoyé à Pako : il crée la vidéo dans Téléchargements. » La vidéo apparaît dans **Téléchargements**, nommée d’après le titre de la page. Si la vidéo a changé de qualité pendant la lecture, Pako la refait dans la meilleure qualité qu’elle a eue, ce qui prend un peu plus de temps.

Une seule vidéo est enregistrée à la fois, jamais dans les fenêtres privées, et jamais une vidéo protégée.

## Téléchargements rapides {#fast-downloads}

Quand un site envoie une vidéo en un seul gros fichier (YouTube, Vimeo…), Pako la télécharge en plusieurs parties en parallèle, puis réunit l’image et le son en un seul fichier. Quand un site envoie une vidéo en beaucoup de petits morceaux (Twitch et la plupart des sites de télévision), Pako récupère beaucoup de morceaux à la fois.

## Bon à savoir {#good-to-know}

- Les sites qui verrouillent leurs vidéos (Netflix, Disney+, Prime Video et autres du même genre) ne peuvent pas être téléchargés. Pako indique « Ce site protège ses vidéos (DRM) : elles ne peuvent donc pas être téléchargées. » et n’essaie jamais de contourner cela. Pour la vidéo en lecture sur une page, il indique « Cette vidéo est protégée, elle ne peut donc pas être téléchargée. »
- Les sites que Pako reconnaît comme sites de vidéos incluent YouTube, Vimeo, Dailymotion, Twitch, TikTok, X, Instagram, Facebook, Reddit, Bilibili, Streamable, Rumble, Odysee, BitChute et SoundCloud. Beaucoup d’autres fonctionnent aussi.
- Le panneau de détails d’un téléchargement de vidéo indique sa qualité sous **Qualité**.
