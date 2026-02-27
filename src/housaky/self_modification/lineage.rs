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
    /// All valid (applied, not rolled-back) branch heads — enables DGM-style
    /// population-based selection rather than single-chain progression.
    pub archive_heads: Vec<String>,
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
            archive_heads: Vec::new(),
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
            // Add to branching archive — keep all valid branch heads.
            // Remove parent from archive heads since it now has a child.
            if let Some(ref parent_id) = node.parent_id {
                self.archive_heads.retain(|h| h != parent_id);
            }
            if !self.archive_heads.contains(&id) {
                self.archive_heads.push(id.clone());
            }
        }
        self.nodes.insert(id, node);
        info!(
            "Lineage: recorded mutation node {} (applied={}, archive_heads={})",
            &self.nodes.keys().last().unwrap_or(&String::new()),
            self.total_applied,
            self.archive_heads.len(),
        );
    }

    pub fn mark_rolled_back(&mut self, node_id: &str) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.rolled_back = true;
            self.total_rolled_back += 1;
            // Reset head to parent
            self.current_head = node.parent_id.clone();
        }
        // Remove from archive — restore parent if it was the previous head
        self.archive_heads.retain(|h| h != node_id);
        if let Some(node) = self.nodes.get(node_id) {
            if let Some(ref parent_id) = node.parent_id {
                let parent_still_valid = self
                    .nodes
                    .get(parent_id.as_str())
                    .map(|p| p.applied && !p.rolled_back)
                    .unwrap_or(false);
                if parent_still_valid && !self.archive_heads.contains(parent_id) {
                    self.archive_heads.push(parent_id.clone());
                }
            }
        }
    }

    /// DGM-style parent selection: choose a mutation node from the archive
    /// weighted by `sigmoid_performance × novelty_bonus`.
    ///
    /// - `sigmoid_performance`: `1 / (1 + exp(-λ(fitness - fitness_mean)))`
    /// - `novelty_bonus`: `1 / (1 + children_count)` — nodes with fewer
    ///   descendants get a boost to keep the population diverse.
    /// - `λ = 3.0` (controls steepness of the sigmoid)
    ///
    /// Returns `None` if the archive is empty.
    pub fn select_parent_dgm(&self) -> Option<&MutationNode> {
        let archive: Vec<&MutationNode> = self
            .archive_heads
            .iter()
            .filter_map(|id| self.nodes.get(id.as_str()))
            .filter(|n| n.applied && !n.rolled_back)
            .collect();

        if archive.is_empty() {
            return None;
        }

        // Compute fitness mean over the archive.
        let fitness_mean = archive.iter().map(|n| n.fitness_after).sum::<f64>()
            / archive.len() as f64;
        let lambda = 3.0_f64;

        // Count children for each archive member.
        let children_count: HashMap<&str, usize> = {
            let mut map: HashMap<&str, usize> = archive
                .iter()
                .map(|n| (n.id.as_str(), 0usize))
                .collect();
            for node in self.nodes.values() {
                if let Some(ref pid) = node.parent_id {
                    if let Some(cnt) = map.get_mut(pid.as_str()) {
                        *cnt += 1;
                    }
                }
            }
            map
        };

        // Compute unnormalised weights.
        let weights: Vec<f64> = archive
            .iter()
            .map(|n| {
                let s = 1.0 / (1.0 + (-lambda * (n.fitness_after - fitness_mean)).exp());
                let children = children_count.get(n.id.as_str()).copied().unwrap_or(0);
                let h = 1.0 / (1.0 + children as f64);
                s * h
            })
            .collect();

        let total_weight: f64 = weights.iter().sum();
        if total_weight <= 0.0 {
            // Fallback: return best fitness node.
            return archive
                .iter()
                .max_by(|a, b| a.fitness_after.partial_cmp(&b.fitness_after).unwrap())
                .copied();
        }

        // Weighted random selection using a deterministic scan based on node IDs
        // (we avoid bringing in `rand` — use a simple hash-based pseudo-random).
        let selector = {
            // Cheap hash: XOR fold the bytes of all archive IDs.
            let mut h: u64 = 0x517c_c1b7_2722_0a95;
            for n in &archive {
                for b in n.id.bytes() {
                    h = h.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(b as u64);
                }
            }
            // Map to [0, 1)
            (h as f64) / (u64::MAX as f64)
        };

        let mut cumulative = 0.0;
        let target = selector * total_weight;
        for (i, w) in weights.iter().enumerate() {
            cumulative += w;
            if cumulative >= target {
                return Some(archive[i]);
            }
        }

        Some(archive[archive.len() - 1])
    }

    /// Return all valid (applied, not rolled-back) archive nodes sorted by
    /// descending fitness — useful for dashboards and debugging.
    pub fn archive_sorted_by_fitness(&self) -> Vec<&MutationNode> {
        let mut nodes: Vec<&MutationNode> = self
            .archive_heads
            .iter()
            .filter_map(|id| self.nodes.get(id.as_str()))
            .filter(|n| n.applied && !n.rolled_back)
            .collect();
        nodes.sort_by(|a, b| b.fitness_after.partial_cmp(&a.fitness_after).unwrap());
        nodes
    }

    /// Count how many children a given node has in the full lineage.
    pub fn children_count(&self, node_id: &str) -> usize {
        self.nodes
            .values()
            .filter(|n| n.parent_id.as_deref() == Some(node_id))
            .count()
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

/// Summary statistics for the mutation lineage archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageStats {
    pub total_nodes: usize,
    pub total_applied: u64,
    pub total_rolled_back: u64,
    pub archive_heads: usize,
    pub best_fitness: f64,
    pub avg_fitness_applied: f64,
    pub avg_fitness_delta: f64,
    pub most_mutated_file: Option<String>,
}

impl MutationLineage {
    /// Compute summary statistics for the entire lineage archive.
    pub fn statistics(&self) -> LineageStats {
        let applied: Vec<&MutationNode> = self.nodes.values().filter(|n| n.applied && !n.rolled_back).collect();
        let avg_fitness_applied = if applied.is_empty() {
            0.0
        } else {
            applied.iter().map(|n| n.fitness_after).sum::<f64>() / applied.len() as f64
        };
        let avg_fitness_delta = if applied.is_empty() {
            0.0
        } else {
            applied.iter().map(|n| n.fitness_after - n.fitness_before).sum::<f64>() / applied.len() as f64
        };

        // Find most-targeted file.
        let mut file_counts: HashMap<&str, usize> = HashMap::new();
        for node in self.nodes.values() {
            *file_counts.entry(node.target_file.as_str()).or_default() += 1;
        }
        let most_mutated_file = file_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(file, _)| file.to_string());

        LineageStats {
            total_nodes: self.nodes.len(),
            total_applied: self.total_applied,
            total_rolled_back: self.total_rolled_back,
            archive_heads: self.archive_heads.len(),
            best_fitness: self.best_fitness,
            avg_fitness_applied,
            avg_fitness_delta,
            most_mutated_file,
        }
    }
}

impl Default for MutationLineage {
    fn default() -> Self {
        Self::new()
    }
}
