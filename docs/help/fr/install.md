# Installer et mettre à jour

Obtenir Pako, le garder à jour, et le retirer si besoin. Pako fonctionne sous Windows 10 et 11 (64 bits).

## Pako Setup ou Pako Portable {#pako-setup-or-pako-portable}

Pako existe en deux versions. C’est le même programme, et les deux se mettent à jour de la même façon.

- **Pako Setup** installe Pako pour vous, sans droits d’administrateur, dans `%LOCALAPPDATA%\Programs\Pako` (vous pouvez choisir un autre dossier). Il ajoute Pako au menu Démarrer (et sur le Bureau si vous le voulez) et à la liste des applications de Windows, pour qu’il se désinstalle comme n’importe quelle appli.
- **Pako Portable** est un seul fichier, `Pako.exe`. Mettez-le dans n’importe quel dossier et lancez-le. Il n’est ajouté ni au menu Démarrer ni à la liste des applications de Windows. Donnez-lui un dossier à lui, que vous seul modifiez (pas votre dossier Téléchargements, où des pages web peuvent enregistrer des fichiers à côté de lui). Sa configuration du premier démarrage a, au départ, les mêmes choix activés que Pako Setup : **Démarrer Pako avec Windows** et **Ajouter Pako à mes navigateurs**. Elle y demande aussi la langue (**Langue**, sa première ligne).

**Paramètres › Fonctions et mises à jour**, le premier onglet des Paramètres, montre laquelle vous avez : **Installé** ou **Portable**, à côté de la version.

### La fenêtre de Pako Setup {#pako-setups-window}

Pako Setup tient dans une seule fenêtre, claire ou sombre comme Windows :

