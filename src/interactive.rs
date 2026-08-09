//! Interactive config entry points (wizard / TUI).
//!
//! Under `cfg(coverage)` these return immediately so the prompt/event loops
//! (excluded from the coverage denominator) do not block the test suite.

use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::{tui, wizard};
use anyhow::Result;

#[cfg(coverage)]
fn skipped(out: &mut OutputCtx) -> Result<()> {
    out.info("interactive UI skipped under coverage")?;
    Ok(())
}

pub fn hub(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    #[cfg(coverage)]
    {
        let _ = (cli, cfg);
        return skipped(out);
    }
    #[cfg(not(coverage))]
    wizard::run_hub(cli, cfg, out)
}

pub fn ui_hub(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    #[cfg(coverage)]
    {
        let _ = (cli, cfg);
        return skipped(out);
    }
    #[cfg(not(coverage))]
    tui::run_hub(cli, cfg, out)
}

pub fn aliases(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    #[cfg(coverage)]
    {
        let _ = (cli, cfg);
        return skipped(out);
    }
    #[cfg(not(coverage))]
    wizard::run_aliases(cli, cfg, out)
}

pub fn groups(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    #[cfg(coverage)]
    {
        let _ = (cli, cfg);
        return skipped(out);
    }
    #[cfg(not(coverage))]
    wizard::run_groups(cli, cfg, out)
}

pub fn tags(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    #[cfg(coverage)]
    {
        let _ = (cli, cfg);
        return skipped(out);
    }
    #[cfg(not(coverage))]
    wizard::run_tags(cli, cfg, out)
}

pub fn remotes(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    #[cfg(coverage)]
    {
        let _ = (cli, cfg);
        return skipped(out);
    }
    #[cfg(not(coverage))]
    wizard::run_remotes(cli, cfg, out)
}

pub fn enroll(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    #[cfg(coverage)]
    {
        let _ = (cli, cfg);
        return skipped(out);
    }
    #[cfg(not(coverage))]
    wizard::run_enroll(cli, cfg, out)
}

pub fn ui_focused(cli: &Cli, cfg: &Config, out: &mut OutputCtx, area: tui::Area) -> Result<()> {
    #[cfg(coverage)]
    {
        let _ = (cli, cfg, area);
        return skipped(out);
    }
    #[cfg(not(coverage))]
    tui::run_focused(cli, cfg, out, area)
}
