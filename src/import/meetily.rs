/// Meetily meeting importer.
///
/// Reads directly from Meetily's SQLite database (meeting_minutes.db).
/// Formats each meeting as a structured call note with:
///   ## Summary / Key Points / Action Items / Full Transcript
///
/// DB location auto-detection order:
///   1. config.meetily.db_path  (user override)
///   2. /opt/homebrew/var/meetily/meeting_minutes.db  (macOS Homebrew)
///   3. ~/Library/Application Support/meetily/meeting_minutes.db  (macOS app)
///   4. ~/.local/share/meetily/meeting_minutes.db  (Linux XDG)
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;

use crate::config::MeetilyConfig;

#[derive(Debug, Clone)]
pub struct MeetilyMeeting {
    pub id: String,
    pub title: String,
    pub created_at: String,
    /// Combined transcript text (joined from transcripts table)
    pub transcript: Option<String>,
    /// AI-generated summary
    pub summary: Option<String>,
    /// Action items as raw text (may be JSON or newline-delimited)
    pub action_items: Option<String>,
    /// Key points as raw text
    pub key_points: Option<String>,
    /// Total call duration in seconds
    pub duration_secs: Option<f64>,
    /// User-written notes taken during the meeting (markdown)
    pub user_notes: Option<String>,
}

impl MeetilyMeeting {
    /// Human-readable duration string e.g. "1h 23m" or "45m"
    pub fn duration_display(&self) -> String {
        let secs = self.duration_secs.unwrap_or(0.0) as u64;
        if secs == 0 {
            return String::new();
        }
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        match (h, m) {
            (0, m) => format!("{m}m"),
            (h, 0) => format!("{h}h"),
            (h, m) => format!("{h}h {m}m"),
        }
    }

    /// Format the meeting's created_at timestamp for display ("May 6, 2026 14:30")
    pub fn date_display(&self) -> String {
        DateTime::parse_from_rfc3339(&self.created_at)
            .or_else(|_| DateTime::parse_from_str(&self.created_at, "%Y-%m-%d %H:%M:%S%.f"))
            .map(|dt| dt.format("%b %-d, %Y  %H:%M").to_string())
            .unwrap_or_else(|_| self.created_at.clone())
    }
}

/// Find the Meetily DB. Returns `None` if no DB can be located.
pub fn find_db(config: &MeetilyConfig) -> Option<PathBuf> {
    if let Some(ref p) = config.db_path {
        if p.exists() {
            return Some(p.clone());
        }
    }

    let candidates: Vec<PathBuf> = vec![
        // Linux — Tauri app (com.meetily.ai)
        dirs::data_local_dir()
            .map(|d| d.join("com.meetily.ai/meeting_minutes.sqlite"))
            .unwrap_or_default(),
        // macOS — Tauri app (com.meetily.ai)
        dirs::home_dir()
            .map(|h| h.join("Library/Application Support/com.meetily.ai/meeting_minutes.sqlite"))
            .unwrap_or_default(),
        // macOS Homebrew legacy
        PathBuf::from("/opt/homebrew/var/meetily/meeting_minutes.db"),
        PathBuf::from("/usr/local/var/meetily/meeting_minutes.db"),
        // macOS app support legacy
        dirs::home_dir()
            .map(|h| h.join("Library/Application Support/meetily/meeting_minutes.db"))
            .unwrap_or_default(),
        // Linux XDG legacy
        dirs::data_local_dir()
            .map(|d| d.join("meetily/meeting_minutes.db"))
            .unwrap_or_default(),
        // Linux fallback legacy
        dirs::home_dir()
            .map(|h| h.join(".meetily/meeting_minutes.db"))
            .unwrap_or_default(),
    ];

    candidates.into_iter().find(|p| p.exists() && p.is_file())
}

