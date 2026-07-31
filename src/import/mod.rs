pub mod api;
pub mod meetily;
pub mod watcher;

use anyhow::Result;
use chrono::Utc;
use std::path::{Path, PathBuf};

use crate::{config::ImportConfig, notes::frontmatter};

/// Convert a file in the inbox into a properly frontmattered .md note.
/// Returns the destination path on success.
pub fn process_inbox_file(src: &Path, notes_dir: &Path, subdir: Option<&str>) -> Result<PathBuf> {
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stem = src
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (body, title) = match ext {
        "md" | "markdown" => {
            let raw = std::fs::read_to_string(src)?;
            let (fm, body) = frontmatter::parse(&raw);
            let title = fm.title.clone().unwrap_or_else(|| stem.clone());
            (body, title)
        }
        "txt" => {
            let content = std::fs::read_to_string(src)?;
            (content, stem.clone())
        }
        _ => anyhow::bail!("Unsupported file type: {ext}"),
    };

    let dest_dir = if let Some(sub) = subdir {
        notes_dir.join(sub)
    } else {
        notes_dir.to_path_buf()
    };
    std::fs::create_dir_all(&dest_dir)?;

    // Avoid name collisions
    let filename = sanitize_filename(&stem);
    let dest = unique_path(&dest_dir, &filename, "md");

    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

    let fm_str = format!(
        "---\ntitle: \"{title}\"\nid: \"{id}\"\ncreated: \"{now}\"\nmodified: \"{now}\"\n---\n\n"
    );
    std::fs::write(&dest, format!("{fm_str}{body}"))?;

    // Remove the source file from inbox
    std::fs::remove_file(src)?;

    Ok(dest)
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .replace(' ', "-")
        .to_lowercase()
}

pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let base = dir.join(format!("{stem}.{ext}"));
    if !base.exists() {
        return base;
    }
    let ts = Utc::now().format("%Y%m%d%H%M%S");
    dir.join(format!("{stem}-{ts}.{ext}"))
}
