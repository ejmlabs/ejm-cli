use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ProviderKind {
    #[default]
    Openai,
    Responses,
    Chatgpt,
    Anthropic,
}

/// Terminal background the TUI palette is tuned for.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum Appearance {
    /// Read `COLORFGBG`, then assume a dark terminal.
    #[default]
    Auto,
    Light,
    Dark,
}

impl From<Appearance> for crate::ejm_tui::Appearance {
    fn from(value: Appearance) -> Self {
        match value {
            Appearance::Auto => Self::Auto,
            Appearance::Light => Self::Light,
            Appearance::Dark => Self::Dark,
        }
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "CLI switches represent independent grants, provider selection and presentation settings."
)]
#[derive(Debug, Parser)]
#[command(
    name = "ejm-cli",
    version,
    about = "A fast, local coding agent built on the open ejm-harness",
    long_about = "A fast, local coding agent built on the open ejm-harness.\n\nStart without a command for the TUI. Tools are read only unless you grant\nwrite or shell access. Local use needs no relay account. Remote access requires\na relay account and local device pairing.",
    after_help = "Examples:\n  ejm-cli --replay                         Try the TUI offline\n  ejm-cli login chatgpt                    Sign in through your browser\n  ejm-cli --provider chatgpt               Use your saved account in the TUI\n  ejm-cli run \"Find the parser\" --json      Stream durable JSON events\n  ejm-cli --session ID                     Resume a saved conversation\n\nProvider keys are read from the environment and never accepted as arguments.\nSaved account credentials stay in the private local auth store.\nIn the TUI: /help shows commands and keyboard shortcuts."
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(
        long,
        global = true,
        help = "Use the deterministic offline echo provider"
    )]
    pub replay: bool,

    #[arg(
        long,
        global = true,
        conflicts_with = "replay",
        value_name = "JSON",
        help = "Read deterministic model responses from a JSON array"
    )]
    pub replay_file: Option<PathBuf>,

    #[arg(long, global = true, value_enum, default_value_t = ProviderKind::Openai, help = "Model provider; ChatGPT uses a saved browser sign-in")]
    pub provider: ProviderKind,

    #[arg(
        long,
        global = true,
        value_name = "REGISTRATION",
        help = "Saved ChatGPT account registration ID or unique prefix"
    )]
    pub account: Option<String>,

    #[arg(
        long,
        global = true,
        env = "EJM_CLI_BASE_URL",
        help = "API base URL for OpenAI compatible or Anthropic API-key access; ChatGPT uses its fixed official endpoint"
    )]
    pub base_url: Option<String>,

    #[arg(
        long,
        global = true,
        env = "EJM_CLI_MODEL",
        help = "Model identifier; ChatGPT defaults to its account catalog, OpenAI compatible APIs to gpt-4.1-mini"
    )]
    pub model: Option<String>,

    #[arg(long, global = true, value_name = "LEVEL", value_parser = clap::builder::PossibleValuesParser::new(ejm_harness::REASONING_EFFORTS.iter().copied()), help = "Reasoning effort for native Responses providers")]
    pub reasoning_effort: Option<String>,

    #[arg(
        long,
        global = true,
        default_value = "EJM_CLI_API_KEY",
        value_name = "NAME",
        help = "Environment variable holding the API key (falls back to the provider's standard variable)"
    )]
    pub api_key_env: String,

    #[arg(
        long,
        global = true,
        value_name = "DIRECTORY",
        help = "Workspace directory (defaults to the current directory)"
    )]
    pub cwd: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        env = "EJM_CLI_STATE_DIR",
        value_name = "DIRECTORY",
        help = "Durable state directory (defaults to the platform user data directory)"
    )]
    pub state_dir: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        value_name = "ID",
        help = "Resume a saved session"
    )]
    pub session: Option<String>,

    #[arg(
        long,
        global = true,
        env = "EJM_CLI_RELAY_URL",
        help = "Explicit HTTPS relay origin for remote access"
    )]
    pub relay: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Keep relay identity and login in memory for this foreground process"
    )]
    pub memory_only: bool,

    #[arg(
        long,
        global = true,
        help = "Grant native file write tools for this process"
    )]
    pub allow_write: bool,

    #[arg(
        long,
        global = true,
        help = "Grant shell execution as the current user"
    )]
    pub allow_shell: bool,

    #[arg(long, global = true, default_value_t = 100, value_parser = clap::value_parser!(usize), help = "Maximum managed sessions")]
    pub max_sessions: usize,

    #[arg(long, global = true, default_value_t = 8, value_parser = clap::value_parser!(usize), help = "Maximum in-flight model requests")]
    pub model_concurrency: usize,

    #[arg(
        long,
        global = true,
        default_value_t = 4,
        help = "Maximum executing local tools across all sessions"
    )]
    pub tool_concurrency: usize,

    #[arg(long, global = true, default_value_t = 512, value_parser = clap::value_parser!(u64).range(1..=1_048_576), help = "Aggregate local tool memory admission budget in MiB")]
    pub tool_memory_mib: u64,

    #[arg(long, global = true, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=256), help = "Maximum model steps in one run")]
    pub max_steps: u32,

    #[arg(long, global = true, default_value_t = 600_000, value_parser = clap::value_parser!(u64).range(1..=86_400_000), help = "Maximum duration of one run in milliseconds")]
    pub run_timeout_ms: u64,

    #[arg(
        long,
        global = true,
        help = "Use a monochrome TUI (also honors NO_COLOR)"
    )]
    pub no_color: bool,

    #[arg(
        long,
        global = true,
        help = "Use ASCII symbols and borders (also used when the locale is not UTF-8)"
    )]
    pub plain: bool,

    #[arg(
        long,
        global = true,
        env = "EJM_CLI_APPEARANCE",
        value_enum,
        default_value_t = Appearance::Auto,
        help = "Palette for a light or dark terminal background"
    )]
    pub appearance: Appearance,

    #[arg(
        long,
        global = true,
        help = "Use an explicitly granted package provider: @publisher/package:provider"
    )]
    pub package_provider: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Theme asset from an enabled package: @publisher/package:theme"
    )]
    pub theme: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Show a granted package widget: @publisher/package:widget (at most four)"
    )]
    pub widget: Vec<String>,

    #[arg(
        long,
        global = true,
        value_name = "URL",
        help = "Package registry origin for package commands and doctor. Overrides EJM_CLI_REGISTRY_URL. Does not change model endpoints. Requires https, or loopback http with HTTP_PROXY, HTTPS_PROXY and ALL_PROXY unset or empty (uppercase and lowercase). NO_PROXY does not bypass this restriction"
    )]
    pub registry: Option<String>,
}

