//! Temporal Reasoning & Time-Aware Memory
//!
//! Implements Allen's interval algebra (13 temporal relations):
//! Before, After, Meets, MetBy, Overlaps, OverlappedBy,
//! Starts, StartedBy, During, Contains, Finishes, FinishedBy, Equal
//!
//! Provides:
//! - TemporalReasoner: answers temporal queries about events
//! - TemporalIndex: efficient time-range queries over episodic memory
//! - Pattern detection: recurring patterns, trends, periodicity
//! - Deadline-aware planning integration

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ── Allen's Interval Algebra ─────────────────────────────────────────────────

/// The 13 relations from Allen's interval algebra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntervalRelation {
    Before,
    After,
    Meets,
    MetBy,
    Overlaps,
    OverlappedBy,
    Starts,
    StartedBy,
    During,
    Contains,
    Finishes,
    FinishedBy,
    Equal,
}

impl IntervalRelation {
    /// Get the inverse of this relation.
    pub fn inverse(&self) -> Self {
        match self {
            IntervalRelation::Before => IntervalRelation::After,
            IntervalRelation::After => IntervalRelation::Before,
            IntervalRelation::Meets => IntervalRelation::MetBy,
            IntervalRelation::MetBy => IntervalRelation::Meets,
            IntervalRelation::Overlaps => IntervalRelation::OverlappedBy,
            IntervalRelation::OverlappedBy => IntervalRelation::Overlaps,
            IntervalRelation::Starts => IntervalRelation::StartedBy,
            IntervalRelation::StartedBy => IntervalRelation::Starts,
            IntervalRelation::During => IntervalRelation::Contains,
            IntervalRelation::Contains => IntervalRelation::During,
            IntervalRelation::Finishes => IntervalRelation::FinishedBy,
            IntervalRelation::FinishedBy => IntervalRelation::Finishes,
            IntervalRelation::Equal => IntervalRelation::Equal,
        }
    }

    /// Human-readable description.
    pub fn describe(&self) -> &'static str {
        match self {
            IntervalRelation::Before => "occurs before",
            IntervalRelation::After => "occurs after",
            IntervalRelation::Meets => "meets (ends exactly when the other starts)",
            IntervalRelation::MetBy => "is met by",
            IntervalRelation::Overlaps => "overlaps with",
            IntervalRelation::OverlappedBy => "is overlapped by",
            IntervalRelation::Starts => "starts at the same time as",
            IntervalRelation::StartedBy => "is started by",
            IntervalRelation::During => "occurs during",
            IntervalRelation::Contains => "contains",
            IntervalRelation::Finishes => "finishes at the same time as",
            IntervalRelation::FinishedBy => "is finished by",
            IntervalRelation::Equal => "is equal to",
        }
    }
}

/// A time interval with a start and end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeInterval {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub label: String,
}

impl TimeInterval {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>, label: &str) -> Self {
        Self {
            start,
            end,
            label: label.to_string(),
        }
    }

    pub fn duration(&self) -> Duration {
        self.end - self.start
    }

    /// Determine the Allen's interval relation between self and other.
    pub fn relation_to(&self, other: &TimeInterval) -> IntervalRelation {
        let tolerance = Duration::seconds(1);

        let starts_equal = (self.start - other.start).num_seconds().abs() <= tolerance.num_seconds();
        let ends_equal = (self.end - other.end).num_seconds().abs() <= tolerance.num_seconds();
        let self_end_meets_other_start =
            (self.end - other.start).num_seconds().abs() <= tolerance.num_seconds();
        let other_end_meets_self_start =
            (other.end - self.start).num_seconds().abs() <= tolerance.num_seconds();

        if starts_equal && ends_equal {
            IntervalRelation::Equal
        } else if self.end < other.start {
            IntervalRelation::Before
        } else if other.end < self.start {
            IntervalRelation::After
        } else if self_end_meets_other_start {
            IntervalRelation::Meets
        } else if other_end_meets_self_start {
            IntervalRelation::MetBy
        } else if starts_equal && self.end < other.end {
            IntervalRelation::Starts
        } else if starts_equal && self.end > other.end {
            IntervalRelation::StartedBy
        } else if ends_equal && self.start > other.start {
            IntervalRelation::Finishes
        } else if ends_equal && self.start < other.start {
            IntervalRelation::FinishedBy
        } else if self.start > other.start && self.end < other.end {
            IntervalRelation::During
        } else if self.start < other.start && self.end > other.end {
            IntervalRelation::Contains
        } else if self.start < other.start && self.end > other.start && self.end < other.end {
            IntervalRelation::Overlaps
        } else {
            IntervalRelation::OverlappedBy
        }
    }
}

