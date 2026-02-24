// AGI Lab Interface Web Module
// Provides HTTP server, WebSocket support, and AGI command endpoints

pub mod server;
pub mod handlers;
pub mod websocket;
pub mod models;
pub mod middleware;

pub use server::start_server;