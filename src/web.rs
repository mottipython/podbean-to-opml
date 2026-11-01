use crate::opml::{generate_opml_xml, generate_unresolved_report};
use crate::parser::parse_content;
use crate::resolver::search_apple_podcasts;
use crate::types::{ExportOpmlRequest, ParseRequest, ResolveRequest, ResolveResult};
use std::sync::Arc;
use tiny_http::{Header, Response, Server};

const HTML_UI: &str = include_str!("web_ui.html");

pub fn run_web_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr).map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
    let server = Arc::new(server);

    println!("=================================================================");
    println!(" 🚀 Podcast Rescue GUI Server running at http://127.0.0.1:{}", port);
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
        let header_cors = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();

        if method == "GET" && (url == "/" || url.starts_with("/index")) {
            let resp = Response::from_string(HTML_UI).with_header(header_html);
            let _ = request.respond(resp);
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

        let _ = request.respond(Response::from_string("404 Not Found").with_status_code(404));
    }

    Ok(())
}
