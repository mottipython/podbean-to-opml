# Store listing text

**Name:** Podcast Rescue for Podbean

**Summary (132 chars max):** Export the podcasts you follow on Podbean to an OPML file for any podcast app. Runs entirely in your browser.

**Description:**
Podbean's app and website make it hard to take your subscriptions with you. Podcast Rescue reads the podcasts you follow, finds each show's public RSS feed, and gives you one OPML file that Pocket Casts, AntennaPod, Overcast, Apple Podcasts and most other podcast apps can import.

How it works
1. Open the extension and paste your Podbean profile link.
2. Click Sync. The extension reads your followed podcasts using your existing Podbean login.
3. Review any show with more than one possible match, or accept the suggested picks.
4. Download the OPML file and import it into your podcast app.

Private by design: no account, no server, no tracking. Show titles are looked up in Apple's public podcast directory. Nothing else leaves your browser.

**Category:** Productivity. **Single purpose:** export followed Podbean podcasts as OPML.

**Permission justifications (paste into the review form):**
- `https://*.podbean.com/*`: read the signed-in user's followed podcasts list from their own Podbean profile pages.
- `https://itunes.apple.com/*`: look up the public RSS feed of each show title in Apple's podcast directory.
- Remote code: none. Data collected: none.
