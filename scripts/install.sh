#!/usr/bin/env bash
#
# RefineAI — one-line installer for end users.
#
# Downloads the latest prebuilt `refineai` binary from GitHub Releases, drops it
# on your PATH, adds an app icon + menu entry, and (on GNOME/Ubuntu) binds the
# Ctrl+Alt+R global shortcut for you. No Rust toolchain required.
#
# Quick install:
#   curl -fsSL https://raw.githubusercontent.com/anurag-verma-india/refineai-linux/main/scripts/install.sh | bash
#
# Options (as args, or env vars when piping through a shell):
#   --uninstall              remove everything this script installs
#   --version <tag>          install a specific release (default: latest)
#   --no-shortcut            skip binding the global shortcut
#   REFINEAI_VERSION=<tag>   same as --version, handy with curl | bash
#   REFINEAI_SHORTCUT=<accel> override the shortcut (default: <Control><Alt>r)
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
SET_SHORTCUT=1
SHORTCUT_BINDING="${REFINEAI_SHORTCUT:-<Control><Alt>r}"

# GNOME custom-keybinding slot we own.
GS_SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
GS_KEY_SCHEMA="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding"
GS_KEY_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/refineai/"

info() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$1" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

refresh_caches() {
    command -v update-desktop-database >/dev/null 2>&1 && \
        update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && \
        gtk-update-icon-cache -q -t -f "$DATA_DIR/icons/hicolor" >/dev/null 2>&1 || true
}

# --- global shortcut (GNOME/Ubuntu) ------------------------------------------
# Only GNOME exposes custom keybindings through gsettings. On anything else we
# print instructions and move on — the binary still works, it just needs a
# hotkey bound by hand.
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
while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall)   uninstall ;;
        --version)     VERSION="${2:?--version needs a tag}"; shift ;;
        --no-shortcut) SET_SHORTCUT=0 ;;
        -h|--help)     sed -n '3,21p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
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

# --- bind the global shortcut -------------------------------------------------
SHORTCUT_OK=0
if [ "$SET_SHORTCUT" -eq 1 ]; then
    if is_gnome; then
        info "Binding shortcut   → $SHORTCUT_BINDING"
        bind_shortcut
        SHORTCUT_OK=1
    else
        warn "couldn't auto-bind a shortcut (this isn't a GNOME session, or gsettings"
        echo "         is unavailable). Bind the command 'refineai' to a hotkey yourself —"
        echo "         see the README for KDE / Sway / Hyprland."
    fi
fi

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

if [ "$SHORTCUT_OK" -eq 1 ]; then
cat <<EOF

Next steps:
  1. Add your Gemini API key:   refineai --settings
     (or launch "RefineAI" from your application menu)
  2. Select text anywhere, press $SHORTCUT_BINDING, and refine.
     (rebind it in Settings → Keyboard → Custom Shortcuts)

Uninstall any time with:
  curl -fsSL $RAW/scripts/install.sh | bash -s -- --uninstall
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

Uninstall any time with:
  curl -fsSL $RAW/scripts/install.sh | bash -s -- --uninstall
EOF
fi
