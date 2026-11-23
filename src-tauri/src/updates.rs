// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Keeping Pako and its tools up to date.
//!
//! Pako's own new versions are published as GitHub releases in a releases-only repo. Each
//! release has `latest.json` (version, SHA-256, size, notes, signatures) and `Pako.exe`. The
//! signature is made with a private key that never leaves the release PC; only the public half
//! is in the app, so a release that isn't signed with it is refused, wherever it came from.
//!
//! Portable update: the new exe is downloaded next to the running one as `<name>.new.exe`
//! and checked. On the next start (or "Restart now"), before anything else runs, the running
//! exe is renamed to `<name>.old.exe` (Windows allows renaming a running exe), the new one takes
//! its name, and it starts "on trial". The old copy is kept until the new version's window has
//! loaded and it has run for about 20 seconds (or quit normally after its window loaded); if it
//! stopped before that, the next start puts the old version back. An old copy found without a
//! trial note (an update put in place by an older Pako) starts a trial the same way. The data
//! folder is never touched.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Where Pako's releases are published.
pub const RELEASES: &str = "https://github.com/sookera/pako-releases/releases";

/// Public half of the release signing key (base64). The private half stays on the release PC.
const PUBLIC_KEY: &str = "Hy25ZR31ovufrFfrE47WT3n3/hLld+F8ehrPZ83YHRw=";

/// The last version published with only the first kind of signature. Anything newer must carry
/// `signature2` (which also covers the size and the notes).
const LAST_V1: &str = "0.1.1";

/// The single-instance plugin's lock: `<identifier>-sim`. Pako 0.1.2 and earlier had the
/// identifier "app.zdownloader" and look only for this name: kept, and held by newer Pakos too
/// (`hold_old_lock`), so older and newer Pakos recognise each other during an update.
const INSTANCE_LOCK: &str = "app.zdownloader-sim";
/// The same lock under today's identifier (tauri.conf.json `identifier`, checked by a test).
const INSTANCE_LOCK_NOW: &str = "com.sookera.pako-sim";
/// The older Pako's single-instance window (`<old identifier>-sic` / `-siw`), which takes a
/// second start's arguments.
const OLD_INSTANCE_CLASS: &str = "app.zdownloader-sic";
const OLD_INSTANCE_WINDOW: &str = "app.zdownloader-siw";

/// How long a new version must run before the old copy is deleted.
pub const TRIAL: Duration = Duration::from_secs(20);

/// What `latest.json` holds.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Manifest {
    pub version: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub notes: String,
    /// Base64 ed25519 signature of `signed_message(version, sha256)`. Still written so older
    /// Pako versions can update.
    pub signature: String,
    /// Base64 ed25519 signature of `signed_message_v2(…)`: covers the size and notes too.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub signature2: String,
}

/// The text a release signs (first kind): the version is part of it, so an old release can't
/// be passed off as a new one.
pub fn signed_message(version: &str, sha256: &str) -> String {
    format!("pako-update:{version}:{sha256}")
}

/// The text a release signs (second kind): also the size and the notes (as their SHA-256, so
/// nothing in them can change where a field ends).
pub fn signed_message_v2(version: &str, sha256: &str, size: u64, notes: &str) -> String {
    format!("pako-update-v2:{version}:{sha256}:{size}:{:x}", Sha256::digest(notes.as_bytes()))
}

fn public_key() -> Result<VerifyingKey> {
    let bytes: [u8; 32] = B64.decode(PUBLIC_KEY)?.try_into().map_err(|_| anyhow!("bad public key"))?;
    Ok(VerifyingKey::from_bytes(&bytes)?)
}

/// Checks a manifest's signature against Pako's public key.
pub fn verify(m: &Manifest) -> Result<()> {
    verify_with(&public_key()?, m)
}

fn verify_with(key: &VerifyingKey, m: &Manifest) -> Result<()> {
    let refused = || anyhow!(tr!("This update isn't signed by Pako's publisher, so it wasn't installed.", "Cette mise à jour n’est pas signée par l’éditeur de Pako : elle n’a donc pas été installée."));
    // A plain version and a real checksum (both end up in file names and links).
    if parse_version(&m.version).is_none() || m.sha256.len() != 64 || !m.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(refused());
    }
    let check = |message: String, sig: &str| -> Result<()> {
        let sig = Signature::from_slice(&B64.decode(sig).map_err(|_| refused())?).map_err(|_| refused())?;
        key.verify(message.as_bytes(), &sig).map_err(|_| refused())
    };
    if !m.signature2.is_empty() {
        check(signed_message_v2(&m.version, &m.sha256, m.size, &m.notes), &m.signature2)
    } else if newer(&m.version, LAST_V1) {
        // Newer releases always carry the second signature: one without it was tampered with.
        Err(refused())
    } else {
        check(signed_message(&m.version, &m.sha256), &m.signature)
    }
}

fn parse_version(v: &str) -> Option<semver::Version> {
    semver::Version::parse(v.trim().trim_start_matches('v')).ok()
}

/// True if version `a` is newer than `b` ("0.1.10" > "0.1.9", "1.0.0" > "1.0.0-beta.2").
pub fn newer(a: &str, b: &str) -> bool {
    match (parse_version(a), parse_version(b)) {
        (Some(x), Some(y)) => x.cmp_precedence(&y).is_gt(),
        // Not a standard version number: compare the numbers in it.
        _ => {
            let parts = |v: &str| -> Vec<u64> { v.trim_start_matches('v').split(['.', '-']).map(|p| p.parse().unwrap_or(0)).collect() };
            parts(a) > parts(b)
        }
    }
}

pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ---------- files next to the exe ----------

fn exe() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

fn sibling(suffix: &str) -> Option<PathBuf> {
    let exe = exe()?;
    let stem = exe.file_stem()?.to_string_lossy().to_string();
    Some(exe.with_file_name(format!("{stem}.{suffix}")))
}

fn new_exe() -> Option<PathBuf> {
    sibling("new.exe")
}

fn old_exe() -> Option<PathBuf> {
    sibling("old.exe")
}

/// A new version that didn't start properly (kept aside until the next update).
fn bad_exe() -> Option<PathBuf> {
    sibling("bad.exe")
}

/// The manifest of the downloaded update (checked again before it is put in place).
fn marker() -> Option<PathBuf> {
    sibling("update.json")
}

// The update's own notes (which version is on trial, which was put back, and the copy of the
// version before an update) live in Pako's data folder, sealed for this Windows user (DPAPI):
// never next to Pako.exe, where a browser (or Pako itself) can save files. A note or an old copy
// found there was not written by Pako: it is never trusted, and an old copy is never run from
// it (see `old_copy_trusted`).

