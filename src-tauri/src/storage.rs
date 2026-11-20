// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Where uploads go, behind one interface (the storage the user picks).
//!
//! Upload, "Delete after", the space bar and, later, Direct's outbox ask a `Store` for what they
//! need (is it connected, how much room, upload, make a folder, delete, set an expiry) without
//! knowing whether it's Gofile or Google Drive. Adding OneDrive or WebDAV later means one more
//! `Host` and its branch in each method here; nothing else changes.

use super::*;

/// Refuses to touch a Drive upload while another Google account than the one it went into is
/// connected (or none). Uploads from before Pako noted the account aren't checked.
pub(super) fn other_account(owner: Option<&str>, connected: Option<&str>) -> Option<String> {
    // Unknown (an older login kept "Google account" when Google didn't say): not compared.
    let owner = owner.filter(|o| !o.is_empty() && *o != "Google account")?;
    if connected.is_some_and(|c| c.eq_ignore_ascii_case(owner)) {
        return None;
    }
    Some(format!("This upload is in the Google Drive of {owner}. Connect that account (Upload › Google Drive), then try again."))
}

/// Gofile's address for contents (delete, update).
pub(super) const GOFILE_API: &str = "https://api.gofile.io/contents";

/// Who deletes an upload when its "Delete after" date comes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExpiryBy {
    /// The service itself, on time even with the PC off (Gofile folders).
    Host,
    /// Pako, the next time it runs after the date (Google Drive has no expiry of its own).
    Pako,
}

/// One place uploads can go.
pub(super) struct Store<'a> {
    s: &'a Sender,
    pub host: Host,
}

impl Sender {
    pub(super) fn store(&self, host: Host) -> Store<'_> {
        Store { s: self, host }
    }

    /// The place a sent item went to.
    pub(super) fn store_of(&self, item: &SentItem) -> Store<'_> {
        self.store(if item.on_drive() { Host::Drive } else { Host::Gofile })
    }
}

impl Store<'_> {
    /// Ready to upload to: Gofile needs no account; Drive needs its sign-in.
    pub async fn connected(&self) -> bool {
        match self.host {
            Host::Gofile => true,
            Host::Drive => self.s.send_google.configured() && self.s.send_google.signed_in().await,
        }
    }

    /// Free room in bytes. Ok(None) = no limit, or the service doesn't say (Gofile).
    pub async fn free_space(&self) -> Result<Option<u64>, String> {
        match self.host {
            Host::Gofile => Ok(None),
            Host::Drive => {
                let (limit, used) = self.s.drive().quota().await.map_err(|f| f.msg)?;
                Ok(limit.map(|l| l.saturating_sub(used)))
            }
        }
    }

    /// Uploads a send: one file, one ZIP, or several files into one folder. Returns the link and
    /// where it is (folder id, file id).
    pub async fn upload(&self, job: &Arc<Job>, payload: Payload, pack: Pack) -> Result<Done, String> {
        // In parts: the ZIP is made as part files first, then they go up like several files
        // (one folder, one link, each part on its own).
        let payload = match payload {
            // "Protect with a password": a 7-Zip archive (in parts when asked), then it goes up
            // like parts (one folder, one link).
            p if job.protected => {
                if self.host == Host::Drive {
                    self.s.drive_ready(job).await?;
                }
                let name = p.name().to_string();
                let files = self.s.make_protected(job, job.mode == super::Mode::Compress).await?;
                Payload::Files { name, entries: files }
            }
            Payload::Zip { name, entries } if job.part_mb > 0 => {
                if self.host == Host::Drive {
                    self.s.drive_ready(job).await?;
                }
                let parts = self.s.make_parts(job, &entries).await?;
                Payload::Files { name, entries: parts }
            }
            p => p,
        };
        match self.host {
            Host::Drive => self.s.run_drive(job, payload, pack).await,
            Host::Gofile => {
                let src = match payload {
                    Payload::Files { entries, .. } => return self.s.run_files(job, entries).await,
                    Payload::File(e) => Source::File(e),
                    Payload::Zip { entries, .. } => Source::Zip(entries, pack),
                };
                let data = self.s.upload(job, &Arc::new(src), &job.name, job.total, None).await.map_err(|f| f.msg)?;
                let link = link_from(&data)?;
                Ok((link, data.parent_folder, data.file_id))
            }
        }
    }

    /// Uploads the rest of a several-files send into its folder ("Try again").
    pub async fn upload_rest(&self, job: &Arc<Job>, left: Vec<Entry>) -> Result<Done, String> {
        match self.host {
            Host::Drive => self.s.run_drive_files(job, left, None).await,
            Host::Gofile => self.s.run_files(job, left).await,
        }
    }

    /// Makes a shared folder and returns (id, link). Gofile makes its folder with the first
    /// upload instead, so it says so.
    pub async fn create_folder(&self, name: &str, parent: Option<&str>) -> Result<(String, String), String> {
        match self.host {
            Host::Drive => {
                let d = self.s.drive();
                let made = d.create_folder(name, parent).await.map_err(|f| f.msg)?;
                let link = d.share_and_link(&made).await?;
                Ok((made.id, link))
            }
            Host::Gofile => Err("Gofile makes its folder with the first file uploaded.".into()),
        }
    }

    /// Deletes what was uploaded (permanently). Already gone counts as done.
    pub async fn delete(&self, item: &SentItem, shared: Option<String>) -> Result<(), String> {
        match self.host {
            Host::Drive => {
                let target = item.folder_id.clone().or(item.file_id.clone()).ok_or("Pako doesn't know where this is in your Google Drive.")?;
                // Another account's Drive would answer "not found", which counts as deleted.
                if let Some(msg) = other_account(item.account.as_deref(), self.s.send_google.email().await.as_deref()) {
                    return Err(msg);
                }
                self.s.drive().delete(&target).await.map_err(|f| f.msg)
            }
            Host::Gofile => {
                let token = token_for(item, shared).ok_or("Pako doesn't have the Gofile key for this file, so it can't delete it.")?;
                let target = item.folder_id.clone().or(item.file_id.clone()).ok_or("Pako doesn't know this file's place on Gofile.")?;
                gofile_delete(&self.s.http, GOFILE_API, &token, &target).await
            }
        }
    }

    /// Sets (Some) or clears (None) when an upload gets deleted. Gofile folders delete
    /// themselves on time; Google Drive has no expiry, so Pako does it.
    pub async fn set_expiry(&self, item: &SentItem, shared: Option<String>, at_ms: Option<u64>) -> Result<ExpiryBy, String> {
        match self.host {
            Host::Drive => Ok(ExpiryBy::Pako),
            Host::Gofile => {
                let token = token_for(item, shared);
                if expiry_by(self.host, item.folder_id.is_some(), token.is_some()) == ExpiryBy::Pako {
                    return Ok(ExpiryBy::Pako);
                }
                let (Some(folder), Some(token)) = (item.folder_id.as_deref(), token) else { return Ok(ExpiryBy::Pako) };
                gofile_expiry(&self.s.http, GOFILE_API, &token, folder, at_ms.map(|ms| ms / 1000)).await?;
                Ok(ExpiryBy::Host)
            }
        }
    }

    /// Direct's outbox (a small note file the receiver reads): comes with Direct part 2.
    #[allow(dead_code)]
    pub async fn write_outbox(&self, _name: &str, _bytes: &[u8]) -> Result<String, String> {
        Err("Not available yet.".into())
    }

    /// Reads someone's outbox note from its public address: comes with Direct part 2.
    #[allow(dead_code)]
    pub async fn read_outbox(&self, _address: &str) -> Result<Vec<u8>, String> {
        Err("Not available yet.".into())
    }
}

