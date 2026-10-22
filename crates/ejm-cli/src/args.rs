use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ProviderKind {
    #[default]
    Openai,
    Responses,
    Chatgpt,
    Claude,
    Anthropic,
}

/// Capacity policy. Automatic activation remains explicit until native crash recovery is verified.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum CapacityMode {
    Auto,
    #[default]
    Fixed,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CapacityCommand {
    #[command(about = "Reset local learned capacity without changing reservations or permissions")]
    Reset,
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
#[derive(Clone, Debug, Parser)]
#[command(
    name = "ejm-cli",
    version,
    about = "A fast, local coding agent built on the open ejm-harness",
    long_about = "A fast, local coding agent built on the open ejm-harness.\n\nStart without a command for the TUI. Tools are read only unless you grant\nwrite or shell access. Local use needs no relay account. Remote access requires\na relay account and local device pairing.",
    after_help = "Examples:\n  ejm-cli                                  Open the TUI and choose a provider\n  ejm-cli login chatgpt                    Sign in through your browser\n  ejm-cli login claude                     Sign in through the official Claude client\n  ejm-cli --provider claude                Use your Claude subscription\n  ejm-cli --provider chatgpt               Use your saved account in the TUI\n  ejm-cli run \"Find the parser\" --json      Stream durable JSON events\n  ejm-cli --session ID                     Resume a saved conversation\n\nProvider keys are read from the environment and never accepted as arguments.\nChatGPT credentials stay in the encrypted harness vault; the native OS service protects its key.\nClaude credentials stay with the official client.\nIn the TUI: type / to choose a command, /model to choose a model,\n/login to pick a sign-in provider, or /help for commands and keyboard shortcuts."
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(long, global = true, value_enum, default_value_t = ProviderKind::Openai, help = "Model provider; chatgpt uses saved sign-in, claude uses the official client's subscription")]
    pub provider: ProviderKind,

    #[arg(
        long,
        global = true,
        env = "EJM_CLI_CLAUDE_PATH",
        value_name = "PATH",
        help = "Official Claude Code executable (defaults to its native installation, then PATH)"
    )]
    pub claude_path: Option<PathBuf>,

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
        help = "Model identifier; Claude defaults to sonnet, ChatGPT to its account catalog, OpenAI compatible APIs to gpt-4.1-mini"
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

    #[arg(
        long,
        global = true,
        value_enum,
        help = "Shared host capacity mode (default: auto on macOS, legacy fixed elsewhere)"
    )]
    #[cfg_attr(target_os = "macos", arg(default_value = "auto"))]
    pub capacity: Option<CapacityMode>,

    #[arg(long, global = true, value_parser = positive_count, help = "Managed-session ceiling (fixed mode default: 100)")]
    pub max_sessions: Option<usize>,

    #[arg(long, global = true, value_parser = positive_count, help = "In-flight model-request ceiling (fixed mode default: 8)")]
    pub model_concurrency: Option<usize>,

    #[arg(long, global = true, value_parser = positive_count, help = "Executing local-tool ceiling (fixed mode default: 4)")]
    pub tool_concurrency: Option<usize>,

    #[arg(long, global = true, value_parser = clap::value_parser!(u64).range(1..=1_048_576), help = "Aggregate admitted tool-memory ceiling in MiB (fixed mode default: 512)")]
    pub tool_memory_mib: Option<u64>,

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
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    #[command(about = "Manage local learned capacity")]
    Capacity {
        #[command(subcommand)]
        command: CapacityCommand,
    },
    #[command(about = "Share or control one session through an end-to-end encrypted relay")]
    Remote {
        #[command(subcommand)]
        command: crate::remote::RemoteCommand,
    },
    #[command(aliases = ["packages", "pkg"], about = "Discover, install, trust, and publish ejm-harness packages")]
    Package {
        #[arg(
            long,
            global = true,
            env = "EJM_REGISTRY_URL",
            value_name = "ORIGIN",
            help = "Package registry origin; required for registry operations (no default)"
        )]
        registry: Option<String>,
        #[command(subcommand)]
        command: crate::packages::PackageCommand,
    },
    #[command(about = "Sign in to ChatGPT or the official Claude client, or show API-key guidance")]
    Login {
        #[arg(value_enum, value_name = "PROVIDER")]
        service: ProviderKind,
        #[arg(
            long,
            help = "For ChatGPT, print the safe sign-in URL instead of opening a browser"
        )]
        no_browser: bool,
    },
    #[command(about = "List saved ChatGPT accounts and safe official Claude client status")]
    Accounts {
        #[arg(long, help = "Print safe account metadata as JSON")]
        json: bool,
    },
    #[command(about = "Remove a saved ChatGPT account, or sign out of the official Claude client")]
    Logout {
        #[arg(value_name = "REGISTRATION|claude")]
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
    pub fn claude_program(&self) -> PathBuf {
        let configured = self.claude_path.as_ref().map(|path| {
            if path.is_relative()
                && path.components().count() > 1
                && let Ok(directory) = std::env::current_dir()
            {
                return directory.join(path);
            }
            path.clone()
        });
        ejm_harness_auth::resolve_claude_code_program(configured.as_deref())
    }

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
            max_jobs: self.tool_concurrency.unwrap_or(4),
            memory_bytes: self.tool_memory_mib.unwrap_or(512) * 1024 * 1024,
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

    pub fn provider_label(&self) -> &str {
        if let Some(provider) = &self.package_provider {
            provider
        } else {
            self.model.as_deref().unwrap_or(match self.provider {
                ProviderKind::Openai | ProviderKind::Responses => "gpt-4.1-mini",
                ProviderKind::Chatgpt => "ChatGPT account model",
                ProviderKind::Claude => "sonnet",
                ProviderKind::Anthropic => "Anthropic",
            })
        }
    }

    pub fn uses_builtin(&self, provider: ProviderKind) -> bool {
        self.provider == provider && self.package_provider.is_none()
    }
}

