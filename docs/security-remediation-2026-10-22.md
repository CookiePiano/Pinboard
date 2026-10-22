# Security redistribution — 2026-10-22 KST

## Scope

Version 0.1.1 replaces active Windows/Linux distribution artifacts and the legacy
Linux source archive. The previous Linux archive still used ZIP 2.6.1, unpatched
GLib 0.18.5, unchecked note IDs, and imported synchronization/MCP/startup settings.
The maintained source already contained the corresponding boundary fixes; this
redistribution delivers those fixes rather than duplicating them.

The pre-change source and active artifacts were backed up outside the project at
`C:\build\pinboard-security-fix-20261022\backup` (730 files, with a SHA-256 inventory).
Historical release candidates and verification evidence are retained as historical.

## Changes

- Bump the package, Cargo and Tauri versions together to 0.1.1.
- Update the yanked Tokio 1.54.0 dependency to compatible patch 1.54.1.
- Reject yanked dependencies in the default Windows security gate and Linux build.
  The Linux build also requires cargo-audit and runs npm/Cargo audits.
- Record the complete application/build input set and exact deliverable sizes and
  SHA-256 hashes in `release-manifest.json`. Both release scripts generate/verify
  the receipt. Verification rejects byte drift, source-file-set drift, duplicate
  paths, path traversal and symbolic links.
- Add `-RequireSignedArtifacts` to the Windows release script. It rejects an
  unsigned/invalid executable, installer or standalone remover before any delivery
  replacement. Local unsigned releases remain explicitly identified as such.
- Replace the old Linux source archive with a curated current source snapshot,
  excluding credentials, dependencies, build caches and raw verification logs.

## Validation

Builds and packaging use isolated Windows/WSL directories. Production notes,
authentication, client configuration and existing running application processes
are preserved. Build/test logs remain under the task's external build directory;
the delivered manifest identifies the exact artifact bytes.

| Repeated check | Result |
| --- | --- |
| Windows frontend/security and seven frontend regression scripts | Passed; Svelte reports 0 errors/warnings |
| Windows optimized library suite | 96 passed |
| Windows NSIS build and standalone remover | Built as 0.1.1 |
| Windows isolated installation/removal compatibility | 32 passed; exact packaged payload hashes retained |
| Windows destination validator | 6 passed; no destination created |
| Windows packaged binary MCP smoke | Version 0.1.1, five tools, unprovisioned note access rejected |
| Linux frontend/security and platform regression scripts | Passed; Svelte reports 0 errors/warnings |
| Linux optimized Rust suites | 95 library, 1 binary and 1 GLib integration test passed |
| Debian amd64 package | Built as 0.1.1; native dependencies resolved |
| Extracted Debian binary under Xvfb/private D-Bus | Six startup/relaunch/isolation checks passed |
| Current and archived lockfiles | Zero vulnerability/yanked findings; seven maintenance warnings |
| GLib vendor integrity | 121 upstream files checked; only the reviewed two-line correction differs |
| Manifest negative checks | Changed artifact bytes and traversal paths rejected |

The initial installer check failed because its expected version was hard-coded to
0.1.0. The check now reads the packaged template version; the complete repeated
fixture passed. The failed fixture's temporary registration/data were cleaned.

Tauri patches Linux bundle-type metadata when packaging Debian files. The delivered
standalone Linux executable is therefore taken from the extracted Debian payload,
not the differently tagged intermediate binary, and its bytes match that payload.
The actual packaged executable was used for the startup checks.

The final platform `release-manifest.json` receipts record the exact delivered
files and all 327 application/build source inputs. The regenerated Linux archive
contains the corresponding code, lockfiles and reviewed GLib backport. Production
installation registrations and the two pre-existing Windows app processes were
preserved during the checks. This does not certify every native visual interaction
or replace the scopes of earlier functional audit evidence.

## Remaining limitations

No usable publisher code-signing certificate was present in either Windows
certificate store. Delivered local executables are unsigned; hash receipts do not
authenticate a publisher. A valid publisher certificate and timestamped signing
remain required for a publisher-authenticated public release.

Cargo reports zero vulnerability-category findings and seven transitive
unmaintained-package warnings. These are recorded rather than suppressed. Replacing
GTK3/Tauri's transitive dependency families requires a separate compatible migration;
no incompatible major versions or new dependency forks were forced into this fix.
The reviewed GLib backport remains byte-exact and integrity-checked.

Notes/exports remain plaintext, and explicitly enabled MCP retains its documented
read/write/delete scope. This redistribution does not migrate the storage format
or change existing grants. Native visual behavior beyond the executed checks is
not newly certified by these security/package changes.
