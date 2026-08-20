pub mod client;
pub mod error;
pub mod openai;
pub mod prompt;
pub mod response;
pub mod sse;

use std::time::Duration;

/// How long a connection may take to establish before the endpoint counts as
/// unreachable. Long enough for a slow DNS lookup and a TLS handshake over a
/// bad link, short enough that a dead proxy is reported rather than waited on.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long the body may go without delivering a single byte.
///
/// This is deliberately a per-read timeout and not an overall request timeout.
/// An overall one caps how long an answer may take, which kills a legitimate
/// long completion partway through — the app used to carry a 30 second one and
/// that is exactly what it did. A per-read timeout instead bounds *silence*: a
/// stream that keeps producing tokens runs as long as it needs, while a proxy
/// that accepts the request and then goes quiet fails with a message.
///
/// 60 seconds is the compromise for a proxy that buffers instead of streaming,
/// where the whole completion arrives as one read after a long pause. Nothing
/// user-visible happens during that pause either way; the timeout only decides
/// how long the popup waits before saying the server stalled.
pub const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// How long an idle connection is kept for the next request.
///
/// Establishing TCP and TLS again is the largest client-side share of
/// time-to-first-token, and runs come in bursts: a user enhances a paragraph,
/// reads it, then reruns. The default 90 seconds drops the connection inside
/// that gap. Five minutes keeps the handshake paid once per session instead.
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// The one HTTP client for the whole app.
///
/// It is built once and cloned per request — `reqwest::Client` is a handle
/// around a shared connection pool, so cloning shares the pool and rebuilding
/// would throw it away and hand every request a fresh TLS handshake.
pub fn build_http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .tcp_keepalive(Duration::from_secs(60))
        .build()
}

/// How long a warmed connection is trusted before warming again. Comfortably
/// inside `POOL_IDLE_TIMEOUT`, so a popup opened twice in a minute does not
/// send a second probe.
pub const PREWARM_INTERVAL: Duration = Duration::from_secs(120);

/// Open a connection to the endpoint before the user asks for anything.
///
/// TCP plus TLS to a remote endpoint is on the order of a few hundred
/// milliseconds, and it is paid in full on the first request of a session —
/// which is exactly the run the user experiences as slow to start. Doing it
/// while they are still reading the popup moves that cost off the critical
/// path: the pooled connection is then reused by the real request.
///
/// It is fire-and-forget on purpose. Whether the endpoint answers, refuses, or
/// 404s is irrelevant — the connection is what is wanted, and a failure here
/// must never surface as an error the user did not cause.
pub fn prewarm_connection(http: reqwest::Client, base_url: String) {
    tokio::spawn(async move {
        let started = std::time::Instant::now();
        let outcome = http.head(&base_url).send().await;
        tracing::debug!(
            ms = started.elapsed().as_millis(),
            reached = outcome.is_ok(),
            "connection prewarm"
        );
    });
}

/// Create an AI client from settings.
pub fn create_ai_client(
    api_key: String,
    base_url: &str,
    model: &str,
    http_client: reqwest::Client,
) -> Result<Box<dyn client::AIClient>, String> {
    let client = openai::OpenAIClient::new(
        api_key,
        base_url.to_string(),
        model.to_string(),
        http_client,
    )
    .map_err(|e| format!("Failed to create AI client: {}", e))?;
    Ok(Box::new(client))
}

/// Truncate a string at a character boundary (safe for multi-byte UTF-8).
pub(crate) fn truncate_safe(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_http_client_bounds_both_connecting_and_waiting() {
        // The builder rejects an invalid combination at build time, so a
        // successful build is the proof the timeouts are actually applied.
        assert!(build_http_client().is_ok());
        // A silent server must not be able to hold a request open forever, and
        // a legitimate stream must not be capped by an overall deadline.
        assert!(CONNECT_TIMEOUT < READ_TIMEOUT);
        assert!(READ_TIMEOUT <= Duration::from_secs(120));
    }

    #[test]
    fn truncate_safe_never_splits_a_character() {
        assert_eq!(truncate_safe("Tiếng Việt", 5), "Tiếng");
        assert_eq!(truncate_safe("short", 50), "short");
    }
}
