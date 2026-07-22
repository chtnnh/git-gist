//! Repository discovery and selection.

use crate::cli::Cli;
use crate::config::{self, Config};
use crate::repo::Repo;
use anyhow::{Context, Result};
use globset::{Glob, GlobSetBuilder};
use jwalk::WalkDir;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveryCache {
    root: PathBuf,
    depth: usize,
    include_submodules: bool,
    scanned_at: u64,
    repos: Vec<PathBuf>,
}

pub fn select_repos(cli: &Cli, cfg: &Config) -> Result<Vec<Repo>> {
    let root = resolve_root(cli, cfg)?;
    let depth = if cfg.depth == 0 {
        usize::MAX
    } else {
        cfg.depth
    };

    let refresh = cli.refresh
        || matches!(
            &cli.command,
            Some(crate::cli::Commands::List { refresh: true })
        );

    let mut paths = if !refresh {
        load_cache(&root, depth, cfg.include_submodules)?
            .unwrap_or_else(|| discover_repos(&root, depth, cfg).unwrap_or_default())
    } else {
        discover_repos(&root, depth, cfg)?
    };

    // Discovery skips the search root itself (child repos only). When the user
    // passes `--root` and that directory is a git repo, include it.
    if cli.root.is_some() && is_git_repo(&root) && !paths.iter().any(|p| p == &root) {
        paths.insert(0, root.clone());
    }

    if refresh || load_cache(&root, depth, cfg.include_submodules)?.is_none() {
        let _ = save_cache(&root, depth, cfg.include_submodules, &paths);
    }

    // Include aliased repos under the search root only when they would also
    // pass discovery depth / ignore rules. Out-of-root aliases stay via `-i`.
    let ignores = build_ignore_set(&root, cfg)?;
    for path in cfg.aliases.values() {
        let canonical = canonicalize_soft(path);
        if !is_under_root(&canonical, &root) {
            continue;
        }
        if !alias_visible_under_root(&canonical, &root, depth, &ignores) {
            continue;
        }
        if is_git_repo(&canonical) && !paths.iter().any(|p| p == &canonical) {
            paths.push(canonical);
        }
    }

    // Resolve include/group targets
    let mut selected: BTreeSet<PathBuf> = if cli.include.is_empty() && cli.group.is_empty() {
        paths.iter().cloned().collect()
    } else {
        BTreeSet::new()
    };

    for target in cli.include.iter().chain(cli.group.iter()) {
        for p in resolve_target(target, cfg, &paths)? {
            selected.insert(p);
        }
    }

    // Excludes (exact match or directory prefix)
    let mut exclude: HashSet<PathBuf> = HashSet::new();
    for target in &cli.exclude {
        for p in resolve_target(target, cfg, &paths)? {
            exclude.insert(p);
        }
    }

    // Tag filter
    if !cli.tag.is_empty() {
        let tag_paths = paths_for_tags(&cli.tag, cfg);
        selected.retain(|p| tag_paths.contains(p));
    }

    // Repo overrides skip
    selected.retain(|p| {
        !cfg.repo_overrides.iter().any(|(key, ov)| {
            if !ov.skip {
                return false;
            }
            let key_path = canonicalize_soft(Path::new(key));
            key_path == *p || Path::new(key) == p.as_path()
        })
    });

    selected.retain(|p| !is_excluded(p, &exclude));

    let mut repos: Vec<Repo> = selected.into_iter().map(Repo::new).collect();
    repos.sort_by(|a, b| a.path.cmp(&b.path));

    // Status-based filters applied later when needed — but for selection flags, filter now
    if cli.only_dirty
        || cli.only_clean
        || cli.only_ahead
        || cli.only_behind
        || cli.only_stashed
        || cli.only_detached
    {
        repos = crate::filters::apply_status_filters(repos, cli, cfg.jobs)?;
    }

    Ok(repos)
}

fn resolve_root(cli: &Cli, cfg: &Config) -> Result<PathBuf> {
    if let Some(root) = cli.root.as_ref().or(cfg.root.as_ref()) {
        return Ok(canonicalize_soft(root));
    }
    Ok(std::env::current_dir()?)
}

pub fn discover_repos(root: &Path, max_depth: usize, cfg: &Config) -> Result<Vec<PathBuf>> {
    let ignores = build_ignore_set(root, cfg)?;

    let root_canon = canonicalize_soft(root);
    let mut found = Vec::new();
    let root = root.to_path_buf();

    for entry in WalkDir::new(&root)
        .max_depth(max_depth)
        .skip_hidden(false)
        .process_read_dir(move |_depth, _path, _state, children| {
            children.retain(|entry| {
                if let Ok(e) = entry {
                    let name = e.file_name.to_string_lossy();
                    // Don't descend into .git directories
                    if name == ".git" {
                        return false;
                    }
                    let path = e.path();
                    let rel = path.strip_prefix(&root).unwrap_or(path.as_path());
                    if ignores.is_match(rel) || ignores.is_match(&path) {
                        return false;
                    }
                }
                true
            });
        })
    {
        let entry = entry?;
        if !entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();
        if path.file_name().and_then(|s| s.to_str()) == Some(".git") {
            continue;
        }
        if is_git_repo(&path) {
            let path_canon = canonicalize_soft(&path);
            // Child repos only — skip the search root itself
            if path_canon == root_canon {
                continue;
            }
            let is_submodule = path.join(".git").is_file();
            if is_submodule && !cfg.include_submodules {
                continue;
            }
            found.push(path_canon);
        }
    }

    found.sort();
    found.dedup();
    Ok(found)
}

