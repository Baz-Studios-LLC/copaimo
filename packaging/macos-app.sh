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
# ALL of it inside one guard, and that matters more than it looks.
#
# This script runs under `set -euo pipefail`, so any unguarded command that fails
# ends the whole packaging — and packaging is the step that produces the release.
# An icon is a nicety; a release is not. Losing the release to a missing `sips`
# would be the same shape of fault that has already cost one: a mac-only path,
# never run anywhere else, taking the build down with it.
#
# So the icon is built in a subshell whose failure is caught, and everything after
# it asks whether the file actually arrived rather than assuming it did.
# Every step checked BY HAND rather than left to `set -e`.
#
# `set -e` does nothing here and that is not obvious: bash suppresses it for any
# command whose status is being tested, and this whole function is called as an
# `if` condition. Written the other way it ran to the end with every `sips`
# failing and returned the status of the final `rm` — success — so it reported
# "icon built" over a bundle with no icon in it. Tested with a `sips` that fails,
# which is the only way that showed up.
build_icon() (
  ICONSET="$OUT/Copaimo.iconset"
  rm -rf "$ICONSET"
  mkdir -p "$ICONSET" || return 1
  # Every size, because the Finder picks one per view and scales nothing kindly.
  for size in 16 32 64 128 256 512; do
    double=$((size * 2))
    sips -z "$size" "$size" "$HERE/Copaimo-icon.png"       --out "$ICONSET/icon_${size}x${size}.png" >/dev/null || return 1
    sips -z "$double" "$double" "$HERE/Copaimo-icon.png"       --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null || return 1
  done
  iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Copaimo.icns" || return 1
  # And the file has to actually be there. `iconutil` returning nought with
  # nothing written would otherwise be reported as a success.
  [ -f "$APP/Contents/Resources/Copaimo.icns" ] || return 1
  rm -rf "$ICONSET"
)

if build_icon; then
  echo "icon built"
else
  echo "could not build the icon; the app keeps the default one" >&2
  rm -rf "$OUT/Copaimo.iconset"
fi

sed "s/__VERSION__/$VERSION/g" "$HERE/Info.plist" > "$APP/Contents/Info.plist"
if [ ! -f "$APP/Contents/Resources/Copaimo.icns" ]; then
  # A bundle that NAMES an icon it does not carry shows a BLANK one, which is
  # worse than the default. So if the icon did not get built, stop naming it.
  sed -i "" "/CFBundleIconFile/,+1d" "$APP/Contents/Info.plist" || true
fi

# Ad-hoc sign so macOS runs it without a "damaged" error; the launcher also
# strips the download quarantine on install.
codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "built $APP"
