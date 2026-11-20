//! Using Private mode's files. The vault file is never decrypted in place:
//! - Videos play in Pako's own player, decrypted on the fly (`preview`'s /vault address): no
//!   readable copy on disk, nothing noted about it.
//! - Other files open in their usual app from a temporary decrypted copy, in a folder only this
//!   Windows user can open (%TEMP%\Pako\private-open), named PrivateFile_<date>_<time>.<ext>
//!   from when it was downloaded (the real name only with "Show real names when opening private
//!   files"), so it doesn't land in that app's title or Windows' Recent. The copy is wiped once
//!   the app lets go of it, when Private mode locks (copies still open: as soon as they're
//!   let go), when Pako closes, and any left over at the next start. Edited: Pako asks whether to
//!   save the changes into the vault.
//! - Export: a normal copy, wherever you pick. Move out of Private: the same, and the row and
//!   its vault file go. Move to Private: a finished download goes into the vault.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::engine::Engine;
use crate::model::Status;
use crate::vault;

type Eng<'a> = State<'a, Arc<Engine>>;

/// A temporary copy in use. Each sits in its own folder with a small note of where it came
/// from (`.pako-copy`), so even a copy left by a Pako that didn't close normally can be saved
/// back into Private mode, or wiped, at the next start.
struct Copy {
    path: PathBuf,
    /// The private list entry, and its vault file (saving back works while locked: the vault's
    /// public key is enough to encrypt).
    id: String,
    target: PathBuf,
    opened: Instant,
    /// Size and time it was written with (to see if the app changed it).
    stamp: Option<(u64, SystemTime)>,
    /// Deleted already (the path is still watched: an app may save it again).
    wiped: bool,
    /// "Save your changes into Private mode?" is waiting for an answer.
    asked: bool,
    /// Saving back failed: tried again from this time on (never wiped meanwhile).
    retry_at: Option<Instant>,
}

static LIVE: Mutex<Vec<Copy>> = Mutex::new(Vec::new());
static WATCHING: AtomicBool = AtomicBool::new(false);

/// Copies are wiped at the earliest this long after opening (some apps read a file, close it,
/// and read it again a moment later).
const GRACE: Duration = Duration::from_secs(30);
const NOTE: &str = ".pako-copy";

pub fn open_dir() -> PathBuf {
    crate::paths::temp_folder().join("private-open")
}

/// The copy's name: PrivateFile_<date>_<time>.<ext> from when it was downloaded, or the real
/// name; "_2", "_3"… when that's taken.
pub fn temp_name(name: &str, finished_secs: u64, real: bool, taken: impl Fn(&str) -> bool) -> String {
    let path = Path::new(name);
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let stem = if real {
        path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "PrivateFile".into())
    } else {
        let when = chrono::DateTime::from_timestamp(finished_secs as i64, 0).unwrap_or_default().with_timezone(&chrono::Local);
        format!("PrivateFile_{}", when.format("%Y-%m-%d_%Hh%M"))
    };
    let mut n = 1;
    loop {
        let candidate = if n == 1 { format!("{stem}{ext}") } else { format!("{stem}_{n}{ext}") };
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn stamp(p: &Path) -> Option<(u64, SystemTime)> {
    let m = std::fs::metadata(p).ok()?;
    Some((m.len(), m.modified().ok()?))
}

fn nanos(t: SystemTime) -> u128 {
    t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
}

/// Another program has the file open (Windows refuses to share it).
fn in_use(p: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        match std::fs::OpenOptions::new().read(true).write(true).share_mode(0).open(p) {
            Ok(_) => false,
            // 32: used by another process; 33: part of it locked.
            Err(e) => matches!(e.raw_os_error(), Some(32) | Some(33)),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = p;
        false
    }
}

/// Marks a readable copy as "from the internet" for Windows (SmartScreen and Office's Protected
/// View still check it), with no address in it. Skipped where the drive can't hold it.
pub fn mark_zone(p: &Path) {
    let mut ads = p.as_os_str().to_owned();
    ads.push(":Zone.Identifier");
    let _ = std::fs::write(PathBuf::from(ads), b"[ZoneTransfer]\r\nZoneId=3\r\n");
}

/// Overwrites with zeros, then deletes (best effort: an SSD may keep old copies of the bytes
/// elsewhere; said in the guide). True when it's gone.
pub fn wipe(p: &Path) -> bool {
    if let Ok(m) = std::fs::metadata(p) {
        if let Ok(f) = std::fs::OpenOptions::new().write(true).open(p) {
            use std::io::Write;
            let mut f = std::io::BufWriter::new(f);
            let zeros = vec![0u8; 1024 * 1024];
            let mut left = m.len();
            while left > 0 {
                let n = left.min(zeros.len() as u64) as usize;
                if f.write_all(&zeros[..n]).is_err() {
                    break;
                }
                left -= n as u64;
            }
            // Written through to the disk before the file goes (else the zeros may never land).
            if let Ok(f) = f.into_inner() {
                let _ = f.sync_all();
            }
        }
    }
    let gone = std::fs::remove_file(p).is_ok() || !p.exists();
    forget_recent(p);
    gone
}

/// Wipes every file in a folder, then the folder (a private download's waiting folder, a copy's).
pub fn wipe_dir(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            wipe_dir(&p);
        } else {
            wipe(&p);
        }
    }
    let _ = std::fs::remove_dir(dir);
}

