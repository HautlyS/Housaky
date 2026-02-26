//! Emergent Communication Protocol
//!
//! Allows agents to develop efficient shorthand communication:
//! - Define symbols for frequently-communicated concepts
//! - Track efficiency (bits per coordination)
//! - Evolve protocol: symbols that speed coordination survive
//! - Shared symbol table with version negotiation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolSymbol {
    pub symbol: String,
    pub meaning: String,
    pub usage_count: u64,
    pub avg_coordination_speedup: f64,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationMetrics {
    pub total_messages: u64,
    pub total_symbols_used: u64,
    pub avg_message_length: f64,
    pub coordination_success_rate: f64,
    pub bits_per_coordination: f64,
}

impl Default for CommunicationMetrics {
    fn default() -> Self {
        Self {
            total_messages: 0,
            total_symbols_used: 0,
            avg_message_length: 0.0,
            coordination_success_rate: 0.0,
            bits_per_coordination: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub sender: String,
    pub receiver: String,
    pub content: String,
    pub symbols_used: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub coordination_id: Option<String>,
}

pub struct EmergentProtocol {
    pub symbol_table: Arc<RwLock<HashMap<String, ProtocolSymbol>>>,
    pub metrics: Arc<RwLock<CommunicationMetrics>>,
    pub message_log: Arc<RwLock<Vec<ProtocolMessage>>>,
    pub protocol_version: Arc<RwLock<u32>>,
}

impl EmergentProtocol {
    pub fn new() -> Self {
        Self {
            symbol_table: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(CommunicationMetrics::default())),
            message_log: Arc::new(RwLock::new(Vec::new())),
            protocol_version: Arc::new(RwLock::new(1)),
        }
    }

    /// Define a new symbol for frequent communication.
    pub async fn define_symbol(&self, symbol: &str, meaning: &str, creator: &str) {
        let ps = ProtocolSymbol {
            symbol: symbol.to_string(),
            meaning: meaning.to_string(),
            usage_count: 0,
            avg_coordination_speedup: 0.0,
            created_at: Utc::now(),
            created_by: creator.to_string(),
            version: *self.protocol_version.read().await,
        };
        self.symbol_table.write().await.insert(symbol.to_string(), ps);
        info!("New protocol symbol: '{}' = '{}'", symbol, meaning);
    }

    /// Encode a message using known symbols.
    pub async fn encode(&self, message: &str) -> (String, Vec<String>) {
        let table = self.symbol_table.read().await;
        let mut encoded = message.to_string();
        let mut symbols_used = Vec::new();

        for (symbol, ps) in table.iter() {
            if encoded.contains(&ps.meaning) {
                encoded = encoded.replace(&ps.meaning, symbol);
                symbols_used.push(symbol.clone());
            }
        }

        (encoded, symbols_used)
    }

    /// Decode a message using known symbols.
    pub async fn decode(&self, message: &str) -> String {
        let table = self.symbol_table.read().await;
        let mut decoded = message.to_string();

        for (symbol, ps) in table.iter() {
            decoded = decoded.replace(symbol, &ps.meaning);
        }

        decoded
    }

    /// Record a message and update metrics.
    pub async fn record_message(&self, msg: ProtocolMessage) {
        // Update symbol usage counts
        let mut table = self.symbol_table.write().await;
        for sym in &msg.symbols_used {
            if let Some(ps) = table.get_mut(sym) {
                ps.usage_count += 1;
            }
        }
        drop(table);

        let mut metrics = self.metrics.write().await;
        metrics.total_messages += 1;
        metrics.total_symbols_used += msg.symbols_used.len() as u64;

        // Update average message length
        let n = metrics.total_messages as f64;
        metrics.avg_message_length =
            (metrics.avg_message_length * (n - 1.0) + msg.content.len() as f64) / n;

        drop(metrics);
        self.message_log.write().await.push(msg);
    }

    /// Evolve the protocol: remove unused symbols, reinforce popular ones.
    pub async fn evolve(&self) {
        let mut table = self.symbol_table.write().await;

        // Remove symbols with very low usage
        let to_remove: Vec<String> = table
            .iter()
            .filter(|(_, ps)| {
                ps.usage_count < 2
                    && (Utc::now() - ps.created_at).num_hours() > 24
            })
            .map(|(k, _)| k.clone())
            .collect();

        for key in &to_remove {
            table.remove(key);
        }

        if !to_remove.is_empty() {
            info!("Evolved protocol: removed {} unused symbols", to_remove.len());
        }

        drop(table);
        *self.protocol_version.write().await += 1;
    }

    pub async fn get_stats(&self) -> EmergentProtocolStats {
        let table = self.symbol_table.read().await;
        let metrics = self.metrics.read().await;
        let version = *self.protocol_version.read().await;

        EmergentProtocolStats {
            total_symbols: table.len(),
            protocol_version: version,
            total_messages: metrics.total_messages,
            avg_message_length: metrics.avg_message_length,
            most_used_symbol: table
                .values()
                .max_by_key(|ps| ps.usage_count)
                .map(|ps| ps.symbol.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentProtocolStats {
    pub total_symbols: usize,
    pub protocol_version: u32,
    pub total_messages: u64,
    pub avg_message_length: f64,
    pub most_used_symbol: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_symbol_encode_decode() {
        let proto = EmergentProtocol::new();
        proto.define_symbol("$ACK", "task acknowledged", "agent-1").await;
        proto.define_symbol("$DONE", "task completed successfully", "agent-1").await;

        let (encoded, symbols) = proto.encode("task acknowledged and task completed successfully").await;
        assert!(encoded.contains("$ACK"));
        assert!(encoded.contains("$DONE"));
        assert_eq!(symbols.len(), 2);

        let decoded = proto.decode(&encoded).await;
        assert!(decoded.contains("task acknowledged"));
    }
}
