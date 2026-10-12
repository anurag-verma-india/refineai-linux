#!/usr/bin/env bash
#
# Cut and publish a RefineAI release in one go.
#
# Bumps the version, builds the release binary, tags, pushes, and creates a
# GitHub Release with the binary attached (via the `gh` CLI). The one-line user
# installer (scripts/install.sh) downloads the asset this uploads, so the names
# here must match docs/RELEASING.md.
#
# Usage:
#   ./release.sh <version> [options]
#
#   <version>              Release version, e.g. 0.1.0 or v0.1.0  (required)
#
# Options:
#   -t, --title <text>     Release title        (default: "RefineAI v<version>")
#   -n, --notes <text>     Release notes text
#   -F, --notes-file <f>   Release notes from a file
#                          (default when neither given: auto-generate from commits)
#       --aarch64          Also cross-build an aarch64 binary (needs `cross`)
#       --draft            Create the release as a draft
#       --prerelease       Mark the release as a prerelease
#       --skip-tests       Don't run `cargo test` before building
#       --no-push          Don't `git push` (gh will still create the tag remotely)
#   -y, --yes              Don't ask for confirmation
#   -h, --help             Show this help
#
# Requires: cargo, git, and the GitHub CLI (`gh`, authenticated with `gh auth login`).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="anurag-verma-india/refineai-linux"
RAW="https://raw.githubusercontent.com/$REPO/main"

info() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$1" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

usage() { sed -n '2,28p' "$0" | sed 's/^#\{1,\} \{0,1\}//; s/^#$//'; exit "${1:-0}"; }

# --- parse args ---------------------------------------------------------------
VERSION=""
TITLE=""
NOTES=""
NOTES_FILE=""
BUILD_AARCH64=0
DRAFT=0
PRERELEASE=0
SKIP_TESTS=0
NO_PUSH=0
ASSUME_YES=0

while [ $# -gt 0 ]; do
    case "$1" in
        -t|--title)      TITLE="${2:?--title needs a value}"; shift ;;
        -n|--notes)      NOTES="${2:?--notes needs a value}"; shift ;;
        -F|--notes-file) NOTES_FILE="${2:?--notes-file needs a path}"; shift ;;
        --aarch64)       BUILD_AARCH64=1 ;;
        --draft)         DRAFT=1 ;;
        --prerelease)    PRERELEASE=1 ;;
        --skip-tests)    SKIP_TESTS=1 ;;
        --no-push)       NO_PUSH=1 ;;
        -y|--yes)        ASSUME_YES=1 ;;
        -h|--help)       usage 0 ;;
        -*)              die "unknown option: $1 (see --help)" ;;
        *)               [ -z "$VERSION" ] && VERSION="$1" || die "unexpected argument: $1" ;;
    esac
    shift
done

[ -n "$VERSION" ] || { warn "a version is required"; usage 2; }

# Normalize: VER has no leading v, TAG always has one.
VER="${VERSION#v}"
TAG="v$VER"
echo "$VER" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$' \
    || die "'$VERSION' is not a semver version (expected e.g. 0.1.0)"

[ -n "$TITLE" ] || TITLE="RefineAI $TAG"

# --- preflight ----------------------------------------------------------------
for t in cargo git gh; do
    command -v "$t" >/dev/null 2>&1 || die "'$t' not found on PATH."
done
gh auth status >/dev/null 2>&1 || die "GitHub CLI not authenticated. Run: gh auth login"
[ -n "$NOTES_FILE" ] && [ ! -f "$NOTES_FILE" ] && die "notes file not found: $NOTES_FILE"

cd "$SCRIPT_DIR"

git rev-parse --git-dir >/dev/null 2>&1 || die "not inside a git repository."
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    warn "working tree has uncommitted changes (ignoring untracked files)."
fi
if git rev-parse "$TAG" >/dev/null 2>&1; then
    die "tag $TAG already exists. Bump the version or delete the tag first."
