use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use clap::Subcommand;
use ejm_harness_exec::Governor;
use ejm_harness_packages::{
    PackageGrants, PackageManager, RegistryClient, delete_registry_token, install_source,
    load_registry_token, migrate_registry_token, pack_directory, save_registry_token,
};
use tokio_util::sync::CancellationToken;

use crate::{
    args::{
        Args, DISCARD_RETIRED_REGISTRY_COMMAND, RegistryChoice, RegistrySource,
        display_registry_url, host_is_meetgerry, registry_override_error, retired_registry_message,
    },
    render::safe_text,
};

#[derive(Debug, Subcommand)]
pub enum PackageCommand {
    Search {
        query: String,
    },
    Info {
        name: String,
        version: String,
    },
    Install {
        source: String,
    },
    Update {
        name: String,
        version: String,
    },
    List,
    Enable {
        name: String,
        #[arg(
            long,
            required = true,
            help = "Trust this exact version as native code running as you"
        )]
        trust: bool,
        #[arg(long, value_name = "EXPORT")]
        tool: Vec<String>,
        #[arg(long, value_name = "EVENT", value_parser = ["run_start", "before_model", "before_tool", "after_tool", "run_end"])]
        hook: Vec<String>,
        #[arg(long, value_name = "EXPORT")]
        grant_provider: Vec<String>,
        #[arg(long, value_name = "EXPORT")]
        command: Vec<String>,
        #[arg(long, value_name = "EXPORT")]
        grant_widget: Vec<String>,
    },
    Disable {
        name: String,
    },
    Remove {
        name: String,
    },
    Pack {
        #[arg(default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Login {
        #[arg(long)]
        no_browser: bool,
    },
    MigrateCredentials {
        #[arg(
            long,
            required = true,
            help = "Confirm every older CLI and IDE registry credential writer has stopped"
        )]
        old_clients_stopped: bool,
    },
    Logout,
    Whoami,
    Publisher {
        handle: String,
        #[arg(long)]
        display_name: String,
    },
    Publish {
        #[arg(default_value = ".")]
        directory: PathBuf,
    },
    /// Unlink a legacy registry-session.json. Sends nothing. Does not delete a directory.
    DiscardRetiredSession,
}

pub fn manager(args: &Args) -> Result<PackageManager> {
    PackageManager::new(args.state_dir().join("packages"))
}

pub async fn run(args: &Args, command: &PackageCommand) -> Result<bool> {
    let manager = manager(args)?;
    if matches!(command, PackageCommand::DiscardRetiredSession) {
        discard_retired_session(manager.root())?;
        return Ok(true);
    }
    let registry = open_registry(args)?;
    dispatch(args, &manager, &registry, command).await?;
    Ok(true)
}

fn open_registry(args: &Args) -> Result<RegistryClient> {
    let choice = args.registry_choice();
    registry_override_error(&choice)?;
    let RegistryChoice::Selected { origin, source } = choice else {
        bail!("registry origin was rejected");
    };
    if source != RegistrySource::Default {
        println!(
            "Registry: {} ({})",
            safe_text(&display_registry_url(&origin)),
            source.label()
        );
    }
    RegistryClient::new(&origin)
}

async fn dispatch(
    args: &Args,
    manager: &PackageManager,
    registry: &RegistryClient,
    command: &PackageCommand,
) -> Result<()> {
    match command {
        PackageCommand::Search { query } => show(&registry.search(query, None).await?)?,
        PackageCommand::Info { name, version } => show(&registry.version(name, version).await?)?,
        PackageCommand::Install { source } => install(args, manager, registry, source).await?,
        PackageCommand::Update { name, version } => {
            ensure!(
                manager.list()?.iter().any(|record| &record.name == name),
                "package is not installed"
            );
            install(args, manager, registry, &format!("{name}@{version}")).await?;
        }
        PackageCommand::List => show(&manager.list()?)?,
        PackageCommand::Enable {
            name,
            trust,
            tool,
            hook,
            grant_provider,
            command,
            grant_widget,
        } => {
            ensure!(*trust, "--trust is required for native extensions");
            let hooks = hook
                .iter()
                .map(|name| serde_json::from_value(serde_json::Value::String(name.clone())))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let record = manager.enable(
                name,
                PackageGrants {
                    tools: tool.clone(),
                    hooks,
                    providers: grant_provider.clone(),
                    commands: command.clone(),
                    widgets: grant_widget.clone(),
                },
            )?;
            show(&record)?;
            println!(
                "Trusted for future sessions. Native packages run as your user; export grants control registration."
            );
        }
        PackageCommand::Disable { name } => {
            manager.disable(name)?;
            println!("Disabled for future sessions. Restart an active CLI to unload it.");
        }
        PackageCommand::Remove { name } => {
            manager.remove(name)?;
            println!("Removed the registration. Active sessions retain their snapshot until exit.");
        }
        PackageCommand::Pack { directory, output } => {
            use std::io::Write;
            let bytes = pack_directory(directory)?;
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output)
                .context("create new package archive")?;
            file.write_all(&bytes)?;
            println!(
                "{} bytes: {}",
                bytes.len(),
                safe_text(&output.display().to_string())
            );
        }
        PackageCommand::Login { no_browser } => login(manager, registry, *no_browser).await?,
        PackageCommand::MigrateCredentials {
            old_clients_stopped,
        } => migrate_credentials(manager, registry, *old_clients_stopped)?,
        PackageCommand::Logout => logout(manager, registry).await?,
        PackageCommand::Whoami => {
            show(
                &registry
                    .me(&require_session(manager, registry.origin())?)
                    .await?,
            )?;
        }
        PackageCommand::Publisher {
            handle,
            display_name,
        } => show(
            &registry
                .claim_publisher(
                    handle,
                    display_name,
                    &require_session(manager, registry.origin())?,
                )
                .await?,
        )?,
        PackageCommand::Publish { directory } => {
            let bytes = pack_directory(directory)?;
            let token = require_session(manager, registry.origin())?;
            show(&registry.publish(bytes, &token).await?)?;
        }
        PackageCommand::DiscardRetiredSession => {}
    }
    Ok(())
}

