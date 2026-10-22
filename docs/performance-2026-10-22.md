# Pinboard performance and memory

Verified on **2026-10-22 KST**. Image-bearing note saves and design selection now
transfer substantially less data. Background snapshots share immutable note
content and cached hashes. These changes reduce transient copying; the measured
idle application footprint did **not** decrease.

## Implementation

- `SharedNote` owns immutable note content through `Arc` and caches the exact
  existing synchronization hash in `OnceLock`. Sync and export snapshots share
  those entries. Replacement creates a new entry; an in-flight snapshot retains
  the previous content. Serialization delegates to `Note`, without cache fields.
- Content saves use `save_note_content`, returning only `updatedAt`. Durable
  content writes run off the UI thread. The originating editor receives no full
  note echo. External edits still update its note window. Existing `update_note`
  and `design_context` commands retain their serialized response contracts.
- `design_note_summaries` omits bodies, format arrays and images.
  `get_note_design` loads the selected image. Design changes survive panel
  switches in per-note save queues and participate in settings/editor flushes
  before export, restore and exit.
- Windows and settings panels load their own components on demand. Settings and
  calendar windows no longer initialize CodeMirror. Font enumeration shares
  concurrent scans, caches successful results per language for five minutes and
  supports an explicit forced refresh.
- Editors queue immutable CodeMirror states and materialize text when saving.
  Ordered format mapping avoids sorting, preserves unchanged array identity and
  retains existing decoration trees where possible. Selection measurements are
  coalesced to one animation frame. Opacity writes retain the latest pending
  value, are serialized and participate in forced flush/retry.
- Calendar changes trigger a debounced refresh, with a 30-second visible-window
  fallback. Updates arriving during a refresh schedule another pass. MCP inbox
  watching replaces 100ms polling, with a bounded hint channel and one-second
  fallback; the existing heartbeat, grants, request limits and cleanup remain.

Note JSON, hash input bytes/order, UTF-16 formatting positions, conflict rules,
the 30-day tombstone policy and the ZIP format remain compatible. Hidden note
windows retain their editor, undo history, selection and scroll state. Folder
sync still validates remote files and keeps its existing fallback polling.

## Measured results

Tests use task-owned data and WebView2 profiles. IPC trials compare the copied
pre-change release build with the integrated candidate on this Windows machine.
The fixture contains ten real PNG backgrounds (about 1.5 MiB of encoded data
each) and one 1 MiB note with 20,000 format ranges. Times are medians of five
calls, including native processing and frontend response decoding.

| Operation | Before | Candidate |
| --- | ---: | ---: |
| Design list response | 17,557,125 bytes | 1,276 bytes |
| Design list latency | 173.6 ms | 1.0 ms |
| Image-note content save response | 1,574,427 bytes | 27 bytes |
| Image-note content save latency | 311.6 ms | 5.5 ms |
| JavaScript requested by initial General settings | 717,876 bytes | 154,290 bytes |

A separate optimized Rust probe compares the former deep snapshot plus fresh
hashing with shared snapshots plus warmed cached hashes. Ten 1.5 MiB embedded
image strings produced median times of **13.63 ms** and **0.0075 ms**, respectively,
over five trials. Pointer-identity and serialization tests establish sharing and
byte-compatible note output. This is not a full synchronization or image decode
benchmark.

An indicative burst of 100 programmatic CodeMirror edits in the 1 MiB/20,000-run
note measured p50 **43.1 → 39.5 ms** and p95 **68.0 → 58.3 ms**. This single burst
does not establish physical typing latency or a general editor speedup. The
extreme document still spends substantial time in the editor/rendering path.

### Idle memory

The following are median aggregate **Private Bytes**, in MiB, across the native
application and its WebView2 process tree. Each case uses five fresh profiles,
eight seconds of startup settling and three samples. All note windows are
hidden through the test-only startup flag; these are not visible-window or
long-duration leak measurements. The final collector excludes processes created
before its application root to avoid counting unrelated reused parent PIDs.

| Fixture | Before | Candidate | Median change |
| --- | ---: | ---: | ---: |
| 1 note window | 189.2 | 189.5 | +0.2% |
| 10 note windows | 651.1 | 657.2 | +0.9% |
| 30 note windows | 1,663.4 | 1,706.2 | +2.6% |
| 1,000 stored notes, 10 windows | 637.5 | 657.6 | +3.2% |
| Large/image fixture, 11 windows | 1,076.2 | 1,076.5 | approximately unchanged |

The short, cold-profile samples vary between launches. They show no idle memory
reduction and do not support a memory-leak conclusion. Keeping every hidden
WebView/editor alive preserves the requested interaction state and leaves a
substantial runtime floor. Snapshot sharing removes content-sized transient
copies, rather than eliminating that per-window cost.

## Verification

- Final Windows optimized Rust library suite: **100 passed**, one explicitly
  ignored performance probe. The performance probe ran separately and passed.
- WSL Ubuntu Rust library suite: **99 passed**, one ignored probe. Optimized GLib
  FFI regression: **1 passed**. This is WSL verification, not native Ubuntu UI.
- Svelte/TypeScript: zero errors and warnings. The nine standalone regression
  scripts pass, including 3,000 differential formatting cases and lazy-save
  failure/retry tests. Linux's Vite-based TypeScript runner also passes.
- Native candidates cover Unicode persistence, format undo/redo, undo after
  hide/reopen, failed-save visibility, hide/export rejection and latest-edit
  retry. Final hidden-window checks repeat content persistence, export rejection,
  retry, content-save ACL, font refresh/cache and ZIP operations.
- Design opacity survives a panel switch and forced export. ZIP import restores
  the exported content and retains this PC's explicit MCP setup/disabled state.
  Forty MCP requests, including work beyond the 32-request batch, complete.
- Calendar date counts reflect the last deletion by the 400ms observation
  without refocusing the calendar. Polling's 30-second interval is
  verified from source; an attempted IPC-call counter was invalid because the
  native invoke binding cannot be replaced and is not used as evidence.
- The production frontend builds. CodeMirror's note chunk still exceeds Vite's
  500 kB advisory threshold; the warning has not been suppressed.

Native keyboard composition through the Windows IME, long-duration leak testing,
visible-window memory scaling, Linux GUI, real external cloud delivery and
installer deployment are outside these measurements.

One visible UI rerun received input while it held desktop focus. That rerun was
discarded; the interrupted note was copied to
`C:\build\pinboard-performance-20261022\interrupted-editor-preserved.json`.
Subsequent native checks used hidden windows without explicit desktop focus.

## Evidence and source ownership

Backups, source hashes, diffs, build/test logs, native harnesses and raw results
are retained under `C:\build\pinboard-performance-20261022`. Relevant receipts
include `baseline-ipc.json`, `verified-ipc.json`, `snapshot-benchmark.log`,
`memory-summary.json`, `verified-memory.json`, `ui-regression-verified.json`,
`native-design-mcp-transfer-verified.json` and `calendar-verified.json`.
Blank-page builds, early collectors and probes using stale browser tab mappings
are excluded from the reported measurements.

The repository has no committed baseline. Concurrent MCP-client/provider,
translation/icon and license-metadata edits were preserved and appear in the
integrated validation tree; they are not changes attributed to this refactor.
The final executable is an isolated verification artifact, not a replacement of
the portable app or installer. No production notes, credentials or external AI
client configurations were used as fixtures or modified by this task.
