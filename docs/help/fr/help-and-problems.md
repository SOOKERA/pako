# Aide et problèmes

## L’aide dans Pako {#help-in-pako}

Appuyez sur **F1** n’importe où dans Pako, ou cliquez sur **Aide** en bas à gauche. L’Aide contient ces guides (avec une zone de recherche), **Nouveautés**, **À propos de Pako** et tous les tutoriels : la **Visite rapide** montre à quoi sert chaque page, et le tutoriel propre à chaque page en montre les détails.

L’Aide parle la langue de Pako (voyez [Anglais ou français](downloading.md#english-or-french)) : en français, les guides et les **Nouveautés** sont en français, et un guide pas encore traduit s’affiche en anglais.

## Trouver un réglage {#finding-a-setting}

**Paramètres** a un onglet par partie de Pako : **Fonctions et mises à jour**, **Extension de navigateur**, **Comptes**, **Fenêtre**, **Notifications**, **Téléchargements**, **Mode privé** et **Envoi**. Dans chaque onglet, les réglages sont rangés en petits groupes avec un titre (dans **Téléchargements** : **Vitesse**, **Enregistrement des fichiers**, **Vidéos**, **Pendant les téléchargements**…). Cliquez sur un onglet, ou utilisez **↑** **↓** et **Entrée** ; **Ctrl+Tab** passe à l’onglet suivant depuis n’importe où dans les Paramètres.

- **Rechercher un réglage**, en haut, trouve un réglage par son nom ou son explication dans tous les onglets, et un groupe par son titre. Choisissez-en un : Pako ouvre son onglet, y défile et le met en lumière, avec le clavier sur son interrupteur ou sa case pour que vous puissiez le changer tout de suite. **Échap** efface la recherche.
- **Avancé**, en bas d’un onglet, contient ce que la plupart des gens ne changent jamais (comme **Connexions par fichier**), sous les mêmes titres de groupe que le reste de l’onglet. Cliquez dessus pour l’ouvrir ; Pako retient s’il est ouvert, pour chaque onglet.
- **Enregistrer** garde vos modifications dans tous les onglets à la fois, et **Annuler** laisse tout comme c’était. Un point à côté du nom d’un onglet signale des modifications pas encore enregistrées. Un clic à côté des Paramètres ne les ferme pas ; avec des modifications non enregistrées, **×** ou **Échap** demande **Enregistrer les modifications ?** avec **Enregistrer**, **Abandonner** ou **Continuer à modifier**.

## Signaler un problème {#report-a-problem}

Quand quelque chose ne va pas, prévenez Sookera en un clic avec **Envoyer le rapport**, ou avec votre propre messagerie. Rien ne part tant que vous ne cliquez pas : vous voyez d’abord tout ce qui part avec vos mots.

1. Ouvrez **Aide › Signaler un problème**.
2. Écrivez ce que vous avez fait et ce que vous avez vu dans **Que s’est-il passé ?**.
3. Vérifiez **Envoyé avec votre rapport**. Cela montre tout ce qui part avec vos mots : la version de Pako, Setup ou Portable, votre version de Windows, le thème, le fuseau horaire de votre PC (les heures de la liste sont celles de votre PC) et, tant que **Joindre les problèmes récents** est activé, les 20 derniers problèmes notés par Pako (cliquez sur **Tout afficher** pour voir chacun). Désactivez **Joindre les problèmes récents** pour les laisser de côté.
4. Choisissez comment l’envoyer :
   - **Envoyer le rapport** (ou **Ctrl+Entrée**) : envoie ce qui est affiché au service de rapports de Sookera (le fuseau horaire ne part que dans les heures des problèmes : avec **Joindre les problèmes récents** désactivé, il n’est pas envoyé). Les rapports restent privés et ne servent qu’à corriger Pako. Une fois le rapport envoyé, Pako affiche sa référence (comme **#42**) : indiquez-la si vous écrivez à support@sookera.com à propos du même problème. S’il ne peut pas partir (pas de connexion, trop de rapports à la fois, ou le service ne répond pas), Pako le dit et garde vos mots : réessayez plus tard, ou utilisez l’un des moyens ci-dessous. Vous pouvez fermer la fenêtre pendant l’envoi : un message dit ensuite s’il est parti.
   - **Par e-mail** : votre messagerie ouvre un nouveau message à support@sookera.com avec tout déjà rempli. Vérifiez-le, puis envoyez-le. L’e-mail, **Copier** et **Enregistrer…** contiennent tous la même chose que **Envoyé avec votre rapport**, avec le fuseau horaire sur sa propre ligne. Si le rapport est trop long pour un lien d’e-mail, Pako met d’abord les détails dans votre presse-papiers, et l’e-mail dit où les coller.
   - **Copier** : pour une messagerie dans votre navigateur (Gmail, Outlook sur le web). Collez-les dans un nouvel e-mail à support@sookera.com.
   - **Enregistrer…** : un fichier texte que vous pouvez joindre à un e-mail.

### La liste des problèmes {#the-list-of-problems}

Pako garde une courte liste privée des 300 derniers problèmes qu’il a rencontrés : les téléchargements et vidéos qui se sont arrêtés, les vérifications de liens, les mises en ligne, les transferts Direct, les recherches de mises à jour et l’extension de navigateur. Elle peut aussi contenir une courte note de la fenêtre quand un tutoriel a mis du temps à s’afficher (quelle partie de la fenêtre l’a fait attendre, en pixels et en millisecondes ; jamais un nom de fichier). Chaque ligne indique quand, où, le message que Pako vous a montré (dans la langue de Pako à ce moment-là), et la version de Pako. C’est le fichier `problems.log` dans le dossier de données de Pako (voir [Où sont gardés vos paramètres et vos listes](install.md#where-your-settings-and-lists-are-kept)).

Avant d’écrire une ligne, Pako retire ce qui pourrait dire qui vous êtes ou ce que vous téléchargez :

- Les liens ne gardent que leur site (`https://example.com`), jamais le reste de l’adresse, même quand ils sont écrits sans `https://`.
- Les noms de dossiers et de fichiers dans les chemins deviennent `…` : un fichier ne garde que son type (`%USERPROFILE%\Downloads\….zip`). Les dossiers propres à Windows (Téléchargements, Documents, Program Files…) restent. Un nom de fichier entre guillemets devient la même chose (`"….zip"`).
- Les cookies, mots de passe, jetons et autres clés, les identifiants longs (identifiants Pako, identifiants Google Drive, identifiants de vidéos aussi), les codes de sécurité Direct et les adresses e-mail sont retirés.
- Votre dossier utilisateur Windows devient `%USERPROFILE%`, et les noms d’utilisateur des autres personnes sont laissés de côté.

Quand beaucoup de problèmes arrivent à la fois, Pako ne note le même problème qu’une fois par minute, et au plus 30 problèmes par minute.

### Si Pako doit se fermer {#if-pako-has-to-close}

Si Pako rencontre un problème qu’il ne peut pas surmonter, il le dit avant de se fermer, et ce problème est dans la liste. Redémarrez Pako : vos téléchargements continuent. Si cela se reproduit, utilisez **Aide › Signaler un problème**. Si la fenêtre de Pako ne peut pas s’ouvrir du tout, le message dit d’installer Microsoft Edge WebView2 Runtime (depuis microsoft.com) et de redémarrer Pako.

La liste reste sur votre PC. Elle ne part que dans un rapport que vous envoyez vous-même, et seulement tant que **Joindre les problèmes récents** est activé.
