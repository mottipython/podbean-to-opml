#!/usr/bin/env bash
# Builds store-ready zips: dist/podcast-rescue-podbean-{chromium,firefox}-<version>.zip
set -euo pipefail
cd "$(dirname "$0")"
ver=$(grep -m1 '^version' ../Cargo.toml | cut -d'"' -f2)
out=../dist/store; mkdir -p "$out"
for target in chromium firefox; do
  tmp=$(mktemp -d)
  cp app.html app.css app.js resolver.js opml.js crawl.js background.js icon*.png "$tmp"
  src=manifest.json; [ $target = firefox ] && src=manifest.firefox.json
  sed "s/__VERSION__/$ver/" $src > "$tmp/manifest.json"
  rm -f "$out/podcast-rescue-podbean-$target-$ver.zip"
  (cd "$tmp" && zip -qr "$OLDPWD/$out/podcast-rescue-podbean-$target-$ver.zip" .)
  [ $target = chromium ] && { rm -rf "$out/unpacked"; cp -r "$tmp" "$out/unpacked"; }
  rm -rf "$tmp"
done
ls "$out"
