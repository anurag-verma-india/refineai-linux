#!/usr/bin/env bash
#
# Install (or uninstall) RefineAI for the current user:
#   - builds the release binary and copies it onto your PATH
#   - installs an app icon and a .desktop entry so RefineAI shows up in your
#     application menu (the entry opens Settings; day-to-day you trigger the
#     popup with a global hotkey — see the printed instructions)
#
# Usage:
#   ./install.sh              build + install
#   ./install.sh --no-build   install an already-built target/release binary
#   ./install.sh --uninstall  remove everything this script installs
#
# No root required — everything lands under ~/.local.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APP_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"

BIN_PATH="$BIN_DIR/refineai"
ICON_PATH="$ICON_DIR/refineai.svg"
DESKTOP_PATH="$APP_DIR/refineai.desktop"

info() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$1" >&2; }

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

BUILD=1
case "${1:-}" in
    --uninstall) uninstall ;;
    --no-build)  BUILD=0 ;;
    "")          ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
esac

if [ "$BUILD" -eq 1 ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: 'cargo' not found. Install Rust from https://rustup.rs, or re-run with --no-build." >&2
        exit 1
    fi
    info "Building release binary"
    cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
fi

SRC_BIN="$SCRIPT_DIR/target/release/refineai"
if [ ! -x "$SRC_BIN" ]; then
    echo "error: $SRC_BIN not found. Build first (drop --no-build)." >&2
    exit 1
fi

info "Installing binary  → $BIN_PATH"
install -Dm755 "$SRC_BIN" "$BIN_PATH"

info "Installing icon    → $ICON_PATH"
install -Dm644 "$SCRIPT_DIR/assets/refineai.svg" "$ICON_PATH"

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

echo
info "Done."

# Nudge if the install dir isn't on PATH yet.
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

Uninstall any time with:  ./install.sh --uninstall
EOF
