use crate::cli::{Cli, TagAction, TagMemberAction};
use crate::config::Config;
use crate::config_ops;
use crate::output::OutputCtx;
use anyhow::Result;
use std::io::Write;

pub fn run(action: &TagAction, cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    match action {
        TagAction::List => {
            if out.is_json() {
                out.write_json(&cfg.tags)?;
            } else if cfg.tags.is_empty() {
                out.info("no tags configured")?;
            } else {
                for (name, members) in &cfg.tags {
                    writeln!(out.stdout(), "{name}\t{}", members.join(", "))?;
                }
            }
            Ok(())
        }
        TagAction::Add { name, members } => {
            if cli.dry_run {
                out.info(&format!(
                    "dry-run: would set tag {name} = [{}]",
                    members.join(", ")
                ))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config_ops::set_tag_members(&mut updated, name, members.clone());
            let saved = config_ops::save(&updated)?;
            out.success(&format!(
                "tag {name} = [{}] ({})",
                members.join(", "),
                saved.display()
            ))?;
            Ok(())
        }
        TagAction::Remove { name } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would remove tag {name}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config_ops::remove_tag(&mut updated, name)?;
            let saved = config_ops::save(&updated)?;
            out.success(&format!("removed tag {name} ({})", saved.display()))?;
            Ok(())
        }
        TagAction::Member { action } => match action {
            TagMemberAction::Add { tag, members } => {
                if cli.dry_run {
                    out.info(&format!(
                        "dry-run: would add to tag {tag}: {}",
                        members.join(", ")
                    ))?;
                    return Ok(());
                }
                let mut updated = cfg.clone();
                for m in members {
                    config_ops::add_tag_member(&mut updated, tag, m)?;
                }
                let saved = config_ops::save(&updated)?;
                out.success(&format!(
                    "added {} member(s) to tag {tag} ({})",
                    members.len(),
                    saved.display()
                ))?;
                Ok(())
            }
            TagMemberAction::Remove { tag, members } => {
                if cli.dry_run {
                    out.info(&format!(
                        "dry-run: would remove from tag {tag}: {}",
                        members.join(", ")
                    ))?;
                    return Ok(());
                }
                let mut updated = cfg.clone();
                for m in members {
                    config_ops::remove_tag_member(&mut updated, tag, m)?;
                }
                let saved = config_ops::save(&updated)?;
                out.success(&format!(
                    "removed {} member(s) from tag {tag} ({})",
                    members.len(),
                    saved.display()
                ))?;
                Ok(())
            }
        },
        TagAction::Wizard => crate::interactive::tags(cli, cfg, out),
        TagAction::Ui => crate::interactive::ui_focused(cli, cfg, out, crate::tui::Area::Tags),
    }
}
