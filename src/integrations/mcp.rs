//! Minimal JSON-RPC MCP transport helpers.
//!
//! Profiles are deliberately generic: discovery provides the tool schema rather
//! than baking individual third-party note applications into Noterm.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use super::{McpProfile, McpTransport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Value,
}

pub enum McpClient {
    Stdio(StdioClient),
    Http(HttpClient),
}

pub struct StdioClient {
    #[allow(dead_code)]
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: u64,
}

pub struct HttpClient {
    url: String,
    bearer_token: Option<String>,
    request_id: u64,
    client: reqwest::blocking::Client,
}

impl McpClient {
    pub fn connect(profile: &McpProfile, secret: Option<String>) -> Result<Self> {
        match profile.transport {
            McpTransport::Stdio => {
                let command = profile
                    .command
                    .as_deref()
                    .context("MCP stdio profile is missing command")?;
                let mut child = Command::new(command)
                    .args(&profile.args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .with_context(|| format!("starting MCP command `{command}`"))?;
                Ok(Self::Stdio(StdioClient {
                    stdin: child.stdin.take().context("opening MCP stdin")?,
                    stdout: BufReader::new(child.stdout.take().context("opening MCP stdout")?),
                    child,
                    request_id: 0,
                }))
            }
            McpTransport::Http => Ok(Self::Http(HttpClient {
                url: profile
                    .url
                    .clone()
                    .context("MCP HTTP profile is missing url")?,
                bearer_token: secret,
                request_id: 0,
                client: reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build()?,
            })),
        }
    }

    pub fn initialize(&mut self) -> Result<Value> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "noterm", "version": env!("CARGO_PKG_VERSION") }
            }),
        )
    }

    pub fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let result = self.request("tools/list", json!({}))?;
        Ok(serde_json::from_value(
            result.get("tools").cloned().unwrap_or_else(|| json!([])),
        )?)
    }

    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        match self {
            McpClient::Stdio(client) => client.request(method, params),
            McpClient::Http(client) => client.request(method, params),
        }
    }
}

impl StdioClient {
    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.request_id += 1;
        let id = self.request_id;
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        writeln!(self.stdin, "{}", serde_json::to_string(&request)?)?;
        self.stdin.flush()?;
        let mut line = String::new();
        loop {
            line.clear();
            if self.stdout.read_line(&mut line)? == 0 {
                bail!("MCP server closed stdout");
            }
            let response: Value = serde_json::from_str(line.trim())?;
            if response.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                bail!("MCP error: {error}");
            }
            return Ok(response.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

impl HttpClient {
    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.request_id += 1;
        let mut request = self.client.post(&self.url)
            .header("Accept", "application/json, text/event-stream")
            .json(&json!({ "jsonrpc": "2.0", "id": self.request_id, "method": method, "params": params }));
        if let Some(token) = &self.bearer_token {
            request = request.bearer_auth(token);
        }
        let response: Value = request
            .send()?
            .error_for_status()?
            .json()
            .context("MCP HTTP server returned a non-JSON response")?;
        if let Some(error) = response.get("error") {
            bail!("MCP error: {error}");
        }
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }
}
