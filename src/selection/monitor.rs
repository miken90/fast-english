use crate::app::AppEvent;
use crate::selection::SelectionContext;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::debug;

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const DEBOUNCE_DELAY: Duration = Duration::from_millis(700);

/// Run the selection monitor loop. Polls for text selection changes via multi-tier
/// detection and sends events through the channel.
///
/// Uses debounce: waits for selection to be stable for DEBOUNCE_DELAY before
/// sending the event, preventing repeated popup opens during active text selection.
///
/// Polls for text selection changes every 500ms.
pub async fn run(tx: mpsc::Sender<AppEvent>) {
    let mut last_sent_text: Option<String> = None;
    let mut last_sent_app: String = String::new();
    let mut pending: Option<(SelectionContext, Instant)> = None;

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;

        match detect_selection() {
            Some(ctx) => {
                let text_changed = last_sent_text.as_ref() != Some(&ctx.text);
                let app_changed = last_sent_app != ctx.source_app;

                if text_changed || app_changed {
                    // Selection changed — start/restart debounce timer
                    let is_new_pending = match &pending {
                        Some((p, _)) => p.text != ctx.text || p.source_app != ctx.source_app,
                        None => true,
                    };
                    if is_new_pending {
                        pending = Some((ctx, Instant::now()));
                    }
                }

                // Check if pending selection has been stable long enough
                if let Some((ref pctx, changed_at)) = pending {
                    if changed_at.elapsed() >= DEBOUNCE_DELAY {
                        debug!(
                            tier = %pctx.detection_tier,
                            app = %pctx.source_app,
                            len = pctx.text.len(),
                            "Selection detected (debounced)"
                        );
                        last_sent_text = Some(pctx.text.clone());
                        last_sent_app = pctx.source_app.clone();
                        let ctx_to_send = pctx.clone();
                        pending = None;
                        let _ = tx.send(AppEvent::TextSelected(ctx_to_send)).await;
                    }
                }
            }
            None => {
                pending = None;
                if last_sent_text.is_some() {
                    last_sent_text = None;
                    last_sent_app.clear();
                    let _ = tx.send(AppEvent::SelectionCleared).await;
                }
            }
        }
    }
}

/// Attempt selection detection using the multi-tier strategy (polling path).
///
/// Tier 1 (UIA TextPattern) and Tier 2 (UIA ValuePattern) only.
/// Tier 3 (clipboard) is NOT used during polling — only on manual trigger.
fn detect_selection() -> Option<SelectionContext> {
    #[cfg(target_os = "windows")]
    {
        // Tier 1: UIA TextPattern — most reliable, works with modern apps
        if let Some(ctx) = super::uia_text::try_get_selection() {
            return Some(ctx);
        }

        // Tier 2: UIA ValuePattern — for simple Edit controls
        if let Some(ctx) = super::uia_value::try_get_selection() {
            return Some(ctx);
        }
    }

    None
}

/// Manual trigger — tries clipboard fallback first for universal app support,
/// then falls back to UIA tiers. Returns the selection context directly.
pub async fn try_manual_capture() -> Option<SelectionContext> {
    // Tier 3: Clipboard fallback first — works across almost all Windows apps
    #[cfg(target_os = "windows")]
    if let Some(ctx) = super::clipboard_fallback::get_selection_via_clipboard() {
        return Some(ctx);
    }

    // Fall back to non-invasive UIA tiers
    if let Some(ctx) = detect_selection() {
        return Some(ctx);
    }

    debug!("Manual capture found no selection");
    None
}
