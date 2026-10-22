# Pixel-art icon update

Completed on **2026-10-22 KST**. This supersedes the
[photo icon update](icon-update-2026-10-21.md). The new icon is the user's Aseprite
pixel-art redraw of the same dog-and-memo image (96 x 96, transparent background).

## Inputs

| File | SHA-256 |
| --- | --- |
| `dog_photo_memo_pixel_96.png` | `FB2DA9E1C76768163040B5F7580AE147AEDE001A06AB47BE8FCD1390C4F110ED` |
| `dog_photo_memo_pixel_768.png` (8x nearest-neighbor) | `A00654808A61396EAB8A76D37876E6BAF0A1CDA7E1FB3AE2235C48775C0D8931` |

## Scope

- The 96-pixel image was enlarged 16x with nearest-neighbor sampling to a
  1536-pixel master, then `tauri icon` regenerated the 17 existing desktop files in
  `src-tauri/icons` (PNG sizes, six-frame ICO, ICNS). Android and iOS outputs were
  not added.
- `src-tauri/icons/icon.svg` and `public/pinboard.svg` now embed the 768-pixel PNG.
- `scripts/installer-art.py` regenerated both installer SVGs and BMPs from the
  768-pixel PNG. Layout, colors and text are unchanged.
- The Windows distribution (`Pinboard.exe`, setup and removal executables) was
  rebuilt with `scripts/release.ps1`, build `0.1.1-bd8884ec851f8603`. Linux packages
  and historical releases were not rebuilt.

## Release script changes

- Rust builds remap the user profile, Cargo home and checkout prefixes with
  `--remap-path-prefix`, so dependency source paths no longer embed the developer
  profile. The script fails if `Pinboard.exe` or the removal executable still
  contains it.
- `PINBOARD_UI_PLUGIN_DIR` defaults to `installer/build` during the Tauri build.
  `makensis` keeps an unset `$%PINBOARD_UI_PLUGIN_DIR%` literal, so the default
  branch in `installer/branding.nsh` was never taken and bundling failed with
  `Plugin not found, cannot call PinboardUI::Init`.

## Verification

- `scripts/security-check.ps1` passed within the release script, including 100
  Rust library tests (1 ignored) and the JavaScript suites.
- The release manifest recorded and verified 4 artifacts and 332 source inputs.
- All six ICO frame payloads were found in `Pinboard.exe` and
  `Pinboard_Uninstall.exe`.
- `Pinboard.exe`, the setup and removal executables, and the nine files extracted
  from the installer contain no developer profile path or user name.
- Generated 256-pixel and 32-pixel icons and the installer sidebar were inspected
  visually. The running application UI and tray were not visually verified.

Work files and the pre-change backup are under `C:\build\pinboard-pixel-icon-20261022`.

| File | SHA-256 |
| --- | --- |
| `Pinboard.exe` | `040DAC958D6C31E57DD989E54364E00EE169573B409FC42966B5595D4594D381` |
| `Pinboard_0.1.1_x64-setup.exe` | `D1E8A42873C883ED11A15A9183BB6E0B0AC45BA77A500711FB6A33457C8D7E5B` |
| `Pinboard_Uninstall.exe` | `0EC8944B8061B25A8CC41445826FFAD3894930E2FA5D124BCA9F9C54912DB057` |
