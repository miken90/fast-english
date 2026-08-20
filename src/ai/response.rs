//! The chat-completions response shape, and the checks that decide whether a
//! successful HTTP status actually carried an answer.
//!
//! An OpenAI-compatible endpoint is a contract about field names, not about
//! behaviour: proxies answer 200 with an error object, with an empty `choices`
//! array, with a choice whose content is `null`, or with a completion the model
//! stopped emitting halfway through. Each of those reaches the user as a blank
//! popup unless it is caught here.
//!
//! The last of those is the one that is not a failure. A completion the model
//! ran out of budget for is still text the user asked for, and throwing it away
//! leaves them with nothing when they could have had most of an answer. It
//! comes back as a `Completion` carrying a notice instead, so the popup can
//! show the text and say plainly that it stops short.

use serde::Deserialize;

use super::error;

/// A usable answer, and anything the user needs told about it.
///
/// `notice` is set when the text is worth showing but incomplete. It is a
/// warning, never an error: the caller renders a result either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub text: String,
    pub notice: Option<String>,
}

impl Completion {
    /// A whole answer, with nothing to warn about. Only the tests build one
    /// by hand — the real paths go through `into_content` or `finish_stream`.
    #[cfg(test)]
    pub fn whole(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            notice: None,
        }
    }
}

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
    pub fn into_content(self) -> Result<Completion, String> {
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

        let notice = check_completion(&content, choice.finish_reason.as_deref())?;
        Ok(Completion {
            text: content,
            notice,
        })
    }
}

/// Whether a completed answer is usable, whichever path produced it, and what
/// the user has to be told about it.
///
/// `length` means the model ran out of output budget mid-sentence. The text is
/// still what the user asked for, so it is returned — but silently handing back
/// a cut-off sentence lets them paste it somewhere without ever knowing, which
/// is why it comes with a notice the result view has to show.
///
/// An answer with no text in it is the one case with nothing to return.
pub fn check_completion(
    content: &str,
    finish_reason: Option<&str>,
) -> Result<Option<String>, String> {
    if content.trim().is_empty() {
        return Err(error::empty_completion_message());
    }
    if finish_reason == Some("length") {
        return Ok(Some(error::truncated_message()));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &str) -> Result<Completion, String> {
        serde_json::from_str::<CompletionResponse>(body)
            .expect("test body is valid JSON")
            .into_content()
    }

    #[test]
    fn a_normal_completion_comes_back_as_its_content() {
        let out = parse(r#"{"choices":[{"message":{"content":"She goes home."}}]}"#).unwrap();
        assert_eq!(out, Completion::whole("She goes home."));
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
    fn a_truncated_completion_keeps_its_text_and_gains_a_warning() {
        let out = parse(
            r#"{"choices":[{"message":{"content":"She goes ho"},"finish_reason":"length"}]}"#,
        )
        .unwrap();

        // The text the user asked for survives — throwing it away leaves them
        // with nothing when most of an answer was already paid for.
        assert_eq!(out.text, "She goes ho");
        let notice = out.notice.expect("a truncated answer carries a warning");
        assert!(notice.contains("cut off"));
        assert!(notice.contains("limit"));

        // A normal stop reason carries no warning.
        let whole = parse(
            r#"{"choices":[{"message":{"content":"She goes home."},"finish_reason":"stop"}]}"#,
        )
        .unwrap();
        assert_eq!(whole.notice, None);
    }

    #[test]
    fn the_shared_check_covers_the_streaming_path_too() {
        assert!(check_completion("", None).is_err());
        assert!(check_completion("text", Some("length")).unwrap().is_some());
        assert_eq!(check_completion("text", Some("stop")).unwrap(), None);
        assert_eq!(check_completion("text", None).unwrap(), None);
    }
}
