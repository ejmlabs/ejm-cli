//! Sensitive pairing stays on the alternate screen and never enters composer history.
use anyhow::{Result, bail, ensure};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    widgets::{Clear, Paragraph, Wrap},
};
use std::{io::IsTerminal, time::Duration};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

static PRIVATE_INPUTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Discard unread secrets before canonical mode can echo them back to the shell.
struct PendingInputCleanup;

impl PendingInputCleanup {
    fn enter() -> Self {
        use std::sync::{Once, atomic::Ordering};

        static PANIC_HOOK: Once = Once::new();
        // TerminalGuard has already installed its hook. Run this cleanup first:
        // panic hooks restore the terminal before stack unwinding drops guards.
        PANIC_HOOK.call_once(|| {
            let previous_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |information| {
                if PRIVATE_INPUTS.load(Ordering::Relaxed) > 0 {
                    Self::discard();
                }
                previous_hook(information);
            }));
        });
        PRIVATE_INPUTS.fetch_add(1, Ordering::Relaxed);
        Self
    }

    fn discard() {
        let _ = crate::terminal::flush_pending_input();
    }
}

impl Drop for PendingInputCleanup {
    fn drop(&mut self) {
        Self::discard();
        PRIVATE_INPUTS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn draw(frame: &mut Frame<'_>, title: &str, body: &str) {
    frame.render_widget(Clear, frame.area());
    let text = format!("{title}\n\n{}\n", crate::render::safe_text(body));
    frame.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: false }),
        frame.area(),
    );
}

pub async fn confirm(body: &str, cancel: CancellationToken) -> Result<bool> {
    ensure_terminal()?;
    let mut terminal = crate::ui::TerminalGuard::enter()?;
    let _pending_input = PendingInputCleanup::enter();
    terminal.reset_input()?;
    terminal.terminal.draw(|frame| {
        draw(
            frame,
            "Local confirmation",
            &format!("{body}\n\nY confirms. N or Esc cancels."),
        );
    })?;
    loop {
        tokio::select! {
            biased;
            () = cancel.cancelled() => bail!("confirmation cancelled"),
            () = tokio::time::sleep(Duration::from_millis(20)) => {
                if terminal.poll_input()? && let Event::Key(key) = terminal.read_input()? &&
                    key.kind == KeyEventKind::Press {
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') { bail!("confirmation cancelled"); }
                    match key.code {
                        KeyCode::Char('y' | 'Y') => { terminal.reset_input()?; return Ok(true); },
                        KeyCode::Char('n' | 'N') | KeyCode::Esc => { terminal.reset_input()?; return Ok(false); },
                        _ => {}
                    }
                }
            }
        }
    }
}

pub async fn read_invitation(cancel: CancellationToken) -> Result<Zeroizing<String>> {
    ensure_terminal()?;
    let mut terminal = crate::ui::TerminalGuard::enter()?;
    let _pending_input = PendingInputCleanup::enter();
    terminal.reset_input()?;
    let mut text = Zeroizing::new(String::new());
    loop {
        terminal.terminal.draw(|frame| draw(frame, "Connect to a shared session",
            &format!("Paste the private invitation here. Its text stays hidden.\n{} bytes entered. Enter connects. Esc cancels.", text.len())))?;
        tokio::select! {
            biased;
            () = cancel.cancelled() => bail!("pairing cancelled"),
            () = tokio::time::sleep(Duration::from_millis(20)) => {
                if !terminal.poll_input()? { continue; }
                match terminal.read_input()? {
                    Event::Paste(value) => {
                        let value = Zeroizing::new(value);
                        ensure!(text.len().saturating_add(value.len()) <= 8192, "invitation exceeds limit");
                        ensure!(!value.chars().any(char::is_control), "invalid invitation");
                        text.push_str(&value);
                    }
                    Event::Key(key) if crate::terminal::accepts_key(key) && key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') => bail!("pairing cancelled"),
                    Event::Key(key) if crate::terminal::accepts_key(key) => match key.code {
                        KeyCode::Esc => bail!("pairing cancelled"),
                        KeyCode::Enter if !text.is_empty() => { terminal.reset_input()?; return Ok(text); },
                        KeyCode::Backspace => { text.pop(); }
                        KeyCode::Char(c) if !c.is_control() => { ensure!(text.len().saturating_add(c.len_utf8()) <= 8192, "invitation exceeds limit"); text.push(c); },
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}

pub async fn login_notice(
    body: &str,
    no_browser: bool,
    url: &str,
    cancel: CancellationToken,
) -> Result<()> {
    ensure_terminal()?;
    let mut terminal = crate::ui::TerminalGuard::enter()?;
    let _pending_input = PendingInputCleanup::enter();
    terminal.reset_input()?;
    terminal.terminal.draw(|frame| {
        draw(
            frame,
            "Relay sign-in",
            &format!("{body}\n\nEnter starts waiting for browser approval. Esc cancels."),
        );
    })?;
    if !no_browser {
        crate::browser::launch(ejm_harness_auth::SecretString::new(url), cancel.clone()).await?;
    }
    loop {
        tokio::select! {
            biased;
            () = cancel.cancelled() => bail!("relay login cancelled"),
            () = tokio::time::sleep(Duration::from_millis(20)) => {
                if terminal.poll_input()? && let Event::Key(key) = terminal.read_input()? &&
                    key.kind == KeyEventKind::Press {
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') { bail!("relay login cancelled"); }
                    match key.code { KeyCode::Enter => { terminal.reset_input()?; return Ok(()); }, KeyCode::Esc => bail!("relay login cancelled"), _ => {} }
                }
            }
        }
    }
}

pub fn ensure_terminal() -> Result<()> {
    ensure!(
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        "pairing and relay login require an interactive terminal; secrets cannot be passed as arguments"
    );
    Ok(())
}
