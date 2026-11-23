// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Direct's words in the language of the PC that shows them.
//!
//! Direct keeps what it says in English: its lines are saved (the history, My devices'
//! history and Links cards), told to your other PCs and to contacts (why a send was turned
//! away, what an order did there), and some are read back (`relay::no_places`, "Not enough
//! room"). Each PC puts them in its own language only when it shows them: the window's views
//! and the answers to My devices' and the relay's commands (see `mod.rs`). So a French Pako
//! shows an English Pako's reasons in French and the other way round, an older Pako still
//! understands everything this one sends, and a switch of language changes every line shown,
//! old ones too. A line this table doesn't know (a newer Pako's) shows as it came.
//!
//! Words only ever shown on this PC and made in a place without such a boundary (the answers
//! of Direct's commands in `lib.rs`, notifications) use `tr!` where they're made.

/// English line, French line. In the English, `{}` stands for words kept as they are (a name,
/// a number, Windows' own reason) and `{*}` for words that are themselves one of these lines or
/// a size (put in French too); the French takes them in the same order, or by place (`{0}`).
/// A more precise line comes before a general one that would match it too. French typography
/// (no-break spaces) is written in the French itself.
const LINES: &[(&str, &str)] = &[
    // ---------- Direct starting ----------
    ("Direct isn't running yet. Open the Direct page to start it.", "Direct n’est pas encore lancé. Ouvrez la page Direct pour le lancer."),
    ("Direct is still starting. Try again in a moment.", "Direct démarre encore. Réessayez dans un instant."),
    ("Direct couldn't set its old key aside ({}).", "Direct n’a pas pu mettre son ancienne clé de côté ({})."),
    ("Direct couldn't read its key ({}). Restart Pako to try again.", "Direct n’a pas pu lire sa clé ({}). Redémarrez Pako pour réessayer."),
    ("Direct couldn't protect its new key ({}).", "Direct n’a pas pu protéger sa nouvelle clé ({})."),
    ("Direct couldn't save its new key ({}). Restart Pako to try again.", "Direct n’a pas pu enregistrer sa nouvelle clé ({}). Redémarrez Pako pour réessayer."),
    ("Direct couldn't save its new key ({}).", "Direct n’a pas pu enregistrer sa nouvelle clé ({})."),
    ("Direct couldn't start ({}).", "Direct n’a pas pu démarrer ({})."),
    // ---------- contacts ----------
    ("Not reachable without internet", "Injoignable sans internet"),
    ("Not reachable right now ({})", "Injoignable pour le moment ({})"),
    ("Not reachable right now", "Injoignable pour le moment"),
    ("They declined. You can ask again after 7 days.", "La personne a refusé. Vous pourrez redemander dans 7 jours."),
    ("No answer yet. Pako asks again later.", "Pas encore de réponse. Pako redemandera plus tard."),
    ("Contact not found.", "Contact introuvable."),
    ("Device not found.", "Appareil introuvable."),
    ("another PC", "un autre PC"),
    // ---------- sending ----------
    ("They aren't connected right now.", "L’autre PC n’est pas connecté pour le moment."),
    ("They aren't connected any more.", "L’autre PC n’est plus connecté."),
    ("Add files first.", "Ajoutez d’abord des fichiers."),
    ("Private files are sent as they are: turn off packing them into a ZIP.", "Les fichiers privés sont envoyés tels quels : désactivez leur mise en ZIP."),
    ("This can't be sent as it is: {*}", "Impossible d’envoyer cela tel quel : {*}"),
    (
        "Too many files (or very long names) for one send. Turn on Compressed ZIP, or send them in a few goes.",
        "Trop de fichiers (ou de noms très longs) pour un seul envoi. Activez ZIP compressé, ou envoyez-les en plusieurs fois.",
    ),
    ("A picked item has no name.", "Un élément choisi n’a pas de nom."),
    ("That's too many files for one send.", "C’est trop de fichiers pour un seul envoi."),
    ("There are no files in what was picked.", "Il n’y a aucun fichier dans ce qui a été choisi."),
    ("Couldn't read {}: {}", "Impossible de lire {} : {}"),
    ("Private mode locked, so this send stopped. Open it and send again.", "Le mode privé s’est verrouillé, donc cet envoi s’est arrêté. Ouvrez-le et envoyez à nouveau."),
    ("{} got shorter while sending.", "{} a raccourci pendant l’envoi."),
    // ---------- how a transfer ended ----------
    ("The connection was lost.", "La connexion a été perdue."),
    ("No answer.", "Pas de réponse."),
    ("You removed this contact.", "Vous avez retiré ce contact."),
    ("They cancelled it.", "Annulé de l’autre côté."),
    ("They declined it.", "Le destinataire l’a refusé."),
    ("They couldn't check the files.", "Le destinataire n’a pas pu vérifier les fichiers."),
    (
        "Their Pako couldn't take it: the files changed on your PC and are now bigger than what they accepted. Send them again.",
        "Le Pako du destinataire n’a pas pu l’accepter : les fichiers ont changé sur votre PC et sont maintenant plus gros que ce qui a été accepté. Envoyez-les à nouveau.",
    ),
    ("Their Pako couldn't take it: {*}", "Le Pako du destinataire n’a pas pu l’accepter : {*}"),
    ("Their Pako couldn't take it.", "Le Pako du destinataire n’a pas pu l’accepter."),
    ("Their Pako couldn't save it: there isn't enough room on their drive.", "Le Pako du destinataire n’a pas pu l’enregistrer : il n’y a pas assez de place sur son disque."),
    ("Their Pako couldn't save it.", "Le Pako du destinataire n’a pas pu l’enregistrer."),
    ("Their Pako sent something this one doesn't understand, so it stopped.", "L’autre Pako a envoyé quelque chose que celui-ci ne comprend pas, donc le transfert s’est arrêté."),
    (
        "{} is gone or can't be read on this PC, so this couldn't be finished. Send it again.",
        "{} n’est plus là ou ne peut pas être lu sur ce PC, donc le transfert n’a pas pu se terminer. Envoyez-le à nouveau.",
    ),
    ("Nothing moved for 2 minutes, so it stopped.", "Rien n’a bougé pendant 2 minutes, donc le transfert s’est arrêté."),
    ("Expired: the other PC didn't come back within 7 days.", "Expiré : l’autre PC n’est pas revenu dans les 7 jours."),
    (
        "Expired: the other PC didn't come back within 7 days, so the part that arrived was deleted.",
        "Expiré : l’autre PC n’est pas revenu dans les 7 jours, donc la partie arrivée a été supprimée.",
    ),
    ("Too many sends in a short time. Try again in a minute.", "Trop d’envois en peu de temps. Réessayez dans une minute."),
    ("They already have {} sends from you waiting for an answer.", "Ce contact a déjà {} envois de votre part en attente de réponse."),
    // ---------- receiving ----------
    ("Asked because it's big for the drive: {*} is free there.", "Demandé, car c’est gros pour ce disque : il y reste {*} de libre."),
    ("This offer is gone.", "Cet envoi n’est plus là."),
    (
        "The files changed on their PC and are now {*} bigger than what you accepted, so it stopped. They can send them again.",
        "Les fichiers ont changé sur le PC de l’expéditeur et font maintenant {*} de plus que ce que vous avez accepté, donc le transfert s’est arrêté. L’expéditeur peut les envoyer à nouveau.",
    ),
    (
        "Not enough room: the files changed on their PC and now need {*} more, {*} is free there.",
        "Pas assez de place : les fichiers ont changé sur le PC de l’expéditeur et demandent maintenant {*} de plus, il y reste {*} de libre.",
    ),
    (
        "Not enough room: this needs {*}, {*} is free there (Pako always leaves {*} free).",
        "Pas assez de place : il faut {*}, il y reste {*} de libre (Pako laisse toujours {*} de libre).",
    ),
    ("Couldn't save the files ({}).", "Impossible d’enregistrer les fichiers ({})."),
    ("Couldn't save the files.", "Impossible d’enregistrer les fichiers."),
    ("Some parts never arrived.", "Certaines parties ne sont jamais arrivées."),
    ("The files didn't arrive intact.", "Les fichiers ne sont pas arrivés intacts."),
    ("{} didn't arrive intact, even when sent again.", "{} n’est pas arrivé intact, même envoyé une seconde fois."),
    ("Couldn't make the folder {}: {}", "Impossible de créer le dossier {} : {}"),
    ("A file name isn't safe: {}", "Un nom de fichier n’est pas sûr : {}"),
    ("A file name isn't safe.", "Un nom de fichier n’est pas sûr."),
    ("Couldn't find a free folder name to save to.", "Impossible de trouver un nom de dossier libre pour enregistrer."),
    ("Couldn't find a free name to save to.", "Impossible de trouver un nom libre pour enregistrer."),
    // (What the receiving Pako checks in an offer.)
    ("The offer has no files, or far too many.", "L’envoi n’a aucun fichier, ou beaucoup trop."),
    ("The offer's name is too long.", "Le nom de l’envoi est trop long."),
    ("A file name is too long.", "Un nom de fichier est trop long."),
    ("The offer lists the same file twice.", "L’envoi contient deux fois le même fichier."),
    ("The sizes don't add up.", "Les tailles ne concordent pas."),
    // ---------- Direct relay (sending from another of your PCs) ----------
    ("That PC no longer lets this one send through it.", "Ce PC ne permet plus à celui-ci d’envoyer par son intermédiaire."),
    ("Couldn't read one of the files.", "Impossible de lire l’un des fichiers."),
    ("A file got shorter while it was being sent.", "Un fichier a raccourci pendant son envoi."),
    ("Couldn't make the folder to save to.", "Impossible de créer le dossier où enregistrer."),
    ("Something went wrong on that PC.", "Un problème est survenu sur ce PC."),
    (
        "{} doesn't let this PC see its Direct contacts. On {}, turn on \"… can control my Pako\" for this PC (and update Pako there).",
        "{0} ne permet pas à ce PC de voir ses contacts Direct. Sur {1}, activez « … peut contrôler mon Pako » pour ce PC (et mettez Pako à jour là-bas).",
    ),
    // (As an older Pako said it.)
    (
        "{} doesn't let this PC see its Direct contacts. On {}, turn on \"This device can control my Pako\" for this PC (and update Pako there).",
        "{0} ne permet pas à ce PC de voir ses contacts Direct. Sur {1}, activez « Cet appareil peut contrôler mon Pako » pour ce PC (et mettez Pako à jour là-bas).",
    ),
    ("That PC isn't connected right now.", "Ce PC n’est pas connecté pour le moment."),
    ("{} isn't connected right now.", "{} n’est pas connecté pour le moment."),
    ("That PC said no.", "Ce PC a refusé."),
    ("Too many asks in a short time. Try again in a minute.", "Trop de demandes en peu de temps. Réessayez dans une minute."),
    ("That contact isn't on this PC any more.", "Ce contact n’est plus sur ce PC."),
    ("That Pako doesn't know this. Update Pako there.", "Ce Pako ne connaît pas cela. Mettez Pako à jour là-bas."),
    ("That contact can't be reached this way.", "Ce contact ne peut pas être joint de cette façon."),
    (
        "{} doesn't let this PC send from it. Turn on \"… can control my Pako\" there for this PC.",
        "{} ne permet pas à ce PC d’envoyer depuis lui. Activez-y « … peut contrôler mon Pako » pour ce PC.",
    ),
    // (As an older Pako said it.)
    (
        "{} doesn't let this PC send from it. Turn on \"This device can control my Pako\" there for this PC.",
        "{} ne permet pas à ce PC d’envoyer depuis lui. Activez-y « Cet appareil peut contrôler mon Pako » pour ce PC.",
    ),
    ("This relay is over.", "Ce relais est terminé."),
    ("That PC already had this send.", "Ce PC avait déjà cet envoi."),
    ("{} declined this PC.", "{} a refusé ce PC."),
    ("That contact can't get files from this PC yet.", "Ce contact ne peut pas encore recevoir de fichiers de ce PC."),
    ("This PC is already passing on {} sends for you. Try again once they're delivered.", "Ce PC transmet déjà {} envois pour vous. Réessayez une fois qu’ils sont livrés."),
    ("There isn't enough room on that PC to pass this on.", "Il n’y a pas assez de place sur ce PC pour transmettre cela."),
    ("That PC couldn't make room for it.", "Ce PC n’a pas pu lui faire de la place."),
    ("That PC didn't take it.", "Ce PC ne l’a pas pris."),
    ("{} can't get files from that PC any more.", "{} ne peut plus recevoir de fichiers de ce PC."),
    ("The copy on that PC is gone.", "La copie sur ce PC n’est plus là."),
    ("Cancelled on the PC that sent it.", "Annulé sur le PC qui l’a envoyé."),
    ("That contact was removed on the PC passing it on.", "Ce contact a été retiré sur le PC qui transmet l’envoi."),
    ("Pako closed before the files got to that PC.", "Pako s’est fermé avant que les fichiers n’arrivent sur ce PC."),
    // A relay's line in My devices' history, and how it ended.
    ("Relayed 1 file to {} for this device · {*}", "1 fichier transmis à {} pour cet appareil · {*}"),
    ("Relayed {} files to {} for this device · {*}", "{} fichiers transmis à {} pour cet appareil · {*}"),
    ("Relayed 1 file to {} · {*}", "1 fichier transmis à {} · {*}"),
    ("Relayed {} files to {} · {*}", "{} fichiers transmis à {} · {*}"),
    ("delivered", "livré"),
    ("cancelled", "annulé"),
    ("expired after 7 days", "expiré après 7 jours"),
    ("didn't go through: {*}", "n’a pas abouti : {*}"),
    ("didn't go through", "n’a pas abouti"),
    ("{} declined", "{} a refusé"),
    // ---------- My devices ----------
    ("Give this PC a name (like Desk PC).", "Donnez un nom à ce PC (comme PC bureau)."),
    ("Name this PC first: your other PC sees it.", "Nommez d’abord ce PC : votre autre PC voit ce nom."),
    ("Answer this in Pako's window.", "Répondez dans la fenêtre de Pako."),
    (
        "That isn't a Pako ID. Copy it again from the other PC (My devices › Add a device).",
        "Ce n’est pas un ID Pako. Copiez-le à nouveau depuis l’autre PC (Mes appareils › Ajouter un appareil).",
    ),
    ("That's this PC's own Pako ID. Enter the other PC's.", "C’est l’ID Pako de ce PC. Saisissez celui de l’autre PC."),
    ("This PC is already one of your devices.", "Ce PC fait déjà partie de vos appareils."),
    ("Give this device a name (like Laptop or Home PC).", "Donnez un nom à cet appareil (comme Portable ou PC maison)."),
    ("Paired as your device", "Associé comme votre appareil"),
    ("Allowed to control this Pako", "Autorisé à contrôler ce Pako"),
    ("No longer allowed to control this Pako", "N’est plus autorisé à contrôler ce Pako"),
    ("Pick a folder.", "Choisissez un dossier."),
    ("That device isn't connected right now.", "Cet appareil n’est pas connecté pour le moment."),
    (
        "{} doesn't let this PC control it. On {}, turn on \"… can control my Pako\" for this PC.",
        "{0} ne permet pas à ce PC de le contrôler. Sur {1}, activez « … peut contrôler mon Pako » pour ce PC.",
    ),
    // (As an older Pako said it.)
    (
        "{} doesn't let this PC control it. On {}, turn on \"This device can control my Pako\" for this PC.",
        "{0} ne permet pas à ce PC de le contrôler. Sur {1}, activez « Cet appareil peut contrôler mon Pako » pour ce PC.",
    ),
    ("It didn't work.", "Cela n’a pas marché."),
    ("Only your own devices can do that.", "Seuls vos propres appareils peuvent faire cela."),
    ("This device isn't paired both ways.", "Cet appareil n’est pas associé dans les deux sens."),
    ("This device isn't allowed to control that Pako.", "Cet appareil n’a pas le droit de contrôler ce Pako."),
    ("Too many orders in a short time. Try again in a minute.", "Trop de commandes en peu de temps. Réessayez dans une minute."),
    ("That Pako can't take orders right now.", "Ce Pako ne peut pas prendre de commandes pour le moment."),
    ("That download isn't in the list any more.", "Ce téléchargement n’est plus dans la liste."),
    ("Unknown priority.", "Priorité inconnue."),
    ("That Pako doesn't know this order. Update Pako there.", "Ce Pako ne connaît pas cette commande. Mettez Pako à jour là-bas."),
    ("{} isn't finished, or isn't there any more.", "{} n’est pas terminé, ou n’est plus là."),
    // What an order did there (My devices' history, Links cards).
    ("Priority of {}: {*}", "Priorité de {} : {*}"),
    ("First", "D’abord"),
    ("Normal", "Normale"),
    ("Later", "Plus tard"),
    ("Paused {}", "Mis en pause : {}"),
    ("Resumed {}", "Repris : {}"),
    ("Removed {} from the list", "Retiré de la liste : {}"),
    ("Sending {}", "Envoi de {}"),
    ("{*}. A folder name wasn't safe: saved in the download folder itself", "{*}. Un nom de dossier n’était pas sûr : enregistré directement dans le dossier de téléchargement"),
    ("{*}. Not taken: {} ({*})", "{*}. Non accepté : {} ({*})"),
    ("Added 1 link to the downloads, {} to Links", "1 lien ajouté aux téléchargements, {} à Liens"),
    ("Added {} links to the downloads, {} to Links", "{} liens ajoutés aux téléchargements, {} à Liens"),
    ("Added 1 link to the downloads", "1 lien ajouté aux téléchargements"),
    ("Added {} links to the downloads", "{} liens ajoutés aux téléchargements"),
    ("Added 1 link to Links", "1 lien ajouté à Liens"),
    ("Added {} links to Links", "{} liens ajoutés à Liens"),
    ("Nothing was added.", "Rien n’a été ajouté."),
    ("http{}: {*} (and {} more)", "http{} : {*} (et {} de plus)"),
    ("http{}: {*}", "http{} : {*}"),
    ("Pako isn't ready yet.", "Pako n’est pas encore prêt."),
    ("not added", "non ajouté"),
    ("Pako there was still busy adding the others: send it again", "Le Pako de là-bas était encore occupé à ajouter les autres : envoyez-le à nouveau"),
    ("Refused: {*} ({*})", "Refusé : {*} ({*})"),
    // An order in a few words, alone or with what that PC said.
    ("Add 1 link: {*}", "Ajouter 1 lien : {*}"),
    ("Add {} links: {*}", "Ajouter {} liens : {*}"),
    ("Change the priority: {*}", "Changer la priorité : {*}"),
    ("Pause: {*}", "Mettre en pause : {*}"),
    ("Resume: {*}", "Reprendre : {*}"),
    ("Remove: {*}", "Retirer : {*}"),
    ("Get a file: {*}", "Récupérer un fichier : {*}"),
    ("An unknown order: {*}", "Une commande inconnue : {*}"),
    ("Add 1 link", "Ajouter 1 lien"),
    ("Add {} links", "Ajouter {} liens"),
    ("Change the priority", "Changer la priorité"),
    ("Pause", "Mettre en pause"),
    ("Resume", "Reprendre"),
    ("Remove", "Retirer"),
    ("Get a file", "Récupérer un fichier"),
    ("An unknown order", "Une commande inconnue"),
    // The window's commands for My devices.
    ("No web link in what you pasted.", "Aucun lien web dans ce que vous avez collé."),
    ("Paste at least one link.", "Collez au moins un lien."),
    ("Up to {} links at once.", "Jusqu’à {} liens à la fois."),
    ("Not a web link: {}", "Pas un lien web : {}"),
    ("A link is far too long.", "Un lien est beaucoup trop long."),
    ("Links to the PC itself (localhost) aren't taken from another PC: {}", "Les liens vers le PC lui-même (localhost) ne sont pas acceptés depuis un autre PC : {}"),
    (
        "This folder name can't be used there: {}. Give a folder inside its download folder, like Course or Course/Week 3 (no \"..\", no drive or \":\", two levels at most).",
        "Ce nom de dossier ne peut pas être utilisé là-bas : {}. Indiquez un dossier dans son dossier de téléchargement, comme Cours ou Cours/Semaine 3 (pas de « .. », pas de lecteur ni de « : », deux niveaux au plus).",
    ),
    ("Pako is still starting.", "Pako démarre encore."),
    ("Nothing to send.", "Rien à envoyer."),
    ("This download is no longer waiting.", "Ce téléchargement n’attend plus."),
    ("A video caught on a page downloads on this PC only.", "Une vidéo repérée sur une page se télécharge uniquement sur ce PC."),
    ("Unknown action.", "Action inconnue."),
    ("Unknown choice.", "Choix inconnu."),
];

