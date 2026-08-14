#!/usr/bin/env bash
# Assemble Ranger.app around an already-built release binary.
# Usage: packaging/macos-app.sh <path-to-binary> <version> [out-dir]
#
# The game loads real files from assets/ at runtime - the map its continents
# come from, the recipe that turns it into ground, and whatever has been
# sculpted at Opificium's terrain bench - so the bundle carries that folder
# BESIDE the binary. Bevy resolves the asset root from the executable's own
# directory, so this is the one arrangement that works from a signed .app.
set -euo pipefail
BIN="${1:?usage: macos-app.sh <binary> <version> [out-dir]}"
VERSION="${2:?need a version}"
OUT="${3:-dist}"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"

APP="$OUT/Ranger.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# The executable's name must match CFBundleExecutable in Info.plist, or macOS
# refuses to launch the bundle.
cp "$BIN" "$APP/Contents/MacOS/ranger"
chmod +x "$APP/Contents/MacOS/ranger"
strip "$APP/Contents/MacOS/ranger" 2>/dev/null || true

cp -R "$ROOT/assets" "$APP/Contents/MacOS/assets"

sed "s/__VERSION__/$VERSION/g" "$HERE/Info.plist" > "$APP/Contents/Info.plist"

# Ad-hoc sign so macOS runs it without a "damaged" error; the launcher also
# strips the download quarantine on install.
codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "built $APP"