/// Who deletes an upload on its date: Gofile folders (every guest upload has one) can be told
/// to, with the upload's key; anything else is up to Pako.
pub(super) fn expiry_by(host: Host, has_folder: bool, has_key: bool) -> ExpiryBy {
    match host {
        Host::Gofile if has_folder && has_key => ExpiryBy::Host,
        _ => ExpiryBy::Pako,
    }
}

/// A delete answer, the same for every service: done, or already gone (404).
pub(super) fn deleted(code: u16, ok_body: bool) -> bool {
    ((200..300).contains(&code) && ok_body) || code == 404
}

/// Deletes a Gofile file or folder. Gone already (404) is fine.
pub(super) async fn gofile_delete(http: &reqwest::Client, base: &str, token: &str, target: &str) -> Result<(), String> {
    let resp = http
        .delete(base)
        .bearer_auth(token)
        .json(&serde_json::json!({ "contentsId": target }))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| net_error(&e))?;
    let code = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let ok_body = serde_json::from_str::<UploadReply>(&text).map(|r| r.status == "ok").unwrap_or(false);
    if deleted(code, ok_body) || text.contains("error-notFound") {
        return Ok(());
    }
    Err(format!("Gofile didn't delete it ({}).", if code >= 300 { code.to_string() } else { "unknown answer".into() }))
}

