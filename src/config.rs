//! Configuration loading and persistence.

use crate::cli::Cli;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub root: Option<PathBuf>,
    #[serde(default = "default_depth")]
    pub depth: usize,
    #[serde(default)]
    pub jobs: Option<usize>,
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub aliases: BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub groups: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub tags: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub remotes: BTreeMap<String, String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, ScaffoldProfile>,
    #[serde(default)]
    pub hook_packs: BTreeMap<String, HookPack>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub include_submodules: bool,
    /// When true, human output shows `name (path)` instead of basename only.
    #[serde(default)]
    pub show_path: bool,
    #[serde(default)]
    pub repo_overrides: BTreeMap<String, RepoOverride>,
    /// Rules that enroll newly discovered repos into aliases / groups / tags.
    #[serde(default)]
    pub auto_enroll: Vec<AutoEnroll>,
    /// Path this config was loaded/saved from (not serialized)
    #[serde(skip)]
    pub path: Option<PathBuf>,
    #[serde(skip)]
    pub local_path: Option<PathBuf>,
    /// Warnings collected during load (not serialized)
    #[serde(skip)]
    pub load_warnings: Vec<String>,
}

fn default_schema() -> u32 {
    CONFIG_SCHEMA_VERSION
}

fn default_depth() -> usize {
    6
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScaffoldProfile {
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub default_branch: Option<String>,
    #[serde(default)]
    pub remotes: BTreeMap<String, String>,
    #[serde(default)]
    pub hooks: Vec<String>,
    pub gitignore: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookPack {
    pub description: Option<String>,
    /// Map of hook name → script body
    pub hooks: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoOverride {
    #[serde(default)]
    pub skip: bool,
    #[serde(default)]
    pub default_args: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Watch a directory and enroll new git repos into aliases, groups, and tags.
///
/// ```toml
/// [[auto_enroll]]
/// path = "/home/you/src"
/// path_prefix = "oss/"
/// depth = 6
/// tags = ["learning"]
/// groups = []
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoEnroll {
    /// Directory to scan for git repositories
    pub path: PathBuf,
    /// Only enroll repos under this relative prefix of `path` (optional)
    #[serde(default)]
    pub path_prefix: Option<String>,
    /// Max walk depth under `path` (default 6)
    #[serde(default = "default_enroll_depth")]
    pub depth: usize,
    /// Groups that should include each newly enrolled alias
    #[serde(default)]
    pub groups: Vec<String>,
    /// Tags that should include each newly enrolled alias
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_enroll_depth() -> usize {
    6
}

impl Config {
    pub fn with_builtins(mut self) -> Self {
        if self.profiles.is_empty() {
            self.profiles.insert(
                "default".into(),
                ScaffoldProfile {
                    default_branch: Some("main".into()),
                    ..Default::default()
                },
            );
        }
        if self.hook_packs.is_empty() {
            let mut pre_commit = BTreeMap::new();
            pre_commit.insert(
                "pre-commit".into(),
                "#!/bin/sh\n# git-gist default pre-commit\nexit 0\n".into(),
            );
            self.hook_packs.insert(
                "noop".into(),
                HookPack {
                    description: Some("No-op hooks for scaffolding".into()),
                    hooks: pre_commit,
                },
            );
            let mut msg = BTreeMap::new();
            msg.insert(
                "commit-msg".into(),
                "#!/bin/sh\n# Reject empty commit messages\ntest -s \"$1\" || exit 1\n".into(),
            );
            self.hook_packs.insert(
                "commit-msg-required".into(),
                HookPack {
                    description: Some("Require non-empty commit messages".into()),
                    hooks: msg,
                },
            );
        }
        self
    }
}

/// Resolve the user home used for `~/.git-gist/`.
///
/// `dirs::home_dir()` on Windows uses the Known Folder API and ignores `HOME`,
/// which breaks test fixtures that redirect the home directory. Prefer explicit
/// `GIT_GIST_HOME`, then `HOME` / `USERPROFILE`, then `dirs::home_dir()`.
pub fn home_dir() -> Result<PathBuf> {
    for key in ["GIT_GIST_HOME", "HOME", "USERPROFILE"] {
        if let Some(raw) = std::env::var_os(key) {
            if !raw.is_empty() {
                return Ok(PathBuf::from(raw));
            }
        }
    }
    dirs::home_dir().context("could not resolve home directory")
}

/// Canonical global config: `~/.git-gist/config.toml`
pub fn global_config_path() -> Result<PathBuf> {
    Ok(home_dir()?.join(".git-gist").join("config.toml"))
}

/// Runtime data directory: `~/.git-gist/`
pub fn data_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".git-gist"))
}

pub fn state_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("state.json"))
}

