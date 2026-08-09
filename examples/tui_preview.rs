//! Render a single `gg config ui` frame to stdout (used for docs screenshots).
//!
//! Usage: cargo run --example tui_preview -- /path/to/config.toml

use git_gist::config::Config;
use git_gist::tui::{self, Area};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("usage: tui_preview <config.toml>"))?;
    let text = std::fs::read_to_string(&path)?;
    let mut cfg: Config = toml::from_str(&text)?;
    cfg.path = Some(path);
    let frame = tui::render_preview(&cfg, Some(Area::Aliases), 88, 18)?;
    print!("{frame}");
    Ok(())
}