/// A new version on trial: `{ version, started }`.
fn trial_file() -> Option<PathBuf> {
    Some(crate::paths::data_dir().join("update-trial.bin"))
}

/// The version that was put back because it didn't start properly.
fn rolled_back_file() -> Option<PathBuf> {
    Some(crate::paths::data_dir().join("update-rolled-back.bin"))
}

/// What the copy kept as `Pako.old.exe` must be: its SHA-256 and version, written right before
/// an update puts the new version in place.
fn old_record_file() -> Option<PathBuf> {
    Some(crate::paths::data_dir().join("update-old.bin"))
}

/// Writes a note sealed for this Windows user.
fn write_sealed(path: &Path, text: &str) {
    if let Ok(blob) = crate::secure::protect(text.as_bytes()) {
        let _ = std::fs::write(path, blob);
    }
}

/// Reads a sealed note (anything else, or one sealed for another user, reads as nothing).
fn read_sealed(path: &Path) -> Option<String> {
    let blob = std::fs::read(path).ok()?;
    String::from_utf8(crate::secure::unprotect(&blob).ok()?).ok()
}

/// The old copy's record: `{ sha256, version }`.
#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
struct OldCopy {
    sha256: String,
    version: String,
}

fn read_old_record() -> Option<OldCopy> {
    serde_json::from_str(&read_sealed(&old_record_file()?)?).ok()
}

fn write_old_record(r: &OldCopy) {
    if let Some(f) = old_record_file() {
        write_sealed(&f, &serde_json::to_string(r).unwrap_or_default());
    }
}

fn forget_old_record() {
    if let Some(f) = old_record_file() {
        let _ = std::fs::remove_file(f);
    }
}

/// The old copy next to Pako.exe is the one Pako kept at its last update: its SHA-256 matches
/// the sealed record. Only then may it ever be put back and started.
fn old_copy_trusted(old: &Path) -> bool {
    old_trusted_by(read_old_record().as_ref(), || sha256_file(old).ok())
}

fn old_trusted_by(record: Option<&OldCopy>, hash: impl FnOnce() -> Option<String>) -> bool {
    match record {
        Some(r) if !r.sha256.is_empty() => hash().is_some_and(|h| h.eq_ignore_ascii_case(&r.sha256)),
        _ => false,
    }
}

/// Notes an older Pako (1.2.x) kept next to Pako.exe: never read (they could have been put there
/// by anything that saves files in that folder), only deleted.
fn forget_notes_next_to_exe() {
    for name in ["trial.json", "rolled-back.txt"] {
        if let Some(f) = sibling(name) {
            let _ = std::fs::remove_file(f);
        }
    }
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h)?;
    Ok(format!("{:x}", h.finalize()))
}

/// The update downloaded and checked, waiting for a restart (its version).
fn ready_version() -> Option<String> {
    let m: Manifest = serde_json::from_str(&std::fs::read_to_string(marker()?).ok()?).ok()?;
    new_exe()?.is_file().then_some(m.version).filter(|v| newer(v, current()))
}

/// The version downloaded and waiting for a restart, if any (read from disk, so a later update
/// that replaced an earlier waiting one shows as that later version).
pub fn ready_now() -> Option<String> {
    ready_version()
}

fn rolled_back_version() -> Option<String> {
    put_back_note(&read_sealed(&rolled_back_file()?)?, current())
}

fn write_rolled_back(version: &str) {
    if let Some(f) = rolled_back_file() {
        write_sealed(&f, version);
    }
}

/// The version a "put back" note is about, while it still says something true: a version newer
/// than this Pako. Once this Pako is that version or a later one (a later update worked), the
/// note is out of date: Settings must not go on saying that version was put back.
fn put_back_note(noted: &str, current: &str) -> Option<String> {
    let v = noted.trim();
    (!v.is_empty() && newer(v, current)).then(|| v.to_string())
}

/// At start: a "put back" note that is out of date goes (see `put_back_note`).
fn forget_old_put_back_note() {
    let Some(f) = rolled_back_file() else { return };
    if f.is_file() && rolled_back_version().is_none() {
        let _ = std::fs::remove_file(f);
    }
}

/// Pako's folder accepts new files (a copy in Program Files or on a read-only drive doesn't).
fn folder_writable() -> Result<()> {
    let dir = exe().and_then(|e| e.parent().map(Path::to_path_buf)).ok_or_else(|| anyhow!(tr!("Can't find where Pako is", "Impossible de trouver où est Pako")))?;
    let test = dir.join(format!(".pako-write-test-{}", uuid::Uuid::new_v4().simple()));
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&test) {
        Ok(f) => {
            drop(f);
            let _ = std::fs::remove_file(&test);
            Ok(())
        }
        Err(_) => bail!(tr!(
            "Pako can't save the update in its folder ({}): Windows doesn't allow changes there. Move the Pako folder somewhere you can change files, such as Documents, then try again.",
            "Pako ne peut pas enregistrer la mise à jour dans son dossier ({}) : Windows n’y autorise pas de changements. Déplacez le dossier de Pako là où vous pouvez modifier des fichiers, comme Documents, puis réessayez.",
            dir.display()
        )),
    }
}

// ---------- state shown in Settings ----------

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub current: String,
    /// A newer version that exists.
    pub latest: Option<String>,
    pub notes: Option<String>,
    /// Downloaded and checked: restart to use it.
    pub ready: bool,
    pub busy: bool,
    pub error: Option<String>,
    /// A check has been made this session.
    pub checked: bool,
}

static STATE: Mutex<Option<View>> = Mutex::new(None);

fn first_view() -> View {
    let error = rolled_back_version().map(|v| tr!("Pako {v} didn't start properly, so Pako {} was put back.", "Pako {v} n’a pas démarré correctement : Pako {} a donc été remis.", current()));
    View { current: current().into(), error, ..Default::default() }
}

pub fn view() -> View {
    let mut s = STATE.lock().unwrap();
    let v = s.get_or_insert_with(first_view);
    if let Some(r) = ready_version() {
        v.ready = true;
        v.latest.get_or_insert(r);
    }
    v.clone()
}

fn set(f: impl FnOnce(&mut View)) {
    let mut s = STATE.lock().unwrap();
    let v = s.get_or_insert_with(first_view);
    f(v);
}

/// Pako's update page is out of reach right now (no network yet after the PC woke up, a
/// timeout, the site down for a moment): the automatic check keeps it to itself and tries again
/// the next hour.
#[derive(Debug)]
struct Unreachable;

