//! Private mode's vault (folder `private` in Pako's data folder).
//!
//! One random key pair is the vault key. Its public half is stored as is, so Pako can put things
//! in (a private download's file and its line in the private list) even while the vault is
//! locked. Its secret half is only stored wrapped: once by the PIN or password and, optionally,
//! once by the recovery code, each through Argon2id. Unlocking unwraps it into memory; locking
//! wipes it. Changing the PIN or password only wraps the same key again.
//!
//! On disk: `vault.json` (the public key and the wraps), `list/<id>.bin` (one private list
//! entry each, sealed to the public key), `files/<id>.pkv` (the files, see `blocks`),
//! `tries.json` (wrong tries, so waiting after them survives a restart).

pub mod blocks;
pub mod crypto;
pub mod recovery;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use crypto::{Cost, Key};

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

#[derive(Serialize, Deserialize, Clone)]
struct Wrap {
    /// "secret" (the PIN or password) or "recovery".
    kind: String,
    salt: String,
    cost: Cost,
    blob: String,
}

/// Private mode as Pako uses it (the wraps readable only here, see `MetaFile`).
#[derive(Clone)]
struct Meta {
    pk: String,
    /// "pin" or "password".
    lock: String,
    wraps: Vec<Wrap>,
}

/// `vault.json` on disk: the wraps are sealed with Windows' own protection for this Windows
/// account (DPAPI), so a copy of the folder can't be tried on another PC or account.
#[derive(Serialize, Deserialize)]
struct MetaFile {
    v: u32,
    pk: String,
    lock: String,
    #[serde(default)]
    has_recovery: bool,
    sealed: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug)]
struct Tries {
    fails: u32,
    /// Unix seconds of the last wrong try.
    last: u64,
}

/// What the window needs to know.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub on: bool,
    pub unlocked: bool,
    /// "pin" or "password".
    pub lock: String,
    pub has_recovery: bool,
    /// Seconds to wait before the next try (after wrong ones).
    pub wait: u64,
}

/// Why unlocking didn't work.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Refused {
    /// The PIN, password or code was wrong (false: had to wait first, or it isn't set up).
    pub wrong: bool,
    pub wait: u64,
    pub message: String,
}

#[derive(Default)]
struct State {
    /// Wrong tries as this run saw them: never lower than this, even if the file is changed.
    tries: Tries,
    sk: Option<StaticSecret>,
    /// Unlocked with the recovery code: a new PIN or password can be set without the old one.
    recovered: bool,
    /// A new recovery code shown, waiting for its last group to be typed back.
    new_code: Option<Zeroizing<String>>,
    /// "New recovery code…" while locked: the key, opened with your PIN or password for that code
    /// only (Private mode stays locked), until it's kept or given up.
    code_sk: Option<StaticSecret>,
    /// Just turned on (the PIN or password was typed a moment ago): the first recovery code is
    /// offered without asking it again. Gone with the first code kept or given up, or a lock.
    fresh: bool,
    /// Keys of files being written in this run (so a private download goes on while locked).
    hot: HashMap<[u8; 16], Key>,
}

pub struct Vault {
    dir: PathBuf,
    st: Mutex<State>,
    /// One unlock (or PIN change) at a time, so wrong tries are counted one by one.
    op: Mutex<()>,
}

pub fn get() -> &'static Vault {
    static V: OnceLock<Vault> = OnceLock::new();
    V.get_or_init(|| Vault::new(crate::paths::data_dir().join("private")))
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Seconds to wait after `fails` wrong tries in a row: none for the first 3, then 5 s, doubling,
/// up to 15 minutes.
pub fn wait_after(fails: u32) -> u64 {
    if fails < 4 {
        return 0;
    }
    (5u64 << (fails - 4).min(20)).min(15 * 60)
}

/// One wrong try is forgiven for every 30 minutes since the last one (so you can't be locked
/// out for long by someone else's tries).
pub const FORGIVE_EVERY: u64 = 30 * 60;

fn counted(t: Tries, now: u64) -> u32 {
    t.fails.saturating_sub((now.saturating_sub(t.last) / FORGIVE_EVERY) as u32)
}

pub fn valid_pin(pin: &str) -> bool {
    pin.len() == 4 && pin.chars().all(|c| c.is_ascii_digit())
}

/// The rules for a password (said in the window): at least 10 characters.
pub fn valid_password(pw: &str) -> bool {
    pw.chars().count() >= 10
}

fn check_secret(secret: &str, kind: &str) -> Result<(), String> {
    match kind {
        "pin" if valid_pin(secret) => Ok(()),
        "pin" => Err(crate::tr!("The PIN is 4 digits.", "Le code PIN fait 4 chiffres.")),
        "password" if valid_password(secret) => Ok(()),
        "password" => Err(crate::tr!("The password needs at least 10 characters.", "Le mot de passe doit faire au moins 10 caractères.")),
        _ => Err(crate::tr!("Pick a PIN or a password.", "Choisissez un code PIN ou un mot de passe.")),
    }
}

