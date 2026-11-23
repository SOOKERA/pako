//! Private mode's commands for the window: turning it on, opening and locking it, the PIN or
//! password, the recovery code, and the private list. The vault itself is in `vault`.

use std::sync::Arc;

use tauri::{Emitter, Manager, State};

use crate::engine::Engine;
use crate::model::Download;
use crate::vault;

type Eng<'a> = State<'a, Arc<Engine>>;

/// Opening and locking happen one at a time (a lock never lands in the middle of an opening).
static OP: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Tells every window how Private mode stands now.
pub fn changed(app: &tauri::AppHandle) {
    let _ = app.emit("private-changed", vault::get().status());
}

/// Work that touches the disk or waits for the vault runs off the window's thread: a Tauri
/// command that isn't async runs on that thread, and the window stutters until it ends (a lock
/// saves the list and wipes copies; any vault call can wait behind an opening's slow PIN check).
async fn slow<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tauri::async_runtime::spawn_blocking(f).await.expect("vault work")
}

#[tauri::command]
pub async fn private_status() -> vault::Status {
    slow(|| vault::get().status()).await
}

/// Turns Private mode on (open afterwards). The recovery code is offered next.
#[tauri::command]
pub async fn private_setup(app: tauri::AppHandle, engine: Eng<'_>, secret: String, kind: String) -> Result<(), String> {
    let secret = zeroize::Zeroizing::new(secret);
    let e = engine.inner().clone();
    slow(move || {
        let _one = OP.lock().unwrap_or_else(|p| p.into_inner());
        vault::get().setup(&secret, &kind)?;
        e.private_opened();
        Ok::<(), String>(())
    })
    .await?;
    watch(&app);
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn private_unlock(app: tauri::AppHandle, engine: Eng<'_>, secret: String) -> Result<(), vault::Refused> {
    let secret = zeroize::Zeroizing::new(secret);
    let e = engine.inner().clone();
    let r = slow(move || {
        let _one = OP.lock().unwrap_or_else(|p| p.into_inner());
        let r = vault::get().unlock(&secret);
        if r.is_ok() {
            e.private_opened();
        }
        r
    })
    .await;
    if r.is_ok() {
        watch(&app);
    }
    changed(&app);
    r
}

/// Forgot the PIN or password: the recovery code opens it, and a new one is asked next.
#[tauri::command]
pub async fn private_unlock_code(app: tauri::AppHandle, engine: Eng<'_>, code: String) -> Result<(), vault::Refused> {
    let code = zeroize::Zeroizing::new(code);
    let e = engine.inner().clone();
    let r = slow(move || {
        let _one = OP.lock().unwrap_or_else(|p| p.into_inner());
        let r = vault::get().unlock_with_code(&code);
        if r.is_ok() {
            e.private_opened();
        }
        r
    })
    .await;
    if r.is_ok() {
        watch(&app);
    }
    changed(&app);
    r
}

/// Locks now (the Lock button, Ctrl+Shift+P while open, and every automatic reason).
pub fn lock_now(app: &tauri::AppHandle, engine: &Engine) {
    let _one = OP.lock().unwrap_or_else(|p| p.into_inner());
    let v = vault::get();
    if !v.is_unlocked() {
        // Locked already: a new recovery code being shown from Settings (and the key opened
        // for it) still goes.
        if v.holds_new_code() {
            v.lock();
            changed(app);
        }
        return;
    }
    v.lock();
    // Password-protected uploads of private files stop too (their readable copies go).
    crate::send::private_protected_locked();
    // Private files being sent (Direct, Upload) stop being read at once.
    crate::private_send::forget_all();
    crate::send::private_parts_locked();
    engine.private_locked();
    crate::private_files::lock_wipe();
    if let Some(p) = tauri::Manager::try_state::<Arc<crate::preview::Preview>>(app) {
        p.forget_vault();
    }
    changed(app);
}

#[tauri::command]
pub async fn private_lock(app: tauri::AppHandle) {
    slow(move || lock_now(&app, &app.state::<Arc<Engine>>())).await
}

#[tauri::command]
pub async fn private_change(app: tauri::AppHandle, current: Option<String>, new: String, kind: String) -> Result<(), String> {
    let (current, new) = (current.map(zeroize::Zeroizing::new), zeroize::Zeroizing::new(new));
    let r = slow(move || vault::get().change_secret(current.as_deref().map(|c| c.as_str()), &new, &kind)).await;
    changed(&app);
    r
}

/// A new recovery code: `current` is your PIN or password (always asked: a new code makes the
/// old one stop working). Locked (Settings): Private mode stays locked.
#[tauri::command]
pub async fn private_new_code(app: tauri::AppHandle, current: Option<String>) -> Result<String, String> {
    let current = current.map(zeroize::Zeroizing::new);
    let r = slow(move || vault::get().new_recovery(current.as_deref().map(|c| c.as_str()))).await;
    changed(&app);
    r
}

/// "New recovery code…" given up.
#[tauri::command]
pub async fn private_drop_code() {
    slow(|| vault::get().drop_new_recovery()).await
}

/// "Save as a file": the code being shown, in a small text file where you pick.
#[tauri::command]
pub async fn private_save_code(path: String) -> Result<(), String> {
    slow(move || save_code(path)).await
}

fn save_code(path: String) -> Result<(), String> {
    let code = vault::get().shown_code().ok_or_else(|| crate::tr!("Make a recovery code first.", "Créez d’abord un code de récupération."))?;
    let text = crate::tr!(
        "Pako: your Private mode recovery code

    {}

It opens Private mode if you forget your PIN or password, and lets you choose a new one.
Keep it somewhere safe, away from this PC. Anyone with it and this PC can open your private files.
",
        "Pako : votre code de récupération du mode privé

    {}

Il ouvre le mode privé si vous oubliez votre code PIN ou votre mot de passe, et vous laisse en choisir un nouveau.
Gardez-le en lieu sûr, loin de ce PC. Avec ce code et ce PC, n’importe qui peut ouvrir vos fichiers privés.
",
        &*code
    );
    std::fs::write(&path, text).map_err(|e| crate::tr!("Couldn't save it: {e}", "Impossible de l’enregistrer : {e}"))
}

#[tauri::command]
pub async fn private_confirm_code(app: tauri::AppHandle, last: String) -> Result<(), String> {
    let r = slow(move || vault::get().confirm_recovery(&last)).await;
    changed(&app);
    r
}

/// Deletes the private list and its files; Private mode is off afterwards.
#[tauri::command]
pub async fn private_reset(app: tauri::AppHandle) -> Result<(), String> {
    slow(move || reset(&app)).await
}

fn reset(app: &tauri::AppHandle) -> Result<(), String> {
    app.state::<Arc<Engine>>().private_reset();
    crate::private_files::lock_wipe();
    // Copies still open go too (nothing to save them into after a reset).
    crate::private_files::reset_wipe();
    let r = vault::get().reset();
    if r.is_err() {
        // Something was still open: at least nothing readable stays behind.
        crate::private_files::wipe_dir(&vault::get().incoming_dir());
    }
    changed(app);
    r
}

/// At start: what private downloads left in their waiting folder (no private download runs yet:
/// they start again once Private mode is opened).
pub fn sweep_incoming() {
    crate::private_files::wipe_dir(&vault::get().incoming_dir());
}

/// "Copy" of the recovery code: kept out of Windows' clipboard history and cloud clipboard, and
/// cleared after a minute if it's still what the clipboard holds.
#[tauri::command]
pub async fn private_copy_code() -> Result<(), String> {
    slow(|| {
        let code = vault::get().shown_code().ok_or_else(|| crate::tr!("Make a recovery code first.", "Créez d’abord un code de récupération."))?;
        clipboard::put_private(&code)
    })
    .await
}

#[cfg(windows)]
mod clipboard {
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW, SetClipboardData};
    use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    unsafe fn put(format: u32, bytes: &[u8]) {
        // SAFETY (caller): the clipboard is open. The memory is handed to the clipboard, or freed.
        unsafe {
            let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
            if h.is_null() {
                return;
            }
            let p = GlobalLock(h) as *mut u8;
            if !p.is_null() {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
                GlobalUnlock(h);
            }
            if SetClipboardData(format, h as _).is_null() {
                GlobalFree(h);
            }
        }
    }

    fn format(name: &str) -> u32 {
        let w: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: a NUL-terminated name.
        unsafe { RegisterClipboardFormatW(w.as_ptr()) }
    }

    pub fn put_private(text: &str) -> Result<(), String> {
        let wide: Vec<u8> = text.encode_utf16().chain(Some(0)).flat_map(|c| c.to_le_bytes()).collect();
        // SAFETY: opened, filled and closed here.
        let seq = unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Err(crate::tr!("Couldn't copy it: another app is using the clipboard. Try again.", "Impossible de le copier : une autre appli utilise le presse-papiers. Réessayez."));
            }
            EmptyClipboard();
            put(13, &wide); // CF_UNICODETEXT
            put(format("ExcludeClipboardContentFromMonitorProcessing"), &[0u8; 4]);
            put(format("CanIncludeInClipboardHistory"), &0u32.to_le_bytes());
            put(format("CanUploadToCloudClipboard"), &0u32.to_le_bytes());
            CloseClipboard();
            GetClipboardSequenceNumber()
        };
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(60));
            // SAFETY: plain calls; only cleared if nothing was copied since.
            unsafe {
                if GetClipboardSequenceNumber() == seq && OpenClipboard(std::ptr::null_mut()) != 0 {
                    EmptyClipboard();
                    CloseClipboard();
                }
            }
        });
        Ok(())
    }
}

