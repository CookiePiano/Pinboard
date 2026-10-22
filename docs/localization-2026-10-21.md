# Chinese and Japanese localization

Verified on 2026-10-21 (Asia/Seoul).

- Korean remains the default. Settings > General > Language now offers Korean (`ko`), English (`en`), Simplified Chinese (`zh`), and Japanese (`ja`).
- Main settings, note tools, font settings, calendar, AI collaboration, native tray menus, window titles, welcome notes, and existing localized native errors support all four languages.
- Calendar dates use `zh-CN` and `ja-JP`. Windows font family labels request the corresponding DirectWrite locale.
- Language preferences survive serialization and backup import/export. Only untouched welcome notes are translated; user edits and the welcome note's sync identity are preserved.
- New catalogs live in `src/lib/locales/`. Catalog shapes are checked against the Korean source. Native translations use `tr!(korean, english; chinese, japanese, arguments...)`.

## Validation

- `npm run check`: zero errors and warnings.
- `node scripts/test-i18n.mjs`: catalog completeness, live switching across all four languages, Korean default, and argument interpolation passed.
- `npm run build`: passed; existing large-chunk advisory remains.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked -j 8`: 56 tests passed on the merged snapshot, including settings round trips and welcome note preservation.
- Isolated browser preview with mocked Tauri IPC: Korean-to-Japanese and Chinese switching, 640px-wide general settings, and Japanese calendar labels checked. This is frontend UI verification, not a live native-app session.

## Concurrent work

Implementation and builds used a separate snapshot and Cargo target directory. Changes from the other session (individual note fonts, window dragging, tray behavior, and transfer preferences) were merged and retained before validation. Only localization source changes and this regression script/document were copied back. No shared executable, installer, release output, lockfile, or user data was replaced. The desktop app needs the normal release build to include these changes.

## Integrated Windows release — 2026-10-21 KST

After the user confirmed that the other session had finished, the latest shared source was verified to retain the Chinese/Japanese localization, note controls, and Linux integration. The frontend type check and four-language regression script passed, as did all 56 Windows Rust library tests.

`scripts/release.ps1` rebuilt and replaced the top-level `Pinboard.exe`, `Pinboard_0.1.0_x64-setup.exe`, `Pinboard_Uninstall.exe`, and notices. All four published SHA-256 hashes match their build artifacts, and the recorded source inputs remained unchanged during the build. Build logs, input hashes, artifact hashes, and the previous distribution are retained in `../../releases/2026-10-21-202135-languages/`.

The existing user process and notes were preserved. Quit Pinboard from its tray menu, then launch the updated executable to load this release. No Windows native UI re-test or Linux rebuild was performed in this release step.
