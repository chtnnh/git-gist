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
fn semantic_cell_styles() {
    use git_gist::output::CellStyle;
    assert_eq!(OutputCtx::tree_style(false), CellStyle::Good);
    assert_eq!(OutputCtx::tree_style(true), CellStyle::Bad);
    assert_eq!(OutputCtx::age_style(None), CellStyle::Dim);
    assert_eq!(OutputCtx::age_style(Some(60)), CellStyle::Good);
    assert_eq!(OutputCtx::age_style(Some(30 * 86400)), CellStyle::Warn);
    assert_eq!(OutputCtx::age_style(Some(90 * 86400)), CellStyle::Bad);
    assert_eq!(OutputCtx::ahead_behind_style(0, 0), CellStyle::Dim);
    assert_eq!(OutputCtx::ahead_behind_style(1, 0), CellStyle::Warn);
    assert_eq!(OutputCtx::ahead_behind_style(1, 1), CellStyle::Bad);
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
fn styled_table_emits_ansi_when_color_on() {
    use comfy_table::{Cell, Color, Table};

    let mut table = Table::new();
    table.force_no_tty().enforce_styling();
    table.add_row(vec![Cell::new("clean").fg(Color::Green)]);
    let s = table.to_string();
    assert!(
        s.contains('\u{1b}'),
        "expected ANSI escapes in styled table, got {s:?}"
    );
}
