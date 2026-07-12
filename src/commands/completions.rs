use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

pub fn run(shell: Shell) -> Result<()> {
    let mut cmd = crate::cli::Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
    Ok(())
}
