use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn init(
    profile_name: Option<&str>,
    path: Option<&Path>,
    cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    let name = profile_name.unwrap_or("default");
    let profile = cfg
        .profiles
        .get(name)
        .with_context(|| format!("unknown profile: {name}"))?
        .clone();

    let requested_target = path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let target = crate::remote_url::init_target_path(&requested_target)?;

    // Resolve the complete remote set before creating the directory or running
    // `git init`, so malformed templates fail without partial scaffolding.
    let resolved_remotes: Vec<_> = profile
        .remotes
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
        files.write(path.clone(), body)?;
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
    use super::{
        apply_settings, git_metadata_path, git_path_from_output, install_pack,
        remove_created_directories, FileJournal,
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
