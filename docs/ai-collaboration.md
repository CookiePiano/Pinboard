# AI collaboration through MCP

Open **Work with AI**, immediately above About in Settings. Select one or more
ChatGPT, Claude or Gemini icon checkboxes, then choose **Connect automatically**.
This explicit action provisions MCP, enables connections and registers Pinboard in
the selected clients. Selection is remembered on this PC. Pinboard does not collect
or store API keys. The separate setup, connection and manual configuration controls
remain available.

## Connection workflow

| Client | Configuration location | Entry format |
| --- | --- | --- |
| ChatGPT: ChatGPT Desktop and Codex CLI | `$CODEX_HOME/config.toml`, or `~/.codex/config.toml` | `[mcp_servers.pinboard]` TOML |
| Claude: Claude Desktop and Claude Code CLI | `%APPDATA%\Claude\claude_desktop_config.json`, the detected Microsoft Store location, and `~/.claude.json` (or `$CLAUDE_CONFIG_DIR/.claude.json`) | `mcpServers.pinboard` JSON |
| Gemini: Antigravity GUI and Antigravity CLI | `~/.gemini/config/mcp_config.json` | `mcpServers.pinboard` JSON |

Automatic registration preserves other entries, TOML comments and existing custom
Pinboard fields such as environment variables and timeouts. It backs up each
changed file as `*.pinboard.bak` before atomic replacement and leaves already-correct
files unchanged. Unreadable or unparsable files are not overwritten; failures appear
per client. Every configuration in a group is parsed before writing that group.
Explicit connection re-enables the Pinboard entry and replaces conflicting remote
transport fields with stdio. If all registrations fail, connections are not enabled.
The status badges mean Configured, Needs reconnecting (different paths or a disabled
entry), or Not connected. Restart clients after
registration to load the server. These badges describe configuration registration,
not a verified live MCP session.

