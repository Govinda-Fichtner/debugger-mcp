//! Documentation handler for embedded GitHub-hosted documentation resources
//!
//! This module provides access to comprehensive markdown documentation hosted on GitHub,
//! allowing AI agents to access detailed guides that complement the inline tool descriptions
//! and workflow resources.

use crate::{Error, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::{Resource, ResourceContents};

/// Cache entry for GitHub-fetched documentation
#[derive(Clone)]
struct CacheEntry {
    content: String,
    fetched_at: Instant,
}

/// Documentation handler that fetches markdown files from GitHub
pub struct DocumentationHandler {
    /// HTTP client for fetching from GitHub
    client: reqwest::Client,
    /// Simple cache for documentation content (5-minute TTL)
    cache: Arc<RwLock<std::collections::HashMap<String, CacheEntry>>>,
    /// Base URL for GitHub raw content
    base_url: String,
}

impl DocumentationHandler {
    /// Create a new documentation handler
    ///
    /// # Arguments
    /// * `github_user` - GitHub username (e.g., "Govinda-Fichtner")
    /// * `repo_name` - Repository name (e.g., "debugger-mcp")
    /// * `branch` - Branch name (e.g., "main")
    pub fn new(github_user: &str, repo_name: &str, branch: &str) -> Self {
        let base_url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}/docs",
            github_user, repo_name, branch
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            base_url,
        }
    }

    /// List all available documentation resources
    pub fn list_resources(&self) -> Vec<Resource> {
        vec![
            Resource {
                uri: "debugger-docs://getting-started".to_string(),
                name: "Getting Started Guide".to_string(),
                description: Some(
                    "Comprehensive introduction for AI agents new to the debugger MCP server. \
                     Covers installation, basic concepts, and your first debugging session."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
            },
            Resource {
                uri: "debugger-docs://guide/async-initialization".to_string(),
                name: "Async Initialization Guide".to_string(),
                description: Some(
                    "Deep dive into the async initialization system. Explains why debugger_start \
                     returns immediately and how to properly wait for initialization to complete."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
            },
            Resource {
                uri: "debugger-docs://guide/workflows".to_string(),
                name: "Complete Debugging Workflows".to_string(),
                description: Some(
                    "Detailed walkthrough of debugging workflows with real-world examples, \
                     including FizzBuzz debugging demonstration and multi-breakpoint patterns."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
            },
            Resource {
                uri: "debugger-docs://troubleshooting".to_string(),
                name: "Troubleshooting Guide".to_string(),
                description: Some(
                    "Common problems and solutions. Covers initialization failures, breakpoint \
                     issues, connection problems, and performance optimization."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
            },
            Resource {
                uri: "debugger-docs://advanced/logging".to_string(),
                name: "Logging System Architecture".to_string(),
                description: Some(
                    "Technical documentation of the comprehensive logging system. Shows how to \
                     interpret logs, use emoji codes for filtering, and debug issues."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
            },
        ]
    }

    /// List resource templates for MCP discovery
    pub fn list_resource_templates() -> Vec<Value> {
        vec![
            json!({
                "uriTemplate": "debugger-docs://getting-started",
                "name": "Getting Started Guide",
                "description": "Comprehensive introduction for AI agents",
                "mimeType": "text/markdown",
                "annotations": {
                    "audience": ["assistant"],
                    "priority": 1.0,
                    "category": "getting-started",
                    "estimatedReadTime": "5 minutes"
                }
            }),
            json!({
                "uriTemplate": "debugger-docs://guide/{topic}",
                "name": "Guide",
                "description": "Detailed guides on specific topics",
                "mimeType": "text/markdown",
                "annotations": {
                    "audience": ["assistant"],
                    "priority": 0.8,
                    "category": "guide",
                    "availableTopics": ["async-initialization", "workflows"]
                }
            }),
            json!({
                "uriTemplate": "debugger-docs://troubleshooting",
                "name": "Troubleshooting Guide",
                "description": "Common problems and solutions",
                "mimeType": "text/markdown",
                "annotations": {
                    "audience": ["assistant"],
                    "priority": 0.7,
                    "category": "troubleshooting"
                }
            }),
            json!({
                "uriTemplate": "debugger-docs://advanced/{topic}",
                "name": "Advanced Topic",
                "description": "Deep technical documentation",
                "mimeType": "text/markdown",
                "annotations": {
                    "audience": ["assistant", "developer"],
                    "priority": 0.5,
                    "category": "advanced",
                    "availableTopics": ["logging"]
                }
            }),
        ]
    }

    /// Read documentation resource by URI
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContents> {
        // Map URI to GitHub file path
        let github_path = self.uri_to_github_path(uri)?;
        let url = format!("{}/{}", self.base_url, github_path);

        // Try to fetch from cache first
        if let Some(cached) = self.get_cached(&url).await {
            tracing::debug!("Documentation cache hit for {}", uri);
            return Ok(ResourceContents {
                uri: uri.to_string(),
                mime_type: "text/markdown".to_string(),
                text: Some(cached),
                blob: None,
            });
        }

        // Fetch from GitHub
        tracing::info!("Fetching documentation from GitHub: {}", url);
        let content = self.fetch_from_github(&url).await?;

        // Cache the result
        self.cache_content(&url, content.clone()).await;

        Ok(ResourceContents {
            uri: uri.to_string(),
            mime_type: "text/markdown".to_string(),
            text: Some(content),
            blob: None,
        })
    }

    /// Map URI to GitHub file path
    fn uri_to_github_path(&self, uri: &str) -> Result<String> {
        if !uri.starts_with("debugger-docs://") {
            return Err(Error::InvalidRequest(format!(
                "Invalid documentation URI: {}",
                uri
            )));
        }

        let path = &uri["debugger-docs://".len()..];

        // Map URIs to actual file names
        let github_file = match path {
            "getting-started" => "GETTING_STARTED.md",
            "guide/async-initialization" => "ASYNC_INIT_IMPLEMENTATION.md",
            "guide/workflows" => "COMPLETE_SOLUTION_SUMMARY.md",
            "troubleshooting" => "TROUBLESHOOTING.md",
            "advanced/logging" => "LOG_VALIDATION_SYSTEM.md",
            _ => {
                return Err(Error::InvalidRequest(format!(
                    "Unknown documentation path: {}",
                    path
                )))
            }
        };

        Ok(github_file.to_string())
    }

    /// Fetch content from GitHub
    async fn fetch_from_github(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to fetch from GitHub: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "GitHub returned status {}: {}",
                response.status(),
                url
            )));
        }

        let content = response
            .text()
            .await
            .map_err(|e| Error::Internal(format!("Failed to read response body: {}", e)))?;

        Ok(content)
    }

    /// Get cached content if available and not expired
    async fn get_cached(&self, url: &str) -> Option<String> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(url) {
            // 5-minute TTL
            if entry.fetched_at.elapsed() < Duration::from_secs(300) {
                return Some(entry.content.clone());
            }
        }
        None
    }

    /// Cache content with timestamp
    async fn cache_content(&self, url: &str, content: String) {
        let mut cache = self.cache.write().await;
        cache.insert(
            url.to_string(),
            CacheEntry {
                content,
                fetched_at: Instant::now(),
            },
        );
    }

    /// Clear the cache (useful for testing)
    #[allow(dead_code)]
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_documentation_handler() {
        let handler = DocumentationHandler::new("user", "repo", "main");
        assert_eq!(
            handler.base_url,
            "https://raw.githubusercontent.com/user/repo/main/docs"
        );
    }

    #[test]
    fn test_list_resources() {
        let handler = DocumentationHandler::new("user", "repo", "main");
        let resources = handler.list_resources();

        assert_eq!(resources.len(), 5);
        assert!(resources
            .iter()
            .any(|r| r.uri == "debugger-docs://getting-started"));
        assert!(resources
            .iter()
            .any(|r| r.uri == "debugger-docs://troubleshooting"));
    }

    #[test]
    fn test_list_resource_templates() {
        let templates = DocumentationHandler::list_resource_templates();
        assert_eq!(templates.len(), 4);
    }

    #[test]
    fn test_uri_to_github_path() {
        let handler = DocumentationHandler::new("user", "repo", "main");

        assert_eq!(
            handler
                .uri_to_github_path("debugger-docs://getting-started")
                .unwrap(),
            "GETTING_STARTED.md"
        );
        assert_eq!(
            handler
                .uri_to_github_path("debugger-docs://guide/async-initialization")
                .unwrap(),
            "ASYNC_INIT_IMPLEMENTATION.md"
        );
        assert_eq!(
            handler
                .uri_to_github_path("debugger-docs://troubleshooting")
                .unwrap(),
            "TROUBLESHOOTING.md"
        );
    }

    #[test]
    fn test_uri_to_github_path_invalid() {
        let handler = DocumentationHandler::new("user", "repo", "main");

        assert!(handler
            .uri_to_github_path("invalid://path")
            .is_err());
        assert!(handler
            .uri_to_github_path("debugger-docs://nonexistent")
            .is_err());
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let handler = DocumentationHandler::new("user", "repo", "main");

        // Initially no cache
        assert!(handler
            .get_cached("https://test.com/doc.md")
            .await
            .is_none());

        // Cache content
        handler
            .cache_content("https://test.com/doc.md", "Test content".to_string())
            .await;

        // Should be cached now
        let cached = handler
            .get_cached("https://test.com/doc.md")
            .await;
        assert_eq!(cached, Some("Test content".to_string()));

        // Clear cache
        handler.clear_cache().await;
        assert!(handler
            .get_cached("https://test.com/doc.md")
            .await
            .is_none());
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let handler = DocumentationHandler::new("user", "repo", "main");

        // Cache with old timestamp
        {
            let mut cache = handler.cache.write().await;
            cache.insert(
                "https://test.com/doc.md".to_string(),
                CacheEntry {
                    content: "Old content".to_string(),
                    fetched_at: Instant::now() - Duration::from_secs(400), // 6+ minutes ago
                },
            );
        }

        // Should be expired (5-minute TTL)
        assert!(handler
            .get_cached("https://test.com/doc.md")
            .await
            .is_none());
    }
}
