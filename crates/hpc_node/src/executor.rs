use async_trait::async_trait;
use hpc_core::contracts::{StorageBackend, DynResult};
use hpc_core::domain::{Job, JobId};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Using an In-Memory mock for simplicity, but named SqliteStorage
// to match your crate name. Replace HashMap with `sqlx::Pool`.
pub struct SqliteStorage {
    // db: sqlx::SqlitePool, 
    mock_store: Arc<Mutex<HashMap<String, Job>>>,
}

impl SqliteStorage {
    pub async fn new(_connection_string: &str) -> DynResult<Self> {
        Ok(Self {
            mock_store: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn save_job(&self, job: &Job) -> DynResult<()> {
        let mut store = self.mock_store.lock().unwrap();
        store.insert(job.id.0.clone(), job.clone());
        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> DynResult<Option<Job>> {
        let store = self.mock_store.lock().unwrap();
        Ok(store.get(&id.0).cloned())
    }
}