/// Removes Windows' Recent entry for one of Pako's copies (apps' own recent lists and jump
/// lists, like Word's, can't be cleaned by Pako: said in the guide).
fn forget_recent(p: &Path) {
    let (Some(appdata), Some(name)) = (std::env::var_os("APPDATA"), p.file_name()) else { return };
    let mut lnk = name.to_os_string();
    lnk.push(".lnk");
    let _ = std::fs::remove_file(PathBuf::from(appdata).join("Microsoft").join("Windows").join("Recent").join(lnk));
}

/// Encrypts a changed copy into Private mode in place of its file (works while locked).
fn save_into(v: &vault::Vault, copy: &Path, target: &Path) -> std::io::Result<u64> {
    // Its private file was deleted meanwhile (Delete, Clear the private list, Reset): never a
    // new file with no list entry, that nothing could open or delete.
    if !target.exists() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "this private file was deleted"));
    }
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(".new");
    let tmp = PathBuf::from(tmp);
    let size = crate::engine::encrypt_into(v, copy, &tmp).inspect_err(|_| crate::vault::blocks::remove(&tmp))?;
    if !target.exists() {
        crate::vault::blocks::remove(&tmp);
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "this private file was deleted"));
    }
    std::fs::rename(&tmp, target).inspect_err(|_| crate::vault::blocks::remove(&tmp))?;
    Ok(size)
}

fn save_back(engine: Option<&Engine>, c: &Copy) -> Result<(), String> {
    let size = save_into(vault::get(), &c.path, &c.target).map_err(|e| format!("Couldn't save your changes into Private mode: {e}"))?;
    if let Some(e) = engine {
        e.private_changed_size(&c.id, size);
    }
    Ok(())
}

/// Wipes a copy and its folder.
fn wipe_copy(c: &Copy) -> bool {
    let gone = wipe(&c.path);
    if gone {
        if let Some(dir) = c.path.parent() {
            let _ = std::fs::remove_file(dir.join(NOTE));
            let _ = std::fs::remove_dir(dir);
        }
    }
    gone
}

fn changed(c: &Copy) -> bool {
    let now = stamp(&c.path);
    now.is_some() && now != c.stamp
}

/// At start: copies left by a Pako that didn't close normally. A changed one is saved into
/// Private mode first (from its note); then every copy is wiped.
pub fn sweep() {
    let Ok(entries) = std::fs::read_dir(open_dir()) else { return };
    let v = vault::get();
    for e in entries.flatten() {
        let dir = e.path();
        if !dir.is_dir() {
            wipe(&dir);
            continue;
        }
        let note: Option<serde_json::Value> = std::fs::read(dir.join(NOTE)).ok().and_then(|b| serde_json::from_slice(&b).ok());
        if let (Some(note), true) = (note, v.is_on()) {
            let copy = PathBuf::from(note["copy"].as_str().unwrap_or_default());
            let target = PathBuf::from(note["target"].as_str().unwrap_or_default());
            let was = (note["len"].as_u64().unwrap_or(0), note["mtime"].as_str().unwrap_or_default().to_string());
            let now = stamp(&copy).map(|(l, t)| (l, nanos(t).to_string()));
            let inside = copy.parent() == Some(dir.as_path()) && target.parent() == Some(v.files_dir().as_path());
            if inside && now.is_some_and(|n| n != was) && target.exists() && save_into(v, &copy, &target).is_err() {
                // Not saved: kept for the next start rather than lost.
                continue;
            }
        }
        wipe_dir(&dir);
    }
    // The folder itself: only this Windows user may open it (an older Pako, or something else,
    // may have made it without that), copies waiting in it included (they stay).
    let _ = crate::video::make_private_dir(&open_dir());
}

