use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

pub fn init(
    profile_name: Option<&str>,
    mode: InitMode,
    path: Option<&Path>,
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
    overrides: InitOverrides<'_>,
) -> Result<()> {
    init_with_mode(profile_name, mode, path, cli, cfg, out, overrides)
}

#[derive(Clone, Copy)]
pub struct InitMode {
    pub yes: bool,
    pub interactive: bool,
    pub allow_interactive: bool,
}

#[derive(Clone, Copy)]
pub struct InitOverrides<'a> {
    pub hooks: &'a [String],
    pub no_hooks: bool,
    pub remotes: &'a [String],
}

fn init_with_mode(
    profile_name: Option<&str>,
    mode: InitMode,
    path: Option<&Path>,
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
    overrides: InitOverrides<'_>,
) -> Result<()> {
    if let Some(name) = profile_name {
        cfg.profiles
            .get(name)
            .with_context(|| format!("unknown profile: {name}"))?;
    }
    if mode.allow_interactive
        && (mode.interactive
            || (!mode.yes
                && overrides.hooks.is_empty()
                && !overrides.no_hooks
                && overrides.remotes.is_empty()
                && std::io::stdin().is_terminal()
                && std::io::stderr().is_terminal()))
    {
        return interactive_init(profile_name, path, cli, cfg, out);
    }
    let name = profile_name.unwrap_or("default");
    let mut profile = cfg
        .profiles
        .get(name)
        .with_context(|| format!("unknown profile: {name}"))?
        .clone();
    if overrides.no_hooks {
        profile.hooks.clear();
    } else if !overrides.hooks.is_empty() {
        profile.hooks = overrides.hooks.to_vec();
    }
    for hook in &profile.hooks {
        if !cfg.hook_packs.contains_key(hook) {
            bail!("unknown hook pack: {hook}");
        }
    }

    let requested_target = path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let target = crate::remote_url::init_target_path(&requested_target)?;

    // Resolve the complete remote set before creating the directory or running
    // `git init`, so malformed templates fail without partial scaffolding.
    let use_catalog_defaults = mode.yes
        || (!mode.interactive
            && (!std::io::stdin().is_terminal() || !std::io::stderr().is_terminal()));
    let remote_specs = if !overrides.remotes.is_empty() {
        overrides
            .remotes
            .iter()
            .map(|name| {
                cfg.remotes
                    .get(name)
                    .map(|value| (name.clone(), value.clone()))
                    .with_context(|| format!("unknown remote catalog entry: {name}"))
            })
            .collect::<Result<BTreeMap<_, _>>>()?
    } else if use_catalog_defaults && profile.remotes.is_empty() {
        cfg.remotes.clone()
    } else {
        profile.remotes.clone()
    };
    let resolved_remotes: Vec<_> = remote_specs
        .iter()
        .map(|(remote_name, value)| {
            crate::repo::validate_remote_name(remote_name)?;
            let spec = cfg.remotes.get(value).map(String::as_str).unwrap_or(value);
            let repo_name = if crate::remote_url::requires_repository_name(spec)? {
                Some(crate::remote_url::target_repository_name(&target)?)
            } else {
                None
            };
            crate::remote_url::resolve_remote_url(spec, repo_name.as_deref())
                .map(|url| (remote_name.as_str(), url))
        })
        .collect::<Result<_>>()?;

    if cli.dry_run {
        out.info(&format!(
            "dry-run: would scaffold {} with profile '{name}'",
            requested_target.display()
        ))?;
        return Ok(());
    }

    let created_directory_boundary = match fs::metadata(&target) {
        Ok(_) => None,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Some(nearest_existing_ancestor(&target)?)
        }
        Err(err) => return Err(err).context("inspect init target"),
    };
    let git_dir_existed = target.join(".git").exists();
    let mut files = FileJournal::default();
    if git_dir_existed {
        files.capture(git_metadata_path(&target, "HEAD")?)?;
        files.capture(git_metadata_path(&target, "config")?)?;
    }
    fs::create_dir_all(&target)?;

    let init_result = crate::repo::git_command()
        .args(["init"])
        .current_dir(&target)
        .status()
        .context("git init");
    if !matches!(init_result, Ok(ref status) if status.success()) {
        if let Err(err) = files.restore() {
            return Err(err).context("restore existing Git metadata after init failure");
        }
        if !git_dir_existed {
            remove_new_git_dir(&target)?;
        }
        if let Some(boundary) = created_directory_boundary.as_deref() {
            remove_created_directories(&target, boundary)?;
        }
        match init_result {
            Ok(_) => bail!("git init failed"),
            Err(err) => return Err(err),
        }
    }

    let mut added_remotes = Vec::new();
    for (remote_name, url) in &resolved_remotes {
        let result = crate::repo::git_command()
            .args(["remote", "add", remote_name, url])
            .current_dir(&target)
            .status()
            .context("git remote add")
            .and_then(|status| {
                if status.success() {
                    Ok(())
                } else {
                    bail!("failed to add remote {remote_name} to {}", target.display())
                }
            });
        if let Err(err) = result {
            let rollback = rollback_remote_setup(
                &target,
                &added_remotes,
                files,
                !git_dir_existed,
                created_directory_boundary.as_deref(),
            );
            if let Err(rollback) = rollback {
                return Err(rollback).context(format!(
                    "{err:#}; rollback of remotes added by this init also failed"
                ));
            }
            return Err(err);
        }
        added_remotes.push(*remote_name);
    }

    let file_result = (|| -> Result<()> {
        if let Some(readme) = &profile.readme {
            files.write(target.join("README.md"), readme)?;
        }
        if let Some(gitignore) = &profile.gitignore {
            files.write(target.join(".gitignore"), gitignore)?;
        }
        if let Some(license) = &profile.license {
            files.write(target.join("LICENSE"), license)?;
        }

        for pack_name in &profile.hooks {
            if let Some(pack) = cfg.hook_packs.get(pack_name) {
                install_pack(&target, pack, &mut files)?;
                out.info(&format!("installed hook pack '{pack_name}'"))?;
            } else {
                out.warn(&format!("hook pack not found: {pack_name}"))?;
            }
        }
        Ok(())
    })();
    if let Err(err) = file_result {
        let rollback = rollback_remote_setup(
            &target,
            &added_remotes,
            files,
            !git_dir_existed,
            created_directory_boundary.as_deref(),
        );
        if let Err(rollback) = rollback {
            return Err(rollback).context(format!("{err:#}; remote rollback also failed"));
        }
        return Err(err);
    }

    let metadata_result = (|| -> Result<()> {
        files.capture(git_metadata_path(&target, "HEAD")?)?;
        files.capture(git_metadata_path(&target, "config")?)?;
        Ok(())
    })();
    if let Err(err) = metadata_result {
        let rollback = rollback_remote_setup(
            &target,
            &added_remotes,
            files,
            !git_dir_existed,
            created_directory_boundary.as_deref(),
        );
        if let Err(rollback) = rollback {
            return Err(rollback).context(format!("{err:#}; remote rollback also failed"));
        }
        return Err(err);
    }
    let settings_result = apply_settings(&target, &profile);
    if let Err(err) = settings_result {
        let rollback = rollback_remote_setup(
            &target,
            &added_remotes,
            files,
            !git_dir_existed,
            created_directory_boundary.as_deref(),
        );
        if let Err(rollback) = rollback {
            return Err(rollback).context(format!("{err:#}; remote rollback also failed"));
        }
        return Err(err);
    }

    out.success(&format!(
        "scaffolded {} with profile '{name}'",
        requested_target.display()
    ))?;
    Ok(())
}