/// Sets a Gofile folder's expiry (Unix seconds), or removes it (None).
pub(super) async fn gofile_expiry(http: &reqwest::Client, base: &str, token: &str, folder: &str, at_secs: Option<u64>) -> Result<(), String> {
    let value = match at_secs {
        Some(s) => serde_json::json!(s),
        None => serde_json::json!(""),
    };
    let resp = http
        .put(format!("{base}/{folder}/update"))
        .bearer_auth(token)
        .json(&serde_json::json!({ "attribute": "expiry", "attributeValue": value }))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| net_error(&e))?;
    let code = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let ok = (200..300).contains(&code) && serde_json::from_str::<UploadReply>(&text).map(|r| r.status == "ok").unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(format!("Gofile didn't take the date ({}).", if code >= 300 { code.to_string() } else { "unknown answer".into() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A one-answer local stand-in: replies `code` + `body`, gives back the request (head, body).
    async fn stand_in(code: u16, body: &'static str) -> (String, tokio::task::JoinHandle<(String, String)>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let (mut got, mut buf) = (Vec::new(), vec![0u8; 8192]);
            let (head, start, len) = loop {
                let n = sock.read(&mut buf).await.unwrap();
                got.extend_from_slice(&buf[..n]);
                if let Some(h) = got.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&got[..h]).to_lowercase();
                    let len: usize = head.lines().find_map(|l| l.strip_prefix("content-length: ")).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
                    break (head, h + 4, len);
                }
            };
            while got.len() < start + len {
                let n = sock.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
            }
            let resp = format!("HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
            sock.write_all(resp.as_bytes()).await.unwrap();
            (head, String::from_utf8_lossy(&got[start..]).to_string())
        });
        (format!("http://{addr}/contents"), server)
    }

    #[tokio::test]
    async fn gofile_delete_sends_the_id_and_takes_gone_as_done() {
        let http = reqwest::Client::new();
        let (base, server) = stand_in(200, r#"{"status":"ok","data":{}}"#).await;
        assert_eq!(gofile_delete(&http, &base, "tok", "folder1").await, Ok(()));
        let (head, body) = server.await.unwrap();
        assert!(head.starts_with("delete /contents "), "{head}");
        assert!(head.contains("authorization: bearer tok"));
        assert_eq!(serde_json::from_str::<serde_json::Value>(&body).unwrap(), serde_json::json!({ "contentsId": "folder1" }));
        // Already gone: done.
        let (base, _s) = stand_in(404, r#"{"status":"error-notFound"}"#).await;
        assert_eq!(gofile_delete(&http, &base, "tok", "folder1").await, Ok(()));
        // Refused or busy: a plain message, tried again later.
        let (base, _s) = stand_in(500, "").await;
        assert_eq!(gofile_delete(&http, &base, "tok", "folder1").await, Err("Gofile didn't delete it (500).".into()));
        let (base, _s) = stand_in(200, r#"{"status":"error-notPremium"}"#).await;
        assert!(gofile_delete(&http, &base, "tok", "folder1").await.is_err());
    }

    #[tokio::test]
    async fn gofile_expiry_sets_and_clears_the_folder_date() {
        let http = reqwest::Client::new();
        let (base, server) = stand_in(200, r#"{"status":"ok","data":{}}"#).await;
        assert_eq!(gofile_expiry(&http, &base, "tok", "folder1", Some(1_800_000_000)).await, Ok(()));
        let (head, body) = server.await.unwrap();
        assert!(head.starts_with("put /contents/folder1/update "), "{head}");
        assert!(head.contains("authorization: bearer tok"));
        assert_eq!(serde_json::from_str::<serde_json::Value>(&body).unwrap(), serde_json::json!({ "attribute": "expiry", "attributeValue": 1_800_000_000u64 }));
        // Keep: the date is removed.
        let (base, server) = stand_in(200, r#"{"status":"ok","data":{}}"#).await;
        assert_eq!(gofile_expiry(&http, &base, "tok", "folder1", None).await, Ok(()));
        let (_, body) = server.await.unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&body).unwrap()["attributeValue"], "");
        // Gofile says no: a plain message.
        let (base, _s) = stand_in(400, r#"{"status":"error-rootFolder"}"#).await;
        assert_eq!(gofile_expiry(&http, &base, "tok", "folder1", Some(1)).await, Err("Gofile didn't take the date (400).".into()));
    }

    /// Gofile and Drive behind the same rules: who deletes on the date, and what counts as deleted.
    #[test]
    fn both_services_follow_the_same_rules() {
        assert_eq!(expiry_by(Host::Gofile, true, true), ExpiryBy::Host);
        assert_eq!(expiry_by(Host::Gofile, false, true), ExpiryBy::Pako);
        assert_eq!(expiry_by(Host::Gofile, true, false), ExpiryBy::Pako);
        assert_eq!(expiry_by(Host::Drive, true, true), ExpiryBy::Pako);
        for code in [200u16, 204] {
            assert!(deleted(code, true));
        }
        assert!(deleted(404, false));
        assert!(!deleted(200, false) && !deleted(500, true) && !deleted(403, false));
    }

    /// Another Google account's Drive would answer "not found" (which counts as deleted): Pako
    /// refuses instead. Uploads from before the account was noted aren't checked.
    #[test]
    fn drive_uploads_need_their_account() {
        assert_eq!(other_account(None, Some("me@gmail.com")), None);
        assert_eq!(other_account(Some("me@gmail.com"), Some("ME@gmail.com")), None);
        let msg = other_account(Some("me@gmail.com"), Some("work@sookera.com")).unwrap();
        assert!(msg.contains("me@gmail.com") && msg.contains("Connect that account"));
        assert!(other_account(Some("me@gmail.com"), None).is_some(), "not connected: not deleted either");
    }
}