// ── Temporal Index ───────────────────────────────────────────────────────────

/// An indexed event with temporal metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEvent {
    pub id: String,
    pub interval: TimeInterval,
    pub event_type: String,
    pub description: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub importance: f64,
}

/// Time-range index over events.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemporalIndex {
    events: Vec<TemporalEvent>,
    by_type: HashMap<String, Vec<usize>>, // event_type → indices
    by_tag: HashMap<String, Vec<usize>>,  // tag → indices
}

impl TemporalIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event to the index.
    pub fn add_event(&mut self, event: TemporalEvent) {
        let idx = self.events.len();

        // Index by type
        self.by_type
            .entry(event.event_type.clone())
            .or_default()
            .push(idx);

        // Index by tags
        for tag in &event.tags {
            self.by_tag.entry(tag.clone()).or_default().push(idx);
        }

        self.events.push(event);
    }

    /// Query events within a time range.
    pub fn query_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&TemporalEvent> {
        self.events
            .iter()
            .filter(|e| e.interval.start >= start && e.interval.end <= end)
            .collect()
    }

    /// Query events of a specific type.
    pub fn query_by_type(&self, event_type: &str) -> Vec<&TemporalEvent> {
        self.by_type
            .get(event_type)
            .map(|indices| indices.iter().filter_map(|&i| self.events.get(i)).collect())
            .unwrap_or_default()
    }

    /// Query events with a specific tag.
    pub fn query_by_tag(&self, tag: &str) -> Vec<&TemporalEvent> {
        self.by_tag
            .get(tag)
            .map(|indices| indices.iter().filter_map(|&i| self.events.get(i)).collect())
            .unwrap_or_default()
    }

    /// Get events that overlap with the given interval.
    pub fn query_overlapping(&self, interval: &TimeInterval) -> Vec<&TemporalEvent> {
        self.events
            .iter()
            .filter(|e| {
                let rel = e.interval.relation_to(interval);
                matches!(
                    rel,
                    IntervalRelation::Overlaps
                        | IntervalRelation::OverlappedBy
                        | IntervalRelation::During
                        | IntervalRelation::Contains
                        | IntervalRelation::Starts
                        | IntervalRelation::StartedBy
                        | IntervalRelation::Finishes
                        | IntervalRelation::FinishedBy
                        | IntervalRelation::Equal
                )
            })
            .collect()
    }

    /// Get the most recent N events.
    pub fn get_recent(&self, count: usize) -> Vec<&TemporalEvent> {
        let mut events: Vec<&TemporalEvent> = self.events.iter().collect();
        events.sort_by(|a, b| b.interval.start.cmp(&a.interval.start));
        events.into_iter().take(count).collect()
    }

    /// Total number of indexed events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

// ── Temporal Reasoner ────────────────────────────────────────────────────────

/// Detected temporal pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPattern {
    pub pattern_type: PatternType,
    pub description: String,
    pub events_involved: Vec<String>, // event IDs
    pub confidence: f64,
    pub period: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    Recurring { period_seconds: i64 },
    Trend { direction: TrendDirection },
    Bursty { avg_gap_seconds: f64 },
    Periodic { period_seconds: i64 },
    Sequential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Answer to a temporal query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAnswer {
    pub query: String,
    pub answer: String,
    pub supporting_events: Vec<String>,
    pub confidence: f64,
}

/// The main temporal reasoning engine.
pub struct TemporalReasoner {
    pub interval_relations: Arc<RwLock<HashMap<(String, String), IntervalRelation>>>,
    pub temporal_index: Arc<RwLock<TemporalIndex>>,
}

impl TemporalReasoner {
    pub fn new() -> Self {
        Self {
            interval_relations: Arc::new(RwLock::new(HashMap::new())),
            temporal_index: Arc::new(RwLock::new(TemporalIndex::new())),
        }
    }

    /// Record a temporal event.
    pub async fn record_event(&self, event: TemporalEvent) {
        // Compute relations with existing events
        let index = self.temporal_index.read().await;
        let mut new_relations = Vec::new();

        for existing in &index.events {
            let relation = event.interval.relation_to(&existing.interval);
            new_relations.push((
                (event.id.clone(), existing.id.clone()),
                relation,
            ));
            new_relations.push((
                (existing.id.clone(), event.id.clone()),
                relation.inverse(),
            ));
        }
        drop(index);

        // Store relations
        let mut relations = self.interval_relations.write().await;
        for (key, rel) in new_relations {
            relations.insert(key, rel);
        }
        drop(relations);

        // Add to index
        self.temporal_index.write().await.add_event(event);
    }

