//! Server-Sent Events decoding for streaming chat completions.
//!
//! The transport hands us arbitrary byte slices, so nothing about an event's
//! framing can be assumed: a single read may carry half a JSON frame, several
//! frames, or the middle of a multi-byte character. The decoder buffers bytes
//! and only ever hands back events it has seen in full.

use serde::Deserialize;

/// One item decoded out of the byte stream.
#[derive(Debug, PartialEq, Eq)]
pub enum SseItem {
    /// A piece of assistant text.
    Delta(String),
    /// The server reported a failure inside the stream body.
    Error(String),
    /// The server signalled the end of the stream.
    Done,
}

#[derive(Default)]
pub struct SseDecoder {
    /// Bytes of a line the server has not terminated yet.
    partial_line: Vec<u8>,
    /// `data:` payloads collected for the event currently being built.
    data: Vec<String>,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed raw bytes, returning every event that completed inside them.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseItem> {
        let mut out = Vec::new();
        for &byte in bytes {
            if byte != b'\n' {
                self.partial_line.push(byte);
                continue;
            }
            let line = std::mem::take(&mut self.partial_line);
            self.handle_line(&line, &mut out);
        }
        out
    }

    /// Flush what the body ended on. Servers usually close after a blank line,
    /// but a truncated one must not swallow the text it already delivered.
    pub fn finish(&mut self) -> Vec<SseItem> {
        let mut out = Vec::new();
        if !self.partial_line.is_empty() {
            let line = std::mem::take(&mut self.partial_line);
            self.handle_line(&line, &mut out);
        }
        self.dispatch(&mut out);
        out
    }

    fn handle_line(&mut self, line: &[u8], out: &mut Vec<SseItem>) {
        // A complete line can never split a UTF-8 character: 0x0A does not
        // appear inside a multi-byte sequence, so the boundary is always safe.
        let line = String::from_utf8_lossy(line);
        let line = line.trim_end_matches('\r');

        if line.is_empty() {
            self.dispatch(out);
            return;
        }
        // Comments are keep-alives, and no other field carries content.
        if line.starts_with(':') {
            return;
        }
        let Some(value) = line.strip_prefix("data:") else {
            return;
        };
        self.data
            .push(value.strip_prefix(' ').unwrap_or(value).to_string());
    }

    /// Turn the accumulated `data:` lines into an event. Per the SSE spec the
    /// lines of one event are joined with newlines.
    fn dispatch(&mut self, out: &mut Vec<SseItem>) {
        if self.data.is_empty() {
            return;
        }
        let payload = self.data.join("\n");
        self.data.clear();

        if payload.trim() == "[DONE]" {
            out.push(SseItem::Done);
            return;
        }

        // A frame we cannot read is skipped, not fatal: one malformed line must
        // not discard an answer that is otherwise still arriving.
        let Ok(frame) = serde_json::from_str::<StreamFrame>(&payload) else {
            return;
        };

        if let Some(error) = frame.error {
            out.push(SseItem::Error(
                error
                    .message
                    .unwrap_or_else(|| "the provider reported an error".to_string()),
            ));
            return;
        }

        for choice in frame.choices {
            if let Some(text) = choice.delta.content {
                if !text.is_empty() {
                    out.push(SseItem::Delta(text));
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct StreamFrame {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    error: Option<StreamError>,
}

#[derive(Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
}

#[derive(Deserialize, Default)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct StreamError {
    message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(content: &str) -> String {
        format!(
            "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{}\"}}}}]}}\n\n",
            content
        )
    }

    fn deltas(items: Vec<SseItem>) -> Vec<String> {
        items
            .into_iter()
            .filter_map(|i| match i {
                SseItem::Delta(t) => Some(t),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_whole_event_yields_its_delta() {
        let mut decoder = SseDecoder::new();
        assert_eq!(deltas(decoder.feed(frame("Hello").as_bytes())), ["Hello"]);
    }

    #[test]
    fn several_events_in_one_read_are_all_returned() {
        let mut decoder = SseDecoder::new();
        let body = format!("{}{}", frame("Hello "), frame("world"));

        assert_eq!(deltas(decoder.feed(body.as_bytes())), ["Hello ", "world"]);
    }

    #[test]
    fn an_event_split_across_reads_is_assembled() {
        let mut decoder = SseDecoder::new();
        let body = frame("Hello");
        let (head, tail) = body.as_bytes().split_at(20); // mid-JSON

        assert!(decoder.feed(head).is_empty());
        assert_eq!(deltas(decoder.feed(tail)), ["Hello"]);
    }

    #[test]
    fn a_character_split_across_reads_survives() {
        let mut decoder = SseDecoder::new();
        // "ế" is three UTF-8 bytes; the read boundary lands inside it.
        let body = frame("Ti\u{1ebf}ng");
        let split = body.find('\u{1ebf}').unwrap() + 1;
        let (head, tail) = body.as_bytes().split_at(split);

        let mut out = deltas(decoder.feed(head));
        out.extend(deltas(decoder.feed(tail)));
        assert_eq!(out, ["Ti\u{1ebf}ng"]);
    }

    #[test]
    fn done_is_reported_and_ends_the_stream() {
        let mut decoder = SseDecoder::new();
        let body = format!("{}data: [DONE]\n\n", frame("Hi"));

        let items = decoder.feed(body.as_bytes());
        assert_eq!(items[0], SseItem::Delta("Hi".to_string()));
        assert_eq!(items[1], SseItem::Done);
    }

    #[test]
    fn a_malformed_frame_is_skipped_without_losing_the_rest() {
        let mut decoder = SseDecoder::new();
        let body = format!("data: {{not json\n\n{}", frame("still here"));

        assert_eq!(deltas(decoder.feed(body.as_bytes())), ["still here"]);
    }

    #[test]
    fn keep_alives_and_other_fields_are_ignored() {
        let mut decoder = SseDecoder::new();
        let body = format!(": ping\nevent: message\nid: 42\n{}", frame("text"));

        assert_eq!(deltas(decoder.feed(body.as_bytes())), ["text"]);
    }

    #[test]
    fn a_multi_line_data_event_is_joined_with_newlines() {
        let mut decoder = SseDecoder::new();
        // The JSON of one event spread over two data: lines, as the SSE spec
        // permits.
        let body = "data: {\"choices\":[{\"delta\":\ndata: {\"content\":\"split\"}}]}\n\n";

        assert_eq!(deltas(decoder.feed(body.as_bytes())), ["split"]);
    }

    #[test]
    fn an_error_frame_is_surfaced() {
        let mut decoder = SseDecoder::new();
        let body = "data: {\"error\":{\"message\":\"rate limited\"}}\n\n";

        assert_eq!(
            decoder.feed(body.as_bytes()),
            [SseItem::Error("rate limited".to_string())]
        );
    }

    #[test]
    fn an_unterminated_final_event_is_flushed_on_finish() {
        let mut decoder = SseDecoder::new();
        // No trailing blank line: the connection ended right after the frame.
        assert!(decoder
            .feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"tail\"}}]}")
            .is_empty());

        assert_eq!(deltas(decoder.finish()), ["tail"]);
    }

    #[test]
    fn carriage_returns_do_not_leak_into_the_text() {
        let mut decoder = SseDecoder::new();
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"crlf\"}}]}\r\n\r\n";

        assert_eq!(deltas(decoder.feed(body.as_bytes())), ["crlf"]);
    }
}
