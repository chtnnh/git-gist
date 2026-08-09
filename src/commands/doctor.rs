use crate::cli::Cli;
use crate::config::{self, Config};
use crate::config_ops;
use crate::output::OutputCtx;
use crate::repo::{ProbeOpts, Repo};
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use which::which;

#[derive(Clone, Serialize)]
struct DoctorFinding {
    level: String,
    repo: Option<String>,
    message: String,
}

pub fn run(repos: &[Repo], _cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    let mut findings = Vec::new();

    if which("git").is_err() {
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
    let show_path = out.show_path;
    let root = out.root.clone();
    let mut repo_findings: Vec<DoctorFinding> = pool.install(|| {
        repos
            .par_iter()
            .flat_map(|repo| {
                let label = repo.label(show_path, root.as_deref());
                let mut local = Vec::new();
                let git = repo.path.join(".git");
                if git.is_file() {
                    local.push(DoctorFinding {
                        level: "info".into(),
                        repo: Some(label.clone()),
                        message: "gitfile (.git file) — likely worktree or submodule".into(),
                    });
                }
                match crate::repo::probe_with(&repo.path, ProbeOpts::DOCTOR) {
                    Ok(status) => {
                        if status.detached {
                            local.push(DoctorFinding {
                                level: "warn".into(),
                                repo: Some(label.clone()),
                                message: format!("detached HEAD at {}", status.branch),
                            });
                        }
                        if let Some(op) = status.in_progress {
                            local.push(DoctorFinding {
                                level: "warn".into(),
                                repo: Some(label.clone()),
                                message: format!("{op} in progress"),
                            });
                        }
                        if status.upstream.is_none() && !status.detached {
                            local.push(DoctorFinding {
                                level: "info".into(),
                                repo: Some(label.clone()),
                                message: "no upstream configured".into(),
                            });
                        }
                    }
                    Err(e) => local.push(DoctorFinding {
                        level: "error".into(),
                        repo: Some(label),
                        message: format!("probe failed: {e}"),
                    }),
                }
                local
            })
            .collect()
    });
    findings.append(&mut repo_findings);

    emit_findings(&findings, repos.len(), out)
}

pub fn run_config(cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    let mut findings = Vec::new();

    let path = cfg.path.clone().unwrap_or(config::global_config_path()?);
    findings.push(DoctorFinding {
        level: "info".into(),
        repo: None,
        message: format!("config path: {}", path.display()),
    });

    for legacy in config::legacy_global_config_paths() {
        if legacy.is_file() && legacy != path {
            findings.push(DoctorFinding {
                level: "info".into(),
                repo: None,
                message: format!("legacy config still present: {}", legacy.display()),
            });
        }
    }

    for w in &cfg.load_warnings {
        findings.push(DoctorFinding {
            level: "warn".into(),
            repo: None,
            message: w.clone(),
        });
    }

    if cfg.auto_enroll.is_empty() {
        findings.push(DoctorFinding {
            level: "warn".into(),
            repo: None,
            message:
                "no [[auto_enroll]] rules — run `gg config enroll wizard` or `gg config wizard`"
                    .into(),
        });
    }

    for (i, rule) in cfg.auto_enroll.iter().enumerate() {
        if !rule.path.is_dir() {
            findings.push(DoctorFinding {
                level: "warn".into(),
                repo: None,
                message: format!(
                    "auto_enroll[{i}] watch path missing: {}",
                    rule.path.display()
                ),
            });
        }
        if let Some(root) = &cfg.root {
            let root_c = root.canonicalize().unwrap_or_else(|_| root.clone());
            let rule_c = rule
                .path
                .canonicalize()
                .unwrap_or_else(|_| rule.path.clone());
            if root_c == rule_c
                && (!rule.groups.is_empty() || !rule.tags.is_empty())
                && rule
                    .path_prefix
                    .as_ref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
            {
                findings.push(DoctorFinding {
                    level: "warn".into(),
                    repo: None,
                    message: format!(
                        "auto_enroll[{i}] path equals config root with groups/tags and no path_prefix"
                    ),
                });
            }
        }
    }

    let stale = config_ops::list_stale_aliases(cfg);
    if !stale.is_empty() {
        findings.push(DoctorFinding {
            level: "warn".into(),
            repo: None,
            message: format!(
                "{} stale alias(es) — run `gg alias prune` or `gg alias wizard` to reclaim short names",
                stale.len()
            ),
        });
        for (name, p) in stale.iter().take(10) {
            findings.push(DoctorFinding {
                level: "info".into(),
                repo: None,
                message: format!("stale alias {name} → {}", p.display()),
            });
        }
    }

    for (group, members) in &cfg.groups {
        for m in members {
            if !cfg.aliases.contains_key(m) {
                findings.push(DoctorFinding {
                    level: "warn".into(),
                    repo: None,
                    message: format!("group `{group}` references missing alias `{m}`"),
                });
            }
        }
    }
    for (tag, members) in &cfg.tags {
        for m in members {
            if !cfg.aliases.contains_key(m) {
                findings.push(DoctorFinding {
                    level: "warn".into(),
                    repo: None,
                    message: format!("tag `{tag}` references missing alias `{m}`"),
                });
            }
        }
    }

    let mut basename_counts: HashMap<String, Vec<String>> = HashMap::new();
    for (name, path) in &cfg.aliases {
        let base = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.clone());
        basename_counts.entry(base).or_default().push(name.clone());
    }
    for (base, names) in basename_counts {
        if names.len() > 1 {
            findings.push(DoctorFinding {
                level: "info".into(),
                repo: None,
                message: format!(
                    "duplicate basename `{base}` across aliases: {}",
                    names.join(", ")
                ),
            });
        }
    }

    emit_findings(&findings, 0, out)
}

fn emit_findings(
    findings: &[DoctorFinding],
    repos_checked: usize,
    out: &mut OutputCtx,
) -> Result<()> {
    if out.is_json() {
        out.write_json(&findings.to_vec())?;
        return Ok(());
    }

    for f in findings {
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
    if repos_checked > 0 {
        out.info(&format!(
            "checked {} repositories, {} findings",
            repos_checked,
            findings.len()
        ))?;
    } else {
        out.info(&format!("{} findings", findings.len()))?;
    }
    Ok(())
}
