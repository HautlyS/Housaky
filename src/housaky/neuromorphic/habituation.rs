use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StimulusRecord {
    pub stimulus_id: String,
    pub stimulus_type: String,
    pub response_strength: f64,
    pub presentation_count: u64,
    pub last_presented: DateTime<Utc>,
    pub first_presented: DateTime<Utc>,
    pub habituated: bool,
    pub dishabituation_threshold: f64,
    pub recovery_rate: f64,
}

impl StimulusRecord {
    pub fn new(stimulus_id: &str, stimulus_type: &str) -> Self {
        let now = Utc::now();
        Self {
            stimulus_id: stimulus_id.to_string(),
            stimulus_type: stimulus_type.to_string(),
            response_strength: 1.0,
            presentation_count: 0,
            last_presented: now,
            first_presented: now,
            habituated: false,
            dishabituation_threshold: 0.2,
            recovery_rate: 0.05,
        }
    }

    pub fn present(&mut self, delta: f64) -> f64 {
        self.presentation_count += 1;
        self.last_presented = Utc::now();

        let decrement = delta * self.response_strength;
        self.response_strength = (self.response_strength - decrement).max(0.0);

        if self.response_strength < self.dishabituation_threshold {
            self.habituated = true;
            debug!(
                "Stimulus '{}' fully habituated after {} presentations",
                self.stimulus_id, self.presentation_count
            );
        }

        self.response_strength
    }

    pub fn recover(&mut self, elapsed_secs: f64) {
        if self.habituated {
            let recovery = self.recovery_rate * elapsed_secs;
            self.response_strength = (self.response_strength + recovery).min(1.0);
            if self.response_strength > self.dishabituation_threshold * 2.0 {
                self.habituated = false;
                debug!("Stimulus '{}' recovered from habituation", self.stimulus_id);
            }
        }
    }

    pub fn dishabituate(&mut self) {
        self.response_strength = 1.0;
        self.habituated = false;
        debug!("Stimulus '{}' dishabituated (novel/strong stimulus)", self.stimulus_id);
    }

    pub fn habituation_ratio(&self) -> f64 {
        1.0 - self.response_strength
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabituationConfig {
    pub base_decrement: f64,
    pub novelty_boost: f64,
    pub recovery_period_secs: f64,
    pub max_stimuli: usize,
    pub intensity_scaling: bool,
}

impl Default for HabituationConfig {
    fn default() -> Self {
        Self {
            base_decrement: 0.1,
            novelty_boost: 0.5,
            recovery_period_secs: 300.0,
            max_stimuli: 1000,
            intensity_scaling: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabituationStats {
    pub total_stimuli: usize,
    pub habituated_count: usize,
    pub avg_response_strength: f64,
    pub total_presentations: u64,
    pub most_habituated: Option<String>,
}

pub struct HabituationSystem {
    pub records: Arc<RwLock<HashMap<String, StimulusRecord>>>,
    pub config: HabituationConfig,
}

impl HabituationSystem {
    pub fn new(config: HabituationConfig) -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn process_stimulus(
        &self,
        stimulus_id: &str,
        stimulus_type: &str,
        intensity: f64,
    ) -> f64 {
        let mut records = self.records.write().await;

        if records.len() >= self.config.max_stimuli && !records.contains_key(stimulus_id) {
            let oldest = records
                .values()
                .min_by(|a, b| a.last_presented.cmp(&b.last_presented))
                .map(|r| r.stimulus_id.clone());
            if let Some(key) = oldest {
                records.remove(&key);
            }
        }

        let record = records
            .entry(stimulus_id.to_string())
            .or_insert_with(|| StimulusRecord::new(stimulus_id, stimulus_type));

        let elapsed = (Utc::now() - record.last_presented).num_seconds() as f64;
        if elapsed > 0.0 {
            record.recover(elapsed);
        }

        let decrement = if self.config.intensity_scaling {
            self.config.base_decrement * (1.0 / (1.0 + intensity))
        } else {
            self.config.base_decrement
        };

        record.present(decrement)
    }

    pub async fn dishabituate(&self, stimulus_id: &str) {
        let mut records = self.records.write().await;
        if let Some(r) = records.get_mut(stimulus_id) {
            r.dishabituate();
        }
    }

    pub async fn dishabituate_type(&self, stimulus_type: &str) {
        let mut records = self.records.write().await;
        for r in records.values_mut() {
            if r.stimulus_type == stimulus_type {
                r.dishabituate();
            }
        }
    }

    pub async fn is_habituated(&self, stimulus_id: &str) -> bool {
        self.records
            .read()
            .await
            .get(stimulus_id)
            .map(|r| r.habituated)
            .unwrap_or(false)
    }

    pub async fn response_strength(&self, stimulus_id: &str) -> f64 {
        self.records
            .read()
            .await
            .get(stimulus_id)
            .map(|r| r.response_strength)
            .unwrap_or(1.0)
    }

    pub async fn should_process(&self, stimulus_id: &str) -> bool {
        let strength = self.response_strength(stimulus_id).await;
        strength > 0.1
    }

    pub async fn apply_to_event_strength(&self, stimulus_id: &str, raw_strength: f64) -> f64 {
        let response = self.response_strength(stimulus_id).await;
        raw_strength * response
    }

    pub async fn run_recovery_cycle(&self) {
        let mut records = self.records.write().await;
        let now = Utc::now();
        for record in records.values_mut() {
            let elapsed = (now - record.last_presented).num_seconds() as f64;
            if elapsed > self.config.recovery_period_secs * 0.1 {
                record.recover(elapsed * 0.1);
            }
        }
    }

    pub async fn stats(&self) -> HabituationStats {
        let records = self.records.read().await;
        let total = records.len();
        let habituated_count = records.values().filter(|r| r.habituated).count();
        let avg_strength = if total > 0 {
            records.values().map(|r| r.response_strength).sum::<f64>() / total as f64
        } else {
            0.0
        };
        let total_presentations: u64 = records.values().map(|r| r.presentation_count).sum();
        let most_habituated = records
            .values()
            .max_by(|a, b| a.habituation_ratio().partial_cmp(&b.habituation_ratio())
                .unwrap_or(std::cmp::Ordering::Equal))
            .map(|r| r.stimulus_id.clone());

        HabituationStats {
            total_stimuli: total,
            habituated_count,
            avg_response_strength: avg_strength,
            total_presentations,
            most_habituated,
        }
    }
}

impl Default for HabituationSystem {
    fn default() -> Self {
        Self::new(HabituationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_habituation_decay() {
        let sys = HabituationSystem::default();
        for _ in 0..35 {
            sys.process_stimulus("tick", "noise", 0.5).await;
        }
        assert!(sys.is_habituated("tick").await);
    }

    #[tokio::test]
    async fn test_dishabituation() {
        let sys = HabituationSystem::default();
        for _ in 0..35 {
            sys.process_stimulus("s1", "noise", 0.5).await;
        }
        assert!(sys.is_habituated("s1").await);
        sys.dishabituate("s1").await;
        assert!(!sys.is_habituated("s1").await);
        assert_eq!(sys.response_strength("s1").await, 1.0);
    }

    #[tokio::test]
    async fn test_novel_stimulus_full_strength() {
        let sys = HabituationSystem::default();
        let strength = sys.response_strength("novel_stimulus").await;
        assert_eq!(strength, 1.0);
    }
}
