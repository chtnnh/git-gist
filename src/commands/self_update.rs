use crate::output::OutputCtx;
use anyhow::Result;
use std::io::Write;

pub fn run(out: &mut OutputCtx) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let repo = env!("CARGO_PKG_REPOSITORY");
    out.info(&format!("gg {version}"))?;
    out.info("self-update checks GitHub releases for a newer binary.")?;
    out.info(&format!("Releases: {repo}/releases"))?;

    // Best-effort: print instructions rather than silently downloading unsigned binaries
    // without explicit user confirmation in a future interactive flow.
    writeln!(
        out.stdout(),
        "\nTo update manually:\n  cargo install --git {repo} --locked\n\
         Or install from your package manager / GitHub release assets.\n\n\
         Automated binary download is intentionally conservative; prefer brew/deb/rpm\n\
         or cargo-dist installers when available."
    )?;

    // Optional `gh` enrichment — excluded from coverage builds (needs network + gh).
    #[cfg(not(coverage))]
    {
        if which::which("gh").is_ok() {
            let output = std::process::Command::new("gh")
                .args(["release", "view", "--json", "tagName,url", "-R"])
                .arg(repo.trim_start_matches("https://github.com/"))
                .output();
            if let Ok(o) = output {
                if o.status.success() {
                    let text = String::from_utf8_lossy(&o.stdout);
                    writeln!(out.stdout(), "Latest release info:\n{text}")?;
                }
            }
        }
    }

    Ok(())
}
