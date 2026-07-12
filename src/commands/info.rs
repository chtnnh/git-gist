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
    _cli: &Cli,
    _cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    let targets: Vec<Repo> = if let Some(p) = path {
        vec![Repo::new(p.to_path_buf())]
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
