# Extension de navigateur

L’extension Pako envoie liens, vidéos et téléchargements de votre navigateur directement à Pako.

## Ajouter Pako à votre navigateur {#add-pako-to-your-browser}

L’extension de Pako fonctionne dans Chrome, Edge, Brave, Vivaldi, Opera (Opera GX aussi), Firefox, et les navigateurs basés sur Firefox : Zen, LibreWolf, Waterfox et Floorp.

### Chrome, Edge, Brave, Vivaldi, Opera {#chrome-edge-brave-vivaldi-opera}

1. Dans Pako, ouvrez **Paramètres › Extension de navigateur**, activez **Connecter Pako à vos navigateurs** et cliquez sur **Enregistrer**. (L’interrupteur **Connecter Pako à vos navigateurs** de Pako Setup, activé au départ, le fait pour vous, tout comme la configuration du premier démarrage quand **Ajouter Pako à mes navigateurs** reste coché.)
2. Sous **Ajouter Pako à votre navigateur**, cliquez sur **Ajouter à Chrome** (ou **Ajouter à Edge**, **Ajouter à Brave**, **Ajouter à Vivaldi**, **Ajouter à Opera**, **Ajouter à Opera GX**). Sa page des extensions s’ouvre, et le dossier de l’extension est copié pour vous (quand les étapes ne disent pas « le dossier est déjà copié », cliquez sur **Copier** à côté du dossier).
3. Sur la page des extensions, activez le **Mode développeur**.
4. Cliquez sur **Charger l’extension non empaquetée**.
5. Cliquez dans la barre d’adresse en haut de la fenêtre qui s’ouvre, appuyez sur **Ctrl+V** puis **Entrée**, et cliquez sur **Sélectionner un dossier**.

Si le navigateur n’ouvre pas sa page des extensions, Pako ouvre le dossier à la place et vous dit quelle adresse taper dans le navigateur. **Ouvrir le dossier de l’extension** (**Paramètres › Extension de navigateur › Avancé**) l’ouvre à tout moment.

### Firefox, Zen, LibreWolf, Waterfox, Floorp {#firefox-zen-librewolf-waterfox-floorp}

La version pour Firefox est fournie avec Pako, signée par Mozilla (Firefox n’ajoute que des extensions signées). Zen, LibreWolf, Waterfox et Floorp prennent la même : les étapes sont les mêmes, avec le nom de ce navigateur à la place de Firefox.

1. Dans Pako, ouvrez **Paramètres › Extension de navigateur**, activez **Connecter Pako à vos navigateurs** et cliquez sur **Enregistrer** (la configuration du premier démarrage le fait pour vous, comme plus haut).
2. Sous **Ajouter Pako à votre navigateur**, cliquez sur **Ajouter à Firefox** (ou **Ajouter à Zen**, **Ajouter à LibreWolf**, **Ajouter à Waterfox**, **Ajouter à Floorp**). Le navigateur s’ouvre et demande s’il faut ajouter Pako : cliquez sur **Ajouter**.
3. Pour que les téléchargements derrière une connexion marchent et que le bouton **Télécharger** s’affiche sur les vidéos, Pako doit être autorisé sur tous les sites : dans le navigateur, ouvrez `about:addons` › **Pako** › **Permissions** et activez **Accéder à vos données sur tous les sites web** si ce n’est pas fait.

Quand vous l’ajoutez, Firefox 140 et suivants (et les navigateurs basés sur ces versions) listent aussi ce que l’extension transmet à Pako sur votre PC : votre connexion au site d’un téléchargement, les adresses des téléchargements et de leurs pages, et les liens d’une page.

Si le navigateur ne s’ouvre pas, Pako ouvre le dossier avec le fichier `pako-firefox.xpi` sélectionné : faites-le glisser dans une fenêtre de ce navigateur. Une copie de Pako compilée par quelqu’un d’autre à partir du code source peut ne pas contenir la version pour Firefox : Pako le dit alors.

### Un autre navigateur {#another-browser}

Pako liste Chrome, Edge, Brave, Vivaldi, Opera, Opera GX, Firefox, Zen, LibreWolf, Waterfox et Floorp, même quand ils sont installés dans un autre dossier ou sur un autre lecteur (Pako demande à Windows les navigateurs de **Paramètres › Applications › Applications par défaut**). La plupart des autres navigateurs sont basés sur l’un d’eux, et l’extension de Pako y fonctionne en général aussi. D’abord, dans Pako : **Paramètres › Extension de navigateur**, activez **Connecter Pako à vos navigateurs**, cliquez sur **Enregistrer**, puis sur **Ouvrir le dossier de l’extension** (sous **Avancé**, en bas de l’onglet).

