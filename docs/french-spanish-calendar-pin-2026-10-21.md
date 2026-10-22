# French, Spanish, and calendar pin modes

Verified on 2026-10-21 (Asia/Seoul).

## French and Spanish

- Settings > General > Language now offers Français (`fr`) and Español (`es`) in addition to Korean, English, Simplified Chinese, and Japanese. With six languages, the picker is a dropdown instead of a segmented control so it fits the 640px minimum settings width. Language names stay in their own language.
- Frontend catalogs: `src/lib/locales/fr.ts`, `src/lib/locales/es.ts`, and the `fr*`/`es*` panel catalogs in `src/lib/locales/panels.ts` (fonts, calendar, AI collaboration). French uses `vous`; Spanish uses `tú`.
- Native text: `tr!` now requires six literals, `tr!(ko, en; zh, ja; fr, es, args...)`, so a missing translation fails to compile. All 66 call sites were extended. `Lang::Fr`/`Lang::Es` append to the enum, which keeps the stored codes of existing languages unchanged.
- The untouched welcome note, calendar dates (`fr-FR`, `es-ES`), and Windows font family labels (`fr-fr`, `es-es` DirectWrite names) follow the new languages.
- Not translated: the NSIS installer and uninstaller UI, which remain Korean/English as before.

## Calendar pin modes

- The calendar header has a position button with the same Normal / Always on top / Desktop panel as notes.
- The mode is stored per device in `local-state.json` as `calendar_pin` (default `normal`), applied when the calendar opens, and restored when it loses focus. Older local state without the field loads as Normal.
- Commands: `calendar_pin`, `set_calendar_pin`. Escape closes the open panel first, then the calendar.

## Validation

- `npm run check`: zero errors and warnings.
- `node scripts/test-i18n.mjs`: six catalogs complete, no Korean or `undefined` left in non-Korean/English catalogs, live switching and interpolation passed. Link, formatting, and calendar date scripts passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`: 57 passed, including French/Spanish settings round trips, six-language `tr!` formatting, welcome note localization, and `calendar_pin` defaults/round trip.
- Browser harness (Vite, mocked Tauri IPC): French calendar showed the saved mode, opened the localized panel, sent `set_calendar_pin` with `desktop`, and closed on outside click; the Spanish language dropdown listed all six languages and saved the selection. This is frontend verification, not a live native-app session.

## Deleting notes from the calendar

- Each note card in the day list has a checkbox. The trash button next to the date heading is enabled once a card is checked; the first click arms it ("click again to delete N"), the second deletes. Escape or changing the selection disarms it.
- Deletion calls the existing `delete_note` command per note, so it is the same tombstone delete as a note's own delete panel: open note windows close and the deletion syncs to other devices. Changing day or month clears the selection, and notes deleted elsewhere drop out of it.
- Browser harness (Korean, mocked IPC): selecting two of three notes, arming, and confirming sent `delete_note` for exactly those two, and the list and count refreshed to the remaining note.