- **En haut à droite**, **English | Français** choisit la langue de Setup (English au départ ; sur un Pako installé, la langue de Pako). Les mots de Setup changent tout de suite ; Pako démarre ensuite dans cette langue. Changez-la plus tard dans **Paramètres › Fenêtre › Apparence › Langue** : le programme de désinstallation, et le prochain Pako Setup sur ce Pako, la suivent (voyez [Anglais ou français](downloading.md#english-or-french)).
- **À gauche**, cinq courtes diapositives montrent ce que fait Pako. Elles défilent toutes seules toutes les quelques secondes, et attendent quand vous pointez dessus. Cliquez sur **‹** ou **›**, ou sur un point, pour les faire défiler vous-même (elles restent alors là où vous les avez mises).
- **À droite**, sous le logo de Pako et la version qu’installe Setup (**Version 1.3.2**, par exemple), tout ce que vous pouvez choisir, d’un seul coup d’œil :
  - **Installer dans** : le dossier, `%LOCALAPPDATA%\Programs\Pako` au départ. **Modifier** en choisit un autre (Pako a son propre dossier dans celui que vous choisissez).
  - **Raccourci sur le Bureau** (activé au départ).
  - **Démarrer Pako avec Windows** (activé au départ, sauf si un Pako portable démarre déjà avec Windows : celui-ci continue de le faire).
  - **Connecter Pako à vos navigateurs** (activé au départ), pour que l’extension de navigateur puisse parler à Pako.
- Cliquez sur **Installer**. La fenêtre affiche **Installation de Pako…** avec une barre, puis **Pako est installé** : **Ouvrir Pako** le lance (**Entrée** aussi) ; **Fermer**, **Échap** et le **✕** de la fenêtre ne le lancent pas.
- Si quelque chose se passe mal pendant l’installation (Pako n’a pas pu être fermé, par exemple), la ligne sous la barre dit quoi, en rouge, et **Fermer** termine Setup.
- Tout marche aussi au clavier : **Tab** passe d’un choix au suivant (un contour montre où vous êtes), **Espace** active ou désactive un interrupteur et clique sur un bouton, **Entrée** installe. Les lecteurs d’écran comme le Narrateur lisent chaque choix, et si un interrupteur est activé.

**Démarrer Pako avec Windows** et **Connecter Pako à vos navigateurs** deviennent des réglages de Pako : au premier démarrage, sa configuration les montre tels que vous les avez laissés, et vous pouvez les changer à tout moment dans les **Paramètres** (**Fenêtre** et **Extension de navigateur**). Si Setup n’a pas pu les transmettre à Pako, **Pako est installé** le dit ; activez-les alors vous-même là-bas. **Raccourci sur le Bureau** n’appartient qu’à Setup : supprimez le raccourci ou relancez Setup pour le changer.

### Où sont gardés vos paramètres et vos listes {#where-your-settings-and-lists-are-kept}

- **Pako Setup** : dans votre dossier d’utilisateur Windows, `%LOCALAPPDATA%\Pako`. Réinstaller ou mettre à jour n’y touche jamais.
- **Pako Portable** : dans un dossier `data` à côté de `Pako.exe`. Si Pako ne peut pas y écrire (dans Program Files, par exemple), il utilise `%LOCALAPPDATA%\Pako` à la place.

Vos fichiers téléchargés ne sont jamais dans ces dossiers : ils vont dans votre dossier de téléchargement.

Pako Setup n’installe pas Pako dans `%LOCALAPPDATA%\Pako` ni dans le dossier d’un Pako portable, pour que vos paramètres et vos listes ne se mélangent pas : **Installer** reste grisé, et la fenêtre dit pourquoi sous les choix (« Pako garde ses paramètres et ses listes dans ce dossier. Choisissez-en un autre. » ou « Une version portable de Pako est dans ce dossier. Choisissez-en un autre, pour que ses listes restent avec elle. »). Pour passer d’un Pako portable à un Pako installé, voyez [De Portable à Setup](#from-portable-to-setup).

Il lui faut aussi un dossier que vous pouvez modifier sans droits d’administrateur, car Pako se met à jour sans eux. Si Windows ne vous laisse pas modifier le dossier choisi, la fenêtre dit pourquoi et **Installer** attend que vous en choisissiez un autre :

- « Seuls les administrateurs peuvent modifier ce dossier : Pako ne pourrait pas s’y mettre à jour. Choisissez un de vos dossiers. » (`Program Files`, par exemple)
- « Windows ne vous laisse pas modifier ce dossier. Choisissez un de vos dossiers. »
- « Ce lecteur ou ce dossier réseau est introuvable. Choisissez un autre dossier. » (un lecteur qui n’est pas branché, par exemple)
- « Le chemin de ce dossier est trop long pour Windows. Choisissez-en un plus court. »

Lancer Pako Setup en tant qu’administrateur n’y change rien : les dossiers comme `Program Files` et `Windows` sont refusés de la même façon. Une installation silencieuse (`/S /D=dossier`) dans un dossier que vous ne pouvez pas modifier s’arrête avant d’installer quoi que ce soit, avec le code de sortie 2.

## Conditions d’utilisation {#terms-of-use}

Pako a des conditions d’utilisation, les mêmes que sur [sookera.com/terms](https://sookera.com/terms). En bref : Pako est gratuit et fourni tel quel, sans garantie ; vous êtes responsable de ce que vous téléchargez, partagez, envoyez et enregistrez avec ; utilisez-le seulement pour ce que vous avez le droit de faire, et suivez les règles des sites que vous utilisez.

- **Pako Setup** le dit sous son bouton **Installer** : « En installant, vous acceptez les conditions d’utilisation. » Cliquez sur **conditions d’utilisation** pour les lire sur sookera.com avant d’installer. Cliquer sur **Installer** (ou **Mettre à jour**) vaut accord, donc Pako ne redemande pas à son démarrage. Une installation lancée sans sa fenêtre (une installation silencieuse) ne compte pas : Pako demande alors à son premier démarrage.
- **Pako Portable** (et une installation silencieuse) les montre au premier démarrage de Pako, avant tout le reste, avec **Accepter et continuer** et **Quitter Pako**. Rien ne fonctionne avant votre accord : pas de téléchargements, ni Direct ni Mes appareils, rien de l’extension de navigateur (elle dit « Pako a besoin que vous acceptiez d’abord ses conditions »), aucune recherche de mises à jour, et l’icône de la zone de notification ne fait qu’ouvrir ou quitter Pako. Fermer la fenêtre ou appuyer sur **Échap** ne les fait pas passer. Dès que vous acceptez, tout démarre, sans redémarrage, et la configuration du premier démarrage suit.
- **Si vous utilisiez Pako avant qu’il ait des conditions** (Pako 1.0.x), il les montre une fois après la mise à jour, sous le titre **Conditions d’utilisation de Pako**. Vos téléchargements et vos listes restent tels quels : les téléchargements en pause et en attente attendent simplement votre accord, et rien n’est perdu. Un Pako qui se met à jour et redémarre dans la zone de notification ne s’affiche pas de lui-même : les conditions s’affichent la prochaine fois que vous ouvrez Pako, ou quand l’extension de navigateur lui envoie un téléchargement (le navigateur garde ce téléchargement en attendant). D’ici là, l’info-bulle de l’icône de la zone de notification dit « Pako : ouvrez-le pour continuer ».

Pako demande une seule fois : il note dans ses paramètres la date des conditions que vous avez acceptées et ne redemande plus, même après les mises à jour suivantes. La dernière version est toujours sur [sookera.com/terms](https://sookera.com/terms) ; **Conditions**, à côté de **Confidentialité** dans l’Aide et dans **Paramètres › Fonctions et mises à jour**, l’ouvre, et **Aide › À propos de Pako › Conditions d’utilisation** montre le texte venu avec votre Pako.

## « Windows a protégé votre ordinateur » {#windows-protected-your-pc}

Pako n’est pas signé avec un certificat payant, donc la première fois que vous le lancez, Windows peut afficher « Windows a protégé votre ordinateur ». Cliquez sur **Informations complémentaires**, puis sur **Exécuter quand même**.

La fenêtre de Pako a besoin de Microsoft Edge WebView2, que Windows 11 et la plupart des PC sous Windows 10 ont déjà. S’il manque, Pako Setup le télécharge chez Microsoft et l’installe. Pako Portable ne le fait pas : il dit de l’installer (voyez [Si Pako doit se fermer](help-and-problems.md#if-pako-has-to-close)).

## Logiciel libre {#free-software}

Pako est un logiciel libre sous licence GNU GPL version 3, avec les conditions supplémentaires de Sookera. **Aide › À propos de Pako** montre les droits d’auteur, la licence et les conditions supplémentaires, avec un lien vers le code source ; **Logiciel libre (GPL-3.0)** dans l’Aide et dans **Paramètres › Fonctions et mises à jour** l’ouvre aussi. Chaque version est livrée avec les mêmes textes que `LICENSE` et `NOTICE.md`, et Pako Setup les met à côté de `Pako.exe`, avec les conditions d’utilisation (`TERMS.md`).

Son code source est sur GitHub : [github.com/sookera/pako](https://github.com/sookera/pako).

## Mises à jour {#updates}

Pako cherche une nouvelle version une fois par jour, même quand il reste ouvert dans la zone de notification pendant des jours.

- Avec **Paramètres › Fonctions et mises à jour › Mettre à jour Pako et ses outils automatiquement** activé (comme au départ), Pako télécharge la nouvelle version de lui-même. Un pop-up et la cloche disent « Pako 0.2.0 est prêt » (par exemple), avec **Redémarrer maintenant**, même quand Pako attend dans la zone de notification. Quand des fichiers se téléchargent, se mettent en ligne ou passent par Direct, **Redémarrer maintenant** demande d’abord, comme Quitter. Si vous ne redémarrez pas, la nouvelle version démarre la prochaine fois que vous lancez Pako. Les outils qu’utilise Pako (pour les vidéos, pour les archives en plusieurs parties) se mettent aussi à jour en silence. Quand ce réglage est désactivé, ils ne sont mis à jour que si vous cliquez sur **Mettre à jour** à côté d’eux, sous **Ce que Pako sait faire** (cliquez dessus pour ouvrir la liste ; elle s’ouvre d’elle-même quand un outil a une mise à jour).
- Quand il est désactivé, Pako se contente de vous prévenir : « Pako 0.2.0 est disponible (Paramètres › Fonctions et mises à jour) ». Cliquez sur **Mettre à jour** là-bas, puis sur **Redémarrer maintenant**.
- **Rechercher des mises à jour** (dans **Paramètres › Fonctions et mises à jour**) vérifie tout de suite (et, avec les mises à jour automatiques activées, télécharge la nouvelle version).

Après une mise à jour, Pako montre une fois les nouveautés : celles de chaque version depuis celle que vous aviez, les plus récentes d’abord, quand une mise à jour en a sauté quelques-unes (l’Aide les a toutes).

Les mises à jour sont sûres :

- Pako n’accepte qu’une nouvelle version signée par Sookera. Tout le reste est refusé, d’où qu’il vienne.
- Vos paramètres et vos listes ne sont pas touchés.
- Si la nouvelle version ne démarre pas correctement, ou si Windows refuse de la lancer (un antivirus, par exemple), la précédente est remise en place et continue de fonctionner, et **Paramètres › Fonctions et mises à jour** le dit. Pako ne retélécharge pas cette version de lui-même ; **Mettre à jour** la réessaie.

Un Pako portable dans un dossier que Windows ne vous laisse pas modifier (Program Files, par exemple) ne peut pas se mettre à jour, et Pako le dit. Déplacez son dossier là où vous pouvez modifier des fichiers, comme Documents.

### Un Pako Setup plus récent sur un Pako installé {#a-newer-pako-setup-over-an-installed-pako}

Vous pouvez aussi lancer un **Pako Setup** plus récent sur le Pako installé. Sa fenêtre s’appelle alors **Mettre à jour Pako** et s’ouvre dans la langue de Pako : elle dit quelle version vous avez et laquelle vous aurez, garde Pako dans son dossier (pas de **Modifier**), et ses interrupteurs montrent ce que vous avez maintenant. Cliquez sur **Mettre à jour**. Si le dossier de Pako ne peut plus servir (il a été installé dans `Program Files` par un ancien Setup, par exemple), la fenêtre le dit et montre **Modifier** : choisissez un de vos dossiers et Pako y va, avec vos paramètres et vos listes (un raccourci épinglé à la barre des tâches peut devoir être épinglé de nouveau ; si Windows ne laisse pas Setup vider l’ancien dossier, ses fichiers y restent pour qu’un administrateur les supprime). Le même Pako Setup sur la même version propose **Réinstaller** ; un plus ancien sur un Pako plus récent propose **Installer cette version** et remet cette version. Si Pako est ouvert, la fenêtre dit « Pako est ouvert : l’installation le ferme d’abord » et le ferme quand vous cliquez sur **Mettre à jour**, sans redemander. Vos paramètres et vos listes, les raccourcis, l’épingle de la barre des tâches, le démarrage avec Windows et le lien avec les navigateurs restent tous comme le montrent les interrupteurs, et Setup ne demande pas s’il faut supprimer quoi que ce soit.

## Désinstaller {#uninstalling}

### Pako Setup {#pako-setup}

1. Ouvrez **Paramètres › Applications** (**Applications installées**) de Windows, trouvez Pako et cliquez sur **Désinstaller**. La fenêtre **Désinstaller Pako** s’ouvre, dans le style et la langue de Pako, avec la version que vous retirez sous le logo de Pako. (Si vous avez installé Pako avec un Pako Setup antérieur à 1.3.0, le programme de désinstallation est encore celui de cette version : une fenêtre plus simple, en anglais, avec une case **Also delete Pako’s settings and lists** et un bouton **Uninstall**. Les mises à jour de Pako lui-même ne le remplacent pas ; lancer une fois un Pako Setup plus récent, si.)
2. Activez **Supprimer aussi les paramètres et listes de Pako** seulement si vous voulez les effacer aussi. C’est désactivé au départ, pour qu’une réinstallation plus tard reprenne là où vous en étiez. Cela ne supprime jamais les fichiers du [mode privé](private-mode.md) : ils ont leur propre interrupteur, **Supprimer aussi les fichiers du mode privé** (désactivé au départ, lui aussi). Vos téléchargements privés chiffrés ne peuvent pas être téléchargés à nouveau, donc ils sont gardés sauf si vous activez celui-ci ; les supprimer est définitif. (Un programme de désinstallation d’avant 1.3.0 les garde aussi.)
3. Cliquez sur **Désinstaller** (ou **Annuler**). Si Pako est ouvert, le programme de désinstallation propose de le fermer d’abord. La fenêtre dit ensuite **Pako est désinstallé** : cliquez sur **Fermer**.

La désinstallation retire Pako lui-même, ses raccourcis, et ce qu’il avait mis en place dans Windows : le lien avec vos navigateurs, les liens de Pako depuis les pages web, et le démarrage avec Windows (le lien avec les navigateurs et le démarrage avec Windows d’un Pako portable restent). Vos fichiers téléchargés restent, tout comme vos mises en ligne sur Gofile ou Google Drive. L’extension de navigateur reste dans votre navigateur jusqu’à ce que vous l’y retiriez.

Si un Pako portable est ouvert pendant que vous désinstallez avec cet interrupteur activé, les paramètres et les listes sont gardés (il les utilise peut-être aussi) et le programme de désinstallation le dit. Pour les supprimer quand même, fermez ce Pako, puis supprimez les dossiers `Pako` et `com.sookera.pako` dans `%LOCALAPPDATA%`.

Avec l’interrupteur activé, le dossier de Pako dans le dossier temporaire de Windows (`%TEMP%\Pako`) part aussi : les enregistrements pas encore transformés en vidéo.

Avec l’interrupteur désactivé, les deux dossiers restent : `Pako` (paramètres et listes) et `com.sookera.pako` (les données propres à la fenêtre, comme sa taille et sa place ; un Pako antérieur à 1.0.0 y gardait aussi ce que les pages de liens enregistraient).

### Pako Portable {#pako-portable}

1. Dans Pako, désactivez **Paramètres › Fenêtre › Démarrer Pako avec Windows** et **Paramètres › Extension de navigateur › Connecter Pako à vos navigateurs**, puis cliquez sur **Enregistrer**.
2. Quittez Pako (clic droit sur l’icône de la zone de notification › **Quitter Pako**).
3. Supprimez `Pako.exe` et son dossier `data`, ainsi que le dossier `com.sookera.pako` dans `%LOCALAPPDATA%` (les données propres à la fenêtre, comme sa taille et sa place ; un Pako antérieur à 1.0.0 y gardait aussi ce que les pages de liens enregistraient). Si vous avez enregistré des vidéos, supprimez aussi le dossier `Pako` dans `%TEMP%`.

Le dossier `data` d’un Pako portable contient vos listes (les liens que vous téléchargez, vos contacts) : toute personne qui peut ouvrir le dossier où se trouve `Pako.exe` peut les lire, alors gardez-le dans un dossier à vous. Ses connexions (Google Drive, Gofile, la clé de votre ID Pako) sont chiffrées pour votre utilisateur Windows et restent illisibles.

## De Portable à Setup {#from-portable-to-setup}

Le Pako installé garde ses paramètres et ses listes ailleurs que le Pako portable, donc il démarre avec des listes vides et repose les questions du premier démarrage.

Pour garder vos listes et vos paramètres (téléchargements, mises en ligne, contacts et historique de Direct, chiffres du Tableau de bord) :

1. Quittez Pako (clic droit sur l’icône de la zone de notification › **Quitter Pako**).
2. Ouvrez le dossier `data` à côté de votre `Pako.exe` portable et copiez tout ce qu’il contient.
3. Tapez `%LOCALAPPDATA%` dans la barre d’adresse de l’Explorateur de fichiers et appuyez sur **Entrée**. Ouvrez-y le dossier `Pako` (créez-le s’il n’existe pas) et collez les fichiers, en remplaçant ceux qui y sont déjà.
4. Lancez le Pako installé. Il reprend au Pako portable le lien avec les navigateurs et le démarrage avec Windows.
5. Dans Chrome, Edge, Brave, Vivaldi ou Opera, l’extension est encore chargée depuis le dossier du Pako portable. Retirez-la sur la page des extensions du navigateur, puis ajoutez-la de nouveau depuis le Pako installé (voyez [Extension de navigateur](browser-extension.md#add-pako-to-your-browser)). Firefox n’a besoin de rien.

Si votre Pako portable n’a pas de dossier `data` à côté de lui, il gardait déjà tout dans `%LOCALAPPDATA%\Pako` : le Pako installé l’y trouve tout seul.

Faites-le sur le même PC et avec le même utilisateur Windows. Vos contacts Direct reconnaissent votre Pako à sa clé, que seul cet utilisateur Windows peut lire : sur un autre PC, Pako crée un nouvel ID Pako.
