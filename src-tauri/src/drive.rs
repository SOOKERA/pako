// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Google Drive without signing in: reading links, file details and the contents of public
//! folders (through Google's Drive API with Pako's own Drive-only app key), and the public
//! download addresses Pako fetches the files from.

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;

const API: &str = "https://www.googleapis.com/drive/v3";
const FIELDS: &str = "id,name,size,md5Checksum,mimeType,resourceKey,shortcutDetails(targetId,targetMimeType,targetResourceKey)";
pub const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const SHORTCUT_MIME: &str = "application/vnd.google-apps.shortcut";

/// Said when Google won't show a folder's contents to Pako (only shared with some people).
pub const FOLDER_NOT_PUBLIC: &str =
    "This folder isn't shared with \"anyone with the link\". Open it in your browser and download from there: Pako catches the download.";

/// Said when Google refuses Pako's own app key (blocked, switched off, out of its daily
/// allowance): the folder may well be public, Pako just can't list it at the moment.
pub const CANT_LIST_NOW: &str =
    "Pako can't list Drive folders right now. Open the folder in your browser and download from there: Pako catches the download.";

/// Said when this copy of Pako was built without its app key.
pub const NO_KEY: &str =
    "This copy of Pako can't list Drive folders. Open the folder in your browser and download from there: Pako catches the download.";

// The same three in French (`folder_browser_message`, `no_key` pick the language of the moment).
pub(crate) const FOLDER_NOT_PUBLIC_FR: &str =
    "Ce dossier n’est pas partagé avec «\u{a0}tous les utilisateurs disposant du lien\u{a0}». Ouvrez-le dans votre navigateur et téléchargez depuis là-bas\u{a0}: Pako récupère le téléchargement.";
pub(crate) const CANT_LIST_NOW_FR: &str =
    "Pako ne peut pas lister les dossiers Drive pour le moment. Ouvrez le dossier dans votre navigateur et téléchargez depuis là-bas\u{a0}: Pako récupère le téléchargement.";
pub(crate) const NO_KEY_FR: &str =
    "Cette copie de Pako ne peut pas lister les dossiers Drive. Ouvrez le dossier dans votre navigateur et téléchargez depuis là-bas\u{a0}: Pako récupère le téléchargement.";

fn in_lang(en: &'static str, fr: &'static str) -> &'static str {
    if crate::i18n::is_fr() { fr } else { en }
}

/// `NO_KEY` in the language of the moment (for engine.rs and found.rs, which still say `NO_KEY`).
#[allow(dead_code)]
pub fn no_key() -> &'static str {
    in_lang(NO_KEY, NO_KEY_FR)
}

