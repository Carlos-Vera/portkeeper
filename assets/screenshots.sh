#!/usr/bin/env bash
# Regenerate the README screenshots (docs/*.png) from the UI with mock data.
# Needs Google Chrome and python3 with Pillow. Usage: bash assets/screenshots.sh
set -euo pipefail
cd "$(dirname "$0")/.."
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
PORT=4179
python3 -m http.server "$PORT" >/dev/null 2>&1 &
SERVER=$!
trap 'kill $SERVER' EXIT
sleep 1

PAD=40
WIN=800
shoot() { # name shot theme
  local url="http://localhost:$PORT/assets/shots.html?shot=$2&theme=$3"
  local common=(--headless=new --hide-scrollbars --disable-gpu --virtual-time-budget=4000 --user-data-dir="$(mktemp -d)")
  local h
  h=$("$CHROME" "${common[@]}" --window-size=$WIN,900 --dump-dom "$url" 2>/dev/null | sed -n 's/.*<title>h:\([0-9]*\)<\/title>.*/\1/p')
  [ -n "$h" ] || { echo "no height for $1"; exit 1; }
  # Headless Chrome has a minimum window width, so render wide and crop to the card + padding.
  "$CHROME" "${common[@]}" --window-size=$WIN,$((h + 2 * PAD)) --force-device-scale-factor=2 \
    --screenshot="docs/$1.png" "$url" 2>/dev/null
  python3 - "docs/$1.png" "$WIN" "$PAD" <<'PY'
import sys
from PIL import Image
path, win, pad = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
im = Image.open(path)
left = ((win - 290) // 2 - pad) * 2
im.crop((left, 0, left + (290 + 2 * pad) * 2, im.height)).save(path, optimize=True)
PY
  echo "docs/$1.png  ($h px card)"
}

shoot active    active   dark
shoot expanded  expanded dark
shoot light     active   light
shoot closed    closed   dark
shoot settings  settings dark
