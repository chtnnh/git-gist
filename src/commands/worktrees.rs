use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::{self, Repo};
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorktreeRow {
    pub repo: String,
    pub path: String,
    pub head: String,
    pub branch: String,
    pub bare: bool,
    pub locked: bool,
    pub prunable: bool,
}

pub fn run(repos: &[Repo], _cli: &Cli, _cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    let mut rows = Vec::new();
    for repo in repos {
        let Ok(porcelain) = repo::git_stdout(&repo.path, &["worktree", "list", "--porcelain"])
        else {
            continue;
        };
        rows.extend(parse_porcelain(&out.repo_label(repo), &porcelain));
    }

    if out.is_json() {
        out.write_json(&rows)?;
        return Ok(());
    }

    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.repo.clone(),
                r.path.clone(),
                r.branch.clone(),
                r.head.clone(),
                flags(r),
            ]
        })
        .collect();
    out.print_table(&["repo", "path", "branch", "head", "flags"], table)?;
    Ok(())
}

pub fn parse_porcelain(repo_name: &str, porcelain: &str) -> Vec<WorktreeRow> {
    let mut rows = Vec::new();
    let mut current = WorktreeRow {
        repo: repo_name.to_string(),
        path: String::new(),
        head: String::new(),
        branch: String::new(),
        bare: false,
        locked: false,
        prunable: false,
    };
    for line in porcelain.lines() {
        if line.is_empty() {
            if !current.path.is_empty() {
                rows.push(current);
                current = WorktreeRow {
                    repo: repo_name.to_string(),
                    path: String::new(),
                    head: String::new(),
                    branch: String::new(),
                    bare: false,
                    locked: false,
                    prunable: false,
                };
            }
            continue;
        }
        if let Some(p) = line.strip_prefix("worktree ") {
            current.path = p.to_string();
        } else if let Some(h) = line.strip_prefix("HEAD ") {
            current.head = h.chars().take(8).collect();
        } else if let Some(b) = line.strip_prefix("branch ") {
            current.branch = b.trim_start_matches("refs/heads/").to_string();
        } else if line == "bare" {
            current.bare = true;
        } else if line.starts_with("locked") {
            current.locked = true;
        } else if line.starts_with("prunable") {
            current.prunable = true;
        }
    }
    if !current.path.is_empty() {
        rows.push(current);
    }
    rows
}

pub fn flags(r: &WorktreeRow) -> String {
    let mut f = Vec::new();
    if r.bare {
        f.push("bare");
    }
    if r.locked {
        f.push("locked");
    }
    if r.prunable {
        f.push("prunable");
    }
    if f.is_empty() {
        "-".into()
    } else {
        f.join(",")
    }
}
