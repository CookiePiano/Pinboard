# Font settings

Implemented and checked on 2026-10-21 KST.

Settings has a **Font settings** section immediately below General. It scans installed font families, supports searching both localized and canonical names, previews Korean, English and numerals, and applies a family, size (10–36px) and weight (400/500/600/700) to all note editors. Restore defaults changes the preview; Apply saves the choice.

Preferences are local to this PC in `settings.json`, under `noteFont` (`family`, `size`, `weight`). Existing settings without this field keep the previous appearance: bundled Pretendard, 14.5px, weight 400. The first line retains its title treatment and scales with the selected size. Markdown code keeps its existing monospace treatment. Changing typography does not rewrite note data or reset the editor's content, selection or history.

Windows enumeration uses DirectWrite's system font collection with update checking and localized display names. Canonical family names are saved, so switching the UI language does not change the selected font. Font files are neither read nor copied. Linux uses `fc-list` from fontconfig; Linux runtime behavior has not been tested in this Windows session.

The existing bundled/system CSS fallback stack handles unavailable fonts and missing glyphs. The settings panel also identifies a selected family that is absent from the current scan. Rescan supports fonts installed while Pinboard is running. Scan and save failures have separate error messages, and failed saves do not show a success message.

## Integration points

- `src-tauri/src/fonts.rs`: enumeration, preferences, validation and Windows enumeration test.
- `src-tauri/src/settings.rs`: backward-compatible local preferences.
- `src-tauri/src/commands.rs`, `lib.rs`: asynchronous `list_fonts` command and save validation.
- `src/lib/settings/FontPanel.svelte`, `font-messages.ts`: font controls and Korean/English copy.
- `src/lib/settings/Settings.svelte`, `src/lib/i18n.ts`: navigation entry.
- `src/lib/ipc.ts`, `src/lib/fonts.ts`, `src/App.svelte`: typed IPC and settings broadcasts.
- `src/lib/note/editor.ts`, `NoteWindow.svelte`: editor-only typography and layout remeasurement.

## Verification

- Svelte/TypeScript check: no errors or warnings in the final check.
- Final shared release Rust test run: 39 passed, including calendar, MCP, storage and font modules.
- Production frontend build and native Windows debug build succeeded using separate output folders.
- The real Windows font collection returned 296 families.
- Native WebView2 test: searched for Noto Sans KR, previewed and applied 18.5px/600, and verified the existing note editor changed from 14.5px/400 only after Apply.
- Note JSON file hashes stayed unchanged after changing fonts.
- A fresh test process restored Noto Sans KR/18.5px/600 from disk.
- A deliberately unavailable family showed the fallback notice and the preview used the default stack.
- Restore defaults and Apply persisted `family: null`, `size: 14.5`, `weight: 400`.
- Korean and English controls were exercised. The original 760×540 CSS viewport and an emulated 640×460 viewport were inspected; the narrow viewport scrolled vertically without horizontal overflow.
- Enumeration error UI is implemented; the runtime fault-injection attempt did not produce a failed scan, so this error state is not claimed as exercised.

Native testing used a separate `PINBOARD_DATA_DIR` and `WEBVIEW2_USER_DATA_FOLDER`; its process and browser session were closed afterwards. Other running Pinboard processes and their data were preserved. Intermediate builds, source baselines and screenshots have been removed.

## Release follow-up — 2026-10-21 KST

After the other sessions finished, the shared final source passed Svelte/TypeScript checks with zero errors/warnings and all 39 Rust tests. The existing release script rebuilt and updated the top-level portable app, NSIS installer, standalone removal wizard and third-party notices. SHA-256 digests matched their respective build outputs. Source input hashes remained unchanged throughout validation and release. Vite retained its existing warning about a JavaScript chunk larger than 500 kB.

The delivered top-level `Pinboard.exe` was launched with separate note data and a separate WebView2 profile. Its native settings window contained Font settings immediately below General and retained the AI collaboration menu. It enumerated 296 font families; selecting Noto Sans KR, 18px and weight 600, then Apply, updated the actual note editor and produced the saved confirmation. The note toolbar also retained its calendar button. The 760×540 CSS viewport was captured and inspected. The test process and its browser session were closed afterwards.

The release log, source hashes, artifact receipts and delivered-build screenshot are retained in the [current release record](verification-2026-10-21.md). Previous release artifacts have been removed. The pre-existing app process and user data were preserved.
