#!/usr/bin/env python3
"""
rescue.py - Podbean to OPML Subscription Rescue Tool
Extracts followed podcasts from a Podbean HTTP Archive (.har),
queries the Apple Podcasts Directory API for canonical RSS feed URLs,
safely disambiguates duplicate/same-name shows using publisher matching,
detects moved feeds (HTTP redirects and <itunes:new-feed-url>),
and generates an OPML 1.0 XML file without silent mismatch errors.

Zero external dependencies - uses standard Python library only.
"""

import os
import sys
import json
import time
import html
import re
import socket
import argparse
import urllib.request
import urllib.parse
import urllib.error
import xml.etree.ElementTree as ET
from datetime import datetime, timezone

# Ensure IPv4 resolution to prevent Akamai/Apple CDN IPv6 rate-limit/403 blocks,
# with graceful fallback if IPv4 resolution fails.
_orig_getaddrinfo = socket.getaddrinfo
def _getaddrinfo_v4(host, port, family=0, type=0, proto=0, flags=0):
    try:
        res = [r for r in _orig_getaddrinfo(host, port, socket.AF_INET, type, proto, flags)]
        if res:
            return res
    except socket.gaierror:
        pass
    return _orig_getaddrinfo(host, port, family, type, proto, flags)
socket.getaddrinfo = _getaddrinfo_v4


class PodcastInfo:
    """Represents a podcast entry extracted from the Podbean archive."""
    def __init__(self, title, author=None):
        self.title = title.strip()
        self.author = author.strip() if author else None

    def __repr__(self):
        return f"PodcastInfo(title={self.title!r}, author={self.author!r})"


def normalize_title(title):
    """Normalizes title string by stripping punctuation and redundant whitespace."""
    if not title:
        return ""
    t = html.unescape(title).strip().lower()
    t = re.sub(r'[^\w\s]', '', t)
    return " ".join(t.split())


def authors_match(author_a, author_b):
    """
    Checks if two author / publisher names match or have significant overlap,
    ignoring case, punctuation, corporate suffixes, and common stopwords.
    """
    if not author_a or not author_b:
        return False
    a = author_a.strip().lower()
    b = author_b.strip().lower()
    if a == b:
        return True

    clean_a = re.sub(r'[^\w\s]', '', a)
    clean_b = re.sub(r'[^\w\s]', '', b)
    if clean_a == clean_b or clean_a in clean_b or clean_b in clean_a:
        return True

    stop_words = {
        "the", "a", "an", "and", "&", "inc", "llc", "ltd", "corp",
        "corporation", "media", "studios", "studio", "productions",
        "production", "network", "podcast", "podcasts", "group", "radio"
    }
    words_a = set(clean_a.split()) - stop_words
    words_b = set(clean_b.split()) - stop_words

    if words_a and words_b and (words_a.issubset(words_b) or words_b.issubset(words_a)):
        return True
    return False


