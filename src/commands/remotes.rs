use crate::cli::{Cli, RemotesAction};
use crate::config::Config;
use crate::config_ops;
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
            Ok(())
        }
        RemotesAction::Add { name, url } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would add remote {name} → {url}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config_ops::add_remote(&mut updated, name, url);
            let saved = config_ops::save(&updated)?;
            out.success(&format!(
                "added remote {name} → {url} ({})",
                saved.display()
            ))?;
            Ok(())
        }
        RemotesAction::Remove { name } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would remove remote {name}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config_ops::remove_remote(&mut updated, name)?;
            let saved = config_ops::save(&updated)?;
            out.success(&format!("removed remote {name} ({})", saved.display()))?;
            Ok(())
        }
        RemotesAction::AddTo { name, as_name } => {
            let spec = cfg
                .remotes
                .get(name)
                .with_context(|| format!("catalog remote not found: {name}"))?
                .clone();
            let remote_name = as_name.as_deref().unwrap_or(name);
            if repos.is_empty() {
                bail!("no repositories selected");
            }
            crate::repo::validate_remote_name(remote_name)?;

            // Expand every URL before changing any repository. A malformed
            // template must not leave a partially updated selection behind.
            let resolved: Vec<_> = repos
                .iter()
                .map(|repo| {
                    let repo_name = if crate::remote_url::requires_repository_name(&spec)? {
                        Some(crate::remote_url::repository_name(&repo.path)?)
                    } else {
                        None
                    };
                    crate::remote_url::resolve_remote_url(&spec, repo_name).map(|url| (repo, url))
                })
                .collect::<Result<_>>()?;
            let mut failures = 0usize;
            for (repo, url) in resolved {
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
                    let status = crate::repo::git_command()
                        .args(["remote", "set-url", remote_name, &url])
                        .current_dir(&repo.path)
                        .status()?;
                    if status.success() {
                        out.success(&format!("{}: updated remote {remote_name}", repo.name))?;
                    } else {
                        failures += 1;
                        out.warn(&format!(
                            "{}: failed to add remote {remote_name}",
                            repo.name
                        ))?;
                    }
                }
            }
            if failures > 0 {
                bail!("failed to apply remote {remote_name} to {failures} repository(s)");
            }
            Ok(())
        }
        RemotesAction::Wizard => crate::interactive::remotes(cli, cfg, out),
        RemotesAction::Ui => {
            crate::interactive::ui_focused(cli, cfg, out, crate::tui::Area::Remotes)
        }
    }
}
