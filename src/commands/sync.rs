use crate::cli::Cli;
use crate::config::Config;
use crate::exec;
use crate::output::{CellStyle, OutputCtx};
use crate::repo::{self, Repo};
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

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

    // Fetch without writing JSON/human output — sync emits its own summary.
    let fetch_results = exec::run_git_inner(repos, &["fetch", "--all", "--prune"], cli, cfg)?;
    let fetch_by_path: HashMap<PathBuf, bool> = fetch_results
        .into_iter()
        .map(|r| (r.repo.path.clone(), r.success && !r.skipped))
        .collect();

    let mut rows = Vec::new();
    for repo in repos {
        let status = repo::probe_status(&repo.path).unwrap_or_default();
        let fetch_ok = fetch_by_path.get(&repo.path).copied().unwrap_or(false);
        let mut pulled = false;
        if should_ff_pull(pull, fetch_ok, &status) {
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

fn should_ff_pull(pull: bool, fetch_ok: bool, status: &repo::RepoStatus) -> bool {
    pull && fetch_ok && !status.dirty && status.behind > 0 && status.ahead == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::RepoStatus;

    #[test]
    fn ff_pull_requires_fetch_ok_and_clean_behind() {
        let behind = RepoStatus {
            dirty: false,
            behind: 2,
            ahead: 0,
            ..Default::default()
        };
        assert!(should_ff_pull(true, true, &behind));
        assert!(!should_ff_pull(true, false, &behind));
        assert!(!should_ff_pull(false, true, &behind));
        assert!(!should_ff_pull(
            true,
            true,
            &RepoStatus {
                dirty: true,
                behind: 2,
                ..Default::default()
            }
        ));
        assert!(!should_ff_pull(
            true,
            true,
            &RepoStatus {
                behind: 0,
                ahead: 1,
                ..Default::default()
            }
        ));
    }
}