/// The line in this PC's language (as it is when this table doesn't know it).
pub fn local(s: &str) -> String {
    if !crate::i18n::is_fr() {
        return s.to_string();
    }
    french(s, 0).unwrap_or_else(|| s.to_string())
}

/// `local`, on a line that may be missing.
pub fn local_opt(s: &mut Option<String>) {
    if let Some(x) = s {
        *x = local(x);
    }
}

/// A contact's name as shown: the placeholder for one who gave no name ("Someone") in this
/// PC's language. The saved and sent name stays the English placeholder (Direct knows it by it).
pub fn who(name: &str) -> String {
    if name == super::node::PLACEHOLDER && crate::i18n::is_fr() {
        "Quelqu’un".to_string()
    } else {
        name.to_string()
    }
}

/// `who` inside a sentence (« envoyé par quelqu’un »): the French stand-in without its capital.
pub fn who_inside(name: &str) -> String {
    if name == super::node::PLACEHOLDER && crate::i18n::is_fr() {
        "quelqu’un".to_string()
    } else {
        name.to_string()
    }
}

fn who_in(c: &mut super::store::Contact) {
    c.name = who(&c.name);
    if let Some(n) = &mut c.their_name {
        *n = who(n);
    }
}

/// For `map_err`: an error in this PC's language.
pub fn say(e: String) -> String {
    local(&e)
}