For supported platform paths and the implementation record, see
[one-click MCP connection](mcp-one-click-2026-10-21.md). The Claude Microsoft Store
location is under `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude\`;
Linux uses `~/.config/Claude/`.

### Manual fallback

Expand **Set up manually** when automatic registration is unavailable or when the
user prefers to edit configuration themselves:

1. Choose **Set up MCP**, then enable **Allow MCP connections**. The switch is
   disabled before provisioning. Starting Pinboard or opening this page does not
   provision MCP. Connections remain off after setup until explicitly enabled.
2. Select the relevant client icons and choose each client's **Copy connection
   settings** action.
3. Merge the `pinboard` entry into the existing configuration file. Preserve all
   other settings and servers. If JSON already has `mcpServers`, add only the
   `pinboard` member within it.
4. Reconnect or restart the client. Complete any account sign-in and tool approval
   in that client.
5. Keep Pinboard running and ask the AI to create a note, for example:
   "Create a yellow Pinboard note listing what I need for tomorrow's meeting."

The ChatGPT option generates configuration for local MCP through Codex, not an
ordinary ChatGPT web conversation. The Gemini option targets Antigravity GUI and CLI;
the older Gemini CLI identifier remains readable for backup compatibility. The
generated configuration includes the current Pinboard executable and data-directory
paths. Copy it again after moving the executable or changing its installation path.

Only the explicit automatic connection action writes external AI client
configuration. Manual setup, opening Settings and startup do not perform that
registration. Pinboard does not retrieve client credentials. Turning connections
off blocks new requests from already connected clients too. This setting
is PC-local and does not synchronize with notes.

Client checkboxes select registration targets and setup instructions. They are not an
authenticated client allowlist. The connection switch is the only permission gate.

Permission scope is versioned in mcp/settings.json as grant_version. An older
create-only grant does not enable the expanded tools after upgrade; explicitly enable
connections again after reviewing the current permission text. Version 2 covers the
five tools below. These instructions do not enforce per-operation approval dialogs.

## Tool contract

Since 2026-10-21 the AI can operate notes the way the user can in the app. The user
asked for this because the AI runs in their own signed-in environment. Before this
change the server exposed only `create_note`, and existing notes were never sent to
the AI. Five tools are available:

| Tool | Arguments | Behavior |
| --- | --- | --- |
| `list_notes` | `query?` (case-insensitive substring, ≤200 chars), `include_hidden?` (default `true`), `limit?` (1–200, default 50) | Live notes, newest `updatedAt` first, plus `total`. Each entry has `id`, `title` (first line, at most 200 characters), `preview` (≤200 chars of the rest), `tint`, `opacity`, `pin`, `visible`, and `createdAt`/`updatedAt` as KST RFC 3339 (`null` for the untouched welcome note). |
| `read_note` | `id` | Same fields with the full `body` (cut at 200,000 chars with `truncated: true`), `font` and `hasBackground`. |
| `create_note` | `body`, `tint?`, `pin?` (`normal`/`top`/`desktop`) | As below. |
| `update_note` | `id`, plus at least one of `body` (replace; may be empty), `append` (adds a new line at the end), `tint`, `opacity` (0–1), `pin`, `visible` | Uses the note window's edit path (`commands::patch_note`/`after_patch`): replacing text clears formatting runs, appending keeps them. `pin` and `visible` are this PC's window state, as in the app, and do not change `updatedAt`. |
| `delete_note` | `id` | The same tombstone delete as the app (`commands::tombstone_note`/`after_delete`): the window closes and the deletion synchronizes. Annotated `destructiveHint`. |

`body` and `append` are limited to 20,000 characters and must be non-blank (except
a replacement `body`). IDs must be canonical lowercase UUIDs from `list_notes`. Tool
arguments never accept file paths or commands, and unknown fields are rejected. The
server instructions ask the AI to confirm before deleting or replacing a whole note.

`create_note` returns the new note ID. Created notes use normal persistence and
synchronization. They copy the
style of the last-written note: the live note with non-blank text and the newest
`updatedAt` (`Store::draft_like_last_written`). The copied style is window size,
note font, tint, opacity and background image. An empty or untouched scratch
note therefore never sets the style of AI notes. Text and formatting runs are not
copied. Notes created with the + button, tray or shortcut keep the newest-created
rule. (2026-10-21: before this change, MCP used the newest-created note, so a
hidden empty note could override the note the user was actually writing in.)

## Local architecture and limits

The client launches the Pinboard executable with
`--mcp --mcp-data-dir <absolute-data-directory>`. This mode handles JSON-RPC over
stdio without starting Tauri, a tray icon, autostart or normal single-instance
handling. Node.js and a separate server installation are unnecessary.

The stdio process forwards requests through `mcp/inbox` and `mcp/replies` in the
application data directory. It opens no TCP port, HTTP server or external tunnel.
The running application owns note persistence; the stdio process does not open a
second note store that could conflict with it.

| Path | Contract |
| --- | --- |
| `mcp/settings.json` | PC-local explicit provisioning and connection permission. Before setup, do not create the MCP directory, settings or worker. Legacy automatically created settings do not count as explicit provisioning. |
| `mcp/runtime.json` | Runtime session and heartbeat. Restart and permission changes replace the session. |
| `mcp/inbox`, `mcp/replies` | Temporary UUID-named requests and results. Enforce 15-second expiry and session matching. Requests are limited to 128 KiB and replies to 2 MiB. Remove completed exchanges and clean up abandoned files. |

A request carries either `op` (a tool call tagged by `tool`) or the original
create-only `note` object. Exactly one must be present. The app keeps accepting
`note` because MCP processes a client started from an earlier executable stay
alive until the client restarts. Replies keep `noteId` for creates and add `result`
for everything else. A creation without `pin` is still sent in the original form,
so an app that has not been restarted since an update can serve it.

Keep these files in the user's private application data directory. This is not an
isolation boundary against malicious software running as the same Windows user
with write access. If a custom data directory is shared, its filesystem permissions
also determine who can access the bridge.

While connections are allowed, note text the AI lists or reads is sent to the
connected client and then becomes subject to that service's data policies and usage
limits. Credentials, settings, sync folders and files outside notes are never
exposed. Configured cloud-folder synchronization also applies to notes the AI
creates, edits or deletes.

## Troubleshooting

- **No note appears:** confirm Pinboard is running and connections are enabled.
  A tray-only process still counts as running.
- **Executable not found:** copy configuration from the current running location.
  Do not manually alter spaces or backslashes in the generated paths.
- **Connection disabled or restarted:** enable connections and send a new request.
  Do not reuse a request queued before permissions changed.
- **Timeout:** first check whether the note was already created. Do not blindly
  retry and create duplicates.
- **Copy fails:** open **View connection settings** and copy the displayed text.
