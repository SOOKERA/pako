# Notifications

Pako vous prévient de deux façons : des pop-ups en bas à droite de l’écran, et la cloche en haut de la fenêtre de Pako, qui garde les choses importantes. Un téléchargement que vous marquez **Important** a un pop-up plus grand, au milieu de l’écran.

## Quand un téléchargement se termine : Désactivé, Pop-up, Important {#when-a-download-finishes-off-pop-up-important}

Chaque téléchargement a son propre choix, **Prévenir une fois terminé** :

- **Désactivé** : ni pop-up ni son quand il se termine.
- **Pop-up** (le réglage habituel) : un pop-up en bas à droite et un petit carillon.
- **Important** : un grand pop-up au milieu de l’écran que vous utilisez, avec un carillon. Il reste jusqu’à ce que vous cliquiez dessus, pour que vous ne puissiez pas le manquer, et ne vous prend jamais le clavier pendant que vous tapez.

Changez-le en un clic partout où vous le voyez :

- la cloche au bout de la ligne du téléchargement : chaque clic passe à l’étape suivante (**Pop-up**, **Important**, **Désactivé**). Pour **Pop-up**, elle n’apparaît que lorsque la souris est sur la ligne ; **Important** (pleine, dans la couleur de Pako) et **Désactivé** (barrée) s’affichent toujours ;
- les détails du téléchargement, sous **Paramètres** ;
- le clic droit : **Prévenir : Désactivé**, **Prévenir : Pop-up**, **Prévenir : Important** ;
- dans **Liens** : **Désactivé**, **Pop-up** ou **Important** à côté de **Télécharger** (dans une fenêtre étroite, dans la flèche à côté), la cloche au bout de la ligne de chaque lien, ou **Télécharger en Important** dans le menu du clic droit d’un lien ;
- la fenêtre **Télécharger avec Pako** (voir [Extension de navigateur](browser-extension.md)).

Les nouveaux téléchargements partent avec **Nouveaux téléchargements** dans **Paramètres › Notifications** (**Pop-up** au départ).

Le pop-up **Important** passe à travers Ne pas déranger, celui de Pako comme celui de Windows. Il ne s’affiche jamais par-dessus quelque chose en plein écran (un jeu, une vidéo en plein écran, une présentation) : il attend que cela se termine, puis s’affiche, et il s’efface si quelque chose passe en plein écran pendant qu’il est affiché. Plusieurs téléchargements qui se terminent ensemble font une seule carte (« 3 téléchargements terminés », avec **Afficher dans le dossier** quand ils sont dans un même dossier). Les cartes qui arrivent ensuite attendent leur tour (« 1 autre ensuite », **Tout fermer**). Cliquez dessus pour ouvrir Pako sur ce téléchargement, ou utilisez **Ouvrir**, **Afficher dans le dossier** ou **×**. Une carte qui n’a jamais pu s’afficher (Pako s’est fermé avant) est gardée dans la cloche.

Un dossier ou une archive en plusieurs parties prévient une seule fois, selon le réglage de son fichier le plus voyant. Une archive en plusieurs parties que Pako extrait tout seul prévient quand l’extraction est finie : **Téléchargé et extrait**.

**Désactivé** ne concerne que la fin : un téléchargement qui s’arrête (une erreur, la limite de Google…) reçoit le pop-up **Téléchargement arrêté** selon les Paramètres, quel que soit son choix ; un téléchargement **Important** le reçoit toujours.

## Pop-ups {#pop-ups}

