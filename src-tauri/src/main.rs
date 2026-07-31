#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use keyring::Entry;
use noterm::{
    config::{Config, VaultConfig},
    integrations::{fabric, mcp::McpClient, McpProfile},
    notes::{self, watcher::scan_dir, Note},
    search::fulltext::FtsIndex,
    sync::SyncDirection,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

struct DesktopState {
    config: Mutex<Config>,
}

#[derive(Serialize)]
struct VaultDto {
    id: String,
    name: String,
    path: String,
}
#[derive(Serialize)]
struct FileDto {
    path: String,
    relative_path: String,
    name: String,
    is_dir: bool,
    depth: usize,
}
#[derive(Serialize)]
struct NoteDto {
    path: String,
    relative_path: String,
    title: String,
    body: String,
    raw: String,
}

fn error<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

fn vault(config: &Config, id: &str) -> Result<VaultConfig, String> {
    config
        .resolved_vaults()
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("Unknown vault `{id}`"))
}

fn note_path(vault: &VaultConfig, relative: &str) -> Result<PathBuf, String> {
    let rel = Path::new(relative);
    if rel.is_absolute()
        || rel
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("Note path must be a relative path inside the selected vault".into());
    }
    let path = vault.path.join(rel);
    if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
        return Err("Only Markdown notes may be opened".into());
    }
    Ok(path)
}

#[tauri::command]
fn list_vaults(state: tauri::State<'_, DesktopState>) -> Result<Vec<VaultDto>, String> {
    let config = state.config.lock().map_err(error)?;
    Ok(config
        .resolved_vaults()
        .into_iter()
        .map(|v| VaultDto {
            id: v.id,
            name: v.name,
            path: v.path.to_string_lossy().to_string(),
        })
        .collect())
}

#[tauri::command]
fn list_files(
    vault_id: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<FileDto>, String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    Ok(scan_dir(&vault.path, config.ui.show_hidden)
        .into_iter()
        .map(|file| FileDto {
            path: file.path.to_string_lossy().to_string(),
            relative_path: file.relative_path,
            name: file.name,
            is_dir: file.is_dir,
            depth: file.depth,
        })
        .collect())
}

#[tauri::command]
fn read_note(
    vault_id: String,
    path: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<NoteDto, String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    let note = Note::from_path(&note_path(&vault, &path)?, &vault.path).map_err(error)?;
    let title = note.title().to_string();
    Ok(NoteDto {
        path: note.path.to_string_lossy().to_string(),
        relative_path: note.relative_path,
        title,
        body: note.body,
        raw: note.raw,
    })
}

#[tauri::command]
fn save_note(
    vault_id: String,
    path: String,
    body: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    let note_path = note_path(&vault, &path)?;
    let mut note = Note::from_path(&note_path, &vault.path).map_err(error)?;
    note.body = body;
    note.frontmatter.modified = Some(Utc::now().to_rfc3339());
    note.save().map_err(error)?;
    if note.frontmatter.publish {
        if let Some(kb_path) = config.kazam.kb_path.as_deref() {
            noterm::export::kazam::export_note(
                &note.path,
                &note.body,
                &note.frontmatter,
                Path::new(kb_path),
            )
            .map_err(error)?;
        }
    }
    let index = FtsIndex::open_or_create(&config.index_dir_for_vault(&vault)).map_err(error)?;
    index
        .index_note(
            &note.relative_path,
            note.title(),
            &note.body,
            note.frontmatter.tags.as_deref().unwrap_or(&[]),
        )
        .map_err(error)
}

#[tauri::command]
fn publish_note(
    vault_id: String,
    path: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    let kb_path = config
        .kazam
        .kb_path
        .as_deref()
        .ok_or("set kazam.kb_path in configuration before publishing")?;
    let note = Note::from_path(&note_path(&vault, &path)?, &vault.path).map_err(error)?;
    noterm::export::kazam::export_note(&note.path, &note.body, &note.frontmatter, Path::new(kb_path))
        .map(|_| ())
        .map_err(error)
}

