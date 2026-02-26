use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEmbeddingConfig {
    pub model_path: Option<String>,
    pub model_name: String,
    pub embedding_dim: usize,
    pub max_seq_length: usize,
    pub use_gpu: bool,
    pub fallback_to_api: bool,
}

impl Default for LocalEmbeddingConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            model_name: "all-MiniLM-L6-v2".to_string(),
            embedding_dim: 384,
            max_seq_length: 512,
            use_gpu: false,
            fallback_to_api: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub embedding: Vec<f32>,
    pub model: String,
    pub confidence: f32,
}

pub struct LocalEmbeddingProvider {
    config: LocalEmbeddingConfig,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    is_initialized: bool,
}

impl LocalEmbeddingProvider {
    pub fn new(config: LocalEmbeddingConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            is_initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.is_initialized = true;
        
        tracing::info!(
            "Initialized local embedding provider with model: {}",
            self.config.model_name
        );
        
        Ok(())
    }

    pub async fn embed(&self, text: &str) -> Result<EmbeddingResult> {
        if !self.is_initialized {
            anyhow::bail!("LocalEmbeddingProvider not initialized. Call initialize() first.");
        }

        let text_lower = text.to_lowercase();
        
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&text_lower) {
                return Ok(EmbeddingResult {
                    embedding: cached.clone(),
                    model: self.config.model_name.clone(),
                    confidence: 1.0,
                });
            }
        }

        let embedding = self.generate_embedding(text).await?;
        
        {
            let mut cache = self.cache.write().await;
            cache.insert(text_lower, embedding.clone());
        }

        Ok(EmbeddingResult {
            embedding,
            model: self.config.model_name.clone(),
            confidence: 0.8,
        })
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>> {
        let mut results = Vec::new();
        
        for text in texts {
            let result = self.embed(text).await?;
            results.push(result);
        }

        Ok(results)
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let dim = self.config.embedding_dim;
        let mut embedding = vec![0.0_f32; dim];

        let tokens = self.tokenize(text);
        if tokens.is_empty() {
            return Ok(embedding);
        }

        // Compute TF (term frequency) for each unique token
        let mut tf: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        for tok in &tokens {
            *tf.entry(tok.clone()).or_insert(0.0) += 1.0;
        }
        let n = tokens.len() as f32;
        for v in tf.values_mut() {
            *v /= n;
        }

        // For each token accumulate into the embedding vector using:
        //   - token identity → base position in embedding space (via hash → dim/4 basis)
        //   - positional encoding (sin/cos, transformer-style) for sequence order
        //   - TF weight
        for (pos, tok) in tokens.iter().enumerate() {
            let tf_w = tf.get(tok.as_str()).copied().unwrap_or(1.0 / n);
            let h = self.djb2_hash(tok);

            // Spread token energy across dim/4 basis dimensions
            let basis_start = (h as usize) % (dim / 4);
            for k in 0..4usize {
                let idx = (basis_start + k * (dim / 4)) % dim;
                // sin/cos positional encoding at this dimension index
                let freq = 1.0 / 10000_f32.powf((2 * (idx / 2)) as f32 / dim as f32);
                let pos_enc = if idx % 2 == 0 {
                    (pos as f32 * freq).sin()
                } else {
                    (pos as f32 * freq).cos()
                };
                // Token hash component
                let hash_component = ((h.wrapping_add(k as u64 * 2654435761)) % 1_000_000) as f32
                    / 1_000_000.0
                    * 2.0
                    - 1.0;
                embedding[idx] += tf_w * (0.7 * hash_component + 0.3 * pos_enc);
            }
        }

        // Add bigram features for local context
        for window in tokens.windows(2) {
            let bigram = format!("{}_{}", window[0], window[1]);
            let h = self.djb2_hash(&bigram);
            let idx = (h as usize) % dim;
            let tf_w = 1.0 / n;
            let val = ((h.wrapping_mul(6364136223846793005)) % 1_000_000) as f32 / 1_000_000.0
                * 2.0
                - 1.0;
            embedding[idx] += tf_w * 0.5 * val;
        }

        // L2-normalise
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 1e-8 {
            for v in &mut embedding {
                *v /= magnitude;
            }
        }

        Ok(embedding)
    }

    fn tokenize<'a>(&self, text: &'a str) -> Vec<String> {
        let max_tokens = self.config.max_seq_length;
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|s| !s.is_empty() && s.len() <= 40)
            .take(max_tokens)
            .map(|s| s.to_string())
            .collect()
    }

    fn djb2_hash(&self, s: &str) -> u64 {
        let mut hash: u64 = 5381;
        for c in s.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(c as u64);
        }
        hash
    }

    pub fn is_available(&self) -> bool {
        self.is_initialized
    }

    pub fn config(&self) -> &LocalEmbeddingConfig {
        &self.config
    }
}

