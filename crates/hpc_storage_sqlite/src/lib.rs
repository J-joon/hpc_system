use async_trait::async_trait;
use hpc_core::contracts::{StorageBackend, DynResult};
use hpc_core::domain::{Event, Facts, Cursor};
use hpc_core::lattice::OrderBot;
use std::sync::{Arc, Mutex};

pub struct SqliteStorage {
    log: Arc<Mutex<Vec<Event>>>,
    facts: Arc<Mutex<Facts>>,
}

impl SqliteStorage {
    pub async fn new(_connection_string: &str) -> DynResult<Self> {
        Ok(Self {
            log: Arc::new(Mutex::new(Vec::new())),
            facts: Arc::new(Mutex::new(OrderBot::bot())),
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn append_event(&self, event: &Event) -> DynResult<Cursor> {
        let mut log = self.log.lock().unwrap();
        let cursor = log.len() as u64;
        log.push(event.clone());
        Ok(cursor)
    }

    async fn get_events(&self, start: Cursor) -> DynResult<Vec<Event>> {
        let log = self.log.lock().unwrap();
        if start >= log.len() as u64 {
            return Ok(vec![]);
        }
        Ok(log[start as usize..].to_vec())
    }

    async fn save_facts(&self, facts: &Facts) -> DynResult<()> {
        let mut current_facts = self.facts.lock().unwrap();
        *current_facts = facts.clone();
        Ok(())
    }

    async fn get_facts(&self) -> DynResult<Facts> {
        let facts = self.facts.lock().unwrap();
        Ok(facts.clone())
    }
}
