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
    let mut current = platform::collect_snapshot()?;
    let mut scroll_offset = 0usize;

    scroll_offset = render(&previous, &current, scroll_offset)?;

    let mut next_refresh = Instant::now() + refresh_interval;

    loop {
        let now = Instant::now();

        let timeout = next_refresh.saturating_duration_since(now);

        if event::poll(timeout)? {
            // Input only changes the viewport; it never re-samples metrics, so
            // the fixed refresh cadence below is unaffected.
            match action_for(event::read()?) {
                Action::Quit => break,
                Action::ScrollUp(n) => scroll_offset = scroll_offset.saturating_sub(n),
                Action::ScrollDown(n) => scroll_offset = scroll_offset.saturating_add(n),
                Action::ScrollToTop => scroll_offset = 0,
                Action::ScrollToBottom => scroll_offset = usize::MAX,
                Action::Redraw => {}
                Action::None => continue,
            }

            // render() clamps the offset to the current frame and viewport.
            scroll_offset = render(&previous, &current, scroll_offset)?;

            continue;
        }

        previous = current;
        current = platform::collect_snapshot()?;

        scroll_offset = render(&previous, &current, scroll_offset)?;

        next_refresh += refresh_interval;
    }

    Ok(())
}
