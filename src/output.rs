//! Colored / JSON output helpers.

use crate::cli::OutputFormat;
use anyhow::Result;
use comfy_table::{
    presets::UTF8_FULL, Attribute, Cell, Color as TableColor, ContentArrangement, Table,
};
use owo_colors::OwoColorize;
use std::io::{self, Stderr, Stdout, Write};

pub struct OutputCtx {
    pub color: bool,
    pub format: OutputFormat,
    pub quiet: bool,
    #[allow(dead_code)]
    pub verbose: u8,
    pub theme: Theme,
    stdout: Stdout,
    stderr: Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Default,
    Mono,
    Vivid,
}

impl Theme {
    pub fn parse(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "mono" => Self::Mono,
            "vivid" => Self::Vivid,
            _ => Self::Default,
        }
    }
}

/// Semantic cell styling for human tables (ignored when color is off or theme is mono).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellStyle {
    #[default]
    Plain,
    /// Success / clean / in sync
    Good,
    /// Warning / mild drift / aging
    Warn,
    /// Error / dirty / stale / conflict
    Bad,
    /// De-emphasized placeholders
    Dim,
}

impl OutputCtx {
    pub fn new(color: bool, format: OutputFormat, quiet: bool, verbose: u8) -> Self {
        Self {
            color,
            format,
            quiet,
            verbose,
            theme: Theme::Default,
            stdout: io::stdout(),
            stderr: io::stderr(),
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn stdout(&mut self) -> &mut Stdout {
        &mut self.stdout
    }

    pub fn stderr(&mut self) -> &mut Stderr {
        &mut self.stderr
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json | OutputFormat::Ndjson)
    }

    pub fn repo_header(&mut self, name: &str, path: &str) -> Result<()> {
        if self.quiet || self.is_json() {
            return Ok(());
        }
        let line = match self.theme {
            Theme::Mono => format!("=== {name} ({path}) ==="),
            Theme::Vivid | Theme::Default => format!("━━ {name} ━━ {path}"),
        };
        if self.color {
            match self.theme {
                Theme::Mono => writeln!(self.stdout, "{}", line.bold())?,
                Theme::Default => writeln!(self.stdout, "{}", line.cyan().bold())?,
                Theme::Vivid => writeln!(self.stdout, "{}", line.magenta().bold())?,
            }
        } else {
            writeln!(self.stdout, "{line}")?;
        }
        Ok(())
    }

    pub fn warn(&mut self, msg: &str) -> Result<()> {
        if self.color {
            writeln!(self.stderr, "{} {msg}", "warn:".yellow().bold())?;
        } else {
            writeln!(self.stderr, "warn: {msg}")?;
        }
        Ok(())
    }

    pub fn info(&mut self, msg: &str) -> Result<()> {
        if self.quiet {
            return Ok(());
        }
        if self.color {
            writeln!(self.stdout, "{}", msg.dimmed())?;
        } else {
            writeln!(self.stdout, "{msg}")?;
        }
        Ok(())
    }

    pub fn success(&mut self, msg: &str) -> Result<()> {
        if self.quiet {
            return Ok(());
        }
        if self.color {
            writeln!(self.stdout, "{}", msg.green())?;
        } else {
            writeln!(self.stdout, "{msg}")?;
        }
        Ok(())
    }

    pub fn write_json<T: serde::Serialize>(&mut self, value: &T) -> Result<()> {
        match self.format {
            OutputFormat::Ndjson => {
                let v = serde_json::to_value(value)?;
                if let Some(arr) = v.as_array() {
                    for item in arr {
                        writeln!(self.stdout, "{}", serde_json::to_string(item)?)?;
                    }
                } else {
                    writeln!(self.stdout, "{}", serde_json::to_string(&v)?)?;
                }
            }
            _ => {
                writeln!(self.stdout, "{}", serde_json::to_string_pretty(value)?)?;
            }
        }
        Ok(())
    }

    pub fn cell(&self, text: impl Into<String>, style: CellStyle) -> Cell {
        let cell = Cell::new(text.into());
        if !self.color || matches!(self.theme, Theme::Mono) {
            return cell;
        }
        match style {
            CellStyle::Plain => cell,
            CellStyle::Good => cell.fg(TableColor::Green),
            CellStyle::Warn => cell.fg(TableColor::Yellow),
            CellStyle::Bad => cell.fg(TableColor::Red),
            CellStyle::Dim => cell.fg(TableColor::DarkGrey),
        }
    }

    /// Color last-commit age: ≥90d red, ≥30d yellow, else green (when known).
    pub fn age_style(age_secs: Option<u64>) -> CellStyle {
        match age_secs {
            None => CellStyle::Dim,
            Some(secs) if secs >= 90 * 86400 => CellStyle::Bad,
            Some(secs) if secs >= 30 * 86400 => CellStyle::Warn,
            Some(_) => CellStyle::Good,
        }
    }

    pub fn tree_style(dirty: bool) -> CellStyle {
        if dirty {
            CellStyle::Bad
        } else {
            CellStyle::Good
        }
    }

    pub fn ahead_behind_style(ahead: u32, behind: u32) -> CellStyle {
        if behind > 0 && ahead > 0 {
            CellStyle::Bad
        } else if behind > 0 || ahead > 0 {
            CellStyle::Warn
        } else {
            CellStyle::Dim
        }
    }

    pub fn print_table(&mut self, headers: &[&str], rows: Vec<Vec<String>>) -> Result<()> {
        let styled: Vec<Vec<Cell>> = rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|c| self.cell(c, CellStyle::Plain))
                    .collect()
            })
            .collect();
        self.print_table_cells(headers, styled)
    }

    pub fn print_table_cells(&mut self, headers: &[&str], rows: Vec<Vec<Cell>>) -> Result<()> {
        if self.is_json() {
            return Ok(());
        }
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);
        if self.color {
            // Honor --color always / config even when stdout isn't detected as a TTY.
            table.force_no_tty().enforce_styling();
        }

        let header_cells: Vec<Cell> = headers
            .iter()
            .map(|h| {
                let mut c = Cell::new(*h).add_attribute(Attribute::Bold);
                if self.color && !matches!(self.theme, Theme::Mono) {
                    c = c.fg(TableColor::Cyan);
                }
                c
            })
            .collect();
        table.set_header(header_cells);

        for row in rows {
            table.add_row(row);
        }
        writeln!(self.stdout, "{table}")?;
        Ok(())
    }
}
