use crate::tasks::Task;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceOfTruth {
    pub label: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoteFrontmatter {
    pub title: Option<String>,
    pub id: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tasks: Option<Vec<Task>>,
    pub status: Option<String>,
    /// Explicit opt-in for publishing this Markdown note to the Kazam KB.
    #[serde(default)]
    pub publish: bool,
    // Freshness / review metadata (Kazam-compatible)
    pub owner: Option<String>,
    pub review_every: Option<String>,
    pub expires: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources_of_truth: Option<Vec<SourceOfTruth>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

pub fn parse(raw: &str) -> (NoteFrontmatter, String) {
    if !raw.starts_with("---") {
        return (NoteFrontmatter::default(), raw.to_string());
    }

    let after_first = &raw[3..];
    let end = after_first.find("\n---");

    match end {
        None => (NoteFrontmatter::default(), raw.to_string()),
        Some(pos) => {
            let yaml_str = &after_first[..pos];
            let body_start = pos + 4; // skip "\n---"
            let body = after_first[body_start..]
                .trim_start_matches('\n')
                .to_string();

            let fm: NoteFrontmatter = serde_yaml::from_str(yaml_str).unwrap_or_default();
            (fm, body)
        }
    }
}

pub fn serialize(fm: &NoteFrontmatter, body: &str) -> String {
    let yaml = serde_yaml::to_string(fm).unwrap_or_default();
    // serde_yaml adds a leading "---\n" so we strip it if present
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml);
    format!("---\n{yaml}---\n\n{body}")
}
