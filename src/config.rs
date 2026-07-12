//! Configuration loading and persistence.

use crate::cli::Cli;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
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
    #[serde(default)]
    pub repo_overrides: BTreeMap<String, RepoOverride>,
    /// Path this config was loaded/saved from (not serialized)
    #[serde(skip)]
    pub path: Option<PathBuf>,
    #[serde(skip)]
    pub local_path: Option<PathBuf>,
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

pub fn global_config_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .context("could not resolve config directory")?;
    Ok(base.join("git-gist").join("config.toml"))
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

pub fn load(cli: &Cli) -> Result<Config> {
    let mut cfg = Config {
        schema_version: CONFIG_SCHEMA_VERSION,
        depth: default_depth(),
        ..Default::default()
    }
    .with_builtins();

    let global_path = global_config_path()?;
    if global_path.is_file() {
        let text = fs::read_to_string(&global_path)
            .with_context(|| format!("reading {}", global_path.display()))?;
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
    if let Some(theme) = &cli.theme {
        cfg.theme = Some(theme.clone());
    }

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
        // Prefer overlay depth if present in file (we can't distinguish easily;
        // always take overlay depth when overlay was parsed from file)
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
    for (k, v) in overlay.repo_overrides {
        base.repo_overrides.insert(k, v);
    }
    if overlay.schema_version != 0 {
        base.schema_version = overlay.schema_version;
    }
    base
}

pub fn save_global(cfg: &Config) -> Result<PathBuf> {
    let path = cfg.path.clone().unwrap_or(global_config_path()?);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut to_save = cfg.clone();
    to_save.path = None;
    to_save.local_path = None;
    // Don't persist builtin packs if user hasn't customized — still fine to persist
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
        other => bail!("unknown or unsupported set key: {other}"),
    }
    Ok(())
}

pub fn cache_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(dirs::cache_dir)
        .context("could not resolve cache directory")?;
    Ok(base.join("git-gist").join("discovery.json"))
}
