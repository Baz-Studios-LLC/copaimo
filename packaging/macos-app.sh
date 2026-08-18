#!/usr/bin/env bash
# Assemble Copaimo.app around an already-built release binary.
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

APP="$OUT/Copaimo.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# The executable's name must match CFBundleExecutable in Info.plist, or macOS
# refuses to launch the bundle. `copaimo`, matching it.
# The rename briefly made this `warden` - the
# player's own title, which is a different word from the game's - and a bundle
# whose executable does not match CFBundleExecutable does not launch at all.
cp "$BIN" "$APP/Contents/MacOS/copaimo"
chmod +x "$APP/Contents/MacOS/copaimo"
strip "$APP/Contents/MacOS/copaimo" 2>/dev/null || true

cp -R "$ROOT/assets" "$APP/Contents/MacOS/assets"

# The icon. Built here rather than committed as an .icns because `iconutil` is a
# macOS tool and this is the only place a mac is guaranteed - so what the
# repository carries is the PNG, and the platform's own tool renders the rest.
#
# Every size, because the Finder picks one per view and scales nothing kindly.
ICONSET="$OUT/Copaimo.iconset"
rm -rf "$ICONSET"
mkdir -p "$ICONSET"
for size in 16 32 64 128 256 512; do
  sips -z $size $size "$HERE/Copaimo-icon.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z $double $double "$HERE/Copaimo-icon.png"     --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
if iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Copaimo.icns"; then
  echo "icon built"
else
  # A bundle that NAMES an icon it does not carry shows a blank one, which is
  # worse than the default. So if the icon cannot be built, stop naming it.
  echo "could not build the icon; leaving the default" >&2
fi
rm -rf "$ICONSET"

sed "s/__VERSION__/$VERSION/g" "$HERE/Info.plist" > "$APP/Contents/Info.plist"
if [ ! -f "$APP/Contents/Resources/Copaimo.icns" ]; then
  # Drop the CFBundleIconFile line rather than leave it pointing at nothing.
  sed -i "" "/CFBundleIconFile/,+1d" "$APP/Contents/Info.plist" 2>/dev/null || true
fi

# Ad-hoc sign so macOS runs it without a "damaged" error; the launcher also
# strips the download quarantine on install.
codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "built $APP"