/// Load all meetings from the Meetily DB, newest first.
pub fn load_meetings(db_path: &std::path::Path) -> Result<Vec<MeetilyMeeting>> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("opening Meetily DB at {}", db_path.display()))?;

    // Check which tables exist
    let has_transcripts: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='transcripts'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    let has_summary_processes: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='summary_processes'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    let has_meeting_notes: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='meeting_notes'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    // Check if transcripts table has a speaker column (added in later Meetily versions)
    let has_speaker: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('transcripts') WHERE name='speaker'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    let mut stmt =
        conn.prepare("SELECT id, title, created_at FROM meetings ORDER BY created_at DESC")?;

    let meetings: Vec<MeetilyMeeting> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .flatten()
        .map(|(id, title, created_at)| {
            let (transcript, summary, action_items, key_points, duration, user_notes) =
                fetch_meeting_content(
                    &conn,
                    &id,
                    has_transcripts,
                    has_summary_processes,
                    has_meeting_notes,
                    has_speaker,
                )
                .unwrap_or_default();

            MeetilyMeeting {
                id,
                title,
                created_at,
                transcript,
                summary,
                action_items,
                key_points,
                duration_secs: duration,
                user_notes,
            }
        })
        .collect();

    Ok(meetings)
}

fn fetch_meeting_content(
    conn: &Connection,
    meeting_id: &str,
    has_transcripts: bool,
    has_summary_processes: bool,
    has_meeting_notes: bool,
    has_speaker: bool,
) -> Result<(
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<f64>,
    Option<String>,
)> {
    let mut transcript_text: Option<String> = None;
    let mut summary: Option<String> = None;
    let mut action_items: Option<String> = None;
    let mut key_points: Option<String> = None;
    let mut total_duration: Option<f64> = None;
    let mut user_notes: Option<String> = None;

    if has_transcripts {
        let sql = if has_speaker {
            "SELECT transcript, summary, action_items, key_points, audio_start_time, audio_end_time, speaker \
             FROM transcripts WHERE meeting_id = ?1 ORDER BY audio_start_time ASC"
        } else {
            "SELECT transcript, summary, action_items, key_points, audio_start_time, audio_end_time, NULL \
             FROM transcripts WHERE meeting_id = ?1 ORDER BY audio_start_time ASC"
        };

        let mut stmt = conn.prepare(sql)?;
        let segments: Vec<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<f64>,
            Option<f64>,
            Option<String>,
        )> = stmt
            .query_map([meeting_id], |r| {
                Ok((
                    r.get::<_, String>(0).unwrap_or_default(),
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<f64>>(4)?,
                    r.get::<_, Option<f64>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })?
            .flatten()
            .collect();

        if !segments.is_empty() {
            let mut lines: Vec<String> = Vec::new();
            for (text, _, _, _, start, _, speaker) in &segments {
                let ts_part = start
                    .map(|s| format!("[{}] ", format_timestamp(s)))
                    .unwrap_or_default();
                let spk_part = match speaker.as_deref() {
                    Some("mic") => "**Mic** ",
                    Some("system") => "**System** ",
                    _ => "",
                };
                lines.push(format!("**{ts_part}**{spk_part}{text}"));
            }
            transcript_text = Some(lines.join("\n\n"));

            for (_, seg_summary, seg_actions, seg_keys, _, end, _) in &segments {
                if seg_summary.is_some() {
                    summary = seg_summary.clone();
                }
                if seg_actions.is_some() {
                    action_items = seg_actions.clone();
                }
                if seg_keys.is_some() {
                    key_points = seg_keys.clone();
                }
                if let Some(e) = end {
                    total_duration = Some(*e);
                }
            }
        }
    }

    // Check summary_processes for a richer AI-generated summary
    if has_summary_processes {
        if let Ok(result_json) = conn.query_row(
            "SELECT result FROM summary_processes WHERE meeting_id = ?1 AND status = 'completed'",
            [meeting_id],
            |r| r.get::<_, String>(0),
        ) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&result_json) {
                if summary.is_none() {
                    summary = v["summary"].as_str().map(String::from);
                }
                if action_items.is_none() {
                    action_items = v["action_items"]
                        .as_str()
                        .map(String::from)
                        .or_else(|| json_array_to_lines(&v["action_items"]));
                }
                if key_points.is_none() {
                    key_points = v["key_points"]
                        .as_str()
                        .map(String::from)
                        .or_else(|| json_array_to_lines(&v["key_points"]));
                }
            }
        }
    }

    // Pull user-written notes from meeting_notes table
    if has_meeting_notes {
        if let Ok(notes) = conn.query_row(
            "SELECT notes_markdown FROM meeting_notes WHERE meeting_id = ?1",
            [meeting_id],
            |r| r.get::<_, Option<String>>(0),
        ) {
            user_notes = notes.filter(|s| !s.trim().is_empty());
        }
    }

    Ok((
        transcript_text,
        summary,
        action_items,
        key_points,
        total_duration,
        user_notes,
    ))
}

