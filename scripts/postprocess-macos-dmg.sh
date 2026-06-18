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

if [[ ! -f "$dmg_path" ]]; then
  echo "DMG not found: $dmg_path" >&2
  exit 66
fi

if [[ ! -f "$readme_path" ]]; then
  echo "README not found: $readme_path" >&2
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

sync
hdiutil detach "$mount_dir" -quiet
mounted=false

hdiutil convert "$rw_dmg" -format UDZO -imagekey zlib-level=9 -o "$final_dmg" -quiet
mv "$final_dmg" "$dmg_path"

echo "Postprocessed macOS DMG: $dmg_path"
