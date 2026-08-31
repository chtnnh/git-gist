//! Resolution of configured remote URL templates for a repository name.

use anyhow::{bail, Context, Result};
use std::path::{Component, Path, PathBuf};

/// Return the UTF-8 directory name used in remote URL expansion.
///
/// Git remote URLs are UTF-8 configuration strings, so reject non-UTF-8 path
/// components consistently instead of silently inserting replacement bytes.
pub fn repository_name(path: &Path) -> Result<&str> {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .context("repository directory name must be valid UTF-8")
}

/// Resolve the directory name for an `init` target, including `.` and `..`.
pub fn target_repository_name(path: &Path) -> Result<String> {
    repository_name(&init_target_path(path)?).map(str::to_owned)
}

/// Resolve an init target without losing symlink semantics in nonexistent suffixes.
pub fn init_target_path(path: &Path) -> Result<PathBuf> {
    match path.canonicalize() {
        Ok(canonical) => Ok(canonical),
        Err(_) => normalize_target_path(path),
    }
}

fn normalize_target_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let components: Vec<_> = absolute.components().collect();
    let mut prefix = PathBuf::new();
    let mut last_existing_prefix = None;
    let mut existing_component_count = 0;
    for (index, component) in components.iter().enumerate() {
        prefix.push(component.as_os_str());
        if prefix.exists() {
            last_existing_prefix = Some(prefix.clone());
            existing_component_count = index + 1;
        }
    }

    // Canonicalize as much of the path as exists before applying any remaining
    // lexical components. This preserves symlink semantics for `link/new/..`.
    let mut normalized = match last_existing_prefix {
        Some(prefix) => prefix
            .canonicalize()
            .context("canonicalize init target prefix")?,
        None => PathBuf::new(),
    };
    for component in components.into_iter().skip(existing_component_count) {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            Component::Normal(name) => normalized.push(name),
        }
    }
    Ok(normalized)
}

/// Expand a configured remote specification for `repo_name`.
///
/// `{name}` and `{repo}` are explicit placeholders. A trailing `/` or `:` is
/// treated as an SSH/URL prefix and receives `<repo_name>.git`. All other
/// values are complete literal URLs.
pub fn requires_repository_name(spec: &str) -> Result<bool> {
    if spec.trim().is_empty() {
        bail!("remote URL specification must not be empty");
    }

    let is_template = spec.contains("{name}") || spec.contains("{repo}");
    let without_supported_placeholders = spec.replace("{name}", "").replace("{repo}", "");
    if without_supported_placeholders.contains('{') || without_supported_placeholders.contains('}')
    {
        bail!("invalid remote template {spec:?}: supported placeholders are {{name}} and {{repo}}");
    }
    Ok(is_template || is_scp_style_prefix(spec))
}

pub fn resolve_remote_url(spec: &str, repo_name: Option<&str>) -> Result<String> {
    let needs_repository_name = requires_repository_name(spec)?;
    if !needs_repository_name {
        return Ok(spec.to_owned());
    }
    let repo_name = repo_name
        .filter(|name| !name.is_empty())
        .context("remote template requires a repository directory name using valid UTF-8")?;

    let is_template = spec.contains("{name}") || spec.contains("{repo}");
    if is_template {
        return Ok(expand_template(spec, repo_name));
    }

    Ok(format!("{spec}{repo_name}.git"))
}

fn expand_template(spec: &str, repo_name: &str) -> String {
    let mut expanded = String::with_capacity(spec.len() + repo_name.len());
    let mut remaining = spec;
    loop {
        let name = remaining.find("{name}");
        let repo = remaining.find("{repo}");
        let next = match (name, repo) {
            (Some(name), Some(repo)) => Some((name.min(repo), 6)),
            (Some(name), None) => Some((name, 6)),
            (None, Some(repo)) => Some((repo, 6)),
            (None, None) => None,
        };
        let Some((index, placeholder_len)) = next else {
            expanded.push_str(remaining);
            return expanded;
        };
        expanded.push_str(&remaining[..index]);
        expanded.push_str(repo_name);
        remaining = &remaining[index + placeholder_len..];
    }
}

