use std::io::{self, Write, stdout};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute, queue,
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};

use crate::cpu::{cpu_usage, usage_bar};
use crate::memory::kib_to_gib;
use crate::model::SystemSnapshot;

pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;

        execute!(
            stdout(),
            EnterAlternateScreen,
            Hide,
            EnableMouseCapture,
            MoveTo(0, 0)
        )?;

        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableMouseCapture, Show, LeaveAlternateScreen);

        let _ = disable_raw_mode();
    }
}

/// Number of terminal rows currently available for the frame.
pub fn viewport_height() -> usize {
    terminal::size().map(|(_, rows)| rows as usize).unwrap_or(0)
}

/// Builds the complete logical frame, one entry per line, without trailing
/// newlines.
pub fn build_frame(previous: &SystemSnapshot, current: &SystemSnapshot) -> Vec<String> {
    let core_count = current.cpu.len().saturating_sub(1);

    let mut lines = vec![
        "rwatch".to_string(),
        "======".to_string(),
        String::new(),
        format!("System: {}", current.system_name),
        format!("CPU:    {} × {}", current.cpu_name, core_count),
        format!("Uptime: {}", current.uptime),
        format!("Load:   {}", current.loadavg),
        String::new(),
        "CPU:".to_string(),
    ];

    for (before, after) in previous.cpu.iter().zip(current.cpu.iter()) {
        let usage = cpu_usage(&before.times, &after.times);
        let bar = usage_bar(usage, 20);

        let name = if before.name == "cpu" {
            "Total"
        } else {
            before.name.as_str()
        };

        lines.push(format!("  {name:<5} [{bar}] {usage:5.1}%"));
    }

    lines.push(String::new());
    lines.push("Memory:".to_string());

    lines.push(format!(
        "  Total:      {:.2} GiB",
        kib_to_gib(current.memory.total_kib)
    ));

    lines.push(format!(
        "  Available:  {:.2} GiB",
        kib_to_gib(current.memory.available_kib)
    ));

    lines.push(format!(
        "  Used:       {:.2} GiB",
        kib_to_gib(current.memory.used_kib())
    ));

    lines.push(format!(
        "  Usage:      {:.1}%",
        current.memory.usage_percent()
    ));

    lines.push(String::new());
    lines.push("Updating every 1s — Ctrl+C or q to quit".to_string());

    lines
}

/// Renders the visible portion of the frame for the current terminal size and
/// returns the clamped scroll offset.
pub fn render(
    previous: &SystemSnapshot,
    current: &SystemSnapshot,
    scroll_offset: usize,
) -> io::Result<usize> {
    let (cols, rows) = terminal::size()?;
    let cols = cols as usize;
    let rows = rows as usize;

    let lines = build_frame(previous, current);

    let max_scroll = lines.len().saturating_sub(rows);
    let scroll_offset = scroll_offset.min(max_scroll);

    let mut out = stdout();

    let mut visible_rows = 0usize;

    for (row, line) in lines.iter().skip(scroll_offset).take(rows).enumerate() {
        queue!(out, MoveTo(0, row as u16), Clear(ClearType::CurrentLine))?;

        // Truncate to the terminal width so the line never wraps. No newline
        // is written: every row is positioned explicitly, and a newline on the
        // last row would scroll the terminal.
        for ch in line.chars().take(cols) {
            write!(out, "{ch}")?;
        }

        visible_rows += 1;
    }

    if visible_rows < rows {
        queue!(
            out,
            MoveTo(0, visible_rows as u16),
            Clear(ClearType::FromCursorDown)
        )?;
    }

    out.flush()?;

    Ok(scroll_offset)
}