/// Pako's Drive-only app key: built into Pako (from the gitignored signing/drive_api_key.txt,
/// see build.rs), else data\drive_api_key.txt (for builds made without it). Never shown.
pub fn api_key() -> Option<String> {
    if let Some(k) = option_env!("PAKO_DRIVE_API_KEY").map(str::trim).filter(|k| !k.is_empty()) {
        return Some(k.to_string());
    }
    std::fs::read_to_string(crate::paths::data_dir().join("drive_api_key.txt"))
        .ok()
        .map(|k| k.trim_start_matches('\u{feff}').trim().to_string())
        .filter(|k| !k.is_empty())
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkKind {
    File,
    Folder,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DriveLink {
    pub id: String,
    pub kind: LinkKind,
    pub resource_key: Option<String>,
    /// "document", "spreadsheets" or "presentation" for docs.google.com links.
    pub doc_type: Option<String>,
}

/// Recognises Drive links: /file/d/ID, /folders/ID, open?id=ID, uc?id=ID,
/// drive.usercontent.google.com/download?id=ID, docs.google.com/document/d/ID ...
pub fn parse_link(link: &str) -> Option<DriveLink> {
    let url = url::Url::parse(link.trim()).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    if !(host == "drive.google.com" || host == "docs.google.com" || host == "drive.usercontent.google.com") {
        return None;
    }
    let query = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.to_string());
    let resource_key = query("resourcekey");
    let segs: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();

    let after = |name: &str| -> Option<String> {
        segs.iter().position(|s| *s == name).and_then(|i| segs.get(i + 1)).map(|s| s.to_string())
    };
    let (id, kind) = if let Some(id) = after("folders") {
        (id, LinkKind::Folder)
    } else if segs.contains(&"file") || segs.contains(&"document") || segs.contains(&"spreadsheets") || segs.contains(&"presentation") {
        (after("d")?, LinkKind::File)
    } else if let Some(id) = query("id") {
        let kind = if segs.contains(&"uc") || segs.contains(&"download") { LinkKind::File } else { LinkKind::Unknown };
        (id, kind)
    } else {
        return None;
    };
    let valid = id.len() >= 10 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    let doc_type = if host == "docs.google.com" {
        ["document", "spreadsheets", "presentation"].into_iter().find(|t| segs.contains(t)).map(String::from)
    } else {
        None
    };
    valid.then_some(DriveLink { id, kind, resource_key, doc_type })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub id: String,
    pub name: String,
    #[serde(default, deserialize_with = "de_opt_u64")]
    pub size: Option<u64>,
    pub md5_checksum: Option<String>,
    pub mime_type: String,
    pub resource_key: Option<String>,
    pub shortcut_details: Option<ShortcutDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutDetails {
    pub target_id: String,
    pub target_resource_key: Option<String>,
}

fn de_opt_u64<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Option<u64>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    Ok(s.and_then(|v| v.parse().ok()))
}

impl FileMeta {
    pub fn is_folder(&self) -> bool {
        self.mime_type == FOLDER_MIME
    }
}

/// For Google Docs/Sheets/Slides/Drawings: how Google's public export link names the kind
/// ("document"…) and the extension of what it exports (Word, Excel, PowerPoint, PNG).
pub fn export_for(mime: &str) -> Option<(&'static str, &'static str)> {
    Some(match mime {
        "application/vnd.google-apps.document" => ("document", "docx"),
        "application/vnd.google-apps.spreadsheet" => ("spreadsheets", "xlsx"),
        "application/vnd.google-apps.presentation" => ("presentation", "pptx"),
        "application/vnd.google-apps.drawing" => ("drawings", "png"),
        _ => return None,
    })
}

pub fn is_google_native(mime: &str) -> bool {
    mime.starts_with("application/vnd.google-apps.")
}

/// Public (no sign-in) download URL. Works for "anyone with the link" files.
/// Docs/Sheets/Slides/Drawings use Google's public export link instead.
pub fn public_url(file_id: &str, resource_key: Option<&str>, doc_type: Option<&str>) -> String {
    let mut u = match doc_type {
        Some("document") => format!("https://docs.google.com/document/d/{file_id}/export?format=docx"),
        Some("spreadsheets") => format!("https://docs.google.com/spreadsheets/d/{file_id}/export?format=xlsx"),
        Some("presentation") => format!("https://docs.google.com/presentation/d/{file_id}/export?format=pptx"),
        Some("drawings") => format!("https://docs.google.com/drawings/d/{file_id}/export/png?"),
        _ => format!("https://drive.usercontent.google.com/download?id={file_id}&export=download&confirm=t"),
    };
    if let Some(rk) = resource_key {
        if !u.ends_with('?') {
            u.push('&');
        }
        u.push_str("resourcekey=");
        u.push_str(&encode_key(rk));
    }
    u.trim_end_matches('?').to_string()
}

/// The file's page in Google Drive (to open in the browser: download there, or make a copy).
pub fn view_url(file_id: &str, resource_key: Option<&str>) -> String {
    match resource_key {
        Some(rk) => format!("https://drive.google.com/file/d/{file_id}/view?resourcekey={}", encode_key(rk)),
        None => format!("https://drive.google.com/file/d/{file_id}/view"),
    }
}

/// A resource key as it goes in a link (it comes from the user's link, already decoded: an
/// "&" or "#" in it would break the link). Usual keys ("0-abc_DEF") stay as they are.
fn encode_key(rk: &str) -> String {
    const KEEP: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~');
    percent_encoding::utf8_percent_encode(rk, KEEP).to_string()
}

pub fn resource_key_header(file_id: &str, resource_key: Option<&str>) -> Option<String> {
    resource_key.map(|rk| format!("{file_id}/{rk}"))
}

/// Error reason from a Drive API error body, e.g. "downloadQuotaExceeded".
pub fn error_reason(body: &str) -> (Option<String>, Option<String>) {
    #[derive(Deserialize)]
    struct E {
        error: Inner,
    }
    #[derive(Deserialize)]
    struct Inner {
        message: Option<String>,
        #[serde(default)]
        errors: Vec<Item>,
    }
    #[derive(Deserialize)]
    struct Item {
        reason: Option<String>,
    }
    match serde_json::from_str::<E>(body) {
        Ok(e) => (e.error.errors.into_iter().find_map(|i| i.reason), e.error.message),
        Err(_) => (None, None),
    }
}

/// Google won't show this item to Pako: it isn't shared with "anyone with the link"
/// (or doesn't exist). Callers offer to open it in the browser instead.
#[derive(Debug)]
pub struct NotPublic;

impl std::fmt::Display for NotPublic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(in_lang("This isn't shared with \"anyone with the link\".", "Ce n’est pas partagé avec «\u{a0}tous les utilisateurs disposant du lien\u{a0}»."))
    }
}