fn finish_logout(revoked: Result<()>, clear: impl FnOnce() -> Result<()>) -> Result<()> {
    clear()?;
    revoked.context("local session removed; remote revocation failed")
}

async fn logout(manager: &PackageManager, registry: &RegistryClient) -> Result<()> {
    let token = require_session(manager, registry.origin())?;
    let revoked = registry.logout(&token).await;
    finish_logout(revoked, || {
        delete_registry_token(manager.root(), registry.origin())
    })?;
    println!("Registry session revoked and removed.");
    Ok(())
}

fn migrate_credentials(
    manager: &PackageManager,
    registry: &RegistryClient,
    old_clients_stopped: bool,
) -> Result<()> {
    let retired = migrate_registry_token(manager.root(), registry.origin(), old_clients_stopped)?;
    println!(
        "{}",
        if retired {
            "Legacy registry session retired."
        } else {
            "No legacy registry session found."
        }
    );
    Ok(())
}

async fn install(
    args: &Args,
    manager: &PackageManager,
    registry: &RegistryClient,
    source: &str,
) -> Result<()> {
    let governor = Arc::new(Governor::new(args.resources())?);
    let record = install_source(
        manager,
        source,
        registry,
        &governor,
        CancellationToken::new(),
    )
    .await?;
    show(&record)?;
    println!(
        "Installed and disabled. Inspect it, then enable the exact version with --trust and named export grants."
    );
    Ok(())
}

async fn login(
    manager: &PackageManager,
    registry: &RegistryClient,
    no_browser: bool,
) -> Result<()> {
    if let Some(message) = retired_registry_session_notice(manager.root())? {
        bail!("{message}");
    }
    let code = registry.device_code().await?;
    println!(
        "Open {} and enter {}",
        safe_text(&code.verification_uri),
        safe_text(&code.user_code)
    );
    println!(
        "Approve only the ejm-cli request you just started. This grants a revocable registry account session."
    );
    if !no_browser {
        crate::browser::launch(
            ejm_harness_auth::SecretString::new(&code.verification_uri),
            CancellationToken::new(),
        )
        .await?;
    }
    let token = registry
        .wait_for_login(&code, CancellationToken::new())
        .await?;
    save_registry_token(manager.root(), registry.origin(), &token)?;
    println!("Signed in to the package registry.");
    Ok(())
}

fn show(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", safe_text(&serde_json::to_string_pretty(value)?));
    Ok(())
}

const LEGACY_SESSION: &str = "registry-session.json";
const MAX_LEGACY_SESSION: u64 = 8192;

fn require_session(
    manager: &PackageManager,
    origin: &str,
) -> Result<ejm_harness_auth::SecretString> {
    // Vault records are origin-scoped. A leftover retired or unusable legacy
    // path must not block revoke/use of a different origin's vault session.
    // Only when the vault has no record for this origin do we surface the
    // legacy notice (so meetgerry leftovers are not mislabeled as migrate).
    match load_registry_token(manager.root(), origin) {
        Ok(token) => Ok(token),
        Err(error) => {
            if let Some(message) = retired_registry_session_notice(manager.root())? {
                bail!("{message}");
            }
            Err(error).with_context(|| {
                format!("no registry session for {}", display_registry_url(origin))
            })
        }
    }
}

fn discard_retired_session(root: &std::path::Path) -> Result<()> {
    match classify_legacy(root)? {
        LegacyPath::Missing => println!("No leftover registry session file."),
        LegacyPath::Session { retired: false } => {
            println!("Saved registry session is not for a meetgerry.ai host. Left it in place.");
        }
        LegacyPath::Directory => bail!("{}", directory_session_message(&session_path(root))),
        LegacyPath::Session { retired: true } | LegacyPath::Remove => {
            unlink_legacy(root)?;
            println!("Discarded the legacy registry session path. No request was sent.");
        }
    }
    Ok(())
}

