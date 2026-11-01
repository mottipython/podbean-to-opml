use crate::types::PodcastShow;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// Normalizes show title for deduplication
fn norm_key(title: &str) -> String {
    title.trim().to_lowercase()
}

/// Recursively scans arbitrary JSON objects/arrays looking for podcast titles and authors.
pub fn scan_json_universal(val: &Value, platform: &str, map: &mut HashMap<String, PodcastShow>) {
    match val {
        Value::Object(obj) => {
            let mut title = None;
            let mut author = None;

            for (k, v) in obj {
                let k_clean = k.to_lowercase().replace(['_', '-'], "");
                if let Value::String(s) = v {
                    let s_trimmed = s.trim();
                    if s_trimmed.len() >= 2 && !s_trimmed.starts_with("http://") && !s_trimmed.starts_with("https://") {
                        if k_clean == "title"
                            || k_clean == "podcasttitle"
                            || k_clean == "podcastname"
                            || k_clean == "collectionname"
                            || k_clean == "showname"
                        {
                            title = Some(s_trimmed.to_string());
                        } else if k_clean == "author"
                            || k_clean == "authorname"
                            || k_clean == "publisher"
                            || k_clean == "artistname"
                            || k_clean == "owner"
                            || k_clean == "creator"
                        {
                            author = Some(s_trimmed.to_string());
                        }
                    }
                }
            }

            // Platform-specific Spotify JSON patterns
            // Spotify GraphQL/Pathfinder queryShowLibrary: item.show.name & item.show.publisher
            if title.is_none() {
                if let Some(name_val) = obj.get("name") {
                    if let Some(name_str) = name_val.as_str() {
                        // Check if parent or sibling has publisher / uri: "spotify:show:..."
                        let is_show = obj.get("uri").and_then(|u| u.as_str()).map_or(false, |u| u.starts_with("spotify:show:"))
                            || obj.get("type").and_then(|t| t.as_str()).map_or(false, |t| t == "show")
                            || obj.contains_key("publisher");

                        if is_show && name_str.trim().len() >= 2 {
                            title = Some(name_str.trim().to_string());
                            if let Some(pub_val) = obj.get("publisher").and_then(|p| p.as_str()) {
                                author = Some(pub_val.trim().to_string());
                            }
                        }
                    }
                }
            }

            // Platform-specific YouTube Music runs pattern
            // {"runs": [{"text": "Show Title"}], ...}
            if title.is_none() {
                if let Some(runs) = obj.get("runs").and_then(|r| r.as_array()) {
                    if !runs.is_empty() {
                        if let Some(first_text) = runs[0].get("text").and_then(|t| t.as_str()) {
                            if obj.get("browseId").and_then(|b| b.as_str()).map_or(false, |b| b.starts_with("MPSP")) {
                                title = Some(first_text.trim().to_string());
                            }
                        }
                    }
                }
            }

            if let Some(t) = title {
                let key = norm_key(&t);
                if let Some(existing) = map.get_mut(&key) {
                    if existing.author.is_none() && author.is_some() {
                        existing.author = author;
                    }
                } else {
                    map.insert(
                        key,
                        PodcastShow {
                            title: t,
                            author,
                            source: platform.to_string(),
                        },
                    );
                }
            }

            // Recurse down children
            for child in obj.values() {
                scan_json_universal(child, platform, map);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                scan_json_universal(item, platform, map);
            }
        }
        _ => {}
    }
}

/// Parses HTML card / link patterns
pub fn scan_html(text: &str, platform: &str, map: &mut HashMap<String, PodcastShow>) {
    // 1. Podbean card-title and data attributes
    if let Ok(re_card) = Regex::new(r#"<a[^>]*class="[^"]*card-title[^"]*"[^>]*>(.*?)</a>"#) {
        for cap in re_card.captures_iter(text) {
            let t = cap[1].trim();
            if !t.is_empty() {
                let key = norm_key(t);
                map.entry(key).or_insert_with(|| PodcastShow {
                    title: t.to_string(),
                    author: None,
                    source: platform.to_string(),
                });
            }
        }
    }

    if let Ok(re_data) = Regex::new(r#"data-dir-podcast-title="([^"]+)"(?:[^>]*data-dir-author="([^"]+)")?"#) {
        for cap in re_data.captures_iter(text) {
            let t = cap[1].trim();
            let a = cap.get(2).map(|m| m.as_str().trim().to_string());
            if !t.is_empty() {
                let key = norm_key(t);
                let entry = map.entry(key).or_insert_with(|| PodcastShow {
                    title: t.to_string(),
                    author: a.clone(),
                    source: platform.to_string(),
                });
                if entry.author.is_none() && a.is_some() {
                    entry.author = a;
                }
            }
        }
    }

    // 2. Spotify / Apple / Generic HTML meta tags: <meta property="og:title" content="Show Title">
    if let Ok(re_meta) = Regex::new(r#"<meta[^>]*property="og:title"[^>]*content="([^"]+)"#) {
        for cap in re_meta.captures_iter(text) {
            let t = cap[1].trim();
            if !t.is_empty() && !t.contains("Spotify - Web Player") && !t.contains("Apple Podcasts") {
                let key = norm_key(t);
                map.entry(key).or_insert_with(|| PodcastShow {
                    title: t.to_string(),
                    author: None,
                    source: platform.to_string(),
                });
            }
        }
    }
}

