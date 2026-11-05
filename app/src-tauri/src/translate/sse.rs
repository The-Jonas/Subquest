//! Minimal parser for the Server-Sent Events stream of `chat/completions`.
//!
//! The body arrives in arbitrary chunks (a JSON line can be split across two
//! network reads), so the parser keeps the unfinished tail between calls.

use serde_json::Value;

/// What one complete `data:` line meant.
#[derive(Debug, PartialEq)]
pub enum SseEvent {
    Delta(String), // a piece of the answer text
    Done,          // `data: [DONE]`
}

/// Incremental SSE parser: feed it raw chunks, get back the events completed so far.
#[derive(Default)]
pub struct SseParser {
    pending: String, // bytes after the last newline, waiting for the rest of the line
}

impl SseParser {
    /// Adds a chunk and returns every event whose line is now complete.
    pub fn push(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.pending.push_str(chunk);
        let mut events = Vec::new();
        while let Some(pos) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=pos).collect();
            if let Some(event) = parse_line(line.trim()) {
                events.push(event);
            }
        }
        events
    }
}

/// Interprets one line: only `data:` lines matter; comments and blank lines are skipped.
fn parse_line(line: &str) -> Option<SseEvent> {
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some(SseEvent::Done);
    }
    let json: Value = serde_json::from_str(data).ok()?;
    let text = json["choices"][0]["delta"]["content"].as_str()?;
    if text.is_empty() {
        return None;
    }
    Some(SseEvent::Delta(text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_lines_split_across_chunks() {
        let mut p = SseParser::default();
        assert!(p.push("data: {\"choices\":[{\"delta\":{\"con").is_empty());
        assert_eq!(
            p.push("tent\":\"Olá\"}}]}\n\n"),
            vec![SseEvent::Delta("Olá".into())]
        );
    }

    #[test]
    fn handles_done_comments_and_empty_deltas() {
        let mut p = SseParser::default();
        let events = p.push(
            ": keep-alive\n\
             data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"\"}}]}\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"A ponte\"}}]}\n\
             data: [DONE]\n",
        );
        assert_eq!(
            events,
            vec![SseEvent::Delta("A ponte".into()), SseEvent::Done]
        );
    }
}
