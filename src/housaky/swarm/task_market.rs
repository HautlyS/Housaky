use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Open,
    Bidding,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub priority: f64,
    pub deadline: Option<DateTime<Utc>>,
    pub status: TaskStatus,
    pub posted_by: String,
    pub assigned_to: Option<String>,
    pub bids: Vec<Bid>,
    pub posted_at: DateTime<Utc>,
    pub estimated_cost: f64,
    pub actual_cost: Option<f64>,
    pub result: Option<String>,
}

impl MarketTask {
    pub fn new(
        title: &str,
        description: &str,
        required_capabilities: Vec<String>,
        priority: f64,
        posted_by: &str,
        estimated_cost: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            description: description.to_string(),
            required_capabilities,
            priority,
            deadline: None,
            status: TaskStatus::Open,
            posted_by: posted_by.to_string(),
            assigned_to: None,
            bids: Vec::new(),
            posted_at: Utc::now(),
            estimated_cost,
            actual_cost: None,
            result: None,
        }
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn best_bid(&self) -> Option<&Bid> {
        self.bids.iter().max_by(|a, b| {
            a.score().partial_cmp(&b.score()).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn is_expired(&self) -> bool {
        self.deadline
            .map(|d| Utc::now() > d)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    pub id: String,
    pub task_id: String,
    pub bidder: String,
    pub offered_cost: f64,
    pub estimated_duration_ms: u64,
    pub capability_match: f64,
    pub reputation: f64,
    pub confidence: f64,
    pub submitted_at: DateTime<Utc>,
    pub rationale: String,
}

impl Bid {
    pub fn new(
        task_id: &str,
        bidder: &str,
        offered_cost: f64,
        estimated_duration_ms: u64,
        capability_match: f64,
        reputation: f64,
        confidence: f64,
        rationale: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            bidder: bidder.to_string(),
            offered_cost,
            estimated_duration_ms,
            capability_match,
            reputation,
            confidence,
            submitted_at: Utc::now(),
            rationale: rationale.to_string(),
        }
    }

    pub fn score(&self) -> f64 {
        let capability_weight = 0.35;
        let reputation_weight = 0.30;
        let confidence_weight = 0.20;
        let speed_weight = 0.15;

        let speed_score = 1.0 / (1.0 + self.estimated_duration_ms as f64 / 10_000.0);
        self.capability_match * capability_weight
            + self.reputation * reputation_weight
            + self.confidence * confidence_weight
            + speed_score * speed_weight
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionResult {
    pub task_id: String,
    pub winner: Option<String>,
    pub winning_bid: Option<Bid>,
    pub total_bids: usize,
    pub auction_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMarketStats {
    pub open_tasks: usize,
    pub assigned_tasks: usize,
    pub completed_tasks: usize,
    pub total_bids: usize,
    pub avg_bids_per_task: f64,
    pub total_cost: f64,
}

pub struct TaskMarket {
    pub tasks: Arc<RwLock<HashMap<String, MarketTask>>>,
    pub completed_tasks: Arc<RwLock<Vec<MarketTask>>>,
    pub auction_history: Arc<RwLock<Vec<AuctionResult>>>,
}

impl TaskMarket {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(Vec::new())),
            auction_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn post_task(&self, task: MarketTask) -> String {
        let id = task.id.clone();
        info!("Task market: posted task '{}' by '{}'", task.title, task.posted_by);
        self.tasks.write().await.insert(id.clone(), task);
        id
    }

    pub async fn submit_bid(&self, bid: Bid) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(&bid.task_id)
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", bid.task_id))?;

        if task.status != TaskStatus::Open && task.status != TaskStatus::Bidding {
            anyhow::bail!("Task {} is not accepting bids (status: {:?})", bid.task_id, task.status);
        }
        if task.is_expired() {
            task.status = TaskStatus::Expired;
            anyhow::bail!("Task {} has expired", bid.task_id);
        }

        task.status = TaskStatus::Bidding;
        info!(
            "Bid submitted: task={} bidder={} score={:.3}",
            bid.task_id,
            bid.bidder,
            bid.score()
        );
        task.bids.push(bid);
        Ok(())
    }

    pub async fn run_auction(&self, task_id: &str) -> anyhow::Result<AuctionResult> {
        let start = std::time::Instant::now();
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;

        let total_bids = task.bids.len();

        let result = if let Some(best) = task.best_bid() {
            let winner = best.bidder.clone();
            let winning_bid = best.clone();
            task.assigned_to = Some(winner.clone());
            task.status = TaskStatus::Assigned;
            info!("Auction won: task={} winner={} score={:.3}", task_id, winner, winning_bid.score());
            AuctionResult {
                task_id: task_id.to_string(),
                winner: Some(winner),
                winning_bid: Some(winning_bid),
                total_bids,
                auction_duration_ms: start.elapsed().as_millis() as u64,
            }
        } else {
            AuctionResult {
                task_id: task_id.to_string(),
                winner: None,
                winning_bid: None,
                total_bids: 0,
                auction_duration_ms: start.elapsed().as_millis() as u64,
            }
        };

        drop(tasks);
        self.auction_history.write().await.push(result.clone());
        Ok(result)
    }

    pub async fn complete_task(&self, task_id: &str, result: &str, actual_cost: f64) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(mut task) = tasks.remove(task_id) {
            task.status = TaskStatus::Completed;
            task.result = Some(result.to_string());
            task.actual_cost = Some(actual_cost);
            info!("Task completed: id={} cost={:.2}", task_id, actual_cost);
            drop(tasks);
            self.completed_tasks.write().await.push(task);
            Ok(())
        } else {
            anyhow::bail!("Task {} not found", task_id)
        }
    }

    pub async fn fail_task(&self, task_id: &str, reason: &str) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = TaskStatus::Failed;
            task.result = Some(format!("FAILED: {}", reason));
        }
        Ok(())
    }

    pub async fn open_tasks_for_agent(&self, capabilities: &[String]) -> Vec<MarketTask> {
        self.tasks
            .read()
            .await
            .values()
            .filter(|t| {
                (t.status == TaskStatus::Open || t.status == TaskStatus::Bidding)
                    && !t.is_expired()
                    && t.required_capabilities
                        .iter()
                        .any(|req| capabilities.contains(req))
            })
            .cloned()
            .collect()
    }

    pub async fn stats(&self) -> TaskMarketStats {
        let tasks = self.tasks.read().await;
        let open = tasks.values().filter(|t| t.status == TaskStatus::Open).count();
        let assigned = tasks.values().filter(|t| t.status == TaskStatus::Assigned || t.status == TaskStatus::InProgress).count();
        let total_bids: usize = tasks.values().map(|t| t.bids.len()).sum();
        let task_count = tasks.len().max(1);
        drop(tasks);

        let completed = self.completed_tasks.read().await;
        let total_cost: f64 = completed.iter().filter_map(|t| t.actual_cost).sum();
        let completed_count = completed.len();
        drop(completed);

        TaskMarketStats {
            open_tasks: open,
            assigned_tasks: assigned,
            completed_tasks: completed_count,
            total_bids,
            avg_bids_per_task: total_bids as f64 / task_count as f64,
            total_cost,
        }
    }
}

impl Default for TaskMarket {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_posting_and_bidding() {
        let market = TaskMarket::new();
        let task = MarketTask::new("analyze code", "analyze rust code", vec!["rust".into()], 0.8, "orchestrator", 1.0);
        let task_id = market.post_task(task).await;

