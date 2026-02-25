//! Hook registry for managing and triggering hooks.
//!
//! The registry provides thread-safe storage and management of hooks,
//! supporting both type-level handlers (e.g., "command") and specific
//! handlers (e.g., "command:new").

use crate::hooks::types::{Hook, HookEvent, HookEventType, HookResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Registry for managing hooks.
///
/// The registry stores hooks and provides methods to register, unregister,
/// and trigger hooks based on events. It supports both:
/// - Type-level handlers: hooks that handle all actions for a given type
/// - Specific handlers: hooks that handle specific actions (e.g., "command:new")
pub struct HookRegistry {
    /// Hooks registered for specific event:action combinations
    specific_hooks: Arc<RwLock<HashMap<String, Vec<HookEntry>>>>,
    /// Hooks registered for entire event types
    type_hooks: Arc<RwLock<HashMap<HookEventType, Vec<HookEntry>>>>,
}

/// A single hook entry with metadata.
#[derive(Clone)]
struct HookEntry {
    /// The hook instance
    hook: Arc<dyn Hook>,
    /// Whether the hook is enabled
    enabled: bool,
    /// Priority for ordering (lower = earlier)
    priority: i32,
}

impl HookEntry {
    fn new(hook: Arc<dyn Hook>) -> Self {
        Self {
            enabled: hook.enabled(),
            priority: hook.priority(),
            hook,
        }
    }
}

impl HookRegistry {
    /// Create a new empty hook registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            specific_hooks: Arc::new(RwLock::new(HashMap::new())),
            type_hooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a hook with the registry.
    ///
    /// The hook will be triggered for all events that match its registered
    /// event types and actions.
    ///
    /// # Arguments
    ///
    /// * `hook` - The hook to register
    ///
    /// # Example
    ///
    /// ```ignore
    /// let registry = HookRegistry::new();
    /// registry.register(my_hook).await;
    /// ```
    pub async fn register<H: Hook + 'static>(&self, hook: H) {
        let hook = Arc::new(hook);
        let entry = HookEntry::new(hook.clone());

        for (event_type, actions) in hook.events() {
            if actions.is_empty() {
                // Register for all actions of this type
                let mut type_hooks = self.type_hooks.write().await;
                type_hooks
                    .entry(event_type)
                    .or_insert_with(Vec::new)
                    .push(entry.clone());
                debug!(
                    "Registered hook '{}' for all {} events",
                    hook.id(),
                    event_type
                );
            } else {
                // Register for specific actions
                let mut specific_hooks = self.specific_hooks.write().await;
                for action in actions {
                    let key = format!("{}:{}", event_type.as_str(), action);
                    let key_for_debug = key.clone();
                    specific_hooks
                        .entry(key)
                        .or_insert_with(Vec::new)
                        .push(entry.clone());
                    debug!(
                        "Registered hook '{}' for event '{}'",
                        hook.id(),
                        key_for_debug
                    );
                }
            }
        }

        info!("Registered hook '{}' ({})", hook.id(), hook.name());
    }

    /// Unregister a hook from the registry.
    ///
    /// # Arguments
    ///
    /// * `hook_id` - The unique identifier of the hook to unregister
    ///
    /// # Returns
    ///
    /// Returns `true` if a hook was removed, `false` if no hook with that ID was found.
    pub async fn unregister(&self, hook_id: &str) -> bool {
        let mut removed = false;

        // Remove from specific hooks
        {
            let mut specific_hooks = self.specific_hooks.write().await;
            for hooks in specific_hooks.values_mut() {
                let original_len = hooks.len();
                hooks.retain(|entry| entry.hook.id() != hook_id);
                if hooks.len() != original_len {
                    removed = true;
                }
            }
        }

        // Remove from type hooks
        {
            let mut type_hooks = self.type_hooks.write().await;
            for hooks in type_hooks.values_mut() {
                let original_len = hooks.len();
                hooks.retain(|entry| entry.hook.id() != hook_id);
                if hooks.len() != original_len {
                    removed = true;
                }
            }
        }

        if removed {
            info!("Unregistered hook '{}'", hook_id);
        } else {
            warn!("Attempted to unregister unknown hook '{}'", hook_id);
        }

        removed
    }

    /// Trigger all hooks that match the given event.
    ///
    /// Hooks are executed in priority order (lower priority values first).
    /// If a hook returns `should_continue = false`, subsequent hooks are
    /// skipped for this event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to trigger hooks for
    ///
    /// # Returns
    ///
    /// Returns a `HookResult` with aggregated results from all triggered hooks.
    pub async fn trigger(&self, event: HookEvent) -> HookResult {
        let full_key = event.full_key();
        debug!("Triggering hooks for event '{}'", full_key);

        // Collect all matching hooks
        let mut all_entries: Vec<HookEntry> = Vec::new();

        // Get specific hooks for this event
        {
            let specific_hooks = self.specific_hooks.read().await;
            if let Some(hooks) = specific_hooks.get(&full_key) {
                all_entries.extend(hooks.iter().cloned());
            }
        }

        // Get type-level hooks for this event type
        {
            let type_hooks = self.type_hooks.read().await;
            if let Some(hooks) = type_hooks.get(&event.event_type) {
                all_entries.extend(hooks.iter().cloned());
            }
        }

        // Sort by priority
        all_entries.sort_by_key(|entry| entry.priority);

        // Deduplicate by hook ID (prefer specific over type-level)
        let mut seen = std::collections::HashSet::new();
        all_entries.retain(|entry| seen.insert(entry.hook.id().to_string()));

        // Filter enabled hooks
        let enabled_hooks: Vec<_> = all_entries.into_iter().filter(|e| e.enabled).collect();

        if enabled_hooks.is_empty() {
            debug!("No hooks registered for event '{}'", full_key);
            return HookResult::continue_result();
        }

        // Execute hooks
        let mut messages = Vec::new();
        let mut should_continue = true;

        for entry in enabled_hooks {
            let hook_id = entry.hook.id();
            debug!("Executing hook '{}' for event '{}'", hook_id, full_key);

            match entry.hook.handle(event.clone()).await {
                Ok(result) => {
                    messages.extend(result.messages);
                    if !result.should_continue {
                        debug!(
                            "Hook '{}' signaled stop for event '{}'",
                            hook_id, full_key
                        );
                        should_continue = false;
                        break;
                    }
                }
                Err(e) => {
                    error!(
                        "Hook '{}' failed for event '{}': {}",
                        hook_id, full_key, e
                    );
                }
            }
        }

        HookResult {
            messages,
            should_continue,
        }
    }

    /// Clear all hooks from the registry.
    pub async fn clear(&self) {
        let mut specific_hooks = self.specific_hooks.write().await;
        let mut type_hooks = self.type_hooks.write().await;
        specific_hooks.clear();
        type_hooks.clear();
        info!("Cleared all hooks from registry");
    }

    /// Get a list of all registered hook keys.
    ///
    /// Returns a vector of strings representing all registered hook keys,
    /// in the format "event_type:action" for specific hooks or "event_type:*"
    /// for type-level hooks.
    pub async fn get_registered_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();

        // Get specific hook keys
        {
            let specific_hooks = self.specific_hooks.read().await;
            keys.extend(specific_hooks.keys().cloned());
        }

        // Get type hook keys
        {
            let type_hooks = self.type_hooks.read().await;
            for event_type in type_hooks.keys() {
                keys.push(format!("{}:*", event_type));
            }
        }

        keys.sort();
        keys.dedup();
        keys
    }

    /// Check if any hooks are registered for a specific event type.
    pub async fn has_hooks_for_type(&self, event_type: HookEventType) -> bool {
        {
            let type_hooks = self.type_hooks.read().await;
            if type_hooks.get(&event_type).map_or(false, |v| !v.is_empty()) {
                return true;
            }
        }

        let specific_hooks = self.specific_hooks.read().await;
        let prefix = format!("{}:", event_type.as_str());
        specific_hooks
            .keys()
            .any(|k| k.starts_with(&prefix))
    }

    /// Get the count of registered hooks.
    pub async fn hook_count(&self) -> usize {
        let mut count = 0;

        {
            let specific_hooks = self.specific_hooks.read().await;
            for hooks in specific_hooks.values() {
                count += hooks.len();
            }
        }

        {
            let type_hooks = self.type_hooks.read().await;
            for hooks in type_hooks.values() {
                count += hooks.len();
            }
        }

        count
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug)]
    struct TestHook {
        id: &'static str,
        name: &'static str,
        events: Vec<(HookEventType, Vec<String>)>,
        call_count: Arc<AtomicUsize>,
    }

    impl TestHook {
        fn new(id: &'static str, events: Vec<(HookEventType, Vec<String>)>) -> Self {
            Self {
                id,
                name: id,
                events,
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    #[async_trait]
    impl Hook for TestHook {
        fn id(&self) -> &str {
            self.id
        }

        fn name(&self) -> &str {
            self.name
        }

        fn events(&self) -> Vec<(HookEventType, Vec<String>)> {
            self.events.clone()
        }

        async fn handle(
            &self,
            _event: HookEvent,
        ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(HookResult::continue_result())
        }
    }

    #[tokio::test]
    async fn test_register_and_trigger() {
        let registry = HookRegistry::new();
        let hook = TestHook::new(
            "test-hook",
            vec![(HookEventType::Command, vec!["execute".to_string()])],
        );

        registry.register(hook).await;

        let event = HookEvent::new(HookEventType::Command, "execute".to_string(), None);
        let result = registry.trigger(event).await;

        assert!(result.should_continue);
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = HookRegistry::new();
        let hook = TestHook::new(
            "test-hook",
            vec![(HookEventType::Command, vec!["execute".to_string()])],
        );

        registry.register(hook).await;
        assert!(registry.unregister("test-hook").await);
        assert!(!registry.unregister("test-hook").await);
    }

    #[tokio::test]
    async fn test_get_registered_keys() {
        let registry = HookRegistry::new();

        let hook1 = TestHook::new(
            "hook1",
            vec![(HookEventType::Command, vec!["execute".to_string()])],
        );
        let hook2 = TestHook::new(
            "hook2",
            vec![(HookEventType::Session, vec![])], // All actions
        );

        registry.register(hook1).await;
        registry.register(hook2).await;

        let keys = registry.get_registered_keys().await;
        assert!(keys.contains(&"command:execute".to_string()));
        assert!(keys.contains(&"session:*".to_string()));
    }

    #[tokio::test]
    async fn test_clear() {
        let registry = HookRegistry::new();
        let hook = TestHook::new(
            "test-hook",
            vec![(HookEventType::Command, vec!["execute".to_string()])],
        );

        registry.register(hook).await;
        assert!(registry.hook_count().await > 0);

        registry.clear().await;
        assert_eq!(registry.hook_count().await, 0);
    }
}
