# Context

Rust crate `podcast-rescue`: exports podcast subscriptions to OPML via a local browser GUI (`src/web.rs`, `src/web_ui.html`).

## Podbean sync (private following list)
- The app cannot see the browser login, so URL fetch never works for a profile. Sync must run inside the logged-in tab.
- Preferred path: `extension/` (Chromium MV3 + Firefox MV2 manifest), served as zips by the app (`src/extension.rs`). Cockpit opens `<profile>#podrescue=<origin>`; `podbean.js` crawls `?page2=N&pageSize=9` and POSTs to `/api/auto-ingest`.
- Fallbacks: bookmarklet / console script (`getRawScriptText` in `web_ui.html`), paste-page HTML.
- Verified live: 70 shows from a real profile (Podbean says 74; extra 4 are likely removed shows). Extension verified in Chrome and Firefox 158.

## Not done
- Safari has no extension (bookmarklet only).
- README download links still point at v0.2.0.
- Resolve + OPML export not re-run on the synced list this session.
