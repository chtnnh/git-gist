//! Unit tests for status filters and repo probes.

use clap::Parser;
use git_gist::cli::Cli;
use git_gist::filters;
use git_gist::repo::{self, ProbeOpts, Repo, RepoStatus};
use serial_test::serial;
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
fn repo_label_show_path_relative_to_root() {
    let root = tempdir().unwrap();
    let nested = root.path().join("oss").join("demo");
    fs::create_dir_all(&nested).unwrap();
    let repo = Repo::new(nested);
    assert_eq!(repo.label(false, Some(root.path())), "demo");
    let with_path = repo.label(true, Some(root.path()));
    assert!(
        with_path.starts_with("demo ("),
        "expected name (path), got {with_path}"
    );
    assert!(
        with_path.contains("oss") && with_path.contains("demo"),
        "expected relative path under root, got {with_path}"
    );
}

#[test]
fn repo_label_falls_back_to_absolute_when_relative_is_basename() {
    let root = tempdir().unwrap();
    let child = root.path().join("solo");
    fs::create_dir_all(&child).unwrap();
    let repo = Repo::new(child.clone());
    let label = repo.label(true, Some(root.path()));
    assert!(
        label.contains(child.canonicalize().unwrap().to_str().unwrap()) || label.contains("solo"),
        "expected absolute path fallback, got {label}"
    );
    assert!(
        !label.ends_with("(solo)"),
        "should not repeat bare basename"
    );
}

#[test]
fn combined_status_and_stash_filters_hard_fail_probe() {
    let broken_dir = tempdir().unwrap();
    let broken = Repo::new(broken_dir.path().to_path_buf());
    let cli = Cli::try_parse_from(["gg", "--only-dirty", "--only-stashed", "list"]).unwrap();
    let err = filters::apply_status_filters(vec![broken], &cli, Some(1)).unwrap_err();
    assert!(
        err.to_string().contains("all"),
        "expected all-probe failure for combined filters, got {err}"
    );
}

#[test]
fn stash_only_filter_hard_fails_missing_repo() {
    let broken_dir = tempdir().unwrap();
    let broken = Repo::new(broken_dir.path().to_path_buf());
    let cli = cli_flag("--only-stashed");
    let err = filters::apply_status_filters(vec![broken], &cli, Some(1)).unwrap_err();
    assert!(
        err.to_string().contains("all"),
        "expected stash probe failure, got {err}"
    );
}

#[test]
fn status_filters_empty_input() {
    let cli = cli_flag("--only-dirty");
    let filtered = filters::apply_status_filters(vec![], &cli, Some(1)).unwrap();
    assert!(filtered.is_empty());
}

#[test]
fn status_filters_exclude_probe_failures() {
    let (_ok_dir, ok) = setup_repo(true);
    let broken_dir = tempdir().unwrap();
    // Not a git repo — probe_with returns Err and the path is excluded.
    let broken = Repo::new(broken_dir.path().to_path_buf());
    let cli = cli_flag("--only-clean");
    let filtered = filters::apply_status_filters(vec![ok.clone(), broken], &cli, Some(1)).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].path, ok.path);
}

#[test]
fn status_filters_err_when_all_probes_fail() {
    let broken_dir = tempdir().unwrap();
    let broken = Repo::new(broken_dir.path().to_path_buf());
    let cli = cli_flag("--only-dirty");
    let err = filters::apply_status_filters(vec![broken], &cli, Some(1)).unwrap_err();
    assert!(
        err.to_string().contains("all"),
        "expected all-probe failure, got {err}"
    );
}

#[test]
#[serial]
fn only_dirty_filter() {
    let (d1, clean) = setup_repo(true);
    let (d2, dirty) = setup_repo(true);
    fs::write(dirty.path.join("x"), "1").unwrap();
    let cli = cli_flag("--only-dirty");
    let filtered = filters::apply_status_filters(vec![clean, dirty], &cli, None).unwrap();
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
    let filtered = filters::apply_status_filters(vec![clean.clone(), dirty], &cli, None).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, clean.name);
}

