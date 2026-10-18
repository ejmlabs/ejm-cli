use std::{process::Stdio, time::Duration};

use anyhow::{Result, bail, ensure};
use ejm_harness_auth::SecretString;
use tokio_util::sync::CancellationToken;

/// Launch only the OS URL handlers selected by `open`, never a model command.
pub async fn launch(url: SecretString, cancel: CancellationToken) -> Result<()> {
    ensure!(!cancel.is_cancelled(), "browser launch cancelled");
    for launcher in open::commands(url.expose_secret()) {
        let mut command = tokio::process::Command::from(launcher);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let Ok(mut child) = command.spawn() else {
            continue;
        };
        let outcome = tokio::select! {
            biased;
            () = cancel.cancelled() => {
                let _ = child.kill().await;
                bail!("browser launch cancelled");
            }
            outcome = tokio::time::timeout(Duration::from_secs(5), child.wait()) => outcome,
        };
        match outcome {
            Ok(Ok(status)) if status.success() => return Ok(()),
            Ok(_) => {}
            Err(_) => {
                let _ = child.kill().await;
            }
        }
    }
    bail!("could not open your browser; retry with ejm-cli login chatgpt --no-browser")
}
