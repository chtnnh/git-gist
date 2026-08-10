//! Parallel execution of git / shell commands across repos.

use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::Repo;
use anyhow::Result;
use rayon::prelude::*;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub struct RunResult {
    pub repo: Repo,
    pub success: bool,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
    /// True when `--fail-fast` stopped scheduling and this repo never ran.
    pub skipped: bool,
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
    if let Some(hint) = misplaced_global_flag_hint(args) {
        anyhow::bail!("{hint}");
    }
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    run_git(repos, &argv, cli, cfg, out)
}

fn misplaced_global_flag_hint(args: &[String]) -> Option<String> {
    // External subcommands swallow trailing globals into git argv. Detect the
    // common footgun: `gg status --dry-run` instead of `gg --dry-run status`.
    const GLOBALS: &[&str] = &[
        "--dry-run",
        "--fail-fast",
        "--timing",
        "--refresh",
        "--only-dirty",
        "--only-clean",
        "--only-ahead",
        "--only-behind",
        "--only-stashed",
        "--only-detached",
        "--include-submodules",
        "--root",
        "--depth",
        "--format",
        "--theme",
        "--show-path",
        "--color",
        "--in",
        "--exclude",
        "--group",
        "--tag",
        "-j",
        "-q",
        "-i",
        "-x",
        "-g",
    ];
    for (idx, arg) in args.iter().enumerate().skip(1) {
        let name = arg.split('=').next().unwrap_or(arg);
        if GLOBALS.contains(&name) {
            return Some(format!(
                "global flag `{name}` was passed after the git verb and would be forwarded to git; \
                 put it before the verb (e.g. `gg {name} {} …`)",
                args[0]
            ));
        }
        // Also catch `--root DIR` style when the flag alone matches and next is a value.
        let _ = idx;
    }
    None
}

/// Build a rayon pool sized from `--jobs` / config.
pub fn job_pool(cfg: &Config) -> Result<rayon::ThreadPool> {
    let jobs = cfg.jobs.unwrap_or_else(num_cpus::get).max(1);
    Ok(rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?)
}

/// Run `git` in each repo without writing to `OutputCtx`.
/// Used by callers (e.g. `sync`) that need per-repo results without JSON side effects.
pub fn run_git_inner(
    repos: &[Repo],
    args: &[&str],
    cli: &Cli,
    cfg: &Config,
) -> Result<Vec<RunResult>> {
    if repos.is_empty() {
        return Ok(Vec::new());
    }

    let pool = job_pool(cfg)?;
    let stop = Arc::new(AtomicBool::new(false));

    let results: Vec<RunResult> = pool.install(|| {
        repos
            .par_iter()
            .map(|repo| {
                if cli.fail_fast && stop.load(Ordering::Relaxed) {
                    return RunResult {
                        repo: repo.clone(),
                        success: false,
                        code: 130,
                        stdout: String::new(),
                        stderr: "skipped (--fail-fast)".into(),
                        duration_ms: 0,
                        skipped: true,
                    };
                }
                let start = Instant::now();
                let output = crate::repo::git_command()
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

                if !success && cli.fail_fast {
                    stop.store(true, Ordering::Relaxed);
                }

                RunResult {
                    repo: repo.clone(),
                    success,
                    code,
                    stdout,
                    stderr,
                    duration_ms: start.elapsed().as_millis(),
                    skipped: false,
                }
            })
            .collect()
    });

    let mut results = results;
    results.sort_by(|a, b| a.repo.path.cmp(&b.repo.path));
    Ok(results)
}

fn report_run_failures(results: &[RunResult], total: usize) -> Result<()> {
    let n_fail = results.iter().filter(|r| !r.success && !r.skipped).count();
    let n_skip = results.iter().filter(|r| r.skipped).count();
    if n_fail == 0 && n_skip == 0 {
        return Ok(());
    }
    if n_skip > 0 {
        anyhow::bail!("{n_fail} of {total} repositories failed ({n_skip} skipped)");
    }
    anyhow::bail!("{n_fail} of {total} repositories failed");
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

    let results = run_git_inner(repos, args, cli, cfg)?;

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
                    "skipped": r.skipped,
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
                    if r.skipped {
                        "skip"
                    } else if r.success {
                        "ok"
                    } else {
                        "fail"
                    },
                    r.duration_ms
                )?;
            }
        }
    }

    report_run_failures(&results, repos.len())
}

/// Resolve Windows shell executable from `COMSPEC`, falling back when unset/blank.
pub fn resolve_windows_shell(comspec: Option<&str>) -> String {
    comspec
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("cmd.exe")
        .to_string()
}

/// Platform shell for `gg each`: `sh -c` on Unix, `COMSPEC /C` (or `cmd.exe`) on Windows.
/// POSIX-only scripts on Windows need Git Bash or WSL.
pub fn shell_for_each() -> (String, String) {
    #[cfg(windows)]
    {
        let program = resolve_windows_shell(std::env::var("COMSPEC").ok().as_deref());
        (program, "/C".into())
    }
    #[cfg(not(windows))]
    {
        ("sh".into(), "-c".into())
    }
}

fn shell_display(program: &str, flag: &str) -> String {
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    format!("{base} {flag}")
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

    let (shell_prog, shell_flag) = shell_for_each();
    let cmd_display = command.join(" ");
    let shell_label = shell_display(&shell_prog, &shell_flag);
    if cli.dry_run {
        for repo in repos {
            out.repo_header(&repo.name, &repo.display_path())?;
            writeln!(out.stdout(), "dry-run: {shell_label} {cmd_display:?}")?;
        }
        return Ok(());
    }

    let pool = job_pool(cfg)?;
    let script = cmd_display.clone();
    let stop = Arc::new(AtomicBool::new(false));

    let results: Vec<RunResult> = pool.install(|| {
        repos
            .par_iter()
            .map(|repo| {
                if cli.fail_fast && stop.load(Ordering::Relaxed) {
                    return RunResult {
                        repo: repo.clone(),
                        success: false,
                        code: 130,
                        stdout: String::new(),
                        stderr: "skipped (--fail-fast)".into(),
                        duration_ms: 0,
                        skipped: true,
                    };
                }
                let start = Instant::now();
                let output = Command::new(&shell_prog)
                    .arg(&shell_flag)
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
                    Err(e) => {
                        #[cfg(windows)]
                        let hint =
                            format!("{e}; set COMSPEC or use Git Bash/WSL for POSIX shell syntax");
                        #[cfg(not(windows))]
                        let hint = e.to_string();
                        (false, 127, String::new(), hint)
                    }
                };
                if !success && cli.fail_fast {
                    stop.store(true, Ordering::Relaxed);
                }
                RunResult {
                    repo: repo.clone(),
                    success,
                    code,
                    stdout,
                    stderr,
                    duration_ms: start.elapsed().as_millis(),
                    skipped: false,
                }
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
                    "skipped": r.skipped,
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
                writeln!(out.stdout(), "  {}ms", r.duration_ms)?;
            }
        }
    }

    report_run_failures(&results, repos.len())
}
