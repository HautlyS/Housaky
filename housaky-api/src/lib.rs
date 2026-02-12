//! Housaky API - Public API and CLI
use anyhow::Result;

pub mod cli;
pub mod server;

pub use cli::*;
pub use server::*;
