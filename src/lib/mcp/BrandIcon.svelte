<script lang="ts">
  import { brandPaths } from "./brand-icons";
  import type { McpClient } from "./ipc";

  let { client }: { client: McpClient } = $props();
</script>

<svg width="28" height="28" viewBox="0 0 24 24" aria-hidden="true" class:claude={client === "claude" || client === "claude_cli"}>
  {#if client === "gemini" || client === "antigravity"}
    <defs>
      <linearGradient id="pinboard-gemini-mark" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#6ba6ff" />
        <stop offset="52%" stop-color="#9c85eb" />
        <stop offset="100%" stop-color="#dc82a0" />
      </linearGradient>
    </defs>
  {/if}
  <path d={brandPaths[client === "claude_cli" ? "claude" : client === "antigravity" ? "gemini" : client]} fill={client === "gemini" || client === "antigravity" ? "url(#pinboard-gemini-mark)" : "currentColor"} />
</svg>

<style>
  svg { flex: none; }
  .claude { color: #d97757; }
</style>
