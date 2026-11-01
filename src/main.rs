use podcast_rescue::browser::open_browser;
use podcast_rescue::opml::{
    clean_opml, generate_csv_export, generate_json_export, generate_opml_xml, generate_unresolved_report,
};
use podcast_rescue::parser::parse_content;
use podcast_rescue::resolver::search_apple_podcasts;
use podcast_rescue::types::{ExportItem, ResolveStatus};
use podcast_rescue::web::run_web_server;
use std::env;
use std::fs;
use std::path::Path;

fn print_banner() {
    println!("=================================================================");
    println!(" Podcast Rescue - Universal Subscriptions & OPML Exporter");
    println!(" Works across Podbean, Spotify, Apple Podcasts & YouTube Music");
    println!("=================================================================");
}

fn print_help() {
    print_banner();
    println!("Usage:");
    println!("  podcast-rescue [OPTIONS] [FILE]\n");
    println!("Default Mode (Web GUI with Browser Walkthrough):");
    println!("  podcast-rescue                  Starts local web server and opens default browser");
    println!("  podcast-rescue --port 8080      Runs on a custom port");
    println!("  podcast-rescue --host 0.0.0.0   Binds to a custom network host (e.g. Docker)\n");
    println!("Terminal / Headless CLI Mode:");
    println!("  podcast-rescue <file.har>       Extracts and resolves subscriptions directly in terminal");
    println!("  podcast-rescue <file.har> -o my.opml -u unresolved.txt");
    println!("  podcast-rescue <file.har> -o my.json           Export to JSON");
    println!("  podcast-rescue <file.har> -o my.csv            Export to CSV spreadsheet");
    println!("  podcast-rescue --verify-feeds   Pings and verifies resolved RSS feeds");
    println!("  podcast-rescue --clean-opml <old.opml> [-o clean.opml]   Validates & fixes existing OPML");
    println!("  podcast-rescue --version        Show version information");
    println!("  podcast-rescue --help           Show this help message\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("podcast-rescue {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    if let Some(pos) = args.iter().position(|a| a == "--clean-opml") {
        return run_clean_opml_mode(&args, pos);
    }

    let is_cli = args.iter().any(|a| a == "--cli")
        || (args.len() > 1 && !args[1].starts_with('-') && Path::new(&args[1]).is_file());

    if is_cli {
        return run_cli_mode(&args);
    }

    // Default mode: launch Web GUI and open browser
    print_banner();

    let mut port = 3000;
    let mut host = "127.0.0.1".to_string();

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(p) = args[i + 1].parse::<u16>() {
                port = p;
            }
            i += 1;
        } else if args[i] == "--host" && i + 1 < args.len() {
            host = args[i + 1].clone();
            i += 1;
        }
        i += 1;
    }

    let no_browser = args.iter().any(|a| a == "--no-browser") || host == "0.0.0.0";
    let url = format!("http://{}:{}", if host == "0.0.0.0" { "localhost" } else { &host }, port);

    if !no_browser {
        println!("[*] Opening your browser to: {}", url);
        open_browser(&url);
    }

    run_web_server(&host, port)
}

fn run_clean_opml_mode(args: &[String], pos: usize) -> Result<(), Box<dyn std::error::Error>> {
    print_banner();

    if pos + 1 >= args.len() {
        eprintln!("Error: --clean-opml requires a path to an OPML file.");
        std::process::exit(1);
    }

    let input_path = &args[pos + 1];
    let mut output_path = "cleaned_podcasts.opml";

    let mut i = 1;
    while i < args.len() {
        if (args[i] == "-o" || args[i] == "--output") && i + 1 < args.len() {
            output_path = &args[i + 1];
            i += 1;
        }
        i += 1;
    }

    println!("[*] Loading and parsing existing OPML file: {}", input_path);
    let content = fs::read_to_string(input_path)
        .map_err(|e| format!("Could not read file '{}': {}", input_path, e))?;

    println!("[*] Checking feed health, following 301/308 redirects, and detecting <itunes:new-feed-url>...\n");
    let resp = clean_opml(&content);

    for (idx, item) in resp.items.iter().enumerate() {
        print!(" [{:02}/{:02}] {} ... ", idx + 1, resp.items.len(), item.title);
        if item.is_alive {
            if item.was_moved {
                println!("RELOCATED -> {}", item.current_feed_url);
            } else {
                println!("ACTIVE");
            }
        } else {
            println!("DEAD ({})", item.error.as_deref().unwrap_or("unreachable"));
        }
    }

    fs::write(output_path, resp.cleaned_opml)?;
    println!("\n[+] Cleaned OPML saved to: {}", output_path);
    println!("=================================================================");
    println!(" OPML HEALTH SUMMARY");
    println!("=================================================================");
    println!(" Total Feeds Checked:   {}", resp.items.len());
    println!(" Active Feeds:          {}", resp.alive_count);
    println!(" Relocated Feeds Fixed: {}", resp.moved_count);
    println!(" Dead / Inactive Feeds: {}", resp.dead_count);
    println!("=================================================================");

    Ok(())
}