    /// Query: "What happened between time A and time B?"
    pub async fn what_happened_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> TemporalAnswer {
        let index = self.temporal_index.read().await;
        let events = index.query_range(start, end);

        let event_descriptions: Vec<String> = events
            .iter()
            .map(|e| format!("{}: {}", e.interval.label, e.description))
            .collect();

        let event_ids: Vec<String> = events.iter().map(|e| e.id.clone()).collect();

        TemporalAnswer {
            query: format!(
                "What happened between {} and {}?",
                start.format("%Y-%m-%d %H:%M"),
                end.format("%Y-%m-%d %H:%M")
            ),
            answer: if event_descriptions.is_empty() {
                "No events recorded in that time range.".to_string()
            } else {
                format!(
                    "{} events occurred: {}",
                    event_descriptions.len(),
                    event_descriptions.join("; ")
                )
            },
            supporting_events: event_ids,
            confidence: if events.is_empty() { 0.5 } else { 0.9 },
        }
    }

    /// Query: "How long since the last event of type X?"
    pub async fn time_since_last(&self, event_type: &str) -> TemporalAnswer {
        let index = self.temporal_index.read().await;
        let events = index.query_by_type(event_type);

        if events.is_empty() {
            return TemporalAnswer {
                query: format!("How long since last '{}'?", event_type),
                answer: format!("No '{}' events have been recorded.", event_type),
                supporting_events: Vec::new(),
                confidence: 0.8,
            };
        }

        let most_recent = events
            .iter()
            .max_by_key(|e| e.interval.end)
            .unwrap();

        let elapsed = Utc::now() - most_recent.interval.end;

        TemporalAnswer {
            query: format!("How long since last '{}'?", event_type),
            answer: format!(
                "Last '{}' was {} ago (at {})",
                event_type,
                Self::format_duration(elapsed),
                most_recent.interval.end.format("%Y-%m-%d %H:%M")
            ),
            supporting_events: vec![most_recent.id.clone()],
            confidence: 0.95,
        }
    }

    /// Query: "What patterns repeat with period P?"
    pub async fn find_patterns(&self, event_type: &str) -> Vec<TemporalPattern> {
        let index = self.temporal_index.read().await;
        let events = index.query_by_type(event_type);

        if events.len() < 3 {
            return Vec::new();
        }

        let mut patterns = Vec::new();

        // Calculate inter-event gaps
        let mut sorted_events: Vec<&&TemporalEvent> = events.iter().collect();
        sorted_events.sort_by_key(|e| e.interval.start);

        let gaps: Vec<i64> = sorted_events
            .windows(2)
            .map(|w| (w[1].interval.start - w[0].interval.end).num_seconds())
            .collect();

        if gaps.is_empty() {
            return patterns;
        }

        // Check for periodicity
        let avg_gap = gaps.iter().sum::<i64>() as f64 / gaps.len() as f64;
        let variance: f64 = gaps
            .iter()
            .map(|&g| (g as f64 - avg_gap).powi(2))
            .sum::<f64>()
            / gaps.len() as f64;
        let std_dev = variance.sqrt();
        let cv = if avg_gap.abs() > 1.0 {
            std_dev / avg_gap.abs()
        } else {
            f64::MAX
        };

        if cv < 0.3 {
            // Low coefficient of variation → periodic
            patterns.push(TemporalPattern {
                pattern_type: PatternType::Periodic {
                    period_seconds: avg_gap as i64,
                },
                description: format!(
                    "'{}' events recur approximately every {}",
                    event_type,
                    Self::format_duration(Duration::seconds(avg_gap as i64))
                ),
                events_involved: sorted_events.iter().map(|e| e.id.clone()).collect(),
                confidence: 1.0 - cv,
                period: Some(Duration::seconds(avg_gap as i64)),
            });
        }

        // Check for bursts
        let min_gap = *gaps.iter().min().unwrap_or(&0);
        let max_gap = *gaps.iter().max().unwrap_or(&0);
        if max_gap > 0 && (max_gap as f64 / (min_gap.max(1)) as f64) > 5.0 {
            patterns.push(TemporalPattern {
                pattern_type: PatternType::Bursty {
                    avg_gap_seconds: avg_gap,
                },
                description: format!(
                    "'{}' events show bursty behavior (gaps range from {} to {})",
                    event_type,
                    Self::format_duration(Duration::seconds(min_gap)),
                    Self::format_duration(Duration::seconds(max_gap)),
                ),
                events_involved: sorted_events.iter().map(|e| e.id.clone()).collect(),
                confidence: 0.7,
                period: None,
            });
        }

        // Check for trend (increasing or decreasing frequency)
        if gaps.len() >= 4 {
            let first_half_avg =
                gaps[..gaps.len() / 2].iter().sum::<i64>() as f64 / (gaps.len() / 2) as f64;
            let second_half_avg =
                gaps[gaps.len() / 2..].iter().sum::<i64>() as f64
                    / (gaps.len() - gaps.len() / 2) as f64;

            let ratio = second_half_avg / first_half_avg.max(1.0);
            if ratio < 0.7 {
                patterns.push(TemporalPattern {
                    pattern_type: PatternType::Trend {
                        direction: TrendDirection::Increasing,
                    },
                    description: format!(
                        "'{}' events are becoming more frequent (gap shrinking by {:.0}%)",
                        event_type,
                        (1.0 - ratio) * 100.0
                    ),
                    events_involved: sorted_events.iter().map(|e| e.id.clone()).collect(),
                    confidence: 0.6,
                    period: None,
                });
            } else if ratio > 1.3 {
                patterns.push(TemporalPattern {
                    pattern_type: PatternType::Trend {
                        direction: TrendDirection::Decreasing,
                    },
                    description: format!(
                        "'{}' events are becoming less frequent (gap growing by {:.0}%)",
                        event_type,
                        (ratio - 1.0) * 100.0
                    ),
                    events_involved: sorted_events.iter().map(|e| e.id.clone()).collect(),
                    confidence: 0.6,
                    period: None,
                });
            }
        }

        patterns
    }

