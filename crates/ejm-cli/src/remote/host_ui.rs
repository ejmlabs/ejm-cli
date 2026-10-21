use super::{
    account::RelayAccount,
    backend::LocalBackend,
    identity::{EndpointIdentity, IdentityStorage},
    invitation::{Enrollment, Invitation, ShareRole},
    transport::{self, HostNotice},
};
use crate::{
    app::{App, RemoteBadge},
    args::Args,
};
use anyhow::{Result, ensure};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

struct Panel {
    title: String,
    body: Zeroizing<String>,
    answer: Option<oneshot::Sender<bool>>,
}

struct HostTask {
    epoch: String,
    expires_at_ms: u64,
    notices: mpsc::Receiver<HostNotice>,
    join: JoinHandle<()>,
}

pub struct HostUi {
    account: Option<RelayAccount>,
    session: Option<String>,
    role: ShareRole,
    connected: bool,
    panel: Option<Panel>,
    shutdown: CancellationToken,
    cancel: CancellationToken,
    task: Option<HostTask>,
}

impl HostUi {
    pub async fn prepare(args: &Args, shutdown: CancellationToken) -> Result<Self> {
        let account = if super::host_role(args).is_some() {
            let origin = super::origin(args)?;
            Some(if args.memory_only {
                #[cfg(test)]
                tests::startup_login_fixture(shutdown.clone()).await?;
                RelayAccount::login(origin, &args.state_dir(), true, false, shutdown.clone())
                    .await?
            } else {
                RelayAccount::load(origin, &args.state_dir(), &shutdown).await?
            })
        } else {
            None
        };
        Ok(Self {
            account,
            session: None,
            role: ShareRole::View,
            connected: false,
            panel: None,
            cancel: shutdown.child_token(),
            shutdown,
            task: None,
        })
    }

    pub async fn share(
        &mut self,
        backend: &LocalBackend,
        args: &Args,
        session: &str,
        role: ShareRole,
    ) -> Result<()> {
        ensure!(
            self.task.is_none(),
            "sharing is already active; use /remote stop first"
        );
        let origin = super::origin(args)?;
        let account = match self.account.clone() {
            Some(account) => account,
            None => RelayAccount::load(origin.clone(), &args.state_dir(), &self.shutdown).await?,
        };
        let storage = if args.memory_only {
            IdentityStorage::MemoryOnly
        } else {
            IdentityStorage::SecretStore
        };
        let identity =
            EndpointIdentity::load_or_create(&origin, &args.state_dir(), storage, &self.shutdown)
                .await?;
        let invite = Invitation::new(origin, identity.public_key(), role, super::now_ms()?)?;
        let body = invite.protected_text()?;
        backend
            .begin_share(
                session,
                invite.routing_id().into(),
                role,
                invite.share_expires_at_ms(),
            )
            .await?;
        self.cancel = self.shutdown.child_token();
        self.session = Some(session.into());
        self.role = role;
        self.connected = false;
        self.panel = Some(Panel {
            title: format!("Share with {role:?} access"),
            body: Zeroizing::new(format!(
                "Private invitation, valid for first pairing for five minutes:\n\n{}\n\nKeep it private. A native client or the account-protected web controller can use it.\nThe host must confirm the controller fingerprint. Enter or Esc hides this panel.\nIdentity storage: {:?}",
                body.as_str(),
                identity.storage()
            )),
            answer: None,
        });
        let backend = backend.clone();
        let cancel = self.cancel.clone();
        let (sender, notices) = mpsc::channel(4);
        let epoch = invite.routing_id().to_owned();
        let expires_at_ms = invite.share_expires_at_ms();
        let join = tokio::spawn(async move {
            let _ = transport::serve_host(
                account,
                identity,
                Enrollment::new(invite),
                backend,
                cancel,
                sender,
            )
            .await;
        });
        self.task = Some(HostTask {
            epoch,
            expires_at_ms,
            notices,
            join,
        });
        Ok(())
    }

