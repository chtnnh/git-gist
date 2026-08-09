//! Unit tests for shared config mutations and validation.

use git_gist::config::{self, AutoEnroll, Config};
use git_gist::config_ops;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn alias_add_remove_rename_and_scrub() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("repo");
    std::fs::create_dir_all(&p).unwrap();
    let mut cfg = Config::default();
    config_ops::add_alias(&mut cfg, "old", p.clone());
    cfg.groups
        .insert("g".into(), vec!["old".into(), "other".into()]);
    cfg.tags.insert("t".into(), vec!["old".into()]);
    config_ops::rename_alias(&mut cfg, "old", "new").unwrap();
    assert!(cfg.aliases.contains_key("new"));
    assert!(!cfg.aliases.contains_key("old"));
    assert_eq!(
        cfg.groups.get("g").unwrap(),
        &vec!["new".to_string(), "other".into()]
    );
    config_ops::remove_alias(&mut cfg, "new").unwrap();
    assert!(!cfg.groups.get("g").unwrap().contains(&"new".into()));
    assert!(cfg.tags.get("t").unwrap().is_empty());
}

#[test]
fn group_and_tag_member_ops() {
    let mut cfg = Config::default();
    config_ops::set_group_members(&mut cfg, "work", vec!["a".into()]);
    config_ops::add_group_member(&mut cfg, "work", "b").unwrap();
    config_ops::add_group_member(&mut cfg, "work", "b").unwrap(); // idempotent
    assert_eq!(cfg.groups.get("work").unwrap().len(), 2);
    config_ops::remove_group_member(&mut cfg, "work", "a").unwrap();
    assert_eq!(cfg.groups.get("work").unwrap(), &vec!["b".to_string()]);
    assert!(config_ops::remove_group_member(&mut cfg, "work", "nope").is_err());
    config_ops::remove_group(&mut cfg, "work").unwrap();
    assert!(config_ops::remove_group(&mut cfg, "work").is_err());

    config_ops::set_tag_members(&mut cfg, "learn", vec!["x".into()]);
    config_ops::add_tag_member(&mut cfg, "learn", "y").unwrap();
    config_ops::remove_tag_member(&mut cfg, "learn", "x").unwrap();
    config_ops::remove_tag(&mut cfg, "learn").unwrap();
    assert!(config_ops::remove_tag(&mut cfg, "learn").is_err());
}

#[test]
fn remotes_and_enroll_and_scalar() {
    let mut cfg = Config::default();
    config_ops::add_remote(&mut cfg, "gh", "git@github.com:org/");
    assert!(config_ops::remove_remote(&mut cfg, "missing").is_err());
    config_ops::remove_remote(&mut cfg, "gh").unwrap();

    config_ops::add_auto_enroll_rule(
        &mut cfg,
        AutoEnroll {
            path: PathBuf::from("/tmp/watch"),
            path_prefix: Some("oss/".into()),
            depth: 3,
            groups: vec!["oss".into()],
            tags: vec![],
        },
    );
    assert_eq!(cfg.auto_enroll.len(), 1);
    let removed = config_ops::remove_auto_enroll_rule(&mut cfg, 0).unwrap();
    assert_eq!(removed.depth, 3);
    assert!(config_ops::remove_auto_enroll_rule(&mut cfg, 0).is_err());

    config_ops::set_scalar(&mut cfg, "depth", "9").unwrap();
    assert_eq!(cfg.depth, 9);
}

#[test]
fn prune_group_under_prefix() {
    let dir = tempdir().unwrap();
    let keep = dir.path().join("oss").join("a");
    let drop = dir.path().join("learning").join("b");
    std::fs::create_dir_all(&keep).unwrap();
    std::fs::create_dir_all(&drop).unwrap();
    let mut cfg = Config::default();
    cfg.aliases.insert("a".into(), keep);
    cfg.aliases.insert("b".into(), drop);
    cfg.aliases
        .insert("dead".into(), dir.path().join("missing"));
    cfg.groups
        .insert("oss".into(), vec!["a".into(), "b".into(), "dead".into()]);
    let removed =
        config_ops::prune_group_members(&mut cfg, "oss", Some(&dir.path().join("oss"))).unwrap();
    assert!(removed.contains(&"b".into()));
    assert!(removed.contains(&"dead".into()));
    assert_eq!(cfg.groups.get("oss").unwrap(), &vec!["a".to_string()]);
}

#[test]
fn error_paths_for_rename_and_members() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("repo");
    std::fs::create_dir_all(&p).unwrap();
    let mut cfg = Config::default();
    config_ops::add_alias(&mut cfg, "a", p.clone());
    config_ops::add_alias(&mut cfg, "b", p.clone());
    assert!(config_ops::rename_alias(&mut cfg, "a", "b").is_err());
    assert!(config_ops::remove_group_member(&mut cfg, "missing", "a").is_err());
    config_ops::set_tag_members(&mut cfg, "t", vec!["a".into()]);
    assert!(config_ops::remove_tag_member(&mut cfg, "t", "nope").is_err());
    assert!(config_ops::remove_tag_member(&mut cfg, "missing", "a").is_err());
    assert!(config_ops::prune_group_members(&mut cfg, "missing", None).is_err());
}

#[test]
fn scan_suggests_multiple_typos_programmatically() {
    let w = config::scan_raw_config(
        r#"
schema_version = 1
include_submodles = true
[[auto_enrol]]
path = "/x"
depth = 2
grupos = ["oss"]
"#,
    );
    assert!(w.iter().any(|s| s.contains("include_submodles")));
    assert!(w.iter().any(|s| s.contains("auto_enrol")));
    // nested field on typo'd table should still be checked when near auto_enroll
    assert!(
        w.iter()
            .any(|s| s.contains("grupos") || s.contains("groups")),
        "{w:?}"
    );
}
