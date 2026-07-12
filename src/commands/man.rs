use anyhow::Result;
use clap::CommandFactory;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn run(output: Option<&Path>) -> Result<()> {
    let cmd = crate::cli::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer = Vec::new();
    man.render(&mut buffer)?;

    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &buffer)?;
        eprintln!("wrote {}", path.display());
    } else {
        io::stdout().write_all(&buffer)?;
    }
    Ok(())
}
