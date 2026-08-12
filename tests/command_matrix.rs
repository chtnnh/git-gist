//! Systematic happy + unhappy CLI coverage for every `gg` command/subcommand.
//!
//! Review checklist — each row is exercised by the named test(s):
//!
//! | Command / subcommand | Happy | Unhappy |
//! |----------------------|-------|---------|
//! | (default) / overview / ov | `happy_default_overview`, `happy_overview_and_ov` | `unhappy_empty_selection_warns` |
//! | list / ls [--refresh] | `happy_list_ls_refresh` | `unhappy_list_worktrees_stale_empty_selection` |
//! | info [PATH] | `happy_info_selection_and_path` | `unhappy_info_non_repo_path_soft_success` |
//! | commits [-n] | `happy_commits` | `unhappy_commits_empty_selection_shows_empty_table`, `unhappy_commits_on_repo_without_commits` |
//! | worktrees | `happy_worktrees` | `unhappy_list_worktrees_stale_empty_selection` |
//! | doctor [--config] | `happy_doctor` | `unhappy_list_worktrees_stale_empty_selection` (empty repos); probe warnings in `coverage_boost` |
//! | each CMD… | `happy_each` | `unhappy_each_empty_command`, `unhappy_each_command_failure` |
//! | config show/path/get/set/edit | `happy_config_show_path_get_set`, `happy_config_edit` | `unhappy_config_get_set_unknown_key`, `unhappy_config_edit_bad_editor` |
//! | config enroll list/add/remove | `happy_config_enroll` (+ `--to-group` / `--to-tag`) | `unhappy_config_enroll_remove_bad_index` |
//! | config / wizard / ui | `happy_interactive_skipped_under_coverage` (coverage only) | `unhappy_interactive_requires_tty` (non-coverage) |
//! | alias list/add/prune/remove | `happy_alias_subcommands`, `happy_catalog_remove_dry_run` | `unhappy_remove_missing_catalog_entries` |
//! | group list/add/member/prune/remove | `happy_group_subcommands`, `happy_catalog_remove_dry_run` | `unhappy_member_remove_missing_group_or_tag`, `unhappy_remove_missing_catalog_entries` |
//! | tag list/add/member/remove | `happy_tag_subcommands`, `happy_catalog_remove_dry_run` | `unhappy_member_remove_missing_group_or_tag`, `unhappy_tag_member_not_in_tag`, `unhappy_remove_missing_catalog_entries` |
//! | init / scaffold [--profile] | `happy_init_and_scaffold` | `unhappy_init_unknown_profile` |
//! | hooks list/install | `happy_hooks` | `unhappy_hooks_unknown_pack_and_empty_selection` |
//! | remotes list/add/add-to/remove | `happy_remotes_subcommands` (+ `--as-name`), `happy_catalog_remove_dry_run` | `unhappy_remotes_add_to_empty_selection`, `unhappy_remove_missing_catalog_entries` |
//! | sync [--pull] | `happy_sync_and_pull` | `unhappy_empty_selection_warns` (+ fetch gate: `coverage_boost`) |
//! | stale [--days] | `happy_stale` | `unhappy_list_worktrees_stale_empty_selection` |
//! | update [--prune-stale / --no-prune-stale / --ask] | `happy_update` | `unhappy_update_without_rules`, `unhappy_update_ask_without_tty_when_stale_present` |
//! | wizard / ui (hub) | see config interactive row | see config interactive row |
//! | git -- ARGS / external passthrough | `happy_git_and_passthrough` | `unhappy_git_subcommand_empty`, `unhappy_passthrough_misplaced_global_flag` |
//! | completions SHELL | `happy_completions_man_version` (all shells) | — |
//! | man [--output] | `happy_completions_man_version` | `unhappy_man_invalid_output_path` |
//! | version / self-update | `happy_completions_man_version` | — |
//! | global --in TARGET | — | `unhappy_selection_include_missing` |

mod common;

use common::{git, Fixture};
#[cfg(not(coverage))]
use predicates::prelude::PredicateBooleanExt;
use std::fs;

/// Repo `r` plus catalog entries for alias/group/tag/remote/enroll mutations.
fn catalog_fixture() -> Fixture {
    let f = Fixture::with_repos(&["r"]);
    let path = Fixture::toml_path(&f.repos[0]);
    f.write_global_config(&format!(
        r#"
schema_version = 1
root = "{root}"

[aliases]
r = "{path}"

[groups]
g = ["r"]

[tags]
t = ["r"]

[remotes]
up = "https://example.com/org/"

[[auto_enroll]]
path = "{root}"
depth = 2

[profiles.minimal]
default_branch = "main"
"#,
        root = Fixture::toml_path(f.root.path()),
        path = path,
    ));
    f
}

// --- Reporting & inspection (happy) ---

