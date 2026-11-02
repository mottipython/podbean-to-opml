# Security policy

## Reporting a vulnerability

Report security problems privately through GitHub:
**Security tab → Report a vulnerability**. Please do not open a public issue for something exploitable.

You can expect an acknowledgement within a week.

## Supported versions

Only the latest release receives fixes.

## How the project is checked

| Check | When | Workflow |
| --- | --- | --- |
| Build, tests, `clippy -D warnings`, `rustfmt`, Docker build and start | every push and pull request | [CI](.github/workflows/ci.yml) |
| `cargo audit` against the RustSec advisory database | every push, pull request, and weekly | [Security](.github/workflows/security.yml) |
| Snyk Code static analysis | every push, pull request, and weekly | [Security](.github/workflows/security.yml) |
| Fuzzing of the parser, resolver and OPML code (Rust and Python harnesses) | weekly and on demand | [Security](.github/workflows/security.yml) |
| Dependency update pull requests | weekly | [Dependabot](.github/dependabot.yml) |
| Secret scanning with push protection | always on | GitHub |

## Design notes

The app is a local web server. It listens on `127.0.0.1` by default.

- Every API call except the sync endpoint needs a random per-run token that only the served page receives.
- The sync endpoint accepts cross-origin posts only from podbean.com, Spotify, Apple Podcasts and YouTube Music over HTTPS.
- Requests whose `Host` is not a loopback name are refused, which blocks DNS rebinding.
- The server exits after 5 idle minutes, or when you press **Quit**.
- The browser extension runs only on `*.podbean.com` and the local app, and sends data only to the app's own address.
- Imported pages are parsed as text; nothing in them is executed.
- Binding to `0.0.0.0` (for example in Docker) exposes the app to your network and turns off the Host and idle checks. Only do that on a trusted network.