/// Universal entrypoint: Parses input content (HAR file JSON, plain JSON, HTML, or line-delimited text)
pub fn parse_content(raw: &str, platform_hint: Option<&str>) -> Vec<PodcastShow> {
    let mut shows_map: HashMap<String, PodcastShow> = HashMap::new();
    let platform = platform_hint.unwrap_or("Universal");

    // 1. Try parsing as JSON (including full HAR files)
    if let Ok(json_val) = serde_json::from_str::<Value>(raw) {
        // If it's a HAR file, extract network response contents
        if let Some(entries) = json_val.pointer("/log/entries").and_then(|e| e.as_array()) {
            for entry in entries {
                if let Some(content_text) = entry.pointer("/response/content/text").and_then(|t| t.as_str()) {
                    let mime = entry
                        .pointer("/response/content/mimeType")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_lowercase();

                    if mime.contains("json") || content_text.trim().starts_with('{') || content_text.trim().starts_with('[') {
                        if let Ok(sub_json) = serde_json::from_str::<Value>(content_text) {
                            scan_json_universal(&sub_json, platform, &mut shows_map);
                        }
                    } else if mime.contains("html") || content_text.contains("<a ") || content_text.contains("<div ") {
                        scan_html(content_text, platform, &mut shows_map);
                    }
                }
            }
        } else {
            // General JSON payload (e.g. Spotify API, YouTube JSON, etc.)
            scan_json_universal(&json_val, platform, &mut shows_map);
        }
    }

    // 2. If nothing was found via JSON, try HTML parsing
    if shows_map.is_empty() && (raw.contains("<html") || raw.contains("<body") || raw.contains("<a ")) {
        scan_html(raw, platform, &mut shows_map);
    }

    // 3. Fallback: Parse line-by-line plaintext (e.g. user pasted list: "Title by Author" or "Title")
    if shows_map.is_empty() {
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            // Check for "Show Title by Author" format
            let (title, author) = if let Some(idx) = trimmed.rfind(" by ") {
                (trimmed[..idx].trim(), Some(trimmed[idx + 4..].trim().to_string()))
            } else if let Some(idx) = trimmed.find(" - ") {
                (trimmed[..idx].trim(), Some(trimmed[idx + 3..].trim().to_string()))
            } else {
                (trimmed, None)
            };

            if title.len() >= 2 {
                let key = norm_key(title);
                shows_map.entry(key).or_insert_with(|| PodcastShow {
                    title: title.to_string(),
                    author,
                    source: platform.to_string(),
                });
            }
        }
    }

    let mut result: Vec<PodcastShow> = shows_map.into_values().collect();
    result.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_universal() {
        let json_str = r#"[
            {"podcast_title": "The Daily", "author": "The New York Times"},
            {"name": "Hardcore History", "publisher": "Dan Carlin", "uri": "spotify:show:123"}
        ]"#;

        let shows = parse_content(json_str, Some("Spotify"));
        assert_eq!(shows.len(), 2);
        assert_eq!(shows[0].title, "Hardcore History");
        assert_eq!(shows[0].author.as_deref(), Some("Dan Carlin"));
        assert_eq!(shows[1].title, "The Daily");
        assert_eq!(shows[1].author.as_deref(), Some("The New York Times"));
    }

    #[test]
    fn test_parse_har_podbean() {
        let har_str = r#"{
            "log": {
                "entries": [
                    {
                        "response": {
                            "content": {
                                "mimeType": "application/json",
                                "text": "[{\"title\": \"A Little Bit Culty\", \"author\": \"Sarah & Anthony\"}]"
                            }
                        }
                    }
                ]
            }
        }"#;

        let shows = parse_content(har_str, Some("Podbean"));
        assert_eq!(shows.len(), 1);
        assert_eq!(shows[0].title, "A Little Bit Culty");
        assert_eq!(shows[0].author.as_deref(), Some("Sarah & Anthony"));
    }

    #[test]
    fn test_parse_plaintext_lines() {
        let raw = "The Daily by The New York Times\nRadiolab\nFocus - BBC World Service";
        let shows = parse_content(raw, None);
        assert_eq!(shows.len(), 3);
        assert_eq!(shows[0].title, "Focus");
        assert_eq!(shows[0].author.as_deref(), Some("BBC World Service"));
    }
}
