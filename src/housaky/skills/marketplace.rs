use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPackage {
    pub manifest: SkillManifest,
    pub implementation: Option<String>,
    pub tests: Vec<String>,
    pub provenance: ProvenanceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub capabilities: Vec<String>,
    pub trust_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceInfo {
    pub source: String,
    pub verified: bool,
    pub created_at: String,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillListing {
    pub name: String,
    pub description: String,
    pub author: String,
    pub trust_score: f64,
    pub download_count: u64,
    pub tags: Vec<String>,
}

pub struct SkillMarketplace {
    local_dir: PathBuf,
    cache: Arc<RwLock<HashMap<String, SkillPackage>>>,
    trust_scores: Arc<RwLock<HashMap<String, f64>>>,
}

impl SkillMarketplace {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        let local_dir = workspace_dir.join(".housaky").join("skills").join("marketplace");
        Self {
            local_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
            trust_scores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.local_dir).await?;
        Ok(())
    }

    pub async fn publish(&self, package: SkillPackage) -> Result<String> {
        let skill_name = package.manifest.name.clone();
        
        let skill_dir = self.local_dir.join(&skill_name);
        tokio::fs::create_dir_all(&skill_dir).await?;

        let manifest_path = skill_dir.join("SKILL.toml");
        let toml_content = toml::to_string_pretty(&package.manifest)?;
        tokio::fs::write(&manifest_path, toml_content).await?;

        if let Some(impl_code) = &package.implementation {
            let impl_path = skill_dir.join("implementation.rs");
            tokio::fs::write(&impl_path, impl_code).await?;
        }

        {
            let mut cache = self.cache.write().await;
            cache.insert(skill_name.clone(), package);
        }

        tracing::info!("Published skill: {} to local marketplace", skill_name);
        
        Ok(skill_name)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SkillListing>> {
        let mut results = Vec::new();
        
        if !self.local_dir.exists() {
            return Ok(results);
        }

        let mut entries = tokio::fs::read_dir(&self.local_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("SKILL.toml");
                if manifest_path.exists() {
                    let content = tokio::fs::read_to_string(&manifest_path).await?;
                    if let Ok(manifest) = toml::from_str::<SkillManifest>(&content) {
                        let query_lower = query.to_lowercase();
                        let matches = manifest.name.to_lowercase().contains(&query_lower)
                            || manifest.description.to_lowercase().contains(&query_lower)
                            || manifest.tags.iter().any(|t| t.to_lowercase().contains(&query_lower));
                        
                        if matches {
                            let trust = {
                                let scores = self.trust_scores.read().await;
                                *scores.get(&manifest.name).unwrap_or(&manifest.trust_score)
                            };

                            results.push(SkillListing {
                                name: manifest.name,
                                description: manifest.description,
                                author: manifest.author,
                                trust_score: trust,
                                download_count: 0,
                                tags: manifest.tags,
                            });
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| b.trust_score.partial_cmp(&a.trust_score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results)
    }

    pub async fn install(&self, name: &str) -> Result<SkillPackage> {
        let package = {
            let cache = self.cache.read().await;
            cache.get(name).cloned()
        };

        if let Some(pkg) = package {
            return Ok(pkg);
        }

        let skill_dir = self.local_dir.join(name);
        let manifest_path = skill_dir.join("SKILL.toml");
        
        if !manifest_path.exists() {
            anyhow::bail!("Skill '{}' not found in marketplace", name);
        }

        let content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: SkillManifest = toml::from_str(&content)?;

        let implementation = {
            let impl_path = skill_dir.join("implementation.rs");
            if impl_path.exists() {
                Some(tokio::fs::read_to_string(&impl_path).await?)
            } else {
                None
            }
        };

        let package = SkillPackage {
            manifest,
            implementation,
            tests: Vec::new(),
            provenance: ProvenanceInfo {
                source: "local".to_string(),
                verified: false,
                created_at: chrono::Utc::now().to_rfc3339(),
                signature: None,
            },
        };

        {
            let mut cache = self.cache.write().await;
            cache.insert(name.to_string(), package.clone());
        }

        tracing::info!("Installed skill: {}", name);
        
        Ok(package)
    }

    pub async fn update_trust_score(&self, name: &str, score: f64) -> Result<()> {
        let mut scores = self.trust_scores.write().await;
        scores.insert(name.to_string(), score);
        
        tracing::info!("Updated trust score for {}: {}", name, score);
        
        Ok(())
    }

    pub async fn list_installed(&self) -> Result<Vec<String>> {
        let mut skills = Vec::new();
        
        if !self.local_dir.exists() {
            return Ok(skills);
        }

        let mut entries = tokio::fs::read_dir(&self.local_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    skills.push(name.to_string());
                }
            }
        }

        Ok(skills)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_marketplace_publish() {
        let temp_dir = tempfile::tempdir().unwrap();
        let marketplace = SkillMarketplace::new(&temp_dir.path().to_path_buf());
        
        marketplace.initialize().await.unwrap();
        
        let package = SkillPackage {
            manifest: SkillManifest {
                name: "test_skill".to_string(),
                version: "1.0.0".to_string(),
                description: "A test skill".to_string(),
                author: "test".to_string(),
                tags: vec!["test".to_string()],
                dependencies: vec![],
                capabilities: vec!["testing".to_string()],
                trust_score: 0.9,
            },
            implementation: Some("fn main() {}".to_string()),
            tests: vec![],
            provenance: ProvenanceInfo {
                source: "test".to_string(),
                verified: true,
                created_at: chrono::Utc::now().to_rfc3339(),
                signature: None,
            },
        };

        let name = marketplace.publish(package).await.unwrap();
        assert_eq!(name, "test_skill");
    }

    #[tokio::test]
    async fn test_marketplace_search() {
        let temp_dir = tempfile::tempdir().unwrap();
        let marketplace = SkillMarketplace::new(&temp_dir.path().to_path_buf());
        
        marketplace.initialize().await.unwrap();
        
        let package = SkillPackage {
            manifest: SkillManifest {
                name: "rust_coding".to_string(),
                version: "1.0.0".to_string(),
                description: "Rust programming skill".to_string(),
                author: "test".to_string(),
                tags: vec!["rust".to_string(), "programming".to_string()],
                dependencies: vec![],
                capabilities: vec!["coding".to_string()],
                trust_score: 0.8,
            },
            implementation: None,
            tests: vec![],
            provenance: ProvenanceInfo {
                source: "test".to_string(),
                verified: false,
                created_at: chrono::Utc::now().to_rfc3339(),
                signature: None,
            },
        };

        marketplace.publish(package).await.unwrap();
        
        let results = marketplace.search("rust").await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "rust_coding");
    }
}
