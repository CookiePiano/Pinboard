First public release of Pinboard for **Windows 11 x64**.

- Translucent desktop notes, formatting, backgrounds, calendar and six UI languages.
- Optional cloud-folder sync, ZIP backup/restore and explicit local AI/MCP integration.
- Startup is opt-in. Installation is per-user; uninstall preserves notes by default.
- Validated input/IPC boundaries, locked dependencies and verified Microsoft WebView2 acquisition.

**This initial release is not Authenticode-signed.** Windows may display an unknown
publisher warning. Do not disable Windows security protections. Verify the official
download, GitHub attestations and SHA-256 before deciding to run it.

Choose the x64 setup EXE for installation, or extract the portable ZIP and run
Pinboard.exe. Notes are stored in your Windows application data directory. Notes
and exports are not encrypted. AI connections can read and modify notes only after
you explicitly set them up and enable them.

[Verify downloads](https://github.com/CookiePiano/Pinboard/blob/v0.1.0/docs/download-verification.md) ·
[Privacy](https://github.com/CookiePiano/Pinboard/blob/v0.1.0/docs/privacy.md) ·
[Code signing policy](https://github.com/CookiePiano/Pinboard/blob/v0.1.0/docs/code-signing-policy.md) ·
[Security](https://github.com/CookiePiano/Pinboard/security/policy)

MIT permits modification and redistribution; only this repository's releases are
official CookiePiano builds. Linux source remains available for development but no
Linux binary is supported by this release. Earlier local 0.1.1 files were internal
development builds; do not force them to downgrade. The manifest identifies the
exact source and artifacts. The SPDX SBOM inventories source dependencies.