/// Private mode locked: copies no app has open are wiped now (changed ones saved into Private
/// mode first: its question can't be answered while it's locked); the others as soon as
/// they're let go (the watcher). A copy already wiped stays watched while its folder is there:
/// an app that saves it again (Notepad keeps no file open) brings it back, and it's saved into
/// Private mode then wiped.
pub fn lock_wipe() {
    let mut live = LIVE.lock().unwrap();
    for c in live.iter_mut() {
        // Its private file is gone (deleted, cleared): nothing to save it into.
        if !c.target.exists() {
            if !in_use(&c.path) {
                c.wiped = wipe_copy(c);
            }
            continue;
        }
        if (c.wiped && !c.path.exists()) || in_use(&c.path) {
            continue;
        }
        if changed(c) {
            if save_back(None, c).is_ok() {
                c.asked = false;
                c.wiped = wipe_copy(c);
            } else {
                c.retry_at = Some(Instant::now() + Duration::from_secs(60));
            }
        } else {
            c.wiped = wipe(&c.path);
        }
    }
    live.retain(|c| !(c.wiped && !c.path.exists() && c.path.parent().is_none_or(|d| !d.exists())));
}

/// Private mode reset (everything in it deleted): every copy goes now, none saved (there's
/// nothing to save into). One an app still holds is wiped as soon as it's let go (the watcher).
pub fn reset_wipe() {
    let mut live = LIVE.lock().unwrap();
    for c in live.iter_mut() {
        c.wiped = wipe_copy(c);
    }
    live.retain(|c| c.path.exists());
}

/// Pako is closing: changed copies are saved into Private mode (nothing you changed is lost),
/// then wiped. A copy that couldn't be saved is left for the next start (see `sweep`).
pub fn close_wipe(engine: &Engine) {
    let live = std::mem::take(&mut *LIVE.lock().unwrap());
    for c in &live {
        if c.path.exists() && changed(c) && save_back(Some(engine), c).is_err() {
            continue;
        }
        wipe_copy(c);
    }
}

fn watch(app: &tauri::AppHandle) {
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        let open = vault::get().is_unlocked();
        let engine = app.state::<Arc<Engine>>();
        let mut ask: Vec<(String, String)> = Vec::new();
        {
            let mut live = LIVE.lock().unwrap();
            let off = vault::get().public_key().is_none();
            for c in live.iter_mut() {
                // Private mode reset, or this private file deleted: nothing to save into, so the
                // copy goes as soon as no app holds it.
                if (off || !c.target.exists()) && c.path.exists() {
                    if !in_use(&c.path) {
                        c.wiped = wipe_copy(c);
                    }
                    continue;
                }
                if c.asked || c.opened.elapsed() < GRACE || c.retry_at.is_some_and(|t| Instant::now() < t) || in_use(&c.path) {
                    continue;
                }
                if changed(c) {
                    if open {
                        c.asked = true;
                        ask.push((c.path.to_string_lossy().to_string(), c.id.clone()));
                    } else if save_back(Some(&engine), c).is_ok() {
                        // Locked: saved into Private mode without asking (nothing shows).
                        c.wiped = wipe_copy(c);
                    } else {
                        c.retry_at = Some(Instant::now() + Duration::from_secs(60));
                    }
                } else if !c.wiped && c.path.exists() {
                    c.wiped = wipe(&c.path);
                }
            }
            live.retain(|c| !(c.wiped && !c.path.exists() && c.path.parent().is_none_or(|d| !d.exists())));
        }
        for (copy, id) in ask {
            let name = engine.find(&id).map(|d| d.name).unwrap_or_default();
            let _ = app.emit("private-save-ask", serde_json::json!({ "copy": copy, "name": name }));
        }
    });
}

