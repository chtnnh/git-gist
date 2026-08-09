//! Enroll newly discovered repos into aliases, groups, and tags.

use crate::auto_enroll::{self, UpdateReport};
use crate::cli::Cli;
use crate::config::Config;
use crate::config_ops;
use crate::output::OutputCtx;
use anyhow::{bail, Result};
use std::io::{self, IsTerminal, Write};

pub fn run(
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
    prune_stale_flag: bool,
    no_prune_stale: bool,
    ask: bool,
) -> Result<()> {
    if cfg.auto_enroll.is_empty() {
        let path = cfg
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.git-gist/config.toml".into());
        for w in &cfg.load_warnings {
            if w.contains("unknown key") && w.contains("auto_enroll") {
                bail!(
                    "no [[auto_enroll]] rules in config (loaded {path})\n\
                     warning: {w}\n\
                     fix the typo or run `gg config wizard` / `gg doctor --config`"
                );
            }
        }
        bail!(
            "no [[auto_enroll]] rules in config (loaded {path})\n\
             add rules then re-run `gg update`, or run `gg config wizard`\n\
             example:\n\
             [[auto_enroll]]\n\
             path = \"/path/to/watch\"\n\
             path_prefix = \"oss/\"\n\
             depth = 6\n\
             tags = [\"learning\"]\n\
             groups = [\"oss\"]"
        );
    }

    let prune_stale = resolve_prune_stale(cli, cfg, out, prune_stale_flag, no_prune_stale, ask)?;
    let dry_run = cli.dry_run;
    let report = auto_enroll::apply_auto_enroll(cfg, dry_run, prune_stale)?;
    print_report(&report, out)
}

fn resolve_prune_stale(
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
    prune_stale_flag: bool,
    no_prune_stale: bool,
    ask: bool,
) -> Result<bool> {
    if no_prune_stale {
        return Ok(false);
    }
    if prune_stale_flag {
        return Ok(true);
    }
    let stale = config_ops::list_stale_aliases(cfg);
    if stale.is_empty() {
        return Ok(false);
    }
    let ask = ask
        || (io::stdin().is_terminal()
            && io::stdout().is_terminal()
            && !cli.dry_run
            && !out.is_json());
    if !ask {
        if cli.verbose > 0 {
            out.warn(&format!(
                "{} stale alias(es) present — pass --prune-stale or --ask to reclaim short names",
                stale.len()
            ))?;
        }
        return Ok(false);
    }
    // Interactive confirm is a TTY prompt; keep it out of the coverage denominator.
    #[cfg(coverage)]
    {
        let _ = (out, stale);
        return Ok(false);
    }
    #[cfg(not(coverage))]
    {
        out.info(&format!(
            "{} stale alias(es) block preferred names:",
            stale.len()
        ))?;
        for (name, path) in stale.iter().take(20) {
            writeln!(out.stdout(), "  {name}\t{}", path.display())?;
        }
        if stale.len() > 20 {
            writeln!(out.stdout(), "  … and {} more", stale.len() - 20)?;
        }
        #[cfg(feature = "wizard")]
        {
            use inquire::Confirm;
            let ok = Confirm::new("Prune stale aliases before enrolling?")
                .with_default(true)
                .prompt()
                .unwrap_or(false);
            Ok(ok)
        }
        #[cfg(not(feature = "wizard"))]
        {
            out.info("re-run with --prune-stale to remove them")?;
            Ok(false)
        }
    }
}

pub fn print_report(report: &UpdateReport, out: &mut OutputCtx) -> Result<()> {
    if out.is_json() {
        out.write_json(report)?;
        return Ok(());
    }

    for w in &report.warnings {
        out.warn(w)?;
    }

    if !report.pruned_stale.is_empty() {
        let prefix = if report.dry_run {
            "would prune"
        } else {
            "pruned"
        };
        out.info(&format!(
            "{prefix} {} stale alias(es): {}",
            report.pruned_stale.len(),
            report.pruned_stale.join(", ")
        ))?;
    }

    if report.added.is_empty() && report.membership_fixed == 0 && report.pruned_stale.is_empty() {
        out.info(&format!(
            "no changes ({} already enrolled under {} rule(s))",
            report.skipped_existing, report.rules
        ))?;
    } else {
        for change in &report.added {
            let mut bits = Vec::new();
            if !change.groups.is_empty() {
                bits.push(format!("groups=[{}]", change.groups.join(", ")));
            }
            if !change.tags.is_empty() {
                bits.push(format!("tags=[{}]", change.tags.join(", ")));
            }
            let suffix = if bits.is_empty() {
                String::new()
            } else {
                format!(" ({})", bits.join(", "))
            };
            let prefix = if report.dry_run { "would add" } else { "added" };
            out.success(&format!(
                "{prefix} {} → {}{suffix}",
                change.alias, change.path
            ))?;
        }
        if report.membership_fixed > 0 {
            let prefix = if report.dry_run {
                "would update"
            } else {
                "updated"
            };
            out.info(&format!(
                "{prefix} group/tag membership for {} existing alias(es)",
                report.membership_fixed
            ))?;
        }
        if !report.added.is_empty() {
            out.info(&format!(
                "{} new alias(es); {} already present",
                report.added.len(),
                report.skipped_existing
            ))?;
        }
    }

    if let Some(path) = &report.saved {
        out.info(&format!("saved {path}"))?;
    } else if report.dry_run
        && (!report.added.is_empty()
            || report.membership_fixed > 0
            || !report.pruned_stale.is_empty())
    {
        out.info("dry-run — config not written")?;
    }

    Ok(())
}