/// Legacy global config locations (pre-1.3.0).
pub fn legacy_global_config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        paths.push(xdg.join("git-gist").join("config.toml"));
    }
    if let Some(base) = dirs::config_dir() {
        let p = base.join("git-gist").join("config.toml");
        if !paths.contains(&p) {
            paths.push(p);
        }
    }
    paths
}

fn legacy_cache_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from) {
        paths.push(xdg.join("git-gist").join("discovery.json"));
    }
    if let Some(base) = dirs::cache_dir() {
        let p = base.join("git-gist").join("discovery.json");
        if !paths.contains(&p) {
            paths.push(p);
        }
    }
    paths
}

/// Migrate legacy config into `~/.git-gist/config.toml` if needed.
fn ensure_migrated_config() -> Result<(PathBuf, Vec<String>)> {
    let mut warnings = Vec::new();
    let dest = global_config_path()?;
    if dest.is_file() {
        return Ok((dest, warnings));
    }
    for legacy in legacy_global_config_paths() {
        if !legacy.is_file() {
            continue;
        }
        // Skip if legacy is a symlink that already points at dest
        if let Ok(target) = fs::read_link(&legacy) {
            if target == dest || canonicalize_soft(&legacy) == canonicalize_soft(&dest) {
                continue;
            }
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&legacy, &dest).with_context(|| {
            format!(
                "migrating config from {} to {}",
                legacy.display(),
                dest.display()
            )
        })?;
        let msg = format!(
            "migrated config from {} → {}",
            legacy.display(),
            dest.display()
        );
        eprintln!("git-gist: {msg}");
        warnings.push(msg);
        return Ok((dest, warnings));
    }
    Ok((dest, warnings))
}

