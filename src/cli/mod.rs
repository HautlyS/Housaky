//! CLI module for Housaky
//!
//! This module contains all CLI-related functionality including:
//! - Argument parsing (`args`)
//! - Command handlers (`handlers`)
//! - Utility functions (`utils`)
//! - Quantum command handlers (`quantum`)
//! - Help system (`help`)

pub mod args;
pub mod handlers;
pub mod help;
pub mod quantum;
pub mod utils;

pub use args::{Cli, Commands, DaemonAction, TuiAction};
pub use handlers::{
    handle_agent, handle_config, handle_daemon, handle_dashboard, handle_doctor, handle_gateway,
    handle_onboard, handle_status,
};
pub use help::{run_tui_command, HelpSystem};
pub use utils::{
    get_api_key_from_keys_manager_or_config, is_daemon_running, is_daemon_running_on, is_first_run,
    print_channel_status,
};
