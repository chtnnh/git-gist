use anyhow::{bail, Context, Result};
use clap::CommandFactory;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn run(output: Option<&Path>) -> Result<()> {
    let mut cmd = crate::cli::Cli::command();
    cmd = cmd.disable_help_subcommand(true);
    cmd.build();

    match output {
        None => {
            let man = clap_mangen::Man::new(cmd);
            let mut buffer = Vec::new();
            man.render(&mut buffer)?;
            io::stdout().write_all(&buffer)?;
            Ok(())
        }
        Some(path) => {
            let dir = resolve_output_dir(path)?;
            fs::create_dir_all(&dir)
                .with_context(|| format!("creating man output directory {}", dir.display()))?;
            let written = write_all_man_pages(cmd, &dir)?;
            for p in &written {
                eprintln!("wrote {}", p.display());
            }
            eprintln!(
                "wrote {} man page(s) under {} (root + nested subcommands)",
                written.len(),
                dir.display()
            );
            Ok(())
        }
    }
}

/// Directory that should receive `gg.1`, `gg-alias.1`, `gg-config-enroll.1`, …
///
/// - Directory path (existing, trailing slash, or no man-section extension) → use as-is
/// - File path like `…/gg.1` → use the parent directory (and still emit siblings)
fn resolve_output_dir(path: &Path) -> Result<PathBuf> {
    if path.is_dir()
        || path
            .to_str()
            .is_some_and(|s| s.ends_with('/') || s.ends_with('\\'))
    {
        return Ok(path.to_path_buf());
    }
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if looks_like_man_filename(name) {
        return match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => Ok(parent.to_path_buf()),
            Some(_) => Ok(PathBuf::from(".")),
            None => bail!("invalid man output path: {}", path.display()),
        };
    }
    // Non-existent path without a man extension — treat as the output directory.
    Ok(path.to_path_buf())
}

fn looks_like_man_filename(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| !ext.is_empty() && ext.chars().all(|c| c.is_ascii_digit()))
}

fn write_all_man_pages(cmd: clap::Command, dir: &Path) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    write_tree(cmd, None, dir, &mut written)?;
    Ok(written)
}

fn write_tree(
    cmd: clap::Command,
    parent_display: Option<&str>,
    dir: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<()> {
    let display = match parent_display {
        None => cmd.get_name().to_string(),
        Some(parent) => format!("{parent}-{}", cmd.get_name()),
    };

    for sub in cmd
        .get_subcommands()
        .filter(|s| !s.is_hide_set())
        .cloned()
        .collect::<Vec<_>>()
    {
        write_tree(sub, Some(&display), dir, written)?;
    }

    let page = cmd.display_name(&display);
    let path = clap_mangen::Man::new(page)
        .generate_to(dir)
        .with_context(|| format!("writing man page for {display}"))?;
    written.push(path);
    Ok(())
}
