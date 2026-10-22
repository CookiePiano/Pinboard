# Full application audit — 2026-10-22 KST

Status: **In progress; Windows shell interaction evidence pending**.
This is the current work ledger, not a completion claim.

Retained screenshots and machine-readable results: [evidence index](verification/full-audit-2026-10-22/README.md).

## Objective and scope

Test every supported application feature, analyze observed behavior, fix defects, optimize and refactor where evidence supports a change, and maintain agent-oriented English documentation. Windows native behavior, persistence, synchronization, import/export, AI integration, localization and platform-specific limitations are separate verification surfaces.

The current worktree is authoritative. Historical successful releases do not prove current behavior. Production note data, client credentials, unrelated configuration and running applications must remain intact. All mutations in tests use task-owned fixtures.

## Baseline

- Source snapshot: `C:\build\pinboard-full-audit-20261022-0025\before`, 270 files copied and hash-verified; manifest `source-baseline.json`.
- Runtime: Node `v24.13.1`, Cargo `1.99.1`, Rust `1.99.1`, native Windows/PowerShell 7.
- Current baseline: Svelte check has zero errors/warnings; links, formatting, calendar dates, six-language catalogs and the 39-command IPC permission contract pass. Rust library baseline: 91 tests pass.
- Documentation scan: all 46 discovered Markdown/text documents under the project were inventoried. The only Hangul in maintained Markdown is a literal icon-source path, a Drive path and a Korean UI acceptance string. These are inside English documents, not untranslated prose.

## Feature coverage ledger

`Core passed` means a current automated test exercises the named logic; it does not certify the full native interaction. `Pending` requires further evidence before completion.

| Feature | Core evidence available | Native/end-to-end status |
| --- | --- | --- |
| Create, edit, save and restore notes | Store/model tests; save queue regressions | Create/edit and restart restoration passed; remaining failure interactions pending |
| Delete, hide, show all, tray and single instance | Tombstone/MCP/store tests | Calendar deletion/retry and MCP hide/show passed; Windows single-instance startup and settings burst passed after WIN-001/002; Linux native tray menu and save-failure transitions passed after SAVE-004 |
| Source/latest-note appearance inheritance | Store tests | Native source/latest creation retained opacity, font and image; body, formats and pin excluded; Linux native 420x360 size inheritance and 28 px source cascade passed |
| Bold, italic, size, highlight, undo/redo, UTF-16 ranges | Formatting script and model validation | Toolbar and the Windows Ctrl+Z/Ctrl+Y bindings passed with Korean/emoji |
| HTTP/HTTPS/www and Markdown links | Link script | Windows HTTP opened the default browser; Linux native clicks delivered all four forms to an isolated external URI handler, while native drag selection and Shift-click did not open URLs |
| Tint, opacity and Windows blur threshold | Validation/model contracts; GLASS-001 retry/cache regressions | Six tints and opacity endpoints persisted; actual desktop captures verify clear background at 0.30 and acrylic at 0.31, including a recreated editor after restore |
| Background image selection, sizing and opacity | Model/ZIP tests | Upload, WebP conversion, opacity, ZIP round trip, 4000x2000 to 2560x1280 resizing, invalid-file preservation and removal passed |
| Global/per-note font, fallback and size reset | Font/model tests, real Windows font enumeration | Global/per-note controls, live font application, missing-font fallback, default reset and size reset passed |
| System/light/dark appearance | Settings serialization | Light/dark and restart persistence passed; emulated native-WebView color-scheme change events switch system mode, while explicit dark overrides light; OS settings unchanged |
| Six languages and untouched welcome-note translation | i18n/welcome tests | All six settings/welcome translations passed in a fresh fixture; edited welcome and user-authored content preserved |
| Note pin modes, drag, snap, resize and focus restoration | Snapping and shared drag-action tests | X11 resize/drag/pinning passed; Windows native overlap and focus restoration passed after WIN-003, including a separate topmost application; user-confirmed Windows drag/snap/resize passed with persisted geometry evidence; Show-desktop visual verification remains open |
| Calendar counts, bounded previews, date ranges and timezone handling | Calendar Rust/TypeScript tests | Empty/populated day, delete failure/retry, month navigation and selected hidden-note opening passed; X11 drag/click recovery passed; Windows native Top flag persists after blur |
| Settings validation and persistence | Settings tests | Five invalid native payloads rejected without modifying saved settings; font/language/theme/shortcut and restart checks passed |
| Autostart and global shortcut registration/conflicts | Shortcut/local-state/capture tests | Native recording, trigger, restart registration and two-process conflict preservation passed after KEY-001; isolated normal-identity autostart on/off and disabled restart persistence passed |
| Folder sync: create/edit/delete, conflicts and tombstone expiry | Sync engine/folder tests | Two live instances passed bidirectional edits, conflicts, edit-vs-delete, deletion and malformed-remote recovery; expiry remains core-tested |
| Legacy Drive compatibility and OAuth callback handling | OAuth/settings tests | No production credentials to be changed |
| ZIP export/import, backup, rollback, validation and image/format preservation | Transfer tests | Native file-picker round trip, automatic pre-import backup, malformed-ZIP rejection and abort on editor-save failure passed; forced file-lock rollback passed through native IPC, with unchanged hashes and resumed editing |
| Explicit MCP provisioning, disabled state and persistence | MCP tests | Explicit setup/enable/disable and both enabled/disabled states after restart passed |
| MCP stdio tools: list/read/create/update/delete/show/hide/pin | Protocol/bridge/operation tests | Live stdio round trips passed, including invalid argument/value rejection |
| Client registration, backup, idempotence and malformed config handling | Temporary-file client tests | Real client configurations remain untouched |
| Permissions, scoped URL opening and hostile payload rejection | IPC security script and Rust validation | Native settings/MCP/export denials and file/javascript opener denials passed. External navigation retained the bundled document; a probe request occurred, so this does not prove network isolation |
| Windows setup and both uninstallers | 34 isolated checks with the WIN-003 payload, including installed hashes and preservation of both test app processes; 6 path cases on unchanged installer sources | Isolated setup/removal passed; production artifacts have not been replaced |
| Linux build/startup/editor/platform behavior | 93 library tests plus optimized GLib regression and frontend suites | Latest optimized binary passed X11 tray/window/link/calendar checks and Wayland GUI/restart; Debian hashes, dependencies and extracted-binary startup passed |

