use crate::types::{
    CleanOpmlItemResult, CleanOpmlResponse, ExportItem, ResolveResult, ResolveStatus,
};
use regex::Regex;

pub fn generate_opml_xml(items: &[ExportItem], unresolved_count: usize) -> String {
    let now = chrono_free_date();
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<opml version=\"1.0\">\n");
    xml.push_str("  <head>\n");
    xml.push_str("    <title>Rescued Podcast Subscriptions</title>\n");
    xml.push_str(&format!("    <dateCreated>{}</dateCreated>\n", now));
    xml.push_str("  </head>\n");
    xml.push_str("  <body>\n");

    if unresolved_count > 0 {
        xml.push_str(&format!(
            "    <!-- Note: {} subscriptions could not be safely resolved. See report for details. -->\n",
            unresolved_count
        ));
    }

    let mut sorted = items.to_vec();
    sorted.sort_by_key(|a| a.title.to_lowercase());

    for item in sorted {
        let escaped_title = escape_xml(&item.title);
        let escaped_url = escape_xml(&item.feed_url);
        xml.push_str(&format!(
            "    <outline text=\"{}\" title=\"{}\" type=\"rss\" xmlUrl=\"{}\" htmlUrl=\"\" />\n",
            escaped_title, escaped_title, escaped_url
        ));
    }

    xml.push_str("  </body>\n");
    xml.push_str("</opml>\n");
    xml
}

