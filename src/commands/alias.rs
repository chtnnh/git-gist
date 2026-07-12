use crate::cli::{AliasAction, Cli};
use crate::config::{self, Config};
use crate::output::OutputCtx;
use anyhow::Result;
use std::io::Write;

pub fn run(action: &AliasAction, cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    match action {
        AliasAction::List => {
            if out.is_json() {
                out.write_json(&cfg.aliases)?;
            } else if cfg.aliases.is_empty() {
                out.info("no aliases configured")?;
            } else {
                for (name, path) in &cfg.aliases {
                    writeln!(out.stdout(), "{name}\t{}", path.display())?;
                }
            }
        }
        AliasAction::Add { name, path } => {
            let resolved = path.canonicalize().unwrap_or_else(|_| path.clone());
            if cli.dry_run {
                out.info(&format!(
                    "dry-run: would alias {name} → {}",
                    resolved.display()
                ))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            updated.aliases.insert(name.clone(), resolved.clone());
            let saved = config::save_global(&updated)?;
            out.success(&format!(
                "alias {name} → {} (saved {})",
                resolved.display(),
                saved.display()
            ))?;
        }
        AliasAction::Remove { name } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would remove alias {name}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            if updated.aliases.remove(name).is_none() {
                anyhow::bail!("alias not found: {name}");
            }
            let saved = config::save_global(&updated)?;
            out.success(&format!("removed alias {name} ({})", saved.display()))?;
        }
    }
    Ok(())
}
