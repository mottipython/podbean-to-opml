use crate::types::{ExportItem, ResolveResult, ResolveStatus};

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
    sorted.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

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
    report.push_str("# This report contains shows that could not be automatically resolved with certainty.\n");
    report.push_str("# Review candidate feeds below to manually add them to your podcast client.\n\n");

    let ambiguous: Vec<&ResolveResult> = results.iter().filter(|r| r.status == ResolveStatus::Ambiguous).collect();
    let not_found: Vec<&ResolveResult> = results.iter().filter(|r| r.status == ResolveStatus::NotFound).collect();

    if !ambiguous.is_empty() {
        report.push_str("=================================================================\n");
        report.push_str(" AMBIGUOUS SUBSCRIPTIONS (Multiple or Near Matches Found)\n");
        report.push_str("=================================================================\n\n");

        for r in ambiguous {
            report.push_str(&format!("Show Title: {}\n", r.original_title));
            report.push_str(&format!("Publisher:  {}\n", r.original_author.as_deref().unwrap_or("Not specified")));
            report.push_str(&format!("Reason:     {}\n", r.reason));
            report.push_str("Candidate Feeds in Directory:\n");
            for (idx, c) in r.candidates.iter().enumerate() {
                report.push_str(&format!("  [{}] {} (by {})\n", idx + 1, c.collection_name, c.artist_name));
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
            report.push_str(&format!("Publisher:  {}\n", r.original_author.as_deref().unwrap_or("Not specified")));
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