impl Vault {
    pub fn new(dir: PathBuf) -> Self {
        let v = Vault { dir, st: Mutex::new(State::default()), op: Mutex::new(()) };
        // A copy of the key file left by an older Pako (or a write cut short) would still open
        // with an old PIN, password or recovery code: never kept.
        wipe_file(&v.meta_bak_path());
        wipe_file(&v.meta_tmp_path());
        v
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn meta_path(&self) -> PathBuf {
        self.dir.join("vault.json")
    }

    fn meta_bak_path(&self) -> PathBuf {
        self.dir.join("vault.json.bak")
    }

    fn meta_tmp_path(&self) -> PathBuf {
        self.dir.join("vault.json.tmp")
    }

    fn tries_path(&self) -> PathBuf {
        self.dir.join("tries.dat")
    }

    pub fn list_dir(&self) -> PathBuf {
        self.dir.join("list")
    }

    pub fn files_dir(&self) -> PathBuf {
        self.dir.join("files")
    }

    /// Where a private video's pieces (or a file the browser downloads) wait until they're
    /// done and encrypted: only this Windows user can open it.
    pub fn incoming_dir(&self) -> PathBuf {
        self.dir.join("incoming")
    }

    pub fn make_incoming(&self) -> std::io::Result<()> {
        crate::video::make_private_dir(&self.incoming_dir())
    }

    fn meta_file(&self) -> Option<MetaFile> {
        serde_json::from_slice(&std::fs::read(self.meta_path()).ok()?).ok()
    }

    fn meta(&self) -> Option<Meta> {
        let f = self.meta_file()?;
        let wraps = crate::secure::unprotect(&B64.decode(&f.sealed).ok()?).ok()?;
        let wraps: Vec<Wrap> = serde_json::from_slice(&Zeroizing::new(wraps)).ok()?;
        Some(Meta { pk: f.pk, lock: f.lock, wraps })
    }

    fn write_meta(&self, m: &Meta) -> Result<(), String> {
        let wraps = Zeroizing::new(serde_json::to_vec(&m.wraps).map_err(|e| e.to_string())?);
        let sealed = crate::secure::protect(&wraps).map_err(|e| crate::tr!("Couldn't save Private mode: {e}", "Impossible d’enregistrer le mode privé : {e}"))?;
        let f = MetaFile { v: 2, pk: m.pk.clone(), lock: m.lock.clone(), has_recovery: m.wraps.iter().any(|w| w.kind == "recovery"), sealed: B64.encode(sealed) };
        let bytes = serde_json::to_vec_pretty(&f).map_err(|e| e.to_string())?;
        // Written whole, then put in place, with NO copy of the one before: an old copy would
        // still open with the PIN, password or recovery code it replaced.
        let tmp = self.meta_tmp_path();
        let written = (|| -> std::io::Result<()> {
            use std::io::Write;
            if let Some(d) = tmp.parent() {
                std::fs::create_dir_all(d)?;
            }
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
            drop(f);
            replace_file(&tmp, &self.meta_path())
        })();
        if let Err(e) = written {
            wipe_file(&tmp);
            return Err(crate::tr!("Couldn't save Private mode: {e}", "Impossible d’enregistrer le mode privé : {e}"));
        }
        // One kept by an older Pako's saves (they made a copy each time): gone too.
        wipe_file(&self.meta_bak_path());
        Ok(())
    }

    pub fn is_on(&self) -> bool {
        self.meta_file().is_some()
    }

    pub fn is_unlocked(&self) -> bool {
        self.st.lock().unwrap().sk.is_some()
    }

    pub fn public_key(&self) -> Option<PublicKey> {
        let m = self.meta_file()?;
        let b: [u8; 32] = B64.decode(m.pk).ok()?.try_into().ok()?;
        Some(PublicKey::from(b))
    }

    /// Wrong tries: from the file (sealed for this Windows account) and from this run's
    /// memory, whichever says more.
    fn tries(&self) -> Tries {
        let disk: Tries = std::fs::read(self.tries_path())
            .ok()
            .and_then(|b| crate::secure::unprotect(&b).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mem = self.st.lock().unwrap().tries;
        if (mem.fails, mem.last) > (disk.fails, disk.last) { mem } else { disk }
    }

    fn set_tries(&self, t: Tries) {
        self.st.lock().unwrap().tries = t;
        if t.fails == 0 {
            let _ = std::fs::remove_file(self.tries_path());
        } else if let Some(b) = serde_json::to_vec(&t).ok().and_then(|j| crate::secure::protect(&j).ok()) {
            let _ = crate::paths::write_atomic(&self.tries_path(), &b);
        }
    }

    /// One more wrong try (counted after what time has forgiven).
    fn note_wrong(&self) {
        let t = self.tries();
        let n = now();
        self.set_tries(Tries { fails: counted(t, n) + 1, last: n });
    }

    /// Seconds still to wait before a try is allowed.
    pub fn wait_now(&self) -> u64 {
        let t = self.tries();
        let n = now();
        let until = t.last + wait_after(counted(t, n));
        until.saturating_sub(n)
    }

    pub fn status(&self) -> Status {
        let m = self.meta_file();
        Status {
            on: m.is_some(),
            unlocked: self.is_unlocked(),
            lock: m.as_ref().map(|m| m.lock.clone()).unwrap_or_default(),
            has_recovery: m.as_ref().is_some_and(|m| m.has_recovery),
            wait: self.wait_now(),
        }
    }

    fn wrap(kind: &str, secret: &[u8], sk: &StaticSecret) -> Result<Wrap, String> {
        let salt = crypto::random::<16>();
        let kek = crypto::stretch(secret, &salt, crypto::COST).ok_or_else(|| crate::tr!("Couldn't make the key", "Impossible de créer la clé"))?;
        let blob = crypto::encrypt(&kek, kind.as_bytes(), sk.as_bytes());
        Ok(Wrap { kind: kind.into(), salt: B64.encode(salt), cost: crypto::COST, blob: B64.encode(blob) })
    }

    fn unwrap(w: &Wrap, secret: &[u8]) -> Option<StaticSecret> {
        let salt = B64.decode(&w.salt).ok()?;
        let kek = crypto::stretch(secret, &salt, w.cost)?;
        let plain = crypto::decrypt(&kek, w.kind.as_bytes(), &B64.decode(&w.blob).ok()?)?;
        let mut arr: [u8; 32] = plain[..].try_into().ok()?;
        let sk = StaticSecret::from(arr);
        // The copy on the stack goes too (the key itself is wiped when it's dropped).
        zeroize::Zeroize::zeroize(&mut arr);
        Some(sk)
    }

    /// Turns Private mode on with a PIN or a password; the vault is open afterwards.
    pub fn setup(&self, secret: &str, kind: &str) -> Result<(), String> {
        check_secret(secret, kind)?;
        if self.is_on() {
            return Err(crate::tr!("Private mode is already on.", "Le mode privé est déjà activé."));
        }
        for d in [self.dir.clone(), self.list_dir(), self.files_dir()] {
            std::fs::create_dir_all(&d).map_err(|e| crate::tr!("Couldn't make Private mode's folder: {e}", "Impossible de créer le dossier du mode privé : {e}"))?;
        }
        let (sk, pk) = crypto::new_pair();
        let wrap = Self::wrap("secret", secret.as_bytes(), &sk)?;
        self.write_meta(&Meta { pk: B64.encode(pk.as_bytes()), lock: kind.into(), wraps: vec![wrap] })?;
        self.set_tries(Tries::default());
        let mut st = self.st.lock().unwrap();
        st.sk = Some(sk);
        st.recovered = false;
        st.fresh = true;
        Ok(())
    }

    fn refuse(&self, wrong: bool) -> Refused {
        let wait = self.wait_now();
        let message = if !wrong {
            crate::tr!("Wait {} before trying again.", "Attendez {} avant de réessayer.", plain_wait(wait))
        } else if wait > 0 {
            crate::tr!("That's not it. Wait {} before trying again.", "Ce n’est pas ça. Attendez {} avant de réessayer.", plain_wait(wait))
        } else {
            crate::tr!("That's not it. Try again.", "Ce n’est pas ça. Réessayez.")
        };
        Refused { wrong, wait, message }
    }

    fn try_unlock(&self, kind: &str, secret: &[u8]) -> Result<(), Refused> {
        let _one = self.op.lock().unwrap();
        let Some(m) = self.meta() else {
            return Err(Refused { wrong: false, wait: 0, message: crate::tr!("Private mode can't be read on this PC or Windows account.", "Le mode privé ne peut pas être lu sur ce PC ou ce compte Windows.") });
        };
        if self.wait_now() > 0 {
            return Err(self.refuse(false));
        }
        // The key must be the vault's own (its public half is stored as is).
        let sk = m.wraps.iter().filter(|w| w.kind == kind).find_map(|w| Self::unwrap(w, secret)).filter(|sk| B64.encode(PublicKey::from(sk).as_bytes()) == m.pk);
        match sk {
            Some(sk) => {
                self.set_tries(Tries::default());
                let mut st = self.st.lock().unwrap();
                st.sk = Some(sk);
                st.recovered = kind == "recovery";
                Ok(())
            }
            None => {
                self.note_wrong();
                Err(self.refuse(true))
            }
        }
    }

    pub fn unlock(&self, secret: &str) -> Result<(), Refused> {
        self.try_unlock("secret", secret.as_bytes())
    }

    pub fn unlock_with_code(&self, code: &str) -> Result<(), Refused> {
        let Some(secret) = recovery::secret_of(code) else {
            return Err(Refused { wrong: false, wait: self.wait_now(), message: crate::tr!("That code has a typo: check each group.", "Ce code contient une faute de frappe : vérifiez chaque groupe.") });
        };
        self.try_unlock("recovery", &secret)
    }

    /// Wipes the key from memory (files being written keep their own keys until they finish).
    pub fn lock(&self) {
        let mut st = self.st.lock().unwrap();
        st.sk = None;
        st.recovered = false;
        st.new_code = None;
        st.code_sk = None;
        st.fresh = false;
    }

    /// Unlocked with the recovery code (a new PIN or password is asked next).
    pub fn recovered(&self) -> bool {
        self.st.lock().unwrap().recovered
    }

    /// A new PIN or password. `current` is needed unless unlocked with the recovery code.
    pub fn change_secret(&self, current: Option<&str>, new: &str, kind: &str) -> Result<(), String> {
        check_secret(new, kind)?;
        let _one = self.op.lock().unwrap();
        let mut m = self.meta().ok_or_else(not_on)?;
        let recovered = self.recovered();
        let sk = if recovered {
            self.st.lock().unwrap().sk.clone().ok_or_else(|| crate::tr!("Open Private mode first.", "Ouvrez d’abord le mode privé."))?
        } else {
            let cur = current.ok_or_else(|| crate::tr!("Type your current PIN or password.", "Saisissez votre code PIN ou mot de passe actuel."))?;
            if self.wait_now() > 0 {
                return Err(self.refuse(false).message);
            }
            match m.wraps.iter().filter(|w| w.kind == "secret").find_map(|w| Self::unwrap(w, cur.as_bytes())) {
                Some(sk) => sk,
                None => {
                    self.note_wrong();
                    return Err(crate::tr!("Your current PIN or password isn't right.", "Votre code PIN ou mot de passe actuel n’est pas le bon."));
                }
            }
        };
        let wrap = Self::wrap("secret", new.as_bytes(), &sk)?;
        m.wraps.retain(|w| w.kind != "secret");
        m.wraps.push(wrap);
        m.lock = kind.into();
        self.write_meta(&m)?;
        self.set_tries(Tries::default());
        let mut st = self.st.lock().unwrap();
        st.recovered = false;
        Ok(())
    }

    /// Makes a new recovery code (shown once); it only counts once its last group is typed back.
    /// Always asks for the current PIN or password (open or locked): a new code makes the old one
    /// stop working, so nobody at an open Pako can do that without it.
    pub fn new_recovery(&self, current: Option<&str>) -> Result<String, String> {
        let _one = self.op.lock().unwrap();
        // Just turned on, or opened with the recovery code: the key is here and the person just
        // proved it (their new PIN or password, or the code itself).
        let (fresh, recovered) = {
            let st = self.st.lock().unwrap();
            (st.fresh && st.sk.is_some(), st.recovered && st.sk.is_some())
        };
        if fresh || recovered {
            let code = recovery::new_code();
            let shown = code.to_string();
            let mut st = self.st.lock().unwrap();
            st.new_code = Some(code);
            st.code_sk = None;
            return Ok(shown);
        }
        let cur = current.filter(|c| !c.is_empty()).ok_or_else(|| crate::tr!("Type your PIN or password.", "Saisissez votre code PIN ou mot de passe."))?;
        if self.wait_now() > 0 {
            return Err(self.refuse(false).message);
        }
        let m = self.meta().ok_or_else(not_on)?;
        let sk = match m.wraps.iter().filter(|w| w.kind == "secret").find_map(|w| Self::unwrap(w, cur.as_bytes())) {
            Some(sk) if B64.encode(PublicKey::from(&sk).as_bytes()) == m.pk => sk,
            _ => {
                self.note_wrong();
                return Err(crate::tr!("That PIN or password isn't right.", "Ce code PIN ou ce mot de passe n’est pas le bon."));
            }
        };
        self.set_tries(Tries::default());
        // Open: the key is here already. Locked (from Settings): opened for this code only;
        // Private mode stays locked.
        let code_sk = if self.is_unlocked() { None } else { Some(sk) };
        let code = recovery::new_code();
        let shown = code.to_string();
        let mut st = self.st.lock().unwrap();
        st.new_code = Some(code);
        st.code_sk = code_sk;
        Ok(shown)
    }

    /// A new recovery code is being shown (its key may be held for it): counts as open for the
    /// idle lock, and goes with every lock.
    pub fn holds_new_code(&self) -> bool {
        let st = self.st.lock().unwrap();
        st.new_code.is_some() || st.code_sk.is_some()
    }

    /// "New recovery code…" given up: the code shown and the key opened for it are forgotten.
    pub fn drop_new_recovery(&self) {
        let mut st = self.st.lock().unwrap();
        st.new_code = None;
        st.code_sk = None;
        st.fresh = false;
    }

    /// The code being shown (for "Save as a file"), until it's confirmed.
    pub fn shown_code(&self) -> Option<Zeroizing<String>> {
        self.st.lock().unwrap().new_code.clone()
    }

    /// The last group typed back: the new code replaces any older one (which stops working).
    pub fn confirm_recovery(&self, last_group: &str) -> Result<(), String> {
        let _one = self.op.lock().unwrap();
        let (code, sk) = {
            let st = self.st.lock().unwrap();
            (st.new_code.clone().ok_or_else(|| crate::tr!("Make a recovery code first.", "Créez d’abord un code de récupération."))?, st.sk.clone().or_else(|| st.code_sk.clone()).ok_or_else(|| crate::tr!("Open Private mode first.", "Ouvrez d’abord le mode privé."))?)
        };
        if !last_group.trim().eq_ignore_ascii_case(&recovery::last_group(&code)) {
            return Err(crate::tr!("That's not the last group of your code.", "Ce n’est pas le dernier groupe de votre code."));
        }
        let secret = recovery::secret_of(&code).ok_or_else(|| crate::tr!("Couldn't read the code", "Impossible de lire le code"))?;
        let wrap = Self::wrap("recovery", &secret, &sk)?;
        let mut m = self.meta().ok_or_else(not_on)?;
        m.wraps.retain(|w| w.kind != "recovery");
        m.wraps.push(wrap);
        self.write_meta(&m)?;
        self.drop_new_recovery();
        Ok(())
    }

    /// Deletes everything: the private list, its files and the keys. Private mode is off after.
    pub fn reset(&self) -> Result<(), String> {
        self.lock();
        self.st.lock().unwrap().hot.clear();
        if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir).map_err(|e| crate::tr!("Couldn't delete everything in Private mode: {e}", "Impossible de tout supprimer dans le mode privé : {e}"))?;
        }
        Ok(())
    }

