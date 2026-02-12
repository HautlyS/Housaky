//! LLM Integration with Core Orchestrator

use anyhow::Result;
use housaky_llm::{LLMEngine, LLMConfig, ChatMessage, Role};

pub struct LLMIntegration {
    engine: LLMEngine,
}

impl LLMIntegration {
    pub fn new() -> Result<Self> {
        let config = LLMConfig::default();
        let engine = LLMEngine::new(config)?;
        Ok(Self { engine })
    }

    pub async fn reason(&self, problem: &str) -> Result<String> {
        let messages = vec![
            ChatMessage {
                role: Role::System,
                content: "You are an AGI reasoning engine.".to_string(),
            },
            ChatMessage {
                role: Role::User,
                content: problem.to_string(),
            },
        ];
        
        let response = self.engine.chat(messages).await?;
        Ok(response.text)
    }

    pub async fn generate_with_context(&self, context: &[String], query: &str) -> Result<String> {
        let mut messages = vec![
            ChatMessage {
                role: Role::System,
                content: "You are an AGI with access to context.".to_string(),
            },
        ];
        
        for ctx in context {
            messages.push(ChatMessage {
                role: Role::Assistant,
                content: ctx.clone(),
            });
        }
        
        messages.push(ChatMessage {
            role: Role::User,
            content: query.to_string(),
        });
        
        let response = self.engine.chat(messages).await?;
        Ok(response.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_integration() {
        let integration = LLMIntegration::new();
        assert!(integration.is_ok());
    }
}
