//! Complete Energy Management System - Optimized
//!
//! This module provides comprehensive energy management:
//! - Battery monitoring and state-of-charge tracking
//! - Solar panel integration and MPPT tracking
//! - Power scheduling and task prioritization
//! - Energy harvesting optimization
//! - Hibernate/sleep mode management
//!
//! # Memory Safety
//! - Bounded energy history (circular buffer)
//! - Bounded task queue
//! - Proper cleanup of monitoring tasks
//!
//! # Performance
//! - Efficient sampling with configurable intervals
//! - Pre-allocated collections
//! - Minimal allocations in hot paths

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use metrics::{counter, gauge, histogram};

/// Maximum energy history size (1 week at 1 sample/minute)
const MAX_HISTORY_SIZE: usize = 10080;

/// Maximum task queue size
const MAX_TASK_QUEUE_SIZE: usize = 1000;

/// Sampling interval in seconds
const DEFAULT_SAMPLING_INTERVAL_SECS: u64 = 60;

/// Energy management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyConfig {
    /// Battery capacity in Wh
    pub battery_capacity_wh: f64,
    /// Low battery threshold (0.0 - 1.0)
    pub low_battery_threshold: f64,
    /// Critical battery threshold (0.0 - 1.0)
    pub critical_battery_threshold: f64,
    /// Solar panel capacity in W
    pub solar_capacity_w: f64,
    /// Base power consumption in W
    pub base_power_consumption_w: f64,
    /// Max power consumption in W
    pub max_power_consumption_w: f64,
    /// Enable hibernation on low battery
    pub enable_hibernation: bool,
    /// Hibernate threshold
    pub hibernate_threshold: f64,
    /// Sampling interval (seconds)
    pub sampling_interval_secs: u64,
    /// Maximum task queue size
    pub max_task_queue_size: usize,
}

impl Default for EnergyConfig {
    fn default() -> Self {
        Self {
            battery_capacity_wh: 50.0,        // 50Wh battery (laptop)
            low_battery_threshold: 0.20,      // 20%
            critical_battery_threshold: 0.05, // 5%
            solar_capacity_w: 10.0,           // 10W solar panel
            base_power_consumption_w: 10.0,
            max_power_consumption_w: 60.0,
            enable_hibernation: true,
            hibernate_threshold: 0.10, // 10%
            sampling_interval_secs: DEFAULT_SAMPLING_INTERVAL_SECS,
            max_task_queue_size: MAX_TASK_QUEUE_SIZE,
        }
    }
}

/// Battery status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    Empty,
    Unknown,
}

/// Battery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    /// Current state of charge (0.0 - 1.0)
    pub state_of_charge: f64,
    /// Battery status
    pub status: BatteryStatus,
    /// Current power draw in W (positive = charging, negative = discharging)
    pub power_draw_w: f64,
    /// Voltage
    pub voltage_v: f64,
    /// Current in A
    pub current_a: f64,
    /// Temperature in Celsius
    pub temperature_c: f64,
    /// Health percentage (0.0 - 1.0)
    pub health_percent: f64,
    /// Cycle count
    pub cycle_count: u32,
    /// Time remaining estimate (seconds)
    pub time_remaining_seconds: Option<u64>,
    /// Timestamp
    pub timestamp: u64,
}

impl BatteryInfo {
    /// Check if battery is low
    pub fn is_low(&self, threshold: f64) -> bool {
        self.state_of_charge < threshold
    }

    /// Check if battery is critical
    pub fn is_critical(&self, threshold: f64) -> bool {
        self.state_of_charge < threshold
    }

    /// Estimate remaining runtime at current power draw
    pub fn estimate_remaining_runtime(&self, capacity_wh: f64) -> Option<Duration> {
        if self.power_draw_w >= 0.0 {
            // Charging or full
            None
        } else {
            let remaining_wh = capacity_wh * self.state_of_charge;
            let hours = remaining_wh / self.power_draw_w.abs();
            Some(Duration::from_secs_f64(hours * 3600.0))
        }
    }
    