    fn rec_aad(id: &str) -> Vec<u8> {
        let mut a = b"pako list entry ".to_vec();
        a.extend_from_slice(id.as_bytes());
        a
    }

    fn rec_path(&self, id: &str) -> PathBuf {
        // Ids are Pako's own (uuid-like); anything else is turned into a safe name.
        let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        self.list_dir().join(format!("{safe}.bin"))
    }

    /// Saves one private list entry (works while locked).
    pub fn put_record(&self, id: &str, bytes: &[u8]) -> Result<(), String> {
        let pk = self.public_key().ok_or_else(not_on)?;
        let blob = crypto::seal(&pk, &Self::rec_aad(id), bytes);
        let _ = std::fs::create_dir_all(self.list_dir());
        crate::paths::write_atomic(&self.rec_path(id), &blob).map_err(|e| e.to_string())
    }

    pub fn remove_record(&self, id: &str) {
        let p = self.rec_path(id);
        let _ = std::fs::remove_file(&p);
        // The copy `write_atomic` keeps of the one before.
        let _ = std::fs::remove_file(p.with_extension("bak"));
    }

    /// Every private list entry (only while unlocked). Damaged ones are skipped.
    pub fn records(&self) -> Result<Vec<(String, Zeroizing<Vec<u8>>)>, String> {
        let sk = self.st.lock().unwrap().sk.clone().ok_or_else(|| crate::tr!("Private mode is locked.", "Le mode privé est verrouillé."))?;
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(self.list_dir()) else { return Ok(out) };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(id) = name.strip_suffix(".bin") else { continue };
            let Ok(blob) = std::fs::read(e.path()) else { continue };
            if let Some(plain) = crypto::unseal(&sk, &Self::rec_aad(id), &blob) {
                out.push((id.to_string(), plain));
            }
        }
        Ok(out)
    }

    /// "Clear the private list": deletes finished private files last written before `cutoff`,
    /// with their list entries (works while locked: only the files' own times are read).
    /// Downloads still running are left alone. Returns the ids deleted.
    pub fn clear_finished_before(&self, cutoff: std::time::SystemTime) -> Vec<String> {
        let mut gone = Vec::new();
        let Ok(entries) = std::fs::read_dir(self.files_dir()) else { return gone };
        for e in entries.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(id) = name.strip_suffix(".pkv") else { continue };
            let finished = blocks::read_header_at(&path).is_ok_and(|h| h.size.is_some());
            let old = e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < cutoff);
            if finished && old && std::fs::remove_file(&path).is_ok() {
                blocks::remove(&path);
                self.remove_record(id);
                gone.push(id.to_string());
            }
        }
        gone
    }

    /// Where a private file is kept.
    pub fn file_path(&self, id: &str) -> PathBuf {
        let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        self.files_dir().join(format!("{safe}.pkv"))
    }

    /// A new private file (works while locked); its key is kept in memory until it's done.
    pub fn create_file(&self, path: &Path, size: Option<u64>) -> std::io::Result<blocks::Writer> {
        let pk = self.public_key().ok_or_else(|| std::io::Error::other(crate::tr!("Private mode isn't on", "Le mode privé n’est pas activé")))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let w = blocks::Writer::create(path, &pk, size)?;
        self.st.lock().unwrap().hot.insert(w.id(), w.key());
        Ok(w)
    }

    /// The key of a private file: kept from this run, or from the vault key (unlocked).
    pub fn key_for(&self, path: &Path) -> std::io::Result<Key> {
        let h = blocks::read_header_at(path)?;
        let st = self.st.lock().unwrap();
        if let Some(k) = st.hot.get(&h.id) {
            return Ok(k.clone());
        }
        let sk = st.sk.as_ref().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::PermissionDenied, crate::tr!("Private mode is locked", "Le mode privé est verrouillé")))?;
        blocks::file_key(sk, &h, &blocks::owner_of(path)).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, crate::tr!("This private file's key doesn't open", "La clé de ce fichier privé ne s’ouvre pas")))
    }

    /// Opens an unfinished private file again (resume).
    pub fn reopen_file(&self, path: &Path) -> std::io::Result<blocks::Writer> {
        let key = self.key_for(path)?;
        let w = blocks::Writer::open(path, key)?;
        self.st.lock().unwrap().hot.insert(w.id(), w.key());
        Ok(w)
    }

    /// A private file is finished (or gone): its key leaves memory.
    pub fn forget_file(&self, path: &Path) {
        if let Ok(h) = blocks::read_header_at(path) {
            self.st.lock().unwrap().hot.remove(&h.id);
        }
    }

    /// Reads a finished private file (only while unlocked: a key kept from writing isn't enough).
    pub fn reader(&self, path: &Path) -> std::io::Result<blocks::Reader> {
        let h = blocks::read_header_at(path)?;
        let key = {
            let st = self.st.lock().unwrap();
            let sk = st.sk.as_ref().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::PermissionDenied, crate::tr!("Private mode is locked", "Le mode privé est verrouillé")))?;
            blocks::file_key(sk, &h, &blocks::owner_of(path)).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, crate::tr!("This private file's key doesn't open", "La clé de ce fichier privé ne s’ouvre pas")))?
        };
        blocks::Reader::open(path, key)
    }

    /// Reads a file Pako just finished writing (checksum, moving it in), with the key kept from
    /// writing, whether or not the vault is open.
    pub fn reader_while_writing(&self, path: &Path) -> std::io::Result<blocks::Reader> {
        let key = self.key_for(path)?;
        blocks::Reader::open(path, key)
    }
}