## Findings and changes

### SAVE-001: A failed older save can replace a newer successful edit

The original `queueBody`/`flush` functions were extracted from the current Svelte file and executed with a controlled rejected first write. The observed write order was `older`, `newer`, `older`. The final retry resurrected stale content because failure handling only checked the debounced pending slot, not newer writes already queued.

`src/lib/note/save-queue.ts` now owns serialization, edit revisions, coalescing and retry eligibility. `NoteWindow.svelte` retains debounce timing and UI error display. A failed write is retried only if it is still the latest edit. Tests cover a newer queued write, a newer debounced edit, retry of the latest failure, Unicode formatting payloads and repeated flush calls. Source reproduction and output are retained as `reproduce-save-race.mjs` and `save-race-before.log` in the audit directory.

### SAVE-002: Failed window-state persistence loses its retry marker

`save_local` cleared `local_dirty` before writing and did not restore it on error. A temporary write failure could therefore leave placement/pin changes unsaved until another edit happened. Failure now restores the dirty marker. Regression tests block `local-state.json` with a directory, remove the obstruction, and verify the next save succeeds without another edit. A clean state must not rewrite its file.

The existing isolated `AppState` test fixture was moved out of the MCP test module into `src-tauri/src/test_support.rs` so state-level tests share one initializer. No production schema or synchronization hash changed.

### CAL-001: Calendar refresh silently clears deletion failures

Severity: medium. A real filesystem failure was injected only into the isolated test note by moving its JSON aside and creating a directory at the same filename. The native `delete_note` command rejected the operation with Windows error 5. Two attempts through the calendar's two-click confirmation left the note present but showed no alert. `loadDay` cleared the shared action error immediately, and its two-second polling would also clear open/pin errors.

Day loading now has a separate error state. Action failures survive automatic refresh, and failed note selections remain available for retry. The rebuilt application passed the same real filesystem failure, retained the alert and selected checkbox across polling, and successfully deleted the note after the obstruction was removed. See `calendar-regression-result.json` and `calendar-delete-error-after.png`.

Selection reconciliation uses a set instead of a nested scan. In a single local 10,000-selected / 10,000-note benchmark, output was identical and processing fell from 210.6 ms to 1.7 ms. This measures only reconciliation, not end-to-end calendar rendering (`calendar-selection-benchmark.json`).

Evidence in the audit directory: `calendar-delete-error-before.webm`, `calendar-delete-step-1.png`, `calendar-delete-step-2.png`, `calendar-delete-step-3.png`. Reproduction: open the test day's notes, select the blocked test note, click Delete twice, and observe the retained note with no visible error. Production notes were not involved.

## Current validation after changes

### SAVE-003: Hiding a note also hides its unsaved-edit failure

Severity: high. With the isolated note's JSON path blocked by a directory, editing and clicking Hide twice reproduced a failed save followed by a hidden native window. The editor still held the new text and retry error in memory, but the native visibility query returned false. The user could lose that pending text on a later application exit without seeing the error.

Hide now leaves a dirty save queue visible and surfaces hide-operation failures. The rebuilt native app retained the error and text with `visible: true` under the same failure. Once the obstruction was removed, clicking Hide persisted the pending text, cleared the error and hid the window. ZIP export during the failure also aborted with the save error and released its busy state. Evidence: `save-hide-before.webm`, `save-hide-step-1.png`, `save-hide-step-2.png`, `save-hide-before-result.json`, `save-hide-after-result.json`, `save-hide-retry-result.json`, `save-hide-after.png`, and `export-save-error.json`. The screenshots capture WebView content even after the host window hides; the native visibility result is the authoritative evidence for hiding.

### SAVE-004: Tray hide/quit and native close bypass editor-save protection

Severity: high. The real Linux tray menu was exercised on a private D-Bus session
with a small StatusNotifier host. With the test note's JSON path obstructed,
Hide All hid every note and Quit exited despite the visible save error. The
persisted body was empty while Korean text remained only in the editor. This is
an observed data-loss path, not an inference from the individual Hide behavior.

