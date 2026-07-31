use anyhow::Result;
use crossterm::event::EventStream;
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

use noterm::{
    app::{self, AppEvent, AppState, Mode},
    config::{self, Config},
    db, import, llm, notes, search, tui,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Handle --version / -V before TUI starts (safe to write to stdout here)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("noterm {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Load config first (creates default if missing)
    let config = Config::load()?;

    // Ensure the standard portable Markdown-vault layout exists.
    config.ensure_vault_layout()?;

    // Open SQLite database
    let db_path = config.db_path();
    let db = db::open(&db_path)?;

    // Set up async event channel
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Create app state
    let mut app = AppState::new(config, tx.clone());

    // Initial file tree scan
    {
        let notes_dir = app.notes_dir.clone();
        let show_hidden = app.config.ui.show_hidden;
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let nodes = tokio::task::spawn_blocking(move || {
                notes::watcher::scan_dir(&notes_dir, show_hidden)
            })
            .await
            .unwrap_or_default();
            tx2.send(AppEvent::FileTreeRefresh(nodes)).ok();
        });
    }

    // Background index all existing notes on startup
    {
        let notes_dir = app.notes_dir.clone();
        let index_dir = app.config.index_dir();
        let show_hidden = app.config.ui.show_hidden;
        let tx2 = tx.clone();
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                let idx = search::fulltext::FtsIndex::open_or_create(&index_dir)?;
                let nodes = notes::watcher::scan_dir(&notes_dir, show_hidden);
                for node in nodes.iter().filter(|n| !n.is_dir) {
                    if let Ok(note) = notes::Note::from_path(&node.path, &notes_dir) {
                        let title = note.frontmatter.title.clone().unwrap_or_default();
                        let tags = note.frontmatter.tags.clone().unwrap_or_default();
                        idx.index_note(&note.relative_path, &title, &note.body, &tags)
                            .ok();
                    }
                }
                anyhow::Ok(())
            })
            .await
            .ok();
            tx2.send(AppEvent::IndexingComplete).ok();
        });
    }

    // Background-embed all existing notes that are missing or stale (runs serially)
    if app.config.search.embed_on_save {
        let notes_dir = app.notes_dir.clone();
        let show_hidden = app.config.ui.show_hidden;
        let llm_config = app.config.llm.clone();
        let db2 = db.clone();
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let notes_dir_scan = notes_dir.clone();
            let nodes = tokio::task::spawn_blocking(move || {
                notes::watcher::scan_dir(&notes_dir_scan, show_hidden)
            })
            .await
            .unwrap_or_default();

            for node in nodes.into_iter().filter(|n| !n.is_dir) {
                let nd = notes_dir.clone();
                let path = node.path.clone();
                if let Ok(Ok(note)) =
                    tokio::task::spawn_blocking(move || notes::Note::from_path(&path, &nd)).await
                {
                    let note_id = note
                        .frontmatter
                        .id
                        .clone()
                        .unwrap_or_else(|| note.relative_path.clone());
                    let content = format!(
                        "{}\n\n{}",
                        note.frontmatter.title.clone().unwrap_or_default(),
                        note.body
                    );
                    run_embed_note(
                        note_id,
                        note.relative_path,
                        note.content_hash,
                        content,
                        llm_config.clone(),
                        db2.clone(),
                        tx2.clone(),
                    )
                    .await;
                }
            }
        });
    }

    // Start inbox folder watcher (runs forever in background)
    {
        let inbox_dir = app.config.import.resolved_watch_dir();
        let notes_dir = app.notes_dir.clone();
        let interval = app.config.import.watch_interval_secs;
        let show_hidden = app.config.ui.show_hidden;
        let tx2 = tx.clone();
        tokio::spawn(import::watcher::run_inbox_watcher(
            inbox_dir,
            notes_dir,
            interval,
            show_hidden,
            tx2,
        ));
    }

    // Check for a newer release in the background
    {
        let tx2 = tx.clone();
        tokio::spawn(async move {
            check_for_update(tx2).await;
        });
    }

    // Start local REST API if enabled in config
    if app.config.import.api_enabled {
        let api_state = import::api::ApiState {
            notes_dir: app.notes_dir.clone(),
            show_hidden: app.config.ui.show_hidden,
            tx: tx.clone(),
        };
        let host = app.config.import.api_host.clone();
        let port = app.config.import.api_port;
        tokio::spawn(async move {
            if let Err(e) = import::api::start(&host, port, api_state).await {
                eprintln!("Import API error: {e}");
            }
        });
    }

    // Initialize TUI
    let mut terminal = tui::init()?;

    // Event stream from crossterm
    let mut event_stream = EventStream::new();

    // 100ms tick for animations / periodic tasks
    let mut tick = time::interval(Duration::from_millis(100));

    // Search debounce tracking
    let mut last_search_query = String::new();
    let mut search_debounce: Option<time::Instant> = None;

    // Track whether we've already spawned a chat task for current loading state
    let mut chat_task_spawned = false;

    // Track whether we've already spawned a summarize task
    let mut summarize_task_spawned = false;

    loop {
        terminal.draw(|f| tui::renderer::render(f, &mut app))?;

        tokio::select! {
            // Keyboard / mouse events from crossterm
            Some(Ok(event)) = event_stream.next() => {
                match tui::keys::handle_event(&mut app, event).await? {
                    tui::keys::Action::Quit => break,
                    tui::keys::Action::Continue => {}
                }

                // Trigger search when query changes in Search mode
                if app.mode == Mode::Search && app.search_query != last_search_query {
                    last_search_query = app.search_query.clone();
                    search_debounce = Some(time::Instant::now());
                }

                // Trigger LLM chat send when loading flag set
                if app.chat_loading && !chat_task_spawned {
                    if let Some(last) = app.chat_messages.last() {
                        if last.role == llm::ChatRole::User {
                            chat_task_spawned = true;
                            let messages = app.chat_messages.clone();
                            let config = app.config.llm.clone();
                            let current_note = app.current_note.clone();
                            let kazam_pages = if app.chat_kazam_context {
                                app.kazam_kb_pages.clone()
                            } else {
                                Vec::new()
                            };
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                send_chat(messages, config, current_note, kazam_pages, tx2).await;
                            });
                        }
                    }
                }
                if !app.chat_loading {
                    chat_task_spawned = false;
                }

                // Trigger summarizer when loading flag is set
                if app.summarize_loading && !summarize_task_spawned {
                    if let Some(note) = &app.current_note {
                        summarize_task_spawned = true;
                        let body = note.body.clone();
                        let summarizer_config = app.config.summarizer.clone();
                        let tx2 = tx.clone();
                        tokio::spawn(async move {
                            run_summarize(body, summarizer_config, tx2).await;
                        });
                    }
                }
                if !app.summarize_loading {
                    summarize_task_spawned = false;
                }

                // Trigger vector search
                if app.mode == Mode::VectorSearch && app.vsearch_loading {
                    let query = app.vsearch_query.clone();
                    let config = app.config.llm.clone();
                    let db2 = db.clone();
                    let tx2 = tx.clone();
                    app.vsearch_loading = false; // prevent re-trigger
                    tokio::spawn(async move {
                        run_vector_search(query, config, db2, tx2).await;
                    });
                }
            }

            // Events from background tasks (LLM chunks, git results, etc.)
            Some(event) = rx.recv() => {
                if matches!(event, AppEvent::ChatDone | AppEvent::ChatError(_)) {
                    chat_task_spawned = false;
                }
                // EmbedRequest and ForceReembed need db — handle before dispatching to AppState
                if let AppEvent::EmbedRequest { note_id, note_path, content_hash, content } = event {
                    let llm_config = app.config.llm.clone();
                    let db2 = db.clone();
                    let tx2 = tx.clone();
                    tokio::spawn(async move {
                        run_embed_note(note_id, note_path, content_hash, content, llm_config, db2, tx2).await;
                    });
                } else if let AppEvent::ForceReembed = event {
                    // Clear all embeddings then re-index every note with the current provider
                    let db2 = db.clone();
                    let notes_dir = app.notes_dir.clone();
                    let show_hidden = app.config.ui.show_hidden;
                    let llm_config = app.config.llm.clone();
                    let tx2 = tx.clone();
                    app.set_status("Re-embedding all notes…".into(), app::StatusLevel::Info);
                    tokio::spawn(async move {
                        // Clear cache
                        if let Err(e) = tokio::task::spawn_blocking({
                            let db3 = db2.clone();
                            move || search::vector::clear_all_embeddings(&db3)
                        }).await.unwrap_or_else(|e| Err(anyhow::anyhow!("{e}"))) {
                            tx2.send(AppEvent::Error(format!("Clear embeddings: {e}"))).ok();
                            return;
                        }

                        // Re-scan and re-embed every note
                        let nd = notes_dir.clone();
                        let nodes = tokio::task::spawn_blocking(move || {
                            notes::watcher::scan_dir(&nd, show_hidden)
                        }).await.unwrap_or_default();

                        for node in nodes.into_iter().filter(|n| !n.is_dir) {
                            let nd2 = notes_dir.clone();
                            let path = node.path.clone();
                            if let Ok(Ok(note)) = tokio::task::spawn_blocking(move || {
                                notes::Note::from_path(&path, &nd2)
                            }).await {
                                let note_id = note.frontmatter.id.clone()
                                    .unwrap_or_else(|| note.relative_path.clone());
                                let content = format!(
                                    "{}\n\n{}",
                                    note.frontmatter.title.clone().unwrap_or_default(),
                                    note.body
                                );
                                run_embed_note(
                                    note_id,
                                    note.relative_path,
                                    note.content_hash,
                                    content,
                                    llm_config.clone(),
                                    db2.clone(),
                                    tx2.clone(),
                                ).await;
                            }
                        }
                        tx2.send(AppEvent::IndexingComplete).ok();
                    });
                } else {
                    app.handle_app_event(event);
                }
            }

            // Periodic tick
            _ = tick.tick() => {
                // Fire debounced search
                if let Some(t) = search_debounce {
                    if t.elapsed() >= Duration::from_millis(300) {
                        search_debounce = None;
                        let query = app.search_query.clone();
                        if !query.is_empty() {
                            let index_dir = app.config.index_dir();
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                run_fts_search(query, index_dir, tx2).await;
                            });
                        }
                    }
                }
            }
        }
    }

    tui::restore()?;
    Ok(())
}