    /// Check if a deadline is at risk based on historical performance.
    pub async fn assess_deadline_risk(
        &self,
        task_type: &str,
        deadline: DateTime<Utc>,
    ) -> DeadlineAssessment {
        let index = self.temporal_index.read().await;
        let similar_events = index.query_by_type(task_type);

        let now = Utc::now();
        let remaining = deadline - now;

        if similar_events.is_empty() {
            return DeadlineAssessment {
                deadline,
                remaining_time: remaining,
                estimated_duration: None,
                risk_level: DeadlineRisk::Unknown,
                confidence: 0.3,
                recommendation: "No historical data for this task type. Cannot assess risk."
                    .to_string(),
            };
        }

        // Calculate average duration from historical events
        let durations: Vec<i64> = similar_events
            .iter()
            .map(|e| e.interval.duration().num_seconds())
            .collect();

        let avg_duration = durations.iter().sum::<i64>() as f64 / durations.len() as f64;
        let max_duration = *durations.iter().max().unwrap_or(&0) as f64;

        let estimated = Duration::seconds(avg_duration as i64);
        let remaining_secs = remaining.num_seconds() as f64;

        let risk_level = if remaining_secs < avg_duration * 0.5 {
            DeadlineRisk::Critical
        } else if remaining_secs < avg_duration {
            DeadlineRisk::High
        } else if remaining_secs < max_duration {
            DeadlineRisk::Medium
        } else {
            DeadlineRisk::Low
        };

        let recommendation = match risk_level {
            DeadlineRisk::Critical => format!(
                "CRITICAL: Remaining time ({}) is less than half the average duration ({}). Consider descoping.",
                Self::format_duration(remaining),
                Self::format_duration(estimated)
            ),
            DeadlineRisk::High => format!(
                "HIGH RISK: Remaining time ({}) is tight relative to average duration ({}). Begin immediately.",
                Self::format_duration(remaining),
                Self::format_duration(estimated)
            ),
            DeadlineRisk::Medium => format!(
                "MODERATE: Remaining time ({}) should suffice based on average ({}), but worst-case ({}) is concerning.",
                Self::format_duration(remaining),
                Self::format_duration(estimated),
                Self::format_duration(Duration::seconds(max_duration as i64))
            ),
            DeadlineRisk::Low => format!(
                "LOW RISK: Remaining time ({}) is comfortable relative to historical average ({}).",
                Self::format_duration(remaining),
                Self::format_duration(estimated)
            ),
            DeadlineRisk::Unknown => "Cannot assess risk without historical data.".to_string(),
        };

        DeadlineAssessment {
            deadline,
            remaining_time: remaining,
            estimated_duration: Some(estimated),
            risk_level,
            confidence: (similar_events.len() as f64 / 10.0).min(0.95),
            recommendation,
        }
    }

    /// Get the temporal relation between two events by ID.
    pub async fn get_relation(
        &self,
        event_a: &str,
        event_b: &str,
    ) -> Option<IntervalRelation> {
        let relations = self.interval_relations.read().await;
        relations
            .get(&(event_a.to_string(), event_b.to_string()))
            .copied()
    }

