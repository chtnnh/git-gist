//! Auto-enroll: discover repos under watch rules and persist aliases/groups/tags.

use crate::cli::Cli;
use crate::config::{self, AutoEnroll, Config};
use crate::config_ops;
use crate::discover;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Clone)]
pub struct EnrollChange {
    pub alias: String,
    pub path: String,
    pub groups: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct UpdateReport {
    pub added: Vec<EnrollChange>,
    pub skipped_existing: usize,
    pub membership_fixed: usize,
    pub pruned_stale: Vec<String>,
    pub rules: usize,
    pub dry_run: bool,
    pub saved: Option<String>,
    pub warnings: Vec<String>,
}

/// Re-scan at least this often so nested repo creation under an unchanged
/// watch-root mtime is still picked up (same order as discovery cache TTL).
const ENROLL_INTERVAL_SECS: u64 = 3600;

#[derive(Debug, Default, Serialize, Deserialize)]
struct EnrollState {
    last_run_unix: u64,
    /// Watch path → last observed mtime (secs)
    watch_mtimes: BTreeMap<String, u64>,
    /// Hash of `[[auto_enroll]]` rules at last successful scan.
    #[serde(default)]
    rules_hash: String,
}

pub fn apply_auto_enroll(cfg: &Config, dry_run: bool, prune_stale: bool) -> Result<UpdateReport> {
    let mut updated = cfg.clone();
    let mut warnings = Vec::new();
    let mut pruned_stale = Vec::new();

    if prune_stale {
        pruned_stale = config_ops::prune_stale_aliases(&mut updated);
    }

    // Dangerous-config warnings
    if let Some(root) = &cfg.root {
        let root_c = canonicalize_soft(root);
        for rule in &cfg.auto_enroll {
            let rule_c = canonicalize_soft(&rule.path);
            if rule_c == root_c
                && (!rule.groups.is_empty() || !rule.tags.is_empty())
                && rule
                    .path_prefix
                    .as_ref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
            {
                warnings.push(format!(
                    "auto_enroll path equals config root ({}) with groups/tags and no path_prefix — \
                     this may pollute curated groups; consider path_prefix or a narrower path",
                    root.display()
                ));
            }
            if !rule.path.is_dir() {
                warnings.push(format!(
                    "auto_enroll watch path missing or not a directory: {}",
                    rule.path.display()
                ));
            }
        }
    }

    let mut added = Vec::new();
    let mut skipped_existing = 0usize;
    let mut membership_fixed = 0usize;

    let mut aliased_paths: HashSet<PathBuf> = updated
        .aliases
        .values()
        .map(|p| canonicalize_soft(p))
        .collect();

    let mut used_names: BTreeSet<String> = updated.aliases.keys().cloned().collect();

    for rule in &cfg.auto_enroll {
        let root = canonicalize_soft(&rule.path);
        if !root.is_dir() {
            continue;
        }
        let depth = if rule.depth == 0 {
            usize::MAX
        } else {
            rule.depth
        };
        let found = discover::discover_repos(&root, depth, cfg)?;
        for repo_path in found {
            let canon = canonicalize_soft(&repo_path);
            if !matches_path_prefix(&canon, &root, rule.path_prefix.as_deref()) {
                continue;
            }
            if aliased_paths.contains(&canon) {
                skipped_existing += 1;
                if ensure_membership(&mut updated, &canon, rule) {
                    membership_fixed += 1;
                }
                continue;
            }

            let alias = unique_alias_name(&canon, &root, &used_names);
            used_names.insert(alias.clone());
            aliased_paths.insert(canon.clone());
            updated.aliases.insert(alias.clone(), canon.clone());

            for g in &rule.groups {
                let members = updated.groups.entry(g.clone()).or_default();
                if !members.iter().any(|m| m == &alias) {
                    members.push(alias.clone());
                }
            }
            for t in &rule.tags {
                let members = updated.tags.entry(t.clone()).or_default();
                if !members.iter().any(|m| m == &alias) {
                    members.push(alias.clone());
                }
            }

            added.push(EnrollChange {
                alias,
                path: canon.display().to_string(),
                groups: rule.groups.clone(),
                tags: rule.tags.clone(),
            });
        }
    }

    for g in &cfg
        .auto_enroll
        .iter()
        .flat_map(|r| r.groups.clone())
        .collect::<BTreeSet<_>>()
    {
        if let Some(before) = cfg.groups.get(g) {
            if let Some(after) = updated.groups.get(g) {
                let growth = after.len().saturating_sub(before.len());
                if growth > 20 {
                    warnings.push(format!(
                        "group `{g}` grew by {growth} members in one enroll run"
                    ));
                }
            }
        } else if updated.groups.get(g).map(|m| m.len()).unwrap_or(0) > 20 {
            warnings.push(format!(
                "group `{g}` gained {} members in one enroll run",
                updated.groups.get(g).map(|m| m.len()).unwrap_or(0)
            ));
        }
    }

    let dirty = !added.is_empty() || membership_fixed > 0 || !pruned_stale.is_empty();
    let saved = if !dry_run && dirty {
        let path = config::save_global(&updated)?;
        if let Err(err) = record_state(cfg) {
            warnings.push(format!(
                "saved config to {} but failed to record enroll throttle state: {err:#}",
                path.display()
            ));
        }
        Some(path.display().to_string())
    } else if !dry_run && !dirty {
        if let Err(err) = record_state(cfg) {
            warnings.push(format!("failed to record enroll throttle state: {err:#}"));
        }
        None
    } else {
        None
    };

    Ok(UpdateReport {
        added,
        skipped_existing,
        membership_fixed,
        pruned_stale,
        rules: cfg.auto_enroll.len(),
        dry_run,
        saved,
        warnings,
    })
}

/// Throttled auto-enroll for selection commands. Mutates `cfg` in-memory when saved.
pub fn maybe_auto_enroll(cfg: &mut Config, cli: &Cli) -> Result<Option<UpdateReport>> {
    if cfg.auto_enroll.is_empty() {
        return Ok(None);
    }
    let force = cli.refresh;
    if !force && !needs_scan(cfg)? {
        return Ok(None);
    }
    let report = apply_auto_enroll(cfg, cli.dry_run, false)?;
    if report.saved.is_some() {
        // Reload aliases into working cfg for this process
        *cfg = config::load(cli)?;
    } else if !report.added.is_empty() || report.membership_fixed > 0 {
        // dry-run or no save — leave cfg as-is
    } else {
        // still update throttle state on empty successful scan (already in apply)
    }
    Ok(Some(report))
}

fn needs_scan(cfg: &Config) -> Result<bool> {
    let state = load_state().unwrap_or_default();
    if state.last_run_unix == 0 {
        return Ok(true);
    }
    let now = now_secs();
    if now.saturating_sub(state.last_run_unix) > ENROLL_INTERVAL_SECS {
        return Ok(true);
    }
    let hash = rules_hash(&cfg.auto_enroll);
    if state.rules_hash != hash {
        return Ok(true);
    }
    for rule in &cfg.auto_enroll {
        let key = rule.path.display().to_string();
        let mtime = dir_mtime_secs(&rule.path);
        match state.watch_mtimes.get(&key) {
            Some(&prev) if Some(prev) == mtime => {}
            _ => return Ok(true),
        }
    }
    Ok(false)
}

fn rules_hash(rules: &[AutoEnroll]) -> String {
    // Stable fingerprint of watch rules so edits invalidate the throttle.
    let mut parts: Vec<String> = rules
        .iter()
        .map(|r| {
            format!(
                "{}|{}|{}|{}|{}",
                r.path.display(),
                r.path_prefix.as_deref().unwrap_or(""),
                r.depth,
                r.groups.join(","),
                r.tags.join(",")
            )
        })
        .collect();
    parts.sort();
    format!("{:x}", simple_hash(&parts.join("\n")))
}

fn simple_hash(s: &str) -> u64 {
    // FNV-1a 64-bit — good enough for config fingerprinting.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn record_state(cfg: &Config) -> Result<()> {
    let mut state = EnrollState {
        last_run_unix: now_secs(),
        watch_mtimes: BTreeMap::new(),
        rules_hash: rules_hash(&cfg.auto_enroll),
    };
    for rule in &cfg.auto_enroll {
        let key = rule.path.display().to_string();
        if let Some(m) = dir_mtime_secs(&rule.path) {
            state.watch_mtimes.insert(key, m);
        }
    }
    let path = config::state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&state)?)?;
    Ok(())
}

