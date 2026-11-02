use crate::types::PodcastShow;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

static RE_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static RE_CARD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a\b[^>]*class=["'][^"']*\b(?:card-title|title|pod-name|item-title)\b[^"']*["'][^>]*>(.*?)</a>"#).unwrap()
});
static RE_DATA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)data-(?:dir-)?(?:podcast-)?title=["']([^"']+)["'](?:[^>]*data-(?:dir-)?author=["']([^"']+)["'])?"#).unwrap()
});
static RE_DETAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a\b([^>]*)href=["'][^"']*/podcast-detail/[^"']*["']([^>]*)>(.*?)</a>"#)
        .unwrap()
});
static RE_ATTR_TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)\btitle=["']([^"']+)["']"#).unwrap());
static RE_ATTR_ALT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)\balt=["']([^"']+)["']"#).unwrap());
static RE_CHANNEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<channel\b[^>]*>(.*?)</channel>"#).unwrap());
static RE_CHANNEL_TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<title\b[^>]*>(.*?)</title>"#).unwrap());
static RE_CHANNEL_AUTHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<itunes:author\b[^>]*>(.*?)</itunes:author>"#).unwrap());
static RE_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<script\b[^>]*type=["']application/(?:ld\+)?json["'][^>]*>(.*?)</script>"#)
        .unwrap()
});
static RE_META_OG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<meta[^>]*property=["']og:title["'][^>]*content=["']([^"']+)["']"#).unwrap()
});
static RE_META_TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<title\b[^>]*>(.*?)</title>"#).unwrap());

/// Normalizes show title for deduplication
fn norm_key(title: &str) -> String {
    title.trim().to_lowercase()
}

