# One-click MCP connection

Verified on 2026-10-21 (Asia/Seoul).

## Why

A user enabled MCP in Pinboard, but the Codex desktop app did not list it. The bridge itself worked: `Pinboard.exe --mcp --mcp-data-dir <data>` answered `initialize` and listed `create_note`. The problem was setup. `~/.codex/config.toml` had no `[mcp_servers.pinboard]` entry, because Pinboard only offered the configuration for the user to copy and paste. With the user's approval, the entry was added to that PC's config, and `codex mcp list` then showed `pinboard` as enabled.

## What changed

- Settings > Work with AI has a **Connect automatically** button for the selected apps. One click provisions MCP if needed, allows connections, and registers Pinboard in each chosen app's configuration (`connect_mcp_clients`).
- Files written (`src-tauri/src/mcp/clients.rs`):
  - Codex: `$CODEX_HOME/config.toml` or `~/.codex/config.toml`, via `toml_edit`. Comments, formatting and other servers are kept. Keys the user added to an existing `pinboard` entry (env, timeouts) are kept as well.
  - Claude Desktop: `%APPDATA%\Claude\claude_desktop_config.json`, plus the Microsoft Store copy under `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude\` when present. On Linux the path is `~/.config/Claude/`.
  - Gemini CLI: `~/.gemini/settings.json`.
- JSON keeps key order. `serde_json` now enables `preserve_order`. Note hashes do not depend on JSON map order, and all existing tests still pass.
- Safety:
  - An unreadable or unparsable file (for example, JSON with comments) is never overwritten. The error points to **Set up manually**.
  - The previous file is copied to `*.pinboard.bak` before an atomic replace.
  - An already-correct file is not rewritten.
- Each app card shows Connected / Needs reconnecting (the entry points to another exe or data folder) / Not connected (`mcp_client_status`). The copy-and-paste instructions moved into a **Set up manually** disclosure.

## Validation

- `cargo test --lib`: 62 passed. New tests cover:
  - TOML comment preservation, idempotence and repointing.
  - JSON key order.
  - Rejection of invalid files.
  - A temp-dir `register` run that checks the backup, the atomic write, the no-op rerun, and that a broken file is left untouched.
- `npm run check`: zero errors. `node scripts/test-i18n.mjs`: six languages complete.
- Browser harness (Korean, mocked IPC):
  - Selecting ChatGPT and Gemini sent `connect_mcp_clients` with `["codex","gemini"]`.
  - The switch turned on, and the ChatGPT badge changed to Connected with a restart notice.
  - A simulated Gemini error was shown per app.
  - This is frontend verification. The one-click flow was not run against the user's real Claude or Gemini configs.
