# Universal Podcast Subscription Rescue & OPML Exporter (`podcast-rescue`)

> **Rescue your podcast subscriptions from walled-garden podcast apps into standard OPML format with an interactive browser GUI.**

[![Release](https://img.shields.io/github/v/release/mottipython/podbean-to-opml?color=10b981&label=Release)](https://github.com/mottipython/podbean-to-opml/releases/latest)
[![CI Build](https://github.com/mottipython/podbean-to-opml/actions/workflows/build.yml/badge.svg)](https://github.com/mottipython/podbean-to-opml/actions/workflows/build.yml)
[![Security: Snyk Passed](https://img.shields.io/badge/Security-Snyk%20Passed-blue)](https://snyk.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Many major podcast platforms famously lack an **"Export Subscriptions"** or **"Export OPML"** feature—locking your library into their ecosystem and making it frustrating to switch to open, privacy-respecting podcast clients like AntennaPod, Pocket Casts, Overcast, Apple Podcasts, or Castro without manually re-subscribing to every show.

This tool extracts your followed podcasts from **Podbean**, **Spotify**, **Apple Podcasts**, **YouTube Music**, or **any generic HAR / JSON / plain-text export**, resolves each show's canonical RSS feed using the Apple Podcasts Directory API, and generates a clean, standard **OPML 1.0 XML** file without silently subscribing you to the wrong shows.

---

## 📦 Downloads & Native Packages

Pre-built binaries and native distribution packages are on the [**latest release**](https://github.com/mottipython/podbean-to-opml/releases/latest).

### 🪟 Windows Packages

| Type | Package | Description |
| :--- | :--- | :--- |
| **Windows Installer** | [**`podcast-rescue-windows-x86_64-setup.exe`**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/podcast-rescue-windows-x86_64-setup.exe) | Setup wizard with Start Menu shortcuts, Desktop icon, PATH integration, and uninstaller |
| **Portable Zip** | [**`podcast-rescue-windows-x86_64.zip`**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/podcast-rescue-windows-x86_64.zip) | Standalone portable executable (`podcast-rescue.exe`) |

### 🐧 Linux Distribution Packages

| Distribution | Package | Install Command |
| :--- | :--- | :--- |
| **Fedora / RHEL / CentOS / openSUSE** | [**`.rpm` Package**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/podcast-rescue-0.4.0-1.fc44.x86_64.rpm) | `sudo dnf install ./podcast-rescue-*.rpm` |
| **Ubuntu / Debian / Linux Mint / Pop!_OS** | [**`.deb` Package**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/podcast-rescue_0.4.0-1_amd64.deb) | `sudo apt install ./podcast-rescue_*.deb` |
| **Arch Linux / Manjaro / EndeavourOS / SteamOS** | [**`.pkg.tar.zst` Package**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/podcast-rescue-0.4.0-1-x86_64.pkg.tar.zst) • [**`PKGBUILD`**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/PKGBUILD) | `sudo pacman -U ./podcast-rescue-*.pkg.tar.zst` |

### 💻 Standalone Portable Binaries

| Operating System | Architecture | Package |
| :--- | :--- | :--- |
| **Windows** | x86_64 (64-bit) | [**`podcast-rescue-windows-x86_64.zip`**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/podcast-rescue-windows-x86_64.zip) (`.exe`) |
| **macOS** | Universal (Apple Silicon M1/M2/M3/M4 & Intel) | [**`podcast-rescue-macos-universal.tar.gz`**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/podcast-rescue-macos-universal.tar.gz) |
| **Linux** | x86_64 | [**`podcast-rescue-linux-x86_64.tar.gz`**](https://github.com/mottipython/podbean-to-opml/releases/download/v0.4.0/podcast-rescue-linux-x86_64.tar.gz) |

---

## 🚀 Features

- **Cross-Platform Browser GUI**: Launches a sleek, responsive local web interface that automatically opens in your default browser on **Windows, macOS, and Linux**.
- **Multi-Platform Support**:
  - 🫘 **Podbean** (followed podcasts from user profiles)
  - 🟢 **Spotify** (saved/followed shows from Spotify Web Player)
  - 🍎 **Apple Podcasts Web** (subscribed library)
  - ▶️ **YouTube Music** (podcast channel subscriptions)
  - 🌐 **Any HAR, JSON, or Plaintext** (drag-and-drop or copy-paste)
- **Browser Extension (Chrome, Edge, Brave, Opera, Vivaldi, Firefox)**: paste your Podbean profile link and sync your private followed-podcasts list as the account you are already logged in to. Installed from inside the app.
- **Step-by-Step Interactive Walkthrough**: Built-in visual guides tailored for each platform showing how to grab your network archive in 3 clicks.
- **Publisher Disambiguation**: Matches publisher / artist metadata (`artistName`) against your subscription data to accurately resolve shows with identical or common names.
- **No Silent Wrong Imports**: Eliminates blind keyword guessing. Ambiguous show titles are presented as clickable candidate cards right in the GUI so you can pick the right feed on the fly.
- **Moved Feeds Detection**: Follows HTTP 301/308 redirects and parses `<itunes:new-feed-url>` tags to capture relocated canonical feeds.
- **OPML Health Cleaner & Relocation Fixer**: Upload an existing OPML file to check feed availability, remove dead feeds, and automatically update migrated feeds.
- **Private & Patreon Feed Support**: Add custom, private, or patron-only RSS feeds directly into your export queue.
- **Multi-Format Export**: 1-click export to standard **OPML 1.0 XML**, structured **JSON**, or spreadsheet-ready **CSV**.
- **1-Click Export**: Instant download of standard `podcasts_rescue.opml` and an `unresolved_report.txt` for shows requiring manual review.
- **Dual Engine & Docker Ready**: High-performance compiled **Rust** core, lightweight Docker container, plus a zero-dependency **Python** fallback script.

---

## 🔌 Browser Extension (paste a profile link, sync as the logged-in account)

Podbean keeps your followed podcasts private to your browser session, which a local app cannot see. The bundled extension runs inside your own Podbean tab, so **Fetch & Ingest** works on a pasted profile link.

1. In the app, click **Install Browser Extension** and follow the steps for your browser.
   - **Chrome, Edge, Brave, Opera, Vivaldi:** download the zip, unzip, then *Load unpacked* on the extensions page (Developer mode on).
   - **Firefox:** download the Firefox zip, then *Load Temporary Add-on* on `about:debugging` (removed on restart).
   - **Safari:** only signed extensions load, so use the **Rescue Library (Bookmark)** button instead.
2. Reload the app, paste your Podbean profile link, click **Fetch & Ingest**. A tab opens, syncs, and the shows appear in the app.

The extension only runs on `*.podbean.com` and the local app, and only sends data to the app's own address.

---

## 🖥️ How to Use (Web GUI Walkthrough)

### 1. Launch the Application

Launch the installed app from your desktop applications menu, or run in the terminal:

```bash
podcast-rescue
```

Or run from source:

```bash
cargo run --release
```

The application will launch and **automatically open your default web browser** to `http://127.0.0.1:3000`.

### 2. Podbean: sync your followed podcasts

Podbean keeps your followed list private to your browser session, so a local app cannot fetch it from a link alone. Pick one:

1. **Extension (recommended):** install it from the app (see [Browser Extension](#-browser-extension-paste-a-profile-link-sync-as-the-logged-in-account)), paste your profile link (`https://www.podbean.com/user-...`), click **Fetch & Ingest**.
2. **Bookmark button (any browser, including Safari):** drag **Rescue Library (Bookmark)** to your bookmarks bar, open your Podbean profile while logged in, click it.
3. **Console script:** click **Launch Profile & Auto-Sync**, then paste the copied script into the browser console (`F12`) on your profile page.
4. **Paste page:** on your profile, `Ctrl+A`, `Ctrl+C`, then **Paste from Clipboard** (only the page you are on).

Your profile page shows "Followed Podcasts: N". Podbean may list fewer shows than N if some were removed.

### 3. Follow the Browser Walkthrough for other platforms

1. **Select your platform tab** (Podbean, Spotify, Apple Podcasts, YouTube Music, Universal, or Clean Existing OPML).
2. **Follow the interactive steps** on the screen to capture your session archive (`.har`).
3. **Drag & drop** your `.har`, `.json`, `.opml`, or text file onto the drop zone (or paste your show list).
4. **Add Private / Patreon Feeds** (optional): Click **`🔒 + Add Private Feed`** in the header to include member-only feeds.
5. Click **"Resolve RSS Feeds via Apple Directory"**.
6. Pick the right feed on any **ambiguous** card (same title from several publishers), then export in **OPML**, **JSON**, or **CSV**.
7. Import the `.opml` into your podcast app (see **Import Guide** in the header).

---

## 💻 Terminal / Headless CLI Mode

If you prefer running in the terminal without opening the browser:

```bash
# Extract and export to standard OPML
podcast-rescue my_archive.har -o my_podcasts.opml -u unresolved.txt

# Export directly to JSON or CSV
podcast-rescue my_archive.har -o my_podcasts.json
podcast-rescue my_archive.har -o my_podcasts.csv

# Enable live HTTP feed verification to detect relocated feeds
podcast-rescue my_archive.har --verify-feeds

# Clean, verify, and fix an existing OPML file
podcast-rescue --clean-opml old_subscriptions.opml -o cleaned_subscriptions.opml

# Run Web GUI on a custom network host and port (e.g. for remote access)
podcast-rescue --host 0.0.0.0 --port 3000 --no-browser
```

---

## 🐳 Docker Deployment

Run the Web GUI headlessly inside a lightweight, non-root Alpine container:

```bash
# Build the Docker image
docker build -t podcast-rescue .

# Run the container (access at http://localhost:3000)
docker run -d -p 3000:3000 --name podcast-rescue podcast-rescue
```4. Click **"Resolve RSS Feeds via Apple Directory"**.
5. If any shows share identical titles, click the candidate card corresponding to your show.
6. Click **"Download OPML XML"** to export your subscriptions!

---

## 💻 Terminal / Headless CLI Mode

If you prefer running in the terminal without opening the browser:

```bash
# Run against a HAR or JSON file
podcast-rescue my_archive.har -o my_podcasts.opml -u unresolved.txt

# Enable live HTTP feed verification to detect relocated feeds
podcast-rescue my_archive.har --verify-feeds

# Run Web GUI on a custom port without auto-launching browser
podcast-rescue --port 8080 --no-browser
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

## 🛡️ Security & Testing

- **Snyk Code SAST:** Passed with 0 High and 0 Medium issues. All DOM manipulations sanitized against XSS.
- **Cargo Audit:** All 85 crate dependencies scanned against the RustSec Advisory Database with 0 vulnerabilities.
- **Automated Fuzz Testing:** 27,500+ iterations across Rust and Python parser, matcher, and OPML generation engines with zero crashes.

```bash
# Run Rust unit & fuzz tests
cargo test
cargo test --test fuzz_test

# Run Python unit & fuzz tests
python3 -m unittest -v test_rescue.py
python3 fuzz_python.py
```

---

## License

[MIT](LICENSE) © Motti Schleider
