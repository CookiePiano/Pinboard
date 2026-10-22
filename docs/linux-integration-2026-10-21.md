# Windows and Ubuntu integration — 2026-10-21 KST

## Delivered state

The Linux changes are now integrated into the shared `source` directory, after
the user confirmed that the other session had completed. The latest Windows
features were preserved: per-note fonts, calendar/window controls, backup and
restore, Chinese/Japanese localization, tray double-click, and re-launch behavior.

The completed Windows source was snapshotted before editing, with local backup
commit `2174b9af64e88a3ed812e9547d28b1642967f08d` under
`/home/<user>/Pinboard-integrated-20261021/source`.
Each shared source file was checked against that snapshot before being written.
Only Linux integration files were copied; unrelated source was not replaced.

The old Linux patch conflicted with the updated single-instance callback.
Integration retained the latest behavior: normal re-launch creates one note,
`--settings` opens settings, and `--autostart` does not create another note.
The Linux tray menu and conditional hotkey initialization were added around it.
The new Linux-only hotkey error includes all four supported languages.

Ubuntu's Node package does not provide native TypeScript stripping. The Linux
build now runs the existing formatting/calendar tests through the project's Vite
resolver, without adding dependencies or changing their assertions. It also
runs the four-language catalog test before building.

## Validation

| Check | Result |
| --- | --- |
| Windows frontend type check | 0 errors, 0 warnings |
| Linux frontend type check | 0 errors, 0 warnings |
| Windows Rust tests | 56 passed, 0 failed |
| Linux Rust tests | 56 passed, 0 failed: 55 library + 1 startup |
| Formatting, calendar, four-language regression scripts | Passed on Linux |
| Windows release and NSIS installer | Rebuilt and published by `scripts/release.ps1` |
| Windows published artifacts | Executable, installer, uninstaller and notices match build hashes |
| Ubuntu amd64 release and Debian package | Rebuilt from the integrated source |
| Packaged Linux binary, X11/Xvfb | GUI tests passed |
| Packaged Linux binary, WSLg Wayland without DISPLAY | GUI tests passed |
| Default backend / single-instance startup | Passed |
| Source diff whitespace check | Passed except unchanged verbatim upstream notice whitespace |

Both Linux GUI runs verified creation, Korean multiline editing, Rust persistence,
desktop-pin preference persistence, settings, installed fonts, MCP remaining
opt-in, and restoration after process restart. They additionally verified the
merged per-note font feature (increment button and 18pt input-change event saved
as 24px, in an enlarged note) and live Japanese/Chinese/Korean switching with
persisted settings. The startup test verified normal, settings, and autostart
re-launch semantics using an isolated D-Bus session and XDG data fixture.

The existing Windows user instance was preserved. Restart it to load the new
executable. Linux test processes were stopped and test data remained separate.
No real notes, credentials, or login startup entries were modified by integration.

## Artifacts and recovery

- Shared Windows distribution: parent of `source`, including `Pinboard.exe`,
  `Pinboard_0.1.0_x64-setup.exe`, `Pinboard_Uninstall.exe`, and notices.
- Linux distribution: `linux-ubuntu-26.04/`, including the updated `.deb`, ELF
  executable, integrated source archive, integration patch and SHA256SUMS.
- The patch is now an integration record against the completed Windows snapshot;
  it has already been applied to shared source. Do not apply it a second time.
- Pre-integration source and distributions:
  `C:\build\pinboard-linux-integration-20261021` (`source-before.tar.gz`,
  `before-release`, `linux-before`). Windows logs and artifact hashes are there.
- Linux build/GUI/startup logs:
  `/home/<user>/Pinboard-integrated-20261021/verification`, with final evidence
  copied into `linux-ubuntu-26.04/verification/integrated`.

The Windows GUI was not re-tested through native input in this integration turn.
Linux GUI validation used WSL2/WSLg, not a separate bare-metal GNOME/KDE session.
Compositor-specific positioning, desktop stacking and tray support retain the
limitations in [the Linux guide](linux-ubuntu-26.04.md). Existing Vite chunk-size,
Linux snapping dead-code, and WSL graphics/portal warnings remain nonfatal.

The earlier [isolated-port report](linux-verification-2026-10-21.md) is historical;
this report supersedes its unmerged-delivery status.
