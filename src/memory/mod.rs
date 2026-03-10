pub mod backend;
pub mod chunker;
pub mod embeddings;
pub mod hygiene;
pub mod intelligent_memory;
pub mod lucid;
pub mod lucid_native;
pub mod markdown;
pub mod none;
pub mod project_context;
pub mod response_cache;
pub mod snapshot;
pub mod traits;
pub mod unified_agi_hub;
pub mod vector;

#[allow(unused_imports)]
pub use backend::{
    classify_memory_backend, default_memory_backend_key, memory_backend_profile,
    selectable_memory_backends, MemoryBackendKind, MemoryBackendProfile,
};
pub use lucid::LucidMemory;
pub use lucid_native::{LucidNativeMemory, LucidNativeConfig, LucidMemoryStats};
pub use markdown::MarkdownMemory;
pub use none::NoneMemory;
pub use response_cache::ResponseCache;
pub use traits::Memory;
#[allow(unused_imports)]
pub use traits::{MemoryCategory, MemoryEntry};
pub use intelligent_memory::{IntelligentMemory, IntelligentMemoryConfig, MemoryImportance, ContextBudget};
pub use project_context::{
    ContextLevel, ProjectContext, ContextEntry, ConnectionRef, ConnectionType,
    ConnectionGraph, ContextSwitcher, FederationContext, FederationAwareContext,
    AgentAwarenessEngine, AgentState, AwarenessLevel, AwarenessContext,
};
pub use unified_agi_hub::{
    UnifiedAGIMemoryHub, UnifiedAGIMemoryConfig, UnifiedMemorySource, UnifiedMemoryEntry,
    CollectiveMindState, PeerInfo, SharedInsight, ConsensusTopic,
    SelfImprovementRecord, ImprovementType, UnifiedMemoryStats,
};

use crate::config::MemoryConfig;
use std::path::Path;

fn create_memory_backend(
    backend_name: &str,
    workspace_dir: &Path,
    unknown_context: &str,
) -> anyhow::Result<Box<dyn Memory>> {
    match classify_memory_backend(backend_name) {
        MemoryBackendKind::Lucid => {
            let config = LucidNativeConfig {
                project_path: workspace_dir.to_path_buf(),
                ..LucidNativeConfig::default()
            };
            Ok(Box::new(LucidNativeMemory::new(config)))
        }
        MemoryBackendKind::Markdown => Ok(Box::new(MarkdownMemory::new(workspace_dir))),
        MemoryBackendKind::None => Ok(Box::new(NoneMemory::new())),
        MemoryBackendKind::Unknown => {
            tracing::warn!(
                "Unknown memory backend '{backend_name}'{unknown_context}, falling back to lucid-native"
            );
            let config = LucidNativeConfig {
                project_path: workspace_dir.to_path_buf(),
                ..LucidNativeConfig::default()
            };
            Ok(Box::new(LucidNativeMemory::new(config)))
        }
    }
}

/// Factory: create the right memory backend from config
pub fn create_memory(
    config: &MemoryConfig,
    workspace_dir: &Path,
    _api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    // Best-effort memory hygiene/retention pass (throttled by state file).
    if let Err(e) = hygiene::run_if_due(config, workspace_dir) {
        tracing::warn!("memory hygiene skipped: {e}");
    }

    // If snapshot_on_hygiene is enabled, export core memories during hygiene.
    if config.snapshot_enabled && config.snapshot_on_hygiene {
        if let Err(e) = snapshot::export_snapshot(workspace_dir) {
            tracing::warn!("memory snapshot skipped: {e}");
        }
    }

    // Auto-hydration: if Lucid memory is unavailable but MEMORY_SNAPSHOT.md exists,
    // restore the "soul" from the snapshot before creating the backend.
    if config.auto_hydrate
        && matches!(
            classify_memory_backend(&config.backend),
            MemoryBackendKind::Lucid
        )
        && snapshot::should_hydrate(workspace_dir)
    {
        tracing::info!("🧬 Cold boot detected — hydrating from MEMORY_SNAPSHOT.md");
        match snapshot::hydrate_from_snapshot(workspace_dir) {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("🧬 Hydrated {count} core memories from snapshot");
                }
            }
            Err(e) => {
                tracing::warn!("memory hydration failed: {e}");
            }
        }
    }

    create_memory_backend(&config.backend, workspace_dir, "")
}

pub fn create_memory_for_migration(
    backend: &str,
    workspace_dir: &Path,
) -> anyhow::Result<Box<dyn Memory>> {
    if matches!(classify_memory_backend(backend), MemoryBackendKind::None) {
        anyhow::bail!(
            "memory backend 'none' disables persistence; choose lucid or markdown before migration"
        );
    }

    create_memory_backend(backend, workspace_dir, " during migration")
}

/// Factory: create an optional response cache from config.
pub fn create_response_cache(config: &MemoryConfig, workspace_dir: &Path) -> Option<ResponseCache> {
    if !config.response_cache_enabled {
        return None;
    }

    match ResponseCache::new(
        workspace_dir,
        config.response_cache_ttl_minutes,
        config.response_cache_max_entries,
    ) {
        Ok(cache) => {
            tracing::info!(
                "💾 Response cache enabled (TTL: {}min, max: {} entries)",
                config.response_cache_ttl_minutes,
                config.response_cache_max_entries
            );
            Some(cache)
        }
        Err(e) => {
            tracing::warn!("Response cache disabled due to error: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn factory_sqlite() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "sqlite".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "sqlite");
    }

    #[test]
    fn factory_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn factory_lucid() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "lucid".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "lucid");
    }

    #[test]
    fn factory_none_uses_noop_memory() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "none".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "none");
    }

    #[test]
    fn factory_unknown_falls_back_to_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "redis".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn migration_factory_lucid() {
        let tmp = TempDir::new().unwrap();
        let mem = create_memory_for_migration("lucid", tmp.path()).unwrap();
        assert_eq!(mem.name(), "lucid");
    }

    #[test]
    fn migration_factory_none_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let error = create_memory_for_migration("none", tmp.path())
            .err()
            .expect("backend=none should be rejected for migration");
        assert!(error.to_string().contains("disables persistence"));
    }
}
