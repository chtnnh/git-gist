use crate::cli::Cli;
use crate::config::Config;
use crate::exec;
use crate::output::{CellStyle, OutputCtx};
use crate::repo::{self, Repo};
use anyhow::Result;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct SyncRow {
    name: String,
    path: String,
    branch: String,
    ahead: u32,
    behind: u32,
    dirty: bool,
    fetch_ok: bool,
    pulled: bool,
}

pub fn run(repos: &[Repo], pull: bool, cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    if repos.is_empty() {
        out.warn("no repositories selected")?;
        return Ok(());
    }

    if cli.dry_run {
        if out.is_json() {
            let rows: Vec<_> = repos
                .iter()
                .map(|repo| {
                    serde_json::json!({
                        "name": repo.name,
                        "path": repo.display_path(),
                        "dry_run": true,
                        "fetch": ["fetch", "--all", "--prune"],
                        "pull": pull,
                    })
                })
                .collect();
            out.write_json(&rows)?;
        } else {
            for repo in repos {
                out.repo_header(&repo.name, &repo.display_path())?;
                writeln!(out.stdout(), "dry-run: git fetch --all --prune")?;
                if pull {
                    writeln!(
                        out.stdout(),
                        "dry-run: git pull --ff-only (if behind && clean)"
                    )?;
                }
            }
        }
        return Ok(());
    }

    // Fetch; ignore aggregate failure so we still report per-repo status
    let fetch_ok = exec::run_git(repos, &["fetch", "--all", "--prune"], cli, cfg, out).is_ok();

    let mut rows = Vec::new();
    for repo in repos {
        let status = repo::probe_status(&repo.path).unwrap_or_default();
        let mut pulled = false;
        if pull && !status.dirty && status.behind > 0 && status.ahead == 0 {
            let outp = repo::git_in(&repo.path, &["pull", "--ff-only"]);
            pulled = outp.map(|o| o.status.success()).unwrap_or(false);
        }
        rows.push(SyncRow {
            name: repo.name.clone(),
            path: repo.display_path(),
            branch: status.branch,
            ahead: status.ahead,
            behind: status.behind,
            dirty: status.dirty,
            fetch_ok,
            pulled,
        });
    }

    if out.is_json() {
        out.write_json(&rows)?;
        return Ok(());
    }

    let table: Vec<Vec<_>> = rows
        .iter()
        .map(|r| {
            let tree = if r.dirty { "dirty" } else { "clean" };
            let ab = format!("{}/{}", r.ahead, r.behind);
            let pull = if r.pulled { "pulled" } else { "-" };
            vec![
                out.cell(out.repo_label_parts(&r.name, &r.path), CellStyle::Plain),
                out.cell(&r.branch, CellStyle::Plain),
                out.cell(tree, OutputCtx::tree_style(r.dirty)),
                out.cell(ab, OutputCtx::ahead_behind_style(r.ahead, r.behind)),
                out.cell(
                    pull,
                    if r.pulled {
                        CellStyle::Good
                    } else {
                        CellStyle::Dim
                    },
                ),
            ]
        })
        .collect();
    out.print_table_cells(&["repo", "branch", "tree", "↑/↓", "pull"], table)?;
    Ok(())
}
