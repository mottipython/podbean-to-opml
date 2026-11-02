use std::process::{Command, Stdio};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BrowserInfo {
    pub name: String,
    pub executable: String,
}

/// Detects available modern browsers on the system across Linux, macOS, and Windows
#[allow(dead_code)]
pub fn detect_browsers() -> Vec<BrowserInfo> {
    let mut detected = Vec::new();

    #[cfg(target_os = "windows")]
    let candidates = [
        ("Microsoft Edge", "msedge.exe"),
        ("Google Chrome", "chrome.exe"),
        ("Brave", "brave.exe"),
        ("Firefox", "firefox.exe"),
    ];

    #[cfg(target_os = "macos")]
    let candidates = [
        (
            "Google Chrome",
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ),
        (
            "Microsoft Edge",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ),
        (
            "Brave",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        ),
        (
            "Firefox",
            "/Applications/Firefox.app/Contents/MacOS/firefox",
        ),
    ];

    #[cfg(target_os = "linux")]
    let candidates = [
        ("Microsoft Edge", "microsoft-edge"),
        ("Google Chrome", "google-chrome"),
        ("Google Chrome Stable", "google-chrome-stable"),
        ("Chromium", "chromium"),
        ("Chromium Browser", "chromium-browser"),
        ("Brave", "brave-browser"),
        ("Firefox", "firefox"),
    ];

    for (name, exe) in candidates {
        if is_executable_available(exe) {
            detected.push(BrowserInfo {
                name: name.to_string(),
                executable: exe.to_string(),
            });
        }
    }

    detected
}

fn is_executable_available(exe: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where")
            .arg(exe)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_or(false, |s| s.success())
    }

    #[cfg(not(target_os = "windows"))]
    {
        if exe.starts_with('/') {
            std::path::Path::new(exe).exists()
        } else {
            Command::new("which")
                .arg(exe)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        }
    }
}

/// Opens the specified URL in the user's default browser across Windows, macOS, and Linux
pub fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .is_ok()
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn().is_ok()
    }

    #[cfg(target_os = "linux")]
    {
        if Command::new("xdg-open").arg(url).spawn().is_ok() {
            return true;
        }

        // Fallbacks if xdg-open is not found
        for browser in [
            "microsoft-edge",
            "google-chrome",
            "chromium",
            "firefox",
            "brave-browser",
        ] {
            if is_executable_available(browser) && Command::new(browser).arg(url).spawn().is_ok() {
                return true;
            }
        }
        false
    }
}

/// Launches a Chromium-based browser with remote debugging enabled for live automation
#[allow(dead_code)]
pub fn launch_automated_browser(target_url: &str, port: u16) -> Option<std::process::Child> {
    let browsers = detect_browsers();
    let chromium_browser = browsers.iter().find(|b| {
        let n = b.name.to_lowercase();
        n.contains("edge") || n.contains("chrome") || n.contains("brave") || n.contains("chromium")
    })?;

    let debug_flag = format!("--remote-debugging-port={}", port);
    let user_data = format!(
        "--user-data-dir={}/.podcast_rescue_profile",
        std::env::temp_dir().display()
    );

    Command::new(&chromium_browser.executable)
        .arg(&debug_flag)
        .arg(&user_data)
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg(target_url)
        .spawn()
        .ok()
}
