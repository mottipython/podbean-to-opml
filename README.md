# Universal Podcast Subscription Rescue & OPML Exporter (`podcast-rescue`)

> **Rescue your podcast subscriptions from walled-garden podcast apps into standard OPML format with a guided browser GUI.**

Many major podcast platforms famously lack an **"Export Subscriptions"** or **"Export OPML"** feature—locking your library into their ecosystem and making it frustrating to switch to open, privacy-respecting podcast clients like AntennaPod, Pocket Casts, Overcast, Apple Podcasts, or Castro without manually re-subscribing to every show.

This tool extracts your followed podcasts from **Podbean**, **Spotify**, **Apple Podcasts**, **YouTube Music**, or **any generic HAR / JSON / plain-text export**, resolves each show's canonical RSS feed using the Apple Podcasts Directory API, and generates a clean, standard **OPML 1.0 XML** file without silently subscribing you to the wrong shows.

---

## 🚀 Features

- **Cross-Platform Browser GUI**: Launches a sleek, responsive local web interface that automatically opens in your default browser on **Windows, macOS, and Linux**.
- **Multi-Platform Support**:
  - 🫘 **Podbean** (followed podcasts from user profiles)
  - 🟢 **Spotify** (saved/followed shows from Spotify Web Player)
  - 🍎 **Apple Podcasts Web** (subscribed library)
  - ▶️ **YouTube Music** (podcast channel subscriptions)
  - 🌐 **Any HAR, JSON, or Plaintext** (drag-and-drop or copy-paste)
- **Step-by-Step Interactive Walkthrough**: Built-in visual guides tailored for each platform showing how to grab your network archive in 3 clicks.
- **Publisher Disambiguation**: Matches publisher / artist metadata (`artistName`) against your subscription data to accurately resolve shows with identical or common names.
- **No Silent Wrong Imports**: Eliminates blind keyword guessing. Ambiguous show titles are presented as clickable candidate cards right in the GUI so you can pick the right feed on the fly.
- **Moved Feeds Detection**: Follows HTTP 301/308 redirects and parses `<itunes:new-feed-url>` tags to capture relocated canonical feeds.
- **1-Click Export**: Instant download of standard `podcasts_rescue.opml` and an `unresolved_report.txt` for shows requiring manual review.
- **Dual Engine**: High-performance compiled **Rust** core with embedded Web GUI, plus a lightweight, zero-dependency **Python** fallback script.

---

## 🖥️ How to Use (Web GUI Walkthrough)

### 1. Run the Application
Clone this repository and run the Rust application:

```bash
cargo run --release
```

Or run the compiled binary:

```bash
./target/release/podcast-rescue
```

The application will start a local server and **automatically open your default web browser** to `http://127.0.0.1:3000`.

### 2. Follow the Browser Walkthrough
1. **Select your platform tab** (Podbean, Spotify, Apple Podcasts, YouTube Music, or Universal).
2. **Follow the interactive steps** on the screen to capture your session archive (`.har`).
3. **Drag & drop** your `.har`, `.json`, or text file onto the drop zone (or paste your show list).
4. Click **"Resolve RSS Feeds via Apple Directory"**.
5. If any shows share identical titles, click the candidate card corresponding to your show.
6. Click **"Download OPML XML"** to export your subscriptions!

---

## 💻 Terminal / Headless CLI Mode

If you prefer running in the terminal without opening the browser:

```bash
# Run against a HAR or JSON file
./target/release/podcast-rescue my_archive.har -o my_podcasts.opml -u unresolved.txt

# Enable live HTTP feed verification to detect relocated feeds
./target/release/podcast-rescue my_archive.har --verify-feeds

# Run Web GUI on a custom port without auto-launching browser
./target/release/podcast-rescue --port 8080 --no-browser
```

---

## 🐍 Python Legacy Script (`rescue.py`)

For minimal environments where Rust is not installed, the original zero-dependency Python script remains fully supported:

```bash
# Basic run
python3 rescue.py podbean.har

# Interactive disambiguation in terminal
python3 rescue.py -i podbean.har

# Verify live feeds and moved URLs
python3 rescue.py --verify-feeds podbean.har
```

---

## 📱 Import into Your Podcast App

Once your `.opml` file is saved, transfer it to your device and import it into any podcast player:

- **AntennaPod**: *Settings* &rarr; *Storage & Data* &rarr; *Import / Export* &rarr; *Import OPML*
- **Pocket Casts**: *Profile* &rarr; *Settings (gear icon)* &rarr; *Import/Export OPML* &rarr; *Import*
- **Overcast**: *Settings* &rarr; *OPML* &rarr; *Import OPML*
- **Castro**: *Settings* &rarr; *Sideloads / Import OPML*
- **Apple Podcasts (Mac)**: *File* &rarr; *Add a Show by URL or Import OPML*

---

## 🛠️ Running the Test Suite

```bash
# Run Rust unit tests
cargo test

# Run Python unit tests
python3 -m unittest -v test_rescue.py
```

---

## License

[MIT](LICENSE) © Motti Schleider
