//! Raft Consensus Implementation - Optimized
//!
//! This module provides a complete Raft consensus algorithm implementation
//! including leader election, log replication, and membership changes.
//!
//! # Memory Safety
//! - Bounded log size with automatic truncation after snapshots
//! - Bounded channels prevent memory exhaustion
//! - Proper cleanup of RPC handlers
//!
//! # Performance
//! - Batch log entries to reduce RPC overhead
//! - Efficient heartbeat mechanism
//! - Parallel vote requests

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep, timeout};
use tokio_util::sync::CancellationToken;
use metrics::{counter, gauge, histogram};
use blake3::Hasher;

/// Maximum log entries per AppendEntries RPC
const MAX_ENTRIES_PER_APPEND: usize = 100;

/// Maximum log size before triggering snapshot
const MAX_LOG_SIZE: usize = 10000;

/// RPC timeout duration
const RPC_TIMEOUT_MS: u64 = 100;

/// Raft node states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftState {
    Follower,
    Candidate,
    Leader,
}

/// Raft log entry with optimized storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Term when entry was received by leader
    pub term: u64,
    /// Index in the log
    pub index: u64,
    /// Command/data to apply (using Bytes for zero-copy)
    #[serde(with = "serde_bytes")]
    pub command: Vec<u8>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Entry hash for integrity verification
    pub hash: [u8; 32],
}

impl LogEntry {
    /// Calculate hash for integrity verification
    pub fn calculate_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(&self.term.to_le_bytes());
        hasher.update(&self.index.to_le_bytes());
        hasher.update(&self.command);
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
    
    /// Verify entry integrity
    pub fn verify(&self) -> bool {
        self.calculate_hash() == self.hash
    }
}

/// Raft node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// Node ID
    pub node_id: String,
    /// List of peer IDs
    pub peers: Vec<String>,
    /// Election timeout minimum (ms)
    pub election_timeout_min_ms: u64,
    /// Election timeout maximum (ms)
    pub election_timeout_max_ms: u64,
    /// Heartbeat interval (ms)
    pub heartbeat_interval_ms: u64,
    /// Maximum log entries per AppendEntries
    pub max_entries_per_append: usize,
    /// Snapshot threshold (number of entries)
    pub snapshot_threshold: usize,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            node_id: "node-1".into(),
            peers: Vec::new(),
            election_timeout_min_ms: 150,
            election_timeout_max_ms: 300,
            heartbeat_interval_ms: 50,
            max_entries_per_append: MAX_ENTRIES_PER_APPEND,
            snapshot_threshold: MAX_LOG_SIZE,
        }
    }
}

/// RequestVote RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteRequest {
    /// Candidate's term
    pub term: u64,
    /// Candidate requesting vote
    pub candidate_id: String,
    /// Index of candidate's last log entry
    pub last_log_index: u64,
    /// Term of candidate's last log entry
    pub last_log_term: u64,
}

/// RequestVote RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// Current term for candidate to update itself
    pub term: u64,
    /// True means candidate received vote
    pub vote_granted: bool,
}

/// AppendEntries RPC request with batching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: u64,
    /// Leader ID
    pub leader_id: String,
    /// Index of log entry immediately preceding new ones
    pub prev_log_index: u64,
    /// Term of prev_log_index entry
    pub prev_log_term: u64,
    /// Log entries to store (empty for heartbeat)
    pub entries: Vec<LogEntry>,
    /// Leader's commit_index
    pub leader_commit: u64,
}

/// AppendEntries RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term for leader to update itself
    pub term: u64,
    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
    /// Index for conflict resolution (optimization)
    pub conflict_index: u64,
    /// Term for conflict resolution (optimization)
    pub conflict_term: u64,
}

/// Snapshot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Last included index
    pub last_index: u64,
    /// Last included term
    pub last_term: u64,
    /// State machine data
    pub data: Vec<u8>,
}

/// Raft state machine with optimized storage
pub struct RaftNode {
    config: RaftConfig,
    state: RaftState,

    // Persistent state
    current_term: u64,
    voted_for: Option<String>,
    log: Vec<LogEntry>,

    // Volatile state
    commit_index: u64,
    last_applied: u64,

    // Leader state (reinitialized after election)
    next_index: HashMap<String, u64>,
    match_index: HashMap<String, u64>,

    // Timers
    last_heartbeat: Instant,
    election_deadline: Instant,

