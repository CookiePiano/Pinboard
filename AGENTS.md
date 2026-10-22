# Pinboard agent guide

## Scope and concurrent editing

- This repository is `Pinboard/source`. The parent directory holds distribution
  artifacts, Linux packages and historical release evidence.
- Follow applicable parent instructions. Preserve other agents' changes, running
  applications, user notes, credentials and existing lockfiles.
- Inspect `git status` and file hashes before editing. An untracked file can be
  existing work; do not treat it as disposable. If no commit baseline exists,
  create a scoped backup outside the repository.
- Recheck each target before applying an edit. When it changed since inspection,
  read and reconcile the latest content instead of overwriting it. Keep patches
  narrow and avoid whole-project formatters during concurrent work.
- Use separate build/test output directories. Do not run `scripts/release.ps1`
  alongside another release or replace parent executables as a side effect of a
  source-only change. Do not commit or stage another agent's work.

## Navigation

- Start with [README.md](README.md) for the user-facing introduction and guides.
  Use [the technical reference](docs/technical-reference.md) for behavior,
  commands, source layout and data.
- Note storage and hashes: `src-tauri/src/model.rs`, `store.rs`, `sync/`.
- Editing and rendering: `src/lib/note/editor.ts`, `formatting.ts`, `links.ts`.
- Shared contracts: `src/lib/ipc.ts` and registered commands in `src-tauri/src/`.
- UI text: `src/lib/i18n.ts`, `locales/`, feature-specific message modules and the
  native `tr!` catalogs. Preserve all supported languages when changing a key.
- MCP: [docs/ai-collaboration.md](docs/ai-collaboration.md).
- Backups: [docs/transfer-2026-10-21.md](docs/transfer-2026-10-21.md).
- Packaging: [installer/README.md](installer/README.md) and
  [docs/linux-ubuntu-26.04.md](docs/linux-ubuntu-26.04.md).

## Compatibility requirements

- Keep note JSON and synchronization hashes compatible with existing data. A hash
  implementation change must preserve its exact input bytes and field ordering.
- Text-format offsets use JavaScript/CodeMirror UTF-16 positions. Preserve
  selection boundaries, undo/redo and text during normalization or migration.
- Window position and pin state are device-local. Note content and supported
  appearance fields synchronize. An edit wins over a deletion; conflicting live
  edits retain a separate copy.
- MCP provisioning is explicit and PC-local. Opening settings, startup and ZIP
  import must not silently provision it or edit external AI client configuration.
- Keep legacy compatibility paths unless the requested change includes migration.
- Keep the README and user guide in Korean by default, with matching English
  editions and language links. Write developer documentation in English. Preserve
  literal paths, identifiers, localized acceptance strings and historical
  verification dates/counts. Public screenshots must use isolated synthetic data;
  never include personal notes, account details or private filesystem paths.

## Validation

Run from this directory with installed tools compatible with the lockfiles:

```text
npm run check
node scripts/test-links.mjs
node scripts/test-formatting.mjs
node scripts/test-calendar-dates.mjs
node scripts/test-i18n.mjs
node scripts/test-save-queue.mjs
node scripts/test-shortcut-capture.mjs
node scripts/test-window-drag.mjs
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
```

Use Node with TypeScript stripping for direct `.ts` imports. For the alternative
Vite runner and Linux/native checks, consult the existing scripts and Linux guide.
Use `--target-dir <isolated-directory>` for Cargo during concurrent work. Build the
frontend in an isolated source snapshot or use a task-owned output directory.
`npm run build` regenerates notices, so account for that write before running it.

For native UI checks, use a task-owned `PINBOARD_DATA_DIR` and, on Windows, a
separate `WEBVIEW2_USER_DATA_FOLDER`. Close only processes created for the task.
State which checks actually ran and which source snapshot they covered. Historical
reports, mocked IPC checks, Windows tests and Linux tests are distinct evidence.

Linux release checks also require `node scripts/test-security.mjs` and
`cargo test --release --locked --manifest-path src-tauri/Cargo.toml --test glib_variant`.
Keep the [reviewed GLib backport](src-tauri/vendor/README.md) byte-exact except for
its documented patch. Do not edit vendored documentation or generated bindings
during application refactoring; integrity checks intentionally reject drift.
