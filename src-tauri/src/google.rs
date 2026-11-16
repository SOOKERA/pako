// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Google sign-in (OAuth for desktop apps, loopback + PKCE) and access-token renewal, for
//! sending with Google Drive ("Pako Send"): `data/google_send_client.json`, login kept
//! encrypted in `data/google_send_login.bin`. It only sees the files Pako uploads itself
//! (`drive.file`), so it needs no Google review. Downloads never sign in (see drive.rs).
//! Access tokens last 1 hour; `access_token()` renews them automatically, so an upload that
//! runs for many hours keeps working.

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::paths::{data_dir, write_atomic};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

/// Which sign-in: its registration file, where its login is kept, what it asks Google for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Account {
    client_file: &'static str,
    login_file: &'static str,
    scopes: &'static str,
    /// Said when the registration file is missing.
    missing: &'static str,
}

/// The sending sign-in ("Pako Send"): only the files Pako uploads, plus who is signed in.
pub const SENDING: Account = Account {
    client_file: "google_send_client.json",
    login_file: "google_send_login.bin",
    scopes: "https://www.googleapis.com/auth/drive.file openid email",
    missing: "Uploading to Google Drive isn't set up on this PC (google_send_client.json is missing from Pako's data folder).",
};

#[derive(Clone, Deserialize)]
struct ClientCreds {
    client_id: String,
    client_secret: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct SavedLogin {
    refresh_token: String,
    email: String,
}

struct Access {
    token: String,
    expires: Instant,
    /// When Google gave us this token.
    got_at: Instant,
}

/// A token renewed this recently is fresh: a "renew" asked by many connections that got
/// "login refused" at the same moment returns it instead of renewing again and again.
const JUST_RENEWED: Duration = Duration::from_secs(10);

/// Longest wait for one answer from Google's sign-in service. Without it, a connection that
/// goes quiet would hold up every Drive upload (they all wait for the renewed login) for good.
const GOOGLE_WAIT: Duration = Duration::from_secs(30);

#[derive(Default)]
struct State {
    login: Option<SavedLogin>,
    access: Option<Access>,
}

pub struct Google {
    http: reqwest::Client,
    account: Account,
    state: Mutex<State>,
    /// Held while renewing the token, so only one renewal runs at a time, without blocking
    /// everything else that reads the state (sign-in status, email) during the network call.
    refresh: Mutex<()>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
    /// What Google granted (space-separated). On Google's page a box can be left unticked.
    #[serde(default)]
    scope: Option<String>,
}

/// The permissions asked for (the web-address ones: "openid" and "email" come back under other
/// names) that Google's answer doesn't list. No list in the answer = nothing known to be missing.
fn missing_scopes<'a>(asked: &'a str, granted: Option<&str>) -> Vec<&'a str> {
    let Some(granted) = granted else { return Vec::new() };
    let have: Vec<&str> = granted.split_whitespace().collect();
    asked.split_whitespace().filter(|s| s.starts_with("https://") && !have.contains(s)).collect()
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
    error_description: Option<String>,
}

fn login_path(a: Account) -> std::path::PathBuf {
    data_dir().join(a.login_file)
}

/// The client built into this Pako (see build.rs): only the sending sign-in has one.
fn built_in(a: Account) -> Option<ClientCreds> {
    if a.client_file != SENDING.client_file {
        return None;
    }
    let id = option_env!("PAKO_SEND_CLIENT_ID")?;
    let secret = option_env!("PAKO_SEND_CLIENT_SECRET")?;
    Some(ClientCreds { client_id: id.to_string(), client_secret: secret.to_string() })
}

/// A registration file in the data folder wins (for testing another client); else the one built in.
fn creds_in(dir: &std::path::Path, a: Account) -> Result<ClientCreds> {
    match std::fs::read_to_string(dir.join(a.client_file)) {
        Ok(text) => serde_json::from_str(&text).with_context(|| format!("{} is not valid", a.client_file)),
        Err(_) => built_in(a).ok_or_else(|| anyhow!(a.missing)),
    }
}

fn creds(a: Account) -> Result<ClientCreds> {
    creds_in(data_dir(), a)
}

// ---------- the old download sign-in (Pako 0.1.1 and before) ----------

/// The old download sign-in: downloads no longer sign in, but a login saved by an older Pako
/// is used once more to remove the temporary copies it may have left in the user's Drive.
const OLD_DOWNLOADS: Account = Account {
    client_file: "google_client.json",
    login_file: "google_login.bin",
    scopes: "",
    missing: "",
};

