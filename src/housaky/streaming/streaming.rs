use crate::providers::{ChatMessage, Provider};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub content: String,
    pub delta: String,
    pub is_complete: bool,
    pub token_count: usize,
    pub elapsed_ms: u64,
    pub tokens_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamState {
    Idle,
    Starting,
    Streaming,
    Completing,
    Complete,
    Error,
}

pub struct StreamingSession {
    pub id: String,
    pub state: StreamState,
    pub content: String,
    pub token_count: usize,
    pub start_time: std::time::Instant,
    pub last_update: std::time::Instant,
    pub chunks_received: usize,
}

impl StreamingSession {
    pub fn new() -> Self {
        Self {
            id: format!("stream_{}", uuid::Uuid::new_v4()),
            state: StreamState::Idle,
            content: String::new(),
            token_count: 0,
            start_time: std::time::Instant::now(),
            last_update: std::time::Instant::now(),
            chunks_received: 0,
        }
    }

    pub fn start(&mut self) {
        self.state = StreamState::Starting;
        self.start_time = std::time::Instant::now();
        self.last_update = std::time::Instant::now();
        self.content.clear();
        self.token_count = 0;
        self.chunks_received = 0;
    }

    pub fn append(&mut self, delta: &str) -> StreamChunk {
        self.content.push_str(delta);
        self.token_count += delta.split_whitespace().count();
        self.chunks_received += 1;
        self.last_update = std::time::Instant::now();
        self.state = StreamState::Streaming;

        let elapsed = self.start_time.elapsed().as_millis() as u64;
        let tps = if elapsed > 0 && self.token_count > 0 {
            (self.token_count as f64) / (elapsed as f64 / 1000.0)
        } else {
            0.0
        };

        StreamChunk {
            id: self.id.clone(),
            content: self.content.clone(),
            delta: delta.to_string(),
            is_complete: false,
            token_count: self.token_count,
            elapsed_ms: elapsed,
            tokens_per_second: tps,
        }
    }

    pub fn complete(&mut self) -> StreamChunk {
        self.state = StreamState::Complete;
        let elapsed = self.start_time.elapsed().as_millis() as u64;
        let tps = if elapsed > 0 && self.token_count > 0 {
            (self.token_count as f64) / (elapsed as f64 / 1000.0)
        } else {
            0.0
        };

        StreamChunk {
            id: self.id.clone(),
            content: self.content.clone(),
            delta: String::new(),
            is_complete: true,
            token_count: self.token_count,
            elapsed_ms: elapsed,
            tokens_per_second: tps,
        }
    }

    pub fn error(&mut self, error: &str) -> StreamChunk {
        self.state = StreamState::Error;
        StreamChunk {
            id: self.id.clone(),
            content: format!("Error: {}", error),
            delta: error.to_string(),
            is_complete: true,
            token_count: 0,
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
            tokens_per_second: 0.0,
        }
    }

