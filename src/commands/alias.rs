use crate::cli::{AliasAction, Cli};
use crate::config::Config;
use crate::config_ops;
use crate::interactive;
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
                    let stale = if config_ops::alias_is_stale(path) {
                        "\tstale"
                    } else {
                        ""
                    };
                    writeln!(out.stdout(), "{name}\t{}{stale}", path.display())?;
                }
            }
            Ok(())
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
            config_ops::add_alias(&mut updated, name, resolved.clone());
            let saved = config_ops::save(&updated)?;
            out.success(&format!(
                "alias {name} → {} (saved {})",
                resolved.display(),
                saved.display()
            ))?;
            Ok(())
        }
        AliasAction::Remove { name } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would remove alias {name}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config_ops::remove_alias(&mut updated, name)?;
            let saved = config_ops::save(&updated)?;
            out.success(&format!("removed alias {name} ({})", saved.display()))?;
            Ok(())
        }
        AliasAction::Prune => {
            let stale = config_ops::list_stale_aliases(cfg);
            if stale.is_empty() {
                out.info("no stale aliases")?;
                return Ok(());
            }
            if cli.dry_run {
                out.info(&format!(
                    "dry-run: would prune {} stale alias(es)",
                    stale.len()
                ))?;
                for (n, p) in &stale {
                    writeln!(out.stdout(), "{n}\t{}", p.display())?;
                }
                return Ok(());
            }
            let mut updated = cfg.clone();
            let removed = config_ops::prune_stale_aliases(&mut updated);
            let saved = config_ops::save(&updated)?;
            out.success(&format!(
                "pruned {} stale alias(es) ({})",
                removed.len(),
                saved.display()
            ))?;
            for n in &removed {
                writeln!(out.stdout(), "{n}")?;
            }
            Ok(())
        }
        AliasAction::Wizard => interactive::aliases(cli, cfg, out),
        AliasAction::Ui => interactive::ui_focused(cli, cfg, out, crate::tui::Area::Aliases),
    }
}