/// Default package registry. Everett approved this host. Do not read
/// `DEFAULT_REGISTRY` while that harness constant is still `harness.meetgerry.ai`.
pub const DEFAULT_PACKAGE_REGISTRY: &str = "https://registry.ejmlabs.com";

/// Matches `ejm_harness_packages::DISCARD_RETIRED_REGISTRY_COMMAND` at harness
/// commit `6c90a0d`. That export is not in the pinned harness, so the CLI owns
/// the same string until the pin moves.
pub const DISCARD_RETIRED_REGISTRY_COMMAND: &str = "ejm-cli package discard-retired-session";

/// Shown when `doctor` cannot parse a registry value. Never the raw input.
pub const INVALID_REGISTRY_URL: &str = "(invalid registry URL)";

/// User-facing retired-registry error. Same wording as the harness message at `6c90a0d`.
#[must_use]
pub fn retired_registry_message() -> String {
    format!(
        "the meetgerry.ai registry is retired; the default registry is {DEFAULT_PACKAGE_REGISTRY}. The old session is not sent or reused. Discard it with `{DISCARD_RETIRED_REGISTRY_COMMAND}` and sign in again"
    )
}

/// Where the registry origin in effect came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistrySource {
    Default,
    Environment,
    Flag,
}

impl RegistrySource {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Environment => "EJM_CLI_REGISTRY_URL",
            Self::Flag => "--registry",
        }
    }
}

