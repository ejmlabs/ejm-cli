//! Native mode restoration and key events at the single terminal-input boundary.
use anyhow::{Context, Result};
#[cfg(any(windows, test))]
use crossterm::event::Event;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Clone)]
pub(crate) struct Modes {
    #[cfg(unix)]
    input: rustix::termios::Termios,
    #[cfg(windows)]
    input: u32,
    #[cfg(windows)]
    output: u32,
}

impl Modes {
    pub(crate) fn capture() -> Result<Self> {
        #[cfg(unix)]
        let input =
            rustix::termios::tcgetattr(std::io::stdin()).context("save terminal input mode")?;
        #[cfg(windows)]
        let input =
            crossterm_winapi::ConsoleMode::from(crossterm_winapi::Handle::current_in_handle()?)
                .mode()
                .context("save console input mode")?;
        #[cfg(windows)]
        let output =
            crossterm_winapi::ConsoleMode::from(crossterm_winapi::Handle::current_out_handle()?)
                .mode()
                .context("save console output mode")?;
        Ok(Self {
            #[cfg(any(unix, windows))]
            input,
            #[cfg(windows)]
            output,
        })
    }

    pub(crate) fn restore(&self) -> Result<()> {
        #[cfg(unix)]
        rustix::termios::tcsetattr(
            std::io::stdin(),
            rustix::termios::OptionalActions::Now,
            &self.input,
        )
        .context("restore terminal input mode")?;
        #[cfg(windows)]
        {
            // Try both handles even when one restoration fails.
            let input = crossterm_winapi::Handle::current_in_handle().and_then(|handle| {
                crossterm_winapi::ConsoleMode::from(handle).set_mode(self.input)
            });
            let output = crossterm_winapi::Handle::current_out_handle().and_then(|handle| {
                crossterm_winapi::ConsoleMode::from(handle).set_mode(self.output)
            });
            input.context("restore console input mode")?;
            output.context("restore console output mode")?;
        }
        Ok(())
    }
}

/// Discard unread OS input before the terminal can echo it again.
pub(crate) fn flush_pending_input() -> Result<()> {
    #[cfg(unix)]
    rustix::termios::tcflush(std::io::stdin(), rustix::termios::QueueSelector::IFlush)
        .context("discard pending terminal input")?;
    #[cfg(windows)]
    {
        // GetStdHandle borrows process stdin. Remove it from the close guard
        // before every fallible operation so neither success nor error closes it.
        let mut standard = winsafe::HSTD::GetStdHandle(winsafe::co::STD_HANDLE::INPUT)
            .context("open console input")?;
        standard
            .leak()
            .FlushConsoleInputBuffer()
            .context("discard pending console input")?;
    }
    Ok(())
}

/// Windows emits repeated key-down records as Press. Only action-related keys
/// need tracking; other repeats can retain their ordinary editing behavior.
#[cfg_attr(not(windows), derive(Default))]
#[cfg(any(windows, test))]
pub(crate) struct KeyInput {
    track_releases: bool,
    down: u8,
}

#[cfg(windows)]
impl Default for KeyInput {
    fn default() -> Self {
        Self::windows()
    }
}

#[cfg(any(windows, test))]
impl KeyInput {
    #[cfg(any(windows, test))]
    pub(crate) const fn windows() -> Self {
        Self {
            track_releases: true,
            down: 0,
        }
    }

    pub(crate) fn normalize(&mut self, input: Event) -> Event {
        let Event::Key(mut key) = input else {
            return input;
        };
        if self.track_releases {
            let bit = match key.code {
                KeyCode::Enter => 1,
                KeyCode::Char('c' | 'C') => 2,
                KeyCode::Char('d' | 'D') => 4,
                KeyCode::Char('k' | 'K') => 8,
                KeyCode::Char('s' | 'S') => 16,
                KeyCode::Char('y' | 'Y') => 32,
                KeyCode::Char('n' | 'N') => 64,
                _ => 0,
            };
            match key.kind {
                KeyEventKind::Release => self.down &= !bit,
                KeyEventKind::Press if self.down & bit != 0 => key.kind = KeyEventKind::Repeat,
                KeyEventKind::Press | KeyEventKind::Repeat => self.down |= bit,
            }
        }
        Event::Key(key)
    }
}

pub(crate) fn accepts_key(key: KeyEvent) -> bool {
    match key.kind {
        KeyEventKind::Press => true,
        KeyEventKind::Release => false,
        KeyEventKind::Repeat => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                !matches!(
                    key.code,
                    KeyCode::Char('c' | 'C' | 'd' | 'D' | 'k' | 'K' | 's' | 'S')
                )
            } else {
                key.code != KeyCode::Enter
                    || key
                        .modifiers
                        .intersects(KeyModifiers::ALT | KeyModifiers::SHIFT)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_down_events_repeat_edits_but_never_actions_until_release() {
        let mut keys = KeyInput {
            track_releases: true,
            down: 0,
        };
        for (code, modifiers) in [
            (KeyCode::Enter, KeyModifiers::NONE),
            (KeyCode::Char('c'), KeyModifiers::CONTROL),
            (KeyCode::Char('k'), KeyModifiers::CONTROL),
        ] {
            let pressed = KeyEvent::new(code, modifiers);
            assert_eq!(keys.normalize(Event::Key(pressed)), Event::Key(pressed));
            let repeated = KeyEvent::new_with_kind(code, modifiers, KeyEventKind::Repeat);
            assert_eq!(keys.normalize(Event::Key(pressed)), Event::Key(repeated));
            assert!(!accepts_key(repeated));
            let released = KeyEvent::new_with_kind(code, modifiers, KeyEventKind::Release);
            keys.normalize(Event::Key(released));
            assert!(!accepts_key(released));
            assert_eq!(keys.normalize(Event::Key(pressed)), Event::Key(pressed));
            keys.normalize(Event::Key(released));
        }
        assert!(accepts_key(KeyEvent::new_with_kind(
            KeyCode::Backspace,
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )));
        assert!(accepts_key(KeyEvent::new_with_kind(
            KeyCode::Char('y'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )));
    }
}
