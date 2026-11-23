# Traduire les guides en français

Note pour qui traduit les guides. Ce fichier n’est pas un guide : Pako ne l’affiche pas et il ne part pas dans help.zip.

## Les fichiers

- Un guide français porte le même nom que l’anglais, dans ce dossier : `docs/help/fr/downloading.md` pour `docs/help/downloading.md`. Tant qu’un guide n’a pas de fichier français, Pako en français montre le guide anglais.
- `README.md` est l’index français : les mêmes guides, une ligne chacun (titre et description). L’ordre des guides vient de l’index anglais ; le titre affiché vient du titre `#` du guide. Ce titre tient sur une ligne de la colonne de gauche de l’Aide : 26 caractères au plus (« Installer et mettre à jour » est à la limite).
- Gardez la structure du guide anglais : les mêmes sections, dans le même ordre.
- `whats-new.md` a les mêmes titres de version que l’anglais, au caractère près et dans le même ordre : `## Next version`, `## 1.3.4`… Le titre `## Next version` reste en anglais (Pako affiche « Prochaine version »). Une version sans section française, ou avec une section vide, s’affiche en anglais. À la sortie d’une version, renommez `## Next version` dans les deux fichiers.

## Les ancres des titres

La fenêtre et les autres guides ouvrent une section par l’ancre du titre anglais (`downloading.md#google-limit-too-many-downloads-today`). Chaque titre `##`, `###`… d’un guide français garde donc l’ancre de son titre anglais, écrite à la fin du titre entre `{#` et `}` :

- anglais : `### "Google limit": too many downloads today`
- français : `### « Limite de Google » : trop de téléchargements aujourd’hui {#google-limit-too-many-downloads-today}`

La règle exacte :

- L’ancre anglaise est le titre anglais en minuscules, sans ses marques Markdown ni sa ponctuation (seuls restent lettres, chiffres, espaces et `-`), chaque espace remplacée par `-`. Un titre répété dans le même guide prend `-1`, puis `-2`…
- `{#…}` vient en dernier sur la ligne, après une espace ; l’ancre ne contient que `a`–`z`, `0`–`9`, `-` et `_`. Pako ne l’affiche pas.
- Le titre `#` du guide (la première ligne) n’en a pas besoin.
- `test/help-guides.test.mjs` vérifie que chaque guide français a exactement les ancres du guide anglais, et que chaque lien mène quelque part.

## Les liens

- Vers un autre guide ou une section : le même nom de fichier et la même ancre anglaise que dans le guide anglais, seul le texte du lien est traduit : `[la limite de Google](downloading.md#google-limit-too-many-downloads-today)`, `[plus bas](#before-a-download-starts)`.
- Les adresses web restent les mêmes.

## Les mots de la fenêtre

- Les noms des boutons, onglets, pages, paramètres et messages s’écrivent exactement comme la fenêtre les affiche en français, en gras comme dans l’anglais : **Envoyer le rapport**, **Paramètres › Téléchargements**. Le français de la fenêtre est dans `src/lib/i18n/fr-*.ts`.
- Les touches comme Windows les nomme en français : Ctrl, Maj, Échap, Entrée, Suppr, Espace (**Ctrl+Maj+P**).
- Le vocabulaire et le ton : le glossaire en tête de `src/lib/i18n/fr.ts` (« vous », appli et jamais « app », Paramètres, Envoi…).

## La typographie, écrite telle quelle

Dans la fenêtre, la typographie française est ajoutée toute seule ; dans les guides, rien ne l’ajoute : elle est écrite dans le fichier.

- Une espace insécable (U+00A0) avant : et à l’intérieur des guillemets « ».
- Une espace fine insécable (U+202F) avant ; ! et ?.
- L’apostrophe ’ (jamais '), les points de suspension … (jamais ...).
- Rien de tout cela dans le code entre accents graves ni dans les adresses.

Le test vérifie aussi ces règles.