/// Registry origin selected for package commands and `doctor`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryChoice {
    Selected {
        origin: String,
        source: RegistrySource,
    },
    Rejected {
        raw: String,
        source: RegistrySource,
        message: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Share or control one session through an end-to-end encrypted relay")]
    Remote {
        #[command(subcommand)]
        command: crate::remote::RemoteCommand,
    },
    #[command(aliases = ["packages", "pkg"], about = "Discover, install, trust, and publish ejm-harness packages")]
    Package {
        #[command(subcommand)]
        command: crate::packages::PackageCommand,
    },
    #[command(about = "Sign in to ChatGPT in your browser, or show provider sign-in guidance")]
    Login {
        #[arg(value_enum, value_name = "PROVIDER")]
        service: ProviderKind,
        #[arg(long, help = "Print the safe sign-in URL instead of opening a browser")]
        no_browser: bool,
    },
    #[command(about = "List saved account registrations without showing credentials")]
    Accounts {
        #[arg(long, help = "Print safe account metadata as JSON")]
        json: bool,
    },
    #[command(about = "Remove a saved account and attempt provider revocation")]
    Logout {
        #[arg(value_name = "REGISTRATION")]
        registration: String,
    },
    #[command(about = "List available models for ChatGPT or OpenAI Responses access")]
    Models {
        #[arg(long, help = "Print the provider model catalog as JSON")]
        json: bool,
    },
    #[command(about = "Run one turn and stream its answer")]
    Run {
        #[arg(required = true, num_args = 1.., value_name = "PROMPT")]
        prompt: Vec<String>,
        #[arg(long, help = "Write one durable event per JSON line")]
        json: bool,
    },
    #[command(about = "List saved sessions")]
    Sessions {
        #[arg(long, help = "Print sessions as JSON")]
        json: bool,
    },
    #[command(about = "Show local configuration, resource limits and executor capabilities")]
    Doctor,
}

impl Args {
    pub fn api_key(&self) -> Option<String> {
        std::env::var(&self.api_key_env)
            .ok()
            .filter(|key| !key.trim().is_empty())
            .or_else(|| {
                (self.api_key_env == "EJM_CLI_API_KEY")
                    .then(|| std::env::var(self.fallback_key_env()).ok())
                    .flatten()
                    .filter(|key| !key.trim().is_empty())
            })
    }

    pub fn fallback_key_env(&self) -> &'static str {
        if self.provider == ProviderKind::Anthropic {
            "ANTHROPIC_API_KEY"
        } else {
            "OPENAI_API_KEY"
        }
    }

    pub fn api_base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .unwrap_or(if self.provider == ProviderKind::Anthropic {
                "https://api.anthropic.com/v1"
            } else {
                "https://api.openai.com/v1"
            })
    }

    pub fn cwd(&self) -> Result<PathBuf> {
        let path = match &self.cwd {
            Some(path) => path.clone(),
            None => std::env::current_dir()?,
        };
        let path = path.canonicalize().context("resolve workspace directory")?;
        anyhow::ensure!(path.is_dir(), "workspace must be a directory");
        Ok(path)
    }

    pub fn state_dir(&self) -> PathBuf {
        if let Some(path) = &self.state_dir {
            return path.clone();
        }
        if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(path).join("ejm-cli");
        }
        if cfg!(target_os = "windows")
            && let Some(path) = std::env::var_os("LOCALAPPDATA")
        {
            return PathBuf::from(path).join("ejm-cli");
        }
        let home = std::env::var_os("HOME").map_or_else(std::env::temp_dir, PathBuf::from);
        if cfg!(target_os = "macos") {
            home.join("Library/Application Support/ejm-cli")
        } else {
            home.join(".local/share/ejm-cli")
        }
    }

    pub fn resources(&self) -> ejm_harness_exec::ResourceLimits {
        ejm_harness_exec::ResourceLimits {
            max_jobs: self.tool_concurrency,
            memory_bytes: self.tool_memory_mib * 1024 * 1024,
            ..ejm_harness_exec::ResourceLimits::default()
        }
    }

    pub fn grants(&self) -> String {
        let mut grants = vec!["read"];
        if self.allow_write {
            grants.push("write");
        }
        if self.allow_shell {
            grants.push("shell (current user)");
        }
        grants.join(" + ")
    }

    /// Resolve `--registry`, then `EJM_CLI_REGISTRY_URL`, then [`DEFAULT_PACKAGE_REGISTRY`].
    ///
    /// A rejected URL or proxy configuration is [`RegistryChoice::Rejected`]
    /// so `doctor` can print it.
    #[must_use]
    pub fn registry_choice(&self) -> RegistryChoice {
        let env = std::env::var("EJM_CLI_REGISTRY_URL")
            .ok()
            .filter(|value| !value.is_empty());
        let choice = registry_choice_from(self.registry.as_deref(), env.as_deref());
        // The pinned harness predates direct HTTP registry transport. Remove
        // this adapter after adopting and verifying that public contract.
        if let RegistryChoice::Selected { origin, source } = &choice
            && origin.starts_with("http://")
            && let Some(name) = [
                "HTTP_PROXY",
                "http_proxy",
                "HTTPS_PROXY",
                "https_proxy",
                "ALL_PROXY",
                "all_proxy",
            ]
            .into_iter()
            .find(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
        {
            return RegistryChoice::Rejected {
                raw: origin.clone(),
                source: *source,
                message: format!(
                    "HTTP loopback registry access is blocked while {name} is set. Unset proxy variables for this invocation or use an HTTPS registry. NO_PROXY does not override this restriction on the current harness pin"
                ),
            };
        }
        choice
    }

    pub fn provider_label(&self) -> &str {
        if let Some(provider) = &self.package_provider {
            provider
        } else if self.replay || self.replay_file.is_some() {
            "offline replay"
        } else {
            self.model.as_deref().unwrap_or(match self.provider {
                ProviderKind::Openai | ProviderKind::Responses => "gpt-4.1-mini",
                ProviderKind::Chatgpt => "ChatGPT account model",
                ProviderKind::Anthropic => "Anthropic",
            })
        }
    }
}

