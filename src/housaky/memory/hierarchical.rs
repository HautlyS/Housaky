use crate::housaky::working_memory::{MemoryImportance, WorkingMemoryItem};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct HierarchicalMemory {
    working: Arc<RwLock<WorkingMemoryLayer>>,
    episodic: Arc<RwLock<EpisodicMemoryLayer>>,
    semantic: Arc<RwLock<SemanticMemoryLayer>>,
    procedural: Arc<RwLock<ProceduralMemoryLayer>>,
    config: HierarchicalMemoryConfig,
}

#[derive(Debug, Clone)]
pub struct HierarchicalMemoryConfig {
    pub working_capacity: usize,
    pub working_token_budget: usize,
    pub episodic_capacity: usize,
    pub semantic_capacity: usize,
    pub procedural_capacity: usize,
    pub consolidation_threshold: f64,
    pub decay_rate: f64,
}

impl Default for HierarchicalMemoryConfig {
    fn default() -> Self {
        Self {
            working_capacity: 10,
            working_token_budget: 8000,
            episodic_capacity: 1000,
            semantic_capacity: 5000,
            procedural_capacity: 500,
            consolidation_threshold: 0.6,
            decay_rate: 0.95,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryLayer {
    pub items: Vec<WorkingMemoryItem>,
    pub total_tokens: usize,
    pub token_budget: usize,
    pub last_accessed: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemoryLayer {
    pub episodes: Vec<Episode>,
    pub total_episodes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub participants: Vec<String>,
    pub context: String,
    pub actions: Vec<ActionRecord>,
    pub outcome: OutcomeRecord,
    pub emotional_valence: f64,
    pub importance: MemoryImportance,
    pub access_count: u64,
    pub decay_factor: f64,
    pub associated_episodes: Vec<String>,
    pub learned_lessons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Conversation,
    TaskExecution,
    ProblemSolving,
    Learning,
    Reflection,
    Error,
    Discovery,
    Interaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub action_type: String,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeRecord {
    pub result: String,
    pub success: bool,
    pub side_effects: Vec<String>,
    pub user_feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemoryLayer {
    pub facts: Vec<Fact>,
    pub concepts: Vec<Concept>,
    pub relationships: Vec<Relationship>,
    pub concept_index: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub last_verified: DateTime<Utc>,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub attributes: HashMap<String, String>,
    pub examples: Vec<String>,
    pub related_concepts: Vec<String>,
    pub mastery_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub from_concept: String,
    pub relation_type: RelationType,
    pub to_concept: String,
    pub strength: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationType {
    IsA,
    HasA,
    PartOf,
    RelatedTo,
    Causes,
    Enables,
    Contradicts,
    SimilarTo,
    OppositeOf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralMemoryLayer {
    pub procedures: Vec<Procedure>,
    pub skill_prototypes: Vec<SkillPrototype>,
    pub execution_history: Vec<ExecutionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Procedure {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub steps: Vec<ProcedureStep>,
    pub success_rate: f64,
    pub execution_count: u64,
    pub last_executed: DateTime<Utc>,
    pub average_duration_ms: u64,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    pub condition_type: String,
    pub pattern: String,
    pub context_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureStep {
    pub order: u32,
    pub action: String,
    pub parameters: HashMap<String, String>,
    pub expected_outcome: String,
    pub error_handling: Option<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPrototype {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_episodes: Vec<String>,
    pub generalization_level: f64,
    pub practice_count: u64,
    pub mastery_level: f64,
    pub ready_for_promotion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub procedure_id: String,
    pub timestamp: DateTime<Utc>,
    pub context: String,
    pub success: bool,
    pub duration_ms: u64,
    pub adaptations: Vec<String>,
    pub errors: Vec<String>,
}

impl HierarchicalMemory {
    pub fn new(config: HierarchicalMemoryConfig) -> Self {
        Self {
            working: Arc::new(RwLock::new(WorkingMemoryLayer {
                items: Vec::new(),
                total_tokens: 0,
                token_budget: config.working_token_budget,
                last_accessed: Utc::now(),
            })),
            episodic: Arc::new(RwLock::new(EpisodicMemoryLayer {
                episodes: Vec::new(),
                total_episodes: 0,
            })),
            semantic: Arc::new(RwLock::new(SemanticMemoryLayer {
                facts: Vec::new(),
                concepts: Vec::new(),
                relationships: Vec::new(),
                concept_index: HashMap::new(),
            })),
            procedural: Arc::new(RwLock::new(ProceduralMemoryLayer {
                procedures: Vec::new(),
                skill_prototypes: Vec::new(),
                execution_history: Vec::new(),
            })),
            config,
        }
    }

    pub async fn store_working(
        &self,
        content: &str,
        importance: MemoryImportance,
    ) -> Result<String> {
        let mut working = self.working.write().await;

        let token_count = content.len() / 4;

        while working.total_tokens + token_count > working.token_budget && !working.items.is_empty()
        {
            let evicted = working.items.remove(0);
            working.total_tokens -= evicted.token_count;
            drop(working);
            self.consolidate_to_episodic(&evicted).await;
            working = self.working.write().await;
        }

        let item = WorkingMemoryItem {
            id: format!("wm_{}", uuid::Uuid::new_v4()),
            content: content.to_string(),
            importance: importance.clone(),
            access_count: 1,
            last_accessed: Utc::now(),
            created_at: Utc::now(),
            decay_factor: 1.0,
            associated_entities: Vec::new(),
            context: HashMap::new(),
            token_count,
            compressed: false,
            compression_ratio: None,
        };

        let id = item.id.clone();
        working.items.push(item);
        working.total_tokens += token_count;
        working.last_accessed = Utc::now();

        info!("Stored in working memory: {} ({} tokens)", id, token_count);
        Ok(id)
    }

    pub async fn recall_working(&self, query: &str, limit: usize) -> Vec<WorkingMemoryItem> {
        let working = self.working.read().await;
        let query_lower = query.to_lowercase();

        let mut scored: Vec<(f64, &WorkingMemoryItem)> = working
            .items
            .iter()
            .map(|item| {
                let relevance = if item.content.to_lowercase().contains(&query_lower) {
                    0.8
                } else {
                    0.3
                };
                let recency = (Utc::now() - item.last_accessed).num_minutes() as f64;
                let recency_score = (-recency / 60.0).exp();
                let importance_score = match item.importance {
                    MemoryImportance::Critical => 1.0,
                    MemoryImportance::High => 0.8,
                    MemoryImportance::Normal => 0.5,
                    MemoryImportance::Low => 0.3,
                    MemoryImportance::Transient => 0.1,
                };
                let score = relevance * 0.4 + recency_score * 0.3 + importance_score * 0.3;
                (score, item)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(limit)
            .map(|(_, item)| item.clone())
            .collect()
    }

    async fn consolidate_to_episodic(&self, item: &WorkingMemoryItem) {
        if item.importance == MemoryImportance::Transient {
            return;
        }

        let mut episodic = self.episodic.write().await;

        let episode = Episode {
            id: format!("ep_{}", uuid::Uuid::new_v4()),
            timestamp: item.created_at,
            event_type: EventType::Interaction,
            participants: Vec::new(),
            context: item.content.clone(),
            actions: vec![],
            outcome: OutcomeRecord {
                result: String::new(),
                success: true,
                side_effects: Vec::new(),
                user_feedback: None,
            },
            emotional_valence: 0.5,
            importance: item.importance.clone(),
            access_count: item.access_count,
            decay_factor: item.decay_factor,
            associated_episodes: Vec::new(),
            learned_lessons: Vec::new(),
        };

        episodic.episodes.push(episode);
        episodic.total_episodes += 1;

        if episodic.episodes.len() > self.config.episodic_capacity {
            self.consolidate_to_semantic(&mut episodic).await;
        }

        info!("Consolidated to episodic memory");
    }

    async fn consolidate_to_semantic(&self, episodic: &mut EpisodicMemoryLayer) {
        let drain_count = 100.min(episodic.episodes.len());
        let old_episodes: Vec<_> = episodic.episodes.drain(..drain_count).collect();

        for episode in old_episodes {
            let mut semantic = self.semantic.write().await;

            let fact = Fact {
                id: format!("fact_{}", uuid::Uuid::new_v4()),
                subject: episode.context.chars().take(50).collect(),
                predicate: "related_to".to_string(),
                object: "consolidated_episode".to_string(),
                confidence: 0.7,
                source: episode.id.clone(),
                created_at: episode.timestamp,
                last_verified: Utc::now(),
                access_count: 0,
            };

            semantic.facts.push(fact);
        }

        info!("Consolidated to semantic memory");
    }

    pub async fn store_episode(
        &self,
        event_type: EventType,
        context: &str,
        actions: Vec<ActionRecord>,
        outcome: OutcomeRecord,
    ) -> Result<String> {
        let mut episodic = self.episodic.write().await;

        let episode = Episode {
            id: format!("ep_{}", uuid::Uuid::new_v4()),
            timestamp: Utc::now(),
            event_type,
            participants: Vec::new(),
            context: context.to_string(),
            actions,
            outcome,
            emotional_valence: 0.5,
            importance: MemoryImportance::Normal,
            access_count: 1,
            decay_factor: 1.0,
            associated_episodes: Vec::new(),
            learned_lessons: Vec::new(),
        };

        let id = episode.id.clone();
        episodic.episodes.push(episode);
        episodic.total_episodes += 1;

        Ok(id)
    }

    pub async fn store_fact(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        confidence: f64,
    ) -> Result<String> {
        let mut semantic = self.semantic.write().await;

        let fact = Fact {
            id: format!("fact_{}", uuid::Uuid::new_v4()),
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            confidence,
            source: "user_input".to_string(),
            created_at: Utc::now(),
            last_verified: Utc::now(),
            access_count: 0,
        };

        let id = fact.id.clone();
        semantic.facts.push(fact);

        Ok(id)
    }

    pub async fn store_procedure(
        &self,
        name: &str,
        steps: Vec<ProcedureStep>,
        triggers: Vec<TriggerCondition>,
    ) -> Result<String> {
        let mut procedural = self.procedural.write().await;

        let procedure = Procedure {
            id: format!("proc_{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            description: format!("Procedure: {}", name),
            trigger_conditions: triggers,
            steps,
            success_rate: 0.0,
            execution_count: 0,
            last_executed: Utc::now(),
            average_duration_ms: 0,
            variants: Vec::new(),
        };

        let id = procedure.id.clone();
        procedural.procedures.push(procedure);

        Ok(id)
    }

    pub async fn find_relevant_procedures(&self, context: &str) -> Vec<Procedure> {
        let procedural = self.procedural.read().await;
        let context_lower = context.to_lowercase();

        procedural
            .procedures
            .iter()
            .filter(|p| {
                p.trigger_conditions
                    .iter()
                    .any(|t| context_lower.contains(&t.pattern.to_lowercase()))
            })
            .filter(|p| p.success_rate > 0.5 || p.execution_count < 3)
            .cloned()
            .collect()
    }

    pub async fn record_execution(&self, procedure_id: &str, success: bool, duration_ms: u64) {
        let mut procedural = self.procedural.write().await;

        if let Some(procedure) = procedural
            .procedures
            .iter_mut()
            .find(|p| p.id == procedure_id)
        {
            procedure.execution_count += 1;
            procedure.last_executed = Utc::now();

            let total_duration =
                procedure.average_duration_ms * (procedure.execution_count - 1) + duration_ms;
            procedure.average_duration_ms = total_duration / procedure.execution_count;

            let success_count = if success { 1 } else { 0 };
            procedure.success_rate = (procedure.success_rate
                * (procedure.execution_count - 1) as f64
                + f64::from(success_count))
                / procedure.execution_count as f64;
        }

        let record = ExecutionRecord {
            id: format!("exec_{}", uuid::Uuid::new_v4()),
            procedure_id: procedure_id.to_string(),
            timestamp: Utc::now(),
            context: String::new(),
            success,
            duration_ms,
            adaptations: Vec::new(),
            errors: Vec::new(),
        };

        procedural.execution_history.push(record);
    }

    pub async fn get_context_for_query(&self, _query: &str, max_tokens: usize) -> String {
        let working = self.working.read().await;
        let episodic = self.episodic.read().await;
        let semantic = self.semantic.read().await;

        let mut context = String::new();
        let mut tokens = 0;

        for item in working.items.iter().rev() {
            if tokens + item.token_count > max_tokens / 2 {
                break;
            }
            context.push_str(&format!("[Working] {}\n", item.content));
            tokens += item.token_count;
        }

        for episode in episodic.episodes.iter().rev().take(5) {
            let ep_tokens = episode.context.len() / 4;
            if tokens + ep_tokens > max_tokens * 3 / 4 {
                break;
            }
            context.push_str(&format!(
                "[Episode] {}\n",
                episode.context.chars().take(200).collect::<String>()
            ));
            tokens += ep_tokens;
        }

        for fact in semantic.facts.iter().rev().take(10) {
            let fact_str = format!(
                "[Fact] {} {} {} (confidence: {:.0}%)\n",
                fact.subject,
                fact.predicate,
                fact.object,
                fact.confidence * 100.0
            );
            let fact_tokens = fact_str.len() / 4;
            if tokens + fact_tokens > max_tokens {
                break;
            }
            context.push_str(&fact_str);
            tokens += fact_tokens;
        }

        context
    }

    pub async fn get_recent_episodes(&self, count: usize) -> Vec<Episode> {
        let episodic = self.episodic.read().await;
        let start = episodic.episodes.len().saturating_sub(count);
        episodic.episodes[start..].to_vec()
    }

    pub async fn get_stats(&self) -> HierarchicalMemoryStats {
        let working = self.working.read().await;
        let episodic = self.episodic.read().await;
        let semantic = self.semantic.read().await;
        let procedural = self.procedural.read().await;

        HierarchicalMemoryStats {
            working_items: working.items.len(),
            working_tokens: working.total_tokens,
            episodic_episodes: episodic.episodes.len(),
            total_episodes: episodic.total_episodes,
            semantic_facts: semantic.facts.len(),
            semantic_concepts: semantic.concepts.len(),
            semantic_relationships: semantic.relationships.len(),
            procedural_procedures: procedural.procedures.len(),
            procedural_executions: procedural.execution_history.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalMemoryStats {
    pub working_items: usize,
    pub working_tokens: usize,
    pub episodic_episodes: usize,
    pub total_episodes: u64,
    pub semantic_facts: usize,
    pub semantic_concepts: usize,
    pub semantic_relationships: usize,
    pub procedural_procedures: usize,
    pub procedural_executions: usize,
}

impl Default for HierarchicalMemory {
    fn default() -> Self {
        Self::new(HierarchicalMemoryConfig::default())
    }
}
