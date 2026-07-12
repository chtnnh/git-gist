//! git-gist library — multi-repo git CLI core.

pub mod cli;
pub mod commands;
pub mod config;
pub mod discover;
pub mod exec;
pub mod filters;
pub mod output;
pub mod repo;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::{Cli, ColorChoice, Commands};
use output::{OutputCtx, Theme};

/// Entry point used by the `gg` binary and integration harnesses.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    run_cli(cli)
}

/// Run with an already-parsed CLI (useful for tests).
pub fn run_cli(cli: Cli) -> Result<()> {
    let color = resolve_color(cli.color);
    let mut out = OutputCtx::new(color, cli.format, cli.quiet, cli.verbose);

    match &cli.command {
        Some(Commands::Completions { shell }) => {
            return commands::completions::run(*shell);
        }
        Some(Commands::Man { output }) => {
            return commands::man::run(output.as_deref());
        }
        Some(Commands::Version) => {
            println!("gg {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    let cfg = config::load(&cli)?;
    if let Some(theme) = cfg.theme.as_deref().or(cli.theme.as_deref()) {
        out = out.with_theme(Theme::parse(theme));
    }

    if matches!(cli.command, Some(Commands::Update)) {
        return commands::update::run(&cli, &cfg, &mut out);
    }

    let selection = discover::select_repos(&cli, &cfg)?;

    match &cli.command {
        Some(Commands::Overview) => commands::overview::run(&selection, &cli, &cfg, &mut out),
        Some(Commands::List { refresh }) => {
            commands::list::run(&selection, *refresh, &cli, &cfg, &mut out)
        }
        Some(Commands::Info { path }) => {
            commands::info::run(&selection, path.as_deref(), &cli, &cfg, &mut out)
        }
        Some(Commands::Commits { number }) => {
            commands::commits::run(&selection, *number, &cli, &cfg, &mut out)
        }
        Some(Commands::Worktrees) => commands::worktrees::run(&selection, &cli, &cfg, &mut out),
        Some(Commands::Doctor) => commands::doctor::run(&selection, &cli, &cfg, &mut out),
        Some(Commands::Each { command }) => {
            commands::each::run(&selection, command, &cli, &cfg, &mut out)
        }
        Some(Commands::Config { action }) => {
            commands::config_cmd::run(action, &cli, &cfg, &mut out)
        }
        Some(Commands::Alias { action }) => commands::alias::run(action, &cli, &cfg, &mut out),
        Some(Commands::Group { action }) => commands::group::run(action, &cli, &cfg, &mut out),
        Some(Commands::Init { profile, path }) => {
            commands::scaffold::init(profile.as_deref(), path.as_deref(), &cli, &cfg, &mut out)
        }
        Some(Commands::Scaffold { profile, path }) => {
            commands::scaffold::init(profile.as_deref(), path.as_deref(), &cli, &cfg, &mut out)
        }
        Some(Commands::Hooks { action }) => {
            commands::hooks::run(action, &selection, &cli, &cfg, &mut out)
        }
        Some(Commands::Remotes { action }) => {
            commands::remotes::run(action, &selection, &cli, &cfg, &mut out)
        }
        Some(Commands::Sync { pull }) => {
            commands::sync::run(&selection, *pull, &cli, &cfg, &mut out)
        }
        Some(Commands::Stale { days }) => {
            commands::stale::run(&selection, *days, &cli, &cfg, &mut out)
        }
        Some(Commands::SelfUpdate) => commands::self_update::run(&mut out),
        Some(Commands::Git { args }) | Some(Commands::External(args)) => {
            exec::passthrough(&selection, args, &cli, &cfg, &mut out)
        }
        None => commands::overview::run(&selection, &cli, &cfg, &mut out),
        Some(Commands::Update) => unreachable!("handled before selection"),
        Some(Commands::Completions { .. } | Commands::Man { .. } | Commands::Version) => {
            unreachable!("handled early")
        }
    }
}

pub fn resolve_color(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            if std::env::var_os("NO_COLOR").is_some() {
                return false;
            }
            use std::io::IsTerminal;
            std::io::stdout().is_terminal()
        }
    }
}

pub fn clap_command() -> clap::Command {
    Cli::command()
}
