//! Turning a failed or unusable completion into something the user can act on.
//!
//! The app talks to any OpenAI-compatible endpoint, including proxies that are
//! nothing like OpenAI on the wire: they buffer, they answer with HTML from a
//! gateway in front of the model, they report failures with HTTP 200 and an
//! error object, and they occasionally send nothing at all. Every one of those
//! has to arrive at the user as a sentence naming the cause and the fix, which
//! is what this module is for. Nothing here ever hands back a bare status code
//! or a raw body dump.

use super::truncate_safe;

/// How much of an unrecognised body is worth showing. Long enough to carry a
/// short provider message, short enough that a stack trace cannot fill the
/// popup.
const BODY_PREVIEW: usize = 200;

/// Describe a request that never produced a response.
///
/// `reqwest` distinguishes connect, timeout and body failures, and the three
/// mean very different things to someone configuring a proxy: unreachable
/// host, a server that accepted the connection and then went quiet, and a
/// connection that died mid-answer.
pub fn describe_transport_error(err: &reqwest::Error, base_url: &str) -> String {
    describe_transport_failure(
        &TransportFailure {
            timed_out: err.is_timeout(),
            while_connecting: err.is_connect(),
            mid_body: err.is_body() || err.is_decode(),
            unsendable: err.is_request(),
        },
        &host_of(base_url),
        &error_chain(err),
    )
}

/// What `reqwest` said went wrong, lifted out of the error object so every
/// branch below can be exercised without provoking the real failure.
pub struct TransportFailure {
    pub timed_out: bool,
    pub while_connecting: bool,
    pub mid_body: bool,
    pub unsendable: bool,
}

fn describe_transport_failure(failure: &TransportFailure, host: &str, chain: &str) -> String {
    // A connect timeout is reported as both, so the timeout branch has to be
    // read first or "did not answer" would be described as "refused".
    if failure.timed_out {
        if failure.while_connecting {
            return format!(
                "{host} did not accept a connection in time. The server or your proxy is unreachable or overloaded — check that it is running."
            );
        }
        return format!(
            "{host} accepted the connection but stopped sending data. The server or your proxy is stalled — try again, or check its status."
        );
    }

    if failure.while_connecting {
        if chain.contains("dns")
            || chain.contains("name or service")
            || chain.contains("failed to lookup")
        {
            return format!(
                "Could not look up {host}. Check the base URL in Settings and your internet connection."
            );
        }
        return format!(
            "Could not connect to {host}. The server refused the connection or is not running — check the base URL in Settings and whether your proxy is up."
        );
    }

    if failure.mid_body {
        return format!("The connection to {host} broke while the answer was arriving. Try again.");
    }

    if failure.unsendable {
        return format!("The request to {host} could not be sent: {chain}");
    }

    format!("Could not reach {host}: {chain}")
}

/// Describe a response the server rejected.
///
/// `retry_after` is the raw `Retry-After` header when the server sent one, and
/// `body` is whatever it answered with — JSON, HTML, or nothing.
pub fn describe_http_status(
    status: u16,
    retry_after: Option<&str>,
    body: &str,
    base_url: &str,
    model: &str,
) -> String {
    let detail = provider_message(body);

    match status {
        401 | 403 => format!(
            "The API key was rejected (HTTP {status}). Open Settings and check the key for {}.{}",
            host_of(base_url),
            suffix(&detail)
        ),
        404 => format!(
            "No chat-completions endpoint at {base_url}, or no model named \"{model}\" (HTTP 404). Check the base URL and model in Settings.{}",
            suffix(&detail)
        ),
        429 => {
            let wait = retry_after
                .map(|v| format!(" Try again in {}.", humanise_retry_after(v)))
                .unwrap_or_else(|| " Wait a moment and try again.".to_string());
            format!("Rate limited by the provider (HTTP 429).{wait}{}", suffix(&detail))
        }
        400 | 413 | 422 if mentions_context_limit(body) => format!(
            "The text is too long for \"{model}\" to process in one request (HTTP {status}). Select less text and run it again.{}",
            suffix(&detail)
        ),
        400 | 422 => format!(
            "The provider rejected the request (HTTP {status}).{}",
            suffix(&detail)
        ),
        500..=599 => format!(
            "The provider failed while answering (HTTP {status}). This is on their side — try again in a moment.{}",
            suffix(&detail)
        ),
        _ => format!("The provider returned HTTP {status}.{}", suffix(&detail)),
    }
}

/// Describe a body that came back with a success status but nothing usable in
/// it. `None` means the response was fine.
///
/// A proxy that reports failures in-band is the reason this exists: HTTP 200
/// carrying `{"error": ...}`, or a `choices` array that is empty, both of which
/// otherwise reach the user as a blank result with no explanation.
pub fn describe_unusable_success(body_error: Option<&str>, had_choice: bool) -> Option<String> {
    if let Some(message) = body_error {
        let message = message.trim();
        if message.is_empty() {
            return Some("The provider reported an error but did not say what it was.".to_string());
        }
        return Some(format!("The provider reported an error: {message}"));
    }
    if !had_choice {
        return Some(
            "The provider answered without a completion. Check that the model name in Settings is one this endpoint serves."
                .to_string(),
        );
    }
    None
}

