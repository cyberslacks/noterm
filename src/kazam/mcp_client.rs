/// Kazam MCP client: connects to `kazam mcp` in the KB directory over stdio.
///
/// Protocol: JSON-RPC 2.0, newline-delimited, sequential (one in-flight request at a time).
/// All I/O is blocking — always call from `spawn_blocking`.
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

pub struct KazamMcpClient {
    #[allow(dead_code)]
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl std::fmt::Debug for KazamMcpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KazamMcpClient")
            .field("next_id", &self.next_id)
            .finish_non_exhaustive()
    }
}

impl KazamMcpClient {
    /// Spawn the current Kazam stdio server in its KB directory and perform the
    /// MCP initialize handshake. Kazam resolves its project from the working
    /// directory; older `--kb` invocation is no longer used.
    pub fn spawn(binary_path: &str, kb_path: &str) -> anyhow::Result<Self> {
        let mut child = Command::new(binary_path)
            .arg("mcp")
            .current_dir(kb_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn kazam: {e}"))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        };

        // MCP initialize
        let init_resp = client.call(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "noterm", "version": env!("CARGO_PKG_VERSION") }
            }),
        )?;

        if init_resp.get("error").is_some() {
            anyhow::bail!("MCP initialize error: {}", init_resp["error"]);
        }

        // Send initialized notification (no response expected)
        client.notify("notifications/initialized", json!({}))?;

        Ok(client)
    }

    /// Send a JSON-RPC request and return the full response Value.
    fn call(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let mut line = serde_json::to_string(&request)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.flush()?;

        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line)?;
        let resp: Value = serde_json::from_str(response_line.trim())?;
        Ok(resp)
    }

    /// Send a JSON-RPC notification (no response expected).
    fn notify(&mut self, method: &str, params: Value) -> anyhow::Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let mut line = serde_json::to_string(&notification)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Call a Kazam MCP tool by name and return the result content.
    fn tool_call(&mut self, tool: &str, args: Value) -> anyhow::Result<Value> {
        let resp = self.call("tools/call", json!({ "name": tool, "arguments": args }))?;
        if let Some(err) = resp.get("error") {
            anyhow::bail!("MCP tool error: {err}");
        }
        Ok(resp["result"].clone())
    }

    pub fn search(&mut self, query: &str) -> anyhow::Result<Vec<Value>> {
        let result = self.tool_call("search", json!({ "query": query }))?;
        Ok(result["content"].as_array().cloned().unwrap_or_default())
    }

    pub fn list_pages(&mut self) -> anyhow::Result<Vec<Value>> {
        let result = self.tool_call("list_pages", json!({}))?;
        Ok(result["content"].as_array().cloned().unwrap_or_default())
    }

    pub fn read_page(&mut self, slug: &str) -> anyhow::Result<Value> {
        let result = self.tool_call("read_page", json!({ "slug": slug }))?;
        Ok(result)
    }

    pub fn write_page(&mut self, slug: &str, yaml: &str) -> anyhow::Result<()> {
        self.tool_call("write_page", json!({ "slug": slug, "content": yaml }))?;
        Ok(())
    }

    pub fn annotate_page(&mut self, slug: &str, text: &str, section: &str) -> anyhow::Result<()> {
        self.tool_call(
            "annotate_page",
            json!({ "slug": slug, "text": text, "section": section }),
        )?;
        Ok(())
    }

    pub fn list_annotations(&mut self, slug: &str) -> anyhow::Result<Vec<Value>> {
        let result = self.tool_call("list_annotations", json!({ "page": slug }))?;
        Ok(result["content"].as_array().cloned().unwrap_or_default())
    }

    pub fn get_config(&mut self) -> anyhow::Result<Value> {
        self.tool_call("get_config", json!({}))
    }

    pub fn update_annotation(&mut self, slug: &str, id: &str, status: &str) -> anyhow::Result<()> {
        self.tool_call(
            "update_annotation",
            json!({ "slug": slug, "id": id, "status": status }),
        )?;
        Ok(())
    }
}