def extract_podcasts(har_path):
    """
    Parses a HAR file and extracts podcast titles along with author/publisher
    metadata from JSON responses and HTML card components.
    Returns a dict mapping normalized title -> PodcastInfo.
    """
    resolved_path = os.path.abspath(har_path)
    if not os.path.isfile(resolved_path):
        raise FileNotFoundError(f"HAR file not found: {resolved_path}")

    print(f"[*] Reading HTTP Archive: {resolved_path}...")
    with open(resolved_path, "r", encoding="utf-8", errors="ignore") as f:
        har_data = json.load(f)

    entries = har_data.get("log", {}).get("entries", [])
    print(f"[*] Found {len(entries)} recorded network entries.")

    podcasts = {}

    def record_podcast(title, author=None):
        cleaned_title = html.unescape(title.strip()) if title else ""
        cleaned_author = html.unescape(author.strip()) if author else None

        if not cleaned_title or len(cleaned_title) < 2 or cleaned_title.lower().startswith("http"):
            return

        norm = cleaned_title.lower()
        if norm not in podcasts:
            podcasts[norm] = PodcastInfo(cleaned_title, cleaned_author)
        elif cleaned_author and not podcasts[norm].author:
            podcasts[norm].author = cleaned_author

    for entry in entries:
        response = entry.get("response", {})
        content = response.get("content", {})
        text = content.get("text", "")
        if not text:
            continue

        mime_type = content.get("mimeType", "").lower()

        # 1. Parse JSON response bodies (API endpoints)
        if "json" in mime_type or text.strip().startswith(("{", "[")):
            try:
                data = json.loads(text)
                def scan_json(obj):
                    if isinstance(obj, dict):
                        local_title = None
                        local_author = None
                        for k, v in obj.items():
                            if not isinstance(v, str):
                                continue
                            k_clean = k.lower().replace("_", "").replace("-", "")
                            if k_clean in ("podcasttitle", "title", "podcastname", "collectionname"):
                                local_title = v
                            elif k_clean in ("author", "authorname", "artistname", "owner", "publisher", "username", "creator"):
                                local_author = v

                        if local_title:
                            record_podcast(local_title, local_author)

                        for v in obj.values():
                            scan_json(v)
                    elif isinstance(obj, list):
                        for item in obj:
                            scan_json(item)
                scan_json(data)
            except Exception:
                pass

        # 2. Parse HTML response bodies (Web & PJAX podcast cards)
        if "card-title" in text:
            matches = re.findall(r'<a[^>]*class=\"[^\"]*card-title[^\"]*\"[^>]*>(.*?)</a>', text, re.DOTALL)
            for raw_title in matches:
                record_podcast(raw_title)

        # Attribute-based titles and authors in HTML
        data_matches = re.finditer(r'data-dir-podcast-title=\"([^\"]+)\"(?:[^>]*data-dir-author=\"([^\"]+)\")?', text)
        for m in data_matches:
            record_podcast(m.group(1), m.group(2) if m.lastindex >= 2 else None)

    return podcasts


def extract_podcast_titles(har_path):
    """
    Legacy compatibility helper: parses a HAR file and returns a set of unique titles.
    """
    podcasts = extract_podcasts(har_path)
    return set(p.title for p in podcasts.values())


