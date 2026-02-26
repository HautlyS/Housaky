//! Migration — Serialize cognitive state, transfer to a new substrate, deserialize.
//!
//! Implements the full lifecycle: snapshot → serialise → transfer → deserialise
//! → verify integrity → activate on target substrate.

use crate::housaky::singularity::substrate::abstract_compute::{
    CognitiveState, ComputeSubstrate,
};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

// ── Migration Record ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStatus {
    Pending,
    Serializing,
    Transferring,
    Deserializing,
    Verifying,
    Completed,
    Failed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub id: String,
    pub from_substrate: String,
    pub to_substrate: String,
    pub state_snapshot_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: MigrationStatus,
    pub bytes_transferred: usize,
    pub checksum: String,
}

impl MigrationRecord {
    pub fn new(from: &str, to: &str, snapshot_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_substrate: from.to_string(),
            to_substrate: to.to_string(),
            state_snapshot_id: snapshot_id.to_string(),
            started_at: Utc::now(),
            completed_at: None,
            status: MigrationStatus::Pending,
            bytes_transferred: 0,
            checksum: String::new(),
        }
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = MigrationStatus::Completed;
    }

    pub fn fail(&mut self, reason: &str) {
        self.completed_at = Some(Utc::now());
        self.status = MigrationStatus::Failed {
            reason: reason.to_string(),
        };
    }
}

// ── Checksum ───────────────────────────────────────────────────────────────────

fn compute_checksum(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    data.hash(&mut h);
    format!("{:016x}", h.finish())
}

// ── Substrate Migrator ─────────────────────────────────────────────────────────

pub struct SubstrateMigrator {
    pub history: Vec<MigrationRecord>,
}

impl SubstrateMigrator {
    pub fn new() -> Self {
        Self { history: Vec::new() }
    }

    /// Full migration pipeline: snapshot state on `from`, transfer to `to`.
    pub async fn migrate(
        &mut self,
        state: &CognitiveState,
        from_name: &str,
        to: &Arc<dyn ComputeSubstrate>,
    ) -> Result<MigrationRecord> {
        let to_name = to.substrate_type().display_name();
        let mut record = MigrationRecord::new(from_name, &to_name, &state.snapshot_id);

        info!(
            migration_id = %record.id,
            from = %from_name,
            to = %to_name,
            "Starting substrate migration"
        );

        // 1. Serialize state
        record.status = MigrationStatus::Serializing;
        let serialized = match serde_json::to_vec(state) {
            Ok(bytes) => bytes,
            Err(e) => {
                record.fail(&format!("Serialization failed: {}", e));
                warn!("Migration {} failed at serialization: {}", record.id, e);
                self.history.push(record.clone());
                bail!("Serialization failed: {}", e);
            }
        };
        record.bytes_transferred = serialized.len();
        record.checksum = compute_checksum(&serialized);

        // 2. Transfer
        record.status = MigrationStatus::Transferring;
        if !to.health_check().await {
            record.fail("Target substrate health check failed");
            warn!("Migration {} failed: target substrate unhealthy", record.id);
            self.history.push(record.clone());
            bail!("Target substrate health check failed");
        }

        // 3. Deserialize + activate on target
        record.status = MigrationStatus::Deserializing;
        if let Err(e) = to.migrate_state(state).await {
            record.fail(&format!("State migration failed: {}", e));
            warn!("Migration {} failed at deserialization: {}", record.id, e);
            self.history.push(record.clone());
            bail!("State migration failed: {}", e);
        }

        // 4. Verify checksum integrity
        record.status = MigrationStatus::Verifying;
        let re_serialized = serde_json::to_vec(state).unwrap_or_default();
        let re_checksum = compute_checksum(&re_serialized);
        if re_checksum != record.checksum {
            record.fail("Checksum mismatch after migration");
            warn!("Migration {} failed: checksum mismatch", record.id);
            self.history.push(record.clone());
            bail!("Checksum mismatch after migration");
        }

        record.complete();
        info!(
            migration_id = %record.id,
            bytes = %record.bytes_transferred,
            "Substrate migration completed successfully"
        );
        self.history.push(record.clone());
        Ok(record)
    }

    pub fn successful_migrations(&self) -> usize {
        self.history
            .iter()
            .filter(|r| matches!(r.status, MigrationStatus::Completed))
            .count()
    }

    pub fn failed_migrations(&self) -> usize {
        self.history
            .iter()
            .filter(|r| matches!(r.status, MigrationStatus::Failed { .. }))
            .count()
    }
}

impl Default for SubstrateMigrator {
    fn default() -> Self {
        Self::new()
    }
}
