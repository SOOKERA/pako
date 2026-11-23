// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// French for Private mode (src/lib/private/*). See ./fr.ts for the glossary and the rules.
// Words: Mode privé (the feature's name, capitalised like the English) · code PIN · mot de passe ·
// code de récupération · verrouiller / déverrouiller · chiffré (encrypted) · déchiffrer (decrypt)
// · copie lisible (readable copy) · Privé (the Downloads list's entry) · mettre en ligne (upload).
import type { Entry } from "./core";

export const privacy = {
  // ---- The private list (PrivateView) ----
  "selected (after a count)": "sélectionnés",
  "Select none (Esc)": "Ne rien sélectionner (Échap)",
  "Select all (Ctrl+A)": "Tout sélectionner (Ctrl+A)",
  "{done} of {total} done": "{done} sur {total} terminés",
  "{name}, folder, {state}": "{name}, dossier, {state}",
  "Close {name}": "Fermer {name}",
  "Decrypt {name}": "Déchiffrer {name}",
  "Delete {name}": "Supprimer {name}",
  "{name}, downloaded {when}": "{name}, téléchargé : {when}",
  "Play in Pako": "Lire dans Pako",
  "Play {name}": "Lire {name}",
  "Send with Direct or Upload": "Envoyer avec Direct ou mettre en ligne",
  "Send {name}": "Envoyer {name}",
  Play: "Lire",
  Enter: "Entrée",
  Del: "Suppr",
  "Decrypt folder…": "Déchiffrer le dossier…",
  "Decrypt…": "Déchiffrer…",
  "Decrypt {n} files…": "Déchiffrer {n} fichiers…",
  "Send with Direct…": "Envoyer avec Direct…",
  "Upload…": "Mettre en ligne…",
  "Delete folder…": "Supprimer le dossier…",
  "Delete…": "Supprimer…",
  "Delete {n} files…": "Supprimer {n} fichiers…",
  "Nothing in Private mode yet": "Rien dans le mode privé pour l’instant",
  "Downloads from your browser's private windows come here, encrypted. So do links you send with {download} (in Links and the Download with Pako window). You can also move a finished download here: right-click it, then {move}.":
    "Les téléchargements des fenêtres privées de votre navigateur arrivent ici, chiffrés. Comme les liens que vous envoyez avec {download} (dans Liens et la fenêtre Télécharger avec Pako). Vous pouvez aussi déplacer ici un téléchargement terminé : faites un clic droit dessus, puis {move}.",
  "Delete {name}?": "Supprimer {name} ?",
  "Delete {n} private files?": "Supprimer {n} fichiers privés ?",
  "It's deleted from Private mode for good: there is no copy in the Recycle Bin.":
    "Il est supprimé définitivement du mode privé : il n’en reste aucune copie dans la Corbeille.",
  "They're deleted from Private mode for good: there is no copy in the Recycle Bin.":
    "Ils sont supprimés définitivement du mode privé : il n’en reste aucune copie dans la Corbeille.",

  // ---- Work running on private files (private.svelte.ts, privateLeave.ts) ----
  Decrypting: "Déchiffrement",
  "Sending to {who}": "Envoi vers {who}",
  "Waiting for {who} to accept": "En attente que {who} accepte",
  "Waiting for {who} to come back": "En attente que {who} revienne",
  "The upload stopped": "La mise en ligne s’est arrêtée",
  "Private mode locked, so its files left New upload. Open it and add them again to upload them.":
    "Le mode privé s’est verrouillé : ses fichiers ont donc quitté Nouvelle mise en ligne. Ouvrez-le et ajoutez-les à nouveau pour les mettre en ligne.",
  "the folder {name}": "le dossier {name}",
  // In "Leaving locks Private mode and stops {words}." (« et arrête d’envoyer … »).
  "sending {what}": "d’envoyer {what}",
  "uploading {what}": "de mettre en ligne {what}",
  "decrypting {what}": "de déchiffrer {what}",
  "the video": "la vidéo",
  "{n} things in progress": "{n} choses en cours",
  "{n} decrypts": { one: "{n} déchiffrement", other: "{n} déchiffrements" },
  "Private mode locked. The video stopped.": "Mode privé verrouillé, ce qui a arrêté la vidéo.",
  "Private mode locked. {list} stopped.": "Mode privé verrouillé, ce qui a arrêté {list}.",
  "Leaving locks Private mode and stops {words}. Leave anyway?": "Quitter verrouille le mode privé et arrête {words}. Quitter quand même ?",

  // ---- Details panel (PrivateDetails) ----
  "Done (how far)": "Fait",
  "Left (time)": "Restant",
  Selected: "Sélection",
  "of {total}": "sur {total}",
  "Select a private download to see its details.": "Sélectionnez un téléchargement privé pour voir ses détails.",
  "In progress": "En cours",
  "{what}: leaving Private mode stops it": "{what} : quitter le mode privé l’arrête",
  "Saved as": "Enregistré sous",
  "Encrypted in Private mode": "Chiffré dans le mode privé",
  "Decrypt…: a readable copy where you pick": "Déchiffrer… : une copie lisible là où vous voulez",
  "Send…": "Envoyer…",
  "Upload…: share it with a link": "Mettre en ligne… : partagez-le avec un lien",
  "Delete…: the file and its place in Private mode": "Supprimer… : le fichier et sa place dans le mode privé",
  "Decrypt…: readable copies where you pick": "Déchiffrer… : des copies lisibles là où vous voulez",
  "Decrypt the whole folder…: a readable copy where you pick": "Déchiffrer tout le dossier… : une copie lisible là où vous voulez",
  "Upload…: share them with a link": "Mettre en ligne… : partagez-les avec un lien",
  "Delete…: the files and their place in Private mode": "Supprimer… : les fichiers et leur place dans le mode privé",

  // ---- PIN pad (PrivatePad) ----
  "{n} minutes": "{n} minutes",
  "Open Private mode": "Ouvrir le mode privé",
  "Type your PIN": "Saisissez votre code PIN",
  "Type your password": "Saisissez votre mot de passe",
  PIN: "Code PIN",
  "Delete a digit": "Effacer un chiffre",
  "Type your recovery code": "Saisissez votre code de récupération",
  "Recovery code": "Code de récupération",
  "Wait {time} before trying again.": "Patientez {time} avant de réessayer.",
  "Forgot your PIN?": "Code PIN oublié ?",
  "Forgot your password?": "Mot de passe oublié ?",
  "Use your PIN": "Utiliser votre code PIN",
  "Use your password": "Utiliser votre mot de passe",

  // ---- Turning it on, changing it (PrivateSetup, SecretPicker) ----
  "Turn on Private mode": "Activer le mode privé",
  "New recovery code": "Nouveau code de récupération",
  "Your recovery code": "Votre code de récupération",
  "Choose a new PIN or password": "Choisissez un nouveau code PIN ou mot de passe",
  "Change PIN or password": "Changer le code PIN ou le mot de passe",
  "Private mode is on": "Le mode privé est activé",
  "New recovery code kept. The old one no longer works on this PC.": "Nouveau code de récupération conservé. L’ancien ne fonctionne plus sur ce PC.",
  "New PIN saved. The old one no longer works on this PC.": "Nouveau code PIN enregistré. L’ancien ne fonctionne plus sur ce PC.",
  "New password saved. The old one no longer works on this PC.": "Nouveau mot de passe enregistré. L’ancien ne fonctionne plus sur ce PC.",
  "Keep the downloads from your browser's private windows behind a PIN or a password.":
    "Gardez les téléchargements des fenêtres privées de votre navigateur derrière un code PIN ou un mot de passe.",
  "They go to a {private} list in Downloads, hidden until you open it.": "Ils vont dans une liste {private} dans Téléchargements, cachée tant que vous ne l’ouvrez pas.",
  "Their files are encrypted on this PC: they can't be opened from File Explorer.":
    "Leurs fichiers sont chiffrés sur ce PC : ils ne peuvent pas être ouverts depuis l’Explorateur de fichiers.",
  "Private mode locks itself when you step away, lock Windows or close Pako.":
    "Le mode privé se verrouille tout seul quand vous vous absentez, verrouillez Windows ou fermez Pako.",
  "If you forget your PIN or password and don't have your recovery code, nobody can open the private list or its files, not even us. The only way back is {reset}, which deletes them.":
    "Si vous oubliez votre code PIN ou votre mot de passe et n’avez pas votre code de récupération, personne ne peut ouvrir la liste privée ni ses fichiers, pas même nous. Le seul moyen de repartir est {reset}, qui les supprime.",
  "Type your PIN or password to make a new recovery code. Private mode stays locked.":
    "Saisissez votre code PIN ou votre mot de passe pour créer un nouveau code de récupération. Le mode privé reste verrouillé.",
  "Your PIN or password": "Votre code PIN ou mot de passe",
  "Your current PIN or password": "Votre code PIN ou mot de passe actuel",
  "Private mode is open with your recovery code. Choose a new PIN or password for next time.":
    "Le mode privé est ouvert avec votre code de récupération. Choisissez un nouveau code PIN ou mot de passe pour la prochaine fois.",
  "Private mode is on.": "Le mode privé est activé.",
  "Downloads from private windows now go to {private} in Downloads. Open it there, or with {keys}.":
    "Les téléchargements des fenêtres privées vont maintenant dans {private}, dans Téléchargements. Ouvrez-le là, ou avec {keys}.",
  "Ctrl+Shift+P": "Ctrl+Maj+P",
  Continue: "Continuer",
  "Turning on…": "Activation…",
  "Saving…": "Enregistrement…",
  "I kept it": "Je l’ai gardé",
  "PIN or password": "Code PIN ou mot de passe",
  "4-digit PIN": "Code PIN à 4 chiffres",
  "Your new PIN": "Votre nouveau code PIN",
  "Your new password": "Votre nouveau mot de passe",
  "The PIN is 4 digits.": "Le code PIN fait 4 chiffres.",
  "At least 10 characters.": "Au moins 10 caractères.",
  "Longer is safer: a few words in a row, with a digit or a sign, is hard to guess.":
    "Plus long, c’est plus sûr : quelques mots à la suite, avec un chiffre ou un signe, c’est difficile à deviner.",
  "A PIN slows down someone at this PC: Pako waits longer after each wrong try. Someone who gets Pako's files and can use your Windows account could try all 10,000 PINs in minutes. A password is the real protection.":
    "Un code PIN ralentit quelqu’un devant ce PC : Pako attend plus longtemps après chaque essai raté. Quelqu’un qui obtient les fichiers de Pako et peut utiliser votre compte Windows pourrait essayer les dix mille codes PIN en quelques minutes. Un mot de passe est la vraie protection.",
  "At least 10 characters. A long one you can remember is best: a few words in a row, with a digit or a sign, works well.":
    "Au moins 10 caractères. Le mieux est un mot de passe long que vous pouvez retenir : quelques mots à la suite, avec un chiffre ou un signe, ça marche bien.",

  // ---- The recovery code (RecoveryCode) ----
  "Recovery code copied for one minute. Paste it somewhere safe, away from this PC.":
    "Code de récupération copié pour une minute. Collez-le en lieu sûr, loin de ce PC.",
  "Preview mode: nothing is saved.": "Mode aperçu : rien n’est enregistré.",
  "Save your recovery code": "Enregistrer votre code de récupération",
  "Pako recovery code.txt": "Code de récupération Pako.txt",
  Text: "Texte",
  "Recovery code saved. Keep that file away from this PC.": "Code de récupération enregistré. Gardez ce fichier loin de ce PC.",
  "Pako recovery code": "Code de récupération Pako",
  "Pako: your Private mode recovery code": "Pako : votre code de récupération du mode privé",
  "It opens Private mode if you forget your PIN or password, and lets you choose a new one. Keep it somewhere safe, away from this PC.":
    "Il ouvre le mode privé si vous oubliez votre code PIN ou votre mot de passe, et vous permet d’en choisir un nouveau. Gardez-le en lieu sûr, loin de ce PC.",
  "Your recovery code: {code}": "Votre code de récupération : {code}",
  "Save as a file": "Enregistrer dans un fichier",
  Print: "Imprimer",
  "Pako shows it only now and never keeps it. With it, you can open Private mode if you forget your PIN or password, and choose a new one. Keep it away from this PC: anyone with it and this PC can open your private files. A newer code replaces it on this PC, but a copy of Pako's files made before still opens with the older one.":
    "Pako ne l’affiche que maintenant et ne le garde jamais. Avec lui, vous pouvez ouvrir le mode privé si vous oubliez votre code PIN ou votre mot de passe, et en choisir un nouveau. Gardez-le loin de ce PC : toute personne qui a ce code et ce PC peut ouvrir vos fichiers privés. Un code plus récent le remplace sur ce PC, mais une copie des fichiers de Pako faite avant s’ouvre toujours avec l’ancien.",
  "To check you kept it, type its last group": "Pour vérifier que vous l’avez gardé, saisissez son dernier groupe",

  // ---- Decrypt… (PrivateDecrypt) ----
  "Take it off the private list": "Le retirer de la liste privée",
  "The readable copy stays where you saved it. Pako keeps no record of it, and the encrypted copy is deleted.":
    "La copie lisible reste là où vous l’avez enregistrée. Pako n’en garde aucune trace, et la copie chiffrée est supprimée.",
  "Move it to Downloads": "Le déplacer dans Téléchargements",
  "It shows in your Downloads list like any finished download. The encrypted copy is deleted.":
    "Il apparaît dans votre liste Téléchargements comme tout téléchargement terminé. La copie chiffrée est supprimée.",
  "Leave it as is": "Le laisser tel quel",
  "It stays in Private mode, still encrypted. You also have the readable copy.": "Il reste dans le mode privé, toujours chiffré. Vous avez aussi la copie lisible.",
  "Take them off the private list": "Les retirer de la liste privée",
  "The readable copies stay where you saved them. Pako keeps no record of them, and the encrypted copies are deleted.":
    "Les copies lisibles restent là où vous les avez enregistrées. Pako n’en garde aucune trace, et les copies chiffrées sont supprimées.",
  "Move them to Downloads": "Les déplacer dans Téléchargements",
  "They show in your Downloads list like any finished download. The encrypted copies are deleted.":
    "Ils apparaissent dans votre liste Téléchargements comme tout téléchargement terminé. Les copies chiffrées sont supprimées.",
  "Leave them as they are": "Les laisser tels quels",
  "They stay in Private mode, still encrypted. You also have the readable copies.": "Ils restent dans le mode privé, toujours chiffrés. Vous avez aussi les copies lisibles.",
  "Where to save {what}, readable": "Où enregistrer {what}, en version lisible",
  "{name} is in your Downloads now.": "{name} est maintenant dans vos Téléchargements.",
  "{name} are in your Downloads now.": "{name} sont maintenant dans vos Téléchargements.",
  "{name} are in your Downloads now. (a folder)": "Le dossier {name} est maintenant dans vos Téléchargements.",
  "{name} is decrypted and off the private list.": "{name} est déchiffré et retiré de la liste privée.",
  "{name} are decrypted and off the private list.": "{name} sont déchiffrés et retirés de la liste privée.",
  "{name} are decrypted and off the private list. (a folder)": "Le dossier {name} est déchiffré et retiré de la liste privée.",
  "Readable copy of {name} saved.": "Copie lisible de {name} enregistrée.",
  "{made} of {count} decrypted. Then: {error}": "{made} sur {count} déchiffrés. Ensuite : {error}",
  "Decrypt {what}": "Déchiffrer {what}",
  "Save the readable file in": "Enregistrer le fichier lisible dans",
  "Save the readable folder in": "Enregistrer le dossier lisible dans",
  "Save the readable files in": "Enregistrer les fichiers lisibles dans",
  "Pick a folder": "Choisissez un dossier",
  "Its files go back to their places inside a folder named {folder}.": "Ses fichiers retrouvent leur place dans un dossier nommé {folder}.",
  Then: "Ensuite",
  "Anyone who can open that folder can read the file. It's marked as from the internet, so Windows checks it before it runs.":
    "Toute personne qui peut ouvrir ce dossier peut lire le fichier. Il est marqué comme venant d’Internet : Windows le vérifie donc avant qu’il s’exécute.",
  "Anyone who can open that folder can read the files. They're marked as from the internet, so Windows checks them before they run.":
    "Toute personne qui peut ouvrir ce dossier peut lire les fichiers. Ils sont marqués comme venant d’Internet : Windows les vérifie donc avant qu’ils s’exécutent.",
  Decrypt: "Déchiffrer",

  // ---- Send with Direct…, Upload… (PrivateSendDirect, PrivateUpload) ----
  "Sending {what} to {who}. Its progress shows here, in Private mode.": "Pako envoie {what} à {who}. La progression s’affiche ici, dans le mode privé.",
  "Send {what} with Direct": "Envoyer {what} avec Direct",
  "Pako reads it decrypted as it goes: no readable copy is made on this PC. The other side gets normal files, and the send shows in Direct's history. You stay here: its progress shows on its row, and leaving Private mode stops it.":
    "Pako le lit déchiffré au fil de l’envoi : aucune copie lisible n’est faite sur ce PC. Le destinataire reçoit des fichiers normaux, et l’envoi apparaît dans l’historique de Direct. Vous restez ici : la progression s’affiche sur sa ligne, et quitter le mode privé l’arrête.",
  "Pako reads them decrypted as they go: no readable copy is made on this PC. The other side gets normal files, and the send shows in Direct's history. You stay here: its progress shows on their rows, and leaving Private mode stops it.":
    "Pako les lit déchiffrés au fil de l’envoi : aucune copie lisible n’est faite sur ce PC. Le destinataire reçoit des fichiers normaux, et l’envoi apparaît dans l’historique de Direct. Vous restez ici : la progression s’affiche sur leurs lignes, et quitter le mode privé l’arrête.",
  "Find a contact or one of your PCs": "Chercher un contact ou un de vos PC",
  "Find a contact": "Chercher un contact",
  "Send to": "Envoyer à",
  "Nobody by that name.": "Personne à ce nom.",
  "No contacts yet: add someone in Direct first.": "Aucun contact pour l’instant : ajoutez d’abord quelqu’un dans Direct.",
  "Uploading {what}. Its progress shows here, in Private mode.": "Pako met en ligne {what}. La progression s’affiche ici, dans le mode privé.",
  "Upload {what}": "Mettre en ligne {what}",
  "Pako reads it decrypted as it goes up. You stay in Private mode: the upload's progress shows on its row. Leaving Private mode stops it.":
    "Pako le lit déchiffré au fil de la mise en ligne. Vous restez dans le mode privé : la progression de la mise en ligne s’affiche sur sa ligne. Quitter le mode privé l’arrête.",
  "Pako reads them decrypted as they go up. You stay in Private mode: the upload's progress shows on their rows. Leaving Private mode stops it.":
    "Pako les lit déchiffrés au fil de la mise en ligne. Vous restez dans le mode privé : la progression de la mise en ligne s’affiche sur leurs lignes. Quitter le mode privé l’arrête.",

  // ---- The player, the header's switch, changes saved back (PrivatePlayer, PrivateSwitch, SaveAsk) ----
  "This video can't be played in Pako.": "Cette vidéo ne peut pas être lue dans Pako.",
  "Normal list": "Liste normale",
  "Normal list: your other downloads. Private mode stays open.": "Liste normale : vos autres téléchargements. Le mode privé reste ouvert.",
  "Private downloads: back to your private list": "Téléchargements privés : retour à votre liste privée",
  Lock: "Verrouiller",
  "Lock Private mode (Ctrl+Shift+P)": "Verrouiller le mode privé (Ctrl+Maj+P)",
  "Your changes are saved in Private mode.": "Vos modifications sont enregistrées dans le mode privé.",
  "Save your changes into Private mode?": "Enregistrer vos modifications dans le mode privé ?",
  "Don't save": "Ne pas enregistrer",
  "{name} was changed in the app you opened it with. Save those changes into Private mode, or keep the file as it was? The temporary copy is deleted either way.":
    "{name} a été modifié dans l’appli avec laquelle vous l’avez ouvert. Enregistrer ces modifications dans le mode privé, ou garder le fichier tel qu’il était ? La copie temporaire est supprimée dans les deux cas.",
  "A private file you opened was changed in the app you opened it with. Save those changes into Private mode, or keep the file as it was? The temporary copy is deleted either way.":
    "Un fichier privé que vous avez ouvert a été modifié dans l’appli avec laquelle vous l’avez ouvert. Enregistrer ces modifications dans le mode privé, ou garder le fichier tel qu’il était ? La copie temporaire est supprimée dans les deux cas.",
  "Save changes": "Enregistrer les modifications",
} satisfies Record<string, Entry>;
