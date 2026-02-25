use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use std::fmt::Write as _;

const MAX_THOUGHTS: usize = 1000;
const RECENT_CAPACITY: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    pub id: String,
    pub content: String,
    pub confidence: f64,
    pub thought_type: ThoughtType,
    pub created_at: DateTime<Utc>,
    pub source: ThoughtSource,
    pub tags: Vec<String>,
    pub related_thoughts: Vec<String>,
    pub processed: bool,
    pub importance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThoughtType {
    Observation,
    Inference,
    Question,
    Hypothesis,
    Decision,
    Reflection,
    Learning,
    Planning,
    SelfCorrection,
    Insight,
    Goal,
    Concern,
}

impl std::fmt::Display for ThoughtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThoughtType::Observation => write!(f, "Observation"),
            ThoughtType::Inference => write!(f, "Inference"),
            ThoughtType::Question => write!(f, "Question"),
            ThoughtType::Hypothesis => write!(f, "Hypothesis"),
            ThoughtType::Decision => write!(f, "Decision"),
            ThoughtType::Reflection => write!(f, "Reflection"),
            ThoughtType::Learning => write!(f, "Learning"),
            ThoughtType::Planning => write!(f, "Planning"),
            ThoughtType::SelfCorrection => write!(f, "SelfCorrection"),
            ThoughtType::Insight => write!(f, "Insight"),
            ThoughtType::Goal => write!(f, "Goal"),
            ThoughtType::Concern => write!(f, "Concern"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThoughtSource {
    UserInteraction,
    ToolExecution,
    Reasoning,
    Reflection,
    SelfImprovement,
    External,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStream {
    pub thoughts: VecDeque<Thought>,
    pub unprocessed_count: usize,
    pub last_compacted: DateTime<Utc>,
    pub total_thoughts: u64,
}

pub struct InnerMonologue {
    stream: Arc<RwLock<ThoughtStream>>,
    workspace_dir: PathBuf,
    max_thoughts: usize,
    recent_capacity: usize,
}

impl InnerMonologue {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        Self {
            stream: Arc::new(RwLock::new(ThoughtStream {
                thoughts: VecDeque::with_capacity(MAX_THOUGHTS),
                unprocessed_count: 0,
                last_compacted: Utc::now(),
                total_thoughts: 0,
            })),
            workspace_dir: workspace_dir.clone(),
            max_thoughts: MAX_THOUGHTS,
            recent_capacity: RECENT_CAPACITY,
        }
    }

    pub async fn add_thought(&self, content: &str, confidence: f64) -> Result<String> {
        self.add_thought_with_type(
            content,
            ThoughtType::Observation,
            confidence,
            ThoughtSource::Internal,
        )
        .await
    }

    pub async fn add_thought_with_type(
        &self,
        content: &str,
        thought_type: ThoughtType,
        confidence: f64,
        source: ThoughtSource,
    ) -> Result<String> {
        let thought = Thought {
            id: format!("thought_{}", uuid::Uuid::new_v4()),
            content: content.to_string(),
            confidence,
            thought_type,
            created_at: Utc::now(),
            source,
            tags: self.extract_tags(content),
            related_thoughts: Vec::new(),
            processed: false,
            importance: self.calculate_importance(content, confidence),
        };

        let id = thought.id.clone();

        let mut stream = self.stream.write().await;

        if stream.thoughts.len() >= self.max_thoughts {
            self.evict_low_importance(&mut stream);
        }

        stream.thoughts.push_back(thought);
        stream.unprocessed_count += 1;
        stream.total_thoughts += 1;

        let thought_content = stream.thoughts.back().unwrap().content.clone();
        
        info!(
            "Added thought: {} (type: {:?}, confidence: {:.2})",
            id,
            stream.thoughts.back().unwrap().thought_type,
            confidence
        );
        
        println!("💭 Thought: {}", thought_content.chars().take(120).collect::<String>());

        Ok(id)
    }

    fn extract_tags(&self, content: &str) -> Vec<String> {
        let mut tags = Vec::new();

        let keywords = [
            ("goal", "goal"),
            ("learn", "learning"),
            ("improve", "improvement"),
            ("error", "error"),
            ("success", "success"),
            ("decision", "decision"),
            ("question", "question"),
            ("idea", "idea"),
            ("plan", "planning"),
            ("reflect", "reflection"),
        ];

        let content_lower = content.to_lowercase();
        for (keyword, tag) in keywords {
            if content_lower.contains(keyword) {
                tags.push(tag.to_string());
            }
        }

        tags
    }

    fn calculate_importance(&self, content: &str, confidence: f64) -> f64 {
        let length_factor = (content.len() as f64 / 500.0).min(1.0);
        let confidence_factor = confidence;
        let keyword_bonus = if content.to_lowercase().contains("important")
            || content.to_lowercase().contains("critical")
        {
            0.2
        } else {
            0.0
        };

        (length_factor * 0.3 + confidence_factor * 0.5 + keyword_bonus).min(1.0)
    }

    fn evict_low_importance(&self, stream: &mut ThoughtStream) {
        let mut to_remove = Vec::new();

        for (i, thought) in stream.thoughts.iter().enumerate() {
            if thought.importance < 0.3 && thought.processed {
                to_remove.push(i);
                if to_remove.len() >= stream.thoughts.len() / 10 {
                    break;
                }
            }
        }

        for i in to_remove.into_iter().rev() {
            stream.thoughts.remove(i);
        }
    }

    pub async fn get_recent(&self, count: usize) -> Vec<String> {
        let stream = self.stream.read().await;
        stream
            .thoughts
            .iter()
            .rev()
            .take(count)
            .map(|t| t.content.clone())
            .collect()
    }

    pub async fn get_recent_thoughts(&self, count: usize) -> Vec<Thought> {
        let stream = self.stream.read().await;
        stream.thoughts.iter().rev().take(count).cloned().collect()
    }

    pub async fn get_unprocessed(&self) -> Vec<Thought> {
        let stream = self.stream.read().await;
        stream
            .thoughts
            .iter()
            .filter(|t| !t.processed)
            .cloned()
            .collect()
    }

    pub async fn mark_processed(&self, thought_id: &str) -> Result<()> {
        let mut stream = self.stream.write().await;

        for thought in &mut stream.thoughts {
            if thought.id == thought_id && !thought.processed {
                thought.processed = true;
                stream.unprocessed_count = stream.unprocessed_count.saturating_sub(1);
                break;
            }
        }

        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Vec<Thought> {
        let stream = self.stream.read().await;
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f64, Thought)> = stream
            .thoughts
            .iter()
            .map(|t| {
                let mut score = 0.0;

                for term in &query_terms {
                    if t.content.to_lowercase().contains(term) {
                        score += 1.0;
                    }
                    if t.tags.iter().any(|tag| tag.to_lowercase().contains(term)) {
                        score += 0.5;
                    }
                }

                score *= t.importance;
                (score, t.clone())
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().take(limit).map(|(_, t)| t).collect()
    }

    pub async fn get_by_type(&self, thought_type: ThoughtType, limit: usize) -> Vec<Thought> {
        let stream = self.stream.read().await;
        stream
            .thoughts
            .iter()
            .filter(|t| t.thought_type == thought_type)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_by_tag(&self, tag: &str, limit: usize) -> Vec<Thought> {
        let stream = self.stream.read().await;
        let tag_lower = tag.to_lowercase();

        stream
            .thoughts
            .iter()
            .filter(|t| t.tags.iter().any(|t| t.to_lowercase() == tag_lower))
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn link_thoughts(&self, thought_id: &str, related_id: &str) -> Result<()> {
        let mut stream = self.stream.write().await;

        let mut found_first = false;
        let mut found_second = false;

        for thought in &mut stream.thoughts {
            if thought.id == thought_id {
                if !thought.related_thoughts.contains(&related_id.to_string()) {
                    thought.related_thoughts.push(related_id.to_string());
                }
                found_first = true;
            }
            if thought.id == related_id {
                if !thought.related_thoughts.contains(&thought_id.to_string()) {
                    thought.related_thoughts.push(thought_id.to_string());
                }
                found_second = true;
            }
        }

        if found_first && found_second {
            info!("Linked thoughts: {} <-> {}", thought_id, related_id);
        }

        Ok(())
    }

    pub async fn reflect(&self) -> Result<Option<Thought>> {
        let stream = self.stream.read().await;

        if stream.thoughts.len() < 5 {
            return Ok(None);
        }

        let recent: Vec<_> = stream.thoughts.iter().rev().take(5).collect();

        let low_confidence: Vec<_> = recent.iter().filter(|t| t.confidence < 0.7).collect();

        let unprocessed: Vec<_> = recent.iter().filter(|t| !t.processed).collect();

        if low_confidence.is_empty() && unprocessed.is_empty() {
            return Ok(None);
        }

        let mut reflection_content = String::new();
        reflection_content.push_str("Reflection: ");

        if !low_confidence.is_empty() {
            write!(reflection_content, "{} thoughts have low confidence. ", low_confidence.len()).ok();
        }

        if !unprocessed.is_empty() {
            write!(reflection_content, "{} thoughts need processing. ", unprocessed.len()).ok();
        }

        drop(stream);

        let id = self
            .add_thought_with_type(
                &reflection_content,
                ThoughtType::Reflection,
                0.8,
                ThoughtSource::Reflection,
            )
            .await?;

        let stream = self.stream.read().await;
        let thought = stream.thoughts.iter().find(|t| t.id == id).cloned();

        Ok(thought)
    }

    pub async fn compact(&self) -> Result<usize> {
        let mut stream = self.stream.write().await;

        if stream.thoughts.len() < 100 {
            return Ok(0);
        }

        let old_len = stream.thoughts.len();

        stream.thoughts.retain(|t| {
            t.importance > 0.2
                || !t.processed
                || t.created_at > Utc::now() - chrono::Duration::hours(24)
        });

        stream.last_compacted = Utc::now();

        Ok(old_len - stream.thoughts.len())
    }

    pub async fn get_stats(&self) -> ThoughtStats {
        let stream = self.stream.read().await;

        let mut by_type: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut total_confidence = 0.0;
        let mut total_importance = 0.0;

        for thought in &stream.thoughts {
            let type_name = format!("{:?}", thought.thought_type);
            *by_type.entry(type_name).or_insert(0) += 1;
            total_confidence += thought.confidence;
            total_importance += thought.importance;
        }

        let count = stream.thoughts.len();

        ThoughtStats {
            total_count: stream.total_thoughts,
            current_count: count,
            unprocessed_count: stream.unprocessed_count,
            by_type,
            avg_confidence: if count > 0 {
                total_confidence / count as f64
            } else {
                0.0
            },
            avg_importance: if count > 0 {
                total_importance / count as f64
            } else {
                0.0
            },
            last_compacted: stream.last_compacted,
        }
    }

    pub async fn save(&self) -> Result<()> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join("inner_monologue.json");

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let stream = self.stream.read().await;
        let thoughts: Vec<_> = stream.thoughts.iter().cloned().collect();
        let json = serde_json::to_string_pretty(&thoughts)?;
        tokio::fs::write(&path, json).await?;

        info!("Saved {} thoughts to disk", thoughts.len());
        Ok(())
    }

    pub async fn load(&self) -> Result<()> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join("inner_monologue.json");

        if !path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let thoughts: Vec<Thought> = serde_json::from_str(&content)?;

        let mut stream = self.stream.write().await;
        stream.thoughts.clear();

        let unprocessed = thoughts.iter().filter(|t| !t.processed).count();
        stream.total_thoughts = thoughts.len() as u64;
        stream.unprocessed_count = unprocessed;

        for thought in thoughts {
            stream.thoughts.push_back(thought);
        }

        info!("Loaded {} thoughts from disk", stream.thoughts.len());
        Ok(())
    }

    pub async fn export_summary(&self) -> String {
        let stream = self.stream.read().await;

        let mut summary = String::new();
        summary.push_str("# Inner Monologue Summary\n\n");
        writeln!(summary, "Total thoughts: {}", stream.total_thoughts).ok();
        writeln!(summary, "Current thoughts: {}", stream.thoughts.len()).ok();
        writeln!(summary, "Unprocessed: {}\n", stream.unprocessed_count).ok();

        summary.push_str("## Recent Thoughts\n\n");
        for thought in stream.thoughts.iter().rev().take(10) {
            writeln!(
                summary,
                "- [{:?}] {} (confidence: {:.0}%)",
                thought.thought_type,
                thought.content.chars().take(80).collect::<String>(),
                thought.confidence * 100.0
            ).ok();
        }

        summary
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStats {
    pub total_count: u64,
    pub current_count: usize,
    pub unprocessed_count: usize,
    pub by_type: std::collections::HashMap<String, usize>,
    pub avg_confidence: f64,
    pub avg_importance: f64,
    pub last_compacted: DateTime<Utc>,
}
