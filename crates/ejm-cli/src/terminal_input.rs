//! One owned input parser, with explicit resets at private screen boundaries.
use anyhow::{Context, Result, bail};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MediaKeyCode, ModifierKeyCode, MouseButton, MouseEvent,
    MouseEventKind,
};
#[cfg(unix)]
use rustix::termios::{OptionalActions, SpecialCodeIndex};
use std::{collections::VecDeque, io};
use termwiz::input::{InputEvent, InputParser, KeyCode as ParsedKey, Modifiers, MouseButtons};
#[cfg(any(windows, test))]
use termwiz::input::{KeyCodeEncodeModes, KeyboardEncoding};
use zeroize::{Zeroize, Zeroizing};

const READ_BYTES: usize = 1024;
const UTF8_TAIL_BYTES: usize = 3;
const MAX_PENDING_BYTES: usize = 64 * 1024;
const MAX_QUEUED_EVENTS: usize = 1024;

// A terminal reader moves this state between guards. It is never reset merely
// because a protected prompt creates a fresh guard while an action key is held.
#[cfg(any(windows, test))]
static NATIVE_KEYS: std::sync::Mutex<Option<crate::terminal::KeyInput>> =
    std::sync::Mutex::new(Some(crate::terminal::KeyInput::windows()));

pub(crate) struct Input {
    parser: InputParser,
    events: VecDeque<Event>,
    pending_bytes: usize,
    utf8_tail: Zeroizing<[u8; UTF8_TAIL_BYTES]>,
    utf8_tail_len: usize,
    size: (u16, u16),
    mouse_button: Option<MouseButton>,
    #[cfg(any(windows, test))]
    keys: crate::terminal::KeyInput,
    #[cfg(any(windows, test))]
    returns_keys: bool,
}

impl Input {
    /// Call after the terminal guard has enabled raw mode.
    pub(crate) fn new() -> Result<Self> {
        let mut input = Self::empty((0, 0));
        input.setup()?;
        #[cfg(windows)]
        input.take_native_keys()?;
        Ok(input)
    }

    fn empty(size: (u16, u16)) -> Self {
        Self {
            parser: InputParser::new(),
            events: VecDeque::new(),
            pending_bytes: 0,
            utf8_tail: Zeroizing::new([0; UTF8_TAIL_BYTES]),
            utf8_tail_len: 0,
            size,
            mouse_button: None,
            #[cfg(any(windows, test))]
            keys: crate::terminal::KeyInput::windows(),
            #[cfg(any(windows, test))]
            returns_keys: false,
        }
    }

    /// The guard owns restoration of the exact original terminal attributes.
    pub(crate) fn setup(&mut self) -> Result<()> {
        #[cfg(unix)]
        {
            let mut modes = rustix::termios::tcgetattr(io::stdin())?;
            modes.special_codes[SpecialCodeIndex::VMIN] = 0;
            modes.special_codes[SpecialCodeIndex::VTIME] = 0;
            rustix::termios::tcsetattr(io::stdin(), OptionalActions::Now, &modes)
                .context("configure nonblocking terminal input")?;
        }
        self.size = crossterm::terminal::size()?;
        Ok(())
    }

    #[cfg(any(windows, test))]
    fn take_native_keys(&mut self) -> Result<()> {
        self.keys = NATIVE_KEYS
            .lock()
            .map_err(|_| anyhow::anyhow!("terminal key state is unavailable"))?
            .take()
            .context("another terminal input reader is active")?;
        self.returns_keys = true;
        Ok(())
    }

    /// Nonblocking: read at most one fixed byte or native-record batch per call.
    pub(crate) fn poll(&mut self) -> Result<bool> {
        if !self.events.is_empty() {
            return Ok(true);
        }
        let parsed = self.read_ready();
        if let Err(error) = parsed {
            self.reset()?;
            return Err(error);
        }
        if self.events.is_empty() {
            let size = crossterm::terminal::size()?;
            if size != self.size {
                self.size = size;
                self.events.push_back(Event::Resize(size.0, size.1));
            }
        }
        Ok(!self.events.is_empty())
    }

