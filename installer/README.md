# Pinboard Windows setup and removal

Setup, the installed `uninstall.exe`, and `Pinboard_Uninstall.exe` share a native dark interface. The app payload and note formats are independent of this presentation layer.

## Presentation

The 900 x 560 logical client area has a 220 px brand/step rail, a scrolling content area, and a fixed footer. The existing dog-and-notepad icon is retained. Colors are `#15191E` (body), `#101419` (rail), `#20272E` (fields), `#72B5E0` (accent), `#F4F7FA` (text), and `#A5B1BC` (secondary text). Korean uses Malgun Gothic and English uses Segoe UI, with 11 pt body text and 24 pt headings.

- Setup combines destination, optional desktop shortcut, and optional launch on one page. Both options start unchecked. Existing installations expose maintenance choices inline.
- Progress uses the real NSIS progress control and expandable detail log. Closing is disabled during the operation. Failure restores a close action and reveals details.
- The standalone remover displays the registered version/path and always preserves notes. A missing installation disables removal.
- The installed uninstaller offers an unchecked local-data deletion option. Completion text reflects that choice.
- Controls retain native keyboard focus and accessibility roles. Layout uses window DPI, clamps to monitor work area, and scrolls when necessary. High contrast uses system colors and native control painting.
- Korean and English follow installer language. `/LANG=1042` and `/LANG=1033` allow explicit review without an extra language popup. Silent/passive/update modes remain supported.

## Source map

| Source | Responsibility |
| --- | --- |
| `native/PinboardUI.cpp` | Presentation-only x86 Unicode NSIS DLL: layout, fonts, colors, progress, DPI and native control painting |
| `native/PinboardVerify.cpp` | Authenticode chain and Microsoft publisher validation; retains a read-only handle until WebView2 setup finishes |
| `branding.nsh` | Shared initialization and page/control macros |
| `setup-pages.nsh` | Setup, maintenance and installed-remover pages |
| `path-validation.nsh` | Read-only path/volume validation, including nonexistent destinations |
| `languages/ui-strings.nsh` | Korean and English copy |
| `template.nsi` | Tauri custom template with original backend and custom page flow |
| `hooks.nsh` | Start menu removal shortcut lifecycle |
| `uninstall-launcher.nsi` | Registration validation, running-process guard and standalone removal |
| `../scripts/build-installer-ui.ps1` | MSVC x86 `/MT` plugin build; no separately installed C++ runtime |
| `../scripts/prepare-installer-ui.mjs` | Windows-only `beforeBundleCommand` integration |
| `../scripts/build-installers.ps1` | Repackage the existing matching-version app into isolated output |
| `../scripts/build-uninstaller.ps1` | Standalone remover build |
| `../scripts/test-installer.ps1` | Isolated installation/removal and data-preservation checks |
| `../scripts/test-installer-paths.ps1` | Production path-validator regression checks |

`template.nsi` derives from Tauri CLI 2.12.3, upstream commit `59b63ab500da9859f12d8888c3298bc29bba0b92`. Unmodified source and the MIT license are retained in `upstream/`. Review that diff when updating Tauri. The custom page flow targets the configured `currentUser` scope with no license page; other scopes or a license page require explicit integration.

The supported `bundle.windows.nsis.template` configuration selects the template. Cached MUI/nsDialogs macros are not replaced. Do not edit generated NSIS files or cached includes. Old `assets/` bitmaps and `installer-art.py` remain historical source assets and are no longer used by this UI.

## Build and verification

Use lockfile-compatible Node/Tauri tools, PowerShell 7, MSVC x86/x64 build tools and Windows SDK. `scripts/prepare-tools.ps1` acquires the SHA-256-pinned NSIS toolchain before plugin compilation. The release workflow uses an isolated Cargo target and its `.tauri/NSIS` cache; `PINBOARD_NSIS_DIR` selects the verified compiler and SDK.

`scripts/release.ps1 -OutputDirectory <empty-output-directory> -TargetDirectory <isolated-target-directory> -SigningPolicy UnsignedInitial` is the full 0.1.0 app release workflow. It publishes output only after all checks succeed. See [release policy](../docs/releasing.md). `scripts/build-installers.ps1 -WorkDirectory <isolated-directory>` packages the parent `Pinboard.exe` without rebuilding it, verifies its version against the configuration, passes `--no-binary-patching`, and verifies its SHA-256 is unchanged. Outputs go under `<isolated-directory>/artifacts`; delivery to the parent directory is separate.

Tauri builds invoke `beforeBundleCommand` automatically. `build-uninstaller.ps1` builds a fresh DLL by default, or accepts an explicitly prepared `-UiDirectory`. `PINBOARD_UI_PLUGIN_DIR` selects an isolated plugin directory. Build scripts restore process-scoped environment variables they change.

Run the repository checks in `../AGENTS.md`, then both installer test scripts. Tests use unique `PinboardInstallerQA_*` identities, an installation path containing spaces, and isolated data sentinels. Fixtures exclude system WebView2 acquisition and retain `results.json`. Inspect actual native screens; a build or mockup is not visual evidence. Dated reports distinguish forced-DPI layout previews from real cross-monitor DPI transitions.

The compatibility fixture records the executable payload SHA-256 from the rendered
NSIS template and compares installed bytes after clean installation, same-version
reinstall, upgrade and silent update, and verifies that unattended downgrades are rejected without modifying the installed payload. This verifies packaging fidelity;
application startup and editor behavior still require separate runtime checks.

WebView2 acquisition uses the private NSIS plugin directory. Before execution,
`PinboardVerify` requires a valid Authenticode chain with revocation checking and
the trusted leaf organization `Microsoft Corporation`. It holds the verified file
against writes and deletion until setup returns. `scripts/test-bootstrapper.ps1`
checks authentic, altered, unsigned, other-publisher and missing files, plus the
write lock, without installing the downloaded runtime.

## Removal contract

The standalone wizard resolves only the current user's `HKCU/Software/Microsoft/Windows/CurrentVersion/Uninstall/Pinboard` registration, checking both registry views. It requires the expected display name, a non-root canonical install folder, `pinboard.exe`, `uninstall.exe`, and a matching command without extra arguments. It never infers the target from its own directory or removes a portable executable.

Restart Manager checks the exact installed binary. A running app or unavailable running-state check blocks removal. The registered uninstaller runs with `/S` and `_?=` so completion can be awaited. Notes/settings are preserved. Only the known uninstaller and an empty install directory are cleaned afterward; the launcher does not recursively remove files. The installed uninstaller retains its explicit local-data deletion option. Sync-folder and cloud contents are outside removal scope.

| Standalone silent exit | Meaning |
| --- | --- |
| `0` | Removal completed |
| `1` | Removal failed or could not be verified |
| `2` | No valid managed installation |
| `3` | App running or running-state check unavailable |

Do not use `/S` against the production registration merely to inspect presentation. Test only with the uniquely named fixtures. Do not copy an installation-bound `uninstall.exe` into the portable directory.
