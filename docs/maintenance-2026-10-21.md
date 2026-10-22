# Project cleanup and storage refactoring — 2026-10-21 KST

## Project cleanup

The repository had no committed baseline. Before editing, 260 maintained source
and evidence files were copied and hash-verified under
`C:\build\pinboard-maintenance-20261021-224922\before`. The associated manifests,
test logs, comparison sources and results are in that external audit directory.

The following generated directories were moved outside the project:

| Original path under `source/` | Files | Logical bytes |
| --- | ---: | ---: |
| `src-tauri/target/` | 9,106 | 8,412,529,812 |
| `dist/` | 98 | 4,018,580 |
| `installer/build/` | 1 | 233,515 |
| `src-tauri/gen/` | 4 | 422,014 |
| `node_modules/.vite/` | 237 | 29,911,675 |
| `node_modules/.vite-temp/` | 0 | 0 |
| **Total** | **9,446** | **8,447,115,596** |

The Cargo cache, including two superseded `portable-backup-*.exe` files, is now at
`cargo-target/` in the audit directory and was reused for validation. The other
directories are under `generated/` there. Automatic approval review rejected both
the batch recursive deletion and a narrower cache-only deletion with the reason
`blocked by policy`. At the end of the initial pass, these files remained outside
the project; that pass did not claim to have freed disk space. Dependencies remain installed. Historical
verification logs, source archives, release artifacts and distribution notices
are retained as evidence. Generic cache, Python bytecode and TypeScript build
metadata patterns were added to `.gitignore`.

### Follow-up removal

The user subsequently requested moving the retained output to the Recycle Bin.
The Windows `Microsoft.VisualBasic.FileIO.FileSystem` API was called with the
explicit `SendToRecycleBin` option for the external Cargo cache, generated
directories, frontend validation build, synthetic JSON files and allocation-probe
executable/debug symbols. Those six paths, containing 9,548 files and
8,483,980,844 logical bytes, are now absent. However, verification found no
matching Recycle Bin shell entries or metadata in any account's C: recycle
directory. Recycle retention was not established and must not be reported as
successful. The cause was not established. The original source snapshot,
manifests, measurement sources/results and verification logs remain present.
See `recycle-results.json` and `recycle-verification.json` in the audit directory.

## Implementation

- `src-tauri/src/store.rs`: share temporary-file creation, durable flush, atomic
  replacement and failure cleanup between byte writes and JSON writes. Serialize
  pretty JSON directly through a buffered file writer, avoiding a complete JSON
  output allocation for image-bearing notes. Preserve the exact serialized bytes.
  Serialization errors now return an I/O error and leave the destination intact.
- The store removes an entry from memory only after disk deletion succeeds or
  the file is already absent. Failed deletion leaves the entry available for a
  retry.
- `src-tauri/src/sync/mod.rs`: index pending local replacements once for tombstone
  cleanup, replacing repeated scans of all pending writes. Check expiry before
  hashing, and reuse the resulting hash for a purge. The cleanup membership work
  is linear in tombstones plus pending writes. The existing 30-day threshold,
  edit-versus-delete behavior and concurrent-edit protection are preserved.
- `src-tauri/src/sync/worker.rs`: borrow the previous baseline instead of cloning
  it before the sync engine makes its own outcome copy.

No persisted schema, synchronization hash format, dependency or lockfile changed.

## Observed validation

- Baseline Windows Rust library tests: **85 passed**.
- Final `cargo test --manifest-path src-tauri/Cargo.toml --locked` with the external
  target directory: **91 library tests passed**; binary and doc-test targets also
  completed successfully with no tests.
- Six added regressions cover byte-exact large-note serialization, serialization
  failure, interrupted writes and failed replacement cleanup, failed purge/retry,
  the tombstone TTL boundary, and a remote edit replacing an expired tombstone.
- Svelte/TypeScript: **zero errors and zero warnings**.
- Link, formatting, calendar-date, six-language and IPC security scripts passed.
- Vite production frontend build passed with output in the audit directory.
  Existing chunk-size and build-timing warnings remain.
- Final hash audit confirmed that the four intended existing files changed, the
  other original maintained files were preserved, and this report was the only
  added maintained file. All six generated directories are absent from the
  project. All 77 distribution and historical release files remain byte-identical;
  both pre-existing Pinboard processes are still running. Changed files use
  UTF-8 without BOM and LF.

A standalone Rust allocation probe compared the original and final storage
helpers using the same approximately 9 MB JSON value, with an embedded image and
escaped Unicode text. It measured seven writes after warm-up on this Windows
machine. Input allocation was excluded from the extra live-heap measurement;
output files were byte-identical.

| Synthetic JSON save | Original | Buffered streaming |
| --- | ---: | ---: |
| Median extra live heap | 18,017,678 bytes | 8,318 bytes |
| Median elapsed time, including durable write | 12.8932 ms | 8.0438 ms |

These measurements describe this workload, not overall application speed. They
are recorded in `memory-probe-results.json` in the audit directory. The existing
portable application and installers were not rebuilt or deployed in this pass;
the source changes take effect after a new application build. Native UI, Linux
and installer behavior were not exercised by this maintenance pass.
