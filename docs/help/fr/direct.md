# Direct

Envoyez des fichiers directement de votre PC au PC d’une personne que vous connaissez, comme une messagerie pour les fichiers. Vous avez besoin de Pako tous les deux.

Pour vos propres PC (votre PC de bureau et votre portable, par exemple), utilisez plutôt [Mes appareils](devices.md) : les fichiers passent sans Accepter, et vous pouvez voir et lancer des téléchargements sur l’autre PC.

## Comment ça marche {#how-it-works}

- Les fichiers vont d’un Pako à l’autre, chiffrés. Rien n’est mis en ligne ni stocké nulle part entre les deux.
- Les deux Pako doivent être ouverts en même temps (dans la zone de notification, ça compte). Les fichiers ne peuvent pas attendre quelqu’un qui est hors ligne.
- Vos contacts et votre historique restent sur votre PC.
- Rien ne passe par un serveur de Sookera. Pour se trouver, les Pako utilisent un annuaire d’adresses public et, quand les deux PC ne peuvent pas se joindre directement, un relais public. Les deux sont gérés par les créateurs de la bibliothèque de connexion qu’utilise Pako. Le relais ne fait que transmettre des données chiffrées.
- Direct démarre la première fois que vous ouvrez la page **Direct** (elle indique « Direct démarre… » un instant), puis avec Pako à partir de là. Avant cela, Pako ne fait aucune connexion Direct. Si vous utilisiez Direct dans un Pako plus ancien, il démarre avec Pako comme avant.
- Une fois lancé, votre Pako se connecte à un relais et inscrit son ID Pako dans l’annuaire, pour que les autres puissent le trouver, même avant que vous ayez des contacts : une personne à qui vous avez donné votre ID peut demander à se connecter. Votre adresse internet n’y est pas inscrite. Pako peut aussi demander à votre box d’ouvrir un port, pour que deux PC puissent se connecter directement.
- Sur le même réseau (le même Wi-Fi ou réseau domestique), les Pako se trouvent sans internet, et les fichiers vont de PC à PC par ce réseau : voir [Sur le même Wi-Fi](#on-the-same-wi-fi).
- Vos contacts peuvent savoir quand votre Pako est ouvert : ils vous voient en ligne, et savent quand ils vous ont vu pour la dernière fois.

## Votre ID Pako et votre nom {#your-pako-id-and-your-name}

En haut de la liste des contacts dans **Direct**, sous **Vous, qui envoyez depuis**, se trouvent votre nom et ce PC sur un seul bouton (« Sam · Ce PC »), et votre **ID Pako** en dessous. (Avant que vous ayez des contacts, l’ID est au milieu de la page.)

Chaque Pako a son propre **ID Pako**, un long code. **Copier l’ID** le copie : envoyez-le aux personnes avec qui vous voulez vous connecter.

**Votre nom** est ce que voient vos contacts. Cliquez dessus pour le changer, saisissez le nouveau et appuyez sur **Entrée** (ou cliquez ailleurs) ; **Échap** garde l’ancien. Vos contacts peuvent toujours vous donner leur propre nom pour vous. Pako vous demande votre nom une fois, avant votre premier contact : quand vous ajoutez quelqu’un, ou quand vous acceptez quelqu’un qui veut se connecter.

La partie PC du bouton vous permet d’envoyer depuis un autre de vos PC : voir [Envoyer depuis un autre de vos PC](#send-from-another-of-your-pcs).

## Ajouter quelqu’un {#add-someone}

1. Demandez-lui son ID Pako, ou envoyez-lui le vôtre.
2. Cliquez sur **Ajouter** (ou **Ajouter un contact**), collez son ID sous **Son ID Pako**, saisissez un nom sous **Son nom** (comment vous l’appelez) et cliquez sur **Ajouter un contact**.
3. Pako la contacte. La première fois, on lui demande d’accepter. Pendant ce temps, elle affiche « En attente de son accord ». Si elle ne répond pas, la ligne indique « Pas encore de réponse. Pako redemandera plus tard. »

Si elle refuse, sa ligne indique « Votre demande a été refusée. Vous pourrez redemander dans 7 jours. » Pendant 7 jours, son Pako vous refuse sans lui demander ; ensuite, **Connecter** redemande.

Vous ne faites cela qu’une fois. Ensuite, vous vous connectez tout seuls dès que vos deux Pako sont ouverts.

### La première fois : Accepter et le code de sécurité {#the-first-time-accept-and-the-safety-code}

Quand quelqu’un vous ajoute, Pako demande : « Rado veut se connecter » (par exemple). Il affiche un **Code de sécurité**. La personne qui vous a ajouté voit le même code au-dessus de votre conversation.

- Si vous la connaissez, vérifiez une fois que vous voyez tous les deux le même code (de vive voix ou par message). Un code différent veut dire que ce n’est pas elle.
- Le nom est celui que la personne a choisi : Pako ne peut pas le vérifier. Si vous avez déjà un contact de ce nom, Pako vous prévient que c’est un autre Pako.
- **Accepter** l’ajoute à vos contacts. **Refuser** la refuse. Fermer la question avec son **×** (son info-bulle indique **Refuser**) ou **Échap** refuse aussi.

Pendant 7 jours, Pako refuse sans vous demander une personne que vous avez refusée ou retirée. Ensuite, si elle vous joint à nouveau, on vous demande comme la première fois.

## Vos contacts {#your-contacts}

La liste des contacts montre chaque personne avec un point sur son image :

- **Gris** : hors ligne. Sous le nom : quand vous l’avez vue pour la dernière fois. Avec beaucoup de contacts, cela peut aussi vouloir dire « pas connecté en ce moment » (voir plus bas) : la ligne indique alors seulement « Vu à l’instant » ou « Vu il y a 20 min », sans « Hors ligne ». « Pas vérifié récemment » veut dire que Pako ne l’appelle plus de lui-même (voir [Vérifier ce contact](#check-this-contact)).
- **Vert** : en ligne, et comment vous êtes connectés (voir [Comment vous êtes connectés](#how-youre-connected)).
- **Bleu** : en ligne sur le même réseau que vous (« Réseau local »).
- **Orange** : en attente de son accord pour vous.

Le point change de couleur tout seul, par exemple quand quelqu’un rejoint ou quitte votre Wi-Fi.

Ce qui attend *votre* réponse est en corail, pas en orange : un **!** sur le contact, le point à côté de **Vous attendent** à gauche, et le nombre sur l’onglet **Direct**.

Les personnes en ligne viennent d’abord, puis celles dont vous attendez l’accord, puis les autres, par nom. Quand il y a beaucoup de contacts, **Chercher dans les contacts** les trouve par nom ou par étiquette.

Pako essaie de joindre vos contacts de lui-même toutes les quelques minutes (pas quelqu’un qui vous a refusé : seul **Connecter** lui redemande ; voir aussi [Vérifier ce contact](#check-this-contact)). **Connecter** (au-dessus de la conversation, **Vérifier maintenant** pour quelqu’un qui n’a pas été vérifié récemment) essaie tout de suite. Un contact qui redémarre son Pako est de nouveau en ligne aussitôt.

Avec une longue liste de contacts, Pako reste léger :

- Il essaie quelques contacts à la fois, pas tous en même temps. Quelqu’un qui ne répond pas est essayé de moins en moins souvent (jusqu’à une fois par heure, mais toujours toutes les quelques minutes tant qu’un transfert avec lui attend de reprendre), et de nouveau tout de suite quand il revient, apparaît sur votre réseau, ou quand votre internet revient.
- Quand beaucoup de contacts sont en ligne, une connexion où rien ne s’est passé pendant environ 10 minutes se ferme. Son point devient gris avec « Vu à l’instant », et ouvrir sa conversation ou lui envoyer des fichiers reconnecte aussitôt. Il peut toujours vous envoyer des fichiers entre-temps. Vos appareils, les transferts et la conversation ouverte restent connectés. Les contacts vérifiés **Souvent**, les favoris, les personnes qui peuvent vous envoyer sans demander et celles avec qui vous avez échangé ces 7 derniers jours restent connectés aussi, jusqu’à 64 connexions ouvertes.

### Vérifier ce contact {#check-this-contact}

Dans le panneau de détails, **Vérifier ce contact** dit comment Pako appelle quelqu’un de lui-même :

- **Automatique** (par défaut) : appelé de moins en moins souvent tant qu’il ne répond pas. Après **30 jours sans rien de nouveau** (aucun fichier dans un sens ou dans l’autre, aucun envoi de votre part, aucun appel de sa part, aucun **Vérifier maintenant**), Pako arrête de l’appeler de lui-même. Les favoris et les personnes qui peuvent vous envoyer sans demander ne s’arrêtent jamais, et sont appelés avant les autres. Vos appareils sont toujours connectés.
- **Souvent…** : choisissez **1 heure**, **Aujourd’hui** (jusqu’à minuit) ou **1 semaine** en dessous. Jusque-là, il est appelé toutes les quelques minutes, en premier, même s’il ne répond pas, et sa connexion reste ouverte même sans activité (jusqu’à 64 connexions ouvertes ; si son Pako la ferme, il est rappelé dans l’heure, ou aussitôt quand vous ouvrez sa conversation ou lui envoyez des fichiers). Le panneau indique jusqu’à quand, comme « Vérifié souvent jusqu’à 18:40 », puis il revient tout seul à ce qu’il était avant : **Manuel** pour un contact réglé sur **Manuel** (« Vérifié souvent jusqu’à 18:40, puis retour à Manuel »), sinon **Automatique**. **Vérifier souvent pendant 1 heure** dans le menu du clic droit fait la même chose.
- **Manuel** : jamais appelé de lui-même.

Un contact que Pako n’appelle plus de lui-même, et qui n’est pas connecté, a un point gris et « Pas vérifié récemment ». **Vérifier maintenant** (dans le panneau de détails, le menu du clic droit, ou au-dessus de la conversation) l’appelle aussitôt, et avec **Automatique** il est de nouveau appelé de lui-même pendant 30 jours. Ouvrir sa conversation ou lui envoyer des fichiers le connecte aussi.

Quoi que vous choisissiez, il peut toujours vous joindre (cela compte aussi comme quelque chose de nouveau), et un transfert avec lui qui attend de reprendre est toujours essayé toutes les 30 secondes pendant sa première heure.

### Filtres et étiquettes à gauche {#filters-and-tags-on-the-left}

- **Tous les contacts**, **En ligne**, **Vous attendent** (des fichiers attendent votre réponse) et **Favoris**.
- **Étiquettes** : vos propres groupes, comme Amis, Famille ou Travail. **Nouvelle étiquette** en crée une. Faites un clic droit sur une étiquette pour la renommer ou la supprimer (supprimer une étiquette garde les contacts).

Les étiquettes restent sur votre PC : vos contacts ne les voient jamais.

### Renommer, favoris, étiquettes {#rename-favourites-tags}

Faites un clic droit sur un contact, ou cliquez sur l’engrenage au-dessus de la conversation, pour :

- **Renommer…** : votre propre nom pour la personne. Si elle se nomme autrement, cela s’affiche à côté de son nom (« se fait appeler … »). Quand elle change de nom, les contacts connectés le voient aussitôt. Un contact que vous n’avez jamais renommé prend son nouveau nom.
- **Étiquettes…** : cochez ses étiquettes, ou saisissez-en une nouvelle et cliquez sur **Ajouter**, puis **Enregistrer**.
- **Ajouter aux favoris** / **Retirer des favoris** : les favoris ont une étoile et leur propre filtre.
- **Mettre en sourdine** / **Retirer la sourdine** : aucun pop-up, aucun son ni aucune ligne dans la cloche de sa part (fichiers qui arrivent, fichiers qui attendent votre réponse, fichiers reçus), et pas de pop-up **Fichiers livrés** quand vos fichiers lui arrivent. Ses fichiers arrivent quand même : un envoi qui attend votre réponse reste dans sa conversation (un **!** sur sa ligne, et le nombre sur **Vous attendent**), et Pako vous demande quand vous l’ouvrez. Un contact en sourdine a une cloche barrée à côté de son nom. Vos appareils ne peuvent pas être mis en sourdine : associer un contact en sourdine comme l’un de vos appareils lui retire la sourdine.
- **Vérifier maintenant**, pour quelqu’un « Pas vérifié récemment », et **Vérifier souvent pendant 1 heure** (voir [Vérifier ce contact](#check-this-contact)).
- **Peut m’envoyer sans demander** (voir plus bas).
- **Enregistrer ses fichiers dans…** : où vont ses fichiers.
- **Effacer l’historique…** (Pako demande d’abord) : vide votre historique avec la personne sur ce PC. Il ne peut pas être récupéré.
- **Retirer le contact** (Pako demande d’abord) : vous n’êtes plus connectés, un transfert avec la personne s’arrête, et pendant 7 jours Pako la refuse sans vous demander. Les fichiers déjà reçus restent sur votre PC.

## Envoyer des fichiers {#send-files}

1. Choisissez la personne à gauche.
2. Déposez des fichiers ou des dossiers sur la page, ou cliquez sur **Choisir des fichiers** ou **Choisir un dossier**. Ils s’ajoutent en bas et restent avec cette personne, comme un brouillon : un autre contact montre les siens. Retirez-en un avec son **×**, ou tous avec **Vider**.
3. Choisissez **Tels quels**, ou **ZIP compressé** pour les mettre d’abord dans un seul fichier ZIP plus petit. **Nom (facultatif)** lui donne un nom.
4. Cliquez sur **Envoyer à Rado** (avec son nom).

On lui demande s’il veut les fichiers, sauf si vous pouvez lui envoyer sans demander. Pendant ce temps, votre conversation affiche « En attente de son accord… ». Une fois accepté, les fichiers passent directement.

Pako joue un son quand vos fichiers sont livrés, et un carillon quand des fichiers arrivent pour vous ou qu’on vous en propose (voir [Notifications](notifications.md#sounds-files-arriving-files-sent)). Un pop-up **Fichiers livrés** le dit aussi (« Rado a reçu Photos de vacances. »), et des pop-ups vous préviennent quand on vous propose des fichiers ou qu’ils commencent à arriver (voir [Notifications](notifications.md)).

Les fichiers ne partent que quand vous êtes tous les deux en ligne : avec quelqu’un hors ligne, **Envoyer à Rado** reste grisé. Vous pouvez quand même choisir les fichiers, et les envoyer dès qu’il revient.

Pour envoyer un fichier depuis le [mode privé](private-mode.md#send-private-files), utilisez **Envoyer avec Direct…** dans la liste privée : Pako le lit déchiffré au fur et à mesure, sans copie lisible sur votre PC.

## Recevoir des fichiers {#receive-files}

Quand un contact vous envoie des fichiers, Pako demande : « Rado veut vous envoyer des fichiers ». Vous voyez le nom, le nombre de fichiers, la taille et les premiers noms de fichiers.

- **Enregistrer dans** montre où ils iront. **Changer** choisit un autre dossier pour cette fois seulement.
- **Accepter** lance le transfert. **Refuser** dit non, et l’expéditeur est prévenu. Fermer la question (**×** ou **Échap**) refuse aussi.
- Une question ne prend votre réponse qu’après un court instant à l’écran, pour qu’un clic de trop ou **Échap** ne la refuse pas par erreur. Il en va de même pour quelqu’un qui veut se connecter.

La question va aussi dans la cloche, et s’affiche en pop-up sauf si la fenêtre de Pako est au premier plan (il vous demande alors dans la fenêtre, sur n’importe quelle page), les deux avec **Accepter** et **Refuser** (voir [Notifications](notifications.md)).

### Peut m’envoyer sans demander {#can-send-to-me-without-asking}

Pour les personnes en qui vous avez confiance, activez **Peut m’envoyer sans demander** (menu du clic droit, ou dans le panneau de détails). Leurs fichiers arrivent alors sans la question. Pako demande quand même quand un envoi est gros pour le disque : « Demandé, car c’est gros pour ce disque ».

### Où vont les fichiers reçus {#where-received-files-go}

Par défaut, les fichiers d’un contact vont dans un dossier à son nom à l’intérieur de votre dossier de téléchargement, comme « De Rado » (un dossier « From Rado » déjà créé par un Pako en anglais continue d’être utilisé). Changez-le avec **Enregistrer ses fichiers dans…** (clic droit), ou **Changer…** à côté de **Ses fichiers vont dans** dans le panneau de détails.

Pako marque les fichiers reçus comme venant d’internet, donc Windows prévient avant de lancer un programme reçu de cette façon. **Ouvrir** sur un programme ou un script reçu ne le lance pas : Pako le montre dans son dossier, pour que vous puissiez l’ouvrir de là si vous lui faites confiance.

## Votre historique avec quelqu’un {#your-history-with-someone}

Choisissez un contact pour voir tout ce qui a été envoyé et reçu avec lui, le plus récent en bas. Chaque élément montre ses fichiers, sa taille, sa date, comment il a voyagé, et comment il s’est terminé : **Livré**, **Reçu**, **Refusé**, **Annulé**, ou ce qui n’a pas marché (par exemple « La connexion a été perdue. »). Les éléments reçus ont **Ouvrir** et **Dossier**.

Pour voir ce que contenait un envoi de plusieurs fichiers ou dossiers, cliquez sur la flèche de sa carte : la liste se déplie sur place, chaque fichier avec sa taille et chaque dossier avec son nombre de fichiers (et **Ouvrir** / **Dossier** sur chacun de ceux que vous avez reçus). Ou cliquez sur la carte elle-même : le panneau de détails à droite (**Ctrl+I**) montre la même liste, et une longue liste y défile. Appuyez sur **Échap** ou cliquez ailleurs pour revenir aux détails du contact. Une liste montre jusqu’à 200 fichiers et dossiers (et combien il y en avait de plus). Pako garde les listes de vos transferts les plus récents, environ 2 000 fichiers et dossiers en tout ; les transferts plus anciens, et ceux d’avant cette version, montrent seulement combien de fichiers ils avaient.

Pendant que des fichiers passent, leur élément montre leur avancement, la vitesse et **Annuler**. La ligne du contact dans la liste le montre aussi : « Envoi 37% · 12 Mo/s » sous son nom et une fine barre le long de la ligne. L’onglet **Direct** à gauche montre combien de questions attendent votre réponse (des fichiers, et des personnes qui veulent se connecter), ou combien de transferts sont en cours.

Si la connexion coupe au milieu (le Wi-Fi tombe, un PC se met en veille, Pako est fermé d’un côté ou de l’autre), le transfert attend au lieu de s’arrêter. Son élément indique « En attente que Rado reprenne · déjà 62% », et une fois que vous êtes de nouveau connectés tous les deux, il reprend tout seul là où il s’était arrêté : rien à cliquer, rien à accepter de nouveau. Ce qui est déjà arrivé reste sur le PC de l’autre, caché, jusqu’à la fin.

- **Annuler** fonctionne des deux côtés pendant l’attente, et supprime ce qui était arrivé.
- Un transfert qui attend 7 jours est effacé sur les deux PC, et son élément indique qu’il a expiré.
- Un fichier modifié entre-temps sur le PC de l’expéditeur est renvoyé depuis le début. Un fichier supprimé ou déplacé là-bas arrête le transfert, et indique lequel. Si les fichiers sont devenus plus gros que ce que vous avez accepté, le transfert s’arrête aussi (« … de plus que ce que vous avez accepté »), sauf si cette personne peut vous envoyer des fichiers sans demander et que la taille en plus n’est pas grosse pour votre disque. Elle peut les envoyer à nouveau.
- À la fin, chaque fichier est comparé à la copie de l’expéditeur. Un fichier qui ne correspond pas est renvoyé une fois tout seul ; s’il ne correspond toujours pas, l’élément le dit. Seul un fichier qui correspond reçoit son nom.
- Avec un Pako plus ancien de l’autre côté, un transfert s’arrête encore quand la connexion coupe (« La connexion a été perdue. »). Renvoyez-le une fois connectés.

Ce qu’un envoi peut aussi indiquer :

- « Trop d’envois en peu de temps. Réessayez dans une minute. » : son Pako l’a refusé parce que beaucoup sont arrivés en même temps.
- « Le Pako du destinataire n’a pas pu l’enregistrer. » ou « Le Pako du destinataire n’a pas pu l’enregistrer : il n’y a pas assez de place sur son disque. » : le problème est sur son PC. Pako ne vous dit jamais quel est son dossier ni combien de place il lui reste.

Pendant que des fichiers passent, Pako garde le PC éveillé (avec **Paramètres › Téléchargements › Garder le PC éveillé pendant les téléchargements** activé), **À la fin** les attend, et quitter Pako demande d’abord (avec **Demander avant de quitter pendant un téléchargement ou une mise en ligne** activé) : « Des fichiers sont en route dans Direct ».

## Envoyer depuis un autre de vos PC {#send-from-another-of-your-pcs}

Chacun de vos PC garde ses propres contacts. Si Rado est un contact de votre PC de la maison mais pas de votre portable, vous pouvez quand même lui envoyer des fichiers depuis le portable : ils passent par le PC de la maison.

Cela fonctionne avec vos propres PC associés dans [Mes appareils](devices.md), quand l’autre PC permet à celui-ci de le contrôler (**… peut contrôler mon Pako**, dans les **Paramètres** de cet appareil sur l’autre PC), et qu’il a un Pako qui sait le faire.

1. Cliquez sur la partie PC du bouton en haut de la liste des contacts (« Sam · **Ce PC** »). Le menu liste **Ce PC** et ces PC. Chacun indique le nom que vous lui avez donné dans Mes appareils (et le nom qu’il se donne, s’il est différent : « PC maison · se nomme PC bureau »), le nom que voient ses contacts et son nombre de contacts. Un PC hors ligne est grisé. (Pour renommer, c’est dans Mes appareils.)
2. Choisissez **PC maison** (par exemple). La liste montre maintenant les contacts de PC maison, lus en direct sur celui-ci, et le bouton montre le nom et l’ID Pako que voient les contacts de PC maison. Les filtres **En ligne** et **Favoris** fonctionnent ; les étiquettes sont celles de ce PC. Rien de ces contacts ne peut être changé d’ici : cela se fait sur PC maison.
3. Choisissez un contact : sa conversation est l’historique de PC maison avec lui, lu en direct sur celui-ci.
4. Ajoutez des fichiers comme d’habitude et cliquez sur **Envoyer à Rado**.

Ce qui se passe :

- Les fichiers vont d’abord sur PC maison, comme un envoi entre vos appareils (il reprend tout seul après une coupure). Là, ils attendent dans un dossier à l’intérieur des données de Pako, jamais dans vos dossiers, et ils ne peuvent être ni ouverts ni utilisés pour autre chose.
- Ensuite, PC maison les propose à Rado comme un envoi normal de sa part. On demande à Rado comme pour n’importe quel envoi de PC maison (ou pas, s’il permet à PC maison de lui envoyer sans demander), avec les mêmes limites et les mêmes questions de taille. Il voit un envoi de PC maison ; rien ne lui dit qu’il vient de votre portable.
- La carte dans la conversation montre les deux étapes : « Vers PC maison 100% · Vers Rado 45% ». **Annuler** l’arrête, où qu’il en soit.
- Si Rado est hors ligne, PC maison les lui propose dès qu’il revient. S’il n’accepte pas dans les 7 jours, ou s’il refuse, la copie sur PC maison est supprimée et la carte le dit (« Rado a refusé »).
- Une fois que Rado les a, la copie sur PC maison est supprimée, sauf si vous avez activé **Garder une copie sur PC maison** à côté du bouton d’envoi : la copie va alors dans le dossier de PC maison pour les fichiers de ce PC (voir [Mes appareils](devices.md#where-files-from-a-device-are-saved)).
- Sur PC maison lui-même, la conversation avec Rado montre les fichiers qui arrivent de votre portable, puis l’envoi à Rado.

Historique :

- Dans la conversation (sur les deux PC), la carte porte la mention « depuis Portable » (le nom que PC maison donne au PC qui l’a envoyée).
- Dans Mes appareils, l’**Historique** de PC maison a une ligne pour cet envoi : « 3 fichiers transmis à Rado · livré » (ou « Rado a refusé »). Pendant l’envoi, la ligne de PC maison dans la liste des appareils indique « Relais de 3 fichiers vers Rado ».
- Le Tableau de bord compte l’envoi une fois, sur le PC qui l’a envoyé à Rado (PC maison). L’étape entre vos deux PC compte comme n’importe quel envoi entre vos appareils.

Seuls vos propres PC associés qui autorisent le contrôle peuvent voir les contacts et les conversations l’un de l’autre, et seuls les fichiers envoyés depuis ce PC dans cet envoi sont transmis : un PC ne peut jamais faire envoyer à un autre des fichiers qui sont déjà sur son disque. Si l’autre PC désactive le contrôle, il quitte aussitôt le menu, et ce qu’il transmettait encore pour ce PC s’arrête (sa ligne dans Mes appareils › Historique indique « n’a pas abouti : Ce PC ne permet plus à celui-ci d’envoyer par son intermédiaire. »).

## Le panneau de détails {#the-details-panel}

Appuyez sur **Ctrl+I**, ou cliquez sur le bouton du panneau au-dessus de la conversation, pour en voir plus sur la personne choisie (faites glisser son bord gauche pour l’élargir ou le rétrécir, voir [Télécharger](downloading.md#follow-your-downloads)) :

- **Connexion** : **État** (**En ligne**, **En attente**, **Pas vérifié récemment**, **Vu récemment** ou **Hors ligne**), le **Trajet** ou quand vous l’avez vue pour la dernière fois, **Vitesse actuelle**, **Vérifier maintenant** quand elle n’a pas été vérifiée récemment, et **Vérifier ce contact** (**Automatique**, **Souvent…**, **Manuel**).
- **Entre vous** : combien vous avez **Envoyé** et **Reçu**, le nombre de **Transferts** et le **Dernier échange**.
- **Vitesse avec** la personne : les vitesses **Plus rapide** et **Moyenne** dans les deux sens, et une barre qui montre comment ce que vous avez échangé a voyagé : **Réseau local** (par votre réseau domestique, en bleu ; affiché dès que quelque chose est passé par là), **Direct** (directement entre les PC par internet) et **Via relais**.
- **À son sujet** : son **ID Pako** (avec un bouton pour le copier), **Son nom** (celui qu’elle a choisi), **Étiquettes**, **Favori**, **Ajouté** (quand elle a été ajoutée), **Mettre en sourdine**, **Peut m’envoyer sans demander**, et **Ses fichiers vont dans**.

## Comment vous êtes connectés {#how-youre-connected}

À côté de « En ligne », Pako indique comment les deux PC sont connectés. Cela s’affiche au-dessus de la conversation, dans le **Trajet** du panneau de détails, et sur chaque transfert aussi :

- **Réseau local** : par votre réseau domestique (le même Wi-Fi, ou un câble sur la même box). Rien ne passe par internet, et c’est en général de loin le plus rapide. Le point du contact est bleu.
- **Direct (IPv6)** : directement entre les deux PC, en IPv6. En général le plus rapide.
- **Direct** : directement entre les deux PC.
- **Via relais** : par un serveur relais, quand les deux réseaux ne permettent pas de connexion directe. Toujours chiffré de bout en bout, souvent plus lent.

Vous ne choisissez pas : Pako prend le meilleur chemin possible. Une connexion peut commencer **Via relais** et passer à un meilleur chemin quelques secondes plus tard.

## Sur le même Wi-Fi {#on-the-same-wi-fi}

Quand un contact et vous êtes sur le même réseau, par exemple à la maison sur le même Wi-Fi, vos deux Pako se trouvent sur ce réseau et se connectent sans internet. Les fichiers vont directement d’un PC à l’autre par le réseau, souvent bien plus vite que par internet, et son point devient bleu : « En ligne · Réseau local ».

- **Sans internet :** Direct continue de fonctionner sur le réseau. Les contacts sur le même réseau se connectent comme d’habitude. Les contacts ailleurs affichent « Injoignable sans internet » jusqu’au retour d’internet ; ensuite, Pako les joint tout seul.
- **Ce que voit votre réseau :** quand vous êtes visible (voir plus bas), votre Pako signale aux autres appareils du même réseau qu’un Pako est là : son ID Pako, et son adresse et son port sur ce réseau. Rien d’autre : ni votre nom, ni vos contacts. Cela reste sur le réseau : votre box ne le transmet pas à internet. Quelqu’un sur le même réseau qui voit votre ID Pako peut demander à se connecter, comme quelqu’un à qui vous l’avez donné : on vous demande d’abord, comme toujours. Seuls vos contacts se connectent tout seuls.
- **Être visible des Pako sur ce réseau**, en bas de la liste des contacts sur la page **Direct** :
  - **Sur les réseaux domestiques** (par défaut) : seulement tant que Windows marque le réseau où vous êtes comme privé (ou comme le réseau de votre entreprise). Sur un réseau que Windows marque comme public, comme le Wi-Fi d’un café ou d’un hôtel, personne ne voit votre ID Pako. La page **Direct** indique alors « Pas visible sur ce réseau (Windows le marque comme public, ou un autre réseau de ce PC). » Quand le PC est sur plusieurs réseaux à la fois (un VPN, ou un réseau virtuel créé par Hyper-V, WSL ou VirtualBox), chacun compte : un seul marqué public suffit pour rester caché. Quand vous changez de réseau, Pako vérifie à nouveau tout seul.
  - **Toujours** : visible sur tous les réseaux, publics compris.
  - **Jamais** : visible sur aucun réseau.

  Même quand vous n’êtes pas visible, votre Pako trouve quand même vos contacts sur le même réseau et s’y connecte (eux doivent être visibles). Pour rendre votre Wi-Fi de la maison privé, voir [la question du pare-feu Windows](#the-windows-firewall-question) plus bas.
- **Les réseaux invités** isolent souvent les appareils : deux PC sur un Wi-Fi invité peuvent ne pas se voir. Ils se connectent quand même par internet.

### La question du pare-feu Windows {#the-windows-firewall-question}

La première fois que Direct démarre, Windows peut demander si Pako peut utiliser le réseau (« Alerte de sécurité Windows »). Pako le signale sur la page **Direct** pendant son démarrage. Autorisez-le sur les **Réseaux privés**, pour que les Pako de votre réseau domestique puissent vous joindre.

Si vous avez cliqué sur **Annuler**, ou si votre Wi-Fi de la maison est réglé comme réseau public, les contacts sur le même Wi-Fi peuvent ne se connecter que par internet (ou pas du tout sans lui), et avec **Sur les réseaux domestiques** ils ne vous y voient pas. Pour corriger cela :

- Rendez votre Wi-Fi de la maison privé : **Paramètres Windows › Réseau et Internet › Wi-Fi ›** votre réseau **› Réseau privé**.
- Ou autorisez Pako : **Sécurité Windows › Pare-feu et protection du réseau › Autoriser une application via le pare-feu**, puis cochez **Privé** à côté de Pako.
