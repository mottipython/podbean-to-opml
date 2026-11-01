use std::panic::{catch_unwind, AssertUnwindSafe};

// Bring modules from the crate
#[path = "../src/types.rs"]
mod types;

#[path = "../src/parser.rs"]
mod parser;

#[path = "../src/resolver.rs"]
mod resolver;

#[path = "../src/opml.rs"]
mod opml;

use parser::parse_content;
use resolver::{authors_match, normalize_title};
use opml::generate_opml_xml;
use types::ExportItem;

/// Simple deterministic Pseudo-Random Generator for fuzzing without external crates
struct FuzzRng {
    state: u64,
}

impl FuzzRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn random_string(&mut self, max_len: usize) -> String {
        let len = (self.next_u32() as usize % max_len) + 1;
        let mut s = String::with_capacity(len);
        let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t\n\r<>&\"'/{}[],:;-_=+*&^%$#@!~`|\\?\x00";

        for _ in 0..len {
            let idx = (self.next_u32() as usize) % alphabet.len();
            s.push(alphabet[idx] as char);
        }
        s
    }

    fn random_unicode_string(&mut self, max_chars: usize) -> String {
        let count = (self.next_u32() as usize % max_chars) + 1;
        let unicode_samples = [
            "🎙️", "🎧", "The Daily", "Focus & Co.", "📻", "<script>alert(1)</script>",
            "&amp;&quot;&#39;", "\u{202E}RTL_OVERRIDE", "null", "undefined",
            "{}", "[]", "\"title\": null", "{\"a\": 1}", "<!-- XML comment -->",
            "<![CDATA[test]]>", "   ", "\r\n\t", "\u{0000}", "\u{FFFD}",
        ];
        let mut s = String::new();
        for _ in 0..count {
            let idx = (self.next_u32() as usize) % unicode_samples.len();
            s.push_str(unicode_samples[idx]);
        }
        s
    }
}

#[test]
fn test_fuzz_parser_no_panics() {
    let mut rng = FuzzRng::new(0x1337BEEF);
    let platforms = [None, Some("Podbean"), Some("Spotify"), Some("Apple"), Some("YouTube"), Some("Universal")];

    for iteration in 0..10_000 {
        let platform = platforms[(rng.next_u32() as usize) % platforms.len()];
        let mode = rng.next_u32() % 4;

        let input = match mode {
            0 => rng.random_string(2048),
            1 => rng.random_unicode_string(50),
            2 => {
                // Synthetic mutated JSON / HAR
                format!(
                    r#"{{"log": {{"entries": [{{"response": {{"content": {{"text": "{}"}}}}}}]}}}}"#,
                    rng.random_string(200).replace('\\', "\\\\").replace('"', "\\\"")
                )
            }
            _ => {
                // Line-based input with weird delimiters
                format!("{}\n{}\n{}", rng.random_string(100), rng.random_unicode_string(10), rng.random_string(50))
            }
        };

        let result = catch_unwind(AssertUnwindSafe(|| {
            parse_content(&input, platform)
        }));

        assert!(
            result.is_ok(),
            "Parser panicked on iteration {} with input:\n{}",
            iteration,
            input
        );
    }
}

#[test]
fn test_fuzz_resolver_matching_no_panics() {
    let mut rng = FuzzRng::new(0xDEADCAFE);

    for iteration in 0..10_000 {
        let a = rng.random_string(200);
        let b = rng.random_string(200);

        let res_norm = catch_unwind(AssertUnwindSafe(|| {
            normalize_title(&a)
        }));
        assert!(res_norm.is_ok(), "normalize_title panicked on iteration {}", iteration);

        let res_match = catch_unwind(AssertUnwindSafe(|| {
            authors_match(&a, &b)
        }));
        assert!(res_match.is_ok(), "authors_match panicked on iteration {}", iteration);
    }
}

#[test]
fn test_fuzz_opml_xml_integrity() {
    let mut rng = FuzzRng::new(0xFEEDFACE);

    for iteration in 0..1_000 {
        let count = (rng.next_u32() as usize % 20) + 1;
        let mut items = Vec::new();

        for _ in 0..count {
            items.push(ExportItem {
                title: rng.random_unicode_string(15),
                feed_url: rng.random_string(80),
            });
        }

        let unresolved = (rng.next_u32() as usize) % 10;
        let xml = generate_opml_xml(&items, unresolved);

        // Ensure output is non-empty and well-formed root tag
        assert!(xml.starts_with("<?xml"), "Iter {}: XML missing header", iteration);
        assert!(xml.contains("<opml version=\"1.0\">"), "Iter {}: missing opml tag", iteration);
        assert!(xml.contains("</opml>"), "Iter {}: missing closing tag", iteration);

        // Ensure dangerous raw XML characters in titles are escaped
        for item in &items {
            if item.title.contains('<') {
                assert!(!xml.contains(&format!("title=\"{}\"", item.title)));
            }
        }
    }
}
