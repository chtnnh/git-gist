//! Parallel execution of git / shell commands across repos.

use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::Repo;
use anyhow::Result;
use rayon::prelude::*;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub struct RunResult {
    pub repo: Repo,
    pub success: bool,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
}

pub fn passthrough(
    repos: &[Repo],
    args: &[String],
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("no git arguments provided");
    }
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    run_git(repos, &argv, cli, cfg, out)
}

pub fn run_git(
    repos: &[Repo],
    args: &[&str],
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    if repos.is_empty() {
        out.warn("no repositories selected")?;
        return Ok(());
    }

    if cli.dry_run {
        for repo in repos {
            out.repo_header(&repo.name, &repo.display_path())?;
            writeln!(
                out.stdout(),
                "dry-run: git {}  (in {})",
                args.join(" "),
                repo.display_path()
            )?;
        }
        return Ok(());
    }

    let jobs = cfg.jobs.unwrap_or_else(num_cpus::get).max(1);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?;

    let stop = Arc::new(AtomicBool::new(false));
    let failures = Arc::new(AtomicUsize::new(0));

    let results: Vec<RunResult> = pool.install(|| {
        repos
            .par_iter()
            .filter_map(|repo| {
                if cli.fail_fast && stop.load(Ordering::Relaxed) {
                    return None;
                }
                let start = Instant::now();
                let output = Command::new("git")
                    .args(args)
                    .current_dir(&repo.path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output();

                let (success, code, stdout, stderr) = match output {
                    Ok(o) => (
                        o.status.success(),
                        o.status.code().unwrap_or(1),
                        String::from_utf8_lossy(&o.stdout).into_owned(),
                        String::from_utf8_lossy(&o.stderr).into_owned(),
                    ),
                    Err(e) => (false, 127, String::new(), e.to_string()),
                };

                if !success {
                    failures.fetch_add(1, Ordering::Relaxed);
                    if cli.fail_fast {
                        stop.store(true, Ordering::Relaxed);
                    }
                }

                Some(RunResult {
                    repo: repo.clone(),
                    success,
                    code,
                    stdout,
                    stderr,
                    duration_ms: start.elapsed().as_millis(),
                })
            })
            .collect()
    });

    // Stable order by path
    let mut results = results;
    results.sort_by(|a, b| a.repo.path.cmp(&b.repo.path));

    if out.is_json() {
        let payload: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "repo": r.repo.display_path(),
                    "name": r.repo.name,
                    "success": r.success,
                    "code": r.code,
                    "stdout": r.stdout,
                    "stderr": r.stderr,
                    "duration_ms": r.duration_ms,
                })
            })
            .collect();
        out.write_json(&payload)?;
    } else {
        for r in &results {
            if cli.quiet && r.success {
                continue;
            }
            out.repo_header(&r.repo.name, &r.repo.display_path())?;
            if !r.stdout.is_empty() {
                write!(out.stdout(), "{}", r.stdout)?;
                if !r.stdout.ends_with('\n') {
                    writeln!(out.stdout())?;
                }
            }
            if !r.stderr.is_empty() {
                write!(out.stderr(), "{}", r.stderr)?;
                if !r.stderr.ends_with('\n') {
                    writeln!(out.stderr())?;
                }
            }
            if cli.timing || cli.verbose > 0 {
                writeln!(
                    out.stdout(),
                    "  [{}] {}ms",
                    if r.success { "ok" } else { "fail" },
                    r.duration_ms
                )?;
            }
        }
    }

    let n_fail = failures.load(Ordering::Relaxed);
    if n_fail > 0 {
        anyhow::bail!("{n_fail} of {} repositories failed", results.len());
    }
    Ok(())
}

pub fn run_shell(
    repos: &[Repo],
    command: &[String],
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("empty command");
    }
    if repos.is_empty() {
        out.warn("no repositories selected")?;
        return Ok(());
    }

    let cmd_display = command.join(" ");
    if cli.dry_run {
        for repo in repos {
            out.repo_header(&repo.name, &repo.display_path())?;
            writeln!(out.stdout(), "dry-run: sh -c {cmd_display:?}")?;
        }
        return Ok(());
    }

    let jobs = cfg.jobs.unwrap_or_else(num_cpus::get).max(1);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?;
    let script = cmd_display.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let failures = Arc::new(AtomicUsize::new(0));

    let results: Vec<RunResult> = pool.install(|| {
        repos
            .par_iter()
            .filter_map(|repo| {
                if cli.fail_fast && stop.load(Ordering::Relaxed) {
                    return None;
                }
                let start = Instant::now();
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(&script)
                    .current_dir(&repo.path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output();
                let (success, code, stdout, stderr) = match output {
                    Ok(o) => (
                        o.status.success(),
                        o.status.code().unwrap_or(1),
                        String::from_utf8_lossy(&o.stdout).into_owned(),
                        String::from_utf8_lossy(&o.stderr).into_owned(),
                    ),
                    Err(e) => (false, 127, String::new(), e.to_string()),
                };
                if !success {
                    failures.fetch_add(1, Ordering::Relaxed);
                    if cli.fail_fast {
                        stop.store(true, Ordering::Relaxed);
                    }
                }
                Some(RunResult {
                    repo: repo.clone(),
                    success,
                    code,
                    stdout,
                    stderr,
                    duration_ms: start.elapsed().as_millis(),
                })
            })
            .collect()
    });

    let mut results = results;
    results.sort_by(|a, b| a.repo.path.cmp(&b.repo.path));

    if out.is_json() {
        let payload: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "repo": r.repo.display_path(),
                    "success": r.success,
                    "code": r.code,
                    "stdout": r.stdout,
                    "stderr": r.stderr,
                    "duration_ms": r.duration_ms,
                })
            })
            .collect();
        out.write_json(&payload)?;
    } else {
        for r in &results {
            out.repo_header(&r.repo.name, &r.repo.display_path())?;
            if !r.stdout.is_empty() {
                print!("{}", r.stdout);
                if !r.stdout.ends_with('\n') {
                    println!();
                }
            }
            if !r.stderr.is_empty() {
                eprint!("{}", r.stderr);
            }
            if cli.timing {
                writeln!(out.stdout(), "  {}ms", r.duration_ms)?;
            }
        }
    }

    let n_fail = failures.load(Ordering::Relaxed);
    if n_fail > 0 {
        anyhow::bail!("{n_fail} of {} repositories failed", results.len());
    }
    Ok(())
}
