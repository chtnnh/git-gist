use crate::cli::{Cli, RemotesAction};
use crate::config::{self, Config};
use crate::output::OutputCtx;
use crate::repo::Repo;
use anyhow::{bail, Context, Result};
use std::io::Write;

pub fn run(
    action: &RemotesAction,
    repos: &[Repo],
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    match action {
        RemotesAction::List => {
            if out.is_json() {
                out.write_json(&cfg.remotes)?;
            } else if cfg.remotes.is_empty() {
                out.info("no remotes in catalog")?;
            } else {
                for (name, url) in &cfg.remotes {
                    writeln!(out.stdout(), "{name}\t{url}")?;
                }
            }
        }
        RemotesAction::Add { name, url } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would add remote {name} → {url}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            updated.remotes.insert(name.clone(), url.clone());
            let saved = config::save_global(&updated)?;
            out.success(&format!(
                "added remote {name} → {url} ({})",
                saved.display()
            ))?;
        }
        RemotesAction::Remove { name } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would remove remote {name}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            if updated.remotes.remove(name).is_none() {
                bail!("remote not found: {name}");
            }
            let saved = config::save_global(&updated)?;
            out.success(&format!("removed remote {name} ({})", saved.display()))?;
        }
        RemotesAction::AddTo { name, as_name } => {
            let url = cfg
                .remotes
                .get(name)
                .with_context(|| format!("catalog remote not found: {name}"))?
                .clone();
            let remote_name = as_name.as_deref().unwrap_or(name);
            if repos.is_empty() {
                bail!("no repositories selected");
            }
            for repo in repos {
                if cli.dry_run {
                    out.info(&format!(
                        "dry-run: would add remote {remote_name} → {url} in {}",
                        repo.name
                    ))?;
                    continue;
                }
                let status = crate::repo::git_command()
                    .args(["remote", "add", remote_name, &url])
                    .current_dir(&repo.path)
                    .status()?;
                if status.success() {
                    out.success(&format!("{}: added remote {remote_name}", repo.name))?;
                } else {
                    // try set-url if exists
                    let status = crate::repo::git_command()
                        .args(["remote", "set-url", remote_name, &url])
                        .current_dir(&repo.path)
                        .status()?;
                    if status.success() {
                        out.success(&format!("{}: updated remote {remote_name}", repo.name))?;
                    } else {
                        out.warn(&format!(
                            "{}: failed to add remote {remote_name}",
                            repo.name
                        ))?;
                    }
                }
            }
        }
    }
    Ok(())
}