/// Opens a private file in its usual app, from a temporary copy.
#[tauri::command]
pub async fn private_open(app: tauri::AppHandle, engine: Eng<'_>, id: String) -> Result<(), String> {
    let d = engine.find(&id).filter(|d| d.vault && d.status == Status::Completed).ok_or("This private file is gone.")?;
    let real = engine.settings().private_real_names;
    let root = open_dir();
    crate::video::make_private_dir(&root).map_err(|e| format!("Couldn't open it: {e}"))?;
    let taken = |n: &str| std::fs::read_dir(&root).map(|it| it.flatten().any(|e| e.path().join(n).exists())).unwrap_or(false);
    let name = temp_name(&d.name, d.finished_at.unwrap_or(d.date / 1000), real, taken);
    let dir = root.join(uuid::Uuid::new_v4().simple().to_string());
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't open it: {e}"))?;
    let out = dir.join(name);
    let src = d.final_path();
    let out2 = out.clone();
    let written = tauri::async_runtime::spawn_blocking(move || -> std::io::Result<()> {
        let r = vault::get().reader(&src)?;
        let mut f = std::io::BufWriter::new(std::fs::File::create(&out2)?);
        // Private mode locking meanwhile stops it (the half-made copy is wiped below).
        r.copy_to(&mut f, &|| !vault::get().is_unlocked())?;
        use std::io::Write;
        f.flush()?;
        if !vault::get().is_unlocked() {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Private mode locked"));
        }
        Ok(())
    })
    .await
    .map_err(|_| "Couldn't open it.".to_string())?;
    if let Err(e) = written {
        wipe_dir(&dir);
        return Err(format!("Couldn't open it: {e}"));
    }
    mark_zone(&out);
    let st = stamp(&out);
    let note = serde_json::json!({
        "copy": out.to_string_lossy(),
        "target": d.final_path().to_string_lossy(),
        "len": st.map(|s| s.0).unwrap_or(0),
        "mtime": st.map(|s| nanos(s.1).to_string()).unwrap_or_default(),
    });
    let _ = std::fs::write(dir.join(NOTE), note.to_string());
    // Locked while it was made: never opened, never left readable.
    let mut live = LIVE.lock().unwrap();
    if !vault::get().is_unlocked() {
        drop(live);
        wipe_dir(&dir);
        return Err("Private mode locked: open it, then open the file again.".into());
    }
    live.push(Copy { path: out.clone(), id, target: d.final_path(), opened: Instant::now(), stamp: st, wiped: false, asked: false, retry_at: None });
    drop(live);
    watch(&app);
    app.opener().open_path(out.display().to_string(), None::<&str>).map_err(|e| e.to_string())?;
    // Windows notes opened files in Recent a moment later.
    let later = out.clone();
    std::thread::spawn(move || {
        for _ in 0..3 {
            std::thread::sleep(Duration::from_secs(3));
            forget_recent(&later);
        }
    });
    Ok(())
}

/// The answer to "Save your changes into Private mode?". A copy that couldn't be saved is
/// kept (asked again a minute later), never wiped.
#[tauri::command]
pub async fn private_keep_changes(app: tauri::AppHandle, copy: String, save: bool) -> Result<(), String> {
    // Saving re-encrypts the whole file: never on the window's thread.
    tauri::async_runtime::spawn_blocking(move || keep_changes(&app.state::<Arc<Engine>>(), copy, save)).await.map_err(|e| e.to_string())?
}

fn keep_changes(engine: &Engine, copy: String, save: bool) -> Result<(), String> {
    // Locked meanwhile: a "Don't save" from a question still on screen never throws the edits
    // away (anyone at the PC could press it); they're saved, as Pako does by itself while locked.
    let save = save || !vault::get().is_unlocked();
    let path = PathBuf::from(&copy);
    let mut live = LIVE.lock().unwrap();
    // Already saved (Private mode locked meanwhile) or gone: nothing to do.
    let Some(i) = live.iter().position(|c| c.path == path) else { return Ok(()) };
    if save {
        if let Err(e) = save_back(Some(engine), &live[i]) {
            live[i].asked = false;
            live[i].retry_at = Some(Instant::now() + Duration::from_secs(60));
            return Err(e);
        }
    }
    wipe_copy(&live[i]);
    live.remove(i);
    Ok(())
}