fn french(s: &str, depth: u8) -> Option<String> {
    if let Some(size) = size_fr(s) {
        return Some(size);
    }
    // The engine's words that travel too (why a link wasn't added, a download's message).
    if let Some(fr) = crate::model::french_of(s) {
        return Some(fr.to_string());
    }
    for (en, fr) in LINES {
        if let Some(caps) = fit(en, s) {
            let caps: Vec<String> = caps
                .into_iter()
                .map(|(c, deep)| if deep && depth < 4 { french(c, depth + 1).unwrap_or_else(|| c.to_string()) } else { c.to_string() })
                .collect();
            return Some(fill(fr, &caps));
        }
    }
    None
}

/// A size as `node::human` writes it ("1.5 GB", "512 B"), in French ("1,5 Go", "512 o").
fn size_fr(s: &str) -> Option<String> {
    let (num, unit) = s.split_once(' ')?;
    let unit = match unit {
        "B" => "o",
        "KB" => "Ko",
        "MB" => "Mo",
        "GB" => "Go",
        "TB" => "To",
        _ => return None,
    };
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(format!("{} {unit}", num.replace('.', ",")))
}

enum Piece<'a> {
    Words(&'a str),
    /// A place for other words; `true`: put in French too.
    Hole(bool),
}

fn pieces(pattern: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut rest = pattern;
    loop {
        let next = match (rest.find("{}"), rest.find("{*}")) {
            (Some(a), Some(b)) if b < a => Some((b, 3, true)),
            (Some(a), _) => Some((a, 2, false)),
            (None, Some(b)) => Some((b, 3, true)),
            (None, None) => None,
        };
        let Some((at, len, deep)) = next else {
            if !rest.is_empty() {
                out.push(Piece::Words(rest));
            }
            return out;
        };
        if at > 0 {
            out.push(Piece::Words(&rest[..at]));
        }
        out.push(Piece::Hole(deep));
        rest = &rest[at + len..];
    }
}

