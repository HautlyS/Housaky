//! Utility functions for `Housaky`.
//!
//! This module contains reusable helper functions used across the codebase.

pub mod markdown;
pub mod time;

pub use markdown::{render_markdown, StyledOutput};

/// Truncate a string to at most `max_chars` characters, appending "..." if truncated.
///
/// This function safely handles multi-byte UTF-8 characters (emoji, CJK, accented characters)
/// by using character boundaries instead of byte indices.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters to keep (excluding "...")
///
/// # Returns
/// * Original string if length <= `max_chars`
/// * Truncated string with "..." appended if length > `max_chars`
///
/// # Examples
/// ```
/// use housaky::util::truncate_with_ellipsis;
///
/// // ASCII string - no truncation needed
/// assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
///
/// // ASCII string - truncation needed
/// assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
///
/// // Multi-byte UTF-8 (emoji) - safe truncation
/// assert_eq!(truncate_with_ellipsis("Hello 🦀 World", 8), "Hello 🦀...");
/// assert_eq!(truncate_with_ellipsis("😀😀😀😀", 2), "😀😀...");
///
/// // Empty string
/// assert_eq!(truncate_with_ellipsis("", 10), "");
/// ```
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => {
            let truncated = &s[..idx];
            format!("{}...", truncated.trim_end())
        }
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_ascii_no_truncation() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("hello world", 50), "hello world");
    }

    #[test]
    fn test_truncate_ascii_with_truncation() {
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
        assert_eq!(
            truncate_with_ellipsis("This is a long message", 10),
            "This is a..."
        );
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate_with_ellipsis("", 10), "");
    }

    #[test]
    fn test_truncate_at_exact_boundary() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_emoji_single() {
        let s = "🦀";
        assert_eq!(truncate_with_ellipsis(s, 10), s);
        assert_eq!(truncate_with_ellipsis(s, 1), s);
    }

    #[test]
    fn test_truncate_emoji_multiple() {
        let s = "😀😀😀😀";
        assert_eq!(truncate_with_ellipsis(s, 2), "😀😀...");
        assert_eq!(truncate_with_ellipsis(s, 3), "😀😀😀...");
    }

    #[test]
    fn test_truncate_mixed_ascii_emoji() {
        assert_eq!(truncate_with_ellipsis("Hello 🦀 World", 8), "Hello 🦀...");
        assert_eq!(truncate_with_ellipsis("Hi 😊", 10), "Hi 😊");
    }

    #[test]
    fn test_truncate_cjk_characters() {
        let s = "这是一个测试消息用来触发崩溃的中文";
        let result = truncate_with_ellipsis(s, 16);
        assert!(result.ends_with("..."));
        assert!(result.is_char_boundary(result.len() - 1));
    }

    #[test]
    fn test_truncate_accented_characters() {
        let s = "café résumé naïve";
        assert_eq!(truncate_with_ellipsis(s, 10), "café résum...");
    }

    #[test]
    fn test_truncate_unicode_edge_case() {
        let s = "aé你好🦀";
        assert_eq!(truncate_with_ellipsis(s, 3), "aé你...");
    }

    #[test]
    fn test_truncate_long_string() {
        let s = "a".repeat(200);
        let result = truncate_with_ellipsis(&s, 50);
        assert_eq!(result.len(), 53);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_zero_max_chars() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "...");
    }
}

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

pub fn to_toml<T: Serialize>(value: &T) -> Result<String> {
    toml::to_string(value).context("Failed to serialize to TOML")
}

pub fn from_toml<T: DeserializeOwned>(toml_str: &str) -> Result<T> {
    toml::from_str(toml_str).context("Failed to deserialize from TOML")
}

pub async fn write_toml_file<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let toml_str = to_toml(value)?;

    let temp_path = path.with_extension("toml.tmp");
    tokio::fs::write(&temp_path, &toml_str).await?;
    tokio::fs::rename(&temp_path, path).await?;

    Ok(())
}

pub async fn read_toml_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let content = tokio::fs::read_to_string(path).await?;
    from_toml(&content)
}

pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = rmp_serde::to_vec(value).context("Failed to serialize to MessagePack")?;
    Ok(bytes)
}

pub fn from_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    rmp_serde::from_slice(bytes).context("Failed to deserialize from MessagePack")
}

pub async fn write_msgpack_file<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let bytes = to_msgpack(value)?;

    let temp_path = path.with_extension("msgpack.tmp");
    tokio::fs::write(&temp_path, &bytes).await?;
    tokio::fs::rename(&temp_path, path).await?;

    Ok(())
}

pub async fn read_msgpack_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = tokio::fs::read(path).await?;
    from_msgpack(&bytes)
}

pub fn expand_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();

    let expanded = shellexpand::tilde(&path_str);
    PathBuf::from(expanded.as_ref())
}

pub fn expand_and_resolve<P: AsRef<Path>, B: AsRef<Path>>(
    path: P,
    base_dir: Option<B>,
) -> PathBuf {
    let expanded = expand_path(path);

    if expanded.is_absolute() {
        expanded
    } else if let Some(base) = base_dir {
        base.as_ref().join(expanded)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&expanded))
            .unwrap_or(expanded)
    }
}

pub trait StorageFormat {
    fn extension(&self) -> &'static str;
}

pub enum TomlFormat {}
pub enum MsgpackFormat {}
pub enum JsonFormat {}

impl StorageFormat for TomlFormat {
    fn extension(&self) -> &'static str {
        "toml"
    }
}

impl StorageFormat for MsgpackFormat {
    fn extension(&self) -> &'static str {
        "msgpack"
    }
}

impl StorageFormat for JsonFormat {
    fn extension(&self) -> &'static str {
        "json"
    }
}
