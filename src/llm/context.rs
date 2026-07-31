use crate::{config::LlmConfig, notes::Note};

pub fn build_system_prompt(
    config: &LlmConfig,
    context_notes: &[Note],
    kazam_pages: &[String],
) -> String {
    let mut prompt = config.system_prompt.clone();

    if !context_notes.is_empty() {
        prompt.push_str("\n\n## Relevant Notes\n");
        for note in context_notes.iter().take(config.max_context_notes) {
            let excerpt = if note.body.len() > 2000 {
                &note.body[..2000]
            } else {
                &note.body
            };
            prompt.push_str(&format!(
                "\n### {}\n```markdown\n{}\n```\n",
                note.relative_path, excerpt
            ));
        }
    }

    if !kazam_pages.is_empty() {
        prompt.push_str("\n\n## Kazam KB Context\n");
        for page in kazam_pages.iter().take(10) {
            let excerpt = if page.len() > 1500 {
                &page[..1500]
            } else {
                page
            };
            prompt.push_str(&format!("\n```\n{excerpt}\n```\n"));
        }
    }

    prompt
}
