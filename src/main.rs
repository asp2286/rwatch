mod cpu;
mod memory;
mod ui;
mod uptime;

use std::error::Error;
use std::fs;
use std::time::{Duration, Instant};

use cpu::read_cpu_snapshots;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use memory::parse_memory_info;
use ui::{TerminalGuard, render};
use uptime::{format_uptime, parse_uptime_seconds};

fn main() -> Result<(), Box<dyn Error>> {
    let _terminal = TerminalGuard::enter()?;

    let refresh_interval = Duration::from_secs(1);

    let mut previous_cpu = read_cpu_snapshots()?;
    let mut next_refresh = Instant::now() + refresh_interval;

    loop {
        let now = Instant::now();

        let timeout = next_refresh.saturating_duration_since(now);

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let ctrl_c = key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL);

                    let q = key.code == KeyCode::Char('q');

                    if ctrl_c || q {
                        break;
                    }
                }

                _ => {
                    // Mouse, resize, other keys — ignore.
                }
            }

            continue;
        }

        let uptime_raw = fs::read_to_string("/proc/uptime")?;
        let loadavg = fs::read_to_string("/proc/loadavg")?;
        let meminfo_raw = fs::read_to_string("/proc/meminfo")?;

        let uptime_seconds = parse_uptime_seconds(&uptime_raw)?;
        let uptime = format_uptime(uptime_seconds);

        let memory = parse_memory_info(&meminfo_raw)?;
        let current_cpu = read_cpu_snapshots()?;

        render(
            &uptime,
            loadavg.trim(),
            &memory,
            &previous_cpu,
            &current_cpu,
        )?;

        previous_cpu = current_cpu;

        next_refresh += refresh_interval;
    }

    Ok(())
}
