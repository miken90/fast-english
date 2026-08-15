use crate::ui::webview_ipc::{check_webview2_runtime, JsToRust};
use crate::ui::webview_settings::SettingsCommand;
use std::sync::mpsc as std_mpsc;
use std::thread::JoinHandle;
use tracing::{error, info, warn};

/// Commands sent from app logic to the WebView popup window.
///
/// All geometry is in **physical pixels**. Logical units would be converted by
/// tao using the window's currently-known monitor, which is the wrong monitor
/// while the popup is being moved onto another display.
#[derive(Debug)]
pub enum WebViewPopupCommand {
    Show {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Hide,
    Resize {
        width: f64,
        height: f64,
    },
    Move {
        x: f64,
        y: f64,
    },
    SetState {
        json: String,
    },
    EvaluateScript {
        js: String,
    },
    Close,
}

/// Unified command type for the single tao EventLoop that manages both windows.
#[derive(Debug)]
enum UiCommand {
    Popup(WebViewPopupCommand),
    Settings(SettingsCommand),
}

/// Spawn a single tao EventLoop thread that hosts both the popup and settings
/// WebView windows. tao only allows one EventLoop per process.
///
/// - `popup_rx`: receives popup commands
/// - `settings_rx`: receives settings commands
/// - `ipc_tx`: forwards JS→Rust messages back to tokio world
#[cfg(target_os = "windows")]
pub fn spawn_webview_thread(
    popup_rx: std_mpsc::Receiver<WebViewPopupCommand>,
    settings_rx: std_mpsc::Receiver<SettingsCommand>,
    ipc_tx: tokio::sync::mpsc::UnboundedSender<JsToRust>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("webview-ui".into())
        .spawn(move || {
            match check_webview2_runtime() {
                Ok(ver) => info!("WebView2 Runtime v{}", ver),
                Err(e) => {
                    error!("WebView2 not available: {}", e);
                    return;
                }
            }
            run_event_loop(popup_rx, settings_rx, ipc_tx);
        })
        .expect("Failed to spawn WebView UI thread")
}

#[cfg(target_os = "windows")]
fn run_event_loop(
    popup_rx: std_mpsc::Receiver<WebViewPopupCommand>,
    settings_rx: std_mpsc::Receiver<SettingsCommand>,
    ipc_tx: tokio::sync::mpsc::UnboundedSender<JsToRust>,
) {
    use tao::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
    use tao::event::{Event, StartCause};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::window::WindowBuilder;
    use wry::{WebContext, WebViewBuilder};

    // --- Assets ---
    let popup_html = include_str!("../../assets/popup/index.html");
    let tokens_css = include_str!("../../assets/popup/tokens.css");
    let base_css = include_str!("../../assets/popup/base.css");
    let components_css = include_str!("../../assets/popup/components.css");
    let popup_css = include_str!("../../assets/popup/popup.css");
    let popup_icons_js = include_str!("../../assets/popup/icons.js");
    let popup_js = include_str!("../../assets/popup/popup.js");
    let full_popup_html = inline_assets(
        popup_html,
        &[
            (tokens_css, "tokens.css"),
            (base_css, "base.css"),
            (components_css, "components.css"),
            (popup_css, "popup.css"),
        ],
        &format!("{}\n{}", popup_icons_js, popup_js),
        "popup.js",
    );

    let settings_html = include_str!("../../assets/popup/settings.html");
    let settings_css = include_str!("../../assets/popup/settings.css");
    let settings_icons_js = include_str!("../../assets/popup/icons.js");
    let settings_js = include_str!("../../assets/popup/settings.js");
    let full_settings_html = inline_assets(
        settings_html,
        &[
            (tokens_css, "tokens.css"),
            (base_css, "base.css"),
            (components_css, "components.css"),
            (settings_css, "settings.css"),
        ],
        &format!("{}\n{}", settings_icons_js, settings_js),
        "settings.js",
    );

    // --- EventLoop (one per process) ---
    #[cfg(target_os = "windows")]
    use tao::platform::windows::{EventLoopBuilderExtWindows, WindowBuilderExtWindows};

    let webview_data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()))
        .join("FastEnglish")
        .join("webview-data");
    let _ = std::fs::create_dir_all(&webview_data_dir);
    let mut webview_context = WebContext::new(Some(webview_data_dir));

    let event_loop = {
        let mut builder = EventLoopBuilder::<UiCommand>::with_user_event();
        #[cfg(target_os = "windows")]
        builder.with_any_thread(true);
        builder.build()
    };
    let proxy = event_loop.create_proxy();

    // --- Popup window (transparent, borderless, always-on-top) ---
    let popup_window = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(false)
        .with_always_on_top(true)
        .with_skip_taskbar(true)
        .with_inner_size(LogicalSize::new(440.0_f64, 340.0_f64))
        .with_visible(false)
        .with_title("Fast English")
        .build(&event_loop)
        .expect("Failed to create popup window");

    let ipc_tx_popup = ipc_tx.clone();
    let popup_webview = WebViewBuilder::new_with_web_context(&mut webview_context)
        .with_html(&full_popup_html)
        .with_ipc_handler(move |request| {
            let body = request.body();
            match JsToRust::from_json(body) {
                Ok(msg) => {
                    let _ = ipc_tx_popup.send(msg);
                }
                Err(e) => warn!("Popup IPC parse error: {} — body_length: {}", e, body.len()),
            }
        })
        .build(&popup_window)
        .expect("Failed to create popup WebView");

    // --- Settings window (normal, borderless, starts hidden) ---
    let settings_window = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(false)
        .with_always_on_top(false)
        .with_skip_taskbar(true)
        .with_inner_size(LogicalSize::new(380.0_f64, 560.0_f64))
        .with_visible(false)
        .with_title("Fast English — Settings")
        .build(&event_loop)
        .expect("Failed to create settings window");

    let ipc_tx_settings = ipc_tx.clone();
    let settings_webview = WebViewBuilder::new_with_web_context(&mut webview_context)
        .with_html(&full_settings_html)
        .with_ipc_handler(move |request| {
            let body = request.body();
            match JsToRust::from_json(body) {
                Ok(msg) => {
                    let _ = ipc_tx_settings.send(msg);
                }
                Err(e) => warn!(
                    "Settings IPC parse error: {} — body_length: {}",
                    e,
                    body.len()
                ),
            }
        })
        .build(&settings_window)
        .expect("Failed to create settings WebView");

    // --- Command forwarders: std_mpsc → tao EventLoopProxy ---
    let proxy_popup = proxy.clone();
    std::thread::Builder::new()
        .name("popup-cmd-fwd".into())
        .spawn(move || {
            while let Ok(cmd) = popup_rx.recv() {
                if proxy_popup.send_event(UiCommand::Popup(cmd)).is_err() {
                    break;
                }
            }
        })
        .expect("Failed to spawn popup command forwarder");

    let proxy_settings = proxy;
    std::thread::Builder::new()
        .name("settings-cmd-fwd".into())
        .spawn(move || {
            while let Ok(cmd) = settings_rx.recv() {
                if proxy_settings.send_event(UiCommand::Settings(cmd)).is_err() {
                    break;
                }
            }
        })
        .expect("Failed to spawn settings command forwarder");

    // --- Unified event loop ---
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                info!("WebView UI event loop started (popup + settings)");
            }
            Event::UserEvent(cmd) => match cmd {
                // --- Popup commands ---
                UiCommand::Popup(pcmd) => match pcmd {
                    WebViewPopupCommand::Show {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        // Position first: it decides which monitor the window
                        // belongs to before the size is applied.
                        popup_window.set_outer_position(PhysicalPosition::new(x, y));
                        popup_window.set_inner_size(PhysicalSize::new(width, height));
                        popup_window.set_visible(true);
                        popup_window.set_focus();
                    }
                    WebViewPopupCommand::Hide => {
                        popup_window.set_visible(false);
                    }
                    WebViewPopupCommand::Resize { width, height } => {
                        popup_window.set_inner_size(PhysicalSize::new(width, height));
                    }
                    WebViewPopupCommand::Move { x, y } => {
                        popup_window.set_outer_position(PhysicalPosition::new(x, y));
                    }
                    WebViewPopupCommand::SetState { json } => {
                        let script = format!("setState({})", json);
                        let _ = popup_webview.evaluate_script(&script);
                    }
                    WebViewPopupCommand::EvaluateScript { js } => {
                        let _ = popup_webview.evaluate_script(&js);
                    }
                    WebViewPopupCommand::Close => {
                        *control_flow = ControlFlow::Exit;
                    }
                },
                // --- Settings commands ---
                UiCommand::Settings(scmd) => match scmd {
                    SettingsCommand::Show => {
                        if let Some(monitor) = settings_window.current_monitor() {
                            let msize: tao::dpi::PhysicalSize<u32> = monitor.size();
                            let wsize = settings_window.inner_size();
                            let cx = (msize.width.saturating_sub(wsize.width)) / 2;
                            let cy = (msize.height.saturating_sub(wsize.height)) / 2;
                            settings_window
                                .set_outer_position(tao::dpi::PhysicalPosition::new(cx, cy));
                        }
                        settings_window.set_visible(true);
                        settings_window.set_focus();
                    }
                    SettingsCommand::Hide => {
                        settings_window.set_visible(false);
                    }
                    SettingsCommand::SetSettings { json } => {
                        let script = format!("setSettings({})", json);
                        let _ = settings_webview.evaluate_script(&script);
                    }
                    SettingsCommand::SetTestResult { json } => {
                        let script = format!("setTestResult({})", json);
                        let _ = settings_webview.evaluate_script(&script);
                    }
                    SettingsCommand::SetSaveFeedback { json } => {
                        let script = format!("setSaveFeedback({})", json);
                        let _ = settings_webview.evaluate_script(&script);
                    }
                    SettingsCommand::Close => {
                        *control_flow = ControlFlow::Exit;
                    }
                },
            },
            _ => {}
        }
    });
}

