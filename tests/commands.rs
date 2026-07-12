//! Integration tests covering every `gg` command and major flags.

mod common;

use common::{git, Fixture};
use predicates::prelude::*;
use std::fs;

#[test]
fn default_invokes_overview() {
    let f = Fixture::with_repos(&["alpha"]);
    f.gg()
        .assert()
        .success()
        .stdout(predicates::str::contains("alpha"));
}

#[test]
fn overview_and_alias_ov() {
    let f = Fixture::with_repos(&["svc"]);
    f.gg()
        .args(["overview", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("svc"));
    f.gg()
        .args(["ov", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"branch\""));
}

#[test]
fn list_and_ls_refresh() {
    let f = Fixture::with_repos(&["a", "b"]);
    f.gg()
        .args(["list", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("a"))
        .stdout(predicates::str::contains("b"));
    f.gg()
        .args(["ls", "--refresh", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"path\""));
}

#[test]
fn info_selection_and_path() {
    let f = Fixture::with_repos(&["app"]);
    f.gg()
        .args(["info", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("branch:"));
    f.gg()
        .args(["info", f.repos[0].to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"name\""));
}

#[test]
fn commits_top_n() {
    let f = Fixture::with_repos(&["repo"]);
    fs::write(f.repos[0].join("x"), "1").unwrap();
    git(&f.repos[0], &["add", "x"]);
    git(&f.repos[0], &["commit", "-m", "second"]);
    f.gg()
        .args(["commits", "-n", "2", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("second"));
}

#[test]
fn worktrees_lists_main() {
    let f = Fixture::with_repos(&["wt"]);
    f.gg()
        .args(["worktrees", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("wt"));
}

#[test]
fn doctor_finds_git() {
    let f = Fixture::with_repos(&["d"]);
    f.gg()
        .args(["doctor", "--color", "never"])
        .assert()
        .success()
        .stdout(predicates::str::contains("found git"));
}

#[test]
fn each_runs_shell() {
    let f = Fixture::with_repos(&["e"]);
    f.gg()
        .args(["each", "pwd"])
        .assert()
        .success()
        .stdout(predicates::str::contains("e"));
}

#[test]
fn each_dry_run() {
    let f = Fixture::with_repos(&["e"]);
    f.gg()
        .args(["--dry-run", "each", "false"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
}

#[test]
fn config_show_path_get_set() {
    let f = Fixture::with_repos(&["c"]);
    f.gg().args(["config", "show"]).assert().success();
    f.gg()
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicates::str::contains("config.toml"));
    f.gg()
        .args(["config", "set", "depth", "4"])
        .assert()
        .success();
    f.gg()
        .args(["config", "get", "depth"])
        .assert()
        .success()
        .stdout(predicates::str::contains("4"));
    f.gg()
        .args(["config", "path", "--local"])
        .assert()
        .success();
}

#[test]
fn alias_crud() {
    let f = Fixture::with_repos(&["api"]);
    let path = f.repos[0].to_str().unwrap();
    f.gg()
        .args(["alias", "add", "api", path])
        .assert()
        .success();
    f.gg()
        .args(["alias", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("api"));
    f.gg()
        .args(["--in", "api", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("api"));
    f.gg().args(["alias", "remove", "api"]).assert().success();
    f.gg()
        .args(["alias", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("api").not());
}

#[test]
fn group_crud_and_select() {
    let f = Fixture::with_repos(&["one", "two"]);
    f.gg()
        .args(["alias", "add", "one", f.repos[0].to_str().unwrap()])
        .assert()
        .success();
    f.gg()
        .args(["alias", "add", "two", f.repos[1].to_str().unwrap()])
        .assert()
        .success();
    f.gg()
        .args(["group", "add", "work", "one", "two"])
        .assert()
        .success();
    f.gg()
        .args(["group", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("work"));
    f.gg()
        .args(["-g", "work", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("one"))
        .stdout(predicates::str::contains("two"));
    f.gg().args(["group", "remove", "work"]).assert().success();
}

#[test]
fn init_and_scaffold() {
    let f = Fixture::new();
    let dest = f.root.path().join("newsvc");
    f.gg()
        .args(["init", "--profile", "default", dest.to_str().unwrap()])
        .assert()
        .success();
    assert!(dest.join(".git").exists() || dest.join(".git").is_file());

    let dest2 = f.root.path().join("scaffolded");
    f.gg()
        .args(["scaffold", "--profile", "default", dest2.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn hooks_list_and_install() {
    let f = Fixture::with_repos(&["h"]);
    f.gg()
        .args(["hooks", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("noop"));
    f.gg()
        .args(["hooks", "install", "commit-msg-required"])
        .assert()
        .success();
    let hook = f.repos[0].join(".git").join("hooks").join("commit-msg");
    assert!(hook.is_file());
}

#[test]
fn remotes_catalog_and_add_to() {
    let f = Fixture::with_repos(&["r"]);
    f.gg()
        .args(["remotes", "add", "mirror", "https://example.com/r.git"])
        .assert()
        .success();
    f.gg()
        .args(["remotes", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("mirror"));
    f.gg()
        .args(["remotes", "add-to", "mirror", "--as-name", "mirror"])
        .assert()
        .success();
    f.gg()
        .args(["remotes", "remove", "mirror"])
        .assert()
        .success();
}

#[test]
fn sync_and_sync_dry_run() {
    let f = Fixture::with_repos(&["s"]);
    f.gg()
        .args(["--dry-run", "sync"])
        .assert()
        .success()
        .stdout(predicates::str::contains("fetch"));
    // sync without remote may still succeed with summary (fetch may fail per-repo)
    let _ = f.gg().args(["sync", "--format", "json"]).assert();
}

#[test]
fn stale_reports_or_empty() {
    let f = Fixture::with_repos(&["old"]);
    f.gg()
        .args(["stale", "--days", "0", "--format", "json"])
        .assert()
        .success();
}

#[test]
fn git_escape_hatch_and_passthrough() {
    let f = Fixture::with_repos(&["g"]);
    f.gg()
        .args(["git", "--", "status", "-sb"])
        .assert()
        .success();
    f.gg()
        .args(["status", "-sb"])
        .assert()
        .success()
        .stdout(predicates::str::contains("g"));
}

#[test]
fn completions_all_shells() {
    let f = Fixture::new();
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        f.gg().args(["completions", shell]).assert().success();
    }
}

#[test]
fn man_stdout_and_file() {
    let f = Fixture::new();
    f.gg().args(["man"]).assert().success();
    let out = f.root.path().join("gg.1");
    f.gg()
        .args(["man", "--output", out.to_str().unwrap()])
        .assert()
        .success();
    assert!(out.is_file());
}

#[test]
fn version_and_self_update() {
    let f = Fixture::new();
    f.gg()
        .args(["version"])
        .assert()
        .success()
        .stdout(predicates::str::contains("gg "));
    f.gg().args(["self-update"]).assert().success();
}

#[test]
fn targeting_exclude_and_root() {
    let f = Fixture::with_repos(&["keep", "drop"]);
    f.gg()
        .args(["--exclude", f.repos[1].to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("keep"))
        .stdout(predicates::str::contains("drop").not());

    f.gg()
        .args(["--root", f.root.path().to_str().unwrap(), "list"])
        .assert()
        .success();
}

#[test]
fn root_flag_ignores_aliases_outside_root() {
    let mut f = Fixture::with_repos(&["local"]);
    let outside = f.home.path().join("elsewhere");
    fs::create_dir_all(&outside).unwrap();
    git(&outside, &["init", "-b", "main"]);
    fs::write(outside.join("README"), "x\n").unwrap();
    git(&outside, &["add", "README"]);
    git(&outside, &["commit", "-m", "init"]);
    f.repos.push(outside.clone());

    f.write_global_config(&format!(
        r#"
schema_version = 1
root = "{root}"
[aliases]
local = "{local}"
elsewhere = "{elsewhere}"
"#,
        root = Fixture::toml_path(f.root.path()),
        local = Fixture::toml_path(&f.repos[0]),
        elsewhere = Fixture::toml_path(&outside),
    ));

    let nested = f.root.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    // Scope to an empty subdirectory: discovery finds nothing under it, and
    // the out-of-root alias must not leak into the selection.
    f.gg()
        .args(["--root", nested.to_str().unwrap(), "ov", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("elsewhere").not())
        .stdout(predicates::str::contains("local").not());

    // Explicit -i still reaches an alias outside the root.
    f.gg()
        .args([
            "--root",
            nested.to_str().unwrap(),
            "--in",
            "elsewhere",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("elsewhere"));
}

#[test]
fn root_flag_includes_root_when_it_is_a_repo() {
    let f = Fixture::new();
    git(f.root.path(), &["init", "-b", "main"]);
    fs::write(f.root.path().join("README"), "x\n").unwrap();
    git(f.root.path(), &["add", "README"]);
    git(f.root.path(), &["commit", "-m", "init"]);

    f.gg()
        .args(["--root", f.root.path().to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            f.root.path().file_name().unwrap().to_str().unwrap(),
        ));
}

#[test]
fn only_dirty_and_only_clean() {
    let f = Fixture::with_repos(&["clean", "dirty"]);
    fs::write(f.repos[1].join("dirty.txt"), "x").unwrap();
    f.gg()
        .args(["--only-dirty", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dirty"))
        .stdout(predicates::str::contains("clean").not());
    f.gg()
        .args(["--only-clean", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("clean"))
        .stdout(predicates::str::contains("dirty").not());
}

#[test]
fn ggignore_and_local_config() {
    let f = Fixture::with_repos(&["visible"]);
    let hidden = f.root.path().join("hidden-area").join("secret");
    fs::create_dir_all(&hidden).unwrap();
    git(&hidden, &["init", "-b", "main"]);
    f.write_ggignore("hidden-area/**\n");
    f.write_local_config("depth = 8\n");
    f.gg()
        .args(["list", "--refresh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("visible"))
        .stdout(predicates::str::contains("secret").not());
}

#[test]
fn passthrough_dry_run_timing_json() {
    let f = Fixture::with_repos(&["p"]);
    f.gg()
        .args(["--dry-run", "--timing", "status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dry-run"));
    f.gg()
        .args(["--format", "json", "rev-parse", "--is-inside-work-tree"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"success\""));
}

#[test]
fn themes_and_ndjson() {
    let f = Fixture::with_repos(&["t"]);
    f.gg()
        .args(["--theme", "vivid", "--color", "always", "overview"])
        .assert()
        .success();
    f.gg()
        .args(["--theme", "mono", "--format", "ndjson", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"name\""));
}

#[test]
fn sync_pull_flag() {
    let f = Fixture::with_repos(&["s"]);
    f.gg()
        .args(["--dry-run", "sync", "--pull"])
        .assert()
        .success()
        .stdout(predicates::str::contains("pull"));
}

#[test]
fn config_json_show() {
    let f = Fixture::with_repos(&["c"]);
    f.gg()
        .args(["--format", "json", "config", "show"])
        .assert()
        .success()
        .stdout(predicates::str::contains("schema_version"));
}

#[test]
fn hooks_list_json() {
    let f = Fixture::new();
    f.gg()
        .args(["--format", "json", "hooks", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("noop"));
}

#[test]
fn remotes_list_json_empty() {
    let f = Fixture::new();
    f.gg()
        .args(["--format", "json", "remotes", "list"])
        .assert()
        .success();
}

#[test]
fn help_works() {
    let f = Fixture::new();
    f.gg().args(["--help"]).assert().success();
    f.gg().args(["overview", "--help"]).assert().success();
}
