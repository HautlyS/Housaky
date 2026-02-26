use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PheromoneTrail {
    pub path: Vec<String>,
    pub strength: f64,
    pub deposited_by: String,
    pub task_type: String,
    pub success_rate: f64,
    pub deposited_at: DateTime<Utc>,
    pub last_reinforced: DateTime<Utc>,
}

impl PheromoneTrail {
    pub fn new(path: Vec<String>, deposited_by: String, task_type: String) -> Self {
        let now = Utc::now();
        Self {
            path,
            strength: 1.0,
            deposited_by,
            task_type,
            success_rate: 0.0,
            deposited_at: now,
            last_reinforced: now,
        }
    }

    pub fn reinforce(&mut self, delta: f64) {
        self.strength = (self.strength + delta).min(10.0);
        self.last_reinforced = Utc::now();
    }

    pub fn evaporate(&mut self, rate: f64) {
        self.strength = (self.strength * (1.0 - rate)).max(0.0);
    }

    pub fn is_expired(&self) -> bool {
        self.strength < 0.01
    }

    pub fn path_key(&self) -> String {
        self.path.join("->")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PheromoneMap {
    pub trails: HashMap<String, PheromoneTrail>,
    pub evaporation_rate: f64,
    pub reinforcement_factor: f64,
    pub total_deposits: u64,
    pub total_evaporations: u64,
}

impl PheromoneMap {
    pub fn new(evaporation_rate: f64, reinforcement_factor: f64) -> Self {
        Self {
            trails: HashMap::new(),
            evaporation_rate,
            reinforcement_factor,
            total_deposits: 0,
            total_evaporations: 0,
        }
    }

    pub fn deposit(&mut self, trail: PheromoneTrail) {
        let key = trail.path_key();
        if let Some(existing) = self.trails.get_mut(&key) {
            existing.reinforce(self.reinforcement_factor * trail.success_rate);
        } else {
            self.trails.insert(key, trail);
        }
        self.total_deposits += 1;
    }

    pub fn evaporate_all(&mut self) {
        let rate = self.evaporation_rate;
        self.trails.values_mut().for_each(|t| t.evaporate(rate));
        self.trails.retain(|_, t| !t.is_expired());
        self.total_evaporations += 1;
    }

    pub fn get_best_path(&self, task_type: &str) -> Option<&PheromoneTrail> {
        self.trails
            .values()
            .filter(|t| t.task_type == task_type)
            .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn top_paths(&self, task_type: &str, n: usize) -> Vec<&PheromoneTrail> {
        let mut matching: Vec<&PheromoneTrail> = self
            .trails
            .values()
            .filter(|t| t.task_type == task_type)
            .collect();
        matching.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
        matching.into_iter().take(n).collect()
    }

    pub fn probability_distribution(&self, task_type: &str) -> Vec<(String, f64)> {
        let trails: Vec<&PheromoneTrail> = self
            .trails
            .values()
            .filter(|t| t.task_type == task_type)
            .collect();

        let total: f64 = trails.iter().map(|t| t.strength).sum();
        if total < 1e-9 {
            return vec![];
        }

        trails
            .iter()
            .map(|t| (t.path_key(), t.strength / total))
            .collect()
    }

    pub fn stats(&self) -> PheromoneStats {
        let active = self.trails.len();
        let avg_strength = if active > 0 {
            self.trails.values().map(|t| t.strength).sum::<f64>() / active as f64
        } else {
            0.0
        };
        PheromoneStats {
            active_trails: active,
            total_deposits: self.total_deposits,
            total_evaporations: self.total_evaporations,
            avg_strength,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PheromoneStats {
    pub active_trails: usize,
    pub total_deposits: u64,
    pub total_evaporations: u64,
    pub avg_strength: f64,
}

pub struct PheromoneService {
    pub map: Arc<RwLock<PheromoneMap>>,
}

impl PheromoneService {
    pub fn new(evaporation_rate: f64, reinforcement_factor: f64) -> Self {
        Self {
            map: Arc::new(RwLock::new(PheromoneMap::new(evaporation_rate, reinforcement_factor))),
        }
    }

    pub async fn deposit(&self, path: Vec<String>, agent_id: &str, task_type: &str, success_rate: f64) {
        let mut trail = PheromoneTrail::new(path, agent_id.to_string(), task_type.to_string());
        trail.success_rate = success_rate;
        self.map.write().await.deposit(trail);
    }

    pub async fn evaporate(&self) {
        self.map.write().await.evaporate_all();
    }

    pub async fn best_path(&self, task_type: &str) -> Option<Vec<String>> {
        self.map.read().await.get_best_path(task_type).map(|t| t.path.clone())
    }

    pub async fn top_paths(&self, task_type: &str, n: usize) -> Vec<Vec<String>> {
        self.map.read().await.top_paths(task_type, n).iter().map(|t| t.path.clone()).collect()
    }

    pub async fn stats(&self) -> PheromoneStats {
        self.map.read().await.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_and_evaporate() {
        let mut map = PheromoneMap::new(0.1, 1.0);
        let trail = PheromoneTrail::new(
            vec!["step1".into(), "step2".into()],
            "agent-1".into(),
            "coding".into(),
        );
        map.deposit(trail);
        assert_eq!(map.trails.len(), 1);
        map.evaporate_all();
        assert!((map.trails.values().next().unwrap().strength - 0.9).abs() < 1e-9);
    }

    #[test]
    fn test_reinforcement() {
        let mut map = PheromoneMap::new(0.1, 2.0);
        let mut trail = PheromoneTrail::new(vec!["a".into()], "agent".into(), "task".into());
        trail.success_rate = 1.0;
        map.deposit(trail.clone());
        map.deposit(trail);
        let t = map.trails.values().next().unwrap();
        assert!(t.strength > 1.0);
    }

    #[tokio::test]
    async fn test_pheromone_service() {
        let svc = PheromoneService::new(0.05, 1.0);
        svc.deposit(vec!["tool_a".into(), "tool_b".into()], "agent-1", "analysis", 0.95).await;
        let best = svc.best_path("analysis").await;
        assert!(best.is_some());
    }
}
