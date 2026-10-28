use std::{future::Future, sync::Arc};

use anyhow::{Context, Result, ensure};
use ejm_harness::{AvailableModel, Harness, Provider};
use ejm_harness_auth::{LogoutResult, RegistrationStatus};
use ejm_harness_exec::Executor;
use tokio::{sync::mpsc, task::JoinSet};
use tokio_util::sync::CancellationToken;

use crate::{
    app::{App, Overlay},
    args::{Args, ProviderKind},
    auth::{self, AccountSummary, LoginProgress},
    extensions::Extensions,
    models,
};

pub enum ConnectionReply {
    Connected {
        args: Box<Args>,
        provider: Arc<dyn Provider>,
        message: String,
    },
    Catalog(Vec<AvailableModel>),
    Accounts(AccountSummary),
    LoggedOut {
        registration: RegistrationStatus,
        result: LogoutResult,
    },
    ClaudeLoggedOut,
}

pub struct ClaudeLogin {
    pub draft: String,
    pub candidate: Args,
}

pub struct PendingCommand {
    pub draft: String,
    changing: bool,
    cancel: CancellationToken,
}

pub struct ConnectionTasks {
    pub jobs: JoinSet<Result<ConnectionReply>>,
    pub progress: mpsc::Receiver<LoginProgress>,
    pub pending: Option<PendingCommand>,
    pub external_login: Option<ClaudeLogin>,
    progress_sender: mpsc::Sender<LoginProgress>,
    executor: Executor,
}

impl ConnectionTasks {
    pub fn new(executor: Executor) -> Self {
        let (progress_sender, progress) = mpsc::channel(8);
        Self {
            jobs: JoinSet::new(),
            progress,
            pending: None,
            external_login: None,
            progress_sender,
            executor,
        }
    }

    pub fn progress_sender(&self) -> mpsc::Sender<LoginProgress> {
        self.progress_sender.clone()
    }

    pub fn changing(&self) -> bool {
        self.external_login.is_some()
            || self
                .pending
                .as_ref()
                .is_some_and(|command| command.changing)
    }

    pub fn spawn<F, Fut>(&mut self, draft: String, changing: bool, operation: F) -> Result<()>
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: Future<Output = Result<ConnectionReply>> + Send + 'static,
    {
        ensure!(
            self.pending.is_none() && self.external_login.is_none(),
            "an account or model command is running; Ctrl+C cancels it"
        );
        let cancel = CancellationToken::new();
        let operation = operation(cancel.clone());
        self.pending = Some(PendingCommand {
            draft,
            changing,
            cancel,
        });
        self.jobs.spawn(async move {
            // Auth operations can commit local credentials before revocation finishes.
            // Let them observe cancellation and report that committed change.
            tokio::time::timeout(std::time::Duration::from_mins(11), operation)
                .await
                .map_err(|_| anyhow::anyhow!("Command timed out; try again."))?
        });
        Ok(())
    }

    pub fn finish(
        &mut self,
        reply: Result<ConnectionReply>,
    ) -> (Option<PendingCommand>, Result<ConnectionReply>) {
        let pending = self.pending.take();
        let cancelled = pending
            .as_ref()
            .is_some_and(|command| command.cancel.is_cancelled());
        while self.progress.try_recv().is_ok() {}
        let reply = if cancelled
            && !matches!(
                &reply,
                Ok(ConnectionReply::LoggedOut { .. } | ConnectionReply::ClaudeLoggedOut)
            ) {
            Err(anyhow::anyhow!("Command cancelled."))
        } else {
            reply
        };
        (pending, reply)
    }

    pub fn cancel(&self) -> bool {
        if let Some(command) = &self.pending {
            command.cancel.cancel();
            true
        } else {
            false
        }
    }

    pub async fn shutdown(&mut self) {
        self.cancel();
        while self.jobs.join_next().await.is_some() {}
    }

    pub fn executor(&self) -> &Executor {
        &self.executor
    }
}

async fn require_idle(harness: &Harness) -> Result<()> {
    ensure!(
        harness.status().await.active_agents == 0,
        "wait for all managed runs to finish before changing the model or account"
    );
    Ok(())
}