/// Marks the old temporary copies (appProperties key = the install that made them).
const OLD_INSTALL_KEY: &str = "zdInstall";

/// How many starts the clean-up is tried at before Pako gives up and tells the user.
const OLD_CLEANUP_TRIES: u32 = 3;

/// What the clean-up came to.
#[derive(Debug, PartialEq)]
pub enum OldCleanup {
    /// Nothing to do, or done quietly.
    Done,
    /// Copies removed (how many).
    Removed(usize),
    /// Didn't work this time: tried again at the next start.
    Later,
    /// Gave up: the names of copies still in the Drive (empty = not known).
    GaveUp(Vec<String>),
}

/// Upgrading from 0.1.1: lists the temporary copies the old download sign-in made (found by
/// this install's mark), deletes them, then signs that login out (revokes it) and deletes it.
/// If something fails, the login is kept and it's tried again at the next start, a few times;
/// then Pako gives up (signs out all the same) and says which copies may be left.
pub async fn clean_up_old_download_login(http: reqwest::Client) -> OldCleanup {
    let dir = data_dir();
    let login_file = dir.join(OLD_DOWNLOADS.login_file);
    let tries_file = dir.join("google_login.cleanup-tries");
    if !login_file.exists() {
        let _ = std::fs::remove_file(&tries_file);
        return OldCleanup::Done;
    }
    let google = Google::for_account(http.clone(), OLD_DOWNLOADS);
    if !google.signed_in().await {
        // Unreadable (another Windows user's, or damaged): nothing it can do.
        let _ = std::fs::remove_file(&login_file);
        return OldCleanup::Done;
    }
    // The copies were marked with this install's id; no id file = no copies were ever made.
    let install = std::fs::read_to_string(dir.join("install_id")).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    let result = match &install {
        None => Ok(Vec::new()),
        Some(id) => remove_old_copies(&google, &http, id).await,
    };
    let tries = std::fs::read_to_string(&tries_file).ok().and_then(|t| t.trim().parse::<u32>().ok()).unwrap_or(0) + 1;
    let outcome = match result {
        Ok(removed) if removed.is_empty() => OldCleanup::Done,
        Ok(removed) => OldCleanup::Removed(removed.len()),
        // The login no longer works (Google forgot it): the copies can't be reached any more.
        Err(_) if !login_file.exists() => OldCleanup::GaveUp(Vec::new()),
        Err(left) if tries >= OLD_CLEANUP_TRIES => OldCleanup::GaveUp(left),
        Err(_) => {
            let _ = write_atomic(&tries_file, tries.to_string().as_bytes());
            return OldCleanup::Later;
        }
    };
    // Done (or given up): the login is revoked at Google and deleted here.
    google.sign_out().await;
    let _ = std::fs::remove_file(&tries_file);
    outcome
}

/// Deletes the old temporary copies. Ok = the names of those removed; Err = the names of those
/// still there (empty when they couldn't even be listed).
async fn remove_old_copies(google: &Google, http: &reqwest::Client, install: &str) -> std::result::Result<Vec<String>, Vec<String>> {
    #[derive(Deserialize)]
    struct Page {
        #[serde(default)]
        files: Vec<Item>,
    }
    #[derive(Deserialize)]
    struct Item {
        id: String,
        name: String,
    }
    let token = match google.access_token(false).await {
        Ok(Some(t)) => t,
        _ => return Err(Vec::new()),
    };
    const API: &str = "https://www.googleapis.com/drive/v3";
    let q = percent_encoding::utf8_percent_encode(
        &format!("appProperties has {{ key='{OLD_INSTALL_KEY}' and value='{install}' }} and trashed = false"),
        percent_encoding::NON_ALPHANUMERIC,
    )
    .to_string();
    let listed = async {
        let resp = http.get(format!("{API}/files?q={q}&fields=files(id,name)&pageSize=1000")).bearer_auth(&token).timeout(GOOGLE_WAIT).send().await?;
        resp.error_for_status()?.json::<Page>().await
    };
    let page = match listed.await {
        Ok(p) => p,
        Err(_) => return Err(Vec::new()),
    };
    let mut removed = Vec::new();
    let mut left = Vec::new();
    for f in page.files {
        let gone = http.delete(format!("{API}/files/{}?supportsAllDrives=true", f.id)).bearer_auth(&token).timeout(GOOGLE_WAIT).send().await;
        match gone {
            // Already gone counts as removed.
            Ok(r) if r.status().is_success() || r.status() == reqwest::StatusCode::NOT_FOUND => removed.push(f.name),
            _ => left.push(f.name),
        }
    }
    if left.is_empty() { Ok(removed) } else { Err(left) }
}

