/// Local REST API for programmatic note creation.
///
/// Endpoints:
///   GET  /api/health          → {"status":"ok","notes_dir":"..."}
///   POST /api/notes           → create note from JSON body
///   POST /api/import          → import raw text as a note
///
/// Example:
///   curl -X POST http://127.0.0.1:7373/api/notes \
///     -H 'Content-Type: application/json' \
///     -d '{"title":"My Note","content":"# Hello\n\nworld","tags":["test"]}'
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use tower_http::cors::{Any, CorsLayer};

use crate::app::AppEvent;
use crate::notes::watcher::scan_dir;

#[derive(Clone)]
pub struct ApiState {
    pub notes_dir: PathBuf,
    pub show_hidden: bool,
    pub tx: UnboundedSender<AppEvent>,
}

#[derive(Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    /// Optional subdirectory within notes_dir
    pub folder: Option<String>,
}

#[derive(Deserialize)]
pub struct ImportTextRequest {
    /// Title derived from this if absent
    pub title: Option<String>,
    pub text: String,
    pub tags: Option<Vec<String>>,
    pub folder: Option<String>,
}

#[derive(Serialize)]
pub struct NoteResponse {
    pub path: String,
    pub id: String,
    pub created: bool,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub notes_dir: String,
    pub api_version: &'static str,
}

pub async fn start(host: &str, port: u16, state: ApiState) -> anyhow::Result<()> {
    let shared = Arc::new(state);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/notes", post(create_note))
        .route("/api/import", post(import_text))
        .layer(cors)
        .with_state(shared);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    tracing::info!("Import API listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        notes_dir: state.notes_dir.to_string_lossy().to_string(),
        api_version: env!("CARGO_PKG_VERSION"),
    })
}

async fn create_note(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<CreateNoteRequest>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking({
        let state = Arc::clone(&state);
        move || {
            create_note_sync(
                &state,
                &req.title,
                req.content.as_deref().unwrap_or(""),
                req.tags.as_deref(),
                req.folder.as_deref(),
            )
        }
    })
    .await;

    match result {
        Ok(Ok(resp)) => {
            state
                .tx
                .send(AppEvent::NoteImported(PathBuf::from(&resp.path)))
                .ok();
            let nodes = scan_dir(&state.notes_dir, state.show_hidden);
            state.tx.send(AppEvent::FileTreeRefresh(nodes)).ok();
            (StatusCode::CREATED, Json(resp)).into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn import_text(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<ImportTextRequest>,
) -> impl IntoResponse {
    let title = req
        .title
        .clone()
        .unwrap_or_else(|| format!("Import {}", Utc::now().format("%Y-%m-%d %H:%M")));

    let result = tokio::task::spawn_blocking({
        let state = Arc::clone(&state);
        move || {
            create_note_sync(
                &state,
                &title,
                &req.text,
                req.tags.as_deref(),
                req.folder.as_deref(),
            )
        }
    })
    .await;

    match result {
        Ok(Ok(resp)) => {
            state
                .tx
                .send(AppEvent::NoteImported(PathBuf::from(&resp.path)))
                .ok();
            let nodes = scan_dir(&state.notes_dir, state.show_hidden);
            state.tx.send(AppEvent::FileTreeRefresh(nodes)).ok();
            (StatusCode::CREATED, Json(resp)).into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

fn create_note_sync(
    state: &ApiState,
    title: &str,
    content: &str,
    tags: Option<&[String]>,
    folder: Option<&str>,
) -> anyhow::Result<NoteResponse> {
    let dest_dir = if let Some(f) = folder {
        state.notes_dir.join(f)
    } else {
        state.notes_dir.clone()
    };
    std::fs::create_dir_all(&dest_dir)?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let tags_yaml = tags
        .filter(|t| !t.is_empty())
        .map(|t| {
            let items = t
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("tags: [{items}]\n")
        })
        .unwrap_or_default();

    let fm = format!(
        "---\ntitle: \"{title}\"\nid: \"{id}\"\ncreated: \"{now}\"\nmodified: \"{now}\"\n{tags_yaml}---\n\n"
    );

    let stem = super::sanitize_filename(title);
    let path = super::unique_path(&dest_dir, &stem, "md");
    std::fs::write(&path, format!("{fm}{content}"))?;

    Ok(NoteResponse {
        path: path.to_string_lossy().to_string(),
        id,
        created: true,
    })
}
