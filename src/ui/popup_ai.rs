use crate::app::{PopupState, WritingToolKind, DEFAULT_TOOLS};
use crate::selection::SelectionContext;
use crate::settings::AppSettings;
use iced::Task;

use super::popup::Message;

/// Create an AI client from settings. Shared between request dispatch and connection test.
pub fn create_ai_client(
    api_key: String,
    base_url: &str,
    model: &str,
) -> Result<Box<dyn crate::ai::client::AIClient>, String> {
    let c = crate::ai::openai::OpenAIClient::new(api_key, base_url.to_string(), model.to_string())
        .map_err(|e| format!("Failed to create AI client: {}", e))?;
    Ok(Box::new(c))
}

/// Start an async AI request and return a Task that produces the result.
pub fn start_request(settings: &AppSettings, input: &str, tool: &WritingToolKind) -> Task<Message> {
    let settings = settings.clone();
    let input = input.to_string();
    let instruction = tool_instruction(tool);

    Task::perform(
        async move {
            let api_key = crate::persistence::KeyStore::load("api")
                .ok()
                .flatten()
                .unwrap_or_default();

            if api_key.is_empty() {
                return Err("No API key configured. Open Settings to add one.".to_string());
            }

            let client: Box<dyn crate::ai::client::AIClient> =
                create_ai_client(api_key, &settings.base_url, &settings.model)?;

            match client.process(&input, &instruction).await {
                Ok(raw) => Ok(crate::ai::prompt::PromptBuilder::clean_output(&raw)),
                Err(e) => Err(e.to_string()),
            }
        },
        Message::AiResponse,
    )
}

fn tool_instruction(tool: &WritingToolKind) -> String {
    match tool {
        WritingToolKind::EnhanceWriting => DEFAULT_TOOLS
            .iter()
            .find(|t| t.id == "enhance-writing")
            .map(|t| t.prompt_instruction)
            .unwrap_or("Improve the writing.")
            .to_string(),
        WritingToolKind::FixGrammar => DEFAULT_TOOLS
            .iter()
            .find(|t| t.id == "fix-grammar")
            .map(|t| t.prompt_instruction)
            .unwrap_or("Fix grammar.")
            .to_string(),
        WritingToolKind::ChangeTone(tone) => tone.prompt_instruction.to_string(),
        WritingToolKind::Translate { from, to } => {
            if from.id == "auto" {
                format!(
                    "Detect the language of the following text and translate it to {}. Return only the translated text.",
                    to.name
                )
            } else {
                format!(
                    "Translate the following text from {} to {}. Return only the translated text.",
                    from.name, to.name
                )
            }
        }
    }
}

/// Extract the SelectionContext from any popup state.
pub fn extract_context(state: &PopupState) -> Option<SelectionContext> {
    match state {
        PopupState::Processing { context, .. } => Some(context.clone()),
        PopupState::Result { context, .. } => Some(context.clone()),
        PopupState::TranslateResult { context, .. } => Some(context.clone()),
        PopupState::Error { context, .. } => Some(context.clone()),
        PopupState::Options(ctx)
        | PopupState::ToneSelection(ctx)
        | PopupState::TranslateSelection(ctx) => Some(ctx.clone()),
        PopupState::Hidden => None,
    }
}

/// Extract the WritingToolKind from processing/result states.
pub fn extract_tool(state: &PopupState) -> Option<WritingToolKind> {
    match state {
        PopupState::Processing { tool, .. } => Some(tool.clone()),
        PopupState::Result { tool, .. } => Some(tool.clone()),
        _ => None,
    }
}
