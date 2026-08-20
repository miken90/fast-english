use super::client::AIClient;
use super::error;
use super::prompt::PromptBuilder;
use super::response::{check_completion, Completion, CompletionResponse};
use super::sse::{SseDecoder, SseItem};
use anyhow::Result;
use async_trait::async_trait;
use std::time::Instant;

pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    model: String,
    http: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(
        api_key: String,
        base_url: String,
        model: String,
        http: reqwest::Client,
    ) -> Result<Self> {
        Ok(Self {
            api_key,
            base_url,
            model,
            http,
        })
    }

    /// The chat-completions body. Streaming and non-streaming differ only in
    /// the `stream` flag, so the request itself is built in one place.
    fn completion_body(
        &self,
        text: &str,
        instruction: &str,
        system_override: Option<&str>,
        max_tokens_override: Option<usize>,
        stream: bool,
    ) -> serde_json::Value {
        let prompt = PromptBuilder::new();
        let system_prompt = system_override.unwrap_or(prompt.system_prompt());
        let max_tokens = max_tokens_override.unwrap_or_else(|| prompt.max_tokens(text));

        serde_json::json!({
            "model": &self.model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": prompt.user_prompt(text, instruction) }
            ],
            "temperature": 0.3,
            "max_tokens": max_tokens,
            "stream": stream,
        })
    }

    /// Send a request and hand back a response that is known to be a success.
    ///
    /// Every failure that can happen before the body is read — unreachable
    /// host, stalled server, rejected key, wrong model, rate limit, gateway
    /// page — is turned into a sentence here, so no caller ever has to deal
    /// with a status code or a raw body.
    async fn send(&self, body: &serde_json::Value) -> Result<reqwest::Response> {
        let resp = self
            .http
            .post(format!("{}/chat/completions", &self.base_url))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(error::describe_transport_error(&e, &self.base_url)))?;

        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }

        // Read the header before the body: consuming the body moves the
        // response, and Retry-After is the only actionable part of a 429.
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let body_text = resp.text().await.unwrap_or_default();

        Err(anyhow::anyhow!(error::describe_http_status(
            status.as_u16(),
            retry_after.as_deref(),
            &body_text,
            &self.base_url,
            &self.model,
        )))
    }
}

#[async_trait]
impl AIClient for OpenAIClient {
    async fn process(
        &self,
        text: &str,
        instruction: &str,
        system_override: Option<&str>,
        max_tokens_override: Option<usize>,
    ) -> Result<Completion> {
        let body = self.completion_body(
            text,
            instruction,
            system_override,
            max_tokens_override,
            false,
        );
        let resp = self.send(&body).await?;

        let raw = resp
            .text()
            .await
            .map_err(|e| anyhow::anyhow!(error::describe_transport_error(&e, &self.base_url)))?;

        let parsed: CompletionResponse = serde_json::from_str(&raw).map_err(|_| {
            anyhow::anyhow!(error::describe_http_status(
                200,
                None,
                &raw,
                &self.base_url,
                &self.model
            ))
        })?;

        parsed
            .into_content()
            .map_err(|reason| anyhow::anyhow!(reason))
    }

