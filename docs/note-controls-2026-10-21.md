# Note controls and window behavior

Implemented and checked on 2026-10-21 KST.

## Changes

- Calendar headers use the same position-only native drag and snapping path as notes. Calendar windows participate in edge alignment, keep their dimensions, and can overlap existing notes. No free-space admission check exists in the current calendar implementation.
- General settings expose only the selection size step and highlighter colors. Selection toolbar, bold, italic, size, and highlighter enable/disable controls were already absent from the source baseline and remain absent.
- Focused settings and calendar windows temporarily rise above pinned notes. Settings retains its normal taskbar style and drops its temporary topmost state after losing focus.
- The color/transparency popover includes installed font families and an editable 6–96pt size with minus/plus controls. The buttons use the general selection size step (1pt by default).
- Font overrides are stored per note, round-trip through sync/backups, and are inherited by new notes along with the other appearance properties. Notes without overrides continue to use the PC's default font. A whole-note size change removes range-specific sizes while retaining bold, italic, and highlights; a family-only change retains range-specific sizes.
- Tray left double-click creates one new note. Re-launching an already running app creates a new note, except `--settings` opens settings and `--autostart` does not create a note. Explicit “Show all notes” remains a separate tray menu action.

## Validation

- `npm run check`: zero errors and warnings, including after publishing the source changes.
- `node scripts/test-formatting.mjs` and `node scripts/test-calendar-dates.mjs`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --locked -j 12`: 52 tests passed. Coverage includes legacy note compatibility, per-note font serialization/hash changes, validation, preservation of independent formatting, inheritance, and backup round trips.
- `npm run tauri build`: release executable and NSIS installer produced. Vite reports the existing large-chunk advisory.
- A task-owned native app with separate data and WebView2 profile was inspected through its own CDP connection. The installed font list loaded; Noto Sans KR selection persisted; plus changed 10.875pt to 11.875pt; direct input saved 18pt as 24 CSS pixels; changing the general step to 2.5pt changed 18 → 20.5 → 18. Bold and highlight survived the whole-note size change. The selection toolbar remained present, and the calendar opened at 760 × 460 logical pixels without a space warning.
- The actual Windows desktop was locked when native screenshot verification was attempted. Physical edge dragging, z-order after a native click, tray double-click, and a visual restart check were not verified. No further Windows input was issued after observing the lock.

## Concurrent-session handling and delivery

The original working tree contained untracked source files. A full source baseline and isolated work/build directories were retained under `C:\build\pinboard-20261021-note-controls`. Concurrent edits were merged using the baseline, rather than replacing their files wholesale. Source files written by this task were checked against the release-build inputs.

Another session updated the shared top-level executable while this task was running, so it was preserved. This build is delivered separately in `releases/2026-10-21-note-controls/` next to the project source, with `Pinboard.exe`, `Pinboard_0.1.0_x64-setup.exe`, and the matching license notices. Quit the existing Pinboard instance before starting this executable to use the new code.

Task evidence is under `C:\build\pinboard-20261021-note-controls`: `cargo-test-final.log`, `release-final.log`, `published-source.json`, `font-panel.png`, and `calendar.png`. Existing user app instances, notes, and authentication were preserved.

## Integrated release — 2026-10-21 KST

After the user confirmed the other session had finished, the current source was checked to retain both the note-control changes and the completed Chinese/Japanese localization. No source rollback or replacement from the older isolated snapshot was needed.

The integrated source passed the frontend type check, formatting/calendar checks, four-language catalog checks, and all 56 Rust tests. `scripts/release.ps1` rebuilt and published the shared top-level `Pinboard.exe`, `Pinboard_0.1.0_x64-setup.exe`, `Pinboard_Uninstall.exe`, and matching license notices. Every published file's SHA-256 matches its build artifact; recorded source inputs remained unchanged during the build.

The prior shared distribution was backed up under `C:\build\pinboard-integrated-20261021-2004\before-release`. Build logs, source input hashes, and artifact verification are retained in that parent directory. The running user instance was preserved; quit and restart Pinboard to load the integrated executable. The earlier native UI verification limitations remain unchanged.