impl std::fmt::Display for Unreachable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&tr!("Couldn't reach Pako's update page", "Impossible de joindre la page des mises à jour de Pako"))
    }
}

impl std::error::Error for Unreachable {}

async fn fetch_manifest(http: &reqwest::Client) -> Result<Manifest> {
    let r = http
        .get(format!("{RELEASES}/latest/download/latest.json"))
        .header(reqwest::header::USER_AGENT, "Pako")
        .send()
        .await
        .map_err(|_| Unreachable)?;
    if r.status().as_u16() == 404 {
        bail!(tr!("No Pako release is published yet.", "Aucune version de Pako n’est encore publiée."));
    }
    if r.status().is_server_error() {
        return Err(anyhow::Error::new(Unreachable).context(tr!("Pako's update page answered {}", "La page des mises à jour de Pako a répondu {}", r.status().as_u16())));
    }
    if !r.status().is_success() {
        bail!(tr!("Pako's update page answered {}", "La page des mises à jour de Pako a répondu {}", r.status().as_u16()));
    }
    let m: Manifest = r.json().await.context(tr!("Pako's update page sent something unexpected", "La page des mises à jour de Pako a envoyé quelque chose d’inattendu"))?;
    verify(&m)?;
    Ok(m)
}

/// Deletes a half-downloaded file unless it was kept.
struct PartGuard(PathBuf);

impl Drop for PartGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Leftovers of earlier downloads that stopped half-way.
fn remove_old_parts() {
    let (Some(exe), Some(new)) = (exe(), new_exe()) else { return };
    let Some(dir) = exe.parent() else { return };
    let prefix = new.with_extension("").file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if prefix.is_empty() {
        return;
    }
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with(&prefix) && name.ends_with(".part") {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

async fn download(http: &reqwest::Client, m: &Manifest) -> Result<()> {
    let (Some(dest), Some(mark)) = (new_exe(), marker()) else { bail!(tr!("Can't find where Pako is", "Impossible de trouver où est Pako")) };
    folder_writable()?;
    remove_old_parts();
    // A name of its own: nothing else can write into this file.
    let part = dest.with_extension(format!("{}.part", uuid::Uuid::new_v4().simple()));
    let guard = PartGuard(part.clone());
    let r = http
        .get(format!("{RELEASES}/download/v{}/Pako.exe", m.version))
        .header(reqwest::header::USER_AGENT, "Pako")
        .send()
        .await
        .context(tr!("Couldn't download the update", "Impossible de télécharger la mise à jour"))?;
    if !r.status().is_success() {
        bail!(tr!("The update download answered {}", "Le téléchargement de la mise à jour a répondu {}", r.status().as_u16()));
    }
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&part)?;
    let mut size = 0u64;
    let mut stream = r.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context(tr!("The update download was cut", "Le téléchargement de la mise à jour a été coupé"))?;
        size += chunk.len() as u64;
        if size > m.size {
            bail!(tr!("The update download is bigger than its signed size, so it was stopped.", "Le téléchargement de la mise à jour dépasse sa taille signée : il a donc été arrêté."));
        }
        file.write_all(&chunk)?;
    }
    file.sync_all()?;
    drop(file);
    // The checksum of what is really on disk, not of what was received.
    if size != m.size || sha256_file(&part)? != m.sha256 {
        bail!(tr!("The downloaded update doesn't match its signed checksum, so it wasn't kept.", "La mise à jour téléchargée ne correspond pas à sa somme de contrôle signée : elle n’a donc pas été gardée."));
    }
    let _ = std::fs::remove_file(&dest);
    std::fs::rename(&part, &dest)?;
    std::mem::forget(guard);
    std::fs::write(mark, serde_json::to_string(m)?)?;
    Ok(())
}

/// One check at a time (two would write the same files).
static CHECKING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Looks for a newer Pako; with `get`, downloads and checks it (then it waits for a restart).
pub async fn check(http: &reqwest::Client, get: bool) -> View {
    run_check(http, get, false).await.0
}

/// The automatic check (once a day): the view, and whether it worked. Only a check that worked
/// counts for the day; anything else is tried again the next hour.
pub async fn auto_check(http: &reqwest::Client, get: bool) -> (View, bool) {
    run_check(http, get, true).await
}

/// A failure the next hour may not have: the update page out of reach.
fn passing(e: &anyhow::Error) -> bool {
    e.downcast_ref::<Unreachable>().is_some()
}

/// What an automatic check's result does: (the day counts as checked, its error shows in
/// Settings). Only a check that worked counts; a passing failure (the update page out of reach,
/// as when the PC has just woken up) also stays out of Settings, which keeps what it showed.
fn after_auto(result: &Result<()>) -> (bool, bool) {
    match result {
        Ok(()) => (true, false),
        Err(e) => (false, !passing(e)),
    }
}

/// `auto` = the daily check: a version that was put back once isn't downloaded again by itself,
/// and a passing failure isn't shown.
async fn run_check(http: &reqwest::Client, get: bool, auto: bool) -> (View, bool) {
    let _one = CHECKING.lock().await;
    let before = view();
    set(|v| {
        v.busy = true;
        v.error = None;
    });
    let result: Result<()> = async {
        let m = fetch_manifest(http).await?;
        if !newer(&m.version, current()) {
            set(|v| {
                v.latest = None;
                v.notes = None;
            });
            return Ok(());
        }
        set(|v| {
            v.latest = Some(m.version.clone());
            v.notes = Some(m.notes.clone()).filter(|n| !n.is_empty());
        });
        // Said right away, not only when "Update" is pressed.
        folder_writable()?;
        let skipped = auto && rolled_back_version().as_deref() == Some(m.version.as_str());
        if get && !skipped && ready_version().as_deref() != Some(m.version.as_str()) {
            download(http, &m).await?;
            if !auto {
                if let Some(f) = rolled_back_file() {
                    let _ = std::fs::remove_file(f);
                }
            }
        }
        Ok(())
    }
    .await;
    let (worked, shown) = if auto { after_auto(&result) } else { (result.is_ok(), true) };
    if let Some(e) = result.as_ref().err().filter(|e| !passing(e)) { crate::errorlog::note(crate::errorlog::Area::Update, &e.to_string()); }
    let error = result.err().map(|e| e.to_string());
    set(|v| {
        v.busy = false;
        if worked || shown {
            v.checked = true;
            v.error = error;
        } else {
            // Out of reach for a moment: Settings keeps what it showed before.
            v.checked = before.checked;
            v.error = before.error;
        }
    });
    (view(), worked)
}

// ---------- once a day ----------

fn stamp() -> PathBuf {
    crate::paths::data_dir().join("update-check.json")
}

