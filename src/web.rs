use crate::opml::{
    clean_opml, generate_csv_export, generate_json_export, generate_opml_xml, generate_unresolved_report,
};
use crate::parser::{parse_content, scrape_url};
use crate::resolver::search_apple_podcasts;
use crate::types::{
    AutoIngestRequest, CleanOpmlRequest, ExportOpmlRequest, ParseRequest, PodcastShow, ResolveRequest,
    ResolveResult, ScrapeUrlRequest,
};
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Response, Server};

const HTML_UI: &str = include_str!("web_ui.html");

pub fn run_web_server(host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", host, port);
    let server = Server::http(&addr).map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
    let server = Arc::new(server);
    let ingest_queue: Arc<Mutex<Vec<PodcastShow>>> = Arc::new(Mutex::new(Vec::new()));

    let display_host = if host == "0.0.0.0" { "localhost" } else { host };
    println!("=================================================================");
    println!(" 🚀 Podcast Rescue Cockpit running at http://{}:{}", display_host, port);
    println!(" Open this URL in your web browser if it did not open automatically.");
    println!(" Press Ctrl+C in terminal to stop.");
    println!("=================================================================");

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().as_str().to_uppercase();

        let header_html = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
        let header_json = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
        let header_xml = Header::from_bytes(&b"Content-Type"[..], &b"text/xml; charset=utf-8"[..]).unwrap();
        let header_text = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
        let header_csv = Header::from_bytes(&b"Content-Type"[..], &b"text/csv; charset=utf-8"[..]).unwrap();
        let header_cors = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
        let header_cors_methods = Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap();
        let header_cors_headers = Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap();

        if method == "OPTIONS" {
            let resp = Response::empty(200)
                .with_header(header_cors)
                .with_header(header_cors_methods)
                .with_header(header_cors_headers);
            let _ = request.respond(resp);
            continue;
        }

        if method == "GET" && (url == "/" || url.starts_with("/index")) {
            let resp = Response::from_string(HTML_UI).with_header(header_html);
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
