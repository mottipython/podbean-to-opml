use crate::opml::{
    clean_opml, generate_csv_export, generate_json_export, generate_opml_xml, generate_unresolved_report,
};
use crate::parser::{parse_content, scrape_url};
use crate::resolver::search_apple_podcasts;
use crate::types::{
    AutoIngestRequest, CleanOpmlRequest, ExportOpmlRequest, ParseRequest, PodcastShow, ResolveRequest,
    ResolveResult, ScrapeUrlRequest,
};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tiny_http::{Header, Response, Server};

const HTML_UI: &str = include_str!("web_ui.html");

/// Exit after this long with no requests (the open UI polls every 2s, so only an abandoned server idles).
const IDLE_EXIT: Duration = Duration::from_secs(30 * 60);

/// Sites whose pages may post synced shows to the app (bookmarklet / console script / extension).
fn origin_allowed(origin: &str) -> bool {
    let host = origin.split("://").nth(1).unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    origin.starts_with("https://")
        && (host == "podbean.com"
            || host.ends_with(".podbean.com")
            || matches!(host, "open.spotify.com" | "podcasts.apple.com" | "music.youtube.com"))
}

fn host_is_local(host: &str) -> bool {
    let name = host.rsplit_once(':').map_or(host, |(n, _)| n);
    matches!(name, "127.0.0.1" | "localhost" | "[::1]")
}

/// Random per-run secret; only the page this server serves ever sees it.
fn new_token() -> String {
    (0..2)
        .map(|_| {
            let mut h = RandomState::new().build_hasher();
            h.write_u128(Instant::now().elapsed().as_nanos());
            format!("{:016x}", h.finish())
        })
        .collect()
}

fn request_header(request: &tiny_http::Request, name: &str) -> String {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default()
}