#[cfg(coverage)]
fn interactive_init(
    _: Option<&str>,
    _: Option<&Path>,
    _: &Cli,
    _: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    out.info("interactive UI skipped under coverage")
}

#[cfg(all(not(coverage), feature = "wizard"))]
fn interactive_init(
    profile_name: Option<&str>,
    path: Option<&Path>,
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    use inquire::{Confirm, MultiSelect, Select};

    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        bail!("--interactive requires an interactive terminal; use --yes in scripts");
    }
    if out.is_json() {
        bail!("--interactive is incompatible with JSON output");
    }
    let names: Vec<_> = cfg.profiles.keys().cloned().collect();
    let initial = profile_name.unwrap_or("default");
    let selected_name = Select::new("Scaffold profile", names)
        .with_starting_cursor(
            cfg.profiles
                .keys()
                .position(|name| name == initial)
                .unwrap_or(0),
        )
        .prompt()?;
    let mut interactive_cfg = cfg.clone();
    let profile = interactive_cfg
        .profiles
        .get_mut(&selected_name)
        .context("selected scaffold profile disappeared")?;
    let hook_names: Vec<_> = cfg.hook_packs.keys().cloned().collect();
    let selected_hook_indexes: Vec<_> = hook_names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| profile.hooks.contains(name).then_some(index))
        .collect();
    profile.hooks = MultiSelect::new("Hook packs", hook_names)
        .with_default(&selected_hook_indexes)
        .prompt()?;
    let include_catalog_remotes = !cfg.remotes.is_empty()
        && Confirm::new("Add catalog remotes?")
            .with_default(profile.remotes.is_empty())
            .prompt()?;
    if include_catalog_remotes {
        let selected =
            MultiSelect::new("Catalog remotes", cfg.remotes.keys().cloned().collect()).prompt()?;
        profile.remotes.extend(
            selected
                .into_iter()
                .filter_map(|name| cfg.remotes.get(&name).map(|value| (name, value.clone())))
                .collect::<BTreeMap<_, _>>(),
        );
    }
    if !Confirm::new("Create README.md?")
        .with_default(profile.readme.is_some())
        .prompt()?
    {
        profile.readme = None;
    } else if profile.readme.is_none() {
        profile.readme = Some("# README\n".into());
    }
    if !Confirm::new("Create LICENSE?")
        .with_default(profile.license.is_some())
        .prompt()?
    {
        profile.license = None;
    } else if profile.license.is_none() {
        profile.license = Some("All rights reserved.\n".into());
    }
    if !Confirm::new("Create .gitignore?")
        .with_default(profile.gitignore.is_some())
        .prompt()?
    {
        profile.gitignore = None;
    } else if profile.gitignore.is_none() {
        profile.gitignore = Some(String::new());
    }
    init_with_mode(
        Some(&selected_name),
        InitMode {
            yes: false,
            interactive: false,
            allow_interactive: false,
        },
        path,
        cli,
        &interactive_cfg,
        out,
        InitOverrides {
            hooks: &[],
            no_hooks: false,
            remotes: &[],
        },
    )
}

