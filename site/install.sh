#!/bin/sh
# Installs or updates Portkeeper on macOS or Linux, without the Gatekeeper dialog.
#   curl -fsSL https://portkeeper.pages.dev/install.sh | sh
# Downloads the latest release from GitHub over HTTPS. Files fetched by curl carry
# no quarantine flag, so macOS does not show the "could not verify" dialog.
# Later updates come from inside the app. Source: https://github.com/MarcAparisiTFG/portkeeper
# Windows: irm https://portkeeper.pages.dev/install.ps1 | iex
set -eu

LATEST="https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

case "$(uname -s)" in
  Darwin)
    case "$(uname -m)" in
      arm64) ARCH=aarch64 ;;
      x86_64) ARCH=x64 ;;
      *) echo "Unsupported CPU: $(uname -m)"; exit 1 ;;
    esac
    echo "Downloading Portkeeper for macOS ($ARCH)…"
    curl -fsSL "$LATEST/Portkeeper_${ARCH}.app.tar.gz" -o "$TMP/app.tar.gz"
    tar -xzf "$TMP/app.tar.gz" -C "$TMP"
    [ -d "$TMP/Portkeeper.app" ] || { echo "Unexpected archive layout"; exit 1; }
    if pgrep -fq "/Applications/Portkeeper.app/Contents/MacOS/portkeeper"; then
      echo "Closing the running Portkeeper…"
      pkill -f "/Applications/Portkeeper.app/Contents/MacOS/portkeeper" || true
      sleep 1
    fi
    rm -rf /Applications/Portkeeper.app
    mv "$TMP/Portkeeper.app" /Applications/Portkeeper.app
    xattr -cr /Applications/Portkeeper.app 2>/dev/null || true
    VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' /Applications/Portkeeper.app/Contents/Info.plist 2>/dev/null || echo '?')"
    [ "${PORTKEEPER_NO_LAUNCH:-}" ] || open /Applications/Portkeeper.app
    echo "Portkeeper $VERSION is installed and running in your menu bar."
    ;;
  Linux)
    [ "$(uname -m)" = "x86_64" ] || { echo "Only x86_64 Linux builds exist for now: https://github.com/MarcAparisiTFG/portkeeper/releases"; exit 1; }
    BIN="$HOME/.local/bin"; APPS="$HOME/.local/share/applications"
    mkdir -p "$BIN" "$APPS"
    echo "Downloading Portkeeper for Linux (x86_64)…"
    curl -fsSL "$LATEST/Portkeeper-Linux-x86_64.AppImage" -o "$TMP/portkeeper"
    chmod +x "$TMP/portkeeper"
    pkill -f "$BIN/portkeeper" 2>/dev/null || true
    mv "$TMP/portkeeper" "$BIN/portkeeper"
    cat > "$APPS/portkeeper.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Portkeeper
Comment=See the ports you have open and who started them
Exec=$BIN/portkeeper
Terminal=false
Categories=Development;Utility;
DESKTOP
    [ "${PORTKEEPER_NO_LAUNCH:-}" ] || (nohup "$BIN/portkeeper" >/dev/null 2>&1 &)
    echo "Portkeeper is installed at $BIN/portkeeper and running in your tray."
    case ":$PATH:" in *":$BIN:"*) ;; *) echo "Tip: add $BIN to your PATH to run it as 'portkeeper'." ;; esac
    ;;
  *)
    echo "This installer is for macOS and Linux. Windows: irm https://portkeeper.pages.dev/install.ps1 | iex"; exit 1 ;;
esac
echo "Updates arrive inside the app. Settings, locks and the Closed list are kept."
