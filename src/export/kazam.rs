/// Export a noterm note as a Kazam-compatible YAML page.
///
/// Output format:
///   title: <note title>
///   shell: document
///   owner: <frontmatter.owner>
///   freshness:
///     review_every: <frontmatter.review_every>
///     expires: <frontmatter.expires>
///     updated: <frontmatter.modified as YYYY-MM-DD>
///   components:
///     - type: markdown
///       body: |
///         <note body>
use std::path::{Path, PathBuf};

use crate::notes::frontmatter::NoteFrontmatter;

/// Build the YAML content for a Kazam page from noterm note data.
pub fn note_to_kazam_yaml(note_path: &Path, body: &str, frontmatter: &NoteFrontmatter) -> String {
    let title = frontmatter.title.as_deref().unwrap_or_else(|| {
        note_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
    });

    let mut lines = vec![
        format!("title: \"{}\"", title.replace('"', "'")),
        "shell: document".to_string(),
    ];

    if let Some(owner) = &frontmatter.owner {
        lines.push(format!("owner: \"{}\"", owner.replace('"', "'")));
    }

    // Freshness block (only if review_every is set)
    if frontmatter.review_every.is_some()
        || frontmatter.expires.is_some()
        || frontmatter.modified.is_some()
    {
        lines.push("freshness:".to_string());
        if let Some(re) = &frontmatter.review_every {
            lines.push(format!("  review_every: \"{}\"", re));
        }
        if let Some(exp) = &frontmatter.expires {
            lines.push(format!("  expires: \"{}\"", &exp[..10.min(exp.len())]));
        }
        if let Some(modified) = &frontmatter.modified {
            lines.push(format!(
                "  updated: \"{}\"",
                &modified[..10.min(modified.len())]
            ));
        }
    }

    // Components block
    lines.push("components:".to_string());
    lines.push("  - type: markdown".to_string());
    lines.push("    body: |".to_string());
    for line in body.lines() {
        lines.push(format!("      {line}"));
    }
    // Ensure trailing newline
    lines.push(String::new());

    lines.join("\n")
}

/// Export a note to `<kb_path>/<slug>.yaml`. Returns the written path.
pub fn export_note(
    note_path: &Path,
    body: &str,
    frontmatter: &NoteFrontmatter,
    kb_path: &Path,
) -> anyhow::Result<PathBuf> {
    let stem = note_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
        .replace(' ', "-");
    let slug = crate::import::sanitize_filename(&stem);
    let dest = kb_path.join(format!("{slug}.yaml"));

    let yaml = note_to_kazam_yaml(note_path, body, frontmatter);
    std::fs::write(&dest, yaml)?;
    Ok(dest)
}