#[tauri::command]
fn create_note(
    vault_id: String,
    title: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<NoteDto, String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    if title.trim().is_empty() {
        return Err("A note title is required".into());
    }
    std::fs::create_dir_all(vault.path.join("notes")).map_err(error)?;
    let stem = noterm::import::sanitize_filename(&title);
    let path = noterm::import::unique_path(&vault.path.join("notes"), &stem, "md");
    let now = Utc::now().to_rfc3339();
    std::fs::write(
        &path,
        format!(
            "---\ntitle: {:?}\nid: \"{}\"\ncreated: \"{}\"\nmodified: \"{}\"\n---\n\n",
            title,
            uuid::Uuid::new_v4(),
            now,
            now
        ),
    )
    .map_err(error)?;
    let note = Note::from_path(&path, &vault.path).map_err(error)?;
    let title = note.title().to_string();
    Ok(NoteDto {
        path: note.path.to_string_lossy().to_string(),
        relative_path: note.relative_path,
        title,
        body: note.body,
        raw: note.raw,
    })
}

#[tauri::command]
fn create_collection(
    vault_id: String,
    name: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<String, String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    let name = noterm::import::sanitize_filename(&name);
    if name.is_empty() || name == "." || name == ".." {
        return Err("A collection name is required".into());
    }
    let path = vault.path.join("notes").join(name);
    if path.exists() {
        return Err("That collection already exists".into());
    }
    std::fs::create_dir_all(&path).map_err(error)?;
    Ok(path.to_string_lossy().to_string())
}

/// Store reviewed external output as a regular vault note rather than hiding it
/// in an integration-specific database. Fabric uses this for a durable inbox.
#[tauri::command]
fn save_inbox_note(
    vault_id: String,
    title: String,
    body: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<NoteDto, String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    if title.trim().is_empty() {
        return Err("A note title is required".into());
    }
    let folder = vault.path.join("notes").join("inbox");
    std::fs::create_dir_all(&folder).map_err(error)?;
    let stem = noterm::import::sanitize_filename(&title);
    let path = noterm::import::unique_path(&folder, &stem, "md");
    let now = Utc::now().to_rfc3339();
    std::fs::write(
        &path,
        format!(
            "---\ntitle: {:?}\nid: \"{}\"\ncreated: \"{}\"\nmodified: \"{}\"\ntags: [fabric, inbox]\n---\n\n{}",
            title,
            uuid::Uuid::new_v4(),
            now,
            now,
            body.trim()
        ),
    )
    .map_err(error)?;
    let note = Note::from_path(&path, &vault.path).map_err(error)?;
    let note_title = note.title().to_string();
    let index = FtsIndex::open_or_create(&config.index_dir_for_vault(&vault)).map_err(error)?;
    index
        .index_note(
            &note.relative_path,
            note.title(),
            &note.body,
            note.frontmatter.tags.as_deref().unwrap_or(&[]),
        )
        .map_err(error)?;
    Ok(NoteDto {
        path: note.path.to_string_lossy().to_string(),
        relative_path: note.relative_path,
        title: note_title,
        body: note.body,
        raw: note.raw,
    })
}

#[tauri::command]
fn search_notes(
    vault_id: String,
    query: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<notes::SearchResult>, String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    let index = FtsIndex::open_or_create(&config.index_dir_for_vault(&vault)).map_err(error)?;
    for node in scan_dir(&vault.path, config.ui.show_hidden)
        .into_iter()
        .filter(|node| !node.is_dir)
    {
        if let Ok(note) = Note::from_path(&node.path, &vault.path) {
            index
                .index_note(
                    &note.relative_path,
                    note.title(),
                    &note.body,
                    note.frontmatter.tags.as_deref().unwrap_or(&[]),
                )
                .map_err(error)?;
        }
    }
    index.search(&query, 50).map_err(error)
}

#[tauri::command]
fn get_config(state: tauri::State<'_, DesktopState>) -> Result<Config, String> {
    Ok(state.config.lock().map_err(error)?.clone())
}

#[tauri::command]
fn save_config(config: Config, state: tauri::State<'_, DesktopState>) -> Result<(), String> {
    config.ensure_vault_layout().map_err(error)?;
    config.write().map_err(error)?;
    *state.config.lock().map_err(error)? = config;
    Ok(())
}

fn secret_entry(name: &str) -> Result<Entry, String> {
    Entry::new("homes.hpm.noterm", name).map_err(error)
}

