use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodcastShow {
    pub title: String,
    pub author: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub collection_name: String,
    pub artist_name: String,
    pub feed_url: String,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResolveStatus {
    Resolved,
    Ambiguous,
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveResult {
    pub status: ResolveStatus,
    pub original_title: String,
    pub original_author: Option<String>,
    pub feed_url: Option<String>,
    pub matched_title: Option<String>,
    pub artist: Option<String>,
    pub artwork_url: Option<String>,
    pub candidates: Vec<Candidate>,
    pub reason: String,
    pub was_moved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseRequest {
    pub content: String,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveRequest {
    pub shows: Vec<PodcastShow>,
    pub verify_feeds: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOpmlRequest {
    pub items: Vec<ExportItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportItem {
    pub title: String,
    pub feed_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOpmlItemResult {
    pub title: String,
    pub original_feed_url: String,
    pub current_feed_url: String,
    pub is_alive: bool,
    pub was_moved: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOpmlResponse {
    pub items: Vec<CleanOpmlItemResult>,
    pub alive_count: usize,
    pub moved_count: usize,
    pub dead_count: usize,
    pub cleaned_opml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOpmlRequest {
    pub opml_content: String,
}