    async fn process_streaming(
        &self,
        text: &str,
        instruction: &str,
        system_override: Option<&str>,
        max_tokens_override: Option<usize>,
        on_chunk: &(dyn for<'a> Fn(&'a str) + Send + Sync),
    ) -> Result<Completion> {
        let body = self.completion_body(
            text,
            instruction,
            system_override,
            max_tokens_override,
            true,
        );

        let started = Instant::now();
        let mut resp = self.send(&body).await?;
        tracing::debug!(ms = started.elapsed().as_millis(), "response headers");

        let mut decoder = SseDecoder::new();
        let mut full = String::new();
        let mut first_delta_at: Option<u128> = None;

        // Reading chunk by chunk keeps this an await point per network read,
        // which is what lets a cancelled request abort the HTTP body promptly.
        // It is also where the client's read timeout applies, so a proxy that
        // goes quiet mid-answer fails instead of hanging.
        loop {
            let next = resp.chunk().await.map_err(|e| {
                anyhow::anyhow!(error::describe_transport_error(&e, &self.base_url))
            })?;
            let Some(bytes) = next else { break };

            for item in decoder.feed(&bytes) {
                match item {
                    SseItem::Delta(delta) => {
                        first_delta_at.get_or_insert_with(|| started.elapsed().as_millis());
                        on_chunk(&delta);
                        full.push_str(&delta);
                    }
                    SseItem::Error(message) => anyhow::bail!("{}", message),
                    SseItem::Done => return self.finish_stream(full, &decoder, first_delta_at),
                }
            }
        }

        for item in decoder.finish() {
            match item {
                SseItem::Delta(delta) => {
                    first_delta_at.get_or_insert_with(|| started.elapsed().as_millis());
                    on_chunk(&delta);
                    full.push_str(&delta);
                }
                SseItem::Error(message) => anyhow::bail!("{}", message),
                SseItem::Done => break,
            }
        }

        self.finish_stream(full, &decoder, first_delta_at)
    }

    async fn test_connection(&self) -> Result<String> {
        let body = serde_json::json!({
            "model": &self.model,
            "messages": [{ "role": "user", "content": "Say hello in 5 words." }],
            "max_tokens": 20,
        });
        let resp = self.send(&body).await?;

        let raw = resp
            .text()
            .await
            .map_err(|e| anyhow::anyhow!(error::describe_transport_error(&e, &self.base_url)))?;
        let parsed: CompletionResponse = serde_json::from_str(&raw).map_err(|_| {
            anyhow::anyhow!(error::describe_http_status(
                200,
                None,
                &raw,
                &self.base_url,
                &self.model
            ))
        })?;

        parsed
            .into_content()
            .map(|completion| completion.text)
            .map_err(|reason| anyhow::anyhow!(reason))
    }
}