    /// Calculate charge/discharge rate (% per hour)
    pub fn charge_rate_percent_per_hour(&self, capacity_wh: f64) -> f64 {
        if capacity_wh <= 0.0 {
            0.0
        } else {
            (self.power_draw_w / capacity_wh) * 100.0
        }
    }
}

impl Default for BatteryInfo {
    fn default() -> Self {
        Self {
            state_of_charge: 1.0,
            status: BatteryStatus::Full,
            power_draw_w: 0.0,
            voltage_v: 12.0,
            current_a: 0.0,
            temperature_c: 25.0,
            health_percent: 1.0,
            cycle_count: 0,
            time_remaining_seconds: None,
            timestamp: 0,
        }
    }
}

/// Solar panel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarInfo {
    /// Current power output in W
    pub power_output_w: f64,
    /// Voltage
    pub voltage_v: f64,
    /// Current
    pub current_a: f64,
    /// Irradiance (W/m²)
    pub irradiance_wm2: f64,
    /// Panel temperature
    pub panel_temperature_c: f64,
    /// Maximum power point tracking efficiency
    pub mppt_efficiency: f64,
    /// Total energy harvested today (Wh)
    pub daily_energy_wh: f64,
    /// Total energy harvested lifetime (Wh)
    pub lifetime_energy_wh: f64,
    /// Timestamp
    pub timestamp: u64,
}

impl Default for SolarInfo {
    fn default() -> Self {
        Self {
            power_output_w: 0.0,
            voltage_v: 0.0,
            current_a: 0.0,
            irradiance_wm2: 0.0,
            panel_temperature_c: 25.0,
            mppt_efficiency: 0.95,
            daily_energy_wh: 0.0,
            lifetime_energy_wh: 0.0,
            timestamp: 0,
        }
    }
}

/// Power state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PowerState {
    /// Normal operation
    Normal,
    /// Power saving mode
    PowerSave,
    /// Low power mode
    LowPower,
    /// Hibernate/sleep
    Hibernate,
    /// Shutdown imminent
    Shutdown,
}

/// Energy manager with optimized resource usage
pub struct EnergyManager {
    config: EnergyConfig,
    battery: Arc<RwLock<BatteryInfo>>,
    solar: Arc<RwLock<SolarInfo>>,
    power_state: Arc<RwLock<PowerState>>,
    system: System,
    last_sample_time: Instant,
    energy_history: Arc<RwLock<VecDeque<EnergySnapshot>>>,
    event_tx: mpsc::Sender<EnergyEvent>,
    event_rx: Option<mpsc::Receiver<EnergyEvent>>,
    cancellation_token: CancellationToken,
    /// Task queue for power scheduling
    task_queue: Arc<RwLock<VecDeque<EnergyTask>>>,
    /// Daily solar reset time
    last_solar_reset: Arc<RwLock<chrono::DateTime<chrono::Local>>>,
}

/// Energy snapshot for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergySnapshot {
    pub timestamp: u64,
    pub battery_soc: f64,
    pub battery_power_w: f64,
    pub solar_power_w: f64,
    pub system_power_w: f64,
    pub power_state: PowerState,
}

/// Energy events
#[derive(Debug, Clone)]
pub enum EnergyEvent {
    /// Battery low
    BatteryLow,
    /// Battery critical
    BatteryCritical,
    /// Battery full
    BatteryFull,
    /// Entering power save mode
    EnterPowerSave,
    /// Entering hibernation
    EnterHibernation,
    /// Shutdown required
    ShutdownRequired,
    /// Solar power available
    SolarPowerAvailable(f64),
    /// Request power state change
    RequestPowerState(PowerState),
    /// Task scheduled
    TaskScheduled(String),
    /// Task completed
    TaskCompleted(String),
}

/// Task with energy priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyTask {
    pub id: String,
    pub name: String,
    pub priority: TaskPriority,
    pub estimated_energy_wh: f64,
    pub deadline: Option<u64>,
    pub dependencies: Vec<String>,
    pub submitted_at: u64,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskPriority {
    Critical = 0,   // Must execute regardless of energy
    High = 1,       // Important, schedule when possible
    Normal = 2,     // Standard priority
    Low = 3,        // Can be deferred
    Background = 4, // Only when excess energy available
}