/// The note for the user, if any: (title, text).
pub fn old_cleanup_note(outcome: &OldCleanup) -> Option<(String, String)> {
    match outcome {
        OldCleanup::Done | OldCleanup::Later => None,
        OldCleanup::Removed(n) => Some((
            "Google Drive tidied".into(),
            format!(
                "Pako removed {n} temporary {} an older version had left in your Google Drive.",
                if *n == 1 { "copy" } else { "copies" }
            ),
        )),
        OldCleanup::GaveUp(names) if names.is_empty() => Some((
            "Temporary copies may be left in your Google Drive".into(),
            "An older Pako made temporary copies in your Google Drive and Pako couldn't remove them. Look for files ending in \"(Pako temporary copy)\" in your Drive and delete them.".into(),
        )),
        OldCleanup::GaveUp(names) => {
            let shown: Vec<&str> = names.iter().take(5).map(String::as_str).collect();
            let more = if names.len() > 5 { format!(" and {} more", names.len() - 5) } else { String::new() };
            Some((
                "Temporary copies are left in your Google Drive".into(),
                format!("Pako couldn't remove these copies an older version made: {}{more}. Delete them in your Google Drive.", shown.join(", ")),
            ))
        }
    }
}

impl Google {
    pub fn for_account(http: reqwest::Client, account: Account) -> Self {
        let login = std::fs::read(login_path(account))
            .ok()
            .and_then(|b| crate::secure::unprotect(&b).ok())
            .and_then(|b| serde_json::from_slice::<SavedLogin>(&b).ok());
        Self { http, account, state: Mutex::new(State { login, access: None }), refresh: Mutex::new(()) }
    }

    /// This PC has the registration file for this sign-in (else signing in can't work).
    pub fn configured(&self) -> bool {
        creds(self.account).is_ok()
    }

    /// Tests: signed in with a stand-in login whose token is `token`.
    #[cfg(test)]
    pub async fn test_token(&self, token: &str) {
        let mut st = self.state.lock().await;
        st.login = Some(SavedLogin { refresh_token: "test".into(), email: "test@example.com".into() });
        let now = Instant::now();
        st.access = Some(Access { token: token.into(), expires: now + Duration::from_secs(3600), got_at: now });
    }

