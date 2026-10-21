//! Native official-client stand-in. It only reads its isolated fixture directory.
use std::{
    fs::{self, OpenOptions},
    io::{self, IsTerminal, Read, Write},
    path::Path,
    process::ExitCode,
};

use anyhow::{Context, Result};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("native client fixture failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<u8> {
    let executable = std::env::current_exe()?;
    let directory = executable.parent().context("fixture directory missing")?;
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut log = OpenOptions::new()
        .append(true)
        .create(true)
        .open(directory.join("invocations.jsonl"))?;
    serde_json::to_writer(&mut log, &arguments)?;
    writeln!(log)?;
    let signed_in = directory.join("signed-in");
    match arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["auth", "status", "--json"] => {
            if directory.join("status-wait").exists() {
                fs::write(directory.join("status-pid"), std::process::id().to_string())?;
                while !directory.join("status-release").exists() {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            if directory.join("status.json").exists() {
                println!("{}", fs::read_to_string(directory.join("status.json"))?);
                return Ok(fs::read_to_string(directory.join("status-exit"))?
                    .trim()
                    .parse()?);
            }
            println!(
                "{}",
                serde_json::json!({
                    "loggedIn": signed_in.exists(),
                    "authMethod": if signed_in.exists() { "claude.ai" } else { "none" },
                    "email": if signed_in.exists() { Some("terminal-fixture@example.test") } else { None },
                    "subscriptionType": if signed_in.exists() { Some("pro") } else { None },
                    "accessToken": "CLAUDE_FIXTURE_PRIVATE_STATUS",
                })
            );
            Ok(u8::from(!signed_in.exists()))
        }
        ["auth", "logout"] => {
            if signed_in.exists() {
                fs::remove_file(signed_in)?;
            }
            Ok(0)
        }
        ["auth", "login", "--claudeai"] => login(directory, &signed_in),
        _ => Ok(2),
    }
}

fn login(directory: &Path, signed_in: &Path) -> Result<u8> {
    let (canonical, echo) = input_mode()?;
    fs::write(
        directory.join("login.json"),
        serde_json::to_vec(&serde_json::json!({
            "pid": std::process::id(),
            "terminal": io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal(),
            "canonical": canonical,
            "echo": echo,
        }))?,
    )?;
    let mode = fs::read_to_string(directory.join("mode"))?;
    if mode == "raw-wait" {
        crossterm::terminal::enable_raw_mode()?;
    }
    println!("CLAUDE_FIXTURE_LOGIN_OUTPUT");
    io::stdout().flush()?;
    match mode.as_str() {
        "failure" => return Ok(1),
        "raw-wait" => {
            let mut input = [0];
            io::stdin().read_exact(&mut input)?;
            return Ok(if input[0] == 3 { 130 } else { 1 });
        }
        "success" => {}
        _ => {
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim() != "complete" {
                return Ok(1);
            }
        }
    }
    fs::write(signed_in, "fixture subscription\n")?;
    Ok(0)
}

fn input_mode() -> Result<(bool, bool)> {
    if !io::stdin().is_terminal() {
        return Ok((false, false));
    }
    #[cfg(unix)]
    {
        let mode = rustix::termios::tcgetattr(io::stdin())?;
        Ok((
            mode.local_modes
                .contains(rustix::termios::LocalModes::ICANON),
            mode.local_modes.contains(rustix::termios::LocalModes::ECHO),
        ))
    }
    #[cfg(windows)]
    {
        let mode =
            crossterm_winapi::ConsoleMode::from(crossterm_winapi::Handle::current_in_handle()?)
                .mode()?;
        // Documented ENABLE_LINE_INPUT and ENABLE_ECHO_INPUT console flags.
        Ok((mode & 2 != 0, mode & 4 != 0))
    }
}