fn build_ignore_set(root: &Path, cfg: &Config) -> Result<globset::GlobSet> {
    let mut ignore_builder = GlobSetBuilder::new();
    for pattern in default_ignores().iter().chain(cfg.ignore.iter()) {
        ignore_builder
            .add(Glob::new(pattern).with_context(|| format!("bad ignore glob: {pattern}"))?);
    }
    let ggignore = root.join(".ggignore");
    if ggignore.is_file() {
        for line in fs::read_to_string(&ggignore)?.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            ignore_builder.add(Glob::new(line)?);
        }
    }
    Ok(ignore_builder.build()?)
}

fn alias_visible_under_root(
    path: &Path,
    root: &Path,
    max_depth: usize,
    ignores: &globset::GlobSet,
) -> bool {
    let path = canonicalize_soft(path);
    let root = canonicalize_soft(root);
    if path == root {
        return true;
    }
    let Ok(rel) = path.strip_prefix(&root) else {
        return false;
    };
    let depth = rel.components().count();
    if depth > max_depth {
        return false;
    }
    // Match ignore against the relative path and each ancestor segment prefix.
    if ignores.is_match(rel) || ignores.is_match(&path) {
        return false;
    }
    let mut acc = PathBuf::new();
    for comp in rel.components() {
        acc.push(comp);
        if ignores.is_match(&acc) {
            return false;
        }
    }
    true
}

fn is_excluded(path: &Path, exclude: &HashSet<PathBuf>) -> bool {
    exclude.iter().any(|e| {
        let e = canonicalize_soft(e);
        path == e.as_path() || path.starts_with(&e)
    })
}

fn default_ignores() -> Vec<String> {
    vec![
        "**/node_modules/**".into(),
        "**/target/**".into(),
        "**/.cache/**".into(),
        "**/vendor/**".into(),
        "**/.venv/**".into(),
        "**/dist/**".into(),
    ]
}

pub fn is_git_repo(path: &Path) -> bool {
    let git = path.join(".git");
    git.is_dir() || git.is_file()
}

fn canonicalize_soft(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn is_under_root(path: &Path, root: &Path) -> bool {
    let path = canonicalize_soft(path);
    let root = canonicalize_soft(root);
    path == root || path.starts_with(&root)
}

pub fn resolve_target(target: &str, cfg: &Config, discovered: &[PathBuf]) -> Result<Vec<PathBuf>> {
    // Group?
    if let Some(members) = cfg.groups.get(target) {
        let mut out = Vec::new();
        for m in members {
            out.extend(resolve_target(m, cfg, discovered)?);
        }
        return Ok(out);
    }
    // Alias?
    if let Some(path) = cfg.aliases.get(target) {
        let p = canonicalize_soft(path);
        return Ok(vec![p]);
    }
    // Path?
    let as_path = PathBuf::from(target);
    if as_path.exists() {
        let p = canonicalize_soft(&as_path);
        if p.is_dir() {
            // Directory: match all discovered repos under this prefix (and the
            // dir itself when it is a git repo).
            let mut matches: Vec<_> = discovered
                .iter()
                .filter(|d| is_under_root(d, &p))
                .cloned()
                .collect();
            if matches.is_empty() {
                matches.push(p);
            }
            return Ok(matches);
        }
        return Ok(vec![p]);
    }
    // Glob against discovered
    if target.contains('*') || target.contains('?') {
        let glob = Glob::new(target)?;
        let matcher = glob.compile_matcher();
        let matches: Vec<_> = discovered
            .iter()
            .filter(|p| matcher.is_match(p) || p.file_name().is_some_and(|n| matcher.is_match(n)))
            .cloned()
            .collect();
        return Ok(matches);
    }
    // Basename match
    let matches: Vec<_> = discovered
        .iter()
        .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some(target))
        .cloned()
        .collect();
    if !matches.is_empty() {
        return Ok(matches);
    }
    anyhow::bail!("unknown target: {target}");
}

pub fn paths_for_tags(tags: &[String], cfg: &Config) -> HashSet<PathBuf> {
    let mut out = HashSet::new();
    for tag in tags {
        if let Some(members) = cfg.tags.get(tag) {
            for m in members {
                if let Some(p) = cfg.aliases.get(m) {
                    out.insert(canonicalize_soft(p));
                } else {
                    out.insert(canonicalize_soft(Path::new(m)));
                }
            }
        }
        // Also check repo_overrides tags
        for (path, ov) in &cfg.repo_overrides {
            if ov.tags.iter().any(|t| t == tag) {
                out.insert(canonicalize_soft(Path::new(path)));
            }
        }
    }
    out
}

fn load_cache(root: &Path, depth: usize, include_submodules: bool) -> Result<Option<Vec<PathBuf>>> {
    let path = config::cache_path()?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    let cache: DiscoveryCache = match serde_json::from_str(&text) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    if cache.root != canonicalize_soft(root)
        || cache.depth != depth
        || cache.include_submodules != include_submodules
    {
        return Ok(None);
    }
    // Expire after 1 hour
    let now = now_secs();
    if now.saturating_sub(cache.scanned_at) > 3600 {
        return Ok(None);
    }
    Ok(Some(cache.repos))
}

fn save_cache(
    root: &Path,
    depth: usize,
    include_submodules: bool,
    repos: &[PathBuf],
) -> Result<()> {
    let path = config::cache_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cache = DiscoveryCache {
        root: canonicalize_soft(root),
        depth,
        include_submodules,
        scanned_at: now_secs(),
        repos: repos.to_vec(),
    };
    fs::write(path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
