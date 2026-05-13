use crate::readers::models::{ReadSourceRequest, ReadSourceResult, ReaderSourceType};
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