/// The stream broke after the answer had already partly arrived.
///
/// The text is kept, so this is a warning shown beside it rather than an error
/// screen. It carries the underlying cause because "the connection dropped" and
/// "the proxy stalled" ask the user for different things next.
pub fn interrupted_message(cause: &str) -> String {
    format!(
        "This answer stops short — the connection broke before the model finished. {} Run it again for the whole answer.",
        cause.trim()
    )
}

/// The model was cut off by its own output budget.
pub fn truncated_message() -> String {
    "The answer was cut off before it finished — the model hit its output limit. Run it again on a shorter selection.".to_string()
}

/// The model answered, but with nothing in it.
pub fn empty_completion_message() -> String {
    "The model returned an empty answer. Try running it again, or rephrase the text.".to_string()
}

/// Input that is not worth a request.
pub fn empty_input_message() -> String {
    "There is no text to work on. Type or select something first.".to_string()
}

/// Pull the human-readable part out of an error body, whatever shape it is in.
fn provider_message(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // `{"error": {"message": ...}}`, `{"error": "..."}`, `{"message": ...}`
        // and `{"detail": ...}` all appear in the wild.
        let candidate = value
            .get("error")
            .and_then(|e| e.get("message").or(Some(e)))
            .or_else(|| value.get("message"))
            .or_else(|| value.get("detail"));
        if let Some(text) = candidate.and_then(json_text) {
            return text;
        }
    }

    // An HTML page is a gateway or a login wall answering instead of the API.
    // Its markup tells the user nothing, so name the shape rather than show it.
    if trimmed.starts_with('<') {
        return "the server returned a web page instead of an API response, which usually means the base URL points at a site rather than an API".to_string();
    }

    truncate_safe(trimmed, BODY_PREVIEW).to_string()
}

fn json_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
            Some(truncate_safe(&value.to_string(), BODY_PREVIEW).to_string())
        }
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn suffix(detail: &str) -> String {
    if detail.is_empty() {
        String::new()
    } else {
        format!(" Provider said: {detail}")
    }
}

/// `Retry-After` is either seconds or an HTTP date. Only the seconds form is
/// worth rendering; a date is left as the server wrote it.
fn humanise_retry_after(value: &str) -> String {
    match value.trim().parse::<u64>() {
        Ok(0) | Ok(1) => "a second".to_string(),
        Ok(secs) if secs < 60 => format!("{secs} seconds"),
        Ok(secs) => format!("{} minutes", secs.div_ceil(60)),
        Err(_) => value.trim().to_string(),
    }
}