#[cfg(not(windows))]
mod clipboard {
    pub fn put_private(_text: &str) -> Result<(), String> {
        Err(crate::tr!("Copy the code by hand.", "Copiez le code à la main."))
    }
}

/// The private list (empty while locked).
#[tauri::command]
pub async fn private_downloads(app: tauri::AppHandle) -> Vec<Download> {
    slow(move || app.state::<Arc<Engine>>().private_downloads()).await
}

/// Removes rows from the private list: their files are deleted from the vault.
#[tauri::command]
pub async fn private_remove(app: tauri::AppHandle, ids: Vec<String>) {
    slow(move || {
        let engine = app.state::<Arc<Engine>>();
        let ids: Vec<String> = ids.into_iter().filter(|id| engine.find(id).is_some_and(|d| d.vault)).collect();
        engine.remove(&ids, false);
    })
    .await
}

/// The one-time Private mode tip (on the first "Done · private" row) was shown.
#[tauri::command]
pub async fn private_tip_seen(app: tauri::AppHandle) {
    slow(move || {
        let _ = app.state::<Arc<Engine>>().change_settings(|s| s.private_tip_seen = true);
    })
    .await
}

/// Starts the watcher that locks Private mode by itself (see `private_lock_rules`); once per run.
pub fn watch(app: &tauri::AppHandle) {
    crate::private_lock_rules::start(app);
}
