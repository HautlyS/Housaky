//! Research Agent — Autonomous paper/documentation reading pipeline.
//!
//! Provides a self-directed research pipeline that reads papers, documentation,
//! and web content, extracts structured knowledge, and integrates it into the
//! knowledge graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{info, warn};
use uuid::Uuid;

// ── Paper Reference ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperReference {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub url: Option<String>,
    pub doi: Option<String>,
    pub abstract_text: Option<String>,
    pub year: Option<u32>,
    pub topics: Vec<String>,
    pub priority: f64,
    pub added_at: DateTime<Utc>,
}

impl PaperReference {
    pub fn new(title: &str, url: Option<&str>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            authors: Vec::new(),
            url: url.map(|s| s.to_string()),
            doi: None,
            abstract_text: None,
            year: None,
            topics: Vec::new(),
            priority: 0.5,
            added_at: Utc::now(),
        }
    }
}

// ── Research Topic ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchTopic {
    pub id: String,
    pub name: String,
    pub description: String,
    pub current_mastery: f64,       // 0.0 – 1.0
    pub target_mastery: f64,
    pub priority: f64,
    pub related_topics: Vec<String>,
    pub keywords: Vec<String>,
    pub papers_read: usize,
    pub last_studied: Option<DateTime<Utc>>,
}

impl ResearchTopic {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            current_mastery: 0.0,
            target_mastery: 0.80,
            priority: 0.5,
            related_topics: Vec::new(),
            keywords: Vec::new(),
            papers_read: 0,
            last_studied: None,
        }
    }

    pub fn mastery_gap(&self) -> f64 {
        (self.target_mastery - self.current_mastery).max(0.0)
    }

    pub fn is_mastered(&self) -> bool {
        self.current_mastery >= self.target_mastery
    }
}

// ── Knowledge Synthesis ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSynthesis {
    pub id: String,
    pub source_paper_ids: Vec<String>,
    pub topic: String,
    pub summary: String,
    pub key_findings: Vec<String>,
    pub actionable_insights: Vec<String>,
    pub confidence: f64,
    pub novelty_score: f64,
    pub synthesised_at: DateTime<Utc>,
}

impl KnowledgeSynthesis {
    pub fn new(topic: &str, summary: &str, source_ids: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_paper_ids: source_ids,
            topic: topic.to_string(),
            summary: summary.to_string(),
            key_findings: Vec::new(),
            actionable_insights: Vec::new(),
            confidence: 0.7,
            novelty_score: 0.0,
            synthesised_at: Utc::now(),
        }
    }
}

// ── Research Pipeline ─────────────────────────────────────────────────────────

pub struct ResearchPipeline {
    pub active_topics: Vec<ResearchTopic>,
    pub paper_queue: VecDeque<PaperReference>,
    pub synthesis_buffer: Vec<KnowledgeSynthesis>,
    pub completed_papers: Vec<PaperReference>,
    pub max_queue_size: usize,
    pub synthesis_threshold: usize, // papers to read before synthesising
}

impl ResearchPipeline {
    pub fn new() -> Self {
        Self {
            active_topics: Vec::new(),
            paper_queue: VecDeque::new(),
            synthesis_buffer: Vec::new(),
            completed_papers: Vec::new(),
            max_queue_size: 100,
            synthesis_threshold: 3,
        }
    }

    pub fn add_topic(&mut self, topic: ResearchTopic) {
        self.active_topics.push(topic);
    }

    pub fn enqueue_paper(&mut self, paper: PaperReference) {
        if self.paper_queue.len() < self.max_queue_size {
            self.paper_queue.push_back(paper);
        } else {
            warn!("Research paper queue full; dropping low-priority paper");
        }
    }

    pub fn enqueue_papers_sorted(&mut self, mut papers: Vec<PaperReference>) {
        papers.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        for p in papers {
            self.enqueue_paper(p);
        }
    }

    /// Pop the highest-priority paper from the queue.
    pub fn next_paper(&mut self) -> Option<PaperReference> {
        self.paper_queue.pop_front()
    }

    /// Mark a paper as read and update topic mastery.
    pub fn mark_paper_read(
        &mut self,
        paper: PaperReference,
        synthesis: Option<KnowledgeSynthesis>,
    ) {
        // Update mastery for relevant topics
        for topic in &mut self.active_topics {
            if paper.topics.iter().any(|t| t == &topic.name)
                || paper.topics.iter().any(|t| topic.keywords.iter().any(|k| t.contains(k.as_str())))
            {
                let delta = 0.05 * paper.priority;
                topic.current_mastery = (topic.current_mastery + delta).min(1.0);
                topic.papers_read += 1;
                topic.last_studied = Some(Utc::now());
            }
        }

        if let Some(syn) = synthesis {
            self.synthesis_buffer.push(syn);
        }
        self.completed_papers.push(paper);
        info!(
            "Research pipeline: {} papers read, {} synthesised",
            self.completed_papers.len(),
            self.synthesis_buffer.len()
        );
    }

