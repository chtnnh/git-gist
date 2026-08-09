//! Interactive inquire-based config wizards.

use crate::cli::{Cli, OutputFormat};
use crate::config::{AutoEnroll, Config};
use crate::config_ops;
use crate::output::OutputCtx;
use anyhow::{bail, Result};
use std::io::IsTerminal;
use std::path::PathBuf;

fn require_tty(cli: &Cli, out: &OutputCtx) -> Result<()> {
    if matches!(cli.format, OutputFormat::Json | OutputFormat::Ndjson) {
        bail!("interactive wizard is incompatible with --format json/ndjson");
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!(
            "interactive wizard requires a TTY — use scriptable commands \
             (`gg alias add`, `gg group add`, …) instead"
        );
    }
    if out.is_json() {
        bail!("interactive wizard is incompatible with JSON output");
    }
    Ok(())
}

#[cfg(not(feature = "wizard"))]
mod stubs {
    use super::*;
    fn disabled() -> Result<()> {
        bail!("wizard feature disabled — rebuild with `--features wizard` or use scriptable CLI")
    }
    pub fn run_hub(_: &Cli, _: &Config, _: &mut OutputCtx) -> Result<()> {
        disabled()
    }
    pub fn run_aliases(_: &Cli, _: &Config, _: &mut OutputCtx) -> Result<()> {
        disabled()
    }
    pub fn run_groups(_: &Cli, _: &Config, _: &mut OutputCtx) -> Result<()> {
        disabled()
    }
    pub fn run_tags(_: &Cli, _: &Config, _: &mut OutputCtx) -> Result<()> {
        disabled()
    }
    pub fn run_remotes(_: &Cli, _: &Config, _: &mut OutputCtx) -> Result<()> {
        disabled()
    }
    pub fn run_enroll(_: &Cli, _: &Config, _: &mut OutputCtx) -> Result<()> {
        disabled()
    }
    pub fn run_settings(_: &Cli, _: &Config, _: &mut OutputCtx) -> Result<()> {
        disabled()
    }
}

#[cfg(not(feature = "wizard"))]
pub use stubs::*;

#[cfg(feature = "wizard")]
mod impl_wizard {
    use super::*;
    use inquire::{Confirm, MultiSelect, Select, Text};

