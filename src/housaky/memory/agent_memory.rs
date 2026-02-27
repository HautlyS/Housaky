//! Persistent SQLite-backed agent memory.
//!
//! This module provides `AgentMemoryStore` — a thin wrapper around a
//! `rusqlite` database that persists consolidated memory facts, procedures,
//! and skills extracted by `MemoryConsolidator`.  Every record written here
//! survives restarts and is queryable by the agent at runtime.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::info;

// ── Record types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryRecord {
    pub id: String,
    pub kind: MemoryKind,
    pub content: String,
    pub source: String,
    pub confidence: f64,
    pub importance: f64,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Fact,
    Procedure,
    Skill,
    Pattern,
    Observation,
    /// Insight gained from episodic consolidation.
    Insight,
    /// General experience promoted from episodic memory.
    Experience,
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        write!(f, "{s}")
    }
}

impl std::str::FromStr for MemoryKind {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .with_context(|| format!("Unknown MemoryKind: {s}"))
    }
}

// ── Store ────────────────────────────────────────────────────────────────────

pub struct AgentMemoryStore {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl AgentMemoryStore {
    /// Open (or create) the SQLite database at `<workspace>/.housaky/agent_memory.db`.
    pub fn open(workspace_dir: &PathBuf) -> Result<Self> {
        let dir = workspace_dir.join(".housaky");
        std::fs::create_dir_all(&dir)?;
        let db_path = dir.join("agent_memory.db");

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open agent memory DB at {db_path:?}"))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS agent_memory (
                 id           TEXT PRIMARY KEY,
                 kind         TEXT NOT NULL,
                 content      TEXT NOT NULL,
                 source       TEXT NOT NULL,
                 confidence   REAL NOT NULL DEFAULT 0.5,
                 importance   REAL NOT NULL DEFAULT 0.5,
                 tags         TEXT NOT NULL DEFAULT '[]',
                 created_at   TEXT NOT NULL,
                 accessed_at  TEXT NOT NULL,
                 access_count INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_kind       ON agent_memory(kind);
             CREATE INDEX IF NOT EXISTS idx_importance ON agent_memory(importance DESC);
             CREATE INDEX IF NOT EXISTS idx_accessed   ON agent_memory(accessed_at DESC);",
        )?;

        info!("AgentMemoryStore opened at {:?}", db_path);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path,
        })
    }

    /// Upsert a memory record (INSERT OR REPLACE).
    pub fn store(&self, record: &AgentMemoryRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(&record.tags)?;
        conn.execute(
            "INSERT OR REPLACE INTO agent_memory
                 (id, kind, content, source, confidence, importance, tags,
                  created_at, accessed_at, access_count)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                record.id,
                record.kind.to_string(),
                record.content,
                record.source,
                record.confidence,
                record.importance,
                tags_json,
                record.created_at.to_rfc3339(),
                record.accessed_at.to_rfc3339(),
                record.access_count as i64,
            ],
        )?;
        Ok(())
    }

    /// Retrieve the `limit` most-important records of a given kind.
    pub fn recall_by_kind(&self, kind: &MemoryKind, limit: usize) -> Result<Vec<AgentMemoryRecord>> {
        let conn = self.conn.lock().unwrap();
        let kind_str = kind.to_string();
        let mut stmt = conn.prepare(
            "SELECT id, kind, content, source, confidence, importance, tags,
                    created_at, accessed_at, access_count
             FROM   agent_memory
             WHERE  kind = ?1
             ORDER  BY importance DESC, access_count DESC
             LIMIT  ?2",
        )?;
        let records = stmt
            .query_map(params![kind_str, limit as i64], row_to_record)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    /// Full-text search across `content` (simple LIKE).
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<AgentMemoryRecord>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{query}%");
        let mut stmt = conn.prepare(
            "SELECT id, kind, content, source, confidence, importance, tags,
                    created_at, accessed_at, access_count
             FROM   agent_memory
             WHERE  content LIKE ?1
             ORDER  BY importance DESC
             LIMIT  ?2",
        )?;
        let records = stmt
            .query_map(params![pattern, limit as i64], row_to_record)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    /// Bump the `access_count` and update `accessed_at` for the given record.
    pub fn touch(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE agent_memory
             SET    access_count = access_count + 1,
                    accessed_at  = ?1
             WHERE  id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    /// Delete records with importance below `threshold` that haven't been
    /// accessed in the last `older_than_days` days (memory pruning).
    pub fn prune(&self, threshold: f64, older_than_days: i64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::days(older_than_days)).to_rfc3339();
        let n = conn.execute(
            "DELETE FROM agent_memory
             WHERE  importance < ?1
             AND    accessed_at < ?2",
            params![threshold, cutoff],
        )?;
        if n > 0 {
            info!("AgentMemoryStore: pruned {} stale records", n);
        }
        Ok(n as u64)
    }

    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    pub fn record_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let n: i64 =
            conn.query_row("SELECT COUNT(*) FROM agent_memory", [], |r| r.get(0))?;
        Ok(n as u64)
    }
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentMemoryRecord> {
    let kind_str: String = row.get(1)?;
    let kind: MemoryKind = kind_str
        .parse()
        .unwrap_or(MemoryKind::Observation);

    let tags_json: String = row.get(6)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

    let created_str: String = row.get(7)?;
    let accessed_str: String = row.get(8)?;

    let created_at = created_str
        .parse::<DateTime<Utc>>()
        .unwrap_or_else(|_| Utc::now());
    let accessed_at = accessed_str
        .parse::<DateTime<Utc>>()
        .unwrap_or_else(|_| Utc::now());

    let access_count: i64 = row.get(9)?;

    Ok(AgentMemoryRecord {
        id: row.get(0)?,
        kind,
        content: row.get(2)?,
        source: row.get(3)?,
        confidence: row.get(4)?,
        importance: row.get(5)?,
        tags,
        created_at,
        accessed_at,
        access_count: access_count as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_recall() {
        let dir = tempfile::tempdir().unwrap();
        let store = AgentMemoryStore::open(&dir.path().to_path_buf()).unwrap();

        let record = AgentMemoryRecord {
            id: uuid::Uuid::new_v4().to_string(),
            kind: MemoryKind::Fact,
            content: "Housaky runs on Rust".to_string(),
            source: "consolidation".to_string(),
            confidence: 0.9,
            importance: 0.8,
            tags: vec!["rust".to_string(), "tech".to_string()],
            created_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
        };

        store.store(&record).unwrap();
        assert_eq!(store.record_count().unwrap(), 1);

        let results = store.recall_by_kind(&MemoryKind::Fact, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Housaky runs on Rust");

        let search = store.search("Rust", 10).unwrap();
        assert_eq!(search.len(), 1);
    }

    #[test]
    fn test_prune() {
        let dir = tempfile::tempdir().unwrap();
        let store = AgentMemoryStore::open(&dir.path().to_path_buf()).unwrap();

        let old_time = (Utc::now() - chrono::Duration::days(10)).to_rfc3339();

        {
            let conn = store.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO agent_memory
                     (id, kind, content, source, confidence, importance, tags,
                      created_at, accessed_at, access_count)
                 VALUES ('old1','fact','stale','test',0.5,0.1,'[]',?1,?1,0)",
                params![old_time],
            )
            .unwrap();
        }

        let pruned = store.prune(0.2, 7).unwrap();
        assert_eq!(pruned, 1);
        assert_eq!(store.record_count().unwrap(), 0);
    }
}
