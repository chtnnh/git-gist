use crate::cli::{Cli, ConfigAction};
use crate::config::{self, Config};
use crate::output::OutputCtx;
use anyhow::Result;
use std::io::Write;

pub fn run(action: &ConfigAction, cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let mut display = cfg.clone();
            display.path = None;
            display.local_path = None;
            let text = toml::to_string_pretty(&display)?;
            if out.is_json() {
                out.write_json(&display)?;
            } else {
                write!(out.stdout(), "{text}")?;
            }
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
        }
        ConfigAction::Get { key } => {
            let value = config::get_dot_key(cfg, key)?;
            writeln!(out.stdout(), "{value}")?;
        }
        ConfigAction::Set { key, value } => {
            if cli.dry_run {
                out.info(&format!("dry-run: would set {key}={value}"))?;
                return Ok(());
            }
            let mut updated = cfg.clone();
            config::set_dot_key(&mut updated, key, value)?;
            let path = config::save_global(&updated)?;
            out.success(&format!("set {key}={value} in {}", path.display()))?;
        }
    }
    Ok(())
}
