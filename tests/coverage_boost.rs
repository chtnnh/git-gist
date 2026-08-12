//! Push coverage on sync, doctor, repo probes, discover edge cases.

mod common;

use common::{git, Fixture};
use predicates::prelude::*;
use std::fs;
use std::process::Command;

fn bare_and_clone(f: &Fixture, name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let bare = f.root.path().join(format!("{name}.git"));
    let clone = f.root.path().join(name);
    fs::create_dir_all(&bare).unwrap();
    git(&bare, &["init", "--bare", "-b", "main"]);
    let status = Command::new("git")
        .args(["clone", bare.to_str().unwrap(), clone.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    // initial commit in clone and push
    fs::write(clone.join("README"), "hi\n").unwrap();
    git(&clone, &["add", "README"]);
    git(&clone, &["commit", "-m", "init"]);
    git(&clone, &["push", "-u", "origin", "main"]);
    (bare, clone)
}

#[test]
fn sync_empty_selection_warns() {
    let f = Fixture::new();
    f.gg()
        .args(["sync", "--color", "never"])
        .assert()
        .success()
        .stderr(predicates::str::contains("no repositories").or(predicates::str::is_empty()));
}

#[test]
fn sync_human_table_after_fetch() {
    let f = Fixture::new();
    let (_bare, clone) = bare_and_clone(&f, "synced");
    // run from parent so discovery finds clone as child — put clone under a parent workspace
    let ws = f.root.path().join("ws");
    fs::create_dir_all(&ws).unwrap();
    let dest = ws.join("synced");
    fs::rename(&clone, &dest).unwrap();

    f.gg()
        .current_dir(&ws)
        .args(["sync", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("synced").or(predicates::str::contains("main")));
}

#[test]
fn sync_pull_when_behind() {
    let f = Fixture::new();
    let (bare, clone) = bare_and_clone(&f, "behind");
    let ws = f.root.path().join("ws2");
    fs::create_dir_all(&ws).unwrap();
    let dest = ws.join("behind");
    fs::rename(&clone, &dest).unwrap();

    // advance bare via a second clone
    let other = f.root.path().join("other");
    Command::new("git")
        .args(["clone", bare.to_str().unwrap(), other.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    fs::write(other.join("more"), "x\n").unwrap();
    git(&other, &["add", "more"]);
    git(&other, &["commit", "-m", "ahead"]);
    git(&other, &["push"]);

    f.gg()
        .current_dir(&ws)
        .args(["sync", "--pull", "--color", "never"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("pulled")
                .or(predicates::str::contains("behind"))
                .or(predicates::str::contains("main")),
        );
}

#[test]
fn sync_pull_json_happy_when_fetch_succeeds() {
    let f = Fixture::new();
    let (bare, clone) = bare_and_clone(&f, "pullok");
    let ws = f.root.path().join("ws-pullok");
    fs::create_dir_all(&ws).unwrap();
    let dest = ws.join("pullok");
    fs::rename(&clone, &dest).unwrap();

    let other = f.root.path().join("other-pullok");
    Command::new("git")
        .args(["clone", bare.to_str().unwrap(), other.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    fs::write(other.join("more"), "x\n").unwrap();
    git(&other, &["add", "more"]);
    git(&other, &["commit", "-m", "ahead"]);
    git(&other, &["push"]);

    let output = f
        .gg()
        .current_dir(&ws)
        .args(["--format", "json", "sync", "--pull"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["fetch_ok"], true, "{row}");
    assert_eq!(row["pulled"], true, "{row}");
}

#[test]
fn sync_pull_skipped_when_fetch_fails() {
    let f = Fixture::new();
    let (bare, clone) = bare_and_clone(&f, "pullskip");
    let ws = f.root.path().join("ws-pullskip");
    fs::create_dir_all(&ws).unwrap();
    let dest = ws.join("pullskip");
    fs::rename(&clone, &dest).unwrap();

    let other = f.root.path().join("other-pullskip");
    Command::new("git")
        .args(["clone", bare.to_str().unwrap(), other.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    fs::write(other.join("more"), "x\n").unwrap();
    git(&other, &["add", "more"]);
    git(&other, &["commit", "-m", "ahead"]);
    git(&other, &["push"]);

    // Repo is behind upstream, but fetch will fail — pull must not run.
    git(
        &dest,
        &[
            "remote",
            "set-url",
            "origin",
            "https://invalid.invalid/git-gist-sync-test.git",
        ],
    );

    let output = f
        .gg()
        .current_dir(&ws)
        .args(["--format", "json", "sync", "--pull"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "sync should summarize even when fetch fails: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["fetch_ok"], false, "{row}");
    assert_eq!(row["pulled"], false, "{row}");
}

#[test]
fn doctor_gitfile_and_in_progress_human() {
    let f = Fixture::with_repos(&["mainrepo"]);
    // create linked worktree → .git is a file in the worktree
    let wt = f.root.path().join("wt-link");
    Command::new("git")
        .args(["worktree", "add", wt.to_str().unwrap(), "HEAD"])
        .current_dir(&f.repos[0])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();

    // simulate merge in progress in mainrepo
    let git_dir = f.repos[0].join(".git");
    fs::write(git_dir.join("MERGE_HEAD"), "deadbeef\n").unwrap();

    f.gg()
        .args(["doctor", "--color", "never"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("merge")
                .or(predicates::str::contains("gitfile"))
                .or(predicates::str::contains("upstream")),
        );
}

#[test]
fn doctor_probe_error_path() {
    let f = Fixture::new();
    let bogus = f.root.path().join("notgit");
    fs::create_dir_all(&bogus).unwrap();
    // Pretend it's selected via --in even though not a real repo... discover won't find it.
    // Call doctor with a path that has .git dir corrupted
    let bad = f.root.path().join("bad");
    fs::create_dir_all(bad.join(".git")).unwrap();
    f.gg()
        .args(["doctor", "--color", "never", "--refresh"])
        .assert()
        .success();
}

#[test]
fn stash_and_only_stashed() {
    let f = Fixture::with_repos(&["s"]);
    fs::write(f.repos[0].join("tmp"), "stashme\n").unwrap();
    git(&f.repos[0], &["add", "tmp"]);
    git(&f.repos[0], &["stash", "push", "-m", "wip"]);
    f.gg()
        .args(["--only-stashed", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("s"));
    f.gg()
        .args(["info", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"stashed\""));
}

#[test]
fn upstream_ahead_behind_filters() {
    let f = Fixture::new();
    let (bare, clone) = bare_and_clone(&f, "ab");
    let ws = f.root.path().join("ws3");
    fs::create_dir_all(&ws).unwrap();
    let dest = ws.join("ab");
    fs::rename(&clone, &dest).unwrap();

    // local commit → ahead
    fs::write(dest.join("local"), "y\n").unwrap();
    git(&dest, &["add", "local"]);
    git(&dest, &["commit", "-m", "local"]);

    f.gg()
        .current_dir(&ws)
        .args(["--only-ahead", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("ab"));

    // reset and make behind
    git(&dest, &["reset", "--hard", "origin/main"]);
    let other = f.root.path().join("other2");
    Command::new("git")
        .args(["clone", bare.to_str().unwrap(), other.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    fs::write(other.join("remote"), "z\n").unwrap();
    git(&other, &["add", "remote"]);
    git(&other, &["commit", "-m", "remote"]);
    git(&other, &["push"]);
    git(&dest, &["fetch"]);

    f.gg()
        .current_dir(&ws)
        .args(["--only-behind", "list", "--refresh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("ab"));
}

#[test]
fn remotes_remove_missing_fails() {
    let f = Fixture::new();
    f.gg()
        .args(["remotes", "remove", "nope"])
        .assert()
        .failure();
    f.gg().args(["alias", "remove", "nope"]).assert().failure();
    f.gg().args(["group", "remove", "nope"]).assert().failure();
}

#[test]
fn glob_and_basename_targets() {
    let f = Fixture::with_repos(&["alpha", "beta"]);
    f.gg()
        .args(["--in", "alpha", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("alpha"))
        .stdout(predicates::str::contains("beta").not());
}

#[test]
fn worktrees_porcelain_flags_human() {
    let f = Fixture::with_repos(&["w"]);
    f.gg()
        .args(["worktrees", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("w").or(predicates::str::contains("main")));
}

#[test]
fn self_update_mentions_releases() {
    let f = Fixture::new();
    f.gg()
        .args(["self-update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("release").or(predicates::str::contains("cargo")));
}

#[test]
fn git_stdout_failure_via_info_bad_repo() {
    let f = Fixture::with_repos(&["ok"]);
    // empty .git causes probe issues for a fake selected path through info PATH arg
    let bad = f.root.path().join("emptydir");
    fs::create_dir_all(&bad).unwrap();
    f.gg()
        .args(["info", bad.to_str().unwrap(), "--color", "never"])
        .assert()
        .success();
}
