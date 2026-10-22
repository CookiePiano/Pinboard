# Documentation and core refactoring — 2026-10-21 KST

## Scope

Nine maintained documents containing Korean prose were rewritten or normalized to
English. README now separates behavior contracts, development commands, source
navigation and data semantics. `AGENTS.md` adds a project entry point with
concurrent-editing safeguards and focused validation commands. Historical dates,
test counts, paths and identifiers were retained. The exact Korean acceptance
string in `followup-2026-10-21.md` and the real Drive mount path in
`transfer-2026-10-21.md` remain literal evidence.

Three implementation modules changed, without altering their public interfaces:

- `src/lib/note/links.ts`: count bracket balance once, then trim the suffix with a
  backward scan and one slice. This removes repeated full-string splitting on
  long unmatched-bracket suffixes and keeps balanced URL brackets.
- `src/lib/note/formatting.ts`: share ordered-run coalescing between normalization
  and range formatting. Compare style fields directly, avoid JSON serialization
  for equality, and avoid re-sorting an already ordered result.
- `src-tauri/src/model.rs`: stream optional-field JSON into SHA-256 instead of
  allocating serialized buffers proportional to embedded images. Preserve field
  order, delimiters and legacy hashes. No persisted schema or dependencies changed
  as part of this refactoring.

## Concurrent changes retained

The source repository had no committed file baseline when work began. Backups,
hash manifests and an isolated validation snapshot were created under
`C:\build\pinboard-codex-refactor-20261021`.

During the task, another session added automatic MCP client registration and
modified README, Cargo inputs, native registration, AI settings and translations.
Those changes were imported into the validation snapshot. The README translation
was reconciled with the latest manual and automatic connection workflows and
calendar deletion behavior. The AI collaboration guide was updated accordingly.
Only this task's explicitly scoped files were written back, after checking their
expected hashes. Other changes were not reverted, staged or committed.

## Observed validation

- Integrated Windows Rust library tests: **63 passed**, including the new hash
  compatibility regression with Unicode, formatting, font preferences, tombstones
  and a 1 MiB embedded-image string.
- Svelte/TypeScript: **zero errors and zero warnings**.
- Link, formatting, calendar-date and six-language regression scripts passed.
- Differential comparison against the pre-refactor implementation: **10,000 URL
  cases and 3,000 formatting cases** produced identical results.
- Integrated Vite production frontend build passed. The existing warning about
  chunks larger than 500 kB remains.

Synthetic microbenchmarks used Node v24.13.1 on Windows x64. Values are the median
of seven runs after a warm-up; they measure these inputs only, not overall app speed.

| Operation | Before | After |
| --- | ---: | ---: |
| Trim a URL with 6,000 trailing bracket/punctuation characters | 88.110 ms | 0.041 ms |
| Normalize 10,000 adjacent format runs | 2.363 ms | 1.148 ms |
| Apply italic across 10,000 format runs | 6.015 ms | 2.379 ms |

Logs, comparison code/results, source manifests and original-file backups are in
the external audit directory above. This pass did not run native UI automation,
Linux builds or installer deployment. Shared executables and release outputs were
not replaced by this task; the running application needs a later coordinated build
and restart to use the refactored code.
