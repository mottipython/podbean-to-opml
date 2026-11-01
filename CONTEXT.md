# Context

Rust crate `podcast-rescue`: exports podcast subscriptions to OPML via a local browser GUI (`src/web.rs`, `src/web_ui.html`).

## Podbean sync (private following list)
- The app cannot see the browser login, so URL fetch never works for a profile. Sync must run inside the logged-in tab.
- Preferred path: `extension/` (Chromium MV3 + Firefox MV2 manifest), served as zips by the app (`src/extension.rs`). Cockpit opens `<profile>#podrescue=<origin>`; `podbean.js` crawls `?page2=N&pageSize=9` and POSTs to `/api/auto-ingest`.
- Fallbacks: bookmarklet / console script (`getRawScriptText` in `web_ui.html`), paste-page HTML.
- Verified live: 70 shows from a real profile (Podbean says 74; extra 4 are likely removed shows). Extension verified in Chrome and Firefox 158.

## Not done
- Safari has no extension (bookmarklet only).
- No release yet containing the extension: cut v0.4.0 (bump Cargo.toml, packaging/package_all.sh, .iss; build rpm/deb/arch locally, Windows/mac from CI artifacts) then drop the README "packages predate the extension" note.

## End-to-end check (2026-11-01)
Real account: extension sync 70 shows -> resolve 62 exact + 8 ambiguous (picked by publisher) -> OPML 70 unique feeds. Works in Chrome and Firefox 158.

## Local server security (v0.4.0)
- Per-run token (`X-Rescue-Token`) required on every `/api/*` call except `/api/auto-ingest`; the served page carries it and wraps `fetch`.
- `/api/auto-ingest` allows cross-origin POST only from podbean.com, Spotify, Apple Podcasts, YouTube Music (HTTPS).
- `Host` must be loopback unless bound to a non-loopback host (DNS-rebinding guard); server exits after 30 idle minutes when loopback-only.