pub async fn command(
    name: &str,
    value: &str,
    app: &mut App,
    harness: &Harness,
    args: &Args,
    extensions: &Extensions,
    tasks: &mut ConnectionTasks,
) -> Result<()> {
    let draft = if value.is_empty() {
        format!("/{name}")
    } else {
        format!("/{name} {value}")
    };
    match name {
        "model" if !value.is_empty() => {
            require_idle(harness).await?;
            let candidate = models::select(args, value)?;
            connect(tasks, draft, candidate, extensions.clone())?;
        }
        "model" | "models" => model_catalog(app, args, tasks, draft)?,
        "login" => {
            app.overlay = Some(Overlay::Login { selected: 0 });
            app.command_picker.dismissed = true;
        }
        "accounts" => {
            let args = args.clone();
            tasks.spawn(draft, false, move |cancel| async move {
                auth::account_summary(&args, cancel)
                    .await
                    .map(ConnectionReply::Accounts)
            })?;
        }
        "account" => {
            require_idle(harness).await?;
            let candidate = if value.eq_ignore_ascii_case("claude") {
                select_claude(args)?
            } else {
                select_account(args, value.into())
            };
            connect(tasks, draft, candidate, extensions.clone())?;
        }
        "logout" => {
            require_idle(harness).await?;
            let args = args.clone();
            let registration = value.to_owned();
            tasks.spawn(draft, true, move |cancel| async move {
                if registration.eq_ignore_ascii_case("claude") {
                    auth::claude_client(&args).logout(cancel).await?;
                    return Ok(ConnectionReply::ClaudeLoggedOut);
                }
                let (registration, result) = auth::revoke(&args, &registration, cancel).await?;
                Ok(ConnectionReply::LoggedOut {
                    registration,
                    result,
                })
            })?;
        }
        _ => {}
    }
    Ok(())
}

pub async fn login(
    service: ProviderKind,
    app: &mut App,
    harness: &Harness,
    args: &Args,
    extensions: &Extensions,
    tasks: &mut ConnectionTasks,
) -> Result<()> {
    if let Some(guidance) = auth::login_guidance(service) {
        app.notice(guidance);
        return Ok(());
    }
    require_idle(harness).await?;
    let draft = if app.composer.text.is_empty() {
        "/login".into()
    } else {
        app.composer.text.clone()
    };
    if service == ProviderKind::Claude {
        ensure!(
            tasks.pending.is_none() && tasks.external_login.is_none(),
            "an account or model command is running; Ctrl+C cancels it"
        );
        tasks.external_login = Some(ClaudeLogin {
            draft,
            candidate: select_claude(args)?,
        });
    } else {
        let mut candidate = args.clone();
        let progress = tasks.progress_sender();
        let extensions = extensions.clone();
        let executor = tasks.executor.clone();
        tasks.spawn(draft, true, move |cancel| async move {
            let registration = auth::login_interactive(&candidate, cancel.clone(), progress).await?;
            candidate = select_account(&candidate, registration.id.clone());
            let provider = models::prepare(&mut candidate, &extensions, &executor, cancel).await
                .context("sign-in was saved; could not select a model, use /model chatgpt MODEL to retry")?;
            let message = format!("Signed in: {}. Registration: {}.", auth::account_label(&registration), registration.id);
            Ok(ConnectionReply::Connected { args: Box::new(candidate), provider, message })
        })?;
        app.status = "Starting ChatGPT sign-in. Ctrl+C cancels.".into();
    }
    Ok(())
}

fn model_catalog(
    app: &mut App,
    args: &Args,
    tasks: &mut ConnectionTasks,
    draft: String,
) -> Result<()> {
    app.notice(format!(
        "Current model: {}. Select with /model MODEL or /model PROVIDER MODEL.",
        args.provider_label()
    ));
    if args.package_provider.is_some()
        || !matches!(
            args.provider,
            ProviderKind::Responses | ProviderKind::Chatgpt
        )
    {
        app.notice("Model catalogs are available for ChatGPT and native OpenAI Responses. This provider accepts an explicit model ID with /model.");
    } else {
        let args = args.clone();
        tasks.spawn(draft, false, move |cancel| async move {
            models::catalog(&args, cancel)
                .await
                .map(ConnectionReply::Catalog)
        })?;
        app.status = "Loading models…".into();
    }
    Ok(())
}

fn select_claude(args: &Args) -> Result<Args> {
    let model = if args.uses_builtin(ProviderKind::Claude) {
        args.model.as_deref().unwrap_or("sonnet")
    } else {
        "sonnet"
    };
    models::select(args, &format!("claude {model}"))
}

