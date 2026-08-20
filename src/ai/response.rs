//! The chat-completions response shape, and the checks that decide whether a
//! successful HTTP status actually carried an answer.
//!
//! An OpenAI-compatible endpoint is a contract about field names, not about
//! behaviour: proxies answer 200 with an error object, with an empty `choices`
//! array, with a choice whose content is `null`, or with a completion the model
//! stopped emitting halfway through. Each of those reaches the user as a blank
//! popup unless it is caught here.

use serde::Deserialize;

use super::error;

#[derive(Deserialize)]
pub struct CompletionResponse {
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub error: Option<ProviderError>,
}

#[derive(Deserialize)]
pub struct Choice {
    #[serde(default)]
    pub message: Message,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct Message {
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct ProviderError {
    pub message: Option<String>,
}

impl CompletionResponse {
    /// The answer, or the reason there is not one.
    pub fn into_content(self) -> Result<String, String> {
        if let Some(reason) = error::describe_unusable_success(
            self.error
                .as_ref()
                .map(|e| e.message.as_deref().unwrap_or("")),
            !self.choices.is_empty(),
        ) {
            return Err(reason);
        }

        let choice = self.choices.into_iter().next().unwrap_or(Choice {
            message: Message::default(),
            finish_reason: None,
        });
        let content = choice.message.content.unwrap_or_default();

        check_completion(&content, choice.finish_reason.as_deref())?;
        Ok(content)
    }
}

/// Whether a completed answer is usable, whichever path produced it.
///
/// `length` means the model ran out of output budget mid-sentence. Handing that
/// back looks like a finished answer, and the user pastes a cut-off sentence
/// into their document without ever being told — so it is reported instead.
pub fn check_completion(content: &str, finish_reason: Option<&str>) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err(error::empty_completion_message());
    }
    if finish_reason == Some("length") {
        return Err(error::truncated_message());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &str) -> Result<String, String> {
        serde_json::from_str::<CompletionResponse>(body)
            .expect("test body is valid JSON")
            .into_content()
    }

    #[test]
    fn a_normal_completion_comes_back_as_its_content() {
        let out = parse(r#"{"choices":[{"message":{"content":"She goes home."}}]}"#).unwrap();
        assert_eq!(out, "She goes home.");
    }

    #[test]
    fn a_two_hundred_carrying_an_error_object_is_rejected() {
        let err = parse(r#"{"error":{"message":"insufficient quota"},"choices":[]}"#).unwrap_err();
        assert!(err.contains("insufficient quota"));
    }

    #[test]
    fn an_empty_choices_array_is_rejected() {
        let err = parse(r#"{"choices":[]}"#).unwrap_err();
        assert!(err.contains("model name in Settings"));
    }

    #[test]
    fn a_null_or_blank_content_is_reported_as_an_empty_answer() {
        assert!(parse(r#"{"choices":[{"message":{"content":null}}]}"#)
            .unwrap_err()
            .contains("empty answer"));
        assert!(parse(r#"{"choices":[{"message":{"content":"   \n "}}]}"#)
            .unwrap_err()
            .contains("empty answer"));
    }

    #[test]
    fn a_truncated_completion_is_reported_instead_of_returned() {
        let err = parse(
            r#"{"choices":[{"message":{"content":"She goes ho"},"finish_reason":"length"}]}"#,
        )
        .unwrap_err();
        assert!(err.contains("cut off"));

        // A normal stop reason is not a failure.
        assert!(parse(
            r#"{"choices":[{"message":{"content":"She goes home."},"finish_reason":"stop"}]}"#
        )
        .is_ok());
    }

    #[test]
    fn the_shared_check_covers_the_streaming_path_too() {
        assert!(check_completion("", None).is_err());
        assert!(check_completion("text", Some("length")).is_err());
        assert!(check_completion("text", Some("stop")).is_ok());
        assert!(check_completion("text", None).is_ok());
    }
}
