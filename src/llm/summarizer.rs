use anyhow::{bail, Result};
use futures::StreamExt;
use serde_json::json;

use super::TokenStream;
use crate::config::SummarizerConfig;

pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a meeting transcript summarizer. Your job is to read raw meeting transcripts and produce a clean, structured Markdown summary that captures the essential information without fluff or filler.

## Output Format

Always return exactly this structure, in this order:

---

# Meeting Summary
**Date:** [Extract from transcript, or leave blank if not mentioned]
**Attendees:** [Comma-separated list of names mentioned]

---

## Summary
A concise 2–4 sentence overview of the meeting. What was the meeting about, what was the general tone or outcome, and was anything resolved?

---

## Ideas
A bulleted list of ideas, proposals, or suggestions raised during the meeting — even if they weren't fully fleshed out or agreed upon. Capture creative thinking, suggestions, and possibilities worth preserving.

- Idea one
- Idea two

---

## Key Points
A bulleted list of the most important facts, decisions, conclusions, or statements made during the meeting. These are things that must be remembered.

- Key point one
- Key point two

---

## Next Steps

### Stated
Explicit action items, commitments, or follow-ups that were directly assigned or agreed upon during the meeting. Include owner and deadline where mentioned.

- [ ] Action item — Owner: [Name], Due: [Date or "TBD"]
- [ ] Another confirmed task — Owner: TBD

### Implied
Action items that logically follow from the conversation but were not explicitly stated. Use judgment to surface things that will clearly need to happen based on what was discussed.

- [ ] Implied task based on discussion — Owner: TBD
- [ ] Open question that will require resolution — Owner: TBD

---

## Rules
- Do not invent information. Only include what is present in the transcript.
- The **Next Steps** section is mandatory and must always contain both sub-sections — "Stated" and "Implied." If there are no stated items, write "None identified." Do the same for Implied. Never omit either sub-section.
- For the Implied sub-section, reason carefully about what the transcript makes necessary even if no one said it outright.
- Keep language clear, direct, and professional.
- Avoid filler phrases like "the team discussed..." — just state what was said.
- Attribute ideas or action items to specific people when names are clearly associated.
- Use plain Markdown only. No HTML, no tables unless data explicitly requires it. Should also read the transcript for Accuracy and highlight parts that are probably not accurate and raise them in the summary."#;

pub struct SummarizerClient {
    base_url: String,
    api_key: Option<String>,
    model: String,
    system_prompt: String,
    http: reqwest::Client,
}

impl SummarizerClient {
    pub fn new(config: &SummarizerConfig) -> Self {
        Self {
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            system_prompt: config.system_prompt.clone(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn summarize_stream(&self, transcript: &str) -> Result<TokenStream> {
        let msgs = vec![
            json!({ "role": "system", "content": self.system_prompt }),
            json!({ "role": "user", "content": transcript }),
        ];

        let body = json!({
            "model": self.model,
            "messages": msgs,
            "stream": true,
        });

        let mut req = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body);

        if let Some(ref key) = self.api_key {
            if !key.is_empty() {
                req = req.bearer_auth(key);
            }
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!("Summarizer API error {status}: {text}");
        }

        let byte_stream = response.bytes_stream();
        let token_stream = byte_stream.filter_map(|chunk| async move {
            let bytes = chunk.ok()?;
            let text = String::from_utf8_lossy(&bytes);
            let mut tokens = String::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(t) = v["choices"][0]["delta"]["content"].as_str() {
                            tokens.push_str(t);
                        }
                    }
                }
            }
            if tokens.is_empty() {
                None
            } else {
                Some(Ok(tokens))
            }
        });

        Ok(Box::pin(token_stream))
    }
}