    #[cfg(unix)]
    fn read_ready(&mut self) -> Result<()> {
        let mut bytes = Zeroizing::new([0_u8; READ_BYTES]);
        match rustix::io::read(io::stdin(), &mut bytes[..]) {
            // Even a short read can end in the middle of an escape sequence.
            // Resolve a bare Escape only after a subsequent no-data poll.
            Ok(count) => self.feed(&bytes[..count], count != 0),
            Err(rustix::io::Errno::AGAIN) => self.feed(&[], false),
            Err(rustix::io::Errno::INTR) => Ok(()),
            Err(error) => Err(error).context("read terminal input"),
        }
    }

    #[cfg(windows)]
    fn read_ready(&mut self) -> Result<()> {
        for _ in 0..READ_BYTES {
            if !crossterm::event::poll(std::time::Duration::ZERO)? {
                return self.feed(&[], false);
            }
            self.feed_native(crossterm::event::read()?)?;
            if !self.events.is_empty() {
                break;
            }
        }
        Ok(())
    }

    /// Dequeue only. Call `poll` first; an empty queue never blocks.
    pub(crate) fn read(&mut self) -> Result<Event> {
        self.events.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::WouldBlock, "no terminal input is ready").into()
        })
    }

    /// Destroy complete and partial input before another screen may receive it.
    pub(crate) fn reset(&mut self) -> Result<()> {
        self.clear_pending();
        #[cfg(windows)]
        let drain = (|| {
            for _ in 0..MAX_QUEUED_EVENTS {
                if !crossterm::event::poll(std::time::Duration::ZERO)? {
                    return Ok(());
                }
                discard_event(self.keys.normalize(crossterm::event::read()?));
            }
            bail!("terminal input cleanup exceeded its event limit")
        })();
        let flush = crate::terminal::flush_pending_input();
        let setup = self.setup();
        flush?;
        #[cfg(windows)]
        drain?;
        setup
    }

    #[cfg(any(windows, test))]
    fn feed_native(&mut self, event: Event) -> Result<()> {
        let event = self.keys.normalize(event);
        let Event::Key(key) = event else {
            return self.queue_native(event);
        };
        if key.kind == crossterm::event::KeyEventKind::Release {
            return Ok(());
        }
        let Some((code, modifiers)) = encoded_key(key) else {
            return self.queue_native(Event::Key(key));
        };
        let bytes = Zeroizing::new(
            code.encode(
                modifiers,
                KeyCodeEncodeModes {
                    // Plain xterm encoding drops Shift/Ctrl on Enter. Preserve
                    // the native newline/shortcut distinction.
                    encoding: if key.code == KeyCode::Enter
                        && key
                            .modifiers
                            .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL)
                    {
                        KeyboardEncoding::CsiU
                    } else {
                        KeyboardEncoding::Xterm
                    },
                    application_cursor_keys: false,
                    newline_mode: false,
                    modify_other_keys: None,
                },
                true,
            )
            .context("adapt native terminal key")?,
        );
        let previous = self.events.len();
        // Crossterm owns native Unicode/surrogate decoding and key kinds.
        // Termwiz owns escape and paste parsing, including embedded Return.
        self.feed(bytes.as_bytes(), true)?;
        for event in self.events.iter_mut().skip(previous) {
            if let Event::Key(parsed) = event {
                parsed.kind = key.kind;
            }
        }
        Ok(())
    }

    #[cfg(any(windows, test))]
    fn queue_native(&mut self, event: Event) -> Result<()> {
        if self.events.len() >= MAX_QUEUED_EVENTS {
            discard_event(event);
            self.clear_pending();
            bail!("terminal input exceeds the event queue limit");
        }
        self.events.push_back(event);
        Ok(())
    }

    fn clear_pending(&mut self) {
        discard_events(&mut self.events);
        // Termwiz owns incomplete escape/paste bytes. A fresh parser prevents
        // their later completion from reaching the next screen.
        self.parser = InputParser::new();
        self.pending_bytes = 0;
        self.utf8_tail.zeroize();
        self.utf8_tail_len = 0;
        self.mouse_button = None;
    }

    fn feed(&mut self, bytes: &[u8], maybe_more: bool) -> Result<()> {
        if bytes.len() > READ_BYTES
            || bytes.len() > MAX_PENDING_BYTES.saturating_sub(self.pending_bytes)
        {
            self.clear_pending();
            bail!("terminal input exceeds its byte limit");
        }
        self.pending_bytes += bytes.len();
        let mut joined = Zeroizing::new([0_u8; READ_BYTES + UTF8_TAIL_BYTES]);
        let len = self.utf8_tail_len + bytes.len();
        joined[..self.utf8_tail_len].copy_from_slice(&self.utf8_tail[..self.utf8_tail_len]);
        joined[self.utf8_tail_len..len].copy_from_slice(bytes);
        self.utf8_tail.zeroize();
        self.utf8_tail_len = 0;
        // Termwiz's paste decoder assumes valid UTF-8 when advancing its byte
        // buffer. Validate before feeding it and carry a split code point only.
        let valid_len = match std::str::from_utf8(&joined[..len]) {
            Ok(_) => len,
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(_) => {
                self.clear_pending();
                bail!("terminal input contains invalid UTF-8");
            }
        };
        let tail = &joined[valid_len..len];
        if tail.len() > UTF8_TAIL_BYTES {
            self.clear_pending();
            bail!("terminal input contains invalid UTF-8");
        }
        self.utf8_tail[..tail.len()].copy_from_slice(tail);
        self.utf8_tail_len = tail.len();
        self.feed_valid(&joined[..valid_len], maybe_more, len)
    }

    fn feed_valid(&mut self, bytes: &[u8], maybe_more: bool, retained_bound: usize) -> Result<()> {
        let mut emitted = false;
        let mut overflow = false;
        // Termwiz maps raw LF and CR to Enter. Keep this app's Ctrl+J editing
        // behavior by identifying only literal LF at the adapter boundary.
        for segment in bytes.split_inclusive(|byte| *byte == b'\n') {
            if let Some(prefix) = segment.strip_suffix(b"\n") {
                self.parse(prefix, true, false, &mut emitted, &mut overflow);
                self.parse(b"\n", maybe_more, true, &mut emitted, &mut overflow);
            } else {
                self.parse(segment, maybe_more, false, &mut emitted, &mut overflow);
            }
        }
        if bytes.is_empty() {
            self.parse(bytes, maybe_more, false, &mut emitted, &mut overflow);
        }
        if overflow {
            self.clear_pending();
            bail!("terminal input exceeds the event queue limit");
        }
        if emitted {
            // The parser does not expose its retained byte count. Counting the
            // whole last read plus its UTF-8 carry bounds any remaining suffix.
            self.pending_bytes = retained_bound;
        }
        Ok(())
    }

    fn parse(
        &mut self,
        bytes: &[u8],
        maybe_more: bool,
        line_feed: bool,
        emitted: &mut bool,
        overflow: &mut bool,
    ) {
        let events = &mut self.events;
        let mouse_button = &mut self.mouse_button;
        self.parser.parse(
            bytes,
            |input| {
                *emitted = true;
                if let Some(event) = adapt(input, line_feed, mouse_button) {
                    if events.len() < MAX_QUEUED_EVENTS {
                        events.push_back(event);
                    } else {
                        discard_event(event);
                        *overflow = true;
                    }
                }
            },
            maybe_more,
        );
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        discard_events(&mut self.events);
        #[cfg(any(windows, test))]
        if self.returns_keys {
            let mut saved = match NATIVE_KEYS.lock() {
                Ok(saved) => saved,
                Err(poisoned) => poisoned.into_inner(),
            };
            *saved = Some(std::mem::replace(
                &mut self.keys,
                crate::terminal::KeyInput::windows(),
            ));
        }
    }
}

