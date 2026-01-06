use async_trait::async_trait;
use hpc_core::contracts::{StorageBackend, DynResult};
use hpc_core::domain::{Event, Facts, Cursor};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use std::path::Path;

pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    pub async fn new<P: AsRef<Path>>(path: P) -> DynResult<Self> {
        let conn = Connection::open(path)?;
        
        // Initialize schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                payload_json TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS facts (
                key TEXT PRIMARY KEY,
                value_json TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn append_event(&self, event: &Event) -> DynResult<Cursor> {
        let conn = self.conn.lock().unwrap();
        let payload_json = serde_json::to_string(&event.payload)?;
        conn.execute(
            "INSERT INTO events (kind, payload_json) VALUES (?1, ?2)",
            params![event.kind, payload_json],
        )?;
        let id = conn.last_insert_rowid();
        // Cursor is 1-based or 0-based? Let's assume the ID is the cursor.
        // If we want it to match Vec index (0-based), we subtract 1 if needed, 
        // but let's just use the rowid.
        Ok(id as u64)
    }

    async fn get_events(&self, start: Cursor) -> DynResult<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT kind, payload_json FROM events WHERE id > ?1 ORDER BY id ASC"
        )?;
        let event_iter = stmt.query_map(params![start], |row| {
            let kind: String = row.get(0)?;
            let payload_json: String = row.get(1)?;
            Ok((kind, payload_json))
        })?;

        let mut events = Vec::new();
        for event_res in event_iter {
            let (kind, payload_json) = event_res?;
            let payload = serde_json::from_str(&payload_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
            })?;
            events.push(Event { kind, payload });
        }
        Ok(events)
    }

    async fn save_facts(&self, facts: &Facts) -> DynResult<()> {
        let conn = self.conn.lock().unwrap();
        let facts_json = serde_json::to_string(facts)?;
        conn.execute(
            "INSERT OR REPLACE INTO facts (key, value_json) VALUES ('singleton', ?1)",
            params![facts_json],
        )?;
        Ok(())
    }

    async fn get_facts(&self) -> DynResult<Facts> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value_json FROM facts WHERE key = 'singleton'")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let facts_json: String = row.get(0)?;
            let facts = serde_json::from_str(&facts_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
            })?;
            Ok(facts)
        } else {
            use hpc_core::lattice::OrderBot;
            Ok(Facts::bot())
        }
    }
}