**Un navigateur basé sur Chromium** (par exemple Arc, Thorium, Yandex ou Ungoogled Chromium) :

1. Dans le navigateur, ouvrez sa page des extensions : tapez `chrome://extensions` dans la barre d’adresse (certains navigateurs ont la leur, comme `arc://extensions`).
2. Activez le **Mode développeur**, puis cliquez sur **Charger l’extension non empaquetée**.
3. Choisissez le dossier **chromium** à l’intérieur du dossier que Pako a ouvert, puis **Sélectionner un dossier**.

**Un autre navigateur basé sur Firefox** :

1. Faites glisser le fichier **pako-firefox.xpi** du dossier que Pako a ouvert (il est à côté du dossier **chromium**) dans une fenêtre du navigateur. Il est aussi sur la page de téléchargement de Pako : ouvrez [github.com/sookera/pako-releases/releases/latest/download/pako-firefox.xpi](https://github.com/sookera/pako-releases/releases/latest/download/pako-firefox.xpi) dans ce navigateur, et il propose de l’ajouter.
2. Cliquez sur **Ajouter** quand il le demande.
3. Dans sa page des modules (`about:addons`) › **Pako** › **Permissions**, activez **Accéder à vos données sur tous les sites web** si ce n’est pas fait.

Cliquez ensuite sur le bouton Pako de la barre d’outils du navigateur : il doit dire **Connecté** (voyez [Vérifier la connexion](#check-its-connected)). S’il dit **Pako n’est pas connecté à ce navigateur**, ce navigateur cherche Pako ailleurs : dites-nous de quel navigateur il s’agit (**Aide › Signaler un problème**), pour que Pako puisse l’ajouter. Les mises à jour de Pako arrivent aussi à l’extension dans ces navigateurs : un navigateur basé sur Chromium les prend tout seul, et un navigateur basé sur Firefox quand vous y faites glisser le nouveau **pako-firefox.xpi** (certains la mettent à jour eux-mêmes).

### Pako Light {#pako-light}

Pako Light est une édition plus petite de l’extension, faite pour le Chrome Web Store, avec une icône bleu-vert. Elle prend vos téléchargements et a les éléments du clic droit, mais elle ne télécharge pas de vidéos : pas de bouton **Télécharger** sur les vidéos, pas de **Vidéos de cette page**, et ses éléments du clic droit ne marchent que sur les liens et les images. Elle ne fait rien non plus sur les sites de vidéos : là, **Envoyer cette page à Pako** (dans sa petite fenêtre et dans le menu du clic droit) et **Envoyer tous les liens de cette page à Pako (pour choisir)** sont grisés, sa petite fenêtre dit « Pako Light ne fonctionne pas sur ce site. », et les liens vers des sites de vidéos sont laissés de côté dans « tous les liens ». Pako lui-même refuse aussi les vidéos envoyées depuis Pako Light. L’extension complète, avec le bouton **Télécharger** sur les vidéos, est fournie avec Pako (voyez plus haut).

Une seule extension prend vos téléchargements, pour qu’un téléchargement n’aille à Pako qu’une fois. Si l’extension complète est activée dans le même navigateur, Pako Light s’efface : pas d’éléments du clic droit à elle, et sa petite fenêtre dit **Non utilisée** et **L’extension Pako complète est activée dans ce navigateur**. Vous pouvez garder les deux, ou retirer Pako Light.

## Vérifier la connexion {#check-its-connected}

Cliquez sur l’icône Pako de la barre d’outils du navigateur. La petite fenêtre dit **Connecté**. Si elle dit **Non connecté**, voyez [Non connecté](#not-connected) plus bas.

La même fenêtre vous permet de :

- Voir les **Vidéos de cette page** et les télécharger (extension complète seulement, voyez plus bas).
- **Envoyer cette page à Pako**.
- Activer ou désactiver **Laisser Pako prendre les téléchargements**.
- Activer ou désactiver le **Bouton Télécharger sur les vidéos** (extension complète seulement).

La petite fenêtre suit le mode clair ou sombre de votre navigateur. L’extension parle la langue de Pako (voyez [Anglais ou français](downloading.md#english-or-french)) : après un changement, elle suit la prochaine fois qu’elle parle à Pako, ou tout de suite quand vous ouvrez sa petite fenêtre. Avec un Pako antérieur à 1.4.0, elle reste en anglais.

La fenêtre de Pako le montre aussi : dans **Paramètres › Extension de navigateur** (et dans la configuration du premier démarrage), un navigateur dont l’extension a parlé à Pako dans les 30 derniers jours affiche **Connecté** en vert, avec **Pako Light** quand c’est cette édition, et quand elle a servi pour la dernière fois (« utilisé aujourd’hui », « utilisé il y a 3 jours »). À la place d’un bouton, il affiche **Installé**. (Pour ajouter l’extension de nouveau après l’avoir retirée, utilisez **Ouvrir le dossier de l’extension**, sous **Avancé**, et chargez-la dans la page des extensions du navigateur.) Un navigateur qui n’a pas servi depuis plus longtemps affiche **Pas utilisé récemment**, avec son bouton **Ajouter à …**. Un navigateur sans ligne sous son nom n’a jamais été connecté. La liste se met à jour quand vous revenez dans la fenêtre de Pako, donc elle dit **Connecté** juste après l’ajout de l’extension.

## Vidéos de cette page {#videos-on-this-page}

Ne téléchargez et n’enregistrez que ce que vous avez le droit de faire, et suivez les règles des sites que vous utilisez.

Avec l’extension complète, Pako repère la vidéo qui se lit sur n’importe quelle page, même dans un lecteur intégré venant d’un autre site. Le bouton Pako de la barre d’outils montre combien il en a trouvé sur cette page. Cliquez dessus : la petite fenêtre les liste sous **Vidéos de cette page**, chacune avec son type (**Flux HLS**, **Flux DASH** ou **Fichier vidéo**), son serveur, et la meilleure qualité quand la page l’indique (**jusqu’à 1080p**). Cliquez sur **Télécharger avec Pako** à côté de l’une d’elles : Pako nomme le fichier d’après le titre de la page et le télécharge comme les autres vidéos (voyez [Vidéos](videos.md)).

- La liste ne vaut que pour cet onglet, et se vide quand l’onglet va sur une autre page ou se ferme.
- Si la vidéo est protégée, la petite fenêtre dit « Cette vidéo est protégée, elle ne peut donc pas être téléchargée. » et ne propose rien.
- Une fois que Pako a trouvé une vidéo sur une page, le bouton **Télécharger** apparaît aussi en haut à droite de la vidéo quand vous pointez dessus (voyez [plus bas](#the-download-button-on-videos)).
- Pendant qu’une vidéo se lit ou est listée, la petite fenêtre propose aussi **Enregistrer cette vidéo**, pour quand Pako ne peut pas l’obtenir (le téléchargement dit « Pako n’a pas pu récupérer cette vidéo. ») (voyez [Enregistrer cette vidéo](videos.md#record-this-video)).

## Le bouton Télécharger sur les vidéos {#the-download-button-on-videos}

Avec l’extension complète, sur YouTube, Vimeo, Twitch, Facebook et Instagram, un bouton **Télécharger** apparaît en haut à droite d’une vidéo quand vous pointez dessus. Cliquez dessus et choisissez **Jusqu’à 1080p**, **Jusqu’à 720p**, **Jusqu’à 480p** ou **Audio seul**. Voyez [Vidéos](videos.md).

Sur toute autre page, le même bouton apparaît sur la vidéo une fois que Pako l’y a trouvée (**Vidéos de cette page**, plus haut), dans le cadre du lecteur qui la lit. Son menu propose **Télécharger cette vidéo** (ou, quand Pako en a trouvé plusieurs, la même liste que la petite fenêtre), et **Enregistrer cette vidéo** pendant qu’une vidéo se lit ou est listée. Une vidéo protégée n’a rien. Les pages où Pako n’a trouvé aucune vidéo ne sont jamais touchées. **Bouton Télécharger sur les vidéos**, dans la petite fenêtre, le désactive sur toutes les pages.

## Menu du clic droit {#right-click-menu}

- Sur un lien : **Télécharger le lien avec Pako**. Sur une image, une vidéo ou un son : **Télécharger l’image avec Pako** (ou **la vidéo**, ou **l’audio**). Sur une image qui est aussi un lien, les deux s’affichent. Cela va directement dans **Téléchargements** (en passant par la fenêtre **Télécharger avec Pako**, si elle est activée). Un lien vers une page de vidéo télécharge la vidéo, dans votre qualité habituelle. Un lecteur qui diffuse par morceaux n’a pas d’adresse à lui : sur un site de vidéos, la vidéo de sa page est téléchargée ; ailleurs, utilisez **Vidéos de cette page** (plus bas) ou **Enregistrer cette vidéo**. Pako Light n’a ces éléments que sur les liens et les images.
- Sur une page : **Envoyer cette page à Pako**. Dans l’extension complète, sur les sites de vidéos, cet élément devient **Télécharger la vidéo avec Pako** : la vidéo se télécharge dans votre qualité habituelle, et une playlist va dans **Liens** (voyez [Vidéos](videos.md)).
- Sur une page : **Envoyer tous les liens de cette page à Pako (pour choisir)**. Ils arrivent dans **Liens**, où vous cochez ce que vous voulez.

Les éléments suivent l’onglet au premier plan, même quand vous passez à une autre fenêtre du navigateur.

## Les téléchargements du navigateur {#downloads-from-the-browser}

Avec **Laisser Pako prendre les téléchargements** activé, les fichiers que vous téléchargez dans le navigateur vont à Pako à la place, par exemple quand vous cliquez sur un bouton Télécharger sur Google Drive ou n’importe quel site.

Ce qui se passe quand vous cliquez sur un téléchargement :

1. Le navigateur ne le télécharge pas : l’extension le prend avant que le navigateur enregistre quoi que ce soit (dans Chrome, Edge et les navigateurs du même genre, même avant une fenêtre **Enregistrer sous**, et dans Vivaldi avant sa propre question, voyez plus bas), et la fenêtre **Télécharger avec Pako** s’ouvre tout de suite avec le nom et la taille du fichier.
2. Quand vous cliquez sur **Télécharger**, la première demande de Pako au serveur est le vrai téléchargement. Un téléchargement ne peut pas passer de la connexion du navigateur à celle de Pako (aucun gestionnaire de téléchargement ne le peut), donc le serveur est sollicité une fois de plus, par Pako. Quelques liens à usage unique refusent cette deuxième demande.
3. Si le serveur donne le même fichier à Pako, rien d’autre ne se passe. Sinon (pas de fichier, une page web à la place, une autre taille), votre navigateur le télécharge quand même et Pako le suit : voyez **Via navigateur** plus bas.

- Les petits fichiers (jusqu’à 2 Mo) restent dans le navigateur.
- Un fichier envoyé par le formulaire d’une page (avec « POST ») ne peut pas être redemandé : le navigateur le garde en pause pendant que Pako vérifie, et continue avec lui si Pako ne peut pas l’obtenir. La fenêtre **Télécharger avec Pako** s’ouvre une fois que Pako l’a vérifié.
- Un fichier d’un site qui vous a demandé de vous connecter avec sa propre boîte de connexion (pas le formulaire d’une page) ne peut pas être demandé par Pako : le navigateur le garde en pause pendant que Pako vérifie, comme plus haut, et continue avec lui si Pako ne peut pas l’obtenir.
- Si votre navigateur n’a pas pu relancer un téléchargement après que Pako n’a pas pu l’obtenir, le bouton Pako affiche un **!** rouge et dit « Pako n’a pas pu récupérer « … », et votre navigateur n’a pas pu le relancer. Téléchargez-le encore une fois depuis la page. » (La ligne de Pako le dit aussi.)
- Avec Pako Light, qui ne peut pas voir comment une page a demandé un fichier, le navigateur garde toujours le téléchargement en pause jusqu’à ce que Pako l’ait (la fenêtre s’ouvre une fois que Pako l’a vérifié, comme plus haut).
- Avec un Pako antérieur à 1.2.0, l’extension fonctionne comme avant : le navigateur garde le téléchargement en pause jusqu’à ce que Pako l’ait vérifié.
- Les fichiers que le navigateur signale comme dangereux restent dans le navigateur.
- Une page de vidéo elle-même (une adresse YouTube ou Vimeo, par exemple) n’est jamais prise : pour les vidéos, utilisez le bouton **Télécharger** ou le menu du clic droit. Pako Light laisse au navigateur tous les téléchargements d’un site de vidéos.
- Dans Firefox, un type de fichier réglé sur **Toujours demander** montre d’abord la question de Firefox ; réglez-le sur **Enregistrer le fichier** dans les paramètres de Firefox (**Fichiers et applications**) pour un passage direct à Pako.
- Vivaldi demande « Télécharger … ? » (Enregistrer, Enregistrer sous, Ouvrir, Annuler) avant qu’une extension puisse voir un téléchargement. Là, l’extension de Pako arrête donc le fichier dès que le serveur répond : Vivaldi ne demande rien et son panneau de téléchargements ne s’ouvre pas. L’onglet affiche une page blanche un instant et revient tout de suite à votre page (un onglet ouvert seulement pour le fichier se ferme), et la fenêtre **Télécharger avec Pako** s’ouvre. Un autre navigateur basé sur Chromium qui demande d’abord a droit à la même chose, une fois que l’extension l’a vu demander deux fois de suite.
- L’onglet reste sur la page de Pako (« Ce téléchargement a été envoyé à Pako. Revenez à la page précédente pour retrouver votre page. ») quand revenir en arrière renverrait un formulaire (la page où vous étiez était la réponse d’un formulaire), et quand une page redemande d’elle-même le même fichier (une page « votre téléchargement démarre dans 5 secondes », même par un nouveau lien à chaque fois) : Pako le reçoit une fois, et la page ne le relance pas encore et encore. Le même fichier, c’est le même nom et la même taille en moins d’une demi-minute, dans le même onglet.
- Certaines pages démarrent un téléchargement avec un lien fait pour lui, sans rien charger (souvent un lien **Télécharger** vers un fichier du même site). Le serveur n’est pas sollicité avant la question de Vivaldi, donc l’extension attrape elle-même le clic : avec **Attraper les liens de téléchargement des pages** activé (dans la petite fenêtre du bouton Pako, sous **Laisser Pako prendre les téléchargements** ; activé par défaut, affiché seulement dans un navigateur qui demande d’abord), un tel clic va directement à Pako, quelle que soit la taille du fichier, et la page reste comme elle est. Si Pako ne peut pas le prendre, votre navigateur le télécharge quand même ; si cela ne marche pas non plus, cliquez de nouveau sur le lien dans la minute : ce clic est laissé à votre navigateur, comme la page le lance. Une page ouverte avant l’activation de l’interrupteur (ou avant que Pako réponde) y a droit une fois rechargée. Une page qui lance beaucoup de téléchargements à la fois (plus de quatre en dix secondes, ou plus d’un depuis une partie cachée de la page) ne voit que les premiers pris : le bouton Pako affiche un **!** rouge qui le dit, et pendant une minute votre navigateur pose ses questions habituelles pour les téléchargements de cette page. Un fichier qu’une page fabrique elle-même (un export ou une image créée dans la page, pas un lien vers le site) ne peut pas aller à Pako : Vivaldi le demande, sauf si vous activez **Enregistrer les fichiers à l’emplacement par défaut sans demander** dans **Paramètres › Téléchargements** de Vivaldi.
- La question de Vivaldi vient toujours en premier pour un fichier envoyé par un formulaire, une fenêtre privée, un site de vidéos, un petit fichier, quand l’extension ne peut pas joindre Pako, et dans un navigateur antérieur à Chrome 128 (2024) : cliquez sur **Enregistrer** et Pako prend le relais, ou activez **Enregistrer les fichiers à l’emplacement par défaut sans demander** dans **Paramètres › Téléchargements** de Vivaldi. Un fichier venant de la boîte de connexion propre à un site retourne à Vivaldi (qui pose alors sa question) : Pako ne peut pas s’y connecter. L’onglet d’une fenêtre privée ouvert juste avant le téléchargement peut lui aussi afficher la page de Pako : le téléchargement continue alors dans votre navigateur, dans cet onglet (s’il ne le peut pas, la page le dit : revenez en arrière et téléchargez-le encore une fois).
- Un fichier que Pako prend de cette façon ne passe jamais par les vérifications de téléchargement de Vivaldi ; Windows le vérifie quand même quand vous l’ouvrez.

### Via navigateur {#via-browser}

Certains fichiers, seul le navigateur peut les obtenir : un site qui refuse les gestionnaires de téléchargement, un proxy ou un VPN qui ne marche que dans le navigateur, un lien qui ne marche qu’une fois. Votre navigateur télécharge alors le fichier, et Pako le suit comme l’un des siens :

- La ligne dans **Téléchargements** affiche **Via navigateur**, avec sa progression, sa vitesse et le temps restant.
- Le téléchargement du navigateur tient en une seule connexion : le téléchargement en plusieurs parties, plus rapide, de Pako, **Télécharger d’abord** et la priorité ne s’y appliquent pas (ils sont grisés).
- **Pause**, **Reprendre** et **Retirer** dans Pako mettent en pause, reprennent ou annulent le téléchargement du navigateur (quand le navigateur le peut). Un téléchargement que le navigateur a arrêté peut reprendre avec **Réessayer** quand le navigateur sait le continuer.
- Quand le navigateur a fini, Pako déplace le fichier dans le dossier que vous avez choisi (ou dans le dossier habituel de Pako), jamais par-dessus un autre fichier, et une fois qu’il y est, la liste des téléchargements du navigateur l’oublie (le fichier reste). Si c’est le même dossier, le fichier reste où il est, sauf si un autre téléchargement de votre liste porte ce nom (un plus ancien dont le fichier a été supprimé depuis, ou un qui n’est pas encore terminé) : il prend alors un nom libre, comme « file (1).zip ». Si Pako ne peut pas le déplacer (le fichier est ouvert, l’autre lecteur est plein), il le dit et le laisse là où le navigateur l’a enregistré, toujours dans la liste des téléchargements du navigateur.
- Si Pako est fermé quand le navigateur termine, Pako liste le fichier la prochaine fois que l’extension le joint (l’extension ne lance pas Pako pour cela).

### Fenêtres privées {#private-windows}

Pako ne fonctionne dans les fenêtres privées que si vous l’y autorisez, sur la page de l’extension dans votre navigateur : **Autoriser en mode navigation privée** (Chrome), **Autoriser en mode InPrivate** (Edge), ou **Exécution dans des fenêtres privées** (Firefox : menu › **Modules complémentaires et thèmes** › Pako). La petite fenêtre de l’extension dit si c’est activé, avec un bouton vers cette page (Firefox ne laisse pas les extensions l’ouvrir, donc la fenêtre dit où aller).

Quand c’est activé, tout ce que vous envoyez depuis une fenêtre privée est privé : les téléchargements que Pako prend, les éléments du clic droit, **Envoyer tous les liens de cette page**, le bouton **Envoyer cette page à Pako** de la fenêtre de l’extension, et le bouton **Télécharger** et sa pastille sur les vidéos. Ils vont à Pako comme les autres, jamais avec votre connexion habituelle (un téléchargement que Pako prend garde la connexion propre à cette fenêtre, tout comme les éléments du clic droit et **Envoyer cette page à Pako** dans Firefox ; les liens et les vidéos partent sans connexion), et Pako n’en garde aucune trace à part le fichier : aucune ligne dans la cloche, rien dans le Tableau de bord ni dans la liste des problèmes, jamais enregistré dans la liste de Pako, et la connexion est oubliée dès que le téléchargement se termine. Un téléchargement terminé affiche **Terminé · privé** et quitte la liste dès que vous l’ouvrez depuis Pako ou cliquez sur son **✕** (le fichier reste), ou sinon 10 minutes après sa fin. Pour le garder plus longtemps, choisissez **1 heure après** ou **Quand vous quittez Pako** dans **Paramètres › Téléchargements › Retirer les téléchargements privés de la liste**. Tant que le [mode privé](private-mode.md) est activé, ce réglage est grisé : les téléchargements des fenêtres privées vont plutôt dans **Privé**, chiffrés sur votre PC, derrière votre code PIN ou votre mot de passe, et gardés jusqu’à ce que vous les supprimiez. Si Pako se ferme avant qu’un téléchargement soit terminé, son fichier inachevé est supprimé au prochain démarrage de Pako. Dans Firefox, le navigateur garde sa propre copie en pause du téléchargement d’une fenêtre privée jusqu’à ce que Pako ait le fichier, donc rien n’est perdu si Pako quitte entre-temps.

Dans Chrome, Edge et les navigateurs du même genre, le téléchargement d’une fenêtre privée est gardé en pause pendant que Pako le vérifie (comme un fichier envoyé par un formulaire, plus haut) : si Pako ne peut pas l’obtenir, il continue dans la fenêtre privée, jamais dans votre fenêtre habituelle. Un fichier Google Drive venant d’une fenêtre privée est téléchargé comme n’importe quel autre fichier, avec la connexion propre à cette fenêtre.

Les liens envoyés depuis une fenêtre privée affichent **privé** avec un cadenas dans **Liens**. Ils ne sont gardés que jusqu’à ce que vous quittiez Pako (jamais enregistrés), ne sont jamais envoyés à un autre de vos PC avec **Télécharger sur** (« Les téléchargements privés restent sur ce PC. »), et se téléchargent comme téléchargements privés. Les téléchargements privés ne s’affichent jamais non plus dans **Mes appareils** sur vos autres PC, donc ils ne peuvent pas y être vus, mis en pause, retirés ou récupérés. **Enregistrer cette vidéo** n’est pas proposé dans les fenêtres privées.

Un téléchargement de votre navigateur qui démarre tout de suite affiche une courte pop-up, **Téléchargement dans Pako**, et « tous les liens de cette page » affiche **Liens ajoutés** (une vidéo envoyée dans **Liens** affiche **Vidéo envoyée à Pako**, une playlist **Playlist envoyée à Pako**). Désactivez-les avec **Envoyé par votre navigateur** dans **Paramètres › Notifications** (voyez [Notifications](notifications.md)).

## La fenêtre Télécharger avec Pako {#the-download-with-pako-window}

Quand un téléchargement vient de votre navigateur, Pako montre d’abord une petite fenêtre **Télécharger avec Pako** :

- **Nom** : changez le nom du fichier si vous voulez.
- **Enregistrer dans** : le dossier. Cliquez sur **Changer** pour en choisir un autre.
- **Qualité** : pour les vidéos seulement.
- La cloche avec **Désactivé**, **Pop-up** et **Important** : comment ce téléchargement vous prévient quand il est terminé (voyez [Notifications](notifications.md#when-a-download-finishes-off-pop-up-important)). Elle part de **Nouveaux téléchargements**, dans **Paramètres › Notifications**.
- **Ne plus demander** : les téléchargements du navigateur démarrent alors tout de suite. Vous pouvez le réactiver dans **Paramètres › Extension de navigateur › Demander avant chaque téléchargement du navigateur**.

Cliquez sur **Télécharger** (ou appuyez sur **Entrée**) pour démarrer, ou sur **Annuler** (ou **Échap**). Quand d’autres téléchargements tournent déjà, vous avez **Lancer maintenant** et **Ajouter à la file** à la place. Si plusieurs téléchargements arrivent ensemble, la fenêtre les montre l’un après l’autre (« 1 sur 3 »).

Avec [Mes appareils](devices.md), une petite flèche à côté de **Télécharger** permet à un autre de vos PC de le télécharger à la place : **Télécharger sur PC maison**, par exemple. Elle n’est là que pendant qu’un de vos appareils qui laisse ce PC le piloter est en ligne. Ce PC reçoit le lien et le télécharge dans son propre dossier de téléchargement, en **D’abord** (comme **Lancer maintenant**), et une vidéo dans la qualité choisie dans cette fenêtre, tout de suite. Le téléchargement quitte ce PC ; le nom et le dossier choisis ici ne sont pas envoyés. Si cela n’a pas marché, la fenêtre dit pourquoi et le téléchargement reste ici. Seul le lien est envoyé, jamais la connexion de votre navigateur, donc un téléchargement qui demande d’être connecté sur ce PC peut ne pas marcher là-bas. **Télécharger** lui-même télécharge toujours sur ce PC.

Avec le [mode privé](private-mode.md) activé, la flèche à côté de **Télécharger** propose aussi **Télécharger en privé** : le fichier va dans le mode privé et ne laisse aucune trace dans votre liste habituelle : les quelques secondes que Pako avait déjà récupérées pour le vérifier sont écrasées et supprimées, et il recommence, chiffré. Le nom que vous avez tapé est gardé ; le dossier n’est pas utilisé (il est dans le mode privé).

Avec **Afficher la petite fenêtre quand un téléchargement du navigateur démarre** activé (**Paramètres › Extension de navigateur**), la petite fenêtre de la zone de notification s’ouvre en bas à droite pour que vous suiviez le téléchargement.

## Après une mise à jour de Pako {#after-a-pako-update}

Pako actualise de lui-même le dossier de son extension à son démarrage, et l’extension prend d’elle-même la nouvelle version la prochaine fois qu’elle parle à Pako (quelques secondes après, une fois que rien n’est en route vers Pako et que rien n’est en cours d’enregistrement). Un navigateur peut aussi continuer à faire tourner l’extension qu’il avait, même après un redémarrage (vu dans Vivaldi) : ouvrir le bouton Pako de la barre d’outils lui fait alors prendre la nouvelle (la petite fenêtre dit « Mise à jour de l’extension Pako… » : rouvrez-la un instant plus tard). Vous pouvez aussi cliquer sur le bouton d’actualisation de Pako dans la page des extensions du navigateur.

Firefox, Zen, LibreWolf, Waterfox et Floorp mettent à jour l’extension de Pako d’eux-mêmes, comme leurs autres modules (pour vérifier maintenant : `about:addons`, le bouton en forme d’engrenage, **Rechercher des mises à jour**). Si vous avez désactivé les mises à jour des modules dans l’un d’eux, ajoutez-la de nouveau avec **Ajouter à …** après la mise à jour de Pako.

Si le bouton Télécharger dit alors « L’extension Pako vient d’être mise à jour. Rechargez cette page et réessayez. », rechargez la page web. S’il dit « L’extension Pako ne répond pas. Ouvrez le bouton Pako de la barre d’outils pour plus de détails. », cliquez sur le bouton Pako : son message dit ce que le navigateur a signalé (aussi affiché quand vous pointez sur le message du bouton).

## Non connecté {#not-connected}

Si l’extension dit **Non connecté**, ou si le bouton Télécharger dit « Pako n’est pas connecté » :

1. Pako démarre tout seul, dans la zone de notification, quand l’extension lui envoie quelque chose. S’il ne le fait pas, lancez-le vous-même depuis son dossier ou le menu Démarrer.
2. Dans Pako, vérifiez que **Paramètres › Extension de navigateur › Connecter Pako à vos navigateurs** est activé.
3. Cliquez sur le bouton d’actualisation de Pako dans la page des extensions du navigateur, puis rechargez la page web.
4. Cliquez sur **Ouvrir Pako** dans la petite fenêtre de l’extension : cela ouvre Pako et le connecte. Autorisez-le si le navigateur le demande.

La petite fenêtre de Pako Light dit aussi où obtenir Pako, au cas où il ne serait pas encore sur ce PC : **Obtenez-le sur pako.sookera.com**.

Si la petite fenêtre dit **Ce Pako est trop ancien pour cette extension**, mettez Pako à jour (**Paramètres › Fonctions et mises à jour**), puis rouvrez la petite fenêtre.

Un **!** rouge sur le bouton Pako de la barre d’outils signifie que quelque chose envoyé à Pako n’est pas passé : cliquez sur le bouton pour voir ce qui s’est passé.

Si ça échoue encore, le message du bouton Télécharger se termine par « (Le navigateur dit : …) ». Ce texte dit exactement ce qui ne va pas ; gardez-le quand vous demandez de l’aide.

## Bon à savoir {#good-to-know}

- L’extension parle à Pako directement sur votre PC. Rien ne passe par Internet.
- Pour **Connecté** dans les Paramètres, Pako ne garde que le nom du navigateur, l’édition (complète ou Pako Light), la version de l’extension quand elle la donne, et la date, sur ce PC (`browsers_seen.json` dans le dossier de données de Pako).
- Dans les fenêtres privées, l’extension ne fait rien sauf si vous l’y avez autorisée dans votre navigateur (voyez **Fenêtres privées** plus haut) ; Pako ne garde alors aucune trace de ce qui en est envoyé, à part le fichier. Sans cela, rien d’une fenêtre privée n’arrive à Pako : le navigateur n’y fait pas tourner l’extension.
- Pour **Vidéos de cette page**, l’extension complète ne regarde que les demandes de vidéo que fait votre navigateur (elle ne les change ni ne les bloque jamais). La liste reste dans la mémoire du navigateur, pour cet onglet seulement, et rien ne va à Pako tant que vous n’avez pas cliqué. Avec une vidéo, Pako reçoit son adresse, la page où elle se lisait, le titre de la page, et la connexion de votre navigateur au serveur de la vidéo seulement.
- La connexion de votre navigateur à un site n’est transmise à Pako que pour les téléchargements dont il s’occupe (ceux que vous envoyez, et les téléchargements du navigateur qu’il prend), gardée chiffrée, et supprimée quand le téléchargement est terminé ou retiré.