    /// What to say when the registration file is missing.
    pub fn missing_note(&self) -> &'static str {
        self.account.missing
    }

    pub async fn email(&self) -> Option<String> {
        self.state.lock().await.login.as_ref().map(|l| l.email.clone())
    }

    pub async fn signed_in(&self) -> bool {
        self.state.lock().await.login.is_some()
    }

    /// A valid access token, renewed when it has less than 5 minutes left.
    /// `force` renews it even if it looks valid (after Google answered 401).
    /// Ok(None) = not signed in.
    pub async fn access_token(&self, force: bool) -> Result<Option<String>> {
        // The usual case: the saved token is still good.
        if let Some(t) = self.cached_token(force).await? {
            return Ok(t);
        }
        // Renew, one at a time. Whoever waited here re-checks first: the one before may
        // just have renewed it.
        let _renewing = self.refresh.lock().await;
        let login = match self.cached_token(force).await? {
            Some(t) => return Ok(t),
            None => match self.state.lock().await.login.clone() {
                Some(l) => l,
                None => return Ok(None),
            },
        };
        // The network call runs without holding the state lock.
        let c = creds(self.account)?;
        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&[
                ("client_id", c.client_id.as_str()),
                ("client_secret", c.client_secret.as_str()),
                ("refresh_token", login.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .timeout(GOOGLE_WAIT)
            .send()
            .await
            .context("Could not reach Google to renew the login")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let err: Option<TokenError> = resp.json().await.ok();
            if let Some(e) = &err {
                if e.error == "invalid_grant" {
                    // The login was revoked or expired: forget it so the user signs in again
                    // (unless they already signed in again while we were asking).
                    let mut st = self.state.lock().await;
                    if st.login.as_ref().map(|l| l.refresh_token == login.refresh_token).unwrap_or(false) {
                        st.login = None;
                        st.access = None;
                        let _ = std::fs::remove_file(login_path(self.account));
                    }
                    bail!("Your Google login expired. Connect Google Drive again in Settings.");
                }
            }
            bail!(
                "Google refused to renew the login ({}{})",
                status.as_u16(),
                err.and_then(|e| e.error_description).map(|d| format!(": {d}")).unwrap_or_default()
            );
        }
        let t: TokenResponse = resp.json().await?;
        let mut st = self.state.lock().await;
        // Signed out (or into another account) meanwhile: don't keep a token for the old login.
        if st.login.as_ref().map(|l| l.refresh_token != login.refresh_token).unwrap_or(true) {
            return Ok(st.login.as_ref().and(st.access.as_ref()).map(|a| a.token.clone()));
        }
        let now = Instant::now();
        st.access = Some(Access {
            token: t.access_token.clone(),
            expires: now + Duration::from_secs(t.expires_in),
            got_at: now,
        });
        Ok(Some(t.access_token))
    }

    /// Some(token) = the saved token can be used (Some(None) = not signed in);
    /// None = it must be renewed.
    async fn cached_token(&self, force: bool) -> Result<Option<Option<String>>> {
        let st = self.state.lock().await;
        if st.login.is_none() {
            return Ok(Some(None));
        }
        if let Some(a) = &st.access {
            let now = Instant::now();
            let fresh = if force {
                // After "login refused": renew, unless someone renewed it a moment ago.
                now.duration_since(a.got_at) < JUST_RENEWED
            } else {
                a.expires > now + Duration::from_secs(300)
            };
            if fresh {
                return Ok(Some(Some(a.token.clone())));
            }
        }
        Ok(None)
    }

    /// Opens Google's sign-in page in the browser and waits (up to 5 minutes) for the answer.
    pub async fn sign_in(&self, open_browser: impl Fn(String)) -> Result<String> {
        let c = creds(self.account)?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let redirect = format!("http://127.0.0.1:{port}");

        let verifier = random_token(48);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_token(16);

        let mut url = url::Url::parse(AUTH_URL)?;
        url.query_pairs_mut()
            .append_pair("client_id", &c.client_id)
            .append_pair("redirect_uri", &redirect)
            .append_pair("response_type", "code")
            .append_pair("scope", self.account.scopes)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent")
            .append_pair("state", &state);
        open_browser(url.to_string());

        let code = tokio::time::timeout(Duration::from_secs(300), wait_for_code(&listener, &state))
            .await
            .map_err(|_| anyhow!("Sign-in timed out. Try again."))??;

        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&[
                ("client_id", c.client_id.as_str()),
                ("client_secret", c.client_secret.as_str()),
                ("code", code.as_str()),
                ("code_verifier", verifier.as_str()),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect.as_str()),
            ])
            .timeout(GOOGLE_WAIT)
            .send()
            .await?;
        if !resp.status().is_success() {
            // Google's own reason in a few words, not its raw answer.
            let why = resp.json::<TokenError>().await.ok().map(|e| format!(" ({})", e.error_description.unwrap_or(e.error))).unwrap_or_default();
            bail!("Google refused the sign-in{why}. Try again.");
        }
        let t: TokenResponse = resp.json().await?;
        // The box for Google Drive was left unticked on Google's page: this login couldn't add
        // a single file. Said now, instead of "connected" with every upload refused.
        if !missing_scopes(self.account.scopes, t.scope.as_deref()).is_empty() {
            let token = t.refresh_token.unwrap_or(t.access_token);
            let _ = self.http.post(REVOKE_URL).form(&[("token", token)]).timeout(GOOGLE_WAIT).send().await;
            bail!("Google didn't give Pako the permission to add files to your Google Drive. Connect again and, on Google's page, tick the box for Google Drive before you continue.");
        }
        let refresh = t
            .refresh_token
            .ok_or_else(|| anyhow!("Google didn't return a lasting login. Try again."))?;

        let email = self.fetch_email(&t.access_token).await.unwrap_or_else(|_| "Google account".into());
        let login = SavedLogin { refresh_token: refresh, email: email.clone() };
        let enc = crate::secure::protect(&serde_json::to_vec(&login)?)?;
        write_atomic(&login_path(self.account), &enc)?;

        let mut st = self.state.lock().await;
        st.login = Some(login);
        let now = Instant::now();
        st.access = Some(Access {
            token: t.access_token,
            expires: now + Duration::from_secs(t.expires_in),
            got_at: now,
        });
        Ok(email)
    }

    /// Forgets the login here at once, then tells Google (without holding anything up while
    /// Google answers, and not for long).
    pub async fn sign_out(&self) {
        let login = {
            let mut st = self.state.lock().await;
            st.access = None;
            let _ = std::fs::remove_file(login_path(self.account));
            st.login.take()
        };
        if let Some(login) = login {
            let _ = self.http.post(REVOKE_URL).form(&[("token", login.refresh_token)]).timeout(GOOGLE_WAIT).send().await;
        }
    }

    async fn fetch_email(&self, token: &str) -> Result<String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct User {
            email_address: Option<String>,
            display_name: Option<String>,
        }
        #[derive(Deserialize)]
        struct About {
            user: User,
        }
        let about: About = self
            .http
            .get("https://www.googleapis.com/drive/v3/about?fields=user(emailAddress,displayName)")
            .bearer_auth(token)
            .timeout(GOOGLE_WAIT)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(about.user.email_address.or(about.user.display_name).unwrap_or_default())
    }
}

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