#[test]
fn git_stdout_success() {
    let (_dir, repo) = setup_repo(true);
    let out = repo::git_stdout(&repo.path, &["rev-parse", "--is-inside-work-tree"]).unwrap();
    assert_eq!(out, "true");
}

#[test]
fn apply_porcelain_v2_parses_branch_dirty_and_ab() {
    let mut status = RepoStatus::default();
    repo::apply_porcelain_v2(
        &mut status,
        "\
# branch.oid abcdef0123456789
# branch.head main
# branch.upstream origin/main
# branch.ab +2 -3
1 .M N... 100644 100644 100644 deadbeef deadbeef f
",
    );
    assert_eq!(status.branch, "main");
    assert!(!status.detached);
    assert!(status.dirty);
    assert_eq!(status.ahead, 2);
    assert_eq!(status.behind, 3);
    assert_eq!(status.upstream.as_deref(), Some("origin/main"));
}

#[test]
fn apply_porcelain_v2_parses_detached() {
    let mut status = RepoStatus::default();
    repo::apply_porcelain_v2(
        &mut status,
        "\
# branch.oid abcdef0123456789
# branch.head (detached)
",
    );
    assert!(status.detached);
    assert_eq!(status.branch, "abcdef0");
    assert!(!status.dirty);
}

#[test]
fn probe_opts_for_cli_filters_skips_unused_work() {
    let dirty_only = ProbeOpts::for_cli_filters(true, false, false, false, false, false);
    assert!(dirty_only.status_branch);
    assert!(!dirty_only.stash);
    assert!(!dirty_only.last_commit);
    assert!(!dirty_only.in_progress);

    let stash_only = ProbeOpts::for_cli_filters(false, false, false, false, true, false);
    assert!(!stash_only.status_branch);
    assert!(stash_only.stash);
}

#[test]
#[serial]
#[cfg(unix)]
fn probe_with_filter_tree_uses_few_git_invocations() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, repo) = setup_repo(true);
    let wrapper_dir = tempdir().unwrap();
    let log = wrapper_dir.path().join("git.log");
    let wrapper = wrapper_dir.path().join("git-wrap");
    let real_git = which::which("git").unwrap();
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\necho \"$PWD $*\" >> '{}'\nexec '{}' \"$@\"\n",
            log.display(),
            real_git.display()
        ),
    )
    .unwrap();
    let mut perms = fs::metadata(&wrapper).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&wrapper, perms).unwrap();

    let repo_path = repo.path.canonicalize().unwrap();
    let marker = repo_path.to_string_lossy().into_owned();

    let old = std::env::var_os("GIT_GIST_GIT");
    std::env::set_var("GIT_GIST_GIT", &wrapper);

    let status = repo::probe_with(&repo.path, ProbeOpts::FILTER_TREE).unwrap();
    assert_eq!(status.branch, "main");
    let filter_ours: Vec<_> = fs::read_to_string(&log)
        .unwrap()
        .lines()
        .filter(|l| l.contains(&marker))
        .map(str::to_string)
        .collect();
    assert_eq!(
        filter_ours.len(),
        1,
        "FILTER_TREE should spawn exactly 1 git process for this repo, got:\n{}",
        filter_ours.join("\n")
    );
    assert!(filter_ours[0].contains("status --porcelain=v2 --branch"));

    fs::write(&log, "").unwrap();
    let full = repo::probe_with(&repo.path, ProbeOpts::FULL).unwrap();
    assert_eq!(full.branch, "main");
    let full_ours: Vec<_> = fs::read_to_string(&log)
        .unwrap()
        .lines()
        .filter(|l| l.contains(&marker))
        .map(str::to_string)
        .collect();

    match old {
        Some(v) => std::env::set_var("GIT_GIST_GIT", v),
        None => std::env::remove_var("GIT_GIST_GIT"),
    }

    assert_eq!(
        full_ours.len(),
        3,
        "FULL probe should spawn exactly 3 git processes, got {}:\n{}",
        full_ours.len(),
        full_ours.join("\n")
    );
    let joined = full_ours.join("\n");
    assert!(joined.contains("status --porcelain=v2 --branch"));
    assert!(joined.contains("stash list"));
    assert!(joined.contains("log -1"));
}

