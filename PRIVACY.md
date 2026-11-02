# Privacy Policy: Podcast Rescue for Podbean

Last updated: 2026-11-02

Podcast Rescue for Podbean is a browser extension that exports the podcasts you follow on Podbean to an OPML file.

**What it reads.** When you click "Sync from Podbean", the extension opens your Podbean profile in a background tab and reads the titles of the podcasts you follow, using your existing Podbean login in that browser.

**What it sends.** For each show title it asks Apple's public podcast directory (`itunes.apple.com`) for the show's RSS feed. Only the show title is sent. Nothing else is sent to Apple, and nothing is sent to the developer or any other server.

**What it stores.** Your show list and the lookup results are kept in the extension's local storage (`storage.local`) on your device so they survive closing the page. "Start over" deletes them. Uninstalling the extension deletes them.

**What it never does.** No accounts, no analytics, no ads, no tracking, no remote code, no data sold or shared. It never reads your Podbean password, cookies or messages, and never changes anything on Podbean.

**Permissions.** `*.podbean.com` to read your followed podcasts; `itunes.apple.com` to look up feeds.

Questions: open an issue at https://github.com/mottipython/podbean-to-opml/issues
