# Télécharger

Ajouter des liens, suivre vos téléchargements, et récupérer des fichiers depuis Google Drive, même gros ou très demandés.

## Ajouter des liens {#add-links}

Vous pouvez ajouter des liens de plusieurs façons :

- Appuyez sur **Ctrl+V** n’importe où dans Pako. Tout ce que vous avez copié est parcouru à la recherche de liens.
- Collez-les dans la zone de recherche en haut. Cette zone cherche aussi dans la page où vous êtes (téléchargements, liens, mises en ligne, contacts ou téléchargements privés) ; sur **Mes appareils** ou le **Tableau de bord**, y taper vous amène à Téléchargements. Changer de page la vide.
- Faites glisser des liens, ou un fichier texte plein de liens, sur la fenêtre.
- Cliquez sur **Ajouter des liens**, collez les liens, et cliquez sur **Ajouter le lien** (ou **Ajouter 3 liens**, et ainsi de suite).
- Avec **Paramètres › Téléchargements › Récupérer les liens copiés** activé, les liens que vous copiez dans une autre appli vont d’eux-mêmes dans Liens (pas quand la fenêtre de Pako est au premier plan, ni les liens déjà dans Pako).

Les nouveaux liens passent d’abord par **Liens**, où Pako les vérifie et où vous choisissez quoi télécharger (voyez [Liens et petite fenêtre](links-and-tray.md)). Cliquez sur **Télécharger** là-bas et ils passent dans **Téléchargements**.

## Suivre vos téléchargements {#follow-your-downloads}

L’onglet **Téléchargements** liste tout, avec **Source**, **Progression**, **Vitesse**, **État** et **Date**. Cliquez sur le titre d’une colonne pour trier par elle ; par défaut, les téléchargements les plus récents sont en haut. Triée par **Vitesse** ou **Progression**, la liste se réordonne toutes les quelques secondes, et reste immobile quand le pointeur est dessus.

Vous choisissez ce que montre la liste, dans chaque liste à colonnes (Téléchargements, Liens, la liste **Mis en ligne** d’Envoi et la liste du mode privé) :

- **Plus large ou plus étroite** : faites glisser le trait à gauche du titre d’une colonne. Double-cliquez dessus pour retrouver la largeur d’origine. Au clavier, appuyez sur **Tab** jusqu’à sélectionner le trait, puis **←** (plus large) et **→** (plus étroite), **Maj** pour de plus grands pas, **Début** pour la plus large et **Fin** pour la plus étroite. Une colonne ne peut pas dépasser la place qui reste : le nom en garde toujours un peu.
- **Afficher ou masquer des colonnes** : faites un clic droit sur les titres des colonnes, ou cliquez sur le petit bouton des colonnes à leur droite, et cochez celles que vous voulez. **Nom** est toujours affiché. **Rétablir les colonnes** remet chaque colonne à sa largeur d’origine.
- Pako s’en souvient pour chaque liste. Dans une fenêtre étroite, les colonnes rétrécissent d’abord, puis les moins importantes s’effacent (« Pas de place pour l’instant » à côté d’elles dans ce menu) et reviennent quand il y a la place (**État** part en dernier, seulement sous environ 460 px de liste) ; une colonne que vous avez masquée reste masquée. Masquer la colonne qui sert au tri remet la liste dans son ordre habituel (Téléchargements : les plus récents d’abord).

La partie gauche filtre la liste :

- **Tout**, **En cours**, **En attente**, **Terminés** et **Problèmes**.
- **Privé**, avec un cadenas, une fois le [mode privé](private-mode.md) activé. Tant qu’il est ouvert, l’en-tête de la liste affiche **Téléchargements privés** et **Verrouiller** (en violet) ; choisir une autre entrée ici le verrouille.
- **Sources** : seulement Google Drive, seulement les liens web, et ainsi de suite.

