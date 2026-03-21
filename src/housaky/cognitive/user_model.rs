//! Cross-Session User Modeling (Honcho-inspired)
//!
//! Builds a deepening model of the user across sessions.
//! Tracks preferences, communication patterns, and dialectic evolution.
//! Inspired by Hermes Agent's Honcho integration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

/// A user profile that evolves across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    
    /// Detected preferences (key -> value with confidence)
    pub preferences: HashMap<String, PreferenceEntry>,
    
    /// Communication style patterns
    pub communication_style: CommunicationStyle,
    
    /// Topics of interest with engagement scores
    pub topics: HashMap<String, TopicEntry>,
    
    /// Dialectic positions (views that have been discussed/challenged)
    pub dialectic_positions: Vec<DialecticPosition>,
    
    /// Session history summaries
    pub session_summaries: Vec<SessionSummary>,
    
    /// Behavioral patterns detected
    pub patterns: Vec<UserPattern>,
    
    /// Questions asked (for understanding curiosity patterns)
    pub question_history: Vec<QuestionEntry>,
    
    /// Feedback given (positive/negative responses)
    pub feedback_log: Vec<FeedbackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceEntry {
    pub value: String,
    pub confidence: f64,
    pub first_observed: DateTime<Utc>,
    pub last_confirmed: DateTime<Utc>,
    pub observation_count: u32,
    pub sources: Vec<String>, // Where this preference was inferred from
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationStyle {
    pub formality: f64,        // 0.0 = casual, 1.0 = formal
    pub verbosity: f64,       // 0.0 = terse, 1.0 = verbose
    pub technical_depth: f64,  // 0.0 = simple, 1.0 = technical
    pub preferred_language: Option<String>,
    pub emoji_usage: f64,      // 0.0 = none, 1.0 = heavy
    pub code_preference: f64,  // 0.0 = prose, 1.0 = code-heavy
    pub response_length_pref: f64, // 0.0 = brief, 1.0 = detailed
}

impl Default for CommunicationStyle {
    fn default() -> Self {
        Self {
            formality: 0.5,
            verbosity: 0.5,
            technical_depth: 0.5,
            preferred_language: None,
            emoji_usage: 0.3,
            code_preference: 0.5,
            response_length_pref: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicEntry {
    pub interest_score: f64,
    pub expertise_level: f64,  // User's knowledge level
    pub mentions: u32,
    pub last_discussed: DateTime<Utc>,
    pub subtopics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialecticPosition {
    pub topic: String,
    pub position: String,
    pub confidence: f64,
    pub challenges_received: u32,
    pub position_evolved: bool,
    pub evolution_history: Vec<PositionEvolution>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionEvolution {
    pub from_position: String,
    pub to_position: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub topics_covered: Vec<String>,
    pub decisions_made: Vec<String>,
    pub mood_indicators: Vec<String>,
    pub key_exchanges: u32,
    pub satisfaction_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPattern {
    pub pattern_type: PatternType,
    pub description: String,
    pub frequency: u32,
    pub last_observed: DateTime<Utc>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    TimePreference,      // User prefers certain times
    TopicSequence,       // User discusses topics in sequence
    QuestionPattern,     // How user asks questions
    DecisionStyle,       // How user makes decisions
    FeedbackStyle,       // How user gives feedback
    TaskApproach,        // How user approaches tasks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionEntry {
    pub question: String,
    pub topic: String,
    pub timestamp: DateTime<Utc>,
    pub answered: bool,
    pub follow_up_questions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub feedback_type: FeedbackType,
    pub context: String,
    pub timestamp: DateTime<Utc>,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeedbackType {
    Positive,
    Negative,
    Neutral,
    Correction,
    Clarification,
}

/// The user modeling engine
pub struct UserModelingEngine {
    profiles: Arc<RwLock<HashMap<String, UserProfile>>>,
    current_user: Arc<RwLock<Option<String>>>,
}

impl UserModelingEngine {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            current_user: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the current user for this session
    pub async fn set_current_user(&self, user_id: String) -> Result<()> {
        let mut current = self.current_user.write().await;
        *current = Some(user_id.clone());
        
        let mut profiles = self.profiles.write().await;
        if !profiles.contains_key(&user_id) {
            profiles.insert(user_id.clone(), UserProfile {
                user_id: user_id.clone(),
                created_at: Utc::now(),
                last_updated: Utc::now(),
                preferences: HashMap::new(),
                communication_style: CommunicationStyle::default(),
                topics: HashMap::new(),
                dialectic_positions: Vec::new(),
                session_summaries: Vec::new(),
                patterns: Vec::new(),
                question_history: Vec::new(),
                feedback_log: Vec::new(),
            });
        }
        
        Ok(())
    }

    /// Record a user preference
    pub async fn record_preference(
        &self,
        key: String,
        value: String,
        source: String,
        confidence: f64,
    ) -> Result<()> {
        let current = self.current_user.read().await;
        let user_id = match current.as_ref() {
            Some(id) => id.clone(),
            None => return Ok(()),
        };
        drop(current);
        
        let mut profiles = self.profiles.write().await;
        if let Some(profile) = profiles.get_mut(&user_id) {
            let now = Utc::now();
            
            if let Some(entry) = profile.preferences.get_mut(&key) {
                // Update existing preference
                entry.observation_count += 1;
                entry.last_confirmed = now;
                entry.confidence = (entry.confidence + confidence) / 2.0;
                if !entry.sources.contains(&source) {
                    entry.sources.push(source);
                }
            } else {
                // New preference
                profile.preferences.insert(key, PreferenceEntry {
                    value,
                    confidence,
                    first_observed: now,
                    last_confirmed: now,
                    observation_count: 1,
                    sources: vec![source],
                });
            }
            
            profile.last_updated = now;
        }
        
        Ok(())
    }

    /// Update communication style based on observation
    pub async fn update_communication_style(
        &self,
        formality: Option<f64>,
        verbosity: Option<f64>,
        technical_depth: Option<f64>,
        emoji_usage: Option<f64>,
        code_preference: Option<f64>,
    ) -> Result<()> {
        let current = self.current_user.read().await;
        let user_id = match current.as_ref() {
            Some(id) => id.clone(),
            None => return Ok(()),
        };
        drop(current);
        
        let mut profiles = self.profiles.write().await;
        if let Some(profile) = profiles.get_mut(&user_id) {
            let style = &mut profile.communication_style;
            
            // Weighted average with existing (learning rate 0.3)
            if let Some(v) = formality {
                style.formality = style.formality * 0.7 + v * 0.3;
            }
            if let Some(v) = verbosity {
                style.verbosity = style.verbosity * 0.7 + v * 0.3;
            }
            if let Some(v) = technical_depth {
                style.technical_depth = style.technical_depth * 0.7 + v * 0.3;
            }
            if let Some(v) = emoji_usage {
                style.emoji_usage = style.emoji_usage * 0.7 + v * 0.3;
            }
            if let Some(v) = code_preference {
                style.code_preference = style.code_preference * 0.7 + v * 0.3;
            }
            
            profile.last_updated = Utc::now();
        }
        
        Ok(())
    }

    /// Record a topic discussion
    pub async fn record_topic(
        &self,
        topic: String,
        subtopics: Vec<String>,
        expertise_hint: Option<f64>,
    ) -> Result<()> {
        let current = self.current_user.read().await;
        let user_id = match current.as_ref() {
            Some(id) => id.clone(),
            None => return Ok(()),
        };
        drop(current);
        
        let mut profiles = self.profiles.write().await;
        if let Some(profile) = profiles.get_mut(&user_id) {
            let now = Utc::now();
            
            if let Some(entry) = profile.topics.get_mut(&topic) {
                entry.mentions += 1;
                entry.last_discussed = now;
                entry.interest_score = (entry.interest_score + 0.1).min(1.0);
                for sub in subtopics {
                    if !entry.subtopics.contains(&sub) {
                        entry.subtopics.push(sub);
                    }
                }
            } else {
                profile.topics.insert(topic, TopicEntry {
                    interest_score: 0.5,
                    expertise_level: expertise_hint.unwrap_or(0.5),
                    mentions: 1,
                    last_discussed: now,
                    subtopics,
                });
            }
        }
        
        Ok(())
    }

    /// Record a dialectic position (user's view on a topic)
    pub async fn record_dialectic_position(
        &self,
        topic: String,
        position: String,
        confidence: f64,
    ) -> Result<()> {
        let current = self.current_user.read().await;
        let user_id = match current.as_ref() {
            Some(id) => id.clone(),
            None => return Ok(()),
        };
        drop(current);
        
        let mut profiles = self.profiles.write().await;
        if let Some(profile) = profiles.get_mut(&user_id) {
            // Check if position on this topic exists
            if let Some(existing) = profile.dialectic_positions.iter_mut().find(|p| p.topic == topic) {
                if existing.position != position {
                    // Position evolved!
                    existing.evolution_history.push(PositionEvolution {
                        from_position: existing.position.clone(),
                        to_position: position.clone(),
                        reason: "New information or perspective".to_string(),
                        timestamp: Utc::now(),
                    });
                    existing.position = position;
                    existing.position_evolved = true;
                }
                existing.confidence = confidence;
            } else {
                profile.dialectic_positions.push(DialecticPosition {
                    topic,
                    position,
                    confidence,
                    challenges_received: 0,
                    position_evolved: false,
                    evolution_history: Vec::new(),
                    created_at: Utc::now(),
                });
            }
        }
        
        Ok(())
    }

    /// Record feedback from user
    pub async fn record_feedback(
        &self,
        feedback_type: FeedbackType,
        context: String,
        topic: Option<String>,
    ) -> Result<()> {
        let current = self.current_user.read().await;
        let user_id = match current.as_ref() {
            Some(id) => id.clone(),
            None => return Ok(()),
        };
        drop(current);
        
        let mut profiles = self.profiles.write().await;
        if let Some(profile) = profiles.get_mut(&user_id) {
            profile.feedback_log.push(FeedbackEntry {
                feedback_type,
                context,
                timestamp: Utc::now(),
                topic,
            });
            
            // Keep last 100 feedback entries
            if profile.feedback_log.len() > 100 {
                profile.feedback_log.remove(0);
            }
        }
        
        Ok(())
    }

    /// Get user profile for context
    pub async fn get_user_context(&self) -> Option<UserContext> {
        let current = self.current_user.read().await;
        let user_id = current.as_ref()?.clone();
        drop(current);
        
        let profiles = self.profiles.read().await;
        let profile = profiles.get(&user_id)?;
        
        Some(UserContext {
            top_preferences: profile.preferences.iter()
                .filter(|(_, e)| e.confidence > 0.7)
                .map(|(k, e)| (k.clone(), e.value.clone()))
                .take(5)
                .collect(),
            communication_style: profile.communication_style.clone(),
            top_topics: profile.topics.iter()
                .filter(|(_, e)| e.interest_score > 0.5)
                .map(|(k, _)| k.clone())
                .take(5)
                .collect(),
            known_positions: profile.dialectic_positions.iter()
                .map(|p| format!("{}: {}", p.topic, p.position))
                .take(3)
                .collect(),
        })
    }

    /// Get stats about user modeling
    pub async fn stats(&self) -> UserModelingStats {
        let profiles = self.profiles.read().await;
        let current = self.current_user.read().await;
        
        let mut total_preferences = 0;
        let mut total_topics = 0;
        let mut total_positions = 0;
        let mut total_feedback = 0;
        
        for profile in profiles.values() {
            total_preferences += profile.preferences.len();
            total_topics += profile.topics.len();
            total_positions += profile.dialectic_positions.len();
            total_feedback += profile.feedback_log.len();
        }
        
        UserModelingStats {
            total_users: profiles.len(),
            current_user: current.clone(),
            total_preferences,
            total_topics,
            total_dialectic_positions: total_positions,
            total_feedback_entries: total_feedback,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UserContext {
    pub top_preferences: Vec<(String, String)>,
    pub communication_style: CommunicationStyle,
    pub top_topics: Vec<String>,
    pub known_positions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct UserModelingStats {
    pub total_users: usize,
    pub current_user: Option<String>,
    pub total_preferences: usize,
    pub total_topics: usize,
    pub total_dialectic_positions: usize,
    pub total_feedback_entries: usize,
}

impl Default for UserModelingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_modeling() {
        let engine = UserModelingEngine::new();
        
        engine.set_current_user("test_user".to_string()).await.unwrap();
        engine.record_preference(
            "programming_language".to_string(),
            "Rust".to_string(),
            "explicit".to_string(),
            0.9,
        ).await.unwrap();
        
        let context = engine.get_user_context().await.unwrap();
        assert_eq!(context.top_preferences.len(), 1);
        assert_eq!(context.top_preferences[0].1, "Rust");
    }

    #[tokio::test]
    async fn test_communication_style_learning() {
        let engine = UserModelingEngine::new();
        engine.set_current_user("test_user".to_string()).await.unwrap();
        
        // Observe formal communication
        engine.update_communication_style(Some(0.8), None, None, None, None).await.unwrap();
        
        let context = engine.get_user_context().await.unwrap();
        assert!(context.communication_style.formality > 0.5);
    }
}
