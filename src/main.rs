#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli_args;
mod history;
mod hotkey;
mod hub_contract;
mod hub_mode;
mod persistence;
mod settings;

mod ai;
mod editing;
mod selection;
mod ui;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("fast_english=info".parse()?))
        .init();

    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown".to_string());
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_string());
        eprintln!("panic at {location}: {payload}");
    }));

    let cli = cli_args::parse(std::env::args());

    // Only the hub-launched copy is single-instance; standalone behaviour
    // stays the same. Named (not `_`) so the mutex lives for the whole process.
    #[cfg(target_os = "windows")]
    let _hub_guard = if cli.hub {
        match hub_mode::acquire()? {
            Some(guard) => Some(guard),
            None => return Ok(()),
        }
    } else {
        None
    };
    #[cfg(target_os = "windows")]
    if cli.hub {
        // On failure the popup's settings button opens our own window instead.
        if let Err(e) = hub_mode::create_settings_request_event() {
            tracing::error!(error = %e, "Hub settings event creation failed");
        }
    }

    info!("Fast English starting...");

    // Initialize COM as STA on the main thread.
    // STA is required for tray-icon/global-hotkey Win32 message loop.
    // The uiautomation crate handles its own COM init on worker threads.
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            anyhow::bail!("COM initialization failed: {:?}", hr);
        }
    }

    info!("COM initialized");

    // Load settings from disk
    let settings = settings::AppSettings::load();
    info!(base_url = %settings.base_url, model = %settings.model, "Settings loaded");

    // Create the async runtime for background tasks (selection monitor, AI)
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;

    // Create event channel for cross-component communication
    let (event_tx, event_rx) = tokio::sync::mpsc::channel::<app::AppEvent>(100);

    // Start selection monitor (background polling).
    // The poll loop only feeds auto-popup, so it follows that setting live via
    // a watch channel the settings handler updates.
    let (auto_popup_tx, auto_popup_rx) = tokio::sync::watch::channel(settings.auto_popup_enabled);
    let monitor_tx = event_tx.clone();
    runtime.spawn(async move {
        selection::monitor::run(monitor_tx, auto_popup_rx).await;
    });

    info!("Selection monitor started");

    // Register global hotkey (continues without it if registration fails)
    #[cfg(target_os = "windows")]
    let _hotkey_manager = hotkey::register_hotkey(event_tx.clone());

    // Create system tray (continues without it if creation fails).
    // Hub mode has no tray: the hub owns it.
    #[cfg(target_os = "windows")]
    let _tray = if cli.hub {
        None
    } else {
        ui::tray::create_tray(event_tx.clone())
    };

    #[cfg(target_os = "windows")]
    if cli.hub {
        hub_mode::spawn_open_listener(event_tx.clone());
    }

    info!("Fast English ready");

    // Run WebView-based popup runtime (blocks until exit).
    ui::popup::run(event_rx, event_tx, settings, auto_popup_tx, runtime)?;

    // Cleanup COM
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Com::CoUninitialize;
        CoUninitialize();
    }

    info!("Fast English exited");
    Ok(())
}