/// Strips nested HTML tags and unescapes common HTML entities
pub fn strip_html_tags(s: &str) -> String {
    let cleaned = RE_TAG.replace_all(s, " ");
    cleaned
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Filters out generic website UI noise, navigation elements, or script artifacts
pub fn is_ui_noise(title: &str) -> bool {
    let lower = title.to_lowercase();
    let t = lower.trim();

    if t.len() < 2 || t.len() > 180 {
        return true;
    }

    if t.starts_with('<')
        || t.starts_with('{')
        || t.starts_with('[')
        || t.starts_with("var ")
        || t.starts_with("let ")
        || t.starts_with("const ")
        || t.starts_with("function")
        || t.starts_with("window.")
        || t.starts_with("document.")
        || t.ends_with(';')
        || t.contains("document.")
        || t.contains("window.")
        || t.contains("function(")
    {
        return true;
    }

    const NOISE_WORDS: &[&str] = &[
        "log in",
        "login",
        "sign in",
        "signin",
        "sign up",
        "signup",
        "podbean",
        "podbean app",
        "apple podcasts",
        "spotify",
        "youtube music",
        "terms of use",
        "terms of service",
        "privacy policy",
        "cookie preferences",
        "manage preferences",
        "cookie policy",
        "about us",
        "contact us",
        "help",
        "faq",
        "dashboard",
        "my account",
        "home",
        "search",
        "browse",
        "discover",
        "categories",
        "top podcasts",
        "featured podcasts",
        "popular podcasts",
        "following",
        "followers",
        "settings",
        "profile",
        "user profile",
        "upgrade",
        "pricing",
        "download",
        "podcast app",
        "listen now",
        "share",
        "follow",
        "unfollow",
        "subscribe",
        "unsubscribe",
        "view all",
        "see all",
        "more",
        "back to top",
        "close",
        "cancel",
        "submit",
        "save",
    ];

    NOISE_WORDS.contains(&t)
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
                    if s_trimmed.len() >= 2
                        && !s_trimmed.starts_with("http://")
                        && !s_trimmed.starts_with("https://")
                    {
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
                        let is_show = obj
                            .get("uri")
                            .and_then(|u| u.as_str())
                            .is_some_and(|u| u.starts_with("spotify:show:"))
                            || (obj.get("type").and_then(|t| t.as_str()) == Some("show"))
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
                            if obj
                                .get("browseId")
                                .and_then(|b| b.as_str())
                                .is_some_and(|b| b.starts_with("MPSP"))
                            {
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

/// Parses HTML card / link patterns, meta tags, RSS/Atom feeds, and embedded JSON scripts
pub fn scan_html(text: &str, platform: &str, map: &mut HashMap<String, PodcastShow>) {
    // 1. Podbean card-title and data attributes: <a class="...card-title..." ...>(.*?)</a>
    for cap in RE_CARD.captures_iter(text) {
        let t = strip_html_tags(&cap[1]);
        if !t.is_empty() && !is_ui_noise(&t) {
            let key = norm_key(&t);
            map.entry(key).or_insert_with(|| PodcastShow {
                title: t,
                author: None,
                source: platform.to_string(),
            });
        }
    }

    for cap in RE_DATA.captures_iter(text) {
        let t = strip_html_tags(&cap[1]);
        let a = cap.get(2).map(|m| strip_html_tags(m.as_str()));
        if !t.is_empty() && !is_ui_noise(&t) {
            let key = norm_key(&t);
            let entry = map.entry(key).or_insert_with(|| PodcastShow {
                title: t,
                author: a.clone(),
                source: platform.to_string(),
            });
            if entry.author.is_none() && a.is_some() {
                entry.author = a;
            }
        }
    }

    // Podbean show detail links: <a href=".../podcast-detail/...">Title</a>
    for cap in RE_DETAIL.captures_iter(text) {
        let mut t = strip_html_tags(&cap[3]);
        // If inner text was empty (e.g. only <img>), check title or alt attribute
        if t.is_empty() {
            let attrs = format!("{} {}", &cap[1], &cap[2]);
            if let Some(m) = RE_ATTR_TITLE
                .captures(&attrs)
                .or_else(|| RE_ATTR_TITLE.captures(&cap[3]))
            {
                t = strip_html_tags(&m[1]);
            } else if let Some(m) = RE_ATTR_ALT.captures(&cap[3]) {
                t = strip_html_tags(&m[1]);
            }
        }
        if !t.is_empty() && !is_ui_noise(&t) {
            let key = norm_key(&t);
            map.entry(key).or_insert_with(|| PodcastShow {
                title: t,
                author: None,
                source: platform.to_string(),
            });
        }
    }

    // 2. RSS / Atom XML feeds
    if text.contains("<rss") || text.contains("<feed") || text.contains("<channel") {
        for cap in RE_CHANNEL.captures_iter(text) {
            let ch = &cap[1];
            let title = RE_CHANNEL_TITLE
                .captures(ch)
                .map(|c| strip_html_tags(&c[1]));
            let author = RE_CHANNEL_AUTHOR
                .captures(ch)
                .map(|c| strip_html_tags(&c[1]));
            if let Some(t) = title {
                if !t.is_empty() && !is_ui_noise(&t) {
                    let key = norm_key(&t);
                    map.entry(key).or_insert_with(|| PodcastShow {
                        title: t,
                        author,
                        source: "RSS Feed".to_string(),
                    });
                }
            }
        }
    }

    // 3. Embedded JSON inside HTML (<script type="application/ld+json"> or <script type="application/json">)
    for cap in RE_SCRIPT.captures_iter(text) {
        let s = cap[1].trim();
        if s.starts_with('{') || s.starts_with('[') {
            if let Ok(val) = serde_json::from_str::<Value>(s) {
                scan_json_universal(&val, platform, map);
            }
        }
    }

    // 4. Spotify / Apple / Generic HTML meta tags: <meta property="og:title" content="Show Title">
    for cap in RE_META_OG.captures_iter(text) {
        let t = strip_html_tags(&cap[1]);
        if !t.is_empty() && !is_ui_noise(&t) {
            let key = norm_key(&t);
            map.entry(key).or_insert_with(|| PodcastShow {
                title: t,
                author: None,
                source: platform.to_string(),
            });
        }
    }
}

/// Universal entrypoint: Parses input content (HAR file JSON, plain JSON, HTML, or line-delimited text)
pub fn parse_content(raw: &str, platform_hint: Option<&str>) -> Vec<PodcastShow> {
    let mut shows_map: HashMap<String, PodcastShow> = HashMap::new();
    let platform = platform_hint.unwrap_or("Universal");
    let trimmed = raw.trim();

    let is_code_or_markup = trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || raw.contains("<!doctype")
        || raw.contains("<!DOCTYPE")
        || raw.contains("<html")
        || raw.contains("<body")
        || raw.contains("<head")
        || raw.contains("<div")
        || raw.contains("<script")
        || raw.contains("<?xml")
        || raw.contains("<rss")
        || raw.contains("<feed")
        || raw.contains("<opml");

    // 1. Try parsing as JSON (including full HAR files)
    if let Ok(json_val) = serde_json::from_str::<Value>(raw) {
        // If it's a HAR file, extract network response contents
        if let Some(entries) = json_val.pointer("/log/entries").and_then(|e| e.as_array()) {
            for entry in entries {
                if let Some(content_text) = entry
                    .pointer("/response/content/text")
                    .and_then(|t| t.as_str())
                {
                    let mime = entry
                        .pointer("/response/content/mimeType")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_lowercase();

                    if mime.contains("json")
                        || content_text.trim().starts_with('{')
                        || content_text.trim().starts_with('[')
                    {
                        if let Ok(sub_json) = serde_json::from_str::<Value>(content_text) {
                            scan_json_universal(&sub_json, platform, &mut shows_map);
                        }
                    } else if mime.contains("html")
                        || content_text.contains("<a ")
                        || content_text.contains("<div ")
                    {
                        scan_html(content_text, platform, &mut shows_map);
                    }
                }
            }
        } else {
            // General JSON payload (e.g. Spotify API, YouTube JSON, etc.)
            scan_json_universal(&json_val, platform, &mut shows_map);
        }
    }

    // 2. If nothing was found via JSON, try HTML/XML parsing
    if shows_map.is_empty() && (raw.contains('<') && raw.contains('>')) {
        scan_html(raw, platform, &mut shows_map);
    }

    // 3. Fallback: Parse line-by-line plaintext ONLY if the input is genuine plaintext (NOT code, HTML, or JSON)
    if shows_map.is_empty() && !is_code_or_markup {
        for line in raw.lines() {
            let l_trimmed = line.trim();
            if l_trimmed.is_empty()
                || l_trimmed.starts_with('#')
                || l_trimmed.starts_with("//")
                || l_trimmed.starts_with('<')
                || l_trimmed.ends_with(';')
                || l_trimmed.starts_with('{')
                || is_ui_noise(l_trimmed)
            {
                continue;
            }

            // Check for "Show Title by Author" or "Show Title - Author" format
            let (title, author) = if let Some(idx) = l_trimmed.rfind(" by ") {
                (
                    l_trimmed[..idx].trim(),
                    Some(l_trimmed[idx + 4..].trim().to_string()),
                )
            } else if let Some(idx) = l_trimmed.find(" - ") {
                (
                    l_trimmed[..idx].trim(),
                    Some(l_trimmed[idx + 3..].trim().to_string()),
                )
            } else {
                (l_trimmed, None)
            };

            if title.len() >= 2 && !is_ui_noise(title) {
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
    result.sort_by_key(|a| a.title.to_lowercase());
    result
}

/// Fetches a remote web page, profile URL, RSS feed, or OPML file and automatically extracts show subscriptions.
pub fn scrape_url(url: &str) -> Result<Vec<PodcastShow>, String> {
    let clean_url = url.trim();
    if !clean_url.starts_with("http://") && !clean_url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(6))
        .redirects(5)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build();

    let resp = agent
        .get(clean_url)
        .call()
        .map_err(|e| format!("Failed to fetch URL '{}': {}", clean_url, e))?;

    let final_url = resp.get_url().to_string();
    if final_url.contains("/login")
        || final_url.contains("/signin")
        || final_url.contains("user/login")
    {
        return Err("Podbean requires you to be logged into your account to view your subscriptions. Headless servers cannot access private accounts without your login cookies. Please use the '1-Click Browser Sync' bookmarklet on your Podbean page to transfer your subscriptions, or drop the saved page HTML here.".to_string());
    }

    let body = resp
        .into_string()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let mut shows = parse_content(&body, None);

    // Detect login wall / auth gates when 0 shows were parsed
    if shows.is_empty()
        && (body.contains("login-form")
            || body.contains("Sign In to Podbean")
            || body.contains("Log in to your account")
            || body.contains("name=\"login\""))
    {
        return Err("Podbean requires you to be logged into your account to view your subscriptions. Headless servers cannot access private accounts without your login cookies. Please use the '1-Click Browser Sync' bookmarklet on your Podbean page to transfer your subscriptions, or drop the saved page HTML here.".to_string());
    }

    // Auto-paginate Podbean profile / user URLs ONLY if Page 1 yielded valid shows
    let is_podbean_profile = clean_url.contains("podbean.com")
        && (clean_url.contains("/user") || clean_url.contains("/following"));

    if is_podbean_profile && !clean_url.contains("page=") && !shows.is_empty() {
        let mut all_shows = shows;
        let mut seen = std::collections::HashSet::new();
        for s in &all_shows {
            seen.insert(s.title.to_lowercase());
        }

        // Crawl up to 5 additional pages; stop as soon as a page returns fewer than 36 shows
        for p in 2..=6 {
            let separator = if clean_url.contains('?') { '&' } else { '?' };
            let page_url = format!("{}{}page={}", clean_url, separator, p);
            if let Ok(page_resp) = agent.get(&page_url).call() {
                if let Ok(page_body) = page_resp.into_string() {
                    let page_shows = parse_content(&page_body, Some("Podbean"));
                    let page_count = page_shows.len();
                    let mut new_in_page = 0;
                    for show in page_shows {
                        let key = show.title.to_lowercase();
                        if !seen.contains(&key) {
                            seen.insert(key);
                            all_shows.push(show);
                            new_in_page += 1;
                        }
                    }
                    if new_in_page == 0 || page_count < 36 {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return Ok(all_shows);
    }

    // If generic parser found nothing, try extracting <meta property="og:title"> or <title>
    if shows.is_empty() {
        let extracted_title = RE_META_OG
            .captures(&body)
            .map(|c| c[1].to_string())
            .or_else(|| RE_META_TITLE.captures(&body).map(|c| c[1].to_string()));

        if let Some(mut title) = extracted_title {
            title = strip_html_tags(&title);
            for suffix in &[
                " | Podbean",
                " - Podbean",
                " - Spotify",
                " on Apple Podcasts",
                " - YouTube",
            ] {
                if let Some(idx) = title.find(suffix) {
                    title = title[..idx].trim().to_string();
                }
            }
            if title.len() >= 2 && !is_ui_noise(&title) {
                shows.push(PodcastShow {
                    title,
                    author: None,
                    source: "URL Scraper".to_string(),
                });
            }
        }
    }

    if shows.is_empty() {
        return Err(format!(
            "No podcasts found at '{}'. If this page requires logging in, use the '1-Click Browser Sync' bookmarklet in your browser while on your Podbean page, or drop the saved page HTML here.",
            clean_url
        ));
    }

    Ok(shows)
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

    #[test]
    fn test_html_does_not_trigger_plaintext_fallback() {
        let dummy_html = r#"<!DOCTYPE html>
<html>
<head><title>Sign In to Podbean</title></head>
<body>
  <div class="login-box">
    <form action="/login" method="post">
      <input type="text" name="login" />
      <button type="submit">Sign In</button>
    </form>
  </div>
  <script>var x = 123; window.alert('hello');</script>
</body>
</html>"#;
        let shows = parse_content(dummy_html, Some("Podbean"));
        assert!(
            shows.is_empty(),
            "HTML without shows should return empty, not parse HTML tags as podcasts!"
        );
    }

    #[test]
    fn test_podbean_card_titles_with_newlines() {
        let html = r#"
            <div class="card">
                <a class="card-title" href="https://example.podbean.com">
                    Hardcore History
                </a>
            </div>
            <div class="card" data-dir-podcast-title="Radiolab" data-dir-author="WNYC Studios"></div>
        "#;
        let shows = parse_content(html, Some("Podbean"));
        assert_eq!(shows.len(), 2);
        assert_eq!(shows[0].title, "Hardcore History");
        assert_eq!(shows[1].title, "Radiolab");
        assert_eq!(shows[1].author.as_deref(), Some("WNYC Studios"));
    }

    #[test]
    fn test_rss_feed_parsing() {
        let rss_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
  <channel>
    <title>The Daily</title>
    <itunes:author>The New York Times</itunes:author>
  </channel>
</rss>"#;
        let shows = parse_content(rss_xml, None);
        assert_eq!(shows.len(), 1);
        assert_eq!(shows[0].title, "The Daily");
        assert_eq!(shows[0].author.as_deref(), Some("The New York Times"));
    }
}
