//! Energy monitoring
use serde::{Deserialize, Serialize};
use std::time::Duration;
use sysinfo::{CpuExt, ProcessExt, System, SystemExt};

/// Energy metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyMetrics {
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u64,
    pub power_estimate_watts: f64,
    pub timestamp: u64,
}

/// Energy monitor
pub struct EnergyMonitor {
    system: System,
    pid: sysinfo::Pid,
}

impl EnergyMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let pid = sysinfo::get_current_pid().unwrap();

        Self { system, pid }
    }

    pub fn sample(&mut self) -> EnergyMetrics {
        self.system.refresh_all();

        let cpu_usage: f32 = self
            .system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>()
            / self.system.cpus().len() as f32;

        let memory_usage = self
            .system
            .process(self.pid)
            .map(|p| p.memory())
            .unwrap_or(0);

        // Rough power estimate: ~10W base + 50W at full CPU
        let power_estimate = 10.0 + (cpu_usage / 100.0) as f64 * 50.0;

        EnergyMetrics {
            cpu_usage_percent: cpu_usage,
            memory_usage_mb: memory_usage / 1024,
            power_estimate_watts: power_estimate,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}