impl std::error::Error for NotPublic {}

/// Google refused Pako's app key itself (not the item): blocked, switched off, expired, or out
/// of its daily allowance. Callers offer to open the item in the browser too.
#[derive(Debug)]
pub struct KeyRefused;

impl std::fmt::Display for KeyRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(in_lang("Google refused Pako's Drive key.", "Google a refusé la clé Drive de Pako."))
    }
}

impl std::error::Error for KeyRefused {}

/// What to tell about a folder Pako couldn't list, when opening it in the browser is the way
/// out (not shared publicly, or Pako's key refused); None for other errors.
pub fn folder_browser_message(e: &anyhow::Error) -> Option<&'static str> {
    if e.downcast_ref::<NotPublic>().is_some() {
        Some(in_lang(FOLDER_NOT_PUBLIC, FOLDER_NOT_PUBLIC_FR))
    } else if e.downcast_ref::<KeyRefused>().is_some() {
        Some(in_lang(CANT_LIST_NOW, CANT_LIST_NOW_FR))
    } else {
        None
    }
}

/// A Drive API answer that means the item isn't visible without an account. Only "not
/// found" says that: with an app key, Google answers that for items that aren't public.
fn means_not_public(status: u16, reason: Option<&str>) -> bool {
    status == 404 || matches!(reason, Some("notFound" | "insufficientFilePermissions"))
}

/// A Drive API answer about Pako's app key (or its Google project), not about the item.
fn means_key_refused(status: u16, reason: Option<&str>) -> bool {
    let key_reason = matches!(
        reason,
        Some(
            "accessNotConfigured"
                | "keyInvalid"
                | "keyExpired"
                | "ipRefererBlocked"
                | "forbidden"
                | "quotaExceeded"
                | "dailyLimitExceeded"
                | "dailyLimitExceededUnreg"
                | "accessDenied"
        )
    );
    // A refusal Google doesn't explain is about the key too: the item's own refusals
    // (not found, no permission) are covered above.
    key_reason || status == 401 || (status == 403 && reason.is_none())
}

/// The listing request for one page of a folder's contents.
fn list_url(folder_id: &str, page_token: Option<&str>, key: &str) -> String {
    let q = format!("'{folder_id}' in parents and trashed = false");
    let enc = |s: &str| percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string();
    let mut url = format!(
        "{API}/files?q={}&fields=nextPageToken,files({FIELDS})&pageSize=1000&supportsAllDrives=true&includeItemsFromAllDrives=true&key={}",
        enc(&q),
        enc(key)
    );
    if let Some(t) = page_token {
        url.push_str("&pageToken=");
        url.push_str(&enc(t));
    }
    url
}

/// Reads public Drive items with Pako's app key (no sign-in, no user data).
pub struct DriveApi<'a> {
    pub http: &'a reqwest::Client,
    pub key: &'a str,
}

