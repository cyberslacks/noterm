//! External-tool integrations shared by the terminal and desktop clients.

pub mod fabric;
pub mod mcp;

use serde::{Deserialize, Serialize};

/// Non-secret connection settings for a reusable MCP server profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    pub transport: McpTransport,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    /// Name of the secret in the operating-system credential store, never the secret itself.
    #[serde(default)]
    pub secret_ref: Option<String>,
    #[serde(default)]
    pub import_folder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    #[default]
    Stdio,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FabricIntegrationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_fabric_binary")]
    pub binary_path: String,
    #[serde(default)]
    pub rest_url: Option<String>,
}

fn default_fabric_binary() -> String {
    "fabric".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationConfig {
    #[serde(default)]
    pub mcp_profiles: Vec<McpProfile>,
    #[serde(default)]
    pub fabric: FabricIntegrationConfig,
}
