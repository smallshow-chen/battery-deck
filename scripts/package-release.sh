#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TAURI_DIR="$ROOT_DIR/src-tauri"
CONFIG_FILE="$TAURI_DIR/tauri.conf.json"
RELEASE_DIR="$ROOT_DIR/release-artifacts"
STAGING_DIR=""

cleanup() {
  if [[ -n "$STAGING_DIR" && -d "$STAGING_DIR" ]]; then
    rm -rf "$STAGING_DIR"
  fi
}

trap cleanup EXIT

if ! command -v plutil >/dev/null 2>&1; then
  echo "plutil is required on macOS." >&2
  exit 1
fi

if ! command -v hdiutil >/dev/null 2>&1; then
  echo "hdiutil is required on macOS." >&2
  exit 1
fi

PRODUCT_NAME="$(plutil -extract productName raw -o - "$CONFIG_FILE")"
VERSION="$(plutil -extract version raw -o - "$CONFIG_FILE")"
MAIN_BINARY_NAME="$(plutil -extract mainBinaryName raw -o - "$CONFIG_FILE")"
ARCH="$(uname -m)"

APP_NAME="${PRODUCT_NAME}.app"
APP_PATH="$TAURI_DIR/target/release/bundle/macos/$APP_NAME"
APP_RELEASE_PATH="$RELEASE_DIR/$APP_NAME"
HELPER_PATH="$TAURI_DIR/target/release/battery-helper"
APP_HELPER_PATH="$APP_PATH/Contents/MacOS/battery-helper"
ZIP_PATH="$RELEASE_DIR/${PRODUCT_NAME}_${VERSION}_${ARCH}.zip"
DMG_PATH="$RELEASE_DIR/${PRODUCT_NAME}_${VERSION}_${ARCH}.dmg"

mkdir -p "$RELEASE_DIR"
rm -rf "$APP_RELEASE_PATH" "$ZIP_PATH" "$DMG_PATH"

echo "[1/6] Building helper binary..."
cd "$TAURI_DIR"
cargo build --release --bin battery-helper

if [[ ! -x "$HELPER_PATH" ]]; then
  echo "Expected helper executable not found: $HELPER_PATH" >&2
  exit 1
fi

echo "[2/6] Building macOS app bundle..."
cd "$ROOT_DIR"
./node_modules/.bin/tauri build --bundles app -- --bin battery-deck

if [[ ! -d "$APP_PATH" ]]; then
  echo "Expected app bundle not found: $APP_PATH" >&2
  exit 1
fi

if [[ ! -x "$APP_PATH/Contents/MacOS/$MAIN_BINARY_NAME" ]]; then
  echo "Expected main executable not found: $APP_PATH/Contents/MacOS/$MAIN_BINARY_NAME" >&2
  exit 1
fi

echo "[3/6] Embedding helper binary..."
cp "$HELPER_PATH" "$APP_HELPER_PATH"
chmod 755 "$APP_HELPER_PATH"

echo "[4/6] Copying app bundle..."
ditto "$APP_PATH" "$APP_RELEASE_PATH"

echo "[5/6] Creating zip archive..."
ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "$ZIP_PATH"

echo "[6/6] Creating dmg archive..."
STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/battery-deck-release.XXXXXX")"
ditto "$APP_PATH" "$STAGING_DIR/$APP_NAME"
ln -s /Applications "$STAGING_DIR/Applications"
hdiutil create \
  -volname "$PRODUCT_NAME" \
  -srcfolder "$STAGING_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH" >/dev/null

echo "Release artifacts:"
echo "  App: $APP_RELEASE_PATH"
echo "  Zip: $ZIP_PATH"
echo "  Dmg: $DMG_PATH"
