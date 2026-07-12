use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::{self, Repo};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct StaleRow {
    name: String,
    path: String,
    age_days: u64,
    last_commit: Option<String>,
}

pub fn run(
    repos: &[Repo],
    days: u64,
    _cli: &Cli,
    _cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    let threshold = days.saturating_mul(86400);
    let mut rows = Vec::new();
    for repo in repos {
        let status = repo::probe_status(&repo.path).unwrap_or_default();
        if let Some(age) = status.last_commit_age_secs {
            if age >= threshold {
                rows.push(StaleRow {
                    name: repo.name.clone(),
                    path: repo.display_path(),
                    age_days: age / 86400,
                    last_commit: status.last_commit_subject,
                });
            }
        } else {
            rows.push(StaleRow {
                name: repo.name.clone(),
                path: repo.display_path(),
                age_days: days,
                last_commit: None,
            });
        }
    }

    if out.is_json() {
        out.write_json(&rows)?;
        return Ok(());
    }

    if rows.is_empty() {
        out.info(&format!("no repos stale beyond {days} days"))?;
        return Ok(());
    }

    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.name.clone(),
                format!("{}d", r.age_days),
                r.last_commit.clone().unwrap_or_else(|| "(none)".into()),
                r.path.clone(),
            ]
        })
        .collect();
    out.print_table(&["repo", "age", "last commit", "path"], table)?;
    Ok(())
}
