/// Kazam KB browser: reads Kazam YAML page files directly (no subprocess required).
///
/// Kazam page format (relevant fields):
///   title: string
///   shell: standard | document | deck
///   owner: string
///   freshness:
///     review_every: Nd/Nw/Nm/Ny
///     expires: YYYY-MM-DD
///   components:
///     - type: markdown
///       body: |
///         …content…
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::notes::freshness::{self, FreshnessStatus};

// Minimal deserialization of a Kazam page YAML file.
#[derive(Debug, Deserialize, Default)]
struct KazamPageYaml {
    #[serde(default)]
    title: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    freshness: Option<KazamFreshnessYaml>,
    #[serde(default)]
    components: Vec<KazamComponent>,
}

#[derive(Debug, Deserialize, Default)]
struct KazamFreshnessYaml {
    #[serde(default)]
    review_every: Option<String>,
    #[serde(default)]
    expires: Option<String>,
    #[serde(default)]
    updated: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KazamComponent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    body: String,
}

/// A Kazam KB page ready to display in the browser panel.
#[derive(Debug, Clone)]
pub struct KazamPage {
    pub path: PathBuf,
    pub slug: String,
    pub title: String,
    pub owner: Option<String>,
    pub review_every: Option<String>,
    pub freshness_status: Option<FreshnessStatus>,
    pub already_imported: bool,
}

/// Scan `kb_path` for Kazam YAML pages. Checks `notes_dir/<import_folder>/<slug>.md`
/// to determine whether each page has already been imported.
pub fn scan_kb(kb_path: &Path, notes_dir: &Path, import_folder: &str) -> Vec<KazamPage> {
    let Ok(entries) = std::fs::read_dir(kb_path) else {
        return Vec::new();
    };

    let mut pages = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        // Skip the kazam manifest itself
        if stem == "kazam" {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(page_yaml) = serde_yaml::from_str::<KazamPageYaml>(&content) else {
            continue;
        };

        let slug = crate::import::sanitize_filename(&stem);
        let title = if page_yaml.title.is_empty() {
            slug.clone()
        } else {
            page_yaml.title.clone()
        };

        let review_every = page_yaml
            .freshness
            .as_ref()
            .and_then(|f| f.review_every.clone());
        let freshness_status = page_yaml.freshness.as_ref().and_then(|f| {
            let info = freshness::compute(
                f.updated.as_deref(),
                f.review_every.as_deref(),
                f.expires.as_deref(),
            )?;
            Some(info.status())
        });

        let import_path = notes_dir.join(import_folder).join(format!("{slug}.md"));
        let already_imported = import_path.exists();

        pages.push(KazamPage {
            path,
            slug,
            title,
            owner: page_yaml.owner,
            review_every,
            freshness_status,
            already_imported,
        });
    }

    // Sort: not-yet-imported first, then alphabetically by title
    pages.sort_by(|a, b| {
        a.already_imported
            .cmp(&b.already_imported)
            .then(a.title.cmp(&b.title))
    });

    pages
}

/// Import a Kazam page into noterm as a markdown note.
/// Concatenates all `type: markdown` component bodies into the note body.
/// Returns the path of the written note.
pub fn import_page(
    page: &KazamPage,
    notes_dir: &Path,
    import_folder: &str,
) -> anyhow::Result<PathBuf> {
    let content = std::fs::read_to_string(&page.path)?;
    let page_yaml: KazamPageYaml = serde_yaml::from_str(&content)?;

    let body: String = page_yaml
        .components
        .iter()
        .filter(|c| c.kind == "markdown")
        .map(|c| c.body.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let dest_dir = notes_dir.join(import_folder);
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(format!("{}.md", page.slug));

    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

    let mut fm_lines = vec![
        "---".to_string(),
        format!("title: \"{}\"", page.title),
        format!("id: \"{id}\""),
        format!("created: \"{now}\""),
        format!("modified: \"{now}\""),
        "tags: [\"kazam\"]".to_string(),
    ];
    if let Some(owner) = &page.owner {
        fm_lines.push(format!("owner: \"{owner}\""));
    }
    if let Some(fr) = &page.review_every {
        fm_lines.push(format!("review_every: \"{fr}\""));
    }
    if let Some(f) = &page_yaml.freshness {
        if let Some(exp) = &f.expires {
            fm_lines.push(format!("expires: \"{exp}\""));
        }
    }
    fm_lines.push("---".to_string());
    fm_lines.push(String::new());

    let full_content = format!("{}\n{body}", fm_lines.join("\n"));
    std::fs::write(&dest, full_content)?;

    Ok(dest)
}

/// Return the import path for an already-imported page (does not check existence).
pub fn import_path(notes_dir: &Path, import_folder: &str, slug: &str) -> PathBuf {
    notes_dir.join(import_folder).join(format!("{slug}.md"))
}
