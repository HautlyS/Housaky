use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
    Interrupt = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuromorphicEvent {
    pub id: String,
    pub event_type: String,
    pub source: String,
    pub payload: serde_json::Value,
    pub priority: EventPriority,
    pub timestamp: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub processed: bool,
    pub latency_budget_us: Option<u64>,
}

impl NeuromorphicEvent {
    pub fn new(event_type: &str, source: &str, payload: serde_json::Value, priority: EventPriority) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.to_string(),
            source: source.to_string(),
            payload,
            priority,
            timestamp: Utc::now(),
            deadline: None,
            processed: false,
            latency_budget_us: None,
        }
    }

    pub fn interrupt(event_type: &str, source: &str, payload: serde_json::Value) -> Self {
        let mut e = Self::new(event_type, source, payload, EventPriority::Interrupt);
        e.latency_budget_us = Some(100);
        e
    }

    pub fn is_expired(&self) -> bool {
        self.deadline.map(|d| Utc::now() > d).unwrap_or(false)
    }

    pub fn age_us(&self) -> i64 {
        (Utc::now() - self.timestamp).num_microseconds().unwrap_or(0)
    }
}

impl PartialEq for NeuromorphicEvent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for NeuromorphicEvent {}

impl PartialOrd for NeuromorphicEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NeuromorphicEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.timestamp.cmp(&self.timestamp))
    }
}

pub type EventHandler = Arc<dyn Fn(NeuromorphicEvent) -> tokio::task::JoinHandle<()> + Send + Sync>;

#[derive(Clone)]
pub struct Subscription {
    pub id: String,
    pub event_type: String,
    pub subscriber: String,
    pub handler: EventHandler,
    pub min_priority: EventPriority,
}