/// Waits for the browser to come back to http://127.0.0.1:<port>/?code=...
async fn wait_for_code(listener: &TcpListener, expected_state: &str) -> Result<String> {
    loop {
        let (mut sock, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        // A browser may open a spare connection and never use it: it doesn't hold up the real one long.
        let n = tokio::time::timeout(Duration::from_secs(5), sock.read(&mut buf)).await.ok().and_then(|r| r.ok()).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/");
        // Something else knocking with an odd request doesn't end the sign-in.
        let Ok(parsed) = url::Url::parse(&format!("http://localhost{path}")) else {
            let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n").await;
            continue;
        };
        let get = |k: &str| parsed.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.to_string());

        let (code, err) = (get("code"), get("error"));
        // e.g. /favicon.ico. Also an answer without this sign-in's own mark: Google always
        // sends it back (with a refusal too), so it's something else knocking (another program,
        // a web page trying ports), and it doesn't end the sign-in.
        if (code.is_none() && err.is_none()) || get("state").as_deref() != Some(expected_state) {
            let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
            let _ = sock.shutdown().await;
            continue;
        }
        let ok = code.is_some() && err.is_none();
        let page = if ok {
            page_html("Signed in", "You can close this tab and go back to Pako.")
        } else {
            page_html("Sign-in not completed", "Go back to Pako and try again.")
        };
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            page.len(),
            page
        );
        let _ = sock.write_all(resp.as_bytes()).await;
        let _ = sock.shutdown().await;

        if let Some(e) = err {
            // Google's own short word for it ("access_denied"), never a long or odd text.
            let e: String = e.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(40).collect();
            bail!("Sign-in was cancelled ({e}).");
        }
        return code.ok_or_else(|| anyhow!("Sign-in answer didn't match. Try again."));
    }
}

