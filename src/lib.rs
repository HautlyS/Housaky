#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::assigning_clones,
    clippy::bool_to_int_with_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::field_reassign_with_default,
    clippy::float_cmp,
    clippy::implicit_clone,
    clippy::items_after_statements,
    clippy::map_unwrap_or,
    clippy::manual_let_else,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::new_without_default,
    clippy::needless_pass_by_value,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::return_self_not_must_use,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_map_or,
    clippy::unused_self,
    clippy::cast_precision_loss,
    clippy::unnecessary_wraps,
    dead_code
)]

pub mod agent;
pub mod channels;
pub mod commands;
pub mod config;
pub mod config_editor;
pub mod cost;
pub mod cron;
pub mod daemon;
pub mod doctor;
pub mod gateway;
pub mod hardware;
pub mod health;
pub mod heartbeat;
pub mod housaky;
pub mod identity;
pub mod integrations;
pub mod memory;
pub mod migration;
pub mod observability;
pub mod onboard;
pub mod peripherals;
pub mod providers;
pub mod rag;
pub mod runtime;
pub mod security;
pub mod service;
pub mod skillforge;
pub mod skills;
pub mod tools;
pub mod tui;
pub mod tunnel;
pub mod util;
pub mod vkm_client;

pub use commands::ChannelCommands;
pub use commands::CronCommands;
pub use commands::GoalCommands;
pub use commands::HardwareCommands;
pub use commands::HousakyCommands;
pub use commands::IntegrationCommands;
pub use commands::MigrateCommands;
pub use commands::ModelCommands;
pub use commands::PeripheralCommands;
pub use commands::ServiceCommands;
pub use commands::SkillCommands;
pub use config::Config;
