//! Web Commands - Search and Fetch
//!
//! Web research capabilities: search via Brave Search API,
//! fetch and extract content from URLs.

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebCommands {
    /// Search the web using Brave Search API
    Search {
        /// Search query
        query: String,
        /// Number of results (1-10)
        #[arg(short = 'n', long, default_value = "5")]
        count: u8,
        /// Country code for region-specific results
        #[arg(long)]
        country: Option<String>,
        /// Freshness filter: day, week, month, year
        #[arg(long)]
        freshness: Option<String>,
    },
    /// Fetch and extract content from URL
    Fetch {
        /// URL to fetch
        url: String,
        /// Extraction mode: markdown or text
        #[arg(long, default_value = "markdown")]
        mode: String,
        /// Max characters to extract
        #[arg(long, default_value = "10000")]
        max_chars: usize,
    },
    /// Quick search and summarize
    Ask {
        /// Question to answer via web search
        question: String,
    },
    /// Check if a URL is reachable
    Check {
        /// URL to check
        url: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub description: String,
    pub score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub url: String,
    pub title: Option<String>,
    pub content: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub enabled: bool,
    pub brave_api_key: Option<String>,
    pub user_agent: String,
    pub timeout_seconds: u32,
    pub max_retries: u8,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            brave_api_key: None,
            user_agent: "Housaky/0.1.0".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

// ============================================================================
// Command Handlers
// ============================================================================

/// Handle web search command
pub async fn handle_search(
    query: &str,
    count: u8,
    _country: Option<&str>,
    _freshness: Option<&str>,
) -> anyhow::Result<()> {
    let config = WebConfig::default();
    
    if let Some(api_key) = &config.brave_api_key {
        let client = reqwest::Client::new();
        let url = "https://api.search.brave.com/res/v1/web/search";
        
        let response = client.get(url)
            .query(&[
                ("q", query),
                ("count", &count.to_string()),
            ])
            .header("Accept", "application/json")
            .header("X-Subscription-Token", api_key)
            .send()
            .await?;

        let results: serde_json::Value = response.json().await?;
        
        if let Some(web_results) = results.get("web").and_then(|w| w.get("results")) {
            println!("Search results for '{}':\n", query);
            for (i, result) in web_results.as_array().unwrap_or(&vec![]).iter().enumerate() {
                let title = result.get("title").and_then(|v| v.as_str()).unwrap_or("No title");
                let url = result.get("url").and_then(|v| v.as_str()).unwrap_or("No URL");
                let desc = result.get("description").and_then(|v| v.as_str()).unwrap_or("No description");
                
                println!("{}. {}", i + 1, title);
                println!("   URL: {}", url);
                println!("   {}\n", desc);
            }
        }
    } else {
        println!("Web search requires Brave Search API key to be configured.");
        println!("Set it via: housaky keys set brave-search <api_key>");
        println!("\nAlternatively, use the brave-search MCP: housaky mcp add brave-search");
    }
    
    Ok(())
}

/// Handle web fetch command
pub async fn handle_fetch(url: &str, mode: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let response = client.get(url)
        .header("User-Agent", "Housaky/0.1.0")
        .send()
        .await?;
    
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/plain")
        .to_string();
    
    let text = response.text().await?;
    
    println!("Fetched: {}", url);
    println!("Content-Type: {}", content_type);
    println!("\n--- Content ---");
    
    if mode == "markdown" || content_type.contains("markdown") {
        println!("{}", text);
    } else {
        let stripped = text.lines()
            .filter(|l| !l.trim().starts_with('<'))
            .take(100)
            .collect::<Vec<_>>()
            .join("\n");
        println!("{}", stripped);
    }
    
    Ok(())
}
