# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

**Prerequisites:** macOS (Apple Silicon), Rust stable, Node.js v18+

```bash
npm install
./scripts/restart-dev.sh                          # Build helper + run tauri dev
./scripts/restart-dev.sh --reinstall-root-helper  # Also reinstall root helper (needs sudo)
./scripts/package-release.sh                      # Full release build (.app, .dmg, .zip in release-artifacts/)
```

The dev script kills old processes, builds `battery-helper` with `cargo build --bin battery-helper`, then runs `cargo tauri dev -- --bin battery-toolkit`.

```bash
cd src-tauri && cargo test    # Run Rust tests (add focused unit tests for parsing/state/helper protocol changes)
cd src-tauri && cargo fmt     # Format Rust code
```

There is no dedicated frontend test framework.

## Architecture

Three-tier macOS battery management app (Apple Silicon only):

```
Frontend (vanilla HTML/CSS/JS)  ←Tauri IPC→  Tauri Backend (Rust)  ←Unix Socket→  Root Helper (Rust)
       src/                                  src-tauri/src/lib.rs                 src-tauri/src/helper.rs
```

The two-process model exists because SMC hardware access requires root, while the GUI runs as the normal user. The helper daemon communicates over a Unix socket at `/tmp/battery-toolkit/com.smallshow.battery-toolkit-helper.sock`.

### Rust modules (`src-tauri/src/`)

| File | Role |
|------|------|
| `lib.rs` | Tauri app bootstrap: 20 IPC commands, system tray with bilingual menu, window management, background battery polling via tokio. All `#[tauri::command]` functions are registered here. |
| `battery.rs` | Data models (`BatteryState`, `DashboardSnapshot`, `BatteryHealth`, `BatteryRealtime`, `ChargerInfo`, `SystemInfo`) + `BatteryCache` (time-based `Mutex` cache, 2s TTL for IOREG/pmset, 60s for charger profile, indefinite for system info). Parses `ioreg`, `pmset`, `system_profiler` shell output. |
| `smc.rs` | Raw IOKit FFI (`extern "C"`) for Apple SMC. `SmcParamStructRaw` is an 80-byte `#[repr(C)]` struct. Auto-probes hardware: tries `CHTE`→`CH0C` for charging, `CHIE`→`CH0J` for adapter, `ACLC` for MagSafe LED. Writes always verify by read-back. |
| `service.rs` | Helper lifecycle: install (via `osascript` with admin privileges), start/stop (via `launchctl bootout`), IPC (JSON over Unix socket with `Request{id, command, payload}` / `Response{id, ok, data, error}`). Falls back from root LaunchDaemon → user LaunchAgent → direct spawn. |
| `helper.rs` | The privileged daemon process. Owns the SMC handle, runs a polling loop (5–15s interval) evaluating charge mode logic, persists `state.json` and `settings.json`. Service label: `com.smallshow.battery-toolkit-helper`. |
| `bin/battery-helper.rs` | Thin entry point calling `helper::run_daemon()`. Built as a separate binary. |

**Dependency direction:** `lib.rs` → all modules. `service.rs` → `battery` (types) + `helper` (paths). `helper.rs` → `battery` + `smc`. `smc.rs` and `battery.rs` are leaf modules.

### Frontend (`src/`)

No framework, no build step — raw HTML/JS/CSS served directly by Tauri (`frontendDist: "../src"` in `tauri.conf.json`, `withGlobalTauri: true`).

- **main.js**: All UI logic. DOM references cached once in a `dom` object. State is plain JS objects merged via `Object.assign`. Polling: full snapshot every 30s, realtime every 15s, paused when window hidden or user scrolling. `requestAnimationFrame` batches DOM updates.
- **i18n.js**: Custom i18n using `data-i18n` attributes. Supports English + Chinese. Exposed as `window.__i18n`. Auto-detects locale from Tauri or `navigator.language`.
- **styles.css**: ~1780 lines. Two-pass CSS cascade: base styles (lines 1–947) then a "Liquid Glass Cascade" override (lines 948–1783) implementing Apple visionOS-inspired translucent glass design. Dark mode via `@media (prefers-color-scheme: dark)`. Responsive breakpoints at 920px and 760px.

### IPC contract

**Frontend → Backend (`invoke`):** `get_dashboard_snapshot`, `get_battery_realtime`, `get_service_logs`, `set_settings`, `charge_to_full`, `charge_to_limit`, `reset_charge_mode`, `disable_charging_cmd`, `enable_adapter_cmd`, `disable_adapter_cmd`, `install_service`, `start_service`, `stop_service`

**Backend → Frontend (events):** `battery-state-changed`, `app-window-visibility-changed`, `app-state-refresh-requested`, `tray-action-error`

## Coding Style

**Rust:** Four-space indentation, `snake_case` functions/modules, `PascalCase` types. All public functions return `Result<T, String>`. Keep SMC and privilege-related changes narrow and auditable.

**Frontend:** Plain ES modules, `camelCase` JS names, existing CSS custom properties. User-facing strings go in `src/i18n.js`.

**Commits:** Concise subjects with Conventional Commit prefixes (`feat:`, `fix:`, `style:`, `docs:`, `chore:`). Example: `feat: add charger detail parsing`.

## macOS-specific details

- **SMC keys:** `CHTE`/`CH0C` (charge enable/disable), `CHIE`/`CH0J` (adapter enable/disable), `ACLC` (MagSafe LED). The probing order matters for cross-hardware compatibility.
- **IOKit link:** `build.rs` links `IOKit` and `CoreFoundation` frameworks.
- **Config paths:** Settings `~/.config/battery-toolkit/settings.json` (user) or `/Library/Application Support/BatteryToolkit/settings.json` (root). State: `~/.local/share/battery-toolkit/state.json` or root equivalent.
- **Helper binary location:** Installed to `/Library/Application Support/BatteryToolkit/bin/battery-helper`.
- **Error handling pattern:** All public functions return `Result<T, String>`. Shell command failures yield `None`/`Err`, never panic. The helper tracks `last_error` in state and logs.
- Non-macOS targets compile via `#[cfg(not(target_os = "macos"))]` stubs returning defaults.
