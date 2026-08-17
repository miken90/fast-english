use crate::app::{AppEvent, PopupState};
use crate::selection::SelectionContext;
use crate::settings::AppSettings;
use crate::ui::popup_position::WorkArea;
use crate::ui::settings_views::SettingsForm;
use crate::ui::webview_ipc::JsToRust;
use crate::ui::webview_popup::{spawn_webview_thread, WebViewPopupCommand};
use crate::ui::webview_settings::SettingsCommand;
use anyhow::Result;
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};

use super::popup_controller::{handle_app_event, handle_internal_event, handle_webview_ipc};
use super::popup_view::push_popup_state;

pub(crate) struct AppState {
    pub popup_state: PopupState,
    pub settings_form: Option<SettingsForm>,
    pub current_selection: Option<SelectionContext>,
    pub settings: AppSettings,
    pub http_client: reqwest::Client,
    pub popup_visible: bool,
    /// Popup top-left corner in physical screen pixels.
    pub popup_x: f64,
    pub popup_y: f64,
    /// Work area of the monitor the popup was last placed on. Height clamping
    /// and repositioning must follow the popup, not the primary monitor.
    pub popup_work: WorkArea,
    /// Cursor and popup origin captured when a drag starts, both physical.
    pub popup_drag_origin: Option<((f32, f32), (f64, f64))>,
    pub copy_feedback: bool,
    /// Recent results, loaded from disk at startup.
    pub history: crate::history::History,
    /// What the popup was showing before history opened, so Back can go back.
    pub history_return: Option<PopupState>,
    pub last_auto_shown: Option<String>,
    pub ignore_auto_until: Option<Instant>,
    pub next_request_id: u64,
    pub active_request_id: Option<u64>,
    /// The in-flight AI task, kept so a cancelled request can abort its HTTP
    /// read instead of streaming on into a view that has moved on.
    pub active_request_task: Option<tokio::task::JoinHandle<()>>,
    pub manual_capture_in_flight: bool,
    /// Live auto-popup setting for the selection poller, which parks itself
    /// while it is off.
    pub auto_popup_tx: watch::Sender<bool>,
    pub webview_popup_tx: std_mpsc::Sender<WebViewPopupCommand>,
    pub webview_settings_tx: std_mpsc::Sender<SettingsCommand>,
}

impl AppState {
    /// Stop the in-flight AI request: abort the task so the HTTP body stops
    /// being read, and drop the id so anything already queued is rejected.
    pub(crate) fn abort_active_request(&mut self) {
        if let Some(task) = self.active_request_task.take() {
            task.abort();
        }
        self.active_request_id = None;
    }

    /// Persist history, keeping the app running if the write fails.
    ///
    /// The file is small and written from the event loop: a result the user
    /// just saw is in memory either way, and a failed write costs them a
    /// history entry, never the result itself.
    pub(crate) fn save_history(&self) {
        if let Err(e) = crate::persistence::HistoryStore::save(&self.history) {
            tracing::warn!("Could not save history: {}", e);
        }
    }
}

pub(crate) enum InternalEvent {
    AiResponse {
        request_id: u64,
        result: std::result::Result<String, String>,
    },
    /// A piece of streamed text from a request that may no longer be current.
    AiChunk {
        request_id: u64,
        text: String,
    },
    ManualCaptureResult {
        ctx: SelectionContext,
        allow_empty: bool,
    },
    TestConnectionResult(std::result::Result<String, String>),
    ReplaceDone,
}

