use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Fabric pattern with content provided through stdin.
///
/// The caller owns the review/apply decision; this function never writes notes.
pub fn run_cli(binary: &str, pattern: &str, input: &str) -> Result<String> {
    let mut child = Command::new(binary)
        .args(["--pattern", pattern])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("starting Fabric executable `{binary}`"))?;

    child
        .stdin
        .take()
        .context("opening Fabric stdin")?
        .write_all(input.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(
            "Fabric failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// A conservative list request for Fabric installations that support `--listpatterns`.
pub fn list_cli_patterns(binary: &str) -> Result<Vec<String>> {
    let output = Command::new(binary)
        .arg("--listpatterns")
        .output()
        .with_context(|| format!("starting Fabric executable `{binary}`"))?;
    if !output.status.success() {
        bail!(
            "Fabric pattern listing failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}
