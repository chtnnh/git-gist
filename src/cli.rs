//! CLI definition for `gg`.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Run git commands across all child git repositories.
#[derive(Debug, Parser)]
#[command(
    name = "gg",
    version,
    about = "Run git across all child repositories",
    long_about = "git-gist (gg) discovers git repositories under a root and runs git \
                  commands (or built-in insights) across them in parallel.",
    after_help = "Passthrough: `gg status` runs `git status` in each selected repo.\n\
                  Escape hatch: `gg git -- status`.",
    allow_external_subcommands = true,
    subcommand_negates_reqs = true
)]
pub struct Cli {
    /// Search root (defaults to cwd or config root)
    #[arg(long, global = true, value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Include only these aliases, paths, or groups (repeatable)
    #[arg(long = "in", short = 'i', global = true, value_name = "TARGET", action = clap::ArgAction::Append)]
    pub include: Vec<String>,

    /// Exclude these aliases, paths, or groups (repeatable)
    #[arg(long, short = 'x', global = true, value_name = "TARGET", action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,

    /// Select a named group (repeatable)
    #[arg(long, short = 'g', global = true, value_name = "GROUP", action = clap::ArgAction::Append)]
    pub group: Vec<String>,

    /// Max directory depth when scanning (0 = unlimited)
    #[arg(long, global = true, value_name = "N")]
    pub depth: Option<usize>,

    /// Parallel jobs (default: number of CPUs)
    #[arg(short = 'j', long, global = true, value_name = "N")]
    pub jobs: Option<usize>,

    /// Stop on first failure
    #[arg(long, global = true)]
    pub fail_fast: bool,

    /// Print what would run without executing
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Quiet output
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Verbose output
    #[arg(short = 'v', long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Show per-repo timing
    #[arg(long, global = true)]
    pub timing: bool,

    /// Color output
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

    /// Output format
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,

    /// Only repos with a dirty working tree
    #[arg(long, global = true)]
    pub only_dirty: bool,

    /// Only clean repos
    #[arg(long, global = true)]
    pub only_clean: bool,

    /// Only repos ahead of upstream
    #[arg(long, global = true)]
    pub only_ahead: bool,

    /// Only repos behind upstream
    #[arg(long, global = true)]
    pub only_behind: bool,

    /// Only repos with stashes
    #[arg(long, global = true)]
    pub only_stashed: bool,

    /// Only detached HEAD repos
    #[arg(long, global = true)]
    pub only_detached: bool,

    /// Include git submodules in discovery
    #[arg(long, global = true)]
    pub include_submodules: bool,

    /// Refresh discovery cache
    #[arg(long, global = true)]
    pub refresh: bool,

    /// Filter by config label/tag (repeatable)
    #[arg(long, global = true, value_name = "TAG", action = clap::ArgAction::Append)]
    pub tag: Vec<String>,

    /// Theme name (default, mono, vivid)
    #[arg(long, global = true, value_name = "NAME")]
    pub theme: Option<String>,

    /// Built-in command, or external git passthrough
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Ndjson,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Multi-repo dashboard (path, branch, dirty, ahead/behind)
    #[command(visible_alias = "ov")]
    Overview,

    /// List discovered/selected repositories
    #[command(visible_alias = "ls")]
    List {
        /// Bypass discovery cache
        #[arg(long)]
        refresh: bool,
    },

    /// Show details for selected repos (or one path)
    Info {
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },

    /// Top-N commits across selection
    Commits {
        #[arg(short = 'n', long, default_value_t = 5)]
        number: usize,
    },

    /// Worktree status across selection
    Worktrees,

    /// Health checks for selection / environment
    Doctor,

    /// Run an arbitrary shell command in each repo
    Each {
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Get or set configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Manage path aliases
    Alias {
        #[command(subcommand)]
        action: AliasAction,
    },

    /// Manage groups of aliases/paths
    Group {
        #[command(subcommand)]
        action: GroupAction,
    },

    /// Scaffold a new git repo from a profile
    Init {
        #[arg(long, value_name = "NAME")]
        profile: Option<String>,
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },

    /// Alias for `init`
    Scaffold {
        #[arg(long, value_name = "NAME")]
        profile: Option<String>,
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },

    /// Install or manage hook packs
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },

    /// Manage reusable remote catalog
    Remotes {
        #[command(subcommand)]
        action: RemotesAction,
    },

    /// Fetch (and optionally pull) across selection with summary
    Sync {
        /// Also pull when fast-forward is possible
        #[arg(long)]
        pull: bool,
    },

    /// List repos with no commits newer than N days
    Stale {
        #[arg(long, default_value_t = 90)]
        days: u64,
    },

    /// Enroll new repos from [[auto_enroll]] rules into aliases/groups/tags
    Update,

    /// Explicit git passthrough (escape hatch for name collisions)
    Git {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        args: Vec<String>,
    },

    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Generate man page
    Man {
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Print version
    Version,

    /// Update gg from GitHub releases (best-effort)
    #[command(name = "self-update")]
    SelfUpdate,

    /// External git passthrough: `gg status`, `gg pull --rebase`, …
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print effective config as TOML
    Show,
    /// Print config file path
    Path {
        #[arg(long)]
        local: bool,
    },
    /// Set a simple key (dot path) in global config
    Set { key: String, value: String },
    /// Get a simple key
    Get { key: String },
}

#[derive(Debug, Subcommand)]
pub enum AliasAction {
    List,
    Add { name: String, path: PathBuf },
    Remove { name: String },
}

#[derive(Debug, Subcommand)]
pub enum GroupAction {
    List,
    Add {
        name: String,
        #[arg(required = true, num_args = 1..)]
        members: Vec<String>,
    },
    Remove {
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum HooksAction {
    /// List available hook packs
    List,
    /// Install a pack into selected repos
    Install { pack: String },
}

#[derive(Debug, Subcommand)]
pub enum RemotesAction {
    List,
    Add {
        name: String,
        url: String,
    },
    Remove {
        name: String,
    },
    /// Add a catalog remote to selected repos
    #[command(name = "add-to")]
    AddTo {
        name: String,
        /// Remote name inside each repo (defaults to catalog name)
        #[arg(long)]
        as_name: Option<String>,
    },
}