The existing export/import freeze-and-acknowledge protocol now lives in
`src-tauri/src/editors.rs`. Backup, Hide All and Quit use the same implementation
and operation lock. A failed flush releases editors, restores visible notes and
aborts hiding/quitting. An accepted exit keeps editing frozen until termination.
Native close requests now invoke the editor's guarded Hide path. The frontend
drains queued saves before acknowledging the freeze. IPC names, persisted schemas
and synchronization hashes are unchanged.

The rebuilt optimized Linux application passed actual tray New, Hide All, Show
All, Settings, Sync and Quit callbacks. Alt+F4 and both tray exit paths retained
the failed edit; removing the obstruction allowed retry, persistence and exit.
The same harness verified native resize, 420x360 source/latest inheritance and a
28 px source cascade. The test host proves application callbacks, not GNOME's
extension installation or the Windows notification area.

Windows native close requests also retained the Korean/emoji edit and visible
error. After removing the obstruction, close saved and hid the note. UI export
aborted during failure, then succeeded with hidden editors; native IPC import
restored the archive and created its pre-import backup. Windows library tests
passed 94 cases and Linux passed 93 after this refactor. Evidence is under
`windows/windows-native-close-*`, `windows/windows-editor-freeze-*` and
`linux/tray-save/` in the evidence directory.

The error label previously described a settings save even in a note. All six
catalogs now identify the note save correctly. The above screenshots predate
that text-only correction; fresh package/render verification is tracked separately.

### NAV-001: Linux startup rejects Tauri's bundled root URL

Severity: critical. Two native Linux WebDriver attempts remained at `about:blank` and never rendered an editor. The diagnostic harness now preserves each available page's URL/HTML and a screenshot when capture succeeds. The locked Tauri source simplifies its default `index.html` page to `tauri://localhost`; unlike HTTP URLs, this custom-scheme URL has an empty path. The application's navigation guard accepted only `/` and `/index.html`.

A focused regression failed against the original guard (`linux-root-red.log`). The guard now accepts an empty path only on its existing trusted origins, while retaining scheme, host, port and credential restrictions. All 94 Windows and 93 Linux Rust library tests then passed. The rebuilt Linux GUI also passed creation, Korean editing, font controls, six-language switching and restart restoration. The separate startup test passed second-instance settings/normal/autostart behavior. Its previous blank-page evidence remains under `/home/<user>/Pinboard-audit-20261022/verification/gui-diagnostic`; current success evidence is linked above.

### DRAG-001: GTK window dragging retains DOM pointer capture

Severity: medium. After a real X11 header drag, the window moved and saved its
position, but the header still held pointer ID 1. Later clicks at a link's
coordinates were delivered to that header, even outside its bounds, and no URL
reached the isolated external handler. Repeated native runs recorded the same behavior. A dedicated
regression asserts that capture is empty after the native move; it failed on the
unmodified release. The earlier movement-only pass was insufficient to prove
that the editor remained interactive afterward.

The `drag_note` command now reports whether pointer-driven movement is required.
Windows retains capture for its custom snapping; GTK releases capture after
handing movement to the window manager, which can consume `pointerup` itself.
Note and calendar headers now share `window-drag.ts`, removing the duplicate note
implementation. Start errors and action destruction release capture as well.
The focused action test covers Windows movement/coalescing, GTK ownership,
interactive controls, failure and cleanup. The rebuilt optimized Linux binary
released capture and passed real pointer clicks through HTTP, HTTPS, www and
Markdown URLs, native drag selection and Shift-click selection. An external
handler configured only in the fixture's XDG environment received the exact URLs
through the real native opener and xdg-open chain. No network access or user
browser association was required. A later native arrow-key event was used to
collapse selection; mixing WebDriver key input with native pointer selection had
produced a harness-only click timing failure.

The calendar passed the same drag/capture regression, month navigation after
drag, and top-pin persistence with the X11 above hint retained after blur.
The optimized binary also passed Wayland editing/restart. Windows passed the
94-test library gate, the action contract and release compilation. Windows
native pointer/snapping verification remains a separate pending item. The red
evidence is in `linux/drag-capture/`; current results are in `linux/drag-fixed/`.

### Navigation probe follow-up

The optimized Windows release was tested against a fresh loopback HTTP server.
Assigning an external URL sent one document GET and received HTTP 200, recorded
by both server logs and a HAR. The bundled URL, editor and Korean/emoji content
remained active. The external response's script marker was absent and its
`/executed` request never occurred. Wry's locked source calls `SetCancel` from
`NavigationStarting`; observed cancellation does not imply zero network traffic.
The security policy now states this distinction. This resolves the earlier probe
ambiguity without claiming a network-isolation guarantee. Evidence:
`navigation-release-result.json`, `navigation-probe-current.jsonl` and
`navigation-release.har` in the Windows audit directory.

### KEY-001: Empty physical key codes prematurely finish shortcut recording

Severity: medium. Windows accessibility input produced `keydown` events with `key` values `Control`, `Alt`, and `F8`, but an empty `code` for each. The recorder tested modifier names only against `code`, so the first Control event submitted the invalid shortcut `Ctrl`. The settings page showed a save error and retained the previous shortcut. This reproduced twice in the real WebView; it does not establish that ordinary hardware key events have the same empty code.

