#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TAURI_DIR="$ROOT_DIR/src-tauri"
CONFIG_FILE="$TAURI_DIR/tauri.conf.json"
RELEASE_DIR="$ROOT_DIR/release-artifacts"
STAGING_DIR=""
ORIGINAL_CONFIG_FILE=""
RESTORE_CONFIG_FILE=0

cleanup() {
  if [[ -n "$STAGING_DIR" && -d "$STAGING_DIR" ]]; then
    rm -rf "$STAGING_DIR"
  fi
  if [[ "$RESTORE_CONFIG_FILE" -eq 1 ]]; then
    printf '%s' "$ORIGINAL_CONFIG_FILE" > "$CONFIG_FILE"
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
REPO_OWNER="${REPO_OWNER:-smallshow-chen}"
REPO_NAME="${REPO_NAME:-battery-deck}"

APP_NAME="${PRODUCT_NAME}.app"
APP_PATH="$TAURI_DIR/target/release/bundle/macos/$APP_NAME"
APP_RELEASE_PATH="$RELEASE_DIR/$APP_NAME"
HELPER_PATH="$TAURI_DIR/target/release/battery-helper"
APP_HELPER_PATH="$APP_PATH/Contents/MacOS/battery-helper"
ZIP_PATH="$RELEASE_DIR/${PRODUCT_NAME}_${VERSION}_${ARCH}.zip"
DMG_PATH="$RELEASE_DIR/${PRODUCT_NAME}_${VERSION}_${ARCH}.dmg"
UPDATER_BUNDLE_PATH="$TAURI_DIR/target/release/bundle/macos/${PRODUCT_NAME}.app.tar.gz"
UPDATER_SIG_PATH="$TAURI_DIR/target/release/bundle/macos/${PRODUCT_NAME}.app.tar.gz.sig"
UPDATER_RELEASE_PATH="$RELEASE_DIR/${PRODUCT_NAME}_${VERSION}_${ARCH}.app.tar.gz"
UPDATER_SIG_RELEASE_PATH="$RELEASE_DIR/${PRODUCT_NAME}_${VERSION}_${ARCH}.app.tar.gz.sig"
LATEST_JSON_PATH="$RELEASE_DIR/latest.json"
UPDATER_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/v${VERSION}/$(basename "$UPDATER_RELEASE_PATH")"
UPDATER_SIGNATURE=""
BUILD_UPDATER_ARTIFACTS=0

if [[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" || -n "${TAURI_SIGNING_PRIVATE_KEY_PATH:-}" ]]; then
  BUILD_UPDATER_ARTIFACTS=1
  if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -n "${TAURI_SIGNING_PRIVATE_KEY_PATH:-}" ]]; then
    TAURI_SIGNING_PRIVATE_KEY="$(cat "$TAURI_SIGNING_PRIVATE_KEY_PATH")"
    export TAURI_SIGNING_PRIVATE_KEY
  fi
fi

mkdir -p "$RELEASE_DIR"
rm -rf "$APP_RELEASE_PATH" "$ZIP_PATH" "$DMG_PATH" "$UPDATER_RELEASE_PATH" "$UPDATER_SIG_RELEASE_PATH" "$LATEST_JSON_PATH"

echo "[1/6] Building helper binary..."
cd "$TAURI_DIR"
cargo build --release --bin battery-helper

if [[ ! -x "$HELPER_PATH" ]]; then
  echo "Expected helper executable not found: $HELPER_PATH" >&2
  exit 1
fi

echo "[2/6] Building macOS app bundle..."
cd "$ROOT_DIR"
if [[ "$BUILD_UPDATER_ARTIFACTS" -eq 1 ]]; then
  ./node_modules/.bin/tauri build --bundles app,updater -- --bin battery-deck
else
  echo "TAURI_SIGNING_PRIVATE_KEY is not set; building local app artifacts only."
  ORIGINAL_CONFIG_FILE="$(cat "$CONFIG_FILE")"
  RESTORE_CONFIG_FILE=1
  perl -0pi -e 's/"createUpdaterArtifacts":\s*true/"createUpdaterArtifacts": false/' "$CONFIG_FILE"
  ./node_modules/.bin/tauri build --bundles app -- --bin battery-deck
fi

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

echo "[5/8] Creating zip archive..."
ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "$ZIP_PATH"

echo "[6/8] Creating dmg archive..."
STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/battery-deck-release.XXXXXX")"
ditto "$APP_PATH" "$STAGING_DIR/$APP_NAME"
ln -s /Applications "$STAGING_DIR/Applications"
hdiutil create \
  -volname "$PRODUCT_NAME" \
  -srcfolder "$STAGING_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH" >/dev/null

if [[ "$BUILD_UPDATER_ARTIFACTS" -eq 1 ]]; then
  echo "[7/8] Copying updater artifacts..."
  if [[ ! -f "$UPDATER_BUNDLE_PATH" || ! -f "$UPDATER_SIG_PATH" ]]; then
    echo "Expected updater artifacts not found. Make sure the updater pubkey is configured." >&2
    exit 1
  fi
  cp "$UPDATER_BUNDLE_PATH" "$UPDATER_RELEASE_PATH"
  cp "$UPDATER_SIG_PATH" "$UPDATER_SIG_RELEASE_PATH"
  UPDATER_SIGNATURE="$(tr -d '\n' < "$UPDATER_SIG_RELEASE_PATH")"

  echo "[8/8] Writing latest.json..."
  cat > "$LATEST_JSON_PATH" <<EOF
{
  "version": "${VERSION}",
  "notes": "Battery Deck ${VERSION}",
  "pub_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platforms": {
    "darwin-aarch64": {
      "signature": "${UPDATER_SIGNATURE}",
      "url": "${UPDATER_URL}"
    }
  }
}
EOF
else
  echo "Skipped updater artifacts and latest.json because TAURI_SIGNING_PRIVATE_KEY is not set."
fi

echo "Release artifacts:"
echo "  App: $APP_RELEASE_PATH"
echo "  Zip: $ZIP_PATH"
echo "  Dmg: $DMG_PATH"
if [[ "$BUILD_UPDATER_ARTIFACTS" -eq 1 ]]; then
  echo "  Updater: $UPDATER_RELEASE_PATH"
  echo "  Signature: $UPDATER_SIG_RELEASE_PATH"
  echo "  Manifest: $LATEST_JSON_PATH"
fi
