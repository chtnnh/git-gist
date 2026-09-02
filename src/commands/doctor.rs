use crate::cli::{Cli, ShellKind};
use crate::config::{self, Config};
use crate::config_ops;
use crate::output::OutputCtx;
use crate::repo::{ProbeOpts, Repo};
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use which::which;

#[derive(Clone, Serialize)]
struct DoctorFinding {
    level: String,
    repo: Option<String>,
    message: String,
}

pub fn run(repos: &[Repo], _cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    let mut findings = Vec::new();

    if which("git").is_err() {
        #[cfg(not(coverage))]
        findings.push(DoctorFinding {
            level: "error".into(),
            repo: None,
            message: "git not found on PATH".into(),
        });
    } else if let Ok(output) = crate::repo::git_command().arg("--version").output() {
        let v = String::from_utf8_lossy(&output.stdout).trim().to_string();
        findings.push(DoctorFinding {
            level: "info".into(),
            repo: None,
            message: format!("found {v}"),
        });
    }

    let pool = crate::exec::job_pool(cfg)?;
    let show_path = out.show_path;
    let root = out.root.clone();
    let mut repo_findings: Vec<DoctorFinding> = pool.install(|| {
        repos
            .par_iter()
            .flat_map(|repo| {
                let label = repo.label(show_path, root.as_deref());
                let mut local = Vec::new();
                let git = repo.path.join(".git");
                if git.is_file() {
                    local.push(DoctorFinding {
                        level: "info".into(),
                        repo: Some(label.clone()),
                        message: "gitfile (.git file) — likely worktree or submodule".into(),
                    });
                }
                match crate::repo::probe_with(&repo.path, ProbeOpts::DOCTOR) {
                    Ok(status) => {
                        if status.detached {
                            local.push(DoctorFinding {
                                level: "warn".into(),
                                repo: Some(label.clone()),
                                message: format!("detached HEAD at {}", status.branch),
                            });
                        }
                        if let Some(op) = status.in_progress {
                            local.push(DoctorFinding {
                                level: "warn".into(),
                                repo: Some(label.clone()),
                                message: format!("{op} in progress"),
                            });
                        }
                        if status.upstream.is_none() && !status.detached {
                            local.push(DoctorFinding {
                                level: "info".into(),
                                repo: Some(label.clone()),
                                message: "no upstream configured".into(),
                            });
                        }
                    }
                    Err(e) => local.push(DoctorFinding {
                        level: "error".into(),
                        repo: Some(label),
                        message: format!("probe failed: {e}"),
                    }),
                }
                local
            })
            .collect()
    });
    findings.append(&mut repo_findings);

    emit_findings(&findings, repos.len(), out)
}

