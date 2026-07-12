use crate::cli::Cli;
use crate::config::Config;
use crate::output::OutputCtx;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn init(
    profile_name: Option<&str>,
    path: Option<&Path>,
    _cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    let name = profile_name.unwrap_or("default");
    let profile = cfg
        .profiles
        .get(name)
        .with_context(|| format!("unknown profile: {name}"))?
        .clone();

    let target = path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));

    fs::create_dir_all(&target)?;

    let status = Command::new("git")
        .args(["init"])
        .current_dir(&target)
        .status()
        .context("git init")?;
    if !status.success() {
        bail!("git init failed");
    }

    if let Some(branch) = &profile.default_branch {
        let _ = Command::new("git")
            .args(["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")])
            .current_dir(&target)
            .status();
    }

    if let Some(user) = &profile.user_name {
        let _ = Command::new("git")
            .args(["config", "user.name", user])
            .current_dir(&target)
            .status();
    }
    if let Some(email) = &profile.user_email {
        let _ = Command::new("git")
            .args(["config", "user.email", email])
            .current_dir(&target)
            .status();
    }

    for (remote_name, url) in &profile.remotes {
        let _ = Command::new("git")
            .args(["remote", "add", remote_name, url])
            .current_dir(&target)
            .status();
    }

    // Also apply catalog remotes referenced by name-only in profile.remotes values? already URLs.

    if let Some(gitignore) = &profile.gitignore {
        fs::write(target.join(".gitignore"), gitignore)?;
    }
    if let Some(license) = &profile.license {
        fs::write(target.join("LICENSE"), license)?;
    }

    for pack_name in &profile.hooks {
        if let Some(pack) = cfg.hook_packs.get(pack_name) {
            install_pack(&target, pack)?;
            out.info(&format!("installed hook pack '{pack_name}'"))?;
        } else {
            out.warn(&format!("hook pack not found: {pack_name}"))?;
        }
    }

    out.success(&format!(
        "scaffolded {} with profile '{name}'",
        target.display()
    ))?;
    Ok(())
}

fn install_pack(repo: &Path, pack: &crate::config::HookPack) -> Result<()> {
    let hooks_dir = repo.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    for (name, body) in &pack.hooks {
        let path = hooks_dir.join(name);
        fs::write(&path, body)?;
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