/// Convert a JSON array of strings to newline-separated bullets
fn json_array_to_lines(v: &serde_json::Value) -> Option<String> {
    v.as_array().map(|arr| {
        arr.iter()
            .filter_map(|item| item.as_str())
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn format_timestamp(secs: f64) -> String {
    let s = secs as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{h:02}:{m:02}:{sec:02}")
    } else {
        format!("{m:02}:{sec:02}")
    }
}

/// Render a MeetilyMeeting into a Markdown call note string.
pub fn render_note(meeting: &MeetilyMeeting, tags: &[String]) -> String {
    let title = &meeting.title;
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let created = &meeting.created_at;
    let date_display = meeting.date_display();
    let duration = meeting.duration_display();

    // Build YAML frontmatter
    let tags_yaml = if tags.is_empty() {
        String::new()
    } else {
        let items = tags
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("tags: [{items}]\n")
    };

    let duration_line = if duration.is_empty() {
        String::new()
    } else {
        format!(" · {duration}")
    };

    let fm = format!(
        "---\ntitle: \"{title}\"\nid: \"{id}\"\ncreated: \"{created}\"\nmodified: \"{now}\"\n{tags_yaml}source: \"meetily\"\nmeetily_id: \"{}\"\n---\n",
        meeting.id
    );

    // Header
    let mut body =
        format!("\n# {title}\n\n> **Call Note** · {date_display}{duration_line}\n\n---\n\n");

    // Summary section
    body.push_str("## Summary\n\n");
    if let Some(ref s) = meeting.summary {
        body.push_str(s.trim());
        body.push_str("\n\n");
    } else {
        body.push_str("*No summary available.*\n\n");
    }

    // Key Points
    if let Some(ref kp) = meeting.key_points {
        let kp = kp.trim();
        if !kp.is_empty() {
            body.push_str("### Key Points\n\n");
            body.push_str(&format_bullets(kp));
            body.push_str("\n\n");
        }
    }

    // Action Items
    if let Some(ref ai) = meeting.action_items {
        let ai = ai.trim();
        if !ai.is_empty() {
            body.push_str("### Action Items\n\n");
            body.push_str(&format_action_items(ai));
            body.push_str("\n\n");
        }
    }

    // User notes taken during the meeting
    if let Some(ref notes) = meeting.user_notes {
        let notes = notes.trim();
        if !notes.is_empty() {
            body.push_str("## Notes\n\n");
            body.push_str(notes);
            body.push_str("\n\n");
        }
    }

    body.push_str("---\n\n## Transcript\n\n");
    if let Some(ref t) = meeting.transcript {
        body.push_str(t.trim());
        body.push_str("\n");
    } else {
        body.push_str("*Transcript not available.*\n");
    }

    format!("{fm}{body}")
}

/// Ensure each line is a bullet; if it already starts with `- `, leave it alone.
fn format_bullets(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let t = l.trim();
            if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("• ") {
                t.to_string()
            } else {
                format!("- {t}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Like format_bullets but uses `- [ ]` checkbox syntax for action items.
fn format_action_items(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let t = l.trim();
            if t.starts_with("- [ ]") || t.starts_with("- [x]") {
                t.to_string()
            } else if t.starts_with("- ") || t.starts_with("* ") {
                format!("- [ ] {}", &t[2..])
            } else {
                format!("- [ ] {t}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Write a meeting note to disk and return the path.
pub fn write_note(
    meeting: &MeetilyMeeting,
    notes_dir: &std::path::Path,
    folder: &str,
    tags: &[String],
) -> Result<PathBuf> {
    let dest_dir = notes_dir.join(folder);
    std::fs::create_dir_all(&dest_dir)?;

    let content = render_note(meeting, tags);
    let stem = super::sanitize_filename(&meeting.title);
    let path = super::unique_path(&dest_dir, &stem, "md");
    std::fs::write(&path, &content)?;

    Ok(path)
}