def check_feed_status(feed_url, timeout=8):
    """
    Inspects a podcast RSS feed URL over HTTP:
    - Follows HTTP 301/302/307/308 redirects to capture moved feeds.
    - Reads the feed header to check for an explicit <itunes:new-feed-url> tag.
    Returns: (effective_url, was_moved, error_message)
    """
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Accept": "application/rss+xml, application/xml, text/xml, */*"
    }
    req = urllib.request.Request(feed_url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            final_url = resp.geturl()
            snippet = resp.read(8192).decode("utf-8", errors="ignore")
            m = re.search(r'<itunes:new-feed-url>\s*(https?://[^\s<]+)\s*</itunes:new-feed-url>', snippet, re.IGNORECASE)
            new_feed = m.group(1).strip() if m else None

            effective_url = new_feed if new_feed else final_url
            moved = (effective_url.rstrip("/") != feed_url.rstrip("/"))
            return effective_url, moved, None
    except urllib.error.HTTPError as e:
        return feed_url, False, f"HTTP {e.code}"
    except Exception as e:
        return feed_url, False, str(e)


def search_apple_podcasts(title, author=None, max_retries=5):
    """
    Queries the Apple Podcasts Search API to discover the RSS feedUrl.
    Compares show title and publisher/artist to prevent ambiguous collisions
    or incorrect blind matches.

    Returns a dict:
    {
        "status": "RESOLVED" | "AMBIGUOUS" | "NOT_FOUND",
        "feed_url": str or None,
        "matched_title": str or None,
        "artist": str or None,
        "candidates": list of dicts,
        "reason": str
    }
    """
    base_url = "https://itunes.apple.com/search"
    params = {
        "media": "podcast",
        "entity": "podcast",
        "term": title,
        "limit": 8
    }
    url = f"{base_url}?{urllib.parse.urlencode(params)}"
    headers = {
        "User-Agent": "iTunes/12.11.3 (Windows; Microsoft Windows 10 x64)",
        "Accept": "*/*"
    }

    req = urllib.request.Request(url, headers=headers)
    results = []

    for attempt in range(max_retries):
        try:
            with urllib.request.urlopen(req, timeout=12) as response:
                if response.status == 200:
                    payload = json.loads(response.read().decode("utf-8"))
                    results = payload.get("results", [])
                    break
        except urllib.error.HTTPError as e:
            if e.code in (429, 403, 503):
                wait_time = (attempt + 1) * 3.5
                time.sleep(wait_time)
            else:
                return {
                    "status": "NOT_FOUND",
                    "feed_url": None,
                    "matched_title": None,
                    "artist": None,
                    "candidates": [],
                    "reason": f"Apple API error (HTTP {e.code})"
                }
        except Exception:
            if attempt == max_retries - 1:
                return {
                    "status": "NOT_FOUND",
                    "feed_url": None,
                    "matched_title": None,
                    "artist": None,
                    "candidates": [],
                    "reason": "Connection timeout or network failure"
                }
            time.sleep(2.0)

    # Filter to results that actually contain a feedUrl
    valid_candidates = [r for r in results if r.get("feedUrl")]
    if not valid_candidates:
        return {
            "status": "NOT_FOUND",
            "feed_url": None,
            "matched_title": None,
            "artist": None,
            "candidates": [],
            "reason": "No valid feeds returned by directory"
        }

    norm_target = normalize_title(title)
    target_lower = title.strip().lower()

    # Partition candidates by match type
    exact_title_matches = []
    close_matches = []

    for cand in valid_candidates:
        cand_title = cand.get("collectionName", "").strip()
        cand_lower = cand_title.lower()
        cand_norm = normalize_title(cand_title)

        if cand_lower == target_lower or cand_norm == norm_target:
            exact_title_matches.append(cand)
        elif (cand_lower.startswith(target_lower + ":") or
              cand_lower.startswith(target_lower + " - ") or
              target_lower.startswith(cand_lower + ":") or
              target_lower.startswith(cand_lower + " - ")):
            close_matches.append(cand)

    # 1. Single exact title match
    if len(exact_title_matches) == 1:
        chosen = exact_title_matches[0]
        return {
            "status": "RESOLVED",
            "feed_url": chosen.get("feedUrl"),
            "matched_title": chosen.get("collectionName", title),
            "artist": chosen.get("artistName", ""),
            "candidates": exact_title_matches,
            "reason": "Exact title match"
        }

    # 2. Multiple exact title matches (Same show name collision)
    if len(exact_title_matches) > 1:
        if author:
            # Attempt disambiguation via publisher/artist
            author_matches = [
                c for c in exact_title_matches
                if authors_match(author, c.get("artistName", ""))
            ]
            if len(author_matches) == 1:
                chosen = author_matches[0]
                return {
                    "status": "RESOLVED",
                    "feed_url": chosen.get("feedUrl"),
                    "matched_title": chosen.get("collectionName", title),
                    "artist": chosen.get("artistName", ""),
                    "candidates": exact_title_matches,
                    "reason": f"Disambiguated duplicate title using publisher: '{chosen.get('artistName')}'"
                }

        # Multiple shows share the exact name and publisher is ambiguous
        return {
            "status": "AMBIGUOUS",
            "feed_url": None,
            "matched_title": None,
            "artist": None,
            "candidates": exact_title_matches,
            "reason": f"Multiple ({len(exact_title_matches)}) shows found with this exact title"
        }

    # 3. No exact title matches: evaluate close/subtitle matches
    if close_matches:
        if author:
            close_author_matches = [
                c for c in close_matches
                if authors_match(author, c.get("artistName", ""))
            ]
            if len(close_author_matches) == 1:
                chosen = close_author_matches[0]
                return {
                    "status": "RESOLVED",
                    "feed_url": chosen.get("feedUrl"),
                    "matched_title": chosen.get("collectionName", title),
                    "artist": chosen.get("artistName", ""),
                    "candidates": close_matches,
                    "reason": "Matched subtitle prefix and publisher"
                }

        return {
            "status": "AMBIGUOUS",
            "feed_url": None,
            "matched_title": None,
            "artist": None,
            "candidates": close_matches,
            "reason": "No exact title match; candidate near-matches found"
        }

    # 4. No matching podcast found (do not silently fall back to unrelated results)
    return {
        "status": "NOT_FOUND",
        "feed_url": None,
        "matched_title": None,
        "artist": None,
        "candidates": [],
        "reason": "No matching podcast found in directory"
    }


