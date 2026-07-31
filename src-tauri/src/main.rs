#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use keyring::Entry;
use noterm::{
    config::{Config, VaultConfig},
    integrations::{fabric, mcp::McpClient, McpProfile},
    notes::{self, watcher::scan_dir, Note},
    search::fulltext::FtsIndex,
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
    let index = FtsIndex::open_or_create(&config.index_dir()).map_err(error)?;
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
    std::fs::create_dir_all(&vault.path).map_err(error)?;
    let stem = noterm::import::sanitize_filename(&title);
    let path = noterm::import::unique_path(&vault.path, &stem, "md");
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
fn search_notes(
    vault_id: String,
    query: String,
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<notes::SearchResult>, String> {
    let config = state.config.lock().map_err(error)?;
    let vault = vault(&config, &vault_id)?;
    let index = FtsIndex::open_or_create(&config.index_dir()).map_err(error)?;
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

fn main() {
    tauri::Builder::default()
        .manage(DesktopState {
            config: Mutex::new(Config::load().expect("load Noterm configuration")),
        })
        .invoke_handler(tauri::generate_handler![
            list_vaults,
            list_files,
            read_note,
            save_note,
            create_note,
            search_notes,
            get_config,
            save_config,
            set_secret,
            delete_secret,
            mcp_list_tools,
            mcp_call_tool,
            fabric_patterns,
            fabric_run
        ])
        .run(tauri::generate_context!())
        .expect("run Noterm desktop application");
}