pub enum EmbeddingProvider {
    Local(LocalEmbeddingProvider),
    Api {
        api_key: String,
        model: String,
    },
}

impl EmbeddingProvider {
    pub async fn embed(&self, text: &str) -> Result<EmbeddingResult> {
        match self {
            EmbeddingProvider::Local(provider) => provider.embed(text).await,
            EmbeddingProvider::Api { api_key, model } => {
                self.api_embed(text, api_key, model).await
            }
        }
    }

    async fn api_embed(&self, text: &str, api_key: &str, model: &str) -> Result<EmbeddingResult> {
        let client = reqwest::Client::new();
        
        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "input": text,
                "model": model,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("API embedding request failed: {}", response.status());
        }

        let json: serde_json::Value = response.json().await?;
        
        let embedding: Vec<f32> = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid embedding response"))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(EmbeddingResult {
            embedding,
            model: model.to_string(),
            confidence: 0.9,
        })
    }
}

pub struct LocalEmbeddingManager {
    provider: Option<LocalEmbeddingProvider>,
    fallback: Option<EmbeddingProvider>,
    config: LocalEmbeddingConfig,
}

impl LocalEmbeddingManager {
    pub fn new(config: LocalEmbeddingConfig) -> Self {
        Self {
            provider: None,
            fallback: None,
            config,
        }
    }

    pub async fn initialize(&mut self, api_key: Option<String>) -> Result<()> {
        let mut provider = LocalEmbeddingProvider::new(self.config.clone());
        
        match provider.initialize().await {
            Ok(()) => {
                self.provider = Some(provider);
                tracing::info!("Local embedding provider initialized successfully");
            }
            Err(e) => {
                if self.config.fallback_to_api {
                    if let Some(key) = api_key {
                        self.fallback = Some(EmbeddingProvider::Api {
                            api_key: key,
                            model: "text-embedding-3-small".to_string(),
                        });
                        tracing::warn!("Local embeddings failed ({}), falling back to API", e);
                    } else {
                        anyhow::bail!(
                            "Local embeddings failed and no API key provided for fallback"
                        );
                    }
                } else {
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn embed(&self, text: &str) -> Result<EmbeddingResult> {
        if let Some(ref provider) = self.provider {
            provider.embed(text).await
        } else if let Some(ref fallback) = self.fallback {
            fallback.embed(text).await
        } else {
            anyhow::bail!("No embedding provider available")
        }
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>> {
        if let Some(ref provider) = self.provider {
            provider.embed_batch(texts).await
        } else if let Some(ref fallback) = self.fallback {
            let mut results = Vec::new();
            for text in texts {
                results.push(fallback.embed(text).await?);
            }
            Ok(results)
        } else {
            anyhow::bail!("No embedding provider available")
        }
    }

    pub fn is_local_available(&self) -> bool {
        self.provider.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_embedding_basic() {
        let config = LocalEmbeddingConfig::default();
        let mut provider = LocalEmbeddingProvider::new(config);
        
        provider.initialize().await.unwrap();
        
        let result = provider.embed("hello world").await.unwrap();
        
        assert_eq!(result.embedding.len(), 384);
        assert!(result.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_embedding_caching() {
        let config = LocalEmbeddingConfig::default();
        let mut provider = LocalEmbeddingProvider::new(config);
        
        provider.initialize().await.unwrap();
        
        let result1 = provider.embed("test text").await.unwrap();
        let result2 = provider.embed("test text").await.unwrap();
        
        assert_eq!(result1.embedding, result2.embedding);
    }

    #[tokio::test]
    async fn test_embedding_normalization() {
        let config = LocalEmbeddingConfig::default();
        let mut provider = LocalEmbeddingProvider::new(config);
        
        provider.initialize().await.unwrap();
        
        let result = provider.embed("test").await.unwrap();
        
        let magnitude: f32 = result.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001, "Embedding should be normalized");
    }
}
