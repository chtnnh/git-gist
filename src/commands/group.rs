use crate::cli::{Cli, GroupAction, GroupMemberAction};
use crate::config::Config;
use crate::config_ops;
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
            Ok(())
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
            config_ops::set_group_members(&mut updated, name, members.clone());
            let saved = config_ops::save(&updated)?;
            out.success(&format!(
                "group {name} = [{}] ({})",
                members.join(", "),
                saved.display()
            ))?;
            Ok(())
        }
        GroupAction::Remove { name } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would remove group {name}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config_ops::remove_group(&mut updated, name)?;
            let saved = config_ops::save(&updated)?;
            out.success(&format!("removed group {name} ({})", saved.display()))?;
            Ok(())
        }
        GroupAction::Member { action } => match action {
            GroupMemberAction::Add { group, members } => {
                if cli.dry_run {
                    out.info(&format!(
                        "dry-run: would add to group {group}: {}",
                        members.join(", ")
                    ))?;
                    return Ok(());
                }
                let mut updated = cfg.clone();
                for m in members {
                    config_ops::add_group_member(&mut updated, group, m)?;
                }
                let saved = config_ops::save(&updated)?;
                out.success(&format!(
                    "added {} member(s) to {group} ({})",
                    members.len(),
                    saved.display()
                ))?;
                Ok(())
            }
            GroupMemberAction::Remove { group, members } => {
                if cli.dry_run {
                    out.info(&format!(
                        "dry-run: would remove from group {group}: {}",
                        members.join(", ")
                    ))?;
                    return Ok(());
                }
                let mut updated = cfg.clone();
                for m in members {
                    config_ops::remove_group_member(&mut updated, group, m)?;
                }
                let saved = config_ops::save(&updated)?;
                out.success(&format!(
                    "removed {} member(s) from {group} ({})",
                    members.len(),
                    saved.display()
                ))?;
                Ok(())
            }
        },
        GroupAction::Prune { name, under } => {
            if cli.dry_run {
                let mut preview = cfg.clone();
                let removed =
                    config_ops::prune_group_members(&mut preview, name, under.as_deref())?;
                out.info(&format!(
                    "dry-run: would prune {} member(s) from {name}",
                    removed.len()
                ))?;
                for m in &removed {
                    writeln!(out.stdout(), "{m}")?;
                }
                return Ok(());
            }
            let mut updated = cfg.clone();
            let removed = config_ops::prune_group_members(&mut updated, name, under.as_deref())?;
            let saved = config_ops::save(&updated)?;
            out.success(&format!(
                "pruned {} member(s) from {name} ({})",
                removed.len(),
                saved.display()
            ))?;
            for m in &removed {
                writeln!(out.stdout(), "{m}")?;
            }
            Ok(())
        }
        GroupAction::Wizard => crate::interactive::groups(cli, cfg, out),
        GroupAction::Ui => crate::interactive::ui_focused(cli, cfg, out, crate::tui::Area::Groups),
    }
}