fn is_due(last: u64, now: u64) -> bool {
    // A clock set back (last check "in the future") must not stop the checks.
    now < last || now - last >= 24 * 3600
}

/// A day has passed since the last automatic check.
pub fn due() -> bool {
    let last: u64 = std::fs::read_to_string(stamp()).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    is_due(last, crate::model::now_secs())
}

pub fn mark_checked() {
    let _ = std::fs::write(stamp(), crate::model::now_secs().to_string());
}

/// Installed tools with a newer version: updated quietly (one in use is skipped and tried next
/// time). Returns the names of the ones updated, and whether every tool could be looked at and
/// brought up to date (false: tried again the next hour).
pub async fn update_tools(http: &reqwest::Client) -> (Vec<String>, bool) {
    use crate::components::{ALL, install, installed_version, is_installing, latest_version, old_build, rebuild_due, same_version};
    let mut done = Vec::new();
    let mut complete = true;
    for k in ALL {
        let Some(have) = installed_version(k) else { continue };
        // An older kind of build (yt-dlp's single exe) is replaced even at the same version.
        let rebuild = old_build(k);
        if have == "installed" && !rebuild {
            continue;
        }
        if is_installing(k) {
            complete = false;
            continue;
        }
        let Ok(latest) = latest_version(http, k).await else {
            complete = false;
            continue;
        };
        if same_version(k, &have, &latest) && !rebuild_due(k, &latest) {
            continue;
        }
        let quiet = |_: &str, _: u64, _: Option<u64>| {};
        if install(http, k, &quiet).await.is_ok() {
            done.push(k.label().to_string());
        } else {
            complete = false;
        }
    }
    (done, complete)
}

// ---------- putting the update in place ----------

#[derive(Serialize, Deserialize, Debug, Default)]
struct Trial {
    version: String,
    /// The new version has started once already.
    started: bool,
}

fn read_trial() -> Option<Trial> {
    serde_json::from_str(&read_sealed(&trial_file()?)?).ok()
}

fn write_trial(t: &Trial) {
    if let Some(f) = trial_file() {
        write_sealed(&f, &serde_json::to_string(t).unwrap_or_default());
    }
}

/// Another Pako is running (it holds the single-instance lock: today's, or the one Pako 0.1.2
/// and earlier use).
pub(crate) fn another_instance_running() -> bool {
    lock_exists(INSTANCE_LOCK_NOW) || lock_exists(INSTANCE_LOCK)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Someone holds the named lock.
fn lock_exists(lock: &str) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
    let name = wide(lock);
    // SAFETY: NUL-terminated name; the handle is closed right away.
    unsafe {
        let h = OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, name.as_ptr());
        if h.is_null() {
            return false;
        }
        CloseHandle(h);
        true
    }
}

/// Pako 0.1.2 and earlier is running: it holds only the old lock (a newer Pako holds both).
/// Their single-instance locks have different names (the identifier changed), so without this
/// an old and a new Pako could run side by side and fight over the same files.
fn older_pako_running() -> bool {
    lock_exists(INSTANCE_LOCK) && !lock_exists(INSTANCE_LOCK_NOW)
}

/// The old lock's handle while this Pako holds it (0 = not held).
static OLD_LOCK: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
/// The window that answers an older Pako's second start (0 = none).
static OLD_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
/// What to do with a start handed over through that window (its arguments, the exe first).
static OLD_START: std::sync::OnceLock<Box<dyn Fn(Vec<String>) + Send + Sync>> = std::sync::OnceLock::new();

/// Called right after `step_aside_for_older`, before the window is made: holds the old lock
/// until `release_old_lock` at exit, so a Pako 0.1.2 or earlier started meanwhile sees a Pako
/// running (its single-instance check and its update step both look for that name) and stays
/// out. (A second start of this Pako takes it too, then leaves at its single-instance check;
/// Windows releases it when that process ends.)
pub fn hold_old_lock() {
    use windows_sys::Win32::System::Threading::CreateMutexW;
    let name = wide(INSTANCE_LOCK);
    // SAFETY: NUL-terminated name; the handle is kept in OLD_LOCK and closed at exit.
    let h = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if !h.is_null() {
        OLD_LOCK.store(h as isize, Ordering::SeqCst);
    }
}

/// At exit, once the engine has stopped (just before the single-instance plugin releases
/// today's lock): the old lock and the old window go, so a start meanwhile doesn't wait for this
/// Pako to be gone. Must run on the thread that made the window (the main one).
pub fn release_old_lock() {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;
    let w = OLD_WINDOW.swap(0, Ordering::SeqCst);
    if w != 0 {
        // SAFETY: a window this thread made.
        unsafe { DestroyWindow(w as _) };
    }
    let h = OLD_LOCK.swap(0, Ordering::SeqCst);
    if h != 0 {
        // SAFETY: the handle CreateMutexW gave, closed once.
        unsafe { CloseHandle(h as _) };
    }
}

/// Once this Pako is the only one running (in `setup`, on the main thread): a hidden window
/// under the name Pako 0.1.2's single-instance plugin looks for (`OLD_INSTANCE_CLASS`,
/// `OLD_INSTANCE_WINDOW`). An older Pako started meanwhile (a kept portable copy, an old
/// shortcut) finds the old lock taken and sends its start here, like a second start of the same
/// Pako: `on_start` gets its arguments (the exe first), so the window comes to the front and a
/// pako:// link opens. Without it, that start would do nothing at all.
pub fn answer_as_older(on_start: impl Fn(Vec<String>) + Send + Sync + 'static) {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, GWL_STYLE, RegisterClassExW, SetWindowLongPtrW, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP, WS_VISIBLE,
    };
    if OLD_START.set(Box::new(on_start)).is_err() {
        return;
    }
    let (class, title) = (wide(OLD_INSTANCE_CLASS), wide(OLD_INSTANCE_WINDOW));
    // SAFETY: NUL-terminated names that outlive the calls; the same window the plugin makes
    // (invisible: layered, transparent, never in the taskbar).
    unsafe {
        let module = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(old_window_proc),
            hInstance: module,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            module,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }
        SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_VISIBLE | WS_POPUP) as isize);
        OLD_WINDOW.store(hwnd as isize, Ordering::SeqCst);
    }
}

/// The arguments in a single-instance message: the folder, then each argument (the exe first),
/// separated by NUL (dwData 1543), or by `|` and ending in NUL (1542, older plugins).
fn handover_args(kind: usize, bytes: &[u8]) -> Option<Vec<String>> {
    let (sep, bytes) = match kind {
        1543 => ('\0', bytes),
        1542 => ('|', bytes.split(|b| *b == 0).next().unwrap_or_default()),
        _ => return None,
    };
    let text = String::from_utf8_lossy(bytes);
    Some(text.split(sep).skip(1).map(String::from).collect())
}

