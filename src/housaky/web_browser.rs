use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub url: String,
    pub text: String,
    pub is_internal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseSession {
    pub id: String,
    pub history: Vec<String>,
    pub current_url: Option<String>,
    pub cookies: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct WebBrowser {
    client: reqwest::Client,
    user_agent: String,
    timeout: Duration,
    max_size: usize,
    allowed_domains: Option<Vec<String>>,
    blocked_domains: Vec<String>,
}

impl WebBrowser {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Housaky-AGI/4.0 (Rust)")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap();

        Self {
            client,
            user_agent: "Housaky-AGI/4.0".to_string(),
            timeout: Duration::from_secs(30),
            max_size: 10 * 1024 * 1024,
            allowed_domains: None,
            blocked_domains: vec!["malware.com".to_string(), "phishing.com".to_string()],
        }
    }

    pub fn with_allowed_domains(domains: Vec<String>) -> Self {
        let mut browser = Self::new();
        browser.allowed_domains = Some(domains);
        browser
    }

    pub async fn fetch(&self, url: &str) -> Result<WebPage> {
        info!("Fetching: {}", url);

        if !self.is_url_allowed(url) {
            bail!("URL not allowed: {}", url);
        }

        let response = self
            .client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

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
            bail!("Unsupported content type: {:?}", content_type);
        }

        let body = response.text().await?;

        if body.len() > self.max_size {
            bail!(
                "Content too large: {} bytes (max: {})",
                body.len(),
                self.max_size
            );
        }

        let (title, content, links, metadata) = self.parse_content(url, &body, content_type_str);

        info!("Fetched {} ({} bytes)", url, body.len());

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
        })
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

        for blocked in &self.blocked_domains {
            if domain.ends_with(blocked) || domain == blocked {
                return false;
            }
        }

        if let Some(ref allowed) = self.allowed_domains {
            let is_allowed = allowed.iter().any(|a| domain.ends_with(a) || domain == a);
            if !is_allowed {
                return false;
            }
        }

        true
    }

    fn parse_content(
        &self,
        base_url: &str,
        body: &str,
        content_type: &str,
    ) -> (
        Option<String>,
        String,
        Vec<LinkInfo>,
        HashMap<String, String>,
    ) {
        if content_type.starts_with("application/json") {
            return (None, body.to_string(), Vec::new(), HashMap::new());
        }

        if content_type.starts_with("text/plain") {
            return (None, body.to_string(), Vec::new(), HashMap::new());
        }

        let mut title = None;
        let mut content = String::new();
        let mut links = Vec::new();
        let mut metadata = HashMap::new();

        let mut in_script = false;
        let mut in_style = false;
        let _in_title = false;
        let _current_tag = String::new();
        let mut current_link_url = String::new();
        let mut current_link_text = String::new();
        let mut in_link = false;

        for line in body.lines() {
            let line_lower = line.to_lowercase();

            if line_lower.contains("<script") {
                in_script = true;
            }
            if line_lower.contains("</script>") {
                in_script = false;
                continue;
            }
            if line_lower.contains("<style") {
                in_style = true;
            }
            if line_lower.contains("</style>") {
                in_style = false;
                continue;
            }

            if in_script || in_style {
                continue;
            }

            if line_lower.contains("<title") {
                let start = line.find('>').map(|i| i + 1).unwrap_or(0);
                let end = line_lower.find("</title>").unwrap_or(line.len());
                let title_text = &line[start..end];
                if !title_text.is_empty() {
                    title = Some(self.clean_text(title_text));
                }
            }

            if line_lower.contains("<a ") || line_lower.contains("<a>") {
                in_link = true;
                if let Some(href_start) = line_lower.find("href=\"") {
                    let start = href_start + 6;
                    if let Some(end) = line[start..].find('"') {
                        current_link_url = line[start..start + end].to_string();
                    }
                }
            }

            if in_link {
                let text_start = line.find('>').map(|i| i + 1).unwrap_or(0);
                let text_end = line_lower.find("</a>").unwrap_or(line.len());
                if text_start < text_end {
                    current_link_text = self.clean_text(&line[text_start..text_end]);
                }

                if !current_link_url.is_empty() && !current_link_text.is_empty() {
                    let full_url = self.resolve_url(base_url, &current_link_url);
                    links.push(LinkInfo {
                        url: full_url,
                        text: current_link_text.clone(),
                        is_internal: self.is_internal_link(base_url, &current_link_url),
                    });
                }

                in_link = false;
                current_link_url.clear();
                current_link_text.clear();
            }

            if line_lower.contains("<meta ") {
                if let Some(name_start) = line_lower.find("name=\"") {
                    let start = name_start + 6;
                    if let Some(end) = line[start..].find('"') {
                        let name = line[start..start + end].to_string();
                        if let Some(content_start) = line_lower.find("content=\"") {
                            let c_start = content_start + 9;
                            if let Some(c_end) = line[c_start..].find('"') {
                                let value = line[c_start..c_start + c_end].to_string();
                                metadata.insert(name, value);
                            }
                        }
                    }
                }
            }

            let text = self.strip_tags(line);
            let text = self.clean_text(&text);
            if !text.is_empty() {
                content.push_str(&text);
                content.push(' ');
            }
        }

        content = self.normalize_whitespace(&content);

        (title, content, links, metadata)
    }

    fn strip_tags(&self, text: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;

        for c in text.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                result.push(c);
            }
        }

        result
    }

    fn clean_text(&self, text: &str) -> String {
        text.replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .trim()
            .to_string()
    }

    fn normalize_whitespace(&self, text: &str) -> String {
        let re = regex::Regex::new(r"\s+").unwrap();
        re.replace_all(text, " ").to_string()
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
            .and_then(|u| u.host_str().map(|s| s.to_string()));

        let link_domain = url::Url::parse(link)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()));

        match (base_domain, link_domain) {
            (Some(b), Some(l)) => b == l,
            _ => false,
        }
    }

    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        info!("Searching for: {}", query);

        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let page = self.fetch(&search_url).await?;

        let mut results = Vec::new();
        let re =
            regex::Regex::new(r#"<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#)
                .unwrap();

        for cap in re.captures_iter(&page.content) {
            if results.len() >= max_results {
                break;
            }

            let url = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let title = cap
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            if !url.is_empty() && !title.is_empty() {
                results.push(SearchResult {
                    title,
                    url,
                    snippet: String::new(),
                    source: "DuckDuckGo".to_string(),
                });
            }
        }

        info!("Found {} search results", results.len());

        Ok(results)
    }

    pub async fn extract_text(&self, url: &str) -> Result<String> {
        let page = self.fetch(url).await?;
        Ok(page.content)
    }

    pub async fn summarize(&self, url: &str, max_length: usize) -> Result<String> {
        let page = self.fetch(url).await?;

        let content = &page.content;

        if content.len() <= max_length {
            return Ok(content.clone());
        }

        let sentences: Vec<&str> = content.split(". ").collect();

        let mut summary = String::new();
        let mut current_length = 0;

        for sentence in sentences {
            if current_length + sentence.len() > max_length {
                break;
            }
            summary.push_str(sentence);
            summary.push_str(". ");
            current_length += sentence.len() + 2;
        }

        Ok(summary.trim().to_string())
    }

    pub async fn follow_links(
        &self,
        url: &str,
        depth: usize,
        max_pages: usize,
    ) -> Result<Vec<WebPage>> {
        let mut pages = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![(url.to_string(), 0)];

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
}

impl Default for WebBrowser {
    fn default() -> Self {
        Self::new()
    }
}

pub mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
