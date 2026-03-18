#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod hotkey;
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
        .enable_all()
        .build()?;

    // Create event channel for cross-component communication
    let (event_tx, event_rx) = tokio::sync::mpsc::channel::<app::AppEvent>(100);

    // Start selection monitor (background polling)
    let monitor_tx = event_tx.clone();
    runtime.spawn(async move {
        selection::monitor::run(monitor_tx).await;
    });

    info!("Selection monitor started");

    // Register global hotkey (continues without it if registration fails)
    #[cfg(target_os = "windows")]
    let _hotkey_manager = hotkey::register_hotkey(event_tx.clone());

    // Create system tray (continues without it if creation fails)
    #[cfg(target_os = "windows")]
    let _tray = ui::tray::create_tray(event_tx.clone());

    info!("Fast English ready");

    // Run WebView-based popup runtime (blocks until exit).
    ui::popup::run(event_rx, event_tx, settings, runtime)?;

    // Cleanup COM
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Com::CoUninitialize;
        CoUninitialize();
    }

    info!("Fast English exited");
    Ok(())
}
