//! Deterministic provider composition exists only in the native test executable.
use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use clap::Parser;
use ejm_harness::{HarnessConfig, ReplayProvider};
use serde::Deserialize;

use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    state: PathBuf,
    claude: PathBuf,
}

#[tokio::test]
#[ignore = "PTY or ConPTY supervisor supplies isolated native fixture paths"]
async fn configured_terminal_interaction_fixture() -> Result<()> {
    let config_path = PathBuf::from(
        std::env::var_os("EJM_TEST_TERMINAL_CONFIG").context("terminal fixture config missing")?,
    );
    ensure!(
        std::fs::metadata(&config_path)?.len() <= 8192,
        "terminal fixture config exceeds bound"
    );
    let config: Config = serde_json::from_slice(&std::fs::read(config_path)?)?;
    let harness = Harness::new(
        HarnessConfig {
            state_dir: config.state.clone(),
            ..HarnessConfig::default()
        },
        Arc::new(ReplayProvider::echo()),
        vec![],
    )?;
    let executor = Executor::new(harness.governor(), config.state.join("artifacts"))?;
    let mut args = Args::try_parse_from(["ejm-cli", "--plain", "--no-color"])?;
    args.state_dir = Some(config.state);
    args.claude_path = Some(config.claude);
    args.package_provider = Some("@fixture/terminal:provider".into());
    let extensions = Extensions::empty()?;
    let backend = LocalBackend::new(harness.clone());
    let remote = HostUi::prepare(&args, CancellationToken::new()).await?;
    let mut app = initial_app(&backend, &args, &extensions).await?;
    app.connection_ready = true;
    app.status = "Ready".into();
    app.notice("Internal terminal fixture ready.");
    let result = run_prepared(
        backend,
        &mut args,
        &extensions,
        remote,
        app,
        ConnectionTasks::new(executor),
        termination_signal(),
    )
    .await;
    let shutdown = harness.shutdown().await;
    result?;
    shutdown
}