    /// Get all events that have a specific relation to a given event.
    pub async fn find_related(
        &self,
        event_id: &str,
        relation: IntervalRelation,
    ) -> Vec<String> {
        let relations = self.interval_relations.read().await;
        relations
            .iter()
            .filter(|((from, _), rel)| from == event_id && **rel == relation)
            .map(|((_, to), _)| to.clone())
            .collect()
    }

    /// Get temporal statistics.
    pub async fn get_stats(&self) -> TemporalStats {
        let index = self.temporal_index.read().await;
        let relations = self.interval_relations.read().await;

        let earliest = index
            .events
            .iter()
            .map(|e| e.interval.start)
            .min();
        let latest = index
            .events
            .iter()
            .map(|e| e.interval.end)
            .max();

        let event_types: Vec<String> = index
            .events
            .iter()
            .map(|e| e.event_type.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        TemporalStats {
            total_events: index.len(),
            total_relations: relations.len(),
            earliest_event: earliest,
            latest_event: latest,
            event_types,
            time_span: match (earliest, latest) {
                (Some(e), Some(l)) => Some(l - e),
                _ => None,
            },
        }
    }

    /// Format a duration into a human-readable string.
    fn format_duration(d: Duration) -> String {
        let total_secs = d.num_seconds().abs();
        if total_secs < 60 {
            format!("{}s", total_secs)
        } else if total_secs < 3600 {
            format!("{}m {}s", total_secs / 60, total_secs % 60)
        } else if total_secs < 86400 {
            format!("{}h {}m", total_secs / 3600, (total_secs % 3600) / 60)
        } else {
            format!("{}d {}h", total_secs / 86400, (total_secs % 86400) / 3600)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlineAssessment {
    pub deadline: DateTime<Utc>,
    pub remaining_time: Duration,
    pub estimated_duration: Option<Duration>,
    pub risk_level: DeadlineRisk,
    pub confidence: f64,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeadlineRisk {
    Low,
    Medium,
    High,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalStats {
    pub total_events: usize,
    pub total_relations: usize,
    pub earliest_event: Option<DateTime<Utc>>,
    pub latest_event: Option<DateTime<Utc>>,
    pub event_types: Vec<String>,
    pub time_span: Option<Duration>,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_before() {
        let a = TimeInterval::new(
            Utc::now() - Duration::hours(2),
            Utc::now() - Duration::hours(1),
            "A",
        );
        let b = TimeInterval::new(
            Utc::now(),
            Utc::now() + Duration::hours(1),
            "B",
        );
        assert_eq!(a.relation_to(&b), IntervalRelation::Before);
    }

    #[test]
    fn test_interval_contains() {
        let a = TimeInterval::new(
            Utc::now() - Duration::hours(3),
            Utc::now() + Duration::hours(3),
            "A",
        );
        let b = TimeInterval::new(
            Utc::now() - Duration::hours(1),
            Utc::now() + Duration::hours(1),
            "B",
        );
        assert_eq!(a.relation_to(&b), IntervalRelation::Contains);
    }

    #[test]
    fn test_interval_equal() {
        let now = Utc::now();
        let a = TimeInterval::new(now, now + Duration::hours(1), "A");
        let b = TimeInterval::new(now, now + Duration::hours(1), "B");
        assert_eq!(a.relation_to(&b), IntervalRelation::Equal);
    }

    #[tokio::test]
    async fn test_temporal_index_query_range() {
        let mut index = TemporalIndex::new();
        let now = Utc::now();

        index.add_event(TemporalEvent {
            id: "e1".to_string(),
            interval: TimeInterval::new(now - Duration::hours(2), now - Duration::hours(1), "Event 1"),
            event_type: "test".to_string(),
            description: "Test event 1".to_string(),
            tags: vec!["important".to_string()],
            metadata: HashMap::new(),
            importance: 0.8,
        });

        let results = index.query_range(now - Duration::hours(3), now);
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_temporal_reasoner_record_and_query() {
        let reasoner = TemporalReasoner::new();
        let now = Utc::now();

        reasoner
            .record_event(TemporalEvent {
                id: "e1".to_string(),
                interval: TimeInterval::new(now - Duration::hours(2), now - Duration::hours(1), "Coding"),
                event_type: "coding".to_string(),
                description: "Implemented causal engine".to_string(),
                tags: vec!["development".to_string()],
                metadata: HashMap::new(),
                importance: 0.9,
            })
            .await;

        let answer = reasoner
            .what_happened_between(now - Duration::hours(3), now)
            .await;
        assert!(answer.supporting_events.len() == 1);
    }
}
