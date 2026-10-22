# Ubuntu 26.04 support

Pinboard uses the existing Rust/Tauri 2 backend and Svelte frontend on Linux.
The Linux executable is a native ELF binary; Wine and Windows executables are
not involved. Build and run it as your normal Linux desktop user.

## Build

Requirements: Ubuntu 26.04, Rust stable, cargo-audit, Linux Node.js 20.19+ or 22.12+, npm.
The release script rejects vulnerable/yanked dependencies before compiling. Maintenance
warnings still require review; the GTK3-compatible GLib backport remains integrity-checked.
Use the same architecture as the target machine (the verified package is amd64).

```bash
bash scripts/setup-linux.sh
bash scripts/build-linux.sh
```

The build uses the existing lockfiles and creates
`src-tauri/target/release/bundle/deb/Pinboard_0.1.0_amd64.deb`.
`CARGO_TARGET_DIR` can redirect the build directory; `CARGO_BUILD_JOBS` defaults
to 8. In WSL, keep the working copy and build output in the Linux filesystem,
and install Linux npm instead of using npm inherited from Windows PATH.

`tauri.linux.conf.json` automatically overrides the Windows-only NSIS target
with Debian packaging. The common Windows configuration remains intact.
The package includes the executable, application launcher, icons, notices,
and dependency declarations. A package built on Ubuntu 26.04 targets Ubuntu
26.04; it does not promise compatibility with older Ubuntu/glibc versions.

## Install and run

```bash
sudo apt install ./Pinboard_0.1.0_amd64.deb
pinboard --settings
```

The default data directory is `$XDG_DATA_HOME/dev.pinboard.notes`, or
`~/.local/share/dev.pinboard.notes` when `XDG_DATA_HOME` is unset.
Existing Windows data is never accessed automatically. To move notes, use the
app's backup/restore or folder synchronization. Device-local window state and
credentials are not portable authentication material.

For a separate test instance, without changing real notes or auto-start:

```bash
PINBOARD_DATA_DIR="$(mktemp -d)" pinboard --settings
```

## Desktop behavior

- The Rust entry point selects X11/XWayland when `DISPLAY` is available and
  `GDK_BACKEND` has not been set. This also applies to login auto-start.
- Set `GDK_BACKEND=wayland` to explicitly select native Wayland. Wayland
  compositors may ignore window positioning, stacking and desktop pinning.
  Global shortcuts need X11/XWayland; without `DISPLAY`, the app starts with
  that feature unavailable and reports an error if it is enabled in settings.
- Linux uses transparency without the Windows acrylic blur or Windows-specific
  edge snapping. Window stacking ultimately depends on the desktop compositor.
- GNOME needs AppIndicator support for the tray menu. The Debian package
  recommends `gnome-shell-extension-appindicator` and `xwayland`; the extension
  may need to be enabled in the user's GNOME session. WSLg itself does not
  provide a full GNOME tray. Open settings from a note, or run
  `pinboard --settings` again; it opens settings in the existing instance.
- Font discovery uses `fc-list` from the packaged `fontconfig` dependency.
- Auto-start is managed by the Tauri autostart plugin for the current Linux
  desktop user in `~/.config/autostart/Pinboard.desktop`. The locked auto-launch
  dependency uses that path even if `XDG_CONFIG_HOME` is customized. It is not
  a Windows startup entry or a system service.

## Verification boundaries

The GTK3-compatible GLib dependency includes a reviewed local soundness backport.
Read [its provenance and checks](../src-tauri/vendor/README.md). The Linux build
script verifies the vendor copy and runs the optimized `glib_variant` regression;
debug-only tests can miss the original crash. Keep the original license and the
modification notice in Linux distributions.

WSL2 runs the Ubuntu 26.04 userspace and a Linux kernel. Build, automated GUI,
and WSLg tests there exercise the Linux binary. They do not establish how a
separate native GNOME/KDE session handles tray extensions or stacking hints.
See the accompanying verification report for checks actually performed.

Automated GUI tests require `tauri-driver` on PATH plus `webkitgtk-webdriver`,
`xvfb`, `xauth`, `openbox` and `xdotool` from Ubuntu. These are test tools, not
runtime requirements. Test the release binary with:

```bash
xvfb-run -a dbus-run-session -- node scripts/test-linux.mjs src-tauri/target/release/pinboard /tmp/pinboard-gui-results
xvfb-run -a dbus-run-session -- node scripts/test-linux-startup.mjs src-tauri/target/release/pinboard /tmp/pinboard-startup-results
xvfb-run -a dbus-run-session -- env PINBOARD_TEST_PRIVATE_BUS=1 node scripts/test-linux-tray.mjs src-tauri/target/release/pinboard /tmp/pinboard-tray-results
```

The GUI harness uses its own `PINBOARD_DATA_DIR`; the startup harness uses
separate XDG directories and skips automatic startup registration. Both
terminate only processes they start. For Wayland, run the GUI harness in an
existing Wayland session with `PINBOARD_TEST_BACKEND=wayland`, without Xvfb;
it removes `DISPLAY` from the child to exercise the no-X11 startup path.

The tray harness additionally needs `gcc`, GIO development metadata and `xprop`.
It compiles a small StatusNotifier host into its output directory and requires a
private session bus. Its real menu callbacks exercise save-failure recovery,
native close, size inheritance, X11 pin/focus hints, header drag and list input.
It never registers a host in the user's desktop bus. This covers application
behavior under Openbox, not installation of a GNOME tray extension.

References: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/),
[Debian packaging](https://v2.tauri.app/distribute/debian/),
[platform configuration](https://v2.tauri.app/develop/configuration-files/),
[tray support](https://v2.tauri.app/learn/system-tray/).
