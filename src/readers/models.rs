use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ReaderSourceType {
    WebPage,
    Pdf,
    Html,
    Text,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReadSourceRequest {
    pub source_url: String,
    pub source_type: Option<ReaderSourceType>,
    pub max_pages: Option<u32>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReadSourceResult {
    pub source_url: String,
    pub title: Option<String>,
    pub raw_text: String,
    pub cleaned_text: String,
    pub detected_type: ReaderSourceType,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub results: Vec<SearchResultItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ReadableDocument {
    pub source_url: String,
    pub title: Option<String>,
    pub text: String,
    pub cleaned_text: String,
    pub detected_type: ReaderSourceType,
    pub metadata: Option<Value>,
}