fi

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)  HOST_ASSET="refineai-x86_64" ;;
    aarch64|arm64) HOST_ASSET="refineai-aarch64" ;;
    *) die "unsupported host architecture '$ARCH'." ;;
esac

# --- confirm ------------------------------------------------------------------
cat <<EOF

  Version   : $VER   (tag $TAG)
  Title     : $TITLE
  Notes     : $( [ -n "$NOTES_FILE" ] && echo "from $NOTES_FILE" || { [ -n "$NOTES" ] && echo "inline" || echo "auto-generated from commits"; } )
  Host asset: $HOST_ASSET
  aarch64   : $( [ "$BUILD_AARCH64" -eq 1 ] && echo "yes (cross)" || echo "no" )
  Draft     : $( [ "$DRAFT" -eq 1 ] && echo "yes" || echo "no" )
  Prerelease: $( [ "$PRERELEASE" -eq 1 ] && echo "yes" || echo "no" )
  Push      : $( [ "$NO_PUSH" -eq 1 ] && echo "no" || echo "yes" )

EOF

if [ "$ASSUME_YES" -ne 1 ]; then
    printf "Proceed? [y/N] "
    read -r reply
    case "$reply" in y|Y|yes|YES) ;; *) die "aborted." ;; esac
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ASSETS=()

# --- 1. bump Cargo.toml version -----------------------------------------------
CUR_VER="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')"
if [ "$CUR_VER" != "$VER" ]; then
    info "Bumping Cargo.toml: $CUR_VER → $VER"
    sed -i -E "0,/^version = \".*\"/s//version = \"$VER\"/" Cargo.toml
    cargo build --release --quiet                 # refresh Cargo.lock
    git add Cargo.toml Cargo.lock
    git commit -m "release: $TAG" --quiet
else
    info "Cargo.toml already at $VER"
fi

# --- 2. tests -----------------------------------------------------------------
if [ "$SKIP_TESTS" -eq 1 ]; then
    warn "skipping tests (--skip-tests)"
else
    info "Running tests"
    cargo test --release --quiet
fi

# --- 3. build host binary -----------------------------------------------------
info "Building release binary ($ARCH)"
cargo build --release --quiet
cp "target/release/refineai" "$WORK/$HOST_ASSET"
ASSETS+=("$WORK/$HOST_ASSET")

# --- 4. optional aarch64 cross-build ------------------------------------------
if [ "$BUILD_AARCH64" -eq 1 ]; then
    command -v cross >/dev/null 2>&1 || die "--aarch64 needs 'cross' (cargo install cross)."
    info "Cross-building aarch64"
    cross build --release --target aarch64-unknown-linux-gnu --quiet
    cp "target/aarch64-unknown-linux-gnu/release/refineai" "$WORK/refineai-aarch64"
    [ "$HOST_ASSET" = "refineai-aarch64" ] || ASSETS+=("$WORK/refineai-aarch64")
fi

# --- 5. tag and push ----------------------------------------------------------
info "Tagging $TAG"
git tag -a "$TAG" -m "$TITLE"

if [ "$NO_PUSH" -ne 1 ]; then
    info "Pushing commits and tag"
    git push
    git push origin "$TAG"
fi

# --- 6. create the GitHub Release ---------------------------------------------
info "Creating GitHub Release"
gh_args=(release create "$TAG" "${ASSETS[@]}" --repo "$REPO" --title "$TITLE")
[ "$DRAFT" -eq 1 ]      && gh_args+=(--draft)
[ "$PRERELEASE" -eq 1 ] && gh_args+=(--prerelease)
if [ -n "$NOTES_FILE" ]; then
    gh_args+=(--notes-file "$NOTES_FILE")
elif [ -n "$NOTES" ]; then
    gh_args+=(--notes "$NOTES")
else
    gh_args+=(--generate-notes)
fi
gh "${gh_args[@]}"

echo
info "Released $TAG 🎉"
echo "  Assets: ${ASSETS[*]##*/}"
echo
echo "  Users can now install with:"
echo "    curl -fsSL $RAW/scripts/install.sh | bash"