/// The words in the places of `pattern`, if `s` is that line.
fn fit<'s>(pattern: &str, s: &'s str) -> Option<Vec<(&'s str, bool)>> {
    let ps = pieces(pattern);
    let mut caps = Vec::new();
    fits(&ps, s, &mut caps).then_some(caps)
}

/// (A place is never empty; the first way that fits, leftmost first, wins.)
fn fits<'s>(ps: &[Piece<'_>], s: &'s str, caps: &mut Vec<(&'s str, bool)>) -> bool {
    match ps.first() {
        None => s.is_empty(),
        Some(Piece::Words(w)) => s.strip_prefix(w).is_some_and(|rest| fits(&ps[1..], rest, caps)),
        Some(Piece::Hole(deep)) => match ps.get(1) {
            None => {
                if s.is_empty() {
                    return false;
                }
                caps.push((s, *deep));
                true
            }
            Some(Piece::Words(w)) => {
                for (at, _) in s.match_indices(w) {
                    if at == 0 {
                        continue;
                    }
                    caps.push((&s[..at], *deep));
                    if fits(&ps[1..], &s[at..], caps) {
                        return true;
                    }
                    caps.pop();
                }
                false
            }
            // (Two places in a row are never written.)
            Some(Piece::Hole(_)) => false,
        },
    }
}

