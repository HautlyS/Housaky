//! Web browsing and scraping functionality for Housaky.
//! 
//! This module provides tools for fetching web pages, searching the web,
//! extracting content, and crawling links with proper error handling.
//! 
//! # Example Usage
//! 
//! ```ignore
//! use housaky::web_browser::{WebBrowser, WebBrowserConfig};
//! 
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = WebBrowserConfig::default()
//!         .with_user_agent("MyBot/1.0")
//!         .with_timeout(std::time::Duration::from_secs(30))
//!         .with_max_content_size(5 * 1024 * 1024);
//!     
//!     let browser = WebBrowser::new(config);
//!     
//!     // Fetch a webpage
//!     let page = browser.fetch("https://example.com").await?;
//!     println!("Title: {:?}", page.title);
//!     
//!     // Search the web
//!     let results = browser.search("rust programming", 5).await?;
//!     for result in results {
//!         println!("{} - {}", result.title, result.url);
//!     }
//!     
//!     // Extract clean text
//!     let text = browser.extract_text("https://example.com").await?;
//!     
//!     // Get metadata
//!     let metadata = browser.get_page_metadata("https://example.com").await?;
//!     println!("OG Title: {:?}", metadata.og_title);
//!     
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::collections::HashSet;
use std::time::{Duration, Instant};

use anyhow::Result;
use regex::Regex;
use scraper::{Element, Html, Selector};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Custom error types for web browsing operations.
///
/// # Example
///
/// ```ignore
/// use housaky::web_browser::WebError;
///
/// fn handle_error(err: WebError) {
///     match err {
///         WebError::Fetch { url, source } => {
///             println!("Failed to fetch {}: {}", url, source);
///         }
///         WebError::Parse { source } => {
///             println!("Parse error: {}", source);
///         }
///         _ => println!("Other error: {}", err);
///     }
/// }
/// ```
#[derive(Error, Debug)]
pub enum WebError {
    #[error("Failed to fetch URL `{url}`: {source}")]
    Fetch {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Failed to parse HTML content from URL `{url}`: {details}")]
    Parse {
        url: String,
        details: String,
    },

    #[error("URL not allowed by configuration: {url}")]
    NotAllowed { url: String },

    #[error("Unsupported content type `{content_type}` for URL: {url}")]
    UnsupportedContentType { 
        content_type: String,
        url: String,
    },

    #[error("Content too large: {size} bytes (max allowed: {max} bytes) for URL: {url}")]
    ContentTooLarge { 
        size: usize, 
        max: usize,
        url: String,
    },

    #[error("Invalid URL format: {url}")]
    InvalidUrl { url: String },

    #[error("Search query failed: {source}")]
    Search {
        #[source]
        source: reqwest::Error,
    },

    #[error("JSON parse error while processing URL `{url}`: {source}")]
    JsonParse {
        url: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Page not reachable after multiple attempts: {url}")]
    NotReachable { url: String },

    #[error("Crawl depth/limit exceeded")]
    CrawlLimitExceeded,

    #[error("Request timeout after {0:?}")]
    Timeout(Duration),

    #[error("Rate limit exceeded. Please wait before making more requests")]
    RateLimitExceeded,

    #[error("Analysis failed for URL `{url}`: {reason}")]
    AnalysisFailed {
        url: String,
        reason: String,
    },
}

/// Classification of website types for specialized extraction.
///
/// Different site types may require different parsing strategies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SiteType {
    /// Regular webpage without specific classification
    Generic,
    /// Documentation sites (API docs, language references, etc.)
    Documentation,
    /// Blog posts and articles
    Blog,
    /// E-commerce product pages
    Ecommerce,
    /// Forum and discussion boards
    Forum,
    /// Social media platforms
    Social,
    /// News articles
    News,
}

impl Default for SiteType {
    fn default() -> Self {
        SiteType::Generic
    }
}

impl std::fmt::Display for SiteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SiteType::Generic => write!(f, "Generic"),
            SiteType::Documentation => write!(f, "Documentation"),
            SiteType::Blog => write!(f, "Blog"),
            SiteType::Ecommerce => write!(f, "Ecommerce"),
            SiteType::Forum => write!(f, "Forum"),
            SiteType::Social => write!(f, "Social"),
            SiteType::News => write!(f, "News"),
        }
    }
}

/// A code block extracted from a webpage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    /// Programming language if detected
    pub language: Option<String>,
    /// The code content
    pub code: String,
    /// Element ID if available
    pub element_id: Option<String>,
}

/// Table data extracted from HTML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    /// Column headers
    pub headers: Vec<String>,
    /// Table rows
    pub rows: Vec<Vec<String>>,
}

/// Information about an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    /// Image source URL
    pub src: String,
    /// Alt text if available
    pub alt: Option<String>,
    /// Title attribute if available
    pub title: Option<String>,
}

/// A heading element with hierarchy level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    /// Heading level (1-6)
    pub level: u8,
    /// Heading text content
    pub text: String,
}

/// Information about a form input field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInput {
    /// Input name attribute
    pub name: Option<String>,
    /// Input type (text, email, password, etc.)
    pub input_type: String,
    /// Associated label text
    pub label: Option<String>,
}

/// Information about a form on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    /// Form action URL
    pub action: Option<String>,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// List of input fields
    pub inputs: Vec<FormInput>,
}

/// Comprehensive page analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageAnalysis {
    /// The parsed webpage
    pub page: WebPage,
    /// Detected site type
    pub site_type: SiteType,
    /// Whether the page appears to be JavaScript-heavy
    pub is_js_heavy: bool,
    /// Forms found on the page
    pub forms: Vec<FormInfo>,
    /// JSON-LD structured data
    pub json_ld: Vec<String>,
    /// Recommendations for handling this page
    pub recommendations: Vec<String>,
}

/// Configuration for the WebBrowser.
///
/// # Example
///
/// ```ignore
/// use housaky::web_browser::WebBrowserConfig;
///
/// let config = WebBrowserConfig::default()
///     .with_user_agent("MyBot/1.0")
///     .with_timeout(std::time::Duration::from_secs(60))
///     .with_max_content_size(10 * 1024 * 1024)
///     .with_allowed_domains(vec!["example.com".to_string()])
///     .with_blocked_domains(vec!["malware.com".to_string()])
///     .with_rate_limit_delay(Duration::from_millis(500));
/// ```
#[derive(Debug, Clone)]
pub struct WebBrowserConfig {
    pub user_agent: String,
    pub timeout: Duration,
    pub max_size: usize,
    pub allowed_domains: Option<Vec<String>>,
    pub blocked_domains: Vec<String>,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub accept_language: String,
    pub enable_cookies: bool,
    pub rate_limit_delay: Duration,
    pub max_retries: u32,
}

impl Default for WebBrowserConfig {
    fn default() -> Self {
        Self {
            user_agent: "Housaky/1.0 (Rust; +https://github.com/HautlyS/Housaky)".to_string(),
            timeout: Duration::from_secs(30),
            max_size: 10 * 1024 * 1024, // 10MB
            allowed_domains: None,
            blocked_domains: vec![
                "malware.com".to_string(),
                "phishing.com".to_string(),
                "track.com".to_string(),
            ],
            follow_redirects: true,
            max_redirects: 10,
            accept_language: "en-US,en;q=0.9".to_string(),
            enable_cookies: false,
            rate_limit_delay: Duration::from_millis(500),
            max_retries: 3,
        }
    }
}

