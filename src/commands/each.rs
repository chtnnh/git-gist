use crate::cli::Cli;
use crate::config::Config;
use crate::exec;
use crate::output::OutputCtx;
use crate::repo::Repo;
use anyhow::Result;

pub fn run(
    repos: &[Repo],
    command: &[String],
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    exec::run_shell(repos, command, cli, cfg, out)
}
