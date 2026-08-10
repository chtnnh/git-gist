//! Pure-function coverage for parsers and target resolution.

use git_gist::commands::worktrees::{flags, parse_porcelain, WorktreeRow};
use git_gist::config::{Config, RepoOverride};
use git_gist::discover::{paths_for_tags, resolve_target};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn parse_porcelain_all_flags() {
    let porcelain = "\
worktree /tmp/a
HEAD abcdef012345
branch refs/heads/main

worktree /tmp/bare
bare
locked reason
prunable
";
    let rows = parse_porcelain("repo", porcelain);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].branch, "main");
    assert_eq!(rows[0].head, "abcdef01");
    assert!(rows[1].bare);
    assert!(rows[1].locked);
    assert!(rows[1].prunable);
    assert_eq!(flags(&rows[1]), "bare,locked,prunable");
    assert_eq!(flags(&rows[0]), "-");
}

#[test]
fn parse_porcelain_empty_and_trailing() {
    assert!(parse_porcelain("r", "").is_empty());
    let rows = parse_porcelain(
        "r",
        "worktree /only\nHEAD 1234567890abcdef\nbranch refs/heads/feat\n",
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].path, "/only");
}

#[test]
fn resolve_target_circular_group_errors() {
    let mut cfg = Config::default().with_builtins();
    cfg.groups.insert("a".into(), vec!["b".into()]);
    cfg.groups.insert("b".into(), vec!["a".into()]);
    let err = resolve_target("a", &cfg, &[]).unwrap_err();
    assert!(
        err.to_string().contains("circular"),
        "expected circular group error, got {err}"
    );
}

#[test]
fn resolve_target_group_alias_glob_unknown() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("alpha-app");
    let b = dir.path().join("beta-app");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let discovered = vec![a.clone(), b.clone()];

    let mut cfg = Config::default().with_builtins();
    cfg.aliases.insert("alpha".into(), a.clone());
    cfg.groups.insert(
        "both".into(),
        vec!["alpha".into(), b.to_string_lossy().into()],
    );

    let got = resolve_target("alpha", &cfg, &discovered).unwrap();
    assert_eq!(got.len(), 1);

    let got = resolve_target("both", &cfg, &discovered).unwrap();
    assert!(got.len() >= 2);

    let got = resolve_target("*-app", &cfg, &discovered).unwrap();
    assert_eq!(got.len(), 2);

    assert!(resolve_target("nope", &cfg, &discovered).is_err());

    let got = resolve_target(a.to_str().unwrap(), &cfg, &discovered).unwrap();
    assert_eq!(got.len(), 1);
}

#[test]
fn paths_for_tags_aliases_and_overrides() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("svc");
    std::fs::create_dir_all(&p).unwrap();
    let mut cfg = Config::default().with_builtins();
    cfg.aliases.insert("svc".into(), p.clone());
    cfg.tags.insert("oss".into(), vec!["svc".into()]);
    cfg.repo_overrides.insert(
        p.display().to_string(),
        RepoOverride {
            skip: false,
            default_args: vec![],
            tags: vec!["extra".into()],
        },
    );

    let set = paths_for_tags(&["oss".into()], &cfg);
    assert!(!set.is_empty());
    let set = paths_for_tags(&["extra".into()], &cfg);
    assert!(!set.is_empty());
}

#[test]
fn worktree_row_equality_smoke() {
    let r = WorktreeRow {
        repo: "r".into(),
        path: "/p".into(),
        head: "abcd".into(),
        branch: "main".into(),
        bare: false,
        locked: false,
        prunable: false,
    };
    assert_eq!(r, r.clone());
    let _ = BTreeMap::<String, PathBuf>::new();
}