def query_apple_podcasts(title, max_retries=5):
    """
    Legacy compatibility wrapper: queries Apple Podcasts and returns (feedUrl, collectionName).
    Returns (None, None) if not resolved.
    """
    result = search_apple_podcasts(title, author=None, max_retries=max_retries)
    if result["status"] == "RESOLVED":
        return result["feed_url"], result["matched_title"]
    return None, None


def generate_opml(podcast_feeds, output_path="podbean_rescue.opml", unresolved_count=0):
    """
    Constructs an OPML 1.0 XML file containing the resolved podcast RSS feeds.
    """
    opml = ET.Element("opml", version="1.0")
    head = ET.SubElement(opml, "head")

    title_elem = ET.SubElement(head, "title")
    title_elem.text = "Podbean Subscriptions"

    date_elem = ET.SubElement(head, "dateCreated")
    date_elem.text = datetime.now(timezone.utc).strftime("%a, %d %b %Y %H:%M:%S GMT")

    body = ET.SubElement(opml, "body")

    if unresolved_count > 0:
        comment = ET.Comment(
            f" Note: {unresolved_count} subscription(s) were ambiguous or unresolved. "
            "See unresolved report for manual review. "
        )
        body.append(comment)

    sorted_feeds = sorted(podcast_feeds, key=lambda x: x["title"].lower())

    for item in sorted_feeds:
        ET.SubElement(
            body,
            "outline",
            text=item["title"],
            title=item["title"],
            type="rss",
            xmlUrl=item["feedUrl"],
            htmlUrl=item.get("htmlUrl", "")
        )

    if hasattr(ET, "indent"):
        ET.indent(opml, space="  ")
        pretty_xml = ET.tostring(opml, encoding="utf-8", xml_declaration=True)
    else:
        pretty_xml = ET.tostring(opml, encoding="utf-8")

    resolved_output = os.path.abspath(output_path)
    with open(resolved_output, "wb") as f:
        f.write(pretty_xml)

    print(f"\n[+] OPML file successfully saved to: {output_path}")


def write_unresolved_report(unresolved_shows, ambiguous_shows, output_path="podbean_unresolved.txt"):
    """
    Writes a structured text report detailing ambiguous and unresolved shows,
    including candidate feeds and authors, to facilitate quick manual review.
    """
    resolved_output = os.path.abspath(output_path)
    with open(resolved_output, "w", encoding="utf-8") as f:
        f.write("# Podbean to OPML - Unresolved Subscriptions Report\n")
        f.write(f"# Generated: {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S UTC')}\n")
        f.write("# This file lists shows that could not be automatically resolved with high confidence.\n")
        f.write("# Review the candidate feeds below to manually add them to your podcast app.\n\n")

        if ambiguous_shows:
            f.write("=" * 65 + "\n")
            f.write(" AMBIGUOUS SUBSCRIPTIONS (Multiple or Near Matches Found)\n")
            f.write("=" * 65 + "\n\n")
            for item in ambiguous_shows:
                title = item["title"]
                author = item.get("author") or "Not specified in archive"
                reason = item.get("reason", "Multiple candidates found")
                candidates = item.get("candidates", [])

                f.write(f"Show Title: {title}\n")
                f.write(f"Publisher:  {author}\n")
                f.write(f"Status:     {reason}\n")
                f.write("Candidate Feeds in Directory:\n")
                for idx, cand in enumerate(candidates, 1):
                    cand_name = cand.get("collectionName", "Unknown Show")
                    cand_artist = cand.get("artistName", "Unknown Publisher")
                    feed_url = cand.get("feedUrl", "No URL")
                    f.write(f"  [{idx}] {cand_name} (by {cand_artist})\n")
                    f.write(f"      Feed: {feed_url}\n")
                f.write("\n" + "-" * 40 + "\n\n")

        if unresolved_shows:
            f.write("=" * 65 + "\n")
            f.write(" UNRESOLVED SUBSCRIPTIONS (No Directory Matches Found)\n")
            f.write("=" * 65 + "\n\n")
            for item in unresolved_shows:
                title = item["title"]
                author = item.get("author") or "Not specified in archive"
                reason = item.get("reason", "Not found in directory")
                f.write(f"Show Title: {title}\n")
                f.write(f"Publisher:  {author}\n")
                f.write(f"Status:     {reason}\n")
                f.write("Note:       May be a private feed, exclusive to Podbean, or removed from Apple Podcasts.\n\n")

    print(f"[+] Unresolved report written to: {output_path}")


