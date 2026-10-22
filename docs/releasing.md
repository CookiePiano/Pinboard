# Release process

## Supported initial release

The maintainer uses the verified **v0.1.0-r2** distribution as the initial public
release, presented as **Pinboard 0.1.0** for Windows 11 x64 under MIT. Application,
installer and package versions remain **0.1.0**. The immutable tag identifies the
exact tested files and their provenance. The unsigned 0.1.0 exception applies;
later application versions still require signing.

The earlier GitHub **v0.1.0** release was removed at the maintainer's request.
Its existing Git tag and historical verification records are preserved; the
current public download is **v0.1.0-r2**.

Local historical 0.1.1 builds were development artifacts. Do not force an existing
higher-version installation to downgrade. Preserve user data and historical
verification records.

The maintainer verifies all public files, licenses, secrets scanning and tests
before the first source push. Protect main with required PRs, security-windows and
resolved conversations; disable force pushes and branch deletion. With one
maintainer, PR approvals are not mandatory, but external changes are reviewed.
Use CODEOWNERS for visibility. Restrict version-tag creation to administrators,
and deny updates/deletion of version tags. Enable private vulnerability reporting,
Dependabot, secret scanning/push protection and immutable releases.

## Build gates

`scripts/release.ps1` requires an isolated TargetDirectory and an empty
OutputDirectory. It never overwrites existing distribution files. The default
SigningPolicy is Signed and requires an expected signer thumbprint plus valid
timestamped signatures. UnsignedInitial is an explicit exception for 0.1.0 only.

The script provisions SHA-256-pinned tools, audits dependencies, scans secrets,
runs frontend/Rust regressions, builds with locked dependencies, tests signature
validation, installers and packaged MCP, produces the portable ZIP and SPDX SBOM,
and scans the final files with updated Defender definitions without remediation.
Any failure blocks release output. Download and scanner failures are failures.
The disposable hosted Windows image disables Defender features and excludes its
drives by default. `prepare-ci-defender.ps1` restores scanning only on that hosted
runner, clears exclusions and disables automatic sample submission. Security CI
tests scanner readiness before tagging; the release scans the final payloads and
requires archive scanning and a non-excluded artifact directory. Developer PC
security settings are never changed by these scripts.
The SPDX file inventories source dependencies, including development and other-OS
lockfile entries; it is not an assertion that all entries ship in the Windows EXE.

Known maintenance notices are recorded by advisory/package/version in
scripts/audit-maintenance.json with a review deadline. No vulnerability or yanked
crate is exempt. New notices require review. Keep the GLib vendor integrity guard.

## Official publication

1. Commit the tested source and lockfiles. Wait for security-windows on that commit.
2. Create v0.1.0-r2 at that exact main commit. Do not move an existing tag.
3. CookiePiano dispatches Release Windows from main for v0.1.0-r2. PRs and forks
   cannot publish. The workflow checks the tag, ancestor and successful check.
4. The GitHub-hosted Windows job rebuilds the release and installs the exact final
   setup, including same-version reinstallation and data preservation, in a
   disposable environment. Attestation runs with narrowly scoped
   permissions; publication permissions exist only in the final draft job.
5. Download the draft's exact files, verify hashes, manifests and build attestations.
   Run the exact downloaded app with isolated synthetic data and a separate
   WebView2 profile. Verify settings X closes, can reopen, and preserves pending
   edits; save errors must keep settings open. Publish the completed draft only
   after all checks pass. Immutable releases lock assets and the tag; changes
   require a new release tag. Never silently replace a published download.
6. Download the public assets again and verify the release attestation and build
   attestation against CookiePiano/Pinboard and the release workflow.

The five public assets are the setup EXE, portable ZIP, release-manifest.json,
SHA256SUMS.txt and sbom.spdx.json. Keep build provenance for each. Never upload
developer executables, app data, tokens, raw verification logs or signing keys.

If a public release is suspected to be compromised, mark it unsafe in the release
notes, warn users through the official repository, investigate the originating
commit/workflow and publish a corrected version. Do not silently replace files.
Revoke affected credentials/certificates only after establishing their exposure.
