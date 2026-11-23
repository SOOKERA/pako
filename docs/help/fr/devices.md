# Mes appareils

Associez une fois vos propres PC, comme votre PC de bureau et votre portable. Ensuite, depuis la page **Mes appareils** :

- envoyez des fichiers entre eux sans que personne n’appuie sur Accepter ;
- voyez en direct les téléchargements de l’autre PC, mettez-les en pause, reprenez-les, changez leur priorité ou retirez-les, et récupérez-en un qui est terminé ;
- envoyez des liens depuis un PC pour que l’autre les télécharge, avec la qualité vidéo et la priorité que vous choisissez, et suivez leur avancement.

**Direct** reste pour les autres personnes. **Mes appareils** est seulement pour vos propres PC.

## Comment ça marche {#how-it-works}

- Mes appareils utilise la même connexion que [Direct](direct.md) : les fichiers et les messages vont de PC à PC, chiffrés, et rien ne passe par un serveur de Sookera.
- Les deux Pako doivent être ouverts en même temps (dans la zone de notification, ça compte). Les ordres et les fichiers ne peuvent pas attendre un PC éteint.
- Sur le même Wi-Fi ou réseau domestique, vos PC se trouvent sans internet et les fichiers passent par ce réseau, en général bien plus vite. Le point de l’appareil devient bleu et il indique **Réseau local**.
- Ouvrir **Mes appareils** démarre Direct s’il n’était pas encore lancé, comme ouvrir la page Direct.
- Un appareil n’apparaît pas parmi les contacts de Direct.

## Associer un PC {#pair-a-pc}

Faites-le une fois pour chaque paire de PC.

1. Sur le premier PC, ouvrez **Mes appareils** et cliquez sur **Ajouter un appareil** (ou **+ Ajouter**). Il affiche l’**ID Pako de ce PC** : cliquez sur **Copier**, et faites passer l’ID sur votre autre PC (un e-mail à vous-même, un message, une clé USB).
2. Sur l’autre PC, ouvrez **Mes appareils** › **Ajouter un appareil**, collez-le sous **ID Pako de l’autre PC**, donnez-lui un nom (comme **PC bureau**) et vérifiez **Ses fichiers sont enregistrés dans** (voir plus bas). Cliquez sur **Associer cet appareil**.
3. Le premier PC affiche « … veut s’associer à ce PC comme l’un de vos appareils », avec un **code de sécurité**. L’autre PC affiche le même code sous son nom. Si les codes sont identiques, vérifiez où vont ses fichiers et cliquez sur **C’est mon appareil**. S’ils ne sont pas identiques, cliquez sur **Refuser**.

   La carte indique aussi ce que l’association permet, et vous prévient si l’un de vos appareils ou de vos contacts porte déjà ce nom : si vous ne venez pas d’ajouter ce PC sur votre autre PC, refusez. Si ce PC n’a pas encore de nom, la carte vous le demande.

Vous pouvez aussi saisir les ID dans l’autre sens : quel que soit le PC qui saisit l’ID de l’autre, c’est l’autre qui vous demande de confirmer. Pako vous demande d’abord un nom pour ce PC s’il n’en a pas encore (votre autre PC le voit).

## Le nom de ce PC {#this-pcs-name}

En bas de la liste des appareils, sous **Ce PC**, se trouve le nom que vos autres PC voient pour celui-ci, comme **PC bureau**. Il ne sert qu’à vos appareils : le nom que voient vos contacts dans Direct (votre nom sous **Vous, qui envoyez depuis**) ne change pas.

- Pour renommer ce PC, cliquez sur son nom, saisissez le nouveau et appuyez sur **Entrée** (ou cliquez ailleurs). **Échap** garde l’ancien nom.
- Vos autres PC voient le nouveau nom aussitôt s’ils sont connectés, sinon à leur prochaine connexion.
- Si vous avez renommé vous-même un appareil sur un PC (**Renommer** dans ses **Paramètres**), ce PC garde votre nom pour lui.
- Tant que ce PC n’a pas de nom, il indique **Nommer ce PC**, et l’association en demande un.

Si l’un de vos contacts Direct est en fait votre propre PC, ajoutez son ID sous **Ajouter un appareil** sur les deux PC : il passe dans **Mes appareils**.

## Où sont enregistrés les fichiers d’un appareil {#where-files-from-a-device-are-saved}