fn page_html(title: &str, text: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=utf-8><title>Pako</title>\
<style>body{{font-family:Segoe UI,system-ui,sans-serif;display:grid;place-items:center;height:100vh;margin:0;background:#f6f7f9;color:#1d2330}}\
div{{background:#fff;padding:32px 40px;border-radius:12px;box-shadow:0 2px 12px rgba(0,0,0,.08);text-align:center}}\
h1{{font-size:20px;margin:0 0 8px}}p{{margin:0;color:#5b6475}}</style></head>\
<body><div><h1>{title}</h1><p>{text}</p></div></body></html>"
    )
}

#[cfg(test)]
mod tests {
    /// A Pako downloaded by anyone has no registration file: the built-in one is used (when this
    /// build has it), and a file in the data folder still wins.
    #[test]
    fn sending_sign_in_works_without_a_file() {
        let dir = std::env::temp_dir().join(format!("pako-creds-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        match option_env!("PAKO_SEND_CLIENT_ID") {
            Some(id) => assert_eq!(super::creds_in(&dir, super::SENDING).unwrap().client_id, id),
            None => assert!(super::creds_in(&dir, super::SENDING).is_err()),
        }
        std::fs::write(dir.join("google_send_client.json"), r#"{"client_id":"file-id","client_secret":"s"}"#).unwrap();
        assert_eq!(super::creds_in(&dir, super::SENDING).unwrap().client_id, "file-id");
        // The old download sign-in never gets the built-in one.
        assert!(super::creds_in(&dir, super::OLD_DOWNLOADS).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    use super::*;

    /// On Google's page the box for Google Drive can be left unticked: the login then can't
    /// upload anything, and Pako says so instead of showing "connected".
    #[test]
    fn a_sign_in_without_the_drive_box_is_noticed() {
        let asked = SENDING.scopes;
        let all = "openid https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/drive.file";
        assert!(missing_scopes(asked, Some(all)).is_empty());
        assert_eq!(missing_scopes(asked, Some("openid https://www.googleapis.com/auth/userinfo.email")), vec!["https://www.googleapis.com/auth/drive.file"]);
        assert!(missing_scopes(asked, None).is_empty(), "Google didn't say: not refused");
        let t: TokenResponse = serde_json::from_str(r#"{"access_token":"a","expires_in":3599}"#).unwrap();
        assert!(t.scope.is_none() && t.refresh_token.is_none());
    }

    /// Only Google's answer to this sign-in (with its own mark) ends the wait: anything else
    /// knocking on the port is turned away and the sign-in goes on.
    #[tokio::test]
    async fn only_the_real_answer_ends_the_sign_in() {
        async fn knock(addr: std::net::SocketAddr, path: &str) -> String {
            let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
            s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).await.unwrap();
            let mut out = Vec::new();
            let _ = s.read_to_end(&mut out).await;
            String::from_utf8_lossy(&out).to_string()
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = tokio::spawn(async move {
            assert!(knock(addr, "/favicon.ico").await.starts_with("HTTP/1.1 404"));
            assert!(knock(addr, "/?error=access_denied").await.starts_with("HTTP/1.1 404"), "no mark: not Google's answer");
            assert!(knock(addr, "/?code=stolen&state=wrong").await.starts_with("HTTP/1.1 404"));
            assert!(knock(addr, "/?code=abc&state=mark").await.contains("Signed in"));
        });
        assert_eq!(wait_for_code(&listener, "mark").await.unwrap(), "abc");
        client.await.unwrap();
        // Google's own refusal (with the mark) ends it, in its short word.
        let client = tokio::spawn(async move { knock(addr, "/?error=access_denied&state=mark").await });
        let err = wait_for_code(&listener, "mark").await.unwrap_err().to_string();
        assert_eq!(err, "Sign-in was cancelled (access_denied).");
        client.await.unwrap();
    }

    #[test]
    fn sending_sign_in_asks_only_for_its_own_files() {
        assert_eq!(SENDING.scopes, "https://www.googleapis.com/auth/drive.file openid email");
        // Never the old download sign-in's files (its login is deleted at start).
        assert_ne!(SENDING.client_file, "google_client.json");
        assert_ne!(SENDING.login_file, "google_login.bin");
        assert!(!SENDING.scopes.contains("drive.readonly"));
    }

    #[test]
    fn missing_registration_says_so_plainly() {
        let dir = std::env::temp_dir().join(format!("pako-google-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        // Without a file, a build that has the sign-in built in uses it; one without says so plainly.
        if option_env!("PAKO_SEND_CLIENT_ID").is_none() {
            let err = creds_in(&dir, SENDING).err().expect("no file, no creds").to_string();
            assert!(err.contains("isn't set up on this PC"), "{err}");
        }
        std::fs::write(dir.join(SENDING.client_file), r#"{"client_id":"a","client_secret":"b"}"#).unwrap();
        let c = creds_in(&dir, SENDING).unwrap();
        assert_eq!(c.client_id, "a");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod old_cleanup_tests {
    use super::*;

    #[test]
    fn notes_after_the_old_clean_up() {
        assert_eq!(old_cleanup_note(&OldCleanup::Done), None);
        assert_eq!(old_cleanup_note(&OldCleanup::Later), None);
        assert!(old_cleanup_note(&OldCleanup::Removed(1)).unwrap().1.contains("1 temporary copy "));
        assert!(old_cleanup_note(&OldCleanup::Removed(3)).unwrap().1.contains("3 temporary copies"));
        assert!(old_cleanup_note(&OldCleanup::GaveUp(vec![])).unwrap().1.contains("(Pako temporary copy)"));
        let names: Vec<String> = (1..=7).map(|i| format!("f{i} (Pako temporary copy)")).collect();
        let (_, text) = old_cleanup_note(&OldCleanup::GaveUp(names)).unwrap();
        assert!(text.contains("f1 (Pako temporary copy)") && text.contains("f5 (Pako temporary copy)"));
        assert!(!text.contains("f6 ") && text.contains("and 2 more"));
    }
}
