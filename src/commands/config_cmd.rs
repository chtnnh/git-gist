use crate::cli::{Cli, ConfigAction, EnrollAction};
use crate::config::{self, AutoEnroll, Config};
use crate::config_ops;
use crate::output::OutputCtx;
use anyhow::Result;
use std::io::Write;
use std::process::Command;

pub fn run(action: &ConfigAction, cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let mut display = cfg.clone();
            display.path = None;
            display.local_path = None;
            display.load_warnings.clear();
            let text = toml::to_string_pretty(&display)?;
            if out.is_json() {
                out.write_json(&display)?;
            } else {
                write!(out.stdout(), "{text}")?;
            }
            Ok(())
        }
        ConfigAction::Path { local } => {
            let path = if *local {
                cfg.local_path
                    .clone()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join(".gg.toml"))
            } else {
                cfg.path.clone().unwrap_or(config::global_config_path()?)
            };
            writeln!(out.stdout(), "{}", path.display())?;
            Ok(())
        }
        ConfigAction::Get { key } => {
            let value = config::get_dot_key(cfg, key)?;
            writeln!(out.stdout(), "{value}")?;
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would set {key}={value}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config_ops::set_scalar(&mut updated, key, value)?;
            let path = config_ops::save(&updated)?;
            out.success(&format!("set {key}={value} in {}", path.display()))?;
            Ok(())
        }
        ConfigAction::Edit => {
            let path = cfg.path.clone().unwrap_or(config::global_config_path()?);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if !path.is_file() {
                std::fs::write(&path, "schema_version = 1\n")?;
            }
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "vi".into());
            let status = Command::new(&editor).arg(&path).status()?;
            if !status.success() {
                anyhow::bail!("editor {editor} exited with {status}");
            }
            out.success(&format!("edited {}", path.display()))?;
            Ok(())
        }
        ConfigAction::Wizard => crate::interactive::hub(cli, cfg, out),
        ConfigAction::Ui => crate::interactive::ui_hub(cli, cfg, out),
        ConfigAction::Enroll { action } => run_enroll(action, cli, cfg, out),
    }
}

fn run_enroll(action: &EnrollAction, cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    match action {
        EnrollAction::List => {
            if out.is_json() {
                out.write_json(&cfg.auto_enroll)?;
            } else if cfg.auto_enroll.is_empty() {
                out.info("no [[auto_enroll]] rules")?;
            } else {
                for (i, rule) in cfg.auto_enroll.iter().enumerate() {
                    let prefix = rule
                        .path_prefix
                        .as_deref()
                        .map(|p| format!(" prefix={p}"))
                        .unwrap_or_default();
                    writeln!(
                        out.stdout(),
                        "{i}\t{}\tdepth={}{prefix}\tgroups=[{}]\ttags=[{}]",
                        rule.path.display(),
                        rule.depth,
                        rule.groups.join(", "),
                        rule.tags.join(", ")
                    )?;
                }
            }
            Ok(())
        }
        EnrollAction::Add {
            path,
            depth,
            path_prefix,
            groups,
            tags,
        } => {
            let rule = AutoEnroll {
                path: path.clone(),
                path_prefix: path_prefix.clone(),
                depth: depth.unwrap_or(6),
                groups: groups.clone(),
                tags: tags.clone(),
            };
            if cli.dry_run {
                out.info(&format!(
                    "dry-run: would add auto_enroll {}",
                    rule.path.display()
                ))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config_ops::add_auto_enroll_rule(&mut updated, rule);
            let saved = config_ops::save(&updated)?;
            out.success(&format!("added auto_enroll rule ({})", saved.display()))?;
            Ok(())
        }
        EnrollAction::Remove { index } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would remove auto_enroll[{index}]"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            let removed = config_ops::remove_auto_enroll_rule(&mut updated, *index)?;
            let saved = config_ops::save(&updated)?;
            out.success(&format!(
                "removed auto_enroll {} ({})",
                removed.path.display(),
                saved.display()
            ))?;
            Ok(())
        }
        EnrollAction::Wizard => crate::interactive::enroll(cli, cfg, out),
        EnrollAction::Ui => crate::interactive::ui_focused(cli, cfg, out, crate::tui::Area::Enroll),
    }
}
