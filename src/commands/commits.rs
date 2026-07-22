use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::{self, Repo};
use anyhow::Result;
use chrono::{TimeZone, Utc};
use serde::Serialize;

#[derive(Serialize)]
struct CommitRow {
    repo: String,
    path: String,
    hash: String,
    author: String,
    date: String,
    subject: String,
    timestamp: i64,
}

pub fn run(
    repos: &[Repo],
    number: usize,
    _cli: &Cli,
    _cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    let mut all = Vec::new();
    for repo in repos {
        let fmt = "%H%x09%an%x09%at%x09%s";
        let n = number.to_string();
        if let Ok(log) = repo::git_stdout(
            &repo.path,
            &["log", &format!("-n{n}"), &format!("--format={fmt}")],
        ) {
            for line in log.lines() {
                let mut parts = line.splitn(4, '\t');
                let hash = parts.next().unwrap_or("").to_string();
                let author = parts.next().unwrap_or("").to_string();
                let ts: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let subject = parts.next().unwrap_or("").to_string();
                let date = Utc
                    .timestamp_opt(ts, 0)
                    .single()
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default();
                all.push(CommitRow {
                    repo: repo.name.clone(),
                    path: repo.display_path(),
                    hash: hash.chars().take(8).collect(),
                    author,
                    date,
                    subject,
                    timestamp: ts,
                });
            }
        }
    }

    all.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
    all.truncate(number.max(1) * repos.len().max(1));
    // Show top-N globally
    let top_n = number;
    if all.len() > top_n {
        all.truncate(top_n);
    }

    if out.is_json() {
        out.write_json(&all)?;
        return Ok(());
    }

    let rows: Vec<Vec<String>> = all
        .iter()
        .map(|c| {
            vec![
                out.repo_label_parts(&c.repo, &c.path),
                c.hash.clone(),
                c.date.clone(),
                c.author.clone(),
                c.subject.clone(),
            ]
        })
        .collect();
    out.print_table(&["repo", "hash", "date", "author", "subject"], rows)?;
    Ok(())
}