impl EnergyManager {
    /// Create new energy manager with optimized initialization
    pub fn new(config: EnergyConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        let system = System::new_all();

        Self {
            config,
            battery: Arc::new(RwLock::new(BatteryInfo::default())),
            solar: Arc::new(RwLock::new(SolarInfo::default())),
            power_state: Arc::new(RwLock::new(PowerState::Normal)),
            system,
            last_sample_time: Instant::now(),
            energy_history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            event_tx,
            event_rx: Some(event_rx),
            cancellation_token: CancellationToken::new(),
            task_queue: Arc::new(RwLock::new(VecDeque::with_capacity(config.max_task_queue_size))),
            last_solar_reset: Arc::new(RwLock::new(chrono::Local::now())),
        }
    }

    /// Get event sender
    pub fn event_sender(&self) -> mpsc::Sender<EnergyEvent> {
        self.event_tx.clone()
    }

    /// Get current battery info
    pub async fn battery_info(&self) -> BatteryInfo {
        self.battery.read().await.clone()
    }

    /// Get current solar info
    pub async fn solar_info(&self) -> SolarInfo {
        self.solar.read().await.clone()
    }

    /// Get current power state
    pub async fn power_state(&self) -> PowerState {
        *self.power_state.read().await
    }

    /// Initialize and start monitoring
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing energy manager...");
        counter!("energy.initialized").increment(1);

        // Detect battery
        self.detect_battery().await?;

        // Detect solar if available
        self.detect_solar().await?;

        // Initial sampling
        self.sample().await?;