pub fn run_config(cfg: &Config, out: &mut OutputCtx) -> Result<()> {
    let mut findings = Vec::new();

    let path = cfg.path.clone().unwrap_or(config::global_config_path()?);
    findings.push(DoctorFinding {
        level: "info".into(),
        repo: None,
        message: format!("config path: {}", path.display()),
    });

    for legacy in config::legacy_global_config_paths() {
        if legacy.is_file() && legacy != path {
            findings.push(DoctorFinding {
                level: "info".into(),
                repo: None,
                message: format!("legacy config still present: {}", legacy.display()),
            });
        }
    }

    for w in &cfg.load_warnings {
        findings.push(DoctorFinding {
            level: "warn".into(),
            repo: None,
            message: w.clone(),
        });
    }

    if cfg.auto_enroll.is_empty() {
        findings.push(DoctorFinding {
            level: "warn".into(),
            repo: None,
            message:
                "no [[auto_enroll]] rules — run `gg config enroll wizard` or `gg config wizard`"
                    .into(),
        });
    }

    for (i, rule) in cfg.auto_enroll.iter().enumerate() {
        if !rule.path.is_dir() {
            findings.push(DoctorFinding {
                level: "warn".into(),
                repo: None,
                message: format!(
                    "auto_enroll[{i}] watch path missing: {}",
                    rule.path.display()
                ),
            });
        }
        if let Some(root) = &cfg.root {
            let root_c = root.canonicalize().unwrap_or_else(|_| root.clone());
            let rule_c = rule
                .path
                .canonicalize()
                .unwrap_or_else(|_| rule.path.clone());
            if root_c == rule_c
                && (!rule.groups.is_empty() || !rule.tags.is_empty())
                && rule
                    .path_prefix
                    .as_ref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
            {
                findings.push(DoctorFinding {
                    level: "warn".into(),
                    repo: None,
                    message: format!(
                        "auto_enroll[{i}] path equals config root with groups/tags and no path_prefix"
                    ),
                });
            }
        }
    }

    let stale = config_ops::list_stale_aliases(cfg);
    if !stale.is_empty() {
        findings.push(DoctorFinding {
            level: "warn".into(),
            repo: None,
            message: format!(
                "{} stale alias(es) — run `gg alias prune` or `gg alias wizard` to reclaim short names",
                stale.len()
            ),
        });
        for (name, p) in stale.iter().take(10) {
            findings.push(DoctorFinding {
                level: "info".into(),
                repo: None,
                message: format!("stale alias {name} → {}", p.display()),
            });
        }
    }

    for (group, members) in &cfg.groups {
        for m in members {
            if !cfg.aliases.contains_key(m) {
                findings.push(DoctorFinding {
                    level: "warn".into(),
                    repo: None,
                    message: format!("group `{group}` references missing alias `{m}`"),
                });
            }
        }
    }
    for (tag, members) in &cfg.tags {
        for m in members {
            if !cfg.aliases.contains_key(m) {
                findings.push(DoctorFinding {
                    level: "warn".into(),
                    repo: None,
                    message: format!("tag `{tag}` references missing alias `{m}`"),
                });
            }
        }
    }

    let mut basename_counts: HashMap<String, Vec<String>> = HashMap::new();
    for (name, path) in &cfg.aliases {
        let base = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.clone());
        basename_counts.entry(base).or_default().push(name.clone());
    }
    for (base, names) in basename_counts {
        if names.len() > 1 {
            findings.push(DoctorFinding {
                level: "info".into(),
                repo: None,
                message: format!(
                    "duplicate basename `{base}` across aliases: {}",
                    names.join(", ")
                ),
            });
        }
    }

    emit_findings(&findings, 0, out)
}

pub fn run_shell(shell: ShellKind, setup: bool, out: &mut OutputCtx) -> Result<()> {
    if setup {
        writeln!(out.stdout(), "{}", setup_snippet(shell))?;
        return Ok(());
    }

    let mut findings = Vec::new();
    for path in shell_startup_files(shell)? {
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(collision) = find_gg_collision(shell, &contents) {
            findings.push(DoctorFinding {
                level: "warn".into(),
                repo: None,
                message: format!(
                    "gg collision in {} ({}) — run `{}` for this shell, then remove or rename its definition in that file and restart {}",
                    path.display(),
                    collision.label(),
                    collision.remediation(shell),
                    shell_name(shell),
                ),
            });
        }
    }

    if findings.is_empty() {
        findings.push(DoctorFinding {
            level: "info".into(),
            repo: None,
            message: format!(
                "no persisted gg collision found in scanned {} startup file(s); use --setup to check the active shell",
                shell_name(shell)
            ),
        });
    }
    emit_findings(&findings, 0, out)
}

fn shell_startup_files(shell: ShellKind) -> Result<Vec<PathBuf>> {
    let home = shell_home_dir()?;
    let paths = match shell {
        ShellKind::Bash => bash_startup_files(&home),
        ShellKind::Zsh => zsh_startup_files(&home),
        ShellKind::Fish => {
            let config_home = std::env::var_os("XDG_CONFIG_HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".config"));
            let fish = config_home.join("fish");
            let mut paths = vec![fish.join("config.fish"), fish.join("functions/gg.fish")];
            if let Ok(entries) = std::fs::read_dir(fish.join("conf.d")) {
                paths.extend(
                    entries
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension()
                                .is_some_and(|extension| extension == "fish")
                        }),
                );
            }
            paths
        }
    };
    Ok(paths)
}

