use crate::readers::models::{ReadSourceRequest, ReadSourceResult, ReaderSourceType};
use crate::readers::web_reader::WebReader;
use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct ReaderService {
    web_reader: WebReader,
}

impl ReaderService {
    pub fn new() -> Self {
        Self {
            web_reader: WebReader::new(),
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