fn ensure_migrated_cache() -> Result<()> {
    let dest = cache_path()?;
    if dest.is_file() {
        return Ok(());
    }
    for legacy in legacy_cache_paths() {
        if !legacy.is_file() {
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let _ = fs::copy(&legacy, &dest);
        break;
    }
    Ok(())
}

pub fn find_local_config(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        for name in [".gg.toml", ".git-gist.toml"] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Known top-level Config field names (serde keys). User-defined map keys under
/// aliases/groups/tags/remotes/profiles/hook_packs/repo_overrides are not validated.
const KNOWN_TOP_LEVEL: &[&str] = &[
    "schema_version",
    "root",
    "depth",
    "jobs",
    "ignore",
    "aliases",
    "groups",
    "tags",
    "remotes",
    "profiles",
    "hook_packs",
    "theme",
    "include_submodules",
    "show_path",
    "repo_overrides",
    "auto_enroll",
];

const KNOWN_AUTO_ENROLL_FIELDS: &[&str] = &["path", "path_prefix", "depth", "groups", "tags"];

const KNOWN_PROFILE_FIELDS: &[&str] = &[
    "user_name",
    "user_email",
    "default_branch",
    "remotes",
    "hooks",
    "gitignore",
    "license",
];

const KNOWN_HOOK_PACK_FIELDS: &[&str] = &["description", "hooks"];

const KNOWN_REPO_OVERRIDE_FIELDS: &[&str] = &["skip", "default_args", "tags"];

/// Scan raw TOML for unknown keys and suggest near-matches (edit distance).
///
/// Serde ignores unknown keys silently; this surfaces likely typos like
/// `auto_enrol` → `auto_enroll` without hardcoding individual misspellings.
pub fn scan_raw_config(text: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let Ok(value) = text.parse::<toml::Value>() else {
        return warnings;
    };
    let Some(table) = value.as_table() else {
        return warnings;
    };
    scan_table_keys(table, KNOWN_TOP_LEVEL, None, &mut warnings);

    if let Some(arr) = table.get("auto_enroll").and_then(|v| v.as_array()) {
        for (i, item) in arr.iter().enumerate() {
            if let Some(t) = item.as_table() {
                scan_table_keys(
                    t,
                    KNOWN_AUTO_ENROLL_FIELDS,
                    Some(&format!("auto_enroll[{i}]")),
                    &mut warnings,
                );
            }
        }
    }
    // Typo'd array-of-tables never land in `auto_enroll` — check any top-level
    // array-of-tables that looked like a near-miss (already warned) for fields too.
    for (key, val) in table {
        if key == "auto_enroll" || !val.is_array() {
            continue;
        }
        if closest_known(key, KNOWN_TOP_LEVEL).is_none() {
            continue;
        }
        if let Some(arr) = val.as_array() {
            for (i, item) in arr.iter().enumerate() {
                if let Some(t) = item.as_table() {
                    scan_table_keys(
                        t,
                        KNOWN_AUTO_ENROLL_FIELDS,
                        Some(&format!("{key}[{i}]")),
                        &mut warnings,
                    );
                }
            }
        }
    }

    if let Some(profiles) = table.get("profiles").and_then(|v| v.as_table()) {
        for (name, profile) in profiles {
            if let Some(t) = profile.as_table() {
                scan_table_keys(
                    t,
                    KNOWN_PROFILE_FIELDS,
                    Some(&format!("profiles.{name}")),
                    &mut warnings,
                );
            }
        }
    }
    if let Some(packs) = table.get("hook_packs").and_then(|v| v.as_table()) {
        for (name, pack) in packs {
            if let Some(t) = pack.as_table() {
                scan_table_keys(
                    t,
                    KNOWN_HOOK_PACK_FIELDS,
                    Some(&format!("hook_packs.{name}")),
                    &mut warnings,
                );
            }
        }
    }
    if let Some(ovs) = table.get("repo_overrides").and_then(|v| v.as_table()) {
        for (name, ov) in ovs {
            if let Some(t) = ov.as_table() {
                scan_table_keys(
                    t,
                    KNOWN_REPO_OVERRIDE_FIELDS,
                    Some(&format!("repo_overrides.{name}")),
                    &mut warnings,
                );
            }
        }
    }

    warnings
}

fn scan_table_keys(
    table: &toml::map::Map<String, toml::Value>,
    known: &[&str],
    context: Option<&str>,
    warnings: &mut Vec<String>,
) {
    for key in table.keys() {
        if known.contains(&key.as_str()) {
            continue;
        }
        let ctx = context.map(|c| format!("{c}.")).unwrap_or_default();
        if let Some(suggestion) = closest_known(key, known) {
            warnings.push(format!(
                "unknown key `{ctx}{key}` — did you mean `{suggestion}`? \
                 (ignored by parser)"
            ));
        } else {
            warnings.push(format!("unknown key `{ctx}{key}` (ignored by parser)"));
        }
    }
}

/// Suggest a known key when edit distance is small relative to length.
pub fn closest_known<'a>(input: &str, known: &[&'a str]) -> Option<&'a str> {
    let input_l = input.to_ascii_lowercase();
    let mut best: Option<(&'a str, usize)> = None;
    for &cand in known {
        let d = edit_distance(&input_l, &cand.to_ascii_lowercase());
        let max_allowed = match cand.len().max(input.len()) {
            0..=3 => 1,
            4..=7 => 2,
            _ => 3,
        };
        if d == 0 || d > max_allowed {
            continue;
        }
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((cand, d));
        }
    }
    best.map(|(k, _)| k)
}

