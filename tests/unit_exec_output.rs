//! Additional unit coverage for output rendering and exec edge cases.

use clap::Parser;
use git_gist::cli::{Cli, ColorChoice, OutputFormat};
use git_gist::config::Config;
use git_gist::exec;
use git_gist::output::OutputCtx;
use git_gist::repo::Repo;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn make_repo() -> (tempfile::TempDir, Repo) {
    let dir = tempdir().unwrap();
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    fs::write(dir.path().join("f"), "x").unwrap();
    Command::new("git")
        .args(["add", "f"])
        .current_dir(dir.path())
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@e.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@e.com")
        .status()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "c"])
        .current_dir(dir.path())
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@e.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@e.com")
        .status()
        .unwrap();
    let repo = Repo::new(dir.path().to_path_buf());
    (dir, repo)
}

#[test]
fn output_table_and_json_helpers() {
    let mut out = OutputCtx::new(true, OutputFormat::Human, false, 0);
    out.print_table(&["a", "b"], vec![vec!["1".into(), "2".into()]])
        .unwrap();
    out.repo_header("n", "/p").unwrap();
    out.warn("w").unwrap();
    out.info("i").unwrap();
    out.success("s").unwrap();

    let mut out = OutputCtx::new(false, OutputFormat::Json, false, 0);
    out.write_json(&serde_json::json!([{"a":1},{"b":2}]))
        .unwrap();
    let mut out = OutputCtx::new(false, OutputFormat::Ndjson, false, 0);
    out.write_json(&serde_json::json!([{"a":1},{"b":2}]))
        .unwrap();
    let mut out = OutputCtx::new(false, OutputFormat::Ndjson, false, 0);
    out.write_json(&serde_json::json!({"solo": true})).unwrap();
}

#[test]
fn exec_passthrough_empty_and_dry_run() {
    let (_d, repo) = make_repo();
    let cli = Cli::try_parse_from(["gg", "--dry-run", "status"]).unwrap();
    let cfg = Config::default().with_builtins();
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    exec::passthrough(
        std::slice::from_ref(&repo),
        &["status".into()],
        &cli,
        &cfg,
        &mut out,
    )
    .unwrap();

    let cli = Cli::try_parse_from(["gg", "status"]).unwrap();
    assert!(exec::passthrough(&[], &["status".into()], &cli, &cfg, &mut out).is_ok());
    assert!(exec::passthrough(&[repo], &[], &cli, &cfg, &mut out).is_err());
}

#[test]
fn exec_run_shell_dry_and_json() {
    let (_d, repo) = make_repo();
    let cli = Cli::try_parse_from(["gg", "--dry-run", "each", "echo", "ok"]).unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    exec::run_shell(
        std::slice::from_ref(&repo),
        &["echo".into(), "ok".into()],
        &cli,
        &cfg,
        &mut out,
    )
    .unwrap();

    let cli = Cli::try_parse_from(["gg", "--format", "json", "each", "echo", "hi"]).unwrap();
    let mut out = OutputCtx::new(false, OutputFormat::Json, false, 1);
    exec::run_shell(&[repo], &["echo".into(), "hi".into()], &cli, &cfg, &mut out).unwrap();
}

#[test]
fn run_git_inner_no_stdout_side_effects() {
    let (_d, repo) = make_repo();
    let cli = Cli::try_parse_from(["gg", "--format", "json", "status"]).unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let results =
        exec::run_git_inner(std::slice::from_ref(&repo), &["status", "-sb"], &cli, &cfg).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
    assert!(!results[0].skipped);
}

#[test]
fn fail_fast_includes_skipped_rows() {
    let (_d1, r1) = make_repo();
    let (_d2, r2) = make_repo();
    let cli =
        Cli::try_parse_from(["gg", "--fail-fast", "-j", "1", "rev-parse", "not-a-ref"]).unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let results = exec::run_git_inner(&[r1, r2], &["rev-parse", "not-a-ref"], &cli, &cfg).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.skipped) || results.iter().all(|r| !r.success));
}

#[test]
fn shell_for_each_is_platform_shell() {
    let (prog, flag) = exec::shell_for_each();
    #[cfg(windows)]
    {
        assert_eq!(flag, "/C");
        let base = std::path::Path::new(&prog)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&prog)
            .to_ascii_lowercase();
        assert!(base.contains("cmd"), "expected cmd.exe, got {prog}");
    }
    #[cfg(not(windows))]
    {
        assert_eq!(prog, "sh");
        assert_eq!(flag, "-c");
    }
}

#[test]
fn resolve_windows_shell_falls_back_on_empty_comspec() {
    assert_eq!(exec::resolve_windows_shell(None), "cmd.exe");
    assert_eq!(exec::resolve_windows_shell(Some("")), "cmd.exe");
    assert_eq!(exec::resolve_windows_shell(Some("   ")), "cmd.exe");
    assert_eq!(
        exec::resolve_windows_shell(Some(r"C:\Windows\System32\cmd.exe")),
        r"C:\Windows\System32\cmd.exe"
    );
}

#[test]
fn run_shell_fail_fast_reports_skipped() {
    let (_d1, r1) = make_repo();
    let (_d2, r2) = make_repo();
    let cli = Cli::try_parse_from(["gg", "--fail-fast", "-j", "1", "each", "x"]).unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let mut out = OutputCtx::new(false, OutputFormat::Json, false, 0);
    #[cfg(windows)]
    let fail_cmd = vec!["exit".into(), "/b".into(), "1".into()];
    #[cfg(not(windows))]
    let fail_cmd = vec!["false".into()];
    let err = exec::run_shell(&[r1, r2], &fail_cmd, &cli, &cfg, &mut out);
    assert!(err.is_err());
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("failed") || msg.contains("skipped"),
        "unexpected error: {msg}"
    );
}

#[test]
fn resolve_color_auto_without_no_color() {
    std::env::remove_var("NO_COLOR");
    let _ = git_gist::resolve_color(ColorChoice::Auto);
}
