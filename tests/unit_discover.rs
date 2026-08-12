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
    std::env::set_var("GIT_GIST_HOME", home.path());
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    std::env::set_var("HOME", home.path());
    std::env::set_var("USERPROFILE", home.path());

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
    std::env::set_var("GIT_GIST_HOME", home.path());
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

#[test]
#[serial]
fn tag_only_selection_and_status_summary() {
    let root = tempdir().unwrap();
    let clean = root.path().join("clean");
    let dirty = root.path().join("dirty");
    for path in [&clean, &dirty] {
        git_init(path);
        fs::write(path.join("f"), "x").unwrap();
        Command::new("git")
            .args(["add", "f"])
            .current_dir(path)
            .env("GIT_AUTHOR_NAME", "T")
            .env("GIT_AUTHOR_EMAIL", "t@e.com")
            .env("GIT_COMMITTER_NAME", "T")
            .env("GIT_COMMITTER_EMAIL", "t@e.com")
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "c"])
            .current_dir(path)
            .env("GIT_AUTHOR_NAME", "T")
            .env("GIT_AUTHOR_EMAIL", "t@e.com")
            .env("GIT_COMMITTER_NAME", "T")
            .env("GIT_COMMITTER_EMAIL", "t@e.com")
            .status()
            .unwrap();
    }
    fs::write(dirty.join("x"), "1").unwrap();

    let home = tempdir().unwrap();
    std::env::set_var("GIT_GIST_HOME", home.path());
    std::env::set_var("HOME", home.path());
    std::env::set_var("USERPROFILE", home.path());
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();

    let mut cfg = Config {
        depth: 4,
        aliases: [
            ("clean".into(), clean.clone()),
            ("dirty".into(), dirty.clone()),
        ]
        .into_iter()
        .collect(),
        tags: [("work".into(), vec!["clean".into(), "dirty".into()])]
            .into_iter()
            .collect(),
        ..Config::default().with_builtins()
    };

    let cli = cli_from(&["--tag", "work", "--only-dirty", "list", "--refresh"]);
    let repos = discover::select_repos(&cli, &mut cfg).unwrap();
    assert_eq!(repos.len(), 1);
    assert!(repos[0].path.ends_with("dirty"));

    std::env::set_current_dir(prev).unwrap();
}

#[test]
#[serial]
fn select_repos_propagates_bad_ggignore() {
    let root = tempdir().unwrap();
    git_init(&root.path().join("a"));
    // Unclosed bracket — invalid glob for globset.
    fs::write(root.path().join(".ggignore"), "bad[pattern\n").unwrap();

    let home = tempdir().unwrap();
    std::env::set_var("GIT_GIST_HOME", home.path());
    std::env::set_var("HOME", home.path());
    std::env::set_var("USERPROFILE", home.path());
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();

    let cli = cli_from(&["list", "--refresh"]);
    let mut cfg = Config {
        depth: 4,
        ..Config::default().with_builtins()
    };
    let err = discover::select_repos(&cli, &mut cfg).unwrap_err();
    assert!(
        err.to_string().contains("discovery") || format!("{err:#}").contains("glob"),
        "expected discovery/glob error, got {err:#}"
    );

    std::env::set_current_dir(prev).unwrap();
}

#[test]
#[serial]
fn auto_enroll_throttle_failure_warns_and_continues() {
    use git_gist::config::AutoEnroll;
    use std::path::PathBuf;

    let root = tempdir().unwrap();
    git_init(&root.path().join("a"));
    let home = tempdir().unwrap();
    std::env::set_var("GIT_GIST_HOME", home.path());
    std::env::set_var("HOME", home.path());
    std::env::set_var("USERPROFILE", home.path());
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));

    // Block throttle state writes; config saves to a separate writable path.
    fs::write(home.path().join(".git-gist"), "not-a-dir").unwrap();

    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();

    let mut cfg = Config {
        depth: 3,
        auto_enroll: vec![AutoEnroll {
            path: root.path().to_path_buf(),
            path_prefix: None,
            depth: 2,
            groups: vec![],
            tags: vec![],
        }],
        path: Some(PathBuf::from("/tmp/unused-config.toml")),
        ..Config::default().with_builtins()
    };
    let cli_soft = cli_from(&["list"]);
    assert!(discover::select_repos(&cli_soft, &mut cfg).is_ok());
    // Throttle failure is non-fatal even when forced — config was persisted.
    let cli_force = cli_from(&["--refresh", "list"]);
    assert!(discover::select_repos(&cli_force, &mut cfg).is_ok());

    std::env::set_current_dir(prev).unwrap();
}

#[test]
#[serial]
fn auto_enroll_refresh_propagates_save_failure() {
    use git_gist::config::AutoEnroll;

    let root = tempdir().unwrap();
    git_init(&root.path().join("a"));
    let home = tempdir().unwrap();
    std::env::set_var("GIT_GIST_HOME", home.path());
    std::env::set_var("HOME", home.path());
    std::env::set_var("USERPROFILE", home.path());
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.path().join("cache"));

    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();

    let bad_config = root.path().join("config-is-dir");
    fs::create_dir_all(&bad_config).unwrap();

    let mut cfg = Config {
        depth: 3,
        auto_enroll: vec![AutoEnroll {
            path: root.path().to_path_buf(),
            path_prefix: None,
            depth: 2,
            groups: vec![],
            tags: vec![],
        }],
        path: Some(bad_config),
        ..Config::default().with_builtins()
    };
    let cli_force = cli_from(&["--refresh", "list"]);
    assert!(
        discover::select_repos(&cli_force, &mut cfg).is_err(),
        "expected save failure to propagate under --refresh"
    );

    std::env::set_current_dir(prev).unwrap();
}