#[cfg(all(not(coverage), not(feature = "wizard")))]
fn interactive_init(
    _: Option<&str>,
    _: Option<&Path>,
    _: &Cli,
    _: &Config,
    _: &mut OutputCtx,
) -> Result<()> {
    bail!("wizard feature disabled — rebuild with --features wizard or use --yes")
}

fn rollback_remote_setup(
    target: &Path,
    remote_names: &[&str],
    files: FileJournal,
    remove_created_git_dir: bool,
    created_directory_boundary: Option<&Path>,
) -> Result<()> {
    let mut failures = Vec::new();
    for remote_name in remote_names.iter().rev() {
        match crate::repo::git_command()
            .args(["remote", "remove", remote_name])
            .current_dir(target)
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(_) => failures.push(format!(
                "failed to remove remote {remote_name} during rollback"
            )),
            Err(err) => failures.push(format!("git remote remove during rollback: {err}")),
        }
    }
    if let Err(err) = files.restore() {
        failures.push(format!("file rollback: {err:#}"));
    }
    if remove_created_git_dir {
        if let Err(err) = remove_new_git_dir(target) {
            failures.push(format!("{err:#}"));
        }
    }
    if let Some(boundary) = created_directory_boundary {
        if let Err(err) = remove_created_directories(target, boundary) {
            failures.push(format!("{err:#}"));
        }
    }
    if !failures.is_empty() {
        bail!("rollback failed: {}", failures.join("; "));
    }
    Ok(())
}

fn nearest_existing_ancestor(path: &Path) -> Result<PathBuf> {
    let mut ancestor = path;
    loop {
        match fs::metadata(ancestor) {
            Ok(_) => return Ok(ancestor.to_path_buf()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                ancestor = ancestor
                    .parent()
                    .context("init target has no existing ancestor")?;
            }
            Err(err) => return Err(err).context("inspect init target ancestor"),
        }
    }
}

fn remove_created_directories(target: &Path, boundary: &Path) -> Result<()> {
    let mut current = target;
    while current != boundary {
        fs::remove_dir(current).context("remove newly created init directory")?;
        current = current
            .parent()
            .context("created init directory has no parent")?;
    }
    Ok(())
}

fn remove_new_git_dir(target: &Path) -> Result<()> {
    let git_dir = target.join(".git");
    if git_dir.is_dir() {
        fs::remove_dir_all(&git_dir).context("remove newly initialized .git directory")?;
    } else if git_dir.exists() {
        fs::remove_file(&git_dir).context("remove newly initialized .git file")?;
    }
    Ok(())
}

#[derive(Default)]
struct FileJournal {
    snapshots: BTreeMap<PathBuf, FileSnapshot>,
    directories: Vec<PathBuf>,
}

struct FileSnapshot {
    contents: Option<Vec<u8>>,
    permissions: Option<fs::Permissions>,
}

impl FileJournal {
    fn create_dir_all(&mut self, path: &Path) -> Result<()> {
        let mut missing = Vec::new();
        let mut current = path;
        while !current.exists() {
            missing.push(current.to_path_buf());
            current = current
                .parent()
                .context("scaffold directory has no parent")?;
        }
        fs::create_dir_all(path).context("create scaffold directory")?;
        self.directories.extend(missing);
        Ok(())
    }

