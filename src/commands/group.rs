use crate::cli::{Cli, GroupAction};
use crate::config::{self, Config};
use crate::output::OutputCtx;
use anyhow::Result;
use std::io::Write;

pub fn run(action: &GroupAction, cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    match action {
        GroupAction::List => {
            if out.is_json() {
                out.write_json(&cfg.groups)?;
            } else if cfg.groups.is_empty() {
                out.info("no groups configured")?;
            } else {
                for (name, members) in &cfg.groups {
                    writeln!(out.stdout(), "{name}\t{}", members.join(", "))?;
                }
            }
        }
        GroupAction::Add { name, members } => {
            if cli.dry_run {
                out.info(&format!(
                    "dry-run: would set group {name} = [{}]",
                    members.join(", ")
                ))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            updated.groups.insert(name.clone(), members.clone());
            let saved = config::save_global(&updated)?;
            out.success(&format!(
                "group {name} = [{}] ({})",
                members.join(", "),
                saved.display()
            ))?;
        }
        GroupAction::Remove { name } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would remove group {name}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            if updated.groups.remove(name).is_none() {
                anyhow::bail!("group not found: {name}");
            }
            let saved = config::save_global(&updated)?;
            out.success(&format!("removed group {name} ({})", saved.display()))?;
        }
    }
    Ok(())
}
