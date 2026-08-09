//! Shared config mutations used by CLI, wizard, and TUI.

use crate::config::{self, AutoEnroll, Config};
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

pub fn add_alias(cfg: &mut Config, name: &str, path: PathBuf) {
    let resolved = path.canonicalize().unwrap_or(path);
    cfg.aliases.insert(name.to_string(), resolved);
}

pub fn remove_alias(cfg: &mut Config, name: &str) -> Result<()> {
    if cfg.aliases.remove(name).is_none() {
        bail!("alias not found: {name}");
    }
    scrub_member_from_groups_tags(cfg, name);
    Ok(())
}

pub fn rename_alias(cfg: &mut Config, old: &str, new: &str) -> Result<()> {
    let path = cfg
        .aliases
        .remove(old)
        .ok_or_else(|| anyhow::anyhow!("alias not found: {old}"))?;
    if cfg.aliases.contains_key(new) {
        cfg.aliases.insert(old.to_string(), path);
        bail!("alias already exists: {new}");
    }
    cfg.aliases.insert(new.to_string(), path);
    rename_member_in_groups_tags(cfg, old, new);
    Ok(())
}

pub fn set_group_members(cfg: &mut Config, name: &str, members: Vec<String>) {
    cfg.groups.insert(name.to_string(), members);
}

pub fn add_group_member(cfg: &mut Config, group: &str, member: &str) -> Result<()> {
    let members = cfg.groups.entry(group.to_string()).or_default();
    if !members.iter().any(|m| m == member) {
        members.push(member.to_string());
    }
    Ok(())
}

pub fn remove_group_member(cfg: &mut Config, group: &str, member: &str) -> Result<()> {
    let Some(members) = cfg.groups.get_mut(group) else {
        bail!("group not found: {group}");
    };
    let before = members.len();
    members.retain(|m| m != member);
    if members.len() == before {
        bail!("member {member} not in group {group}");
    }
    Ok(())
}

pub fn remove_group(cfg: &mut Config, name: &str) -> Result<()> {
    if cfg.groups.remove(name).is_none() {
        bail!("group not found: {name}");
    }
    Ok(())
}

pub fn set_tag_members(cfg: &mut Config, name: &str, members: Vec<String>) {
    cfg.tags.insert(name.to_string(), members);
}

pub fn add_tag_member(cfg: &mut Config, tag: &str, member: &str) -> Result<()> {
    let members = cfg.tags.entry(tag.to_string()).or_default();
    if !members.iter().any(|m| m == member) {
        members.push(member.to_string());
    }
    Ok(())
}

pub fn remove_tag_member(cfg: &mut Config, tag: &str, member: &str) -> Result<()> {
    let Some(members) = cfg.tags.get_mut(tag) else {
        bail!("tag not found: {tag}");
    };
    let before = members.len();
    members.retain(|m| m != member);
    if members.len() == before {
        bail!("member {member} not in tag {tag}");
    }
    Ok(())
}

pub fn remove_tag(cfg: &mut Config, name: &str) -> Result<()> {
    if cfg.tags.remove(name).is_none() {
        bail!("tag not found: {name}");
    }
    Ok(())
}

pub fn add_remote(cfg: &mut Config, name: &str, url: &str) {
    cfg.remotes.insert(name.to_string(), url.to_string());
}

pub fn remove_remote(cfg: &mut Config, name: &str) -> Result<()> {
    if cfg.remotes.remove(name).is_none() {
        bail!("remote not found: {name}");
    }
    Ok(())
}

pub fn add_auto_enroll_rule(cfg: &mut Config, rule: AutoEnroll) {
    cfg.auto_enroll.push(rule);
}

pub fn remove_auto_enroll_rule(cfg: &mut Config, index: usize) -> Result<AutoEnroll> {
    if index >= cfg.auto_enroll.len() {
        bail!("auto_enroll index out of range: {index}");
    }
    Ok(cfg.auto_enroll.remove(index))
}

pub fn set_scalar(cfg: &mut Config, key: &str, value: &str) -> Result<()> {
    config::set_dot_key(cfg, key, value)
}

/// Alias whose path does not exist on disk.
pub fn alias_is_stale(path: &Path) -> bool {
    !path.exists()
}

pub fn list_stale_aliases(cfg: &Config) -> Vec<(String, PathBuf)> {
    cfg.aliases
        .iter()
        .filter(|(_, p)| alias_is_stale(p))
        .map(|(n, p)| (n.clone(), p.clone()))
        .collect()
}

/// Remove stale aliases and scrub them from groups/tags. Returns removed names.
pub fn prune_stale_aliases(cfg: &mut Config) -> Vec<String> {
    let stale: Vec<String> = list_stale_aliases(cfg)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    for name in &stale {
        cfg.aliases.remove(name);
        scrub_member_from_groups_tags(cfg, name);
    }
    stale
}

/// Remove group members that are stale aliases and/or outside an optional path prefix.
pub fn prune_group_members(
    cfg: &mut Config,
    group: &str,
    under: Option<&Path>,
) -> Result<Vec<String>> {
    let Some(members) = cfg.groups.get(group).cloned() else {
        bail!("group not found: {group}");
    };
    let mut removed = Vec::new();
    let mut kept = Vec::new();
    for m in members {
        let Some(path) = cfg.aliases.get(&m) else {
            removed.push(m);
            continue;
        };
        if alias_is_stale(path) {
            removed.push(m);
            continue;
        }
        if let Some(prefix) = under {
            let path_c = path.canonicalize().unwrap_or_else(|_| path.clone());
            let prefix_c = prefix
                .canonicalize()
                .unwrap_or_else(|_| prefix.to_path_buf());
            if !(path_c == prefix_c || path_c.starts_with(&prefix_c)) {
                removed.push(m);
                continue;
            }
        }
        kept.push(m);
    }
    cfg.groups.insert(group.to_string(), kept);
    Ok(removed)
}

fn scrub_member_from_groups_tags(cfg: &mut Config, name: &str) {
    for members in cfg.groups.values_mut() {
        members.retain(|m| m != name);
    }
    for members in cfg.tags.values_mut() {
        members.retain(|m| m != name);
    }
}

fn rename_member_in_groups_tags(cfg: &mut Config, old: &str, new: &str) {
    for members in cfg.groups.values_mut() {
        for m in members.iter_mut() {
            if m == old {
                *m = new.to_string();
            }
        }
    }
    for members in cfg.tags.values_mut() {
        for m in members.iter_mut() {
            if m == old {
                *m = new.to_string();
            }
        }
    }
}

pub fn save(cfg: &Config) -> Result<PathBuf> {
    config::save_global(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn prune_stale_removes_and_scrubs() {
        let dir = tempdir().unwrap();
        let live = dir.path().join("live");
        fs_create_dir(&live);
        let mut cfg = Config::default();
        cfg.aliases.insert("live".into(), live);
        cfg.aliases
            .insert("dead".into(), dir.path().join("missing"));
        cfg.groups
            .insert("g".into(), vec!["live".into(), "dead".into()]);
        cfg.tags.insert("t".into(), vec!["dead".into()]);
        let removed = prune_stale_aliases(&mut cfg);
        assert_eq!(removed, vec!["dead".to_string()]);
        assert!(!cfg.aliases.contains_key("dead"));
        assert_eq!(cfg.groups.get("g").unwrap(), &vec!["live".to_string()]);
        assert!(cfg.tags.get("t").unwrap().is_empty());
    }

    fn fs_create_dir(p: &Path) {
        std::fs::create_dir_all(p).unwrap();
    }
}
