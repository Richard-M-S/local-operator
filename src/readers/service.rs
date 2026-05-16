use std::sync::Arc;

use crate::config::SearchConfig;
use crate::readers::models::{
    ReadSourceRequest, ReadSourceResult, ReaderSourceType, SearchResults,
};
use crate::readers::search::{DuckDuckGoHtmlSearchProvider, SearchProvider};
use crate::readers::web_reader::WebReader;
use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct ReaderService {
    web_reader: WebReader,
    search_provider: Arc<dyn SearchProvider>,
}

impl ReaderService {
    pub fn new(config: SearchConfig) -> Self {
        Self {
            web_reader: WebReader::new(),
            search_provider: build_search_provider(&config),
        }
    }

    pub async fn read_url(&self, url: String) -> Result<ReadSourceResult> {
        let request = ReadSourceRequest {
            source_url: url,
            source_type: None,
            max_pages: None,
            metadata: None,
        };

        self.read_source(request).await
    }

    pub async fn search_web(&self, query: String, limit: usize) -> Result<SearchResults> {
        self.search_provider.search(query, limit).await
    }

    pub async fn read_source(&self, request: ReadSourceRequest) -> Result<ReadSourceResult> {
        let source_type = request
            .source_type
            .clone()
            .unwrap_or_else(|| Self::detect_source_type(&request.source_url));

        match source_type {
            ReaderSourceType::Pdf => Err(anyhow!("PDF reader not implemented yet")),
            ReaderSourceType::WebPage
            | ReaderSourceType::Html
            | ReaderSourceType::Text
            | ReaderSourceType::Unknown => self.web_reader.read_source(request).await,
        }
    }

    fn detect_source_type(source_url: &str) -> ReaderSourceType {
        let url = source_url.to_lowercase();

        if url.ends_with(".pdf") {
            ReaderSourceType::Pdf
        } else if url.ends_with(".html") || url.ends_with(".htm") {
            ReaderSourceType::Html
        } else if url.ends_with(".txt") {
            ReaderSourceType::Text
        } else {
            ReaderSourceType::WebPage
        }
    }
}

fn build_search_provider(config: &SearchConfig) -> Arc<dyn SearchProvider> {
    match config.provider.as_str() {
        "duckduckgo_html" => Arc::new(DuckDuckGoHtmlSearchProvider::new()),
        _ => Arc::new(DuckDuckGoHtmlSearchProvider::new()),
    }
}
