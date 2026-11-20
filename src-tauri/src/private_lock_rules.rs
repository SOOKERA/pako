//! Private mode locks itself: after the set time with no keyboard or mouse anywhere on the PC
//! (Windows' own idle time, not just Pako's window), and at once when Windows locks (Win+L), the
//! PC sleeps or the screen saver starts. Pako minimised or closed locks it too (see lib.rs's
//! window events). A private video playing in Pako's player, with Pako's window in front, keeps
//! it open while it plays (the window says so, see `private_playing`); Windows locking still locks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use tauri::Manager;

use crate::engine::Engine;

static STARTED: AtomicBool = AtomicBool::new(false);
/// A private video is playing in Pako's player with Pako's window in front.
static PLAYING: AtomicBool = AtomicBool::new(false);

/// How long with no input before locking (None = never).
pub fn lock_after(choice: &str) -> Option<Duration> {
    let mins = match choice {
        "1m" => 1,
        "15m" => 15,
        "30m" => 30,
        "never" => return None,
        _ => 5,
    };
    Some(Duration::from_secs(mins * 60))
}

/// Why it locks now, if it should (pure: the checks are passed in).
#[derive(Debug, PartialEq, Eq)]
pub enum Why {
    /// Pako's window was minimised, or hidden in the tray (closed to the tray).
    Hidden,
    Slept,
    WindowsLocked,
    ScreenSaver,
    Idle,
}

pub fn decide(hidden: bool, slept: bool, windows_locked: bool, screen_saver: bool, idle: Duration, limit: Option<Duration>, playing: bool) -> Option<Why> {
    if hidden {
        return Some(Why::Hidden);
    }
    if slept {
        return Some(Why::Slept);
    }
    if windows_locked {
        return Some(Why::WindowsLocked);
    }
    if screen_saver {
        return Some(Why::ScreenSaver);
    }
    match limit {
        Some(l) if idle >= l && !playing => Some(Why::Idle),
        _ => None,
    }
}

/// The two clocks moved apart (the PC slept, or Pako was frozen that long): more than 5 s.
pub fn slept_between(mono: Duration, wall: Option<Duration>) -> bool {
    let gap = Duration::from_secs(5);
    match wall {
        Some(w) => w > mono + gap || mono > Duration::from_secs(1) + gap,
        // The wall clock went back: can't tell, so it counts.
        None => true,
    }
}

#[tauri::command]
pub fn private_playing(playing: bool) {
    PLAYING.store(playing, Ordering::Relaxed);
}

/// Pako's window hears from Windows the moment the session locks (Win+L) or the PC is about to
/// sleep, and locks Private mode right then; the check every second stays as a fallback.
#[cfg(windows)]
fn hear_windows(app: &tauri::AppHandle) {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Power::RegisterSuspendResumeNotification;
    use windows_sys::Win32::System::RemoteDesktop::{WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION};
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DEVICE_NOTIFY_WINDOW_HANDLE, PBT_APMSUSPEND, WM_POWERBROADCAST, WM_WTSSESSION_CHANGE};
    static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();
    const WTS_SESSION_LOCK: usize = 7;
    unsafe extern "system" fn hook(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM, _id: usize, _data: usize) -> LRESULT {
        let lock = (msg == WM_WTSSESSION_CHANGE && w == WTS_SESSION_LOCK) || (msg == WM_POWERBROADCAST && w == PBT_APMSUSPEND as usize);
        if lock {
            if let Some(app) = APP.get() {
                // Right now, before Windows locks or the PC sleeps; on a thread of its own, so the
                // window never waits for the list to be saved and copies wiped (or for an opening
                // still checking its PIN).
                let app = app.clone();
                std::thread::spawn(move || crate::private::lock_now(&app, &app.state::<Arc<Engine>>()));
            }
        }
        // SAFETY: passing the message on, as every subclass must.
        unsafe { DefSubclassProc(hwnd, msg, w, l) }
    }
    let Some(win) = app.get_webview_window("main") else { return };
    let Ok(h) = win.hwnd() else { return };
    let _ = APP.set(app.clone());
    let hwnd = h.0 as HWND;
    // SAFETY: Pako's own main window, alive as long as Pako; the hook only reads the message.
    unsafe {
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
        RegisterSuspendResumeNotification(hwnd as _, DEVICE_NOTIFY_WINDOW_HANDLE);
        SetWindowSubclass(hwnd, Some(hook), 0x5052_4956, 0);
    }
}

#[cfg(not(windows))]
fn hear_windows(_app: &tauri::AppHandle) {}

