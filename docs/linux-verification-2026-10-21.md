# Linux verification — 2026-10-21 KST

Historical report for the isolated Linux port. It was subsequently integrated
into the completed Windows source; see
[the integration report](linux-integration-2026-10-21.md) for current delivery status.

## Isolation and provenance

The Windows source was being edited in another Codex session. No files in that
working tree, its Git index, Windows executables, or real note data were edited
by this Linux task. The original repository had no commits at the time of the
snapshot, so a worktree could not provide a useful committed baseline.

An independent source snapshot was created at
`/home/<user>/Pinboard-linux-20261021/source`, with local baseline commit
`40de3b78a91cb0edf6aac2cd8cebe590ab0029bb`.
The Linux changes are delivered as an independent source archive and a patch.
They have intentionally not been merged into the concurrently edited source.
Later Windows-session changes are not included in this snapshot.

The patch excludes generated `THIRD-PARTY-NOTICES.txt`: regenerate it on the
target OS with `npm run notices`. The Linux source archive and package contain
the Linux-generated notices. Preserve any newer Windows changes when merging;
run `git apply --check` before applying the patch to a separate integration copy.

## Implementation

- Existing Rust/Tauri backend retained; the existing Svelte UI remains.
- Linux-specific Tauri configuration builds a native amd64 Debian package.
- Rust startup prefers X11/XWayland when available, honors explicit GTK backend
  selection, and leaves a pure Wayland session usable without X11 hotkeys.
- Linux tray menus support the platform's menu interaction. Re-running with
  `--settings` opens settings in the existing process, including without a tray.
- Build/setup and automated Linux GUI/startup test scripts added.
- License collection now handles the Linux-only dlopen2 and libappindicator-sys
  packages without dropping their notices or bypassing license checks.

## Environment and results

Ubuntu 26.04.1 LTS, x86_64, WSL2; Rust 1.99.0; Cargo 1.99.0;
Linux Node.js 22.22.1; GTK 3.24.52; WebKitGTK 2.52.6.
The Ubuntu development/test packages were installed for this task. No full
distribution upgrade was performed. tauri-driver 2.1.0 was installed only into
the task's `tools` directory.

| Check | Observed result |
| --- | --- |
| Full `scripts/build-linux.sh` | Passed with locked dependencies |
| `svelte-check` | 0 errors, 0 warnings |
| Rust unit tests | 50 passed, 0 failed (49 library + 1 Linux startup) |
| Optimized release and Debian bundle | Passed; `Pinboard_0.1.0_amd64.deb` |
| Native executable inspection | ELF 64-bit x86-64 Linux executable, stripped |
| Dynamic dependencies | All resolved in Ubuntu 26.04 |
| Package dependency simulation | `apt-get -s install` resolved successfully |
| X11 GUI, binary extracted from final `.deb` | Passed under Xvfb/Openbox |
| Pure Wayland GUI, same packaged binary | Passed under WSLg, with `DISPLAY` removed |
| Default GTK backend and single instance | Passed under X11, without `GDK_BACKEND` override |
| Shell/JavaScript script syntax | Passed |
| Source diff whitespace | Passed excluding verbatim generated upstream notices |

Both final-package GUI tests verified Korean rendering, new-note creation,
Korean multiline editing, Rust-backed file persistence, desktop-pin preference
persistence, settings opening, real fontconfig enumeration, MCP remaining
unprovisioned, and restoration after restarting the Linux process. Screenshots
of notes, settings, fonts and restored content are included with the delivery.
Pin-preference persistence does not prove compositor stacking behavior.

The startup test verified that the second `--settings` process exits successfully
while the first process opens the settings window. First-run autostart registration
was also observed in `~/.config/autostart/Pinboard.desktop`; the test-created entry
was removed. The repeatable startup test now pre-initializes its local fixture to
avoid writing that entry. All test processes were terminated; the package itself
was extracted and tested, not installed system-wide.

## Remaining platform differences

- This is native Linux userspace testing inside WSL2/WSLg, not a separate
  bare-metal GNOME/KDE desktop test. GNOME tray extensions, login behavior and
  compositor-specific window stacking still depend on the user's desktop.
- X11/XWayland is preferred for placement and pinning. Pure Wayland may ignore
  those hints. Global hotkeys require X11/XWayland.
- Windows acrylic blur and Windows edge-snapping hooks are not implemented on
  Linux; Linux keeps translucent note backgrounds.
- WSLg/Xvfb emitted nonfatal EGL, portal/PipeWire and AppIndicator deprecation
  messages. GUI tests passed without disabling the WebKit sandbox. Two existing
  Rust dead-code warnings concern the Windows-only snapping implementation.
- The package targets Ubuntu 26.04 amd64. Compatibility with older distributions
  or ARM64 was not tested. The glibc floor reflects symbols in this built binary.

See [Linux setup and behavior](linux-ubuntu-26.04.md) for build/install commands
and the official Tauri references used for the port.
