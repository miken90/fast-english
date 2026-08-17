use crate::app::WritingToolKind;
use crate::selection::SelectionContext;

use super::popup_runtime::{AppState, InternalEvent};
use super::popup_view::push_popup_state;
use crate::app::PopupState;
use tokio::sync::mpsc;

pub(crate) fn start_ai_request(
    state: &mut AppState,
    context: SelectionContext,
    tool: WritingToolKind,
    source_text: String,
    internal_tx: mpsc::UnboundedSender<InternalEvent>,
) {
    // A request that is being replaced stops reading its body here, so its
    // remaining chunks are never even produced.
    state.abort_active_request();

    let request_id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    state.active_request_id = Some(request_id);

    let input = source_text.clone();
    state.popup_state = PopupState::Processing {
        context,
        tool: tool.clone(),
        source_text,
    };
    push_popup_state(state);

    let settings = state.settings.clone();
    let http_client = state.http_client.clone();
    let task = tokio::spawn(async move {
        let api_key = crate::persistence::KeyStore::load("api")
            .ok()
            .flatten()
            .unwrap_or_default();

        let result = if api_key.is_empty() {
            Err("No API key configured. Open Settings to add one.".to_string())
        } else {
            match crate::ai::create_ai_client(
                api_key,
                &settings.base_url,
                &settings.model,
                http_client,
            ) {
                Ok(client) => {
                    let instruction = tool_instruction(&tool);
                    let system_override = system_override_for_tool(&tool);
                    let max_tokens = max_tokens_for_tool(&input, &tool);

                    // Streamed text is only ever painted raw. `clean_output`
                    // decides where a result ends from paragraph boundaries,
                    // which a half-arrived answer does not have, so it runs
                    // once at the end over the whole text.
                    let streamed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                    let chunk_tx = internal_tx.clone();
                    let saw_chunk = streamed.clone();
                    let on_chunk = move |delta: &str| {
                        saw_chunk.store(true, std::sync::atomic::Ordering::Relaxed);
                        let _ = chunk_tx.send(InternalEvent::AiChunk {
                            request_id,
                            text: delta.to_string(),
                        });
                    };

                    match client
                        .process_streaming(
                            &input,
                            &instruction,
                            system_override,
                            max_tokens,
                            &on_chunk,
                        )
                        .await
                    {
                        Ok(raw) => Ok(clean_output_for_tool(&raw, &tool)),
                        Err(e) => Err(stream_error_message(
                            &e.to_string(),
                            streamed.load(std::sync::atomic::Ordering::Relaxed),
                        )),
                    }
                }
                Err(e) => Err(e),
            }
        };

        let _ = internal_tx.send(InternalEvent::AiResponse { request_id, result });
    });

    state.active_request_task = Some(task);
}

/// Whether a background message still belongs to the request on screen.
///
/// Every streamed chunk goes through this alongside the final response: a
/// cancelled or superseded request can have chunks already queued, and those
/// must never paint into the request that replaced it.
pub(crate) fn accepts_response(active_request_id: Option<u64>, request_id: u64) -> bool {
    active_request_id == Some(request_id)
}

/// Whether a streamed chunk may be painted right now.
///
/// Beyond the id, the popup has to still be showing the request: once it has
/// moved to a result, an error, or been dismissed, there is no live surface for
/// partial text.
pub(crate) fn accepts_stream_chunk(
    active_request_id: Option<u64>,
    popup_state: &PopupState,
    request_id: u64,
) -> bool {
    accepts_response(active_request_id, request_id)
        && matches!(popup_state, PopupState::Processing { .. })
}

/// Whether a response earns a place in history.
///
/// Only a completed answer for the request still on screen counts. A cancelled
/// or superseded request fails the id guard — cancelling clears the id, so its
/// response is rejected here exactly as its chunks are — and a failed one has
/// no result text worth keeping.
pub(crate) fn records_history(
    active_request_id: Option<u64>,
    request_id: u64,
    result: &std::result::Result<String, String>,
) -> bool {
    accepts_response(active_request_id, request_id) && result.is_ok()
}

/// A stream that dies after partial text must not read as a finished answer.
fn stream_error_message(error: &str, had_partial_text: bool) -> String {
    if had_partial_text {
        format!("The response was cut off: {}", error)
    } else {
        error.to_string()
    }
}