fn load_state() -> Result<EnrollState> {
    let path = config::state_path()?;
    if !path.is_file() {
        return Ok(EnrollState::default());
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

fn dir_mtime_secs(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn matches_path_prefix(repo: &Path, watch_root: &Path, prefix: Option<&str>) -> bool {
    let Some(prefix) = prefix.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    let prefix = prefix
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    let Ok(rel) = repo.strip_prefix(watch_root) else {
        return false;
    };
    let rel_s = rel.to_string_lossy().replace('\\', "/");
    rel_s == prefix || rel_s.starts_with(&format!("{prefix}/"))
}

fn ensure_membership(cfg: &mut Config, path: &Path, rule: &AutoEnroll) -> bool {
    let Some(alias) = cfg
        .aliases
        .iter()
        .find(|(_, p)| canonicalize_soft(p) == path)
        .map(|(n, _)| n.clone())
    else {
        return false;
    };
    let mut changed = false;
    for g in &rule.groups {
        let members = cfg.groups.entry(g.clone()).or_default();
        if !members.iter().any(|m| m == &alias) {
            members.push(alias.clone());
            changed = true;
        }
    }
    for t in &rule.tags {
        let members = cfg.tags.entry(t.clone()).or_default();
        if !members.iter().any(|m| m == &alias) {
            members.push(alias.clone());
            changed = true;
        }
    }
    changed
}

pub fn unique_alias_name(repo: &Path, watch_root: &Path, used: &BTreeSet<String>) -> String {
    let basename = repo
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".into());

    let relative = repo
        .strip_prefix(watch_root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/").replace('/', "-"))
        .filter(|s| !s.is_empty());

    let candidates = [
        Some(basename.clone()),
        relative,
        Some(format!(
            "{}-{}",
            watch_root
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "watch".into()),
            basename
        )),
    ];

    for c in candidates.into_iter().flatten() {
        let sanitized = sanitize_alias(&c);
        if !sanitized.is_empty() && !used.contains(&sanitized) {
            return sanitized;
        }
    }

    let base = sanitize_alias(&basename);
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}-{n}");
        if !used.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn sanitize_alias(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    s.trim_matches('-').to_string()
}

fn canonicalize_soft(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_names_prefer_basename() {
        let used = BTreeSet::new();
        let root = PathBuf::from("/watch");
        let repo = PathBuf::from("/watch/foo");
        assert_eq!(unique_alias_name(&repo, &root, &used), "foo");
    }

    #[test]
    fn unique_names_disambiguate() {
        let mut used = BTreeSet::new();
        used.insert("foo".into());
        let root = PathBuf::from("/watch");
        let repo = PathBuf::from("/watch/nested/foo");
        assert_eq!(unique_alias_name(&repo, &root, &used), "nested-foo");
    }

    #[test]
    fn path_prefix_filters() {
        let root = PathBuf::from("/tech");
        let oss = PathBuf::from("/tech/oss/proj");
        let learn = PathBuf::from("/tech/learning/x");
        assert!(matches_path_prefix(&oss, &root, Some("oss")));
        assert!(!matches_path_prefix(&learn, &root, Some("oss")));
        assert!(matches_path_prefix(&learn, &root, None));
        // Windows-style prefix separators must match forward-slash relative paths.
        assert!(matches_path_prefix(&oss, &root, Some("oss\\proj")));
        assert!(matches_path_prefix(
            &PathBuf::from("/tech/oss/pkg/r"),
            &root,
            Some("oss\\pkg")
        ));
    }

    #[test]
    fn rules_hash_changes_with_prefix() {
        let a = vec![AutoEnroll {
            path: PathBuf::from("/w"),
            path_prefix: Some("oss".into()),
            depth: 3,
            groups: vec![],
            tags: vec![],
        }];
        let b = vec![AutoEnroll {
            path: PathBuf::from("/w"),
            path_prefix: Some("learn".into()),
            depth: 3,
            groups: vec![],
            tags: vec![],
        }];
        assert_ne!(rules_hash(&a), rules_hash(&b));
        assert_eq!(rules_hash(&a), rules_hash(&a));
    }
}
