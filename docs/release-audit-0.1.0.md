# Initial public release audit

Local checks completed on 2026-10-22 KST for the 0.1.0 Windows x64 candidate.
Historical 0.1.1 executables remain private development artifacts and were not
overwritten. The official release must be rebuilt from the final tagged commit
by the release workflow; this local record does not claim a future CI result.

## Threats and controls

| Threat | Control and remaining boundary |
| --- | --- |
| Unauthorized edits to official source | Owner-only write access, required pull requests and security checks, CODEOWNERS, no force pushes or branch deletion; MFA confirmed by the owner |
| Modified or substituted official downloads | Immutable release assets and tags, GitHub build provenance, source/artifact manifest and SHA-256 checksums; verify the official repository identity |
| Malicious forks or misleading third-party packages | MIT permits modification; identify official downloads explicitly. The maintainer cannot prevent third parties from publishing modified copies |
| Compromised dependencies or CI tools | Locked dependencies, pinned Action commits and downloaded tool hashes, dependency audit, reviewed and expiring maintenance exceptions, least-privilege jobs |
| Replaced WebView2 installer | Private temporary path, Authenticode chain/revocation checks, Microsoft publisher check and a retained file lock |
| Secrets or private notes in source | Exclusions, redacted Gitleaks scan, reviewed public file inventory; no raw local verification captures |
| Release failure overwrites existing binaries | Isolated build/staging output, empty output requirement and fail-closed validation |

## Completed local checks

- Frontend type/Svelte checks: zero errors and warnings; all seven frontend
  regression scripts passed.
- Rust tests: 100 passed, zero failed, one deliberately ignored performance probe.
- IPC permission boundary and byte-exact vendored GLib integrity checks passed.
- npm and Cargo audits: no reported vulnerabilities or yanked crates. Seven
  maintenance warnings are individually recorded with a 2027-01-22 review expiry.
- Gitleaks found no secrets in the scanned source. Manifest tests rejected altered
  payloads, duplicate entries, unsafe paths, missing files and source/version drift.
- Microsoft bootstrapper verification accepted the authentic fixture and rejected
  tampered, unsigned, other-publisher and missing files; the write lock held.
- Installer path checks and 33 installation/removal assertions passed, including
  downgrade rejection, payload hashes, retained notes and preserved existing apps.
- Packaged MCP handshake/tool enumeration passed; unprovisioned and traversal
  access were denied without automatically enabling MCP.
- The isolated native application launched and reported a responding window.
  Interactive editing and visual inspection were not completed in this pass:
  the Windows Computer Use helper failed during initialization. Historical UI
  evidence is separate and must not be represented as testing this exact build.
- Defender completed a non-remediating scan without detections using definition
  1.461.145.0 (updated 2026-10-22 KST). A scan is not a guarantee of harmlessness.
- Workflow syntax passed actionlint 1.7.12.

## Release limitations

The first 0.1.0 release is explicitly unsigned. Windows publisher reputation is
not established by a checksum or GitHub attestation. SignPath application
materials are prepared, not submitted or approved. Signed publication from
0.1.1 onward requires the separately documented signing integration and checks.
Windows Sandbox is unavailable on the local Windows 11 Home host. The official
workflow additionally tests the exact final installer on a disposable GitHub
Windows runner. See [release policy](releasing.md), [signing policy](code-signing-policy.md)
and [download verification](download-verification.md).
