//! Unit tests for output helpers and color resolution.

use git_gist::cli::{ColorChoice, OutputFormat};
use git_gist::output::{OutputCtx, Theme};
use git_gist::resolve_color;
use serial_test::serial;

#[test]
fn theme_parse() {
    assert_eq!(Theme::parse("mono"), Theme::Mono);
    assert_eq!(Theme::parse("VIVID"), Theme::Vivid);
    assert_eq!(Theme::parse("default"), Theme::Default);
    assert_eq!(Theme::parse("other"), Theme::Default);
}

#[test]
fn resolve_color_always_never() {
    assert!(resolve_color(ColorChoice::Always));
    assert!(!resolve_color(ColorChoice::Never));
}

#[test]
#[serial]
fn resolve_color_respects_no_color() {
    std::env::set_var("NO_COLOR", "1");
    assert!(!resolve_color(ColorChoice::Auto));
    std::env::remove_var("NO_COLOR");
}

#[test]
fn output_ctx_json_flags() {
    let out = OutputCtx::new(false, OutputFormat::Json, false, 0);
    assert!(out.is_json());
    let out = OutputCtx::new(false, OutputFormat::Ndjson, false, 0).with_theme(Theme::Mono);
    assert!(out.is_json());
    let out = OutputCtx::new(false, OutputFormat::Human, true, 1);
    assert!(!out.is_json());
}

#[test]
fn clap_command_builds() {
    let cmd = git_gist::clap_command();
    assert_eq!(cmd.get_name(), "gg");
}
