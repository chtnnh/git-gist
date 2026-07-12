//! Enroll newly discovered repos into aliases, groups, and tags.

use crate::cli::Cli;
use crate::config::{self, AutoEnroll, Config};
use crate::discover;
use crate::output::OutputCtx;
use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct EnrollChange {
    alias: String,
    path: String,
    groups: Vec<String>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct UpdateReport {
    added: Vec<EnrollChange>,
    skipped_existing: usize,
    membership_fixed: usize,
    rules: usize,
    dry_run: bool,
    saved: Option<String>,
}

pub fn run(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    if cfg.auto_enroll.is_empty() {
        bail!(
            "no [[auto_enroll]] rules in config — add rules then re-run `gg update`\n\
             example:\n\
             [[auto_enroll]]\n\
             path = \"/path/to/watch\"\n\
             depth = 6\n\
             tags = [\"learning\"]\n\
             groups = [\"oss\"]"
        );
    }

    let dry_run = cli.dry_run;
    let report = apply_auto_enroll(cfg, dry_run)?;

    if out.is_json() {
        out.write_json(&report)?;
        return Ok(());
    }

    if report.added.is_empty() && report.membership_fixed == 0 {
        out.info(&format!(
            "no changes ({} already enrolled under {} rule(s))",
            report.skipped_existing, report.rules
        ))?;
    } else {
        for change in &report.added {
            let mut bits = Vec::new();
            if !change.groups.is_empty() {
                bits.push(format!("groups=[{}]", change.groups.join(", ")));
            }
            if !change.tags.is_empty() {
                bits.push(format!("tags=[{}]", change.tags.join(", ")));
            }
            let suffix = if bits.is_empty() {
                String::new()
            } else {
                format!(" ({})", bits.join(", "))
            };
            let prefix = if dry_run { "would add" } else { "added" };
            out.success(&format!(
                "{prefix} {} → {}{suffix}",
                change.alias, change.path
            ))?;
        }
        if report.membership_fixed > 0 {
            let prefix = if dry_run { "would update" } else { "updated" };
            out.info(&format!(
                "{prefix} group/tag membership for {} existing alias(es)",
                report.membership_fixed
            ))?;
        }
        out.info(&format!(
            "{} new alias(es); {} already present",
            report.added.len(),
            report.skipped_existing
        ))?;
    }

    if let Some(path) = &report.saved {
        out.info(&format!("saved {path}"))?;
    } else if dry_run && (!report.added.is_empty() || report.membership_fixed > 0) {
        out.info("dry-run — config not written")?;
    }

    Ok(())
}

/// Apply all `auto_enroll` rules and optionally persist.
fn apply_auto_enroll(cfg: &Config, dry_run: bool) -> Result<UpdateReport> {
    let mut updated = cfg.clone();
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

    let dirty = !added.is_empty() || membership_fixed > 0;
    let saved = if !dry_run && dirty {
        let path = config::save_global(&updated)?;
        Some(path.display().to_string())
    } else {
        None
    };

    Ok(UpdateReport {
        added,
        skipped_existing,
        membership_fixed,
        rules: cfg.auto_enroll.len(),
        dry_run,
        saved,
    })
}

/// Returns true if group/tag membership changed.
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
    fn unique_names_numeric_suffix_and_sanitize() {
        let mut used = BTreeSet::new();
        used.insert("foo".into());
        used.insert("nested-foo".into());
        used.insert("watch-foo".into());
        let root = PathBuf::from("/watch");
        let repo = PathBuf::from("/watch/nested/foo");
        assert_eq!(unique_alias_name(&repo, &root, &used), "foo-2");

        used.insert("foo-2".into());
        assert_eq!(unique_alias_name(&repo, &root, &used), "foo-3");

        assert_eq!(sanitize_alias("my repo!"), "my-repo");
        assert_eq!(sanitize_alias("---"), "");
    }
}
