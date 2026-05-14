use crate::readers::models::{
    ReadSourceRequest, ReadSourceResult, ReaderSourceType, SearchResultItem, SearchResults,
};
use anyhow::{Context, Result};
use regex::Regex;
use reqwest::Client;

#[derive(Clone)]
pub struct WebReader {
    client: Client,
}

impl WebReader {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("LocalOperator/1.0")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub async fn read_source(&self, request: ReadSourceRequest) -> Result<ReadSourceResult> {
        let html = self
            .client
            .get(&request.source_url)
            .send()
            .await
            .context("failed to send request to source URL")?
            .error_for_status()
            .context("source URL returned error status")?
            .text()
            .await
            .context("failed to read response body")?;

        let title = Self::extract_title(&html);
        let detected_type = request
            .source_type
            .unwrap_or_else(|| Self::detect_type(&request.source_url));
        let cleaned_text = Self::extract_text(&html);

        Ok(ReadSourceResult {
            source_url: request.source_url,
            title,
            raw_text: html,
            cleaned_text,
            detected_type,
            metadata: request.metadata,
        })
    }

    pub async fn search_web(&self, query: String, limit: usize) -> Result<SearchResults> {
        let html = self
            .client
            .get("https://duckduckgo.com/html/")
            .query(&[("q", query.as_str())])
            .send()
            .await
            .context("failed to send search request")?
            .error_for_status()
            .context("search provider returned error status")?
            .text()
            .await
            .context("failed to read search response body")?;

        let results = Self::extract_search_results(&html, limit);

        Ok(SearchResults { query, results })
    }

    fn detect_type(source_url: &str) -> ReaderSourceType {
        let url = source_url.to_lowercase();
        if url.ends_with(".pdf") {
            ReaderSourceType::Pdf
        } else if url.ends_with(".html") || url.ends_with(".htm") {
            ReaderSourceType::Html
        } else {
            ReaderSourceType::WebPage
        }
    }

    fn extract_title(html: &str) -> Option<String> {
        let title_re = Regex::new(r"(?is)<title>(.*?)</title>").unwrap();
        title_re
            .captures(html)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
    }

    fn extract_text(html: &str) -> String {
        let script_re = Regex::new(r"(?is)<script.*?>.*?</script>").unwrap();
        let style_re = Regex::new(r"(?is)<style.*?>.*?</style>").unwrap();
        let comment_re = Regex::new(r"(?is)<!--.*?-->").unwrap();
        let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();

        let without_scripts = script_re.replace_all(html, " ");
        let without_styles = style_re.replace_all(&without_scripts, " ");
        let without_comments = comment_re.replace_all(&without_styles, " ");
        let without_tags = tag_re.replace_all(&without_comments, " ");

        let decoded = Self::decode_html_entities(&without_tags);
        let collapsed = decoded
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        collapsed.trim().to_string()
    }

    fn extract_search_results(html: &str, limit: usize) -> Vec<SearchResultItem> {
        let result_re = Regex::new(
            r#"(?is)<a[^>]+class="[^"]*result__a[^"]*"[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#,
        )
        .unwrap();
        let snippet_re =
            Regex::new(r#"(?is)<a[^>]+class="[^"]*result__snippet[^"]*"[^>]*>(.*?)</a>"#).unwrap();
        let snippets = snippet_re
            .captures_iter(html)
            .filter_map(|caps| caps.get(1).map(|m| Self::clean_html_fragment(m.as_str())))
            .collect::<Vec<_>>();

        result_re
            .captures_iter(html)
            .take(limit)
            .enumerate()
            .filter_map(|(index, caps)| {
                let raw_url = caps.get(1)?.as_str();
                let title = caps.get(2).map(|m| Self::clean_html_fragment(m.as_str()))?;
                let url = Self::normalize_search_url(raw_url);

                Some(SearchResultItem {
                    title,
                    url,
                    snippet: snippets.get(index).cloned(),
                })
            })
            .collect()
    }

    fn clean_html_fragment(fragment: &str) -> String {
        let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();
        let without_tags = tag_re.replace_all(fragment, " ");
        Self::decode_html_entities(&without_tags)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn normalize_search_url(raw_url: &str) -> String {
        let decoded = Self::decode_html_entities(raw_url);
        if let Some(start) = decoded.find("uddg=") {
            let value = &decoded[start + 5..];
            let end = value.find('&').unwrap_or(value.len());
            return Self::percent_decode(&value[..end]);
        }

        if decoded.starts_with("//") {
            return format!("https:{}", decoded);
        }

        decoded
    }

    fn percent_decode(input: &str) -> String {
        let bytes = input.as_bytes();
        let mut output = Vec::with_capacity(bytes.len());
        let mut index = 0;

        while index < bytes.len() {
            if bytes[index] == b'%' && index + 2 < bytes.len() {
                if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                    if let Ok(value) = u8::from_str_radix(hex, 16) {
                        output.push(value);
                        index += 3;
                        continue;
                    }
                }
            }

            output.push(bytes[index]);
            index += 1;
        }

        String::from_utf8_lossy(&output).to_string()
    }

    fn decode_html_entities(input: &str) -> String {
        let mut output = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '&' {
                let mut entity = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == ';' {
                        chars.next();
                        break;
                    }
                    entity.push(next_ch);
                    chars.next();
                }

                match entity.as_str() {
                    "nbsp" => output.push(' '),
                    "amp" => output.push('&'),
                    "lt" => output.push('<'),
                    "gt" => output.push('>'),
                    "quot" => output.push('"'),
                    "apos" => output.push('\''),
                    entity if entity.starts_with("#x") => {
                        if let Ok(code) = u32::from_str_radix(&entity[2..], 16) {
                            if let Some(decoded) = std::char::from_u32(code) {
                                output.push(decoded);
                            }
                        }
                    }
                    entity if entity.starts_with('#') => {
                        if let Ok(code) = entity[1..].parse::<u32>() {
                            if let Some(decoded) = std::char::from_u32(code) {
                                output.push(decoded);
                            }
                        }
                    }
                    _ => {
                        output.push('&');
                        output.push_str(&entity);
                        output.push(';');
                    }
                }
            } else {
                output.push(ch);
            }
        }

        output
    }
}
