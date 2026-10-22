# Pinboard 0.1.0 — previous font-settings release

This historical receipt is superseded by [the feature update verification](features-2026-10-21.md). The hashes below describe the earlier 18:16 KST build, not the current delivered files.

Release built on **2026-10-21 at 18:16:29 KST**. This record describes the latest delivered portable app, installer, standalone removal wizard and third-party notices. Earlier build snapshots, executable backups and superseded verification records have been removed.

## Final release validation

The retained final-release receipt records Svelte/TypeScript checks with **zero errors and warnings**, **39 passing Rust tests**, unchanged source inputs during validation/build, and artifact hashes matching their build outputs. The production frontend, native release, NSIS installer and removal wizard built successfully. Vite reported its existing JavaScript chunk-size warning.

Native verification used the delivered executable with separate note data and a separate WebView2 profile. Font settings appeared below General, the AI collaboration menu and note calendar button remained present, and the Windows font collection returned 296 families. Applying Noto Sans KR at 18px/600 updated the note editor and saved the preference. The 760 × 540 CSS viewport was captured; the test process and browser session were closed afterwards.

The cleanup rechecked all four delivered artifact hashes and every source input listed in the retained source receipt. Application tests were not rerun during cleanup. User notes, settings, authentication and the running app were preserved.

## Delivered artifacts

| Artifact | SHA-256 |
| --- | --- |
| `Pinboard.exe` | `04FCBA49CADAEFF7D3E25E34EB533F1A7CACD156C8FE14928CE0FB6C8863514A` |
| `Pinboard_0.1.0_x64-setup.exe` | `8054D9690C0772F7BB6BB8F7133E953964E282F020093C8119BD703DA7A47F94` |
| `Pinboard_Uninstall.exe` | `8B260506A5D5C9A8FF561DDD2849EFE3E754FFA47A84CCCEFA91FCF4941E6346` |
| `THIRD-PARTY-NOTICES.txt` | `6ABDE63307374882ACC9A116FC7029305A90F52EAAE0FD1EF8A1B68A5940E2FA` |

## Retained evidence

- [Artifact receipt](verification/release-artifacts.json)
- [Final verification receipt](verification/release-verification.json)
- [Source input hashes](verification/release-source-inputs.json)
- [Release build log](verification/release-build.txt)
- [Delivered font settings screenshot](verification/released-font-settings.png)

The evidence was relocated from `C:\build\pinboard-fonts-release-20261021-181442` before removing temporary build directories. Historical paths inside the build log identify the original build environment.
