# Security review — 2026-10-21 (KST)

## Scope and result

Reviewed Pinboard's Windows desktop source, file and cloud-folder synchronization,
ZIP import/export, Tauri IPC permissions and navigation, the local MCP bridge,
legacy Google OAuth callback, dependency lockfiles, and release automation.
The review used synthetic data in a separate source/build directory and WebView2
profile. No real notes, OAuth credentials, external AI configuration files, or
running user applications were changed by the verification.

Claude CLI was editing this repository concurrently. Security edits were backed up
outside the repository and applied with per-file hash checks. Calendar deletion,
note-hash compatibility, AI-client setup, expanded MCP tools, localization and
branding changes were preserved. No files were staged or committed. Deliverables
are in a separate security release directory; the parent portable executable was
not replaced by this review.

This is a scoped source review and adversarial regression check, not proof that
the application has no vulnerabilities. Windows results do not establish Linux
safety. Public release requirements and reporting guidance are in [SECURITY.md](../SECURITY.md).

## Findings and changes

| Finding | Preconditions and impact | Resolution |
| --- | --- | --- |
| Untrusted note IDs used in filesystem operations — high | An attacker able to supply synchronized note JSON could substitute a filename/path or another note ID. This could affect files outside the intended note path or corrupt another note. | Validate canonical UUIDs at storage and sync boundaries; match payload IDs to requested/file IDs; reject symlink and oversized note files. Invalid sync input aborts the pass instead of becoming an apparent deletion. |
| Backup could select a sync destination and restore local permissions — high | A user opening an untrusted backup could unknowingly activate its folder destination or MCP/startup choices. | Restore notes locally with sync off and no folder destination. Preserve this PC's actual MCP, startup and credential state. Do not probe an archived network path. Display a reconnect warning. |
| Native privileges were shared across webviews — defense in depth | Compromised note content/UI would have had the same native commands as settings. No working script-injection exploit was established. | Explicitly register all 39 app commands for Tauri ACL enforcement. Grant account, backup, startup and MCP configuration commands only to settings. Restrict link opening to HTTP/HTTPS. Deny external in-app navigation and new windows; strengthen CSP. |
| Unbounded or excessive imported/synchronized data — medium | A malicious archive or sync file could consume excessive memory or CPU. | Cap actual JSON reads, compressed and decompressed backup sizes, note/window counts, body bytes, format counts and background payloads. Use capped streaming export. Validate format bounds, font values and numeric ranges. |
| MCP permission expansion reused an earlier grant — high for existing opt-in users | The concurrent feature change adds read, update and delete to a formerly create-only bridge. Reusing the older enabled flag would silently expand consent and expose existing note text. | Persist a versioned grant. Earlier grants start disabled until explicitly re-enabled. Keep all operations behind the live permission/session/expiry checks. Mark replacement and deletion as destructive in tool annotations. Bound titles/previews so the maximum list fits the reply limit. |
| OAuth callback parsing and cancellation — medium, legacy path | A local process could send malformed or unrelated callback requests, cancel login or hold a connection open. The OAuth UI is currently retired. | Validate GET/path/state and duplicate parameters before accepting either success or error. Ignore unauthenticated requests; enforce a global deadline and per-connection timeouts. Set no-store and restrictive response headers. Keep outbound token/API requests HTTPS-only without redirects. |
| Predictable temporary file writes and sensitive parser errors — defense in depth | Shared writable locations could expose or interfere with staged files; parser errors could repeat config contents. | Use unique create-new atomic files, restrictive Unix permissions and cleanup. Apply the same writer to AI configuration backups/replacements; return generic parse errors. Same-user malware remains outside the bridge's security boundary. |
| Vulnerable ZIP dependency and missing release audit — dependency risk | `zip 2.6.1` had three vulnerability-category advisories. The flagged extraction APIs were not used by Pinboard, but retaining that dependency increased risk. | Upgrade to `zip 9.0.5`, update the lockfile/notices, add local release checks and a pinned, read-only GitHub workflow, and enable weekly Dependabot configuration. |

