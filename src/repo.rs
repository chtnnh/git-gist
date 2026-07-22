//! Repository model and lightweight git probes via shelling out to git.

use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
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

    /// Human label: basename, or `name (relative-or-absolute path)` when `show_path`.
    pub fn label(&self, show_path: bool, root: Option<&Path>) -> String {
        if !show_path {
            return self.name.clone();
        }
        let path_part = root
            .and_then(|r| {
                let root = r.canonicalize().unwrap_or_else(|_| r.to_path_buf());
                let path = self
                    .path
                    .canonicalize()
                    .unwrap_or_else(|_| self.path.clone());
                path.strip_prefix(&root)
                    .ok()
                    .map(|p| p.display().to_string())
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.display_path());
        // Prefer a relative path that adds information; fall back to absolute.
        let path_part = if path_part == self.name {
            self.display_path()
        } else {
            path_part
        };
        format!("{} ({})", self.name, path_part)
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

/// Which fields to gather. Fewer bits ⇒ fewer `git` process spawns.
#[derive(Debug, Clone, Copy)]
pub struct ProbeOpts {
    /// `git status --porcelain=v2 --branch` → branch, dirty, upstream, ahead/behind, detached.
    pub status_branch: bool,
    /// `git stash list`
    pub stash: bool,
    /// `git log -1 --format=%ct%x00%s`
    pub last_commit: bool,
    /// Filesystem markers under the git dir (no git process).
    pub in_progress: bool,
}

impl ProbeOpts {
    pub const FULL: Self = Self {
        status_branch: true,
        stash: true,
        last_commit: true,
        in_progress: true,
    };

    pub const FILTER_TREE: Self = Self {
        status_branch: true,
        stash: false,
        last_commit: false,
        in_progress: false,
    };

    pub const FILTER_STASH: Self = Self {
        status_branch: false,
        stash: true,
        last_commit: false,
        in_progress: false,
    };

    pub const STALE: Self = Self {
        status_branch: false,
        stash: false,
        last_commit: true,
        in_progress: false,
    };

    pub const DOCTOR: Self = Self {
        status_branch: true,
        stash: false,
        last_commit: false,
        in_progress: true,
    };

    pub fn for_cli_filters(
        only_dirty: bool,
        only_clean: bool,
        only_ahead: bool,
        only_behind: bool,
        only_stashed: bool,
        only_detached: bool,
    ) -> Self {
        let need_tree = only_dirty || only_clean || only_ahead || only_behind || only_detached;
        Self {
            status_branch: need_tree,
            stash: only_stashed,
            last_commit: false,
            in_progress: false,
        }
    }
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
    probe_with(repo, ProbeOpts::FULL)
}

pub fn probe_with(repo: &Path, opts: ProbeOpts) -> Result<RepoStatus> {
    let mut status = RepoStatus::default();

    if opts.status_branch {
        match git_stdout(repo, &["status", "--porcelain=v2", "--branch"]) {
            Ok(out) => apply_porcelain_v2(&mut status, &out),
            Err(_) => status.branch = "(unknown)".into(),
        }
    }

    if opts.stash {
        if let Ok(out) = git_stdout(repo, &["stash", "list"]) {
            status.stashed = if out.is_empty() {
                0
            } else {
                out.lines().count() as u32
            };
        }
    }

    if opts.last_commit {
        if let Ok(out) = git_stdout(repo, &["log", "-1", "--format=%ct%x00%s"]) {
            let mut parts = out.split('\0');
            if let Some(epoch) = parts.next() {
                if let Ok(ts) = epoch.parse::<u64>() {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(ts);
                    status.last_commit_age_secs = Some(now.saturating_sub(ts));
                }
            }
            if let Some(subj) = parts.next() {
                if !subj.is_empty() {
                    status.last_commit_subject = Some(subj.to_string());
                }
            }
        }
    }

    if opts.in_progress {
        status.in_progress = detect_in_progress(repo);
    }

    Ok(status)
}

/// Parse `git status --porcelain=v2 --branch` into status fields.
pub fn apply_porcelain_v2(status: &mut RepoStatus, out: &str) {
    let mut oid: Option<&str> = None;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("# branch.oid ") {
            oid = Some(rest.trim());
        } else if let Some(rest) = line.strip_prefix("# branch.head ") {
            let rest = rest.trim();
            if rest == "(detached)" || rest.starts_with("(detached") {
                status.detached = true;
                status.branch = oid
                    .map(|o| o.chars().take(7).collect())
                    .unwrap_or_else(|| "detached".into());
            } else {
                status.branch = rest.to_string();
            }
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            status.upstream = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            status.dirty = true;
        }
    }
    if status.branch.is_empty() {
        status.branch = "(unknown)".into();
    }
}

pub fn resolve_git_dir(repo: &Path) -> PathBuf {
    let git = repo.join(".git");
    if git.is_dir() {
        return git;
    }
    if git.is_file() {
        if let Ok(contents) = fs::read_to_string(&git) {
            for line in contents.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("gitdir:") {
                    let p = PathBuf::from(rest.trim());
                    return if p.is_absolute() { p } else { repo.join(p) };
                }
            }
        }
    }
    git
}

fn detect_in_progress(repo: &Path) -> Option<String> {
    let git_dir = resolve_git_dir(repo);
    for (marker, label) in [
        ("MERGE_HEAD", "merge"),
        ("rebase-merge", "rebase"),
        ("rebase-apply", "rebase"),
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
        ("BISECT_LOG", "bisect"),
    ] {
        if git_dir.join(marker).exists() {
            return Some(label.into());
        }
    }
    None
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
