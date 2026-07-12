//! Cross-command verification that global flags are honored.

mod common;

use common::{git, Fixture};
use predicates::prelude::*;
use std::fs;

#[test]
fn selection_flags_on_reporting_commands() {
    let f = Fixture::with_repos(&["keep", "drop"]);
    let keep = f.repos[0].to_str().unwrap();
    let drop = f.repos[1].to_str().unwrap();

    // Put global flags before the command so external passthrough (`status`)
    // does not swallow `--dry-run` into git argv.
    let cmds: &[&[&str]] = &[
        &["ov", "--format", "json"],
        &["list", "--format", "json"],
        &["info", "--format", "json"],
        &["commits", "-n", "1", "--format", "json"],
        &["worktrees", "--format", "json"],
        &["doctor", "--format", "json"],
        &["stale", "--days", "0", "--format", "json"],
        &["sync", "--format", "json"],
        &["each", "true"],
        &["status"],
    ];

    for cmd in cmds {
        let mut args = vec!["--in", keep, "--exclude", drop, "-j", "1", "--dry-run"];
        args.extend_from_slice(cmd);
        f.gg()
            .args(&args)
            .assert()
            .success()
            .stdout(predicates::str::contains("keep"))
            .stdout(predicates::str::contains("drop").not());
    }
}

#[test]
fn root_and_depth_on_list_and_overview() {
    let mut f = Fixture::new();
    let nested = f.root.path().join("lvl1").join("deep");
    fs::create_dir_all(&nested).unwrap();
    git(&nested, &["init", "-b", "main"]);
    fs::write(nested.join("README"), "x\n").unwrap();
    git(&nested, &["add", "README"]);
    git(&nested, &["commit", "-m", "init"]);
    f.repos.push(nested);

    // depth=1 from fixture root cannot see lvl1/deep
    f.gg()
        .args([
            "--root",
            f.root.path().to_str().unwrap(),
            "--depth",
            "1",
            "list",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("deep").not());

    f.gg()
        .args([
            "--root",
            f.root.path().to_str().unwrap(),
            "--depth",
            "3",
            "ov",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("deep"));
}

#[test]
fn metadata_commands_ignore_bad_selection_flags() {
    let f = Fixture::new();
    for args in [
        vec!["--in", "does-not-exist", "alias", "list"],
        vec!["--in", "does-not-exist", "group", "list"],
        vec!["--in", "does-not-exist", "config", "show"],
        vec!["--in", "does-not-exist", "hooks", "list"],
        vec!["--in", "does-not-exist", "remotes", "list"],
        vec!["--in", "does-not-exist", "self-update"],
    ] {
        f.gg().args(&args).assert().success();
    }
}

#[test]
fn dry_run_on_mutating_commands() {
    let f = Fixture::with_repos(&["app"]);
    let dest = f.root.path().join("would-scaffold");

    f.gg()
        .args([
            "--dry-run",
            "init",
            "--profile",
            "default",
            dest.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    assert!(!dest.join(".git").exists());

    let hook_path = f.repos[0].join(".git").join("hooks").join("pre-commit");
    assert!(!hook_path.exists());
    f.gg()
        .args(["--dry-run", "hooks", "install", "noop"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    assert!(!hook_path.exists(), "dry-run must not install hooks");

    f.write_global_config(
        r#"
schema_version = 1
[remotes]
up = "https://example.com/up.git"
"#,
    );
    f.gg()
        .args(["--dry-run", "remotes", "add-to", "up"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    let remotes = std::process::Command::new("git")
        .args(["remote"])
        .current_dir(&f.repos[0])
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&remotes.stdout).contains("up"),
        "dry-run must not add remotes"
    );

    f.gg()
        .args([
            "--dry-run",
            "alias",
            "add",
            "tmp",
            f.repos[0].to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    f.gg()
        .args(["alias", "list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("tmp").not());

    f.gg()
        .args(["--dry-run", "group", "add", "g", "tmp"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    f.gg()
        .args(["group", "list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"g\"").not());

    f.gg()
        .args(["--dry-run", "config", "set", "theme", "mono"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    f.gg()
        .args(["config", "get", "theme"])
        .assert()
        .success()
        .stdout(predicates::str::contains("mono").not());
}

#[test]
fn only_dirty_honored_by_info_path() {
    let f = Fixture::with_repos(&["clean", "dirty"]);
    fs::write(f.repos[1].join("x"), "dirt").unwrap();

    f.gg()
        .args([
            "--only-dirty",
            "info",
            f.repos[0].to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("clean").not());

    f.gg()
        .args([
            "--only-dirty",
            "info",
            f.repos[1].to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("dirty"));
}

#[test]
fn quiet_hides_successful_each_output() {
    let f = Fixture::with_repos(&["a"]);
    f.gg()
        .args(["-q", "each", "echo", "hello-from-each"])
        .assert()
        .success()
        .stdout(predicates::str::contains("hello-from-each").not());
}

#[test]
fn format_json_on_catalog_commands() {
    let f = Fixture::new();
    f.gg()
        .args(["--format", "json", "alias", "list"])
        .assert()
        .success();
    f.gg()
        .args(["--format", "json", "group", "list"])
        .assert()
        .success();
    f.gg()
        .args(["--format", "json", "hooks", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("{"));
    f.gg()
        .args(["--format", "json", "remotes", "list"])
        .assert()
        .success();
    f.gg()
        .args(["--format", "json", "config", "show"])
        .assert()
        .success()
        .stdout(predicates::str::contains("schema_version"));
}