pub fn run_web_server(host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", host, port);
    let server = Server::http(&addr).map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
    let server = Arc::new(server);
    let ingest_queue: Arc<Mutex<Vec<PodcastShow>>> = Arc::new(Mutex::new(Vec::new()));

    let token = new_token();
    let loopback_only = host == "127.0.0.1" || host == "localhost" || host == "::1";
    let mut last_request = Instant::now();

    let display_host = if host == "0.0.0.0" { "localhost" } else { host };
    println!("=================================================================");
    println!(" 🚀 Podcast Rescue Cockpit running at http://{}:{}", display_host, port);
    println!(" Open this URL in your web browser if it did not open automatically.");
    println!(" Press Ctrl+C in terminal to stop.");
    println!("=================================================================");

    loop {
        let mut request = match server.recv_timeout(Duration::from_secs(30)) {
            Ok(Some(r)) => r,
            Ok(None) if loopback_only && last_request.elapsed() > IDLE_EXIT => {
                println!("Idle for 30 minutes, shutting down.");
                break;
            }
            Ok(None) => continue,
            Err(_) => break,
        };
        last_request = Instant::now();
        let url = request.url().to_string();
        let method = request.method().as_str().to_uppercase();
        let origin = request_header(&request, "Origin");
        let is_ingest = url == "/api/auto-ingest";

        let header_html = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
        let header_json = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
        let header_xml = Header::from_bytes(&b"Content-Type"[..], &b"text/xml; charset=utf-8"[..]).unwrap();
        let header_text = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
        let header_csv = Header::from_bytes(&b"Content-Type"[..], &b"text/csv; charset=utf-8"[..]).unwrap();
        // Cross-origin access only for the sync endpoint, and only from known podcast sites.
        let header_cors = if is_ingest && origin_allowed(&origin) {
            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], origin.as_bytes()).unwrap()
        } else {
            Header::from_bytes(&b"Vary"[..], &b"Origin"[..]).unwrap()
        };
        let header_cors_methods = Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap();
        let header_cors_headers = Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type, X-Rescue-Token"[..]).unwrap();

        let forbidden = |request: tiny_http::Request| {
            let _ = request.respond(Response::from_string("{\"error\": \"Forbidden\"}").with_status_code(403));
        };
        // Blocks DNS rebinding: a rebound hostname never matches a loopback Host.
        if loopback_only && !host_is_local(&request_header(&request, "Host")) {
            forbidden(request);
            continue;
        }
        if url.starts_with("/api/") && !is_ingest && request_header(&request, "X-Rescue-Token") != token {
            forbidden(request);
            continue;
        }

        if method == "OPTIONS" {
            let resp = Response::empty(200)
                .with_header(header_cors)
                .with_header(header_cors_methods)
                .with_header(header_cors_headers);
            let _ = request.respond(resp);
            continue;
        }

        if method == "GET" && (url == "/" || url.starts_with("/index")) {
            let resp = Response::from_string(HTML_UI.replace("__VERSION__", env!("CARGO_PKG_VERSION")).replace("__TOKEN__", &token)).with_header(header_html);
            let _ = request.respond(resp);
            continue;
        }

        if method == "GET" && (url == "/podcast-rescue-extension.zip" || url == "/podcast-rescue-extension-firefox.zip") {
            let firefox = url.contains("firefox");
            let name = if firefox { "podcast-rescue-extension-firefox.zip" } else { "podcast-rescue-extension.zip" };
            let resp = Response::from_data(crate::extension::extension_zip(firefox))
                .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/zip"[..]).unwrap())
                .with_header(
                    Header::from_bytes(
                        &b"Content-Disposition"[..],
                        format!("attachment; filename=\"{name}\"").as_bytes(),
                    )
                    .unwrap(),
                );
            let _ = request.respond(resp);
            continue;
        }

        if method == "GET" && url == "/api/poll-ingest" {
            let mut queue = ingest_queue.lock().unwrap();
            let shows: Vec<PodcastShow> = queue.drain(..).collect();
            let json_resp = serde_json::json!({
                "count": shows.len(),
                "shows": shows
            });
            let resp = Response::from_string(json_resp.to_string())
                .with_header(header_json)
                .with_header(header_cors);
            let _ = request.respond(resp);
            continue;
        }

        if method == "POST" && url == "/api/auto-ingest" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_str::<AutoIngestRequest>(&body) {
                    let mut queue = ingest_queue.lock().unwrap();
                    let count = req.shows.len();
                    queue.extend(req.shows.clone());

                    let json_resp = serde_json::json!({
                        "count": count,
                        "shows": req.shows,
                        "status": "queued"
                    });
                    let resp = Response::from_string(json_resp.to_string())
                        .with_header(header_json)
                        .with_header(header_cors);
                    let _ = request.respond(resp);
                    continue;
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/parse" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_str::<ParseRequest>(&body) {
                    let shows = parse_content(&req.content, req.platform.as_deref());
                    let json_resp = serde_json::json!({
                        "count": shows.len(),
                        "shows": shows
                    });
                    let resp = Response::from_string(json_resp.to_string())
                        .with_header(header_json)
                        .with_header(header_cors);
                    let _ = request.respond(resp);
                    continue;
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/scrape-url" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_str::<ScrapeUrlRequest>(&body) {
                    match scrape_url(&req.url) {
                        Ok(shows) => {
                            let json_resp = serde_json::json!({
                                "count": shows.len(),
                                "shows": shows
                            });
                            let resp = Response::from_string(json_resp.to_string())
                                .with_header(header_json)
                                .with_header(header_cors);
                            let _ = request.respond(resp);
                            continue;
                        }
                        Err(err) => {
                            let json_err = serde_json::json!({
                                "error": err
                            });
                            let resp = Response::from_string(json_err.to_string())
                                .with_header(header_json)
                                .with_header(header_cors)
                                .with_status_code(400);
                            let _ = request.respond(resp);
                            continue;
                        }
                    }
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/resolve" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_str::<ResolveRequest>(&body) {
                    let verify = req.verify_feeds.unwrap_or(false);
                    let mut results = Vec::new();

                    for show in req.shows {
                        let res = search_apple_podcasts(&show.title, show.author.as_deref(), verify);
                        results.push(res);
                    }

                    let json_resp = serde_json::json!({
                        "count": results.len(),
                        "results": results
                    });
                    let resp = Response::from_string(json_resp.to_string())
                        .with_header(header_json)
                        .with_header(header_cors);
                    let _ = request.respond(resp);
                    continue;
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/export/opml" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_str::<ExportOpmlRequest>(&body) {
                    let xml = generate_opml_xml(&req.items, 0);
                    let resp = Response::from_string(xml)
                        .with_header(header_xml)
                        .with_header(header_cors);
                    let _ = request.respond(resp);
                    continue;
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/export/json" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_str::<ExportOpmlRequest>(&body) {
                    let json = generate_json_export(&req.items);
                    let resp = Response::from_string(json)
                        .with_header(header_json)
                        .with_header(header_cors);
                    let _ = request.respond(resp);
                    continue;
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/export/csv" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_str::<ExportOpmlRequest>(&body) {
                    let csv = generate_csv_export(&req.items);
                    let resp = Response::from_string(csv)
                        .with_header(header_csv)
                        .with_header(header_cors);
                    let _ = request.respond(resp);
                    continue;
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/clean/opml" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_str::<CleanOpmlRequest>(&body) {
                    let clean_resp = clean_opml(&req.opml_content);
                    let json_resp = serde_json::to_string(&clean_resp).unwrap_or_default();
                    let resp = Response::from_string(json_resp)
                        .with_header(header_json)
                        .with_header(header_cors);
                    let _ = request.respond(resp);
                    continue;
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/export/unresolved" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(results) = serde_json::from_str::<Vec<ResolveResult>>(&body) {
                    let report = generate_unresolved_report(&results);
                    let resp = Response::from_string(report)
                        .with_header(header_text)
                        .with_header(header_cors);
                    let _ = request.respond(resp);
                    continue;
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        if method == "POST" && url == "/api/open-browser" {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_ok() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(target_url) = json.get("url").and_then(|u| u.as_str()) {
                        if target_url.starts_with("http://") || target_url.starts_with("https://") {
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("cmd").args(["/C", "start", "", target_url]).spawn();
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("open").arg(target_url).spawn();
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("xdg-open").arg(target_url).spawn();

                            let resp = Response::from_string("{\"status\":\"ok\"}")
                                .with_header(header_json)
                                .with_header(header_cors);
                            let _ = request.respond(resp);
                            continue;
                        }
                    }
                }
            }
            let _ = request.respond(Response::from_string("{\"error\": \"Invalid request\"}").with_status_code(400));
            continue;
        }

        let _ = request.respond(Response::from_string("404 Not Found").with_status_code(404));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_podcast_sites_may_post_cross_origin() {
        assert!(origin_allowed("https://www.podbean.com"));
        assert!(origin_allowed("https://open.spotify.com"));
        assert!(!origin_allowed("https://podbean.com.evil.com"));
        assert!(!origin_allowed("http://www.podbean.com"));
        assert!(!origin_allowed("https://evil.com"));
        assert!(!origin_allowed(""));
    }

    #[test]
    fn only_loopback_hosts_are_local() {
        assert!(host_is_local("127.0.0.1:3000"));
        assert!(host_is_local("localhost"));
        assert!(!host_is_local("evil.com:3000"));
        assert!(!host_is_local("127.0.0.1.evil.com"));
    }
}
