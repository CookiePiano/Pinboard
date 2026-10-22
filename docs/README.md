# Documentation index

## For users

- [한국어 사용 가이드](user-guide.md)
- [English user guide](user-guide.en.md)
- [简体中文使用指南](user-guide.zh.md)
- [日本語の使い方ガイド](user-guide.ja.md)
- [Guide d’utilisation en français](user-guide.fr.md)
- [Guía de uso en español](user-guide.es.md)
- [Download Pinboard](https://github.com/CookiePiano/Pinboard/releases)

## For developers

1. Read [the agent guide](../AGENTS.md) for workspace safeguards and required checks.
2. Read [the technical reference](technical-reference.md) for supported behavior and data invariants.
3. Locate the feature owner below before editing. Run the matching tests and the required repository checks.
4. Use a dated receipt only as evidence for its named snapshot. Verify current source and binary hashes independently.

The README and user guides cover Korean, English, Simplified Chinese, Japanese, French and Spanish, with Korean as the default. The language links at the top of each README lead to the corresponding introduction; each guide links back to its own language's README. Developer documentation is maintained in English. Korean UI acceptance strings, literal filesystem paths, and user-content examples remain unchanged where they are part of a contract or historical evidence. Developer home directories in recorded paths are written as `<user>` placeholders. Application translations are product data and must not be converted to English-only catalogs.

## Current guides

| Need | Guide | Main source owner |
| --- | --- | --- |
| Behavior, storage and development | [Technical reference](technical-reference.md) | `src-tauri/src/lib.rs`, `model.rs`, `store.rs` |
| Editing, formatting and save ordering | [Application behavior](technical-reference.md#behavior-and-invariants) | `src/lib/note/`, especially `editor.ts`, `formatting.ts`, `save-queue.ts` |
| Cloud folder synchronization | [Cloud folder setup](google-drive-setup.md) | `src-tauri/src/sync/` |
| AI connection and tool contract | [AI collaboration](ai-collaboration.md) | `src-tauri/src/mcp/`, `src/lib/settings/AiPanel.svelte` |
| Fonts | [Font settings](font-settings.md) | `src-tauri/src/fonts.rs`, `src/lib/fonts.ts` |
| Windows packaging and removal | [Installer guide](../installer/README.md) | `installer/`, `scripts/build-installers.ps1` |
| Linux build and platform limits | [Ubuntu guide](linux-ubuntu-26.04.md) | `src-tauri/src/platform/linux.rs`, Linux scripts |
| Security model | [Security policy](../SECURITY.md) | Capabilities, validation, transfer and MCP modules |
| Official release controls | [Release guide](releasing.md) | `.github/workflows/`, `scripts/release.ps1` |
| Download authenticity | [Verification guide](download-verification.md) | Release attestations, checksums and receipt |
| Signing exception and future signing | [Signing policy](code-signing-policy.md) | 0.1.0 exception; [SignPath preparation](signpath-application.md) |
| Local data and network behavior | [Privacy policy](privacy.md) | Local storage, opt-in sync and MCP |
| GTK3-compatible GLib fix | [Backport provenance](../src-tauri/vendor/README.md) | `src-tauri/vendor/`, integrity guard and optimized Linux regression |
| Redistribution notices | [License guide](../licenses/README.md) | Lockfiles and `scripts/generate-notices.mjs` |

## Active verification

[Settings close, 2026-10-22](settings-close-2026-10-22.md) records the v0.1.0
close-permission failure, the settings-only fix and local runtime checks for
successful saves, write failures and retries. Existing release assets are unchanged.

[Performance and memory, 2026-10-22](performance-2026-10-22.md) records shared
snapshots, smaller IPC payloads, window/panel loading and background scheduling.
Its idle-memory results are separate from transient-copy and latency improvements.

[Security redistribution, 2026-10-22](security-remediation-2026-10-22.md) records the
0.1.1 artifact replacement, dependency checks and remaining signing limitation.

[Full application audit, 2026-10-22](full-audit-2026-10-22.md) tracks the requested all-feature validation, fixes and refactoring. Its pending rows are not claims of success. The audit uses isolated note data and build output; user notes, credentials and running production applications are outside the test fixtures.

## Historical evidence

Files named with a date record the scope and limitations of that pass. Do not merge their counts into a current test total, or infer that a historical binary contains later source changes.

- [Core refactoring](refactoring-2026-10-21.md) and [storage maintenance](maintenance-2026-10-21.md).
- [Feature release](features-2026-10-21.md), [calendar/MCP follow-up](followup-2026-10-21.md), [ZIP transfer](transfer-2026-10-21.md), and [MCP client connection](mcp-one-click-2026-10-21.md).
- [Security review](security-review-2026-10-21.md), [Linux verification](linux-verification-2026-10-21.md), and [Linux integration](linux-integration-2026-10-21.md).
- [Pixel-art icon update](icon-pixel-art-2026-10-22.md), superseding the [photo icon update](icon-update-2026-10-21.md).
- Native installer evidence is retained privately under `verification/installer-2026-10-22/`; raw local captures are excluded from the public source.

Other dated feature records and their `verification/` receipts remain historical source material. Preserve their dates, limitations, identifiers, hashes and original paths when restructuring prose.
