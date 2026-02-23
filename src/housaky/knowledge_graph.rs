use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityId(String);

impl EntityId {
    pub fn new(name: &str) -> Self {
        Self(format!("entity_{}", name.to_lowercase().replace(' ', "_")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub entity_type: EntityType,
    pub aliases: Vec<String>,
    pub attributes: HashMap<String, String>,
    pub importance: f64,
    pub access_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityType {
    Concept,
    Person,
    Organization,
    Technology,
    Project,
    Skill,
    Tool,
    API,
    Document,
    Code,
    Event,
    Location,
    Time,
    Resource,
    Problem,
    Solution,
    Goal,
    Memory,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub from_entity: EntityId,
    pub to_entity: EntityId,
    pub relation_type: RelationType,
    pub weight: f64,
    pub attributes: HashMap<String, String>,
    pub evidence: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationType {
    IsA,
    HasA,
    PartOf,
    RelatedTo,
    DependsOn,
    Implements,
    Uses,
    Creates,
    Modifies,
    Causes,
    Precedes,
    Follows,
    SimilarTo,
    DifferentFrom,
    Contains,
    BelongsTo,
    Knows,
    Mentions,
    References,
    Contradicts,
    Supports,
    Requires,
    Produces,
    Custom(String),
}

impl RelationType {
    pub fn as_str(&self) -> &str {
        match self {
            RelationType::IsA => "is_a",
            RelationType::HasA => "has_a",
            RelationType::PartOf => "part_of",
            RelationType::RelatedTo => "related_to",
            RelationType::DependsOn => "depends_on",
            RelationType::Implements => "implements",
            RelationType::Uses => "uses",
            RelationType::Creates => "creates",
            RelationType::Modifies => "modifies",
            RelationType::Causes => "causes",
            RelationType::Precedes => "precedes",
            RelationType::Follows => "follows",
            RelationType::SimilarTo => "similar_to",
            RelationType::DifferentFrom => "different_from",
            RelationType::Contains => "contains",
            RelationType::BelongsTo => "belongs_to",
            RelationType::Knows => "knows",
            RelationType::Mentions => "mentions",
            RelationType::References => "references",
            RelationType::Contradicts => "contradicts",
            RelationType::Supports => "supports",
            RelationType::Requires => "requires",
            RelationType::Produces => "produces",
            RelationType::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub entities: HashMap<EntityId, Entity>,
    pub relations: Vec<Relation>,
    pub index: HashMap<EntityId, Vec<usize>>,
    pub reverse_index: HashMap<EntityId, Vec<usize>>,
    pub type_index: HashMap<String, Vec<EntityId>>,
    pub last_compacted: DateTime<Utc>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            relations: Vec::new(),
            index: HashMap::new(),
            reverse_index: HashMap::new(),
            type_index: HashMap::new(),
            last_compacted: Utc::now(),
        }
    }
}

pub struct KnowledgeGraphEngine {
    graph: Arc<RwLock<KnowledgeGraph>>,
    workspace_dir: PathBuf,
    max_entities: usize,
    max_relations: usize,
    decay_factor: f64,
}

impl KnowledgeGraphEngine {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        Self {
            graph: Arc::new(RwLock::new(KnowledgeGraph::new())),
            workspace_dir: workspace_dir.clone(),
            max_entities: 10000,
            max_relations: 50000,
            decay_factor: 0.99,
        }
    }

    pub async fn add_entity(
        &self,
        name: &str,
        entity_type: EntityType,
        source: &str,
    ) -> Result<EntityId> {
        let mut graph = self.graph.write().await;

        let id = EntityId::new(name);

        if let Some(existing) = graph.entities.get_mut(&id) {
            existing.access_count += 1;
            existing.importance = (existing.importance + 0.1).min(1.0);
            existing.updated_at = Utc::now();
            return Ok(id);
        }

        if graph.entities.len() >= self.max_entities {
            self.prune_low_importance_entities(&mut graph);
        }

        let entity = Entity {
            id: id.clone(),
            name: name.to_string(),
            entity_type: entity_type.clone(),
            aliases: Vec::new(),
            attributes: HashMap::new(),
            importance: 0.5,
            access_count: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source: source.to_string(),
            confidence: 0.8,
        };

        let type_key = format!("{:?}", entity_type);
        graph
            .type_index
            .entry(type_key)
            .or_default()
            .push(id.clone());
        graph.entities.insert(id.clone(), entity);

        info!("Added entity: {} ({:?})", name, entity_type);

        self.save_graph(&graph).await?;

        Ok(id)
    }

    fn prune_low_importance_entities(&self, graph: &mut KnowledgeGraph) {
        let to_remove: Vec<EntityId> = graph
            .entities
            .iter()
            .filter(|(_, e)| e.importance < 0.2 && e.access_count < 3)
            .map(|(id, _)| id.clone())
            .take(graph.entities.len() / 10)
            .collect();

        for id in &to_remove {
            graph.entities.remove(id);

            if let Some(relation_indices) = graph.index.remove(id) {
                for idx in relation_indices.iter().rev() {
                    if *idx < graph.relations.len() {
                        graph.relations.remove(*idx);
                    }
                }
            }

            graph.reverse_index.remove(id);
        }

        info!("Pruned {} low-importance entities", to_remove.len());
    }

    pub async fn add_relation(
        &self,
        from: &EntityId,
        to: &EntityId,
        relation_type: RelationType,
        evidence: Option<&str>,
    ) -> Result<String> {
        let mut graph = self.graph.write().await;

        if !graph.entities.contains_key(from) || !graph.entities.contains_key(to) {
            return Err(anyhow::anyhow!("One or both entities not found"));
        }

        if graph.relations.len() >= self.max_relations {
            self.prune_weak_relations(&mut graph);
        }

        let relation = Relation {
            id: format!("rel_{}", uuid::Uuid::new_v4()),
            from_entity: from.clone(),
            to_entity: to.clone(),
            relation_type: relation_type.clone(),
            weight: 0.5,
            attributes: HashMap::new(),
            evidence: evidence.map(|s| vec![s.to_string()]).unwrap_or_default(),
            created_at: Utc::now(),
            confidence: 0.7,
        };

        let relation_id = relation.id.clone();
        let relation_idx = graph.relations.len();

        graph
            .index
            .entry(from.clone())
            .or_default()
            .push(relation_idx);
        graph
            .reverse_index
            .entry(to.clone())
            .or_default()
            .push(relation_idx);
        graph.relations.push(relation);

        if let Some(from_entity) = graph.entities.get_mut(from) {
            from_entity.updated_at = Utc::now();
        }
        if let Some(to_entity) = graph.entities.get_mut(to) {
            to_entity.updated_at = Utc::now();
        }

        info!(
            "Added relation: {:?} --{:?}-> {:?}",
            from, relation_type, to
        );

        self.save_graph(&graph).await?;

        Ok(relation_id)
    }

    fn prune_weak_relations(&self, graph: &mut KnowledgeGraph) {
        let initial_len = graph.relations.len();

        graph
            .relations
            .retain(|r| r.weight >= 0.2 && r.confidence >= 0.3);

        graph.index.clear();
        graph.reverse_index.clear();

        for (idx, relation) in graph.relations.iter().enumerate() {
            graph
                .index
                .entry(relation.from_entity.clone())
                .or_default()
                .push(idx);
            graph
                .reverse_index
                .entry(relation.to_entity.clone())
                .or_default()
                .push(idx);
        }

        info!(
            "Pruned {} weak relations",
            initial_len - graph.relations.len()
        );
    }

    pub async fn get_entity(&self, id: &EntityId) -> Option<Entity> {
        let graph = self.graph.read().await;
        graph.entities.get(id).cloned()
    }

    pub async fn find_entity_by_name(&self, name: &str) -> Option<Entity> {
        let graph = self.graph.read().await;
        let name_lower = name.to_lowercase();

        graph
            .entities
            .values()
            .find(|e| {
                e.name.to_lowercase() == name_lower
                    || e.aliases.iter().any(|a| a.to_lowercase() == name_lower)
            })
            .cloned()
    }

    pub async fn get_related_entities(
        &self,
        id: &EntityId,
        max_depth: usize,
    ) -> Vec<(Entity, RelationType, usize)> {
        let graph = self.graph.read().await;
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue: std::collections::VecDeque<(EntityId, usize)> =
            std::collections::VecDeque::new();

        queue.push_back((id.clone(), 0));
        visited.insert(id.clone());

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            if let Some(relation_indices) = graph.index.get(&current_id) {
                for &idx in relation_indices {
                    if let Some(relation) = graph.relations.get(idx) {
                        if !visited.contains(&relation.to_entity) {
                            visited.insert(relation.to_entity.clone());

                            if let Some(entity) = graph.entities.get(&relation.to_entity) {
                                result.push((
                                    entity.clone(),
                                    relation.relation_type.clone(),
                                    depth + 1,
                                ));
                            }

                            queue.push_back((relation.to_entity.clone(), depth + 1));
                        }
                    }
                }
            }
        }

        result
    }

    pub async fn find_path(
        &self,
        from: &EntityId,
        to: &EntityId,
    ) -> Option<Vec<(Entity, RelationType)>> {
        let graph = self.graph.read().await;
        let mut visited = HashSet::new();
        let mut parent: HashMap<EntityId, (EntityId, RelationType)> = HashMap::new();
        let mut queue: std::collections::VecDeque<EntityId> = std::collections::VecDeque::new();

        queue.push_back(from.clone());
        visited.insert(from.clone());

        while let Some(current) = queue.pop_front() {
            if current == *to {
                let mut path = Vec::new();
                let mut current = to.clone();

                while let Some((prev, rel_type)) = parent.get(&current) {
                    if let Some(entity) = graph.entities.get(&current) {
                        path.push((entity.clone(), rel_type.clone()));
                    }
                    current = prev.clone();
                }

                path.reverse();
                return Some(path);
            }

            if let Some(relation_indices) = graph.index.get(&current) {
                for &idx in relation_indices {
                    if let Some(relation) = graph.relations.get(idx) {
                        if !visited.contains(&relation.to_entity) {
                            visited.insert(relation.to_entity.clone());
                            parent.insert(
                                relation.to_entity.clone(),
                                (current.clone(), relation.relation_type.clone()),
                            );
                            queue.push_back(relation.to_entity.clone());
                        }
                    }
                }
            }
        }

        None
    }

    pub async fn search(&self, query: &str, limit: usize) -> Vec<Entity> {
        let graph = self.graph.read().await;
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored_entities: Vec<(f64, &Entity)> = graph
            .entities
            .values()
            .map(|e| {
                let mut score = 0.0;

                for term in &query_terms {
                    if e.name.to_lowercase().contains(term) {
                        score += 2.0;
                    }

                    if e.aliases.iter().any(|a| a.to_lowercase().contains(term)) {
                        score += 1.0;
                    }

                    for attr_value in e.attributes.values() {
                        if attr_value.to_lowercase().contains(term) {
                            score += 0.5;
                        }
                    }
                }

                score *= e.importance;
                (score, e)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored_entities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored_entities
            .iter()
            .take(limit)
            .map(|(_, e)| (*e).clone())
            .collect()
    }

    pub async fn query(&self, query: GraphQuery) -> Vec<Entity> {
        let graph = self.graph.read().await;

        match query {
            GraphQuery::ByType(entity_type) => {
                let type_key = format!("{:?}", entity_type);
                graph
                    .type_index
                    .get(&type_key)
                    .map(|ids| {
                        ids.iter()
                            .filter_map(|id| graph.entities.get(id))
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default()
            }
            GraphQuery::ByAttribute(key, value) => graph
                .entities
                .values()
                .filter(|e| {
                    e.attributes
                        .get(&key)
                        .map(|v| v.contains(&value))
                        .unwrap_or(false)
                })
                .cloned()
                .collect(),
            GraphQuery::RelatedTo(entity_id, relation_type) => graph
                .index
                .get(&entity_id)
                .map(|indices| {
                    indices
                        .iter()
                        .filter_map(|&idx| graph.relations.get(idx))
                        .filter(|r| {
                            relation_type
                                .as_ref()
                                .map_or(true, |rt| r.relation_type == *rt)
                        })
                        .filter_map(|r| graph.entities.get(&r.to_entity))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default(),
            GraphQuery::MostImportant(n) => {
                let mut entities: Vec<_> = graph.entities.values().cloned().collect();
                entities.sort_by(|a, b| {
                    b.importance
                        .partial_cmp(&a.importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                entities.into_iter().take(n).collect()
            }
            GraphQuery::Recent(n) => {
                let mut entities: Vec<_> = graph.entities.values().cloned().collect();
                entities.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                entities.into_iter().take(n).collect()
            }
        }
    }

    pub async fn extract_from_text(&self, text: &str, source: &str) -> Result<Vec<EntityId>> {
        let mut entity_ids = Vec::new();

        let patterns = [
            (r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)\b", EntityType::Concept),
            (
                r"\b(Rust|Python|JavaScript|TypeScript|Go|Java|C\+\+|Ruby)\b",
                EntityType::Technology,
            ),
            (
                r"\b(API|SDK|CLI|TUI|GUI|HTTP|REST|GraphQL)\b",
                EntityType::API,
            ),
            (
                r"\b(GitHub|AWS|Azure|GCP|Docker|Kubernetes)\b",
                EntityType::Organization,
            ),
        ];

        for (pattern, entity_type) in patterns {
            let re = regex::Regex::new(pattern).unwrap();
            for cap in re.captures_iter(text) {
                if let Some(name) = cap.get(1) {
                    let id = self
                        .add_entity(name.as_str(), entity_type.clone(), source)
                        .await?;
                    entity_ids.push(id);
                }
            }
        }

        if entity_ids.len() >= 2 {
            for i in 0..entity_ids.len() - 1 {
                for j in i + 1..entity_ids.len() {
                    if self
                        .should_create_relation(&entity_ids[i], &entity_ids[j])
                        .await
                    {
                        let _ = self
                            .add_relation(
                                &entity_ids[i],
                                &entity_ids[j],
                                RelationType::RelatedTo,
                                Some(&format!("Co-occurred in: {}", source)),
                            )
                            .await;
                    }
                }
            }
        }

        Ok(entity_ids)
    }

    async fn should_create_relation(&self, from: &EntityId, to: &EntityId) -> bool {
        // Never create self-loops
        if from == to {
            return false;
        }

        let graph = self.graph.read().await;

        // Skip if either entity is missing
        let from_entity = match graph.entities.get(from) {
            Some(e) => e.clone(),
            None => return false,
        };
        let to_entity = match graph.entities.get(to) {
            Some(e) => e.clone(),
            None => return false,
        };

        // Skip if the relation already exists in either direction
        let already_exists = graph.relations.iter().any(|r| {
            (r.from_entity == *from && r.to_entity == *to)
                || (r.from_entity == *to && r.to_entity == *from)
        });
        if already_exists {
            return false;
        }

        // Only link entities whose type pairing is semantically meaningful
        use EntityType::{Concept, Organization, Technology, API};
        let meaningful = matches!(
            (&from_entity.entity_type, &to_entity.entity_type),
            (Technology | API | Concept | Organization, Technology)
                | (Concept | Technology, Concept)
                | (Technology, API | Organization)
        );

        meaningful
    }

    pub async fn get_stats(&self) -> GraphStats {
        let graph = self.graph.read().await;

        let entity_type_counts: HashMap<String, usize> =
            graph.entities.values().fold(HashMap::new(), |mut acc, e| {
                let type_name = format!("{:?}", e.entity_type);
                *acc.entry(type_name).or_insert(0) += 1;
                acc
            });

        let relation_type_counts: HashMap<String, usize> =
            graph.relations.iter().fold(HashMap::new(), |mut acc, r| {
                *acc.entry(r.relation_type.as_str().to_string()).or_insert(0) += 1;
                acc
            });

        GraphStats {
            entity_count: graph.entities.len(),
            relation_count: graph.relations.len(),
            entity_type_distribution: entity_type_counts,
            relation_type_distribution: relation_type_counts,
            avg_importance: graph.entities.values().map(|e| e.importance).sum::<f64>()
                / graph.entities.len().max(1) as f64,
            avg_confidence: graph.relations.iter().map(|r| r.confidence).sum::<f64>()
                / graph.relations.len().max(1) as f64,
        }
    }

    pub async fn save_graph(&self, graph: &KnowledgeGraph) -> Result<()> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join("knowledge_graph.json");

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(graph)?;
        tokio::fs::write(&path, json).await?;

        Ok(())
    }

    pub async fn load_graph(&self) -> Result<()> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join("knowledge_graph.json");

        if !path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let loaded: KnowledgeGraph = serde_json::from_str(&content)?;

        let mut graph = self.graph.write().await;
        *graph = loaded;

        info!(
            "Loaded knowledge graph with {} entities and {} relations",
            graph.entities.len(),
            graph.relations.len()
        );

        Ok(())
    }

    pub async fn decay_importance(&self) {
        let mut graph = self.graph.write().await;

        for entity in graph.entities.values_mut() {
            entity.importance *= self.decay_factor;
        }

        for relation in &mut graph.relations {
            relation.weight *= self.decay_factor;
        }

        info!("Applied importance decay to knowledge graph");
    }

    pub async fn save(&self) -> Result<()> {
        let graph = self.graph.read().await;
        self.save_graph(&graph).await
    }

    pub async fn persist(&self) -> Result<()> {
        let graph = self.graph.read().await;
        self.save_graph(&graph).await
    }

    pub async fn export_dot(&self) -> String {
        let graph = self.graph.read().await;

        let mut dot = String::from("digraph KnowledgeGraph {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n\n");

        for entity in graph.entities.values() {
            let label = entity.name.replace('"', "\\\"");
            let color = match entity.importance {
                x if x >= 0.8 => "gold",
                x if x >= 0.5 => "lightblue",
                _ => "lightgray",
            };
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\" style=filled fillcolor={}];\n",
                entity.id.as_str(),
                label,
                color
            ));
        }

        dot.push('\n');

        for relation in &graph.relations {
            let label = relation.relation_type.as_str();
            let style = if relation.confidence >= 0.7 {
                "solid"
            } else {
                "dashed"
            };
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\" style={}];\n",
                relation.from_entity.as_str(),
                relation.to_entity.as_str(),
                label,
                style
            ));
        }

        dot.push_str("}\n");
        dot
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphQuery {
    ByType(EntityType),
    ByAttribute(String, String),
    RelatedTo(EntityId, Option<RelationType>),
    MostImportant(usize),
    Recent(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub entity_count: usize,
    pub relation_count: usize,
    pub entity_type_distribution: HashMap<String, usize>,
    pub relation_type_distribution: HashMap<String, usize>,
    pub avg_importance: f64,
    pub avg_confidence: f64,
}

impl Default for KnowledgeGraphEngine {
    fn default() -> Self {
        Self::new(&PathBuf::from("."))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRule {
    pub id: String,
    pub name: String,
    pub premise: Vec<RulePattern>,
    pub conclusion: RulePattern,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePattern {
    pub entity_type: Option<String>,
    pub entity_name: Option<String>,
    pub relation: Option<String>,
    pub target_type: Option<String>,
    pub target_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub inferred_entities: Vec<Entity>,
    pub inferred_relations: Vec<Relation>,
    pub confidence: f64,
    pub rules_applied: Vec<String>,
    pub reasoning_chain: Vec<ReasoningStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub rule_id: String,
    pub premise_matches: Vec<String>,
    pub conclusion: String,
    pub confidence: f64,
}

pub struct InferenceEngine {
    rules: Vec<InferenceRule>,
    max_depth: usize,
    enable_chain_reasoning: bool,
}

impl InferenceEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            rules: Vec::new(),
            max_depth: 5,
            enable_chain_reasoning: true,
        };
        engine.add_default_rules();
        engine
    }

    fn add_default_rules(&mut self) {
        self.rules.push(InferenceRule {
            id: "rule_isa_transitive".to_string(),
            name: "Is-A Transitivity".to_string(),
            premise: vec![
                RulePattern {
                    entity_type: None,
                    entity_name: None,
                    relation: Some("IsA".to_string()),
                    target_type: Some("Concept".to_string()),
                    target_name: None,
                },
                RulePattern {
                    entity_type: Some("Concept".to_string()),
                    entity_name: None,
                    relation: Some("IsA".to_string()),
                    target_type: None,
                    target_name: None,
                },
            ],
            conclusion: RulePattern {
                entity_type: None,
                entity_name: None,
                relation: Some("RelatedTo".to_string()),
                target_type: None,
                target_name: None,
            },
            confidence: 0.8,
            evidence: vec!["Transitive closure of Is-A relation".to_string()],
        });

        self.rules.push(InferenceRule {
            id: "rule_uses_implies_depends".to_string(),
            name: "Uses Implies Depends".to_string(),
            premise: vec![RulePattern {
                entity_type: None,
                entity_name: None,
                relation: Some("Uses".to_string()),
                target_type: None,
                target_name: None,
            }],
            conclusion: RulePattern {
                entity_type: None,
                entity_name: None,
                relation: Some("DependsOn".to_string()),
                target_type: None,
                target_name: None,
            },
            confidence: 0.7,
            evidence: vec!["If A uses B, A depends on B".to_string()],
        });

        self.rules.push(InferenceRule {
            id: "rule_partof_implies_contains".to_string(),
            name: "PartOf Implies Contains".to_string(),
            premise: vec![RulePattern {
                entity_type: None,
                entity_name: None,
                relation: Some("PartOf".to_string()),
                target_type: None,
                target_name: None,
            }],
            conclusion: RulePattern {
                entity_type: None,
                entity_name: None,
                relation: Some("Contains".to_string()),
                target_type: None,
                target_name: None,
            },
            confidence: 0.75,
            evidence: vec!["Inverse of PartOf relation".to_string()],
        });

        self.rules.push(InferenceRule {
            id: "rule_creates_implies_uses".to_string(),
            name: "Creates Implies Uses".to_string(),
            premise: vec![RulePattern {
                entity_type: None,
                entity_name: None,
                relation: Some("Creates".to_string()),
                target_type: None,
                target_name: None,
            }],
            conclusion: RulePattern {
                entity_type: None,
                entity_name: None,
                relation: Some("Uses".to_string()),
                target_type: None,
                target_name: None,
            },
            confidence: 0.6,
            evidence: vec!["Creators typically use their tools".to_string()],
        });
    }

    pub fn add_rule(&mut self, rule: InferenceRule) {
        self.rules.push(rule);
    }

    pub fn infer(&self, knowledge_graph: &KnowledgeGraph, _query: &str) -> InferenceResult {
        let mut inferred_entities = Vec::new();
        let mut inferred_relations = Vec::new();
        let mut rules_applied = Vec::new();
        let mut reasoning_chain = Vec::new();
        let mut total_confidence = 0.0;
        let mut rule_count = 0;

        for rule in &self.rules {
            if self.rule_matches_premise(rule, knowledge_graph) {
                rules_applied.push(rule.name.clone());

                let conclusion = self.apply_rule_conclusion(rule, knowledge_graph);
                if let Some((entity, relation)) = conclusion {
                    inferred_entities.push(entity.clone());
                    inferred_relations.push(relation.clone());

                    reasoning_chain.push(ReasoningStep {
                        rule_id: rule.id.clone(),
                        premise_matches: vec!["matched".to_string()],
                        conclusion: format!("{:?} -> {:?}", relation.relation_type, entity.name),
                        confidence: rule.confidence,
                    });

                    total_confidence += rule.confidence;
                    rule_count += 1;
                }
            }
        }

        let confidence = if rule_count > 0 {
            total_confidence / f64::from(rule_count)
        } else {
            0.0
        };

        InferenceResult {
            inferred_entities,
            inferred_relations,
            confidence,
            rules_applied,
            reasoning_chain,
        }
    }

    fn rule_matches_premise(&self, rule: &InferenceRule, graph: &KnowledgeGraph) -> bool {
        for premise in &rule.premise {
            let matches = graph.relations.iter().any(|rel| {
                let type_match = premise.entity_type.as_ref().map_or(true, |et| {
                    graph
                        .entities
                        .get(&rel.from_entity)
                        .map(|e| format!("{:?}", e.entity_type).contains(et))
                        .unwrap_or(false)
                });

                let rel_match = premise
                    .relation
                    .as_ref()
                    .map_or(true, |r| rel.relation_type.as_str() == r);

                let target_match = premise.target_type.as_ref().map_or(true, |tt| {
                    graph
                        .entities
                        .get(&rel.to_entity)
                        .map(|e| format!("{:?}", e.entity_type).contains(tt))
                        .unwrap_or(false)
                });

                type_match && rel_match && target_match
            });

            if !matches {
                return false;
            }
        }
        true
    }

    fn apply_rule_conclusion(
        &self,
        rule: &InferenceRule,
        graph: &KnowledgeGraph,
    ) -> Option<(Entity, Relation)> {
        let from_entity = graph
            .relations
            .iter()
            .find(|r| {
                rule.premise.iter().any(|p| {
                    p.relation
                        .as_ref()
                        .map_or(false, |rel| r.relation_type.as_str() == rel)
                })
            })
            .map(|r| r.from_entity.clone())?;

        let to_entity = graph
            .relations
            .iter()
            .find(|r| {
                rule.premise.iter().any(|p| {
                    p.relation
                        .as_ref()
                        .map_or(false, |rel| r.relation_type.as_str() == rel)
                })
            })
            .map(|r| r.to_entity.clone())?;

        let relation_type = match rule.conclusion.relation.as_deref() {
            Some("RelatedTo") => RelationType::RelatedTo,
            Some("DependsOn") => RelationType::DependsOn,
            Some("Contains") => RelationType::Contains,
            Some("Uses") => RelationType::Uses,
            _ => RelationType::Custom("inferred".to_string()),
        };

        let inferred_relation = Relation {
            id: format!("inferred_{}", uuid::Uuid::new_v4()),
            from_entity: from_entity.clone(),
            to_entity: to_entity.clone(),
            relation_type: relation_type.clone(),
            weight: rule.confidence,
            attributes: HashMap::new(),
            evidence: rule.evidence.clone(),
            created_at: chrono::Utc::now(),
            confidence: rule.confidence,
        };

        let from_ent = graph.entities.get(&from_entity)?.clone();
        let _to_ent = graph.entities.get(&to_entity)?.clone();

        Some((from_ent, inferred_relation))
    }

    pub fn forward_chain(
        &self,
        knowledge_graph: &KnowledgeGraph,
        max_iterations: usize,
    ) -> Vec<InferenceResult> {
        let mut results = Vec::new();
        let mut current_graph = knowledge_graph.clone();

        for _ in 0..max_iterations {
            let result = self.infer(&current_graph, "");

            if result.inferred_entities.is_empty() {
                break;
            }

            for entity in &result.inferred_entities {
                current_graph
                    .entities
                    .insert(entity.id.clone(), entity.clone());
            }

            for relation in &result.inferred_relations {
                current_graph.relations.push(relation.clone());
            }

            results.push(result);
        }

        results
    }

    pub fn backward_chain(
        &self,
        knowledge_graph: &KnowledgeGraph,
        target: &EntityId,
    ) -> Vec<ReasoningStep> {
        let mut reasoning_chain = Vec::new();

        if let Some(entity) = knowledge_graph.entities.get(target) {
            for relation in &knowledge_graph.relations {
                if relation.to_entity == *target {
                    for rule in &self.rules {
                        if rule.conclusion.relation.as_deref()
                            == Some(relation.relation_type.as_str())
                        {
                            reasoning_chain.push(ReasoningStep {
                                rule_id: rule.id.clone(),
                                premise_matches: vec![format!(
                                    "{} -> {:?}",
                                    relation.from_entity.as_str(),
                                    relation.relation_type
                                )],
                                conclusion: format!(
                                    "{} is {:?}",
                                    entity.name, relation.relation_type
                                ),
                                confidence: rule.confidence * relation.confidence,
                            });
                        }
                    }
                }
            }
        }

        reasoning_chain
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}