fn discard_event(event: Event) {
    if let Event::Paste(mut text) = event {
        text.zeroize();
    }
}

fn discard_events(events: &mut VecDeque<Event>) {
    for event in events.drain(..) {
        discard_event(event);
    }
}

#[cfg(any(windows, test))]
fn encoded_key(input: KeyEvent) -> Option<(ParsedKey, Modifiers)> {
    if input
        .modifiers
        .intersects(KeyModifiers::HYPER | KeyModifiers::META)
    {
        return None;
    }
    let mut modifiers = Modifiers::NONE;
    for (source, target) in [
        (KeyModifiers::SHIFT, Modifiers::SHIFT),
        (KeyModifiers::CONTROL, Modifiers::CTRL),
        (KeyModifiers::ALT, Modifiers::ALT),
        (KeyModifiers::SUPER, Modifiers::SUPER),
    ] {
        if input.modifiers.contains(source) {
            modifiers |= target;
        }
    }
    let code = match input.code {
        KeyCode::Char(character) => ParsedKey::Char(character),
        KeyCode::Enter => ParsedKey::Enter,
        KeyCode::Esc => ParsedKey::Escape,
        KeyCode::Backspace => ParsedKey::Backspace,
        KeyCode::Tab => ParsedKey::Tab,
        KeyCode::BackTab => {
            modifiers |= Modifiers::SHIFT;
            ParsedKey::Tab
        }
        KeyCode::Left => ParsedKey::LeftArrow,
        KeyCode::Right => ParsedKey::RightArrow,
        KeyCode::Up => ParsedKey::UpArrow,
        KeyCode::Down => ParsedKey::DownArrow,
        KeyCode::Home => ParsedKey::Home,
        KeyCode::End => ParsedKey::End,
        KeyCode::PageUp => ParsedKey::PageUp,
        KeyCode::PageDown => ParsedKey::PageDown,
        KeyCode::Delete => ParsedKey::Delete,
        KeyCode::Insert => ParsedKey::Insert,
        KeyCode::F(number) => ParsedKey::Function(number),
        _ => return None,
    };
    Some((code, modifiers))
}