Sélectionnez un téléchargement pour voir ses détails à droite (**Ctrl+I** affiche ou masque ce panneau). Vous y voyez ses parties, où il est enregistré, s’il peut reprendre après une pause, un **Aperçu** pour un fichier vidéo pas encore terminé (lisez un court extrait avant qu’il soit complet ; les types autres que MP4 et WebM demandent **Aperçu de plus de types de vidéos** dans **Paramètres › Fonctions et mises à jour** ; une vidéo de YouTube ou d’un autre site lit plutôt un court extrait demandé au site), et des boutons pour **Pause**, **Reprendre** ou **Réessayer** (**Ouvrir** une fois terminé), ouvrir son **Dossier**, ou le **Retirer**.

Chaque panneau de détails de Pako (Téléchargements, Liens, Envoi, Direct, mode privé) peut être élargi ou rétréci : faites glisser son bord gauche. Pako se souvient de la largeur pour chaque page. Double-cliquez sur le bord pour revenir à la largeur d’origine. Au clavier, appuyez sur **Tab** jusqu’à sélectionner le bord, puis **←** et **→** (**Maj** pour de plus grands pas), **Début** pour le plus large et **Fin** pour le plus étroit. Quand la fenêtre est étroite, le panneau rétrécit d’abord, pour que la liste garde sa place.

**Reprendre** et **Pause** en haut agissent sur les téléchargements sélectionnés, ou sur tous ceux affichés quand rien n’est sélectionné. **Retirer** agit sur ceux qui sont sélectionnés.

