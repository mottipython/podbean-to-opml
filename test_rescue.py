#!/usr/bin/env python3
"""
test_rescue.py - Unit tests for rescue.py
Tests extraction, title/author matching, ambiguity resolution, feed checking, and OPML output.
"""

import unittest
import json
import tempfile
import os
import xml.etree.ElementTree as ET
from unittest.mock import patch, MagicMock

import rescue
from rescue import (
    PodcastInfo,
    normalize_title,
    authors_match,
    extract_podcasts,
    extract_podcast_titles,
    search_apple_podcasts,
    check_feed_status,
    generate_opml,
    write_unresolved_report,
)


class TestRescue(unittest.TestCase):

    def test_normalize_title(self):
        self.assertEqual(normalize_title("The Daily & Co."), "the daily co")
        self.assertEqual(normalize_title("  Planet Money!  "), "planet money")
        self.assertEqual(normalize_title("Show &#38; Tell"), "show tell")

    def test_authors_match(self):
        # Exact and normalized
        self.assertTrue(authors_match("The New York Times", "New York Times"))
        self.assertTrue(authors_match("Vox Media", "Vox"))
        self.assertTrue(authors_match("WNYC Studios", "WNYC"))
        self.assertTrue(authors_match("BBC World Service", "BBC"))
        self.assertTrue(authors_match("Sarah Edmondson & Anthony Ames", "Sarah Edmondson and Anthony Ames"))

        # Non-matches
        self.assertFalse(authors_match("BBC", "iHeartPodcasts"))
        self.assertFalse(authors_match("NPR", "Daily Wire"))
        self.assertFalse(authors_match(None, "NPR"))
        self.assertFalse(authors_match("NPR", None))

    def test_extract_podcasts_from_har(self):
        har_payload = {
            "log": {
                "entries": [
                    {
                        "response": {
                            "content": {
                                "mimeType": "application/json",
                                "text": json.dumps([
                                    {
                                        "title": "Adventures with Dead Jews",
                                        "author": "Tablet Studios"
                                    },
                                    {
                                        "podcast_title": "A Little Bit Culty",
                                        "artist_name": "Sarah & Anthony"
                                    }
                                ])
                            }
                        }
                    },
                    {
                        "response": {
                            "content": {
                                "mimeType": "text/html",
                                "text": """
                                    <a class="nav-link card-title" href="/show3">RadioLab</a>
                                    <div data-dir-podcast-title="Hardcore History" data-dir-author="Dan Carlin"></div>
                                """
                            }
                        }
                    }
                ]
            }
        }

        with tempfile.NamedTemporaryFile("w", suffix=".har", delete=False) as f:
            json.dump(har_payload, f)
            temp_path = f.name

        try:
            podcasts = extract_podcasts(temp_path)
            self.assertEqual(len(podcasts), 4)

            # Check JSON extracted
            dead_jews = podcasts["adventures with dead jews"]
            self.assertEqual(dead_jews.title, "Adventures with Dead Jews")
            self.assertEqual(dead_jews.author, "Tablet Studios")

            culty = podcasts["a little bit culty"]
            self.assertEqual(culty.title, "A Little Bit Culty")
            self.assertEqual(culty.author, "Sarah & Anthony")

            # Check HTML card extracted
            radiolab = podcasts["radiolab"]
            self.assertEqual(radiolab.title, "RadioLab")
            self.assertIsNone(radiolab.author)

            # Check data attribute extracted
            hh = podcasts["hardcore history"]
            self.assertEqual(hh.title, "Hardcore History")
            self.assertEqual(hh.author, "Dan Carlin")

            # Check legacy wrapper
            titles = extract_podcast_titles(temp_path)
            self.assertEqual(len(titles), 4)
            self.assertIn("RadioLab", titles)
        finally:
            os.remove(temp_path)

    @patch("urllib.request.urlopen")
    def test_search_apple_podcasts_single_exact_match(self, mock_urlopen):
        mock_response = MagicMock()
        mock_response.status = 200
        mock_response.read.return_value = json.dumps({
            "results": [
                {
                    "collectionName": "Adventures with Dead Jews",
                    "artistName": "Tablet Studios",
                    "feedUrl": "https://feeds.megaphone.fm/deadjews"
                }
            ]
        }).encode("utf-8")
        mock_urlopen.return_value.__enter__.return_value = mock_response

        res = search_apple_podcasts("Adventures with Dead Jews", author="Tablet Studios")
        self.assertEqual(res["status"], "RESOLVED")
        self.assertEqual(res["feed_url"], "https://feeds.megaphone.fm/deadjews")
        self.assertEqual(res["matched_title"], "Adventures with Dead Jews")

    @patch("urllib.request.urlopen")
    def test_search_apple_podcasts_multiple_exact_disambiguated_by_publisher(self, mock_urlopen):
        mock_response = MagicMock()
        mock_response.status = 200
        mock_response.read.return_value = json.dumps({
            "results": [
                {
                    "collectionName": "Focus",
                    "artistName": "Focus on the Family",
                    "feedUrl": "https://feed1.example.com"
                },
                {
                    "collectionName": "Focus",
                    "artistName": "BBC World Service",
                    "feedUrl": "https://feed2.example.com"
                }
            ]
        }).encode("utf-8")
        mock_urlopen.return_value.__enter__.return_value = mock_response

        # Searching with BBC as publisher should resolve to the second feed
        res = search_apple_podcasts("Focus", author="BBC")
        self.assertEqual(res["status"], "RESOLVED")
        self.assertEqual(res["feed_url"], "https://feed2.example.com")
        self.assertEqual(res["artist"], "BBC World Service")

    @patch("urllib.request.urlopen")
    def test_search_apple_podcasts_multiple_exact_ambiguous_without_author(self, mock_urlopen):
        mock_response = MagicMock()
        mock_response.status = 200
        mock_response.read.return_value = json.dumps({
            "results": [
                {
                    "collectionName": "Focus",
                    "artistName": "Focus on the Family",
                    "feedUrl": "https://feed1.example.com"
                },
                {
                    "collectionName": "Focus",
                    "artistName": "BBC World Service",
                    "feedUrl": "https://feed2.example.com"
                }
            ]
        }).encode("utf-8")
        mock_urlopen.return_value.__enter__.return_value = mock_response

        # Without author, cannot disambiguate -> AMBIGUOUS
        res = search_apple_podcasts("Focus", author=None)
        self.assertEqual(res["status"], "AMBIGUOUS")
        self.assertIsNone(res["feed_url"])
        self.assertEqual(len(res["candidates"]), 2)

    @patch("urllib.request.urlopen")
    def test_search_apple_podcasts_no_loose_fallback(self, mock_urlopen):
        mock_response = MagicMock()
        mock_response.status = 200
        # Apple returns unrelated results
        mock_response.read.return_value = json.dumps({
            "results": [
                {
                    "collectionName": "The Headlines",
                    "artistName": "The New York Times",
                    "feedUrl": "https://feeds.simplecast.com/headlines"
                },
                {
                    "collectionName": "Morning Wire",
                    "artistName": "Daily Wire",
                    "feedUrl": "https://feeds.example.com/wire"
                }
            ]
        }).encode("utf-8")
        mock_urlopen.return_value.__enter__.return_value = mock_response

        # Searching for 'Daily News' with no matching title should NOT return 'The Headlines'
        res = search_apple_podcasts("Daily News")
        self.assertEqual(res["status"], "NOT_FOUND")
        self.assertIsNone(res["feed_url"])

    @patch("urllib.request.urlopen")
    def test_check_feed_status_moved_feed(self, mock_urlopen):
        mock_response = MagicMock()
        mock_response.status = 200
        mock_response.geturl.return_value = "https://oldfeed.example.com/rss"
        # RSS contains itunes:new-feed-url
        mock_response.read.return_value = b"""<?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Moved Show</title>
                <itunes:new-feed-url>https://newfeed.example.com/rss</itunes:new-feed-url>
            </channel>
        </rss>
        """
        mock_urlopen.return_value.__enter__.return_value = mock_response

        eff_url, moved, err = check_feed_status("https://oldfeed.example.com/rss")
        self.assertEqual(eff_url, "https://newfeed.example.com/rss")
        self.assertTrue(moved)
        self.assertIsNone(err)

    def test_generate_opml_and_unresolved_report(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            opml_path = os.path.join(tmpdir, "test.opml")
            unresolved_path = os.path.join(tmpdir, "unresolved.txt")

            feeds = [
                {"title": "Show B", "feedUrl": "https://b.example.com"},
                {"title": "Show A", "feedUrl": "https://a.example.com"},
            ]
            generate_opml(feeds, opml_path, unresolved_count=2)

            self.assertTrue(os.path.exists(opml_path))
            with open(opml_path, "r", encoding="utf-8") as xml_f:
                content = xml_f.read()
            self.assertIn("<opml", content)
            self.assertIn('title="Show A"', content)
            self.assertIn('xmlUrl="https://a.example.com"', content)
            self.assertIn('title="Show B"', content)
            self.assertIn('xmlUrl="https://b.example.com"', content)

            # Check unresolved report
            ambiguous = [
                {
                    "title": "Ambiguous Show",
                    "author": "Indie Network",
                    "reason": "Multiple candidates",
                    "candidates": [
                        {"collectionName": "Ambiguous Show", "artistName": "Indie 1", "feedUrl": "https://feed1.com"},
                        {"collectionName": "Ambiguous Show", "artistName": "Indie 2", "feedUrl": "https://feed2.com"}
                    ]
                }
            ]
            unresolved = [
                {
                    "title": "Private Show",
                    "author": "Acme",
                    "reason": "Not found in directory"
                }
            ]
            write_unresolved_report(unresolved, ambiguous, unresolved_path)
            self.assertTrue(os.path.exists(unresolved_path))
            with open(unresolved_path, "r") as f:
                content = f.read()
                self.assertIn("Ambiguous Show", content)
                self.assertIn("Private Show", content)
                self.assertIn("https://feed1.com", content)


if __name__ == "__main__":
    unittest.main()