/// "Decrypt…": a readable copy where you pick, then the private file stays ("leave"), goes
/// ("off"), or becomes an ordinary download ("downloads"). Progress goes to the window as
/// "private-decrypt" { id, done, total }.
#[tauri::command]
pub async fn private_decrypt(app: tauri::AppHandle, engine: Eng<'_>, id: String, dir: String, then: String, with_folder: Option<bool>) -> Result<String, String> {
    let tell = app.clone();
    let which = id.clone();
    let out = engine
        .inner()
        .private_decrypt(&id, PathBuf::from(dir), &then, with_folder.unwrap_or(false), move |done, total| {
            let _ = tell.emit("private-decrypt", serde_json::json!({ "id": which, "done": done, "total": total }));
        })
        .await?;
    Ok(out.display().to_string())
}

/// Move to Private: a finished download goes into the vault (its readable file is deleted).
/// Returns what to tell you when the readable file couldn't be deleted.
#[tauri::command]
pub async fn private_move_in(engine: Eng<'_>, id: String) -> Result<Option<String>, String> {
    engine.inner().move_to_private(&id).await
}

/// Lets go of everything at close (copies wiped, edits kept, see `close_wipe`).
pub fn at_close(app: &tauri::AppHandle) {
    let engine = app.state::<Arc<Engine>>();
    close_wipe(&engine);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_names() {
        // 2026-11-18 14:32 local time isn't fixed in a test: check the shape.
        let n = temp_name("Report final.pdf", 1_795_000_000, false, |_| false);
        assert!(n.starts_with("PrivateFile_") && n.ends_with(".pdf") && n.contains('h'), "{n}");
        assert!(!n.contains("Report"));
        let taken = n.clone();
        let second = temp_name("Report final.pdf", 1_795_000_000, false, |c| c == taken);
        assert!(second.ends_with("_2.pdf"), "{second}");
        assert_eq!(temp_name("Report final.pdf", 0, true, |_| false), "Report final.pdf");
        assert_eq!(temp_name("noext", 0, true, |_| false), "noext");
    }

    #[test]
    fn a_changed_copy_is_saved_while_locked_and_never_lost() {
        let v = vault::Vault::new(std::env::temp_dir().join(format!("pako-copyvault-{}", uuid::Uuid::new_v4().simple())));
        v.setup("1357", "pin").unwrap();
        v.lock();
        let target = v.file_path("copytest");
        let w = v.create_file(&target, Some(5)).unwrap();
        w.write_at(b"first", 0).unwrap();
        w.finish(5).unwrap();
        drop(w);
        let dir = std::env::temp_dir().join(format!("pako-copy-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let copy = dir.join("PrivateFile.txt");
        std::fs::write(&copy, b"changed text").unwrap();
        let c = Copy { path: copy.clone(), id: "copytest".into(), target: target.clone(), opened: Instant::now(), stamp: None, wiped: false, asked: true, retry_at: None };
        assert!(changed(&c));
        // Locked: saved with the vault's public key only.
        save_into(&v, &c.path, &c.target).unwrap();
        v.unlock("1357").unwrap();
        let mut back = Vec::new();
        v.reader(&target).unwrap().copy_to(&mut back, &|| false).unwrap();
        assert_eq!(back, b"changed text");
        // Can't be saved (its place is a file, not a folder): an error, and the copy stays.
        assert!(save_into(&v, &copy, &copy.join("x.pkv")).is_err());
        assert!(copy.exists());
        assert!(wipe_copy(&c));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wipe_removes() {
        let d = std::env::temp_dir().join(format!("pako-wipe-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("a.txt");
        std::fs::write(&p, b"secret").unwrap();
        assert!(!in_use(&p));
        assert!(wipe(&p));
        assert!(!p.exists());
    }
}