pub async fn complete_claude_login(
    mut request: ClaudeLogin,
    extensions: &Extensions,
    executor: &Executor,
    cancel: CancellationToken,
) -> Result<ConnectionReply> {
    let status = auth::claude_login(&request.candidate, cancel.child_token()).await?;
    let provider = models::prepare(&mut request.candidate, extensions, executor, cancel)
        .await
        .context(
            "Claude sign-in completed; could not select a model, use /account claude to retry",
        )?;
    Ok(ConnectionReply::Connected {
        args: Box::new(request.candidate),
        provider,
        message: auth::ClaudeAccount::Available { status }.label(),
    })
}

fn select_account(args: &Args, registration: String) -> Args {
    let mut selected = args.clone();
    if !args.uses_builtin(ProviderKind::Chatgpt) {
        selected.model = None;
    }
    selected.provider = ProviderKind::Chatgpt;
    selected.account = Some(registration);
    selected.base_url = None;
    selected.package_provider = None;
    selected
}

pub(crate) fn connect(
    tasks: &mut ConnectionTasks,
    draft: String,
    mut candidate: Args,
    extensions: Extensions,
) -> Result<()> {
    let executor = tasks.executor.clone();
    tasks.spawn(draft, true, move |cancel| async move {
        let provider = models::prepare(&mut candidate, &extensions, &executor, cancel).await?;
        let message = if candidate.uses_builtin(ProviderKind::Claude) {
            format!(
                "Using claude {} through the official client subscription.",
                candidate.provider_label()
            )
        } else {
            format!("Using model {}.", candidate.provider_label())
        };
        Ok(ConnectionReply::Connected {
            args: Box::new(candidate),
            provider,
            message,
        })
    })
}

