# Podbean to OPML Exporter (`podbean-to-opml`)

> **Rescue your podcast subscriptions from Podbean's walled garden and export them into standard OPML format.**

Podbean famously lacks an **"Export Subscriptions"** or **"Export OPML"** feature, making it frustrating to switch to open podcast clients like AntennaPod, Pocket Casts, Overcast, Apple Podcasts, or Castro without manually re-subscribing to every show.

This tool extracts your followed podcasts from a standard HTTP Archive (`.har`) captured in your web browser, resolves each show's canonical RSS feed using the Apple Podcasts Directory API, and generates a clean, standard **OPML 1.0 XML** file without silently subscribing you to the wrong shows.

---

## Features

- **Zero dependencies**: Uses Python 3 standard library only (`json`, `urllib`, `xml.etree`). No `pip install` required.
- **Robust extraction**: Extracts podcast titles and publisher metadata from both JSON API payloads and HTML/PJAX card structures in the archive.
- **Publisher Disambiguation**: Matches publisher / artist names to accurately resolve shows with identical or common names.
- **No Silent Wrong Imports**: Ambiguous collisions and unlisted shows are flagged for review rather than blindly falling back to unrelated search results.
- **Moved Feeds Detection**: Optional `--verify-feeds` follows HTTP redirects (301/308) and reads `<itunes:new-feed-url>` tags to capture relocated feeds.
- **Interactive Review**: Optional interactive prompt (`-i` / `--interactive`) lets you pick the right feed on the fly for ambiguous matches.
- **Unresolved Subscriptions Report**: Automatically writes `podbean_unresolved.txt` with candidate links and notes for any shows that require manual review.
- **Universal compatibility**: Generates valid OPML 1.0 files ready to import into any podcast app.

---

## How to Use

### Step 1: Export Your Podbean Network Archive (`.har`)

1. Open your desktop browser (Chrome, Firefox, Brave, Edge).
2. Log in to [Podbean](https://www.podbean.com).
3. Open your browser's Developer Tools by pressing <kbd>F12</kbd> (or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>I</kbd> / <kbd>Cmd</kbd>+<kbd>Option</kbd>+<kbd>I</kbd>).
4. Click on the **Network** tab in Developer Tools.
5. In the browser, navigate to your user profile page (e.g., `https://www.podbean.com/user-...`).
6. Scroll down to **Followed Podcasts** and click through all pages (e.g., page 1, 2, 3...) until you've viewed all your followed shows so their metadata loads in the Network tab.
7. In the Developer Tools Network tab:
   - **Chrome / Brave / Edge**: Click the export icon (downward arrow) or right-click any network request and select **"Save all as HAR with content"**.
   - **Firefox**: Click the gear icon in the Network tab and choose **"Save All As HAR"**.
8. Save the file as `podbean.har` in the same directory as this script (or anywhere convenient).

---

### Step 2: Run the Rescue Script

Clone or download this repository, then run:

```bash
python3 rescue.py
```

By default, the script looks for `podbean.har` in the current directory, outputs `podbean_rescue.opml`, and logs any unresolved or ambiguous shows to `podbean_unresolved.txt`.

#### Advanced Options:

```bash
# Specify custom input and output paths
python3 rescue.py /path/to/my_archive.har -o my_podcasts.opml -u unresolved.txt

# Interactively resolve ambiguous matches on the terminal
python3 rescue.py -i

# Verify live feeds over HTTP to follow redirects and detect relocated feeds (<itunes:new-feed-url>)
python3 rescue.py --verify-feeds

# Run in headless/script mode without any terminal prompts
python3 rescue.py --non-interactive
```

### Sample Output:

```
=================================================================
 Podbean to OPML Subscription Rescue Tool
=================================================================
[*] Reading HTTP Archive: podbean.har...
[*] Found 414 recorded network entries.

[*] Extracted and deduplicated 70 unique podcasts.
[*] Querying Apple Podcasts API for canonical RSS feeds...

 [01/70] Searching: 'A Little Bit Culty' (by Sarah Edmondson & Anthony Ames)... FOUND
       Feed: https://rss.pdrl.fm/...
 [02/70] Searching: 'Adventures with Dead Jews' (by Tablet Studios)... FOUND
       Feed: https://feeds.megaphone.fm/deadjews
...
[+] OPML file successfully saved to: podbean_rescue.opml

=================================================================
 RESCUE SUMMARY
=================================================================
 Total Unique Podcasts:   70
 Successfully Resolved:   69
 Ambiguous Matches:        1 (candidates logged for review)
 Unresolved (Not Found):   0
=================================================================

[!] 1 subscription(s) could not be resolved automatically.
    See 'podbean_unresolved.txt' for candidate links and manual subscription details.
```

---

### Step 3: Import into Your Podcast App

Transfer `podbean_rescue.opml` to your phone or desktop:

- **AntennaPod**: *Settings* &rarr; *Storage & Data* &rarr; *Import / Export* &rarr; *Import OPML*
- **Pocket Casts**: *Profile* &rarr; *Settings (gear icon)* &rarr; *Import/Export OPML* &rarr; *Import*
- **Overcast**: *Settings* &rarr; *OPML* &rarr; *Import OPML*
- **Castro**: *Settings* &rarr; *Sideloads / Import OPML*
- **Apple Podcasts (Mac)**: *File* &rarr; *Add a Show by URL or Import OPML*

---

## License

[MIT](LICENSE) © Motti Schleider