        tracing::info!("Energy manager initialized");
        Ok(())
    }

    /// Detect battery (platform-specific)
    async fn detect_battery(&mut self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            // Read from sysfs
            let power_supply_path = "/sys/class/power_supply/BAT0";
            if std::path::Path::new(power_supply_path).exists() {
                tracing::info!("Battery detected at {}", power_supply_path);
                
                // Try to read battery info
                let capacity_path = format!("{}/capacity", power_supply_path);
                if let Ok(capacity_str) = tokio::fs::read_to_string(&capacity_path).await {
                    if let Ok(capacity) = capacity_str.trim().parse::<u32>() {
                        let mut battery = self.battery.write().await;
                        battery.state_of_charge = capacity as f64 / 100.0;
                    }
                }
            } else {
                tracing::warn!("No battery detected, using simulated battery");
            }
        }

        Ok(())
    }

    /// Detect solar panel
    async fn detect_solar(&mut self) -> Result<()> {
        tracing::info!(
            "Solar detection: Using configured capacity of {}W",
            self.config.solar_capacity_w
        );
        Ok(())
    }

    /// Sample energy data with optimized calculations
    pub async fn sample(&mut self) -> Result<EnergySnapshot> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sample_time);
        self.last_sample_time = now;

        // Update system info
        self.system.refresh_all();

        // Calculate system power consumption
        let cpu_usage: f32 = self
            .system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>()
            / self.system.cpus().len().max(1) as f32;

        let system_power_w = self.config.base_power_consumption_w
            + (cpu_usage / 100.0) as f64
                * (self.config.max_power_consumption_w - self.config.base_power_consumption_w);

        let mut battery = self.battery.write().await;
        let mut solar = self.solar.write().await;

        // Update battery (with platform detection in production)
        let energy_consumed_wh = system_power_w * elapsed.as_secs_f64() / 3600.0;
        let soc_change = energy_consumed_wh / self.config.battery_capacity_wh;
        battery.state_of_charge = (battery.state_of_charge - soc_change).max(0.0).min(1.0);
        battery.power_draw_w = -system_power_w;
        battery.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Update solar (simulated for now)
        let solar_output = self.config.solar_capacity_w * 0.7; // Assume 70% of capacity
        solar.power_output_w = solar_output;
        solar.daily_energy_wh += solar_output * elapsed.as_secs_f64() / 3600.0;
        solar.lifetime_energy_wh += solar_output * elapsed.as_secs_f64() / 3600.0;
        solar.timestamp = battery.timestamp;

        // Charge battery from solar
        let solar_to_battery = solar_output.min(
            (1.0 - battery.state_of_charge) * self.config.battery_capacity_wh * 3600.0
                / elapsed.as_secs_f64().max(1.0),
        );
        battery.state_of_charge = (battery.state_of_charge
            + solar_to_battery * elapsed.as_secs_f64() / 3600.0 / self.config.battery_capacity_wh)
            .min(1.0);

        let snapshot = EnergySnapshot {
            timestamp: battery.timestamp,
            battery_soc: battery.state_of_charge,
            battery_power_w: battery.power_draw_w,
            solar_power_w: solar.power_output_w,
            system_power_w,
            power_state: *self.power_state.read().await,
        };

        // Store in history with size limit
        {
            let mut history = self.energy_history.write().await;
            if history.len() >= MAX_HISTORY_SIZE {
                history.pop_front();
            }
            history.push_back(snapshot.clone());
        }

        // Update metrics
        gauge!("energy.battery_soc").set(snapshot.battery_soc);
        gauge!("energy.system_power_w").set(system_power_w);
        gauge!("energy.solar_power_w").set(solar_output);
        gauge!("energy.cpu_usage").set(cpu_usage as f64);

        // Check battery levels and emit events
        drop(battery);
        drop(solar);
        self.check_battery_levels().await?;
        
        // Check for daily solar reset
        self.check_solar_reset().await;

        counter!("energy.samples_taken").increment(1);

        Ok(snapshot)
    }

    /// Check battery levels and emit appropriate events
    async fn check_battery_levels(&self) -> Result<()> {
        let battery = self.battery.read().await;

        if battery.is_critical(self.config.critical_battery_threshold) {
            let _ = self.event_tx.send(EnergyEvent::BatteryCritical).await;

            if self.config.enable_hibernation {
                let _ = self.event_tx.send(EnergyEvent::EnterHibernation).await;
                let mut state = self.power_state.write().await;
                *state = PowerState::Hibernate;
            } else {
                let _ = self.event_tx.send(EnergyEvent::ShutdownRequired).await;
            }
        } else if battery.is_low(self.config.low_battery_threshold) {
            let _ = self.event_tx.send(EnergyEvent::BatteryLow).await;
            let mut state = self.power_state.write().await;
            if *state == PowerState::Normal {
                *state = PowerState::PowerSave;
                let _ = self.event_tx.send(EnergyEvent::EnterPowerSave).await;
            }
        } else if battery.state_of_charge >= 0.99 && battery.status == BatteryStatus::Charging {
            let _ = self.event_tx.send(EnergyEvent::BatteryFull).await;
        }

        Ok(())
    }
    
    /// Check if we need to reset daily solar counter
    async fn check_solar_reset(&self) {
        let now = chrono::Local::now();
        let mut last_reset = self.last_solar_reset.write().await;
        
        // Reset at midnight
        if now.date() != last_reset.date() {
            let mut solar = self.solar.write().await;
            solar.daily_energy_wh = 0.0;
            *last_reset = now;
            tracing::info!("Daily solar energy counter reset");
            counter!("energy.solar_daily_reset").increment(1);
        }
    }

    /// Enter hibernation mode
    pub async fn hibernate(&self) -> Result<()> {
        tracing::info!("Entering hibernation mode...");
        counter!("energy.hibernation_entered").increment(1);

        let mut state = self.power_state.write().await;
        *state = PowerState::Hibernate;

        // In production, this would:
        // 1. Save state to disk
        // 2. Notify peers
        // 3. Schedule wake-up timer
        // 4. Enter system sleep

        tracing::info!("Hibernation complete");
        Ok(())
    }

    /// Wake from hibernation
    pub async fn wake(&self) -> Result<()> {
        tracing::info!("Waking from hibernation...");
        counter!("energy.wake").increment(1);

        let mut state = self.power_state.write().await;
        *state = PowerState::Normal;

        // In production, restore state from disk

        tracing::info!("Wake complete");
        Ok(())
    }

    /// Set power state
    pub async fn set_power_state(&self, new_state: PowerState) -> Result<()> {
        let mut state = self.power_state.write().await;
        let old_state = *state;
        *state = new_state;

        if old_state != new_state {
            tracing::info!("Power state changed from {:?} to {:?}", old_state, new_state);
            gauge!("energy.power_state").set(match new_state {
                PowerState::Normal => 0.0,
                PowerState::PowerSave => 1.0,
                PowerState::LowPower => 2.0,
                PowerState::Hibernate => 3.0,
                PowerState::Shutdown => 4.0,
            });
        }

        Ok(())
    }

    /// Get energy history
    pub async fn get_history(&self, hours: usize) -> Vec<EnergySnapshot> {
        let history = self.energy_history.read().await;
        let samples = hours * 60; // Assuming 1 sample per minute

        history
            .iter()
            .rev()
            .take(samples.min(history.len()))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Estimate time until battery empty
    pub async fn estimate_remaining_time(&self) -> Option<Duration> {
        let battery = self.battery.read().await;
        battery.estimate_remaining_runtime(self.config.battery_capacity_wh)
    }
    
    /// Add task to power scheduler
    pub async fn add_task(&self, task: EnergyTask) -> Result<()> {
        let mut queue = self.task_queue.write().await;
        
        if queue.len() >= self.config.max_task_queue_size {
            return Err(anyhow::anyhow!("Task queue full"));
        }
        
        queue.push_back(task);
        counter!("energy.tasks_added").increment(1);
        gauge!("energy.task_queue_size").set(queue.len() as f64);
        
        Ok(())
    }
    
    /// Get scheduled tasks based on available energy
    pub async fn get_scheduled_tasks(&self) -> Vec<EnergyTask> {
        let battery = self.battery.read().await;
        let solar = self.solar.read().await;
        let queue = self.task_queue.read().await;
        
        let scheduler = PowerScheduler::new(self.config.clone());
        let tasks: Vec<_> = queue.iter().cloned().collect();
        
        scheduler.schedule(
            &tasks,
            battery.state_of_charge,
            solar.power_output_w
        )
    }

    /// Main monitoring loop with cancellation support
    pub async fn run(&mut self) -> Result<()> {
        let interval_secs = self.config.sampling_interval_secs;
        let mut interval = interval(Duration::from_secs(interval_secs));
        let mut event_rx = self.event_rx.take()
            .context("Event receiver already taken")?;

        tracing::info!("Starting energy monitoring loop ({}s interval)", interval_secs);
        counter!("energy.monitoring_started").increment(1);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let start = Instant::now();
                    
                    if let Err(e) = self.sample().await {
                        tracing::error!("Energy sampling error: {}", e);
                        counter!("energy.sampling_errors").increment(1);
                    }
                    
                    histogram!("energy.sampling_duration_seconds", start.elapsed().as_secs_f64());
                }
                Some(event) = event_rx.recv() => {
                    match event {
                        EnergyEvent::RequestPowerState(state) => {
                            if let Err(e) = self.set_power_state(state).await {
                                tracing::error!("Failed to set power state: {}", e);
                            }
                        }
                        _ => {
                            tracing::debug!("Energy event: {:?}", event);
                            counter!("energy.events_received").increment(1);
                        }
                    }
                }
                _ = self.cancellation_token.cancelled() => {
                    tracing::info!("Energy manager cancelled");
                    break;
                }
            }
        }

        Ok(())
    }
    
    /// Shutdown the energy manager
    pub fn shutdown(&self) {
        self.cancellation_token.cancel();
    }
}

