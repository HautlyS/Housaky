use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub mutation_op: String,
    pub target_file: String,
    pub target_function: String,
    pub rationale: String,
    pub fitness_before: f64,
    pub fitness_after: f64,
    pub applied: bool,
    pub rolled_back: bool,
    pub timestamp: DateTime<Utc>,
    pub rollback_patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationLineage {
    pub nodes: HashMap<String, MutationNode>,
    pub root_ids: Vec<String>,
    pub current_head: Option<String>,
    pub total_applied: u64,
    pub total_rolled_back: u64,
    pub best_node_id: Option<String>,
    pub best_fitness: f64,
}

impl MutationLineage {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_ids: Vec::new(),
            current_head: None,
            total_applied: 0,
            total_rolled_back: 0,
            best_node_id: None,
            best_fitness: 0.0,
        }
    }

    pub fn add_node(&mut self, node: MutationNode) {
        let id = node.id.clone();
        if node.parent_id.is_none() {
            self.root_ids.push(id.clone());
        }
        if node.applied {
            self.total_applied += 1;
            if node.fitness_after > self.best_fitness {
                self.best_fitness = node.fitness_after;
                self.best_node_id = Some(id.clone());
            }
            self.current_head = Some(id.clone());
        }
        self.nodes.insert(id, node);
        info!(
            "Lineage: recorded mutation node {} (applied={})",
            &self.nodes.keys().last().unwrap_or(&String::new()),
            self.total_applied
        );
    }

    pub fn mark_rolled_back(&mut self, node_id: &str) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.rolled_back = true;
            self.total_rolled_back += 1;
            // Reset head to parent
            self.current_head = node.parent_id.clone();
        }
    }

    pub fn ancestors_of(&self, node_id: &str) -> Vec<&MutationNode> {
        let mut ancestors = Vec::new();
        let mut current = self.nodes.get(node_id);
        while let Some(node) = current {
            if let Some(ref parent_id) = node.parent_id {
                current = self.nodes.get(parent_id.as_str());
                if let Some(parent) = current {
                    ancestors.push(parent);
                }
            } else {
                break;
            }
        }
        ancestors
    }

    pub fn rollback_chain(&self, node_id: &str) -> Vec<String> {
        let mut patches = Vec::new();
        let ancestors = self.ancestors_of(node_id);
        for ancestor in ancestors {
            if !ancestor.rollback_patch.is_empty() {
                patches.push(ancestor.rollback_patch.clone());
            }
        }
        patches
    }

    pub fn applied_nodes(&self) -> Vec<&MutationNode> {
        let mut nodes: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.applied && !n.rolled_back)
            .collect();
        nodes.sort_by_key(|n| &n.timestamp);
        nodes
    }

    pub fn persist(&self, workspace_dir: &PathBuf) -> anyhow::Result<()> {
        let path = workspace_dir.join(".housaky").join("mutation_lineage.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn load(workspace_dir: &PathBuf) -> anyhow::Result<Self> {
        let path = workspace_dir.join(".housaky").join("mutation_lineage.json");
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&json)?)
    }
}

impl Default for MutationLineage {
    fn default() -> Self {
        Self::new()
    }
}