pub struct EventBus {
    pub queues: Arc<RwLock<HashMap<String, VecDeque<NeuromorphicEvent>>>>,
    pub priority_queue: Arc<Mutex<BinaryHeap<NeuromorphicEvent>>>,
    pub subscriptions: Arc<RwLock<Vec<Subscription>>>,
    pub dead_letter: Arc<RwLock<Vec<NeuromorphicEvent>>>,
    pub metrics: Arc<RwLock<EventBusMetrics>>,
    pub max_queue_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventBusMetrics {
    pub total_published: u64,
    pub total_processed: u64,
    pub total_dropped: u64,
    pub total_expired: u64,
    pub avg_latency_us: f64,
    pub events_by_type: HashMap<String, u64>,
    pub events_by_priority: HashMap<String, u64>,
}

impl EventBus {
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            queues: Arc::new(RwLock::new(HashMap::new())),
            priority_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            dead_letter: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(EventBusMetrics::default())),
            max_queue_size,
        }
    }

    pub async fn subscribe(
        &self,
        event_type: &str,
        subscriber: &str,
        handler: EventHandler,
        min_priority: EventPriority,
    ) -> String {
        let sub = Subscription {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.to_string(),
            subscriber: subscriber.to_string(),
            handler,
            min_priority,
        };
        let id = sub.id.clone();
        self.subscriptions.write().await.push(sub);
        debug!("EventBus: subscribed '{}' to '{}'", subscriber, event_type);
        id
    }

    pub async fn unsubscribe(&self, subscription_id: &str) {
        self.subscriptions.write().await.retain(|s| s.id != subscription_id);
    }

    pub async fn publish(&self, event: NeuromorphicEvent) {
        if event.is_expired() {
            let mut metrics = self.metrics.write().await;
            metrics.total_expired += 1;
            return;
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.total_published += 1;
            *metrics.events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;
            *metrics.events_by_priority.entry(format!("{:?}", event.priority)).or_insert(0) += 1;
        }

        let mut pq = self.priority_queue.lock().await;
        if pq.len() >= self.max_queue_size {
            if let Some(lowest) = pq.iter().min_by_key(|e| &e.priority) {
                if lowest.priority < event.priority {
                    let lowest_id = lowest.id.clone();
                    let items: Vec<NeuromorphicEvent> = pq.drain().filter(|e| e.id != lowest_id).collect();
                    for item in items { pq.push(item); }
                    pq.push(event);
                    self.metrics.write().await.total_dropped += 1;
                } else {
                    self.metrics.write().await.total_dropped += 1;
                    warn!("EventBus: dropped low-priority event (queue full)");
                }
            }
        } else {
            pq.push(event);
        }
    }

    pub async fn publish_to(&self, lane: &str, event: NeuromorphicEvent) {
        let mut queues = self.queues.write().await;
        let queue = queues.entry(lane.to_string()).or_default();
        if queue.len() >= self.max_queue_size {
            queue.pop_front();
            self.metrics.write().await.total_dropped += 1;
        }
        queue.push_back(event);
    }

    pub async fn poll(&self) -> Option<NeuromorphicEvent> {
        self.priority_queue.lock().await.pop()
    }

    pub async fn poll_lane(&self, lane: &str) -> Option<NeuromorphicEvent> {
        self.queues.write().await.get_mut(lane)?.pop_front()
    }

    pub async fn drain_lane(&self, lane: &str) -> Vec<NeuromorphicEvent> {
        self.queues
            .write()
            .await
            .get_mut(lane)
            .map(|q| q.drain(..).collect())
            .unwrap_or_default()
    }

    pub async fn dispatch_all(&self) {
        let mut processed = 0u64;
        loop {
            let event = {
                let mut pq = self.priority_queue.lock().await;
                pq.pop()
            };
            let Some(mut event) = event else { break };

            if event.is_expired() {
                self.dead_letter.write().await.push(event);
                self.metrics.write().await.total_expired += 1;
                continue;
            }

            let subs: Vec<Subscription> = self
                .subscriptions
                .read()
                .await
                .iter()
                .filter(|s| {
                    (s.event_type == event.event_type || s.event_type == "*")
                        && event.priority >= s.min_priority
                })
                .cloned()
                .collect();

            let start_us = (Utc::now() - event.timestamp).num_microseconds().unwrap_or(0);

            for sub in subs {
                let e = event.clone();
                let _handle = (sub.handler)(e);
            }

            event.processed = true;
            processed += 1;

            let latency_us = start_us.max(0) as f64;
            let mut metrics = self.metrics.write().await;
            metrics.total_processed += 1;
            let n = metrics.total_processed as f64;
            metrics.avg_latency_us =
                (metrics.avg_latency_us * (n - 1.0) + latency_us) / n;
        }

        if processed > 0 {
            debug!("EventBus: dispatched {} events", processed);
        }
    }

    pub async fn pending_count(&self) -> usize {
        self.priority_queue.lock().await.len()
    }

    pub async fn metrics(&self) -> EventBusMetrics {
        self.metrics.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_and_poll() {
        let bus = EventBus::new(1000);
        let event = NeuromorphicEvent::new(
            "sensor_spike",
            "temperature_sensor",
            serde_json::json!({"value": 42.0}),
            EventPriority::High,
        );
        bus.publish(event).await;
        let polled = bus.poll().await;
        assert!(polled.is_some());
        assert_eq!(polled.unwrap().event_type, "sensor_spike");
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let bus = EventBus::new(1000);

        bus.publish(NeuromorphicEvent::new("low", "src", serde_json::json!({}), EventPriority::Low)).await;
        bus.publish(NeuromorphicEvent::new("critical", "src", serde_json::json!({}), EventPriority::Critical)).await;
        bus.publish(NeuromorphicEvent::new("normal", "src", serde_json::json!({}), EventPriority::Normal)).await;

        let first = bus.poll().await.unwrap();
        assert_eq!(first.event_type, "critical");
    }

    #[tokio::test]
    async fn test_lane_publish() {
        let bus = EventBus::new(100);
        let e = NeuromorphicEvent::new("gpio", "pin_3", serde_json::json!({"state": 1}), EventPriority::Interrupt);
        bus.publish_to("hardware", e).await;
        let polled = bus.poll_lane("hardware").await;
        assert!(polled.is_some());
    }
}