Shortcut interpretation now lives in `settings/shortcut-capture.ts`. It ignores modifiers identified by either field, retains physical-code behavior, and falls back only for unambiguous letters, digits, function and navigation keys. IME composition and ambiguous punctuation without a code remain invalid. `test-shortcut-capture.mjs` covers the observed Windows event sequence and the existing physical-key behavior, and is included in Windows/Linux validation. The rebuilt native app accepted the same input as Ctrl+Alt+F8 without an alert or extra note during recording. Pressing it afterward created exactly one note. Evidence: `shortcut-native-events.json`, `shortcut-empty-code-before.webm`, `shortcut-before-step-{1,2,3}.png`, `shortcut-fixed-record-result.json`, and `shortcut-fixed-trigger-result.json` in the audit directory.

### WIN-001: Native window creation can deadlock during an early second launch

The normal-identity Windows fixture froze at `about:blank` when a second
`--settings` launch arrived before the first WebView finished initialization.
The original process stopped responding, and a normal second launch exited
without creating its requested note. The locked Tauri WebView2 implementation
documents that synchronous window creation inside native event handlers can
deadlock. Evidence: `single-instance-hang-before.json`,
`single-instance-hang-before.log` and `single-instance-hang-page.json` in the audit
directory.

`dispatch_window_action` now runs NewNote, Settings and ShowAll on Tauri's blocking
worker pool. Tray, global shortcut and single-instance callbacks share this
dispatcher. The isolated executable uses `dev.pinboard.audit20261022`, with no
`PINBOARD_DATA_DIR` override, so it exercises the actual single-instance and
autostart plugins without modifying production registration. Early settings,
normal and autostart relaunches now exit successfully and create exactly the
expected number of notes (`windows-startup-result.json`).

### WIN-002: Concurrent settings requests can create duplicate settings windows

After WIN-001, startup and a second-instance worker could both pass the
existing-window check before either finished building Settings. Two WebViews with
the same settings label were observed. Startup now uses the worker dispatcher,
and `open_settings` serializes its check/build pair off the UI thread. Eight
concurrent settings relaunches left exactly one settings WebView and two note
WebViews. All secondary processes exited with code 0. Final Windows library
regressions passed: 94 tests (`rust-native-events-final.log`).

The fixture's autostart switch was exercised off/on/off against its own Run entry.
After process restart, both the switch and registry remained off, with
`autostart_initialized: true`. The real Pinboard Run entry matched its baseline
throughout (`autostart-restart-result.json`).

### LIB-001: The Linux GLib dependency crashes during optimized string iteration

The original GLib 0.18.5 crate's iterator tests crashed with SIGSEGV in an optimized
Ubuntu build, reproducing RUSTSEC-2024-0429. GTK3 pins the compatible 0.18 API;
the fixed 0.20 API cannot be substituted without changing that dependency graph.
The upstream 0.18 backport request was closed without merging as of the
2026-10-22 inspection.

The application now vendors 0.18.5 with exactly the two-line merged upstream
correction: the pointer passed as a C out-argument is mutable. The
[backport record](../src-tauri/vendor/README.md) contains archive/source provenance,
the fix commit, license and verification commands. All 121 original files are
hash-checked after reversing only those two lines in memory. Git attributes
preserve those bytes on Windows checkouts. The security gate and notice generator
run the integrity guard; Linux packaging runs an optimized integration regression.
The notice generator now retains reviewed path-dependency notices and identifies
the modification instead of skipping local dependencies.

The same 11 optimized upstream tests passed after the correction. The Linux
application's optimized iterator regression, 93 library tests, release executable
GUI/restart harness and six startup checks also passed. A text scan of 8,672
Linux-resolved Rust source files found references only within GLib's implementation,
tests and documentation; this is not a whole-program reachability proof. The fix
does not depend on assuming the vulnerable path is unreachable. Evidence is in
`verification/full-audit-2026-10-22/linux/glib-backport/`.

The latest Windows security gate passed 94 library tests, frontend regressions and
Svelte validation. Cargo reports zero vulnerability-category findings and seven
unmaintained-package warnings. The local path dependency no longer appears in
Cargo audit's advisory output; that scanner limitation is explicitly documented,
and remediation is supported by source integrity plus the optimized red/green
runtime evidence. No advisory ignore was added.

### Extended settings checks

The global font UI saved Noto Sans KR, 20 px and weight 600; the existing welcome editor's computed style immediately matched. Native creation from a source and from the latest surviving note retained opacity, font and background while excluding body, inline formats and pin state. Five invalid settings payloads were rejected without changing current settings. See `native-settings-result.json` and `font-global.png`.

Six language changes preserved the already edited welcome note and a user-authored note's exact file hash (`edited-welcome-language-result.json`). A fresh `native-c` fixture then translated the pristine welcome note into all six languages (`pristine-welcome-language-result.json`). Two running instances registered F8 and F10: attempting to replace F10 with the occupied F8 failed, retained F10, and pressing F10 from the other instance's window still created exactly one note in the correct process.

The design page resized a 4000x2000 PNG to 2560x1280 WebP. A text-file upload showed an error while preserving the image; Remove cleared the stored background while preserving the note body. A nonexistent global font produced the documented fallback notice and the default preview family. The UI's Reset/Apply restored the default font. Calendar top pin persisted as `calendar_pin: "top"` in the isolated local state. MCP enabled state survived the rebuilt app's restart and was disabled again afterward.