## Boundaries retained

- Notes and exported backups remain plaintext. Disk encryption and account/cloud
  access controls are needed for confidentiality; Pinboard is not a secrets vault.
- A writable sync folder is trusted input for authorized edits/deletions. Validation
  does not authenticate the writer or provide end-to-end encryption.
- When explicitly enabled, MCP can read and modify existing notes. Connected AI
  services may receive note text. Client selection is not an authenticated allowlist.
  Prompt instructions to confirm deletion/replacement are guidance to the client,
  not an enforced approval dialog for every operation.
- Loopback WebView2 debugging was enabled only for the isolated test process.
  Normal MCP uses local stdio/files and does not open a network listener.
- No automatic updater exists. Any future updater needs independent signed-update
  verification before installation.

## Validation

- `npm audit --audit-level=low`: zero findings.
- `cargo audit`: zero vulnerability-category findings after the ZIP update; eight
  informational entries remain, detailed below.
- Rust library regression suite: **85 passed**, including malicious IDs, mismatched
  sync payloads, poisoned sync folders, ZIP size bombs, import permission isolation,
  hostile OAuth callbacks, expired/disabled MCP requests, grant migration and bounded replies.
- Svelte/TypeScript check, link, formatting, calendar-date and localization checks,
  and the 39-command security contract were run in the isolated build snapshot.
- Native WebView2 checks exercised text rendering, note/settings/calendar behavior,
  denied settings privileges from a note window, rejected file/JavaScript URL opens,
  blocked external navigation/popups, and malicious archive import with synthetic notes.
  Imported settings did not activate folder sync or MCP.
- The final release binary was also exercised through its real MCP stdio transport:
  all five tools were denied with an old grant; explicit enablement allowed
  list/read/create/update/delete; deleted notes were excluded from subsequent reads.
- Release executable and NSIS installer were built with the locked dependencies.
  Installer compilation does not establish install/uninstall behavior on a clean PC;
  no installer was executed against the user's installation during this review.
- The GitHub workflow and Dependabot configuration have not run on GitHub. They
  require publication to the repository. Real cloud OAuth and multi-PC synchronization
  were not exercised; protocol and sync behavior were tested locally.

The separate release directory contains exact artifact hashes, the source-file
manifest and build notes. Later concurrent source edits require their own checks;
the manifest identifies the snapshot covered by the delivered executable.

## Outstanding release work

**Windows signing:** the local EXE and NSIS artifacts are unsigned. Obtain the
publisher's signing certificate/service, sign and timestamp the executable and
installer, verify signatures, and regenerate checksums after signing. Signing was
not attempted without publisher credentials. A SHA-256 checksum is integrity
evidence, not publisher authentication.

**Linux dependency issue:** the full Cargo lockfile still includes the GLib
`VariantStrIter` unsoundness in `glib 0.18.5`
([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)). Seven
additional maintenance entries cover `proc-macro-error`, `zbus_names`, `zvariant`
and `zvariant_derive` versions. All eight flagged package/version entries were
absent from the Windows dependency graph. The Linux GTK path needs an upstream-
compatible fix/reachability review before Linux distribution; this review does
not approve a Linux release. Do not suppress advisories or force incompatible
major versions into Tauri's GTK stack.

**Repository publication:** review the Git index and historical verification
artifacts for private notes, paths and credentials. Ignore patterns and a clean
pattern scan do not guarantee that a public repository contains no private data.
Enable GitHub private vulnerability reporting after creating the repository.

## References

- [Tauri capabilities](https://v2.tauri.app/security/capabilities/) and
  [content security policy](https://v2.tauri.app/security/csp/).
- [RustSec advisory database](https://rustsec.org/) and
  [ZIP crate documentation](https://docs.rs/zip/latest/zip/).
- ZIP update addressed advisory IDs `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`,
  and `RUSTSEC-2026-0369` reported by the local Cargo audit database.
- [MCP permissions and tool contract](ai-collaboration.md).
