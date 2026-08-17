use crate::app::AppEvent;
use crate::selection::SelectionContext;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
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
///
/// Polling only produces auto-popups, so while `auto_popup` is off the loop
/// parks on the watch channel instead of making cross-process UIA calls that
/// nothing consumes. Flipping the setting resumes it in place — no restart.
/// The manual hotkey path does not go through here at all; it calls
/// [`try_manual_capture`] directly and keeps working while the loop is parked.
pub async fn run(tx: mpsc::Sender<AppEvent>, mut auto_popup: watch::Receiver<bool>) {
    let mut last_sent_text: Option<String> = None;
    let mut last_sent_app: String = String::new();
    let mut pending: Option<(SelectionContext, Instant)> = None;

    loop {
        if !*auto_popup.borrow_and_update() {
            // Drop the debounce state: a selection made while auto-popup was
            // off must not replay the moment it is switched back on.
            pending = None;
            last_sent_text = None;
            last_sent_app.clear();
            if wait_until_enabled(&mut auto_popup).await.is_err() {
                return; // Settings owner is gone — the app is shutting down.
            }
        }

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

/// Park until auto-popup is on. `Err` means every sender was dropped.
async fn wait_until_enabled(
    auto_popup: &mut watch::Receiver<bool>,
) -> Result<(), watch::error::RecvError> {
    while !*auto_popup.borrow_and_update() {
        auto_popup.changed().await?;
    }
    Ok(())
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

/// Manual trigger — tries UIA tiers first (non-invasive, no timing dependency),
/// then falls back to clipboard capture. Returns the selection context directly.
pub async fn try_manual_capture() -> Option<SelectionContext> {
    // Tier 1/2: UIA first — works without clipboard simulation, no modifier timing issues
    if let Some(ctx) = detect_selection() {
        return Some(ctx);
    }

    // Tier 3: Clipboard fallback — for apps where UIA doesn't expose selection.
    // Runs inside spawn_blocking because the clipboard capture path uses
    // blocking std::thread::sleep calls (up to ~1.88s total) which would
    // otherwise block a Tokio worker thread.
    #[cfg(target_os = "windows")]
    match tokio::task::spawn_blocking(|| super::clipboard_fallback::get_selection_via_clipboard())
        .await
    {
        Ok(Some(ctx)) => return Some(ctx),
        Ok(None) => {}
        Err(e) => tracing::error!("Clipboard capture task failed: {}", e),
    }

    debug!("Manual capture found no selection");
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn an_enabled_poller_does_not_park() {
        let (_tx, mut rx) = watch::channel(true);

        timeout(Duration::from_millis(100), wait_until_enabled(&mut rx))
            .await
            .expect("wait_until_enabled parked while auto-popup was on")
            .expect("channel closed");
    }

    #[tokio::test]
    async fn a_disabled_poller_parks_until_the_setting_is_turned_back_on() {
        let (tx, mut rx) = watch::channel(false);

        assert!(
            timeout(Duration::from_millis(100), wait_until_enabled(&mut rx))
                .await
                .is_err(),
            "the poll loop kept running with auto-popup off"
        );

        tx.send(true).expect("no receiver");
        timeout(Duration::from_millis(100), wait_until_enabled(&mut rx))
            .await
            .expect("the poll loop did not resume when auto-popup was re-enabled")
            .expect("channel closed");
    }

    #[tokio::test]
    async fn a_parked_poller_gives_up_when_the_settings_owner_is_gone() {
        let (tx, mut rx) = watch::channel(false);
        drop(tx);

        assert!(wait_until_enabled(&mut rx).await.is_err());
    }
}
