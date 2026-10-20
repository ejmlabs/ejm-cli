use anyhow::{Context, Result};
use clap::{Arg, Command as ClapCommand, builder::StringValueParser};

#[derive(Clone)]
pub struct CommandEntry {
    pub name: String,
    pub arguments: String,
    pub description: String,
}

impl CommandEntry {
    pub fn usage(&self) -> String {
        if self.arguments.is_empty() {
            format!("/{}", self.name)
        } else {
            format!("/{} {}", self.name, self.arguments)
        }
    }

    pub fn completion(&self) -> String {
        format!(
            "/{}{}",
            self.name,
            if self.arguments.is_empty() { "" } else { " " }
        )
    }
}

struct Builtin {
    name: &'static str,
    arguments: &'static str,
    description: &'static str,
    minimum: usize,
    maximum: usize,
}

const BUILTINS: &[Builtin] = &[
    Builtin {
        name: "remote",
        arguments: "share [view|control]|status|stop",
        description: "Share this session, inspect sharing or stop remote access",
        minimum: 1,
        maximum: 2,
    },
    Builtin {
        name: "help",
        arguments: "",
        description: "Show commands and keyboard shortcuts",
        minimum: 0,
        maximum: 0,
    },
    Builtin {
        name: "status",
        arguments: "",
        description: "Show the connection, grants and concurrency",
        minimum: 0,
        maximum: 0,
    },
    Builtin {
        name: "new",
        arguments: "[TITLE]",
        description: "Create a session in this workspace",
        minimum: 0,
        maximum: usize::MAX,
    },
    Builtin {
        name: "sessions",
        arguments: "",
        description: "Choose a saved session",
        minimum: 0,
        maximum: 0,
    },
    Builtin {
        name: "agents",
        arguments: "",
        description: "Choose among managed sessions",
        minimum: 0,
        maximum: 0,
    },
    Builtin {
        name: "resume",
        arguments: "ID",
        description: "Open a session by its ID or unique prefix",
        minimum: 1,
        maximum: 1,
    },
    Builtin {
        name: "queue",
        arguments: "MESSAGE",
        description: "Queue a follow-up turn, or start when idle",
        minimum: 1,
        maximum: usize::MAX,
    },
    Builtin {
        name: "steer",
        arguments: "MESSAGE",
        description: "Send direction to the active run",
        minimum: 1,
        maximum: usize::MAX,
    },
    Builtin {
        name: "stop",
        arguments: "",
        description: "Cancel work in the selected session",
        minimum: 0,
        maximum: 0,
    },
    Builtin {
        name: "extensions",
        arguments: "",
        description: "Show enabled packages and their grants",
        minimum: 0,
        maximum: 0,
    },
    Builtin {
        name: "widgets",
        arguments: "pause|resume",
        description: "Pause or resume package widgets",
        minimum: 1,
        maximum: 1,
    },
    Builtin {
        name: "theme",
        arguments: "NAME",
        description: "Select a theme from an enabled package",
        minimum: 1,
        maximum: 1,
    },
    Builtin {
        name: "prompt",
        arguments: "NAME [INPUT]",
        description: "Submit a package prompt and optional input",
        minimum: 1,
        maximum: usize::MAX,
    },
    Builtin {
        name: "skill",
        arguments: "NAME [INPUT]",
        description: "Submit a package skill and optional input",
        minimum: 1,
        maximum: usize::MAX,
    },
    Builtin {
        name: "reload",
        arguments: "@publisher/package",
        description: "Reload an idle native package",
        minimum: 1,
        maximum: 1,
    },
    Builtin {
        name: "quit",
        arguments: "",
        description: "Stop managed runs and exit",
        minimum: 0,
        maximum: 0,
    },
    Builtin {
        name: "exit",
        arguments: "",
        description: "Stop managed runs and exit",
        minimum: 0,
        maximum: 0,
    },
];