impl<'a> DriveApi<'a> {
    /// GET a Drive API answer. Short hiccups ("slow down", Google busy, network blip) are
    /// tried again a few times (1s, 2s, 4s, 8s, 16s), so adding a big folder doesn't fail
    /// because of one bad moment in hundreds of requests. "Not visible" gives `NotPublic`.
    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str, rk_header: Option<String>) -> Result<T> {
        const MAX_RETRIES: u32 = 5;
        let mut retries = 0;
        loop {
            let mut req = self.http.get(url);
            if let Some(h) = &rk_header {
                req = req.header("X-Goog-Drive-Resource-Keys", h);
            }
            // (error, seconds Google asked us to wait) for answers worth trying again.
            let failure: (anyhow::Error, Option<u64>) = match req.send().await {
                Err(e) => (anyhow!(crate::tr!("Could not reach Google Drive: {}", "Impossible de joindre Google Drive : {}", crate::fetch::network_message(&e))), None),
                // An answer cut off on the way (a network blip): tried again like the rest, so
                // page 40 of a big folder doesn't fail the whole folder.
                Ok(resp) if resp.status().is_success() => match resp.json::<T>().await {
                    Ok(v) => return Ok(v),
                    Err(_) => (anyhow!(crate::tr!("Google Drive's answer didn't arrive in full. Try again.", "La réponse de Google Drive n’est pas arrivée en entier. Réessayez.")), None),
                },
                Ok(resp) => {
                    let status = resp.status();
                    let retry_after = resp
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.trim().parse::<u64>().ok())
                        .map(|s| s.min(60));
                    let body = resp.text().await.unwrap_or_default();
                    let (reason, message) = error_reason(&body);
                    if means_not_public(status.as_u16(), reason.as_deref()) {
                        return Err(NotPublic.into());
                    }
                    if means_key_refused(status.as_u16(), reason.as_deref()) {
                        return Err(KeyRefused.into());
                    }
                    let msg = friendly_api_error(status.as_u16(), reason.as_deref(), message.as_deref());
                    let slow_down = matches!(
                        reason.as_deref(),
                        Some("rateLimitExceeded" | "userRateLimitExceeded" | "sharingRateLimitExceeded")
                    );
                    if !(status.as_u16() == 429 || status.is_server_error() || slow_down) {
                        bail!(msg);
                    }
                    (anyhow!(msg), retry_after)
                }
            };
            if retries >= MAX_RETRIES {
                return Err(failure.0);
            }
            let wait = failure.1.unwrap_or(1 << retries).max(1);
            retries += 1;
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }
    }

    pub async fn meta(&self, id: &str, resource_key: Option<&str>) -> Result<FileMeta> {
        let key = percent_encoding::utf8_percent_encode(self.key, percent_encoding::NON_ALPHANUMERIC);
        let url = format!("{API}/files/{id}?fields={FIELDS}&supportsAllDrives=true&key={key}");
        let mut m: FileMeta = self.get_json(&url, resource_key_header(id, resource_key)).await?;
        // Follow shortcuts to the real file.
        if m.mime_type == SHORTCUT_MIME {
            if let Some(sc) = m.shortcut_details.clone() {
                let target = Box::pin(self.meta(&sc.target_id, sc.target_resource_key.as_deref())).await?;
                m = FileMeta { name: m.name, ..target };
            }
        }
        Ok(m)
    }

    /// All files inside a folder (and its subfolders), with their path relative to the folder.
    pub async fn list_recursive(&self, folder_id: &str, resource_key: Option<&str>) -> Result<Vec<(PathBuf, FileMeta)>> {
        let mut out = Vec::new();
        // Folders already listed: a shortcut pointing back up (a loop) is listed only once,
        // so listing always ends.
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack = vec![(folder_id.to_string(), resource_key.map(String::from), PathBuf::new())];
        while let Some((id, rk, rel)) = stack.pop() {
            if !visited.insert(id.clone()) {
                continue;
            }
            for child in self.list_children(&id, rk.as_deref()).await? {
                let mut child = child;
                if child.mime_type == SHORTCUT_MIME {
                    match self.meta(&child.id, child.resource_key.as_deref()).await {
                        Ok(m) => child = m,
                        Err(_) => continue,
                    }
                }
                let name = crate::names::sanitize(&child.name);
                if child.is_folder() {
                    stack.push((child.id.clone(), child.resource_key.clone(), rel.join(name)));
                } else {
                    out.push((rel.clone(), child));
                }
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.name.cmp(&b.1.name)));
        Ok(out)
    }

    async fn list_children(&self, folder_id: &str, resource_key: Option<&str>) -> Result<Vec<FileMeta>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Page {
            next_page_token: Option<String>,
            #[serde(default)]
            files: Vec<FileMeta>,
        }
        let mut files = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let url = list_url(folder_id, page_token.as_deref(), self.key);
            let page: Page = self.get_json(&url, resource_key_header(folder_id, resource_key)).await?;
            files.extend(page.files);
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => break,
            }
        }
        Ok(files)
    }
}