    fn write(&mut self, path: PathBuf, body: &str) -> Result<()> {
        self.capture(path.clone())?;
        fs::write(path, body).context("write scaffold file")
    }

    fn capture(&mut self, path: PathBuf) -> Result<()> {
        if !self.snapshots.contains_key(&path) {
            let contents = match fs::read(&path) {
                Ok(contents) => Some(contents),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                Err(err) => return Err(err).context("snapshot scaffold file"),
            };
            let permissions = if contents.is_some() {
                Some(
                    fs::metadata(&path)
                        .context("snapshot scaffold file permissions")?
                        .permissions(),
                )
            } else {
                None
            };
            self.snapshots.insert(
                path.clone(),
                FileSnapshot {
                    contents,
                    permissions,
                },
            );
        }
        Ok(())
    }

    fn restore(self) -> Result<()> {
        let mut failures = Vec::new();
        for (path, snapshot) in self.snapshots {
            let result = match snapshot.contents {
                Some(contents) => (|| -> Result<()> {
                    fs::write(&path, contents).context("restore scaffold file")?;
                    if let Some(permissions) = snapshot.permissions {
                        fs::set_permissions(&path, permissions)
                            .context("restore scaffold file permissions")?;
                    }
                    Ok(())
                })(),
                None if path.exists() => fs::remove_file(&path).context("remove scaffold file"),
                None => Ok(()),
            };
            if let Err(err) = result {
                failures.push(format!("{err:#}"));
            }
        }
        for path in self.directories {
            if let Err(err) = fs::remove_dir(&path).context("remove scaffold directory") {
                failures.push(format!("{err:#}"));
            }
        }
        if !failures.is_empty() {
            bail!("file rollback failed: {}", failures.join("; "));
        }
        Ok(())
    }
}

fn apply_settings(target: &Path, profile: &crate::config::ScaffoldProfile) -> Result<()> {
    let mut commands: Vec<Vec<String>> = Vec::new();
    if let Some(branch) = &profile.default_branch {
        commands.push(vec![
            "symbolic-ref".into(),
            "HEAD".into(),
            format!("refs/heads/{branch}"),
        ]);
    }
    if let Some(user) = &profile.user_name {
        commands.push(vec!["config".into(), "user.name".into(), user.clone()]);
    }
    if let Some(email) = &profile.user_email {
        commands.push(vec!["config".into(), "user.email".into(), email.clone()]);
    }
    for command in commands {
        let args: Vec<_> = command.iter().map(String::as_str).collect();
        let status = crate::repo::git_command()
            .args(args)
            .current_dir(target)
            .status()
            .context("apply scaffold git setting")?;
        if !status.success() {
            bail!("failed to apply scaffold git setting");
        }
    }
    Ok(())
}

fn git_metadata_path(target: &Path, name: &str) -> Result<PathBuf> {
    let output = crate::repo::git_command()
        .args(["rev-parse", "--git-path", name])
        .current_dir(target)
        .output()
        .context("resolve git metadata path")?;
    if !output.status.success() {
        bail!("failed to resolve git metadata path: {name}");
    }
    let value = String::from_utf8(output.stdout).context("git metadata path must be UTF-8")?;
    let path = PathBuf::from(value.trim());
    Ok(if path.is_absolute() {
        path
    } else {
        target.join(path)
    })
}

fn git_common_dir_path(target: &Path) -> Result<PathBuf> {
    let output = crate::repo::git_command()
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(target)
        .output()
        .context("resolve common git directory")?;
    if !output.status.success() {
        bail!("failed to resolve common git directory");
    }
    git_path_from_output(target, output.stdout, "common git directory")
}

#[cfg(unix)]
fn git_path_from_output(target: &Path, mut output: Vec<u8>, _kind: &str) -> Result<PathBuf> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    if output.last() == Some(&b'\n') {
        output.pop();
    }
    let path = PathBuf::from(OsString::from_vec(output));
    Ok(if path.is_absolute() {
        path
    } else {
        target.join(path)
    })
}

#[cfg(not(unix))]
fn git_path_from_output(target: &Path, output: Vec<u8>, kind: &str) -> Result<PathBuf> {
    let value = String::from_utf8(output).with_context(|| format!("{kind} must be UTF-8"))?;
    let path = PathBuf::from(value.trim());
    Ok(if path.is_absolute() {
        path
    } else {
        target.join(path)
    })
}

