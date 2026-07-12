//! Unit tests targeting remaining repo/discover/config branches.

use clap::Parser;
use git_gist::cli::Cli;
use git_gist::config::{self, Config};
use git_gist::discover;
use git_gist::repo::{self, Repo};
use serial_test::serial;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn git(cwd: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@e.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@e.com")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "{args:?}");
}

#[test]
fn repo_new_fallback_name() {
    let repo = Repo::new(std::path::PathBuf::from("/"));
    assert!(!repo.name.is_empty());
}

#[test]
fn git_stdout_errors_on_failure() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-b", "main"]);
    let err = repo::git_stdout(dir.path(), &["rev-parse", "no-such-ref"]);
    assert!(err.is_err());
}

#[test]
fn probe_in_progress_markers() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-b", "main"]);
    fs::write(dir.path().join("f"), "x").unwrap();
    git(dir.path(), &["add", "f"]);
    git(dir.path(), &["commit", "-m", "c"]);
    for (marker, _label) in [
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
        ("BISECT_LOG", "bisect"),
    ] {
        let git_dir = dir.path().join(".git");
        let path = git_dir.join(marker);
        fs::write(&path, "x\n").unwrap();
        let status = repo::probe_status(dir.path()).unwrap();
        assert!(status.in_progress.is_some(), "expected {marker}");
        let _ = fs::remove_file(&path);
    }
}

#[test]
fn probe_unknown_branch_on_empty_repo() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-b", "main"]);
    // no commits — HEAD may be unborn
    let status = repo::probe_status(dir.path()).unwrap();
    assert!(!status.branch.is_empty());
}

#[test]
#[serial]
fn discover_ggignore_and_cache_roundtrip() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    std::env::set_var("HOME", home.path());

    let a = root.path().join("keep");
    let b = root.path().join("skipdir").join("hidden");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    git(&a, &["init"]);
    git(&b, &["init"]);
    fs::write(root.path().join(".ggignore"), "skipdir/**\n").unwrap();

    let cfg = Config {
        depth: 5,
        ..Config::default().with_builtins()
    };
    let found = discover::discover_repos(root.path(), 5, &cfg).unwrap();
    assert!(found.iter().any(|p| p.ends_with("keep")));
    assert!(!found.iter().any(|p| p.to_string_lossy().contains("hidden")));

    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();
    let cli = Cli::try_parse_from(["gg", "list"]).unwrap();
    let _ = discover::select_repos(&cli, &cfg).unwrap();
    // second call should hit cache
    let _ = discover::select_repos(&cli, &cfg).unwrap();
    std::env::set_current_dir(prev).unwrap();
}

#[test]
#[serial]
fn config_set_root_and_include_submodules() {
    let home = tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("HOME", home.path());
    let mut cfg = Config::default().with_builtins();
    cfg.schema_version = config::CONFIG_SCHEMA_VERSION;
    cfg.path = Some(config::global_config_path().unwrap());
    config::set_dot_key(&mut cfg, "root", home.path().to_str().unwrap()).unwrap();
    config::set_dot_key(&mut cfg, "include_submodules", "yes").unwrap();
    assert!(cfg.include_submodules);
    let _ = config::save_global(&cfg).unwrap();
    assert_eq!(
        config::get_dot_key(&cfg, "include_submodules").unwrap(),
        "true"
    );
    assert_eq!(
        config::get_dot_key(&cfg, "schema_version").unwrap(),
        config::CONFIG_SCHEMA_VERSION.to_string()
    );
    let _ = config::get_dot_key(&cfg, "jobs").unwrap();
    let _ = config::get_dot_key(&cfg, "root").unwrap();
}

#[test]
fn only_ahead_behind_stashed_via_filters_module() {
    use git_gist::filters;
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-b", "main"]);
    fs::write(dir.path().join("f"), "x").unwrap();
    git(dir.path(), &["add", "f"]);
    git(dir.path(), &["commit", "-m", "c"]);
    let repo = Repo::new(dir.path().to_path_buf());
    let cli = Cli::try_parse_from(["gg", "--only-ahead", "list"]).unwrap();
    let _ = filters::apply_status_filters(vec![repo], &cli, None).unwrap();
}