async fn run_embed_note(
    note_id: String,
    note_path: String,
    content_hash: String,
    content: String,
    llm_config: config::LlmConfig,
    db: db::Db,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let model = llm::embed_model_name(&llm_config);

    // Skip if embedding is already current
    let skip = tokio::task::spawn_blocking({
        let db2 = db.clone();
        let id = note_id.clone();
        let hash = content_hash.clone();
        let m = model.clone();
        move || !search::vector::needs_embedding(&db2, &id, &hash, &m)
    })
    .await
    .unwrap_or(false);

    if skip {
        return;
    }

    let client = llm::make_embed_client(&llm_config);
    match client.embed(&content).await {
        Ok(embedding) => {
            let note_path_for_event = note_path.clone();
            let result = tokio::task::spawn_blocking(move || {
                search::vector::store_embedding(
                    &db,
                    &note_id,
                    &note_path,
                    &content_hash,
                    &embedding,
                    &model,
                )
            })
            .await;
            match result {
                Ok(Ok(())) => {
                    tx.send(AppEvent::EmbeddingDone(note_path_for_event.into()))
                        .ok();
                }
                Ok(Err(e)) => {
                    tx.send(AppEvent::Error(format!("Embed store: {e}"))).ok();
                }
                Err(e) => {
                    tx.send(AppEvent::Error(format!("Embed task: {e}"))).ok();
                }
            }
        }
        Err(e) => {
            tx.send(AppEvent::Error(format!("Embed: {e}"))).ok();
        }
    }
}

