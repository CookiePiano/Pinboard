# SignPath Foundation application preparation

Status: preparation only. No application has been submitted and no approval or
certificate is claimed. Submit only after the initial public binary release.

- Project: Pinboard, desktop sticky notes for Windows 11 x64.
- Source: https://github.com/CookiePiano/Pinboard
- License: MIT, with preserved third-party notices and source dependency SBOM.
- Initial release: https://github.com/CookiePiano/Pinboard/releases/tag/v0.1.0
- Owner, author, reviewer and proposed signing approver: CookiePiano.
- Maintainer MFA: confirmed by the maintainer on 2026-10-22 KST.
- Signing policy: docs/code-signing-policy.md; privacy policy: docs/privacy.md.
- Build: official GitHub-hosted Windows workflow, pinned dependencies/tool downloads,
  source commit/tag receipt, required checks, artifact attestation and immutable release.
- Own signing targets: Pinboard.exe, setup EXE, installed uninstaller, standalone
  remover, PinboardUI.dll and PinboardVerify.dll. Do not re-sign upstream binaries.

The submission should include the real successful release/workflow URLs and
explain project activity and users honestly. The foundation requires a released,
maintained open-source project and reviews its reputation. Approval and timing are
not guaranteed: https://signpath.org/terms

After approval, configure the approved organization/project/policy identifiers,
restricted CI token, artifact metadata constraints and manual signing approval.
Review NSIS's embedded EXE/uninstaller signing sequence before enabling signing;
signing only the outer installer is insufficient. Test the actual installed files.
Publish the required provider attribution only when service is actually granted.
Use a new release version for signed artifacts, preserving immutable 0.1.0.
