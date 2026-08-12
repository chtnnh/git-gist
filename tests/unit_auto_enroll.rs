//! Unit tests for auto-enroll apply / throttle / warnings.

use clap::Parser;
use git_gist::auto_enroll;
use git_gist::cli::Cli;
use git_gist::config::{self, AutoEnroll, Config};
use serial_test::serial;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
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

fn init_repo(path: &std::path::Path) {
    fs::create_dir_all(path).unwrap();
    git(path, &["init", "-b", "main"]);
    fs::write(path.join("f"), "x").unwrap();
    git(path, &["add", "f"]);
    git(path, &["commit", "-m", "c"]);
}

fn set_test_home(home: &std::path::Path) {
    std::env::set_var("GIT_GIST_HOME", home);
    std::env::set_var("HOME", home);
    std::env::set_var("USERPROFILE", home);
    std::env::set_var("XDG_CONFIG_HOME", home.join("config"));
    std::env::set_var("XDG_CACHE_HOME", home.join("cache"));
}

#[test]
#[serial]
fn apply_warns_on_root_equals_watch_and_missing_path() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    set_test_home(home.path());

    init_repo(&root.path().join("a"));

    let cfg = Config {
        root: Some(root.path().to_path_buf()),
        path: Some(home.path().join(".git-gist/config.toml")),
        auto_enroll: vec![
            AutoEnroll {
                path: root.path().to_path_buf(),
                path_prefix: None,
                depth: 3,
                groups: vec!["all".into()],
                tags: vec![],
            },
            AutoEnroll {
                path: root.path().join("does-not-exist"),
                path_prefix: None,
                depth: 2,
                groups: vec![],
                tags: vec!["t".into()],
            },
        ],
        ..Config::default()
    };
    fs::create_dir_all(cfg.path.as_ref().unwrap().parent().unwrap()).unwrap();

    let report = auto_enroll::apply_auto_enroll(&cfg, true, false).unwrap();
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("equals config root")),
        "{:?}",
        report.warnings
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("missing") || w.contains("not a directory")),
        "{:?}",
        report.warnings
    );
    assert!(!report.added.is_empty());
}

#[test]
#[serial]
fn apply_fixes_membership_and_prunes_stale() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    set_test_home(home.path());

    let repo = root.path().join("proj");
    init_repo(&repo);

    let mut aliases = BTreeMap::new();
    aliases.insert("proj".into(), repo.clone());
    aliases.insert("gone".into(), root.path().join("missing"));
    let cfg = Config {
        path: Some(home.path().join(".git-gist/config.toml")),
        aliases,
        auto_enroll: vec![AutoEnroll {
            path: root.path().to_path_buf(),
            path_prefix: None,
            depth: 4,
            groups: vec!["g".into()],
            tags: vec!["t".into()],
        }],
        ..Config::default()
    };
    fs::create_dir_all(cfg.path.as_ref().unwrap().parent().unwrap()).unwrap();

    let report = auto_enroll::apply_auto_enroll(&cfg, false, true).unwrap();
    assert!(report.pruned_stale.contains(&"gone".into()));
    assert!(report.membership_fixed >= 1 || report.saved.is_some());
    let reloaded = config::load(&Cli::try_parse_from(["gg", "list"]).unwrap()).unwrap();
    assert!(reloaded
        .groups
        .get("g")
        .is_some_and(|m| m.contains(&"proj".into())));
    assert!(reloaded
        .tags
        .get("t")
        .is_some_and(|m| m.contains(&"proj".into())));
}

#[test]
#[serial]
fn apply_warns_when_group_grows_a_lot() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    set_test_home(home.path());

    for i in 0..22 {
        init_repo(&root.path().join(format!("r{i}")));
    }

    let cfg = Config {
        path: Some(home.path().join(".git-gist/config.toml")),
        auto_enroll: vec![AutoEnroll {
            path: root.path().to_path_buf(),
            path_prefix: None,
            depth: 2,
            groups: vec!["bulk".into()],
            tags: vec![],
        }],
        ..Config::default()
    };
    fs::create_dir_all(cfg.path.as_ref().unwrap().parent().unwrap()).unwrap();

    let report = auto_enroll::apply_auto_enroll(&cfg, true, false).unwrap();
    assert!(report.added.len() >= 22);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("bulk") && w.contains("members")),
        "{:?}",
        report.warnings
    );
}

#[test]
#[serial]
fn maybe_auto_enroll_throttles_after_scan() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    set_test_home(home.path());

    init_repo(&root.path().join("one"));

    let mut cfg = Config {
        path: Some(home.path().join(".git-gist/config.toml")),
        root: Some(root.path().to_path_buf()),
        auto_enroll: vec![AutoEnroll {
            path: root.path().to_path_buf(),
            path_prefix: None,
            depth: 3,
            groups: vec![],
            tags: vec![],
        }],
        ..Config::default()
    };
    fs::create_dir_all(cfg.path.as_ref().unwrap().parent().unwrap()).unwrap();
    config::save_global(&cfg).unwrap();

    let cli = Cli::try_parse_from(["gg", "list"]).unwrap();
    let first = auto_enroll::maybe_auto_enroll(&mut cfg, &cli).unwrap();
    assert!(first.is_some());

    // Reload so aliases match disk; second call should throttle (mtime unchanged).
    cfg = config::load(&cli).unwrap();
    let second = auto_enroll::maybe_auto_enroll(&mut cfg, &cli).unwrap();
    assert!(second.is_none(), "expected throttle skip");

    let cli_refresh = Cli::try_parse_from(["gg", "--refresh", "list"]).unwrap();
    let forced = auto_enroll::maybe_auto_enroll(&mut cfg, &cli_refresh).unwrap();
    assert!(forced.is_some());
}