    pub async fn poll(&mut self, app: &mut App, backend: &LocalBackend, now: u64) -> bool {
        let mut confirmation = false;
        if self
            .panel
            .as_ref()
            .and_then(|panel| panel.answer.as_ref())
            .is_some_and(oneshot::Sender::is_closed)
        {
            self.panel = None;
            app.dirty = true;
        }
        if let Some(task) = &self.task
            && now >= task.expires_at_ms
            && self.session.is_some()
        {
            backend.stop_share_if(&task.epoch).await;
            self.cancel.cancel();
            self.panel = None;
            self.session = None;
            self.connected = false;
            app.notice("Sharing expired. The local session continues.");
        }
        while let Some(notice) = self
            .task
            .as_mut()
            .and_then(|task| task.notices.try_recv().ok())
        {
            // Each receiver belongs to one share. Once it ends, delayed notices
            // cannot restore a panel or connection badge during cleanup.
            if self.session.is_none() {
                continue;
            }
            match notice {
                HostNotice::Confirm {
                    fingerprint,
                    answer,
                } => {
                    if answer.is_closed() {
                        continue;
                    }
                    self.panel = Some(Panel {
                        title: "Confirm controller identity".into(),
                        body: Zeroizing::new(format!(
                            "Controller fingerprint:\n\n{fingerprint}\n\nCompare it with the controller screen. Y grants {:?} access. N or Esc rejects.",
                            self.role
                        )),
                        answer: Some(answer),
                    });
                    confirmation = true;
                }
                HostNotice::Connected => {
                    self.connected = true;
                    self.panel = None;
                }
                HostNotice::Listening | HostNotice::Disconnected => self.connected = false,
                HostNotice::Failed => {
                    self.panel = None;
                    self.session = None;
                    self.connected = false;
                    app.error("Sharing ended. Relay access or encrypted transport failed. The local session continues.");
                }
            }
            app.dirty = true;
        }
        // A notice is not proof that the sender has finished. Retain the handle
        // until it can be joined before allowing another share.
        if self
            .task
            .as_ref()
            .is_some_and(|task| task.join.is_finished())
            && let Some(task) = self.task.take()
        {
            let result = task.join.await;
            backend.stop_share_if(&task.epoch).await;
            self.cancel.cancel();
            self.panel = None;
            self.connected = false;
            if self.session.take().is_some() {
                app.notice("Sharing ended. The local session continues.");
            }
            if result.is_err() {
                app.error("Sharing ended because its connection task failed. The local session continues.");
            }
            app.dirty = true;
        }
        app.remote = self.session.as_ref().map(|_| RemoteBadge {
            controller: false,
            label: format!(
                "Sharing {:?}: {}",
                self.role,
                if self.connected {
                    "controller connected"
                } else {
                    "waiting for controller"
                }
            ),
        });
        confirmation
    }

    pub fn draw(&self, frame: &mut Frame<'_>) {
        if let Some(panel) = &self.panel {
            super::protected::draw(frame, &panel.title, &panel.body);
        }
    }

    pub fn has_panel(&self) -> bool {
        self.panel.is_some()
    }

    pub fn handle_panel(&mut self, input: &Event) -> bool {
        let Some(panel) = &mut self.panel else {
            return false;
        };
        if let Event::Key(key) = input
            && key.kind == KeyEventKind::Press
        {
            let confirmation = match key.code {
                KeyCode::Char('y' | 'Y') if panel.answer.is_some() => Some(true),
                KeyCode::Char('n' | 'N') | KeyCode::Esc => Some(false),
                KeyCode::Enter if panel.answer.is_none() => Some(false),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(false),
                _ => None,
            };
            if let Some(confirmed) = confirmation {
                if let Some(answer) = panel.answer.take() {
                    let _ = answer.send(confirmed);
                }
                self.panel = None;
            }
        }
        true
    }

    pub fn is_command(input: &Event, app: &App) -> bool {
        matches!(input, Event::Key(key) if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter &&
            !key.modifiers.intersects(KeyModifiers::ALT | KeyModifiers::SHIFT))
            && app
                .composer
                .text
                .strip_prefix("/remote")
                .is_some_and(|tail| tail.is_empty() || tail.starts_with(char::is_whitespace))
    }

    pub async fn command(
        &mut self,
        text: &str,
        backend: &LocalBackend,
        args: &Args,
        session: &str,
    ) -> Result<Option<String>> {
        let parsed = crate::commands::parse(text.trim_start_matches('/'))?;
        ensure!(parsed.name == "remote", "invalid remote command");
        match parsed
            .value
            .split_whitespace()
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["share"] | ["share", "view"] => {
                self.share(backend, args, session, ShareRole::View).await?;
                Ok(None)
            }
            ["share", "control"] => {
                self.share(backend, args, session, ShareRole::Control)
                    .await?;
                Ok(None)
            }
            ["stop"] => {
                self.stop(backend).await;
                Ok(Some("Sharing stopped. Pending remote prompts were discarded. Already started work continues locally unless cancelled.".into()))
            }
            ["status"] => Ok(Some(if self.session.is_some() {
                format!(
                    "Sharing {:?}; controller {}. /remote stop revokes access.",
                    self.role,
                    if self.connected {
                        "connected"
                    } else {
                        "absent"
                    }
                )
            } else {
                "Sharing is off.".into()
            })),
            _ => {
                anyhow::bail!("Usage: /remote share [view|control], /remote status or /remote stop")
            }
        }
    }

    pub fn session_changed(&self, session: &str) -> bool {
        self.session
            .as_ref()
            .is_some_and(|current| current != session)
    }

    pub async fn settle(&self, backend: &LocalBackend, now: u64) -> Result<()> {
        if self.session.is_some() {
            backend.settle(now).await?;
        }
        Ok(())
    }

    pub async fn stop(&mut self, backend: &LocalBackend) {
        self.cancel.cancel();
        if let Some(task) = self.task.take() {
            backend.stop_share_if(&task.epoch).await;
            task.join.abort();
            let _ = task.join.await;
        }
        self.panel = None;
        self.session = None;
        self.connected = false;
    }
}

impl Drop for HostUi {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(task) = &self.task {
            task.join.abort();
        }
    }
}

#[cfg(test)]
#[path = "host_ui_tests.rs"]
mod tests;
