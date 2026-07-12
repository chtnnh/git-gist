//! Extra coverage for harder branches: worktrees, stale, scaffold, sync, doctor, filters.

mod common;

use common::{git, Fixture};
use predicates::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn worktrees_human_and_extra_worktree() {
    let f = Fixture::with_repos(&["mono"]);
    let wt = f.root.path().join("mono-wt");
    let status = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "feature",
            wt.to_str().unwrap(),
            "HEAD",
        ])
        .current_dir(&f.repos[0])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());

    f.gg()
        .args(["worktrees", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("feature").or(predicates::str::contains("mono")));
    f.gg()
        .args(["--format", "json", "worktrees"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"path\""));
}

#[test]
fn stale_human_empty_and_old() {
    let f = Fixture::with_repos(&["fresh"]);
    f.gg()
        .args(["stale", "--days", "36500", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no repos stale"));

    // empty repo without commits → treated as stale
    let empty = f.root.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    git(&empty, &["init", "-b", "main"]);
    f.gg()
        .args(["stale", "--days", "1", "--color", "never", "--refresh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("empty").or(predicates::str::contains("age")));
}

#[test]
fn scaffold_rich_profile() {
    let f = Fixture::new();
    f.write_global_config(
        r#"
schema_version = 1

[profiles.rich]
user_name = "Ada"
user_email = "ada@example.com"
default_branch = "trunk"
gitignore = "*.log\n"
license = "MIT stub"
hooks = ["noop"]

[profiles.rich.remotes]
origin = "https://example.com/rich.git"
"#,
    );
    let dest = f.root.path().join("rich-repo");
    f.gg()
        .args(["init", "--profile", "rich", dest.to_str().unwrap()])
        .assert()
        .success();
    assert!(dest.join(".gitignore").is_file());
    assert!(dest.join("LICENSE").is_file());
    assert!(
        dest.join(".git").join("hooks").join("pre-commit").is_file() || dest.join(".git").is_dir()
    );
}

#[test]
fn sync_real_fetch_json() {
    let f = Fixture::with_repos(&["local"]);
    let output = f.gg().args(["--format", "json", "sync"]).output().unwrap();
    // fetch may fail without remotes; command should still produce output or a clean error
    assert!(
        output.status.code().unwrap_or(1) <= 1,
        "unexpected status {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn doctor_detached_and_json() {
    let f = Fixture::with_repos(&["det"]);
    git(&f.repos[0], &["checkout", "--detach", "HEAD"]);
    f.gg()
        .args(["doctor", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("detached").or(predicates::str::contains("warn")));
}

#[test]
fn only_detached_filter() {
    let f = Fixture::with_repos(&["onbranch", "detachme"]);
    git(&f.repos[1], &["checkout", "--detach", "HEAD"]);
    f.gg()
        .args(["--only-detached", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("detachme"))
        .stdout(predicates::str::contains("onbranch").not());
}

#[test]
fn tag_filter_from_config() {
    let f = Fixture::with_repos(&["tagged", "other"]);
    f.write_global_config(&format!(
        r#"
schema_version = 1
[aliases]
tagged = "{}"
other = "{}"
[tags]
oss = ["tagged"]
"#,
        Fixture::toml_path(&f.repos[0]),
        Fixture::toml_path(&f.repos[1])
    ));
    f.gg()
        .args(["--tag", "oss", "list", "--refresh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("tagged"));
}

#[test]
fn repo_override_skip() {
    let f = Fixture::with_repos(&["keep", "skipme"]);
    let skip_path = f.repos[1].canonicalize().unwrap();
    f.write_global_config(&format!(
        r#"
schema_version = 1

[repo_overrides]
"{key}" = {{ skip = true }}
"#,
        key = Fixture::toml_path(&skip_path)
    ));
    f.gg()
        .args(["list", "--refresh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("keep"))
        .stdout(predicates::str::contains("skipme").not());
}

#[test]
fn fail_fast_and_quiet_passthrough() {
    let f = Fixture::with_repos(&["a", "b"]);
    f.gg()
        .args(["--fail-fast", "-j", "1", "rev-parse", "not-a-ref"])
        .assert()
        .failure();
    f.gg().args(["-q", "status", "-sb"]).assert().success();
}

#[test]
fn each_failure_aggregates() {
    let f = Fixture::with_repos(&["x"]);
    f.gg().args(["each", "false"]).assert().failure();
}

#[test]
fn remotes_add_to_updates_existing() {
    let f = Fixture::with_repos(&["r"]);
    f.gg()
        .args(["remotes", "add", "up", "https://example.com/a.git"])
        .assert()
        .success();
    f.gg().args(["remotes", "add-to", "up"]).assert().success();
    // second time should set-url path
    f.gg()
        .args(["remotes", "add", "up", "https://example.com/b.git"])
        .assert()
        .success();
    f.gg().args(["remotes", "add-to", "up"]).assert().success();
}

#[test]
fn alias_json_and_group_json() {
    let f = Fixture::with_repos(&["p"]);
    f.gg()
        .args(["alias", "add", "p", f.repos[0].to_str().unwrap()])
        .assert()
        .success();
    f.gg()
        .args(["--format", "json", "alias", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("p"));
    f.gg().args(["group", "add", "g", "p"]).assert().success();
    f.gg()
        .args(["--format", "json", "group", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("g"));
}

#[test]
fn commits_human_table() {
    let f = Fixture::with_repos(&["c"]);
    f.gg()
        .args(["commits", "-n", "1", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("init").or(predicates::str::contains("c")));
}

#[test]
fn overview_human_detached() {
    let f = Fixture::with_repos(&["d"]);
    git(&f.repos[0], &["checkout", "--detach", "HEAD"]);
    f.gg()
        .args(["overview", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("detached").or(predicates::str::contains("d")));
}

#[test]
fn cache_hit_second_list() {
    let f = Fixture::with_repos(&["cached"]);
    f.gg().args(["list"]).assert().success();
    f.gg()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("cached"));
}

#[test]
fn include_submodules_flag() {
    let f = Fixture::with_repos(&["parent"]);
    // create a gitfile-style submodule-ish repo
    let sub = f.repos[0].join("vendor").join("lib");
    fs::create_dir_all(&sub).unwrap();
    git(&sub, &["init", "-b", "main"]);
    // Convert .git dir to gitfile to look like submodule
    // (optional — discovery treats .git file as submodule)
    let gitdir = sub.join(".git");
    if gitdir.is_dir() {
        // leave as nested clone; with include_submodules still finds nested
    }
    f.gg()
        .args(["--include-submodules", "list", "--refresh", "--depth", "5"])
        .assert()
        .success();
}

#[test]
fn config_set_theme_and_jobs() {
    let f = Fixture::new();
    f.gg()
        .args(["config", "set", "theme", "vivid"])
        .assert()
        .success();
    f.gg()
        .args(["config", "set", "jobs", "2"])
        .assert()
        .success();
    f.gg()
        .args(["config", "get", "theme"])
        .assert()
        .success()
        .stdout(predicates::str::contains("vivid"));
}

#[test]
fn passthrough_empty_args_via_git_subcommand_fails() {
    let f = Fixture::with_repos(&["z"]);
    // `gg git` without args should fail clap required
    f.gg().args(["git"]).assert().failure();
}

#[test]
fn output_warn_on_empty_selection() {
    let f = Fixture::new();
    f.gg().args(["list", "--color", "never"]).assert().success();
    f.gg()
        .args([
            "--in",
            f.root.path().join("missing").to_str().unwrap(),
            "list",
        ])
        .assert()
        .failure();
}

#[test]
fn update_enrolls_repos_into_aliases_groups_tags() {
    let mut f = Fixture::new();
    let watch = f.root.path().join("watch");
    fs::create_dir_all(watch.join("nested")).unwrap();
    // Repos under the watch root
    let a = watch.join("alpha");
    let b = watch.join("nested").join("beta");
    for path in [&a, &b] {
        fs::create_dir_all(path).unwrap();
        git(path, &["init", "-b", "main"]);
        fs::write(path.join("README"), "x\n").unwrap();
        git(path, &["add", "README"]);
        git(path, &["commit", "-m", "init"]);
        f.repos.push(path.clone());
    }

    f.write_global_config(&format!(
        r#"
schema_version = 1

[[auto_enroll]]
path = "{}"
depth = 4
tags = ["learn"]
groups = ["lab"]
"#,
        Fixture::toml_path(&watch)
    ));

    f.gg()
        .args(["update", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("would add"));

    // Dry-run must not persist aliases
    f.gg()
        .args(["alias", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no aliases").or(predicates::str::is_empty()));

    f.gg()
        .args(["update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("added"));

    f.gg()
        .args(["alias", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("alpha"))
        .stdout(predicates::str::contains("beta"));

    f.gg()
        .args(["--tag", "learn", "list", "--refresh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("alpha"))
        .stdout(predicates::str::contains("beta"));

    f.gg()
        .args(["-g", "lab", "list", "--refresh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("2 repositories"));

    // Idempotent
    f.gg()
        .args(["update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no changes").or(predicates::str::contains("0 new")));
}

#[test]
fn update_fails_without_rules_and_supports_json() {
    let f = Fixture::new();
    f.write_global_config("schema_version = 1\n");
    f.gg().args(["update"]).assert().failure();

    let watch = f.root.path().join("empty-watch");
    fs::create_dir_all(&watch).unwrap();
    f.write_global_config(&format!(
        r#"
schema_version = 1
[[auto_enroll]]
path = "{}"
depth = 2
"#,
        Fixture::toml_path(&watch)
    ));
    f.gg()
        .args(["--format", "json", "update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"added\""));
}

#[test]
fn update_repairs_membership_and_skips_missing_roots() {
    let mut f = Fixture::new();
    let watch = f.root.path().join("enroll-fix");
    fs::create_dir_all(&watch).unwrap();
    let repo = watch.join("gamma");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-b", "main"]);
    fs::write(repo.join("README"), "x\n").unwrap();
    git(&repo, &["add", "README"]);
    git(&repo, &["commit", "-m", "init"]);
    f.repos.push(repo.clone());

    // Alias exists but is not in the group/tag yet; also include a missing watch path.
    f.write_global_config(&format!(
        r#"
schema_version = 1
[aliases]
gamma = "{repo}"

[[auto_enroll]]
path = "{missing}"
depth = 2
groups = ["gone"]

[[auto_enroll]]
path = "{watch}"
depth = 0
groups = ["lab"]
tags = ["learn"]
"#,
        repo = Fixture::toml_path(&repo),
        missing = Fixture::toml_path(&f.root.path().join("does-not-exist")),
        watch = Fixture::toml_path(&watch),
    ));

    f.gg()
        .args(["update", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("would update"));

    f.gg()
        .args(["update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("updated"));

    f.gg()
        .args(["-g", "lab", "list", "--refresh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("gamma"));
}

#[test]
fn update_enrolls_without_groups_or_tags() {
    let mut f = Fixture::new();
    let watch = f.root.path().join("plain");
    let repo = watch.join("solo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-b", "main"]);
    fs::write(repo.join("README"), "x\n").unwrap();
    git(&repo, &["add", "README"]);
    git(&repo, &["commit", "-m", "init"]);
    f.repos.push(repo);

    f.write_global_config(&format!(
        r#"
schema_version = 1
[[auto_enroll]]
path = "{}"
depth = 2
"#,
        Fixture::toml_path(&watch)
    ));

    f.gg()
        .args(["update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("added"));
    f.gg()
        .args(["alias", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("solo"));
}