fn install_pack(
    repo: &Path,
    pack: &crate::config::HookPack,
    files: &mut FileJournal,
) -> Result<()> {
    let hooks_dir = git_common_dir_path(repo)?.join("hooks");
    files.create_dir_all(&hooks_dir)?;
    for (name, body) in &pack.hooks {
        let path = hooks_dir.join(name);
        let contents = if name == "pre-commit" && path.exists() {
            let previous = fs::read_to_string(&path).context("read existing scaffold hook")?;
            format!(
                "#!/bin/sh\nset -e\n{}\n{}\n",
                previous
                    .strip_prefix("#!/bin/sh\n")
                    .unwrap_or(&previous)
                    .trim_end()
                    .strip_suffix("exit 0")
                    .unwrap_or_else(|| previous.trim_end()),
                body.strip_prefix("#!/bin/sh\n").unwrap_or(body)
            )
        } else {
            body.clone()
        };
        files.write(path.clone(), &contents)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::git_path_from_output;
    use super::{
        apply_settings, git_metadata_path, install_pack, remove_created_directories, FileJournal,
    };
    use crate::config::{HookPack, ScaffoldProfile};
    use std::collections::BTreeMap;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn file_journal_restores_existing_and_new_files() {
        let dir = tempdir().unwrap();
        let existing = dir.path().join("existing");
        let created = dir.path().join("created");
        fs::write(&existing, "original").unwrap();
        let mut journal = FileJournal::default();
        journal.write(existing.clone(), "changed").unwrap();
        journal.write(created.clone(), "new").unwrap();
        journal.restore().unwrap();
        assert_eq!(fs::read_to_string(existing).unwrap(), "original");
        assert!(!created.exists());
    }

    #[test]
    fn resolves_git_metadata_and_applies_profile_settings() {
        let dir = tempdir().unwrap();
        assert!(Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success());
        let profile = ScaffoldProfile {
            default_branch: Some("trunk".into()),
            user_name: Some("Test User".into()),
            user_email: Some("test@example.test".into()),
            ..Default::default()
        };
        apply_settings(dir.path(), &profile).unwrap();
        assert!(git_metadata_path(dir.path(), "HEAD").unwrap().is_file());
        assert!(git_metadata_path(dir.path(), "config").unwrap().is_file());
    }

    #[test]
    fn installs_hooks_in_the_common_git_dir_for_a_linked_worktree() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        let worktree = dir.path().join("worktree");
        fs::create_dir(&repo).unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["commit", "--allow-empty", "-m", "initial"],
            vec![
                "worktree",
                "add",
                "-b",
                "feature",
                worktree.to_str().unwrap(),
            ],
        ] {
            assert!(Command::new("git")
                .args(args)
                .current_dir(&repo)
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.test")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.test")
                .status()
                .unwrap()
                .success());
        }
        let pack = HookPack {
            hooks: BTreeMap::from([("pre-commit".into(), "#!/bin/sh\nexit 0\n".into())]),
            ..Default::default()
        };

        install_pack(&worktree, &pack, &mut FileJournal::default()).unwrap();

        assert!(repo.join(".git").join("hooks").join("pre-commit").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn preserves_non_utf8_git_path_output() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let dir = tempdir().unwrap();
        let path = git_path_from_output(dir.path(), b"hooks-\xff\n".to_vec(), "hooks").unwrap();
        assert_eq!(
            path,
            dir.path().join(OsString::from_vec(b"hooks-\xff".to_vec()))
        );
    }

    #[test]
    fn helper_error_and_noop_paths_are_covered() {
        let dir = tempdir().unwrap();
        assert!(git_metadata_path(dir.path(), "HEAD").is_err());
        apply_settings(dir.path(), &ScaffoldProfile::default()).unwrap();

        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();
        remove_created_directories(&child, dir.path()).unwrap();
        assert!(!parent.exists());

        let removed_before_restore = dir.path().join("removed-before-restore");
        let mut journal = FileJournal::default();
        journal
            .write(removed_before_restore.clone(), "temporary")
            .unwrap();
        fs::remove_file(&removed_before_restore).unwrap();
        journal.restore().unwrap();
        assert!(!removed_before_restore.exists());

        let created_dir = dir.path().join("journal").join("nested");
        let mut journal = FileJournal::default();
        journal.create_dir_all(&created_dir).unwrap();
        journal.restore().unwrap();
        assert!(!dir.path().join("journal").exists());

        let blocked_dir = dir.path().join("blocked");
        let mut journal = FileJournal::default();
        journal.create_dir_all(&blocked_dir).unwrap();
        fs::write(blocked_dir.join("left-behind"), "x").unwrap();
        assert!(journal.restore().is_err());
    }
}
