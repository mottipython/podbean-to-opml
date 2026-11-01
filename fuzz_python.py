#!/usr/bin/env python3
"""
fuzz_python.py - Fuzz testing harness for rescue.py
Stresses extraction, author matching, title normalization, and OPML output with random/adversarial inputs.
"""

import os
import sys
import tempfile
import random
import string
import json

import rescue
from rescue import (
    normalize_title,
    authors_match,
    extract_podcasts,
    generate_opml,
)


def random_string(max_len=100):
    chars = string.ascii_letters + string.digits + " \t\r\n<>&\"'/{}[]:;!@#$%^*()_+-=\x00"
    length = random.randint(1, max_len)
    return "".join(random.choice(chars) for _ in range(length))


def random_unicode(max_len=50):
    samples = [
        "🎙️", "🎧", "The Daily", "Focus & Co.", "📻", "<script>alert(1)</script>",
        "&amp;&quot;&#39;", "\u202eRTL_OVERRIDE", "null", "undefined",
        "{}", "[]", "\"title\": null", "{\"a\": 1}", "<!-- XML comment -->",
        "<![CDATA[test]]>", "   ", "\r\n\t", "\x00", "\ufffd"
    ]
    count = random.randint(1, max_len)
    return "".join(random.choice(samples) for _ in range(count))


def fuzz_titles_and_authors(iterations=5000):
    print(f"[*] Fuzzing normalize_title and authors_match ({iterations} iterations)...")
    for _ in range(iterations):
        s1 = random_string(80) if random.random() < 0.5 else random_unicode(20)
        s2 = random_string(80) if random.random() < 0.5 else random_unicode(20)

        t = normalize_title(s1)
        assert isinstance(t, str)

        m = authors_match(s1, s2)
        assert isinstance(m, bool)
    print("    [+] Title normalization & author matching: PASS")


def fuzz_har_extraction(iterations=1000):
    print(f"[*] Fuzzing HAR extraction ({iterations} iterations)...")
    for _ in range(iterations):
        # Generate random or malformed JSON payloads
        dice = random.random()
        if dice < 0.3:
            payload = random_string(500)
        elif dice < 0.6:
            payload = json.dumps({
                "log": {
                    "entries": [
                        {
                            "response": {
                                "content": {
                                    "mimeType": random.choice(["application/json", "text/html", "application/octet-stream"]),
                                    "text": random_string(300)
                                }
                            }
                        }
                    ]
                }
            })
        else:
            payload = json.dumps({
                "log": {
                    "entries": [
                        {
                            "response": {
                                "content": {
                                    "mimeType": "application/json",
                                    "text": json.dumps([
                                        {"title": random_unicode(10), "author": random_unicode(10)},
                                        {random_string(10): random_string(20)}
                                    ])
                                }
                            }
                        }
                    ]
                }
            })

        with tempfile.NamedTemporaryFile("w", suffix=".har", delete=False) as f:
            f.write(payload)
            tmp_name = f.name

        try:
            # Should safely handle invalid JSON or malformed content without crashing
            extract_podcasts(tmp_name)
        except (json.JSONDecodeError, UnicodeDecodeError):
            pass  # Expected on invalid JSON files
        except Exception as e:
            print(f"[!] Unexpected exception on payload: {e}")
            raise
        finally:
            if os.path.exists(tmp_name):
                os.remove(tmp_name)
    print("    [+] HAR extraction fuzzing: PASS")


def fuzz_opml_generation(iterations=500):
    print(f"[*] Fuzzing OPML XML generation ({iterations} iterations)...")
    for _ in range(iterations):
        count = random.randint(1, 15)
        feeds = [
            {
                "title": random_unicode(10) if random.random() < 0.5 else random_string(30),
                "feedUrl": random_string(50)
            }
            for _ in range(count)
        ]
        with tempfile.NamedTemporaryFile("w", suffix=".opml", delete=False) as f:
            tmp_out = f.name

        try:
            generate_opml(feeds, tmp_out, unresolved_count=random.randint(0, 5))
            with open(tmp_out, "r", encoding="utf-8", errors="ignore") as f:
                content = f.read()
                assert "<opml" in content
                assert "</opml>" in content
        finally:
            if os.path.exists(tmp_out):
                os.remove(tmp_out)
    print("    [+] OPML XML generation: PASS")


if __name__ == "__main__":
    print("=" * 65)
    print(" Python Subsystem Fuzz Testing Harness")
    print("=" * 65)
    fuzz_titles_and_authors()
    fuzz_har_extraction()
    fuzz_opml_generation()
    print("=" * 65)
    print(" ALL FUZZ TESTS COMPLETED SUCCESSFULLY!")
    print("=" * 65)