After WIN-001/002, native Ctrl+Alt+F8 again created exactly one persisted note; the
test shortcut was disabled afterward. System appearance responded to emulated
WebView color-scheme events in both directions, and explicit dark mode stayed dark
under a light system preference. These tests did not change Windows display or
theme settings (`shortcut-dispatch-regression.json`, `system-theme-*.json`,
`explicit-theme-override.json`).

### Native URL boundaries

A real click on the note's HTTP URL opened Chrome, which requested the task-owned loopback probe and displayed its expected heading. The CLI's initial element-center click landed between wrapped text lines; clicking inside a measured text rectangle exercised the actual link. This was a test targeting issue, not an application defect. The two task-created Chrome tabs were closed.

The native opener rejected `file:` and `javascript:` URLs. Assigning an external URL to the WebView retained the bundled URL, editor and note body after settling (`navigation-denied-settled-result.json`). The probe nevertheless recorded a request for that attempted URL. The navigation guard proves document confinement, not suppression of every outbound request; network behavior needs further inspection before making a stronger claim.

### Native editing and presentation details

The current source builds successfully with a relative `frontendDist` overlay. An absolute Windows drive path in the temporary overlay was parsed as a URL by Tauri's untagged `FrontendDist` enum, producing an empty test window; the project configuration was not changed.

The isolated WebView2 session has exercised note creation, Korean/emoji editing, toolbar bold/italic/size/highlight, undo with Ctrl+Z and redo with Ctrl+Y, six tints, transparency endpoints, all three pin-mode controls, installed Noto Sans KR selection, per-note size changes with inline-size reset, light/dark modes, all six live languages, background PNG conversion and transparency, and calendar empty/populated days. Persisted JSON was inspected for body, UTF-16 formatting, font, tint and opacity. Restart preserved the text, formatting, selected font, Korean language and dark theme. These observations do not yet prove native z-order, cross-monitor behavior or all error paths.

The locked CodeMirror keymap binds Windows redo to Ctrl+Y. Ctrl+Shift+Z is a macOS/Linux binding, so it is not a separate promised Windows command. Ctrl+B has no binding in the editor; toolbar formatting is the supported tested route.

### Live MCP and permission checks

`test-mcp-live.mjs` in the audit directory launches the current executable with `--mcp --mcp-data-dir <isolated fixture>` and checks newline-delimited JSON-RPC. Unprovisioned and disabled states reject tool execution. The enabled run passed 16 round trips covering initialization, discovery, search, read/create/append/update, visibility, pin modes, deletion, invalid opacity/path/unknown arguments and persisted tombstones. Disabling again revoked access. Client auto-registration was not invoked and real client configuration files were not written.

Native note-window calls to settings save, MCP settings and ZIP export were denied by the capability system. Invalid note paths were rejected. Out-of-range finite opacity is intentionally clamped to the valid range rather than rejected; the diagnostic `native-negative-ipc.json` records that distinction.

### ZIP transfer

The UI exported `Pinboard_2026-10-22_00-50-42-071.zip` into Downloads, retained as `export-ui.zip` in the audit directory. After another editor change, the native Windows file picker imported it. All live notes matched the exported body, tint, opacity, formats, font and background; `transfer-roundtrip-result.json` records the assertions. The automatic pre-import archive retained the later edit. A malformed ZIP was rejected with every existing note file hash unchanged. See `export-success.png`, `import-success.png`, and `invalid-import-result.json`.

### Forced restore rollback

A read-only file handle denied renaming the isolated `settings.json`. Importing a
valid archive through the running settings WebView's native `import_archive`
command failed with Windows sharing violation 32, after the transaction reached
the settings replacement. Both note JSONs, settings, local state and the other
pre-existing files retained their exact hashes. The automatic pre-import backup
was present, and neither the restore journal nor staging directory remained.
Editors were released from their inert state; a subsequent Korean/emoji edit was
saved successfully. The lock helper exited normally and released its handle.

This check used native IPC rather than the file picker: the Windows automation
helper returned inconsistent focus/index data for the owned dialog, so that dialog
was canceled. The earlier round-trip check separately proves native file-picker
operation. Evidence: the `restore-rollback-*-result.json` files.

### Two-instance folder synchronization

Two current binaries run against `native-a` and `native-b`, separate WebView2 profiles and one task-owned `shared-folder`. This tests the folder synchronization backend and native workers, not Google Drive's external upload service.

- Initial A-to-B transfer retained the complete note, including formatting and background.
- A UI edit on B reached A. B's top pin and A's normal pin remained device-local.
- With both workers offline, distinct edits converged to the original and a conflict copy on both devices, preserving both bodies.
- A concurrent edit survived a deletion; a subsequent uncontested deletion propagated to both devices.
- Malformed remote JSON put both workers in the error state without changing either local note hash. Restoring the valid remote and syncing returned both workers to idle.

Evidence: `sync-roundtrip-result.json`, `sync-conflict-result.json`, `sync-edit-delete-result.json`, `sync-delete-result.json`, `sync-malformed-*.json`, `sync-recovered-*.json`, and `sync-error.png`.

### Automated checks

