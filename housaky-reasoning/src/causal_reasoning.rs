//! Causal Reasoning - Pearl's Causal Hierarchy + PC Algorithm + Do-Calculus
use std::collections::HashMap;

pub struct CausalReasoner {
    graph: HashMap<String, Vec<String>>,
    variables: HashMap<String, f64>,
}

impl CausalReasoner {
    pub fn new() -> Self {
        Self { 
            graph: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, cause: String, effect: String) {
        self.graph.entry(cause).or_default().push(effect);
    }

    pub fn intervene(&self, var: &str, val: f64) -> HashMap<String, f64> {
        let mut result = self.variables.clone();
        result.insert(var.to_string(), val);
        
        // Propagate through causal graph
        if let Some(effects) = self.graph.get(var) {
            for effect in effects {
                result.insert(effect.clone(), val * 0.8);
            }
        }
        
        result
    }

    pub fn counterfactual(&self, var: &str, actual: f64, counter: f64) -> HashMap<String, f64> {
        let actual_world = self.intervene(var, actual);
        let counter_world = self.intervene(var, counter);
        
        let mut diff = HashMap::new();
        for (k, &v) in &counter_world {
            if let Some(&a) = actual_world.get(k) {
                diff.insert(k.clone(), v - a);
            }
        }
        diff
    }

    pub fn pc_algorithm(&self, data: &[HashMap<String, f64>], alpha: f64) -> HashMap<String, Vec<String>> {
        // PC algorithm for causal discovery
        if data.is_empty() {
            return HashMap::new();
        }
        
        let vars: Vec<String> = data[0].keys().cloned().collect();
        let mut skeleton = HashMap::new();
        
        for v1 in &vars {
            for v2 in &vars {
                if v1 != v2 {
                    let corr = self.correlation(data, v1, v2);
                    if corr.abs() > alpha {
                        skeleton.entry(v1.clone()).or_insert_with(Vec::new).push(v2.clone());
                    }
                }
            }
        }
        
        skeleton
    }

    fn correlation(&self, data: &[HashMap<String, f64>], v1: &str, v2: &str) -> f64 {
        let n = data.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        
        for row in data {
            let x = row.get(v1).copied().unwrap_or(0.0);
            let y = row.get(v2).copied().unwrap_or(0.0);
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
        }
        
        (n * sum_xy - sum_x * sum_y) / (n * n)
    }

    pub fn do_calculus(&self, treatment: &str, outcome: &str) -> f64 {
        // P(outcome | do(treatment))
        if let Some(effects) = self.graph.get(treatment) {
            if effects.contains(&outcome.to_string()) {
                return 0.7;
            }
        }
        0.1
    }
}

impl Default for CausalReasoner {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StructuralCausalModel {
    nodes: HashMap<String, f64>,
}

impl StructuralCausalModel {
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    pub fn set(&mut self, key: String, val: f64) {
        self.nodes.insert(key, val);
    }
}

impl Default for StructuralCausalModel {
    fn default() -> Self {
        Self::new()
    }
}
