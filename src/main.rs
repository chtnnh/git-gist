//! git-gist (`gg`) — run git across all child repositories.

use git_gist::run;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