fn shell_home_dir() -> Result<PathBuf> {
    for key in ["HOME", "USERPROFILE"] {
        if let Some(value) = std::env::var_os(key).filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(value));
        }
    }
    config::home_dir()
}

fn bash_startup_files(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![home.join(".bashrc"), home.join(".bash_aliases")];
    if let Some(login) = [".bash_profile", ".bash_login", ".profile"]
        .into_iter()
        .map(|name| home.join(name))
        .find(|path| std::fs::File::open(path).is_ok())
    {
        paths.push(login);
    }
    paths
}

fn zsh_startup_files(home: &Path) -> Vec<PathBuf> {
    if let Some(dotdir) = std::env::var_os("ZDOTDIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return zsh_files_from(&dotdir);
    }

    let dotdir = zshenv_dotdir(home).unwrap_or_else(|| home.to_path_buf());
    let mut paths = vec![home.join(".zshenv")];
    paths.extend(zsh_files_after_zshenv(&dotdir));
    paths
}

fn zsh_files_from(dotdir: &Path) -> Vec<PathBuf> {
    let mut paths = vec![dotdir.join(".zshenv")];
    paths.extend(zsh_files_after_zshenv(dotdir));
    paths
}

fn zsh_files_after_zshenv(dotdir: &Path) -> Vec<PathBuf> {
    vec![
        dotdir.join(".zprofile"),
        dotdir.join(".zshrc"),
        dotdir.join(".zlogin"),
    ]
}

fn zshenv_dotdir(home: &Path) -> Option<PathBuf> {
    let contents = std::fs::read_to_string(home.join(".zshenv")).ok()?;
    contents.lines().find_map(|line| {
        let words = shell_words(line)?;
        let assignment = match words.as_slice() {
            [assignment] => assignment,
            [command, assignment] if command == "export" => assignment,
            _ => return None,
        };
        let value = assignment.strip_prefix("ZDOTDIR=")?;
        Some(expand_home_prefix(value, home))
    })
}

fn expand_home_prefix(value: &str, home: &Path) -> PathBuf {
    if let Some(suffix) = value.strip_prefix("${XDG_CONFIG_HOME:-$HOME/.config}") {
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        return config_home.join(suffix.trim_start_matches('/'));
    }
    for prefix in ["$HOME", "${HOME}", "~"] {
        if let Some(suffix) = value.strip_prefix(prefix) {
            return home.join(suffix.trim_start_matches('/'));
        }
    }
    PathBuf::from(value)
}

#[derive(Clone)]
enum ShellCollision {
    Alias,
    GlobalAlias,
    Function,
    Abbreviation(String),
}

impl ShellCollision {
    fn label(&self) -> &'static str {
        match self {
            Self::Alias => "alias",
            Self::GlobalAlias => "global alias",
            Self::Function => "function",
            Self::Abbreviation(_) => "abbreviation",
        }
    }

    fn remediation(&self, shell: ShellKind) -> String {
        match (shell, self) {
            (ShellKind::Bash, Self::Alias) => "unalias gg".into(),
            (ShellKind::Bash, Self::Function) => "unset -f gg".into(),
            (ShellKind::Zsh, Self::Alias | Self::GlobalAlias) => "unalias 'gg'".into(),
            (ShellKind::Zsh, Self::Function) => "unfunction gg".into(),
            (ShellKind::Fish, Self::Alias | Self::Function) => "functions -e gg".into(),
            (ShellKind::Fish, Self::Abbreviation(name)) => {
                format!("abbr --erase -- {}", fish_escape(name))
            }
            _ => "remove the existing gg definition".into(),
        }
    }
}

