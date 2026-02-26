use super::event_bus::NeuromorphicEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorEvent {
    pub sensor_id: String,
    pub sensor_type: SensorType,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SensorEvent {
    pub fn new(sensor_id: &str, sensor_type: SensorType, value: f64, unit: &str) -> Self {
        Self {
            sensor_id: sensor_id.to_string(),
            sensor_type,
            value,
            unit: unit.to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SensorType {
    Temperature,
    Pressure,
    Motion,
    Light,
    Sound,
    Humidity,
    Voltage,
    Current,
    GPIO,
    Serial,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareAction {
    pub action_type: HardwareActionType,
    pub target_device: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub priority: u8,
}

impl HardwareAction {
    pub fn new(action_type: HardwareActionType, target_device: &str) -> Self {
        Self {
            action_type,
            target_device: target_device.to_string(),
            parameters: HashMap::new(),
            priority: 128,
        }
    }

    pub fn with_param(mut self, key: &str, value: serde_json::Value) -> Self {
        self.parameters.insert(key.to_string(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HardwareActionType {
    SetGPIO,
    WriteSerial,
    SetPWM,
    TriggerAlarm,
    ActivateRelay,
    SoftwareInterrupt,
    EmitEvent,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexCondition {
    pub condition_type: ConditionType,
    pub threshold: f64,
    pub hysteresis: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConditionType {
    GreaterThan,
    LessThan,
    AbsGreaterThan,
    RateOfChange,
    Always,
}

impl ReflexCondition {
    pub fn evaluate(&self, value: f64, prev_value: Option<f64>) -> bool {
        match self.condition_type {
            ConditionType::GreaterThan => value > self.threshold + self.hysteresis,
            ConditionType::LessThan => value < self.threshold - self.hysteresis,
            ConditionType::AbsGreaterThan => value.abs() > self.threshold + self.hysteresis,
            ConditionType::RateOfChange => {
                if let Some(prev) = prev_value {
                    (value - prev).abs() > self.threshold
                } else {
                    false
                }
            }
            ConditionType::Always => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexArc {
    pub id: String,
    pub name: String,
    pub trigger: SensorType,
    pub condition: ReflexCondition,
    pub response: HardwareAction,
    pub latency_budget_us: u64,
    pub bypass_reasoning: bool,
    pub enabled: bool,
    pub activation_count: u64,
    pub last_activated: Option<DateTime<Utc>>,
    pub cooldown_ms: u64,
}

impl ReflexArc {
    pub fn new(
        name: &str,
        trigger: SensorType,
        condition: ReflexCondition,
        response: HardwareAction,
        latency_budget_us: u64,
        bypass_reasoning: bool,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            trigger,
            condition,
            response,
            latency_budget_us,
            bypass_reasoning,
            enabled: true,
            activation_count: 0,
            last_activated: None,
            cooldown_ms: 10,
        }
    }

    pub fn temperature_safety(device: &str, threshold: f64) -> Self {
        Self::new(
            "temperature-safety",
            SensorType::Temperature,
            ReflexCondition {
                condition_type: ConditionType::GreaterThan,
                threshold,
                hysteresis: 1.0,
            },
            HardwareAction::new(HardwareActionType::TriggerAlarm, device)
                .with_param("reason", serde_json::json!("over-temperature")),
            500,
            true,
        )
    }

    pub fn gpio_interrupt(pin: &str, target: &str) -> Self {
        Self::new(
            &format!("gpio-reflex-{}", pin),
            SensorType::GPIO,
            ReflexCondition {
                condition_type: ConditionType::GreaterThan,
                threshold: 0.5,
                hysteresis: 0.0,
            },
            HardwareAction::new(HardwareActionType::SoftwareInterrupt, target)
                .with_param("pin", serde_json::json!(pin)),
            100,
            true,
        )
    }

    pub fn in_cooldown(&self) -> bool {
        if let Some(last) = self.last_activated {
            let elapsed_ms = (Utc::now() - last).num_milliseconds();
            elapsed_ms < self.cooldown_ms as i64
        } else {
            false
        }
    }

    pub fn activate(&mut self) -> Option<&HardwareAction> {
        if !self.enabled || self.in_cooldown() {
            return None;
        }
        self.activation_count += 1;
        self.last_activated = Some(Utc::now());
        Some(&self.response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexResult {
    pub reflex_id: String,
    pub reflex_name: String,
    pub action: HardwareAction,
    pub triggered_at: DateTime<Utc>,
    pub latency_us: u64,
    pub bypassed_reasoning: bool,
}

pub struct ReflexArcSystem {
    pub arcs: Arc<RwLock<Vec<ReflexArc>>>,
    pub sensor_history: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    pub execution_log: Arc<RwLock<Vec<ReflexResult>>>,
    pub max_history: usize,
}

impl ReflexArcSystem {
    pub fn new() -> Self {
        Self {
            arcs: Arc::new(RwLock::new(Vec::new())),
            sensor_history: Arc::new(RwLock::new(HashMap::new())),
            execution_log: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    pub async fn register_reflex(&self, arc: ReflexArc) {
        info!("ReflexArc: registered '{}' (bypass_reasoning={})", arc.name, arc.bypass_reasoning);
        self.arcs.write().await.push(arc);
    }

    pub async fn process_sensor_event(&self, event: &SensorEvent) -> Vec<ReflexResult> {
        let start = std::time::Instant::now();
        let mut results = Vec::new();

        {
            let mut history = self.sensor_history.write().await;
            let h = history.entry(event.sensor_id.clone()).or_default();
            h.push(event.value);
            if h.len() > self.max_history {
                h.remove(0);
            }
        }

        let prev_value = {
            let history = self.sensor_history.read().await;
            history
                .get(&event.sensor_id)
                .and_then(|h| if h.len() >= 2 { h.get(h.len() - 2).copied() } else { None })
        };

        let mut arcs = self.arcs.write().await;
        for arc in arcs.iter_mut() {
            if !arc.enabled || arc.trigger != event.sensor_type {
                continue;
            }
            if arc.condition.evaluate(event.value, prev_value) {
                let arc_name = arc.name.clone();
                let arc_id = arc.id.clone();
                let latency_budget = arc.latency_budget_us;
                let bypass = arc.bypass_reasoning;
                if let Some(action) = arc.activate() {
                    let latency_us = start.elapsed().as_micros() as u64;
                    debug!(
                        "ReflexArc '{}' triggered: latency={}µs budget={}µs",
                        arc_name, latency_us, latency_budget
                    );
                    if latency_us > latency_budget {
                        tracing::warn!(
                            "ReflexArc '{}' exceeded latency budget: {}µs > {}µs",
                            arc_name, latency_us, latency_budget
                        );
                    }
                    results.push(ReflexResult {
                        reflex_id: arc_id,
                        reflex_name: arc_name,
                        action: action.clone(),
                        triggered_at: Utc::now(),
                        latency_us,
                        bypassed_reasoning: bypass,
                    });
                }
            }
        }
        drop(arcs);

        if !results.is_empty() {
            let mut log = self.execution_log.write().await;
            log.extend(results.clone());
            if log.len() > 10_000 {
                let drain_count = log.len() - 10_000;
                log.drain(0..drain_count);
            }
        }

        results
    }

    pub async fn process_neuromorphic_event(
        &self,
        event: &NeuromorphicEvent,
    ) -> Vec<ReflexResult> {
        let sensor_type = match event.event_type.as_str() {
            "temperature" => SensorType::Temperature,
            "gpio" | "gpio_interrupt" => SensorType::GPIO,
            "motion" => SensorType::Motion,
            "pressure" => SensorType::Pressure,
            "sound" | "audio" => SensorType::Sound,
            "serial" => SensorType::Serial,
            _ => SensorType::Custom(event.event_type.clone()),
        };

        let value = event
            .payload
            .get("value")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let sensor_event = SensorEvent::new(&event.source, sensor_type, value, "");
        self.process_sensor_event(&sensor_event).await
    }

    pub async fn to_neuromorphic_events(&self, results: &[ReflexResult]) -> Vec<NeuromorphicEvent> {
        results
            .iter()
            .map(|r| {
                NeuromorphicEvent::interrupt(
                    &format!("reflex:{}", r.reflex_name),
                    "reflex_arc_system",
                    serde_json::json!({
                        "action": r.action.action_type,
                        "device": r.action.target_device,
                        "latency_us": r.latency_us,
                        "bypassed_reasoning": r.bypassed_reasoning,
                    }),
                )
            })
            .collect()
    }

    pub async fn enable_reflex(&self, name: &str) {
        let mut arcs = self.arcs.write().await;
        if let Some(arc) = arcs.iter_mut().find(|a| a.name == name) {
            arc.enabled = true;
        }
    }

    pub async fn disable_reflex(&self, name: &str) {
        let mut arcs = self.arcs.write().await;
        if let Some(arc) = arcs.iter_mut().find(|a| a.name == name) {
            arc.enabled = false;
        }
    }

    pub async fn total_activations(&self) -> u64 {
        self.arcs.read().await.iter().map(|a| a.activation_count).sum()
    }
}

impl Default for ReflexArcSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_temperature_reflex() {
        let system = ReflexArcSystem::new();
        system.register_reflex(ReflexArc::temperature_safety("cooling_fan", 80.0)).await;

        let event = SensorEvent::new("temp-1", SensorType::Temperature, 85.0, "celsius");
        let results = system.process_sensor_event(&event).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action.action_type, HardwareActionType::TriggerAlarm);
    }

    #[tokio::test]
    async fn test_no_trigger_below_threshold() {
        let system = ReflexArcSystem::new();
        system.register_reflex(ReflexArc::temperature_safety("fan", 80.0)).await;

        let event = SensorEvent::new("temp-1", SensorType::Temperature, 70.0, "celsius");
        let results = system.process_sensor_event(&event).await;
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_cooldown_prevents_rapid_firing() {
        let system = ReflexArcSystem::new();
        let mut arc = ReflexArc::temperature_safety("fan", 50.0);
        arc.cooldown_ms = 10_000;
        system.register_reflex(arc).await;

        let event = SensorEvent::new("t", SensorType::Temperature, 90.0, "C");
        let r1 = system.process_sensor_event(&event).await;
        let r2 = system.process_sensor_event(&event).await;
        assert_eq!(r1.len(), 1);
        assert_eq!(r2.len(), 0);
    }
}