pub fn friendly_api_error(status: u16, reason: Option<&str>, message: Option<&str>) -> String {
    match (status, reason) {
        (_, Some("downloadQuotaExceeded")) => {
            crate::tr!("Google's download limit for this shared file is reached for today.", "La limite de téléchargement de Google pour ce fichier partagé est atteinte pour aujourd’hui.")
        }
        (_, Some("cannotDownloadAbusiveFile")) => {
            crate::tr!("Google flagged this file as possibly harmful and blocks downloading it.", "Google a signalé ce fichier comme peut-être dangereux et bloque son téléchargement.")
        }
        (_, Some("rateLimitExceeded" | "userRateLimitExceeded" | "sharingRateLimitExceeded")) => {
            crate::tr!("Google asked us to slow down.", "Google nous a demandé de ralentir.")
        }
        (404, _) => crate::tr!(
            "File not found, or it isn't shared with \"anyone with the link\".",
            "Fichier introuvable, ou il n’est pas partagé avec « tous les utilisateurs disposant du lien »."
        ),
        (403, Some("cannotDownloadFile")) => crate::tr!("The owner turned off downloading for this file.", "Le propriétaire a désactivé le téléchargement de ce fichier."),
        _ => match message {
            Some(m) => crate::tr!("Google Drive error {status}: {m}", "Erreur Google Drive {status} : {m}"),
            None => crate::tr!("Google Drive error {status}", "Erreur Google Drive {status}"),
        },
    }
}