/// Classic Levenshtein distance.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

pub fn load(cli: &Cli) -> Result<Config> {
    let mut cfg = Config {
        schema_version: CONFIG_SCHEMA_VERSION,
        depth: default_depth(),
        ..Default::default()
    }
    .with_builtins();

    let (global_path, mut warnings) = ensure_migrated_config()?;
    let _ = ensure_migrated_cache();

    if global_path.is_file() {
        let text = fs::read_to_string(&global_path)
            .with_context(|| format!("reading {}", global_path.display()))?;
        warnings.extend(scan_raw_config(&text));
        if !text.trim().is_empty() {
            let parsed: Config = toml::from_str(&text)
                .with_context(|| format!("parsing {}", global_path.display()))?;
            cfg = merge_config(cfg, parsed);
        }
        cfg.path = Some(global_path.clone());
        migrate_schema(&mut cfg)?;
    } else {
        cfg.path = Some(global_path);
    }

    let cwd = std::env::current_dir()?;
    if let Some(local) = find_local_config(&cwd) {
        let text =
            fs::read_to_string(&local).with_context(|| format!("reading {}", local.display()))?;
        warnings.extend(scan_raw_config(&text));
        if !text.trim().is_empty() {
            let parsed: Config =
                toml::from_str(&text).with_context(|| format!("parsing {}", local.display()))?;
            cfg = merge_config(cfg, parsed);
        }
        cfg.local_path = Some(local);
    }

    // CLI overrides
    if let Some(root) = &cli.root {
        cfg.root = Some(root.clone());
    }
    if let Some(depth) = cli.depth {
        cfg.depth = depth;
    }
    if let Some(jobs) = cli.jobs {
        cfg.jobs = Some(jobs);
    }
    if cli.include_submodules {
        cfg.include_submodules = true;
    }
    if cli.show_path {
        cfg.show_path = true;
    }
    if let Some(theme) = &cli.theme {
        cfg.theme = Some(theme.clone());
    }

    for w in &warnings {
        // Always surface unknown-key / migration notices; other noise stays verbose-only.
        if cli.verbose > 0 || w.contains("unknown key") || w.contains("migrated") {
            let _ = writeln!(std::io::stderr(), "git-gist: warning: {w}");
        }
    }
    cfg.load_warnings = warnings;

    Ok(cfg)
}

fn migrate_schema(cfg: &mut Config) -> Result<()> {
    if cfg.schema_version == 0 {
        cfg.schema_version = CONFIG_SCHEMA_VERSION;
    }
    if cfg.schema_version > CONFIG_SCHEMA_VERSION {
        bail!(
            "config schema_version {} is newer than supported {}",
            cfg.schema_version,
            CONFIG_SCHEMA_VERSION
        );
    }
    cfg.schema_version = CONFIG_SCHEMA_VERSION;
    Ok(())
}

/// Merge `overlay` onto `base` (overlay wins for set fields).
fn merge_config(mut base: Config, overlay: Config) -> Config {
    if overlay.root.is_some() {
        base.root = overlay.root;
    }
    if overlay.depth != default_depth() || base.depth == default_depth() {
        base.depth = overlay.depth;
    }
    if overlay.jobs.is_some() {
        base.jobs = overlay.jobs;
    }
    if !overlay.ignore.is_empty() {
        base.ignore.extend(overlay.ignore);
    }
    for (k, v) in overlay.aliases {
        base.aliases.insert(k, v);
    }
    for (k, v) in overlay.groups {
        base.groups.insert(k, v);
    }
    for (k, v) in overlay.tags {
        base.tags.insert(k, v);
    }
    for (k, v) in overlay.remotes {
        base.remotes.insert(k, v);
    }
    for (k, v) in overlay.profiles {
        base.profiles.insert(k, v);
    }
    for (k, v) in overlay.hook_packs {
        base.hook_packs.insert(k, v);
    }
    if overlay.theme.is_some() {
        base.theme = overlay.theme;
    }
    if overlay.include_submodules {
        base.include_submodules = true;
    }
    if overlay.show_path {
        base.show_path = true;
    }
    for (k, v) in overlay.repo_overrides {
        base.repo_overrides.insert(k, v);
    }
    if !overlay.auto_enroll.is_empty() {
        base.auto_enroll.extend(overlay.auto_enroll);
    }
    if overlay.schema_version != 0 {
        base.schema_version = overlay.schema_version;
    }
    base
}

