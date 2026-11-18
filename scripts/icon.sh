#!/usr/bin/env bash
#
# Regenerate the Alfred workflow icon (workflow/icon.png) from the icon of the
# installed Brave Browser app. Alfred uses a PNG, so we convert the .icns.
#
# Requires: Brave Browser in /Applications, sips (ships with macOS).
#
# Usage: scripts/icon.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

ICNS="/Applications/Brave Browser.app/Contents/Resources/app.icns"

if [[ ! -f "$ICNS" ]]; then
  echo "error: $ICNS not found. Install Brave Browser first." >&2
  exit 1
fi

sips -s format png -Z 512 "$ICNS" --out workflow/icon.png >/dev/null

echo "==> Wrote workflow/icon.png ($(sips -g pixelWidth -g pixelHeight workflow/icon.png | awk '/pixel/{print $2}' | paste -sd x -))"
