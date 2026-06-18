#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <path-to-dmg> [readme-path]" >&2
  exit 64
fi

if [[ "${OSTYPE:-}" != darwin* ]]; then
  echo "macOS DMG postprocessing requires macOS." >&2
  exit 1
fi

dmg_path="$1"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readme_path="${2:-"$script_dir/macos-dmg-readme.txt"}"
background_source="$script_dir/macos-dmg-background.swift"

if [[ ! -f "$dmg_path" ]]; then
  echo "DMG not found: $dmg_path" >&2
  exit 66
fi

if [[ ! -f "$readme_path" ]]; then
  echo "README not found: $readme_path" >&2
  exit 66
fi

if [[ ! -f "$background_source" ]]; then
  echo "DMG background source not found: $background_source" >&2
  exit 66
fi

tmp_dir="$(mktemp -d)"
mount_dir="$tmp_dir/mount"
rw_dmg="$tmp_dir/harubble-rw.dmg"
final_dmg="$tmp_dir/harubble-final.dmg"
mounted=false

cleanup() {
  if [[ "$mounted" == true ]]; then
    hdiutil detach "$mount_dir" -quiet >/dev/null 2>&1 ||
      hdiutil detach "$mount_dir" -force -quiet >/dev/null 2>&1 ||
      true
  fi
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

mkdir -p "$mount_dir"

hdiutil convert "$dmg_path" -format UDRW -o "$rw_dmg" -quiet
hdiutil attach "$rw_dmg" -readwrite -noverify -noautoopen -mountpoint "$mount_dir" -quiet
mounted=true

apps=()
while IFS= read -r -d '' app; do
  apps+=("$app")
done < <(find "$mount_dir" -maxdepth 1 -type d -name '*.app' -print0)

if [[ "${#apps[@]}" -ne 1 ]]; then
  echo "Expected exactly one .app bundle in DMG, found ${#apps[@]}." >&2
  printf 'Found app bundles:\n' >&2
  printf '  %s\n' "${apps[@]}" >&2
  exit 1
fi

cp "$readme_path" "$mount_dir/README-macOS.txt"
if [[ ! -e "$mount_dir/Applications" ]]; then
  ln -s /Applications "$mount_dir/Applications"
fi
mkdir -p "$mount_dir/.background"
swift "$background_source" "$mount_dir/.background/harubble-dmg-background.png"
chflags hidden "$mount_dir/.background" 2>/dev/null || true
SetFile -a V "$mount_dir/.background" 2>/dev/null || true

osascript - "$mount_dir" <<'APPLESCRIPT'
on run argv
  set mountPath to item 1 of argv
  set dmgFolder to POSIX file mountPath as alias
  set backgroundFile to POSIX file (mountPath & "/.background/harubble-dmg-background.png") as alias

  tell application "Finder"
    open dmgFolder
    delay 1

    set dmgWindow to container window of dmgFolder
    set current view of dmgWindow to icon view
    set toolbar visible of dmgWindow to false
    set statusbar visible of dmgWindow to false
    set bounds of dmgWindow to {100, 100, 820, 520}

    set dmgViewOptions to icon view options of dmgWindow
    set background picture of dmgViewOptions to backgroundFile
    set arrangement of dmgViewOptions to not arranged
    set icon size of dmgViewOptions to 72

    set position of item "README-macOS.txt" of dmgFolder to {180, 122}
    set position of item "Harubble.app" of dmgFolder to {180, 254}
    set position of item "Applications" of dmgFolder to {540, 254}
    update dmgFolder without registering applications
    delay 1
    close dmgWindow
  end tell
end run
APPLESCRIPT

sync
hdiutil detach "$mount_dir" -quiet
mounted=false

hdiutil convert "$rw_dmg" -format UDZO -imagekey zlib-level=9 -o "$final_dmg" -quiet
mv "$final_dmg" "$dmg_path"

echo "Postprocessed macOS DMG: $dmg_path"
