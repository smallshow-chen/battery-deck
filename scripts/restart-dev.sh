#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TAURI_DIR="$ROOT_DIR/src-tauri"
SERVICE_LABEL="com.smallshow.battery-toolkit-helper"
SYSTEM_HELPER_PATH="/Library/Application Support/BatteryToolkit/bin/battery-helper"
SYSTEM_PLIST_PATH="/Library/LaunchDaemons/${SERVICE_LABEL}.plist"

reinstall_root_helper=0

for arg in "$@"; do
  case "$arg" in
    --reinstall-root-helper)
      reinstall_root_helper=1
      ;;
    --help|-h)
      cat <<'EOF'
Usage:
  ./scripts/restart-dev.sh
  ./scripts/restart-dev.sh --reinstall-root-helper

Options:
  --reinstall-root-helper   Copy the freshly built battery-helper into the
                            system LaunchDaemon location and restart it.
EOF
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 1
      ;;
  esac
done

cd "$TAURI_DIR"

echo "[1/4] Stopping old dev processes..."
pkill -f 'target/debug/battery-deck' >/dev/null 2>&1 || true
pkill -f 'cargo tauri dev -- --bin battery-deck' >/dev/null 2>&1 || true

echo "[2/4] Building battery-helper..."
cargo build --bin battery-helper

if [[ "$reinstall_root_helper" -eq 1 ]]; then
  echo "[3/4] Reinstalling root helper..."
  echo "Source helper:"
  stat -f '%Sm %N' -t '%Y-%m-%d %H:%M:%S' "target/debug/battery-helper"
  shasum -a 256 "target/debug/battery-helper"

  sudo install -d -m 755 "/Library/Application Support/BatteryToolkit/bin"
  if [[ -f "$SYSTEM_PLIST_PATH" ]]; then
    sudo launchctl bootout system/"$SERVICE_LABEL" >/dev/null 2>&1 || true
    sudo cp -f "target/debug/battery-helper" "$SYSTEM_HELPER_PATH"
    sudo chown root:wheel "$SYSTEM_HELPER_PATH"
    sudo chmod 755 "$SYSTEM_HELPER_PATH"
    sudo rm -f /tmp/battery-toolkit/com.smallshow.battery-toolkit-helper.sock
    sudo rm -f /tmp/battery-toolkit/com.smallshow.battery-toolkit-helper.pid
    sudo rm -f /tmp/battery-toolkit/com.smallshow.battery-toolkit-helper.log
    sudo rm -f /tmp/battery-toolkit/com.smallshow.battery-toolkit-helper.stdout.log
    sudo rm -f /tmp/battery-toolkit/com.smallshow.battery-toolkit-helper.stderr.log
    sudo launchctl bootstrap system "$SYSTEM_PLIST_PATH"
    sudo launchctl kickstart -k system/"$SERVICE_LABEL" >/dev/null 2>&1 || true

    echo "Installed helper:"
    sudo stat -f '%Sm %N' -t '%Y-%m-%d %H:%M:%S' "$SYSTEM_HELPER_PATH"
    sudo shasum -a 256 "$SYSTEM_HELPER_PATH"
  else
    echo "Root plist not found at $SYSTEM_PLIST_PATH" >&2
    echo "Install the root service once from the app UI first." >&2
    exit 1
  fi
fi

echo "[4/4] Starting Tauri dev app..."
exec cargo tauri dev -- --bin battery-deck
