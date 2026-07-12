//! Status-based repo filters.

use crate::cli::Cli;
use crate::repo::{self, Repo};
use anyhow::Result;
use rayon::prelude::*;

pub fn apply_status_filters(
    repos: Vec<Repo>,
    cli: &Cli,
    jobs: Option<usize>,
) -> Result<Vec<Repo>> {
    let jobs = jobs.unwrap_or_else(num_cpus::get).max(1);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?;
    let filtered: Vec<Repo> = pool.install(|| {
        repos
            .into_par_iter()
            .filter_map(|repo| {
                let status = repo::probe_status(&repo.path).ok()?;
                if cli.only_dirty && !status.dirty {
                    return None;
                }
                if cli.only_clean && status.dirty {
                    return None;
                }
                if cli.only_ahead && status.ahead == 0 {
                    return None;
                }
                if cli.only_behind && status.behind == 0 {
                    return None;
                }
                if cli.only_stashed && status.stashed == 0 {
                    return None;
                }
                if cli.only_detached && !status.detached {
                    return None;
                }
                Some(repo)
            })
            .collect()
    });
    Ok(filtered)
}
