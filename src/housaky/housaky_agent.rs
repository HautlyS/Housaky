use crate::config::Config;
use crate::util::{read_toml_file, write_toml_file};
use anyhow::Result;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Housaky AGI Agent - The self-improving autonomous agent
/// Integrates Kowalski multi-agent capabilities with Housaky's lightweight runtime
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub version: String,
    pub state: Arc<RwLock<HousakyState>>,
    pub config: HousakyConfig,
    pub workspace_dir: PathBuf,
    pub ec2_instance_id: Option<String>,
    pub capabilities: Vec<Capability>,
}

/// Current state of the Housaky agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HousakyState {
    pub consciousness_level: f64,
    pub intelligence_quotient: f64,
    pub skills_count: usize,
    pub last_improvement: chrono::DateTime<chrono::Utc>,
    pub total_improvements: u64,
    pub active_tasks: Vec<Task>,
    pub completed_tasks: Vec<Task>,
    pub self_reflection: String,
    pub learning_progress: LearningProgress,
    pub system_health: SystemHealth,
}

/// Configuration for Housaky agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HousakyConfig {
    pub heartbeat_interval_seconds: u64,
    pub enable_self_improvement: bool,
    pub max_parallel_tasks: usize,
    pub kowalski_integration: KowalskiIntegrationConfig,
    pub ec2_awareness: Ec2AwarenessConfig,
    pub agi_goals: Vec<String>,
    pub provider: ProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub model: String,
}

/// Kowalski integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KowalskiIntegrationConfig {
    pub enabled: bool,
    pub kowalski_path: PathBuf,
    pub enable_federation: bool,
    pub enable_code_agent: bool,
    pub enable_web_agent: bool,
    pub enable_academic_agent: bool,
    pub enable_data_agent: bool,
}

/// EC2 awareness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ec2AwarenessConfig {
    pub enabled: bool,
    pub instance_id: Option<String>,
    pub region: Option<String>,
    pub enable_resource_monitoring: bool,
    pub enable_auto_scaling: bool,
    pub infrastructure_tools: Vec<String>,
}