async fn run_fts_search(
    query: String,
    index_dir: std::path::PathBuf,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let result = tokio::task::spawn_blocking(move || {
        let index = search::fulltext::FtsIndex::open_or_create(&index_dir)?;
        index.search(&query, 20)
    })
    .await;

    match result {
        Ok(Ok(results)) => {
            tx.send(AppEvent::SearchResults(results)).ok();
        }
        Ok(Err(e)) => {
            tx.send(AppEvent::Error(format!("Search error: {e}"))).ok();
        }
        Err(e) => {
            tx.send(AppEvent::Error(format!("Search task error: {e}")))
                .ok();
        }
    }
}

async fn run_vector_search(
    query: String,
    llm_config: config::LlmConfig,
    db: db::Db,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let client = llm::make_embed_client(&llm_config);
    match client.embed(&query).await {
        Ok(embedding) => {
            let model = llm::embed_model_name(&llm_config);
            let result = tokio::task::spawn_blocking(move || {
                search::vector::top_k_similar(&db, &embedding, &model, 10)
            })
            .await;

            match result {
                Ok(Ok(results)) => {
                    tx.send(AppEvent::VectorSearchResults(results)).ok();
                }
                Ok(Err(e)) => {
                    tx.send(AppEvent::Error(format!("Vector search: {e}"))).ok();
                }
                Err(e) => {
                    tx.send(AppEvent::Error(format!("Vector task: {e}"))).ok();
                }
            }
        }
        Err(e) => {
            tx.send(AppEvent::Error(format!("Embed error: {e}"))).ok();
            tx.send(AppEvent::VectorSearchResults(vec![])).ok();
        }
    }
}

