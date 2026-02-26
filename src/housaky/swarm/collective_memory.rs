use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMemoryEntry {
    pub id: String,
    pub key: String,
    pub value: serde_json::Value,
    pub source_agent: String,
    pub confidence: f64,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access_count: u64,
    pub tags: Vec<String>,
    pub vector: Option<Vec<f32>>,
}

impl SharedMemoryEntry {
    pub fn new(key: &str, value: serde_json::Value, source_agent: &str, confidence: f64) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            value,
            source_agent: source_agent.to_string(),
            confidence,
            version: 1,
            created_at: now,
            updated_at: now,
            access_count: 0,
            tags: Vec::new(),
            vector: None,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_vector(mut self, v: Vec<f32>) -> Self {
        self.vector = Some(v);
        self
    }

    pub fn update(&mut self, value: serde_json::Value, source_agent: &str, confidence: f64) {
        if confidence >= self.confidence {
            self.value = value;
            self.source_agent = source_agent.to_string();
            self.confidence = confidence;
            self.version += 1;
            self.updated_at = Utc::now();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub key: String,
    pub existing_version: u64,
    pub incoming_version: u64,
    pub resolution: ConflictResolution,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConflictResolution {
    KeepExisting,
    TakeIncoming,
    Merge,
    HigherConfidenceWins,
}

pub struct CollectiveMemory {
    pub store: Arc<RwLock<HashMap<String, SharedMemoryEntry>>>,
    pub conflict_log: Arc<RwLock<Vec<ConflictRecord>>>,
    pub resolution_strategy: ConflictResolution,
    pub max_entries: usize,
    /// Optional SQLite database path. When set, `persist()` and `load()` are active.
    pub db_path: Option<PathBuf>,
}

impl CollectiveMemory {
    pub fn new(resolution_strategy: ConflictResolution, max_entries: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            conflict_log: Arc::new(RwLock::new(Vec::new())),
            resolution_strategy,
            max_entries,
            db_path: None,
        }
    }

    /// Create a CollectiveMemory backed by a SQLite file at `db_path`.
    /// Immediately initialises the schema and loads existing entries.
    pub fn new_persistent(
        resolution_strategy: ConflictResolution,
        max_entries: usize,
        db_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let path = db_path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::init_schema(&path)?;
        Ok(Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            conflict_log: Arc::new(RwLock::new(Vec::new())),
            resolution_strategy,
            max_entries,
            db_path: Some(path),
        })
    }

    fn init_schema(db_path: &Path) -> anyhow::Result<()> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS collective_memory (
                id          TEXT    PRIMARY KEY,
                key         TEXT    NOT NULL UNIQUE,
                value_json  TEXT    NOT NULL,
                source_agent TEXT   NOT NULL,
                confidence  REAL    NOT NULL,
                version     INTEGER NOT NULL,
                created_at  TEXT    NOT NULL,
                updated_at  TEXT    NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 0,
                tags_json   TEXT    NOT NULL DEFAULT '[]',
                vector_json TEXT
            );
            CREATE TABLE IF NOT EXISTS conflict_log (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                key             TEXT    NOT NULL,
                existing_version INTEGER NOT NULL,
                incoming_version INTEGER NOT NULL,
                resolution      TEXT    NOT NULL,
                resolved_at     TEXT    NOT NULL
            );",
        )?;
        Ok(())
    }

    /// Persist the in-memory store to SQLite (upsert all entries).
    pub async fn persist(&self) -> anyhow::Result<()> {
        let db_path = match &self.db_path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        let entries: Vec<SharedMemoryEntry> = self.store.read().await.values().cloned().collect();
        let conflicts: Vec<ConflictRecord> = self.conflict_log.read().await.clone();
        let entry_count = entries.len();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = Connection::open(&db_path)?;
            conn.execute_batch("BEGIN;")?;
            for e in &entries {
                let tags = serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".to_string());
                let vector = e.vector.as_ref()
                    .and_then(|v| serde_json::to_string(v).ok());
                let value_json = serde_json::to_string(&e.value)
                    .unwrap_or_else(|_| "null".to_string());
                conn.execute(
                    "INSERT INTO collective_memory
                        (id, key, value_json, source_agent, confidence, version,
                         created_at, updated_at, access_count, tags_json, vector_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                     ON CONFLICT(key) DO UPDATE SET
                        value_json   = excluded.value_json,
                        source_agent = excluded.source_agent,
                        confidence   = excluded.confidence,
                        version      = excluded.version,
                        updated_at   = excluded.updated_at,
                        access_count = excluded.access_count,
                        tags_json    = excluded.tags_json,
                        vector_json  = excluded.vector_json",
                    params![
                        e.id,
                        e.key,
                        value_json,
                        e.source_agent,
                        e.confidence,
                        e.version as i64,
                        e.created_at.to_rfc3339(),
                        e.updated_at.to_rfc3339(),
                        e.access_count as i64,
                        tags,
                        vector,
                    ],
                )?;
            }
            for c in &conflicts {
                conn.execute(
                    "INSERT OR IGNORE INTO conflict_log
                        (key, existing_version, incoming_version, resolution, resolved_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        c.key,
                        c.existing_version as i64,
                        c.incoming_version as i64,
                        format!("{:?}", c.resolution),
                        c.resolved_at.to_rfc3339(),
                    ],
                )?;
            }
            conn.execute_batch("COMMIT;")?;
            Ok(())
        })
        .await??;
        info!(
            "CollectiveMemory persisted {} entries to SQLite",
            entry_count
        );
        Ok(())
    }

    /// Load all entries from SQLite into the in-memory store.
    pub async fn load(&self) -> anyhow::Result<()> {
        let db_path = match &self.db_path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        let (entries, conflicts) = tokio::task::spawn_blocking(move || -> anyhow::Result<(Vec<SharedMemoryEntry>, Vec<ConflictRecord>)> {
            let conn = Connection::open(&db_path)?;

            let mut stmt = conn.prepare(
                "SELECT id, key, value_json, source_agent, confidence, version,
                        created_at, updated_at, access_count, tags_json, vector_json
                 FROM collective_memory",
            )?;

            let entries: Vec<SharedMemoryEntry> = stmt.query_map([], |row| {
                let value_json: String = row.get(2)?;
                let tags_json: String = row.get(9)?;
                let vector_json: Option<String> = row.get(10)?;
                let created_str: String = row.get(6)?;
                let updated_str: String = row.get(7)?;
                Ok((value_json, tags_json, vector_json, created_str, updated_str,
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(8)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(value_json, tags_json, vector_json, created_str, updated_str,
                          id, key, source_agent, confidence, version, access_count)| {
                let value = serde_json::from_str(&value_json).ok()?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                let vector: Option<Vec<f32>> = vector_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok());
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .ok()?;
                let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|d| d.with_timezone(&Utc))
                    .ok()?;
                Some(SharedMemoryEntry {
                    id,
                    key,
                    value,
                    source_agent,
                    confidence,
                    version: version as u64,
                    created_at,
                    updated_at,
                    access_count: access_count as u64,
                    tags,
                    vector,
                })
            })
            .collect();

            let conflicts: Vec<ConflictRecord> = Vec::new();
            Ok((entries, conflicts))
        })
        .await??;

        let mut store = self.store.write().await;
        for e in &entries {
            store.insert(e.key.clone(), e.clone());
        }
        *self.conflict_log.write().await = conflicts;
        info!(
            "CollectiveMemory loaded {} entries from SQLite",
            entries.len()
        );
        Ok(())
    }

    /// Convenience: load at startup, gracefully ignoring errors.
    pub async fn load_or_warn(&self) {
        if let Err(e) = self.load().await {
            warn!("CollectiveMemory: failed to load from SQLite: {}", e);
        }
    }

    pub async fn write(
        &self,
        key: &str,
        value: serde_json::Value,
        source_agent: &str,
        confidence: f64,
        tags: Vec<String>,
    ) {
        let mut store = self.store.write().await;

        if store.len() >= self.max_entries && !store.contains_key(key) {
            let oldest_key = store
                .values()
                .min_by(|a, b| a.updated_at.cmp(&b.updated_at))
                .map(|e| e.key.clone());
            if let Some(k) = oldest_key {
                store.remove(&k);
            }
        }

        if let Some(existing) = store.get_mut(key) {
            let conflict = ConflictRecord {
                key: key.to_string(),
                existing_version: existing.version,
                incoming_version: existing.version + 1,
                resolution: self.resolution_strategy.clone(),
                resolved_at: Utc::now(),
            };

            match self.resolution_strategy {
                ConflictResolution::TakeIncoming => {
                    existing.update(value, source_agent, confidence);
                }
                ConflictResolution::HigherConfidenceWins => {
                    if confidence > existing.confidence {
                        existing.update(value, source_agent, confidence);
                    }
                }
                ConflictResolution::Merge => {
                    if let (serde_json::Value::Object(existing_map), serde_json::Value::Object(new_map)) =
                        (&mut existing.value, &value)
                    {
                        let existing_map = existing_map.clone();
                        if let serde_json::Value::Object(ref mut em) = existing.value {
                            for (k, v) in new_map {
                                em.insert(k.clone(), v.clone());
                            }
                            for (k, v) in &existing_map {
                                em.entry(k.clone()).or_insert_with(|| v.clone());
                            }
                        }
                        existing.version += 1;
                        existing.updated_at = Utc::now();
                    } else {
                        existing.update(value, source_agent, confidence);
                    }
                }
                ConflictResolution::KeepExisting => {}
            }

            drop(store);
            self.conflict_log.write().await.push(conflict);
        } else {
            let entry = SharedMemoryEntry::new(key, value, source_agent, confidence)
                .with_tags(tags);
            store.insert(key.to_string(), entry);
            info!("Collective memory: new entry '{}' from agent '{}'", key, source_agent);
        }
    }

    pub async fn read(&self, key: &str) -> Option<SharedMemoryEntry> {
        let mut store = self.store.write().await;
        if let Some(entry) = store.get_mut(key) {
            entry.access_count += 1;
            Some(entry.clone())
        } else {
            None
        }
    }

    pub async fn search_by_tag(&self, tag: &str) -> Vec<SharedMemoryEntry> {
        self.store
            .read()
            .await
            .values()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    pub async fn search_by_prefix(&self, prefix: &str) -> Vec<SharedMemoryEntry> {
        self.store
            .read()
            .await
            .values()
            .filter(|e| e.key.starts_with(prefix))
            .cloned()
            .collect()
    }

    pub async fn cosine_search(&self, query_vector: &[f32], top_k: usize) -> Vec<SharedMemoryEntry> {
        let store = self.store.read().await;
        let mut scored: Vec<(f32, SharedMemoryEntry)> = store
            .values()
            .filter_map(|e| {
                e.vector.as_ref().map(|v| {
                    let sim = cosine_similarity(query_vector, v);
                    (sim, e.clone())
                })
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(_, e)| e).collect()
    }

    pub async fn agent_contributions(&self, agent_id: &str) -> Vec<SharedMemoryEntry> {
        self.store
            .read()
            .await
            .values()
            .filter(|e| e.source_agent == agent_id)
            .cloned()
            .collect()
    }

    pub async fn stats(&self) -> CollectiveMemoryStats {
        let store = self.store.read().await;
        let total = store.len();
        let avg_confidence = if total > 0 {
            store.values().map(|e| e.confidence).sum::<f64>() / total as f64
        } else {
            0.0
        };
        let conflicts = self.conflict_log.read().await.len();
        CollectiveMemoryStats { total_entries: total, avg_confidence, total_conflicts: conflicts }
    }

    pub async fn remove(&self, key: &str) -> bool {
        self.store.write().await.remove(key).is_some()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-9 || norm_b < 1e-9 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveMemoryStats {
    pub total_entries: usize,
    pub avg_confidence: f64,
    pub total_conflicts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_and_read() {
        let mem = CollectiveMemory::new(ConflictResolution::HigherConfidenceWins, 1000);
        mem.write("fact:1", serde_json::json!("Rust is fast"), "agent-1", 0.9, vec!["fact".into()]).await;
        let entry = mem.read("fact:1").await.unwrap();
        assert_eq!(entry.value, serde_json::json!("Rust is fast"));
        assert_eq!(entry.access_count, 1);
    }

    #[tokio::test]
    async fn test_conflict_resolution_higher_confidence() {
        let mem = CollectiveMemory::new(ConflictResolution::HigherConfidenceWins, 1000);
        mem.write("key", serde_json::json!("old"), "agent-1", 0.5, vec![]).await;
        mem.write("key", serde_json::json!("new"), "agent-2", 0.9, vec![]).await;
        let entry = mem.read("key").await.unwrap();
        assert_eq!(entry.value, serde_json::json!("new"));
    }

    #[tokio::test]
    async fn test_conflict_resolution_keep_existing() {
        let mem = CollectiveMemory::new(ConflictResolution::KeepExisting, 1000);
        mem.write("key", serde_json::json!("original"), "agent-1", 0.5, vec![]).await;
        mem.write("key", serde_json::json!("override"), "agent-2", 0.9, vec![]).await;
        let entry = mem.read("key").await.unwrap();
        assert_eq!(entry.value, serde_json::json!("original"));
    }

    #[tokio::test]
    async fn test_tag_search() {
        let mem = CollectiveMemory::new(ConflictResolution::TakeIncoming, 1000);
        mem.write("k1", serde_json::json!(1), "a", 0.8, vec!["rust".into()]).await;
        mem.write("k2", serde_json::json!(2), "a", 0.8, vec!["python".into()]).await;
        let results = mem.search_by_tag("rust").await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_cosine_search() {
        let mem = CollectiveMemory::new(ConflictResolution::TakeIncoming, 1000);
        let mut e1 = SharedMemoryEntry::new("vec1", serde_json::json!("A"), "a", 0.9);
        e1.vector = Some(vec![1.0, 0.0, 0.0]);
        let mut e2 = SharedMemoryEntry::new("vec2", serde_json::json!("B"), "a", 0.9);
        e2.vector = Some(vec![0.0, 1.0, 0.0]);
        mem.store.write().await.insert("vec1".to_string(), e1);
        mem.store.write().await.insert("vec2".to_string(), e2);
        let results = mem.cosine_search(&[1.0, 0.0, 0.0], 1).await;
        assert_eq!(results[0].key, "vec1");
    }
}
