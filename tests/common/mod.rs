//! Shared helpers for integration tests.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{tempdir, TempDir};

pub struct Fixture {
    pub root: TempDir,
    pub home: TempDir,
    pub repos: Vec<PathBuf>,
}

impl Fixture {
    pub fn new() -> Self {
        let root = tempdir().unwrap();
        let home = tempdir().unwrap();
        Self {
            root,
            home,
            repos: Vec::new(),
        }
    }

    pub fn with_repos(names: &[&str]) -> Self {
        let mut f = Self::new();
        for name in names {
            f.add_repo(name, true);
        }
        f
    }

    pub fn add_repo(&mut self, name: &str, with_commit: bool) -> PathBuf {
        let path = self.root.path().join(name);
        fs::create_dir_all(&path).unwrap();
        git(&path, &["init", "-b", "main"]);
        if with_commit {
            fs::write(path.join("README"), format!("{name}\n")).unwrap();
            git(&path, &["add", "README"]);
            git(&path, &["commit", "-m", &format!("init {name}")]);
        }
        self.repos.push(path.clone());
        path
    }

    pub fn gg(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("gg").unwrap();
        cmd.env("XDG_CONFIG_HOME", self.home.path().join("config"));
        cmd.env("XDG_CACHE_HOME", self.home.path().join("cache"));
        // Windows `dirs::home_dir` ignores HOME; production code prefers these
        // env vars so fixtures can redirect `~/.git-gist/` on all platforms.
        cmd.env("GIT_GIST_HOME", self.home.path());
        cmd.env("HOME", self.home.path());
        cmd.env("USERPROFILE", self.home.path());
        cmd.env("NO_COLOR", "1");
        cmd.current_dir(self.root.path());
        cmd
    }

    pub fn config_dir(&self) -> PathBuf {
        self.home.path().join(".git-gist")
    }

    pub fn write_global_config(&self, body: &str) {
        let dir = self.config_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.toml"), body).unwrap();
    }

    /// Escape a path for embedding in a double-quoted TOML string (Windows-safe).
    pub fn toml_path(path: &Path) -> String {
        path.display().to_string().replace('\\', "\\\\")
    }

    pub fn write_local_config(&self, body: &str) {
        fs::write(self.root.path().join(".gg.toml"), body).unwrap();
    }

    pub fn write_ggignore(&self, body: &str) {
        fs::write(self.root.path().join(".ggignore"), body).unwrap();
    }
}

pub fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?} failed in {}", cwd.display());
}