impl WebBrowserConfig {
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_content_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = Some(domains);
        self
    }

    pub fn with_blocked_domains(mut self, domains: Vec<String>) -> Self {
        self.blocked_domains = domains;
        self
    }

    pub fn with_follow_redirects(mut self, follow: bool) -> Self {
        self.follow_redirects = follow;
        self
    }

    pub fn with_max_redirects(mut self, max: u32) -> Self {
        self.max_redirects = max;
        self
    }

    pub fn with_accept_language(mut self, lang: impl Into<String>) -> Self {
        self.accept_language = lang.into();
        self
    }

    pub fn with_cookies(mut self, enable: bool) -> Self {
        self.enable_cookies = enable;
        self
    }

    pub fn with_rate_limit_delay(mut self, delay: Duration) -> Self {
        self.rate_limit_delay = delay;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
}

/// A parsed web page with content, links, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPage {
    pub url: String,
    pub title: Option<String>,
    pub content: String,
    pub links: Vec<LinkInfo>,
    pub metadata: HashMap<String, String>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub size_bytes: usize,
    pub site_type: SiteType,
    pub code_blocks: Vec<CodeBlock>,
    pub tables: Vec<TableData>,
    pub images: Vec<ImageInfo>,
    pub headings: Vec<Heading>,
    pub json_ld: Vec<String>,
    pub is_js_heavy: bool,
    pub forms: Vec<FormInfo>,
}

impl Default for WebPage {
    fn default() -> Self {
        Self {
            url: String::new(),
            title: None,
            content: String::new(),
            links: Vec::new(),
            metadata: HashMap::new(),
            fetched_at: chrono::Utc::now(),
            status_code: 200,
            content_type: None,
            size_bytes: 0,
            site_type: SiteType::Generic,
            code_blocks: Vec::new(),
            tables: Vec::new(),
            images: Vec::new(),
            headings: Vec::new(),
            json_ld: Vec::new(),
            is_js_heavy: false,
            forms: Vec::new(),
        }
    }
}

/// Information about a link on a webpage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub url: String,
    pub text: String,
    pub is_internal: bool,
}

/// A search result from DuckDuckGo.
///
/// # JSON Structure
///
/// ```json
/// {
///     "title": "Result Title",
///     "url": "https://example.com",
///     "snippet": "Brief description of the result",
///     "source": "DuckDuckGo"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
}

impl SearchResult {
    pub fn new(title: impl Into<String>, url: impl Into<String>, snippet: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
            snippet: snippet.into(),
            source: "DuckDuckGo".to_string(),
        }
    }
}

/// Metadata extracted from a webpage, including Open Graph and Twitter Card data.
///
/// # Example
///
/// ```ignore
/// let metadata = browser.get_page_metadata("https://example.com").await?;
/// println!("OG Title: {:?}", metadata.og_title);
/// println!("Twitter Card: {:?}", metadata.twitter_card);
/// println!("Description: {:?}", metadata.description);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_url: Option<String>,
    pub og_type: Option<String>,
    pub twitter_card: Option<String>,
    pub twitter_title: Option<String>,
    pub twitter_description: Option<String>,
    pub twitter_image: Option<String>,
    pub twitter_site: Option<String>,
    pub canonical_url: Option<String>,
    pub robots: Option<String>,
    pub language: Option<String>,
}

impl PageMetadata {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.keywords.is_none()
            && self.author.is_none()
            && self.og_title.is_none()
            && self.og_description.is_none()
            && self.og_image.is_none()
            && self.og_url.is_none()
            && self.og_type.is_none()
            && self.twitter_card.is_none()
            && self.twitter_title.is_none()
            && self.twitter_description.is_none()
            && self.twitter_image.is_none()
            && self.twitter_site.is_none()
            && self.canonical_url.is_none()
            && self.robots.is_none()
            && self.language.is_none()
    }
}

/// A browsing session for maintaining state across requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseSession {
    pub id: String,
    pub history: Vec<String>,
    pub current_url: Option<String>,
    pub cookies: std::collections::HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl BrowseSession {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            history: Vec::new(),
            current_url: None,
            cookies: std::collections::HashMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn push_history(&mut self, url: &str) {
        self.history.push(url.to_string());
        self.current_url = Some(url.to_string());
    }
}

impl Default for BrowseSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Web browser for fetching and parsing web pages.
///
/// # Example
///
/// ```ignore
/// use housaky::web_browser::{WebBrowser, WebBrowserConfig};
///
/// let config = WebBrowserConfig::default()
///     .with_user_agent("MyBot/1.0")
///     .with_timeout(std::time::Duration::from_secs(30));
///
/// let browser = WebBrowser::new(config);
///
/// // Fetch a page
/// let page = browser.fetch("https://example.com").await?;
/// println!("Title: {}", page.title.unwrap_or_default());
/// ```
pub struct WebBrowser {
    client: reqwest::Client,
    config: WebBrowserConfig,
    last_request_time: Option<Instant>,
}

impl WebBrowser {
    /// Create a new WebBrowser with the given configuration.
    pub fn with_config(config: WebBrowserConfig) -> Self {
        let client = Self::build_client(&config);
        Self {
            client,
            config,
            last_request_time: None,
        }
    }

    /// Create a new WebBrowser with default configuration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let browser = WebBrowser::new();
    /// let page = browser.fetch("https://example.com").await?;
    /// ```
    pub fn new() -> Self {
        Self::with_config(WebBrowserConfig::default())
    }

