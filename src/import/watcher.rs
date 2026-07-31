use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time;

use crate::app::AppEvent;
use crate::notes::watcher::scan_dir;

/// Poll the inbox directory on the given interval. For each .md or .txt file
/// found, process it into the notes_dir and fire events so the TUI updates.
pub async fn run_inbox_watcher(
    inbox_dir: PathBuf,
    notes_dir: PathBuf,
    interval_secs: u64,
    show_hidden: bool,
    tx: UnboundedSender<AppEvent>,
) {
    // Ensure inbox dir exists
    if let Err(e) = std::fs::create_dir_all(&inbox_dir) {
        tx.send(AppEvent::Error(format!("Cannot create inbox dir: {e}")))
            .ok();
        return;
    }

    let mut ticker = time::interval(Duration::from_secs(interval_secs));
    ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        sweep_inbox(&inbox_dir, &notes_dir, show_hidden, &tx).await;
    }
}

async fn sweep_inbox(
    inbox: &std::path::Path,
    notes_dir: &std::path::Path,
    show_hidden: bool,
    tx: &UnboundedSender<AppEvent>,
) {
    let inbox = inbox.to_path_buf();
    let notes_dir = notes_dir.to_path_buf();
    let tx2 = tx.clone();

    tokio::task::spawn_blocking(move || {
        let Ok(entries) = std::fs::read_dir(&inbox) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "md" | "markdown" | "txt") {
                continue;
            }

            match super::process_inbox_file(&path, &notes_dir, Some("notes/inbox")) {
                Ok(dest) => {
                    tx2.send(AppEvent::NoteImported(dest.clone())).ok();
                    // Refresh file tree
                    let nodes = scan_dir(&notes_dir, show_hidden);
                    tx2.send(AppEvent::FileTreeRefresh(nodes)).ok();
                }
                Err(e) => {
                    tx2.send(AppEvent::Error(format!(
                        "Import failed for {}: {e}",
                        path.display()
                    )))
                    .ok();
                }
            }
        }
    })
    .await
    .ok();
}