unsafe extern "system" fn old_window_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{DefWindowProcW, WM_COPYDATA};
    if msg == WM_COPYDATA {
        // SAFETY: WM_COPYDATA's lParam is a COPYDATASTRUCT valid for this call.
        let cds = unsafe { &*(lparam as *const COPYDATASTRUCT) };
        let bytes = if cds.lpData.is_null() {
            &[][..]
        } else {
            // SAFETY: cbData bytes at lpData, valid for this call.
            unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as usize) }
        };
        let Some(args) = handover_args(cds.dwData, bytes) else { return 1 };
        if let Some(f) = OLD_START.get() {
            f(args);
        }
        return 1;
    }
    // SAFETY: the default handling for everything else.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// At start, before the window: an older Pako already runs. This start is handed to it, like a
/// second start of the same Pako (its window comes to the front, a pako:// link opens there).
/// If it can't be reached (it is closing), waits a few seconds for it to be gone. Returns true
/// when this process should exit.
pub fn step_aside_for_older() -> bool {
    if !older_pako_running() {
        return false;
    }
    if hand_over_to_older() {
        return true;
    }
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(250));
        if !older_pako_running() {
            return false;
        }
    }
    true
}

/// What the single-instance plugin sends the running Pako: the folder, then each argument
/// (the exe first), separated by NUL.
fn handover_message(cwd: &str, args: &[String]) -> String {
    let mut data = cwd.to_string();
    for a in args {
        data.push('\0');
        data.push_str(a);
    }
    data
}

/// Sends this start's arguments to the older Pako's single-instance window, the way its plugin
/// expects them. False when its window wasn't found or didn't answer.
fn hand_over_to_older() -> bool {
    use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, FindWindowW, GetWindowThreadProcessId, SMTO_NORMAL, SendMessageTimeoutW, WM_COPYDATA,
    };
    /// The plugin's "folder and arguments separated by NUL" message.
    const NUL_SEPARATED: usize = 1543;
    let (class, title) = (wide(OLD_INSTANCE_CLASS), wide(OLD_INSTANCE_WINDOW));
    let cwd = std::env::current_dir().unwrap_or_default().to_string_lossy().into_owned();
    let args: Vec<String> = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    let data = handover_message(&cwd, &args);
    // SAFETY: NUL-terminated names; `data` and `cds` outlive the (synchronous) send.
    unsafe {
        let hwnd = FindWindowW(class.as_ptr(), title.as_ptr());
        if hwnd.is_null() {
            return false;
        }
        // Lets the older Pako bring its window to the front.
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != 0 {
            AllowSetForegroundWindow(pid);
        }
        let cds = COPYDATASTRUCT { dwData: NUL_SEPARATED, cbData: data.len() as u32, lpData: data.as_ptr() as *mut _ };
        SendMessageTimeoutW(hwnd, WM_COPYDATA, 0, &cds as *const _ as isize, SMTO_NORMAL, 10_000, std::ptr::null_mut()) != 0
    }
}

/// What to do with a version on trial at start.
#[derive(Debug, PartialEq)]
enum TrialStep {
    /// Nothing on trial (or a stale note: forget it).
    None,
    /// The first start of the new version: it is on trial now.
    Start,
    /// The new version started before and stopped within the trial: put the old one back.
    RollBack,
}

/// `old_trusted`: an old copy is there and it is the one Pako recorded at its last update (see
/// `old_copy_trusted`). An old copy Pako didn't record (put there by something else, or by a
/// Pako 1.2.x that kept no record) is never put back: there is nothing to try.
fn trial_step(t: Option<&Trial>, current: &str, old_trusted: bool) -> TrialStep {
    if !old_trusted {
        return TrialStep::None;
    }
    match t {
        Some(t) if t.version == current && t.started => TrialStep::RollBack,
        // The first start of this version.
        _ => TrialStep::Start,
    }
}

/// When this Pako started (the trial counts from here).
static STARTED: LazyLock<Instant> = LazyLock::new(Instant::now);
/// Pako's window has loaded and reached the engine.
static READY: AtomicBool = AtomicBool::new(false);

/// Pako's window has loaded: a version on trial is kept once it has also run `TRIAL`.
pub fn app_ready() {
    if READY.swap(true, Ordering::SeqCst) {
        return;
    }
    let left = TRIAL.saturating_sub(STARTED.elapsed());
    std::thread::spawn(move || {
        std::thread::sleep(left);
        trial_passed();
    });
}

/// Quitting normally: a version on trial whose window loaded worked.
pub fn quit_normally() {
    if READY.load(Ordering::SeqCst) {
        trial_passed();
    }
}

/// The new version ran long enough (or quit normally): it is kept, the old copy goes.
fn trial_passed() {
    let Some(t) = read_trial() else { return };
    if t.version != current() {
        return;
    }
    if let Some(f) = trial_file() {
        let _ = std::fs::remove_file(f);
    }
    forget_old_record();
    // Tried again for a while if Windows still holds it (otherwise the next start would see an
    // old copy without a note and put this version on trial again).
    remove_old_copies();
}

/// Deletes the copy from the last update (it may still be closing: tried again for a while).
fn remove_old_copies() {
    for p in [old_exe(), bad_exe()].into_iter().flatten() {
        if p.is_file() && std::fs::remove_file(&p).is_err() {
            std::thread::spawn(move || {
                for _ in 0..10 {
                    std::thread::sleep(Duration::from_secs(3));
                    if std::fs::remove_file(&p).is_ok() {
                        break;
                    }
                }
            });
        }
    }
}

/// Moves `from` to `to`; if that fails, copies it (so `to` exists either way).
fn move_or_copy(from: &Path, to: &Path) -> bool {
    std::fs::rename(from, to).is_ok() || std::fs::copy(from, to).is_ok()
}