    pub fn run_hub(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
        require_tty(cli, out)?;
        let mut draft = cfg.clone();
        let mut dirty = false;
        loop {
            let choice = Select::new(
                "What would you like to manage?",
                vec![
                    "Aliases",
                    "Groups",
                    "Tags",
                    "Remotes",
                    "Auto-enroll rules",
                    "Settings",
                    "Prune stale aliases",
                    "Preview & save",
                    "Quit",
                ],
            )
            .prompt()?;
            match choice {
                "Aliases" => {
                    if edit_aliases(&mut draft)? {
                        dirty = true;
                    }
                }
                "Groups" => {
                    if edit_groups(&mut draft)? {
                        dirty = true;
                    }
                }
                "Tags" => {
                    if edit_tags(&mut draft)? {
                        dirty = true;
                    }
                }
                "Remotes" => {
                    if edit_remotes(&mut draft)? {
                        dirty = true;
                    }
                }
                "Auto-enroll rules" => {
                    if edit_enroll(&mut draft)? {
                        dirty = true;
                    }
                }
                "Settings" => {
                    if edit_settings(&mut draft)? {
                        dirty = true;
                    }
                }
                "Prune stale aliases" => {
                    if prune_stale(&mut draft, out)? {
                        dirty = true;
                    }
                }
                "Preview & save" => {
                    if save_draft(&draft, dirty, out)? {
                        dirty = false;
                        draft = crate::config::load(cli)?;
                    }
                }
                "Quit" => {
                    if dirty
                        && Confirm::new("Discard unsaved changes?")
                            .with_default(false)
                            .prompt()?
                    {
                        break;
                    }
                    if !dirty {
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    pub fn run_aliases(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
        require_tty(cli, out)?;
        let mut draft = cfg.clone();
        if edit_aliases(&mut draft)? {
            let path = config_ops::save(&draft)?;
            out.success(&format!("saved {}", path.display()))?;
        }
        Ok(())
    }

    pub fn run_groups(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
        require_tty(cli, out)?;
        let mut draft = cfg.clone();
        if edit_groups(&mut draft)? {
            let path = config_ops::save(&draft)?;
            out.success(&format!("saved {}", path.display()))?;
        }
        Ok(())
    }

    pub fn run_tags(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
        require_tty(cli, out)?;
        let mut draft = cfg.clone();
        if edit_tags(&mut draft)? {
            let path = config_ops::save(&draft)?;
            out.success(&format!("saved {}", path.display()))?;
        }
        Ok(())
    }

    pub fn run_remotes(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
        require_tty(cli, out)?;
        let mut draft = cfg.clone();
        if edit_remotes(&mut draft)? {
            let path = config_ops::save(&draft)?;
            out.success(&format!("saved {}", path.display()))?;
        }
        Ok(())
    }

    pub fn run_enroll(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
        require_tty(cli, out)?;
        let mut draft = cfg.clone();
        if edit_enroll(&mut draft)? {
            let path = config_ops::save(&draft)?;
            out.success(&format!("saved {}", path.display()))?;
        }
        Ok(())
    }

    pub fn run_settings(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
        require_tty(cli, out)?;
        let mut draft = cfg.clone();
        if edit_settings(&mut draft)? {
            let path = config_ops::save(&draft)?;
            out.success(&format!("saved {}", path.display()))?;
        }
        Ok(())
    }

    fn edit_aliases(cfg: &mut Config) -> Result<bool> {
        let action =
            Select::new("Aliases", vec!["Add", "Remove", "Prune stale", "Done"]).prompt()?;
        match action {
            "Add" => {
                let name = Text::new("Alias name").prompt()?;
                let path = Text::new("Path").prompt()?;
                config_ops::add_alias(cfg, &name, PathBuf::from(path));
                Ok(true)
            }
            "Remove" => {
                if cfg.aliases.is_empty() {
                    println!("no aliases");
                    return Ok(false);
                }
                let names: Vec<_> = cfg.aliases.keys().cloned().collect();
                let name = Select::new("Remove alias", names).prompt()?;
                config_ops::remove_alias(cfg, &name)?;
                Ok(true)
            }
            "Prune stale" => {
                let removed = config_ops::prune_stale_aliases(cfg);
                println!("pruned {} stale alias(es)", removed.len());
                Ok(!removed.is_empty())
            }
            _ => Ok(false),
        }
    }

    fn edit_groups(cfg: &mut Config) -> Result<bool> {
        let mut options = cfg.groups.keys().cloned().collect::<Vec<_>>();
        options.push("(new group)".into());
        options.push("(done)".into());
        let pick = Select::new("Group", options).prompt()?;
        if pick == "(done)" {
            return Ok(false);
        }
        let name = if pick == "(new group)" {
            Text::new("Group name").prompt()?
        } else {
            pick
        };
        let aliases: Vec<_> = cfg.aliases.keys().cloned().collect();
        if aliases.is_empty() {
            println!("no aliases to assign — add aliases first");
            return Ok(false);
        }
        let defaults = cfg.groups.get(&name).cloned().unwrap_or_default();
        let selected = MultiSelect::new("Members", aliases)
            .with_default(
                &defaults
                    .iter()
                    .filter_map(|d| cfg.aliases.keys().position(|k| k == d))
                    .collect::<Vec<_>>(),
            )
            .prompt()?;
        config_ops::set_group_members(cfg, &name, selected);
        Ok(true)
    }

    fn edit_tags(cfg: &mut Config) -> Result<bool> {
        let mut options = cfg.tags.keys().cloned().collect::<Vec<_>>();
        options.push("(new tag)".into());
        options.push("(done)".into());
        let pick = Select::new("Tag", options).prompt()?;
        if pick == "(done)" {
            return Ok(false);
        }
        let name = if pick == "(new tag)" {
            Text::new("Tag name").prompt()?
        } else {
            pick
        };
        let aliases: Vec<_> = cfg.aliases.keys().cloned().collect();
        if aliases.is_empty() {
            println!("no aliases to assign");
            return Ok(false);
        }
        let defaults = cfg.tags.get(&name).cloned().unwrap_or_default();
        let selected = MultiSelect::new("Members", aliases)
            .with_default(
                &defaults
                    .iter()
                    .filter_map(|d| cfg.aliases.keys().position(|k| k == d))
                    .collect::<Vec<_>>(),
            )
            .prompt()?;
        config_ops::set_tag_members(cfg, &name, selected);
        Ok(true)
    }

    fn edit_remotes(cfg: &mut Config) -> Result<bool> {
        let action = Select::new("Remotes", vec!["Add", "Remove", "Done"]).prompt()?;
        match action {
            "Add" => {
                let name = Text::new("Remote name").prompt()?;
                let url = Text::new("URL template").prompt()?;
                config_ops::add_remote(cfg, &name, &url);
                Ok(true)
            }
            "Remove" => {
                if cfg.remotes.is_empty() {
                    println!("no remotes");
                    return Ok(false);
                }
                let names: Vec<_> = cfg.remotes.keys().cloned().collect();
                let name = Select::new("Remove remote", names).prompt()?;
                config_ops::remove_remote(cfg, &name)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn edit_enroll(cfg: &mut Config) -> Result<bool> {
        let action =
            Select::new("Auto-enroll", vec!["Add rule", "Remove rule", "Done"]).prompt()?;
        match action {
            "Add rule" => {
                let path = Text::new("Watch path").prompt()?;
                let depth: usize = Text::new("Depth")
                    .with_default("6")
                    .prompt()?
                    .parse()
                    .unwrap_or(6);
                let path_prefix = Text::new("path_prefix (optional, e.g. oss/)")
                    .with_default("")
                    .prompt()?;
                let groups = if cfg.groups.is_empty() {
                    Vec::new()
                } else {
                    let names: Vec<_> = cfg.groups.keys().cloned().collect();
                    MultiSelect::new("Assign to groups", names)
                        .prompt()
                        .unwrap_or_default()
                };
                let tags = if cfg.tags.is_empty() {
                    Vec::new()
                } else {
                    let names: Vec<_> = cfg.tags.keys().cloned().collect();
                    MultiSelect::new("Assign to tags", names)
                        .prompt()
                        .unwrap_or_default()
                };
                config_ops::add_auto_enroll_rule(
                    cfg,
                    AutoEnroll {
                        path: PathBuf::from(path),
                        path_prefix: if path_prefix.trim().is_empty() {
                            None
                        } else {
                            Some(path_prefix)
                        },
                        depth,
                        groups,
                        tags,
                    },
                );
                Ok(true)
            }
            "Remove rule" => {
                if cfg.auto_enroll.is_empty() {
                    println!("no rules");
                    return Ok(false);
                }
                let labels: Vec<_> = cfg
                    .auto_enroll
                    .iter()
                    .enumerate()
                    .map(|(i, r)| format!("{i}: {}", r.path.display()))
                    .collect();
                let pick = Select::new("Remove rule", labels).prompt()?;
                let index: usize = pick.split(':').next().unwrap().parse().unwrap_or(0);
                config_ops::remove_auto_enroll_rule(cfg, index)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn edit_settings(cfg: &mut Config) -> Result<bool> {
        let key = Select::new(
            "Setting",
            vec![
                "depth",
                "jobs",
                "theme",
                "show_path",
                "include_submodules",
                "root",
                "(done)",
            ],
        )
        .prompt()?;
        if key == "(done)" {
            return Ok(false);
        }
        let current = config::get_dot_key(cfg, key).unwrap_or_default();
        let value = Text::new(&format!("New value for {key}"))
            .with_default(&current)
            .prompt()?;
        config_ops::set_scalar(cfg, key, &value)?;
        Ok(true)
    }

    fn prune_stale(cfg: &mut Config, out: &mut OutputCtx) -> Result<bool> {
        let stale = config_ops::list_stale_aliases(cfg);
        if stale.is_empty() {
            out.info("no stale aliases")?;
            return Ok(false);
        }
        let labels: Vec<_> = stale
            .iter()
            .map(|(n, p)| format!("{n} → {}", p.display()))
            .collect();
        let defaults: Vec<usize> = (0..labels.len()).collect();
        let selected = MultiSelect::new("Prune stale aliases", labels)
            .with_default(&defaults)
            .prompt()?;
        let names: Vec<_> = selected
            .iter()
            .filter_map(|s| s.split(" → ").next().map(|s| s.to_string()))
            .collect();
        for n in &names {
            let _ = config_ops::remove_alias(cfg, n);
        }
        out.info(&format!("will prune {} alias(es) on save", names.len()))?;
        Ok(!names.is_empty())
    }

    fn save_draft(draft: &Config, dirty: bool, out: &mut OutputCtx) -> Result<bool> {
        if !dirty {
            out.info("no changes to save")?;
            return Ok(false);
        }
        if Confirm::new("Save config?").with_default(true).prompt()? {
            let path = config_ops::save(draft)?;
            out.success(&format!("saved {}", path.display()))?;
            if let Ok(report) = crate::auto_enroll::apply_auto_enroll(draft, true, false) {
                if !report.added.is_empty() {
                    out.info(&format!(
                        "{} repo(s) would be enrolled on next update",
                        report.added.len()
                    ))?;
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    use crate::config;
}

#[cfg(feature = "wizard")]
pub use impl_wizard::*;
