//! Ratatui config manager (hub + scoped area views).

use crate::cli::{Cli, OutputFormat};
use crate::config::Config;
use crate::config_ops;
use crate::output::OutputCtx;
use anyhow::{bail, Result};
use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Aliases,
    Groups,
    Tags,
    Remotes,
    Enroll,
    Settings,
}

fn require_tty(cli: &Cli) -> Result<()> {
    if matches!(cli.format, OutputFormat::Json | OutputFormat::Ndjson) {
        bail!("TUI is incompatible with --format json/ndjson");
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!(
            "TUI requires a TTY — try `gg config wizard` or scriptable commands \
             (`gg alias add`, …)"
        );
    }
    Ok(())
}

#[cfg(not(feature = "tui"))]
mod stubs {
    use super::*;
    fn disabled() -> Result<()> {
        bail!("tui feature disabled — try `gg config wizard` or rebuild with `--features tui`")
    }
    pub fn run_hub(_: &Cli, _: &Config, _: &mut OutputCtx) -> Result<()> {
        disabled()
    }
    pub fn run_focused(_: &Cli, _: &Config, _: &mut OutputCtx, _: Area) -> Result<()> {
        disabled()
    }
    pub fn render_preview(_: &Config, _: Option<Area>, _: u16, _: u16) -> Result<String> {
        disabled()
    }
}

#[cfg(not(feature = "tui"))]
pub use stubs::*;