async fn run_summarize(
    transcript: String,
    config: config::SummarizerConfig,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let client = llm::summarizer::SummarizerClient::new(&config);
    match client.summarize_stream(&transcript).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(token) => {
                        tx.send(AppEvent::SummaryChunk(token)).ok();
                    }
                    Err(e) => {
                        tx.send(AppEvent::SummaryError(e.to_string())).ok();
                        return;
                    }
                }
            }
            tx.send(AppEvent::SummaryDone).ok();
        }
        Err(e) => {
            tx.send(AppEvent::SummaryError(e.to_string())).ok();
        }
    }
}

async fn check_for_update(tx: mpsc::UnboundedSender<AppEvent>) {
    const CURRENT: &str = env!("CARGO_PKG_VERSION");
    const API_URL: &str = "https://api.github.com/repos/cyberslacks/Noterm/releases/latest";

    let client = match reqwest::Client::builder()
        .user_agent(format!("noterm/{CURRENT}"))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let resp = match client.get(API_URL).send().await {
        Ok(r) => r,
        Err(_) => return,
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return,
    };

    let tag = match json.get("tag_name").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return,
    };

    // Strip leading 'v' for comparison
    let latest = tag.trim_start_matches('v');
    if latest != CURRENT {
        tx.send(AppEvent::UpdateAvailable(tag)).ok();
    }
}

async fn send_chat(
    messages: Vec<llm::ChatMessage>,
    llm_config: config::LlmConfig,
    current_note: Option<notes::Note>,
    kazam_pages: Vec<String>,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let context_notes: Vec<notes::Note> = current_note.into_iter().collect();
    let system = llm::context::build_system_prompt(&llm_config, &context_notes, &kazam_pages);
    let client = llm::make_client(&llm_config);

    match client.chat_stream(messages, &system).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(token) => {
                        tx.send(AppEvent::ChatChunk(token)).ok();
                    }
                    Err(e) => {
                        tx.send(AppEvent::ChatError(e.to_string())).ok();
                        return;
                    }
                }
            }
            tx.send(AppEvent::ChatDone).ok();
        }
        Err(e) => {
            tx.send(AppEvent::ChatError(e.to_string())).ok();
        }
    }
}