/// A `friendly_api_error` that says Google won't give this file (not found, flagged, turned
/// off by its owner, its daily limit, an error), in either language: another host is tried.
/// (For found.rs's `host_gave_up`, which still reads the English words.)
#[allow(dead_code)]
pub fn is_api_refusal(msg: &str) -> bool {
    [
        "File not found",
        "Google flagged",
        "The owner turned off",
        "Google Drive error",
        "Google's download limit",
        "Fichier introuvable",
        "Google a signalé",
        "Le propriétaire a désactivé",
        "Erreur Google Drive",
        "La limite de téléchargement de Google",
    ]
    .iter()
    .any(|p| msg.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_links_keep_their_type() {
        let d = parse_link("https://docs.google.com/document/d/1AbCdEfGhIjKlMnOp/edit?usp=sharing").unwrap();
        assert_eq!(d.doc_type.as_deref(), Some("document"));
        let s = parse_link("https://docs.google.com/spreadsheets/d/1AbCdEfGhIjKlMnOp/edit").unwrap();
        assert_eq!(s.doc_type.as_deref(), Some("spreadsheets"));
        let f = parse_link("https://drive.google.com/file/d/1AbCdEfGhIjKlMnOp/view").unwrap();
        assert_eq!(f.doc_type, None);
    }

    #[test]
    fn public_links_for_files_and_docs() {
        assert!(public_url("ID1234567890", None, None).starts_with("https://drive.usercontent.google.com/download?id=ID1234567890"));
        assert_eq!(
            public_url("ID1234567890", None, Some("presentation")),
            "https://docs.google.com/presentation/d/ID1234567890/export?format=pptx"
        );
        assert_eq!(public_url("ID1234567890", None, Some("drawings")), "https://docs.google.com/drawings/d/ID1234567890/export/png");
        assert_eq!(
            public_url("ID1234567890", Some("0-rk"), Some("drawings")),
            "https://docs.google.com/drawings/d/ID1234567890/export/png?resourcekey=0-rk"
        );
        assert!(public_url("ID1234567890", Some("0-rk"), None).ends_with("&confirm=t&resourcekey=0-rk"));
        assert_eq!(export_for("application/vnd.google-apps.spreadsheet"), Some(("spreadsheets", "xlsx")));
        assert_eq!(view_url("ID1234567890", None), "https://drive.google.com/file/d/ID1234567890/view");
        // A key with characters that would break the link is encoded; usual keys aren't.
        assert!(public_url("ID1234567890", Some("0-a&b#c"), None).ends_with("&resourcekey=0-a%26b%23c"));
        assert_eq!(view_url("ID1234567890", Some("0-rk_X")), "https://drive.google.com/file/d/ID1234567890/view?resourcekey=0-rk_X");
    }

    #[test]
    fn folder_listing_request() {
        let first = list_url("FOLDER12345", None, "k-1");
        assert!(first.starts_with("https://www.googleapis.com/drive/v3/files?q=%27FOLDER12345%27%20in%20parents%20and%20trashed%20%3D%20false"));
        assert!(first.contains("&pageSize=1000&supportsAllDrives=true&includeItemsFromAllDrives=true&key=k%2D1"));
        assert!(first.contains("fields=nextPageToken,files("));
        assert!(!first.contains("pageToken="));
        let next = list_url("FOLDER12345", Some("tok/en=="), "k-1");
        assert!(next.ends_with("&pageToken=tok%2Fen%3D%3D"));
        assert_eq!(resource_key_header("FOLDER12345", Some("0-abc")).as_deref(), Some("FOLDER12345/0-abc"));
        assert_eq!(resource_key_header("FOLDER12345", None), None);
    }

    #[test]
    fn refusals_mean_not_public() {
        assert!(means_not_public(404, Some("notFound")));
        assert!(means_not_public(404, None));
        assert!(means_not_public(403, Some("insufficientFilePermissions")));
        // Refusals of Pako's key aren't "not shared".
        for r in ["forbidden", "accessNotConfigured", "keyInvalid", "keyExpired", "quotaExceeded", "dailyLimitExceeded"] {
            assert!(!means_not_public(403, Some(r)), "{r}");
            assert!(means_key_refused(403, Some(r)), "{r}");
        }
        assert!(means_key_refused(400, Some("keyInvalid")));
        assert!(means_key_refused(403, None));
        assert!(means_key_refused(401, None));
        assert!(!means_not_public(403, None));
        assert!(!means_not_public(403, Some("rateLimitExceeded")));
        assert!(!means_key_refused(403, Some("rateLimitExceeded")));
        assert!(!means_key_refused(403, Some("downloadQuotaExceeded")));
        assert!(!means_key_refused(404, Some("notFound")));
        assert!(!means_not_public(500, None));
        assert!(!means_not_public(429, None));
        let k: anyhow::Error = KeyRefused.into();
        assert_eq!(folder_browser_message(&k), Some(CANT_LIST_NOW));
        let n: anyhow::Error = NotPublic.into();
        assert_eq!(folder_browser_message(&n), Some(FOLDER_NOT_PUBLIC));
        assert_eq!(folder_browser_message(&anyhow!("other")), None);
    }

    /// Real check against Google (needs the app key and a network): lists one public folder.
    /// Run with PAKO_REAL_DRIVE_FOLDER=<folder link>; the key comes from signing/drive_api_key.txt.
    #[tokio::test]
    #[ignore]
    async fn real_public_folder() {
        let Ok(link) = std::env::var("PAKO_REAL_DRIVE_FOLDER") else { return };
        let key = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../signing/drive_api_key.txt"))
            .map(|k| k.trim().to_string())
            .expect("signing/drive_api_key.txt");
        let dl = parse_link(&link).expect("a Drive folder link");
        let http = reqwest::Client::new();
        let api = DriveApi { http: &http, key: &key };
        let meta = api.meta(&dl.id, dl.resource_key.as_deref()).await.expect("folder details");
        let files = api.list_recursive(&meta.id, dl.resource_key.as_deref()).await.expect("folder listing");
        println!("RESULT folder {:?}: {} files, first: {:?}", meta.name, files.len(), files.first().map(|f| &f.1.name));
        assert!(meta.is_folder());
    }
}
