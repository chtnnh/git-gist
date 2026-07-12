use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::Repo;
use anyhow::Result;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct ListRow {
    name: String,
    path: String,
}

pub fn run(
    repos: &[Repo],
    _refresh: bool,
    _cli: &Cli,
    _cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    // Discovery refresh is handled in `discover::select_repos` when
    // `List { refresh: true }` or `--refresh` is set. This command only renders
    // the already-filtered selection.
    let rows: Vec<ListRow> = repos
        .iter()
        .map(|r| ListRow {
            name: r.name.clone(),
            path: r.display_path(),
        })
        .collect();

    if out.is_json() {
        out.write_json(&rows)?;
        return Ok(());
    }

    for r in &rows {
        writeln!(out.stdout(), "{}\t{}", r.name, r.path)?;
    }
    out.info(&format!("{} repositories", rows.len()))?;
    Ok(())
}