pub fn generate_unresolved_report(results: &[ResolveResult]) -> String {
    let mut report = String::new();
    report.push_str("# Podcast Rescue - Unresolved Subscriptions Report\n");
    report.push_str(
        "# This report contains shows that could not be automatically resolved with certainty.\n",
    );
    report.push_str(
        "# Review candidate feeds below to manually add them to your podcast client.\n\n",
    );

    let ambiguous: Vec<&ResolveResult> = results
        .iter()
        .filter(|r| r.status == ResolveStatus::Ambiguous)
        .collect();
    let not_found: Vec<&ResolveResult> = results
        .iter()
        .filter(|r| r.status == ResolveStatus::NotFound)
        .collect();

    if !ambiguous.is_empty() {
        report.push_str("=================================================================\n");
        report.push_str(" AMBIGUOUS SUBSCRIPTIONS (Multiple or Near Matches Found)\n");
        report.push_str("=================================================================\n\n");

        for r in ambiguous {
            report.push_str(&format!("Show Title: {}\n", r.original_title));
            report.push_str(&format!(
                "Publisher:  {}\n",
                r.original_author.as_deref().unwrap_or("Not specified")
            ));
            report.push_str(&format!("Reason:     {}\n", r.reason));
            report.push_str("Candidate Feeds in Directory:\n");
            for (idx, c) in r.candidates.iter().enumerate() {
                report.push_str(&format!(
                    "  [{}] {} (by {})\n",
                    idx + 1,
                    c.collection_name,
                    c.artist_name
                ));
                report.push_str(&format!("      Feed: {}\n", c.feed_url));
            }
            report.push_str("\n----------------------------------------\n\n");
        }
    }

    if !not_found.is_empty() {
        report.push_str("=================================================================\n");
        report.push_str(" UNRESOLVED SUBSCRIPTIONS (No Directory Matches Found)\n");
        report.push_str("=================================================================\n\n");

        for r in not_found {
            report.push_str(&format!("Show Title: {}\n", r.original_title));
            report.push_str(&format!(
                "Publisher:  {}\n",
                r.original_author.as_deref().unwrap_or("Not specified")
            ));
            report.push_str(&format!("Status:     {}\n", r.reason));
            report.push_str("Note:       May be exclusive to the platform, private, or removed from Apple Podcasts.\n\n");
        }
    }

    report
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn chrono_free_date() -> String {
    // Standard RFC 822 format or ISO timestamp
    "Sun, 01 Nov 2026 12:00:00 GMT".to_string()
}

pub fn generate_json_export(items: &[ExportItem]) -> String {
    let mut sorted = items.to_vec();
    sorted.sort_by_key(|a| a.title.to_lowercase());
    serde_json::to_string_pretty(&sorted).unwrap_or_else(|_| "[]".to_string())
}

pub fn generate_csv_export(items: &[ExportItem]) -> String {
    let mut sorted = items.to_vec();
    sorted.sort_by_key(|a| a.title.to_lowercase());
    let mut csv = String::from("Title,Feed URL\n");
    for item in sorted {
        let clean_title = item.title.replace('"', "\"\"");
        let clean_url = item.feed_url.replace('"', "\"\"");
        csv.push_str(&format!("\"{}\",\"{}\"\n", clean_title, clean_url));
    }
    csv
}

pub fn parse_opml_feeds(opml_content: &str) -> Vec<ExportItem> {
    let mut items = Vec::new();
    let re_outline = Regex::new(r#"<outline\b([^>]+)>"#).unwrap();
    let re_xml_url = Regex::new(r#"xmlUrl=["']([^"']+)["']"#).unwrap();
    let re_text = Regex::new(r#"(?:text|title)=["']([^"']+)["']"#).unwrap();

    for cap in re_outline.captures_iter(opml_content) {
        let attrs = &cap[1];
        if let Some(xml_cap) = re_xml_url.captures(attrs) {
            let feed_url = unescape_xml(&xml_cap[1]);
            let title = if let Some(text_cap) = re_text.captures(attrs) {
                unescape_xml(&text_cap[1])
            } else {
                feed_url.clone()
            };
            if !feed_url.is_empty() {
                items.push(ExportItem { title, feed_url });
            }
        }
    }
    items
}

pub fn clean_opml(opml_content: &str) -> CleanOpmlResponse {
    let raw_items = parse_opml_feeds(opml_content);
    let mut items = Vec::new();
    let mut alive_count = 0;
    let mut moved_count = 0;
    let mut dead_count = 0;
    let mut cleaned_export_items = Vec::new();

    for item in raw_items {
        let (current_url, was_moved, err) = crate::resolver::verify_feed_status(&item.feed_url);
        let is_alive = err.is_none();
        if is_alive {
            alive_count += 1;
            if was_moved {
                moved_count += 1;
            }
            cleaned_export_items.push(ExportItem {
                title: item.title.clone(),
                feed_url: current_url.clone(),
            });
        } else {
            dead_count += 1;
        }

        items.push(CleanOpmlItemResult {
            title: item.title,
            original_feed_url: item.feed_url,
            current_feed_url: current_url,
            is_alive,
            was_moved,
            error: err,
        });
    }

    let cleaned_opml = generate_opml_xml(&cleaned_export_items, dead_count);

    CleanOpmlResponse {
        items,
        alive_count,
        moved_count,
        dead_count,
        cleaned_opml,
    }
}

fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_json_export() {
        let items = vec![
            ExportItem {
                title: "Show B".to_string(),
                feed_url: "https://example.com/b.rss".to_string(),
            },
            ExportItem {
                title: "Show A".to_string(),
                feed_url: "https://example.com/a.rss".to_string(),
            },
        ];
        let json = generate_json_export(&items);
        assert!(json.contains("\"title\": \"Show A\""));
        assert!(json.contains("\"feed_url\": \"https://example.com/a.rss\""));
        // Check sorting: Show A comes before Show B
        let pos_a = json.find("Show A").unwrap();
        let pos_b = json.find("Show B").unwrap();
        assert!(pos_a < pos_b);
    }

    #[test]
    fn test_generate_csv_export() {
        let items = vec![ExportItem {
            title: "Show \"Quoted\"".to_string(),
            feed_url: "https://example.com/rss".to_string(),
        }];
        let csv = generate_csv_export(&items);
        assert!(csv.starts_with("Title,Feed URL\n"));
        assert!(csv.contains("\"Show \"\"Quoted\"\"\",\"https://example.com/rss\"\n"));
    }

    #[test]
    fn test_parse_opml_feeds() {
        let sample_opml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="1.0">
  <head><title>Subscriptions</title></head>
  <body>
    <outline text="The Daily" title="The Daily" type="rss" xmlUrl="https://feeds.simplecast.com/54nAGcIl" />
    <outline text="Radiolab &amp; More" xmlUrl="https://feeds.wnyc.org/radiolab" />
  </body>
</opml>"#;
        let feeds = parse_opml_feeds(sample_opml);
        assert_eq!(feeds.len(), 2);
        assert_eq!(feeds[0].title, "The Daily");
        assert_eq!(feeds[0].feed_url, "https://feeds.simplecast.com/54nAGcIl");
        assert_eq!(feeds[1].title, "Radiolab & More");
        assert_eq!(feeds[1].feed_url, "https://feeds.wnyc.org/radiolab");
    }
}