#[test]
fn happy_default_overview() {
    let f = Fixture::with_repos(&["alpha"]);
    f.gg().assert().success();
}

#[test]
fn happy_overview_and_ov() {
    let f = Fixture::with_repos(&["svc"]);
    f.gg().args(["overview"]).assert().success();
    f.gg().args(["ov", "--format", "json"]).assert().success();
}

#[test]
fn happy_list_ls_refresh() {
    let f = Fixture::with_repos(&["a", "b"]);
    f.gg().args(["list"]).assert().success();
    f.gg().args(["ls", "--refresh"]).assert().success();
}

#[test]
fn happy_info_selection_and_path() {
    let f = Fixture::with_repos(&["app"]);
    f.gg().args(["info"]).assert().success();
    f.gg()
        .args(["info", f.repos[0].to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn happy_commits() {
    let f = Fixture::with_repos(&["repo"]);
    f.gg().args(["commits", "-n", "1"]).assert().success();
    f.gg()
        .args(["commits", "--format", "json"])
        .assert()
        .success();
}

#[test]
fn happy_worktrees() {
    let f = Fixture::with_repos(&["wt"]);
    f.gg().args(["worktrees"]).assert().success();
}

#[test]
fn happy_doctor() {
    let f = Fixture::with_repos(&["d"]);
    f.gg().args(["doctor"]).assert().success();
    f.gg().args(["doctor", "--config"]).assert().success();
}

#[test]
fn happy_stale() {
    let f = Fixture::with_repos(&["fresh"]);
    f.gg().args(["stale", "--days", "90"]).assert().success();
    f.gg()
        .args(["stale", "--days", "36500", "--format", "json"])
        .assert()
        .success();
}

#[test]
fn happy_sync_and_pull() {
    let f = Fixture::with_repos(&["s"]);
    f.gg().args(["sync"]).assert().success();
    f.gg()
        .args(["sync", "--pull", "--dry-run"])
        .assert()
        .success();
    f.gg().args(["--format", "json", "sync"]).assert().success();
}

#[test]
fn happy_each() {
    let f = Fixture::with_repos(&["e"]);
    f.gg()
        .args(["each", "git", "rev-parse", "--show-toplevel"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "each", "true"])
        .assert()
        .success();
}

// --- Config subtree (happy) ---

#[test]
fn happy_config_show_path_get_set() {
    let f = catalog_fixture();
    f.gg().args(["config", "show"]).assert().success();
    f.gg().args(["config", "path"]).assert().success();
    f.gg()
        .args(["config", "path", "--local"])
        .assert()
        .success();
    f.gg()
        .args(["config", "set", "depth", "4"])
        .assert()
        .success();
    f.gg().args(["config", "get", "depth"]).assert().success();
}

#[test]
fn happy_config_edit() {
    let f = catalog_fixture();
    f.gg()
        .env("EDITOR", "true")
        .args(["config", "edit"])
        .assert()
        .success();
}

#[test]
fn happy_config_enroll() {
    let f = catalog_fixture();
    f.gg().args(["config", "enroll", "list"]).assert().success();
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
            "--to-tag",
            "t",
        ])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "config", "enroll", "remove", "0"])
        .assert()
        .success();
}

// --- Alias / group / tag / remotes (happy) ---

#[test]
fn happy_alias_subcommands() {
    let f = catalog_fixture();
    let p = f.root.path().join("extra");
    fs::create_dir_all(&p).unwrap();
    git(&p, &["init", "-b", "main"]);
    f.gg()
        .args(["alias", "list", "--format", "json"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "alias", "add", "extra", p.to_str().unwrap()])
        .assert()
        .success();
    f.gg().args(["alias", "prune"]).assert().success();
}