Les pop-ups sont de petites cartes en bas à droite de l’écran, juste au-dessus de la barre des tâches. Ils ne vous détournent jamais de ce que vous faites : l’appli où vous êtes reste au premier plan. Ils suivent l’apparence claire ou sombre de Pako (voir [Clair ou sombre](downloading.md#light-or-dark)).

Cliquez sur un pop-up pour ouvrir Pako sur la bonne page (le téléchargement, la mise en ligne, la personne dans Direct…). Le **×** le ferme. Certains ont des boutons :

- Un téléchargement terminé : **Ouvrir** et **Afficher dans le dossier** (aussi sur le pop-up **Important** au milieu de l’écran).
- Une mise en ligne terminée : **Voir dans Envoi**.
- Des fichiers que quelqu’un vous envoie : **Refuser** et **Accepter**.
- Quelqu’un qui veut se connecter : **Voir**. Cela ouvre Pako, où vous comparez le code de sécurité (voir [Direct](direct.md)). Pour un PC qui veut s’associer comme l’un de vos appareils, cela ouvre **Mes appareils**.

Jusqu’à trois s’affichent à la fois, le plus récent en bas. Quand d’autres attendent, la pile indique « 2 autres en attente » (par exemple).

### Combien de temps ils restent {#how-long-they-stay}

- La plupart s’en vont tout seuls après environ six secondes. Tant que la souris est dessus, ils attendent, et restent encore au moins quelques secondes après qu’elle est partie.
- Les questions restent jusqu’à ce que vous y répondiez ou les fermiez : des fichiers en attente de votre réponse, quelqu’un qui veut se connecter, et **Pako a besoin de vous** (une page de lien qui a besoin de vous, comme une vérification « Je ne suis pas un robot »). Elles s’en vont aussi toutes seules une fois la question réglée ailleurs : répondue dans Pako, la page de lien terminée ou fermée, ou l’autre personne a abandonné.
- Le même lien qui redemande remplace son pop-up précédent.

Certaines nouvelles ne s’affichent pas en pop-up quand la fenêtre de Pako les montre déjà : une mise en ligne qui se termine ou échoue pendant que la fenêtre est ouverte, et les nouvelles de [Direct](direct.md) et de [Mes appareils](devices.md) pendant que la fenêtre de Pako est au premier plan sur cette page. Sur toute autre page, ou avec Pako derrière une autre appli, elles s’affichent en pop-up : les fichiers qui arrivent de quelqu’un qui peut envoyer sans demander (ou de votre autre PC), les fichiers reçus, et **Fichiers livrés** quand votre envoi arrive à destination (aussi quand il est passé par votre autre PC). Les fichiers qu’on vous propose et les demandes de connexion sont posés dans la fenêtre de Pako sur toutes les pages : ils ne s’affichent donc en pop-up que lorsque Pako est derrière une autre appli ou caché. Un téléchargement terminé réglé sur **Pop-up** ne s’affiche pas en pop-up pendant que la petite fenêtre (celle de l’icône de la zone de notification) est ouverte à l’écran : vous y voyez le téléchargement se terminer. **Important** s’affiche toujours. Les téléchargements échoués et les autres nouvelles s’affichent quand même en pop-up, et la cloche reçoit toujours la ligne si son interrupteur **Cloche** est activé.

Un téléchargement dans le [mode privé](private-mode.md) ne montre jamais son nom : son pop-up (le pop-up **Important** aussi) et sa ligne dans la cloche disent seulement « Un téléchargement privé est terminé » (ou s’est arrêté).

## La cloche {#the-bell}

La cloche en haut de la fenêtre de Pako garde les choses importantes pendant 30 jours. Un nombre dessus indique combien sont non lues, et l’icône de la zone de notification reçoit aussi un point.

Par défaut, elle garde :

- **Quelqu’un veut se connecter**
- **Fichiers en attente de votre réponse**
- **Pako a besoin de vous**
- **Nouvelle version de Pako prête** : une nouvelle version est téléchargée, avec **Redémarrer maintenant**
- **Vidéo impossible à récupérer** : une vidéo envoyée avec le bouton **Télécharger** du lecteur que Pako n’a pas pu récupérer (le bouton avait déjà indiqué **Envoyé à Pako ✓**)

D’autres types peuvent y aller aussi : activez leur interrupteur **Cloche** dans **Paramètres › Notifications**.

Cliquez sur la cloche pour voir la liste, groupée par **Aujourd’hui** et **Plus tôt** :

- Cliquez sur une ligne pour ouvrir Pako sur la bonne page. Elle compte alors comme lue.
- Les fichiers en attente de votre réponse ont **Refuser** et **Accepter** directement là (les fichiers vont dans le dossier habituel de ce contact). Quelqu’un qui veut se connecter a **Voir**.
- Une fois une question réglée, sa ligne dit comment : **Accepté**, **Refusé**, **Résolu**, **Terminé**, **Pas terminé** ou **N’attend plus**.
- **Tout marquer comme lu** efface le nombre et le point de la zone de notification. **Vider** vide la liste.
- La roue dentée ouvre **Paramètres › Notifications**.

Les lignes de plus de 30 jours s’en vont toutes seules, et la cloche garde au plus les 300 plus récentes.

## Paramètres › Notifications {#settings--notifications}

En haut, sous **Quand un téléchargement se termine**, deux choix :

- **Nouveaux téléchargements** : **Désactivé**, **Pop-up** ou **Important**, le réglage de départ de chaque nouveau téléchargement. Chaque téléchargement peut quand même le changer. Si vous aviez désactivé le pop-up **Téléchargement terminé** dans un ancien Pako, cela démarre sur **Désactivé**.
- **Son quand un téléchargement se termine** : **Pop-up et Important** (au départ), **Important seulement** ou **Jamais**.

Sous **Par type de notification**, chaque type a deux interrupteurs : **Pop-up** et **Cloche**. **Téléchargement terminé** n’a que **Cloche** : son pop-up se choisit pour chaque téléchargement (il indique **Au cas par cas**). Ils sont groupés :

- **Téléchargements** : **Téléchargement terminé**, **Téléchargement arrêté**, **Envoyé par votre navigateur**, **Vidéo impossible à récupérer**.
- **Envoi** : **Mise en ligne terminée**, **Mise en ligne impossible**, **Mise en ligne supprimée à la date prévue**.
- **Direct** : **Quelqu’un veut se connecter**, **Fichiers en attente de votre réponse**, **Des fichiers arrivent**, **Fichiers reçus**, **Fichiers livrés**. Ils couvrent aussi [Mes appareils](devices.md).
- **Pako** : **Pako a besoin de vous**, **Veille et arrêt**, **Nouvelle version de Pako prête**, **Autres messages**.

Chaque type s’affiche en pop-up au départ. Cliquez sur **Enregistrer** pour garder vos changements.

### Sons : fichiers qui arrivent, fichiers envoyés {#sounds-files-arriving-files-sent}

Pako a deux sons, un pour chaque sens :

- Un téléchargement terminé sonne comme le dit **Son quand un téléchargement se termine** (voir plus haut) : avec **Pop-up et Important**, chaque téléchargement réglé sur **Pop-up** ou **Important** ; avec **Important seulement**, seulement ceux-là ; avec **Jamais**, aucun. **Désactivé** ne sonne jamais. Le téléchargement **Important** sonne quand son pop-up apparaît.
- **Jouer un son quand des fichiers arrivent** (activé au départ) : le même carillon doux quand des fichiers arrivent dans [Direct](direct.md) ou [Mes appareils](devices.md), et quand quelqu’un vous propose des fichiers et que Pako attend votre réponse. Un envoi que vous devez accepter sonne deux fois (quand il est proposé, puis quand il est arrivé) ; celui de quelqu’un qui peut envoyer sans demander sonne une fois, quand il est arrivé. Un contact en sourdine ne fait aucun son.
- **Jouer un son quand des fichiers sont envoyés** (activé au départ) : un son plus clair quand une [mise en ligne](upload.md) se termine (une fois pour toute la mise en ligne, aussi pour un ZIP en parties), et quand un envoi Direct ou Mes appareils est livré. Un envoi qui passe par un autre de vos PC sonne une fois arrivé chez le contact. Un fichier qu’un autre de vos PC demande (**Récupérer ce fichier** dans [Mes appareils](devices.md)) ne fait pas de son d’envoi ici : ce PC sonne quand il arrive.

Les deux jouent aussi pendant que la petite fenêtre est ouverte. Quand le même son devrait rejouer dans les deux secondes environ (plusieurs téléchargements qui se terminent ensemble), il joue une seule fois ; un dossier sonne une fois, quand son dernier fichier est fini. Quand les deux sons arrivent au même moment, l’un joue après l’autre. Ils restent silencieux pendant Ne pas déranger, et pendant que Windows dit « pas maintenant » quand **Suivre Ne pas déranger de Windows** est activé, sauf le carillon d’un téléchargement **Important**, qui vient avec son pop-up.

### Ne pas déranger {#do-not-disturb}

Sous **Moments de calme** :

- **Ne pas déranger** : ni pop-ups ni son, sauf ceux d’un téléchargement **Important** (voir plus haut). La cloche continue de recueillir (les types dont l’interrupteur **Cloche** est activé).
- **Suivre Ne pas déranger de Windows** (activé au départ, sous **Avancé** en bas de l’onglet) : ni pop-ups ni sons pendant les jeux et vidéos en plein écran, les présentations, ou le Ne pas déranger de Windows lui-même. La cloche continue de recueillir. (**Important** ne s’affiche jamais par-dessus quelque chose en plein écran, quoi qu’en dise ce réglage : il attend.)

Les pop-ups qui auraient dû s’afficher pendant ce temps sont sautés, pas gardés pour plus tard (la cloche a les types dont l’interrupteur **Cloche** est activé). Les pop-ups déjà à l’écran se cachent, et reviennent quand c’est fini avec le reste de leur temps.