pub async fn apply_reply(
    reply: Result<ConnectionReply>,
    app: &mut App,
    harness: &Harness,
    args: &mut Args,
    callbacks_idle: bool,
) -> Result<()> {
    match reply? {
        ConnectionReply::Connected {
            args: candidate,
            provider,
            message,
        } => {
            ensure!(
                callbacks_idle,
                "wait for extension callbacks before changing the model"
            );
            harness
                .configure_provider(Some(provider), &models::binding(&candidate)?)
                .await?;
            *args = *candidate;
            app.connection_ready = true;
            app.notice(message);
        }
        ConnectionReply::Catalog(models) => {
            ensure!(
                !models.is_empty(),
                "the provider returned no available models"
            );
            if !matches!(app.overlay, Some(Overlay::Login { .. })) {
                let selected = models
                    .iter()
                    .position(|model| Some(model.slug.as_str()) == args.model.as_deref())
                    .unwrap_or(0);
                app.overlay = Some(Overlay::Models { models, selected });
            }
        }
        ConnectionReply::Accounts(accounts) => {
            if accounts.chatgpt_unavailable {
                app.notice(auth::VAULT_RECOVERY);
            } else if accounts.chatgpt.is_empty() {
                app.notice("No saved accounts. Use /login to sign in to ChatGPT.");
            } else {
                app.notice(
                    accounts
                        .chatgpt
                        .iter()
                        .map(|account| {
                            format!(
                                "{}  {}  {}",
                                account.id,
                                if account.signed_in {
                                    "signed in"
                                } else {
                                    "signed out"
                                },
                                auth::account_label(account)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
            app.notice(accounts.claude.label());
        }
        ConnectionReply::LoggedOut {
            registration,
            result,
        } => {
            if args.uses_builtin(ProviderKind::Chatgpt)
                && args
                    .account
                    .as_ref()
                    .is_none_or(|id| registration.id.starts_with(id))
            {
                app.connection_ready = false;
                harness
                    .configure_provider(None, &models::binding(args)?)
                    .await?;
            }
            app.notice(format!("Removed saved registration {}.", registration.id));
            if !result.remote_revoked {
                app.notice("Provider revocation was not confirmed. Review connected apps in your provider account.");
            }
        }
        ConnectionReply::ClaudeLoggedOut => {
            if args.uses_builtin(ProviderKind::Claude) {
                app.connection_ready = false;
                harness
                    .configure_provider(None, &models::binding(args)?)
                    .await?;
            }
            app.notice("Signed out of Claude in the official client.");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tasks() -> Result<(tempfile::TempDir, ConnectionTasks)> {
        let state = tempfile::tempdir()?;
        let executor = Executor::new(
            Arc::new(ejm_harness_exec::Governor::new(
                ejm_harness_exec::ResourceLimits::default(),
            )?),
            state.path().join("artifacts"),
        )?;
        Ok((state, ConnectionTasks::new(executor)))
    }

    #[tokio::test]
    async fn cancelling_a_completed_command_prevents_its_reply_from_applying() -> Result<()> {
        let (_state, mut tasks) = tasks()?;
        tasks.spawn("/models".into(), false, |_| async {
            Ok(ConnectionReply::Catalog(Vec::new()))
        })?;
        let reply = tasks.jobs.join_next().await.context("missing task")??;
        ensure!(
            tasks.cancel(),
            "completed command lost its cancellation token"
        );

        let (pending, reply) = tasks.finish(reply);
        ensure!(
            pending.context("missing command draft")?.draft == "/models",
            "cancelled command lost its draft"
        );
        ensure!(
            reply
                .err()
                .context("cancelled reply was accepted")?
                .to_string()
                == "Command cancelled.",
            "cancellation was not reported"
        );
        ensure!(!tasks.cancel(), "finished command remained pending");
        Ok(())
    }

    #[tokio::test]
    async fn cancelled_logout_reports_its_committed_local_change() -> Result<()> {
        let (_state, mut tasks) = tasks()?;
        let (committed, committed_receiver) = tokio::sync::oneshot::channel();
        tasks.spawn(
            "/logout fixture-account".into(),
            true,
            |cancel| async move {
                committed
                    .send(())
                    .map_err(|()| anyhow::anyhow!("commit observer closed"))?;
                cancel.cancelled().await;
                Ok(ConnectionReply::LoggedOut {
                    registration: RegistrationStatus {
                        id: "fixture-account".into(),
                        issuer: "https://auth.openai.com".into(),
                        subject: "fixture-subject".into(),
                        client_id: "fixture-client".into(),
                        email: None,
                        signed_in: true,
                        plan_enabled: true,
                        expires_at: None,
                    },
                    result: LogoutResult {
                        remote_revoked: false,
                    },
                })
            },
        )?;
        tokio::time::timeout(std::time::Duration::from_secs(1), committed_receiver).await??;
        ensure!(
            tasks.cancel(),
            "logout was not pending after the local commit"
        );
        let reply = tokio::time::timeout(std::time::Duration::from_secs(1), tasks.jobs.join_next())
            .await?
            .context("missing logout task")??;

        let (pending, reply) = tasks.finish(reply);
        ensure!(
            pending.context("missing logout draft")?.draft == "/logout fixture-account",
            "logout lost its command draft"
        );
        match reply? {
            ConnectionReply::LoggedOut {
                registration,
                result,
            } => {
                ensure!(
                    registration.id == "fixture-account",
                    "wrong account was logged out"
                );
                ensure!(
                    !result.remote_revoked,
                    "remote revocation was incorrectly confirmed"
                );
            }
            _ => anyhow::bail!("cancelled logout lost its committed change"),
        }
        ensure!(
            !tasks.changing(),
            "completed logout still blocked connection changes"
        );
        Ok(())
    }

    #[tokio::test]
    async fn cancelled_claude_logout_keeps_the_committed_sign_out() -> Result<()> {
        let (_state, mut tasks) = tasks()?;
        tasks.spawn("/logout claude".into(), true, |_| async {
            Ok(ConnectionReply::ClaudeLoggedOut)
        })?;
        let reply = tasks
            .jobs
            .join_next()
            .await
            .context("missing logout task")??;
        ensure!(
            tasks.cancel(),
            "logout lost cancellation before application"
        );
        let (_, reply) = tasks.finish(reply);
        ensure!(
            matches!(reply?, ConnectionReply::ClaudeLoggedOut),
            "committed sign-out was hidden by cancellation"
        );
        Ok(())
    }

    #[tokio::test]
    async fn finishing_a_command_discards_its_queued_progress() -> Result<()> {
        let (_state, mut tasks) = tasks()?;
        let progress = tasks.progress_sender();
        tasks.spawn("/login".into(), true, |_| async move {
            progress.try_send(LoginProgress::Starting)?;
            Ok(ConnectionReply::Catalog(Vec::new()))
        })?;
        let reply = tasks
            .jobs
            .join_next()
            .await
            .context("missing login task")??;
        let (_, reply) = tasks.finish(reply);
        reply?;
        ensure!(
            tasks.progress.try_recv().is_err(),
            "stale sign-in progress survived completion"
        );
        Ok(())
    }
}
