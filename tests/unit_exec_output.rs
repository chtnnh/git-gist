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
    let cli = Cli::try_parse_from(["gg", "--dry-run", "each", "true"]).unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    exec::run_shell(
        std::slice::from_ref(&repo),
        &["true".into()],
        &cli,
        &cfg,
        &mut out,
    )
    .unwrap();

    let cli = Cli::try_parse_from(["gg", "--format", "json", "each", "true"]).unwrap();
    let mut out = OutputCtx::new(false, OutputFormat::Json, false, 1);
    exec::run_shell(&[repo], &["echo".into(), "hi".into()], &cli, &cfg, &mut out).unwrap();
}

#[test]
fn resolve_color_auto_without_no_color() {
    std::env::remove_var("NO_COLOR");
    let _ = git_gist::resolve_color(ColorChoice::Auto);
}