/// At the very start, before the window or anything else: puts a downloaded update in place
/// and starts it. Returns true when this process should exit (another version is starting).
pub fn apply_at_start() -> bool {
    LazyLock::force(&STARTED);
    forget_old_put_back_note();
    // As Windows gave them (a name that isn't valid Unicode is passed on unchanged).
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    // "Restart now": wait for the Pako that asked to be gone first.
    let mut waited = false;
    if let Some(i) = args.iter().position(|a| a == "--wait-pid") {
        if let Some(pid) = args.get(i + 1).and_then(|p| p.to_str()).and_then(|p| p.parse::<u32>().ok()) {
            wait_for_exit(pid, Duration::from_secs(20));
            // A Pako slow to quit still holds its lock: swapping under it would leave this start
            // handing over to a Pako that is closing, and nothing running after. Waited for a
            // while longer; still there, this start changes nothing (it hands over as usual).
            let until = Instant::now() + Duration::from_secs(40);
            while another_instance_running() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(250));
            }
            waited = !another_instance_running();
        }
    }
    // Another Pako is running and uses this exe: change nothing under it (this start only hands
    // over to it). "Restart now" waited for the old one to quit.
    if !waited && another_instance_running() {
        return false;
    }
    let (Some(exe), Some(new), Some(old), Some(mark)) = (exe(), new_exe(), old_exe(), marker()) else { return false };
    forget_notes_next_to_exe();

    let trial = read_trial();
    let old_trusted = old.is_file() && old_copy_trusted(&old);
    match trial_step(trial.as_ref(), current(), old_trusted) {
        TrialStep::Start => {
            write_trial(&Trial { version: current().into(), started: true });
            // On trial: the old copy stays until `trial_passed`.
            return false;
        }
        TrialStep::RollBack => {
            if let Some(bad) = bad_exe() {
                let _ = std::fs::remove_file(&bad);
                if std::fs::rename(&exe, &bad).is_ok() {
                    if move_or_copy(&old, &exe) {
                        write_rolled_back(current());
                        forget_old_record();
                        if let Some(f) = trial_file() {
                            let _ = std::fs::remove_file(f);
                        }
                        // This process is the version that didn't start properly, now named
                        // `bad`: it quits even if the old one can't be started from here (the
                        // next start is the old one, under Pako's own name).
                        let _ = std::process::Command::new(&exe).args(strip_wait_pid(args)).spawn();
                        return true;
                    }
                    // Couldn't put the old one back: keep this one.
                    let _ = move_or_copy(&bad, &exe);
                }
            }
            if let Some(f) = trial_file() {
                let _ = std::fs::remove_file(f);
            }
        }
        TrialStep::None => {
            if trial.is_some() {
                if let Some(f) = trial_file() {
                    let _ = std::fs::remove_file(f);
                }
            }
            // An old copy Pako didn't record goes, unrun.
            remove_old_copies();
            forget_old_record();
        }
    }

    if !new.is_file() {
        return false;
    }
    // Checked again here: the file sat on disk since it was downloaded.
    let checked = std::fs::read_to_string(&mark)
        .ok()
        .and_then(|s| serde_json::from_str::<Manifest>(&s).ok())
        .filter(|m| verify(m).is_ok() && newer(&m.version, current()))
        .filter(|m| sha256_file(&new).ok().as_deref() == Some(m.sha256.as_str()));
    let Some(m) = checked else {
        let _ = std::fs::remove_file(&new);
        let _ = std::fs::remove_file(&mark);
        return false;
    };
    let version = m.version.trim_start_matches('v').to_string();
    // Written first: the new version reads them as soon as it starts. The record says which copy
    // may be put back (this one, about to become the old copy).
    let Ok(this_sha) = sha256_file(&exe) else { return false };
    write_old_record(&OldCopy { sha256: this_sha, version: current().into() });
    write_trial(&Trial { version: version.clone(), started: false });
    match swap_and_start(&exe, &new, &old, |exe| std::process::Command::new(exe).args(strip_wait_pid(args)).spawn().is_ok()) {
        Swap::InPlace => {
            let _ = std::fs::remove_file(&mark);
            true
        }
        Swap::PutBack => {
            // Windows wouldn't start it (an antivirus or a company rule that blocks new
            // programs): this version stays, under its own name, and says so in Settings. The
            // automatic check doesn't fetch that version again; "Update" tries again.
            if let Some(f) = trial_file() {
                let _ = std::fs::remove_file(f);
            }
            let _ = std::fs::remove_file(&new);
            let _ = std::fs::remove_file(&mark);
            write_rolled_back(&version);
            forget_old_record();
            false
        }
        Swap::Unchanged => {
            if let Some(f) = trial_file() {
                let _ = std::fs::remove_file(f);
            }
            forget_old_record();
            false
        }
    }
}

/// What `swap_and_start` did.
#[derive(Debug, PartialEq)]
enum Swap {
    /// The new version is in place: started (or, if it couldn't be started and this version
    /// couldn't be put back, waiting for the next start). This process quits.
    InPlace,
    /// It couldn't be put in place: nothing changed.
    Unchanged,
    /// It was put in place but Windows wouldn't start it: everything is back as it was.
    PutBack,
}

/// Puts `new` in place of the running `exe` (which becomes `old`) and starts it. If it can't be
/// started, the running version gets its own name back: otherwise it would go on running as
/// `old` (start with Windows, pako:// links and the browser helper would follow it there, and
/// `old` is deleted later) while `exe` is a program Windows won't start. `exe` exists whatever
/// happens.
fn swap_and_start(exe: &Path, new: &Path, old: &Path, start: impl FnOnce(&Path) -> bool) -> Swap {
    let _ = std::fs::remove_file(old);
    if std::fs::rename(exe, old).is_err() {
        return Swap::Unchanged;
    }
    if std::fs::rename(new, exe).is_err() {
        let _ = move_or_copy(old, exe);
        return Swap::Unchanged;
    }
    if start(exe) {
        return Swap::InPlace;
    }
    if std::fs::rename(exe, new).is_ok() {
        if std::fs::rename(old, exe).is_ok() {
            return Swap::PutBack;
        }
        // Pako.exe must exist: the new one goes back in place.
        let _ = move_or_copy(new, exe);
    }
    Swap::InPlace
}

fn strip_wait_pid(args: Vec<OsString>) -> Vec<OsString> {
    let mut out = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--wait-pid" {
            it.next();
        } else {
            out.push(a);
        }
    }
    out
}

/// The arguments "Restart now" starts Pako with: only the wait (never the browser's or sign-in's
/// arguments, so the new Pako adds nothing again). `tray`: Pako's window was hidden when Restart
/// now was clicked (from the pop-up or the bell): the new Pako stays in the tray too, and doesn't
/// pop up (not even on the terms of use: they show when the user opens Pako).
fn restart_args(pid: u32, tray: bool) -> Vec<String> {
    let mut a = vec!["--wait-pid".to_string(), pid.to_string()];
    if tray {
        a.push(crate::system::TRAY_ARG.into());
    }
    a
}

/// "Restart now": starts this exe again, told to wait until this process has quit (the update is
/// put in place then). The caller quits right after.
pub fn restart(tray: bool) -> Result<()> {
    let exe = exe().ok_or_else(|| anyhow!(tr!("Can't find where Pako is", "Impossible de trouver où est Pako")))?;
    std::process::Command::new(exe).args(restart_args(std::process::id(), tray)).spawn().context(tr!("Couldn't restart Pako", "Impossible de redémarrer Pako"))?;
    Ok(())
}