Chaque appareil a son propre dossier. Il est choisi au moment de l’association : par défaut `<votre dossier de téléchargement>\De <nom de l’appareil>`, et **Changer…** en choisit un autre.

**Les fichiers de cet appareil sont enregistrés ici, et seulement ici.** L’autre PC ne choisit jamais un emplacement sur ce PC : il ne peut écrire nulle part ailleurs, ne peut pas voir vos dossiers, et les noms qui essaient de sortir du dossier sont refusés, comme dans Direct. Vous pouvez changer le dossier plus tard dans les **Paramètres** de l’appareil.

## Envoyer des fichiers entre vos appareils {#send-files-between-your-devices}

Choisissez l’appareil à gauche. Sa page a quatre onglets : **Fichiers**, **Liens**, **Téléchargements** et **Paramètres**.

Son onglet **Fichiers** montre tout ce qui a été envoyé entre les deux PC, avec l’avancement, la vitesse, et le chemin pris (**Réseau local**, **Direct** ou **Via relais**).

- **Déposez des fichiers ou des dossiers** n’importe où sur la page de l’appareil, ou sur l’appareil dans la liste, ou cliquez sur **Choisir des fichiers** / **Choisir un dossier**. Ils partent aussitôt.
- L’autre PC ne demande rien : les fichiers de vos propres appareils arrivent tout seuls, dans le dossier de cet appareil. (Si un envoi est très gros pour le disque là-bas, il demande quand même.)
- **Annuler** arrête un fichier en route. **Ouvrir** et **Dossier** montrent ce que vous avez reçu. Les programmes et les scripts ne sont jamais lancés depuis Pako : ils sont montrés dans leur dossier.
- La flèche sur une carte de plusieurs fichiers ou dossiers déplie ce qu’elle contient, chacun avec sa taille (et **Ouvrir** / **Dossier** sur chacun de ceux que vous avez reçus).
- Si la connexion coupe (un PC se met en veille, le Wi-Fi tombe, Pako est fermé), le fichier attend : « En attente que PC maison reprenne · déjà 62% ». Il reprend tout seul là où il s’était arrêté quand les deux PC sont de nouveau connectés, comme dans [Direct](direct.md#your-history-with-someone). **Annuler** supprime ce qui était arrivé ; après 7 jours, il est effacé.

## Voir et contrôler les téléchargements de l’autre PC {#see-and-control-the-other-pcs-downloads}

Ouvrez l’onglet **Téléchargements** de l’appareil. Tant qu’il est ouvert et que l’appareil est en ligne, vous voyez ses téléchargements en direct : nom, taille, avancement, vitesse et état.

- **Pause** et **Reprendre** agissent sur le téléchargement de ce PC.
- Le bouton de priorité (une flèche, ou un tiret pour Normale) change sa priorité là-bas : **Priorité : D’abord**, **Normale** ou **Plus tard**, comme dans Téléchargements. Un téléchargement réglé sur D’abord ou Plus tard l’indique sur sa ligne. Si ce PC a un Pako plus ancien, il répond qu’il ne connaît pas cet ordre : mettez Pako à jour là-bas.
- **×** le retire de la liste de ce PC. Un fichier terminé reste sur ce PC.
- **Récupérer ce fichier** (téléchargements terminés seulement) envoie le fichier sur ce PC, comme un envoi normal, dans le dossier de cet appareil ici.

## Envoyer des liens à télécharger là-bas {#send-links-to-download-there}

L’onglet **Liens** de l’appareil montre les liens que ce PC lui a envoyés à télécharger, une carte par envoi, le plus récent en bas. Les envois depuis la page **Liens** et depuis la fenêtre **Télécharger avec Pako** y ont aussi une carte.

- Chaque carte indique combien de liens, leur priorité et leur qualité, quand ils ont été envoyés et où ils vont : « 3 liens · Normale · La meilleure jusqu’à 1080p », « Envoyé aujourd’hui 14:02 · dans Téléchargements ».
- Tant que ce PC est en ligne, la ligne suivante indique leur avancement là-bas, d’après sa liste de téléchargements : « 2 terminés · 1 en cours 64% ». Un PC avec un Pako plus ancien ne dit pas quels téléchargements il a créés : ses cartes indiquent seulement **Envoyé**.
- **Afficher les … liens** les liste, avec les mêmes boutons que l’onglet **Téléchargements** (pause, reprise, priorité, retrait, **Récupérer ce fichier**).
- Ce PC garde les 200 derniers envois pour chaque appareil.

Pour en envoyer d’autres, utilisez la zone en bas :

1. Collez un ou plusieurs liens dans **Collez des liens à télécharger par PC maison** (par exemple).
2. Choisissez la **Qualité** des vidéos (**La meilleure jusqu’à 1080p** par défaut, comme dans **Paramètres › Téléchargements**, une hauteur, ou **Audio seul**) et la **Priorité** (**D’abord**, **Normale** ou **Plus tard**).
3. **Dans le dossier** est facultatif : un nom de dossier, créé dans le propre dossier de téléchargement de ce PC (par exemple **Cours**, ou **Cours/Semaine 3** ; `\` fonctionne comme `/`). Un nom qui ne peut pas être utilisé là-bas (avec `..`, un lecteur ou `:`, ou plus de deux niveaux) n’est pas envoyé : la ligne sous la zone dit pourquoi.
4. Cliquez sur **Envoyer** (ou appuyez sur **Entrée**).

Les fichiers et les vidéos seules partent aussitôt. La ligne sous la zone dit ce qui s’est passé là-bas (« 2 liens ajoutés aux téléchargements sur PC maison. »), ou pourquoi ça n’a pas marché.

Une **playlist**, un **dossier Google Drive** ou une **page de lien** que Pako doit parcourir n’est jamais envoyé tel quel : ce PC l’ouvre d’abord, dans une carte de l’onglet (« Lecture de la playlist… »). Ensuite, la carte liste ses vidéos ou ses fichiers avec des cases à cocher, chaque vidéo avec sa propre qualité (celle de la zone au départ). Décochez ce que vous ne voulez pas et cliquez sur **Envoyer 5 à PC maison** (par exemple), ou **Annuler**. Chaque vidéo ou fichier coché part comme un lien à part, dans un dossier au nom de la playlist ou du dossier Drive là-bas (ou **Dans le dossier**, si vous l’avez rempli). S’il ne peut pas être ouvert, la carte dit pourquoi. Si ce PC n’en prend pas certains, ils restent dans la carte avec la raison, pour être envoyés à nouveau.

Seuls les liens web (http et https) peuvent être envoyés. Tout le reste est refusé. Les liens vers le PC lui-même (localhost) sont refusés aussi. Seul le lien part, avec sa qualité, sa priorité et son nom de dossier : jamais la connexion ni les cookies de votre navigateur.

## Ce que l’autre PC en fait {#what-the-other-pc-does-with-them}

- **Une vidéo** de YouTube ou d’un autre site de vidéos se télécharge aussitôt, dans la qualité choisie, ou la plus proche en dessous qu’a cette vidéo, comme un téléchargement de vidéo normal. La fenêtre **Télécharger avec Pako** ne s’ouvre pas là-bas, et la vidéo n’attend pas dans **Liens**.
- **Un fichier** va directement dans ses téléchargements, avec sa priorité.
- Tout va dans le propre dossier de téléchargement de ce PC, ou dans le dossier nommé pour l’envoi **à l’intérieur** de ce dossier. **L’autre PC ne peut ajouter un dossier que dans son dossier de téléchargement** : jamais ailleurs. Un nom de dossier est nettoyé comme un nom de fichier (un nom réservé de Windows comme `CON` reçoit un `_` devant). Un nom qui n’est pas sûr (comme `..`, `C:\`, un chemin réseau, ou plus de deux niveaux) est refusé par le PC qui l’envoie ; si l’un d’eux arrive quand même, il n’est pas utilisé : ces fichiers vont directement dans le dossier de téléchargement, et son historique le dit. Un fichier qui porte le même nom qu’un fichier déjà présent reçoit un nouveau nom, comme d’habitude ; rien n’est écrasé.
- Une vidéo dont le site tarde à répondre peut encore être en cours de lecture après la réponse de ce PC : elle compte comme prise. Si elle ne peut ensuite pas être téléchargée, elle reste là-bas comme téléchargement en échec (la carte ici indique **en échec** ; **Reprendre** réessaie).
- Beaucoup de liens à la fois (un grand dossier Drive) : ce que ce PC n’a pas pu ajouter en 40 secondes environ revient comme non accepté (« Le Pako de là-bas était encore occupé à ajouter les autres : envoyez-le à nouveau »), donc rien n’est ajouté deux fois.
- Un lien venant d’un PC avec un Pako plus ancien fonctionne comme avant : la page d’une vidéo ou une page de lien va d’abord dans **Liens** sur ce PC.

## Télécharger sur un autre PC depuis Liens {#download-on-another-pc-from-links}

Les liens que vous avez sur ce PC peuvent être téléchargés par un autre de vos PC. De deux façons :

- **Pour une fois**, avec **Télécharger sur** à côté de **Télécharger** : dans **Liens**, le bouton avec l’icône de PC dans la barre d’outils, la petite flèche à côté de **Télécharger** dans les détails d’un lien, ou clic droit › **Télécharger sur …** ; dans la fenêtre **Télécharger avec Pako**, la petite flèche à côté de **Télécharger** (voir [Extension de navigateur](browser-extension.md#the-download-with-pako-window)). Elle liste **Ce PC** et chacun de vos appareils qui est en ligne et qui permet à ce PC de le contrôler ; quand il n’y en a aucun, elle n’est pas affichée. Le choix ne vaut que pour cette fois, quel que soit le réglage du lien lui-même.
- **Pour un lien, retenu** : sélectionnez-le dans **Liens** et réglez **Télécharger sur** dans ses détails (voir [Liens et petite fenêtre](links-and-tray.md#download-on-another-pc)). Ensuite, **Télécharger** l’envoie à ce PC.

Chaque lien part avec :

- sa **qualité**, pour une vidéo : celle choisie sur sa ligne dans **Liens** (ou **Audio seul**), celle choisie dans la fenêtre **Télécharger avec Pako**, ou sinon celle par défaut de ce PC (**Paramètres › Téléchargements**) ;
- sa **priorité** : celle réglée sur sa ligne, ou **D’abord** avec **Télécharger d’abord** (et depuis la fenêtre **Télécharger avec Pako**, comme son **Lancer maintenant**) ;
- pour une vidéo d’une playlist ou un fichier d’un dossier Drive ouvert dans **Liens** : le nom de la playlist ou du dossier, pour qu’il arrive dans un dossier de ce nom là-bas.

Une playlist ouverte dans **Liens** est envoyée vidéo par vidéo (seulement celles qui sont cochées), et un dossier Drive fichier par fichier. Une playlist dont les vidéos ne sont pas encore listées, ou une page de lien que Pako n’a pas encore ouverte, n’est pas envoyée : elle reste dans **Liens** avec une note sur sa ligne (« Une playlist : ouvrez-la d’abord dans Liens pour choisir les vidéos. »). Un fichier proposé par plusieurs hébergeurs est envoyé avec l’hébergeur qu’il utiliserait ici.

Une fois que ce PC les a pris, ils quittent **Liens** ici, et un message le dit (« 2 liens ajoutés aux téléchargements sur PC maison. »). Un lien qu’il n’a pas pu prendre, ou des liens pour un PC hors ligne, restent ici avec une note sur la ligne qui dit pourquoi. Un lien qui ne fonctionne que si vous êtes connecté sur ce PC peut ne pas fonctionner là-bas : seul le lien part, jamais la connexion de votre navigateur.

## … peut contrôler mon Pako {#this-device-can-control-my-pako}

Dans l’onglet **Paramètres** de l’appareil, **… peut contrôler mon Pako** indique si ce PC peut voir les téléchargements de ce PC-ci et donner les ordres décrits plus haut. C’est activé à l’association, et vous pouvez le désactiver pour chaque appareil. Une fois désactivé, son tout prochain ordre est refusé et il ne voit plus vos téléchargements. Les fichiers entre les deux PC passent toujours sans demander.

Ce qu’un appareil que vous autorisez peut faire ici, et rien d’autre :

- voir la liste des téléchargements de ce PC (jamais ses téléchargements privés venant de la fenêtre privée d’un navigateur : un appareil ne peut ni les voir, ni les changer, ni les récupérer, et un lien qu’il envoie ne rejoint jamais l’un d’eux, même pour la même vidéo) ;
- ajouter des liens web aux téléchargements, avec une qualité vidéo et une priorité, dans le propre dossier de téléchargement de ce PC ou un dossier qu’il nomme à l’intérieur ;
- mettre en pause, reprendre, changer la priorité d’un téléchargement, ou le retirer de la liste ;
- récupérer un téléchargement terminé de cette liste.

Il ne peut jamais ouvrir ni lancer quoi que ce soit sur ce PC, choisir un dossier, ou atteindre un fichier qui n’est pas un téléchargement terminé de votre liste. Si l’autre PC n’autorise pas ce PC, son onglet **Téléchargements** le dit et explique comment l’activer là-bas.

La même autorisation permet à l’autre PC d’envoyer des fichiers à vos contacts Direct en passant par celui-ci (voir [Envoyer depuis un autre de vos PC](direct.md#send-from-another-of-your-pcs)) : il peut voir les contacts Direct de ce PC et votre historique avec eux, et envoyer des fichiers que ce PC propose ensuite à l’un d’eux comme un envoi de sa part. Seuls les fichiers qu’il envoie dans cet envoi sont transmis ; ils sont gardés entre-temps dans les propres données de Pako ici (jamais dans vos dossiers), et supprimés une fois livrés (ou gardés dans le dossier de cet appareil, s’il l’a demandé), refusés, ou après 7 jours. Désactiver cette autorisation (ou retirer l’appareil) arrête aussitôt ce qu’il a demandé à ce PC de transmettre, et supprime ces fichiers.

## Historique {#history}

L’onglet **Paramètres** de l’appareil liste ce que les deux PC ont fait l’un avec l’autre : liens ajoutés, téléchargements mis en pause, retirés ou dont la priorité a changé, fichiers demandés, ordres refusés, changements d’autorisation, et fichiers envoyés à un contact Direct en passant par l’autre PC (« 3 fichiers transmis à Rado · livré »), avec qui l’a fait et quand. Il reste sur ce PC. **Effacer** le supprime de ce PC.

Pendant que des fichiers vont à un contact Direct en passant par un appareil, la ligne de cet appareil dans la liste le dit : « Relais de 3 fichiers vers Rado ». Dans son onglet **Fichiers**, cet envoi indique pour qui il est (« pour Rado »).

## Renommer ou retirer un appareil {#rename-or-remove-a-device}

Dans l’onglet **Paramètres** de l’appareil : **Renommer** change le nom que vous voyez sur ce PC. **Retirer cet appareil** met fin à l’association : il ne peut plus envoyer sans demander ni contrôler ce Pako, et la connexion se ferme. Les fichiers déjà reçus restent. Pour l’associer à nouveau plus tard, ajoutez-le à nouveau sous **Ajouter un appareil** sur ce PC ; l’autre PC confirme. Si l’autre PC le montre encore comme l’un de ses appareils, retirez-le d’abord là-bas aussi, puis ajoutez-le à nouveau là-bas.

## Questions {#questions}

**L’appareil indique « Confirmez sur l’autre PC ».**
L’association demande un oui sur les deux PC. Ouvrez **Mes appareils** sur l’autre PC : il vous le demande là-bas. Les deux Pako doivent être ouverts.

**L’onglet Téléchargements indique que l’autre PC ne permet pas à ce PC de voir ses téléchargements.**
Sur l’autre PC, ouvrez **Mes appareils**, choisissez ce PC, puis **Paramètres**, et activez **… peut contrôler mon Pako**.

**Un contact de Direct peut-il contrôler mon Pako ?**
Non. Seul un PC que les deux côtés ont associé comme « mon appareil », et que vous autorisez, peut voir vos téléchargements ou donner des ordres. Les contacts peuvent seulement vous proposer des fichiers, que vous acceptez ou refusez.

**L’un de mes contacts Direct dit que ce PC est l’un de ses appareils.**
Ne cliquez sur **C’est mon appareil** que si ce contact est vraiment votre propre PC : une fois associé, il peut envoyer des fichiers ici sans demander, voir les téléchargements de ce PC et en ajouter, récupérer ceux qui sont terminés, voir vos contacts Direct et votre historique avec eux, et leur envoyer des fichiers via ce PC. **Pas maintenant** cache la question jusqu’au redémarrage de Pako.

**L’appareil reste sur « Connecté · pas encore confirmé sur celui-ci ».**
L’autre PC a un Pako plus ancien qui ne connaît pas Mes appareils : mettez Pako à jour là-bas, puis ajoutez ce PC là-bas sous **Ajouter un appareil**.
