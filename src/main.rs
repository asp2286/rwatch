mod cpu;
mod memory;
mod ui;
mod platform;
mod model;
mod uptime;

use std::error::Error;
use std::time::{Duration, Instant};

use crossterm::event::{
    self,
    Event,
    KeyCode,
    KeyEventKind,
    KeyModifiers,
};

use ui::{TerminalGuard, render};

fn main() -> Result<(), Box<dyn Error>> {
    let _terminal = TerminalGuard::enter()?;

    let refresh_interval = Duration::from_secs(1);

    let mut previous = platform::collect_snapshot()?;
    let mut next_refresh = Instant::now() + refresh_interval;

    loop {
        let now = Instant::now();

        let timeout = next_refresh.saturating_duration_since(now);

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let ctrl_c =
                        key.code == KeyCode::Char('c')
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

        let current = platform::collect_snapshot()?;

        render(
            &previous,
            &current,
        )?;

        previous = current;

        next_refresh += refresh_interval;
    }

    Ok(())
}