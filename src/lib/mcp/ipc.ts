import { invoke } from "@tauri-apps/api/core";

export interface McpSettings {
  provisioned: boolean;
  enabled: boolean;
  available: boolean;
  executable: string;
  arguments: string[];
}

export type McpClient = "codex" | "claude" | "claude_cli" | "antigravity" | "gemini";

export const providers = [
  { id: "codex", name: "ChatGPT", hint: "codexHint" },
  { id: "claude", name: "Claude", hint: "claudeHint" },
  { id: "antigravity", name: "Gemini", hint: "antigravityHint" },
] as const;

export type Registration = "registered" | "outdated" | "missing";
export interface ClientStatus { client: McpClient; path: string; registration: Registration }
export interface ConnectResult {
  settings: McpSettings;
  clients: ClientStatus[];
  errors: { client: McpClient; message: string }[];
}
export const mcpApi = {
  getSettings: () => invoke<McpSettings>("get_mcp_settings"),
  provision: () => invoke<McpSettings>("provision_mcp"),
  setEnabled: (enabled: boolean) => invoke<McpSettings>("set_mcp_enabled", { enabled }),
  clientStatus: () => invoke<ClientStatus[]>("mcp_client_status"),
  connect: (targets: McpClient[]) => invoke<ConnectResult>("connect_mcp_clients", { targets }),
};

export function configuration(settings: McpSettings, client: McpClient): string {
  if (client === "codex") {
    return [
      "[mcp_servers.pinboard]",
      `command = ${JSON.stringify(settings.executable)}`,
      `args = ${JSON.stringify(settings.arguments)}`,
      "enabled = true",
      "startup_timeout_sec = 15",
      "tool_timeout_sec = 25",
    ].join("\n");
  }
  return JSON.stringify({ mcpServers: { pinboard: { type: "stdio", command: settings.executable, args: settings.arguments } } }, null, 2);
}

export const configurationPaths: Record<McpClient, string> = {
  codex: "~/.codex/config.toml",
  claude: "%APPDATA%\\Claude\\claude_desktop_config.json · ~/.claude.json",
  claude_cli: "~/.claude.json",
  antigravity: "~/.gemini/config/mcp_config.json",
  gemini: "~/.gemini/settings.json",
};