fn fill(fr: &str, caps: &[String]) -> String {
    let mut out = String::new();
    let mut next = 0;
    let mut rest = fr;
    while let Some(at) = rest.find('{') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        if let Some(end) = after.find('}') {
            let inner = &after[..end];
            let k = if inner.is_empty() || inner == "*" {
                next += 1;
                Some(next - 1)
            } else {
                inner.parse::<usize>().ok()
            };
            if let Some(c) = k.and_then(|k| caps.get(k)) {
                out.push_str(c);
                rest = &after[end + 1..];
                continue;
            }
        }
        out.push('{');
        rest = after;
    }
    out.push_str(rest);
    out
}

// ---------- the views, in this PC's language ----------

fn in_history(list: &mut [super::store::HistoryItem]) {
    for h in list {
        local_opt(&mut h.message);
        local_opt(&mut h.via);
    }
}

fn in_transfers(list: &mut [super::node::TransferView]) {
    for t in list {
        local_opt(&mut t.message);
        local_opt(&mut t.via);
    }
}

fn in_relays(list: &mut [super::node::relay::RelayView]) {
    for r in list {
        local_opt(&mut r.message);
        r.to_name = who(&r.to_name);
        r.device_name = who(&r.device_name);
    }
}

/// Direct's view (the Direct page).
pub fn in_state(mut s: super::node::State) -> super::node::State {
    if !crate::i18n::is_fr() {
        return s;
    }
    for c in &mut s.contacts {
        local_opt(&mut c.note);
        who_in(&mut c.contact);
    }
    for r in &mut s.requests {
        r.name = who(&r.name);
    }
    for o in &mut s.offers {
        local_opt(&mut o.asked_because);
        o.contact_name = who(&o.contact_name);
    }
    in_transfers(&mut s.transfers);
    in_history(&mut s.history);
    in_relays(&mut s.relays);
    s
}

