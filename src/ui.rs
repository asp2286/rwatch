use std::io::{self, Write, stdout};

use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
        disable_raw_mode, enable_raw_mode,
    },
};
use crossterm::cursor::{Hide, Show};
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
            MoveTo(0, 0)
        )?;

        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();

        let _ = execute!(
            stdout(),
            Show,
            LeaveAlternateScreen
        );
    }
}

pub fn render(
    previous: &SystemSnapshot,
    current: &SystemSnapshot,
) -> io::Result<()> {
    let mut out = stdout();

    execute!(
        out,
        MoveTo(0, 0)
    )?;

    let core_count = current.cpu.len().saturating_sub(1);

    write!(out, "rwatch\r\n")?;
    write!(out, "======\r\n")?;
    write!(out, "\r\n")?;

    write!(out, "System: {}\r\n", current.system_name)?;
    write!(out, "CPU:    {} × {}\r\n", current.cpu_name, core_count)?;
    write!(out, "Uptime: {}\r\n", current.uptime)?;
    write!(out, "Load:   {}\r\n", current.loadavg)?;

    write!(out, "\r\n")?;
    write!(out, "CPU:\r\n")?;

    for (before, after) in previous.cpu.iter().zip(current.cpu.iter()) {
        let usage = cpu_usage(&before.times, &after.times);
        let bar = usage_bar(usage, 20);

        if before.name == "cpu" {
            write!(out, "  {:<5} [{bar}] {:5.1}%\r\n", "Total", usage)?;
        } else {
            write!(out, "  {:<5} [{bar}] {:5.1}%\r\n", before.name, usage)?;
        }
    }

    write!(out, "\r\n")?;
    write!(out, "Memory:\r\n")?;

    write!(
        out,
        "  Total:      {:.2} GiB\r\n",
        kib_to_gib(current.memory.total_kib)
    )?;

    write!(
        out,
        "  Available:  {:.2} GiB\r\n",
        kib_to_gib(current.memory.available_kib)
    )?;

    write!(
        out,
        "  Used:       {:.2} GiB\r\n",
        kib_to_gib(current.memory.used_kib())
    )?;

    write!(
        out,
        "  Usage:      {:.1}%\r\n",
        current.memory.usage_percent()
    )?;

    write!(out, "\r\n")?;
    write!(out, "Updating every 1s — Ctrl+C or q to quit\r\n")?;

    execute!(out, Clear(ClearType::FromCursorDown))?;
    
    out.flush()
}