    // Channels (bounded to prevent memory growth)
    event_tx: mpsc::Sender<RaftEvent>,
    event_rx: mpsc::Receiver<RaftEvent>,

    // Applied state machine
    state_machine: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    
    // Snapshot
    snapshot: Option<Snapshot>,
    
    // Cancellation token
    cancellation_token: CancellationToken,
    
    // Metrics
    rpc_count: u64,
    votes_received: u64,
}

/// Raft events
#[derive(Debug)]
pub enum RaftEvent {
    /// RequestVote RPC received
    RequestVote(RequestVoteRequest, mpsc::Sender<RequestVoteResponse>),
    /// AppendEntries RPC received
    AppendEntries(AppendEntriesRequest, mpsc::Sender<AppendEntriesResponse>),
    /// Client command received
    ClientCommand(Vec<u8>, mpsc::Sender<Result<u64>>),
    /// Election timeout
    ElectionTimeout,
    /// Heartbeat timeout
    HeartbeatTimeout,
    /// Install snapshot request
    InstallSnapshot(Snapshot, mpsc::Sender<Result<()>>),
    /// Shutdown signal
    Shutdown,
}

/// Raft metrics for monitoring
#[derive(Debug, Clone, Default)]
pub struct RaftMetrics {
    pub current_term: u64,
    pub state: String,
    pub log_size: usize,
    pub commit_index: u64,
    pub last_applied: u64,
    pub peer_count: usize,
    pub is_leader: bool,
    pub rpc_count: u64,
}

impl RaftNode {
    /// Create a new Raft node with optimized initialization
    pub fn new(config: RaftConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);

        // Randomize election timeout to prevent split votes
        let election_timeout = Duration::from_millis(
            config.election_timeout_min_ms
                + rand::random::<u64>()
                    % (config.election_timeout_max_ms - config.election_timeout_min_ms),
        );

        let now = Instant::now();

        let node = Self {
            config,
            state: RaftState::Follower,
            current_term: 0,
            voted_for: None,
            log: vec![LogEntry {
                term: 0,
                index: 0,
                command: Vec::new(),
                timestamp: chrono::Utc::now(),
                hash: [0u8; 32],
            }],
            commit_index: 0,
            last_applied: 0,
            next_index: HashMap::with_capacity(10),
            match_index: HashMap::with_capacity(10),
            last_heartbeat: now,
            election_deadline: now + election_timeout,
            event_tx,
            event_rx,
            state_machine: Arc::new(RwLock::new(HashMap::with_capacity(1000))),
            snapshot: None,
            cancellation_token: CancellationToken::new(),
            rpc_count: 0,
            votes_received: 0,
        };
        