#[cfg(feature = "tui")]
mod impl_tui {
    use super::*;
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use crossterm::execute;
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::backend::{CrosstermBackend, TestBackend};
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs};
    use ratatui::Terminal;
    use std::io;

    const AREAS: [Area; 6] = [
        Area::Aliases,
        Area::Groups,
        Area::Tags,
        Area::Remotes,
        Area::Enroll,
        Area::Settings,
    ];

    struct App {
        draft: Config,
        dirty: bool,
        area_idx: usize,
        scoped: bool,
        list_state: ListState,
        status: String,
        quit: bool,
    }

    impl App {
        fn new(cfg: Config, focus: Option<Area>) -> Self {
            let area_idx = focus
                .and_then(|a| AREAS.iter().position(|x| *x == a))
                .unwrap_or(0);
            let mut list_state = ListState::default();
            list_state.select(Some(0));
            Self {
                draft: cfg,
                dirty: false,
                area_idx,
                scoped: focus.is_some(),
                list_state,
                status: "j/k navigate · Tab switch · d delete · p prune · s save · q quit".into(),
                quit: false,
            }
        }

        fn area(&self) -> Area {
            AREAS[self.area_idx]
        }

        fn items(&self) -> Vec<String> {
            match self.area() {
                Area::Aliases => self
                    .draft
                    .aliases
                    .iter()
                    .map(|(n, p)| {
                        let stale = if config_ops::alias_is_stale(p) {
                            " [stale]"
                        } else {
                            ""
                        };
                        format!("{n} → {}{stale}", p.display())
                    })
                    .collect(),
                Area::Groups => self
                    .draft
                    .groups
                    .iter()
                    .map(|(n, m)| format!("{n} ({})", m.len()))
                    .collect(),
                Area::Tags => self
                    .draft
                    .tags
                    .iter()
                    .map(|(n, m)| format!("{n} ({})", m.len()))
                    .collect(),
                Area::Remotes => self
                    .draft
                    .remotes
                    .iter()
                    .map(|(n, u)| format!("{n} → {u}"))
                    .collect(),
                Area::Enroll => self
                    .draft
                    .auto_enroll
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        let prefix = r
                            .path_prefix
                            .as_deref()
                            .map(|p| format!(" [{p}]"))
                            .unwrap_or_default();
                        format!("{i}: {}{prefix}", r.path.display())
                    })
                    .collect(),
                Area::Settings => vec![
                    format!("depth = {}", self.draft.depth),
                    format!("jobs = {:?}", self.draft.jobs),
                    format!("theme = {:?}", self.draft.theme),
                    format!("show_path = {}", self.draft.show_path),
                    format!("include_submodules = {}", self.draft.include_submodules),
                    format!("root = {:?}", self.draft.root),
                ],
            }
        }

        fn selected_name(&self) -> Option<String> {
            let idx = self.list_state.selected()?;
            match self.area() {
                Area::Aliases => self.draft.aliases.keys().nth(idx).cloned(),
                Area::Groups => self.draft.groups.keys().nth(idx).cloned(),
                Area::Tags => self.draft.tags.keys().nth(idx).cloned(),
                Area::Remotes => self.draft.remotes.keys().nth(idx).cloned(),
                Area::Enroll => Some(idx.to_string()),
                Area::Settings => None,
            }
        }

        fn delete_selected(&mut self) {
            let Some(name) = self.selected_name() else {
                return;
            };
            let result = match self.area() {
                Area::Aliases => config_ops::remove_alias(&mut self.draft, &name),
                Area::Groups => config_ops::remove_group(&mut self.draft, &name),
                Area::Tags => config_ops::remove_tag(&mut self.draft, &name),
                Area::Remotes => config_ops::remove_remote(&mut self.draft, &name),
                Area::Enroll => {
                    if let Ok(i) = name.parse::<usize>() {
                        config_ops::remove_auto_enroll_rule(&mut self.draft, i).map(|_| ())
                    } else {
                        Ok(())
                    }
                }
                Area::Settings => Ok(()),
            };
            match result {
                Ok(()) => {
                    self.dirty = true;
                    self.status = format!("deleted {name}");
                    let len = self.items().len();
                    if len == 0 {
                        self.list_state.select(None);
                    } else {
                        let i = self.list_state.selected().unwrap_or(0).min(len - 1);
                        self.list_state.select(Some(i));
                    }
                }
                Err(e) => self.status = format!("error: {e}"),
            }
        }

        fn prune_stale(&mut self) {
            let removed = config_ops::prune_stale_aliases(&mut self.draft);
            if removed.is_empty() {
                self.status = "no stale aliases".into();
            } else {
                self.dirty = true;
                self.status = format!("pruned {} stale alias(es)", removed.len());
            }
        }

        fn save(&mut self) -> Result<()> {
            let path = config_ops::save(&self.draft)?;
            self.dirty = false;
            self.status = format!("saved {}", path.display());
            Ok(())
        }
    }

    pub fn run_hub(cli: &Cli, cfg: &Config, out: &mut OutputCtx) -> Result<()> {
        require_tty(cli)?;
        run_app(App::new(cfg.clone(), None), out)
    }

    pub fn run_focused(cli: &Cli, cfg: &Config, out: &mut OutputCtx, area: Area) -> Result<()> {
        require_tty(cli)?;
        run_app(App::new(cfg.clone(), Some(area)), out)
    }

    /// Render one TUI frame to a plain-text grid (for docs / tests; no TTY required).
    pub fn render_preview(
        cfg: &Config,
        focus: Option<Area>,
        width: u16,
        height: u16,
    ) -> Result<String> {
        let mut app = App::new(cfg.clone(), focus);
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend)?;
        terminal.draw(|f| draw(f, &mut app))?;
        let buffer = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..height {
            for x in 0..width {
                let cell = buffer
                    .cell((x, y))
                    .map(|c| c.symbol().to_string())
                    .unwrap_or_else(|| " ".into());
                out.push_str(&cell);
            }
            out.push('\n');
        }
        Ok(out)
    }

    fn run_app(mut app: App, out: &mut OutputCtx) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = loop {
            terminal.draw(|f| draw(f, &mut app))?;
            if event::poll(std::time::Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if app.dirty {
                                app.status =
                                    "unsaved changes — press s to save, Q to quit anyway".into();
                                // second q with shift
                            } else {
                                app.quit = true;
                            }
                        }
                        KeyCode::Char('Q') => app.quit = true,
                        KeyCode::Char('s') => {
                            if let Err(e) = app.save() {
                                app.status = format!("save failed: {e}");
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Delete => app.delete_selected(),
                        KeyCode::Char('p') => app.prune_stale(),
                        KeyCode::Tab | KeyCode::Right if !app.scoped => {
                            app.area_idx = (app.area_idx + 1) % AREAS.len();
                            app.list_state.select(Some(0));
                        }
                        KeyCode::BackTab | KeyCode::Left if !app.scoped => {
                            app.area_idx = (app.area_idx + AREAS.len() - 1) % AREAS.len();
                            app.list_state.select(Some(0));
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let len = app.items().len();
                            if len > 0 {
                                let i = app.list_state.selected().unwrap_or(0);
                                app.list_state.select(Some((i + 1) % len));
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            let len = app.items().len();
                            if len > 0 {
                                let i = app.list_state.selected().unwrap_or(0);
                                app.list_state.select(Some((i + len - 1) % len));
                            }
                        }
                        _ => {}
                    }
                }
            }
            if app.quit {
                break Ok(());
            }
        };

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        if !app.dirty {
            out.info(&app.status)?;
        }
        result
    }

    fn draw(f: &mut ratatui::Frame, app: &mut App) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(2),
            ])
            .split(f.area());

        let titles: Vec<Line> = AREAS
            .iter()
            .map(|a| {
                let label = match a {
                    Area::Aliases => "Aliases",
                    Area::Groups => "Groups",
                    Area::Tags => "Tags",
                    Area::Remotes => "Remotes",
                    Area::Enroll => "Enroll",
                    Area::Settings => "Settings",
                };
                Line::from(label)
            })
            .collect();
        let tabs = Tabs::new(titles).select(app.area_idx).block(
            Block::default().borders(Borders::ALL).title(if app.dirty {
                "gg config ui *"
            } else {
                "gg config ui"
            }),
        );
        f.render_widget(tabs, chunks[0]);

        let items: Vec<ListItem> = app.items().into_iter().map(ListItem::new).collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(area_title(app.area())),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut app.list_state);

        let status = Paragraph::new(Span::raw(app.status.clone()))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, chunks[2]);
    }

    fn area_title(area: Area) -> &'static str {
        match area {
            Area::Aliases => "Aliases",
            Area::Groups => "Groups",
            Area::Tags => "Tags",
            Area::Remotes => "Remotes",
            Area::Enroll => "Auto-enroll",
            Area::Settings => "Settings",
        }
    }
}

#[cfg(feature = "tui")]
pub use impl_tui::*;
