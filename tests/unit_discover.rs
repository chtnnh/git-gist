//! Unit tests for discovery and selection helpers.

use clap::Parser;
use git_gist::cli::Cli;
use git_gist::config::Config;
use git_gist::discover::{self, is_git_repo};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

use serial_test::serial;

fn git_init(path: &std::path::Path) {
    fs::create_dir_all(path).unwrap();
    let status = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
}

fn cli_from(args: &[&str]) -> Cli {
    let mut full = vec!["gg"];
    full.extend_from_slice(args);
    Cli::try_parse_from(full).unwrap()
}

#[test]
fn is_git_repo_detects_dir() {
    let dir = tempdir().unwrap();
    assert!(!is_git_repo(dir.path()));
    git_init(dir.path());
    assert!(is_git_repo(dir.path()));
}

#[test]
fn discover_skips_root_and_ignored() {
    let root = tempdir().unwrap();
    git_init(root.path()); // should be skipped as root
    let child = root.path().join("child");
    git_init(&child);
    let ignored = root.path().join("node_modules").join("pkg");
    git_init(&ignored);

    let cfg = Config::default().with_builtins();
    let found = discover::discover_repos(root.path(), 6, &cfg).unwrap();
    assert!(found.iter().any(|p| p.ends_with("child")));
    assert!(!found
        .iter()
        .any(|p| p.to_string_lossy().contains("node_modules")));
    assert!(!found
        .iter()
        .any(|p| p == &root.path().canonicalize().unwrap()));
}

#[test]
#[serial]
fn select_repos_respects_include() {
    let root = tempdir().unwrap();
    let a = root.path().join("a");
    let b = root.path().join("b");
    git_init(&a);
    git_init(&b);

    let home = tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    std::env::set_var("HOME", home.path());

    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();

    let cli = cli_from(&["--in", a.to_str().unwrap(), "list"]);
    let mut cfg = Config {
        depth: 4,
        ..Config::default().with_builtins()
    };
    let repos = discover::select_repos(&cli, &mut cfg).unwrap();
    assert_eq!(repos.len(), 1);
    assert!(repos[0].path.ends_with("a"));

    std::env::set_current_dir(prev).unwrap();
}

#[test]
#[serial]
fn basename_target_resolution() {
    let root = tempdir().unwrap();
    let named = root.path().join("payments");
    git_init(&named);

    let home = tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    std::env::set_var("HOME", home.path());
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();

    let cli = cli_from(&["--in", "payments", "list", "--refresh"]);
    let mut cfg = Config {
        depth: 4,
        ..Config::default().with_builtins()
    };
    let repos = discover::select_repos(&cli, &mut cfg).unwrap();
    assert_eq!(repos.len(), 1);

    std::env::set_current_dir(prev).unwrap();
}