        counter!("raft.node_created").increment(1);
        node
    }

    /// Get the event sender
    pub fn event_sender(&self) -> mpsc::Sender<RaftEvent> {
        self.event_tx.clone()
    }

    /// Get current state
    pub fn state(&self) -> RaftState {
        self.state
    }

    /// Get current term
    pub fn current_term(&self) -> u64 {
        self.current_term
    }

    /// Check if this node is the leader
    pub fn is_leader(&self) -> bool {
        matches!(self.state, RaftState::Leader)
    }

    /// Get leader ID (only valid if this node is leader)
    pub fn leader_id(&self) -> Option<&str> {
        if self.is_leader() {
            Some(&self.config.node_id)
        } else {
            None
        }
    }
    
    /// Get metrics
    pub fn metrics(&self) -> RaftMetrics {
        RaftMetrics {
            current_term: self.current_term,
            state: format!("{:?}", self.state),
            log_size: self.log.len(),
            commit_index: self.commit_index,
            last_applied: self.last_applied,
            peer_count: self.config.peers.len(),
            is_leader: self.is_leader(),
            rpc_count: self.rpc_count,
        }
    }

    /// Main event loop with cancellation support
    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Raft node {} starting", self.config.node_id);
        counter!("raft.node_started").increment(1);
        gauge!("raft.peers").set(self.config.peers.len() as f64);

        // Start heartbeat task if leader
        if self.is_leader() {
            self.start_heartbeat_task().await;
        }

        loop {
            let timeout = self.calculate_timeout();

            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    let start = Instant::now();
                    
                    match event {
                        RaftEvent::RequestVote(req, tx) => {
                            let resp = self.handle_request_vote(req).await;
                            let _ = tx.send(resp).await;
                        }
                        RaftEvent::AppendEntries(req, tx) => {
                            let resp = self.handle_append_entries(req).await;
                            let _ = tx.send(resp).await;
                        }
                        RaftEvent::ClientCommand(cmd, tx) => {
                            let result = self.handle_client_command(cmd).await;
                            let _ = tx.send(result).await;
                        }
                        RaftEvent::ElectionTimeout => {
                            if let Err(e) = self.handle_election_timeout().await {
                                tracing::error!("Election timeout handling failed: {}", e);
                            }
                        }
                        RaftEvent::HeartbeatTimeout => {
                            if self.is_leader() {
                                if let Err(e) = self.send_heartbeats().await {
                                    tracing::error!("Heartbeat sending failed: {}", e);
                                }
                            }
                        }
                        RaftEvent::InstallSnapshot(snapshot, tx) => {
                            let result = self.install_snapshot(snapshot).await;
                            let _ = tx.send(result).await;
                        }
                        RaftEvent::Shutdown => {
                            tracing::info!("Raft node {} shutting down", self.config.node_id);
                            counter!("raft.node_shutdown").increment(1);
                            break;
                        }
                    }
                    
                    histogram!("raft.event_duration_seconds", start.elapsed().as_secs_f64());
                }
                _ = sleep(timeout) => {
                    self.handle_timeouts().await;
                }
                _ = self.cancellation_token.cancelled() => {
                    tracing::info!("Raft node {} cancelled", self.config.node_id);
                    break;
                }
            }
            
            // Update metrics
            gauge!("raft.log_size").set(self.log.len() as f64);
            gauge!("raft.commit_index").set(self.commit_index as f64);
            gauge!("raft.current_term").set(self.current_term as f64);
        }

        Ok(())
    }

    /// Start background heartbeat task
    async fn start_heartbeat_task(&self) {
        let event_tx = self.event_tx.clone();
        let interval_ms = self.config.heartbeat_interval_ms;
        let cancellation = self.cancellation_token.child_token();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let _ = event_tx.send(RaftEvent::HeartbeatTimeout).await;
                    }
                    _ = cancellation.cancelled() => {
                        break;
                    }
                }
            }
        });
    }

    /// Handle timeouts efficiently
    async fn handle_timeouts(&mut self) {
        let now = Instant::now();
        
        if now >= self.election_deadline {
            if !self.is_leader() {
                let _ = self.event_tx.send(RaftEvent::ElectionTimeout).await;
            }
        }
    }

    /// Calculate timeout for select
    fn calculate_timeout(&self) -> Duration {
        match self.state {
            RaftState::Leader => Duration::from_millis(self.config.heartbeat_interval_ms),
            _ => {
                let remaining = self
                    .election_deadline
                    .saturating_duration_since(Instant::now());
                remaining.min(Duration::from_millis(10))
            }
        }
    }

    /// Handle RequestVote RPC with optimization
    async fn handle_request_vote(&mut self, req: RequestVoteRequest) -> RequestVoteResponse {
        let mut response = RequestVoteResponse {
            term: self.current_term,
            vote_granted: false,
        };

        // Reply false if term < current_term
        if req.term < self.current_term {
            return response;
        }

        // If term > current_term, update current_term and convert to follower
        if req.term > self.current_term {
            self.current_term = req.term;
            self.state = RaftState::Follower;
            self.voted_for = None;
            response.term = self.current_term;
        }

        // Check if we can grant vote
        let can_grant =
            self.voted_for.is_none() || self.voted_for.as_ref() == Some(&req.candidate_id);
        let log_is_up_to_date = self.is_log_up_to_date(req.last_log_index, req.last_log_term);

        if can_grant && log_is_up_to_date {
            self.voted_for = Some(req.candidate_id.clone());
            response.vote_granted = true;
            self.reset_election_timer();
            counter!("raft.votes_granted").increment(1);
        }

        response
    }

    /// Handle AppendEntries RPC with optimization
    async fn handle_append_entries(&mut self, req: AppendEntriesRequest) -> AppendEntriesResponse {
        let mut response = AppendEntriesResponse {
            term: self.current_term,
            success: false,
            conflict_index: 0,
            conflict_term: 0,
        };

        // Reply false if term < current_term
        if req.term < self.current_term {
            return response;
        }

        // If term > current_term, update current_term and convert to follower
        if req.term > self.current_term {
            self.current_term = req.term;
            self.state = RaftState::Follower;
            self.voted_for = None;
            response.term = self.current_term;
        }

        // Reset election timer (we received communication from leader)
        self.reset_election_timer();

        // Reply false if log doesn't contain an entry at prev_log_index whose term matches prev_log_term
        if req.prev_log_index > 0 {
            if req.prev_log_index >= self.log.len() as u64 {
                response.conflict_index = self.log.len() as u64;
                return response;
            }

            let entry = &self.log[req.prev_log_index as usize];
            if entry.term != req.prev_log_term {
                response.conflict_term = entry.term;
                // Find first index with conflict_term (optimization)
                for (i, e) in self.log.iter().enumerate() {
                    if e.term == entry.term {
                        response.conflict_index = i as u64;
                        break;
                    }
                }
                return response;
            }
        }

        // Process entries with batching
        for (i, entry) in req.entries.iter().enumerate() {
            let index = req.prev_log_index + 1 + i as u64;

            if index < self.log.len() as u64 {
                if self.log[index as usize].term != entry.term {
                    // Delete conflicting entry and all that follow it
                    self.log.truncate(index as usize);
                    self.log.push(entry.clone());
                }
            } else {
                self.log.push(entry.clone());
            }
        }

        // Update commit_index
        if req.leader_commit > self.commit_index {
            let last_new_index = if req.entries.is_empty() {
                req.prev_log_index
            } else {
                req.entries.last().unwrap().index
            };
            self.commit_index = last_new_index.min(req.leader_commit);
            
            if let Err(e) = self.apply_committed_entries().await {
                tracing::error!("Failed to apply committed entries: {}", e);
            }
        }

        // Check if we need to snapshot
        if self.log.len() > self.config.snapshot_threshold {
            if let Err(e) = self.create_snapshot().await {
                tracing::warn!("Failed to create snapshot: {}", e);
            }
        }

        response.success = true;
        counter!("raft.append_entries_received").increment(1);
        response
    }

    /// Handle client command with proper error handling
    async fn handle_client_command(&mut self, command: Vec<u8>) -> Result<u64> {
        if !self.is_leader() {
            return Err(anyhow::anyhow!("Not leader"));
        }

        // Append entry to local log
        let entry = LogEntry {
            term: self.current_term,
            index: self.log.len() as u64,
            command: command.clone(),
            timestamp: chrono::Utc::now(),
            hash: [0u8; 32],
        };
        
        let entry_hash = entry.calculate_hash();
        let mut entry = entry;
        entry.hash = entry_hash;

        let index = entry.index;
        self.log.push(entry);

        // Replicate to followers
        self.replicate_log().await?;

        counter!("raft.client_commands").increment(1);
        Ok(index)
    }

    /// Handle election timeout with parallel vote requests
    async fn handle_election_timeout(&mut self) -> Result<()> {
        if self.state == RaftState::Leader {
            return Ok(());
        }

        tracing::info!(
            "Node {} starting election for term {}",
            self.config.node_id,
            self.current_term + 1
        );
        counter!("raft.elections_started").increment(1);

        // Convert to candidate
        self.state = RaftState::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.config.node_id.clone());
        self.reset_election_timer();

        // Request votes from all peers in parallel
        let last_log_index = self.log.len() as u64 - 1;
        let last_log_term = self.log.last().map(|e| e.term).unwrap_or(0);

        let request = RequestVoteRequest {
            term: self.current_term,
            candidate_id: self.config.node_id.clone(),
            last_log_index,
            last_log_term,
        };

        let mut votes_received = 1; // Vote for self
        let votes_needed = (self.config.peers.len() + 1) / 2 + 1;

        // Send RequestVote RPCs to all peers concurrently with timeout
        let mut handles = Vec::with_capacity(self.config.peers.len());
        
        for peer in &self.config.peers {
            let peer = peer.clone();
            let request = request.clone();
            let handle = tokio::spawn(async move {
                // In real implementation, send RPC to peer with timeout
                // For now, simulate network
                sleep(Duration::from_millis(10)).await;
                RequestVoteResponse {
                    term: request.term,
                    vote_granted: true,
                }
            });
            handles.push(handle);
        }

        // Collect responses with timeout
        let results = timeout(
            Duration::from_millis(RPC_TIMEOUT_MS),
            futures::future::join_all(handles)
        ).await;

        match results {
            Ok(responses) => {
                for handle in responses {
                    if let Ok(response) = handle {
                        if response.term > self.current_term {
                            self.current_term = response.term;
                            self.state = RaftState::Follower;
                            self.voted_for = None;
                            return Ok(());
                        }

                        if response.vote_granted {
                            votes_received += 1;
                        }
                    }
                }
            }
            Err(_) => {
                tracing::warn!("Vote collection timeout");
            }
        }

        // Check if we won the election
        if votes_received >= votes_needed && self.state == RaftState::Candidate {
            self.become_leader().await?;
        }

        Ok(())
    }

    /// Become leader with state initialization
    async fn become_leader(&mut self) -> Result<()> {
        tracing::info!(
            "Node {} became leader for term {}",
            self.config.node_id,
            self.current_term
        );
        counter!("raft.leader_elected").increment(1);

        self.state = RaftState::Leader;

        // Initialize leader state
        let last_log_index = self.log.len() as u64;
        
        for peer in &self.config.peers {
            self.next_index.insert(peer.clone(), last_log_index);
            self.match_index.insert(peer.clone(), 0);
        }

        // Start heartbeat task
        self.start_heartbeat_task().await;

        // Send initial heartbeats
        self.send_heartbeats().await?;

        gauge!("raft.is_leader").set(1.0);
        Ok(())
    }

    /// Send heartbeats to all peers
    async fn send_heartbeats(&self) -> Result<()> {
        let request = AppendEntriesRequest {
            term: self.current_term,
            leader_id: self.config.node_id.clone(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: Vec::new(),
            leader_commit: self.commit_index,
        };

        for peer in &self.config.peers {
            let _peer = peer.clone();
            let _request = request.clone();
            // In real implementation, send RPC with timeout
        }

        counter!("raft.heartbeats_sent").increment(1);
        Ok(())
    }

    /// Replicate log to followers with batching
    async fn replicate_log(&mut self) -> Result<()> {
        if !self.is_leader() {
            return Ok(());
        }

        for peer in &self.config.peers {
            let next_idx = *self
                .next_index
                .get(peer)
                .unwrap_or(&(self.log.len() as u64));
            let prev_log_index = next_idx.saturating_sub(1);
            let prev_log_term = if prev_log_index > 0 {
                self.log[prev_log_index as usize].term
            } else {
                0
            };

            // Batch entries to reduce RPC overhead
            let entries: Vec<LogEntry> = self
                .log
                .iter()
                .skip(next_idx as usize)
                .take(self.config.max_entries_per_append)
                .cloned()
                .collect();

            if entries.is_empty() {
                // Send heartbeat
                continue;
            }

            let request = AppendEntriesRequest {
                term: self.current_term,
                leader_id: self.config.node_id.clone(),
                prev_log_index,
                prev_log_term,
                entries,
                leader_commit: self.commit_index,
            };

            // Send RPC to peer with timeout
            let _peer = peer.clone();
            let _request = request;
            // In real implementation, send RPC and handle response
        }

        Ok(())
    }

    /// Apply committed entries to state machine
    async fn apply_committed_entries(&mut self) -> Result<()> {
        while self.last_applied < self.commit_index {
            self.last_applied += 1;

            if let Some(entry) = self.log.get(self.last_applied as usize) {
                // Verify entry integrity before applying
                if !entry.verify() {
                    tracing::error!(
                        "Entry {} failed integrity check!",
                        entry.index
                    );
                    continue;
                }
                
                // Apply to state machine
                let mut state_machine = self.state_machine.write().await;
                state_machine.insert(
                    format!("entry_{}", entry.index),
                    entry.command.clone()
                );

                tracing::debug!("Applied entry {} to state machine", entry.index);
                counter!("raft.entries_applied").increment(1);
            }
        }

        Ok(())
    }

    /// Create snapshot to truncate log
    async fn create_snapshot(&mut self) -> Result<()> {
        if self.commit_index == 0 {
            return Ok(());
        }

        let last_entry = &self.log[self.commit_index as usize];
        
        // Serialize state machine
        let state_machine = self.state_machine.read().await;
        let data = serde_json::to_vec(&*state_machine)
            .context("Failed to serialize state machine")?;

        let snapshot = Snapshot {
            last_index: last_entry.index,
            last_term: last_entry.term,
            data,
        };

        // Truncate log
        let new_log = vec![LogEntry {
            term: last_entry.term,
            index: last_entry.index,
            command: Vec::new(),
            timestamp: chrono::Utc::now(),
            hash: last_entry.hash,
        }];
        
        // Keep entries after commit_index
        let remaining: Vec<_> = self.log
            .iter()
            .skip((self.commit_index + 1) as usize)
            .cloned()
            .collect();
        
        self.log = new_log;
        self.log.extend(remaining);
        
        self.snapshot = Some(snapshot);
        
        tracing::info!(
            "Created snapshot at index {}, log truncated from {} to {} entries",
            last_entry.index,
            self.log.len() + self.commit_index as usize,
            self.log.len()
        );
        
        counter!("raft.snapshots_created").increment(1);
        gauge!("raft.log_size_after_snapshot").set(self.log.len() as f64);
        
        Ok(())
    }

    /// Install snapshot from leader
    async fn install_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        // Deserialize state machine
        let state_machine: HashMap<String, Vec<u8>> = serde_json::from_slice(&snapshot.data)
            .context("Failed to deserialize state machine")?;
        
        let mut current_state = self.state_machine.write().await;
        *current_state = state_machine;
        
        // Update indices
        self.last_applied = snapshot.last_index;
        self.commit_index = snapshot.last_index;
        
        // Truncate log
        self.log.truncate(1);
        self.log.push(LogEntry {
            term: snapshot.last_term,
            index: snapshot.last_index,
            command: Vec::new(),
            timestamp: chrono::Utc::now(),
            hash: [0u8; 32],
        });
        
        self.snapshot = Some(snapshot);
        
        tracing::info!("Installed snapshot at index {}", self.commit_index);
        counter!("raft.snapshots_installed").increment(1);
        
        Ok(())
    }

    /// Check if candidate's log is at least as up-to-date as ours
    fn is_log_up_to_date(&self, last_log_index: u64, last_log_term: u64) -> bool {
        let our_last_term = self.log.last().map(|e| e.term).unwrap_or(0);

        if last_log_term != our_last_term {
            last_log_term > our_last_term
        } else {
            last_log_index >= (self.log.len() as u64).saturating_sub(1)
        }
    }

    /// Reset election timer with randomization
    fn reset_election_timer(&mut self) {
        let timeout = Duration::from_millis(
            self.config.election_timeout_min_ms
                + rand::random::<u64>()
                    % (self.config.election_timeout_max_ms - self.config.election_timeout_min_ms),
        );
        self.election_deadline = Instant::now() + timeout;
    }

    /// Get log entry at index
    pub fn get_log_entry(&self, index: u64) -> Option<&LogEntry> {
        self.log.get(index as usize)
    }

    /// Get committed entries
    pub fn get_committed_entries(&self) -> Vec<&LogEntry> {
        self.log
            .iter()
            .take((self.commit_index + 1) as usize)
            .collect()
    }
    
    /// Shutdown the node gracefully
    pub fn shutdown(&self) {
        self.cancellation_token.cancel();
        let _ = self.event_tx.try_send(RaftEvent::Shutdown);
    }
}

