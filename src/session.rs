//! Local-only UI session state; never stored in a vault or synchronized.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    #[serde(default)]
    pub active_vault_id: Option<String>,
    #[serde(default)]
    pub last_note_by_vault: BTreeMap<String, String>,
}

impl SessionState {
    fn path() -> std::path::PathBuf {
        crate::config::Config::data_dir().join("session.json")
    }
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) {
        if let Some(parent) = Self::path().parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(raw) = serde_json::to_string_pretty(self) {
            std::fs::write(Self::path(), raw).ok();
        }
    }
}
