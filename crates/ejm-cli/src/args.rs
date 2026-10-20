use std::path::PathBuf;

use anyhow::{Context, Result, bail};

pub(crate) const INVALID_BASE_URL: &str =
    "invalid provider base URL. Change --base-url or EJM_CLI_BASE_URL.";
pub(crate) const CLEARTEXT_BASE_URL: &str = "provider base URL must use https, or http on a loopback host (localhost, 127.0.0.0/8, or ::1). Change --base-url or EJM_CLI_BASE_URL.";

pub(crate) fn proxy_block_message(variable: &str) -> String {
    format!(
        "http provider URLs on loopback cannot be used while {variable} is set unless NO_PROXY covers the host. Change --base-url or EJM_CLI_BASE_URL."
    )
}

/// Result of checking the provider base URL the user would actually use.
#[derive(Debug)]
pub(crate) enum EndpointCheck<'a> {
    Allowed(&'a str),
    Rejected(String),
}
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
    long_about = "A fast, local coding agent built on the open ejm-harness.\n\nStart without a command for the TUI. Tools are read only unless you grant\nwrite or shell access. No account or server connection is needed.",
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
        help = "API base URL for OpenAI compatible or Anthropic API-key access. Use https, or http on localhost, 127.0.0.0/8, or ::1. HTTP on loopback is refused while HTTP_PROXY, http_proxy, ALL_PROXY, or all_proxy is set unless NO_PROXY covers that host. ChatGPT and Responses use a fixed official endpoint"
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
}

#[derive(Debug, Subcommand)]
pub enum Command {
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

    fn effective_base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .unwrap_or(if self.provider == ProviderKind::Anthropic {
                "https://api.anthropic.com/v1"
            } else {
                "https://api.openai.com/v1"
            })
    }

    /// The provider base URL after the https-or-loopback check.
    ///
    /// # Errors
    ///
    /// Returns an error when the effective URL is not `https` and is not `http`
    /// on a loopback host, when that loopback URL would be sent through a proxy,
    /// or when it contains userinfo, a query, or a fragment. The error names
    /// `--base-url` and `EJM_CLI_BASE_URL` and does not include the supplied URL
    /// or a proxy value.
    pub fn api_base_url(&self) -> Result<&str> {
        match self.endpoint_check()? {
            EndpointCheck::Allowed(url) => Ok(url),
            EndpointCheck::Rejected(_) => bail!("{CLEARTEXT_BASE_URL}"),
        }
    }

    pub(crate) fn endpoint_check(&self) -> Result<EndpointCheck<'_>> {
        self.endpoint_check_with(&crate::endpoint::ProxySnapshot::from_env())
    }

    pub(crate) fn endpoint_check_with(
        &self,
        proxies: &crate::endpoint::ProxySnapshot,
    ) -> Result<EndpointCheck<'_>> {
        let url = self.effective_base_url();
        match crate::endpoint::require_https_or_loopback(url) {
            Ok(()) => {
                if let Some(variable) = crate::endpoint::blocking_proxy_variable(url, proxies) {
                    bail!("{}", proxy_block_message(variable));
                }
                Ok(EndpointCheck::Allowed(url))
            }
            Err(crate::endpoint::HttpsLoopbackError::Invalid) => bail!("{INVALID_BASE_URL}"),
            Err(crate::endpoint::HttpsLoopbackError::Cleartext) => Ok(EndpointCheck::Rejected(
                crate::endpoint::display_without_userinfo(url),
            )),
        }
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

    fn parsed(args: &[&str]) -> Args {
        Args::try_parse_from(args).unwrap()
    }

    fn refusal(url: &str) -> String {
        match parsed(&["ejm-cli", "--base-url", url])
            .endpoint_check_with(&crate::endpoint::ProxySnapshot::none())
        {
            Ok(EndpointCheck::Allowed(value)) => format!("allowed {value}"),
            Ok(EndpointCheck::Rejected(_)) => CLEARTEXT_BASE_URL.to_owned(),
            Err(error) => error.to_string(),
        }
    }

    fn allows(url: &str) -> bool {
        matches!(
            parsed(&["ejm-cli", "--base-url", url])
                .endpoint_check_with(&crate::endpoint::ProxySnapshot::none()),
            Ok(EndpointCheck::Allowed(value)) if value == url
        )
    }

    #[test]
    fn api_base_url_accepts_https_and_loopback_only() {
        assert_eq!(
            parsed(&["ejm-cli"]).api_base_url().unwrap(),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            parsed(&["ejm-cli", "--provider", "anthropic"])
                .api_base_url()
                .unwrap(),
            "https://api.anthropic.com/v1"
        );
        for url in [
            "https://api.deepseek.com/v1",
            "http://localhost:11434/v1",
            "http://LOCALHOST/v1",
            "http://127.0.0.1:9/v1",
            "http://127.0.0.2/v1",
            "http://[::1]:9/v1",
            "http://[0:0:0:0:0:0:0:1]/v1",
            "http://localhost\\@evil.invalid/v1",
        ] {
            assert!(allows(url), "{url}");
        }
        for url in [
            "http://api.example.invalid/v1",
            "http://localhost.evil.invalid/v1",
            "http://127.0.0.1.nip.invalid/v1",
            "http://foo.localhost/v1",
            "http://192.168.1.10:11434/v1",
            "http://0.0.0.0/v1",
            "http://10.1.2.3/v1",
            "http://172.16.0.1/v1",
            "http://169.254.169.254/v1",
            "http://[fe80::1]/v1",
            "http://[fc00::1]/v1",
            "http://[::]",
            "http://[::ffff:127.0.0.1]/v1",
            "http://evil.invalid\\@localhost/v1",
        ] {
            let text = refusal(url);
            assert!(text.contains("must use https"), "{url}: {text}");
            assert!(
                text.contains("--base-url") && text.contains("EJM_CLI_BASE_URL"),
                "{text}"
            );
            assert!(!text.contains(url), "{text}");
            assert!(!text.contains("secret"), "{text}");
        }
        for url in [
            "ftp://files.example.invalid/v1",
            "file:///tmp/a",
            "ws://api.example.invalid/v1",
            "api.example.invalid/v1",
            "https://user:secret@api.example.invalid/v1",
            "http://user:secret@localhost/v1",
            "http://evil.invalid@localhost/v1",
            "https://api.example.invalid/v1?key=sk-secret",
        ] {
            let text = refusal(url);
            assert!(text.contains("invalid provider base URL"), "{url}: {text}");
            assert!(!text.contains("must use https"), "{url}: {text}");
            assert!(!text.contains("secret"), "{text}");
            assert!(!text.contains("sk-secret"), "{text}");
            assert!(!text.contains(url), "{text}");
        }
        let empty = refusal("");
        assert!(empty.contains("invalid provider base URL"), "{empty}");
        assert!(!empty.contains("must use https"), "{empty}");
        let blocked = parsed(&["ejm-cli", "--base-url", "http://127.0.0.1/v1"])
            .endpoint_check_with(&crate::endpoint::ProxySnapshot {
                http_proxy_variable: Some("HTTP_PROXY"),
                no_proxy: None,
            })
            .unwrap_err()
            .to_string();
        assert!(blocked.contains("HTTP_PROXY"), "{blocked}");
        assert!(blocked.contains("NO_PROXY"), "{blocked}");
        assert!(!blocked.contains("http://"), "{blocked}");
    }
}