        let bid = Bid::new(&task_id, "agent-1", 0.9, 5000, 0.95, 0.88, 0.9, "I specialize in Rust");
        market.submit_bid(bid).await.unwrap();

        let result = market.run_auction(&task_id).await.unwrap();
        assert_eq!(result.winner, Some("agent-1".to_string()));
        assert_eq!(result.total_bids, 1);
    }

    #[tokio::test]
    async fn test_best_bid_selection() {
        let market = TaskMarket::new();
        let task = MarketTask::new("task", "desc", vec!["python".into()], 0.5, "orch", 1.0);
        let task_id = market.post_task(task).await;

        market.submit_bid(Bid::new(&task_id, "low-quality", 0.5, 100_000, 0.4, 0.3, 0.5, "")).await.unwrap();
        market.submit_bid(Bid::new(&task_id, "high-quality", 0.8, 1_000, 0.95, 0.95, 0.95, "")).await.unwrap();

        let result = market.run_auction(&task_id).await.unwrap();
        assert_eq!(result.winner, Some("high-quality".to_string()));
    }

    #[tokio::test]
    async fn test_task_completion() {
        let market = TaskMarket::new();
        let task = MarketTask::new("t", "d", vec![], 0.5, "o", 1.0);
        let task_id = market.post_task(task).await;
        market.complete_task(&task_id, "done", 0.7).await.unwrap();
        let stats = market.stats().await;
        assert_eq!(stats.completed_tasks, 1);
    }
}
