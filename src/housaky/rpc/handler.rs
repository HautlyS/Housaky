use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use crate::housaky::memory::Memory;
use crate::housaky::skills::Skills;
use crate::housaky::a2a::A2AManager;
use crate::housaky::goals::GoalManager;
use crate::housaky::heartbeat::HousakyHeartbeat;
use crate::housaky::config::Config;

/// Trait defining the RPC methods that the handler must implement.
#[async_trait::async_trait]
pub trait RpcHandler: Send + Sync {
    // Memory operations
    async fn memory_store(&self, key: String, value: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
    async fn memory_recall(&self, query: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
    async fn memory_search(&self, query: String, limit: usize) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
    async fn memory_forget(&self, key: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    // Skills operations
    async fn skill_list(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
    async fn skill_get(&self, name: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
    async fn skill_run(&self, name: String, inputs: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    // A2A operations
    async fn a2a_send(&self, message: String, target: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    // Goals operations
    async fn goal_set(&self, description: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
    async fn goal_list(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    // Heartbeat operations
    async fn heartbeat_trigger(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    // Configuration operations
    async fn config_get(&self, key: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
    async fn config_set(&self, key: String, value: Option<Value>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    // System operations
    async fn system_version(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
}

/// A default implementation of the RpcHandler that uses the existing Housaky subsystems.
pub struct DefaultRpcHandler {
    memory: Arc<RwLock<Memory>>,
    skills: Arc<RwLock<Skills>>,
    a2a_manager: Arc<RwLock<A2AManager>>,
    goal_manager: Arc<RwLock<GoalManager>>,
    heartbeat: Arc<RwLock<HousakyHeartbeat>>,
    config: Arc<RwLock<Config>>,
}

impl DefaultRpcHandler {
    /// Create a new DefaultRpcHandler with the given subsystems.
    pub fn new(
        memory: Arc<RwLock<Memory>>,
        skills: Arc<RwLock<Skills>>,
        a2a_manager: Arc<RwLock<A2AManager>>,
        goal_manager: Arc<RwLock<GoalManager>>,
        heartbeat: Arc<RwLock<HousakyHeartbeat>>,
        config: Arc<RwLock<Config>>,
    ) -> Self {
        Self {
            memory,
            skills,
            a2a_manager,
            goal_manager,
            heartbeat,
            config,
        }
    }
}

#[async_trait::async_trait]
impl RpcHandler for DefaultRpcHandler {
    async fn memory_store(&self, key: String, value: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut mem = self.memory.write().await;
        mem.store(&key, &value).await?;
        Ok(json!(true))
    }

    async fn memory_recall(&self, query: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mem = self.memory.read().await;
        let result = mem.recall(&query).await?;
        Ok(serde_json::to_value(result)?)
    }

    async fn memory_search(&self, query: String, limit: usize) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mem = self.memory.read().await;
        let results = mem.search(&query, limit).await?;
        Ok(serde_json::to_value(results)?)
    }

    async fn memory_forget(&self, key: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut mem = self.memory.write().await;
        mem.forget(&key).await?;
        Ok(json!(true))
    }

    async fn skill_list(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let skills = self.skills.read().await;
        let list = skills.list().await?;
        Ok(serde_json::to_value(list)?)
    }

    async fn skill_get(&self, name: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let skills = self.skills.read().await;
        let skill = skills.get(&name).await?;
        Ok(serde_json::to_value(skill)?)
    }

    async fn skill_run(&self, name: String, inputs: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut skills = self.skills.write().await;
        let result = skills.run(&name, inputs).await?;
        Ok(serde_json::to_value(result)?)
    }

    async fn a2a_send(&self, message: String, target: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut a2a = self.a2a_manager.write().await;
        a2a.send(&message, &target).await?;
        Ok(json!(true))
    }

    async fn goal_set(&self, description: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut goals = self.goal_manager.write().await;
        let goal_id = goals.set(&description).await?;
        Ok(serde_json::to_value(goal_id)?)
    }

    async fn goal_list(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let goals = self.goal_manager.read().await;
        let list = goals.list().await?;
        Ok(serde_json::to_value(list)?)
    }

    async fn heartbeat_trigger(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut hb = self.heartbeat.write().await;
        hb.trigger().await?;
        Ok(json!(true))
    }

    async fn config_get(&self, key: String) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let config = self.config.read().await;
        let value = config.get(&key)?;
        Ok(serde_json::to_value(value)?)
    }

    async fn config_set(&self, key: String, value: Option<Value>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut config = self.config.write().await;
        config.set(&key, value)?;
        Ok(json!(true))
    }

    async fn system_version(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::to_value(env!("CARGO_PKG_VERSION"))?)
    }
}