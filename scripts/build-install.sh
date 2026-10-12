#!/usr/bin/env bash
#
# Build RefineAI from source and install it for the current user:
#   - builds the release binary and copies it onto your PATH
#   - installs an app icon and a .desktop entry so RefineAI shows up in your
#     application menu (the entry opens Settings)
#   - binds the Ctrl+Alt+R global shortcut on GNOME/Ubuntu
#
# For end users who just want the app, prefer the prebuilt one-liner installer
# (scripts/install.sh) — no Rust toolchain needed. This script is for building
# from a checkout.
#
# Usage (run from anywhere; paths resolve to the repo root):
#   scripts/build-install.sh              build + install
#   scripts/build-install.sh --no-build   install an already-built target/release binary
#   scripts/build-install.sh --no-shortcut skip binding the global shortcut
#   scripts/build-install.sh --uninstall  remove everything this script installs
#
# REFINEAI_SHORTCUT=<accel> overrides the shortcut (default: <Control><Alt>r).
# No root required — everything lands under ~/.local.

set -euo pipefail

# Repo root is the parent of this script's directory (scripts/).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APP_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"

BIN_PATH="$BIN_DIR/refineai"
ICON_PATH="$ICON_DIR/refineai.svg"
DESKTOP_PATH="$APP_DIR/refineai.desktop"

SET_SHORTCUT=1
SHORTCUT_BINDING="${REFINEAI_SHORTCUT:-<Control><Alt>r}"

# GNOME custom-keybinding slot we own.
GS_SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
GS_KEY_SCHEMA="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding"
GS_KEY_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/refineai/"

info() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$1" >&2; }

refresh_caches() {
    command -v update-desktop-database >/dev/null 2>&1 && \
        update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && \
        gtk-update-icon-cache -q -t -f "$DATA_DIR/icons/hicolor" >/dev/null 2>&1 || true
}

# --- global shortcut (GNOME/Ubuntu) ------------------------------------------
is_gnome() {
    command -v gsettings >/dev/null 2>&1 || return 1
    gsettings list-schemas 2>/dev/null | grep -qx "$GS_SCHEMA"
}

bind_shortcut() {
    local existing new
    existing="$(gsettings get "$GS_SCHEMA" custom-keybindings 2>/dev/null || echo "@as []")"
    case "$existing" in
        *"$GS_KEY_PATH"*) new="$existing" ;;                 # already in the list
        "@as []"|"[]")    new="['$GS_KEY_PATH']" ;;          # empty list
        *)                new="${existing%]}, '$GS_KEY_PATH']" ;;  # append
    esac
    gsettings set "$GS_SCHEMA" custom-keybindings "$new"
    gsettings set "$GS_KEY_SCHEMA:$GS_KEY_PATH" name 'RefineAI'
    gsettings set "$GS_KEY_SCHEMA:$GS_KEY_PATH" command 'refineai'
    gsettings set "$GS_KEY_SCHEMA:$GS_KEY_PATH" binding "$SHORTCUT_BINDING"
}

unbind_shortcut() {
    local existing new
    existing="$(gsettings get "$GS_SCHEMA" custom-keybindings 2>/dev/null || echo "@as []")"
    case "$existing" in
        *"$GS_KEY_PATH"*)
            new="$(printf '%s' "$existing" \
                | sed -e "s|'$GS_KEY_PATH', ||" \
                      -e "s|, '$GS_KEY_PATH'||" \
                      -e "s|'$GS_KEY_PATH'||")"
            gsettings set "$GS_SCHEMA" custom-keybindings "$new" 2>/dev/null || true
            gsettings reset-recursively "$GS_KEY_SCHEMA:$GS_KEY_PATH" 2>/dev/null || true
            ;;
    esac
}

uninstall() {
    info "Removing RefineAI"
    rm -f "$BIN_PATH" "$ICON_PATH" "$DESKTOP_PATH"
    is_gnome && unbind_shortcut
    refresh_caches
    info "Removed binary, icon, desktop entry and shortcut."
    echo "   Config and history were left untouched:"
    echo "     ~/.config/refineai/      ~/.local/share/refineai/"
    exit 0
}

# --- parse args ---------------------------------------------------------------
BUILD=1
while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall)   uninstall ;;
        --no-build)    BUILD=0 ;;
        --no-shortcut) SET_SHORTCUT=0 ;;
        -h|--help)     sed -n '3,22p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done

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

# --- bind the global shortcut -------------------------------------------------
SHORTCUT_OK=0
if [ "$SET_SHORTCUT" -eq 1 ]; then
    if is_gnome; then
        info "Binding shortcut   → $SHORTCUT_BINDING"
        bind_shortcut
        SHORTCUT_OK=1
    else
        warn "couldn't auto-bind a shortcut (this isn't a GNOME session, or gsettings"
        echo "         is unavailable). Bind the command 'refineai' to a hotkey yourself."
    fi
fi

refresh_caches

echo
info "Done."

# Nudge if the install dir isn't on PATH yet.
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) warn "$BIN_DIR is not on your PATH. Add this to your shell profile:"
       echo "       export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

if [ "$SHORTCUT_OK" -eq 1 ]; then
cat <<EOF

Next steps:
  1. Add your Gemini API key:   refineai --settings
     (or launch "RefineAI" from your application menu)
  2. Select text anywhere, press $SHORTCUT_BINDING, and refine.

Uninstall any time with:  scripts/build-install.sh --uninstall
EOF
else
cat <<EOF

Next steps:
  1. Add your Gemini API key:   refineai --settings
     (or launch "RefineAI" from your application menu)
  2. Bind a global shortcut to the command:  refineai
     GNOME:  Settings → Keyboard → Custom Shortcuts → +
             Name: RefineAI   Command: refineai   Shortcut: e.g. Ctrl+Alt+R
  3. Select text anywhere, press the shortcut, and refine.

Uninstall any time with:  scripts/build-install.sh --uninstall
EOF
fi
