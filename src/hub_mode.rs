//! `--hub` mode: the my-utilities hub owns the tray and the settings UI, and
//! signals this process through named events in the user's session.

use crate::app::AppEvent;
use std::sync::OnceLock;
use tokio::sync::mpsc::Sender;
use tracing::{error, info};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE,
};

const HUB_MUTEX: PCWSTR = w!("Local\\FastEnglish.Hub");
const OPEN_EVENT: PCWSTR = w!("Local\\FastEnglish.Hub.Open");
const HISTORY_EVENT: PCWSTR = w!("Local\\FastEnglish.Hub.History");
const SETTINGS_EVENT_NAME: PCWSTR = w!("Local\\FastEnglish.Hub.SettingsRequested");

/// Raw event handle of the settings-request event; set once in hub mode.
/// Stored as `isize` because `HANDLE` is not `Sync`.
static SETTINGS_EVENT: OnceLock<isize> = OnceLock::new();

/// Holds the single-instance mutex for the life of the process.
pub struct HubGuard(HANDLE);

impl Drop for HubGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// `Ok(None)` means another `--hub` copy already owns the mutex.
pub fn acquire() -> windows::core::Result<Option<HubGuard>> {
    acquire_named(HUB_MUTEX)
}

fn acquire_named(name: PCWSTR) -> windows::core::Result<Option<HubGuard>> {
    unsafe {
        let handle = CreateMutexW(None, true, name)?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            return Ok(None);
        }
        Ok(Some(HubGuard(handle)))
    }
}

/// Create the auto-reset event the hub sets to ask for a settings window, and
/// remember it for [`request_hub_settings`].
pub fn create_settings_request_event() -> windows::core::Result<()> {
    let handle = create_auto_reset_event(SETTINGS_EVENT_NAME)?;
    let _ = SETTINGS_EVENT.set(handle.0 as isize);
    Ok(())
}

/// In hub mode the hub owns settings; its watcher opens the Fast English page.
/// Returns false in standalone mode (no event), so the caller opens its own
/// settings window.
pub fn request_hub_settings() -> bool {
    SETTINGS_EVENT
        .get()
        .map(|&raw| request_named(HANDLE(raw as *mut _)))
        .unwrap_or(false)
}

fn request_named(handle: HANDLE) -> bool {
    unsafe { SetEvent(handle).is_ok() }
}

/// The hub's "Open" action sets `Local\FastEnglish.Hub.Open`; focus does not
/// return to the previous window after the flyout closes, so the hotkey path
/// cannot be used. Each signal opens the popup like the tray item did.
pub fn spawn_open_listener(tx: Sender<AppEvent>) {
    spawn_listener(OPEN_EVENT, AppEvent::OpenPopupRequested, tx);
}

/// The hub's "Recent results" action sets `Local\FastEnglish.Hub.History`.
pub fn spawn_history_listener(tx: Sender<AppEvent>) {
    spawn_listener(HISTORY_EVENT, AppEvent::HistoryRequested, tx);
}

fn spawn_listener(name: PCWSTR, event: AppEvent, tx: Sender<AppEvent>) {
    let handle = match create_auto_reset_event(name) {
        Ok(h) => h.0 as isize,
        Err(e) => {
            error!(error = %e, "Hub event creation failed");
            return;
        }
    };
    std::thread::spawn(move || {
        let handle = HANDLE(handle as *mut _);
        info!(?event, "Hub listener started");
        loop {
            if unsafe { WaitForSingleObject(handle, INFINITE) } != WAIT_OBJECT_0 {
                break;
            }
            if tx.blocking_send(event.clone()).is_err() {
                break;
            }
        }
        unsafe {
            let _ = CloseHandle(handle);
        }
    });
}

fn create_auto_reset_event(name: PCWSTR) -> windows::core::Result<HANDLE> {
    unsafe { CreateEventW(None, false, false, name) }
}

#[cfg(test)]
pub(crate) fn signal_names() -> Vec<String> {
    unsafe {
        vec![
            SETTINGS_EVENT_NAME.to_string().unwrap(),
            OPEN_EVENT.to_string().unwrap(),
            HISTORY_EVENT.to_string().unwrap(),
        ]
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use windows::Win32::System::Threading::{OpenEventW, EVENT_ALL_ACCESS};

    /// Unique per process so parallel test runs and a running hub never clash.
    fn test_name(tag: &str) -> Vec<u16> {
        format!("Local\\FastEnglish.Test.{tag}.{}\0", std::process::id())
            .encode_utf16()
            .collect()
    }

    #[test]
    fn second_acquire_returns_none() {
        let name = test_name("mutex");
        let first = acquire_named(PCWSTR(name.as_ptr())).unwrap();
        assert!(first.is_some());
        let second = acquire_named(PCWSTR(name.as_ptr())).unwrap();
        assert!(second.is_none());
    }

    #[test]
    fn request_hub_settings_false_without_event() {
        assert!(!request_hub_settings());
    }

    #[test]
    fn settings_request_event_is_set() {
        let name = test_name("settings");
        let handle = create_auto_reset_event(PCWSTR(name.as_ptr())).unwrap();
        assert!(request_named(handle));
        unsafe {
            let opened = OpenEventW(EVENT_ALL_ACCESS, false, PCWSTR(name.as_ptr())).unwrap();
            assert_eq!(WaitForSingleObject(opened, 0), WAIT_OBJECT_0);
            let _ = CloseHandle(opened);
            let _ = CloseHandle(handle);
        }
    }

    #[test]
    fn open_signal_sends_open_popup_request() {
        let name = test_name("open");
        let (tx, mut rx) = tokio::sync::mpsc::channel::<AppEvent>(4);
        spawn_listener(PCWSTR(name.as_ptr()), AppEvent::OpenPopupRequested, tx);
        unsafe {
            let ev = OpenEventW(EVENT_ALL_ACCESS, false, PCWSTR(name.as_ptr())).unwrap();
            SetEvent(ev).unwrap();
            let _ = CloseHandle(ev);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match rx.try_recv() {
                Ok(AppEvent::OpenPopupRequested) => break,
                Ok(_) => panic!("unexpected event"),
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => panic!("open signal did not reach the channel"),
            }
        }
    }

    #[test]
    fn history_signal_sends_history_request() {
        let name = test_name("history");
        let (tx, mut rx) = tokio::sync::mpsc::channel::<AppEvent>(4);
        spawn_listener(PCWSTR(name.as_ptr()), AppEvent::HistoryRequested, tx);
        unsafe {
            let ev = OpenEventW(EVENT_ALL_ACCESS, false, PCWSTR(name.as_ptr())).unwrap();
            SetEvent(ev).unwrap();
            let _ = CloseHandle(ev);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match rx.try_recv() {
                Ok(AppEvent::HistoryRequested) => break,
                Ok(_) => panic!("unexpected event"),
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => panic!("history signal did not reach the channel"),
            }
        }
    }
}