/// Local equivalent of `ejm_harness_packages::retired_registry_session_notice`.
///
/// The pinned harness does not export that function. Registry session commands
/// and `doctor` call this instead. The check uses `lstat`, does not follow a
/// symlink, and does not return a token. Usable tokens come only from the vault.
///
/// # Errors
///
/// Returns an error when the legacy path cannot be inspected.
pub fn retired_registry_session_notice(packages_root: &std::path::Path) -> Result<Option<String>> {
    match classify_legacy(packages_root)? {
        LegacyPath::Session { retired: true } => Ok(Some(retired_registry_message())),
        LegacyPath::Missing | LegacyPath::Session { retired: false } => Ok(None),
        LegacyPath::Remove | LegacyPath::Directory => unusable_session_message(packages_root),
    }
}

fn unusable_session_message(root: &std::path::Path) -> Result<Option<String>> {
    let path = session_path(root);
    Ok(match classify_legacy(root)? {
        LegacyPath::Remove => Some(unsafe_session_message(&path)),
        LegacyPath::Directory => Some(directory_session_message(&path)),
        LegacyPath::Missing | LegacyPath::Session { .. } => None,
    })
}

fn session_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(LEGACY_SESSION)
}

fn unsafe_session_message(path: &std::path::Path) -> String {
    format!(
        "{} cannot be used. Delete it with `{DISCARD_RETIRED_REGISTRY_COMMAND}`. Nothing was sent.",
        path.display()
    )
}

fn directory_session_message(path: &std::path::Path) -> String {
    format!(
        "{} is a directory. Remove that directory yourself. ejm-cli does not delete directories.",
        path.display()
    )
}

enum LegacyPath {
    Missing,
    Session { retired: bool },
    Remove,
    Directory,
}

fn classify_legacy(root: &std::path::Path) -> Result<LegacyPath> {
    let path = session_path(root);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LegacyPath::Missing);
        }
        Err(error) => return Err(error).context("inspect registry session"),
    };
    let kind = metadata.file_type();
    if kind.is_symlink() {
        return Ok(LegacyPath::Remove);
    }
    if kind.is_dir() {
        return Ok(LegacyPath::Directory);
    }
    if !kind.is_file() || !legacy_mode_is_private(&metadata) || metadata.len() > MAX_LEGACY_SESSION
    {
        return Ok(LegacyPath::Remove);
    }
    match parsed_origin(&path) {
        Ok(origin) if url::Url::parse(&origin).is_ok() => Ok(LegacyPath::Session {
            retired: origin_is_retired(&origin),
        }),
        _ => Ok(LegacyPath::Remove),
    }
}

fn legacy_mode_is_private(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode().trailing_zeros() >= 6
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        true
    }
}

fn parsed_origin(path: &std::path::Path) -> Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).context("read registry session")?;
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1024];
    let max = usize::try_from(MAX_LEGACY_SESSION).unwrap_or(usize::MAX);
    loop {
        let read = file.read(&mut buf).context("read registry session")?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) > max {
            bail!("registry session exceeds byte limit");
        }
        bytes.extend_from_slice(&buf[..read]);
    }
    let session: LegacySessionFile =
        serde_json::from_slice(&bytes).context("invalid registry session")?;
    drop(session.token);
    Ok(session.origin)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySessionFile {
    origin: String,
    token: String,
}

fn unlink_legacy(root: &std::path::Path) -> Result<()> {
    let path = session_path(root);
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_dir() => {
            bail!("{}", directory_session_message(&path))
        }
        Ok(_) => std::fs::remove_file(&path).context("remove retired registry session"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("inspect registry session"),
    }
}

fn origin_is_retired(origin: &str) -> bool {
    url::Url::parse(origin)
        .ok()
        .and_then(|url| url.host().map(|host| host_is_meetgerry(&host)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, ffi::OsStr};

    use clap::Parser;

    use super::*;
    use crate::args::Command;

    #[test]
    fn failed_revocation_still_clears_local_session() {
        let cleared = Cell::new(false);
        let result = finish_logout(Err(anyhow::anyhow!("fixture revocation failure")), || {
            cleared.set(true);
            Ok(())
        });
        assert!(cleared.get());
        assert!(result.is_err());
    }

    #[test]
    fn local_clear_failure_prevents_logout_success() {
        let result = finish_logout(Ok(()), || Err(anyhow::anyhow!("fixture storage failure")));
        assert!(result.is_err());
    }

    #[test]
    fn registry_migration_requires_confirmation_before_storage() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let state = directory.path().join("state");
        let result = Args::try_parse_from([
            OsStr::new("ejm-cli"),
            OsStr::new("--state-dir"),
            state.as_os_str(),
            OsStr::new("package"),
            OsStr::new("migrate-credentials"),
        ]);
        assert!(result.is_err());
        assert!(!state.exists());
        Ok(())
    }

    #[test]
    fn registry_migration_accepts_explicit_confirmation() -> Result<()> {
        let args = Args::try_parse_from([
            "ejm-cli",
            "package",
            "migrate-credentials",
            "--old-clients-stopped",
        ])?;
        assert!(matches!(
            args.command,
            Some(Command::Package {
                command: PackageCommand::MigrateCredentials {
                    old_clients_stopped: true
                }
            })
        ));
        Ok(())
    }
}
