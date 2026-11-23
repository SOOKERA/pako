# Mode privé

Le mode privé garde les téléchargements des fenêtres privées de votre navigateur derrière un code PIN ou un mot de passe. Leurs fichiers sont chiffrés sur ce PC : ils ne peuvent pas être ouverts depuis l’Explorateur de fichiers, seulement depuis Pako, une fois le mode privé ouvert.

Il est désactivé tant que vous ne l’activez pas. Sans lui, le téléchargement d’une fenêtre privée se passe comme décrit dans [Extension de navigateur › Fenêtres privées](browser-extension.md#private-windows) : **Terminé · privé**, puis il quitte la liste.

## L’activer {#turn-it-on}

1. Allez dans **Paramètres › Mode privé** et cliquez sur **Activer…** à côté de **Mode privé : garder les téléchargements privés derrière un code PIN**.
2. Lisez ce qu’il fait, puis cliquez sur **Continuer**.
3. Choisissez **Code PIN à 4 chiffres** ou **Mot de passe**, saisissez-le deux fois et cliquez sur **Activer le mode privé**.
   - Un code PIN ralentit quelqu’un devant ce PC : Pako attend plus longtemps après chaque essai raté. Quelqu’un qui obtient les fichiers de Pako et peut utiliser votre compte Windows pourrait essayer les dix mille codes PIN en quelques minutes. Un mot de passe est la vraie protection (au moins 10 caractères ; quelques mots à la suite, avec un chiffre ou un signe, c’est difficile à deviner).
4. Pako propose un **code de récupération** : 8 groupes de 4 lettres et chiffres. Gardez-le avec **Copier**, **Enregistrer dans un fichier** ou **Imprimer**, puis saisissez son dernier groupe et cliquez sur **Je l’ai gardé**. Ou cliquez sur **Passer**.

Pako n’affiche le code de récupération qu’à ce moment-là et ne le garde jamais. Avec lui, vous pouvez ouvrir le mode privé si vous oubliez votre code PIN ou votre mot de passe, et en choisir un nouveau. Gardez-le loin de ce PC : toute personne qui a ce code et ce PC peut ouvrir vos fichiers privés.

**Si vous oubliez votre code PIN ou votre mot de passe et n’avez pas votre code de récupération, personne ne peut ouvrir la liste privée ni ses fichiers, pas même nous.** Le seul moyen de repartir est **Réinitialiser le mode privé**, qui les supprime.

## Ce qui y va {#what-goes-in}

Une fois qu’il est activé :

- Les téléchargements des fenêtres privées de votre navigateur vont dans le mode privé, pas dans votre liste normale. Ils sont chiffrés pendant le téléchargement (on peut les mettre en pause et les reprendre comme d’habitude) et restent jusqu’à ce que vous les supprimiez. La fenêtre **Télécharger avec Pako** ne s’affiche pas pour eux.
- Les vidéos d’une fenêtre privée sont chiffrées dès qu’elles sont terminées. Jusque-là, leurs morceaux attendent, lisibles, dans le dossier propre à Pako, que seul votre compte Windows peut ouvrir (sur un disque formaté en FAT32 ou exFAT, comme certaines clés USB, Windows ne peut pas réserver un dossier à un seul compte). Ils sont supprimés quand la vidéo est terminée, retirée, ou au démarrage suivant.
- Un fichier que votre navigateur télécharge pour Pako (**Via navigateur**) est lisible, sous son vrai nom, dans le dossier de téléchargement de votre navigateur jusqu’à ce qu’il soit terminé ; Pako le chiffre alors dans le mode privé et le supprime de ce dossier. S’il ne peut pas être supprimé, Pako dit où il se trouve : supprimez-le vous-même.
- **Télécharger en privé** envoie n’importe quel lien directement dans le mode privé, même depuis une fenêtre normale : dans **Liens** (la flèche à côté du bouton principal, le menu du clic droit, ou la flèche à côté de **Télécharger** dans les détails), et dans la fenêtre **Télécharger avec Pako** (la flèche à côté de **Télécharger**). Depuis **Liens**, il est chiffré dès son premier octet. Depuis la fenêtre **Télécharger avec Pako**, les quelques secondes que Pako avait déjà récupérées pour le vérifier sont écrasées et supprimées, et il recommence, chiffré. Dans les deux cas, il quitte aussitôt vos listes normales, et rien de lui n’y est gardé. C’est proposé tant que le mode privé est activé, verrouillé ou ouvert.
- Un téléchargement de dossier (un dossier Google Drive, une archive en plusieurs parties, une playlist) reste groupé : une seule ligne de dossier dans la liste privée, ses fichiers dedans, dans leurs propres sous-dossiers.
- Vous pouvez y déplacer un téléchargement terminé : faites un clic droit dessus dans **Téléchargements**, puis **Déplacer vers Privé**. Son fichier lisible est supprimé (s’il ne peut pas l’être, Pako dit où il se trouve). Ce qui était déjà compté dans le Tableau de bord reste compté, et les copies faites avant (sauvegardes, OneDrive ou autres dossiers synchronisés, la Corbeille) ne sont pas touchées.

Les téléchargements privés n’apparaissent jamais dans votre liste normale, la petite fenêtre, le Tableau de bord, la liste des problèmes ou **Mes appareils**. La cloche dit seulement « Un téléchargement privé est terminé » (ou s’est arrêté), sans son nom.

## L’ouvrir {#open-it}

Dans **Téléchargements**, le dernier élément sous **Afficher** est **Privé**, avec un cadenas. Tant que le mode privé est verrouillé, il ressemble aux autres : rien ne laisse deviner ce qu’il contient. Cliquez dessus (ou appuyez sur **Ctrl+Maj+P** n’importe où dans Pako, ou cliquez sur **Ouvrir le mode privé…** dans **Paramètres › Mode privé**) et saisissez votre code PIN ou votre mot de passe.

- Les 3 premiers essais ratés ne coûtent rien. Après le 4e, Pako vous fait attendre avant le suivant : 5 secondes, puis deux fois plus longtemps à chaque fois, jusqu’à 15 minutes, même si vous redémarrez Pako. Chaque période de 30 minutes sans essai efface un essai raté.
- **Code PIN oublié ?** (ou **Mot de passe oublié ?**) prend votre code de récupération à la place. Pako vous demande ensuite de choisir un nouveau code PIN ou mot de passe.

Tant que le mode privé est ouvert, l’élément **Privé** devient violet et indique combien de fichiers il contient, et l’en-tête de la liste a deux boutons violets en haut à droite :

- sur la liste privée, **Liste normale** (vos autres téléchargements ; le mode privé reste ouvert) et **Verrouiller** ;
- sur la liste normale, **Téléchargements privés** (retour à la liste privée, sans code PIN) et **Verrouiller**.

Ces deux listes sont les seuls endroits où le mode privé reste ouvert. Aller ailleurs le verrouille : une autre page (**Liens**, **Envoi**, **Direct**, **Mes appareils**, **Tableau de bord**), un filtre à gauche (**Tout**, **En cours**, **En attente**, **Terminés**, **Problèmes**, une source), **Paramètres** ou **Aide**, quelle que soit la façon d’y arriver (le côté gauche, un pop-up, la cloche, un raccourci). Les fenêtres propres au mode privé (Déchiffrer, Envoyer avec Direct, Mettre en ligne, Supprimer, le lecteur, ses menus du clic droit), la cloche elle-même et la zone de recherche ne le verrouillent pas.

Si quelque chose tourne encore dans le mode privé (un envoi, une mise en ligne, un déchiffrement, ou une vidéo en lecture), Pako demande d’abord, par exemple « Quitter verrouille le mode privé et arrête d’envoyer Vacances.mp4. Quitter quand même ? » : **Quitter et verrouiller** continue (et l’arrête), **Rester** vous garde là où vous étiez. Quand rien ne tourne, il se verrouille aussitôt sans demander. Le verrouillage tout seul (voir [Il se verrouille tout seul](#it-locks-itself)) ne demande jamais.

## La liste privée {#the-private-list}

**Téléchargements privés** liste ce qui est dans le mode privé, les plus récents en premier. Un fichier terminé indique quand il a été téléchargé : la même heure que celle qui sert à nommer ses copies temporaires. Un téléchargement de dossier est une seule ligne de dossier (avec le nombre de fichiers qu’il contient) ; cliquez sur sa flèche pour le fermer ou l’ouvrir. Ses colonnes **Taille** et **Téléchargé** peuvent être élargies ou rétrécies, ou masquées par un clic droit sur les titres des colonnes, comme dans toutes les listes (voir [Télécharger](downloading.md#follow-your-downloads)). Quand **Téléchargé** est masquée, l’état de chaque fichier s’affiche sous son nom. Pako retient les largeurs et les colonnes que vous avez masquées, rien sur les fichiers.

- **Cliquez** sur un fichier ou un dossier pour le voir dans le panneau de détails à droite : sa progression et sa vitesse, sa provenance, quand il a été ajouté et téléchargé, sa **Priorité** et la façon dont il vous prévient quand il est terminé (**Désactivé**, **Pop-up**, **Important**), avec ses boutons en bas : **Ouvrir** (ou **Lire**), **Déchiffrer…**, **Envoyer…**, **Mettre en ligne…** et **Supprimer…** (ou **Pause**, **Reprendre** ou **Réessayer**, et **Supprimer…**, pendant le téléchargement). **Ctrl+I**, ou le bouton du panneau de détails dans la barre de titre (le même que sur toutes les pages), masque ou affiche le panneau ; faites glisser son bord gauche pour l’élargir ou le rétrécir.
- **En choisir plusieurs** : cochez leurs cases, ou faites **Ctrl**+clic et **Maj**+clic, ou appuyez sur **Ctrl+A** pour tous (**Échap** n’en choisit aucun). Le haut affiche « 3 sélectionnés » avec **Déchiffrer 3 fichiers…** (quand ils sont tous terminés), **Mettre 3 en pause** / **Reprendre 3** et **Supprimer 3 fichiers…**, et le panneau de détails règle leur priorité et leur notification ensemble. Le clic droit fait la même chose.
- **Touches** : **↑ ↓**, **Début**, **Fin** parcourent la liste, **Espace** coche la ligne où vous êtes, **Maj+F10** ou la touche **Menu** ouvre son menu du clic droit, **Ctrl+F** cherche dans les noms des téléchargements privés (la zone de recherche en haut), et **Ctrl+I** affiche ou masque le panneau de détails.

- **Double-cliquez** sur un fichier, ou cliquez sur son bouton **▶** / ouvrir :
  - Une vidéo se lit dans le lecteur propre à Pako. Elle est déchiffrée pendant la lecture : aucune copie lisible n’est écrite nulle part, et Pako ne note rien à son sujet. Les formats autres que MP4 et WebM nécessitent **Aperçu de plus de types de vidéos** dans **Paramètres › Fonctions et mises à jour**.
  - Tout le reste s’ouvre dans son appli habituelle à partir d’une copie temporaire, nommée comme `PrivateFile_2026-11-18_14h32.pdf` (quand il a été téléchargé), pour que son vrai nom n’apparaisse ni dans le titre de cette appli ni dans les fichiers récents de Windows. Activez **Afficher les vrais noms en ouvrant les fichiers privés** dans **Paramètres › Mode privé** pour garder le vrai nom. Les copies temporaires et les fichiers déchiffrés portent la marque « venant d’Internet » de Windows : Windows et Office les vérifient donc avant qu’ils s’exécutent ou s’ouvrent en modification.
- **Déchiffrer…** : enregistre un fichier lisible dans le dossier que vous choisissez (d’abord votre dossier de téléchargement ; **Changer…** pour en choisir un autre). Un fichier déjà présent n’est jamais remplacé : le nouveau reçoit « (2) ». À la même étape, choisissez ce qui se passe ensuite :
  - **Le retirer de la liste privée** : le fichier lisible reste là où vous l’avez enregistré, Pako n’en garde aucune trace, et la copie chiffrée est supprimée.
  - **Le déplacer dans Téléchargements** : il apparaît dans votre liste Téléchargements comme tout téléchargement terminé (**Ouvrir le fichier**, **Afficher dans le dossier**…). Le Tableau de bord ne le compte pas après coup. La copie chiffrée est supprimée.
  - **Le laisser tel quel** : il reste dans le mode privé, toujours chiffré, et vous avez aussi le fichier lisible.

  Sa fenêtre se ferme et il se déchiffre sur place : la ligne et le panneau de détails montrent où il en est (« Déchiffrement · 40 % »), et une courte note dit quand c’est fini. Si le déchiffrement s’arrête en cours de route (un disque plein, le mode privé qui se verrouille), le fichier à moitié fait est supprimé et le fichier privé reste tel qu’il était. Toute personne qui peut ouvrir ce dossier peut lire le fichier.
- **Déchiffrer le dossier…** (sur une ligne de dossier, ou par un clic droit dessus) : la même chose pour chaque fichier qu’il contient, dans un dossier du même nom à l’endroit que vous choisissez, chaque fichier de retour dans son propre sous-dossier. Plusieurs fichiers choisis sont déchiffrés de la même façon, côte à côte.
- **Envoyer avec Direct…** et **Mettre en ligne…** (le bouton d’envoi sur sa ligne, ses détails, le clic droit, ou pour plusieurs fichiers choisis) : voir [Envoyer des fichiers privés](#send-private-files).
- **Supprimer…** : le supprime définitivement du mode privé (il n’en reste aucune copie dans la Corbeille).
- **Liste normale** (en haut à droite) revient à votre liste normale ; le mode privé reste ouvert. **Verrouiller**, à côté, le verrouille.

### Envoyer des fichiers privés {#send-private-files}

Un fichier privé terminé (ou un dossier, ou plusieurs fichiers choisis) peut être envoyé à quelqu’un sans être déchiffré d’abord :

- **Envoyer avec Direct…** : choisissez un de vos contacts ou un de vos PC. Pako lit les fichiers déchiffrés au fil de l’envoi : aucune copie lisible n’est écrite sur ce PC. Le destinataire reçoit des fichiers normaux, sous leurs vrais noms (un dossier garde son organisation).
- **Mettre en ligne…** : une fenêtre avec les mêmes choix que **Nouvelle mise en ligne** dans Envoi (où ça va, **Supprimer après**, comment partent plusieurs fichiers, **Protéger par un mot de passe**), ici même, dans le mode privé. Les fichiers sont mis en ligne sous leurs vrais noms, et Pako les lit déchiffrés au fil de la mise en ligne. **Protéger par un mot de passe** fonctionne quand même : 7-Zip a besoin de fichiers sur le disque pour cela, donc Pako écrit des copies lisibles dans son propre dossier privé (que seul votre compte Windows peut ouvrir) et les supprime dès que l’archive est faite, ou si sa création échoue ou que vous annulez (et au démarrage suivant, si Pako s’est arrêté entre-temps). **ZIP en parties** fonctionne aussi : Pako fait une partie à la fois, dans ce même dossier, et supprime chaque partie (en l’écrasant d’abord) dès qu’elle est en ligne, si bien qu’une seule partie à la fois est brièvement lisible, dans un dossier que seul votre compte Windows peut ouvrir. Après une coupure, une pause ou un redémarrage, la partie suivante est refaite à partir des fichiers privés.

Vous restez dans le mode privé pendant l’envoi : les lignes des fichiers et le panneau de détails le montrent (« Envoi vers Rado · 35 % », « Mise en ligne · 60 % »). L’envoi ne fonctionne que tant que le mode privé est ouvert : s’il se verrouille, ou si vous le quittez (Pako demande d’abord), l’envoi s’arrête (renvoyez-le une fois le mode privé ouvert). Comme tout envoi, il apparaît avec le nom du fichier dans l’historique de Direct (ou **Mes appareils**) ou dans **Mis en ligne**, ainsi que dans ses pop-ups (**Fichiers livrés**, la fin de la mise en ligne), la cloche et la petite fenêtre : le fichier est à vous, vous le partagez ; ce que Pako garde privé, c’est sa propre trace de vos téléchargements. Si le mode privé se verrouille pendant que sa fenêtre Mettre en ligne est ouverte, la fenêtre se ferme et rien n’est mis en ligne.

### Copies temporaires {#temporary-copies}

Pako supprime une copie temporaire (en l’écrasant d’abord) dès que l’appli qui l’a ouverte la relâche, quand le mode privé se verrouille (ou dès que l’appli la relâche, si elle l’a encore ouverte), quand Pako se ferme, et toutes celles qui restent au démarrage suivant de Pako.

Si vous avez modifié la copie dans son appli, Pako demande **Enregistrer vos modifications dans le mode privé ?** : **Enregistrer les modifications** met la nouvelle version dans le mode privé, **Ne pas enregistrer** garde l’ancienne. Tant que le mode privé est verrouillé, Pako ne demande pas : vos modifications y sont enregistrées. Si Pako se ferme avant (y compris depuis la zone de notification), ou s’arrête de façon inattendue, vos modifications sont enregistrées dans le mode privé à ce moment-là, ou à son démarrage suivant. Si l’enregistrement ne fonctionne pas, la copie est gardée et Pako réessaie : elle n’est jamais supprimée avec vos modifications dedans.

Bon à savoir : la liste des fichiers récents d’une appli et sa liste de raccourcis (celle de Word, par exemple, ou celle du clic droit dans la barre des tâches) peuvent encore montrer le nom temporaire ; Pako ne peut pas les nettoyer. Sur un SSD, la suppression ne peut pas garantir que toutes les anciennes copies des octets ont disparu du disque.

## Il se verrouille tout seul {#it-locks-itself}

Le mode privé se verrouille :

- après **5 minutes** sans utiliser le clavier ni la souris où que ce soit sur le PC (changez-le dans **Paramètres › Mode privé › Verrouiller le mode privé après** : 1, 5, 15 ou 30 minutes, ou **Jamais pour inactivité**, qui le verrouille quand même pour toutes les raisons ci-dessous). Une vidéo privée en lecture dans le lecteur de Pako, avec la fenêtre de Pako au premier plan, le garde ouvert pendant la lecture ;
- aussitôt quand Windows se verrouille (**Win+L**) ou que le PC est sur le point de se mettre en veille, quand l’écran de veille démarre, et quand Pako est réduit, caché dans la zone de notification ou fermé ;
- quand vous cliquez sur **Verrouiller** (ou appuyez sur **Ctrl+Maj+P** quand la liste privée est affichée) ;
- quand vous quittez ses deux listes : voir [L’ouvrir](#open-it) plus haut.

Le verrouillage revient à votre liste normale et ferme le lecteur de Pako. Les téléchargements privés encore en cours continuent, cachés, dans le mode privé.

Un téléchargement privé qui n’était pas terminé quand Pako s’est fermé ne reprend pas tout seul au démarrage suivant : Pako a besoin de votre code PIN ou de votre mot de passe pour continuer à l’écrire. Il reprend dès que vous ouvrez à nouveau le mode privé.

## Vider la liste privée {#clear-the-private-list}

Dans **Paramètres › Mode privé › Vider la liste privée**, choisissez **3 heures après**, **12 heures après**, **3 jours après** ou **Après un nombre d’heures…** : les fichiers sont alors supprimés du mode privé ce temps-là après la fin de leur téléchargement, qu’il soit ouvert ou verrouillé. **Jamais** (le réglage par défaut) les garde jusqu’à ce que vous les supprimiez. Les téléchargements encore en cours restent.

## Changer votre code PIN, mot de passe ou code de récupération {#change-your-pin-password-or-recovery-code}

Dans **Paramètres › Mode privé** :

- **Changer le code PIN ou le mot de passe…** : saisissez celui que vous avez, puis le nouveau (vous pouvez passer d’un code PIN à un mot de passe, et inversement). Vos fichiers ne sont pas touchés. L’ancien ne fonctionne plus sur ce PC, mais une copie des fichiers de Pako faite avant le changement s’ouvre toujours avec lui.
- **Nouveau code de récupération…** : saisissez d’abord votre code PIN ou votre mot de passe (le mode privé reste verrouillé), puis Pako crée un nouveau code ; une fois que vous avez tapé son dernier groupe, l’ancien code ne fonctionne plus sur ce PC (une copie des fichiers de Pako faite avant s’ouvre toujours avec lui). **Annuler** oublie le nouveau code.
- **Réinitialiser…** (à côté de **Réinitialiser le mode privé**) : supprime définitivement la liste privée et tous les fichiers qu’elle contient, et désactive le mode privé.

## Comment ça marche {#how-it-works}

Le mode privé a sa propre clé, créée au hasard quand vous l’activez. Votre code PIN ou votre mot de passe (et votre code de récupération) ne font que verrouiller cette clé, par une étape volontairement lente (environ une demi-seconde par essai sur un PC récent), et cette clé verrouillée est aussi scellée par Windows pour votre compte, si bien qu’une copie du dossier de Pako ne peut pas être essayée sur un autre PC ou un autre compte. Changer votre code PIN ou votre mot de passe verrouille à nouveau la même clé : rien d’autre ne change. Chaque fichier a aussi sa propre clé, liée à sa ligne dans la liste privée, et chaque fichier et chaque ligne de la liste sont chiffrés (XChaCha20-Poly1305) et vérifiés à la lecture, si bien qu’un fichier modifié, échangé ou abîmé est refusé plutôt qu’affiché. Pako peut ajouter des téléchargements pendant que le mode privé est verrouillé sans pouvoir y lire quoi que ce soit.

Tout reste sur ce PC, dans le dossier `private` du dossier de données de Pako. Désinstaller Pako le garde, même avec **Supprimer aussi les paramètres et listes de Pako** coché, sauf si vous cochez aussi **Supprimer aussi les fichiers du mode privé** (voir [Installer et mettre à jour](install.md#uninstalling)).
