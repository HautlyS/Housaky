//! Data Flow Graph — Module interconnection graph with typed edges.
//!
//! Represents the full data-flow topology of a cognitive architecture as a
//! directed graph. Provides cycle detection, reachability analysis, critical-path
//! computation, and topological ordering for evaluation scheduling.

use crate::housaky::architecture_search::module_genome::{ModuleConnection, ModuleSpec};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

// ── Graph Node ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub module_id: String,
    pub module_name: String,
    pub outgoing: Vec<String>, // connection IDs
    pub incoming: Vec<String>, // connection IDs
    pub depth: usize,          // topological depth (0 = root)
    pub is_critical: bool,     // on the critical path
}

// ── Data Flow Graph ───────────────────────────────────────────────────────────

/// Directed graph representing data flow between cognitive modules.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataFlowGraph {
    pub nodes: HashMap<String, GraphNode>,       // module_id → node
    pub connections: HashMap<String, ModuleConnection>, // conn_id → connection
}

impl DataFlowGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build graph from a list of modules and connections.
    pub fn build(modules: &[ModuleSpec], connections: &[ModuleConnection]) -> Self {
        let mut g = Self::new();

        for m in modules {
            if m.enabled {
                g.nodes.insert(
                    m.id.clone(),
                    GraphNode {
                        module_id: m.id.clone(),
                        module_name: m.name.clone(),
                        outgoing: Vec::new(),
                        incoming: Vec::new(),
                        depth: 0,
                        is_critical: false,
                    },
                );
            }
        }

        for c in connections {
            if g.nodes.contains_key(&c.from) && g.nodes.contains_key(&c.to) {
                g.connections.insert(c.id.clone(), c.clone());
                if let Some(n) = g.nodes.get_mut(&c.from) {
                    n.outgoing.push(c.id.clone());
                }
                if let Some(n) = g.nodes.get_mut(&c.to) {
                    n.incoming.push(c.id.clone());
                }
                if c.bidirectional {
                    if let Some(n) = g.nodes.get_mut(&c.to) {
                        n.outgoing.push(c.id.clone());
                    }
                    if let Some(n) = g.nodes.get_mut(&c.from) {
                        n.incoming.push(c.id.clone());
                    }
                }
            }
        }

        g.compute_depths();
        g.mark_critical_path();
        g
    }

    /// Kahn's algorithm for topological ordering and depth assignment.
    fn compute_depths(&mut self) {
        let mut in_degree: HashMap<String, usize> = self
            .nodes
            .keys()
            .map(|id| (id.clone(), 0))
            .collect();

        for conn in self.connections.values() {
            *in_degree.entry(conn.to.clone()).or_insert(0) += 1;
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut depths: HashMap<String, usize> = HashMap::new();
        while let Some(id) = queue.pop_front() {
            let depth = depths.get(&id).cloned().unwrap_or(0);
            if let Some(node) = self.nodes.get_mut(&id) {
                node.depth = depth;
            }
            let outgoing: Vec<String> = if let Some(node) = self.nodes.get(&id) {
                node.outgoing
                    .iter()
                    .filter_map(|cid| self.connections.get(cid).map(|c| c.to.clone()))
                    .collect()
            } else {
                vec![]
            };
            for next_id in outgoing {
                depths.entry(next_id.clone()).and_modify(|d| {
                    if depth + 1 > *d {
                        *d = depth + 1;
                    }
                }).or_insert(depth + 1);
                let deg = in_degree.entry(next_id.clone()).or_insert(1);
                *deg = deg.saturating_sub(1);
                if *deg == 0 {
                    queue.push_back(next_id);
                }
            }
        }

        for (id, depth) in &depths {
            if let Some(node) = self.nodes.get_mut(id) {
                node.depth = *depth;
            }
        }
    }

    /// Mark nodes on the longest (critical) path.
    fn mark_critical_path(&mut self) {
        let max_depth = self.nodes.values().map(|n| n.depth).max().unwrap_or(0);
        for node in self.nodes.values_mut() {
            node.is_critical = node.depth == max_depth || node.depth == 0;
        }
    }

    /// Check whether the graph contains any cycles.
    pub fn has_cycles(&self) -> bool {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut rec_stack: HashSet<&str> = HashSet::new();

        for id in self.nodes.keys() {
            if self.dfs_cycle(id.as_str(), &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle<'a>(
        &'a self,
        id: &'a str,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
    ) -> bool {
        if rec_stack.contains(id) {
            return true;
        }
        if visited.contains(id) {
            return false;
        }
        visited.insert(id);
        rec_stack.insert(id);

        if let Some(node) = self.nodes.get(id) {
            for cid in &node.outgoing {
                if let Some(conn) = self.connections.get(cid) {
                    if self.dfs_cycle(conn.to.as_str(), visited, rec_stack) {
                        return true;
                    }
                }
            }
        }
        rec_stack.remove(id);
        false
    }

    /// BFS reachability: which module IDs are reachable from `start_id`?
    pub fn reachable_from(&self, start_id: &str) -> HashSet<String> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(start_id.to_string());

        while let Some(id) = queue.pop_front() {
            if visited.contains(&id) {
                continue;
            }
            visited.insert(id.clone());
            if let Some(node) = self.nodes.get(&id) {
                for cid in &node.outgoing {
                    if let Some(conn) = self.connections.get(cid) {
                        if !visited.contains(&conn.to) {
                            queue.push_back(conn.to.clone());
                        }
                    }
                }
            }
        }
        visited
    }

    /// Topological order of module IDs (roots first).
    pub fn topological_order(&self) -> Vec<String> {
        let mut ordered: Vec<(&str, usize)> = self
            .nodes
            .iter()
            .map(|(id, node)| (id.as_str(), node.depth))
            .collect();
        ordered.sort_by_key(|&(_, d)| d);
        ordered.into_iter().map(|(id, _)| id.to_string()).collect()
    }

    /// Connectivity metrics for fitness evaluation.
    pub fn connectivity_score(&self) -> f64 {
        let n = self.nodes.len();
        if n <= 1 {
            return 1.0;
        }
        let e = self.connections.len() as f64;
        let max_edges = (n * (n - 1)) as f64;
        (e / max_edges).min(1.0)
    }

    /// Count isolated modules (no connections at all).
    pub fn isolated_module_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|n| n.incoming.is_empty() && n.outgoing.is_empty())
            .count()
    }

    /// Summary for display/debug.
    pub fn summary(&self) -> DataFlowSummary {
        DataFlowSummary {
            node_count: self.nodes.len(),
            connection_count: self.connections.len(),
            max_depth: self.nodes.values().map(|n| n.depth).max().unwrap_or(0),
            has_cycles: self.has_cycles(),
            connectivity_score: self.connectivity_score(),
            isolated_modules: self.isolated_module_count(),
            critical_path_length: self
                .nodes
                .values()
                .filter(|n| n.is_critical)
                .count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowSummary {
    pub node_count: usize,
    pub connection_count: usize,
    pub max_depth: usize,
    pub has_cycles: bool,
    pub connectivity_score: f64,
    pub isolated_modules: usize,
    pub critical_path_length: usize,
}
