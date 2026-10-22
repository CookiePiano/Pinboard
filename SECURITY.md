# Security policy

Pinboard is a local desktop notes application. Use the latest security-reviewed build and
keep Windows, Microsoft Edge WebView2, and your cloud folder client updated.

## Trust boundaries

- Notes, settings, exported ZIPs, and cloud-folder copies are **not encrypted by Pinboard**.
  They inherit the protection of the user account, filesystem, disk encryption, and cloud
  provider. Do not treat Pinboard as a password vault. OAuth credentials stay in the OS keyring.
- Sync folders must be private to trusted users/devices. Anyone able to edit that folder
  can edit or delete synchronized notes. Validation prevents path traversal and malformed
  content; it does not authenticate the author or provide end-to-end encryption.
- MCP is opt-in and local to the PC. Once enabled, connected AI clients can list, read,
  create, update, hide/show, pin, and delete notes; note text may leave this PC through
  that client's service. An old create-only grant is disabled on upgrade until explicitly
  re-enabled for the expanded scope. The AI is instructed to confirm destructive changes,
  but those instructions are not an enforced per-operation approval dialog. MCP files and
  session identifiers are not a boundary against malicious software running as the same user.
- Importing an archive restores notes locally. It never selects the archived network
  destination or changes this PC's MCP permission, startup setting, or account credentials.
  Reconnect a trusted sync folder explicitly after import.
- Notes accept canonical UUID IDs, at most 1 MiB of UTF-8 body text, 20,000 formatting runs,
  and 8 MiB of encoded raster background data. Note JSON reads are capped at 16 MiB;
  archives at 64 MiB uncompressed and 10,000 notes/windows. Invalid local note files remain
  on disk but are not loaded. Invalid sync files stop that pass without treating them as deletions.
- Webviews load bundled pages only. Settings privileges are confined to the settings window.
  Note links can open HTTP/HTTPS in the default browser; they cannot launch arbitrary schemes.
  Navigation rejection is a document-execution boundary, not network isolation: a
  Windows WebView2 navigation probe reached an HTTP server before cancellation,
  while the bundled document remained active and the response script did not run.
  Do not rely on the navigation callback to prevent all outbound requests.

## Release requirements

1. Commit both lockfiles and build with `--locked`; use `npm ci` for a clean dependency install.
2. Run `scripts/security-check.ps1` with `cargo-audit` and `gitleaks` available. `scripts/release.ps1` runs
   this gate before producing deliverables. Scanner/network failures must fail the release.
   Review all informational warnings as well as vulnerabilities. Yanked releases fail the
   default gate; new or expired maintenance notices also block release. The seven reviewed
   advisory/package/version entries are recorded in `scripts/audit-maintenance.json`;
   `-StrictAudit` rejects even these maintenance warnings. The Linux build requires
   `cargo-audit` and runs dependency audits before compiling.
3. The GitHub security workflow runs checks on pushes, pull requests and weekly schedules
   with read-only permissions and pinned action commits. Dependabot checks npm, Cargo and Actions.
   These files take effect only after the repository is published; no remote settings are changed locally.
4. The initial public **0.1.0** release is explicitly unsigned. The release script's
   default policy is Signed, requiring valid signatures, an expected certificate
   thumbprint and timestamps. Only 0.1.0 may use the explicit UnsignedInitial policy.
   GitHub attestations and immutable releases establish the official repository origin;
   SHA-256 alone does not authenticate a publisher. SignPath application preparation
   is not approval. See [Code signing policy](docs/code-signing-policy.md).
   Release builds use isolated output and never replace installed/parent executables.
5. Do not publish credentials, signing keys, app-data folders, raw user backups, or private
   verification logs. `.gitignore` is a safeguard, not a substitute for reviewing the Git index.
   Review historical `docs/verification` records before making the repository public.
6. No automatic updater is implemented. If added later, verify signed update metadata and
   artifacts, enforce HTTPS, and fail closed on verification errors before installation.
7. Keep `release-manifest.json` with each delivered platform bundle. It records the version,
   source identity and exact artifact hashes. Verify it with `scripts/release-manifest.mjs`;
   `--source` also checks the current build-input file set and bytes. This detects drift
   against a trusted receipt; it does not authenticate an attacker-replaced receipt.
8. Scan final executable files and archives with current Defender definitions. A detection,
   unavailable scanner, stale definitions or scan error blocks publication. Keep private
   scan evidence. Do not submit files to external scanners without an explicit decision.
9. The installer verifies the Microsoft WebView2 bootstrapper using Windows trust and
   signer checks, retains a non-write/non-delete-shared handle through execution, and
   uses a private temporary path. Login startup and MCP remain opt-in.

## Official releases and modified copies

MIT permits third-party forks, changes and redistribution. Official repository write
and release access belongs to CookiePiano only. Main requires a PR and successful
checks; external PRs are reviewed by the maintainer and have no release credentials.
Immutable releases prevent asset/tag replacement. Verify both the official repository
and the specific workflow provenance; a hash or signature is not a malware guarantee.
See [download verification](docs/download-verification.md) and [privacy](docs/privacy.md).

## Linux dependency backport and remaining advisories

GLib 0.18.5 in Tauri's GTK3 dependency graph has
[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html): an immutable
out-pointer passed to C produces undefined behavior in string iteration. The
2026-10-22 optimized upstream reproduction crashed with SIGSEGV. Pinboard now
vendors the original crate with the exact two-line upstream correction, preserving
the GTK3-compatible API/ABI. Its 11 optimized iterator tests and the application's
optimized Linux regression pass. The [backport guide](src-tauri/vendor/README.md)
records the source, checksums, patch and required validation.

The integrity guard verifies all 121 original files after reversing only the
reviewed correction in memory. Security checks and license generation run that
guard; the Linux build also runs the optimized application regression. A scan of
8,672 Linux-resolved Rust source files found iterator references only inside GLib
itself, including its tests/docs. This is not proof of whole-program unreachability;
the fix removes the unsound implementation regardless of its current callers.

The current Cargo scan reports zero vulnerability-category findings and seven
transitive unmaintained-package warnings. Cargo audit does not report this local
GLib path dependency, so the reduced warning count is not evidence of remediation
by itself. Do not add advisory ignores or force an incompatible GLib major into
GTK. Review the remaining maintenance warnings and run native Linux/package checks
before distribution; passing Windows checks does not establish Linux behavior.

## Reporting

Once hosted on GitHub, enable **Private vulnerability reporting** under repository security
settings and accept reports through the repository's **Security > Report a vulnerability**
entry. Until a private channel is enabled, contact the maintainer to arrange one; do not post
working exploits, credentials, or personal note contents in public issues.

Include the app version, OS, installation type, reproduction steps using synthetic notes,
and the observed impact. See [the security review](docs/security-review-2026-10-21.md)
for the scope and validation of the current changes.
