use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::{ProbeOpts, Repo};
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;
use std::io::Write;
use which::which;

#[derive(Serialize)]
struct DoctorFinding {
    level: String,
    repo: Option<String>,
    message: String,
}

pub fn run(repos: &[Repo], _cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    let mut findings = Vec::new();

    if which("git").is_err() {
        // Unreachable in normal CI/dev where git is required to build/test.
        #[cfg(not(coverage))]
        findings.push(DoctorFinding {
            level: "error".into(),
            repo: None,
            message: "git not found on PATH".into(),
        });
    } else if let Ok(output) = crate::repo::git_command().arg("--version").output() {
        let v = String::from_utf8_lossy(&output.stdout).trim().to_string();
        findings.push(DoctorFinding {
            level: "info".into(),
            repo: None,
            message: format!("found {v}"),
        });
    }

    let pool = crate::exec::job_pool(cfg)?;
    let mut repo_findings: Vec<DoctorFinding> = pool.install(|| {
        repos
            .par_iter()
            .flat_map(|repo| {
                let mut local = Vec::new();
                let git = repo.path.join(".git");
                if git.is_file() {
                    local.push(DoctorFinding {
                        level: "info".into(),
                        repo: Some(repo.name.clone()),
                        message: "gitfile (.git file) — likely worktree or submodule".into(),
                    });
                }
                match crate::repo::probe_with(&repo.path, ProbeOpts::DOCTOR) {
                    Ok(status) => {
                        if status.detached {
                            local.push(DoctorFinding {
                                level: "warn".into(),
                                repo: Some(repo.name.clone()),
                                message: format!("detached HEAD at {}", status.branch),
                            });
                        }
                        if let Some(op) = status.in_progress {
                            local.push(DoctorFinding {
                                level: "warn".into(),
                                repo: Some(repo.name.clone()),
                                message: format!("{op} in progress"),
                            });
                        }
                        if status.upstream.is_none() && !status.detached {
                            local.push(DoctorFinding {
                                level: "info".into(),
                                repo: Some(repo.name.clone()),
                                message: "no upstream configured".into(),
                            });
                        }
                    }
                    Err(e) => local.push(DoctorFinding {
                        level: "error".into(),
                        repo: Some(repo.name.clone()),
                        message: format!("probe failed: {e}"),
                    }),
                }
                local
            })
            .collect()
    });
    findings.append(&mut repo_findings);

    if out.is_json() {
        out.write_json(&findings)?;
        return Ok(());
    }

    for f in &findings {
        let prefix = match f.level.as_str() {
            "error" => "error",
            "warn" => "warn",
            _ => "info",
        };
        if let Some(repo) = &f.repo {
            writeln!(out.stdout(), "[{prefix}] {repo}: {}", f.message)?;
        } else {
            writeln!(out.stdout(), "[{prefix}] {}", f.message)?;
        }
    }
    out.info(&format!(
        "checked {} repositories, {} findings",
        repos.len(),
        findings.len()
    ))?;
    Ok(())
}