impl OpenAIClient {
    /// Decide whether the text a stream delivered is an answer.
    ///
    /// A stream can end three ways that all look like success at the transport
    /// layer: a finished answer, nothing at all, and an answer the model was
    /// cut off partway through. The last of those is still a result — it comes
    /// back with a notice rather than being thrown away.
    fn finish_stream(
        &self,
        full: String,
        decoder: &SseDecoder,
        first_delta_ms: Option<u128>,
    ) -> Result<Completion> {
        tracing::debug!(
            first_token_ms = first_delta_ms.map(|ms| ms as u64),
            chars = full.chars().count(),
            finish_reason = decoder.finish_reason(),
            "stream finished"
        );
        let notice = check_completion(&full, decoder.finish_reason())
            .map_err(|reason| anyhow::anyhow!(reason))?;
        Ok(Completion { text: full, notice })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// A one-shot HTTP server that answers with exactly the bytes it is given.
    ///
    /// Every exception in this module is about a server that does not behave
    /// like the API contract says, so the tests need to be able to send a
    /// malformed, truncated, or silent response on purpose. `hold_open` keeps
    /// the connection alive without sending anything more, which is the
    /// stalled-stream case.
    async fn serve_once(response: &'static [u8], hold_open: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            // Drain the request headers so the client's write completes.
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let _ = socket.write_all(response).await;
            let _ = socket.flush().await;
            if hold_open {
                // Never send another byte, never close: the read timeout is
                // the only thing that can end this.
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        format!("http://{addr}/v1")
    }

    fn client_for(base_url: &str, http: reqwest::Client) -> OpenAIClient {
        OpenAIClient::new(
            "test-key".to_string(),
            base_url.to_string(),
            "gpt-4.1-mini".to_string(),
            http,
        )
        .unwrap()
    }

    fn quick_client() -> reqwest::Client {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .read_timeout(Duration::from_millis(400))
            .build()
            .unwrap()
    }

    /// Run a streaming request against a canned response and collect whatever
    /// text was painted before it ended.
    async fn stream_against(
        response: &'static [u8],
        hold_open: bool,
    ) -> (Result<Completion>, Vec<String>, String) {
        let base = serve_once(response, hold_open).await;
        let client = client_for(&base, quick_client());
        let painted = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = painted.clone();
        let on_chunk = move |delta: &str| sink.lock().unwrap().push(delta.to_string());

        let out = client
            .process_streaming("text", "Fix grammar.", None, Some(64), &on_chunk)
            .await;
        let chunks = painted.lock().unwrap().clone();
        (out, chunks, base)
    }

    /// The transport tests must not race the stall test's 400ms read budget,
    /// so they get their own client with room to fail on their own terms.
    fn patient_client() -> reqwest::Client {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .read_timeout(Duration::from_secs(5))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn a_refused_connection_names_the_endpoint_and_the_setting() {
        // Binding and then dropping a listener leaves a port the OS knows
        // nothing is on, so the connection is refused rather than filtered.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let base = format!("http://{addr}/v1");
        let client = client_for(&base, patient_client());
        let msg = client
            .process("text", "Fix.", None, None)
            .await
            .unwrap_err()
            .to_string();

        assert!(msg.contains("Could not connect"), "{msg}");
        assert!(msg.contains(&addr.to_string()), "{msg}");
        assert!(msg.contains("Settings"), "{msg}");
    }

    #[tokio::test]
    async fn a_name_that_does_not_resolve_is_reported_as_a_lookup_failure() {
        // `.invalid` is reserved by RFC 2606 and can never resolve, so this
        // needs no network of its own.
        let client = client_for("http://fast-english-test.invalid/v1", patient_client());
        let msg = client
            .process("text", "Fix.", None, None)
            .await
            .unwrap_err()
            .to_string();

        assert!(msg.contains("fast-english-test.invalid"), "{msg}");
        assert!(msg.contains("Settings"), "{msg}");
        // Whichever way the resolver fails, the user is never shown a raw
        // transport error.
        assert!(!msg.contains("reqwest"), "{msg}");
    }

    #[tokio::test]
    async fn a_stream_that_stalls_mid_answer_fails_instead_of_hanging() {
        let started = Instant::now();
        let (out, painted, _) = stream_against(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n\
              data: {\"choices\":[{\"delta\":{\"content\":\"half a sen\"}}]}\n\n",
            true,
        )
        .await;

        let msg = out.unwrap_err().to_string();
        assert!(msg.contains("stopped sending data"), "{msg}");
        // The text that did arrive was painted before the stall.
        assert_eq!(painted, ["half a sen"]);
        // And the read timeout, not the 30s the server holds, is what ended it.
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn a_rejected_key_is_reported_as_a_key_problem() {
        let (out, painted, _) = stream_against(
            b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
              {\"error\":{\"message\":\"Incorrect API key\"}}",
            false,
        )
        .await;

        let msg = out.unwrap_err().to_string();
        assert!(msg.contains("API key was rejected"), "{msg}");
        assert!(painted.is_empty());
    }

    #[tokio::test]
    async fn a_rate_limit_surfaces_the_wait_the_server_asked_for() {
        let (out, _, _) = stream_against(
            b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 20\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            false,
        )
        .await;

        let msg = out.unwrap_err().to_string();
        assert!(msg.contains("Rate limited"), "{msg}");
        assert!(msg.contains("20 seconds"), "{msg}");
    }

    #[tokio::test]
    async fn a_gateway_html_page_is_not_pasted_into_the_popup() {
        let (out, _, _) = stream_against(
            b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
              <html><head><title>502</title></head><body>x</body></html>",
            false,
        )
        .await;

        let msg = out.unwrap_err().to_string();
        assert!(msg.contains("web page instead of an API response"), "{msg}");
        assert!(!msg.contains("<html>"), "{msg}");
    }

    #[tokio::test]
    async fn a_server_error_with_an_empty_body_still_says_whose_fault_it_is() {
        let (out, _, _) = stream_against(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            false,
        )
        .await;

        let msg = out.unwrap_err().to_string();
        assert!(msg.contains("their side"), "{msg}");
        assert!(!msg.contains("Provider said"), "{msg}");
    }

    #[tokio::test]
    async fn a_two_hundred_carrying_an_error_frame_is_an_error() {
        let (out, painted, _) = stream_against(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n\
              data: {\"error\":{\"message\":\"upstream model unavailable\"}}\n\n",
            false,
        )
        .await;

        let msg = out.unwrap_err().to_string();
        assert!(msg.contains("upstream model unavailable"), "{msg}");
        assert!(painted.is_empty());
    }

    #[tokio::test]
    async fn a_stream_that_delivers_nothing_is_reported_as_an_empty_answer() {
        let (out, _, _) = stream_against(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n\
              data: {\"choices\":[{\"delta\":{}}]}\n\ndata: [DONE]\n\n",
            false,
        )
        .await;

        assert!(out.unwrap_err().to_string().contains("empty answer"));
    }

    #[tokio::test]
    async fn a_truncated_answer_is_kept_and_flagged_rather_than_discarded() {
        let (out, painted, _) = stream_against(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n\
              data: {\"choices\":[{\"delta\":{\"content\":\"She goes ho\"}}]}\n\n\
              data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n",
            false,
        )
        .await;

        let completion = out.expect("a truncated answer is still an answer");
        assert_eq!(completion.text, "She goes ho");
        let notice = completion.notice.expect("and it carries a warning");
        assert!(notice.contains("cut off"), "{notice}");
        assert!(notice.contains("shorter selection"), "{notice}");
        assert_eq!(painted, ["She goes ho"]);
    }

    #[tokio::test]
    async fn a_normal_stream_still_comes_back_whole() {
        let (out, painted, _) = stream_against(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n\
              data: {\"choices\":[{\"delta\":{\"content\":\"She goes \"}}]}\n\n\
              data: {\"choices\":[{\"delta\":{\"content\":\"home.\"},\"finish_reason\":\"stop\"}]}\n\n\
              data: [DONE]\n\n",
            false,
        )
        .await;

        assert_eq!(out.unwrap(), Completion::whole("She goes home."));
        assert_eq!(painted, ["She goes ", "home."]);
    }

    #[tokio::test]
    async fn a_proxy_that_buffers_instead_of_streaming_still_works() {
        // No SSE at all: a plain JSON completion answered to a `stream: true`
        // request. The decoder finds no frames, so the body has to be usable
        // some other way — here it is not, and the user is told so rather than
        // shown a blank result.
        let (out, _, _) = stream_against(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
              {\"choices\":[{\"message\":{\"content\":\"She goes home.\"}}]}",
            false,
        )
        .await;

        assert!(out.unwrap_err().to_string().contains("empty answer"));
    }

    #[tokio::test]
    async fn cancelling_mid_stream_stops_the_request() {
        let base = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n\
              data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
            true,
        )
        .await;
        let painted = Arc::new(AtomicBool::new(false));
        let saw = painted.clone();

        let task = tokio::spawn(async move {
            let client = client_for(&base, quick_client());
            let on_chunk = move |_: &str| saw.store(true, Ordering::Relaxed);
            client
                .process_streaming("text", "Fix.", None, Some(64), &on_chunk)
                .await
        });

        // Long enough for the first frame, far short of the server's hold.
        tokio::time::sleep(Duration::from_millis(150)).await;
        task.abort();

        assert!(task.await.unwrap_err().is_cancelled());
        assert!(painted.load(Ordering::Relaxed), "the first chunk painted");
    }

    #[test]
    fn the_request_body_carries_the_streaming_flag_and_the_budget() {
        let client = client_for("http://127.0.0.1:1/v1", reqwest::Client::new());
        let body = client.completion_body("hi", "Fix.", None, Some(321), true);

        assert_eq!(body["stream"], true);
        assert_eq!(body["max_tokens"], 321);
        assert_eq!(body["model"], "gpt-4.1-mini");
        assert_eq!(body["messages"][0]["role"], "system");
        assert!(body["messages"][1]["content"]
            .as_str()
            .unwrap()
            .contains("do not answer it"));

        // A system override replaces the shared rules wholesale.
        let overridden = client.completion_body("hi", "Fix.", Some("custom"), None, false);
        assert_eq!(overridden["messages"][0]["content"], "custom");
        assert_eq!(overridden["stream"], false);
    }
}
