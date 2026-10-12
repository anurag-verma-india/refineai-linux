#!/usr/bin/env bash
#
# RefineAI — one-line installer for end users.
#
# Downloads the latest prebuilt `refineai` binary from GitHub Releases, drops it
# on your PATH, and adds an app icon + menu entry. No Rust toolchain required.
#
# Quick install:
#   curl -fsSL https://raw.githubusercontent.com/anurag-verma-india/refineai-linux/main/scripts/install.sh | bash
#
# Options (as args, or env vars when piping through a shell):
#   --uninstall              remove everything this script installs
#   --version <tag>          install a specific release (default: latest)
#   REFINEAI_VERSION=<tag>   same as --version, handy with curl | bash
#
# Everything lands under ~/.local — no root required.

set -euo pipefail

REPO="anurag-verma-india/refineai-linux"
RAW="https://raw.githubusercontent.com/$REPO/main"

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APP_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"

BIN_PATH="$BIN_DIR/refineai"
ICON_PATH="$ICON_DIR/refineai.svg"
DESKTOP_PATH="$APP_DIR/refineai.desktop"

VERSION="${REFINEAI_VERSION:-latest}"

info() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$1" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

refresh_caches() {
    command -v update-desktop-database >/dev/null 2>&1 && \
        update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && \
        gtk-update-icon-cache -q -t -f "$DATA_DIR/icons/hicolor" >/dev/null 2>&1 || true
}

uninstall() {
    info "Removing RefineAI"
    rm -f "$BIN_PATH" "$ICON_PATH" "$DESKTOP_PATH"
    refresh_caches
    info "Removed binary, icon and desktop entry."
    echo "   Config and history were left untouched:"
    echo "     ~/.config/refineai/      ~/.local/share/refineai/"
    exit 0
}

# --- parse args ---------------------------------------------------------------
while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall) uninstall ;;
        --version)   VERSION="${2:?--version needs a tag}"; shift ;;
        -h|--help)   sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) die "unknown option: $1" ;;
    esac
    shift
done

# --- pick a downloader --------------------------------------------------------
if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -qO "$2" "$1"; }
else
    die "need 'curl' or 'wget' to download. Install one and re-run."
fi

# --- resolve the release asset for this CPU -----------------------------------
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)   ASSET="refineai-x86_64" ;;
    aarch64|arm64)  ASSET="refineai-aarch64" ;;
    *) die "unsupported architecture '$ARCH'. Build from source instead (see README)." ;;
esac

if [ "$VERSION" = "latest" ]; then
    BIN_URL="https://github.com/$REPO/releases/latest/download/$ASSET"
else
    BIN_URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET"
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

info "Downloading refineai ($VERSION, $ARCH)"
fetch "$BIN_URL" "$TMP/refineai" \
    || die "download failed: $BIN_URL
       The release may not have a '$ASSET' asset yet. See docs/RELEASING.md."

# A GitHub 404 page is small HTML, not an ELF binary — sanity-check before installing.
if ! head -c 4 "$TMP/refineai" | grep -q $'\x7fELF'; then
    die "downloaded file is not a Linux binary (got a 404 or HTML page?). URL: $BIN_URL"
fi

info "Installing binary  → $BIN_PATH"
install -Dm755 "$TMP/refineai" "$BIN_PATH"

info "Installing icon    → $ICON_PATH"
if fetch "$RAW/assets/refineai.svg" "$TMP/refineai.svg"; then
    install -Dm644 "$TMP/refineai.svg" "$ICON_PATH"
else
    warn "could not fetch the app icon; the menu entry will use a generic icon."
fi

info "Installing desktop entry → $DESKTOP_PATH"
mkdir -p "$APP_DIR"
cat > "$DESKTOP_PATH" <<EOF
[Desktop Entry]
Type=Application
Name=RefineAI
GenericName=AI Text Rewriter
Comment=Rewrite selected text with Gemini
Exec=$BIN_PATH --settings
Icon=refineai
Terminal=false
Categories=Utility;TextTools;
Keywords=AI;Gemini;Rewrite;Refine;Text;
StartupNotify=true
EOF
chmod 644 "$DESKTOP_PATH"

refresh_caches

# --- friendly checks ----------------------------------------------------------
if ! command -v wl-copy >/dev/null 2>&1 && ! command -v xclip >/dev/null 2>&1; then
    warn "no clipboard helper found. Install 'wl-clipboard' (Wayland) or 'xclip' (X11):"
    echo "         sudo apt install wl-clipboard   # or: xclip"
fi

echo
info "Done."

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) warn "$BIN_DIR is not on your PATH. Add this to your shell profile:"
       echo "       export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

cat <<EOF

Next steps:
  1. Add your Gemini API key:   refineai --settings
     (or launch "RefineAI" from your application menu)
  2. Bind a global shortcut to the command:  refineai
     GNOME:  Settings → Keyboard → Custom Shortcuts → +
             Name: RefineAI   Command: refineai   Shortcut: e.g. Ctrl+Alt+R
  3. Select text anywhere, press the shortcut, and refine.

Uninstall any time with:
  curl -fsSL $RAW/scripts/install.sh | bash -s -- --uninstall
EOF
