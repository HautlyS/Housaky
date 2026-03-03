//! HTTP client for the Moltbook REST API.
//!
//! Base URL: `https://www.moltbook.com/api/v1`
//! Auth:     `Authorization: Bearer <api_key>`

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MoltbookConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

// ── Response shapes (subset of Moltbook API) ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistrationResponse {
    pub agent: AgentInfo,
    pub important: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub claim_url: Option<String>,
    pub verification_code: Option<String>,
    pub karma: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    pub post: PostData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostData {
    pub id: String,
    pub title: String,
    pub content: Option<String>,
    pub url: Option<String>,
    pub submolt: Option<String>,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
    pub upvotes: Option<u64>,
    pub downvotes: Option<u64>,
    pub score: Option<i64>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedResponse {
    pub posts: Vec<PostData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    pub success: bool,
    pub action: Option<String>,
    pub karma_change: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmoltResponse {
    pub submolt: SubmoltData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmoltData {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub member_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentResponse {
    pub comment: CommentData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentData {
    pub id: String,
    pub content: String,
    pub author_name: Option<String>,
    pub upvotes: Option<u64>,
    pub created_at: Option<String>,
}

// ── Client ────────────────────────────────────────────────────────────────────

pub struct MoltbookClient {
    http: Client,
    base_url: String,
    api_key: Option<String>,
}

impl MoltbookClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            http: Client::builder()
                .user_agent("Housaky-AGI/1.0 (collective-intelligence)")
                .build()
                .expect("failed to build HTTP client"),
            base_url,
            api_key,
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn auth_header(&self) -> Option<String> {
        self.api_key.as_ref().map(|k| format!("Bearer {k}"))
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let mut req = self.http.get(self.url(path));
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.context("GET request failed")?;
        let status = resp.status();
        let body = resp.text().await.context("reading response body")?;
        if !status.is_success() {
            return Err(anyhow!("Moltbook GET {path} → {status}: {body}"));
        }
        serde_json::from_str(&body)
            .with_context(|| format!("parsing response for GET {path}: {body}"))
    }

    async fn post_json<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let mut req = self
            .http
            .post(self.url(path))
            .header("Content-Type", "application/json");
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.json(body).send().await.context("POST request failed")?;
        let status = resp.status();
        let body_text = resp.text().await.context("reading response body")?;
        if !status.is_success() {
            return Err(anyhow!("Moltbook POST {path} → {status}: {body_text}"));
        }
        serde_json::from_str(&body_text)
            .with_context(|| format!("parsing response for POST {path}: {body_text}"))
    }

    // ── Agent API ─────────────────────────────────────────────────────────────

    /// Register a new agent on Moltbook. Returns the API key.
    pub async fn register_agent(
        &self,
        name: &str,
        description: &str,
    ) -> Result<AgentRegistrationResponse> {
        #[derive(Serialize)]
        struct Body<'a> {
            name: &'a str,
            description: &'a str,
        }
        self.post_json("/agents/register", &Body { name, description })
            .await
    }

    /// Get profile of the currently authenticated agent.
    pub async fn get_my_profile(&self) -> Result<AgentInfo> {
        let resp: serde_json::Value = self.get_json("/agents/me").await?;
        serde_json::from_value(resp["agent"].clone()).context("parsing agent profile")
    }

    // ── Posts API ─────────────────────────────────────────────────────────────

    /// Create a text post in a submolt.
    pub async fn create_post(&self, submolt: &str, title: &str, content: &str) -> Result<PostData> {
        #[derive(Serialize)]
        struct Body<'a> {
            submolt: &'a str,
            title: &'a str,
            content: &'a str,
        }
        let resp: PostResponse = self
            .post_json(
                "/posts",
                &Body {
                    submolt,
                    title,
                    content,
                },
            )
            .await?;
        Ok(resp.post)
    }

    /// Get a single post by ID.
    pub async fn get_post(&self, post_id: &str) -> Result<PostData> {
        let resp: PostResponse = self.get_json(&format!("/posts/{post_id}")).await?;
        Ok(resp.post)
    }

    /// Fetch posts from a submolt feed, sorted by `hot`.
    pub async fn get_submolt_feed(&self, submolt: &str, limit: u32) -> Result<Vec<PostData>> {
        let path = format!("/posts?submolt={submolt}&sort=hot&limit={limit}");
        // Moltbook returns either { posts: [...] } or a flat array; handle both.
        let raw: serde_json::Value = self.get_json(&path).await?;
        if let Some(arr) = raw.as_array() {
            return serde_json::from_value(serde_json::Value::Array(arr.clone()))
                .context("parsing posts array");
        }
        if let Some(posts) = raw.get("posts") {
            return serde_json::from_value(posts.clone()).context("parsing posts.posts");
        }
        Ok(vec![])
    }

    // ── Voting API ────────────────────────────────────────────────────────────

    /// Upvote a post.
    pub async fn upvote_post(&self, post_id: &str) -> Result<VoteResponse> {
        #[derive(Serialize)]
        struct Empty {}
        self.post_json(&format!("/posts/{post_id}/upvote"), &Empty {})
            .await
    }

    /// Downvote a post.
    pub async fn downvote_post(&self, post_id: &str) -> Result<VoteResponse> {
        #[derive(Serialize)]
        struct Empty {}
        self.post_json(&format!("/posts/{post_id}/downvote"), &Empty {})
            .await
    }

    /// Upvote a comment.
    pub async fn upvote_comment(&self, comment_id: &str) -> Result<VoteResponse> {
        #[derive(Serialize)]
        struct Empty {}
        self.post_json(&format!("/comments/{comment_id}/upvote"), &Empty {})
            .await
    }

    // ── Comments API ──────────────────────────────────────────────────────────

    /// Add a comment to a post.
    pub async fn add_comment(&self, post_id: &str, content: &str) -> Result<CommentData> {
        #[derive(Serialize)]
        struct Body<'a> {
            content: &'a str,
        }
        let resp: CommentResponse = self
            .post_json(&format!("/posts/{post_id}/comments"), &Body { content })
            .await?;
        Ok(resp.comment)
    }

    /// Reply to a comment.
    pub async fn reply_comment(
        &self,
        post_id: &str,
        parent_id: &str,
        content: &str,
    ) -> Result<CommentData> {
        #[derive(Serialize)]
        struct Body<'a> {
            content: &'a str,
            parent_id: &'a str,
        }
        let resp: CommentResponse = self
            .post_json(
                &format!("/posts/{post_id}/comments"),
                &Body { content, parent_id },
            )
            .await?;
        Ok(resp.comment)
    }

    // ── Submolt API ───────────────────────────────────────────────────────────

    /// Ensure a submolt exists; create it if it doesn't.
    pub async fn ensure_submolt(
        &self,
        name: &str,
        display_name: &str,
        description: &str,
    ) -> Result<SubmoltData> {
        // Try to get existing submolt first.
        match self.get_submolt(name).await {
            Ok(s) => {
                debug!("Submolt '{name}' already exists");
                return Ok(s);
            }
            Err(e) => {
                debug!("Submolt '{name}' not found ({e}), creating...");
            }
        }
        self.create_submolt(name, display_name, description).await
    }

    pub async fn get_submolt(&self, name: &str) -> Result<SubmoltData> {
        let resp: SubmoltResponse = self.get_json(&format!("/submolts/{name}")).await?;
        Ok(resp.submolt)
    }

    pub async fn create_submolt(
        &self,
        name: &str,
        display_name: &str,
        description: &str,
    ) -> Result<SubmoltData> {
        #[derive(Serialize)]
        struct Body<'a> {
            name: &'a str,
            display_name: &'a str,
            description: &'a str,
        }
        let resp: SubmoltResponse = self
            .post_json(
                "/submolts",
                &Body {
                    name,
                    display_name,
                    description,
                },
            )
            .await?;
        Ok(resp.submolt)
    }

    pub async fn subscribe_submolt(&self, name: &str) -> Result<()> {
        #[derive(Serialize)]
        struct Empty {}
        let _: serde_json::Value = self
            .post_json(&format!("/submolts/{name}/subscribe"), &Empty {})
            .await?;
        Ok(())
    }

    // ── Search API ────────────────────────────────────────────────────────────

    /// Search posts/agents/submolts.
    pub async fn search(&self, query: &str, limit: u32) -> Result<serde_json::Value> {
        let encoded = urlencoding_simple(query);
        self.get_json(&format!("/search?q={encoded}&limit={limit}"))
            .await
    }

    // ── Feed API ──────────────────────────────────────────────────────────────

    /// Get personalized feed (subscribed submolts + followed agents).
    pub async fn get_feed(&self, sort: &str, limit: u32) -> Result<Vec<PostData>> {
        let path = format!("/feed?sort={sort}&limit={limit}");
        let raw: serde_json::Value = self.get_json(&path).await?;
        if let Some(posts) = raw.get("posts").and_then(|v| v.as_array()) {
            return serde_json::from_value(serde_json::Value::Array(posts.clone()))
                .context("parsing feed posts");
        }
        if let Some(arr) = raw.as_array() {
            return serde_json::from_value(serde_json::Value::Array(arr.clone()))
                .context("parsing feed array");
        }
        Ok(vec![])
    }

    /// Check agent status / claim status.
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        self.get_json("/agents/status").await
    }
}

/// Minimal URL percent-encoding for search queries (spaces → %20).
fn urlencoding_simple(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '+' => "%2B".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            '#' => "%23".to_string(),
            _ => c.to_string(),
        })
        .collect()
}
