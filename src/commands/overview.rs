use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::{self, Repo};
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct OverviewRow {
    name: String,
    path: String,
    branch: String,
    dirty: bool,
    ahead: u32,
    behind: u32,
    upstream: Option<String>,
    age: Option<String>,
    in_progress: Option<String>,
}

pub fn run(repos: &[Repo], _cli: &Cli, _cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    if repos.is_empty() {
        out.warn("no repositories found")?;
        return Ok(());
    }

    let rows: Vec<OverviewRow> = repos
        .par_iter()
        .map(|repo| {
            let status = repo::probe_status(&repo.path).unwrap_or_default();
            OverviewRow {
                name: repo.name.clone(),
                path: repo.display_path(),
                branch: if status.detached {
                    format!("detached@{}", status.branch)
                } else {
                    status.branch
                },
                dirty: status.dirty,
                ahead: status.ahead,
                behind: status.behind,
                upstream: status.upstream,
                age: status.last_commit_age_secs.map(repo::format_age),
                in_progress: status.in_progress,
            }
        })
        .collect();

    let mut rows = rows;
    rows.sort_by(|a, b| a.path.cmp(&b.path));

    if out.is_json() {
        out.write_json(&rows)?;
        return Ok(());
    }

    let table_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.name.clone(),
                r.branch.clone(),
                if r.dirty {
                    "dirty".into()
                } else {
                    "clean".into()
                },
                format!("{}/{}", r.ahead, r.behind),
                r.age.clone().unwrap_or_else(|| "-".into()),
                r.in_progress.clone().unwrap_or_else(|| "-".into()),
            ]
        })
        .collect();

    out.print_table(
        &["repo", "branch", "tree", "↑/↓", "age", "state"],
        table_rows,
    )?;
    out.info(&format!("{} repositories", rows.len()))?;
    Ok(())
}