pub fn save_global(cfg: &Config) -> Result<PathBuf> {
    let path = cfg
        .path
        .clone()
        .unwrap_or_else(|| global_config_path().expect("home"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut to_save = cfg.clone();
    to_save.path = None;
    to_save.local_path = None;
    to_save.load_warnings.clear();
    let text = toml::to_string_pretty(&to_save)?;
    fs::write(&path, text)?;
    Ok(path)
}

pub fn get_dot_key(cfg: &Config, key: &str) -> Result<String> {
    let value = match key {
        "root" => cfg
            .root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        "depth" => cfg.depth.to_string(),
        "jobs" => cfg
            .jobs
            .map(|j| j.to_string())
            .unwrap_or_else(|| num_cpus::get().to_string()),
        "theme" => cfg.theme.clone().unwrap_or_else(|| "default".into()),
        "include_submodules" => cfg.include_submodules.to_string(),
        "show_path" => cfg.show_path.to_string(),
        "schema_version" => cfg.schema_version.to_string(),
        other => bail!("unknown config key: {other}"),
    };
    Ok(value)
}

pub fn set_dot_key(cfg: &mut Config, key: &str, value: &str) -> Result<()> {
    match key {
        "root" => cfg.root = Some(PathBuf::from(value)),
        "depth" => cfg.depth = value.parse().context("depth must be a number")?,
        "jobs" => cfg.jobs = Some(value.parse().context("jobs must be a number")?),
        "theme" => cfg.theme = Some(value.to_string()),
        "include_submodules" => {
            cfg.include_submodules = matches!(value, "1" | "true" | "yes" | "on")
        }
        "show_path" => cfg.show_path = matches!(value, "1" | "true" | "yes" | "on"),
        other => bail!("unknown or unsupported set key: {other}"),
    }
    Ok(())
}

pub fn cache_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("discovery.json"))
}

fn canonicalize_soft(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_detects_auto_enrol_via_edit_distance() {
        let w = scan_raw_config("[[auto_enrol]]\npath = \"/tmp\"\n");
        assert!(
            w.iter()
                .any(|s| s.contains("auto_enrol") && s.contains("auto_enroll")),
            "{w:?}"
        );
    }

    #[test]
    fn scan_detects_show_pth_and_nested_field_typo() {
        let w = scan_raw_config(
            r#"
show_pth = true
[[auto_enroll]]
path = "/tmp"
path_prefx = "oss/"
"#,
        );
        assert!(w
            .iter()
            .any(|s| s.contains("show_pth") && s.contains("show_path")));
        assert!(w
            .iter()
            .any(|s| s.contains("path_prefx") && s.contains("path_prefix")));
    }

    #[test]
    fn scan_ok_for_correct_key() {
        let w = scan_raw_config("[[auto_enroll]]\npath = \"/tmp\"\npath_prefix = \"x\"\n");
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn edit_distance_basics() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("auto_enrol", "auto_enroll"), 1);
        assert_eq!(
            closest_known("auto_enrol", KNOWN_TOP_LEVEL),
            Some("auto_enroll")
        );
        assert_eq!(closest_known("zzzzzz", KNOWN_TOP_LEVEL), None);
    }
}
