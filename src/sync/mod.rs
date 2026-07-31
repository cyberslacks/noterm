//! Portable, vault-level synchronization.
//!
//! Markdown files are always the source of truth.  Git uses the existing
//! libgit2 integration; Google Drive and OneDrive use a configured `rclone`
//! remote so OAuth credentials stay with rclone rather than Noterm.

use anyhow::{bail, Context, Result};
use std::{path::Path, process::Command};

use crate::config::{SyncConfig, SyncProvider};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Push,
    Pull,
}

pub fn sync_cloud_vault(config: &SyncConfig, vault: &Path, direction: SyncDirection) -> Result<()> {
    if config.provider == SyncProvider::Git {
        bail!("Git sync is handled by the Git panel; configure a branch before pushing or pulling")
    }

    let remote = config
        .rclone_remote
        .as_deref()
        .filter(|remote| !remote.trim().is_empty())
        .context("set sync.rclone_remote to an rclone remote such as `gdrive:noterm`")?;
    let local = vault.to_string_lossy().to_string();
    let (source, destination) = match direction {
        SyncDirection::Push => (local.as_str(), remote),
        SyncDirection::Pull => (remote, local.as_str()),
    };

    let status = Command::new("rclone")
        .args(["sync", source, destination, "--exclude", ".noterm/**"])
        .status()
        .context("run rclone; install it and configure the selected cloud remote first")?;
    if !status.success() {
        bail!("rclone sync exited with {status}");
    }
    Ok(())
}