Une ligne marquée **Via navigateur** est un fichier que seul votre navigateur pouvait récupérer : le navigateur le télécharge, d’un seul tenant (sans le téléchargement en plusieurs parties, plus rapide, de Pako), et Pako le suit et le déplace dans votre dossier une fois terminé. Ses boutons **Pause**, **Reprendre** et **Retirer** agissent sur le téléchargement du navigateur. Voyez [Extension de navigateur](browser-extension.md#via-browser).

## Rapide et régulier {#fast-and-steady}

- Pako découpe un gros fichier en plusieurs parties et les télécharge en même temps. Le panneau de détails montre les parties qui se remplissent.
- **À la fois** (barre du bas) règle combien de téléchargements tournent ensemble, jusqu’à 20. Utilisez **−** et **+** pour le changer. Les autres attendent leur tour.
- **Paramètres › Téléchargements** a aussi une **Limite de vitesse**, et sous **Avancé**, en bas de l’onglet, **Connexions par fichier**. Avec **Auto** activé (par défaut), un fichier démarre avec 4 connexions et Pako en ajoute tant que cela accélère le fichier, jusqu’à 16 (8 pour les fichiers Google Drive). Désactivez **Auto** pour fixer un nombre de 1 à 32.
- Si la connexion coupe, Pako attend et reprend là où il s’est arrêté. L’état affiche **Reconnexion** ou **Pas d’internet** pendant ce temps.
- **Pause** garde ce qui est fait. **Reprendre** continue à partir de là, même après avoir fermé Pako ou redémarré le PC, quand le site le permet. Le panneau de détails le dit sous **Après une pause**.
- Si le fichier sur le serveur a été remplacé depuis le début du téléchargement, Pako ne mélange pas les deux : il s’arrête et dit « Le fichier a changé sur le serveur depuis le début de ce téléchargement ». Utilisez **Recommencer**.
- **Recommencer** (clic droit sur un téléchargement) jette ce qui est fait et recommence de zéro.
- Si un lien répond avec une page web (une page de connexion, ou « lien expiré ») au lieu du fichier, Pako s’arrête et dit « Le serveur a envoyé une page web au lieu du fichier ». Ouvrez le lien dans votre navigateur et téléchargez-le là : Pako récupère le téléchargement. Un téléchargement du navigateur qui s’avère être une page web reste dans le navigateur.
- Un fichier que vous avez choisi dans **Liens** parmi plusieurs hébergeurs emporte les autres hébergeurs comme secours. Si son hébergeur échoue pour de bon, ou ne répond plus pendant 5 minutes d’affilée, Pako continue tout seul avec le suivant : la ligne affiche **depuis** et le nouvel hébergeur, et les détails montrent les **Secours** restants et les hébergeurs **Essayés**. Si le fichier a la même taille sur le nouvel hébergeur, Pako compare quelques petits morceaux de celui-ci avec ce qu’il a déjà, et reprend à partir de là s’ils correspondent. Quand tous les hébergeurs échouent, le téléchargement s’arrête et son message se termine par « Les 3 hébergeurs ont tous été essayés » et leurs noms ; **Réessayer** refait le tour une fois de plus. Voyez [Le même fichier sur plusieurs hébergeurs](links-and-tray.md#same-file-on-several-hosts).

## Quand tout est terminé {#when-everything-is-done}

**À la fin** (barre du bas) dit à Pako quoi faire une fois que tous les téléchargements sont terminés (il attend aussi les mises en ligne et les transferts Direct) : **Ne rien faire**, **Mettre en veille** ou **Arrêter**. Avant la mise en veille ou l’arrêt, Pako affiche un court compte à rebours que vous pouvez annuler. Le choix revient à **Ne rien faire** une fois l’action faite, et à chaque démarrage de Pako.

Les téléchargements en cours à la fermeture de Pako continuent d’eux-mêmes au prochain démarrage. Pour qu’ils attendent plutôt **Reprendre**, désactivez **Paramètres › Téléchargements › Avancé › Reprendre les téléchargements au démarrage de Pako**.

Avec **Paramètres › Téléchargements › Garder le PC éveillé pendant les téléchargements** activé, Windows ne se met pas en veille au milieu d’un téléchargement, d’une mise en ligne ou d’un transfert Direct.

Les téléchargements peuvent aussi être limités à certaines heures : **Paramètres › Téléchargements › Télécharger seulement à certaines heures**. Les téléchargements en attente affichent alors **Prévu · 22:00** (par exemple), et la barre du bas indique quand ils démarrent.

## Google Drive {#google-drive}

Vous n’avez besoin de vous connecter à rien.

- **Un fichier partagé** (« Tous les utilisateurs disposant du lien ») : collez son lien. Pako le télécharge directement chez Google, en parties parallèles.
- **Un dossier partagé** (« Tous les utilisateurs disposant du lien ») : collez son lien. Pako liste chaque fichier à l’intérieur, sous-dossiers compris, dans **Liens**. Cochez ce que vous voulez et cliquez sur **Télécharger**. Pas besoin d’attendre que Google fasse un zip.
- Après un téléchargement depuis Drive, Pako compare le fichier avec la référence qu’en garde Google (**Paramètres › Téléchargements › Avancé › Vérifier les fichiers Drive une fois finis**). Le **Contrôle du fichier** du panneau de détails affiche « Réussi : le fichier est complet » quand ils correspondent.

### « Limite Google » : trop de téléchargements aujourd’hui {#google-limit-too-many-downloads-today}

Quand un fichier partagé a été téléchargé trop de fois en une journée, Google le bloque pendant un moment. Le téléchargement affiche **Limite Google**, et ses détails disent quoi faire :

1. Cliquez sur **Ouvrir dans Drive** (panneau de détails), ou faites un clic droit sur le téléchargement › **Ouvrir dans Drive (faire une copie)**.
2. Cliquez sur **Ajouter un raccourci dans Drive**, puis dans votre Drive faites un clic droit dessus et choisissez **Créer une copie**.
3. Téléchargez votre copie : Pako la récupère (avec l’extension de navigateur). Supprimez la copie ensuite pour libérer la place.

Vous pouvez aussi attendre : Pako réessaie de lui-même l’original toutes les quelques heures, ou cliquez sur **Réessayer**.

### « Navigateur seulement » : fichiers partagés avec certaines personnes {#only-in-the-browser-files-shared-with-certain-people}

Certains fichiers ne sont partagés qu’avec certains comptes, pas avec « Tous les utilisateurs disposant du lien ». Pako affiche **Navigateur seulement** :

1. Cliquez sur **Ouvrir dans le navigateur** (panneau de détails), ou faites un clic droit sur le téléchargement › **Ouvrir dans le navigateur pour télécharger**.
2. Connectez-vous au bon compte Google dans votre navigateur si besoin, et cliquez sur Télécharger là-bas.
3. Pako récupère le téléchargement et le termine, toujours en parties parallèles.

Il en va de même pour un dossier que Google ne veut pas lister : ouvrez-le dans votre navigateur et téléchargez depuis là-bas. Si Google refuse Pako lui-même pendant un moment (pas le dossier), Pako dit « Pako ne peut pas lister les dossiers Drive pour le moment » et propose le même chemin par votre navigateur.

## Avant qu’un téléchargement démarre {#before-a-download-starts}

- **Déjà téléchargé ?** Si un fichier est déjà dans vos téléchargements ou dans son dossier, Pako demande d’abord. **Télécharger à nouveau** garde l’ancien fichier et enregistre une deuxième copie sous un nouveau nom. Quand seuls certains liens sont déjà téléchargés, **L’ignorer** (ou **Les ignorer**) télécharge seulement les autres. **Annuler** ne télécharge rien.
- **Espace disque.** Si le lecteur est trop plein, le téléchargement s’arrête avec un message qui dit combien il lui faut et combien il reste de libre. Libérez de la place, puis cliquez sur **Réessayer**.

## Clic droit sur un téléchargement {#right-click-a-download}

- **Télécharger d’abord** le place devant les autres, même en dehors des heures choisies.
- **Pause**, **Reprendre**, ou **Réessayer** pour un téléchargement arrêté.
- **Priorité : D’abord**, **Normale** ou **Plus tard**, et **Placer en haut de la file** / **Placer en bas de la file**.
- **Prévenir : Désactivé**, **Prévenir : Pop-up** ou **Prévenir : Important** : comment il vous prévient quand il se termine. **Pop-up** (l’habituel) est un pop-up en bas à droite et un son ; **Important** est un grand pop-up au milieu de l’écran, qui reste jusqu’à ce que vous cliquiez dessus. La cloche au bout de sa ligne le change aussi, un clic à la fois (voyez [Notifications](notifications.md#when-a-download-finishes-off-pop-up-important)).
- **Ouvrir le fichier**, **Afficher dans le dossier**, **Ouvrir le lien dans le navigateur**, **Copier le lien**, **Renommer…**
- **Déplacer vers Privé** (un fichier terminé, une fois le [mode privé](private-mode.md) activé) : le chiffre dans le mode privé et le retire de cette liste.
- **Extraire…** (un .zip, .7z ou .rar terminé, ou la première partie d’une archive en plusieurs parties) : 7-Zip l’extrait dans un dossier qui porte son nom, à côté de lui (une archive en plusieurs parties : là où sont ses parties). Une archive protégée par un mot de passe le demande ; Pako le donne à 7-Zip et ne le garde pas. Un mauvais mot de passe est refusé sans rien écrire. Il faut 7-Zip (**Extraire les archives en plusieurs parties** dans **Paramètres › Fonctions et mises à jour › Ce que Pako sait faire**).
- **Retirer**. Pour un téléchargement terminé, Pako demande s’il faut seulement **Retirer de la liste**, ou **Supprimer aussi le fichier** (dans la Corbeille). Les parties pas terminées sont toujours supprimées.

Un clic droit sur une partie vide de la liste propose **Coller des liens**, **Ajouter des liens…**, **Tout sélectionner** et **Retirer les terminés**.

## Raccourcis clavier {#keyboard-shortcuts}

- **Ctrl+V** : coller des liens (n’importe où).
- **Ctrl+F** ou **Ctrl+K** : chercher dans la page où vous êtes.
- **Ctrl+I** : afficher ou masquer le panneau de détails.
- **Ctrl+Tab** : passer de Téléchargements à Liens et inversement.
- **Ctrl+Maj+P** : ouvrir le [mode privé](private-mode.md) (une fois activé), ou le verrouiller quand la liste privée est affichée.
- **↑ ↓**, **Début**, **Fin** : se déplacer dans la liste ; maintenez **Maj** pour en sélectionner plusieurs.

- **Espace** : sélectionner ou désélectionner la ligne où vous êtes (la cocher ou la décocher, dans Liens).
- **Ctrl+A** : tout sélectionner. **Échap** : effacer la sélection.
- **Suppr** : retirer. **F2** : renommer. **Entrée** : ouvrir un fichier terminé.
- **Maj+F10** ou la touche **Menu** : le menu du clic droit de la ligne.
- **Ctrl+Z** : annuler le dernier retrait, pendant quelques secondes et dans la même liste (des liens, ou des téléchargements terminés retirés de la liste sans supprimer le fichier). Le message qui s’affiche a aussi **Annuler**.
- **F1** : ouvrir l’Aide.

La liste privée a ses propres touches, presque les mêmes (voyez [Mode privé](private-mode.md#the-private-list)) ; **Ctrl+Tab** ne s’y applique pas.

## Clair ou sombre {#light-or-dark}

Vous choisissez l’apparence dans la configuration du premier démarrage (sombre, sauf si vous en choisissez une autre). Cliquez sur le soleil en haut à droite, à côté de la cloche, pour passer en clair ; cliquez sur la lune pour revenir. Le choix est gardé, et la petite fenêtre de la zone de notification, les pop-ups et la fenêtre **Télécharger avec Pako** le suivent.

Dans **Paramètres › Fenêtre › Apparence › Thème**, choisissez **Sombre**, **Clair** ou **Comme Windows**. **Comme Windows** suit le mode des applications choisi dans Windows (**Paramètres › Personnalisation › Couleurs**), même pendant que Pako est ouvert. La fenêtre de l’extension de navigateur suit le mode clair ou sombre de votre navigateur.

## Anglais ou français {#english-or-french}

Pako parle anglais ou français. Il ne devine rien d’après Windows : il demande, et reste en anglais tant que vous ne choisissez pas le français.

- **Pako Setup** demande en premier, avec **English | Français** en haut à droite de sa fenêtre (voyez [La fenêtre de Pako Setup](install.md#pako-setups-window)).
- **La configuration du premier démarrage** demande dans sa première ligne, **Langue** : c’est là qu’un Pako portable demande.
- **Plus tard**, choisissez **English** ou **Français** dans **Paramètres › Fenêtre › Apparence › Langue**. Les Paramètres l’affichent tout de suite ; **Enregistrer** le garde, **Annuler** revient en arrière.

Le changement touche toutes les fenêtres d’un coup (la petite fenêtre de la zone de notification, les pop-ups, la fenêtre **Télécharger avec Pako**), le menu de l’icône de la zone de notification, les notifications, les guides de l’Aide et les Nouveautés, et l’extension de navigateur (voyez [Vérifier la connexion](browser-extension.md#check-its-connected)). Un guide pas encore traduit s’affiche en anglais. Les noms que vous donnez (contacts, appareils, fichiers) restent tels que vous les avez écrits.

## Bon à savoir {#good-to-know}

- Les téléchargements vont dans le dossier choisi au premier démarrage de Pako. Changez-le dans **Paramètres › Téléchargements › Dossier de téléchargement**.
- Le dossier d’un téléchargement peut être changé tant qu’il n’a pas démarré (panneau de détails, **Enregistrer dans**).
- Pendant qu’un fichier se télécharge, il est dans son dossier avec `.pakopart` ajouté à son nom (`film.mp4.pakopart`), et prend son vrai nom une fois complet. Les téléchargements laissés inachevés par un Pako plus ancien reprennent à partir de ce qu’ils ont.
- Faites glisser des téléchargements terminés hors de la liste, dans n’importe quel dossier.
- Les archives en plusieurs parties (.part1.rar, .7z.001, .zip.001…) sont extraites quand toutes les parties sont terminées (**Paramètres › Téléchargements › Extraire les archives en plusieurs parties**, ou **Extraire quand toutes les parties sont terminées** dans les détails de l’ensemble). Il faut 7-Zip : installez **Extraire les archives en plusieurs parties** (qui apporte 7-Zip) dans **Paramètres › Fonctions et mises à jour › Ce que Pako sait faire**. Un fichier du même nom déjà dans le dossier est gardé : celui qui est extrait prend un nouveau nom. Une archive en plusieurs parties protégée par un mot de passe attend : ses détails affichent **Saisissez le mot de passe…** (aussi dans son menu du clic droit), et elle est extraite une fois que vous l’avez tapé.
- Tout ce que Pako sait de vos téléchargements reste sur votre PC.