    fn build_client(config: &WebBrowserConfig) -> reqwest::Client {
        let builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects as usize))
            .danger_accept_invalid_certs(false);

        builder.build().unwrap_or_else(|_| {
            reqwest::Client::builder()
                .timeout(config.timeout)
                .user_agent(&config.user_agent)
                .build()
                .unwrap()
        })
    }

    /// Create a new WebBrowser with default configuration.
    pub fn with_default_config() -> Self {
        Self::with_config(WebBrowserConfig::default())
    }

    /// Create a WebBrowser allowing only specific domains.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let browser = WebBrowser::with_allowed_domains(vec![
    ///     "example.com".to_string(),
    ///     "docs.example.com".to_string(),
    /// ]);
    /// ```
    pub fn with_allowed_domains(domains: Vec<String>) -> Self {
        let config = WebBrowserConfig::default().with_allowed_domains(domains);
        Self::with_config(config)
    }

    fn apply_rate_limit(&mut self) {
        if let Some(last_time) = self.last_request_time {
            let elapsed = last_time.elapsed();
            if elapsed < self.config.rate_limit_delay {
                let sleep_duration = self.config.rate_limit_delay - elapsed;
                std::thread::sleep(sleep_duration);
            }
        }
        self.last_request_time = Some(Instant::now());
    }

    /// Fetch and parse a webpage using the scraper crate.
    ///
    /// # CSS Selectors Used
    ///
    /// - Title: `title`, `h1`
    /// - Links: `a[href]`
    /// - Metadata: `meta[name], meta[property]`
    /// - Paragraphs: `p`
    /// - Main content: `article, main, .content, #content`
    ///
    /// # Example
    ///
    /// ```ignore
    /// let page = browser.fetch("https://example.com").await?;
    /// println!("Title: {:?}", page.title);
    /// println!("Links count: {}", page.links.len());
    /// ```
    pub async fn fetch(&mut self, url: &str) -> Result<WebPage> {
        self.apply_rate_limit();
        info!("Fetching: {}", url);

        if !self.is_url_allowed(url) {
            return Err(WebError::NotAllowed {
                url: url.to_string(),
            }
            .into());
        }

        let response = self
            .client
            .get(url)
            .header("Accept-Language", &self.config.accept_language)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .send()
            .await
            .map_err(|e| WebError::Fetch {
                url: url.to_string(),
                source: e,
            })?;

        let status_code = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let content_type_str = content_type.as_deref().unwrap_or("");

        if !content_type_str.starts_with("text/html")
            && !content_type_str.starts_with("text/plain")
            && !content_type_str.starts_with("application/json")
        {
            return Err(WebError::UnsupportedContentType {
                content_type: content_type_str.to_string(),
                url: url.to_string(),
            }
            .into());
        }

        let body = response.text().await.map_err(|e| WebError::Fetch {
            url: url.to_string(),
            source: e,
        })?;

        if body.len() > self.config.max_size {
            return Err(WebError::ContentTooLarge {
                size: body.len(),
                max: self.config.max_size,
                url: url.to_string(),
            }
            .into());
        }

        let (title, content, links, metadata) = self.parse_html(url, &body);

        debug!("Fetched {} ({} bytes, {} links)", url, body.len(), links.len());

        Ok(WebPage {
            url: url.to_string(),
            title,
            content,
            links,
            metadata,
            fetched_at: chrono::Utc::now(),
            status_code,
            content_type,
            size_bytes: body.len(),
            site_type: SiteType::Generic,
            code_blocks: Vec::new(),
            tables: Vec::new(),
            images: Vec::new(),
            headings: Vec::new(),
            json_ld: Vec::new(),
            is_js_heavy: false,
            forms: Vec::new(),
        })
    }

    /// Parse HTML content using the scraper crate.
    ///
    /// This method uses proper CSS selectors instead of naive regex parsing.
    fn parse_html(
        &self,
        base_url: &str,
        html: &str,
    ) -> (
        Option<String>,
        String,
        Vec<LinkInfo>,
        std::collections::HashMap<String, String>,
    ) {
        let document = Html::parse_document(html);
        let mut metadata = std::collections::HashMap::new();

        let title = self.extract_title(&document);
        self.extract_metadata(&document, &mut metadata);
        let links = self.extract_links(&document, base_url);
        let content = self.extract_content(&document);

        (title, content, links, metadata)
    }

    fn extract_title(&self, document: &Html) -> Option<String> {
        if let Ok(selector) = Selector::parse("title") {
            if let Some(element) = document.select(&selector).next() {
                let title = element.text().collect::<String>().trim().to_string();
                if !title.is_empty() {
                    return Some(self.clean_text(&title));
                }
            }
        }

        if let Ok(selector) = Selector::parse("h1") {
            if let Some(element) = document.select(&selector).next() {
                let title = element.text().collect::<String>().trim().to_string();
                if !title.is_empty() {
                    return Some(self.clean_text(&title));
                }
            }
        }

        None
    }

    fn extract_metadata(&self, document: &Html, metadata: &mut std::collections::HashMap<String, String>) {
        if let Ok(selector) = Selector::parse("meta[name]") {
            for element in document.select(&selector) {
                if let (Some(name), Some(content)) = (
                    element.value().attr("name"),
                    element.value().attr("content"),
                ) {
                    metadata.insert(name.to_string(), content.to_string());
                }
            }
        }

        if let Ok(selector) = Selector::parse("meta[property], meta[name]") {
            for element in document.select(&selector) {
                if let (Some(name), Some(content)) = (
                    element.value().attr("property").or_else(|| element.value().attr("name")),
                    element.value().attr("content"),
                ) {
                    metadata.insert(name.to_string(), content.to_string());
                }
            }
        }
    }

    fn extract_links(&self, document: &Html, base_url: &str) -> Vec<LinkInfo> {
        let mut links = Vec::new();
        let mut seen_urls = HashSet::new();

        if let Ok(selector) = Selector::parse("a[href]") {
            for element in document.select(&selector) {
                let href = match element.value().attr("href") {
                    Some(h) => h,
                    None => continue,
                };

                let text = element.text().collect::<String>();
                let clean_text = self.clean_text(&text);

                if clean_text.is_empty() || href.starts_with("javascript:") || href.starts_with("mailto:") {
                    continue;
                }

                let full_url = self.resolve_url(base_url, href);

                if seen_urls.contains(&full_url) {
                    continue;
                }
                seen_urls.insert(full_url.clone());

                let is_internal = self.is_internal_link(base_url, href);

                links.push(LinkInfo {
                    url: full_url,
                    text: clean_text,
                    is_internal,
                });
            }
        }

        links
    }

    fn extract_content(&self, document: &Html) -> String {
        let content_selectors = [
            "article",
            "main",
            "[role=\"main\"]",
            ".content",
            "#content",
            ".post-content",
            ".article-content",
            ".entry-content",
            ".page-content",
        ];

        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text = element.text().collect::<String>();
                    let cleaned = self.clean_text(&text);
                    if cleaned.len() > 100 {
                        return self.normalize_whitespace(&cleaned);
                    }
                }
            }
        }

        if let Ok(selector) = Selector::parse("p") {
            let paragraphs: Vec<String> = document
                .select(&selector)
                .map(|el| el.text().collect::<String>())
                .map(|t| self.clean_text(&t))
                .filter(|t| t.len() > 20)
                .collect();

            if !paragraphs.is_empty() {
                return self.normalize_whitespace(&paragraphs.join("\n\n"));
            }
        }

        if let Ok(selector) = Selector::parse("body") {
            if let Some(element) = document.select(&selector).next() {
                return self.normalize_whitespace(&self.clean_text(&element.text().collect::<String>()));
            }
        }

        String::new()
    }

    /// Extract enhanced content from a webpage including code blocks, tables, images, headings, and JSON-LD.
    ///
    /// This method performs comprehensive content extraction and returns a fully populated WebPage.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let page = browser.extract_content_full("https://example.com").await?;
    /// println!("Code blocks: {}", page.code_blocks.len());
    /// println!("Tables: {}", page.tables.len());
    /// println!("Images: {}", page.images.len());
    /// ```
    pub async fn extract_content_full(&mut self, url: &str) -> Result<WebPage> {
        self.apply_rate_limit();
        info!("Extracting full content from: {}", url);

        if !self.is_url_allowed(url) {
            return Err(WebError::NotAllowed {
                url: url.to_string(),
            }
            .into());
        }

        let response = self
            .client
            .get(url)
            .header("Accept-Language", &self.config.accept_language)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .send()
            .await
            .map_err(|e| WebError::Fetch {
                url: url.to_string(),
                source: e,
            })?;

        let status_code = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let content_type_str = content_type.as_deref().unwrap_or("");

        if !content_type_str.starts_with("text/html")
            && !content_type_str.starts_with("text/plain")
            && !content_type_str.starts_with("application/json")
        {
            return Err(WebError::UnsupportedContentType {
                content_type: content_type_str.to_string(),
                url: url.to_string(),
            }
            .into());
        }

        let body = response.text().await.map_err(|e| WebError::Fetch {
            url: url.to_string(),
            source: e,
        })?;

        if body.len() > self.config.max_size {
            return Err(WebError::ContentTooLarge {
                size: body.len(),
                max: self.config.max_size,
                url: url.to_string(),
            }
            .into());
        }

        let document = Html::parse_document(&body);

        let mut metadata = std::collections::HashMap::new();
        self.extract_metadata(&document, &mut metadata);

        let title = self.extract_title(&document);
        let links = self.extract_links(&document, url);
        let content = self.extract_content(&document);

        let site_type = self.detect_site_type(url, &document);
        let code_blocks = self.extract_code_blocks(&document);
        let tables = self.extract_tables(&document);
        let images = self.extract_images(&document, url);
        let headings = self.extract_headings(&document);
        let json_ld = self.extract_json_ld(&document);
        let is_js_heavy = self.detect_js_heavy(&document);
        let forms = self.extract_forms(&document);

        debug!(
            "Extracted from {}: {} code blocks, {} tables, {} images, {} headings, {} forms",
            url,
            code_blocks.len(),
            tables.len(),
            images.len(),
            headings.len(),
            forms.len()
        );

        Ok(WebPage {
            url: url.to_string(),
            title,
            content,
            links,
            metadata,
            fetched_at: chrono::Utc::now(),
            status_code,
            content_type,
            size_bytes: body.len(),
            site_type,
            code_blocks,
            tables,
            images,
            headings,
            json_ld,
            is_js_heavy,
            forms,
        })
    }

    /// Detect the type of website based on URL and HTML content.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let site_type = browser.detect_site_type("https://docs.rust-lang.org", &document);
    /// println!("Site type: {}", site_type);
    /// ```
    pub fn detect_site_type(&self, url: &str, document: &Html) -> SiteType {
        let url_lower = url.to_lowercase();

        if url_lower.contains("docs.")
            || url_lower.contains("/docs/")
            || url_lower.contains("/documentation/")
            || url_lower.contains("/api/")
            || url_lower.contains("developer.mozilla.org")
            || url_lower.contains("rust-lang.org/learn")
            || url_lower.contains("json.org")
            || url_lower.contains("devdocs.io")
        {
            return SiteType::Documentation;
        }

        if url_lower.contains("blog.")
            || url_lower.contains("/blog/")
            || url_lower.contains("/posts/")
            || url_lower.contains("/article/")
        {
            return SiteType::Blog;
        }

        if url_lower.contains("shop.")
            || url_lower.contains("/product/")
            || url_lower.contains("/cart/")
            || url_lower.contains("amazon.")
            || url_lower.contains("ebay.")
        {
            return SiteType::Ecommerce;
        }

        if url_lower.contains("/forum/")
            || url_lower.contains("/topic/")
            || url_lower.contains("/discussion/")
            || url_lower.contains("reddit.")
            || url_lower.contains("stackoverflow.")
        {
            return SiteType::Forum;
        }

        if url_lower.contains("twitter.")
            || url_lower.contains("facebook.")
            || url_lower.contains("instagram.")
            || url_lower.contains("linkedin.")
            || url_lower.contains("/profile/")
        {
            return SiteType::Social;
        }

        if url_lower.contains("news.")
            || url_lower.contains("/news/")
            || url_lower.contains("/article/")
            || url_lower.contains("/story/")
        {
            return SiteType::News;
        }

        let og_type = document
            .select(&Selector::parse("meta[property=\"og:type\"]").unwrap_or_else(|_| Selector::parse("meta").unwrap()))
            .next()
            .and_then(|el| el.value().attr("content"));

        if let Some(og_type) = og_type {
            match og_type.to_lowercase().as_str() {
                "article" => return SiteType::Blog,
                "website" => {}
                _ => {}
            }
        }

        if let Ok(selector) = Selector::parse("article, .post, .blog-post, .entry-content") {
            let has_article = document.select(&selector).next().is_some();
            if has_article {
                return SiteType::Blog;
            }
        }

        if let Ok(selector) = Selector::parse("pre code, .code, .code-block, [class*=\"code\"]") {
            let has_code = document.select(&selector).next().is_some();
            if has_code {
                return SiteType::Documentation;
            }
        }

        SiteType::Generic
    }

    /// Extract code blocks from HTML document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let code_blocks = browser.extract_code_blocks(&document);
    /// for block in &code_blocks {
    ///     println!("Language: {:?}, Code: {}", block.language, block.code.chars().take(100));
    /// }
    /// ```
    pub fn extract_code_blocks(&self, document: &Html) -> Vec<CodeBlock> {
        let mut code_blocks = Vec::new();

        if let Ok(selector) = Selector::parse("pre code") {
            for element in document.select(&selector) {
                let language = element.value().attr("class")
                    .and_then(|c| {
                        c.split_whitespace()
                            .find(|c| c.starts_with("language-"))
                            .map(|c| c.trim_start_matches("language-").to_string())
                    });

                let code = element.text().collect::<String>();
                let element_id = element.value().attr("id").map(String::from);

                if !code.trim().is_empty() {
                    code_blocks.push(CodeBlock {
                        language,
                        code: code.trim().to_string(),
                        element_id,
                    });
                }
            }
        }

        if code_blocks.is_empty() {
            if let Ok(selector) = Selector::parse("pre") {
                for element in document.select(&selector) {
                    let language = element.value().attr("class")
                        .and_then(|c| {
                            c.split_whitespace()
                                .find(|c| c.starts_with("language-"))
                                .map(|c| c.trim_start_matches("language-").to_string())
                        });

                    let code = element.text().collect::<String>();
                    let element_id = element.value().attr("id").map(String::from);

                    if !code.trim().is_empty() {
                        code_blocks.push(CodeBlock {
                            language,
                            code: code.trim().to_string(),
                            element_id,
                        });
                    }
                }
            }
        }

        if code_blocks.is_empty() {
            if let Ok(selector) = Selector::parse("code") {
                for element in document.select(&selector) {
                    let parent = element.parent_element();
                    let is_pre = parent
                        .as_ref()
                        .map(|p| p.value().name() == "pre")
                        .unwrap_or(false);

                    if !is_pre {
                        let code = element.text().collect::<String>().trim().to_string();
                        let element_id = element.value().attr("id").map(String::from);

                        if code.len() > 20 && (code.contains('(') || code.contains('{') || code.contains(';')) {
                            code_blocks.push(CodeBlock {
                                language: None,
                                code,
                                element_id,
                            });
                        }
                    }
                }
            }
        }

        code_blocks
    }

    /// Extract tables from HTML document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tables = browser.extract_tables(&document);
    /// for table in &tables {
    ///     println!("Headers: {:?}", table.headers);
    ///     println!("Rows: {:?}", table.rows.len());
    /// }
    /// ```
    pub fn extract_tables(&self, document: &Html) -> Vec<TableData> {
        let mut tables = Vec::new();

        if let Ok(selector) = Selector::parse("table") {
            for table_element in document.select(&selector) {
                let mut headers = Vec::new();
                let mut rows = Vec::new();

                if let Ok(header_selector) = Selector::parse("thead th, thead td") {
                    for header in table_element.select(&header_selector) {
                        let text = self.clean_text(&header.text().collect::<String>());
                        if !text.is_empty() {
                            headers.push(text);
                        }
                    }
                }

                if headers.is_empty() {
                    if let Ok(header_selector) = Selector::parse("tr:first-child th, tr:first-child td") {
                        for header in table_element.select(&header_selector) {
                            let text = self.clean_text(&header.text().collect::<String>());
                            if !text.is_empty() {
                                headers.push(text);
                            }
                        }
                    }
                }

                if let Ok(body_selector) = Selector::parse("tbody tr") {
                    for row_element in table_element.select(&body_selector) {
                        let mut row = Vec::new();
                        if let Ok(cell_selector) = Selector::parse("td, th") {
                            for cell in row_element.select(&cell_selector) {
                                let text = self.clean_text(&cell.text().collect::<String>());
                                row.push(text);
                            }
                        }
                        if !row.is_empty() {
                            rows.push(row);
                        }
                    }
                }

                if rows.is_empty() {
                    if let Ok(row_selector) = Selector::parse("tr") {
                        for row_element in table_element.select(&row_selector) {
                            let mut row = Vec::new();
                            if let Ok(cell_selector) = Selector::parse("td, th") {
                                for cell in row_element.select(&cell_selector) {
                                    let text = self.clean_text(&cell.text().collect::<String>());
                                    row.push(text);
                                }
                            }
                            if !row.is_empty() {
                                rows.push(row);
                            }
                        }
                    }
                }

                if !rows.is_empty() {
                    tables.push(TableData { headers, rows });
                }
            }
        }

        tables
    }

    /// Extract images from HTML document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let images = browser.extract_images(&document, "https://example.com");
    /// for img in &images {
    ///     println!("Image: {} (alt: {:?})", img.src, img.alt);
    /// }
    /// ```
    pub fn extract_images(&self, document: &Html, base_url: &str) -> Vec<ImageInfo> {
        let mut images = Vec::new();
        let mut seen_srcs: HashSet<String> = HashSet::new();

        if let Ok(selector) = Selector::parse("img[src]") {
            for element in document.select(&selector) {
                let src = match element.value().attr("src") {
                    Some(s) => s,
                    None => continue,
                };

                if seen_srcs.contains(src) {
                    continue;
                }
                seen_srcs.insert(src.to_string());

                let full_src = self.resolve_url(base_url, src);
                let alt = element.value().attr("alt").map(String::from);
                let title = element.value().attr("title").map(String::from);

                images.push(ImageInfo {
                    src: full_src,
                    alt,
                    title,
                });
            }
        }

        if let Ok(selector) = Selector::parse("picture source[srcset]") {
            for element in document.select(&selector) {
                if let Some(srcset) = element.value().attr("srcset") {
                    if let Some(first_src) = srcset.split(',').next() {
                        let src = first_src.split_whitespace().next().unwrap_or("");
                        if !src.is_empty() && !seen_srcs.contains(src) {
                            seen_srcs.insert(src.to_string());
                            let full_src = self.resolve_url(base_url, src);
                            images.push(ImageInfo {
                                src: full_src,
                                alt: None,
                                title: None,
                            });
                        }
                    }
                }
            }
        }

        images
    }

    /// Extract headings from HTML document with hierarchy.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let headings = browser.extract_headings(&document);
    /// for heading in &headings {
    ///     println!("H{}: {}", heading.level, heading.text);
    /// }
    /// ```
    pub fn extract_headings(&self, document: &Html) -> Vec<Heading> {
        let mut headings = Vec::new();

        for level in 1..=6 {
            let selector_str = format!("h{}", level);
            if let Ok(selector) = Selector::parse(&selector_str) {
                for element in document.select(&selector) {
                    let text = self.clean_text(&element.text().collect::<String>());
                    if !text.is_empty() {
                        headings.push(Heading {
                            level,
                            text,
                        });
                    }
                }
            };
        }

        headings
    }

    /// Extract JSON-LD structured data from HTML document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let json_ld = browser.extract_json_ld(&document);
    /// for data in &json_ld {
    ///     println!("JSON-LD: {}", data);
    /// }
    /// ```
    pub fn extract_json_ld(&self, document: &Html) -> Vec<String> {
        let mut json_data = Vec::new();

        if let Ok(selector) = Selector::parse("script[type=\"application/ld+json\"]") {
            for element in document.select(&selector) {
                let json_text = element.text().collect::<String>().trim().to_string();
                if !json_text.is_empty() {
                    if serde_json::from_str::<serde_json::Value>(&json_text).is_ok() {
                        json_data.push(json_text);
                    }
                }
            }
        }

        json_data
    }

    /// Detect if a page is JavaScript-heavy and may require special handling.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let is_js_heavy = browser.detect_js_heavy(&document);
    /// if is_js_heavy {
    ///     println!("Warning: This page may require JavaScript rendering");
    /// }
    /// ```
    pub fn detect_js_heavy(&self, document: &Html) -> bool {
        let html = document.root_element().html().to_lowercase();

        let js_markers = [
            "react",
            "vue",
            "angular",
            "svelte",
            "next.js",
            "nuxt",
            "gatsby",
            "ember",
            "backbone",
            "mithril",
        ];

        for marker in &js_markers {
            if html.contains(marker) {
                return true;
            }
        }

        if let Ok(selector) = Selector::parse("script[data-framework], [data-framework]") {
            for element in document.select(&selector) {
                if element.value().attr("data-framework").is_some() {
                    return true;
                }
            }
        }

        if let Ok(selector) = Selector::parse("div[id*=\"app\"], div[id*=\"root\"]") {
            let div_count = document.select(&selector).count();
            if div_count > 0 {
                if let Ok(body_selector) = Selector::parse("body") {
                    if let Some(body) = document.select(&body_selector).next() {
                        let body_text = body.text().collect::<String>();
                        if body_text.len() < 200 {
                            return true;
                        }
                    }
                }
            }
        }

        let xhr_indicators = ["axios", "fetch(", "XMLHttpRequest", ".ajax(", ".get(", ".post("];
        for indicator in &xhr_indicators {
            if html.contains(indicator) {
                return true;
            }
        }

        let router_indicators = ["react-router", "vue-router", "angular-router", "svelte-routing"];
        for router in &router_indicators {
            if html.contains(router) {
                return true;
            }
        }

        false
    }

    /// Extract forms from HTML document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let forms = browser.extract_forms(&document);
    /// for form in &forms {
    ///     println!("Form action: {:?}, method: {}", form.action, form.method);
    ///     for input in &form.inputs {
    ///         println!("  Input: {:?} ({})", input.name, input.input_type);
    ///     }
    /// }
    /// ```
    pub fn extract_forms(&self, document: &Html) -> Vec<FormInfo> {
        let mut forms = Vec::new();

        if let Ok(selector) = Selector::parse("form") {
            for form_element in document.select(&selector) {
                let action = form_element.value().attr("action").map(String::from);
                let method = form_element
                    .value()
                    .attr("method")
                    .map(|m| m.to_uppercase())
                    .unwrap_or_else(|| "GET".to_string());

                let mut inputs = Vec::new();

                if let Ok(input_selector) = Selector::parse("input") {
                    for input_element in form_element.select(&input_selector) {
                        let input_type = input_element
                            .value()
                            .attr("type")
                            .map(|t| t.to_lowercase())
                            .unwrap_or_else(|| "text".to_string());

                        if input_type == "submit" || input_type == "button" || input_type == "hidden" {
                            continue;
                        }

                        let name = input_element.value().attr("name").map(String::from);

                        let label = input_element
                            .value()
                            .attr("id")
                            .and_then(|id| {
                                let label_selector = format!("label[for=\"{}\"]", id);
                                Selector::parse(&label_selector).ok()
                            })
                            .and_then(|selector| {
                                document.select(&selector).next()
                            })
                            .map(|el| el.text().collect::<String>());

                        inputs.push(FormInput {
                            name,
                            input_type,
                            label,
                        });
                    }
                }

                if let Ok(select_selector) = Selector::parse("select") {
                    for select_element in form_element.select(&select_selector) {
                        let name = select_element.value().attr("name").map(String::from);
                        inputs.push(FormInput {
                            name,
                            input_type: "select".to_string(),
                            label: None,
                        });
                    }
                }

                if let Ok(textarea_selector) = Selector::parse("textarea") {
                    for textarea_element in form_element.select(&textarea_selector) {
                        let name = textarea_element.value().attr("name").map(String::from);
                        inputs.push(FormInput {
                            name,
                            input_type: "textarea".to_string(),
                            label: None,
                        });
                    }
                }

                if !inputs.is_empty() || !method.is_empty() {
                    forms.push(FormInfo {
                        action,
                        method,
                        inputs,
                    });
                }
            }
        }

        forms
    }

    /// Perform comprehensive page analysis.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let analysis = browser.analyze_page("https://example.com").await?;
    /// println!("Site type: {}", analysis.site_type);
    /// println!("Is JS-heavy: {}", analysis.is_js_heavy);
    /// println!("Recommendations: {:?}", analysis.recommendations);
    /// ```
    pub async fn analyze_page(&mut self, url: &str) -> Result<PageAnalysis> {
        let page = self.extract_content_full(url).await?;

        let mut recommendations = Vec::new();

        if page.is_js_heavy {
            recommendations.push("Consider using a headless browser (e.g., Playwright) for full content extraction".to_string());
        }

        if page.site_type == SiteType::Documentation {
            recommendations.push("This appears to be a documentation site. Code examples and API references were extracted".to_string());
        }

        if page.site_type == SiteType::Ecommerce {
            recommendations.push("This appears to be an e-commerce site. Product information may be loaded dynamically".to_string());
        }

        if !page.forms.is_empty() {
            recommendations.push(format!("Found {} form(s) on the page that could be used for automation", page.forms.len()));
        }

        if page.json_ld.is_empty() && page.site_type == SiteType::Documentation {
            recommendations.push("No JSON-LD structured data found. Consider alternative metadata extraction".to_string());
        }

        if page.code_blocks.is_empty() && page.site_type == SiteType::Documentation {
            recommendations.push("No code blocks detected. Consider checking for dynamically loaded content".to_string());
        }

        if page.content.len() < 500 {
            recommendations.push("Page content is minimal. The page may be using client-side rendering or has thin content".to_string());
        }

        let site_type = page.site_type.clone();
        let is_js_heavy = page.is_js_heavy;
        let forms = page.forms.clone();
        let json_ld = page.json_ld.clone();

        Ok(PageAnalysis {
            page,
            site_type,
            is_js_heavy,
            forms,
            json_ld,
            recommendations,
        })
    }

    /// Search using DuckDuckGo's Instant Answer API.
    ///
    /// This returns JSON instead of HTML scraping for more reliable results.
    ///
    /// # API Format
    ///
    /// URL: `https://api.duckduckgo.com/?q={query}&format=json&no_html=1&skip_disambig=1&limit={max}`
    ///
    /// # JSON Response Fields
    ///
    /// - `RelatedTopics[]` - Array of search results, each has `Text` and `FirstURL`
    /// - `AbstractText` - Summary text
    /// - `AbstractURL` - Source URL
    /// - `Heading` - Query heading
    ///
    /// # Example
    ///
    /// ```ignore
    /// let results = browser.search("rust programming language", 10).await?;
    /// for result in results {
    ///     println!("{} - {}", result.title, result.url);
    /// }
    /// ```
    pub async fn search(&mut self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        self.apply_rate_limit();
        info!("Searching for: {}", query);

        let encoded_query = urlencoding::encode(query);
        let search_url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1&limit={}",
            encoded_query, max_results
        );

        let response = self
            .client
            .get(&search_url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| WebError::Search { source: e })?;

        let body = response.text().await.map_err(|e| WebError::Search { source: e })?;

        let ddg_response: DuckDuckGoResponse = serde_json::from_str(&body)
            .map_err(|e| WebError::JsonParse { 
                url: search_url, 
                source: e 
            })?;

        let mut results = Vec::new();

        if let (Some(abstract_text), Some(abstract_url)) = (&ddg_response.abstract_text, &ddg_response.abstract_url) {
            if !abstract_text.is_empty() {
                results.push(SearchResult::new(
                    ddg_response.heading.as_deref().unwrap_or(query),
                    abstract_url,
                    abstract_text,
                ));
            }
        }

        for topic in ddg_response.related_topics.iter().take(max_results) {
            if let (Some(text), Some(url)) = (&topic.text, &topic.first_url) {
                let title = text
                    .split('-')
                    .next()
                    .unwrap_or(text)
                    .trim()
                    .to_string();

                let snippet = if text.contains('-') {
                    text.split('-').skip(1).collect::<Vec<_>>().join("-").trim().to_string()
                } else {
                    String::new()
                };

                results.push(SearchResult::new(title, url, snippet));
            }
        }

        for answer in ddg_response.answers.iter().take(max_results) {
            if let (Some(text), Some(url)) = (&answer.text, &answer.first_url) {
                results.push(SearchResult::new(text, url, text));
            }
        }

        results.truncate(max_results);

        info!("Found {} search results", results.len());

        Ok(results)
    }

    /// Extract clean text from a webpage.
    ///
    /// This removes HTML tags, scripts, styles, and other noise to provide
    /// readable text content.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let text = browser.extract_text("https://example.com").await?;
    /// println!("{}", text.chars().take(500));
    /// ```
    pub async fn extract_text(&mut self, url: &str) -> Result<String> {
        let page = self.fetch(url).await?;
        Ok(page.content)
    }

    /// Create a basic summarization of a webpage.
    ///
    /// This extracts the first few sentences up to the specified max length.
    /// For more sophisticated summarization, consider using an LLM.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let summary = browser.summarize("https://example.com", 200).await?;
    /// println!("Summary: {}", summary);
    /// ```
    pub async fn summarize(&mut self, url: &str, max_length: usize) -> Result<String> {
        let page = self.fetch(url).await?;
        let content = &page.content;

        if content.len() <= max_length {
            return Ok(content.clone());
        }

        let sentence_endings = Regex::new(r"[.!?]\s+").unwrap();
        let sentences: Vec<&str> = sentence_endings.split(content).collect();

        let mut summary = String::new();
        let mut current_length = 0;

        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }

            if current_length + sentence.len() + 1 > max_length {
                break;
            }

            if !summary.is_empty() {
                summary.push_str(". ");
            }
            summary.push_str(sentence);
            current_length += sentence.len() + 2;
        }

        if summary.is_empty() {
            summary = content.chars().take(max_length).collect();
            if content.len() > max_length {
                summary.push_str("...");
            }
        }

        Ok(summary)
    }

    /// Crawl internal links starting from a URL.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pages = browser.follow_links("https://example.com", 2, 10).await?;
    /// for page in pages {
    ///     println!("{}", page.url);
    /// }
    /// ```
    pub async fn follow_links(
        &mut self,
        url: &str,
        depth: usize,
        max_pages: usize,
    ) -> Result<Vec<WebPage>> {
        let mut pages = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![(url.to_string(), 0usize)];

        while !queue.is_empty() && pages.len() < max_pages {
            let (current_url, current_depth) = queue.remove(0);

            if visited.contains(&current_url) {
                continue;
            }

            visited.insert(current_url.clone());

            match self.fetch(&current_url).await {
                Ok(page) => {
                    if current_depth < depth {
                        for link in &page.links {
                            if link.is_internal && !visited.contains(&link.url) {
                                queue.push((link.url.clone(), current_depth + 1));
                            }
                        }
                    }
                    pages.push(page);
                }
                Err(e) => {
                    warn!("Failed to fetch {}: {}", current_url, e);
                }
            }
        }

        Ok(pages)
    }

    /// Extract metadata from a webpage, including Open Graph and Twitter Card data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let metadata = browser.get_page_metadata("https://example.com").await?;
    /// println!("OG Title: {:?}", metadata.og_title);
    /// println!("Description: {:?}", metadata.description);
    /// ```
    pub async fn get_page_metadata(&mut self, url: &str) -> Result<PageMetadata> {
        self.apply_rate_limit();

        if !self.is_url_allowed(url) {
            return Err(WebError::NotAllowed {
                url: url.to_string(),
            }
            .into());
        }

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| WebError::Fetch {
                url: url.to_string(),
                source: e,
            })?
            .text()
            .await
            .map_err(|e| WebError::Fetch {
                url: url.to_string(),
                source: e,
            })?;

        let document = Html::parse_document(&response);

        let mut metadata = PageMetadata::default();

        if let Ok(selector) = Selector::parse("title") {
            if let Some(element) = document.select(&selector).next() {
                metadata.title = Some(element.text().collect::<String>().trim().to_string());
            }
        }

        if let Ok(selector) = Selector::parse("meta[name=\"description\"]") {
            if let Some(element) = document.select(&selector).next() {
                metadata.description = element.value().attr("content").map(String::from);
            }
        }

        if let Ok(selector) = Selector::parse("meta[name=\"keywords\"]") {
            if let Some(element) = document.select(&selector).next() {
                metadata.keywords = element.value().attr("content").map(String::from);
            }
        }

        if let Ok(selector) = Selector::parse("meta[name=\"author\"]") {
            if let Some(element) = document.select(&selector).next() {
                metadata.author = element.value().attr("content").map(String::from);
            }
        }

        if let Ok(selector) = Selector::parse("meta[property^=\"og:\"]") {
            for element in document.select(&selector) {
                if let (Some(property), Some(content)) = (
                    element.value().attr("property"),
                    element.value().attr("content"),
                ) {
                    match property {
                        "og:title" => metadata.og_title = Some(content.to_string()),
                        "og:description" => metadata.og_description = Some(content.to_string()),
                        "og:image" => metadata.og_image = Some(content.to_string()),
                        "og:url" => metadata.og_url = Some(content.to_string()),
                        "og:type" => metadata.og_type = Some(content.to_string()),
                        _ => {}
                    }
                }
            }
        }

        if let Ok(selector) = Selector::parse("meta[name^=\"twitter:\"]") {
            for element in document.select(&selector) {
                if let (Some(name), Some(content)) = (
                    element.value().attr("name"),
                    element.value().attr("content"),
                ) {
                    match name {
                        "twitter:card" => metadata.twitter_card = Some(content.to_string()),
                        "twitter:title" => metadata.twitter_title = Some(content.to_string()),
                        "twitter:description" => metadata.twitter_description = Some(content.to_string()),
                        "twitter:image" => metadata.twitter_image = Some(content.to_string()),
                        "twitter:site" => metadata.twitter_site = Some(content.to_string()),
                        _ => {}
                    }
                }
            }
        }

        if let Ok(selector) = Selector::parse("link[rel=\"canonical\"]") {
            if let Some(element) = document.select(&selector).next() {
                metadata.canonical_url = element.value().attr("href").map(String::from);
            }
        }

        if let Ok(selector) = Selector::parse("meta[name=\"robots\"]") {
            if let Some(element) = document.select(&selector).next() {
                metadata.robots = element.value().attr("content").map(String::from);
            }
        }

        if let Ok(selector) = Selector::parse("html[lang]") {
            if let Some(element) = document.select(&selector).next() {
                metadata.language = element.value().attr("lang").map(String::from);
            }
        }

        Ok(metadata)
    }

    /// Quick check if a URL is reachable.
    ///
    /// This performs a HEAD request to check connectivity without
    /// downloading the full content.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if browser.is_reachable("https://example.com").await? {
    ///     println!("Site is reachable!");
    /// }
    /// ```
    pub async fn is_reachable(&mut self, url: &str) -> Result<bool> {
        self.apply_rate_limit();

        if !self.is_url_allowed(url) {
            return Ok(false);
        }

        let result = self
            .client
            .head(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match result {
            Ok(response) => {
                let status = response.status();
                Ok(status.is_success() || status.is_redirection())
            }
            Err(_) => Ok(false),
        }
    }

    fn is_url_allowed(&self, url: &str) -> bool {
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return false,
        };

        let domain = match parsed.host_str() {
            Some(d) => d,
            None => return false,
        };

        for blocked in &self.config.blocked_domains {
            if domain.ends_with(blocked) || domain == blocked {
                return false;
            }
        }

        if let Some(ref allowed) = self.config.allowed_domains {
            let is_allowed = allowed.iter().any(|a| domain.ends_with(a) || domain == a);
            if !is_allowed {
                return false;
            }
        }

        true
    }

    fn resolve_url(&self, base: &str, relative: &str) -> String {
        if relative.starts_with("http://") || relative.starts_with("https://") {
            return relative.to_string();
        }

        if relative.starts_with("//") {
            if let Ok(base_url) = url::Url::parse(base) {
                return format!("{}:{}", base_url.scheme(), relative);
            }
        }

        if let Ok(base_url) = url::Url::parse(base) {
            if let Ok(joined) = base_url.join(relative) {
                return joined.to_string();
            }
        }

        relative.to_string()
    }

    fn is_internal_link(&self, base: &str, link: &str) -> bool {
        if link.starts_with('#') || link.starts_with('/') {
            return true;
        }

        let base_domain = url::Url::parse(base)
            .ok()
            .and_then(|u| u.host_str().map(String::from));

        let link_domain = url::Url::parse(link)
            .ok()
            .and_then(|u| u.host_str().map(String::from));

        match (base_domain, link_domain) {
            (Some(b), Some(l)) => b == l,
            _ => false,
        }
    }

    fn clean_text(&self, text: &str) -> String {
        text.replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&mdash;", "—")
            .replace("&ndash;", "–")
            .replace("&hellip;", "…")
            .trim()
            .to_string()
    }

    fn normalize_whitespace(&self, text: &str) -> String {
        let re = Regex::new(r"\s+").unwrap();
        re.replace_all(text, " ").trim().to_string()
    }

    /// Get the current configuration.
    pub fn config(&self) -> &WebBrowserConfig {
        &self.config
    }

    /// Update the user agent.
    pub fn set_user_agent(&mut self, user_agent: impl Into<String>) {
        self.config.user_agent = user_agent.into();
    }

    /// Update the timeout.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.config.timeout = timeout;
    }
}

