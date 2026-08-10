//! Status-based repo filters.

use crate::cli::Cli;
use crate::repo::{self, ProbeOpts, Repo};
use anyhow::Result;
use rayon::prelude::*;

pub fn apply_status_filters(repos: Vec<Repo>, cli: &Cli, jobs: Option<usize>) -> Result<Vec<Repo>> {
    if repos.is_empty() {
        return Ok(repos);
    }
    let jobs = jobs.unwrap_or_else(num_cpus::get).max(1);
    let opts = ProbeOpts::for_cli_filters(
        cli.only_dirty,
        cli.only_clean,
        cli.only_ahead,
        cli.only_behind,
        cli.only_stashed,
        cli.only_detached,
    );
    let total = repos.len();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?;
    let outcomes: Vec<(Repo, Result<crate::repo::RepoStatus, String>)> = pool.install(|| {
        repos
            .into_par_iter()
            .map(|repo| {
                let path = repo.path.clone();
                let probed = repo::probe_with(&path, opts).map_err(|e| e.to_string());
                (repo, probed)
            })
            .collect()
    });

    let mut probe_failures = 0usize;
    let mut filtered = Vec::new();
    for (repo, probed) in outcomes {
        let status = match probed {
            Ok(s) => s,
            Err(err) => {
                probe_failures += 1;
                eprintln!("git-gist: probe failed for {}: {err}", repo.path.display());
                continue;
            }
        };
        if cli.only_dirty && !status.dirty {
            continue;
        }
        if cli.only_clean && status.dirty {
            continue;
        }
        if cli.only_ahead && status.ahead == 0 {
            continue;
        }
        if cli.only_behind && status.behind == 0 {
            continue;
        }
        if cli.only_stashed && status.stashed == 0 {
            continue;
        }
        if cli.only_detached && !status.detached {
            continue;
        }
        filtered.push(repo);
    }

    if probe_failures == total {
        anyhow::bail!("probe failed for all {total} repositories");
    }
    Ok(filtered)
}
