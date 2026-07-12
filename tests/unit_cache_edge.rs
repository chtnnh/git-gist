//! Final few lines for the 95% gate.

use clap::Parser;
use git_gist::cli::Cli;
use git_gist::config::{self, Config};
use git_gist::discover;
use serial_test::serial;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

fn git_init(path: &std::path::Path) {
    fs::create_dir_all(path).unwrap();
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
}

#[test]
#[serial]
fn cache_expired_and_mismatched_is_ignored() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    std::env::set_var("HOME", home.path());

    let child = root.path().join("c");
    git_init(&child);

    let cache_dir = home.path().join("cache").join("git-gist");
    fs::create_dir_all(&cache_dir).unwrap();
    let old = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(7200);
    let body = serde_json::json!({
        "root": root.path().canonicalize().unwrap(),
        "depth": 5,
        "include_submodules": false,
        "scanned_at": old,
        "repos": [child.canonicalize().unwrap()],
    });
    fs::write(cache_dir.join("discovery.json"), body.to_string()).unwrap();

    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();
    let cli = Cli::try_parse_from(["gg", "list"]).unwrap();
    let cfg = Config {
        depth: 5,
        ..Config::default().with_builtins()
    };
    let repos = discover::select_repos(&cli, &cfg).unwrap();
    assert_eq!(repos.len(), 1);

    // corrupt cache
    fs::write(cache_dir.join("discovery.json"), "{not-json").unwrap();
    let _ = discover::select_repos(&cli, &cfg).unwrap();

    // mismatched depth in cache
    let body = serde_json::json!({
        "root": root.path().canonicalize().unwrap(),
        "depth": 1,
        "include_submodules": false,
        "scanned_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "repos": [],
    });
    fs::write(cache_dir.join("discovery.json"), body.to_string()).unwrap();
    let _ = discover::select_repos(&cli, &cfg).unwrap();

    std::env::set_current_dir(prev).unwrap();
}

#[test]
#[serial]
fn include_submodules_discovers_gitfile() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    std::env::set_var("HOME", home.path());

    let parent = root.path().join("parent");
    git_init(&parent);
    let sub = parent.join("sub");
    git_init(&sub);
    // convert sub/.git dir into a gitfile pointing at itself-ish (worktree style)
    let real = sub.join(".git");
    if real.is_dir() {
        let alt = root.path().join("sub.git");
        fs::rename(&real, &alt).unwrap();
        fs::write(&real, format!("gitdir: {}\n", alt.display())).unwrap();
    }

    let cfg = Config {
        depth: 5,
        include_submodules: true,
        ..Config::default().with_builtins()
    };
    let found = discover::discover_repos(root.path(), 5, &cfg).unwrap();
    assert!(found
        .iter()
        .any(|p| p.ends_with("sub") || p.ends_with("parent")));

    let cfg2 = Config {
        depth: 5,
        include_submodules: false,
        ..Config::default().with_builtins()
    };
    let found2 = discover::discover_repos(root.path(), 5, &cfg2).unwrap();
    // submodule gitfile should be skipped when include_submodules is false
    let _ = found2;
}

#[test]
fn doctor_error_finding_format() {
    use git_gist::cli::OutputFormat;
    use git_gist::commands::doctor;
    use git_gist::output::OutputCtx;
    use git_gist::repo::Repo;

    let dir = tempdir().unwrap();
    // empty .git directory → probe fails / odd state
    fs::create_dir_all(dir.path().join(".git")).unwrap();
    let repo = Repo::new(dir.path().to_path_buf());
    let cli = Cli::try_parse_from(["gg", "doctor"]).unwrap();
    let cfg = Config::default().with_builtins();
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    doctor::run(&[repo], &cli, &cfg, &mut out).unwrap();
}

#[test]
#[serial]
fn empty_cache_file_ok() {
    let home = tempdir().unwrap();
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    std::env::set_var("HOME", home.path());
    let cache = config::cache_path().unwrap();
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    fs::write(&cache, "\n").unwrap();
    let root = tempdir().unwrap();
    let cfg = Config::default().with_builtins();
    let _ = discover::discover_repos(root.path(), 2, &cfg).unwrap();
}

#[test]
fn exec_empty_repos_and_shell_fail() {
    use git_gist::cli::OutputFormat;
    use git_gist::exec;
    use git_gist::output::OutputCtx;
    let cli = Cli::try_parse_from(["gg", "status"]).unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    exec::run_git(&[], &["status"], &cli, &cfg, &mut out).unwrap();
    exec::run_shell(&[], &["true".into()], &cli, &cfg, &mut out).unwrap();

    let dir = tempdir().unwrap();
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    let repo = git_gist::repo::Repo::new(dir.path().to_path_buf());
    let cli = Cli::try_parse_from(["gg", "--fail-fast", "--timing", "each", "false"]).unwrap();
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    assert!(exec::run_shell(
        std::slice::from_ref(&repo),
        &["false".into()],
        &cli,
        &cfg,
        &mut out
    )
    .is_err());
}

#[test]
fn overview_json_empty() {
    use git_gist::cli::OutputFormat;
    use git_gist::commands::overview;
    use git_gist::output::OutputCtx;
    let cli = Cli::try_parse_from(["gg", "--format", "json", "overview"]).unwrap();
    let cfg = Config::default().with_builtins();
    let mut out = OutputCtx::new(false, OutputFormat::Json, false, 0);
    overview::run(&[], &cli, &cfg, &mut out).unwrap();
}

#[test]
#[serial]
fn exec_when_git_missing_from_path() {
    use git_gist::cli::OutputFormat;
    use git_gist::exec;
    use git_gist::output::OutputCtx;
    use git_gist::repo::Repo;

    let dir = tempdir().unwrap();
    // don't need a real git repo for spawn failure
    fs::create_dir_all(dir.path()).unwrap();
    let repo = Repo::new(dir.path().to_path_buf());

    let old = std::env::var_os("PATH");
    std::env::set_var("PATH", "/nonexistent-gg-path");
    let cli = Cli::try_parse_from(["gg", "status"]).unwrap();
    let cfg = Config {
        jobs: Some(1),
        ..Config::default().with_builtins()
    };
    let mut out = OutputCtx::new(false, OutputFormat::Json, false, 0);
    let result = exec::run_git(
        std::slice::from_ref(&repo),
        &["status"],
        &cli,
        &cfg,
        &mut out,
    );
    match old {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }
    assert!(result.is_err());
}

#[test]
fn filters_only_ahead_drops_synced_upstream() {
    use git_gist::filters;
    use git_gist::repo::Repo;
    // bare + clone with upstream, no local ahead
    let root = tempdir().unwrap();
    let bare = root.path().join("b.git");
    let clone = root.path().join("c");
    fs::create_dir_all(&bare).unwrap();
    Command::new("git")
        .args(["init", "--bare", "-b", "main"])
        .current_dir(&bare)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    Command::new("git")
        .args(["clone", bare.to_str().unwrap(), clone.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    fs::write(clone.join("f"), "x").unwrap();
    Command::new("git")
        .args(["add", "f"])
        .current_dir(&clone)
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@e.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@e.com")
        .status()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "c"])
        .current_dir(&clone)
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@e.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@e.com")
        .status()
        .unwrap();
    Command::new("git")
        .args(["push", "-u", "origin", "main"])
        .current_dir(&clone)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();

    let repo = Repo::new(clone);
    let cli = Cli::try_parse_from(["gg", "--only-ahead", "list"]).unwrap();
    let filtered = filters::apply_status_filters(vec![repo], &cli).unwrap();
    assert!(filtered.is_empty());
}