fn positive_count(value: &str) -> std::result::Result<usize, String> {
    value
        .parse::<std::num::NonZeroUsize>()
        .map(std::num::NonZeroUsize::get)
        .map_err(|_| "expected a positive integer".into())
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
    fn capacity_keeps_omitted_ceilings_distinct_from_fixed_defaults() {
        let fixed = Args::try_parse_from(["ejm-cli"]).unwrap();
        assert_eq!(
            fixed.capacity,
            if cfg!(target_os = "macos") {
                Some(CapacityMode::Auto)
            } else {
                None
            }
        );
        assert_eq!(fixed.max_sessions, None);
        assert_eq!(fixed.model_concurrency, None);
        assert_eq!(fixed.resources().max_jobs, 4);
        assert_eq!(fixed.resources().memory_bytes, 512 * 1024 * 1024);
        let automatic =
            Args::try_parse_from(["ejm-cli", "--capacity", "auto", "--model-concurrency", "3"])
                .unwrap();
        assert_eq!(automatic.capacity, Some(CapacityMode::Auto));
        assert_eq!(automatic.model_concurrency, Some(3));
        assert_eq!(automatic.tool_concurrency, None);
        let explicit = Args::try_parse_from([
            "ejm-cli",
            "--capacity",
            "fixed",
            "--max-sessions",
            "4096",
            "--tool-concurrency",
            "2",
            "--tool-memory-mib",
            "256",
        ])
        .unwrap();
        assert_eq!(explicit.max_sessions, Some(4096));
        assert_eq!(explicit.resources().max_jobs, 2);
        assert_eq!(explicit.resources().memory_bytes, 256 * 1024 * 1024);
        for option in [
            "--max-sessions",
            "--model-concurrency",
            "--tool-concurrency",
            "--tool-memory-mib",
        ] {
            assert!(Args::try_parse_from(["ejm-cli", option, "0"]).is_err());
        }
        assert!(Args::try_parse_from(["ejm-cli", "--capacity", "unknown"]).is_err());
    }

    #[test]
    fn capacity_reset_is_a_local_shell_command() {
        let args = Args::try_parse_from(["ejm-cli", "capacity", "reset"]).unwrap();
        assert!(matches!(
            args.command,
            Some(Command::Capacity {
                command: CapacityCommand::Reset
            })
        ));
        assert!(Args::try_parse_from(["ejm-cli", "capacity", "reset", "--all"]).is_err());
    }

    #[test]
    fn accepts_global_flags_after_command() {
        let args =
            Args::try_parse_from(["ejm-cli", "run", "Explain this", "--json", "--allow-write"])
                .unwrap();
        assert!(args.allow_write && !args.allow_shell);
        assert!(matches!(
            args.command,
            Some(Command::Run { json: true, .. })
        ));
    }

    #[test]
    fn grants_are_explicit_and_keys_cannot_be_passed_as_arguments() {
        let args = Args::try_parse_from(["ejm-cli"]).unwrap();
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
                },
                ..
            }) if grant_provider == ["offline"] && grant_widget == ["status"]
        ));
        assert!(
            Args::try_parse_from(["ejm-cli", "package", "enable", "@example/workflow"]).is_err()
        );
    }

    #[test]
    fn registry_migration_requires_explicit_old_writer_exclusion() {
        assert!(Args::try_parse_from(["ejm-cli", "package", "migrate-credentials"]).is_err());
        let args = Args::try_parse_from([
            "ejm-cli",
            "package",
            "migrate-credentials",
            "--old-clients-stopped",
        ])
        .unwrap();
        assert!(matches!(
            args.command,
            Some(Command::Package {
                command: crate::packages::PackageCommand::MigrateCredentials {
                    old_clients_stopped: true
                },
                ..
            })
        ));
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
}