fn run_cli_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    print_banner();

    let mut file_path = "podbean.har";
    let mut output_file = "podcasts_rescue.opml";
    let mut output_unresolved = "unresolved_report.txt";
    let mut verify_feeds = false;
    let mut format_override: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--cli" => {}
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_file = &args[i + 1];
                    i += 1;
                }
            }
            "-u" | "--unresolved" => {
                if i + 1 < args.len() {
                    output_unresolved = &args[i + 1];
                    i += 1;
                }
            }
            "--format" => {
                if i + 1 < args.len() {
                    format_override = Some(args[i + 1].to_lowercase());
                    i += 1;
                }
            }
            "--verify-feeds" => {
                verify_feeds = true;
            }
            other if !other.starts_with('-') => {
                file_path = other;
            }
            _ => {}
        }
        i += 1;
    }

    println!("[*] Reading archive: {}...", file_path);
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Could not read file '{}': {}", file_path, e))?;

    let shows = parse_content(&content, None);
    println!("[*] Extracted {} unique podcast shows.", shows.len());

    let mut resolved_items = Vec::new();
    let mut all_results = Vec::new();

    println!("[*] Querying Apple Podcasts Directory API for canonical RSS feeds...\n");

    for (idx, show) in shows.iter().enumerate() {
        let author_str = show.author.as_deref().map(|a| format!(" (by {})", a)).unwrap_or_default();
        print!(" [{:02}/{:02}] Searching: '{}'{}... ", idx + 1, shows.len(), show.title, author_str);

        let res = search_apple_podcasts(&show.title, show.author.as_deref(), verify_feeds);
        match res.status {
            ResolveStatus::Resolved => {
                let feed = res.feed_url.clone().unwrap_or_default();
                if res.was_moved {
                    println!("MOVED -> {}", feed);
                } else {
                    println!("FOUND\n       Feed: {}", feed);
                }
                resolved_items.push(ExportItem {
                    title: res.matched_title.clone().unwrap_or_else(|| show.title.clone()),
                    feed_url: feed,
                });
            }
            ResolveStatus::Ambiguous => {
                println!("AMBIGUOUS ({} candidate feeds found)", res.candidates.len());
            }
            ResolveStatus::NotFound => {
                println!("NOT FOUND ({})", res.reason);
            }
        }
        all_results.push(res);
    }

    let unresolved_count = shows.len() - resolved_items.len();
    
    // Determine export format
    let is_json = format_override.as_deref() == Some("json") || output_file.ends_with(".json");
    let is_csv = format_override.as_deref() == Some("csv") || output_file.ends_with(".csv");

    if is_json {
        let json = generate_json_export(&resolved_items);
        fs::write(output_file, json)?;
        println!("\n[+] JSON export saved to: {}", output_file);
    } else if is_csv {
        let csv = generate_csv_export(&resolved_items);
        fs::write(output_file, csv)?;
        println!("\n[+] CSV export saved to: {}", output_file);
    } else {
        let xml = generate_opml_xml(&resolved_items, unresolved_count);
        fs::write(output_file, xml)?;
        println!("\n[+] OPML file saved to: {}", output_file);
    }

    if unresolved_count > 0 {
        let report = generate_unresolved_report(&all_results);
        fs::write(output_unresolved, report)?;
        println!("[+] Unresolved report saved to: {}", output_unresolved);
    }

    println!("\n=================================================================");
    println!(" RESCUE SUMMARY");
    println!("=================================================================");
    println!(" Total Shows Extracted:   {}", shows.len());
    println!(" Successfully Resolved:   {}", resolved_items.len());
    println!(" Unresolved / Ambiguous:  {}", unresolved_count);
    println!("=================================================================");

    Ok(())
}