    /// Topics ordered by mastery gap × priority (most urgent first).
    pub fn prioritised_topics(&self) -> Vec<&ResearchTopic> {
        let mut topics: Vec<&ResearchTopic> = self.active_topics.iter().collect();
        topics.sort_by(|a, b| {
            let score_a = a.mastery_gap() * a.priority;
            let score_b = b.mastery_gap() * b.priority;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        topics
    }

    /// Check whether enough papers have been read to trigger a synthesis pass.
    pub fn should_synthesise(&self) -> bool {
        self.completed_papers.len() % self.synthesis_threshold == 0
            && !self.completed_papers.is_empty()
    }

    pub fn pipeline_stats(&self) -> ResearchStats {
        ResearchStats {
            active_topics: self.active_topics.len(),
            papers_in_queue: self.paper_queue.len(),
            papers_completed: self.completed_papers.len(),
            syntheses_produced: self.synthesis_buffer.len(),
            mastered_topics: self.active_topics.iter().filter(|t| t.is_mastered()).count(),
            average_mastery: if self.active_topics.is_empty() {
                0.0
            } else {
                self.active_topics.iter().map(|t| t.current_mastery).sum::<f64>()
                    / self.active_topics.len() as f64
            },
        }
    }
}

impl Default for ResearchPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ── HTTP fetch + text extraction ──────────────────────────────────────────────

/// Fetches a URL and extracts readable plain text using the scraper crate.
/// Returns the extracted text (up to `max_chars`) or an error.
pub async fn fetch_url_text(url: &str, max_chars: usize) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Housaky-ResearchAgent/1.0")
        .build()?;

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("HTTP {}: {}", response.status(), url);
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = response.text().await?;

    // If JSON / plain text, return directly
    if content_type.contains("application/json") || content_type.contains("text/plain") {
        return Ok(body.chars().take(max_chars).collect());
    }

    // HTML — extract visible text with scraper
    let document = scraper::Html::parse_document(&body);
    let selector = scraper::Selector::parse("p, h1, h2, h3, li, blockquote, article, section")
        .unwrap_or_else(|_| scraper::Selector::parse("body").unwrap());

    let mut text = String::with_capacity(max_chars.min(65536));
    for element in document.select(&selector) {
        let t = element.text().collect::<Vec<_>>().join(" ");
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            text.push_str(trimmed);
            text.push('\n');
            if text.len() >= max_chars {
                break;
            }
        }
    }

    Ok(text.chars().take(max_chars).collect())
}

impl ResearchPipeline {
    /// Fetch the next queued paper's URL, extract text, produce a synthesis,
    /// and mark the paper as read. Returns `None` if the queue is empty.
    pub async fn process_next_paper(&mut self) -> Option<KnowledgeSynthesis> {
        let paper = self.next_paper()?;

        let url = match &paper.url {
            Some(u) => u.clone(),
            None => {
                warn!("Paper '{}' has no URL; skipping HTTP fetch", paper.title);
                self.mark_paper_read(paper, None);
                return None;
            }
        };

        let text = match fetch_url_text(&url, 8192).await {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to fetch '{}': {}", url, e);
                // Still mark as processed so we don't retry indefinitely
                self.mark_paper_read(paper, None);
                return None;
            }
        };

        // Extract key sentences as findings (first N non-trivial sentences)
        let key_findings: Vec<String> = text
            .split(['.', '!', '?'])
            .map(|s| s.trim().to_string())
            .filter(|s| s.split_whitespace().count() >= 6)
            .take(5)
            .collect();

        let summary = if text.len() > 300 {
            format!("{}…", &text[..300])
        } else {
            text.clone()
        };

        let mut synthesis = KnowledgeSynthesis::new(
            &paper.topics.first().cloned().unwrap_or_else(|| paper.title.clone()),
            &summary,
            vec![paper.id.clone()],
        );
        synthesis.key_findings = key_findings;
        synthesis.confidence = 0.75;
        synthesis.novelty_score = (1.0 - paper.priority).max(0.1);

        info!(
            "Processed paper '{}': extracted {} chars, {} findings",
            paper.title,
            text.len(),
            synthesis.key_findings.len()
        );

        self.mark_paper_read(paper, Some(synthesis.clone()));
        Some(synthesis)
    }

    /// Run a full research cycle: process up to `batch_size` queued papers.
    /// Returns the syntheses produced in this batch.
    pub async fn run_cycle(&mut self, batch_size: usize) -> Vec<KnowledgeSynthesis> {
        let mut produced = Vec::new();
        for _ in 0..batch_size {
            if self.paper_queue.is_empty() {
                break;
            }
            if let Some(syn) = self.process_next_paper().await {
                produced.push(syn);
            }
        }
        info!(
            "Research cycle complete: {} syntheses produced, {} papers remaining",
            produced.len(),
            self.paper_queue.len()
        );
        produced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchStats {
    pub active_topics: usize,
    pub papers_in_queue: usize,
    pub papers_completed: usize,
    pub syntheses_produced: usize,
    pub mastered_topics: usize,
    pub average_mastery: f64,
}