/// My devices' view.
pub fn in_devices(mut s: super::node::devices::DevicesState) -> super::node::devices::DevicesState {
    if !crate::i18n::is_fr() {
        return s;
    }
    for d in &mut s.devices {
        local_opt(&mut d.view.note);
        who_in(&mut d.view.contact);
    }
    for r in &mut s.requests {
        r.name = who(&r.name);
    }
    in_transfers(&mut s.transfers);
    in_history(&mut s.history);
    in_relays(&mut s.relays);
    for l in &mut s.log {
        l.text = local(&l.text);
    }
    for c in &mut s.sent {
        c.text = local(&c.text);
    }
    for r in s.remote.values_mut() {
        for i in &mut r.items {
            local_opt(&mut i.message);
        }
    }
    s
}

/// Another of your PCs' history with one of its contacts (its lines, in this PC's language).
pub fn in_talk(mut t: super::node::relay::RemoteTalk) -> super::node::relay::RemoteTalk {
    if !crate::i18n::is_fr() {
        return t;
    }
    in_history(&mut t.history);
    for m in &mut t.moving {
        local_opt(&mut m.message);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_stays_as_it_is() {
        crate::i18n::test_french(false);
        assert_eq!(local("They declined it."), "They declined it.");
        assert_eq!(local("Paused Movie.mkv"), "Paused Movie.mkv");
    }

    #[test]
    fn lines_in_french() {
        crate::i18n::test_french(true);
        assert_eq!(local("They declined it."), "Le destinataire l’a refusé.");
        assert_eq!(local("Paused Movie: the end.mkv"), "Mis en pause\u{a0}: Movie: the end.mkv");
        assert_eq!(local("Priority of a.mp4: First"), "Priorité de a.mp4\u{a0}: D’abord");
        assert_eq!(local("Added 1 link to the downloads"), "1 lien ajouté aux téléchargements");
        assert_eq!(local("Added 3 links to the downloads, 2 to Links"), "3 liens ajoutés aux téléchargements, 2 à Liens");
        assert_eq!(
            local("Added 2 links to the downloads. Not taken: 1 (https://x.test/a: Not a web link: y)"),
            "2 liens ajoutés aux téléchargements. Non accepté\u{a0}: 1 (https://x.test/a\u{a0}: Pas un lien web\u{a0}: y)"
        );
        assert_eq!(local("Relayed 3 files to Rado · didn't go through: The connection was lost."), "3 fichiers transmis à Rado · n’a pas abouti\u{a0}: La connexion a été perdue.");
        assert_eq!(local("Relayed 1 file to Rado for this device · Rado declined"), "1 fichier transmis à Rado pour cet appareil · Rado a refusé");
        assert_eq!(local("Pause: Only your own devices can do that."), "Mettre en pause\u{a0}: Seuls vos propres appareils peuvent faire cela.");
        assert_eq!(
            local("Not enough room: this needs 1.5 GB, 200.0 MB is free there (Pako always leaves 1.0 GB free)."),
            "Pas assez de place\u{a0}: il faut 1,5 Go, il y reste 200,0 Mo de libre (Pako laisse toujours 1,0 Go de libre)."
        );
        assert_eq!(local("Home PC doesn't let this PC control it. On Home PC, turn on \"… can control my Pako\" for this PC."), "Home PC ne permet pas à ce PC de le contrôler. Sur Home PC, activez «\u{a0}… peut contrôler mon Pako\u{a0}» pour ce PC.");
        assert_eq!(local("Home PC doesn't let this PC control it. On Home PC, turn on \"This device can control my Pako\" for this PC."), "Home PC ne permet pas à ce PC de le contrôler. Sur Home PC, activez «\u{a0}Cet appareil peut contrôler mon Pako\u{a0}» pour ce PC.");
        // The engine's words that travel: why a link wasn't added there, worded here.
        assert_eq!(local("Nothing was added."), "Rien n’a été ajouté.");
        assert_eq!(local("https://x.test/a: the folder is empty"), "https://x.test/a\u{a0}: le dossier est vide");
        assert_eq!(crate::model::in_english("le dossier est vide"), "the folder is empty");
        assert_eq!(crate::model::in_english("Something else"), "Something else");
        // Messages that start with travelling words and end with a code.
        assert_eq!(crate::model::in_english("Fichier introuvable sur le serveur (404)."), "File not found on the server (404).");
        assert_eq!(crate::model::french_of("File not found on the server (404).").as_deref(), Some("Fichier introuvable sur le serveur (404)."));
        assert_eq!(
            crate::model::in_english("Le serveur a refusé l’accès (403). Le lien a peut-être expiré."),
            "The server refused access (403). The link may have expired."
        );
        assert_eq!(crate::model::in_english("Erreur du serveur (502)"), "Server error (502)");
        assert_eq!(crate::model::french_of("The server answered 418").as_deref(), Some("Le serveur a répondu 418"));
        // Not a code after the words: left as it is.
        assert_eq!(crate::model::in_english("Erreur du serveur de Google"), "Erreur du serveur de Google");
        // A line from a newer Pako: as it came.
        assert_eq!(local("Something new happened."), "Something new happened.");
        // The placeholder for a contact who gave no name, in French; a real name as it is.
        assert_eq!(who("Someone"), "Quelqu’un");
        assert_eq!(who("Rado"), "Rado");
        crate::i18n::test_french(false);
        assert_eq!(who("Someone"), "Someone");
    }

    #[test]
    fn every_line_has_its_places_in_french() {
        for (en, fr) in LINES {
            let holes = pieces(en).iter().filter(|p| matches!(p, Piece::Hole(_))).count();
            let places = fr.matches('{').count();
            assert_eq!(holes, places, "{en}");
            assert!(!fr.contains(" :") && !fr.contains(" ;") && !fr.contains(" ?") && !fr.contains(" !") && !fr.contains("« ") && !fr.contains(" »") && !fr.contains('\''), "French typography: {fr}");
        }
    }
}