#[test]
#[serial]
fn maybe_auto_enroll_rescan_after_interval() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    set_test_home(home.path());

    init_repo(&root.path().join("one"));

    let mut cfg = Config {
        path: Some(home.path().join(".git-gist/config.toml")),
        root: Some(root.path().to_path_buf()),
        auto_enroll: vec![AutoEnroll {
            path: root.path().to_path_buf(),
            path_prefix: None,
            depth: 3,
            groups: vec![],
            tags: vec![],
        }],
        ..Config::default()
    };
    fs::create_dir_all(cfg.path.as_ref().unwrap().parent().unwrap()).unwrap();
    config::save_global(&cfg).unwrap();

    let cli = Cli::try_parse_from(["gg", "list"]).unwrap();
    assert!(auto_enroll::maybe_auto_enroll(&mut cfg, &cli)
        .unwrap()
        .is_some());
    cfg = config::load(&cli).unwrap();
    assert!(auto_enroll::maybe_auto_enroll(&mut cfg, &cli)
        .unwrap()
        .is_none());

    // Age the throttle state past ENROLL_INTERVAL without changing watch mtime.
    let state_path = config::state_path().unwrap();
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["last_run_unix"] = serde_json::json!(1u64);
    fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap()).unwrap();

    let again = auto_enroll::maybe_auto_enroll(&mut cfg, &cli).unwrap();
    assert!(again.is_some(), "expected rescan after interval elapsed");
}

#[test]
fn unique_alias_numeric_suffix_and_sanitize() {
    use std::collections::BTreeSet;
    let mut used = BTreeSet::new();
    used.insert("foo".into());
    used.insert("nested-foo".into());
    used.insert("watch-foo".into());
    let root = PathBuf::from("/watch");
    let repo = PathBuf::from("/watch/nested/foo");
    let name = auto_enroll::unique_alias_name(&repo, &root, &used);
    assert!(name.starts_with("foo-") || name.contains("foo"));
    assert!(!used.contains(&name));
}

#[test]
#[serial]
fn apply_succeeds_when_record_state_blocked() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    set_test_home(home.path());

    init_repo(&root.path().join("fresh"));

    let cfg = Config {
        path: Some(home.path().join(".git-gist/config.toml")),
        root: Some(root.path().to_path_buf()),
        auto_enroll: vec![AutoEnroll {
            path: root.path().to_path_buf(),
            path_prefix: None,
            depth: 3,
            groups: vec![],
            tags: vec![],
        }],
        ..Config::default()
    };
    fs::create_dir_all(cfg.path.as_ref().unwrap().parent().unwrap()).unwrap();
    // Block throttle state write while leaving config.toml writable.
    fs::create_dir_all(home.path().join(".git-gist/state.json")).unwrap();

    let report = auto_enroll::apply_auto_enroll(&cfg, false, false).unwrap();
    assert!(report.saved.is_some(), "{report:?}");
    assert!(
        report.warnings.iter().any(|w| w.contains("throttle state")),
        "{:?}",
        report.warnings
    );
    assert!(!report.added.is_empty());

    let reloaded = config::load(&Cli::try_parse_from(["gg", "list"]).unwrap()).unwrap();
    assert!(reloaded.aliases.values().any(|p| p.ends_with("fresh")));
}

#[test]
#[serial]
fn maybe_auto_enroll_reloads_cfg_after_state_write_failure() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    set_test_home(home.path());

    init_repo(&root.path().join("live"));

    let mut cfg = Config {
        path: Some(home.path().join(".git-gist/config.toml")),
        root: Some(root.path().to_path_buf()),
        auto_enroll: vec![AutoEnroll {
            path: root.path().to_path_buf(),
            path_prefix: None,
            depth: 3,
            groups: vec![],
            tags: vec![],
        }],
        ..Config::default()
    };
    fs::create_dir_all(cfg.path.as_ref().unwrap().parent().unwrap()).unwrap();
    config::save_global(&cfg).unwrap();
    fs::create_dir_all(home.path().join(".git-gist/state.json")).unwrap();

    assert!(cfg.aliases.is_empty());
    let cli = Cli::try_parse_from(["gg", "list"]).unwrap();
    let report = auto_enroll::maybe_auto_enroll(&mut cfg, &cli)
        .unwrap()
        .expect("expected enroll run");
    assert!(report.saved.is_some());
    assert!(
        cfg.aliases
            .values()
            .any(|p| p.file_name().is_some_and(|n| n == "live")),
        "cfg should reload new aliases in-memory: {:?}",
        cfg.aliases
    );
}
