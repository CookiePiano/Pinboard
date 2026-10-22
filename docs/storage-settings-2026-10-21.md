# Storage settings handoff — 2026-10-21 KST

This record describes the earlier storage UI. The current release uses **This PC / External cloud** and documents Google Drive for Desktop only. See [the transfer update](transfer-2026-10-21.md) for the current behavior and verification.

OAuth client ID, secret, account connection, and OAuth setup instructions have been removed from the settings UI and both translation catalogs. Legacy `sync: "gdrive"` settings deserialize as `"off"`; note files, folder paths, other preferences, and existing credentials are retained. The old backend code and credential storage remain for compatibility, without a selectable OAuth mode in this UI. Internal persisted values remain `"off"` and `"folder"` to preserve existing folder settings.

## Scoped source changes

- `src/lib/settings/SyncPanel.svelte`: two storage choices, progressive folder setup, cancellation and error handling, pending-action controls.
- `src/lib/i18n.ts`: Korean and English storage copy, including the sidebar's local-storage status.
- `src-tauri/src/settings.rs`: legacy OAuth-mode migration and migration/default tests.
- `README.md` and `docs/google-drive-setup.md`: updated folder connection instructions.

Changes to shortcuts, fonts, note appearance, startup and shared settings are included in the [current release](verification-2026-10-21.md). Intermediate build outputs and backups have been removed.

## Verification

- Initial isolated snapshot: frontend type checks passed with zero errors/warnings; production frontend build and native debug build succeeded. Vite retained the existing warning about a chunk larger than 500 kB.
- Final shared release: frontend type checks passed with zero errors/warnings and all **39 Rust tests passed**, including local defaults, legacy OAuth migration and folder synchronization.
- The native test executable used a dedicated data directory and WebView2 profile, and was stopped after the initial screen check. Final UI interaction checks used the actual production frontend in a task-owned headless browser with mocked IPC and folder-picker responses, avoiding desktop focus interference.
- Browser checks passed for local defaults, cloud setup before connecting a folder, picker cancellation, folder selection, manual sync, switching back to local while retaining the selected folder, reconnecting without another picker, save failure preserving the prior selection, picker failure preserving the connection, Korean/English copy, and restoring both modes after reload.
- No OAuth inputs or OAuth IPC calls were present in the exercised UI flow. The 640 × 460 English view had no horizontal overflow; content remains vertically scrollable.
- Changed source/docs files were verified as UTF-8 without BOM and LF. Scoped whitespace checks passed.

Real cloud-account synchronization, a native OS folder-picker interaction, and a new installer deployment were not performed in this pass.

## Release evidence

Intermediate UI captures and earlier release receipts have been removed. The latest delivered executable, installer, removal wizard and third-party notices are documented in the [current release record](verification-2026-10-21.md), with final hashes and retained build evidence.
