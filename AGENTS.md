# Repository Guidelines

## Project Structure & Module Organization

This is a Tauri v2 macOS battery management app. Frontend code lives in `src/`: `index.html` defines the dashboard, `main.js` handles Tauri invokes and DOM updates, `i18n.js` contains English/Chinese strings, and `styles.css` owns styling. Rust backend code lives in `src-tauri/src/`: `lib.rs` registers commands, tray behavior, and polling; `battery.rs` gathers battery/system data; `service.rs` manages the privileged helper; `helper.rs` and `bin/battery-helper.rs` implement the root daemon; `smc.rs` contains Apple SMC FFI. Icons and Tauri configuration are under `src-tauri/icons/`, `src-tauri/capabilities/`, and `src-tauri/tauri.conf.json`. Scripts are in `scripts/`.

## Build, Test, and Development Commands

- `npm install`: install dependencies from `package-lock.json`.
- `npm run tauri dev`: run the app through the Tauri CLI for local development.
- `./scripts/restart-dev.sh`: restart development; use `--reinstall-root-helper` after helper installation changes.
- `npm run build:release` or `./scripts/package-release.sh`: build release artifacts into `release-artifacts/`.
- `cd src-tauri && cargo test`: run Rust tests when present and compile test targets.
- `cd src-tauri && cargo fmt`: format Rust code before committing.

## Coding Style & Naming Conventions

Use Rust 2021 conventions: four-space indentation, `snake_case` for functions/modules, `PascalCase` for types, and explicit `Result` handling where failures can occur. Keep privileged-service and SMC changes narrow and easy to audit. Frontend code is plain ES modules; prefer small DOM helpers, `camelCase` JavaScript names, and existing CSS custom properties. Keep user-facing strings in `src/i18n.js`.

## Testing Guidelines

There is no dedicated frontend test framework. For Rust changes, add focused unit tests near the code they cover when parsing, state transitions, or helper protocol behavior changes. Always run `cargo test` after backend edits. For UI or Tauri command changes, manually verify with `npm run tauri dev` or `./scripts/restart-dev.sh`, including service status, tray actions, and battery controls where applicable.

## Commit & Pull Request Guidelines

Recent history uses concise subjects and occasional Conventional Commit prefixes, such as `feat:`, `style:`, `docs:`, and `chore:`. Follow that style: `feat: add charger detail parsing` or `fix: handle missing helper socket`. Pull requests should describe behavior changes, list verification commands, mention macOS/helper privilege impacts, and include screenshots for visible UI changes. Link related issues when available.

## Security & Configuration Tips

Treat helper installation, LaunchDaemon behavior, Unix socket communication, and SMC writes as security-sensitive. Do not commit local signing material, generated release bundles, or machine-specific service state. Keep `src-tauri/capabilities/default.json` permissions minimal and update it only when a Tauri API need is clear.