#[must_use]
pub fn registry_choice_from(flag: Option<&str>, env: Option<&str>) -> RegistryChoice {
    let (raw, source) = if let Some(flag) = flag {
        (flag, RegistrySource::Flag)
    } else if let Some(env) = env.filter(|value| !value.is_empty()) {
        (env, RegistrySource::Environment)
    } else {
        (DEFAULT_PACKAGE_REGISTRY, RegistrySource::Default)
    };
    match validate_registry_origin(raw) {
        Ok(origin) => RegistryChoice::Selected { origin, source },
        Err(message) => RegistryChoice::Rejected {
            raw: raw.to_owned(),
            source,
            message,
        },
    }
}

/// Loopback host for cleartext HTTP. Shared with provider base URL checks:
/// `localhost`, `127.0.0.0/8`, or `::1`. DNS is not consulted.
#[must_use]
pub fn host_is_loopback(host: &url::Host<&str>) -> bool {
    match host {
        url::Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
        url::Host::Ipv4(address) => address.is_loopback(),
        url::Host::Ipv6(address) => address.is_loopback(),
    }
}

pub(crate) fn host_is_meetgerry(host: &url::Host<&str>) -> bool {
    let url::Host::Domain(domain) = host else {
        return false;
    };
    let name = domain.trim_end_matches('.').to_ascii_lowercase();
    name == "meetgerry.ai" || name.ends_with(".meetgerry.ai")
}

fn validate_registry_origin(raw: &str) -> Result<String, String> {
    let url = url::Url::parse(raw).map_err(|_| "invalid registry URL".to_owned())?;
    let Some(host) = url.host() else {
        return Err("invalid registry URL".to_owned());
    };
    if host_is_meetgerry(&host) {
        return Err(retired_registry_message());
    }
    let loopback = host_is_loopback(&host);
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        if url.scheme() == "http" {
            return Err(
                "registry URL must use https, or http on a loopback host (localhost, 127.0.0.0/8, or ::1)"
                    .to_owned(),
            );
        }
        return Err("invalid registry URL".to_owned());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err("a registry must be a plain origin".to_owned());
    }
    let mut origin = url;
    origin.set_path("/");
    origin.set_query(None);
    origin.set_fragment(None);
    Ok(origin.to_string())
}

/// Printable registry URL for both `doctor` registry lines.
///
/// This follows the shape of `display_without_userinfo` on ejm-cli pull request
/// 11: scheme, host, port, and path, with userinfo, query, and fragment removed.
/// Control characters are stripped. A value that does not parse is a fixed
/// placeholder, never the raw input.
#[must_use]
pub fn display_registry_url(value: &str) -> String {
    let Ok(url) = url::Url::parse(value) else {
        return INVALID_REGISTRY_URL.to_owned();
    };
    let Some(host) = url.host_str() else {
        return INVALID_REGISTRY_URL.to_owned();
    };
    let port = match url.port() {
        Some(port) => format!(":{port}"),
        None => String::new(),
    };
    let path = match url.path() {
        "" | "/" => "",
        path => path,
    };
    format!("{}://{host}{port}{path}", url.scheme())
        .chars()
        .filter(|character| !character.is_control())
        .collect()
}