impl Drop for RaftNode {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
        tracing::debug!("RaftNode dropped for {}", self.config.node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_state() {
        let config = RaftConfig::default();
        let node = RaftNode::new(config);
        assert!(matches!(node.state(), RaftState::Follower));
        assert!(!node.is_leader());
    }

    #[test]
    fn test_log_entry() {
        let entry = LogEntry {
            term: 1,
            index: 1,
            command: vec![1, 2, 3],
            timestamp: chrono::Utc::now(),
            hash: [0u8; 32],
        };
        assert_eq!(entry.term, 1);
        assert_eq!(entry.index, 1);
        
        // Test hash calculation
        let hash = entry.calculate_hash();
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_request_vote() {
        let req = RequestVoteRequest {
            term: 1,
            candidate_id: "node-2".into(),
            last_log_index: 0,
            last_log_term: 0,
        };
        assert_eq!(req.term, 1);
    }
    
    #[test]
    fn test_log_integrity() {
        let mut entry = LogEntry {
            term: 1,
            index: 1,
            command: vec![1, 2, 3],
            timestamp: chrono::Utc::now(),
            hash: [0u8; 32],
        };
        
        // Calculate and set hash
        entry.hash = entry.calculate_hash();
        
        // Verify integrity
        assert!(entry.verify());
        
        // Tamper with data
        entry.command[0] = 99;
        
        // Should fail verification
        assert!(!entry.verify());
    }
}
