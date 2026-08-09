//! git-gist library — multi-repo git CLI core.

pub mod auto_enroll;
pub mod cli;
pub mod commands;
pub mod config;
pub mod config_ops;
pub mod discover;
pub mod exec;
pub mod filters;
pub mod interactive;
pub mod output;
pub mod repo;
pub mod tui;
pub mod wizard;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::{Cli, ColorChoice, Commands, HooksAction, RemotesAction};
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

    let mut cfg = config::load(&cli)?;
    if let Some(theme) = cfg.theme.as_deref().or(cli.theme.as_deref()) {
        out = out.with_theme(Theme::parse(theme));
    }
    out = out.with_show_path(cfg.show_path || cli.show_path, cfg.root.clone());

    // Commands that never use repository selection — skip discovery so global
    // selection flags cannot fail or slow them down.
    if !command_needs_selection(&cli.command) {
        return run_without_selection(&cli, &cfg, &mut out);
    }

    let selection = discover::select_repos(&cli, &mut cfg)?;

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
        Some(Commands::Doctor { config }) => {
            if *config {
                commands::doctor::run_config(&cfg, &mut out)
            } else {
                commands::doctor::run(&selection, &cli, &cfg, &mut out)
            }
        }
        Some(Commands::Each { command }) => {
            commands::each::run(&selection, command, &cli, &cfg, &mut out)
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
        Some(Commands::Git { args }) | Some(Commands::External(args)) => {
            exec::passthrough(&selection, args, &cli, &cfg, &mut out)
        }
        None => commands::overview::run(&selection, &cli, &cfg, &mut out),
        Some(
            Commands::Update { .. }
            | Commands::Config { .. }
            | Commands::Alias { .. }
            | Commands::Group { .. }
            | Commands::Tag { .. }
            | Commands::Init { .. }
            | Commands::Scaffold { .. }
            | Commands::SelfUpdate
            | Commands::Wizard
            | Commands::Ui
            | Commands::Completions { .. }
            | Commands::Man { .. }
            | Commands::Version,
        ) => unreachable!("handled by run_without_selection"),
    }
}

fn command_needs_selection(command: &Option<Commands>) -> bool {
    match command {
        None => true,
        Some(
            Commands::Overview
            | Commands::List { .. }
            | Commands::Info { .. }
            | Commands::Commits { .. }
            | Commands::Worktrees
            | Commands::Doctor { config: false }
            | Commands::Each { .. }
            | Commands::Sync { .. }
            | Commands::Stale { .. }
            | Commands::Git { .. }
            | Commands::External(_),
        ) => true,
        Some(Commands::Hooks {
            action: HooksAction::Install { .. },
        }) => true,
        Some(Commands::Remotes {
            action: RemotesAction::AddTo { .. },
        }) => true,
        Some(_) => false,
    }
}

fn run_without_selection(cli: &Cli, cfg: &config::Config, out: &mut OutputCtx) -> Result<()> {
    match &cli.command {
        Some(Commands::Update {
            prune_stale,
            no_prune_stale,
            ask,
        }) => commands::update::run(cli, cfg, out, *prune_stale, *no_prune_stale, *ask),
        Some(Commands::Config { action: None }) => interactive::hub(cli, cfg, out),
        Some(Commands::Config {
            action: Some(action),
        }) => commands::config_cmd::run(action, cli, cfg, out),
        Some(Commands::Alias { action }) => commands::alias::run(action, cli, cfg, out),
        Some(Commands::Group { action }) => commands::group::run(action, cli, cfg, out),
        Some(Commands::Tag { action }) => commands::tag::run(action, cli, cfg, out),
        Some(Commands::Wizard) => interactive::hub(cli, cfg, out),
        Some(Commands::Ui) => interactive::ui_hub(cli, cfg, out),
        Some(Commands::Init { profile, path }) => {
            commands::scaffold::init(profile.as_deref(), path.as_deref(), cli, cfg, out)
        }
        Some(Commands::Scaffold { profile, path }) => {
            commands::scaffold::init(profile.as_deref(), path.as_deref(), cli, cfg, out)
        }
        Some(Commands::SelfUpdate) => commands::self_update::run(out),
        Some(Commands::Hooks { action }) => commands::hooks::run(action, &[], cli, cfg, out),
        Some(Commands::Remotes { action }) => commands::remotes::run(action, &[], cli, cfg, out),
        Some(Commands::Doctor { config: true }) => commands::doctor::run_config(cfg, out),
        _ => unreachable!("command_needs_selection should be false only for the arms above"),
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
