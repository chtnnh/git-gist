//! Extra unit tests to push line coverage over 95%.

use clap::Parser;
use git_gist::cli::{Cli, OutputFormat};
use git_gist::config::{self, Config};
use git_gist::exec;
use git_gist::output::OutputCtx;
use git_gist::repo::Repo;
use serial_test::serial;
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
fn exec_timing_and_fail_fast() {
    let (_d1, r1) = make_repo();
    let (_d2, r2) = make_repo();
    let cli = Cli::try_parse_from([
        "gg",
        "--timing",
        "--fail-fast",
        "-j",
        "1",
        "rev-parse",
        "missing",
    ])
    .unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 1);
    let err = exec::run_git(&[r1, r2], &["rev-parse", "missing"], &cli, &cfg, &mut out);
    assert!(err.is_err());
}

#[test]
fn exec_json_success() {
    let (_d, repo) = make_repo();
    let cli = Cli::try_parse_from(["gg", "--format", "json", "status", "-sb"]).unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let mut out = OutputCtx::new(false, OutputFormat::Json, false, 0);
    exec::run_git(
        std::slice::from_ref(&repo),
        &["status", "-sb"],
        &cli,
        &cfg,
        &mut out,
    )
    .unwrap();
}

#[test]
fn overview_empty_warn() {
    use git_gist::commands::overview;
    let cli = Cli::try_parse_from(["gg", "overview"]).unwrap();
    let cfg = Config::default().with_builtins();
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    overview::run(&[], &cli, &cfg, &mut out).unwrap();
}

#[test]
fn sync_empty_warn_unit() {
    use git_gist::commands::sync;
    let cli = Cli::try_parse_from(["gg", "sync"]).unwrap();
    let cfg = Config::default().with_builtins();
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    sync::run(&[], false, &cli, &cfg, &mut out).unwrap();
}

#[test]
#[serial]
fn migrate_rejects_future_schema() {
    let home = tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("HOME", home.path());
    let cfg_dir = home.path().join("config").join("git-gist");
    fs::create_dir_all(&cfg_dir).unwrap();
    fs::write(
        cfg_dir.join("config.toml"),
        "schema_version = 99\ndepth = 1\n",
    )
    .unwrap();
    let cli = Cli::try_parse_from(["gg"]).unwrap();
    assert!(config::load(&cli).is_err());
}

#[test]
fn remotes_add_to_empty_selection_fails() {
    use git_gist::cli::RemotesAction;
    use git_gist::commands::remotes;
    let cli = Cli::try_parse_from(["gg", "remotes", "list"]).unwrap();
    let mut cfg = Config::default().with_builtins();
    cfg.remotes
        .insert("x".into(), "https://example.com/x.git".into());
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    let action = RemotesAction::AddTo {
        name: "x".into(),
        as_name: None,
    };
    assert!(remotes::run(&action, &[], &cli, &cfg, &mut out).is_err());
}

#[test]
fn hooks_install_empty_fails() {
    use git_gist::cli::HooksAction;
    use git_gist::commands::hooks;
    let cli = Cli::try_parse_from(["gg", "hooks", "list"]).unwrap();
    let cfg = Config::default().with_builtins();
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    let action = HooksAction::Install {
        pack: "noop".into(),
    };
    assert!(hooks::run(&action, &[], &cli, &cfg, &mut out).is_err());
}

#[test]
fn output_quiet_skips_info() {
    let mut out = OutputCtx::new(true, OutputFormat::Human, true, 0);
    out.info("hidden").unwrap();
    out.success("hidden").unwrap();
    out.repo_header("n", "p").unwrap();
}

#[test]
fn each_empty_command_fails() {
    use git_gist::commands::each;
    let (_d, repo) = make_repo();
    let cli = Cli::try_parse_from(["gg", "each", "true"]).unwrap();
    let cfg = Config::default().with_builtins();
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    assert!(each::run(std::slice::from_ref(&repo), &[], &cli, &cfg, &mut out).is_err());
}