pub fn catalog(package_commands: &[(String, String)]) -> Vec<CommandEntry> {
    let mut entries = BUILTINS
        .iter()
        .map(|command| CommandEntry {
            name: command.name.into(),
            arguments: command.arguments.into(),
            description: command.description.into(),
        })
        .collect::<Vec<_>>();
    for (qualified, description) in package_commands {
        let short = qualified.rsplit(':').next().unwrap_or(qualified);
        let unique = !BUILTINS.iter().any(|builtin| builtin.name == short)
            && package_commands
                .iter()
                .filter(|(name, _)| name.rsplit(':').next() == Some(short))
                .count()
                == 1;
        entries.push(CommandEntry {
            name: if unique {
                short.into()
            } else {
                qualified.clone()
            },
            arguments: "[INPUT]".into(),
            description: crate::render::safe_text(description).replace('\n', " "),
        });
    }
    entries
}

pub struct ParsedCommand<'a> {
    pub name: String,
    pub value: &'a str,
}

fn builtin_parser(command: &Builtin) -> ClapCommand {
    let mut parser = ClapCommand::new(command.name)
        .disable_help_flag(true)
        .disable_version_flag(true);
    if command.maximum > 0 {
        let mut argument = Arg::new("arguments")
            .num_args(command.minimum..=command.maximum)
            .required(command.minimum > 0)
            .allow_hyphen_values(true);
        if command.name == "widgets" {
            argument = argument.value_parser(["pause", "resume"]);
        }
        parser = parser.arg(argument);
    }
    parser
}

pub fn parse(input: &str) -> Result<ParsedCommand<'_>> {
    let parser = ClapCommand::new("slash")
        .no_binary_name(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .subcommand_required(true)
        .allow_external_subcommands(true)
        .external_subcommand_value_parser(StringValueParser::new())
        .subcommands(BUILTINS.iter().map(builtin_parser));
    let matches = parser
        .try_get_matches_from(input.split_whitespace())
        .map_err(|_| {
            let name = input.split_whitespace().next().unwrap_or("help");
            let arguments = BUILTINS
                .iter()
                .find(|command| command.name == name)
                .map_or("", |command| command.arguments);
            anyhow::anyhow!("Usage: /{name} {arguments}")
        })?;
    let (name, _) = matches.subcommand().context("Usage: /help")?;
    let value = input
        .trim_start()
        .strip_prefix(name)
        .unwrap_or("")
        .trim_start();
    let value = if BUILTINS
        .iter()
        .find(|command| command.name == name)
        .is_none_or(|command| command.maximum == usize::MAX)
    {
        value
    } else {
        value.trim_end()
    };
    Ok(ParsedCommand {
        name: name.into(),
        value,
    })
}

#[derive(Default)]
pub struct CommandPicker {
    pub selected: usize,
    pub dismissed: bool,
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Invalid offline command fixtures must fail the test immediately."
)]
mod tests {
    use super::*;

    #[test]
    fn reports_builtin_usage_instead_of_package_resolution_errors() {
        for (name, value) in [("resume", ""), ("stop", "now"), ("widgets", "other")] {
            assert!(parse(&format!("{name} {value}")).is_err());
        }
        assert!(parse("queue a follow-up with words").is_ok());
    }

    #[test]
    fn clap_parsing_preserves_free_form_text_and_package_input() {
        for name in ["queue", "steer", "@one/tools:hello"] {
            let input = format!("{name} don't change  --help\nnext line\n ");
            let parsed = parse(&input).unwrap();
            assert_eq!(parsed.name, name);
            assert_eq!(parsed.value, "don't change  --help\nnext line\n ");
        }
    }

    #[test]
    fn package_collisions_keep_builtins_and_use_qualified_names() {
        let entries = catalog(&[
            ("@one/tools:help".into(), "Package help".into()),
            ("@one/tools:hello".into(), "First greeting".into()),
            ("@two/tools:hello".into(), "Second greeting".into()),
            ("@one/tools:unique".into(), "A unique command".into()),
        ]);
        assert!(entries.iter().any(|entry| entry.name == "help"));
        assert!(entries.iter().any(|entry| entry.name == "@one/tools:help"));
        assert!(entries.iter().any(|entry| entry.name == "@two/tools:hello"));
        assert!(entries.iter().any(|entry| entry.name == "unique"));
        assert!(!entries.iter().any(|entry| entry.name == "hello"));
    }
}
