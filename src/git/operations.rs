use super::{GitCommit, GitStatus};
use anyhow::Result;
use std::path::Path;

// All git2 operations must run in spawn_blocking because git2::Repository is not Send.

pub fn get_status(repo_path: &Path) -> Result<GitStatus> {
    let repo = git2::Repository::open(repo_path)?;

    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from))
        .unwrap_or_else(|| "HEAD".into());

    let statuses = repo.statuses(None)?;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let s = entry.status();

        if s.intersects(
            git2::Status::INDEX_NEW | git2::Status::INDEX_MODIFIED | git2::Status::INDEX_DELETED,
        ) {
            staged.push(path.clone());
        }
        if s.intersects(git2::Status::WT_MODIFIED | git2::Status::WT_DELETED) {
            unstaged.push(path.clone());
        }
        if s.contains(git2::Status::WT_NEW) {
            untracked.push(path.clone());
        }
    }

    Ok(GitStatus {
        branch,
        ahead: 0,
        behind: 0,
        staged,
        unstaged,
        untracked,
    })
}

pub fn get_log(repo_path: &Path, limit: usize) -> Result<Vec<GitCommit>> {
    let repo = git2::Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commits = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= limit {
            break;
        }
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        commits.push(GitCommit {
            hash: oid.to_string()[..8].to_string(),
            message: commit.summary().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            time: format_time(commit.time().seconds()),
        });
    }

    Ok(commits)
}

pub fn stage_all(repo_path: &Path) -> Result<()> {
    let repo = git2::Repository::open(repo_path)?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

pub fn commit(repo_path: &Path, message: &str) -> Result<()> {
    let repo = git2::Repository::open(repo_path)?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let sig = repo.signature()?;

    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    Ok(())
}

pub fn push(
    repo_path: &Path,
    remote: &str,
    branch: &str,
    git_username: Option<String>,
    git_token: Option<String>,
) -> Result<()> {
    let repo = git2::Repository::open(repo_path)?;
    let mut remote = repo.find_remote(remote)?;

    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(make_callbacks(git_username, git_token));

    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    remote.push(&[refspec.as_str()], Some(&mut push_opts))?;
    Ok(())
}

pub fn pull(
    repo_path: &Path,
    remote: &str,
    branch: &str,
    git_username: Option<String>,
    git_token: Option<String>,
) -> Result<()> {
    let repo = git2::Repository::open(repo_path)?;
    let mut remote_obj = repo.find_remote(remote)?;

    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(make_callbacks(git_username, git_token));
    remote_obj.fetch(&[branch], Some(&mut fetch_opts), None)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_fast_forward() {
        let refname = format!("refs/heads/{branch}");
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    }

    Ok(())
}

/// Build a `RemoteCallbacks` with a multi-strategy credential chain:
/// 1. HTTPS token (username + token from settings) — tried if the remote asks for user/pass
/// 2. SSH agent — tried if the remote asks for an SSH key
/// 3. SSH key files (~/.ssh/id_ed25519, id_rsa, id_ecdsa) without passphrase
///
/// The `tried` flag prevents infinite retry loops: libgit2 calls the callback again on
/// auth failure, and returning an error on the second call terminates the loop cleanly.
fn make_callbacks(
    git_username: Option<String>,
    git_token: Option<String>,
) -> git2::RemoteCallbacks<'static> {
    let tried = std::cell::Cell::new(false);

    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |_url, username, allowed_types| {
        // Return a hard error on second call to prevent libgit2 retry loops.
        if tried.replace(true) {
            return Err(git2::Error::from_str(
                "Authentication failed. Set git_username / git_token in Settings (S key).",
            ));
        }

        let user = username.unwrap_or("git");

        // HTTPS token (explicit config takes priority over SSH).
        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            if let (Some(ref u), Some(ref p)) = (&git_username, &git_token) {
                return git2::Cred::userpass_plaintext(u, p);
            }
        }

        // SSH agent.
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            if let Ok(cred) = git2::Cred::ssh_key_from_agent(user) {
                return Ok(cred);
            }
            // Fall through to key files if agent fails.
            if let Some(home) = dirs::home_dir() {
                for name in &["id_ed25519", "id_rsa", "id_ecdsa"] {
                    let path = home.join(".ssh").join(name);
                    if path.exists() {
                        if let Ok(cred) = git2::Cred::ssh_key(user, None, &path, None) {
                            return Ok(cred);
                        }
                    }
                }
            }
        }

        Err(git2::Error::from_str(
            "No credentials found. Configure git_username / git_token in Settings (S key).",
        ))
    });
    callbacks
}

fn format_time(unix: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(unix, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "unknown".into())
}
