# Settings close verification — 2026-10-22 KST

## Failure and fix

The published Windows v0.1.0 settings window stays open after clicking X.
Reproduced using the official portable executable, SHA-256
`6fedaa07f616f3ba48dc85636af3090cd44af30e364782ed164e1bb6111861c0`,
with isolated synthetic note data and a separate WebView2 profile.
This was a local Windows test, not a test on the reporter's second PC.

The settings close handler flushes design edits, then accepts a second close
request. Tauri's `onCloseRequested` wrapper completes that accepted request by
calling `destroy()`. The shared capability allowed `close`, but no capability
allowed `destroy`. The actual rejection was:

```text
Command plugin:window|destroy not allowed by ACL
```

Add `core:window:allow-destroy` only to the local `settings` capability. Note and
calendar windows do not receive it. The save-before-close handler is unchanged.
The security contract test now requires this settings grant and rejects granting
it to other windows. See [the capability](../src-tauri/capabilities/settings.json)
and [the guard](../scripts/test-security.mjs).

## Runtime checks

A local optimized build from the preceding main commit
`2e73e77996a506e8eb8c54d3c0b45f34e029f3c0` plus the capability fix passed:

- X closes the settings window; the note window and app remain running.
- Settings opens again and closes on subsequent requests.
- Attaching a synthetic image, changing its transparency and requesting close
  immediately persists the pending edit before the window disappears.
- Making the isolated note file read-only causes a visible save error and keeps
  settings open. Restoring write access and pressing X retries the save and closes
  settings. The requested transparency and original note text are retained.

The tested executable SHA-256 is
`da6d494f0dd7f71909e1dc579e27a0e5e74b94ce8815b76f947f671d08ba9462`.
It embeds the production frontend and uses the release Cargo profile with locked
dependencies. It is a local validation build, not an official downloadable release
or an installed-setup test. No production notes or account connections were used.

Local Svelte checking reported zero errors and warnings. Security, six-language,
save queue, links, formatting, calendar dates, shortcut and drag checks passed.
GitHub checks on the associated PR provide the full Windows regression result.

## Existing downloads

The immutable v0.1.0 installer and ZIP are unchanged and still have the issue.
As a temporary workaround, minimize settings, or use the notification-area
**Quit Pinboard** command and reopen the app.

The correction is distributed separately as
[v0.1.0-r2](https://github.com/CookiePiano/Pinboard/releases/tag/v0.1.0-r2), retaining
application version 0.1.0 at the maintainer's request. Download its setup again;
the original v0.1.0 file cannot update itself. Follow the
[release process](releasing.md) and [signing policy](code-signing-policy.md).
The checks above describe the earlier local validation build; r2's own manifest,
workflow and release notes identify the final distribution and its validation.