- Save queue regression script passes.
- Svelte check: zero errors and warnings.
- Rust library: 94 passed after the local-state and bundled-root regressions.
- The release validation script and agent guide now include the save queue regression command.
- The Vite-based Linux TypeScript runner now includes the save queue and link regressions; Ubuntu's Node cannot import `.ts` directly. Current Linux links, formatting, calendar dates, save queue, i18n and Svelte checks passed. Before NAV-001, 92 Linux library tests passed while the GUI failed at startup; the successful rebuilt runtime result supersedes that failure.
- The Linux GUI harness now selects the current language dropdown and checks all six languages, replacing its obsolete button selectors.
- The snapping module is now compiled only on Windows or in tests, matching its sole production caller and retaining its platform-neutral unit tests. The latest Linux rebuild completed without the previous production dead-code warnings.
- Before LIB-001, the Windows gate reported eight advisory warnings, including GLib unsoundness. The reviewed backport and later scan supersede that state; seven maintenance warnings remain and no ignore was added.
- Current Linux TypeScript (including shortcut capture), Svelte and 93 library tests passed. The rebuilt executable passed the full GUI harness under WSLg Wayland with `DISPLAY` removed, including Korean editing, six languages, font controls, MCP opt-in and restart restoration. See `linux/wayland/` evidence.
- Documentation validation currently covers 32 Markdown files and 100 resolved local links. The three Markdown files containing Hangul retain only previously identified literal paths/UI acceptance strings. Product translations, licenses and historical evidence were preserved.

### Current package verification after DRAG-001

Windows passed the security gate, 94 library cases, frontend regressions and
optimized release compilation. The rebuilt installer payload passed 27 isolated
compatibility checks and six destination-path cases. Linux passed 93 library
cases, the optimized GLib regression, frontend checks, Wayland GUI/restart and
12 native X11 interaction groups. All builds use the existing lockfiles.

The latest Windows executable reopened the prior isolated data and restored the
exact Korean/emoji body without an error. Its native drag command reported the
Windows manual-movement contract. This verifies startup, persistence and the
command result, not physical pointer movement or snapping. See
`windows-drag-fixed-restart.json` and its screenshot.

Debian extraction verified all six package checksums, notices, dependency loading
and the six startup/single-instance cases against the extracted executable. The
packaged ELF differs from the raw release only in Tauri's documented
`__TAURI_BUNDLE_TYPE_VAR_UNK` to `__TAURI_BUNDLE_TYPE_VAR_DEB` marker; the verification
normalizes exactly that marker and compares every remaining byte. This is package
payload verification, not a system-wide installation or bare-metal GNOME test.

Vite 8's separate WebSocket server caused a port-conflict diagnostic in a test
loader despite HMR being disabled. The three SSR-only test loaders now explicitly
disable WebSockets. The drag and localization suites passed again without that
diagnostic; the application runtime is unaffected.

### GLASS-001: Backdrop cache outlives a native note window

Source review found that the global blur cache retained a note label after its
window was destroyed. ZIP restore destroys editors and recreates the same labels,
so an unchanged opacity could skip the new window's effect request. The cache
also recorded a requested value before `set_effects` accepted its dispatch.

Each new native note now resets its cache entry; Destroyed removes it even during
restore. Only accepted effect dispatches enter the cache. Unchanged successful
requests remain coalesced. Two regressions cover retry, duplicate suppression and
cache reset. An extracted-production-function probe with a rejected dispatcher
failed on the original function (exit 101) and passed on the corrected function.
Tauri queues the compositor call and discards its internal result, so this retry
claim covers dispatch failure, not every OS compositor failure.

Windows passed the full security/frontend gate and 96 library cases; Linux passed 95. The rebuilt Linux release
passed the 11-check Wayland flow, 12 native X11 groups and extracted-Debian startup
and checksum checks. The final Windows Tauri-built candidate passed 28 isolated
installer checks (the additional item preserves the running task fixture).
The earlier six path cases still apply: the installer/path sources are unchanged.

On the current Windows candidate, actual desktop captures show clear underlying
settings text at opacity 0.30 and an acrylic backdrop at 0.31. The UI slider is
transparency, so these correspond to 70% and 69%. Native IPC export/import then
recreated the same note UUID and retained its exact Unicode body and opacity 0.31.
The new editor rendered with its backdrop. This is native compositor evidence,
distinct from WebView screenshots and the dispatcher regression above.

A Cargo-only intermediate build opened blank WebViews because it omitted the
Tauri build configuration. It and `package-win-glass-fixed` are excluded from
delivery. The corrected `npm run tauri -- build --no-bundle -- --locked` build
rendered normally; its package is `package-win-glass-tauri`, with hashes recorded
under `windows/glass-candidate-sha256.json`. No production executable was replaced.

### WIN-003: A normal note obscures the newly focused application after blur

A passive WinForms checkerboard fixture provided an ordinary overlapping window
without reading user data or injecting input. Native desktop captures verified
that Top remains visible over the focused fixture, while Desktop rises on note
focus and returns behind the fixture on blur.

Normal mode failed after a focus/blur cycle: the checkerboard window held native
focus and the note reported `[is_focused, is_always_on_top] = [false, false]`, yet
the note continued to cover it. Clicking the uncovered checkerboard did not
remove the occlusion. Both captures are retained as
`windows-stack-normal-blur-before.png` and `windows-stack-normal-blur-repeat.png`.
This shows why the earlier native flag result alone was insufficient.