#[test]
fn happy_group_subcommands() {
    let f = catalog_fixture();
    f.gg().args(["group", "list"]).assert().success();
    f.gg()
        .args(["--dry-run", "group", "add", "newg", "r"])
        .assert()
        .success();
    f.gg()
        .args(["group", "member", "add", "g", "r"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "group", "member", "remove", "g", "r"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "group", "prune", "g"])
        .assert()
        .success();
    f.gg()
        .args([
            "--dry-run",
            "group",
            "prune",
            "g",
            "--under",
            f.root.path().to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn happy_tag_subcommands() {
    let f = catalog_fixture();
    f.gg().args(["tag", "list"]).assert().success();
    f.gg()
        .args(["--dry-run", "tag", "add", "newt", "r"])
        .assert()
        .success();
    f.gg()
        .args(["tag", "member", "add", "t", "r"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "tag", "member", "remove", "t", "r"])
        .assert()
        .success();
}

#[test]
fn happy_remotes_subcommands() {
    let f = catalog_fixture();
    f.gg().args(["remotes", "list"]).assert().success();
    f.gg()
        .args([
            "--dry-run",
            "remotes",
            "add",
            "mirror",
            "https://x.test/r.git",
        ])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "remotes", "add-to", "up"])
        .assert()
        .success();
    f.gg()
        .args([
            "--dry-run",
            "remotes",
            "add-to",
            "up",
            "--as-name",
            "upstream",
        ])
        .assert()
        .success();
}

// --- Scaffold / hooks / update (happy) ---

#[test]
fn happy_init_and_scaffold() {
    let f = catalog_fixture();
    let target = f.root.path().join("newproj");
    f.gg()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();
    let target2 = f.root.path().join("scaffolded");
    f.gg()
        .args([
            "scaffold",
            "--profile",
            "minimal",
            target2.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn happy_hooks() {
    let f = catalog_fixture();
    f.gg().args(["hooks", "list"]).assert().success();
    f.gg()
        .args(["--dry-run", "hooks", "install", "noop"])
        .assert()
        .success();
}

#[test]
fn happy_update() {
    let f = catalog_fixture();
    f.gg().args(["update", "--dry-run"]).assert().success();
    f.gg()
        .args(["update", "--prune-stale", "--dry-run"])
        .assert()
        .success();
    f.gg()
        .args(["update", "--no-prune-stale", "--dry-run"])
        .assert()
        .success();
}

// --- Meta / passthrough (happy) ---

#[test]
fn happy_git_and_passthrough() {
    let f = Fixture::with_repos(&["p"]);
    f.gg().args(["git", "--", "status"]).assert().success();
    f.gg().args(["status"]).assert().success();
    f.gg()
        .args(["--dry-run", "rev-parse", "--show-toplevel"])
        .assert()
        .success();
}

#[test]
fn happy_completions_man_version() {
    let f = Fixture::new();
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        f.gg().args(["completions", shell]).assert().success();
    }
    f.gg().args(["man"]).assert().success();
    let out_dir = f.root.path().join("manpages");
    f.gg()
        .args(["man", "--output", out_dir.to_str().unwrap()])
        .assert()
        .success();
    f.gg().args(["version"]).assert().success();
    f.gg().args(["self-update"]).assert().success();
}

// --- Interactive entrypoints ---

const INTERACTIVE_ENTRYPOINTS: &[&[&str]] = &[
    &["wizard"],
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
];

#[cfg(not(coverage))]
#[test]
fn unhappy_interactive_requires_tty() {
    let f = catalog_fixture();
    for args in INTERACTIVE_ENTRYPOINTS {
        f.gg()
            .args(args.iter().copied())
            .assert()
            .failure()
            .stderr(predicates::str::contains("TTY").or(predicates::str::contains("terminal")));
    }
}

#[cfg(coverage)]
#[test]
fn happy_interactive_skipped_under_coverage() {
    let f = catalog_fixture();
    for args in INTERACTIVE_ENTRYPOINTS {
        f.gg()
            .args(args.iter().copied())
            .assert()
            .success()
            .stdout(predicates::str::contains("skipped"));
    }
}

// --- Unhappy: catalog not-found ---

#[test]
fn unhappy_remove_missing_catalog_entries() {
    let f = catalog_fixture();
    f.gg().args(["alias", "remove", "nope"]).assert().failure();
    f.gg().args(["group", "remove", "nope"]).assert().failure();
    f.gg().args(["tag", "remove", "nope"]).assert().failure();
    f.gg()
        .args(["remotes", "remove", "nope"])
        .assert()
        .failure();
}

#[test]
fn unhappy_member_remove_missing_group_or_tag() {
    let f = catalog_fixture();
    f.gg()
        .args(["group", "member", "remove", "missing", "r"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("group not found"));
    f.gg()
        .args(["tag", "member", "remove", "missing", "r"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("tag not found"));
    f.gg()
        .args(["group", "member", "remove", "g", "not-a-member"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not in group"));
}

#[test]
fn unhappy_tag_member_not_in_tag() {
    let f = catalog_fixture();
    f.gg()
        .args(["tag", "member", "remove", "t", "not-a-member"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not in tag"));
}

#[test]
fn unhappy_config_enroll_remove_bad_index() {
    let f = catalog_fixture();
    f.gg()
        .args(["config", "enroll", "remove", "99"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("out of range"));
}

#[test]
fn unhappy_config_get_set_unknown_key() {
    let f = catalog_fixture();
    f.gg()
        .args(["config", "get", "not_a_real_key"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown"));
    f.gg()
        .args(["config", "set", "not_a_real_key", "x"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown"));
}

#[test]
fn unhappy_init_unknown_profile() {
    let f = catalog_fixture();
    let target = f.root.path().join("badprof");
    f.gg()
        .args([
            "init",
            "--profile",
            "does-not-exist",
            target.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown profile"));
}

#[test]
fn unhappy_hooks_unknown_pack_and_empty_selection() {
    let f = catalog_fixture();
    f.gg()
        .args(["hooks", "install", "not-a-pack"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown hook pack"));
    let empty = Fixture::new();
    empty
        .gg()
        .args(["hooks", "install", "noop"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("no repositories"));
}

#[test]
fn unhappy_remotes_add_to_empty_selection() {
    let f = Fixture::new();
    f.write_global_config(
        r#"
schema_version = 1
[remotes]
up = "https://example.com/org/"
"#,
    );
    f.gg()
        .args(["remotes", "add-to", "up"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("no repositories"));
}

#[test]
fn unhappy_update_without_rules() {
    let f = Fixture::new();
    f.write_global_config("schema_version = 1\n");
    f.gg()
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("auto_enroll"));
}

#[test]
fn unhappy_update_ask_without_tty_when_stale_present() {
    let f = catalog_fixture();
    f.write_global_config(&format!(
        r#"
schema_version = 1
root = "{root}"
[aliases]
r = "{path}"
gone = "/no/such/path"
[groups]
g = ["r"]
[tags]
t = ["r"]
[remotes]
up = "https://example.com/org/"
[[auto_enroll]]
path = "{root}"
depth = 2
"#,
        root = Fixture::toml_path(f.root.path()),
        path = Fixture::toml_path(&f.repos[0]),
    ));
    f.gg()
        .args(["update", "--ask"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("interactive terminal"));
}

#[test]
fn unhappy_config_edit_bad_editor() {
    let f = catalog_fixture();
    f.gg()
        .env("EDITOR", "false")
        .args(["config", "edit"])
        .assert()
        .failure();
}

#[test]
fn unhappy_each_empty_command() {
    Fixture::with_repos(&["x"])
        .gg()
        .args(["each"])
        .assert()
        .failure();
}

#[test]
fn unhappy_each_command_failure() {
    let f = Fixture::with_repos(&["x"]);
    #[cfg(windows)]
    f.gg().args(["each", "exit", "/b", "1"]).assert().failure();
    #[cfg(not(windows))]
    f.gg().args(["each", "false"]).assert().failure();
}

#[test]
fn happy_catalog_remove_dry_run() {
    let f = catalog_fixture();
    f.gg()
        .args(["--dry-run", "alias", "remove", "r"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "group", "remove", "g"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "tag", "remove", "t"])
        .assert()
        .success();
    f.gg()
        .args(["--dry-run", "remotes", "remove", "up"])
        .assert()
        .success();
}

#[test]
fn unhappy_git_subcommand_empty() {
    Fixture::new().gg().args(["git"]).assert().failure();
}

#[test]
fn unhappy_passthrough_misplaced_global_flag() {
    Fixture::with_repos(&["r"])
        .gg()
        .args(["status", "--fail-fast"])
        .assert()
        .failure();
}

#[test]
fn unhappy_selection_include_missing() {
    Fixture::with_repos(&["r"])
        .gg()
        .args(["--in", "missing-alias", "list"])
        .assert()
        .failure();
}

#[test]
fn unhappy_man_invalid_output_path() {
    Fixture::new()
        .gg()
        .args(["man", "--output", "/dev/null/impossible/file.1"])
        .assert()
        .failure();
}

// --- Unhappy: empty selection warnings (success with stderr) ---

#[test]
fn unhappy_empty_selection_warns() {
    let f = Fixture::new();
    f.gg()
        .args(["sync"])
        .assert()
        .success()
        .stderr(predicates::str::contains("no repositories"));
    f.gg()
        .args(["overview"])
        .assert()
        .success()
        .stderr(predicates::str::contains("no repositories"));
}

#[test]
fn unhappy_list_worktrees_stale_empty_selection() {
    let f = Fixture::new();
    f.gg()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0 repositories"));
    f.gg()
        .args(["worktrees"])
        .assert()
        .success()
        .stdout(predicates::str::contains("repo"));
    f.gg()
        .args(["stale", "--days", "90"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no repos stale"));
    f.gg()
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("findings"));
}

#[test]
fn unhappy_commits_empty_selection_shows_empty_table() {
    let f = Fixture::new();
    f.gg()
        .args(["commits"])
        .assert()
        .success()
        .stdout(predicates::str::contains("repo"));
}

#[test]
fn unhappy_commits_on_repo_without_commits() {
    let f = Fixture::new();
    let empty = f.root.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    git(&empty, &["init", "-b", "main"]);
    f.gg()
        .args(["--refresh", "commits", "-n", "1"])
        .assert()
        .success();
}

#[test]
fn unhappy_info_non_repo_path_soft_success() {
    let f = Fixture::new();
    f.gg()
        .args(["info", f.root.path().to_str().unwrap()])
        .assert()
        .success();
}