    pub fn tokens_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_millis() as f64 / 1000.0;
        if elapsed > 0.0 {
            self.token_count as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

pub struct StreamingManager {
    current_session: Arc<RwLock<StreamingSession>>,
    chunk_sender: broadcast::Sender<StreamChunk>,
    state_sender: broadcast::Sender<StreamState>,
}

impl StreamingManager {
    pub fn new() -> Self {
        let (chunk_sender, _) = broadcast::channel(256);
        let (state_sender, _) = broadcast::channel(16);

        Self {
            current_session: Arc::new(RwLock::new(StreamingSession::new())),
            chunk_sender,
            state_sender,
        }
    }

    pub async fn start_stream(&self) {
        let mut session = self.current_session.write().await;
        session.start();
        let _ = self.state_sender.send(StreamState::Starting);
        info!("Started streaming session: {}", session.id);
    }

    pub async fn append_chunk(&self, delta: &str) {
        let mut session = self.current_session.write().await;
        let chunk = session.append(delta);
        let _ = self.chunk_sender.send(chunk);
    }

    pub async fn complete_stream(&self) -> StreamChunk {
        let mut session = self.current_session.write().await;
        let chunk = session.complete();
        let _ = self.chunk_sender.send(chunk.clone());
        let _ = self.state_sender.send(StreamState::Complete);
        info!(
            "Completed streaming session: {} ({} tokens)",
            session.id, session.token_count
        );
        chunk
    }

    pub async fn error_stream(&self, error: &str) {
        let mut session = self.current_session.write().await;
        let chunk = session.error(error);
        let _ = self.chunk_sender.send(chunk);
        let _ = self.state_sender.send(StreamState::Error);
    }

    pub fn subscribe_chunks(&self) -> broadcast::Receiver<StreamChunk> {
        self.chunk_sender.subscribe()
    }

    pub fn subscribe_state(&self) -> broadcast::Receiver<StreamState> {
        self.state_sender.subscribe()
    }

    pub async fn get_current_content(&self) -> String {
        self.current_session.read().await.content.clone()
    }

    pub async fn get_current_state(&self) -> StreamState {
        self.current_session.read().await.state.clone()
    }

    pub async fn get_stats(&self) -> StreamStats {
        let session = self.current_session.read().await;
        StreamStats {
            session_id: session.id.clone(),
            state: session.state.clone(),
            content_length: session.content.len(),
            token_count: session.token_count,
            chunks_received: session.chunks_received,
            elapsed_ms: session.elapsed_ms(),
            tokens_per_second: session.tokens_per_second(),
        }
    }

    pub async fn stream_chat(
        &self,
        provider: &dyn Provider,
        model: &str,
        messages: &[ChatMessage],
        temperature: f64,
    ) -> Result<String> {
        self.start_stream().await;

        let result = provider
            .chat_with_history(messages, model, temperature)
            .await;

        match result {
            Ok(response) => {
                self.append_chunk(&response).await;
                let final_chunk = self.complete_stream().await;
                Ok(final_chunk.content)
            }
            Err(e) => {
                self.error_stream(&e.to_string()).await;
                Err(e)
            }
        }
    }

    pub async fn stream_chat_incremental(
        &self,
        provider: &dyn Provider,
        model: &str,
        messages: &[ChatMessage],
        temperature: f64,
        chunk_size: usize,
    ) -> Result<String> {
        self.start_stream().await;

        let full_response = provider
            .chat_with_history(messages, model, temperature)
            .await?;

        let mut remaining = full_response.as_str();
        while !remaining.is_empty() {
            let chunk_end = remaining
                .char_indices()
                .nth(chunk_size)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());

            let chunk = &remaining[..chunk_end];
            self.append_chunk(chunk).await;
            remaining = &remaining[chunk_end..];

            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        let final_chunk = self.complete_stream().await;
        Ok(final_chunk.content)
    }

    pub async fn simulate_stream(&self, content: &str, delay_ms: u64) {
        self.start_stream().await;

        let words: Vec<&str> = content.split_whitespace().collect();
        let chunk_size = (words.len() / 20).max(1);

        for chunk in words.chunks(chunk_size) {
            let chunk_text = chunk.join(" ") + " ";
            self.append_chunk(&chunk_text).await;
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }

        self.complete_stream().await;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub session_id: String,
    pub state: StreamState,
    pub content_length: usize,
    pub token_count: usize,
    pub chunks_received: usize,
    pub elapsed_ms: u64,
    pub tokens_per_second: f64,
}

impl Default for StreamingManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StreamingTask {
    manager: Arc<StreamingManager>,
    provider: Box<dyn Provider>,
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
}

impl StreamingTask {
    pub fn new(
        manager: Arc<StreamingManager>,
        provider: Box<dyn Provider>,
        model: String,
        messages: Vec<ChatMessage>,
        temperature: f64,
    ) -> Self {
        Self {
            manager,
            provider,
            model,
            messages,
            temperature,
        }
    }

    pub async fn run(self) -> Result<String> {
        self.manager
            .stream_chat(
                self.provider.as_ref(),
                &self.model,
                &self.messages,
                self.temperature,
            )
            .await
    }
}
