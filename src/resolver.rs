use crate::types::{Candidate, ResolveResult, ResolveStatus};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;

/// Normalizes title string by stripping punctuation and redundant whitespace
pub fn normalize_title(title: &str) -> String {
    let re = Regex::new(r"[^\w\s]").unwrap();
    let cleaned = re.replace_all(title, " ");
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Checks if two author / publisher names match or have significant overlap
pub fn authors_match(author_a: &str, author_b: &str) -> bool {
    let a = author_a.trim().to_lowercase();
    let b = author_b.trim().to_lowercase();
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }

    let re_punct = Regex::new(r"[^\w\s]").unwrap();
    let clean_a = re_punct.replace_all(&a, " ").to_string();
    let clean_b = re_punct.replace_all(&b, " ").to_string();

    let clean_a_trim = clean_a.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean_b_trim = clean_b.split_whitespace().collect::<Vec<_>>().join(" ");

    if clean_a_trim == clean_b_trim || clean_a_trim.contains(&clean_b_trim) || clean_b_trim.contains(&clean_a_trim) {
        return true;
    }

    let stop_words: HashSet<&str> = [
        "the", "a", "an", "and", "&", "inc", "llc", "ltd", "corp", "corporation",
        "media", "studios", "studio", "productions", "production", "network",
        "podcast", "podcasts", "group", "radio",
    ]
    .iter()
    .copied()
    .collect();

    let words_a: HashSet<&str> = clean_a_trim.split_whitespace().filter(|w| !stop_words.contains(w)).collect();
    let words_b: HashSet<&str> = clean_b_trim.split_whitespace().filter(|w| !stop_words.contains(w)).collect();

    if !words_a.is_empty() && !words_b.is_empty() && (words_a.is_subset(&words_b) || words_b.is_subset(&words_a)) {
        return true;
    }

    false
}

/// Inspects a podcast RSS feed URL over HTTP:
/// Follows HTTP 301/308 redirects and parses <itunes:new-feed-url> to detect relocated feeds.
pub fn verify_feed_status(feed_url: &str) -> (String, bool, Option<String>) {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .redirects(5)
        .build();

    match agent.get(feed_url).call() {
        Ok(resp) => {
            let final_url = resp.get_url().to_string();
            // Read first 8KB of body to check for itunes:new-feed-url
            let mut reader = resp.into_reader();
            let mut buf = [0u8; 8192];
            let n = std::io::Read::read(&mut reader, &mut buf).unwrap_or(0);
            let snippet = String::from_utf8_lossy(&buf[..n]);

            let re_new_feed = Regex::new(r"(?i)<itunes:new-feed-url>\s*(https?://[^\s<]+)\s*</itunes:new-feed-url>").unwrap();
            let new_feed = re_new_feed.captures(&snippet).map(|c| c[1].trim().to_string());

            let effective_url = new_feed.unwrap_or(final_url);
            let moved = effective_url.trim_end_matches('/') != feed_url.trim_end_matches('/');
            (effective_url, moved, None)
        }
        Err(e) => (feed_url.to_string(), false, Some(e.to_string())),
    }
}