fn modifiers(input: Modifiers) -> KeyModifiers {
    let input = input.remove_positional_mods();
    let mut modifiers = KeyModifiers::NONE;
    for (source, target) in [
        (Modifiers::SHIFT, KeyModifiers::SHIFT),
        (Modifiers::CTRL, KeyModifiers::CONTROL),
        (Modifiers::ALT, KeyModifiers::ALT),
        (Modifiers::SUPER, KeyModifiers::SUPER),
    ] {
        if input.contains(source) {
            modifiers |= target;
        }
    }
    modifiers
}

fn key(code: ParsedKey, mut modifiers: KeyModifiers, line_feed: bool) -> Option<Event> {
    let code = match code {
        ParsedKey::Enter if line_feed => {
            modifiers |= KeyModifiers::CONTROL;
            KeyCode::Char('j')
        }
        ParsedKey::Cancel => {
            modifiers |= KeyModifiers::CONTROL;
            KeyCode::Char('c')
        }
        ParsedKey::Char('\0') => KeyCode::Null,
        ParsedKey::Char('\r') | ParsedKey::Enter => KeyCode::Enter,
        ParsedKey::Char(character) => KeyCode::Char(character),
        ParsedKey::Backspace => KeyCode::Backspace,
        ParsedKey::Tab if modifiers.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
        ParsedKey::Tab => KeyCode::Tab,
        ParsedKey::Escape => KeyCode::Esc,
        ParsedKey::LeftArrow | ParsedKey::ApplicationLeftArrow => KeyCode::Left,
        ParsedKey::RightArrow | ParsedKey::ApplicationRightArrow => KeyCode::Right,
        ParsedKey::UpArrow | ParsedKey::ApplicationUpArrow => KeyCode::Up,
        ParsedKey::DownArrow | ParsedKey::ApplicationDownArrow => KeyCode::Down,
        ParsedKey::Home | ParsedKey::KeyPadHome => KeyCode::Home,
        ParsedKey::End | ParsedKey::KeyPadEnd => KeyCode::End,
        ParsedKey::PageUp | ParsedKey::KeyPadPageUp => KeyCode::PageUp,
        ParsedKey::PageDown | ParsedKey::KeyPadPageDown => KeyCode::PageDown,
        ParsedKey::Insert => KeyCode::Insert,
        ParsedKey::Delete => KeyCode::Delete,
        ParsedKey::Function(number) => KeyCode::F(number),
        ParsedKey::CapsLock => KeyCode::CapsLock,
        ParsedKey::NumLock => KeyCode::NumLock,
        ParsedKey::ScrollLock => KeyCode::ScrollLock,
        ParsedKey::PrintScreen => KeyCode::PrintScreen,
        ParsedKey::Pause => KeyCode::Pause,
        ParsedKey::Menu | ParsedKey::Applications => KeyCode::Menu,
        ParsedKey::KeyPadBegin => KeyCode::KeypadBegin,
        ParsedKey::VolumeMute => KeyCode::Media(MediaKeyCode::MuteVolume),
        ParsedKey::VolumeDown => KeyCode::Media(MediaKeyCode::LowerVolume),
        ParsedKey::VolumeUp => KeyCode::Media(MediaKeyCode::RaiseVolume),
        ParsedKey::MediaNextTrack => KeyCode::Media(MediaKeyCode::TrackNext),
        ParsedKey::MediaPrevTrack => KeyCode::Media(MediaKeyCode::TrackPrevious),
        ParsedKey::MediaStop => KeyCode::Media(MediaKeyCode::Stop),
        ParsedKey::MediaPlayPause => KeyCode::Media(MediaKeyCode::PlayPause),
        ParsedKey::Shift | ParsedKey::LeftShift => KeyCode::Modifier(ModifierKeyCode::LeftShift),
        ParsedKey::RightShift => KeyCode::Modifier(ModifierKeyCode::RightShift),
        ParsedKey::Control | ParsedKey::LeftControl => {
            KeyCode::Modifier(ModifierKeyCode::LeftControl)
        }
        ParsedKey::RightControl => KeyCode::Modifier(ModifierKeyCode::RightControl),
        ParsedKey::Alt | ParsedKey::LeftAlt => KeyCode::Modifier(ModifierKeyCode::LeftAlt),
        ParsedKey::RightAlt => KeyCode::Modifier(ModifierKeyCode::RightAlt),
        ParsedKey::Super | ParsedKey::LeftWindows => KeyCode::Modifier(ModifierKeyCode::LeftSuper),
        ParsedKey::RightWindows => KeyCode::Modifier(ModifierKeyCode::RightSuper),
        ParsedKey::Hyper => KeyCode::Modifier(ModifierKeyCode::LeftHyper),
        ParsedKey::Meta => KeyCode::Modifier(ModifierKeyCode::LeftMeta),
        _ => return None,
    };
    Some(Event::Key(KeyEvent::new(code, modifiers)))
}