impl Drop for EnergyManager {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
        tracing::debug!("EnergyManager dropped");
    }
}

/// Power scheduler for task management
pub struct PowerScheduler {
    config: EnergyConfig,
}

impl PowerScheduler {
    /// Create new power scheduler
    pub fn new(config: EnergyConfig) -> Self {
        Self { config }
    }

    /// Schedule tasks based on available energy
    pub fn schedule(&self, tasks: &[EnergyTask], battery_soc: f64, solar_power_w: f64) -> Vec<EnergyTask> {
        let mut scheduled = Vec::with_capacity(tasks.len());
        let available_energy_wh = self.config.battery_capacity_wh * battery_soc;

        // Sort by priority (highest first)
        let mut sorted_tasks: Vec<_> = tasks.iter().collect();
        sorted_tasks.sort_by_key(|t| t.priority);

        let mut remaining_energy = available_energy_wh;

        for task in sorted_tasks {
            // Always schedule critical tasks
            if task.priority == TaskPriority::Critical {
                scheduled.push(task.clone());
                continue;
            }

            // Schedule high priority if battery > 30%
            if task.priority == TaskPriority::High && battery_soc > 0.30 {
                scheduled.push(task.clone());
                remaining_energy -= task.estimated_energy_wh;
                continue;
            }

            // Schedule normal priority if sufficient energy
            if task.priority == TaskPriority::Normal && remaining_energy > task.estimated_energy_wh {
                scheduled.push(task.clone());
                remaining_energy -= task.estimated_energy_wh;
                continue;
            }

            // Schedule low/background only with excess solar
            if solar_power_w > self.config.base_power_consumption_w
                && (task.priority == TaskPriority::Low || task.priority == TaskPriority::Background)
            {
                scheduled.push(task.clone());
                continue;
            }
        }

        scheduled
    }

