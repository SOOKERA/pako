# Envoi

Partagez de gros fichiers avec un lien. Pako les met en ligne sur Gofile ou dans votre propre Google Drive, copie le lien, et tout le monde peut le télécharger dans son navigateur.

## Comment se passe le partage {#how-sharing-works}

1. **Vous déposez les fichiers** sur la page Envoi.
2. **Pako les met en ligne**, sur Gofile ou dans votre Google Drive. Rien ne passe par nos serveurs.
3. **Vos destinataires ouvrent le lien** dans n’importe quel navigateur, ou le collent dans **Liens** dans leur propre Pako.

## Mettre des fichiers en ligne {#upload-files}

1. Ouvrez **Envoi** à gauche.
2. Cliquez sur **Choisir des fichiers** ou **Choisir un dossier**, ou déposez des fichiers et des dossiers sur la page. Ils s’ajoutent dans **Nouvelle mise en ligne** : rien n’est encore mis en ligne.
3. Choisissez où ils vont et comment (voir plus bas), puis cliquez sur **Mettre en ligne**.

Ajoutez-en d’autres avec **Ajouter des fichiers** et **Ajouter un dossier**, retirez-en un avec son **×**, ou videz tout avec **Vider**. Un élément qui ne peut pas être lu dit pourquoi sous son nom : retirez-le pour mettre en ligne le reste.

