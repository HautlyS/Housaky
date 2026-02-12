//! Housaky Energy - Energy management
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessExt, System, SystemExt};

pub mod energy_manager;
pub mod monitor;

pub use energy_manager::*;
pub use monitor::*;
