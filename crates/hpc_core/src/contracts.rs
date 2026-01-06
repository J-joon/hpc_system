use async_trait::async_trait;
use crate::domain::{Event, Facts, Cursor, Poke};
use std::error::Error;

// Type alias for generic errors to keep signatures clean
pub type DynResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Appends an event to the log and returns its cursor (index).
    async fn append_event(&self, event: &Event) -> DynResult<Cursor>;
    
    /// Retrieves events starting from the given cursor.
    async fn get_events(&self, start: Cursor) -> DynResult<Vec<Event>>;

    /// Persists the current converged facts.
    async fn save_facts(&self, facts: &Facts) -> DynResult<()>;

    /// Retrieves the current converged facts.
    async fn get_facts(&self) -> DynResult<Facts>;
}

#[async_trait]
pub trait TransportBackend: Send + Sync {
    /// Pushes a notification (poke) to a specific generator.
    async fn send_poke(&self, poke: &Poke) -> DynResult<()>;

    /// Pushes a notification (poke) to a specific URL.
    async fn send_poke_to_url(&self, poke: &Poke, url: &str) -> DynResult<()>;
}

#[async_trait]
pub trait ExecutorBackend: Send + Sync {
    /// In the new model, executors might react to specific events or facts.
    /// For now, we keep a generic execution trigger.
    async fn execute_payload(&self, event: &Event) -> DynResult<()>;
}