/// Queries Apple Podcasts Search API to resolve a show title and publisher.
pub fn search_apple_podcasts(title: &str, author: Option<&str>, verify_feed: bool) -> ResolveResult {
    let encoded_term = utf8_percent_encode(title, NON_ALPHANUMERIC).to_string();
    let url = format!(
        "https://itunes.apple.com/search?media=podcast&entity=podcast&term={}&limit=8",
        encoded_term
    );

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent("iTunes/12.11.3 (Windows; Microsoft Windows 10 x64)")
        .build();

    let mut response_val: Option<Value> = None;
    for attempt in 0..3 {
        match agent.get(&url).call() {
            Ok(resp) => {
                if let Ok(val) = resp.into_json::<Value>() {
                    response_val = Some(val);
                    break;
                }
            }
            Err(ureq::Error::Status(code, _)) if code == 429 || code == 403 || code == 503 => {
                std::thread::sleep(Duration::from_millis(500 * (attempt + 1)));
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(400));
            }
        }
    }

    let payload = match response_val {
        Some(p) => p,
        None => {
            return ResolveResult {
                status: ResolveStatus::NotFound,
                original_title: title.to_string(),
                original_author: author.map(ToString::to_string),
                feed_url: None,
                matched_title: None,
                artist: None,
                artwork_url: None,
                candidates: Vec::new(),
                reason: "Directory query timed out or returned no response".to_string(),
                was_moved: false,
            };
        }
    };

    let empty_vec = Vec::new();
    let raw_results = payload.get("results").and_then(|r| r.as_array()).unwrap_or(&empty_vec);

    let mut valid_candidates = Vec::new();
    for res in raw_results {
        if let Some(feed) = res.get("feedUrl").and_then(|f| f.as_str()) {
            let col_name = res.get("collectionName").and_then(|c| c.as_str()).unwrap_or(title);
            let artist_name = res.get("artistName").and_then(|a| a.as_str()).unwrap_or("");
            let artwork = res.get("artworkUrl100").and_then(|a| a.as_str()).map(ToString::to_string);

            valid_candidates.push(Candidate {
                collection_name: col_name.to_string(),
                artist_name: artist_name.to_string(),
                feed_url: feed.to_string(),
                artwork_url: artwork,
            });
        }
    }

    if valid_candidates.is_empty() {
        return ResolveResult {
            status: ResolveStatus::NotFound,
            original_title: title.to_string(),
            original_author: author.map(ToString::to_string),
            feed_url: None,
            matched_title: None,
            artist: None,
            artwork_url: None,
            candidates: Vec::new(),
            reason: "No active RSS feed found in directory".to_string(),
            was_moved: false,
        };
    }

    let norm_target = normalize_title(title);
    let target_lower = title.trim().to_lowercase();

    let mut exact_matches = Vec::new();
    let mut close_matches = Vec::new();

    for cand in &valid_candidates {
        let cand_lower = cand.collection_name.trim().to_lowercase();
        let cand_norm = normalize_title(&cand.collection_name);

        if cand_lower == target_lower || cand_norm == norm_target {
            exact_matches.push(cand.clone());
        } else if cand_lower.starts_with(&format!("{}:", target_lower))
            || cand_lower.starts_with(&format!("{} -", target_lower))
            || target_lower.starts_with(&format!("{}:", cand_lower))
            || target_lower.starts_with(&format!("{} -", cand_lower))
        {
            close_matches.push(cand.clone());
        }
    }

    // 1. Single exact title match
    if exact_matches.len() == 1 {
        let chosen = exact_matches.remove(0);
        let mut final_feed = chosen.feed_url;
        let mut was_moved = false;

        if verify_feed {
            let (eff_url, moved, _) = verify_feed_status(&final_feed);
            if moved {
                final_feed = eff_url;
                was_moved = true;
            }
        }

        return ResolveResult {
            status: ResolveStatus::Resolved,
            original_title: title.to_string(),
            original_author: author.map(ToString::to_string),
            feed_url: Some(final_feed),
            matched_title: Some(chosen.collection_name),
            artist: Some(chosen.artist_name),
            artwork_url: chosen.artwork_url,
            candidates: Vec::new(),
            reason: "Exact title match".to_string(),
            was_moved,
        };
    }

    // 2. Multiple exact title matches (Same-name collision)
    if exact_matches.len() > 1 {
        if let Some(target_author) = author {
            let author_matches: Vec<Candidate> = exact_matches
                .iter()
                .filter(|c| authors_match(target_author, &c.artist_name))
                .cloned()
                .collect();

            if author_matches.len() == 1 {
                let chosen = author_matches.into_iter().next().unwrap();
                let mut final_feed = chosen.feed_url;
                let mut was_moved = false;

                if verify_feed {
                    let (eff_url, moved, _) = verify_feed_status(&final_feed);
                    if moved {
                        final_feed = eff_url;
                        was_moved = true;
                    }
                }

                return ResolveResult {
                    status: ResolveStatus::Resolved,
                    original_title: title.to_string(),
                    original_author: author.map(ToString::to_string),
                    feed_url: Some(final_feed),
                    matched_title: Some(chosen.collection_name),
                    artist: Some(chosen.artist_name),
                    artwork_url: chosen.artwork_url,
                    candidates: exact_matches,
                    reason: "Disambiguated duplicate title using publisher metadata".to_string(),
                    was_moved,
                };
            }
        }

        // Multiple matches remain ambiguous
        return ResolveResult {
            status: ResolveStatus::Ambiguous,
            original_title: title.to_string(),
            original_author: author.map(ToString::to_string),
            feed_url: None,
            matched_title: None,
            artist: None,
            artwork_url: None,
            candidates: exact_matches,
            reason: "Multiple shows found with this exact title (requires selection)".to_string(),
            was_moved: false,
        };
    }

    // 3. No exact matches, evaluate close subtitle matches
    if !close_matches.is_empty() {
        if let Some(target_author) = author {
            let close_author_matches: Vec<Candidate> = close_matches
                .iter()
                .filter(|c| authors_match(target_author, &c.artist_name))
                .cloned()
                .collect();

            if close_author_matches.len() == 1 {
                let chosen = close_author_matches.into_iter().next().unwrap();
                return ResolveResult {
                    status: ResolveStatus::Resolved,
                    original_title: title.to_string(),
                    original_author: author.map(ToString::to_string),
                    feed_url: Some(chosen.feed_url),
                    matched_title: Some(chosen.collection_name),
                    artist: Some(chosen.artist_name),
                    artwork_url: chosen.artwork_url,
                    candidates: close_matches,
                    reason: "Matched subtitle prefix and publisher".to_string(),
                    was_moved: false,
                };
            }
        }

        return ResolveResult {
            status: ResolveStatus::Ambiguous,
            original_title: title.to_string(),
            original_author: author.map(ToString::to_string),
            feed_url: None,
            matched_title: None,
            artist: None,
            artwork_url: None,
            candidates: close_matches,
            reason: "Near-match candidates found in directory".to_string(),
            was_moved: false,
        };
    }

    // 4. No matching podcast found (Do NOT blindly fall back to unrelated results)
    ResolveResult {
        status: ResolveStatus::NotFound,
        original_title: title.to_string(),
        original_author: author.map(ToString::to_string),
        feed_url: None,
        matched_title: None,
        artist: None,
        artwork_url: None,
        candidates: Vec::new(),
        reason: "No matching podcast found in directory".to_string(),
        was_moved: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authors_match_logic() {
        assert!(authors_match("The New York Times", "New York Times"));
        assert!(authors_match("Vox Media", "Vox"));
        assert!(authors_match("BBC World Service", "BBC"));
        assert!(!authors_match("BBC", "iHeartPodcasts"));
    }

    #[test]
    fn test_normalize_title_logic() {
        assert_eq!(normalize_title("The Daily & Co."), "the daily co");
        assert_eq!(normalize_title("Planet Money!"), "planet money");
    }
}
