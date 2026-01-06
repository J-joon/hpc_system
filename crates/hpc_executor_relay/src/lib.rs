use async_trait::async_trait;
use hpc_core::contracts::{ExecutorBackend, DynResult};
use hpc_core::domain::Event;

pub struct RelayExecutor;

impl RelayExecutor {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl ExecutorBackend for RelayExecutor {
    async fn execute_payload(&self, event: &Event) -> DynResult<()> {
        println!("[Executor] Executing event kind: {}", event.kind);
        Ok(())
    }
}