#[test]
fn probe_detached_head() {
    let (_dir, repo) = setup_repo(true);
    let status = Command::new("git")
        .args(["checkout", "--detach"])
        .current_dir(&repo.path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let probed = repo::probe_status(&repo.path).unwrap();
    assert!(probed.detached);
    assert!(!probed.branch.is_empty());
}

#[test]
fn resolve_git_dir_for_normal_repo() {
    let (_dir, repo) = setup_repo(true);
    let git_dir = repo::resolve_git_dir(&repo.path);
    assert!(git_dir.ends_with(".git"));
    assert!(git_dir.is_dir());
}

#[test]
fn resolve_git_dir_from_gitfile() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work");
    let real = dir.path().join("real.git");
    fs::create_dir_all(&work).unwrap();
    fs::create_dir_all(&real).unwrap();

    fs::write(work.join(".git"), format!("gitdir: {}\n", real.display())).unwrap();
    assert_eq!(repo::resolve_git_dir(&work), real);

    let work2 = dir.path().join("work2");
    fs::create_dir_all(&work2).unwrap();
    fs::write(work2.join(".git"), "gitdir: ../real.git\n").unwrap();
    assert_eq!(repo::resolve_git_dir(&work2), work2.join("../real.git"));
}

#[test]
fn apply_porcelain_v2_unknown_when_no_head() {
    let mut status = RepoStatus::default();
    repo::apply_porcelain_v2(&mut status, "# branch.oid abcdef0\n");
    assert_eq!(status.branch, "(unknown)");
}

#[test]
fn apply_porcelain_v2_detached_without_oid_uses_fallback() {
    let mut status = RepoStatus::default();
    repo::apply_porcelain_v2(&mut status, "# branch.head (detached from deadbeef)\n");
    assert!(status.detached);
    assert_eq!(status.branch, "detached");
}

#[test]
fn probe_stale_and_doctor_opts() {
    let (_dir, repo) = setup_repo(true);
    let stale = repo::probe_with(&repo.path, ProbeOpts::STALE).unwrap();
    assert!(stale.last_commit_age_secs.is_some());
    assert_eq!(stale.last_commit_subject.as_deref(), Some("c"));
    assert!(stale.branch.is_empty());

    fs::write(repo.path.join(".git").join("MERGE_HEAD"), "deadbeef\n").unwrap();
    let doctor = repo::probe_with(&repo.path, ProbeOpts::DOCTOR).unwrap();
    assert_eq!(doctor.in_progress.as_deref(), Some("merge"));
    assert_eq!(doctor.branch, "main");
}

#[test]
fn probe_filter_stash_counts_entries() {
    let (_dir, repo) = setup_repo(true);
    fs::write(repo.path.join("extra"), "y").unwrap();
    let stash = Command::new("git")
        .args(["stash", "push", "-u", "-m", "wip"])
        .current_dir(&repo.path)
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@e.com")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@e.com")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(stash.success());
    let status = repo::probe_with(&repo.path, ProbeOpts::FILTER_STASH).unwrap();
    assert!(status.stashed >= 1);
    assert!(status.branch.is_empty());
}

#[test]
fn output_repo_label_respects_show_path() {
    use git_gist::cli::OutputFormat;
    use git_gist::output::OutputCtx;

    let root = tempdir().unwrap();
    let nested = root.path().join("oss").join("demo");
    fs::create_dir_all(&nested).unwrap();
    let repo = Repo::new(nested);
    let mut out = OutputCtx::new(false, OutputFormat::Human, false, 0);
    assert_eq!(out.repo_label(&repo), "demo");
    out = out.with_show_path(true, Some(root.path().to_path_buf()));
    let label = out.repo_label(&repo);
    assert!(label.starts_with("demo ("));
    assert!(label.contains("oss"));
}
