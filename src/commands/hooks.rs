use crate::cli::{Cli, HooksAction};
use crate::config::Config;
use crate::output::OutputCtx;
use crate::repo::Repo;
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn run(
    action: &HooksAction,
    repos: &[Repo],
    _cli: &Cli,
    cfg: &Config,
    out: &mut OutputCtx,
) -> Result<()> {
    match action {
        HooksAction::List => {
            if out.is_json() {
                let map: serde_json::Map<String, serde_json::Value> = cfg
                    .hook_packs
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            serde_json::json!({
                                "description": v.description,
                                "hooks": v.hooks.keys().collect::<Vec<_>>(),
                            }),
                        )
                    })
                    .collect();
                out.write_json(&map)?;
            } else {
                for (name, pack) in &cfg.hook_packs {
                    let desc = pack.description.as_deref().unwrap_or("");
                    let hooks: Vec<_> = pack.hooks.keys().cloned().collect();
                    writeln!(out.stdout(), "{name}\t{}\t[{}]", desc, hooks.join(", "))?;
                }
            }
        }
        HooksAction::Install { pack } => {
            let pack_def = cfg
                .hook_packs
                .get(pack)
                .with_context(|| format!("unknown hook pack: {pack}"))?
                .clone();
            if repos.is_empty() {
                bail!("no repositories selected");
            }
            for repo in repos {
                install_pack(&repo.path, &pack_def)?;
                out.success(&format!("installed '{pack}' into {}", repo.display_path()))?;
            }
        }
    }
    Ok(())
}

fn install_pack(repo: &Path, pack: &crate::config::HookPack) -> Result<()> {
    let git_dir = crate::repo::git_stdout(repo, &["rev-parse", "--git-dir"])?;
    let hooks_dir = if Path::new(&git_dir).is_absolute() {
        Path::new(&git_dir).join("hooks")
    } else {
        repo.join(&git_dir).join("hooks")
    };
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