fn wait_for_exit(pid: u32, limit: Duration) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
    // SAFETY: plain handle use; a null handle means the process is already gone.
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !h.is_null() {
            WaitForSingleObject(h, limit.as_millis() as u32);
            CloseHandle(h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// A file named like one of Windows' DLLs (dwmapi.dll, uxtheme.dll…) next to a portable
    /// Pako.exe must never be loaded: the linker setting covers the DLLs Pako.exe names, `main`
    /// the ones asked for while it runs, and the release script refuses an exe without it.
    #[test]
    fn pako_exe_takes_dlls_from_windows_only() {
        assert!(include_str!("../build.rs").contains(r#"println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");"#));
        let main = include_str!("main.rs");
        let call = main.find("    system_dlls_only();").expect("main calls system_dlls_only");
        assert!(call < main.find("std::env::args_os()").unwrap(), "before anything else");
        assert!(main.contains("SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_USER_DIRS)"));
        assert!(include_str!("../../scripts/release.ps1").contains("ToUInt16($bytes, $cfgAt + 0x4E) -ne 0x800"));
    }

    #[test]
    fn compares_versions() {
        assert!(newer("0.1.10", "0.1.9"));
        assert!(newer("v1.0.0", "0.9.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("0.1.0", "0.2.0"));
        // Pre-releases come before the release they lead to.
        assert!(newer("1.0.0", "1.0.0-beta.2"));
        assert!(!newer("1.0.0-beta.2", "1.0.0"));
        assert!(newer("1.0.0-beta.10", "1.0.0-beta.2"));
        assert!(newer("1.0.0-rc.1", "1.0.0-beta.9"));
        // Build details don't make a version newer.
        assert!(!newer("1.0.0+build.2", "1.0.0+build.1"));
    }

    fn manifest(key: &SigningKey, version: &str, v2: bool) -> Manifest {
        let sha256 = "ab".repeat(32);
        let (size, notes) = (1234, "Fixes".to_string());
        let signature = B64.encode(key.sign(signed_message(version, &sha256).as_bytes()).to_bytes());
        let signature2 =
            if v2 { B64.encode(key.sign(signed_message_v2(version, &sha256, size, &notes).as_bytes()).to_bytes()) } else { String::new() };
        Manifest { version: version.into(), sha256, size, notes, signature, signature2 }
    }

    #[test]
    fn verifies_second_kind_signatures() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let vk = key.verifying_key();
        let m = manifest(&key, "0.2.0", true);
        assert!(verify_with(&vk, &m).is_ok());
        // Size and notes are covered now.
        assert!(verify_with(&vk, &Manifest { size: 99, ..m.clone() }).is_err());
        assert!(verify_with(&vk, &Manifest { notes: "Other".into(), ..m.clone() }).is_err());
        assert!(verify_with(&vk, &Manifest { version: "0.2.1".into(), ..m.clone() }).is_err());
        // Dropping the second signature from a new release doesn't fall back to the first kind.
        assert!(verify_with(&vk, &Manifest { signature2: String::new(), ..m.clone() }).is_err());
        // Another key: refused.
        let other = SigningKey::from_bytes(&[8u8; 32]).verifying_key();
        assert!(verify_with(&other, &m).is_err());
    }

    #[test]
    fn still_verifies_first_kind_manifests() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let vk = key.verifying_key();
        let m = manifest(&key, LAST_V1, false);
        assert!(verify_with(&vk, &m).is_ok());
        assert!(verify_with(&vk, &Manifest { sha256: "cd".repeat(32), ..m.clone() }).is_err());
        // Older manifests (no signature2 field at all) still read.
        let json = format!(r#"{{"version":"0.1.0","sha256":"{}","size":1,"signature":"x"}}"#, "ab".repeat(32));
        assert!(serde_json::from_str::<Manifest>(&json).is_ok());
        // Newer than the last first-kind release, with only the first kind: refused.
        assert!(verify_with(&vk, &manifest(&key, "0.1.2", false)).is_err());
        // Odd versions or checksums are refused before anything else.
        assert!(verify_with(&vk, &Manifest { version: "../0.1.0".into(), ..m.clone() }).is_err());
        assert!(verify_with(&vk, &Manifest { sha256: "zz".into(), ..m }).is_err());
    }

    #[test]
    fn writes_both_signatures_compatibly() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let m = manifest(&key, "0.3.0", true);
        let back: Manifest = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back.signature2, m.signature2);
        // What an older Pako checks is still there and valid.
        let sig = Signature::from_slice(&B64.decode(&m.signature).unwrap()).unwrap();
        assert!(key.verifying_key().verify(signed_message("0.3.0", &m.sha256).as_bytes(), &sig).is_ok());
    }

    /// A built release, before it's published: `PAKO_MANIFEST=dist/<version>/latest.json cargo
    /// test --lib -- --ignored built_release`. Both signatures must check out with the key built
    /// into Pako (the first kind is what Pako 0.1.1 checks).
    #[test]
    #[ignore]
    fn built_release_is_signed_with_pakos_key() {
        let path = std::env::var("PAKO_MANIFEST").expect("PAKO_MANIFEST");
        let m: Manifest = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        verify(&m).expect("second kind");
        let key = public_key().unwrap();
        let sig = Signature::from_slice(&B64.decode(&m.signature).unwrap()).unwrap();
        key.verify(signed_message(&m.version, &m.sha256).as_bytes(), &sig).expect("first kind");
    }

    #[test]
    fn the_embedded_key_is_valid() {
        assert!(public_key().is_ok());
    }

    #[test]
    fn drops_the_wait_argument() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let os = |v: &[&str]| v.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(strip_wait_pid(os(&["--tray", "--wait-pid", "42"])), os(&["--tray"]));
        assert_eq!(restart_args(42, false), a(&["--wait-pid", "42"]));
        assert_eq!(restart_args(42, true), a(&["--wait-pid", "42", "--tray"]));
    }

    fn swap_dir() -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("pako-swap-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let (exe, new, old) = (dir.join("Pako.exe"), dir.join("Pako.new.exe"), dir.join("Pako.old.exe"));
        std::fs::write(&exe, b"running").unwrap();
        std::fs::write(&new, b"update").unwrap();
        (dir, exe, new, old)
    }

    /// The update goes in place and starts; the running version is kept as the old copy.
    #[test]
    fn swaps_in_the_update() {
        let (dir, exe, new, old) = swap_dir();
        let mut started = None;
        assert_eq!(swap_and_start(&exe, &new, &old, |p| { started = Some(std::fs::read(p).unwrap()); true }), Swap::InPlace);
        assert_eq!(started.as_deref(), Some(&b"update"[..]));
        assert_eq!(std::fs::read(&old).unwrap(), b"running");
        assert!(!new.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Windows won't start the update (an antivirus, a company rule): the running version gets
    /// its own name back, the update waits where it was.
    #[test]
    fn an_update_that_wont_start_is_put_back() {
        let (dir, exe, new, old) = swap_dir();
        assert_eq!(swap_and_start(&exe, &new, &old, |_| false), Swap::PutBack);
        assert_eq!(std::fs::read(&exe).unwrap(), b"running");
        assert_eq!(std::fs::read(&new).unwrap(), b"update");
        assert!(!old.exists());
        // Nothing to put in place: nothing changes.
        std::fs::remove_file(&new).unwrap();
        assert_eq!(swap_and_start(&exe, &new, &old, |_| true), Swap::Unchanged);
        assert_eq!(std::fs::read(&exe).unwrap(), b"running");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// "Pako X didn't start properly" is said only while X is newer than this Pako: after a
    /// later update worked, the note is out of date.
    #[test]
    fn a_put_back_note_ends_once_a_later_update_worked() {
        // 1.0.1 was put back, this Pako is 1.0.0: still true.
        assert_eq!(put_back_note("1.0.1\n", "1.0.0"), Some("1.0.1".into()));
        // Now on 1.0.2 (or on 1.0.1 itself, after "Update" was pressed again): out of date.
        assert_eq!(put_back_note("1.0.1", "1.0.2"), None);
        assert_eq!(put_back_note("1.0.1", "1.0.1"), None);
        assert_eq!(put_back_note("  ", "1.0.0"), None);
    }

    #[test]
    fn decides_the_trial() {
        let t = |v: &str, started: bool| Trial { version: v.into(), started };
        assert_eq!(trial_step(Some(&t("0.2.0", false)), "0.2.0", true), TrialStep::Start);
        assert_eq!(trial_step(Some(&t("0.2.0", true)), "0.2.0", true), TrialStep::RollBack);
        // No old copy to go back to: nothing to do.
        assert_eq!(trial_step(Some(&t("0.2.0", true)), "0.2.0", false), TrialStep::None);
        assert_eq!(trial_step(None, "0.2.0", false), TrialStep::None);
        // A recorded old copy without a note about this version: this start is the trial's
        // first, the old copy stays.
        assert_eq!(trial_step(None, "0.2.0", true), TrialStep::Start);
        assert_eq!(trial_step(Some(&t("0.2.0", true)), "0.1.1", true), TrialStep::Start);
        // Stopped before its trial passed (the note says started): the old copy goes back.
        assert_eq!(trial_step(Some(&t("0.1.1", true)), "0.1.1", true), TrialStep::RollBack);
    }

    /// Only the copy Pako recorded at its last update may be put back and run: a `Pako.old.exe`
    /// planted next to Pako.exe (by a browser download, an extracted archive) never is.
    #[test]
    fn only_the_recorded_old_copy_is_trusted() {
        let r = OldCopy { sha256: "AB12".into(), version: "1.3.0".into() };
        assert!(old_trusted_by(Some(&r), || Some("ab12".into())));
        // Another file under that name, no record, an empty record, an unreadable file: no.
        assert!(!old_trusted_by(Some(&r), || Some("cd34".into())));
        assert!(!old_trusted_by(None, || Some("ab12".into())));
        assert!(!old_trusted_by(Some(&OldCopy::default()), || Some(String::new())));
        assert!(!old_trusted_by(Some(&r), || None));
        // Untrusted means nothing to roll back to, whatever the trial note says.
        let t = Trial { version: "1.3.1".into(), started: true };
        assert_eq!(trial_step(Some(&t), "1.3.1", false), TrialStep::None);
    }

    #[test]
    fn a_failed_automatic_check_is_tried_again() {
        // Worked: the day counts, nothing to show.
        assert_eq!(after_auto(&Ok(())), (true, false));
        // Out of reach (no network yet, the site down): tried again next hour, not shown.
        assert_eq!(after_auto(&Err(Unreachable.into())), (false, false));
        assert_eq!(after_auto(&Err(anyhow::Error::new(Unreachable).context("Pako's update page answered 503"))), (false, false));
        // Anything else: tried again next hour too, and shown in Settings.
        assert_eq!(after_auto(&Err(anyhow!("Pako's folder can't be written to"))), (false, true));
        assert_eq!(after_auto(&Err(anyhow!("No Pako release is published yet."))), (false, true));
    }

    #[test]
    fn daily_check_survives_a_clock_set_back() {
        assert!(is_due(0, 100_000));
        assert!(!is_due(100_000, 100_000 + 3600));
        assert!(is_due(100_000, 100_000 + 24 * 3600));
        assert!(is_due(200_000, 100_000));
    }

    /// The single-instance locks follow the identifier: today's from tauri.conf.json, and the
    /// one Pako 0.1.2 and earlier hold (never renamed, or old and new wouldn't see each other).
    #[test]
    fn instance_locks_match_the_identifiers() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let id = conf["identifier"].as_str().unwrap();
        assert_eq!(INSTANCE_LOCK_NOW, format!("{id}-sim"));
        assert_eq!(INSTANCE_LOCK, "app.zdownloader-sim");
        assert_eq!((OLD_INSTANCE_CLASS, OLD_INSTANCE_WINDOW), ("app.zdownloader-sic", "app.zdownloader-siw"));
    }

    #[test]
    fn handover_message_is_nul_separated() {
        let args = vec![r"C:\Pako\Pako.exe".to_string(), "pako://add?u=a|b".to_string()];
        assert_eq!(handover_message(r"C:\Users", &args), "C:\\Users\0C:\\Pako\\Pako.exe\0pako://add?u=a|b");
        assert_eq!(handover_message("D:", &[]), "D:");
    }

    /// A start an older Pako hands over is read back the way its plugin wrote it.
    #[test]
    fn handover_from_an_older_pako_is_read() {
        let args = vec![r"C:\Old\Pako.exe".to_string(), "pako://connect".to_string()];
        let msg = handover_message(r"C:\Users", &args);
        assert_eq!(handover_args(1543, msg.as_bytes()), Some(args.clone()));
        assert_eq!(handover_args(1542, b"C:\\Users|C:\\Old\\Pako.exe|pako://connect\0junk"), Some(args));
        assert_eq!(handover_args(7, b"x"), None);
    }
}
