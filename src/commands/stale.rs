use crate::cli::Cli;
use crate::config::Config;
use crate::output::{CellStyle, OutputCtx};
use crate::repo::{self, ProbeOpts, Repo};
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct StaleRow {
    name: String,
    path: String,
    age_days: u64,
    last_commit: Option<String>,
}

pub fn run(repos: &[Repo], days: u64, _cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    let threshold = days.saturating_mul(86400);
    let pool = crate::exec::job_pool(cfg)?;
    let mut rows: Vec<StaleRow> = pool.install(|| {
        repos
            .par_iter()
            .filter_map(|repo| {
                let status = repo::probe_with(&repo.path, ProbeOpts::STALE).unwrap_or_default();
                if let Some(age) = status.last_commit_age_secs {
                    if age >= threshold {
                        Some(StaleRow {
                            name: repo.name.clone(),
                            path: repo.display_path(),
                            age_days: age / 86400,
                            last_commit: status.last_commit_subject,
                        })
                    } else {
                        None
                    }
                } else {
                    Some(StaleRow {
                        name: repo.name.clone(),
                        path: repo.display_path(),
                        age_days: days,
                        last_commit: None,
                    })
                }
            })
            .collect()
    });
    rows.sort_by(|a, b| a.path.cmp(&b.path));

    if out.is_json() {
        out.write_json(&rows)?;
        return Ok(());
    }

    if rows.is_empty() {
        out.info(&format!("no repos stale beyond {days} days"))?;
        return Ok(());
    }

    let table: Vec<Vec<_>> = rows
        .iter()
        .map(|r| {
            let age_secs = Some(r.age_days.saturating_mul(86400));
            vec![
                out.cell(out.repo_label_parts(&r.name, &r.path), CellStyle::Plain),
                out.cell(format!("{}d", r.age_days), OutputCtx::age_style(age_secs)),
                out.cell(
                    r.last_commit.clone().unwrap_or_else(|| "(none)".into()),
                    CellStyle::Dim,
                ),
                out.cell(&r.path, CellStyle::Dim),
            ]
        })
        .collect();
    out.print_table_cells(&["repo", "age", "last commit", "path"], table)?;
    Ok(())
}