fn tool_instruction(tool: &WritingToolKind) -> String {
    match tool {
        WritingToolKind::EnhanceWriting => crate::app::DEFAULT_TOOLS
            .iter()
            .find(|t| t.id == "enhance-writing")
            .map(|t| t.prompt_instruction)
            .unwrap_or("Improve the writing.")
            .to_string(),
        WritingToolKind::FixGrammar => crate::app::DEFAULT_TOOLS
            .iter()
            .find(|t| t.id == "fix-grammar")
            .map(|t| t.prompt_instruction)
            .unwrap_or("Fix grammar.")
            .to_string(),
        WritingToolKind::ConvertPrompt => crate::app::DEFAULT_TOOLS
            .iter()
            .find(|t| t.id == "convert-prompt")
            .map(|t| t.prompt_instruction)
            .unwrap_or("Turn this text into a clear prompt.")
            .to_string(),
        WritingToolKind::Translate { from, to } => {
            if from.id == "auto" {
                format!(
                    "Detect the language and translate to {}. Return only the translation.",
                    to.name
                )
            } else {
                format!(
                    "Translate from {} to {}. Return only the translation.",
                    from.name, to.name
                )
            }
        }
    }
}

fn system_override_for_tool(tool: &WritingToolKind) -> Option<&'static str> {
    match tool {
        WritingToolKind::ConvertPrompt => {
            Some(crate::ai::prompt::PromptBuilder::new().convert_prompt_system_prompt())
        }
        _ => None,
    }
}

fn clean_output_for_tool(raw: &str, tool: &WritingToolKind) -> String {
    match tool {
        WritingToolKind::ConvertPrompt => {
            crate::ai::prompt::PromptBuilder::clean_convert_prompt_output(raw)
        }
        _ => crate::ai::prompt::PromptBuilder::clean_output(raw),
    }
}

pub(crate) fn max_tokens_for_tool(input: &str, tool: &WritingToolKind) -> Option<usize> {
    match tool {
        WritingToolKind::ConvertPrompt => {
            Some(crate::ai::prompt::PromptBuilder::new().max_tokens_for_convert(input))
        }
        WritingToolKind::Translate { .. } => {
            Some(crate::ai::prompt::PromptBuilder::new().max_tokens_for_translate(input))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::{SelectionContext, Tier};

    fn context() -> SelectionContext {
        SelectionContext {
            text: String::from("source"),
            bounds: None,
            source_app: String::from("test"),
            detection_tier: Tier::ClipboardFallback,
            is_editable: false,
        }
    }

    fn processing() -> PopupState {
        PopupState::Processing {
            context: context(),
            tool: WritingToolKind::EnhanceWriting,
            source_text: String::from("source"),
        }
    }

    #[test]
    fn chunks_from_a_superseded_request_are_dropped() {
        // Request 1 is streaming when the user reruns; request 2 takes over.
        let active = Some(2);

        assert!(!accepts_stream_chunk(active, &processing(), 1));
        assert!(accepts_stream_chunk(active, &processing(), 2));
    }

    #[test]
    fn chunks_stop_once_the_request_is_cancelled_or_dismissed() {
        // abort_active_request clears the id; late chunks already in the
        // channel arrive after that.
        assert!(!accepts_stream_chunk(None, &processing(), 1));
        assert!(!accepts_response(None, 1));
    }

    #[test]
    fn chunks_are_dropped_once_the_popup_has_left_processing() {
        let done = PopupState::Result {
            context: context(),
            tool: WritingToolKind::EnhanceWriting,
            source_text: String::from("source"),
            output: String::from("done"),
            can_replace: false,
        };

        // The id still matches — only the view has moved on, and there is no
        // live region for partial text any more.
        assert!(accepts_response(Some(1), 1));
        assert!(!accepts_stream_chunk(Some(1), &done, 1));
        assert!(!accepts_stream_chunk(Some(1), &PopupState::Hidden, 1));
    }

    #[test]
    fn only_a_completed_result_is_recorded_in_history() {
        let ok: std::result::Result<String, String> = Ok("She goes home.".to_string());
        let failed: std::result::Result<String, String> = Err("HTTP 500".to_string());

        assert!(records_history(Some(1), 1, &ok));
        // Cancelled or dismissed: abort_active_request cleared the id before
        // this response arrived.
        assert!(!records_history(None, 1, &ok));
        // Superseded by a rerun.
        assert!(!records_history(Some(2), 1, &ok));
        // Finished, but with an error — there is no result to keep.
        assert!(!records_history(Some(1), 1, &failed));
    }

    #[test]
    fn a_cut_off_stream_is_reported_as_cut_off() {
        let partial = stream_error_message("connection reset", true);
        assert!(partial.contains("cut off"));
        assert!(partial.contains("connection reset"));

        // Nothing was painted, so the plain provider error is the whole story.
        assert_eq!(
            stream_error_message("HTTP 401: bad key", false),
            "HTTP 401: bad key"
        );
    }
}
