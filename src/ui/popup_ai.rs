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
    tokio::spawn(async move {
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
                    match client
                        .process(&input, &instruction, system_override, max_tokens)
                        .await
                    {
                        Ok(raw) => Ok(clean_output_for_tool(&raw, &tool)),
                        Err(e) => Err(e.to_string()),
                    }
                }
                Err(e) => Err(e),
            }
        };

        let _ = internal_tx.send(InternalEvent::AiResponse { request_id, result });
    });
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
        _ => None,
    }
}
