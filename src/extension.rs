//! Serves the bundled browser extension as a zip, built in memory (stored, no compression).

const VERSION: &str = env!("CARGO_PKG_VERSION");

const CHROMIUM_MANIFEST: &str = include_str!("../extension/manifest.json");
const FIREFOX_MANIFEST: &str = include_str!("../extension/manifest.firefox.json");

const SCRIPTS: [(&str, &str); 2] = [
    ("cockpit.js", include_str!("../extension/cockpit.js")),
    ("podbean.js", include_str!("../extension/podbean.js")),
];

const ICONS: [(&str, &[u8]); 3] = [
    ("icon16.png", include_bytes!("../extension/icon16.png")),
    ("icon48.png", include_bytes!("../extension/icon48.png")),
    ("icon128.png", include_bytes!("../extension/icon128.png")),
];

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Chromium browsers (Chrome, Edge, Brave, Opera, Vivaldi) take the MV3 manifest; Firefox needs MV2 so site access is granted at install.
pub fn extension_zip(firefox: bool) -> Vec<u8> {
    let manifest = if firefox { FIREFOX_MANIFEST } else { CHROMIUM_MANIFEST };
    let text = std::iter::once(("manifest.json", manifest)).chain(SCRIPTS);
    let files: Vec<(&str, Vec<u8>)> = text
        .map(|(name, body)| (name, body.replace("__VERSION__", VERSION).into_bytes()))
        .chain(ICONS.map(|(name, bytes)| (name, bytes.to_vec())))
        .collect();
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in &files {
        let (crc, len, offset) = (crc32(data), data.len() as u32, out.len() as u32);
        let path = format!("podcast-rescue-extension/{name}");
        let head = |sig: &[u8; 4], ver: &[u8]| -> Vec<u8> {
            let mut h = sig.to_vec();
            h.extend_from_slice(ver);
            h.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]); // flags, method=stored, time, date
            h.extend_from_slice(&crc.to_le_bytes());
            h.extend_from_slice(&len.to_le_bytes());
            h.extend_from_slice(&len.to_le_bytes());
            h.extend_from_slice(&(path.len() as u16).to_le_bytes());
            h
        };
        out.extend(head(b"PK\x03\x04", &[20, 0]));
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(path.as_bytes());
        out.extend_from_slice(data);

        central.extend(head(b"PK\x01\x02", &[20, 0, 20, 0]));
        central.extend_from_slice(&[0; 12]); // extra, comment, disk, attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(path.as_bytes());
    }
    let (cd_offset, cd_len) = (out.len() as u32, central.len() as u32);
    out.extend(central);
    out.extend_from_slice(b"PK\x05\x06\0\0\0\0");
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_len.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_reference() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn zips_start_and_end_with_zip_signatures() {
        for firefox in [false, true] {
            let zip = extension_zip(firefox);
            assert_eq!(&zip[..4], b"PK\x03\x04");
            assert!(zip.windows(4).any(|w| w == b"PK\x05\x06"));
        }
    }
}