The locked Tao implementation demotes with `HWND_NOTOPMOST`.
[Microsoft's SetWindowPos contract](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)
places that window above the normal group. The correction restores a normal
note below the current foreground normal window after Tauri updates its flag.
It preserves focus, size and position, skips the desktop and topmost group, and
does not demote a newer Top/focus state. The same helper covers programmatic pin
changes and Settings blur. Desktop classification is shared with the existing
foreground hook.

The corrected optimized binary passed the same native focus/click sequence:
Normal now becomes visually covered by the focused checkerboard. Top remains
above it, an unfocused Top-to-Normal change restores the correct order, and
Desktop still rises on focus and returns behind on blur. A second, explicitly
topmost checkerboard verified that Normal can rise above a pinned application
while focused and returns below it with `[false, false]` after blur. Calendar
Top persisted as `[mode, focused, topmost] = ["top", false, true]`; that calendar
check verifies native flags, while the note checks also verify visible overlap.
See `windows-stack-result.json` and its before/after captures.

The Windows full security/frontend gate and 96 library tests passed. The rebuilt
package passed 34 checks, including five installed-payload hash checks and
preservation of both running test fixtures. The staged Windows candidate was
refreshed to executable SHA-256
`E71A202F3839E0FE08CBC9AAFD1367372B9E77F5AB3A59A4DDD253033B7EA451`.
Only `platform/windows.rs` changed in the application since GLASS-001. Linux
excludes that module with `#[cfg(windows)]`; its validated binaries and runtime
results remain unchanged. The Linux source copy received the same Windows-only
file, and all 81 source-input hashes match before staging.

## Session state and delivery

At the earlier DRAG-001 checkpoint, Windows test instances and their named browser sessions were closed. The
normal-identity fixture's autostart and shortcut remain disabled; both file-lock
helpers exited and no filesystem obstruction remains. The loopback probe and its
two Chrome tabs were closed. Linux GUI/startup and tray-probe scripts terminated
their test processes. The original production processes 22880 and 9704 remained
running. Scoped backups, fixture data, scripts and detailed logs remain in the
audit directories.

The current GLASS-001 fixture is `native-g`, process 35060, CDP port 19331 and
browser session `pinboard-glass-7db9ed74ae4a`. It remains open for the requested
manual pointer check on the synthetic note beginning `Release verification 한글`.
Verify live process/path ownership before cleanup; these are observed identifiers,
not permission to terminate a future process that reuses the number. Earlier
fixtures F and the Cargo-only G process were stopped. All current build/test jobs
have exited successfully; the final Windows security gate also passed.

The later `native-h` stacking fixture and both passive backdrop programs were
closed after verification. Its browser session `pinboard-stacking-7db9ed74ae4a`
is closed. Fixture G remains open only for the pending manual pointer check; it
still runs the earlier GLASS-001 image, whose drag implementation is unchanged.
The current release and staged candidate include WIN-003. Production processes
9704 and 22880 were preserved throughout.

Parent distribution hashes are unchanged by this full audit: portable app `222AAA21A86608C55CD9D4C0A809C66CAFE5235548E0F2C119B3606F6C8ECAC1`, installer `586D73667F0C68314A0C4D16DFFC60842CA215437A2D276988E9A30BDB32BB90`, standalone remover `F6BA3C727097041B0A624AC692DB9616A1741041898C196BCEDCEB1E7E298483`. The new app fixes are source changes and isolated test builds until final verified delivery.

## Remaining work

Exercise the remaining native workflows in the ledger: Windows notification-area
interactions and visual behavior after the shell's Show-desktop gesture.
Physical dragging/snapping/resizing passed the later user-confirmed check.
Note pin/focus behavior now has desktop evidence after WIN-003, and the
acrylic threshold has desktop evidence after GLASS-001. The shared
hide-all/quit save protocol, native close and Linux size inheritance now have
runtime evidence after SAVE-004. The Windows UI helper
lists Settings but does not expose note/tool windows as independent targets; do
not fabricate handles or treat WebView screenshots as native stacking evidence.
The private Linux D-Bus host exercises the registered tray menu and supersedes
the earlier hostless probe timeout. Link selection, native calendar behavior,
the navigation probe and Wayland after GLib/DRAG-001 are now covered above.
Complete the final source/delivery review and deliberate verified delivery. Keep production
executables unchanged until deliberate verified delivery. Do not mark this audit
complete from unit tests or selected successful workflows alone.

## Required final cleanup

The user requested cleanup after the full goal is finished (2026-10-22 KST).
Before marking completion, retain final deliverables and the minimum reproducible
verification record, then remove task-created intermediate builds, duplicate
logs, test profiles/data and disposable caches. If ordinary deletion fails,
attempt the Recycle Bin. Verify resolved targets and stop only task-owned
processes first; preserve user notes, unrelated releases and active applications.

The primary task-owned roots are `C:\build\pinboard-full-audit-20261022-0025`
and `/home/<user>/Pinboard-audit-20261022`. Inventory earlier installer work
separately before cleanup. Do not delete evidence still needed for the remaining
native checks. Cleanup and its result are required work, not yet completed.

## Staging and payload review at 03:40 KST

The installer fixture now hashes the executable identified by the rendered NSIS
template and verifies the installed bytes after each clean install, reinstall,
upgrade, downgrade and silent update. All 33 checks passed against the final
Tauri-built payload; the recorded SHA-256 is
`BDC26CF20BD7815016C221728884694C54DD5FB037DE5A642FFD0BBEC335FA40`.
This adds package-content evidence, not Windows pointer/tray coverage.

Both platform candidates and their matching notices were copied into
`releases/full-audit-2026-10-22-candidate` in the parent workspace. Its manifest
records artifact hashes. All 81 application/build inputs matched between the
current Windows source and Linux build copy before staging. The folder is
explicitly labeled a candidate; production distribution files remain unchanged.
The manual pointer check is still pending, with the synthetic note's persisted
position/size unchanged at the latest observation.

## Blocked checkpoint at 04:07 KST

The same native interaction limitation has remained across three consecutive
goal turns. Independent defect correction, packaging and integrity verification
continued while the manual check was pending. The last turn revalidated existing
evidence but did not advance the remaining interaction checks.

At this checkpoint the task fixture process 35060 remains live, with its original
03:28:22 KST start time. The synthetic note still persists at x=80, y=100,
width=360, height=300. This unchanged state does not prove a drag failure; it
provides no evidence that the requested manual check occurred. A fresh native
window inventory again exposes Settings but neither the note nor the Windows
taskbar as independently targetable windows. Earlier target-guard recovery did
not enable physical note input. No build or test job is awaiting completion.

All eight staged artifacts and 81 application/build source inputs passed hash
verification. The four stacking reproduction files are retained, and the
documentation check passed 100 local links across 32 Markdown files. Repeating
these checks cannot replace physical drag/snap/resize, notification-area or
Show-desktop evidence.

Resume with the pending manual check on the synthetic note beginning
`Release verification 한글`, or a supported native tool route that exposes these
targets. Then finish the remaining Windows shell checks, final delivery review,
distribution replacement and requested cleanup. Production applications and
data remain preserved; task fixture G stays open for the pending check. Final
cleanup remains required and has not been performed.

## User-confirmed Windows pointer check

The user subsequently confirmed that physical movement, edge snapping and corner
resizing felt natural. The task note's persisted bounds changed from
`80,100,360,300` to `-6,0,569.3333333333334,246.66666666666666`; the negative outer
x coordinate is consistent with the invisible Windows resize border accounted
for by the snapping code. This is user-observed native behavior, not automated
pointer replay. See `windows/windows-manual-pointer-result.json` for the exact
report, timestamp, loaded binary and state. The later WIN-003 change does not
alter drag, snapping or resizing implementation.

The pointer gate is now satisfied. Notification-area and Show-desktop checks,
final delivery and cleanup remain pending. Fixture G alone was changed to
English so its notification menu can be distinguished from the user's Korean
production instances. Its synthetic note is prepared in Desktop pin mode for
the shell check; production settings and notes were not changed.

## Direct Computer Use shell attempt at 04:23 KST

The user explicitly asked the agent to perform both remaining checks through
Computer Use instead of delegating them back. The agent dispatched `Win+D`,
then another `Win+D` during recovery, and `Win+B` for the notification area.
The first state capture failed with `window is not a usable app window` after
the desktop transition. Later recovery reported `window is minimized`, followed
by `window id 2236902 was not found` after activation and fresh selection. A
JavaScript-kernel reset did not restore the missing target. No fabricated window
identifier or alternative input-injection helper was used.

The note's window queries reported visible=true, minimized=false, focused=false
and topmost=true after the first desktop transition and again at the end. These
query results do not establish actual desktop overlap or return-to-bottom
behavior; the native event hook uses direct OS ordering calls. No visual pass
is claimed from those flags. The fresh notification-area inventory returned
only Sticky Notes and Chrome, with no taskbar or tray target, so Hide All,
Show All and double-click could not be exercised through Computer Use.

The remaining shell checks are tool-limited, not user-confirmed failures of
Pinboard. The user declined further manual delegation; do not repeat the same
manual requests. Preserve the pending verification scope and cleanup requirement.
See `windows/windows-shell-computer-use-attempt.json` for the limited evidence.

## Cleanup completed on 2026-10-22 KST

The user subsequently requested immediate cleanup, superseding the earlier
instruction to wait for full-goal completion. Reviewed Windows build targets,
intermediate packages/executables, disposable WebView profiles and root logs
were deleted, together with the audit's WSL target, frontend output, disposable
dependency copy and duplicate runtime-verification directory. The repository's
generated `dist` was also removed. Four junctions to shared dependencies were
preserved instead of traversed.

The cleanup deleted approximately 58.93 GiB of logical file content. Five
additional test executables (about 54 MiB) went to the Recycle Bin after automatic
approval review rejected their deletion with `blocked by policy`; the reversible
Recycle Bin operation succeeded. Exact paths and byte totals are recorded in
`cleanup-windows.json`, `cleanup-linux.tsv`, `cleanup-recycled.json` and
`cleanup-summary.json` in the evidence directory. WSL file deletion does not
imply automatic shrinking of its host-side virtual disk.

Task fixture G and its named browser session were closed after checking process
ownership. Production process 9704 remained running. Source code, scoped source
backups, small test-data fixtures, required evidence and final staged candidates
were retained. All eight staged hashes, 81 source-input hashes and four parent
distribution hashes were unchanged after cleanup. Historical build-directory
references above describe earlier evidence; removed outputs now require a rebuild.

The requested cleanup is complete. Remaining shell verification and deliberate
final distribution replacement are separate unfinished work; cleanup does not
mark the full audit complete.
