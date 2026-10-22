# Code signing policy

## Current status

Version 0.1.0 is intentionally unsigned. SHA-256 receipts detect changes relative
to a trusted receipt; they are not publisher certificates or malware guarantees.
Official downloads are exclusively linked from
https://github.com/CookiePiano/Pinboard/releases . GitHub build and release
attestations provide independently verifiable repository provenance.

The maintainer-requested `v0.1.0-r2` correction retains application version 0.1.0
and the same unsigned exception. It is a separate immutable distribution with its
own tag, hashes and attestations. The exception does not cover later app versions.

## Responsible people

- Author/committer: [CookiePiano](https://github.com/CookiePiano)
- Reviewer of external contributions: CookiePiano
- Release and future signing approver: CookiePiano

The maintainer uses MFA. External contributors do not receive repository write,
release, signing or secret access. Changes go through PRs and required checks.
The project has one maintainer; it does not claim independent two-person review.

## Planned SignPath application

We intend to apply to [SignPath Foundation](https://signpath.org/terms) after the
first public release. Application preparation does not imply approval, a
certificate, sponsorship or an entitlement to service. No SignPath token or
private key is committed, and no signing request is submitted by the initial CI.

After approval, publish the required provider attribution, approved project
identity and certificate verification information here. Each signing request must
be manually approved by the maintainer. Restrict signing to reviewed source built
by the official GitHub workflow, and verify timestamps and the expected signer on
the app, installers, uninstallers and project-owned native DLLs. Never re-sign
third-party binaries as if they were Pinboard's own code.

Signed distributions will use version 0.1.1 or later. Existing immutable release
assets will not be replaced. See [Privacy](privacy.md) and [Releasing](releasing.md).
