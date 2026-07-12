use crate::cli::Cli;
use crate::config::Config;
use crate::output::{CellStyle, OutputCtx};
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
    #[serde(skip)]
    age_secs: Option<u64>,
    #[serde(skip)]
    detached: bool,
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
                age_secs: status.last_commit_age_secs,
                detached: status.detached,
            }
        })
        .collect();

    let mut rows = rows;
    rows.sort_by(|a, b| a.path.cmp(&b.path));

    if out.is_json() {
        out.write_json(&rows)?;
        return Ok(());
    }

    let table_rows: Vec<Vec<_>> = rows
        .iter()
        .map(|r| {
            let tree = if r.dirty { "dirty" } else { "clean" };
            let ab = format!("{}/{}", r.ahead, r.behind);
            let age = r.age.clone().unwrap_or_else(|| "-".into());
            let state = r.in_progress.clone().unwrap_or_else(|| "-".into());
            let branch_style = if r.detached {
                CellStyle::Warn
            } else {
                CellStyle::Plain
            };
            let state_style = if r.in_progress.is_some() {
                CellStyle::Bad
            } else {
                CellStyle::Dim
            };
            vec![
                out.cell(&r.name, CellStyle::Plain),
                out.cell(&r.branch, branch_style),
                out.cell(tree, OutputCtx::tree_style(r.dirty)),
                out.cell(ab, OutputCtx::ahead_behind_style(r.ahead, r.behind)),
                out.cell(age, OutputCtx::age_style(r.age_secs)),
                out.cell(state, state_style),
            ]
        })
        .collect();

    out.print_table_cells(
        &["repo", "branch", "tree", "↑/↓", "age", "state"],
        table_rows,
    )?;
    out.info(&format!("{} repositories", rows.len()))?;
    Ok(())
}