/// Starts the watcher (once per run; it does nothing while Private mode is locked).
pub fn start(app: &tauri::AppHandle) {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    hear_windows(app);
    let app = app.clone();
    std::thread::spawn(move || {
        let mut mono = Instant::now();
        let mut wall = SystemTime::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let (m, w) = (mono.elapsed(), SystemTime::now().duration_since(wall).ok());
            mono = Instant::now();
            wall = SystemTime::now();
            // Open, or a new recovery code being shown (its key held for it): both lock.
            let v = crate::vault::get();
            if !v.is_unlocked() && !v.holds_new_code() {
                continue;
            }
            let engine = app.state::<Arc<Engine>>();
            let limit = lock_after(&engine.settings().private_lock_after);
            let hidden = app
                .get_webview_window("main")
                .map(|win| !win.is_visible().unwrap_or(true) || win.is_minimized().unwrap_or(false))
                .unwrap_or(false);
            let why = decide(hidden, slept_between(m, w), windows_locked(), screen_saver_running(), idle_for(), limit, PLAYING.load(Ordering::Relaxed));
            if why.is_some() {
                PLAYING.store(false, Ordering::Relaxed);
                crate::private::lock_now(&app, &engine);
            }
        }
    });
}

/// Time since the last keyboard or mouse input anywhere in this Windows session.
#[cfg(windows)]
pub fn idle_for() -> Duration {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    // SAFETY: a valid struct with its size set.
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return Duration::ZERO;
    }
    // SAFETY: no arguments.
    let now = unsafe { GetTickCount() };
    Duration::from_millis(now.wrapping_sub(info.dwTime) as u64)
}

#[cfg(not(windows))]
pub fn idle_for() -> Duration {
    Duration::ZERO
}

/// Windows is locked (or shows its sign-in or another secure screen): the desktop that gets
/// the keyboard isn't the user's own ("Default").
#[cfg(windows)]
pub fn windows_locked() -> bool {
    use windows_sys::Win32::System::StationsAndDesktops::{CloseDesktop, GetUserObjectInformationW, OpenInputDesktop, DESKTOP_READOBJECTS, UOI_NAME};
    // SAFETY: plain call; the handle is closed below.
    let desk = unsafe { OpenInputDesktop(0, 0, DESKTOP_READOBJECTS) };
    if desk.is_null() {
        return true;
    }
    let mut name = [0u16; 64];
    let mut needed = 0u32;
    // SAFETY: valid handle and buffer of the size given.
    let ok = unsafe { GetUserObjectInformationW(desk, UOI_NAME, name.as_mut_ptr() as _, (name.len() * 2) as u32, &mut needed) };
    // SAFETY: opened above.
    unsafe { CloseDesktop(desk) };
    if ok == 0 {
        return false;
    }
    let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    !String::from_utf16_lossy(&name[..len]).eq_ignore_ascii_case("Default")
}

#[cfg(not(windows))]
pub fn windows_locked() -> bool {
    false
}

#[cfg(windows)]
pub fn screen_saver_running() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETSCREENSAVERRUNNING};
    let mut running: i32 = 0;
    // SAFETY: the out value is a BOOL.
    let ok = unsafe { SystemParametersInfoW(SPI_GETSCREENSAVERRUNNING, 0, &mut running as *mut i32 as _, 0) };
    ok != 0 && running != 0
}

#[cfg(not(windows))]
pub fn screen_saver_running() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_rules() {
        let five = lock_after("5m");
        assert_eq!(five, Some(Duration::from_secs(300)));
        assert_eq!(lock_after("never"), None);
        assert_eq!(lock_after("anything"), Some(Duration::from_secs(300)), "the default");
        let s = Duration::from_secs;
        assert_eq!(decide(false, false, false, false, s(10), five, false), None);
        assert_eq!(decide(false, false, false, false, s(300), five, false), Some(Why::Idle));
        // A private video playing keeps it open...
        assert_eq!(decide(false, false, false, false, s(900), five, true), None);
        // ...but never past Windows locking, sleep or the screen saver.
        assert_eq!(decide(false, false, true, false, s(0), five, true), Some(Why::WindowsLocked));
        assert_eq!(decide(false, true, false, false, s(0), None, true), Some(Why::Slept));
        assert_eq!(decide(false, false, false, true, s(0), None, false), Some(Why::ScreenSaver));
        assert_eq!(decide(false, false, false, false, s(99_999), None, false), None, "never: only the other reasons");
        assert_eq!(decide(true, false, false, false, s(0), None, true), Some(Why::Hidden), "minimised or in the tray");
    }

    #[test]
    fn sleep_is_seen_from_the_clocks() {
        let s = Duration::from_secs;
        assert!(!slept_between(s(1), Some(s(1))));
        assert!(slept_between(s(1), Some(s(600))), "the PC slept: the wall clock jumped");
        assert!(slept_between(s(40), Some(s(40))), "frozen that long");
        assert!(slept_between(s(1), None), "the clock went back");
    }

    #[test]
    fn idle_time_reads() {
        // Some value, never a panic (a test machine may have no input at all).
        let _ = idle_for();
        let _ = windows_locked();
        let _ = screen_saver_running();
    }
}