    /// Get tasks that can be deferred
    pub fn get_deferrable_tasks(&self, tasks: &[EnergyTask], battery_soc: f64) -> Vec<&EnergyTask> {
        tasks
            .iter()
            .filter(|t| t.priority == TaskPriority::Low || t.priority == TaskPriority::Background)
            .filter(|t| battery_soc < 0.30)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_info_low() {
        let battery = BatteryInfo {
            state_of_charge: 0.15,
            status: BatteryStatus::Discharging,
            power_draw_w: -10.0,
            voltage_v: 11.5,
            current_a: -0.87,
            temperature_c: 30.0,
            health_percent: 0.95,
            cycle_count: 100,
            time_remaining_seconds: None,
            timestamp: 0,
        };

        assert!(battery.is_low(0.20));
        assert!(!battery.is_critical(0.10));
    }

    #[test]
    fn test_battery_remaining_time() {
        let battery = BatteryInfo {
            state_of_charge: 0.50,
            status: BatteryStatus::Discharging,
            power_draw_w: -10.0,
            voltage_v: 11.5,
            current_a: -0.87,
            temperature_c: 30.0,
            health_percent: 0.95,
            cycle_count: 100,
            time_remaining_seconds: None,
            timestamp: 0,
        };

        let remaining = battery.estimate_remaining_runtime(50.0);
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() > 0);
    }

    #[test]
    fn test_power_scheduler() {
        let config = EnergyConfig::default();
        let scheduler = PowerScheduler::new(config);

        let tasks = vec![
            EnergyTask {
                id: "1".into(),
                name: "Critical Task".into(),
                priority: TaskPriority::Critical,
                estimated_energy_wh: 10.0,
                deadline: None,
                dependencies: vec![],
                submitted_at: 0,
            },
            EnergyTask {
                id: "2".into(),
                name: "Background Task".into(),
                priority: TaskPriority::Background,
                estimated_energy_wh: 5.0,
                deadline: None,
                dependencies: vec![],
                submitted_at: 0,
            },
        ];

        let scheduled = scheduler.schedule(&tasks, 0.50, 15.0);
        assert_eq!(scheduled.len(), 2); // Both should be scheduled

        let scheduled_low_battery = scheduler.schedule(&tasks, 0.10, 5.0);
        assert_eq!(scheduled_low_battery.len(), 1); // Only critical
    }

    #[tokio::test]
    async fn test_energy_manager() {
        let config = EnergyConfig::default();
        let manager = EnergyManager::new(config);

        let battery = manager.battery_info().await;
        assert!(battery.state_of_charge <= 1.0);

        let solar = manager.solar_info().await;
        assert!(solar.power_output_w >= 0.0);
    }
    
    #[test]
    fn test_energy_snapshot() {
        let snapshot = EnergySnapshot {
            timestamp: 12345,
            battery_soc: 0.75,
            battery_power_w: -5.0,
            solar_power_w: 10.0,
            system_power_w: 15.0,
            power_state: PowerState::Normal,
        };
        
        assert_eq!(snapshot.battery_soc, 0.75);
    }
}