pub fn registry_override_error(choice: &RegistryChoice) -> Result<()> {
    let RegistryChoice::Rejected {
        source, message, ..
    } = choice
    else {
        return Ok(());
    };
    let hint = match source {
        RegistrySource::Flag | RegistrySource::Environment => {
            "Change --registry or EJM_CLI_REGISTRY_URL."
        }
        RegistrySource::Default => "The built-in registry origin is invalid.",
    };
    bail!("{message}. {hint}");
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Fixture arguments must parse or fail this isolated test."
)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn accepts_global_flags_after_command() {
        let args = Args::try_parse_from([
            "ejm-cli",
            "run",
            "Explain this",
            "--json",
            "--replay",
            "--allow-write",
        ])
        .unwrap();
        assert!(args.replay && args.allow_write && !args.allow_shell);
        assert!(matches!(
            args.command,
            Some(Command::Run { json: true, .. })
        ));
    }

    #[test]
    fn grants_are_explicit_and_keys_cannot_be_passed_as_arguments() {
        let args = Args::try_parse_from(["ejm-cli", "--replay"]).unwrap();
        assert_eq!(args.grants(), "read");
        assert!(Args::try_parse_from(["ejm-cli", "--api-key", "secret"]).is_err());
        assert!(Args::try_parse_from(["ejm-cli", "run"]).is_err());
    }

    #[test]
    fn package_grants_do_not_shadow_global_provider_or_widget_options() {
        let args = Args::try_parse_from([
            "ejm-cli",
            "--provider",
            "responses",
            "--widget",
            "@example/host:status",
            "package",
            "enable",
            "@example/workflow",
            "--trust",
            "--grant-provider",
            "offline",
            "--grant-widget",
            "status",
        ])
        .unwrap();
        assert_eq!(args.provider, ProviderKind::Responses);
        assert_eq!(args.widget, ["@example/host:status"]);
        assert!(matches!(
            args.command,
            Some(Command::Package {
                command: crate::packages::PackageCommand::Enable {
                    trust: true, grant_provider, grant_widget, ..
                }
            }) if grant_provider == ["offline"] && grant_widget == ["status"]
        ));
        assert!(
            Args::try_parse_from(["ejm-cli", "package", "enable", "@example/workflow"]).is_err()
        );
    }

    #[test]
    fn chatgpt_login_and_account_selection_do_not_accept_credentials() {
        let args = Args::try_parse_from([
            "ejm-cli",
            "login",
            "chatgpt",
            "--no-browser",
            "--account",
            "registration",
        ])
        .unwrap();
        assert!(matches!(
            args.command,
            Some(Command::Login {
                service: ProviderKind::Chatgpt,
                no_browser: true
            })
        ));
        assert_eq!(args.account.as_deref(), Some("registration"));
        assert!(
            Args::try_parse_from(["ejm-cli", "login", "chatgpt", "--access-token", "secret"])
                .is_err()
        );
        assert!(Args::try_parse_from(["ejm-cli", "login", "chatgpt", "--code", "secret"]).is_err());
    }

    #[test]
    fn provider_defaults_preserve_compatibility_without_selecting_a_chatgpt_model() {
        let compat = Args::try_parse_from(["ejm-cli"]).unwrap();
        assert_eq!(compat.provider, ProviderKind::Openai);
        let chatgpt = Args::try_parse_from(["ejm-cli", "--provider", "chatgpt"]).unwrap();
        assert_eq!(chatgpt.provider, ProviderKind::Chatgpt);
        let command = Args::command();
        for name in ["model", "base_url"] {
            assert_eq!(
                command
                    .get_arguments()
                    .find(|arg| arg.get_id() == name)
                    .unwrap()
                    .get_default_values(),
                &[] as &[clap::builder::OsStr]
            );
        }
        let anthropic = Args::try_parse_from(["ejm-cli", "--provider", "anthropic"]).unwrap();
        assert_eq!(anthropic.fallback_key_env(), "ANTHROPIC_API_KEY");
        assert!(
            Args::try_parse_from([
                "ejm-cli",
                "--provider",
                "responses",
                "--reasoning-effort",
                "unknown"
            ])
            .is_err()
        );
    }

    #[test]
    fn registry_precedence_is_flag_then_environment_then_default() {
        let default = registry_choice_from(None, None);
        assert_eq!(
            default,
            RegistryChoice::Selected {
                origin: "https://registry.ejmlabs.com/".to_owned(),
                source: RegistrySource::Default,
            }
        );
        let env = registry_choice_from(None, Some("http://127.0.0.1:8787"));
        assert!(matches!(
            env,
            RegistryChoice::Selected {
                source: RegistrySource::Environment,
                ..
            }
        ));
        let flag = registry_choice_from(Some("http://localhost:9"), Some("http://127.0.0.1:8787"));
        assert!(matches!(
            flag,
            RegistryChoice::Selected {
                ref origin,
                source: RegistrySource::Flag,
            } if origin == "http://localhost:9/"
        ));
        let args =
            Args::try_parse_from(["ejm-cli", "--registry", "http://127.0.0.2:8787", "doctor"])
                .unwrap();
        assert!(matches!(
            registry_choice_from(args.registry.as_deref(), None),
            RegistryChoice::Selected {
                source: RegistrySource::Flag,
                ..
            }
        ));
    }

    #[test]
    fn registry_rules_accept_loopback_and_reject_meetgerry_lan_and_paths() {
        for raw in [
            "https://registry.ejmlabs.com",
            "https://notmeetgerry.ai",
            "http://localhost:8787",
            "http://LOCALHOST:8787",
            "http://127.0.0.1:8787",
            "http://127.0.0.2:8787",
            "http://[::1]:8787",
        ] {
            assert!(
                matches!(
                    registry_choice_from(Some(raw), None),
                    RegistryChoice::Selected { .. }
                ),
                "{raw}"
            );
        }
        let retired = registry_choice_from(None, Some("https://other.meetgerry.ai"));
        assert!(matches!(
            &retired,
            RegistryChoice::Rejected { message, .. }
                if message.contains("meetgerry.ai registry is retired")
                    && message.contains(DEFAULT_PACKAGE_REGISTRY)
                    && message.contains(DISCARD_RETIRED_REGISTRY_COMMAND)
        ));
        for raw in [
            "https://harness.meetgerry.ai",
            "https://meetgerry.ai",
            "http://registry.example.com",
            "http://0.0.0.0:8787",
            "http://192.168.1.10:8787",
            "http://[::]:8787",
            "http://[::ffff:127.0.0.1]:8787",
            "https://ejmlabs.com/registry",
            "https://user:pw@registry.ejmlabs.com",
            "ftp://registry.ejmlabs.com",
        ] {
            assert!(
                matches!(
                    registry_choice_from(Some(raw), None),
                    RegistryChoice::Rejected { .. }
                ),
                "{raw}"
            );
        }
        let error = registry_override_error(&registry_choice_from(
            Some("http://registry.example.com"),
            None,
        ))
        .unwrap_err()
        .to_string();
        assert!(error.contains("https"));
        assert!(error.contains("--registry"));
        assert!(error.contains("EJM_CLI_REGISTRY_URL"));
    }

    #[test]
    fn loopback_predicate_matches_the_provider_rule() {
        for (raw, expected) in [
            ("http://127.0.0.2/", true),
            ("http://[::1]/", true),
            ("http://0.0.0.0/", false),
            ("http://192.168.0.1/", false),
            ("http://[::ffff:127.0.0.1]/", false),
        ] {
            let url = url::Url::parse(raw).unwrap();
            let host = url.host().unwrap();
            assert_eq!(host_is_loopback(&host), expected, "{raw}");
        }
    }

    #[test]
    fn display_registry_url_drops_secrets_and_rejects_raw_input() {
        let shown = display_registry_url("https://user:secret@registry.example/index?q=1#frag");
        assert_eq!(shown, "https://registry.example/index");
        assert!(!shown.contains("secret"));
        assert!(!shown.contains('?'));
        assert!(!shown.contains('#'));
        let forged = "https://evil.example\nRegistry: https://registry.ejmlabs.com (default)";
        assert_eq!(display_registry_url(forged), INVALID_REGISTRY_URL);
        assert_eq!(display_registry_url("not a url"), INVALID_REGISTRY_URL);
        assert!(!display_registry_url(forged).contains('\n'));
        assert_eq!(
            display_registry_url("https://registry.ejmlabs.com/"),
            "https://registry.ejmlabs.com"
        );
    }
}
