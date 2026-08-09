//! Unit tests for config loading, merge, and key get/set.

use clap::Parser;
use git_gist::cli::Cli;
use git_gist::config::{self, Config, CONFIG_SCHEMA_VERSION};
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

fn empty_cli() -> Cli {
    Cli::try_parse_from(["gg"]).unwrap()
}

#[test]
fn schema_version_constant() {
    assert_eq!(CONFIG_SCHEMA_VERSION, 1);
}

#[test]
fn with_builtins_adds_default_profile_and_hooks() {
    let cfg = Config::default().with_builtins();
    assert!(cfg.profiles.contains_key("default"));
    assert!(cfg.hook_packs.contains_key("noop"));
    assert!(cfg.hook_packs.contains_key("commit-msg-required"));
}

#[test]
fn get_set_dot_keys() {
    let mut cfg = Config::default().with_builtins();
    config::set_dot_key(&mut cfg, "depth", "3").unwrap();
    assert_eq!(config::get_dot_key(&cfg, "depth").unwrap(), "3");
    config::set_dot_key(&mut cfg, "theme", "vivid").unwrap();
    assert_eq!(config::get_dot_key(&cfg, "theme").unwrap(), "vivid");
    config::set_dot_key(&mut cfg, "include_submodules", "true").unwrap();
    assert!(cfg.include_submodules);
    config::set_dot_key(&mut cfg, "show_path", "true").unwrap();
    assert!(cfg.show_path);
    assert_eq!(config::get_dot_key(&cfg, "show_path").unwrap(), "true");
    config::set_dot_key(&mut cfg, "jobs", "2").unwrap();
    assert_eq!(cfg.jobs, Some(2));
    assert!(config::set_dot_key(&mut cfg, "nope", "x").is_err());
    assert!(config::get_dot_key(&cfg, "nope").is_err());
}

fn set_test_home(home: &std::path::Path) {
    std::env::set_var("GIT_GIST_HOME", home);
    std::env::set_var("HOME", home);
    std::env::set_var("USERPROFILE", home);
}

#[test]
#[serial]
fn home_dir_prefers_git_gist_home_env() {
    let home = tempdir().unwrap();
    let other = tempdir().unwrap();
    std::env::set_var("HOME", other.path());
    std::env::set_var("USERPROFILE", other.path());
    std::env::set_var("GIT_GIST_HOME", home.path());
    assert_eq!(config::home_dir().unwrap(), home.path());
}

#[test]
#[serial]
fn load_merges_global_and_local() {
    let home = tempdir().unwrap();
    let root = tempdir().unwrap();
    let cfg_dir = home.path().join(".git-gist");
    fs::create_dir_all(&cfg_dir).unwrap();
    fs::write(
        cfg_dir.join("config.toml"),
        "schema_version = 1\ndepth = 2\nignore = [\"**/vendor/**\"]\n",
    )
    .unwrap();
    fs::write(
        root.path().join(".gg.toml"),
        "depth = 9\ntheme = \"mono\"\n",
    )
    .unwrap();

    set_test_home(home.path());
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();

    let cfg = config::load(&empty_cli()).unwrap();
    assert_eq!(cfg.depth, 9);
    assert_eq!(cfg.theme.as_deref(), Some("mono"));
    assert!(cfg.ignore.iter().any(|i| i.contains("vendor")));

    std::env::set_current_dir(prev).unwrap();
}

#[test]
#[serial]
fn save_global_roundtrip() {
    let home = tempdir().unwrap();
    set_test_home(home.path());

    let mut cfg = Config::default().with_builtins();
    cfg.path = Some(config::global_config_path().unwrap());
    cfg.depth = 5;
    cfg.aliases.insert("x".into(), home.path().join("repo"));
    let path = config::save_global(&cfg).unwrap();
    assert!(path.is_file());
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("depth = 5"));
    assert!(
        path.ends_with(".git-gist/config.toml") || path.to_string_lossy().contains(".git-gist")
    );
}

#[test]
#[serial]
fn empty_config_file_is_ok() {
    let home = tempdir().unwrap();
    let cfg_dir = home.path().join(".git-gist");
    fs::create_dir_all(&cfg_dir).unwrap();
    fs::write(cfg_dir.join("config.toml"), "   \n").unwrap();
    set_test_home(home.path());
    let cfg = config::load(&empty_cli()).unwrap();
    assert_eq!(cfg.schema_version, CONFIG_SCHEMA_VERSION);
}

#[test]
#[serial]
fn migrates_legacy_xdg_config() {
    let home = tempdir().unwrap();
    let legacy = home.path().join("config").join("git-gist");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(
        legacy.join("config.toml"),
        "schema_version = 1\ndepth = 4\n",
    )
    .unwrap();
    set_test_home(home.path());
    std::env::set_var("XDG_CONFIG_HOME", home.path().join("config"));
    let cfg = config::load(&empty_cli()).unwrap();
    assert_eq!(cfg.depth, 4);
    assert!(home.path().join(".git-gist").join("config.toml").is_file());
}

#[test]
fn find_local_config_walks_up() {
    let root = tempdir().unwrap();
    let nested = root.path().join("a").join("b");
    fs::create_dir_all(&nested).unwrap();
    fs::write(root.path().join(".git-gist.toml"), "depth = 1\n").unwrap();
    let found = config::find_local_config(&nested).unwrap();
    assert!(found.ends_with(".git-gist.toml"));
}

#[test]
fn scan_raw_detects_typo() {
    let w = config::scan_raw_config("[[auto_enrol]]\npath=\"/x\"\n");
    assert!(w.iter().any(|s| s.contains("did you mean")));
}
