#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PACKAGE_JSON="$ROOT_DIR/package.json"
CARGO_TOML="$ROOT_DIR/src-tauri/Cargo.toml"
TAURI_CONFIG="$ROOT_DIR/src-tauri/tauri.conf.json"
RELEASE_DIR="$ROOT_DIR/release-artifacts"
ORIGINAL_PACKAGE_JSON=""
ORIGINAL_CARGO_TOML=""
ORIGINAL_TAURI_CONFIG=""

usage() {
  cat <<'EOF'
Usage:
  ./scripts/release.sh <version>

Example:
  ./scripts/release.sh 0.0.2

This script:
  1. Updates version numbers
  2. Builds release artifacts
  3. Commits the version bump
  4. Creates and pushes a git tag
  5. Creates a GitHub release and uploads zip/dmg/updater assets
EOF
}

require_clean_worktree() {
  if [[ -n "$(git status --short)" ]]; then
    echo "Working tree is not clean. Commit or stash changes before releasing." >&2
    exit 1
  fi
}

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required command: $command_name" >&2
    exit 1
  fi
}

restore_versions_on_failure() {
  local exit_code="$?"
  if [[ "$exit_code" -ne 0 ]]; then
    [[ -n "$ORIGINAL_PACKAGE_JSON" ]] && printf '%s' "$ORIGINAL_PACKAGE_JSON" > "$PACKAGE_JSON"
    [[ -n "$ORIGINAL_CARGO_TOML" ]] && printf '%s' "$ORIGINAL_CARGO_TOML" > "$CARGO_TOML"
    [[ -n "$ORIGINAL_TAURI_CONFIG" ]] && printf '%s' "$ORIGINAL_TAURI_CONFIG" > "$TAURI_CONFIG"
  fi
  exit "$exit_code"
}

trap restore_versions_on_failure EXIT

if [[ $# -ne 1 ]]; then
  usage
  exit 1
fi

VERSION="$1"
TAG="v$VERSION"
ARCH="$(uname -m)"
ZIP_PATH="$RELEASE_DIR/Battery Deck_${VERSION}_${ARCH}.zip"
DMG_PATH="$RELEASE_DIR/Battery Deck_${VERSION}_${ARCH}.dmg"
UPDATER_PATH="$RELEASE_DIR/Battery Deck_${VERSION}_${ARCH}.app.tar.gz"
UPDATER_SIG_PATH="$RELEASE_DIR/Battery Deck_${VERSION}_${ARCH}.app.tar.gz.sig"
LATEST_JSON_PATH="$RELEASE_DIR/latest.json"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Version must use semver format, for example 0.0.2" >&2
  exit 1
fi

require_command git
require_command gh
require_command perl

cd "$ROOT_DIR"
require_clean_worktree

ORIGINAL_PACKAGE_JSON="$(cat "$PACKAGE_JSON")"
ORIGINAL_CARGO_TOML="$(cat "$CARGO_TOML")"
ORIGINAL_TAURI_CONFIG="$(cat "$TAURI_CONFIG")"

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Git tag already exists: $TAG" >&2
  exit 1
fi

if gh release view "$TAG" >/dev/null 2>&1; then
  echo "GitHub release already exists: $TAG" >&2
  exit 1
fi

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  cat >&2 <<'EOF'
TAURI_SIGNING_PRIVATE_KEY is not set.

This release flow calls `scripts/package-release.sh`, which builds Tauri updater artifacts and requires the updater signing private key.

Set these environment variables before running release:
  export TAURI_SIGNING_PRIVATE_KEY="$(cat /path/to/tauri.key)"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your-password"  # only if needed

If you are releasing from CI, inject the same values from repository secrets.
EOF
  exit 1
fi

echo "[1/6] Updating version numbers to $VERSION..."
perl -0pi -e 's/"version":\s*"[^"]+"/"version": "'"$VERSION"'"/' "$PACKAGE_JSON"
perl -0pi -e 's/^version = "[^"]+"/version = "'"$VERSION"'"/m' "$CARGO_TOML"
perl -0pi -e 's/"version":\s*"[^"]+"/"version": "'"$VERSION"'"/' "$TAURI_CONFIG"

echo "[2/6] Building release artifacts..."
"$ROOT_DIR/scripts/package-release.sh"

if [[ ! -f "$ZIP_PATH" || ! -f "$DMG_PATH" || ! -f "$UPDATER_PATH" || ! -f "$UPDATER_SIG_PATH" || ! -f "$LATEST_JSON_PATH" ]]; then
  echo "Expected release artifacts were not generated." >&2
  exit 1
fi

echo "[3/6] Committing release version..."
git add "$PACKAGE_JSON" "$CARGO_TOML" "$TAURI_CONFIG"
git commit -m "chore: release $VERSION"

echo "[4/6] Creating git tag..."
git tag "$TAG"

echo "[5/6] Pushing commit and tag..."
git push origin main
git push origin "$TAG"

echo "[6/6] Creating GitHub release..."
gh release create "$TAG" \
  "$ZIP_PATH" \
  "$DMG_PATH" \
  "$UPDATER_PATH" \
  "$UPDATER_SIG_PATH" \
  "$LATEST_JSON_PATH" \
  --title "Battery Deck $TAG" \
  --notes "Release $TAG for Apple Silicon Macs.

Assets:
- $(basename "$ZIP_PATH")
- $(basename "$DMG_PATH")
- $(basename "$UPDATER_PATH")
- $(basename "$UPDATER_SIG_PATH")
- $(basename "$LATEST_JSON_PATH")

Notes:
- Not notarized yet
- Requires a privileged helper for hardware-level battery control"

echo "Release completed: $TAG"