fn adapt(input: InputEvent, line_feed: bool, pressed: &mut Option<MouseButton>) -> Option<Event> {
    match input {
        InputEvent::Key(input) => key(input.key, modifiers(input.modifiers), line_feed),
        InputEvent::Paste(text) => Some(Event::Paste(text)),
        InputEvent::Mouse(mouse) => {
            let positive = mouse.mouse_buttons.contains(MouseButtons::WHEEL_POSITIVE);
            let kind = if mouse.mouse_buttons.contains(MouseButtons::VERT_WHEEL) {
                if positive {
                    MouseEventKind::ScrollUp
                } else {
                    MouseEventKind::ScrollDown
                }
            } else if mouse.mouse_buttons.contains(MouseButtons::HORZ_WHEEL) {
                if positive {
                    MouseEventKind::ScrollLeft
                } else {
                    MouseEventKind::ScrollRight
                }
            } else {
                let button = [
                    (MouseButtons::LEFT, MouseButton::Left),
                    (MouseButtons::RIGHT, MouseButton::Right),
                    (MouseButtons::MIDDLE, MouseButton::Middle),
                ]
                .into_iter()
                .find_map(|(flag, button)| mouse.mouse_buttons.contains(flag).then_some(button));
                let kind = match (button, *pressed) {
                    (Some(button), Some(previous)) if button == previous => {
                        MouseEventKind::Drag(button)
                    }
                    (Some(button), _) => MouseEventKind::Down(button),
                    (None, Some(previous)) => MouseEventKind::Up(previous),
                    (None, None) => MouseEventKind::Moved,
                };
                *pressed = button;
                kind
            };
            Some(Event::Mouse(MouseEvent {
                kind,
                column: mouse.x.saturating_sub(1),
                row: mouse.y.saturating_sub(1),
                modifiers: modifiers(mouse.modifiers),
            }))
        }
        InputEvent::Resized { cols, rows } => {
            Some(Event::Resize(cols.try_into().ok()?, rows.try_into().ok()?))
        }
        InputEvent::PixelMouse(_) | InputEvent::Wake => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_text(input: &mut Input, text: &str) -> Result<()> {
        for character in text.chars() {
            let (code, modifiers) = match character {
                '\x1b' => (KeyCode::Esc, KeyModifiers::NONE),
                '\r' => (KeyCode::Enter, KeyModifiers::NONE),
                '\n' => (KeyCode::Char('j'), KeyModifiers::CONTROL),
                _ => (KeyCode::Char(character), KeyModifiers::NONE),
            };
            input.feed_native(Event::Key(KeyEvent::new(code, modifiers)))?;
            input.feed_native(Event::Key(KeyEvent::new_with_kind(
                code,
                modifiers,
                crossterm::event::KeyEventKind::Release,
            )))?;
        }
        Ok(())
    }

    #[test]
    fn native_paste_keeps_unicode_and_return_and_discards_private_tails() -> Result<()> {
        let mut input = Input::empty((80, 24));
        native_text(&mut input, "\x1b[200~café\r\nsecond \u{1f980}\x1b[201~")?;
        assert_eq!(
            input.read()?,
            Event::Paste("café\r\nsecond \u{1f980}".into())
        );
        assert!(input.events.is_empty(), "paste admitted an action key");
        for text in ["\x1b[200~private\r\x1b[201~\r", "\x1b[200~private\r"] {
            native_text(&mut input, text)?;
            input.clear_pending();
            native_text(&mut input, "\x1b[201~safe")?;
            let expected = "safe"
                .chars()
                .map(|character| {
                    Event::Key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                })
                .collect::<Vec<_>>();
            assert_eq!(input.events.drain(..).collect::<Vec<_>>(), expected);
        }
        Ok(())
    }

    #[test]
    fn native_navigation_and_held_actions_keep_the_dispatch_policy() -> Result<()> {
        use crossterm::event::KeyEventKind::{Press, Release, Repeat};

        let mut input = Input::empty((80, 24));
        let navigation = KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL);
        input.feed_native(Event::Key(navigation))?;
        assert_eq!(input.read()?, Event::Key(navigation));
        for modifiers in [
            KeyModifiers::SHIFT,
            KeyModifiers::CONTROL,
            KeyModifiers::ALT | KeyModifiers::SHIFT,
        ] {
            let newline = KeyEvent::new(KeyCode::Enter, modifiers);
            input.feed_native(Event::Key(newline))?;
            assert_eq!(input.read()?, Event::Key(newline));
            input.feed_native(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Enter,
                modifiers,
                Release,
            )))?;
        }
        for code in [KeyCode::Enter, KeyCode::Char('y'), KeyCode::Backspace] {
            for kind in [Press, Press, Release, Press] {
                input.feed_native(Event::Key(KeyEvent::new_with_kind(
                    code,
                    KeyModifiers::NONE,
                    kind,
                )))?;
            }
            for kind in [Press, Repeat, Press] {
                let Event::Key(key) = input.read()? else {
                    bail!("native key became a non-key event");
                };
                assert_eq!(key.code, code);
                assert_eq!(
                    key.kind,
                    if code == KeyCode::Backspace {
                        Press
                    } else {
                        kind
                    }
                );
                if code == KeyCode::Enter {
                    assert_eq!(crate::terminal::accepts_key(key), kind == Press);
                }
            }
        }
        assert!(input.events.is_empty());
        Ok(())
    }

    #[test]
    fn guard_handoff_keeps_held_consent_keys_blocked_until_release() -> Result<()> {
        use crossterm::event::KeyEventKind::{Press, Release, Repeat};

        let mut first = Input::empty((80, 24));
        first.take_native_keys()?;
        for code in [KeyCode::Enter, KeyCode::Char('y')] {
            first.feed_native(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))?;
        }
        first.clear_pending();
        drop(first);

        let mut next = Input::empty((80, 24));
        next.take_native_keys()?;
        for code in [KeyCode::Enter, KeyCode::Char('y')] {
            next.feed_native(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))?;
            assert_eq!(
                next.read()?,
                Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, Repeat))
            );
            next.feed_native(Event::Key(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                Release,
            )))?;
            next.feed_native(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))?;
            assert_eq!(
                next.read()?,
                Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, Press))
            );
            next.feed_native(Event::Key(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                Release,
            )))?;
        }
        Ok(())
    }

    #[test]
    fn private_boundary_discards_complete_and_incomplete_pastes() -> Result<()> {
        for bytes in [
            b"\x1b[200~private-complete\x1b[201~\r".as_slice(),
            b"\x1b[200~private-partial".as_slice(),
        ] {
            let mut input = Input::empty((80, 24));
            input.feed(bytes, false)?;
            input.clear_pending();
            input.feed(b"\x1b[201~safe", false)?;
            let expected = "safe"
                .chars()
                .map(|character| {
                    Event::Key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                })
                .collect::<Vec<_>>();
            assert_eq!(input.events.drain(..).collect::<Vec<_>>(), expected);
        }
        Ok(())
    }

    #[test]
    fn incomplete_input_limit_discards_state_before_accepting_more() -> Result<()> {
        let mut input = Input::empty((80, 24));
        input.feed(b"\x1b[200~", false)?;
        let bytes = [b'x'; READ_BYTES];
        for _ in 0..(MAX_PENDING_BYTES / READ_BYTES - 1) {
            input.feed(&bytes, true)?;
        }
        assert!(input.feed(&bytes, true).is_err());
        input.feed(b"\x1b[201~", false)?;
        assert!(
            input.events.is_empty(),
            "an oversized private paste survived reset"
        );
        Ok(())
    }

    #[test]
    fn invalid_utf8_paste_rejects_without_reaching_the_parser() -> Result<()> {
        for malformed in [b"\xff".as_slice(), b"\xf0\x28\x8c\x28".as_slice()] {
            let mut input = Input::empty((80, 24));
            input.feed(b"\x1b[200~private", true)?;
            assert!(input.feed(malformed, true).is_err());
            input.feed(b"\x1b[201~safe", false)?;
            assert_eq!(input.events.len(), 4);
            assert!(
                input
                    .events
                    .iter()
                    .all(|event| !matches!(event, Event::Paste(_)))
            );
        }
        Ok(())
    }

    #[test]
    fn split_unicode_is_preserved_and_private_reset_discards_its_tail() -> Result<()> {
        let mut input = Input::empty((80, 24));
        input.feed(b"\x1b[200~", true)?;
        for byte in "\u{1f980}".as_bytes() {
            input.feed(&[*byte], true)?;
        }
        input.feed(b"\x1b[201~", false)?;
        assert_eq!(input.read()?, Event::Paste("\u{1f980}".into()));
        input.feed(&[0xf0, 0x9f], true)?;
        input.clear_pending();
        input.feed(b"a", false)?;
        assert_eq!(
            input.read()?,
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
        );
        assert!(input.events.is_empty());
        Ok(())
    }

    #[test]
    fn fragmented_navigation_waits_for_more_and_bare_escape_resolves_when_idle() -> Result<()> {
        let mut input = Input::empty((80, 24));
        input.feed(b"\x1b[", true)?;
        assert!(input.events.is_empty());
        input.feed(b"A", true)?;
        assert_eq!(
            input.read()?,
            Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        );
        input.feed(b"\x1b", true)?;
        assert!(input.events.is_empty());
        input.feed(b"", false)?;
        assert_eq!(
            input.read()?,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        );
        input.feed(b"x", true)?;
        assert_eq!(
            input.read()?,
            Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
        );
        Ok(())
    }

    #[test]
    fn ordinary_navigation_shortcuts_unicode_paste_and_scroll_survive_adaptation() -> Result<()> {
        let mut input = Input::empty((80, 24));
        input.feed(b"\x1b[D\x1b[1;5C\x03\n\r\x1b[Z\x1b[200~", true)?;
        input.feed(
            "line one\n\u{1f980}\x1b[201~\x1b[<64;42;12M\x1b[<65;42;12M".as_bytes(),
            false,
        )?;
        let key = |code, modifiers| Event::Key(KeyEvent::new(code, modifiers));
        let scroll = |kind| {
            Event::Mouse(MouseEvent {
                kind,
                column: 41,
                row: 11,
                modifiers: KeyModifiers::NONE,
            })
        };
        assert_eq!(
            input.events.drain(..).collect::<Vec<_>>(),
            vec![
                key(KeyCode::Left, KeyModifiers::NONE),
                key(KeyCode::Right, KeyModifiers::CONTROL),
                key(KeyCode::Char('c'), KeyModifiers::CONTROL),
                key(KeyCode::Char('j'), KeyModifiers::CONTROL),
                key(KeyCode::Enter, KeyModifiers::NONE),
                key(KeyCode::BackTab, KeyModifiers::SHIFT),
                Event::Paste("line one\n\u{1f980}".into()),
                scroll(MouseEventKind::ScrollUp),
                scroll(MouseEventKind::ScrollDown),
            ]
        );
        Ok(())
    }
}
