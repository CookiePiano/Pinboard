# Pinboard

[한국어](README.md) | **English**

**Windows 11 x64 · First public version: 0.1.0 · MIT**

[Official downloads](https://github.com/CookiePiano/Pinboard/releases) ·
[Verify a download](docs/download-verification.md) ·
[Privacy](docs/privacy.md) · [Code signing policy](docs/code-signing-policy.md)

The initial 0.1.0 binaries are **not Authenticode-signed**. Windows may display an
unknown-publisher warning. Check the official repository, GitHub attestations and
checksums; do not disable Windows security protections. A future signed release
will use a new version, never replace an existing immutable release.
Linux source is retained for development; this release supports Windows 11 x64 only.

Pinboard is a translucent desktop sticky-note application for Windows and Linux,
built with Tauri, Rust, Svelte, TypeScript and CodeMirror. Windows 11 adds a frosted
backdrop. Notes can synchronize between PCs through a shared cloud folder.

The repository opens in Korean by default. Use the language links above to switch
between Korean and English documentation. Linked technical documents are in English
unless indicated otherwise. Change the application's display language separately
in Settings > General.

For agent work, start with [AGENTS.md](AGENTS.md). Dated documents in `docs/` are
historical implementation and verification records, not proof that the current
working tree or installed binary has been tested.
Use the [documentation index](docs/README.md) to distinguish current contracts,
feature ownership, active audits and historical release evidence.

## Behavior and invariants

- Each note has six tint choices and adjustable opacity. At 70% transparency or
  higher, Windows blur is disabled and text receives a subtle outline.
- The note toolbar exposes New note, Calendar, Settings and Delete. Deletion
  requires a second click. The application imposes no note-count limit.
- New note from a note inherits its tint, opacity, size, background image and image
  opacity. Creation from the tray or shortcut uses the latest surviving note.
  Content, inline formatting and pin mode are not copied.
- A failed note save retains the latest edit for retry. Hide, native close and
  tray Hide All keep failed edits visible. Quit and ZIP export abort if an editor
  cannot flush; removing the storage obstruction allows a retry.
- Pin modes are Normal, Always on top and Desktop. Clicking a note temporarily
  raises it while focused; losing focus restores its selected mode.
- Windows dragging snaps to work-area edges and other notes. Corner snapping preserves
  size and does not maximize the window. Drag again to detach.
- HTTP, HTTPS and `www.` addresses are underlined and open in the default browser
  on click. Select by dragging or use arrow keys to edit characters inside a link.
- The editor supports selected Markdown features and styles the first line as a
  title. Task checkboxes are not interactive; existing `- [ ]` and `- [x]` text is
  preserved as ordinary text.
- Selecting text exposes **B / I / − / + / H** for bold, italic, size and highlight.
  Size changes default to 1pt. General settings configure the step and highlighter
  color, with six presets and a custom picker. Formatting participates in undo,
  redo, persistence and synchronization.
- Settings > Design attaches JPG, PNG or WebP note backgrounds. Images retain
  their aspect ratio, fill from the center, have independent opacity and sync
  with the note.
- Settings > General selects System, Light or Dark appearance for all windows.
  It also selects Korean (`ko`), English (`en`), Simplified Chinese (`zh`), Japanese
  (`ja`), French (`fr`) or Spanish (`es`). Changes apply immediately and persist.
  Language changes never translate user-authored note content.
- Calendar dates show note previews. Selecting a preview opens only that note.
  The calendar's own pin mode persists on this PC. Select note-card checkboxes and
  click the trash button beside the date twice to delete the selected notes.
  These deletions synchronize just like deletions from note windows.
- Storage choices are This PC and External cloud. External cloud connects a
  folder managed by Google Drive for Desktop; that application handles sign-in.
- Settings > Sync exports notes, images, formatting, settings and window placement
  to a `Pinboard_YYYY-MM-DD_HH-mm-ss-SSS.zip` in Downloads. Import restores the
  archive and first backs up the previous state under `import-backups`.
- AI collaboration requires explicit MCP setup on each PC, followed by enabling
  connections. Before setup, the switch is disabled. Startup, opening settings
  and ZIP import never implicitly provision MCP.
- In Settings > Work with AI, select ChatGPT/Codex, Claude or Gemini and choose
  **Connect automatically** to provision MCP, enable connections and register the
  `pinboard` entry in each selected client's configuration. Other settings and
  TOML comments are preserved; changed files are backed up as `*.pinboard.bak`.
  Invalid configuration files are left untouched and reported per client. Use
  **Set up manually** for the copy-and-paste fallback. Client cards show Connected,
  Needs reconnecting or Not connected. Restart each client after registration.
  Separate setup and connection controls remain available.
- Notes created through MCP copy the window size, font, tint, opacity and
  background of the last-written note (non-blank text, newest edit). A tint the AI
  app requests explicitly still wins.
- While connections are allowed, the AI can operate notes like the user:
  `list_notes`, `read_note`, `create_note` (with optional pin), `update_note`
  (replace or append text, color, opacity, pin, show/hide) and `delete_note`. See
  [AI collaboration](docs/ai-collaboration.md#tool-contract).
- Pinboard runs in the tray and normally enforces a single instance. Login startup
  is opt-in in Settings > General. Existing OS startup choices are preserved.

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| User-configured combination | Create a note globally, when shortcuts are enabled |
| `Esc` | Close the open panel |

The global New note shortcut is disabled by default. Enable it in Settings >
General > Shortcuts, choose Change, then press a combination containing Ctrl, Alt
or the Windows key. If another application owns that combination, Pinboard reports
an error and retains the previous shortcut. Changes take effect immediately and
persist. Standard editor copy, paste, undo and redo shortcuts remain available.

## Development

Run commands from the repository root (`source/` in the local delivery folder). Use versions compatible with `package-lock.json` and
`src-tauri/Cargo.lock`; check installed tools before building. Windows requires
the Rust MSVC toolchain and WebView2 (included with Windows 11). Standalone `.mjs`
tests importing `.ts` files require Node with TypeScript stripping.
`scripts/test-linux-typescript.mjs` provides a Vite-based runner for links,
formatting, calendar, save-queue and shortcut-capture tests when native stripping is unavailable.

```powershell
npm ci
npm run tauri dev
npm run check
node scripts/test-links.mjs
node scripts/test-formatting.mjs
node scripts/test-calendar-dates.mjs
node scripts/test-i18n.mjs
node scripts/test-save-queue.mjs
node scripts/test-performance.mjs
node scripts/test-shortcut-capture.mjs
node scripts/test-window-drag.mjs
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
```

Build and distribution commands:

```powershell
npm run tauri build
.\scripts\release.ps1 -OutputDirectory C:\build\Pinboard-0.1.0\artifacts -TargetDirectory C:\build\Pinboard-0.1.0\target -SigningPolicy UnsignedInitial
```

`npm run tauri build` creates the installer under
`src-tauri/target/release/bundle/nsis`. `scripts/release.ps1` requires separate
build and empty output directories and never replaces the parent distribution.
It provisions hash-pinned tools, tests, scans and packages before copying the
completed release to the output directory. It remaps the user
profile, Cargo home and checkout paths with `--remap-path-prefix`, and fails if the
built executables still contain the developer profile path. A plain
`npm run tauri build` does not apply this remapping.

```text
Pinboard/
  Pinboard.exe                       Portable application
  Pinboard_<version>_x64-setup.exe    Installer
  Pinboard_Uninstall.exe              Installed-app removal wizard
  THIRD-PARTY-NOTICES.txt             Required distribution notices
  source/                            Source repository
```

Set `PINBOARD_DATA_DIR` to isolate application data for testing. This also disables
single-instance enforcement, allowing multiple isolated instances on one PC.
Give each test instance its own data directory. Use `--settings` to open Settings
at startup. Data without a language preference defaults to Korean. Untouched
welcome notes follow the selected language; edited notes are preserved.

The release script does not change an installed or running application. It emits
an installer, portable ZIP, SPDX source-dependency inventory, manifest and checksums.
The GitHub release workflow builds the reviewed tag, verifies the exact installer
on a disposable runner, issues provenance and creates a draft. See
[the release process](docs/releasing.md).

See [installer/README.md](installer/README.md) for installation, removal, branding
and verification. `Pinboard_Uninstall.exe` targets only the current user's installed
application, preserves notes and settings, and leaves the portable executable
untouched. It asks the user to close the installed application when necessary.

## Source map

| Path | Responsibility |
| --- | --- |
| `src-tauri/src/model.rs` | Persisted note schema and backward-compatible sync hashes |
| `src-tauri/src/store.rs` | Local note storage and device-local state |
| `src-tauri/src/sync/mod.rs` | Three-way sync, conflicts, tombstone cleanup and engine tests |
| `src-tauri/src/sync/folder.rs` | Folder backend |
| `src-tauri/src/sync/gdrive.rs`, `oauth.rs` | Legacy direct Drive integration; not selectable in current settings |
| `src-tauri/src/sync/worker.rs` | Local-change scheduling (1.5s), folder watching and periodic polling |
| `src-tauri/src/platform/` | OS-specific desktop pinning, window styles and rounded corners |
| `src-tauri/src/windows.rs` | Note/settings windows and PC-local placement |
| `src-tauri/src/transfer.rs` | ZIP export/import, validation, rollback and recovery |
| `src-tauri/src/editors.rs` | Shared editor flush/freeze handshake for backup, hide and exit |
| `src/lib/window-drag.ts` | Shared note/calendar pointer ownership and drag cleanup |
| `src-tauri/src/mcp/` | Local stdio protocol, explicit provisioning and live-app bridge |
| `src/lib/note/` | Note UI, CodeMirror integration, links and inline formatting |
| `src/lib/calendar/`, `src-tauri/src/calendar.rs` | Calendar UI and native commands |
| `src/lib/settings/` | Settings UI |

## Data and synchronization

The default Windows data directory is `%APPDATA%\dev.pinboard.notes`.

| File | Meaning |
| --- | --- |
| `notes/<id>.json` | Note content; deletion tombstones remain for 30 days |
| `local-state.json` | This PC's window position, size and pin mode; not synchronized |
| `settings.json` | Application preferences |
| `sync-state.json` | Last synchronization baseline for each backend |

Concurrent edits keep the newer version under the original note ID and preserve
the other version as a conflict copy. An edit wins over a deletion. Legacy direct
Google Drive settings migrate to local storage; note files and credentials are
retained. See [cloud folder setup](docs/google-drive-setup.md),
[transfer behavior](docs/transfer-2026-10-21.md) and
[AI collaboration](docs/ai-collaboration.md) for detailed contracts.

## Linux (Ubuntu 26.04)

Build scripts, Debian packaging, desktop behavior and verification boundaries are
documented in [docs/linux-ubuntu-26.04.md](docs/linux-ubuntu-26.04.md).

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev pkg-config
```

GNOME Wayland restricts application-controlled positioning and stacking. Use
`GDK_BACKEND=x11 pinboard` through XWayland for position restoration and Desktop
pinning. Linux supports transparency without Windows blur. This distribution's
cloud-folder instructions and observed provider checks concern Google Drive for
Desktop on Windows.

## Security

Read [SECURITY.md](SECURITY.md) before publishing or enabling external integrations.
The first public version is 0.1.0. Earlier local 0.1.1 artifacts and their
[remediation record](docs/security-remediation-2026-10-22.md) are internal development
history, not a previously published GitHub release. Security fixes remain included.
It describes plaintext storage, trusted sync folders, MCP permissions, signing and
reporting requirements. The [2026-10-21 security review](docs/security-review-2026-10-21.md)
records the earlier fixes and Windows verification. The current
[GLib backport](src-tauri/vendor/README.md) addresses the Linux iterator issue while
preserving GTK3 compatibility; the full audit records current validation limits.
The release script runs dependency audits and regression checks before building.

## License

Pinboard is released under the [MIT License](LICENSE).

## Third-party notices

Copyright and license notices are retained in
[THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt), rather than listed in About.
The installer includes them; the portable application writes its embedded copy
to the data directory. Preserve these notices even for free distribution.
`npm run build` regenerates them from installed dependencies and lockfiles.
See [licenses/README.md](licenses/README.md) for upstream sources and supplements.