fn find_gg_collision(shell: ShellKind, contents: &str) -> Option<ShellCollision> {
    let function = r"(?m)^\s*(?:function\s+gg(?:\s|;|\(|\{|$)|gg\s*\(\s*\))";
    let matches = |pattern| {
        regex::Regex::new(pattern)
            .expect("shell collision regex is valid")
            .is_match(contents)
    };

    match shell {
        ShellKind::Bash if alias_defines_gg(contents) => Some(ShellCollision::Alias),
        ShellKind::Bash if matches(function) => Some(ShellCollision::Function),
        ShellKind::Zsh if global_alias_defines_gg(contents) => Some(ShellCollision::GlobalAlias),
        ShellKind::Zsh if alias_defines_gg(contents) => Some(ShellCollision::Alias),
        ShellKind::Zsh if matches(function) => Some(ShellCollision::Function),
        ShellKind::Fish if matches(r"(?m)^\s*alias(?:\s+--?[[:alnum:]-]+)*\s+gg(?:=|\s|$)") => {
            Some(ShellCollision::Alias)
        }
        ShellKind::Fish if matches(function) => Some(ShellCollision::Function),
        ShellKind::Fish => fish_abbr_defines_gg(contents).map(ShellCollision::Abbreviation),
        _ => None,
    }
}

fn alias_defines_gg(contents: &str) -> bool {
    contents
        .lines()
        .filter_map(alias_operands)
        .any(|(_, operands)| operands.iter().any(|operand| operand.starts_with("gg=")))
}

fn global_alias_defines_gg(contents: &str) -> bool {
    contents
        .lines()
        .filter_map(alias_operands)
        .any(|(global, operands)| {
            global && operands.iter().any(|operand| operand.starts_with("gg="))
        })
}

fn alias_operands(line: &str) -> Option<(bool, Vec<String>)> {
    let words = shell_words(line)?;
    if words.first().map(String::as_str) != Some("alias") {
        return None;
    }

    let mut global = false;
    let mut options = true;
    let mut operands = Vec::new();
    for word in words.into_iter().skip(1) {
        if options && word == "--" {
            options = false;
        } else if options && (word.starts_with('-') || word.starts_with('+')) {
            let flags = word.trim_start_matches(['-', '+']);
            if flags.contains('g') {
                global = true;
            }
            if flags.contains('p')
                || flags.contains('m')
                || flags.contains('s')
                || flags.contains('L')
            {
                return None;
            }
        } else {
            options = false;
            operands.push(word);
        }
    }
    Some((global, operands))
}

fn fish_abbr_defines_gg(contents: &str) -> Option<String> {
    let mut top_level_abbreviations = HashSet::new();
    for line in contents.lines() {
        let Some(words) = shell_words(line) else {
            continue;
        };
        if words.first().map(String::as_str) != Some("abbr") {
            continue;
        }
        let option_words = words
            .iter()
            .skip(1)
            .take_while(|word| word.as_str() != "--");
        if option_words
            .clone()
            .any(|word| matches!(word.as_str(), "--erase" | "-e" | "--query" | "-q"))
        {
            continue;
        }
        if let Some(rename_index) = words
            .iter()
            .skip(1)
            .take_while(|word| word.as_str() != "--")
            .position(|word| word == "--rename")
        {
            let rename_index = rename_index + 1;
            let is_command_scoped = words[..rename_index].iter().any(|word| {
                matches!(word.as_str(), "--command" | "-c")
                    || word.starts_with("--command=")
                    || word.starts_with("-c")
            });
            if !is_command_scoped {
                if let (Some(old), Some(new)) =
                    (words.get(rename_index + 1), words.get(rename_index + 2))
                {
                    if top_level_abbreviations.remove(old) {
                        if new == "gg" {
                            return Some(new.clone());
                        }
                        top_level_abbreviations.insert(new.clone());
                    }
                }
            }
            continue;
        }
        let mut index = 1;
        let mut name = None;
        let mut command_scoped = false;
        let mut regex_matches_gg = false;
        let mut has_regex = false;
        let mut options = true;
        while let Some(word) = words.get(index) {
            if options && word == "--" {
                options = false;
                index += 1;
            } else if options && matches!(word.as_str(), "--position" | "-p" | "--function" | "-f")
            {
                index += 2;
            } else if options && matches!(word.as_str(), "--command" | "-c") {
                command_scoped = true;
                index += 2;
            } else if options && (word.starts_with("--command=") || word.starts_with("-c")) {
                command_scoped = true;
                index += 1;
            } else if options
                && word.starts_with('-')
                && !word.starts_with("--")
                && !word.starts_with("-r")
                && word[1..].contains('r')
            {
                has_regex = true;
                regex_matches_gg = words
                    .get(index + 1)
                    .is_some_and(|pattern| fish_regex_matches_gg(pattern));
                index += 2;
            } else if options && matches!(word.as_str(), "--regex" | "-r") {
                has_regex = true;
                regex_matches_gg = words
                    .get(index + 1)
                    .is_some_and(|pattern| fish_regex_matches_gg(pattern));
                index += 2;
            } else if options && (word.starts_with("--regex=") || word.starts_with("-r")) {
                has_regex = true;
                let pattern = word
                    .strip_prefix("--regex=")
                    .or_else(|| word.strip_prefix("-r"))
                    .unwrap_or_default();
                regex_matches_gg = fish_regex_matches_gg(pattern);
                index += 1;
            } else if options && word.starts_with('-') {
                index += 1;
            } else {
                name.get_or_insert_with(|| word.clone());
                index += 1;
            }
        }
        let Some(name) = name else {
            continue;
        };
        let is_top_level = !command_scoped;
        if is_top_level && !has_regex {
            top_level_abbreviations.insert(name.clone());
        }
        if is_top_level
            && if has_regex {
                regex_matches_gg
            } else {
                name == "gg"
            }
        {
            return Some(name);
        }
    }
    None
}

