mod cpu;
mod memory;
mod model;
mod platform;
mod ui;
mod uptime;

use std::error::Error;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};

use ui::{TerminalGuard, render, viewport_height};

const MOUSE_SCROLL_LINES: usize = 3;

/// What an input event asks the main loop to do.
enum Action {
    Quit,
    ScrollUp(usize),
    ScrollDown(usize),
    ScrollToTop,
    ScrollToBottom,
    Redraw,
    None,
}

fn page_size() -> usize {
    viewport_height().saturating_sub(1).max(1)
}

fn action_for(event: Event) -> Action {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            let ctrl_c =
                key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);

            match key.code {
                _ if ctrl_c => Action::Quit,
                KeyCode::Char('q') => Action::Quit,
                KeyCode::Up => Action::ScrollUp(1),
                KeyCode::Down => Action::ScrollDown(1),
                KeyCode::PageUp => Action::ScrollUp(page_size()),
                KeyCode::PageDown => Action::ScrollDown(page_size()),
                KeyCode::Home => Action::ScrollToTop,
                KeyCode::End => Action::ScrollToBottom,
                _ => Action::None,
            }
        }

        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollUp => Action::ScrollUp(MOUSE_SCROLL_LINES),
            MouseEventKind::ScrollDown => Action::ScrollDown(MOUSE_SCROLL_LINES),
            _ => Action::None,
        },

        Event::Resize(_, _) => Action::Redraw,

        _ => Action::None,
    }
}

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