pub mod account;
pub mod backend;
pub mod channel;
pub mod controller;
pub mod host_ui;
pub mod identity;
pub mod invitation;
pub mod protected;
pub mod protocol;
mod secrets;
pub mod transport;
mod vault;

use crate::args::{Args, Command};
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use identity::{EndpointIdentity, IdentityStorage, RelayOrigin};
use invitation::{Invitation, ShareRole};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Subcommand)]
pub enum RemoteCommand {
    #[command(about = "Sign in to the configured relay with browser approval")]
    Login {
        #[arg(long)]
        no_browser: bool,
    },
    #[command(about = "List cached relay account metadata without contacting the service")]
    Accounts,
    #[command(about = "Remove the relay login and request server revocation")]
    Logout,
    #[command(
        about = "Migrate legacy relay secrets after stopping all older host/controller processes"
    )]
    MigrateCredentials,
    #[command(about = "List relay devices or revoke one")]
    Devices {
        #[arg(long, value_name = "DEVICE_ID")]
        revoke: Option<String>,
    },
    #[command(about = "Open the local TUI and share its selected session")]
    Host {
        #[arg(
            long,
            help = "Explicitly grant prompt, steer, queue and cancel control"
        )]
        control: bool,
    },
    #[command(about = "Read a private invitation in a protected terminal and connect")]
    Connect,
}

pub fn origin(args: &Args) -> Result<RelayOrigin> {
    RelayOrigin::parse(
        args.relay
            .as_deref()
            .context("set --relay https://YOUR-RELAY or EJM_CLI_RELAY_URL")?,
    )
}

pub fn host_role(args: &Args) -> Option<ShareRole> {
    match &args.command {
        Some(Command::Remote {
            command: RemoteCommand::Host { control },
        }) => Some(if *control {
            ShareRole::Control
        } else {
            ShareRole::View
        }),
        _ => None,
    }
}

pub fn now_ms() -> Result<u64> {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )
    .context("clock is outside the supported range")
}

pub async fn run(args: &Args, command: &RemoteCommand) -> Result<bool> {
    let cancel = CancellationToken::new();
    tokio::select! {
        biased;
        _ = tokio::signal::ctrl_c() => { cancel.cancel(); bail!("relay operation cancelled"); }
        result = execute(args, command, cancel.clone()) => result,
    }
}

async fn execute(args: &Args, command: &RemoteCommand, cancel: CancellationToken) -> Result<bool> {
    let state = args.state_dir();
    if matches!(command, RemoteCommand::Accounts) {
        println!(
            "{}",
            account::cached(&state, &cancel)
                .await?
                .unwrap_or_else(|| "No saved relay account.".into())
        );
        return Ok(true);
    }
    let origin = origin(args)?;
    match command {
        RemoteCommand::MigrateCredentials => {
            migrate_credentials(args.memory_only, state, origin, cancel).await?;
        }
        RemoteCommand::Login { no_browser } => {
            protected::ensure_terminal()?;
            let account =
                account::RelayAccount::login(origin, &state, args.memory_only, *no_browser, cancel)
                    .await?;
            println!(
                "Relay account: {}\nDevice: {}",
                crate::render::safe_text(&account.account_id),
                crate::render::safe_text(&account.device_id)
            );
            if args.memory_only {
                println!(
                    "Memory-only login ends with this process. Use remote host or connect --memory-only to sign in and use the relay in one process."
                );
            }
        }
        RemoteCommand::Logout => {
            let account = account::RelayAccount::load(origin, &state, &cancel).await?;
            if account.logout(&state, &cancel).await? {
                println!(
                    "Relay login removed and server revocation confirmed. Existing leases expire within 15 minutes."
                );
            } else {
                println!(
                    "Relay login removed. Server revocation is unconfirmed; use the account page to revoke the device."
                );
            }
        }
        RemoteCommand::Devices { revoke } => {
            let account = account::RelayAccount::load(origin, &state, &cancel).await?;
            let result = if let Some(id) = revoke {
                anyhow::ensure!(account::valid_id(id), "invalid device ID");
                account
                    .json(
                        reqwest::Method::POST,
                        &format!("/v1/devices/{id}/revoke"),
                        Some(serde_json::json!({})),
                    )
                    .await?
            } else {
                account
                    .json(reqwest::Method::GET, "/v1/devices", None)
                    .await?
            };
            println!(
                "{}",
                crate::render::safe_text(&serde_json::to_string_pretty(&result)?)
            );
        }
        RemoteCommand::Connect => {
            protected::ensure_terminal()?;
            let account = if args.memory_only {
                account::RelayAccount::login(origin.clone(), &state, true, false, cancel.clone())
                    .await?
            } else {
                account::RelayAccount::load(origin.clone(), &state, &cancel).await?
            };
            let storage = if args.memory_only {
                IdentityStorage::MemoryOnly
            } else {
                IdentityStorage::SecretStore
            };
            let identity =
                EndpointIdentity::load_or_create(&origin, &state, storage, &cancel).await?;
            let text = protected::read_invitation(cancel.clone()).await?;
            let invitation = Invitation::decode(&text, &origin, now_ms()?)?;
            drop(text);
            return controller::run(args, account, identity, invitation, cancel).await;
        }
        RemoteCommand::Accounts | RemoteCommand::Host { .. } => {
            bail!("invalid remote command composition")
        }
    }
    Ok(true)
}

async fn migrate_credentials(
    memory_only: bool,
    state: std::path::PathBuf,
    origin: RelayOrigin,
    cancel: CancellationToken,
) -> Result<()> {
    anyhow::ensure!(
        !memory_only,
        "credential migration requires persistent vault access"
    );
    let stopped = protected::confirm(
        "Stop every older ejm-cli host and controller that can write this relay's credentials. Confirm only after they have stopped. New vault locks cannot exclude old clients. Downgrade to an older keyring writer is unsupported after cutover. Continue with this controlled upgrade?",
        cancel.clone(),
    ).await?;
    anyhow::ensure!(
        stopped,
        "relay migration deferred; both stores remain unchanged; stop older clients and retry, or explicitly choose --memory-only"
    );
    let cleaned = vault::blocking(&cancel, move |cancel| {
        vault::migrate(&state, &origin, stopped, cancel)
    })
    .await?;
    if cleaned {
        println!(
            "Relay vault migration verified. Legacy entries were removed. Keep older clients stopped; direct downgrade is unsupported."
        );
    } else {
        println!(
            "Relay vault migration verified. Legacy cleanup is incomplete; the vault remains authoritative. Keep older clients stopped and retry remote migrate-credentials. Logout retains migration markers."
        );
    }
    Ok(())
}

#[cfg(test)]
mod test_support;