#[tauri::command]
fn set_secret(name: String, value: String) -> Result<(), String> {
    secret_entry(&name)?.set_password(&value).map_err(error)
}

#[tauri::command]
fn delete_secret(name: String) -> Result<(), String> {
    secret_entry(&name)?.delete_credential().map_err(error)
}

fn profile(config: &Config, id: &str) -> Result<McpProfile, String> {
    config
        .integrations
        .mcp_profiles
        .iter()
        .find(|profile| profile.id == id)
        .cloned()
        .ok_or_else(|| format!("Unknown MCP profile `{id}`"))
}

fn profile_secret(profile: &McpProfile) -> Option<String> {
    profile
        .secret_ref
        .as_ref()
        .and_then(|name| secret_entry(name).ok()?.get_password().ok())
}

#[tauri::command]
fn mcp_list_tools(
    profile_id: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<noterm::integrations::mcp::McpTool>, String> {
    let config = state.config.lock().map_err(error)?;
    let profile = profile(&config, &profile_id)?;
    let mut client = McpClient::connect(&profile, profile_secret(&profile)).map_err(error)?;
    client.initialize().map_err(error)?;
    client.list_tools().map_err(error)
}

#[tauri::command]
fn mcp_call_tool(
    profile_id: String,
    name: String,
    arguments: Value,
    state: tauri::State<'_, DesktopState>,
) -> Result<Value, String> {
    let config = state.config.lock().map_err(error)?;
    let profile = profile(&config, &profile_id)?;
    let mut client = McpClient::connect(&profile, profile_secret(&profile)).map_err(error)?;
    client.initialize().map_err(error)?;
    client.call_tool(&name, arguments).map_err(error)
}

#[tauri::command]
fn fabric_patterns(state: tauri::State<'_, DesktopState>) -> Result<Vec<String>, String> {
    let config = state.config.lock().map_err(error)?;
    fabric::list_cli_patterns(&config.integrations.fabric.binary_path).map_err(error)
}

#[tauri::command]
fn fabric_run(
    pattern: String,
    input: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<String, String> {
    let config = state.config.lock().map_err(error)?;
    fabric::run_cli(&config.integrations.fabric.binary_path, &pattern, &input).map_err(error)
}

#[tauri::command]
fn sync_vault(
    vault_id: String,
    direction: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), String> {
    let config = state.config.lock().map_err(error)?.clone();
    let vault = vault(&config, &vault_id)?;
    let direction = match direction.as_str() {
        "push" => SyncDirection::Push,
        "pull" => SyncDirection::Pull,
        _ => return Err("Sync direction must be `push` or `pull`".into()),
    };
    let sync = vault.sync.clone().unwrap_or_else(|| config.sync.clone());
    let git = vault.git.clone().unwrap_or_else(|| config.git.clone());
    match sync.provider {
        noterm::config::SyncProvider::Git => {
            let remote = git
                .remote
                .as_deref()
                .ok_or("set git.remote in configuration before syncing")?;
            let branch = git
                .branch
                .as_deref()
                .ok_or("set git.branch in configuration before syncing")?;
            match direction {
                SyncDirection::Push => noterm::git::operations::push(
                    &vault.path,
                    remote,
                    branch,
                    git.git_username,
                    git.git_token,
                ),
                SyncDirection::Pull => noterm::git::operations::pull(
                    &vault.path,
                    remote,
                    branch,
                    git.git_username,
                    git.git_token,
                ),
            }
            .map_err(error)
        }
        _ => noterm::sync::sync_cloud_vault(&sync, &vault.path, direction).map_err(error),
    }
}

fn main() {
    let config = Config::load().expect("load Noterm configuration");
    config.ensure_vault_layout().expect("create Noterm vault layout");
    tauri::Builder::default()
        .manage(DesktopState { config: Mutex::new(config) })
        .invoke_handler(tauri::generate_handler![
            list_vaults,
            list_files,
            read_note,
            save_note,
            publish_note,
            create_note,
            create_collection,
            save_inbox_note,
            search_notes,
            get_config,
            save_config,
            set_secret,
            delete_secret,
            mcp_list_tools,
            mcp_call_tool,
            fabric_patterns,
            fabric_run,
            sync_vault
        ])
        .run(tauri::generate_context!())
        .expect("run Noterm desktop application");
}