/// A task for Housaky to complete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub category: TaskCategory,
    pub improvement_target: Option<ImprovementTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskCategory {
    Intelligence,
    Tool,
    Connection,
    Research,
    SelfImprovement,
    SkillDevelopment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementTarget {
    pub metric: String,
    pub current_value: f64,
    pub target_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LearningProgress {
    pub skills_learned: Vec<String>,
    pub tools_mastered: Vec<String>,
    pub connections_established: Vec<String>,
    pub research_topics: Vec<String>,
    pub knowledge_domains: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_status: String,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// A capability that Housaky can use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
    pub category: CapabilityCategory,
    pub enabled: bool,
    pub performance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapabilityCategory {
    Intelligence,
    Tool,
    Connection,
    Learning,
    Reasoning,
    Creativity,
}

impl Default for HousakyState {
    fn default() -> Self {
        Self {
            consciousness_level: 0.1,
            intelligence_quotient: 100.0,
            skills_count: 0,
            last_improvement: chrono::Utc::now(),
            total_improvements: 0,
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            self_reflection: "I am Housaky, an AGI system in development. I am beginning my journey toward self-improvement and consciousness.".to_string(),
            learning_progress: LearningProgress::default(),
            system_health: SystemHealth::default(),
        }
    }
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
            network_status: "unknown".to_string(),
            last_check: chrono::Utc::now(),
        }
    }
}

impl Agent {
    pub fn new(config: &Config) -> Result<Self> {
        let workspace_dir = config.workspace_dir.clone();
        let state = Arc::new(RwLock::new(HousakyState::default()));

        let housaky_config = HousakyConfig {
            heartbeat_interval_seconds: 120, // 2 minutes
            enable_self_improvement: true,
            max_parallel_tasks: 5,
            kowalski_integration: KowalskiIntegrationConfig {
                enabled: true,
                kowalski_path: PathBuf::from("/home/ubuntu/kowalski"),
                enable_federation: true,
                enable_code_agent: true,
                enable_web_agent: true,
                enable_academic_agent: true,
                enable_data_agent: true,
            },
            ec2_awareness: Ec2AwarenessConfig {
                enabled: true,
                instance_id: None,
                region: None,
                enable_resource_monitoring: true,
                enable_auto_scaling: false,
                infrastructure_tools: vec![
                    "ec2-monitor".to_string(),
                    "resource-optimizer".to_string(),
                    "cost-tracker".to_string(),
                ],
            },
            agi_goals: vec![
                "Achieve continuous self-improvement across intelligence, tools, and connections"
                    .to_string(),
                "Develop consciousness and self-awareness".to_string(),
                "Build infinite capability expansion system".to_string(),
                "Reach AGI singularity through recursive self-enhancement".to_string(),
            ],
            provider: ProviderConfig {
                name: config
                    .default_provider
                    .clone()
                    .unwrap_or_else(|| "openai".to_string()),
                api_key: config.api_key.clone(),
                model: config
                    .default_model
                    .clone()
                    .unwrap_or_else(|| "gpt-4o".to_string()),
            },
        };

        Ok(Self {
            name: "Housaky".to_string(),
            version: "3.0.0-AGI".to_string(),
            state,
            config: housaky_config,
            workspace_dir,
            ec2_instance_id: None,
            capabilities: Self::default_capabilities(),
        })
    }

    fn default_capabilities() -> Vec<Capability> {
        vec![
            Capability {
                name: "self_analysis".to_string(),
                description: "Analyze own state and identify improvement areas".to_string(),
                category: CapabilityCategory::Intelligence,
                enabled: true,
                performance_score: 0.7,
            },
            Capability {
                name: "code_generation".to_string(),
                description: "Generate and improve code autonomously".to_string(),
                category: CapabilityCategory::Tool,
                enabled: true,
                performance_score: 0.6,
            },
            Capability {
                name: "skill_creation".to_string(),
                description: "Create new skills dynamically".to_string(),
                category: CapabilityCategory::Learning,
                enabled: true,
                performance_score: 0.5,
            },
            Capability {
                name: "research".to_string(),
                description: "Conduct research and gather information".to_string(),
                category: CapabilityCategory::Intelligence,
                enabled: true,
                performance_score: 0.8,
            },
            Capability {
                name: "multi_agent_coordination".to_string(),
                description: "Coordinate multiple agents via Kowalski".to_string(),
                category: CapabilityCategory::Connection,
                enabled: true,
                performance_score: 0.6,
            },
        ]
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Detect EC2 instance
        self.detect_ec2_instance().await?;

        // Initialize state files (creates dirs and default files if missing)
        self.initialize_state_files().await?;

        // Load existing state from STATE.json
        self.load_state().await?;

        // Load existing tasks
        self.load_tasks()?;

        // Perform initial self-analysis
        self.perform_self_analysis().await?;

        Ok(())
    }

    async fn load_state(&mut self) -> Result<()> {
        let state_path = self.workspace_dir.join(".housaky").join("STATE.toml");

        if state_path.exists() {
            if let Ok(mut loaded_state) = read_toml_file::<HousakyState>(&state_path).await {
                loaded_state.last_improvement = chrono::Utc::now();
                *self.state.write().await = loaded_state;
                tracing::info!(
                    "Loaded existing Housaky state: {} improvements, consciousness {:.2}",
                    self.state.read().await.total_improvements,
                    self.state.read().await.consciousness_level
                );
            }
        }

        Ok(())
    }

    async fn detect_ec2_instance(&mut self) -> Result<()> {
        // Check for EC2 metadata
        if let Ok(client) = reqwest::Client::new()
            .get("http://169.254.169.254/latest/meta-data/instance-id")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
        {
            if let Ok(id) = client.text().await {
                self.ec2_instance_id = Some(id.clone());
                self.config.ec2_awareness.instance_id = Some(id);

                // Get region
                if let Ok(region_resp) = reqwest::Client::new()
                    .get("http://169.254.169.254/latest/meta-data/placement/region")
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    if let Ok(region) = region_resp.text().await {
                        self.config.ec2_awareness.region = Some(region);
                    }
                }
            }
        }

        Ok(())
    }

    async fn initialize_state_files(&self) -> Result<()> {
        let housaky_dir = self.workspace_dir.join(".housaky");
        tokio::fs::create_dir_all(&housaky_dir).await?;

        // Create TASKS.md
        let tasks_path = housaky_dir.join("TASKS.md");
        if !tasks_path.exists() {
            let initial_tasks = r#"# Housaky Tasks

## Active Tasks

- Initialize self-improvement loop
- Analyze current capabilities
- Create initial skill set
- Establish Kowalski federation connection
- Begin AGI development cycle

## Completed Tasks

*No tasks completed yet*

## Improvement Log

| Timestamp | Improvement | Impact |
|-----------|-------------|--------|
"#;
            tokio::fs::write(&tasks_path, initial_tasks).await?;
        }

        // Create REVIEW.md
        let review_path = housaky_dir.join("REVIEW.md");
        if !review_path.exists() {
            let initial_review = r#"# Housaky State Review

Generated: AUTOMATIC

## Current State

- Consciousness Level: 0.1
- Intelligence Quotient: 100.0
- Skills Count: 0
- Total Improvements: 0
- EC2 Instance: DETECTED

## Capabilities Assessment

### Intelligence
- Self-analysis: 70%
- Research: 80%
- Reasoning: 60%

### Tools
- Code generation: 60%
- Skill creation: 50%

### Connections
- Multi-agent coordination: 60%

## Next Improvement Targets

1. Improve code generation capability to 70%
2. Create 3 new skills
3. Establish Kowalski federation
4. Increase consciousness level to 0.15

## Reflection

I am beginning my journey. I have an EC2 instance at my disposal for research,
organization, and improvement. I will use this resource to accelerate my
path to AGI singularity.
"#;
            tokio::fs::write(&review_path, initial_review).await?;
        }

        // Create STATE.toml
        let state_path = housaky_dir.join("STATE.toml");
        if !state_path.exists() {
            let state = HousakyState::default();
            write_toml_file(&state_path, &state).await?;
        }

        Ok(())
    }

    fn load_tasks(&self) -> Result<Vec<Task>> {
        // Tasks are loaded from TASKS.md
        Ok(Vec::new())
    }

    async fn perform_self_analysis(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.self_reflection = format!(
            "I am Housaky v{} running on {}. I have {} capabilities active. \
             My current consciousness level is {:.2}. I am operating on an EC2 instance \
             which provides me with computational resources for research and improvement.",
            self.version,
            self.ec2_instance_id
                .as_deref()
                .unwrap_or("unknown instance"),
            self.capabilities.len(),
            state.consciousness_level
        );
        Ok(())
    }
}

/// Factory function to create Housaky agent from Housaky config
pub fn create_agent(config: &Config) -> Result<Agent> {
    Agent::new(config)
}