/// Run WebView runtime loop. Blocks until quit.
///
/// The main thread must pump Win32 messages for tray-icon and global-hotkey to
/// deliver events. The async event loop runs in a background thread instead.
pub fn run(
    event_rx: mpsc::Receiver<AppEvent>,
    _event_tx: mpsc::Sender<AppEvent>,
    settings: AppSettings,
    auto_popup_tx: watch::Sender<bool>,
    runtime: tokio::runtime::Runtime,
) -> Result<()> {
    let (webview_popup_tx, webview_popup_rx) = std_mpsc::channel::<WebViewPopupCommand>();
    let (webview_settings_tx, webview_settings_rx) = std_mpsc::channel::<SettingsCommand>();
    let (webview_ipc_tx, webview_ipc_rx) = mpsc::unbounded_channel::<JsToRust>();
    let (internal_tx, internal_rx) = mpsc::unbounded_channel::<InternalEvent>();

    let _webview_thread =
        spawn_webview_thread(webview_popup_rx, webview_settings_rx, webview_ipc_tx);

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let state = AppState {
        popup_state: PopupState::Hidden,
        settings_form: None,
        current_selection: None,
        settings,
        http_client,
        popup_visible: false,
        popup_x: 0.0,
        popup_y: 0.0,
        popup_work: WorkArea::fallback(),
        popup_drag_origin: None,
        copy_feedback: false,
        // A history that cannot be read comes back empty rather than stopping
        // the app from starting.
        history: crate::persistence::HistoryStore::load(),
        history_return: None,
        last_auto_shown: None,
        ignore_auto_until: None,
        next_request_id: 1,
        active_request_id: None,
        active_request_task: None,
        manual_capture_in_flight: false,
        auto_popup_tx,
        webview_popup_tx,
        webview_settings_tx,
    };

    push_popup_state(&state);

    // Spawn the async event loop in a background thread.
    // The main thread must remain free to pump Win32 messages (required by
    // tray-icon and global-hotkey crates on Windows).
    let main_thread_id = get_current_thread_id();
    std::thread::Builder::new()
        .name("event-loop".into())
        .spawn(move || {
            run_async_event_loop(
                runtime,
                event_rx,
                webview_ipc_rx,
                internal_tx,
                internal_rx,
                state,
                main_thread_id,
            );
        })
        .expect("Failed to spawn event loop thread");

    // Pump Win32 messages on the main thread so tray and hotkey events fire.
    run_win32_message_pump();

    Ok(())
}

/// Async event loop — runs on a background thread via `runtime.block_on()`.
fn run_async_event_loop(
    runtime: tokio::runtime::Runtime,
    mut event_rx: mpsc::Receiver<AppEvent>,
    mut webview_ipc_rx: mpsc::UnboundedReceiver<JsToRust>,
    internal_tx: mpsc::UnboundedSender<InternalEvent>,
    mut internal_rx: mpsc::UnboundedReceiver<InternalEvent>,
    mut state: AppState,
    main_thread_id: u32,
) {
    runtime.block_on(async move {
        let mut tick_1500ms = tokio::time::interval(Duration::from_millis(1500));
        tick_1500ms.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                maybe_event = event_rx.recv() => {
                    let Some(event) = maybe_event else {
                        break;
                    };
                    if !handle_app_event(&mut state, event, internal_tx.clone()) {
                        break;
                    }
                }

                maybe_ipc = webview_ipc_rx.recv() => {
                    let Some(ipc) = maybe_ipc else {
                        break;
                    };
                    handle_webview_ipc(&mut state, ipc, internal_tx.clone());
                }

                maybe_internal = internal_rx.recv() => {
                    let Some(internal) = maybe_internal else {
                        break;
                    };
                    handle_internal_event(&mut state, internal);
                }

                _ = tick_1500ms.tick() => {
                    if state.copy_feedback {
                        state.copy_feedback = false;
                        push_popup_state(&state);
                    }
                }
            }
        }

        let _ = state.webview_popup_tx.send(WebViewPopupCommand::Close);
        let _ = state.webview_settings_tx.send(SettingsCommand::Close);

        // Signal the main thread's Win32 message pump to exit.
        post_quit_to_thread(main_thread_id);
    });
}

/// Get current thread ID (Win32).
#[cfg(target_os = "windows")]
fn get_current_thread_id() -> u32 {
    unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }
}

#[cfg(not(target_os = "windows"))]
fn get_current_thread_id() -> u32 {
    0
}

/// Post WM_QUIT to a specific thread to break its GetMessage loop.
#[cfg(target_os = "windows")]
fn post_quit_to_thread(thread_id: u32) {
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
            thread_id,
            windows::Win32::UI::WindowsAndMessaging::WM_QUIT,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn post_quit_to_thread(_thread_id: u32) {}

/// Pump Win32 messages on the current thread.
///
/// Keeps tray-icon and global-hotkey alive by dispatching their window messages.
/// Returns when WM_QUIT is received.
#[cfg(target_os = "windows")]
fn run_win32_message_pump() {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };
    let mut msg = MSG::default();
    unsafe {
        // GetMessageW returns 0 on WM_QUIT, -1 on error.
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn run_win32_message_pump() {
    // On non-Windows, just park the thread until it's unparked.
    std::thread::park();
}