Les fichiers du [mode privé](private-mode.md#send-private-files) se mettent en ligne depuis le mode privé : **Mettre en ligne…** dans la liste privée ouvre une fenêtre avec les mêmes choix que **Nouvelle mise en ligne**, et vous restez dans le mode privé pendant la mise en ligne (le quitter l’arrête). Pako les lit déchiffrés au fur et à mesure. Toutes les façons fonctionnent : tels quels, un ZIP, **ZIP en parties** (chaque partie n’est créée que quand vient son tour et supprimée dès qu’elle est en ligne, donc une seule partie à la fois se trouve brièvement sur ce PC, dans un dossier que seul votre compte Windows peut ouvrir), ou protégés par un mot de passe.

Quand la mise en ligne est terminée, Pako copie le lien pour vous : collez-le à qui vous voulez. Il joue aussi un petit son (désactivez-le dans **Paramètres › Notifications** : **Jouer un son quand des fichiers sont envoyés**).

## Où ça va : Mettre en ligne sur {#where-it-goes-upload-to}

- **Gofile** : gratuit, sans compte. Gofile garde les fichiers tant qu’on les télécharge. Après 10 jours sans téléchargement, ils peuvent être retirés.
- **Google Drive** : votre propre Drive, partagé avec toute personne qui a le lien. Le fichier y reste jusqu’à ce que vous le supprimiez. Pako met tout ce qu’il met en ligne dans un dossier de votre Drive appelé « Pako uploads ». Si un Pako plus ancien a créé un dossier appelé « Pako sends », Pako continue à utiliser celui-là, et ce que vous avez mis en ligne avant reste où il est.

Votre choix est gardé pour la prochaine mise en ligne. Vous pouvez aussi le régler dans **Paramètres › Envoi › Mettre en ligne sur**.

## Supprimer après {#delete-after}

Les mises en ligne peuvent se supprimer toutes seules après **3 jours**, **7 jours** ou **30 jours**, ou **Jamais**. **7 jours** est le choix habituel, et Pako garde celui que vous choisissez.

- Sur Gofile, Gofile supprime le fichier lui-même ce jour-là quand il accepte la date, même PC éteint. Sinon, Pako le supprime dès qu’il est en marche après ce jour-là.
- Sur Google Drive, Pako le supprime dès qu’il est en marche après ce jour-là.

Si Pako ne peut pas encore en supprimer une, il affiche **Pas encore supprimé** et réessaie toutes les heures.

## Plusieurs fichiers, ou un seul fichier compressé {#several-files-or-one-compressed-file}

**Plusieurs fichiers** (ou un dossier) peuvent être mis en ligne de quatre façons :

- **Tels quels** : un lien vers un dossier avec les fichiers tels quels. La personne les voit tout de suite.
- **Un ZIP** : mis dans un seul fichier ZIP, sans compression (le plus rapide).
- **ZIP compressé** : mis dans un seul fichier ZIP plus petit. C’est plus long, et c’est utile pour les documents et le texte, pas pour les vidéos ni les photos.
- **ZIP en parties** : un seul fichier ZIP, sans compression, découpé en parties numérotées (« Photos.zip.001 », « Photos.zip.002 »…) qui vont dans un seul dossier, avec un seul lien. Le meilleur choix pour les grosses mises en ligne sur une connexion instable (voir plus bas).

Le choix habituel se trouve dans **Paramètres › Envoi › Quand vous mettez en ligne plusieurs fichiers**, avec les mêmes noms. Le choix dans **Nouvelle mise en ligne** ne vaut que pour cette mise en ligne.

**Nom** donne le nom du dossier ou du fichier ZIP. Il est facultatif : le texte gris montre le nom que Pako utilise sinon.

Un **fichier seul** est mis en ligne tel quel. Sous **Compression**, choisissez **ZIP compressé** pour le mettre plutôt dans un fichier ZIP plus petit, ou **ZIP en parties**, avec un **Nom** si vous voulez.

### ZIP en parties {#zip-in-parts}

Chaque partie est mise en ligne à part : si la connexion coupe, seule la partie en cours recommence, et les parties déjà en ligne y restent.

- **Taille des parties** : **500 Mo** (l’habituelle), **1 Go**, **2 Go**, ou **Personnalisée** pour saisir une taille en Mo, entre 50 Mo et 10 Go. Les petites parties recommencent plus vite ; les grandes font moins de fichiers. La taille habituelle se trouve dans **Paramètres › Envoi › Avancé › Taille de chaque partie**.
- Pako crée d’abord les parties sur votre PC (**Création des parties**), dans son propre dossier de données : il lui faut donc autant d’espace libre à cet endroit que le ZIP. Elles sont supprimées une fois toutes les parties en ligne, ou quand vous annulez ou retirez la mise en ligne.
- La mise en ligne indique combien de parties sont en ligne (« 3 sur 7 parties en ligne »). **Réessayer** met en ligne seulement les parties qui ne le sont pas encore, dans le même dossier, donc le lien reste le même. Si Pako a été fermé ou si le PC a perdu le courant, la mise en ligne revient sous **Mise en ligne en cours** au démarrage de Pako, et continue toute seule avec les parties pas encore en ligne dès que Gofile ou votre Google Drive est joignable (elle réessaie discrètement tant que la connexion est coupée). Désactivez **Paramètres › Envoi › Avancé › Reprendre les mises en ligne inachevées au démarrage de Pako** pour qu’elle attende plutôt **Réessayer**.

**Pour les personnes qui téléchargent** : il leur faut toutes les parties, dans un seul dossier.

- Avec 7-Zip : ouvrez la première partie (« Photos.zip.001 »). 7-Zip rassemble les parties et montre ce qu’il y a dedans.
- Avec Pako, depuis un lien Google Drive : collez le lien dans **Liens**. Pako télécharge toutes les parties dans un seul dossier et, avec **Extraire les archives en plusieurs parties** activé (ou **Extraire quand toutes les parties sont terminées** dans les détails de l’ensemble), rassemble le ZIP et le décompresse.
- Depuis un lien Gofile : téléchargez chaque partie depuis la page de Gofile, puis ouvrez la première avec 7-Zip.

## Protéger par un mot de passe {#protect-with-a-password}

Activez **Protéger par un mot de passe** dans **Nouvelle mise en ligne** et saisissez un mot de passe deux fois (au moins 8 caractères). Ce que vous avez choisi est alors verrouillé sur votre PC avant de partir : il est mis en ligne comme une seule archive 7-Zip (`Photos.7z`), chiffrée avec ce mot de passe (AES-256), noms des fichiers compris. **ZIP en parties** la crée en parties (`Photos.7z.001`, `.002`…) ; **ZIP compressé** la compresse. Elle va toujours dans un dossier avec un seul lien.

- Il faut 7-Zip : installez **Extraire les archives en plusieurs parties** (qui apporte 7-Zip) dans **Paramètres › Fonctions et mises à jour › Ce que Pako sait faire** (**Nouvelle mise en ligne** vous le rappelle).
- Pako crée d’abord l’archive dans son propre dossier de données (**Verrouillage avec votre mot de passe**) : il lui faut donc à peu près autant d’espace libre à cet endroit que ce que vous avez choisi. Elle est supprimée une fois tout mis en ligne.
- Pako ne garde jamais le mot de passe, même pas pour **Réessayer** : si la mise en ligne doit recommencer, Pako vous demande de le saisir à nouveau deux fois, puis **Remettre en ligne**. Donnez le mot de passe vous-même, pas dans le même message que le lien.
- **Pour les personnes qui téléchargent** : elles l’ouvrent avec 7-Zip (gratuit), ou avec Pako (clic droit dessus dans **Téléchargements**, puis **Extraire…**), et saisissent le mot de passe. Sans lui, elles ne peuvent pas voir ce qu’il y a dedans, même pas les noms.

## Pendant la mise en ligne {#while-it-uploads}

Les mises en ligne en cours s’affichent sous **Mise en ligne en cours**, avec leur avancement, la vitesse et le temps restant. **Annuler** en arrête une : ce qui était déjà en ligne est supprimé aussi. Après un **Réessayer**, **Annuler** garde ce qui était déjà en ligne, et la mise en ligne reste dans la liste comme **Pas tout mis en ligne**.

Si certains fichiers (ou certaines parties) ne sont pas passés, la mise en ligne le dit. **Réessayer** met en ligne seulement ceux qui manquent, dans le même dossier, donc le lien reste le même. **Copier le lien** fonctionne déjà pour les fichiers en ligne. Le **×** fait disparaître le message.

La petite fenêtre de la zone de notification montre aussi les mises en ligne : passez à **Envoi** en haut.

## La liste Mis en ligne {#the-uploaded-list}

Tout ce que vous avez mis en ligne est listé sous **Mis en ligne**, du plus récent au plus ancien :

- **Nom**, le nombre de fichiers, la taille et la date, et **ZIP en 7 parties** (par exemple) pour un ZIP en parties.
- **Mis en ligne sur** : **Gofile** ou **Google Drive**, et quand la mise en ligne se supprime (**Supprimé dans 3 jours**, **Supprimé demain**, **Conservé**…). À côté de **Gofile** : combien de fois on l’a téléchargée (**Pas encore téléchargé**, **Téléchargé une fois**, **Téléchargé 4 fois**). Seul Gofile donne cette information ; Google Drive ne la donne pas, donc les mises en ligne sur Drive n’en affichent aucune. Gofile compte les téléchargements, pas qui a téléchargé. Pour un ZIP en parties, il compte les téléchargements complets (quelqu’un qui prend toutes les parties compte une fois). Dans une fenêtre étroite, le nombre se trouve dans les détails de la mise en ligne.
- **Lien** : cliquez dessus pour ouvrir la page de téléchargement. **Copier le lien** le copie.

**Mis en ligne sur** et **Lien** peuvent être élargies ou rétrécies, ou masquées par un clic droit sur les titres des colonnes (voir [Télécharger](downloading.md#follow-your-downloads)). Dans une fenêtre étroite, le lien s’efface en premier.

Les nombres sont vérifiés quand vous ouvrez **Envoi**, au plus toutes les quelques minutes. La flèche ronde à côté de **Mis en ligne** vérifie à nouveau.

Ce que vous pouvez voir sur une mise en ligne Gofile :

- **Gofile pourrait bientôt le retirer** : une mise en ligne sans date de suppression que personne n’a téléchargée depuis un moment. Gofile peut la retirer après 10 jours sans téléchargement.
- **Plus sur Gofile** : elle a été supprimée ou a expiré, et le lien ne fonctionne plus. **Retirer** l’enlève de la liste.
- **Pas tout mis en ligne** : seuls certains fichiers sont en ligne. Le lien fonctionne pour eux. Tant que Pako réessaie encore, il indique **Mise en ligne**. Pour un ZIP en parties, il faut toutes les parties pour l’ouvrir.

**Supprimer les téléchargés** (à côté de **Mis en ligne**) supprime de Gofile tout ce qui a été téléchargé au moins une fois, après vous avoir demandé.

### Clic droit sur une mise en ligne {#right-click-an-upload}

Faites un clic droit dessus, ou cliquez sur la flèche au bout de sa ligne, pour :

- **Copier le lien**, **Ouvrir la page de téléchargement**, **Afficher les détails**.
- **Garder 7 jours de plus** et **Garder (ne pas supprimer)**, ou **Supprimer dans 7 jours** pour une mise en ligne conservée.
- **Retirer de la liste** : seulement de la liste de Pako. Le fichier reste sur Gofile ou dans votre Drive.
- **Supprimer maintenant de Gofile…** ou **Supprimer maintenant de Google Drive…** : le lien cesse de fonctionner pour tout le monde et le fichier ne peut pas être récupéré (sur Drive, il ne va pas dans la corbeille). Votre copie sur ce PC n’est pas touchée.

## Le panneau de détails {#the-details-panel}

Sélectionnez une mise en ligne et appuyez sur **Ctrl+I** (ou double-cliquez dessus) pour voir ses détails à droite :

- **Taille**, **Fichiers** et **Téléchargements**.
- **Lien** : le lien en code QR (visez-le avec l’appareil photo d’un téléphone pour ouvrir le lien). Le code est créé sur votre PC.
- **Supprimer après** : quand la mise en ligne se supprime et qui s’en charge, avec **Garder 7 jours de plus**, **Garder** ou **Supprimer dans 7 jours**.
- **Fichiers inclus** : les fichiers qu’elle contient, avec leur taille.
- **Déroulement** : quand elle a été mise en ligne, sa **Durée**, la **Vitesse moyenne** et la **Vitesse de pointe**, son **Format** (**Tels quels**, **Un ZIP**, **ZIP compressé**, **ZIP en 7 parties**, ou **Tel quel** pour un fichier seul), et où (**Mis en ligne sur**).
- **Téléchargements par d’autres** (Gofile) : un petit graphique des téléchargements au fil du temps, et à peu près quand a eu lieu le dernier.
- **Note** : pour qui, ce qu’elle contient. Seulement sur ce PC.
- En bas : **Copier le lien**, **Ouvrir la page** (la page de téléchargement dans votre navigateur) et **Supprimer maintenant…**.

## Votre Google Drive {#your-google-drive}

Pour mettre en ligne dans votre propre Drive, connectez-le une fois : cliquez sur **Connecter Google Drive** dans **Paramètres › Comptes** (sous **Google Drive pour les mises en ligne**), ou sur la barre qui s’affiche dans **Envoi** quand **Google Drive** est choisi. La page de Google s’ouvre dans votre navigateur : connectez-vous et autorisez l’accès. Si la page de Google affiche une case pour Google Drive, cochez-la avant de continuer. Sans elle, Pako ne se connecte pas et indique « Google n’a pas donné à Pako l’autorisation d’ajouter des fichiers à votre Google Drive. Connectez-vous à nouveau et, sur la page de Google, cochez la case de Google Drive avant de continuer. »

- Pako ne voit que les fichiers qu’il met en ligne, rien d’autre dans votre Drive.
- Le connecter fait de **Google Drive** le choix habituel pour les mises en ligne. Vous pouvez toujours choisir **Gofile** pour n’importe quelle mise en ligne.
- Cette connexion ne sert qu’aux mises en ligne. Les téléchargements depuis Drive n’en ont jamais besoin.
- **Déconnecter** (dans **Paramètres › Comptes**) revient à Gofile. Ce que vous avez mis en ligne reste dans votre Drive.

### La barre Drive {#the-drive-bar}

Avec **Google Drive** choisi, une barre au-dessus de la liste montre la place qui reste : « 120 Go libres dans votre Google Drive » (par exemple), la place utilisée, et celle que prennent les mises en ligne de Pako (« Mises en ligne de Pako : 3 · 2,4 Go »). La flèche ronde vérifie à nouveau.

Avant une mise en ligne, **Nouvelle mise en ligne** indique la place qu’il faut et celle qui est libre. Quand ça ne tient pas, Pako vous demande d’abord :

- **Faire de la place…** liste vos mises en ligne dans votre Drive, de la plus ancienne à la plus récente, celles de plus d’une semaine étant cochées. **Supprimer de Google Drive** supprime pour de bon celles qui sont cochées. Vos copies sur ce PC ne sont pas touchées.
- **Mettre plutôt en ligne sur Gofile** passe à Gofile et met en ligne là-bas.

Si des mises en ligne de votre liste sont dans le Drive d’un autre compte Google, la barre le dit : connectez ce compte pour les supprimer, ou pour laisser Pako les supprimer à leur date.

## Bon à savoir {#good-to-know}

- Avec la fenêtre de Pako ouverte, une mise en ligne terminée affiche un message en bas de la fenêtre. Sinon, c’est un pop-up (voir [Notifications](notifications.md)).
- Si Pako se ferme pendant que plusieurs fichiers sont mis en ligne **Tels quels**, ce qui était déjà en ligne reste dans la liste comme **Pas tout mis en ligne**. Un ZIP en parties revient lui aussi sous **Mise en ligne en cours** et continue avec les parties restantes (sur Gofile, une fois sa première partie en ligne), ou attend **Réessayer** si **Reprendre les mises en ligne inachevées au démarrage de Pako** est désactivé. Un ZIP ou un fichier seul qui n’a pas fini n’est pas gardé.
- Votre liste de mises en ligne et vos notes restent sur votre PC.
- Vous ne créez pas de compte Gofile, mais Gofile en crée un compte invité à votre première mise en ligne. Pako garde sa clé sur votre PC (chiffrée), pour lire les nombres de téléchargements et pour supprimer vos mises en ligne.
- Pour Google Drive, Pako garde votre connexion sur votre PC (chiffrée) et lit l’adresse e-mail de votre compte, pour montrer quel compte est connecté.
