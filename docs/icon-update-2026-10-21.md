# Dog Photo Memo icon update

> Superseded by the [pixel-art icon update](icon-pixel-art-2026-10-22.md).

Completed on **2026-10-21 at 21:34 KST** using the user's replacement image set
from a local `Dog_Photo_Memo` folder, modified at 21:26 KST.
The approved image shows the dog holding a memo while carrying an orange treat.
This supersedes the first icon image applied earlier in this session.

## Scope

- Replaced all 18 desktop icon files in `src-tauri/icons`, including transparent
  PNG sizes, the six-frame Windows ICO, macOS ICNS, and the embedded-image SVG.
- Replaced `public/pinboard.svg`, which supplies the Settings sidebar, About panel,
  and the favicon configured in `index.html`. Existing component geometry is retained.
- Replaced both installer SVGs and their 24-bit BMP exports. The sidebar remains
  328 x 628 and the header 300 x 114; layout, text, colors and typography are unchanged.
- The existing tray uses `default_window_icon`; application windows, shortcuts,
  setup and removal executables use the configured native icon resources.
- Rebuilt and replaced the three Windows distribution executables next to `source`.
  Existing applications were left running. They use the new resources after restart.
- Linux/macOS source assets are updated. Existing Linux packages and historical
  releases were not rebuilt or rewritten.

The 1024-pixel input SHA-256 is
`386CAA8CE9B5CE339FCBCEF9C040C8A95031A6C01CC1D525DA96105CA2E58080`.
The 512-pixel input SHA-256 is
`632BDB489F03DC50A3986A4BBCF29511B8C51DBC49E019C2AE9508EE8ACE39AB`.

## Verification and concurrent work

Work and backups are retained under
`C:\build\pinboard-dog-icon-20261021-2130`. The input PNGs were frozen there before
generation. Assets were generated with the installed Tauri CLI and Inkscape;
System.Drawing converted the installer exports to 24-bit BMPs.

All 23 asset destinations were hash-checked before replacement. Builds used an
isolated source snapshot and separate outputs. Concurrent security/MCP changes
were included in that snapshot; subsequent README/security-review edits were
preserved. No Git staging or commits were performed.

- `scripts/security-check.ps1` passed: npm audit, cargo audit under the existing
  warning policy, the 39-command security contract, Svelte checks with zero errors
  or warnings, four JavaScript suites, and 85 Rust tests.
- The locked Windows release build, NSIS bundle and standalone remover build passed.
- Verified 15 RGBA PNGs, four SVGs containing the exact supplied image bytes,
  both BMP dimensions/depths, the ICNS container and all six ICO frames.
- All six new ICO frame payloads were found in each of the three built executables.
  Delivered file hashes match those verified artifacts.
- Visually inspected the supplied image and both generated installer artwork images.
  The isolated native app launch was rejected by automatic approval review with
  `blocked by policy`; no more specific reason was supplied. Actual application UI,
  tray rendering, and native installer screens were not visually verified here.

See `image-inputs.json`, `icon-changes.json`, `snapshot-inputs.json`,
`delivery-source-inputs.json`, `icon-verification.json`, `security-check.txt`,
`release-build.txt`, and `delivery-result.json` in the work directory for evidence.
Current pre-replacement distribution files are in `distribution-backup`.

## Delivered Windows SHA-256

| File | SHA-256 |
| --- | --- |
| `Pinboard.exe` | `C96A922E687329911C41A75D6CA20C735BF9832F9388B0EF111D93801255D996` |
| `Pinboard_0.1.0_x64-setup.exe` | `4338F55BC1190DC2A0C3B2687D95904CD546F9B9ABE1BD60F7ADBB63B05D177F` |
| `Pinboard_Uninstall.exe` | `71C6A14CEEE6B174A731215FE4B73A7A68552917FFFDC1B9E4064DC089C63FA7` |

The existing third-party notices were byte-identical and retained. Audit warnings,
signing status and platform boundaries remain as documented in `SECURITY.md` and
the security review; this icon update makes no additional security claim.