fn mentions_context_limit(body: &str) -> bool {
    let lower = body.to_lowercase();
    [
        "context length",
        "context_length",
        "context window",
        "too long",
        "maximum context",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

/// The host part of a base URL, for messages. Falls back to the whole string
/// when it is not a URL we can pick apart — a wrong value is exactly the case
/// worth showing verbatim.
fn host_of(base_url: &str) -> String {
    let without_scheme = base_url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(base_url);
    let host = without_scheme
        .split(['/', '?'])
        .next()
        .unwrap_or(without_scheme);
    if host.is_empty() {
        base_url.to_string()
    } else {
        host.to_string()
    }
}

/// Flatten a `reqwest` error and its sources into one lowercase string, which
/// is the only way to tell a DNS failure from a refused connection: the crate
/// reports both as `is_connect`.
fn error_chain(err: &reqwest::Error) -> String {
    let mut parts = vec![err.to_string()];
    let mut source = std::error::Error::source(err);
    while let Some(inner) = source {
        parts.push(inner.to_string());
        source = inner.source();
    }
    parts.join(": ").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://proxy.example.com/v1";
    const MODEL: &str = "gpt-4.1-mini";

    fn http(status: u16, body: &str) -> String {
        describe_http_status(status, None, body, BASE, MODEL)
    }

    #[test]
    fn a_rejected_key_names_the_key_and_the_host() {
        let msg = http(401, r#"{"error":{"message":"Incorrect API key"}}"#);
        assert!(msg.contains("API key was rejected"));
        assert!(msg.contains("proxy.example.com"));
        assert!(msg.contains("Incorrect API key"));
        assert!(http(403, "").contains("API key was rejected"));
    }

    #[test]
    fn a_404_points_at_the_base_url_and_the_model() {
        let msg = http(404, "");
        assert!(msg.contains(BASE));
        assert!(msg.contains(MODEL));
    }

    #[test]
    fn a_rate_limit_surfaces_retry_after_when_the_server_sends_one() {
        let with = describe_http_status(429, Some("30"), "", BASE, MODEL);
        assert!(with.contains("30 seconds"));

        let minutes = describe_http_status(429, Some("90"), "", BASE, MODEL);
        assert!(minutes.contains("2 minutes"));

        // A date is passed through rather than mis-rendered as seconds.
        let date =
            describe_http_status(429, Some("Wed, 21 Oct 2026 07:28:00 GMT"), "", BASE, MODEL);
        assert!(date.contains("21 Oct 2026"));

        // No header at all still gives an instruction.
        assert!(http(429, "").contains("Wait a moment"));
    }

    #[test]
    fn a_server_failure_says_it_is_the_providers_side() {
        for status in [500, 502, 503] {
            assert!(http(status, "").contains("their side"), "status {status}");
        }
    }

    #[test]
    fn an_over_length_input_is_named_as_such_instead_of_dumping_the_400() {
        let body = r#"{"error":{"message":"This model's maximum context length is 8192 tokens"}}"#;
        let msg = http(400, body);
        assert!(msg.contains("too long"));
        assert!(msg.contains(MODEL));

        // An unrelated 400 keeps the generic wording.
        let other = http(400, r#"{"error":{"message":"unsupported parameter"}}"#);
        assert!(!other.contains("too long"));
        assert!(other.contains("unsupported parameter"));
    }

    #[test]
    fn an_html_error_page_is_named_not_pasted() {
        let msg = http(
            502,
            "<!DOCTYPE html><html><body>502 Bad Gateway</body></html>",
        );
        assert!(msg.contains("web page instead of an API response"));
        assert!(!msg.contains("<html>"));
    }

    #[test]
    fn a_non_json_body_is_shown_but_bounded() {
        let msg = http(500, &"boom ".repeat(200));
        assert!(msg.contains("boom"));
        assert!(msg.len() < 400);
    }

    #[test]
    fn an_empty_body_leaves_the_status_sentence_alone() {
        let msg = http(503, "   ");
        assert!(!msg.contains("Provider said"));
    }

    #[test]
    fn a_success_carrying_an_error_object_is_still_an_error() {
        let msg = describe_unusable_success(Some("quota exceeded"), false).unwrap();
        assert!(msg.contains("quota exceeded"));

        // An error with no message still has to say something.
        assert!(describe_unusable_success(Some("  "), false).is_some());
    }

    #[test]
    fn a_success_with_no_choices_points_at_the_model_setting() {
        let msg = describe_unusable_success(None, false).unwrap();
        assert!(msg.contains("model name in Settings"));

        // A normal response is not an error.
        assert!(describe_unusable_success(None, true).is_none());
    }

    fn failure(timed_out: bool, while_connecting: bool) -> TransportFailure {
        TransportFailure {
            timed_out,
            while_connecting,
            mid_body: false,
            unsendable: false,
        }
    }

    #[test]
    fn a_connect_timeout_is_not_described_as_a_refusal() {
        // The proxy has an address but never completes the handshake — a hung
        // box, not a closed port.
        let msg = describe_transport_failure(&failure(true, true), "proxy.example.com", "");
        assert!(msg.contains("did not accept a connection in time"));
        assert!(msg.contains("proxy.example.com"));
    }

    #[test]
    fn a_stalled_response_is_described_as_a_stall() {
        let msg = describe_transport_failure(&failure(true, false), "proxy.example.com", "");
        assert!(msg.contains("stopped sending data"));
    }

    #[test]
    fn a_refused_connection_and_a_dns_failure_read_differently() {
        let refused = describe_transport_failure(
            &failure(false, true),
            "proxy.example.com",
            "connection refused",
        );
        assert!(refused.contains("refused the connection"));

        let dns = describe_transport_failure(
            &failure(false, true),
            "proxy.example.com",
            "dns error: failed to lookup address information",
        );
        assert!(dns.contains("Could not look up"));
    }

    #[test]
    fn a_connection_dying_mid_answer_says_so() {
        let msg = describe_transport_failure(
            &TransportFailure {
                timed_out: false,
                while_connecting: false,
                mid_body: true,
                unsendable: false,
            },
            "proxy.example.com",
            "connection reset",
        );
        assert!(msg.contains("broke while the answer was arriving"));
    }

    #[test]
    fn the_two_incomplete_answer_warnings_say_which_one_happened() {
        let hit_the_limit = truncated_message();
        assert!(hit_the_limit.contains("cut off"));
        assert!(hit_the_limit.contains("output limit"));

        let broke = interrupted_message("The connection to proxy.example.com broke.");
        assert!(broke.contains("stops short"));
        // The cause is carried through: a dropped connection and a stalled
        // proxy need different things from the user next.
        assert!(broke.contains("proxy.example.com"));
        assert!(broke.contains("Run it again"));
    }

    #[test]
    fn a_base_url_without_a_scheme_still_yields_a_host() {
        assert_eq!(host_of("https://api.openai.com/v1"), "api.openai.com");
        assert_eq!(host_of("http://127.0.0.1:8317/v1"), "127.0.0.1:8317");
        assert_eq!(host_of("not a url"), "not a url");
    }
}