fn fish_regex_matches_gg(pattern: &str) -> bool {
    pcre2::bytes::Regex::new(&format!("\\A(?:{pattern})\\z"))
        .is_ok_and(|regex| regex.is_match(b"gg").unwrap_or(false))
}

fn fish_escape(name: &str) -> String {
    format!("'{}'", name.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn shell_words(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut word_started = false;
    let mut quote = None;
    let mut escaped = false;

    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if escaped {
            word.push(ch);
            word_started = true;
            escaped = false;
        } else if ch == '\\' && quote != Some('\'') {
            if quote == Some('"') && !matches!(chars.peek(), Some('\\' | '"' | '$' | '\n')) {
                word.push(ch);
                word_started = true;
            } else {
                escaped = true;
            }
        } else if matches!(ch, '\'' | '"') {
            word_started = true;
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            } else {
                word.push(ch);
            }
        } else if ch == '#' && quote.is_none() && !word_started {
            break;
        } else if ch.is_whitespace() && quote.is_none() {
            if word_started {
                words.push(std::mem::take(&mut word));
                word_started = false;
            }
        } else {
            word.push(ch);
            word_started = true;
        }
    }
    if quote.is_some() || escaped {
        return None;
    }
    if word_started {
        words.push(word);
    }
    Some(words)
}

fn shell_name(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Bash => "bash",
        ShellKind::Zsh => "zsh",
        ShellKind::Fish => "fish",
    }
}