/// Inline CSS and JS assets into the HTML so `with_html()` works without relative paths.
///
/// `css_files` is a slice of `(css_content, css_filename)` pairs — each matching
/// `<link rel="stylesheet" href="{css_filename}">` is replaced with an inline `<style>` block.
fn inline_assets(html: &str, css_files: &[(&str, &str)], js: &str, js_filename: &str) -> String {
    let mut result = html.to_string();
    for &(css_content, css_filename) in css_files {
        let css_link = format!(r#"<link rel="stylesheet" href="{}">"#, css_filename);
        result = result.replace(&css_link, &format!("<style>{}</style>", css_content));
    }
    let js_tag = format!(r#"<script src="{}"></script>"#, js_filename);
    result = result.replace(&js_tag, &format!("<script>{}</script>", js));
    result
}

/// Non-Windows stub — WebView is Windows-only.
#[cfg(not(target_os = "windows"))]
pub fn spawn_webview_thread(
    _popup_rx: std_mpsc::Receiver<WebViewPopupCommand>,
    _settings_rx: std_mpsc::Receiver<SettingsCommand>,
    _ipc_tx: tokio::sync::mpsc::UnboundedSender<JsToRust>,
) -> JoinHandle<()> {
    std::thread::spawn(|| {
        tracing::warn!("WebView UI is not available on this platform");
    })
}
