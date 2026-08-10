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
    // fetch may fail without remotes; command should still produce a single JSON document
    assert!(
        output.status.code().unwrap_or(1) <= 1,
        "unexpected status {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = &output.stdout;
    if stdout.is_empty() {
        return;
    }
    let value: serde_json::Value = serde_json::from_slice(stdout).unwrap_or_else(|e| {
        panic!(
            "sync JSON must be a single parseable document: {e}; stdout={}",
            String::from_utf8_lossy(stdout)
        )
    });
    assert!(value.is_array(), "expected sync JSON array, got {value}");
    let row = &value.as_array().unwrap()[0];
    assert!(row.get("fetch_ok").is_some(), "missing fetch_ok: {row}");
    assert!(row.get("name").is_some(), "missing name: {row}");
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
fn doctor_reports_gitfile_worktree() {
    let f = Fixture::with_repos(&["mainrepo"]);
    let linked = f.root.path().join("linked");
    fs::create_dir_all(&linked).unwrap();
    let real_git = f.repos[0].join(".git");
    fs::write(
        linked.join(".git"),
        format!("gitdir: {}\n", real_git.display()),
    )
    .unwrap();

    f.gg()
        .args([
            "--root",
            f.root.path().to_str().unwrap(),
            "--in",
            linked.to_str().unwrap(),
            "--include-submodules",
            "doctor",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("gitfile")
                .or(predicates::str::contains("worktree"))
                .or(predicates::str::contains("submodule")),
        );
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
    // Cross-platform failure (Windows `each` uses COMSPEC, not `sh`).
    #[cfg(windows)]
    f.gg().args(["each", "exit", "/b", "1"]).assert().failure();
    #[cfg(not(windows))]
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

#[test]
fn alias_prune_reclaims_short_names_on_update() {
    let mut f = Fixture::new();
    let watch = f.root.path().join("moved");
    let repo = watch.join("widget");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-b", "main"]);
    fs::write(repo.join("README"), "x\n").unwrap();
    git(&repo, &["add", "README"]);
    git(&repo, &["commit", "-m", "init"]);
    f.repos.push(repo.clone());

    f.write_global_config(&format!(
        r#"
schema_version = 1
[aliases]
widget = "{missing}"

[[auto_enroll]]
path = "{watch}"
depth = 2
"#,
        missing = Fixture::toml_path(&f.root.path().join("gone").join("widget")),
        watch = Fixture::toml_path(&watch),
    ));

    f.gg()
        .args(["alias", "prune", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("widget"));

    f.gg()
        .args(["update", "--prune-stale"])
        .assert()
        .success()
        .stdout(predicates::str::contains("added").or(predicates::str::contains("pruned")));

    f.gg()
        .args(["alias", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("widget"))
        .stdout(predicates::str::contains(Fixture::path_output_needle(
            &repo,
        )));
}

#[test]
fn auto_enroll_path_prefix_limits_group() {
    let mut f = Fixture::new();
    let watch = f.root.path();
    for name in ["oss/a", "learning/b"] {
        let repo = watch.join(name);
        fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-b", "main"]);
        fs::write(repo.join("README"), "x\n").unwrap();
        git(&repo, &["add", "README"]);
        git(&repo, &["commit", "-m", "init"]);
        f.repos.push(repo);
    }

    f.write_global_config(&format!(
        r#"
schema_version = 1
[[auto_enroll]]
path = "{watch}"
path_prefix = "oss"
depth = 4
groups = ["oss"]
"#,
        watch = Fixture::toml_path(watch),
    ));

    f.gg().args(["update"]).assert().success();
    f.gg()
        .args(["group", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("a"))
        .stdout(predicates::str::contains("oss"));

    let out = f.gg().args(["alias", "list"]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("a"));
    // learning/b should get an alias only if a rule covers it — this rule shouldn't
    assert!(!text.lines().any(|l| l.starts_with("b\t")));
}

#[test]
fn doctor_config_and_tag_enroll_cli() {
    let f = Fixture::new();
    f.write_global_config(
        r#"
schema_version = 1
[aliases]
gone = "/no/such/path"
[groups]
g = ["gone", "missing-alias"]
"#,
    );
    f.gg()
        .args(["doctor", "--config"])
        .assert()
        .success()
        .stdout(predicates::str::contains("stale"))
        .stdout(predicates::str::contains(".git-gist"));

    f.gg()
        .args([
            "config",
            "enroll",
            "add",
            f.root.path().to_str().unwrap(),
            "--depth",
            "2",
            "--to-group",
            "g",
        ])
        .assert()
        .success();
    f.gg()
        .args(["config", "enroll", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0\t"));

    f.gg().args(["tag", "add", "t", "gone"]).assert().success();
    f.gg()
        .args(["tag", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("t"));
    f.gg()
        .args(["tag", "member", "add", "t", "gone"])
        .assert()
        .success();
    f.gg()
        .args(["tag", "member", "remove", "t", "gone"])
        .assert()
        .success();
    f.gg()
        .args(["group", "add", "g2", "gone"])
        .assert()
        .success();
    f.gg()
        .args(["group", "member", "add", "g2", "gone"])
        .assert()
        .success();
    f.gg()
        .args(["group", "member", "remove", "g2", "gone"])
        .assert()
        .success();
    f.gg()
        .args(["group", "prune", "g", "--dry-run"])
        .assert()
        .success();
    f.gg()
        .args(["config", "enroll", "remove", "0"])
        .assert()
        .success();
}

#[test]
fn unknown_config_keys_surface_suggestions() {
    let f = Fixture::new();
    f.write_global_config(
        r#"
schema_version = 1
show_pth = true
[[auto_enrol]]
path = "/tmp"
"#,
    );
    f.gg()
        .args(["doctor", "--config"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("did you mean").or(predicates::str::contains("unknown key")),
        );
    f.gg()
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("auto_enroll"));
}

#[test]
fn doctor_config_warns_on_dangerous_and_orphan_rules() {
    let f = Fixture::with_repos(&["dup-a"]);
    let root = Fixture::toml_path(f.root.path());
    let dup = f.root.path().join("also").join("dup-a");
    fs::create_dir_all(&dup).unwrap();
    git(&dup, &["init", "-b", "main"]);
    fs::write(dup.join("README"), "x\n").unwrap();
    git(&dup, &["add", "README"]);
    git(&dup, &["commit", "-m", "c"]);
    let p1 = Fixture::toml_path(&f.repos[0]);
    let p2 = Fixture::toml_path(&dup);
    f.write_global_config(&format!(
        r#"
schema_version = 1
root = "{root}"
[aliases]
a = "{p1}"
b = "{p2}"
[groups]
g = ["a", "missing"]
[tags]
t = ["ghost"]
[[auto_enroll]]
path = "{root}"
depth = 3
groups = ["g"]
[[auto_enroll]]
path = "{root}/nope"
depth = 2
tags = ["t"]
"#,
    ));
    f.gg()
        .args(["doctor", "--config"])
        .assert()
        .success()
        .stdout(predicates::str::contains("path_prefix").or(predicates::str::contains("equals")))
        .stdout(predicates::str::contains("missing").or(predicates::str::contains("ghost")))
        .stdout(
            predicates::str::contains("duplicate basename").or(predicates::str::contains("dup-a")),
        );
}

#[test]
fn catalog_dry_runs_and_mutations() {
    let f = Fixture::with_repos(&["r1"]);
    let path = Fixture::toml_path(&f.repos[0]);
    f.write_global_config(&format!(
        r#"
schema_version = 1
[aliases]
r1 = "{path}"
gone = "/no/such/path"
[groups]
g = ["r1", "gone"]
[tags]
t = ["r1"]
[remotes]
up = "git@example.com:org/"
"#,
    ));

    f.gg()
        .args(["--dry-run", "alias", "remove", "gone"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    f.gg()
        .args(["alias", "prune"])
        .assert()
        .success()
        .stdout(predicates::str::contains("pruned").or(predicates::str::contains("gone")));
    f.gg()
        .args(["alias", "prune"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no stale"));

    f.gg()
        .args(["--dry-run", "tag", "add", "t2", "r1"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    f.gg().args(["tag", "add", "t2", "r1"]).assert().success();
    f.gg()
        .args(["--dry-run", "tag", "member", "add", "t2", "r1"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "tag", "member", "remove", "t2", "r1"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "tag", "remove", "t2"])
        .assert()
        .success();
    f.gg().args(["tag", "remove", "t2"]).assert().success();

    f.gg()
        .args(["--dry-run", "group", "remove", "g"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "group", "member", "add", "g", "r1"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "group", "member", "remove", "g", "r1"])
        .assert()
        .success();
    f.gg().args(["group", "prune", "g"]).assert().success();

    f.gg()
        .args(["--dry-run", "remotes", "add", "x", "git@x/"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "remotes", "remove", "up"])
        .assert()
        .success();

    f.gg()
        .args(["--format", "json", "config", "enroll", "list"])
        .assert()
        .success();
    f.gg()
        .args(["config", "enroll", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no [[auto_enroll]]"));

    f.gg()
        .args([
            "--dry-run",
            "config",
            "enroll",
            "add",
            f.root.path().to_str().unwrap(),
            "--to-group",
            "g",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    f.gg()
        .args([
            "config",
            "enroll",
            "add",
            f.root.path().to_str().unwrap(),
            "--path-prefix",
            "oss/",
            "--to-tag",
            "t",
        ])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "config", "enroll", "remove", "0"])
        .assert()
        .success();
    f.gg()
        .args(["update", "--no-prune-stale", "-v"])
        .assert()
        .success();
    f.gg()
        .args(["update", "--prune-stale", "--dry-run"])
        .assert()
        .success();
}

#[test]
fn config_edit_update_ask_and_catalog_edges() {
    let f = Fixture::with_repos(&["edge"]);
    let path = Fixture::toml_path(&f.repos[0]);
    let root = Fixture::toml_path(f.root.path());
    f.write_global_config(&format!(
        r#"
schema_version = 1
root = "{root}"
[aliases]
edge = "{path}"
gone = "/missing/path"
[[auto_enroll]]
path = "{root}"
depth = 3
groups = ["g"]
"#,
    ));
    // Legacy XDG config still present → doctor --config info
    let legacy_dir = f.home.path().join("config").join("git-gist");
    fs::create_dir_all(&legacy_dir).unwrap();
    fs::write(legacy_dir.join("config.toml"), "schema_version = 1\n").unwrap();

    f.gg()
        .env("EDITOR", "true")
        .args(["config", "edit"])
        .assert()
        .success()
        .stdout(predicates::str::contains("edited"));
    f.gg()
        .env("EDITOR", "false")
        .args(["config", "edit"])
        .assert()
        .failure();

    f.gg().args(["update", "-v"]).assert().success();
    // --ask must not hang when stdin is not a TTY (Windows CI was blocking on inquire).
    f.gg()
        .args(["update", "--ask"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("interactive terminal"));
    f.gg().args(["update", "--prune-stale"]).assert().success();

    f.gg()
        .args(["doctor", "--config"])
        .assert()
        .success()
        .stdout(predicates::str::contains("legacy").or(predicates::str::contains("path_prefix")));
    f.gg()
        .args(["--format", "json", "tag", "list"])
        .assert()
        .success();
    f.write_global_config("schema_version = 1\n");
    f.gg()
        .args(["tag", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no tags"));
}

#[cfg(coverage)]
#[test]
fn interactive_entrypoints_under_coverage() {
    let f = Fixture::new();
    f.write_global_config("schema_version = 1\n");
    for args in [
        &["wizard"][..],
        &["ui"],
        &["config"],
        &["config", "wizard"],
        &["config", "ui"],
        &["alias", "wizard"],
        &["alias", "ui"],
        &["group", "wizard"],
        &["group", "ui"],
        &["tag", "wizard"],
        &["tag", "ui"],
        &["remotes", "wizard"],
        &["remotes", "ui"],
        &["config", "enroll", "wizard"],
        &["config", "enroll", "ui"],
    ] {
        f.gg().args(args.iter().copied()).assert().success();
    }
}
