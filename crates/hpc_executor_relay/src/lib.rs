use async_trait::async_trait;
use hpc_core::contracts::{ExecutorBackend, DynResult};
use hpc_core::domain::Event;
use tokio::process::Command;
use std::process::ExitStatus;

pub struct RelayExecutor;

impl RelayExecutor {
    pub fn new() -> Self { Self }

    pub async fn run_command(&self, cmd_str: &str) -> DynResult<ExitStatus> {
        println!("[Executor] Running command: {}", cmd_str);
        
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(cmd_str)
            .spawn()?;

        let status = child.wait().await?;
        Ok(status)
    }
}

#[async_trait]
impl ExecutorBackend for RelayExecutor {
    async fn execute_payload(&self, event: &Event) -> DynResult<()> {
        println!("[Executor] Received execution request for kind: {}", event.kind);
        // Dispatching based on event kind will be handled by the caller in the node main loop
        Ok(())
    }
}
