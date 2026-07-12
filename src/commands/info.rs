use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::{self, Repo};
use anyhow::Result;
use serde::Serialize;
use std::io::Write;
use std::path::Path;

#[derive(Serialize)]
struct InfoRow {
    name: String,
    path: String,
    branch: String,
    dirty: bool,
    ahead: u32,
    behind: u32,
    detached: bool,
    stashed: u32,
    upstream: Option<String>,
    last_commit: Option<String>,
    in_progress: Option<String>,
}

pub fn run(
    repos: &[Repo],
    path: Option<&Path>,
    cli: &Cli,
    _cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    let targets: Vec<Repo> = if let Some(p) = path {
        let repo = Repo::new(p.to_path_buf());
        if selection_filters_active(cli) {
            // Selection already applied filters; only show the path if it survived.
            if repos.iter().any(|r| paths_equal(&r.path, &repo.path)) {
                vec![repo]
            } else {
                Vec::new()
            }
        } else {
            // No selection filters: allow probing an arbitrary path.
            vec![repo]
        }
    } else {
        repos.to_vec()
    };

    let rows: Vec<InfoRow> = targets
        .iter()
        .filter_map(|repo| {
            let status = repo::probe_status(&repo.path).ok()?;
            Some(InfoRow {
                name: repo.name.clone(),
                path: repo.display_path(),
                branch: status.branch,
                dirty: status.dirty,
                ahead: status.ahead,
                behind: status.behind,
                detached: status.detached,
                stashed: status.stashed,
                upstream: status.upstream,
                last_commit: status.last_commit_subject,
                in_progress: status.in_progress,
            })
        })
        .collect();

    if out.is_json() {
        out.write_json(&rows)?;
        return Ok(());
    }

    for r in &rows {
        out.repo_header(&r.name, &r.path)?;
        writeln!(out.stdout(), "  branch:    {}", r.branch)?;
        writeln!(out.stdout(), "  dirty:     {}", r.dirty)?;
        writeln!(out.stdout(), "  ahead:     {}", r.ahead)?;
        writeln!(out.stdout(), "  behind:    {}", r.behind)?;
        writeln!(out.stdout(), "  detached:  {}", r.detached)?;
        writeln!(out.stdout(), "  stashed:   {}", r.stashed)?;
        if let Some(u) = &r.upstream {
            writeln!(out.stdout(), "  upstream:  {u}")?;
        }
        if let Some(c) = &r.last_commit {
            writeln!(out.stdout(), "  commit:    {c}")?;
        }
        if let Some(s) = &r.in_progress {
            writeln!(out.stdout(), "  in-progress: {s}")?;
        }
    }
    Ok(())
}

fn selection_filters_active(cli: &Cli) -> bool {
    !cli.include.is_empty()
        || !cli.group.is_empty()
        || !cli.exclude.is_empty()
        || !cli.tag.is_empty()
        || cli.only_dirty
        || cli.only_clean
        || cli.only_ahead
        || cli.only_behind
        || cli.only_stashed
        || cli.only_detached
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    let a = a.canonicalize().unwrap_or_else(|_| a.to_path_buf());
    let b = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());
    a == b
}