fn is_scp_style_prefix(spec: &str) -> bool {
    if spec.contains("://") || !(spec.ends_with('/') || spec.ends_with(':')) {
        return false;
    }
    let Some((authority, _path)) = spec.split_once(':') else {
        return false;
    };
    authority.contains('@') && !authority.contains('/')
}

#[cfg(test)]
mod tests {
    use super::resolve_remote_url;

    #[test]
    fn expands_supported_placeholders() {
        assert_eq!(
            resolve_remote_url("git@github.com:org/{name}.git", Some("demo")).unwrap(),
            "git@github.com:org/demo.git"
        );
        assert_eq!(
            resolve_remote_url("https://example.test/{repo}", Some("demo")).unwrap(),
            "https://example.test/demo"
        );
    }

    #[test]
    fn expands_documented_prefixes() {
        assert_eq!(
            resolve_remote_url("git@forgejo:org/", Some("demo")).unwrap(),
            "git@forgejo:org/demo.git"
        );
        assert_eq!(
            resolve_remote_url("git@forgejo:org:", Some("demo")).unwrap(),
            "git@forgejo:org:demo.git"
        );
        assert_eq!(
            resolve_remote_url("git@forgejo:", Some("demo")).unwrap(),
            "git@forgejo:demo.git"
        );
        assert_eq!(
            resolve_remote_url("git@forgejo:/srv/repos/", Some("demo")).unwrap(),
            "git@forgejo:/srv/repos/demo.git"
        );
    }

    #[test]
    fn retains_local_path_remotes_that_end_in_a_separator() {
        for spec in [
            "/srv/repos/shared/",
            "../shared/",
            "file:/srv/repos/shared/",
            "C:/repos/shared/",
        ] {
            assert_eq!(resolve_remote_url(spec, None).unwrap(), spec);
        }
    }

    #[test]
    fn retains_complete_urls() {
        assert_eq!(
            resolve_remote_url("https://example.test/org/demo.git", None).unwrap(),
            "https://example.test/org/demo.git"
        );
        assert_eq!(
            resolve_remote_url("https://example.test/org/shared/", None).unwrap(),
            "https://example.test/org/shared/"
        );
    }

    #[test]
    fn rejects_unknown_placeholders() {
        let err = resolve_remote_url("git@example.test/{project}.git", Some("demo")).unwrap_err();
        assert!(err.to_string().contains("supported placeholders"));
    }

    #[test]
    fn template_mode_does_not_also_apply_prefix_expansion() {
        assert_eq!(
            resolve_remote_url("https://example.test/{name}/", Some("demo")).unwrap(),
            "https://example.test/demo/"
        );
    }

    #[test]
    fn validates_the_specification_before_substituting_repo_name() {
        assert_eq!(
            resolve_remote_url("git@example.test/{name}.git", Some("project{legacy}")).unwrap(),
            "git@example.test/project{legacy}.git"
        );
    }

    #[test]
    fn does_not_reexpand_placeholders_inside_repository_names() {
        assert_eq!(
            resolve_remote_url("git@example.test/{name}", Some("x{repo}")).unwrap(),
            "git@example.test/x{repo}"
        );
    }

    #[test]
    fn rejects_blank_specifications() {
        let err = resolve_remote_url(" \t", None).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn literal_urls_do_not_require_a_repository_name() {
        assert_eq!(
            resolve_remote_url("https://example.test/team/shared.git", None).unwrap(),
            "https://example.test/team/shared.git"
        );
    }

    #[test]
    fn dynamic_urls_require_a_repository_name() {
        let err = resolve_remote_url("git@forgejo:chtnnh/", None).unwrap_err();
        assert!(err.to_string().contains("repository directory name"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_utf8_repository_directory_names() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use std::path::PathBuf;

        let path = PathBuf::from(OsString::from_vec(b"project-\xff".to_vec()));
        let err = super::repository_name(&path).unwrap_err();
        assert!(err.to_string().contains("UTF-8"));
    }
}
