use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionQuery {
    pub query: String,
    pub target_subsystem: Option<Subsystem>,
    pub depth: QueryDepth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Subsystem {
    KnowledgeGraph,
    Beliefs,
    Goals,
    Memory,
    Reasoning,
    Capabilities,
    Skills,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryDepth {
    Shallow,
    Medium,
    Deep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionResponse {
    pub query: String,
    pub answer: String,
    pub confidence: f64,
    pub sources: Vec<String>,
    pub subsystem: Subsystem,
}

pub struct NaturalLanguageIntrospector {
    skills: Vec<String>,
    capabilities: Vec<String>,
    goals: Vec<String>,
    knowledge_topics: Vec<String>,
}

impl NaturalLanguageIntrospector {
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            capabilities: Vec::new(),
            goals: Vec::new(),
            knowledge_topics: Vec::new(),
        }
    }

    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_goals(mut self, goals: Vec<String>) -> Self {
        self.goals = goals;
        self
    }

    pub fn with_knowledge_topics(mut self, topics: Vec<String>) -> Self {
        self.knowledge_topics = topics;
        self
    }

    pub async fn query(&self, q: &str) -> Result<IntrospectionResponse> {
        let query = self.parse_query(q).await?;
        
        match query.target_subsystem {
            Some(Subsystem::KnowledgeGraph) => self.query_knowledge_graph(q).await,
            Some(Subsystem::Beliefs) => self.query_beliefs(q).await,
            Some(Subsystem::Goals) => self.query_goals(q).await,
            Some(Subsystem::Memory) => self.query_memory(q).await,
            Some(Subsystem::Reasoning) => self.query_reasoning(q).await,
            Some(Subsystem::Capabilities) => self.query_capabilities(q).await,
            Some(Subsystem::Skills) => self.query_skills(q).await,
            _ => self.query_all(q).await,
        }
    }

    async fn parse_query(&self, q: &str) -> Result<IntrospectionQuery> {
        let q_lower = q.to_lowercase();
        
        let subsystem = if q_lower.contains("believe") || q_lower.contains("belief") {
            Some(Subsystem::Beliefs)
        } else if q_lower.contains("goal") || q_lower.contains("plan") {
            Some(Subsystem::Goals)
        } else if q_lower.contains("memory") || q_lower.contains("remember") {
            Some(Subsystem::Memory)
        } else if q_lower.contains("reason") || q_lower.contains("think") {
            Some(Subsystem::Reasoning)
        } else if q_lower.contains("knowledge") || q_lower.contains("know") {
            Some(Subsystem::KnowledgeGraph)
        } else if q_lower.contains("capabilit") || q_lower.contains("skill") {
            Some(Subsystem::Capabilities)
        } else {
            Some(Subsystem::All)
        };

        let depth = if q_lower.contains("detail") || q_lower.contains("explain") {
            QueryDepth::Deep
        } else if q_lower.contains("brief") || q_lower.contains("summary") {
            QueryDepth::Shallow
        } else {
            QueryDepth::Medium
        };

        Ok(IntrospectionQuery {
            query: q.to_string(),
            target_subsystem: subsystem,
            depth,
        })
    }

    async fn query_all(&self, q: &str) -> Result<IntrospectionResponse> {
        let mut answers = Vec::new();
        let mut sources = Vec::new();

        let skills_resp = self.query_skills(q).await?;
        answers.push(skills_resp.answer);
        sources.extend(skills_resp.sources);

        let caps_resp = self.query_capabilities(q).await?;
        answers.push(caps_resp.answer);
        sources.extend(caps_resp.sources);

        let goals_resp = self.query_goals(q).await?;
        answers.push(goals_resp.answer);
        sources.extend(goals_resp.sources);

        let answer = answers.join("\n\n");

        Ok(IntrospectionResponse {
            query: q.to_string(),
            answer,
            confidence: 0.7,
            sources,
            subsystem: Subsystem::All,
        })
    }

    async fn query_knowledge_graph(&self, q: &str) -> Result<IntrospectionResponse> {
        let topics_str = if self.knowledge_topics.is_empty() {
            "No knowledge topics tracked".to_string()
        } else {
            format!("Known topics: {}", self.knowledge_topics.join(", "))
        };

        Ok(IntrospectionResponse {
            query: q.to_string(),
            answer: topics_str,
            confidence: 0.8,
            sources: vec!["knowledge_graph".to_string()],
            subsystem: Subsystem::KnowledgeGraph,
        })
    }

    async fn query_beliefs(&self, q: &str) -> Result<IntrospectionResponse> {
        Ok(IntrospectionResponse {
            query: q.to_string(),
            answer: "Belief system is active and tracking multiple beliefs with confidence levels.".to_string(),
            confidence: 0.7,
            sources: vec!["belief_tracker".to_string()],
            subsystem: Subsystem::Beliefs,
        })
    }

    async fn query_goals(&self, q: &str) -> Result<IntrospectionResponse> {
        let goals_str = if self.goals.is_empty() {
            "No active goals".to_string()
        } else {
            format!("Active goals: {}", self.goals.join(", "))
        };

        Ok(IntrospectionResponse {
            query: q.to_string(),
            answer: goals_str,
            confidence: 0.9,
            sources: vec!["goal_engine".to_string()],
            subsystem: Subsystem::Goals,
        })
    }

    async fn query_memory(&self, q: &str) -> Result<IntrospectionResponse> {
        Ok(IntrospectionResponse {
            query: q.to_string(),
            answer: "Memory system is active with working, episodic, and semantic memory stores.".to_string(),
            confidence: 0.8,
            sources: vec!["memory".to_string()],
            subsystem: Subsystem::Memory,
        })
    }

    async fn query_reasoning(&self, q: &str) -> Result<IntrospectionResponse> {
        Ok(IntrospectionResponse {
            query: q.to_string(),
            answer: "Reasoning engine supports CoT, ReAct, ToT, and Reflexion strategies.".to_string(),
            confidence: 0.8,
            sources: vec!["reasoning_engine".to_string()],
            subsystem: Subsystem::Reasoning,
        })
    }

    async fn query_capabilities(&self, q: &str) -> Result<IntrospectionResponse> {
        let caps_str = if self.capabilities.is_empty() {
            "Capability tracking is initializing...".to_string()
        } else {
            format!("Current capabilities: {}", self.capabilities.join(", "))
        };

        Ok(IntrospectionResponse {
            query: q.to_string(),
            answer: caps_str,
            confidence: 0.9,
            sources: vec!["capability_tracker".to_string()],
            subsystem: Subsystem::Capabilities,
        })
    }

    async fn query_skills(&self, q: &str) -> Result<IntrospectionResponse> {
        let skills_str = if self.skills.is_empty() {
            "No skills registered".to_string()
        } else {
            format!("Available skills: {}", self.skills.join(", "))
        };

        Ok(IntrospectionResponse {
            query: q.to_string(),
            answer: skills_str,
            confidence: 0.9,
            sources: vec!["skill_registry".to_string()],
            subsystem: Subsystem::Skills,
        })
    }
}

impl Default for NaturalLanguageIntrospector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_query_beliefs() {
        let introspector = NaturalLanguageIntrospector::new();
        let query = introspector.parse_query("What do I believe about safety?").await.unwrap();
        
        assert!(matches!(query.target_subsystem, Some(Subsystem::Beliefs)));
    }

    #[tokio::test]
    async fn test_parse_query_goals() {
        let introspector = NaturalLanguageIntrospector::new();
        let query = introspector.parse_query("What are my current goals?").await.unwrap();
        
        assert!(matches!(query.target_subsystem, Some(Subsystem::Goals)));
    }

    #[tokio::test]
    async fn test_query_skills() {
        let introspector = NaturalLanguageIntrospector::new()
            .with_skills(vec!["coding".to_string(), "research".to_string()]);
        
        let response = introspector.query_skills("What skills do I have?").await.unwrap();
        
        assert!(response.answer.contains("coding"));
        assert!(response.answer.contains("research"));
    }

    #[tokio::test]
    async fn test_query_all() {
        let introspector = NaturalLanguageIntrospector::new()
            .with_skills(vec!["test_skill".to_string()])
            .with_capabilities(vec!["reasoning".to_string()])
            .with_goals(vec!["learn".to_string()]);

        let response = introspector.query_all("Tell me about yourself").await.unwrap();

        assert!(response.sources.len() >= 3);
    }
}