/// For the window's commands: Private mode isn't set up.
fn not_on() -> String {
    crate::tr!("Private mode isn't on.", "Le mode privé n’est pas activé.")
}

/// "5 seconds", "2 minutes".
pub fn plain_wait(secs: u64) -> String {
    if secs >= 90 {
        let m = secs.div_ceil(60);
        crate::tr!("{m} minutes", "{m} minutes")
    } else if secs == 1 {
        crate::tr!("1 second", "1 seconde")
    } else {
        crate::tr!("{secs} seconds", "{secs} secondes")
    }
}

/// Overwrites a small file with zeros, written through to the disk, then deletes it. Nothing
/// when it isn't there.
fn wipe_file(path: &Path) {
    use std::io::Write;
    let Ok(m) = std::fs::metadata(path) else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(path) {
        let _ = f.write_all(&vec![0u8; m.len() as usize]);
        let _ = f.sync_all();
    }
    let _ = std::fs::remove_file(path);
}

/// Puts `from` in the place of `to` (replacing it), with no copy of the old one.
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
        let w = |p: &Path| p.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (a, b) = (w(from), w(to));
        // SAFETY: both are NUL-terminated wide strings that live through the call.
        if unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(from, to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> Vault {
        let d = std::env::temp_dir().join(format!("pako-vault-store-{}-{}", std::process::id(), uuid::Uuid::new_v4().simple()));
        Vault::new(d)
    }

    #[test]
    fn setup_lock_unlock_with_pin() {
        let v = vault();
        assert!(!v.status().on);
        assert!(v.setup("12", "pin").is_err());
        v.setup("4821", "pin").unwrap();
        assert!(v.status().unlocked);
        v.put_record("a1", b"{\"name\":\"x\"}").unwrap();
        v.lock();
        assert!(!v.is_unlocked());
        assert!(v.records().is_err(), "locked: the list can't be read");
        // Entries can still be added while locked.
        v.put_record("a2", b"{\"name\":\"y\"}").unwrap();
        assert!(v.unlock("0000").unwrap_err().wrong);
        v.unlock("4821").unwrap();
        let mut names: Vec<String> = v.records().unwrap().into_iter().map(|(id, _)| id).collect();
        names.sort();
        assert_eq!(names, ["a1", "a2"]);
        // Nothing readable on disk.
        let raw = std::fs::read(v.list_dir().join("a1.bin")).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("name"));
    }

    #[test]
    fn wrong_tries_make_you_wait_and_survive_a_restart() {
        let v = vault();
        v.setup("my long password", "password").unwrap();
        v.lock();
        for _ in 0..3 {
            let r = v.unlock("nope nope").unwrap_err();
            assert!(r.wrong);
            assert_eq!(r.wait, 0);
        }
        let r = v.unlock("nope nope").unwrap_err();
        assert!(r.wait > 0);
        // Even the right one waits now.
        assert!(!v.unlock("my long password").unwrap_err().wrong);
        // A new Pako (same folder) still waits.
        let again = Vault::new(v.dir().to_path_buf());
        assert!(again.wait_now() > 0);
        assert_eq!(wait_after(3), 0);
        assert_eq!(wait_after(4), 5);
        assert_eq!(wait_after(5), 10);
        assert_eq!(wait_after(100), 900);
        // Time forgives one wrong try per 30 minutes.
        assert_eq!(counted(Tries { fails: 6, last: 1000 }, 1000 + 2 * FORGIVE_EVERY + 5), 4);
        assert_eq!(counted(Tries { fails: 2, last: 0 }, 99 * FORGIVE_EVERY), 0);
        // Deleting the file doesn't reset this run's count.
        let _ = std::fs::remove_file(again.tries_path());
        assert!(v.wait_now() > 0);
        // The wraps on disk can't be read as they are (sealed for this Windows account).
        let raw = std::fs::read_to_string(v.dir().join("vault.json")).unwrap();
        assert!(raw.contains("sealed") && !raw.contains("salt"));
    }

    #[test]
    fn change_secret_keeps_the_same_key() {
        let v = vault();
        v.setup("1111", "pin").unwrap();
        v.put_record("r", b"kept").unwrap();
        assert!(v.change_secret(Some("2222"), "a new password", "password").is_err());
        v.change_secret(Some("1111"), "a new password", "password").unwrap();
        v.lock();
        assert!(v.unlock("1111").is_err());
        v.unlock("a new password").unwrap();
        assert_eq!(&v.records().unwrap()[0].1[..], b"kept");
        assert_eq!(v.status().lock, "password");
    }

    #[test]
    fn recovery_code_unlocks_and_sets_a_new_pin() {
        let v = vault();
        v.setup("1111", "pin").unwrap();
        let code = v.new_recovery(None).unwrap();
        assert!(v.confirm_recovery("XXXX").is_err());
        assert!(!v.status().has_recovery);
        v.confirm_recovery(&recovery::last_group(&code)).unwrap();
        assert!(v.status().has_recovery);
        v.put_record("r", b"kept").unwrap();
        v.lock();
        // Forgot the PIN.
        assert!(!v.unlock_with_code("AAAA-BBBB").unwrap_err().wrong, "a typo isn't a wrong try");
        v.unlock_with_code(&code.to_lowercase()).unwrap();
        assert!(v.recovered());
        v.change_secret(None, "9999", "pin").unwrap();
        v.lock();
        v.unlock("9999").unwrap();
        assert_eq!(&v.records().unwrap()[0].1[..], b"kept");
        // A newer code replaces the old one: open, it still needs the PIN.
        assert!(v.new_recovery(None).is_err(), "open, but not just turned on: the PIN is needed");
        assert!(v.new_recovery(Some("1234")).is_err(), "wrong PIN");
        let code2 = v.new_recovery(Some("9999")).unwrap();
        v.confirm_recovery(&recovery::last_group(&code2)).unwrap();
        v.lock();
        assert!(v.unlock_with_code(&code).unwrap_err().wrong);
        v.unlock_with_code(&code2).unwrap();
    }

    /// "New recovery code…" from Settings, locked: your PIN or password makes it, Private mode
    /// stays locked, the old code stops working; a wrong PIN counts as a wrong try.
    #[test]
    fn a_new_recovery_code_while_locked_needs_the_pin() {
        let v = vault();
        v.setup("1111", "pin").unwrap();
        let old = v.new_recovery(None).unwrap();
        v.confirm_recovery(&recovery::last_group(&old)).unwrap();
        v.lock();
        assert!(v.new_recovery(None).is_err(), "locked: the PIN is needed");
        assert!(v.new_recovery(Some("2222")).is_err(), "wrong PIN");
        let code = v.new_recovery(Some("1111")).unwrap();
        assert!(!v.is_unlocked(), "still locked");
        v.confirm_recovery(&recovery::last_group(&code)).unwrap();
        assert!(!v.is_unlocked(), "still locked after keeping it");
        assert!(v.confirm_recovery(&recovery::last_group(&code)).is_err(), "the key opened for it is gone");
        assert!(v.unlock_with_code(&old).unwrap_err().wrong, "the old code no longer works");
        v.unlock_with_code(&code).unwrap();
        // Given up: nothing kept.
        v.lock();
        let _ = v.new_recovery(Some("1111")).unwrap();
        v.drop_new_recovery();
        assert!(v.confirm_recovery("XXXX").is_err());
        // Shown while locked and left there: a lock forgets it (and the key opened for it).
        let _ = v.new_recovery(Some("1111")).unwrap();
        assert!(v.holds_new_code());
        v.lock();
        assert!(!v.holds_new_code());
        assert!(v.shown_code().is_none());
    }

    /// Changing the PIN or the recovery code never leaves a copy of the key file behind (an old
    /// copy would still open with the old PIN or code); one left by an older Pako goes at start.
    #[test]
    fn no_copy_of_the_key_file_opens_with_an_old_pin() {
        let v = vault();
        v.setup("1111", "pin").unwrap();
        v.change_secret(Some("1111"), "2222", "pin").unwrap();
        let code = v.new_recovery(Some("2222")).unwrap();
        v.confirm_recovery(&recovery::last_group(&code)).unwrap();
        assert!(!v.meta_bak_path().exists(), "no copy of the old key file");
        assert!(!v.meta_tmp_path().exists());
        v.lock();
        assert!(v.unlock("1111").unwrap_err().wrong, "the old PIN doesn't open");
        // A copy left by an older Pako: gone when the vault is opened at start.
        let before = std::fs::read(v.meta_path()).unwrap();
        std::fs::write(v.meta_bak_path(), &before).unwrap();
        let again = Vault::new(v.dir().to_path_buf());
        assert!(!again.meta_bak_path().exists(), "an old copy goes at start");
        again.unlock("2222").unwrap();
    }

    #[test]
    fn files_go_in_while_locked_and_read_only_unlocked() {
        let v = vault();
        v.setup("1234", "pin").unwrap();
        v.lock();
        let p = v.file_path("f1");
        let w = v.create_file(&p, Some(100_000)).unwrap();
        w.write_at(&vec![7u8; 60_000], 0).unwrap();
        w.sync().unwrap();
        drop(w);
        // Paused and resumed in the same run, still locked: its key was kept.
        let w = v.reopen_file(&p).unwrap();
        w.set_size(Some(100_000));
        w.write_at(&vec![7u8; 40_000], 60_000).unwrap();
        w.finish(100_000).unwrap();
        assert!(v.reader(&p).is_err(), "locked: can't be read");
        v.forget_file(&p);
        assert!(v.reopen_file(&p).is_err(), "key gone once finished");
        v.unlock("1234").unwrap();
        let r = v.reader(&p).unwrap();
        let mut out = Vec::new();
        r.copy_to(&mut out, &|| false).unwrap();
        assert_eq!(out, vec![7u8; 100_000]);
    }

    #[test]
    fn clearing_takes_only_old_finished_files() {
        let v = vault();
        v.setup("1234", "pin").unwrap();
        v.lock();
        let done = v.file_path("done");
        let w = v.create_file(&done, Some(10)).unwrap();
        w.write_at(&[1u8; 10], 0).unwrap();
        w.finish(10).unwrap();
        v.put_record("done", b"x").unwrap();
        let running = v.file_path("running");
        let w2 = v.create_file(&running, None).unwrap();
        w2.write_at(&[1u8; 10], 0).unwrap();
        w2.sync().unwrap();
        v.put_record("running", b"y").unwrap();
        // Nothing is old enough yet.
        assert!(v.clear_finished_before(std::time::SystemTime::now() - std::time::Duration::from_secs(3600)).is_empty());
        let gone = v.clear_finished_before(std::time::SystemTime::now() + std::time::Duration::from_secs(60));
        assert_eq!(gone, ["done"]);
        assert!(!done.exists() && running.exists());
        assert!(!v.list_dir().join("done.bin").exists() && v.list_dir().join("running.bin").exists());
    }

    #[test]
    fn reset_deletes_everything() {
        let v = vault();
        v.setup("1234", "pin").unwrap();
        v.put_record("x", b"y").unwrap();
        v.reset().unwrap();
        assert!(!v.status().on);
        assert!(!v.dir().exists());
    }
}