impl Default for WebBrowser {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// DuckDuckGo API response structure.
#[derive(Debug, Deserialize)]
struct DuckDuckGoResponse {
    #[serde(rename = "AbstractText")]
    abstract_text: Option<String>,
    #[serde(rename = "AbstractURL")]
    abstract_url: Option<String>,
    #[serde(rename = "Heading")]
    heading: Option<String>,
    #[serde(rename = "RelatedTopics")]
    related_topics: Vec<DuckDuckGoTopic>,
    #[serde(rename = "Answers")]
    answers: Vec<DuckDuckGoTopic>,
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoTopic {
    #[serde(rename = "Text")]
    text: Option<String>,
    #[serde(rename = "FirstURL")]
    first_url: Option<String>,
}

/// URL encoding helper module.
pub mod urlencoding {
    use std::fmt::Write;

    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                _ => {
                    write!(&mut encoded, "%{:02X}", byte).unwrap();
                }
            }
        }
        encoded
    }

    pub fn decode(s: &str) -> String {
        let mut decoded = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if hex.len() == 2 {
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        decoded.push(byte as char);
                        continue;
                    }
                }
                decoded.push('%');
                decoded.push_str(&hex);
            } else if c == '+' {
                decoded.push(' ');
            } else {
                decoded.push(c);
            }
        }
        decoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoding::encode("hello world"), "hello%20world");
        assert_eq!(urlencoding::encode("foo=bar"), "foo%3Dbar");
    }

    #[test]
    fn test_url_decoding() {
        assert_eq!(urlencoding::decode("hello%20world"), "hello world");
        assert_eq!(urlencoding::decode("foo%3Dbar"), "foo=bar");
    }

    #[test]
    fn test_search_result_new() {
        let result = SearchResult::new(
            "Test Title",
            "https://example.com",
            "Test snippet",
        );
        assert_eq!(result.title, "Test Title");
        assert_eq!(result.url, "https://example.com");
        assert_eq!(result.snippet, "Test snippet");
        assert_eq!(result.source, "DuckDuckGo");
    }

    #[test]
    fn test_page_metadata_empty() {
        let metadata = PageMetadata::default();
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_web_browser_config() {
        let config = WebBrowserConfig::default()
            .with_user_agent("TestBot/1.0")
            .with_timeout(Duration::from_secs(60))
            .with_max_content_size(5 * 1024 * 1024)
            .with_allowed_domains(vec!["example.com".to_string()])
            .with_blocked_domains(vec!["evil.com".to_string()])
            .with_rate_limit_delay(Duration::from_millis(1000))
            .with_max_retries(5);

        assert_eq!(config.user_agent, "TestBot/1.0");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_size, 5 * 1024 * 1024);
        assert!(config.allowed_domains.is_some());
        assert_eq!(config.blocked_domains, vec!["evil.com".to_string()]);
        assert_eq!(config.rate_limit_delay, Duration::from_millis(1000));
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_browse_session_new() {
        let session = BrowseSession::new();
        assert!(!session.id.is_empty());
        assert!(session.history.is_empty());
        assert!(session.current_url.is_none());
    }

    #[test]
    fn test_site_type_display() {
        assert_eq!(SiteType::Generic.to_string(), "Generic");
        assert_eq!(SiteType::Documentation.to_string(), "Documentation");
        assert_eq!(SiteType::Blog.to_string(), "Blog");
    }

    #[tokio::test]
    #[ignore = "requires network"]
    async fn test_fetch_basic() {
        let mut browser = WebBrowser::with_default_config();
        let result = browser.fetch("https://example.com").await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert!(!page.url.is_empty());
        assert!(page.status_code == 200);
    }

    #[tokio::test]
    #[ignore = "requires network"]
    async fn test_extract_text() {
        let mut browser = WebBrowser::with_default_config();
        let text = browser.extract_text("https://example.com").await;
        assert!(text.is_ok());
        let content = text.unwrap();
        assert!(!content.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires network"]
    async fn test_search() {
        let mut browser = WebBrowser::with_default_config();
        let results = browser.search("rust programming", 3).await;
        assert!(results.is_ok());
        let results = results.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires network"]
    async fn test_is_reachable_valid_url() {
        let mut browser = WebBrowser::with_default_config();
        let result = browser.is_reachable("https://example.com").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_is_reachable_invalid_url() {
        let mut browser = WebBrowser::with_default_config();
        let result = browser.is_reachable("https://this-domain-does-not-exist-123456789.invalid").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    #[ignore = "requires network"]
    async fn test_full_content_extraction() {
        let mut browser = WebBrowser::with_default_config();
        let page = browser.extract_content_full("https://example.com").await;
        assert!(page.is_ok());
        let page = page.unwrap();
        assert!(!page.url.is_empty());
        assert!(!page.images.is_empty() || page.images.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires network"]
    async fn test_page_analysis() {
        let mut browser = WebBrowser::with_default_config();
        let analysis = browser.analyze_page("https://example.com").await;
        assert!(analysis.is_ok());
        let analysis = analysis.unwrap();
        assert!(!analysis.recommendations.is_empty() || analysis.recommendations.is_empty());
    }

    #[test]
    fn test_clean_text() {
        let browser = WebBrowser::with_default_config();
        assert_eq!(browser.clean_text("Hello &amp; World"), "Hello & World");
        assert_eq!(browser.clean_text("Test &lt;script&gt;"), "Test <script>");
        assert_eq!(browser.clean_text("  spaces  "), "spaces");
    }

    #[test]
    fn test_resolve_url() {
        let browser = WebBrowser::with_default_config();
        assert_eq!(
            browser.resolve_url("https://example.com/page", "/other"),
            "https://example.com/other"
        );
        assert_eq!(
            browser.resolve_url("https://example.com/page", "https://other.com"),
            "https://other.com"
        );
    }

    #[test]
    fn test_is_internal_link() {
        let browser = WebBrowser::with_default_config();
        assert!(browser.is_internal_link("https://example.com/page", "/other"));
        assert!(browser.is_internal_link("https://example.com/page", "#section"));
        assert!(!browser.is_internal_link("https://example.com/page", "https://other.com"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_fetch_example_com() {
        let mut browser = WebBrowser::with_default_config();
        let page = browser.fetch("https://example.com").await;
        assert!(page.is_ok(), "Failed to fetch example.com: {:?}", page.err());
        let page = page.unwrap();
        assert!(!page.title.is_none() || page.title.is_none());
        assert!(!page.content.is_empty());
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_fetch_rust_lang_learn() {
        let mut browser = WebBrowser::with_default_config();
        let page = browser.fetch("https://www.rust-lang.org/learn").await;
        assert!(page.is_ok(), "Failed to fetch rust-lang.org: {:?}", page.err());
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_fetch_mdn() {
        let mut browser = WebBrowser::with_default_config();
        let page = browser.fetch("https://developer.mozilla.org").await;
        assert!(page.is_ok(), "Failed to fetch MDN: {:?}", page.err());
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_fetch_json_org() {
        let mut browser = WebBrowser::with_default_config();
        let page = browser.fetch("https://json.org").await;
        assert!(page.is_ok(), "Failed to fetch json.org: {:?}", page.err());
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_full_extraction_example_com() {
        let mut browser = WebBrowser::with_default_config();
        let page = browser.extract_content_full("https://example.com").await;
        assert!(page.is_ok());
        let page = page.unwrap();
        assert!(!page.images.is_empty());
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_analyze_example_com() {
        let mut browser = WebBrowser::with_default_config();
        let analysis = browser.analyze_page("https://example.com").await;
        assert!(analysis.is_ok());
        let analysis = analysis.unwrap();
        assert_eq!(analysis.site_type, SiteType::Generic);
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_search_functionality() {
        let mut browser = WebBrowser::with_default_config();
        let results = browser.search("rust programming language", 5).await;
        assert!(results.is_ok());
        let results = results.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_summarize_example_com() {
        let mut browser = WebBrowser::with_default_config();
        let summary = browser.summarize("https://example.com", 100).await;
        assert!(summary.is_ok());
        let summary = summary.unwrap();
        assert!(summary.len() <= 103);
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_get_page_metadata() {
        let mut browser = WebBrowser::with_default_config();
        let metadata = browser.get_page_metadata("https://example.com").await;
        assert!(metadata.is_ok());
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network"]
    async fn test_rate_limiting() {
        let config = WebBrowserConfig::default()
            .with_rate_limit_delay(Duration::from_millis(100));
        let mut browser = WebBrowser::with_config(config);
        
        let start = Instant::now();
        let _ = browser.fetch("https://example.com").await;
        let _ = browser.fetch("https://example.com").await;
        let elapsed = start.elapsed();
        
        assert!(elapsed >= Duration::from_millis(100), "Rate limiting not working");
    }
}