fn setup_snippet(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Bash => {
            r#"# Add this to ~/.bashrc, or evaluate it once with:
# eval "$(command 'gg' doctor --shell bash --setup)"
# This setup does not overwrite an existing gg alias or function.
if alias gg >/dev/null 2>&1; then
  printf '%s\n' 'git-gist: gg is already an alias; run unalias gg, then remove or rename it before enabling git-gist.' >&2
elif declare -F gg >/dev/null 2>&1; then
  printf '%s\n' 'git-gist: gg is already a function; run unset -f gg, then remove or rename it before enabling git-gist.' >&2
else
  eval "$(command 'gg' completions bash)"
fi"#
        }
        ShellKind::Zsh => {
            r#"# Add this to your zsh startup file, or evaluate it once with:
# eval "$(command 'gg' doctor --shell zsh --setup)"
# This setup does not overwrite an existing gg alias or function.
if (( $+aliases[gg] )); then
  print -u2 "git-gist: gg is already an alias; run unalias 'gg', then remove or rename it before enabling git-gist."
elif (( $+galiases[gg] )); then
  print -u2 "git-gist: gg is already a global alias; run unalias 'gg', then remove or rename it before enabling git-gist."
elif (( $+functions[gg] )); then
  print -u2 'git-gist: gg is already a function; run unfunction gg, then remove or rename it before enabling git-gist.'
else
  eval "$(command 'gg' completions zsh)"
fi"#
        }
        ShellKind::Fish => {
            r#"# Add this to your fish startup file, or evaluate it once with:
# command 'gg' doctor --shell fish --setup | source
# This setup does not overwrite an existing gg alias, function, or abbreviation.
function __gg_has_top_level_abbr
    for definition in (abbr --show)
        printf '%s\n' "$definition" | read --tokenize --list words
        set -l separator (contains --index -- -- $words)
        set -q separator[1]; or continue
        set -l name "$words[(math "$separator + 1")]"
        set -l regex
        set -l commands
        set -l i 1
        while test $i -lt $separator
            set -l next (math "$i + 1")
            switch $words[$i]
                case --regex
                    set regex "$words[$next]"
                    set i (math "$i + 2")
                    continue
                case --command
                    set --append commands "$words[$next]"
                    set i (math "$i + 2")
                    continue
            end
            set i (math "$i + 1")
        end
        if set -q commands[1]; and not contains -- '' $commands
            continue
        end
        if set -q regex[1]
            string match --quiet --regex -- "^(?:$regex)\$" gg 2>/dev/null; and return 0
        else if test "$name" = gg
            return 0
        end
    end
    return 1
end

if functions -q gg
    echo 'git-gist: gg is already a function; run functions -e gg, then remove or rename it before enabling git-gist.' >&2
    functions -e __gg_has_top_level_abbr
    return 1
else if __gg_has_top_level_abbr
    echo 'git-gist: a Fish abbreviation expands top-level gg; remove or rename it before enabling git-gist.' >&2
    functions -e __gg_has_top_level_abbr
    return 1
else
    if not complete -c gg | string match -q '*'
        command 'gg' completions fish | source
    end
end
functions -e __gg_has_top_level_abbr"#
        }
    }
}