def prompt_user_choice(podcast, candidates):
    """
    Interactively prompts the user to choose among candidate matches.
    """
    print(f"\n [?] Ambiguous match for '{podcast.title}' (Extracted Publisher: {podcast.author or 'None'}):")
    limit = min(len(candidates), 5)
    for idx in range(limit):
        cand = candidates[idx]
        print(f"     [{idx + 1}] {cand.get('collectionName')} (by {cand.get('artistName', 'Unknown')})")
        print(f"         Feed: {cand.get('feedUrl')}")
    print("     [s] Skip / leave unresolved for manual review")

    while True:
        try:
            choice = input(f"     Select [1-{limit}, s]: ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print("\nSkipping...")
            return None
        if choice in ("s", ""):
            return None
        if choice.isdigit() and 1 <= int(choice) <= limit:
            return candidates[int(choice) - 1]
        print(f"     Invalid selection. Please choose 1-{limit} or 's' to skip.")


def main():
    parser = argparse.ArgumentParser(
        description="Rescue podcast subscriptions from a Podbean HTTP Archive (HAR) file into OPML without silent mismatch errors."
    )
    parser.add_argument(
        "har_file",
        nargs="?",
        default="podbean.har",
        help="Path to the Podbean .har file (default: podbean.har)"
    )
    parser.add_argument(
        "-o", "--output",
        default="podbean_rescue.opml",
        help="Output OPML filename (default: podbean_rescue.opml)"
    )
    parser.add_argument(
        "-u", "--unresolved",
        default="podbean_unresolved.txt",
        help="Output report for unresolved and ambiguous subscriptions (default: podbean_unresolved.txt)"
    )
    parser.add_argument(
        "-i", "--interactive",
        action="store_true",
        default=False,
        help="Prompt interactively to choose between ambiguous matches or confirm near-matches"
    )
    parser.add_argument(
        "--non-interactive",
        action="store_true",
        default=False,
        help="Disable interactive prompts even when running in a terminal"
    )
    parser.add_argument(
        "--verify-feeds",
        action="store_true",
        default=False,
        help="Verify resolved RSS feeds over HTTP to follow redirects and detect moved feeds (<itunes:new-feed-url>)"
    )
    args = parser.parse_args()

    # Determine whether interactive prompting should be enabled
    interactive_mode = args.interactive or (sys.stdin.isatty() and not args.non_interactive)

    print("=" * 65)
    print(" Podbean to OPML Subscription Rescue Tool")
    print("=" * 65)

    podcasts_map = extract_podcasts(args.har_file)
    total_shows = len(podcasts_map)
    print(f"\n[*] Extracted and deduplicated {total_shows} unique podcasts.")

    sorted_podcasts = sorted(podcasts_map.values(), key=lambda p: p.title.lower())

    resolved_feeds = {}
    ambiguous_shows = []
    unresolved_shows = []

    print(f"[*] Querying Apple Podcasts API for canonical RSS feeds...\n")

    for idx, podcast in enumerate(sorted_podcasts, start=1):
        author_display = f" (by {podcast.author})" if podcast.author else ""
        print(f" [{idx:02d}/{total_shows:02d}] Searching: '{podcast.title}'{author_display}...", end=" ", flush=True)

        res = search_apple_podcasts(podcast.title, author=podcast.author)
        status = res["status"]

        if status == "RESOLVED":
            feed_url = res["feed_url"]
            print("FOUND")
            if res.get("reason") and "Disambiguated" in res["reason"]:
                print(f"       -> {res['reason']}")

            if args.verify_feeds and feed_url:
                print(f"       Verifying feed status...", end=" ", flush=True)
                eff_url, moved, err = check_feed_status(feed_url)
                if err:
                    print(f"WARN ({err})")
                elif moved:
                    print(f"MOVED -> {eff_url}")
                    feed_url = eff_url
                else:
                    print("OK")

            print(f"       Feed: {feed_url}")
            resolved_feeds[podcast.title] = {
                "title": res["matched_title"] or podcast.title,
                "feedUrl": feed_url
            }

        elif status == "AMBIGUOUS":
            print("AMBIGUOUS")
            chosen = None
            if interactive_mode:
                chosen = prompt_user_choice(podcast, res["candidates"])

            if chosen:
                chosen_feed = chosen.get("feedUrl")
                if args.verify_feeds and chosen_feed:
                    eff_url, moved, _ = check_feed_status(chosen_feed)
                    if moved:
                        chosen_feed = eff_url
                print(f"       Selected: '{chosen.get('collectionName')}' (Feed: {chosen_feed})")
                resolved_feeds[podcast.title] = {
                    "title": chosen.get("collectionName", podcast.title),
                    "feedUrl": chosen_feed
                }
            else:
                ambiguous_shows.append({
                    "title": podcast.title,
                    "author": podcast.author,
                    "reason": res.get("reason", "Multiple candidates"),
                    "candidates": res.get("candidates", [])
                })

        else:  # NOT_FOUND
            print("NOT FOUND")
            unresolved_shows.append({
                "title": podcast.title,
                "author": podcast.author,
                "reason": res.get("reason", "No directory match found")
            })

        time.sleep(0.7)

    # Automatic retry pass for temporary network rate limits on unresolved shows
    if unresolved_shows:
        retry_candidates = [item for item in unresolved_shows if "timeout" in item["reason"].lower() or "error" in item["reason"].lower()]
        if retry_candidates:
            print(f"\n[*] Waiting 5s before retrying {len(retry_candidates)} throttled shows...")
            time.sleep(5)
            remaining_unresolved = []
            for item in unresolved_shows:
                if item not in retry_candidates:
                    remaining_unresolved.append(item)
                    continue
                print(f" [Retry] Searching: '{item['title']}'...", end=" ", flush=True)
                res = search_apple_podcasts(item["title"], author=item.get("author"), max_retries=6)
                if res["status"] == "RESOLVED":
                    print("FOUND")
                    print(f"       Feed: {res['feed_url']}")
                    resolved_feeds[item["title"]] = {
                        "title": res["matched_title"] or item["title"],
                        "feedUrl": res["feed_url"]
                    }
                else:
                    print("NOT FOUND")
                    remaining_unresolved.append(item)
                time.sleep(1.0)
            unresolved_shows = remaining_unresolved

    total_unresolved = len(ambiguous_shows) + len(unresolved_shows)
    feed_list = list(resolved_feeds.values())

    generate_opml(feed_list, args.output, unresolved_count=total_unresolved)

    if total_unresolved > 0:
        write_unresolved_report(unresolved_shows, ambiguous_shows, args.unresolved)

    print("\n" + "=" * 65)
    print(" RESCUE SUMMARY")
    print("=" * 65)
    print(f" Total Unique Podcasts:   {total_shows}")
    print(f" Successfully Resolved:   {len(feed_list)}")
    print(f" Ambiguous Matches:       {len(ambiguous_shows)} (candidates logged for review)")
    print(f" Unresolved (Not Found):  {len(unresolved_shows)}")
    print("=" * 65)

    if total_unresolved > 0:
        print(f"\n[!] {total_unresolved} subscription(s) could not be resolved automatically.")
        print(f"    See '{args.unresolved}' for candidate links and manual subscription details.")


if __name__ == "__main__":
    main()
