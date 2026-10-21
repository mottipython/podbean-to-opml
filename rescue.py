#!/usr/bin/env python3
"""
rescue.py - Podbean to OPML Subscription Rescue Tool
Extracts followed podcasts from a Podbean HTTP Archive (.har),
deduplicates show titles using a set, queries the Apple Podcasts
API for RSS feed URLs, and generates an OPML 1.0 XML file.

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
from xml.dom import minidom
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


def extract_podcast_titles(har_path):
    """
    Parses a HAR file and extracts podcast titles from JSON responses
    and HTML card components. Deduplicates titles using a Python set.
    """
    if not os.path.isfile(har_path):
        raise FileNotFoundError(f"HAR file not found: {har_path}")

    print(f"[*] Reading HTTP Archive: {har_path}...")
    with open(har_path, "r", encoding="utf-8", errors="ignore") as f:
        har_data = json.load(f)

    entries = har_data.get("log", {}).get("entries", [])
    print(f"[*] Found {len(entries)} recorded network entries.")

    # Deduplicate podcast titles using a Python set
    unique_titles = set()

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
                        for k, v in obj.items():
                            if k.lower() in ("podcast_title", "title", "podcastname", "collectionname") and isinstance(v, str):
                                cleaned = html.unescape(v.strip())
                                if cleaned and len(cleaned) > 1:
                                    unique_titles.add(cleaned)
                            else:
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
                title = html.unescape(raw_title.strip())
                if title:
                    unique_titles.add(title)

        # Attribute-based titles
        data_title_matches = re.findall(r'data-dir-podcast-title=\"([^\"]+)\"', text)
        for raw_title in data_title_matches:
            title = html.unescape(raw_title.strip())
            if title:
                unique_titles.add(title)

    return unique_titles


def query_apple_podcasts(title, max_retries=5):
    """
    Queries the Apple Podcasts Search API to discover the RSS feedUrl
    for a podcast title. Includes exponential backoff for rate limits.
    """
    base_url = "https://itunes.apple.com/search"
    params = {
        "media": "podcast",
        "entity": "podcast",
        "term": title,
        "limit": 5
    }
    url = f"{base_url}?{urllib.parse.urlencode(params)}"
    headers = {
        "User-Agent": "iTunes/12.11.3 (Windows; Microsoft Windows 10 x64)",
        "Accept": "*/*"
    }

    req = urllib.request.Request(url, headers=headers)

    for attempt in range(max_retries):
        try:
            with urllib.request.urlopen(req, timeout=12) as response:
                if response.status == 200:
                    payload = json.loads(response.read().decode("utf-8"))
                    results = payload.get("results", [])
                    if not results:
                        return None, None

                    # Check for exact case-insensitive title match first
                    for res in results:
                        col_name = res.get("collectionName", "")
                        feed = res.get("feedUrl")
                        if col_name.strip().lower() == title.lower() and feed:
                            return feed, col_name

                    # Fall back to first result containing a valid feedUrl
                    for res in results:
                        feed = res.get("feedUrl")
                        if feed:
                            return feed, res.get("collectionName", title)

                    return None, None
        except urllib.error.HTTPError as e:
            if e.code in (429, 403, 503):
                wait_time = (attempt + 1) * 3.5
                time.sleep(wait_time)
            else:
                return None, None
        except Exception:
            if attempt == max_retries - 1:
                return None, None
            time.sleep(2.0)

    return None, None


def generate_opml(podcast_feeds, output_path="podbean_rescue.opml"):
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

    raw_xml = ET.tostring(opml, encoding="utf-8")
    parsed_dom = minidom.parseString(raw_xml)
    pretty_xml = parsed_dom.toprettyxml(indent="  ", encoding="utf-8")

    with open(output_path, "wb") as f:
        f.write(pretty_xml)

    print(f"\n[+] OPML file successfully saved to: {output_path}")


def main():
    parser = argparse.ArgumentParser(
        description="Rescue podcast subscriptions from a Podbean HTTP Archive (HAR) file into OPML."
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
    args = parser.parse_args()

    print("=" * 65)
    print(" Podbean to OPML Subscription Rescue Tool")
    print("=" * 65)

    unique_titles = extract_podcast_titles(args.har_file)
    print(f"\n[*] Extracted and deduplicated {len(unique_titles)} unique podcast titles using set.")

    sorted_titles = sorted(list(unique_titles))
    resolved_feeds = {}
    missing_titles = []

    print(f"[*] Querying Apple Podcasts API for canonical RSS feeds...\n")

    for idx, title in enumerate(sorted_titles, start=1):
        print(f" [{idx:02d}/{len(sorted_titles):02d}] Searching: '{title}'...", end=" ", flush=True)
        feed_url, _ = query_apple_podcasts(title)

        if feed_url:
            print("FOUND")
            print(f"       Feed: {feed_url}")
            resolved_feeds[title] = feed_url
        else:
            print("THROTTLED / NOT FOUND")
            missing_titles.append(title)

        time.sleep(0.7)

    # Automatic retry pass for any titles that hit API rate limits
    if missing_titles:
        print(f"\n[*] Waiting 5s before retrying {len(missing_titles)} unresolved shows...")
        time.sleep(5)
        retry_missing = []
        for idx, title in enumerate(missing_titles, start=1):
            print(f" [Retry {idx:02d}/{len(missing_titles):02d}] Searching: '{title}'...", end=" ", flush=True)
            feed_url, _ = query_apple_podcasts(title, max_retries=6)
            if feed_url:
                print("FOUND")
                print(f"       Feed: {feed_url}")
                resolved_feeds[title] = feed_url
            else:
                print("NOT FOUND")
                retry_missing.append(title)
            time.sleep(1.0)
        missing_titles = retry_missing

    feed_list = [{"title": t, "feedUrl": url} for t, url in resolved_feeds.items()]
    generate_opml(feed_list, args.output)

    print("\n" + "=" * 65)
    print(" RESCUE SUMMARY")
    print("=" * 65)
    print(f" Total Unique Podcasts:  {len(sorted_titles)}")
    print(f" Successfully Resolved:  {len(feed_list)}")
    print(f" Unresolved Podcasts:    {len(missing_titles)}")

    if missing_titles:
        print("\nUnresolved show titles:")
        for missing in missing_titles:
            print(f" - {missing}")

    print("=" * 65)


if __name__ == "__main__":
    main()
