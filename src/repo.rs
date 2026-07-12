//! Repository model and lightweight git probes via shelling out to git.

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a `git` command, honoring `GIT_GIST_GIT` (override used in tests).
pub fn git_command() -> Command {
    let bin = std::env::var("GIT_GIST_GIT").unwrap_or_else(|_| "git".to_string());
    Command::new(bin)
}

#[derive(Debug, Clone, Serialize)]
pub struct Repo {
    pub path: PathBuf,
    pub name: String,
}

impl Repo {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self { path, name }
    }

    pub fn display_path(&self) -> String {
        self.path.display().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RepoStatus {
    pub branch: String,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
    pub detached: bool,
    pub stashed: u32,
    pub upstream: Option<String>,
    pub last_commit_age_secs: Option<u64>,
    pub last_commit_subject: Option<String>,
    pub in_progress: Option<String>,
}

pub fn git_in(repo: &Path, args: &[&str]) -> Result<std::process::Output> {
    git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("running git in {}", repo.display()))
}

pub fn git_stdout(repo: &Path, args: &[&str]) -> Result<String> {
    let out = git_in(repo, args)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("git {:?} failed: {}", args, stderr.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn probe_status(repo: &Path) -> Result<RepoStatus> {
    let mut status = RepoStatus::default();

    // Branch / detached
    match git_stdout(repo, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok(b) if b == "HEAD" => {
            status.detached = true;
            status.branch = git_stdout(repo, &["rev-parse", "--short", "HEAD"])
                .unwrap_or_else(|_| "detached".into());
        }
        Ok(b) => status.branch = b,
        Err(_) => status.branch = "(unknown)".into(),
    }

    // Dirty: porcelain
    if let Ok(out) = git_stdout(repo, &["status", "--porcelain"]) {
        status.dirty = !out.is_empty();
    }

    // Upstream ahead/behind
    if let Ok(upstream) = git_stdout(repo, &["rev-parse", "--abbrev-ref", "@{upstream}"]) {
        status.upstream = Some(upstream);
        if let Ok(counts) = git_stdout(
            repo,
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        ) {
            let mut parts = counts.split_whitespace();
            status.ahead = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            status.behind = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }

    // Stashes
    if let Ok(out) = git_stdout(repo, &["stash", "list"]) {
        status.stashed = if out.is_empty() {
            0
        } else {
            out.lines().count() as u32
        };
    }

    // Last commit
    if let Ok(epoch) = git_stdout(repo, &["log", "-1", "--format=%ct"]) {
        if let Ok(ts) = epoch.parse::<u64>() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(ts);
            status.last_commit_age_secs = Some(now.saturating_sub(ts));
        }
    }
    if let Ok(subj) = git_stdout(repo, &["log", "-1", "--format=%s"]) {
        status.last_commit_subject = Some(subj);
    }

    // In-progress operations
    let git_dir = git_stdout(repo, &["rev-parse", "--git-dir"]).unwrap_or_else(|_| ".git".into());
    let git_dir_path = if Path::new(&git_dir).is_absolute() {
        PathBuf::from(&git_dir)
    } else {
        repo.join(&git_dir)
    };
    for (marker, label) in [
        ("MERGE_HEAD", "merge"),
        ("rebase-merge", "rebase"),
        ("rebase-apply", "rebase"),
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
        ("BISECT_LOG", "bisect"),
    ] {
        if git_dir_path.join(marker).exists() {
            status.in_progress = Some(label.into());
            break;
        }
    }

    Ok(status)
}

pub fn format_age(secs: u64) -> String {
    const DAY: u64 = 86400;
    const HOUR: u64 = 3600;
    const MIN: u64 = 60;
    if secs >= DAY * 2 {
        format!("{}d", secs / DAY)
    } else if secs >= DAY {
        "1d".into()
    } else if secs >= HOUR {
        format!("{}h", secs / HOUR)
    } else if secs >= MIN {
        format!("{}m", secs / MIN)
    } else {
        format!("{secs}s")
    }
}
