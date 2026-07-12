//! Unit tests for status filters and repo probes.

use clap::Parser;
use git_gist::cli::Cli;
use git_gist::filters;
use git_gist::repo::{self, Repo};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn setup_repo(with_commit: bool) -> (tempfile::TempDir, Repo) {
    let dir = tempdir().unwrap();
    let status = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    if with_commit {
        fs::write(dir.path().join("f"), "x").unwrap();
        let add = Command::new("git")
            .args(["add", "f"])
            .current_dir(dir.path())
            .env("GIT_AUTHOR_NAME", "T")
            .env("GIT_AUTHOR_EMAIL", "t@e.com")
            .env("GIT_COMMITTER_NAME", "T")
            .env("GIT_COMMITTER_EMAIL", "t@e.com")
            .status()
            .unwrap();
        assert!(add.success());
        let commit = Command::new("git")
            .args(["commit", "-m", "c"])
            .current_dir(dir.path())
            .env("GIT_AUTHOR_NAME", "T")
            .env("GIT_AUTHOR_EMAIL", "t@e.com")
            .env("GIT_COMMITTER_NAME", "T")
            .env("GIT_COMMITTER_EMAIL", "t@e.com")
            .status()
            .unwrap();
        assert!(commit.success());
    }
    let repo = Repo::new(dir.path().to_path_buf());
    (dir, repo)
}

fn cli_flag(flag: &str) -> Cli {
    Cli::try_parse_from(["gg", flag, "list"]).unwrap()
}

#[test]
fn probe_status_clean_and_dirty() {
    let (_dir, repo) = setup_repo(true);
    let status = repo::probe_status(&repo.path).unwrap();
    assert!(!status.dirty);
    assert_eq!(status.branch, "main");

    fs::write(repo.path.join("extra"), "y").unwrap();
    let status = repo::probe_status(&repo.path).unwrap();
    assert!(status.dirty);
}

#[test]
fn format_age_buckets() {
    assert_eq!(repo::format_age(30), "30s");
    assert_eq!(repo::format_age(120), "2m");
    assert_eq!(repo::format_age(7200), "2h");
    assert_eq!(repo::format_age(86400), "1d");
    assert_eq!(repo::format_age(200000), "2d");
}

#[test]
fn only_dirty_filter() {
    let (d1, clean) = setup_repo(true);
    let (d2, dirty) = setup_repo(true);
    fs::write(dirty.path.join("x"), "1").unwrap();
    let cli = cli_flag("--only-dirty");
    let filtered = filters::apply_status_filters(vec![clean, dirty], &cli).unwrap();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].path.ends_with(d2.path().file_name().unwrap()));
    drop(d1);
}

#[test]
fn only_clean_filter() {
    let (_d1, clean) = setup_repo(true);
    let (_d2, dirty) = setup_repo(true);
    fs::write(dirty.path.join("x"), "1").unwrap();
    let cli = cli_flag("--only-clean");
    let filtered = filters::apply_status_filters(vec![clean.clone(), dirty], &cli).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, clean.name);
}

#[test]
fn git_stdout_success() {
    let (_dir, repo) = setup_repo(true);
    let out = repo::git_stdout(&repo.path, &["rev-parse", "--is-inside-work-tree"]).unwrap();
    assert_eq!(out, "true");
}
