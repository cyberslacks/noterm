use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationStatus {
    Pending,
    Incorporated,
    Ignored,
    Stale,
}

impl Default for AnnotationStatus {
    fn default() -> Self {
        AnnotationStatus::Pending
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationSource {
    Cli,
    Agent,
}

impl Default for AnnotationSource {
    fn default() -> Self {
        AnnotationSource::Cli
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub section: String,
    pub added: String,
    #[serde(default)]
    pub status: AnnotationStatus,
    #[serde(default)]
    pub source: AnnotationSource,
}

/// Convert a note path to a slug: stem, lowercase, spaces → dashes.
pub fn note_slug(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
        .replace(' ', "-")
}

/// Kazam-compatible `.kazam/annotations/<slug>/` directory relative to the
/// notes directory. Existing `.annotations/` files remain readable as a
/// migration fallback in `load_annotations`.
pub fn annotations_dir(notes_dir: &Path, slug: &str) -> PathBuf {
    notes_dir.join(".kazam").join("annotations").join(slug)
}

fn legacy_annotations_dir(notes_dir: &Path, slug: &str) -> PathBuf {
    notes_dir.join(".annotations").join(slug)
}

/// Load all annotation YAML files for a given slug, sorted pending-first.
pub fn load_annotations(notes_dir: &Path, slug: &str) -> Vec<Annotation> {
    let dir = annotations_dir(notes_dir, slug);
    let dir = if dir.exists() {
        dir
    } else {
        legacy_annotations_dir(notes_dir, slug)
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut annotations: Vec<Annotation> = entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("yaml"))
        .filter_map(|e| {
            let content = std::fs::read_to_string(e.path()).ok()?;
            serde_yaml::from_str::<Annotation>(&content).ok()
        })
        .collect();

    annotations.sort_by(|a, b| {
        let rank = |s: &AnnotationStatus| match s {
            AnnotationStatus::Pending => 0,
            AnnotationStatus::Stale => 1,
            AnnotationStatus::Incorporated => 2,
            AnnotationStatus::Ignored => 3,
        };
        rank(&a.status)
            .cmp(&rank(&b.status))
            .then(b.added.cmp(&a.added))
    });
    annotations
}

/// Persist a single annotation to its YAML file.
pub fn save_annotation(notes_dir: &Path, slug: &str, ann: &Annotation) -> anyhow::Result<()> {
    let dir = annotations_dir(notes_dir, slug);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.yaml", ann.id));
    std::fs::write(path, serde_yaml::to_string(ann)?)?;
    Ok(())
}

/// Remove an annotation by ID (no-op if already gone).
pub fn delete_annotation(notes_dir: &Path, slug: &str, id: &str) -> anyhow::Result<()> {
    let path = annotations_dir(notes_dir, slug).join(format!("{id}.yaml"));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Mark any `Pending` annotations as `Stale` when `note_modified` is newer than `added`.
pub fn mark_stale_annotations(notes_dir: &Path, slug: &str, note_modified: &str) {
    let date_str = &note_modified[..10.min(note_modified.len())];
    let modified_days = crate::notes::freshness::parse_iso_date(date_str);
    let dir = annotations_dir(notes_dir, slug);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut ann) = serde_yaml::from_str::<Annotation>(&content) else {
            continue;
        };
        if ann.status != AnnotationStatus::Pending {
            continue;
        }
        let added_days = crate::notes::freshness::parse_iso_date(&ann.added);
        if let (Some(m), Some(a)) = (modified_days, added_days) {
            if m > a {
                ann.status = AnnotationStatus::Stale;
                if let Ok(new_content) = serde_yaml::to_string(&ann) {
                    std::fs::write(&path, new_content).ok();
                }
            }
        }
    }
}

/// Count pending annotations; used to populate the cached count in `AppState`.
pub fn count_pending(notes_dir: &Path, slug: &str) -> usize {
    load_annotations(notes_dir, slug)
        .iter()
        .filter(|a| a.status == AnnotationStatus::Pending)
        .count()
}

/// Kazam's annotation review status: pending annotations become due soon in
/// seven days and overdue after fourteen. Resolved annotations are fresh.
pub fn annotation_freshness(
    annotation: &Annotation,
    today: &str,
) -> crate::notes::freshness::FreshnessStatus {
    use crate::notes::freshness::{parse_iso_date, FreshnessStatus};
    if matches!(
        annotation.status,
        AnnotationStatus::Incorporated | AnnotationStatus::Ignored
    ) {
        return FreshnessStatus::Fresh;
    }
    let Some(added) = parse_iso_date(&annotation.added) else {
        return FreshnessStatus::Fresh;
    };
    let Some(today) = parse_iso_date(today) else {
        return FreshnessStatus::Fresh;
    };
    let days_until_due = 14 - (today - added);
    if days_until_due < 0 {
        FreshnessStatus::Overdue {
            days_overdue: -days_until_due,
        }
    } else if days_until_due <= 7 {
        FreshnessStatus::DueSoon { days_until_due }
    } else {
        FreshnessStatus::Fresh
    }
}

/// Generate a new annotation ID: `ann-YYYY-MM-DD-XXXX`.
pub fn new_annotation_id() -> String {
    let today = crate::notes::freshness::today_iso();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    format!("ann-{today}-{:04x}", hasher.finish() & 0xffff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_path() {
        assert_eq!(
            note_slug(&Path::new("/notes/My Important Note.md")),
            "my-important-note"
        );
        assert_eq!(note_slug(&Path::new("plain.md")), "plain");
    }

    #[test]
    fn annotation_id_format() {
        let id = new_annotation_id();
        assert!(id.starts_with("ann-"), "id should start with 'ann-': {id}");
        assert!(id.len() >= 18, "id too short: {id}");
    }

    #[test]
    fn pending_annotations_follow_kazam_decay_window() {
        let annotation = Annotation {
            id: "ann-test".into(),
            text: "Review this".into(),
            author: String::new(),
            section: String::new(),
            added: "2026-01-01".into(),
            status: AnnotationStatus::Pending,
            source: AnnotationSource::Cli,
        };
        assert!(matches!(
            annotation_freshness(&annotation, "2026-01-09"),
            crate::notes::freshness::FreshnessStatus::DueSoon { .. }
        ));
        assert!(matches!(
            annotation_freshness(&annotation, "2026-01-16"),
            crate::notes::freshness::FreshnessStatus::Overdue { .. }
        ));
    }
}