fn emit_findings(
    findings: &[DoctorFinding],
    repos_checked: usize,
    out: &mut OutputCtx,
) -> Result<()> {
    if out.is_json() {
        out.write_json(&findings.to_vec())?;
        return Ok(());
    }

    for f in findings {
        let prefix = match f.level.as_str() {
            "error" => "error",
            "warn" => "warn",
            _ => "info",
        };
        if let Some(repo) = &f.repo {
            writeln!(out.stdout(), "[{prefix}] {repo}: {}", f.message)?;
        } else {
            writeln!(out.stdout(), "[{prefix}] {}", f.message)?;
        }
    }
    if repos_checked > 0 {
        out.info(&format!(
            "checked {} repositories, {} findings",
            repos_checked,
            findings.len()
        ))?;
    } else {
        out.info(&format!("{} findings", findings.len()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        alias_defines_gg, expand_home_prefix, fish_abbr_defines_gg, global_alias_defines_gg,
        shell_words, ShellCollision,
    };
    use crate::cli::ShellKind;

    #[test]
    fn parses_quoted_alias_operands_and_ignores_non_defining_options() {
        assert!(alias_defines_gg("alias ll='ls -l' 'gg=git gui'"));
        assert!(!alias_defines_gg("alias foo='echo gg=bar'"));
        assert!(!alias_defines_gg("alias -p gg='git gui'"));
        assert!(!alias_defines_gg("alias -s gg='git gui'"));
        assert!(global_alias_defines_gg("alias -g -- 'gg=git gui'"));
    }

    #[test]
    fn distinguishes_fish_abbreviation_operations() {
        assert!(fish_abbr_defines_gg("abbr --add --function isatty gg 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr --add --set-cursor gg 'git gui %'").is_some());
        assert!(fish_abbr_defines_gg("abbr --add --command git gg checkout").is_none());
        assert!(fish_abbr_defines_gg("abbr -a -c git gg checkout").is_none());
        assert!(fish_abbr_defines_gg("abbr -a -r '^gg$' -- gg 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr --add --regex '^gg$' gitgui 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr --add --regex 'g|gg' gitgui 'git gui'").is_some());
        assert_eq!(
            fish_abbr_defines_gg("abbr --add gitgui --regex '^gg$' 'git gui'"),
            Some("gitgui".into())
        );
        assert!(fish_abbr_defines_gg("abbr --add gg --command git checkout").is_none());
        assert!(fish_abbr_defines_gg("abbr --rename old gg").is_none());
        assert!(fish_abbr_defines_gg("abbr --add old 'git gui'\nabbr --rename old gg").is_some());
        assert!(fish_abbr_defines_gg("abbr --show\nabbr --add gg 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr --rename --command git old gg").is_none());
        assert!(fish_abbr_defines_gg("abbr --rename -cgit old gg").is_none());
        assert!(fish_abbr_defines_gg("abbr old --rename gg").is_none());
        assert!(fish_abbr_defines_gg("abbr --rename old --command git gg").is_none());
        assert!(fish_abbr_defines_gg("abbr --add --set-cursor foo gg").is_none());
        assert!(fish_abbr_defines_gg("not-abbr gg").is_none());
        assert!(fish_abbr_defines_gg("abbr --add -- gg git -e").is_some());
        assert!(fish_abbr_defines_gg("abbr --add -cgit gg checkout").is_none());
        assert!(fish_abbr_defines_gg("abbr --add --command=git gg checkout").is_none());
        assert!(fish_abbr_defines_gg("abbr --add --command= gg checkout").is_none());
        assert!(fish_abbr_defines_gg("abbr --add --regex='^gg$' gitgui 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr --add -r'^gg$' gitgui 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr -ar '^gg$' gitgui 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr --add --regex '(?=gg$)gg' gitgui 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr --add --regex 'g\\Kg' gitgui 'git gui'").is_some());
        assert!(fish_abbr_defines_gg("abbr --add --regex '^foo$' gg 'git gui'").is_none());
        assert!(fish_abbr_defines_gg("abbr gg --erase").is_none());
        assert!(fish_abbr_defines_gg("abbr gg --query").is_none());
        assert!(fish_abbr_defines_gg("abbr --erase gg").is_none());
        assert!(fish_abbr_defines_gg("abbr --query gg").is_none());
    }

    #[test]
    fn parses_shell_words_and_renders_each_collision_remediation() {
        assert_eq!(
            shell_words("alias gg='git gui' # only a comment"),
            Some(vec!["alias".into(), "gg=git gui".into()])
        );
        assert_eq!(
            shell_words("alias gg=git\\ gui"),
            Some(vec!["alias".into(), "gg=git gui".into()])
        );
        assert_eq!(shell_words("alias gg='unterminated"), None);
        assert_eq!(
            shell_words("abbr --regex \"^g\\\\w$\" name expansion"),
            Some(vec![
                "abbr".into(),
                "--regex".into(),
                "^g\\w$".into(),
                "name".into(),
                "expansion".into(),
            ])
        );

        assert_eq!(ShellCollision::Alias.label(), "alias");
        assert_eq!(ShellCollision::Function.label(), "function");
        assert_eq!(
            ShellCollision::Abbreviation("gg".into()).label(),
            "abbreviation"
        );
        assert_eq!(
            ShellCollision::Function.remediation(ShellKind::Bash),
            "unset -f gg"
        );
        assert_eq!(
            ShellCollision::Function.remediation(ShellKind::Zsh),
            "unfunction gg"
        );
        assert_eq!(
            ShellCollision::Function.remediation(ShellKind::Fish),
            "functions -e gg"
        );
        assert_eq!(
            ShellCollision::Abbreviation("gg".into()).remediation(ShellKind::Fish),
            "abbr --erase -- 'gg'"
        );
    }

    #[test]
    fn expands_zdotdir_home_prefixes() {
        let home = std::path::Path::new("/home/example");
        assert_eq!(
            expand_home_prefix("$HOME/.config/zsh", home),
            home.join(".config/zsh")
        );
        assert_eq!(
            expand_home_prefix("${HOME}/.config/zsh", home),
            home.join(".config/zsh")
        );
        assert_eq!(
            expand_home_prefix("~/.config/zsh", home),
            home.join(".config/zsh")
        );
    }
}
