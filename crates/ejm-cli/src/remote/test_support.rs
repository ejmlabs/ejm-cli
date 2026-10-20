//! A cancellable offline tool gates real harness runs in contract tests.
#[cfg(test)]
pub use fixture::Gate;

#[cfg(test)]
mod fixture {
    use anyhow::{Result, bail};
    use async_trait::async_trait;
    use ejm_harness::{Effect, Tool, ToolCall, ToolContext, ToolDefinition, ToolOutput};
    use std::time::Duration;
    use tokio::{sync::Semaphore, time::timeout};

    pub struct Gate {
        entered: Semaphore,
        released: Semaphore,
    }

    impl Default for Gate {
        fn default() -> Self {
            Self {
                entered: Semaphore::new(0),
                released: Semaphore::new(0),
            }
        }
    }

    #[async_trait]
    impl Tool for Gate {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "gate".into(),
                description: "Local deterministic test gate".into(),
                input_schema: serde_json::json!({"type":"object","additionalProperties":false}),
                effect: Effect::Read,
                version: "1".into(),
            }
        }

        async fn execute(&self, _: ToolCall, context: ToolContext) -> Result<ToolOutput> {
            self.entered.add_permits(1);
            tokio::select! {
                biased;
                () = context.cancel.cancelled() => bail!("test run cancelled"),
                permit = self.released.acquire() => { permit?.forget(); Ok(ToolOutput::text("gate released")) },
            }
        }
    }

    impl Gate {
        pub async fn wait_until_entered(&self) -> Result<()> {
            timeout(Duration::from_secs(3), self.entered.acquire())
                .await??
                .forget();
            Ok(())
        }

        pub fn release(&self) {
            self.released.add_permits(1);
        }
    }
}
